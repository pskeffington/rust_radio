import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

from run_prebuffer_trace_ab import parse_args, run_trial


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


if __name__ == "__main__":
    unittest.main()
