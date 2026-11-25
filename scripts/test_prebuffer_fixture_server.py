import json
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import unittest


REPO_ROOT = Path(__file__).resolve().parent.parent
SERVER = REPO_ROOT / "scripts/prebuffer_fixture_server.py"


class PrebufferFixtureServerTests(unittest.TestCase):
    def test_captured_timing_trace_can_add_scheduled_stalls(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture = root / "fixture.mp3"
            fixture.write_bytes(b"ab")
            trace = root / "trace.json"
            trace.write_text(
                json.dumps(
                    [
                        {"body_bytes": 1, "interval_us": None},
                        {"body_bytes": 1, "interval_us": 1_000},
                    ]
                )
            )
            process = subprocess.Popen(
                [
                    sys.executable,
                    "-B",
                    str(SERVER),
                    "--port",
                    "0",
                    "--fixture",
                    str(fixture),
                    "--timing-trace-file",
                    str(trace),
                    "--stall-schedule",
                    "1:350",
                ],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            ready = threading.Event()
            lines = []

            def read_ready_line():
                lines.append(process.stdout.readline())
                ready.set()

            reader = threading.Thread(target=read_ready_line, daemon=True)
            reader.start()
            try:
                self.assertTrue(ready.wait(5), "fixture server did not report readiness")
                self.assertIn("timing_trace_samples=2", lines[0])
                self.assertIn("stall_schedule=((1, 350),)", lines[0])
                self.assertIsNone(process.poll(), "fixture server exited before serving")
            finally:
                process.terminate()
                process.communicate(timeout=5)
                reader.join(timeout=1)


if __name__ == "__main__":
    unittest.main()
