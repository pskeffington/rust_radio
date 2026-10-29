import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from run_prebuffer_trace_ab import parse_args, repetition_order, run_trial, summarize


class PrebufferTraceAbTests(unittest.TestCase):
    def parse_with_temporary_cli(self, *args):
        with tempfile.NamedTemporaryFile() as cli:
            with patch.object(sys, "argv", ["run_prebuffer_trace_ab.py", "--cli", cli.name, *args]):
                return parse_args()

    def test_curated_station_mode_selects_only_catalog_ids(self):
        args = self.parse_with_temporary_cli("--station", "skylab")
        self.assertEqual(args.station, "skylab")
        self.assertIsNone(args.url)

        with contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit):
                self.parse_with_temporary_cli("--station", "unknown-station")

    def test_arbitrary_urls_and_mixed_sources_are_rejected(self):
        for args in (
            ("--url", "https://example.com/live"),
            ("--url", "http://127.0.0.1.evil/live"),
            ("--url", "http://user:pass@127.0.0.1/live"),
            ("--station", "skylab", "--url", "http://127.0.0.1/live"),
        ):
            with self.subTest(args=args), contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit):
                    self.parse_with_temporary_cli(*args)

    def test_curated_trial_uses_station_cli_command(self):
        diagnostics = {
            "output_prebuffer_ms": 350,
            "buffer_ready_latency_observed_ms": 400.0,
            "first_play_latency_observed_ms": 450.0,
            "output_samples_consumed_current_attempt": 1234,
            "prebuffer_fill_latency_observed_ms": 350.0,
            "underrun_samples": 0,
        }
        output = f"RUST RADIO DIAGNOSTICS {json.dumps(diagnostics)}\n"
        args = self.parse_with_temporary_cli("--station", "skylab")
        with patch("run_prebuffer_trace_ab.subprocess.run") as run:
            run.return_value.returncode = 0
            run.return_value.stdout = output
            run.return_value.stderr = ""
            record = run_trial(args, 1, "A", "balanced")

        command = run.call_args.args[0]
        self.assertEqual(command[1:3], ["listen", "skylab"])
        self.assertEqual(record["profile"], "balanced")
        self.assertEqual(record["output_prebuffer_ms"], 350)

    def test_repetition_order_counterbalances_first_profile(self):
        first = repetition_order(0, "balanced", "resilient")
        second = repetition_order(1, "balanced", "resilient")

        self.assertEqual([variant for variant, _ in first], ["A", "B", "B", "A"])
        self.assertEqual([variant for variant, _ in second], ["B", "A", "A", "B"])
        self.assertEqual(first[0][1], "balanced")
        self.assertEqual(second[0][1], "resilient")

    def test_summary_retains_trial_spread_and_paired_latency_differences(self):
        values = (10.0, 12.0, 20.0, 25.0)
        records = []
        for index, (variant, value) in enumerate(zip(("A", "B", "B", "A"), values)):
            records.append(
                {
                    "trial": index + 1,
                    "variant": variant,
                    "buffer_ready_latency_observed_ms": value,
                    "first_play_latency_observed_ms": value + 5,
                    "prebuffer_fill_latency_observed_ms": value - 3,
                    "output_samples_consumed_current_attempt": 100,
                    "underrun_samples": 0,
                }
            )

        result = summarize(records, ("balanced", "resilient"))

        balanced = result["A"]
        self.assertEqual(balanced["mean_buffer_ready_latency_observed_ms"], 17.5)
        self.assertEqual(
            balanced["latency_ms"]["buffer_ready"],
            {"mean": 17.5, "median": 17.5, "min": 10.0, "max": 25.0},
        )
        paired = result["paired_latency_difference_ms"]
        self.assertEqual(paired["pairs"], 2)
        self.assertEqual(
            paired["metrics"]["buffer_ready"],
            {"mean": 1.5, "median": 1.5, "min": -2.0, "max": 5.0},
        )
        self.assertIn("A minus B", paired["definition"])

    def test_summary_rejects_unpaired_trial_order(self):
        record = {
            "variant": "A",
            "buffer_ready_latency_observed_ms": 10.0,
            "first_play_latency_observed_ms": 12.0,
            "prebuffer_fill_latency_observed_ms": 8.0,
            "output_samples_consumed_current_attempt": 100,
            "underrun_samples": 0,
        }

        mismatched_pair = [
            record,
            dict(record),
            {**record, "variant": "B"},
            {**record, "variant": "B"},
        ]
        with self.assertRaisesRegex(ValueError, "one A and one B"):
            summarize(mismatched_pair, ("balanced", "resilient"))


if __name__ == "__main__":
    unittest.main()
