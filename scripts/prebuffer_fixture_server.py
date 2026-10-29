#!/usr/bin/env python3
"""Serve the bundled synthetic MP3 with a repeatable chunk gap for A/B tests."""

from argparse import ArgumentParser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
from time import sleep
from urllib.parse import parse_qs, urlsplit


DEFAULT_FIXTURE = (
    Path(__file__).resolve().parent.parent
    / "crates/rust-radio-audio/tests/fixtures/prebuffer-stress.mp3"
)


class FixtureHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def do_GET(self):
        query = parse_qs(urlsplit(self.path).query)
        try:
            stall_ms = int(query.get("stall_ms", [str(self.server.stall_ms)])[0])
        except ValueError:
            self.send_error(400, "stall_ms must be an integer")
            return
        if not 0 <= stall_ms <= 5_000:
            self.send_error(400, "stall_ms must be between 0 and 5000")
            return

        self.send_response(200)
        self.send_header("Content-Type", "audio/mpeg")
        self.send_header("Transfer-Encoding", "chunked")
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.flush()
        fixture = self.server.fixture
        chunk_size = self.server.chunk_bytes
        schedule = dict(self.server.stall_schedule)
        timing_trace = self.server.timing_trace
        offset = 0
        index = 0
        try:
            while offset < len(fixture):
                trace_sample = timing_trace[index] if index < len(timing_trace) else None
                if index > 0:
                    if trace_sample is not None:
                        sleep(trace_sample[0] / 1_000_000)
                    else:
                        sleep(self.server.interval_ms / 1_000)
                scheduled_stall_ms = schedule.get(index)
                if scheduled_stall_ms is not None:
                    sleep(scheduled_stall_ms / 1_000)
                elif not timing_trace and not schedule and index == self.server.stall_after_chunks and stall_ms:
                    sleep(stall_ms / 1_000)
                payload_size = trace_sample[1] if trace_sample is not None else chunk_size
                payload = fixture[offset : offset + payload_size]
                self.wfile.write(f"{len(payload):X}\r\n".encode("ascii"))
                self.wfile.write(payload)
                self.wfile.write(b"\r\n")
                self.wfile.flush()
                offset += len(payload)
                index += 1
            self.wfile.write(b"0\r\n\r\n")
            self.wfile.flush()
            print(
                f"served {len(fixture)} bytes; "
                f"schedule={self.server.stall_schedule or ((self.server.stall_after_chunks, stall_ms),)}",
                flush=True,
            )
        except (BrokenPipeError, ConnectionResetError):
            print("client stopped playback before fixture ended", flush=True)

    def log_message(self, fmt, *args):
        pass


class LoopbackServer(ThreadingHTTPServer):
    allow_reuse_address = True
    daemon_threads = True


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=18743)
    parser.add_argument("--stall-ms", type=int, default=500)
    parser.add_argument("--stall-after-chunks", type=int, default=20)
    parser.add_argument(
        "--stall-schedule",
        help="comma-separated chunk-index:delay-ms pairs, e.g. 20:350,38:500,54:250",
    )
    parser.add_argument(
        "--timing-trace-file",
        type=Path,
        help="JSON response-body chunk timing samples from extract_live_timing_trace.py",
    )
    parser.add_argument("--chunk-bytes", type=int, default=1600)
    parser.add_argument("--interval-ms", type=int, default=100)
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    args = parser.parse_args()
    if not 0 <= args.port <= 65535:
        parser.error("port must be between 0 and 65535")
    if not 0 <= args.stall_ms <= 5_000:
        parser.error("stall-ms must be between 0 and 5000")
    if args.stall_after_chunks < 0 or args.chunk_bytes <= 0:
        parser.error("stall-after-chunks must be nonnegative and chunk-bytes positive")
    if not 1 <= args.interval_ms <= 5_000:
        parser.error("interval-ms must be between 1 and 5000")
    try:
        fixture = args.fixture.read_bytes()
    except OSError as error:
        parser.error(f"cannot read fixture: {error}")
    if not fixture:
        parser.error("fixture file is empty")

    stall_schedule = []
    if args.stall_schedule:
        try:
            for item in args.stall_schedule.split(","):
                chunk_text, delay_text = item.split(":", maxsplit=1)
                chunk_index, delay_ms = int(chunk_text), int(delay_text)
                if chunk_index <= 0 or not 1 <= delay_ms <= 5_000:
                    raise ValueError
                stall_schedule.append((chunk_index, delay_ms))
        except ValueError:
            parser.error("stall-schedule must use positive chunk:delay-ms pairs (1..5000ms)")
        if len(stall_schedule) > 32 or len({chunk for chunk, _ in stall_schedule}) != len(stall_schedule):
            parser.error("stall-schedule must contain at most 32 unique chunk indices")
        if stall_schedule != sorted(stall_schedule):
            parser.error("stall-schedule chunk indices must be increasing")

    interval_trace = []
    if args.timing_trace_file:
        if stall_schedule:
            parser.error("timing-trace-file cannot be combined with stall-schedule")
        try:
            trace_samples = json.loads(args.timing_trace_file.read_text())
        except (OSError, json.JSONDecodeError) as error:
            parser.error(f"cannot read timing trace: {error}")
        if not isinstance(trace_samples, list) or not 2 <= len(trace_samples) <= 256:
            parser.error("timing trace must contain 2..256 response-body samples")
        for index, sample in enumerate(trace_samples):
            if (
                not isinstance(sample, dict)
                or not isinstance(sample.get("body_bytes"), int)
                or isinstance(sample.get("body_bytes"), bool)
            ):
                parser.error("timing trace samples must contain body_bytes and interval_us")
            body_bytes = sample["body_bytes"]
            interval_us = sample.get("interval_us")
            if not 1 <= body_bytes <= 65_536:
                parser.error("timing trace body_bytes entries must be in 1..65536")
            if index == 0:
                if interval_us is not None:
                    parser.error("the first timing trace interval must be null")
            elif (
                not isinstance(interval_us, int)
                or isinstance(interval_us, bool)
                or not 0 <= interval_us <= 10_000_000
            ):
                parser.error("timing trace intervals must be integer microseconds in 0..10000000")
            interval_trace.append((interval_us or 0, body_bytes))
        if sum(sample["body_bytes"] for sample in trace_samples) > len(fixture):
            parser.error("fixture is too small for the captured timing trace")

    server = LoopbackServer(("127.0.0.1", args.port), FixtureHandler)
    server.stall_ms = args.stall_ms
    server.stall_after_chunks = args.stall_after_chunks
    server.stall_schedule = tuple(stall_schedule)
    server.chunk_bytes = args.chunk_bytes
    server.interval_ms = args.interval_ms
    server.fixture = fixture
    server.timing_trace = tuple(interval_trace)
    print(
        f"serving synthetic MP3 on http://127.0.0.1:{server.server_port}/live"
        f"; default_stall={args.stall_ms}ms after chunk {args.stall_after_chunks}; "
        f"chunk_bytes={args.chunk_bytes}; interval_ms={args.interval_ms}; "
        f"stall_schedule={server.stall_schedule}; "
        f"timing_trace_samples={len(server.timing_trace)}; fixture_bytes={len(fixture)}",
        flush=True,
    )
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
