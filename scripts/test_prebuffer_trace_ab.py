import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from run_prebuffer_trace_ab import (
    parse_args,
    main,
    repetition_order,
    run_trial,
    summarize,
    validate_curated_station,
)


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

    def test_trial_rejects_extra_url_diagnostics_without_leaking_values(self):
        diagnostics = {
            "output_prebuffer_ms": 350,
            "buffer_ready_latency_observed_ms": 400.0,
            "first_play_latency_observed_ms": 450.0,
            "output_samples_consumed_current_attempt": 1234,
            "prebuffer_fill_latency_observed_ms": 350.0,
            "underrun_samples": 0,
            "resolved_url": "http://127.0.0.1/live?token=private-token",
        }
        args = self.parse_with_temporary_cli(
            "--url", "http://127.0.0.1/live?token=private-token"
        )
        with patch("run_prebuffer_trace_ab.subprocess.run") as run:
            run.return_value.returncode = 0
            run.return_value.stdout = (
                f"RUST RADIO DIAGNOSTICS {json.dumps(diagnostics)}\n"
            )
            run.return_value.stderr = ""
            with self.assertRaisesRegex(RuntimeError, "invalid diagnostics schema") as error:
                run_trial(args, 1, "A", "balanced")

        self.assertNotIn("private-token", str(error.exception))
        self.assertNotIn("127.0.0.1", str(error.exception))

    def test_trial_failure_does_not_echo_cli_output(self):
        secret = "private-query-token"
        args = self.parse_with_temporary_cli(
            "--url", f"http://127.0.0.1/live?token={secret}"
        )
        with patch("run_prebuffer_trace_ab.subprocess.run") as run:
            run.return_value.returncode = 2
            run.return_value.stdout = f"failed URL contains {secret}"
            run.return_value.stderr = f"stream error includes {secret}"
            with self.assertRaisesRegex(RuntimeError, "diagnostic records=0") as error:
                run_trial(args, 1, "A", "balanced")

        self.assertNotIn(secret, str(error.exception))

    def test_trial_rejects_non_finite_or_mistyped_metrics(self):
        base = {
            "output_prebuffer_ms": 350,
            "buffer_ready_latency_observed_ms": 400.0,
            "first_play_latency_observed_ms": 450.0,
            "output_samples_consumed_current_attempt": 1234,
            "prebuffer_fill_latency_observed_ms": 350.0,
            "underrun_samples": 0,
        }
        malformed = (
            {**base, "buffer_ready_latency_observed_ms": float("nan")},
            {**base, "buffer_ready_latency_observed_ms": float("inf")},
            {**base, "buffer_ready_latency_observed_ms": 10**400},
            {**base, "first_play_latency_observed_ms": 1_000_000.0},
            {**base, "first_play_latency_observed_ms": True},
            {**base, "output_samples_consumed_current_attempt": -1},
            {**base, "output_samples_consumed_current_attempt": 104_448_001},
            {**base, "underrun_samples": False},
            {**base, "output_prebuffer_ms": True},
        )
        args = self.parse_with_temporary_cli("--station", "wpr-news")
        for diagnostics in malformed:
            with self.subTest(diagnostics=diagnostics):
                with patch("run_prebuffer_trace_ab.subprocess.run") as run:
                    run.return_value.returncode = 0
                    run.return_value.stdout = (
                        f"RUST RADIO DIAGNOSTICS {json.dumps(diagnostics)}\n"
                    )
                    run.return_value.stderr = ""
                    with self.assertRaises(RuntimeError):
                        run_trial(args, 1, "A", "balanced")

    def test_curated_station_preflight_requires_decoded_json(self):
        validation = {
            "station_id": "wpr-news",
            "attempts": 1,
            "recovered": False,
            "status": "decoded",
            "stage": "decoded_audio",
            "checked_at_unix": 1234,
            "content_type": "audio/mpeg",
            "codec": "MP3",
            "sample_rate": 44100,
            "channels": 1,
            "decoded_frames": 1152,
            "decoded": True,
        }
        report = {
            "schema_version": 1,
            "checked_at_unix": 1234,
            "results": [validation],
        }
        args = self.parse_with_temporary_cli("--station", "wpr-news")
        with patch("run_prebuffer_trace_ab.subprocess.run") as run:
            run.return_value.returncode = 0
            run.return_value.stdout = json.dumps(report)
            run.return_value.stderr = ""

            validation = validate_curated_station(args)

        self.assertEqual(validation, report)
        self.assertEqual(
            run.call_args.args[0][1:],
            ["validate", "wpr-news", "--json"],
        )
        self.assertEqual(run.call_args.kwargs["timeout"], 60)

    def test_curated_station_preflight_rejects_non_decoded_or_wrong_station(self):
        args = self.parse_with_temporary_cli("--station", "wpr-news")
        base = {
            "station_id": "wpr-news",
            "attempts": 1,
            "recovered": False,
            "status": "unavailable",
            "stage": "endpoint_reached",
            "checked_at_unix": 1234,
            "content_type": None,
            "codec": None,
            "sample_rate": None,
            "channels": None,
            "decoded_frames": 0,
            "decoded": False,
        }
        wrong_station = {
            **base,
            "status": "decoded",
            "stage": "decoded_audio",
            "decoded": True,
            "decoded_frames": 1152,
            "station_id": "skylab",
        }
        sensitive = {
            **wrong_station,
            "station_id": "wpr-news",
            "resolved_url": "https://example.test/live?token=secret",
        }
        reports = (
            {"schema_version": 1, "checked_at_unix": 1234, "results": [base]},
            {"schema_version": 1, "checked_at_unix": 1234, "results": [wrong_station]},
            {"schema_version": True, "checked_at_unix": 1234, "results": [base]},
            {"schema_version": 1, "checked_at_unix": 1234, "results": [sensitive]},
            {"schema_version": 1, "checked_at_unix": 1234, "results": []},
            {
                "schema_version": 1,
                "checked_at_unix": 1234,
                "resolved_url": "https://example.test/live?token=secret",
                "results": [base],
            },
        )
        for report in reports:
            with self.subTest(report=report):
                with patch("run_prebuffer_trace_ab.subprocess.run") as run:
                    run.return_value.returncode = 0
                    run.return_value.stdout = json.dumps(report)
                    run.return_value.stderr = ""
                    with self.assertRaisesRegex(RuntimeError, "preflight"):
                        validate_curated_station(args)

    def test_main_aborts_before_playback_when_station_preflight_fails(self):
        args = self.parse_with_temporary_cli("--station", "wpr-news")
        with (
            patch("run_prebuffer_trace_ab.parse_args", return_value=args),
            patch(
                "run_prebuffer_trace_ab.validate_curated_station",
                side_effect=RuntimeError("station did not decode"),
            ),
            patch("run_prebuffer_trace_ab.run_trial") as run_trial,
            contextlib.redirect_stderr(io.StringIO()),
        ):
            with self.assertRaisesRegex(RuntimeError, "did not decode"):
                main()
        run_trial.assert_not_called()

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
