#!/usr/bin/env python3
"""Extract one bounded response-body timing trace from the ignored Rust test log."""

import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("log_file", type=Path)
    parser.add_argument("--station", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    prefix = f"RUST RADIO TIMING TRACE {args.station} "
    try:
        log = args.log_file.read_text()
    except OSError as error:
        parser.error(f"cannot read timing log: {error}")
    lines = [line[len(prefix) :] for line in log.splitlines() if line.startswith(prefix)]
    if not lines:
        parser.error(f"no timing trace found for station {args.station}")

    try:
        samples = json.loads(lines[-1])
    except json.JSONDecodeError as error:
        parser.error(f"timing trace is invalid JSON: {error}")
    if not isinstance(samples, list) or not 2 <= len(samples) <= 256:
        parser.error("timing trace must contain 2..256 response-body samples")
    for index, sample in enumerate(samples):
        if not isinstance(sample, dict):
            parser.error("timing trace entries must be objects")
        body_bytes = sample.get("body_bytes")
        interval_us = sample.get("interval_us")
        if not isinstance(body_bytes, int) or isinstance(body_bytes, bool) or not 1 <= body_bytes <= 65_536:
            parser.error("body_bytes must be an integer in 1..65536")
        if index == 0:
            if interval_us is not None:
                parser.error("the first interval_us value must be null")
        elif not isinstance(interval_us, int) or isinstance(interval_us, bool) or not 0 <= interval_us <= 10_000_000:
            parser.error("interval_us values must be integer microseconds in 0..10000000")

    args.output.write_text(json.dumps(samples, separators=(",", ":")) + "\n")
    print(f"saved {len(samples)} bounded timing samples for {args.station}")


if __name__ == "__main__":
    main()
