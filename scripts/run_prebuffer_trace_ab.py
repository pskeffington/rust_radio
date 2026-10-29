#!/usr/bin/env python3
"""Run muted native-output A/B trials against loopback or curated audio."""

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import platform
import subprocess
from urllib.parse import urlsplit


PROFILES = {
    "low-latency": 200,
    "balanced": 350,
    "resilient": 500,
}
REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_URL = "http://127.0.0.1:18743/live"
CATALOG_PATH = REPO_ROOT / "catalog/curated-stations.json"


def parse_args():
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group()
    source.add_argument("--url", help="loopback-only fixture URL")
    source.add_argument(
        "--station",
        choices=sorted(station["id"] for station in json.loads(CATALOG_PATH.read_text())["stations"]),
        help="curated station ID; playback uses the CLI's publisher allowlist",
    )
    parser.add_argument("--cli", type=Path, default=REPO_ROOT / "target/debug/rust-radio-cli")
    parser.add_argument("--seconds", type=int, default=8)
    parser.add_argument("--volume", type=int, default=0)
    parser.add_argument("--output", default="system")
    parser.add_argument("--repetitions", type=int, default=2)
    parser.add_argument("--profile-a", choices=PROFILES, default="low-latency")
    parser.add_argument("--profile-b", choices=PROFILES, default="balanced")
    parser.add_argument("--output-file", type=Path)
    args = parser.parse_args()

    if args.station is None:
        args.url = args.url or DEFAULT_URL
        parsed_url = urlsplit(args.url)
        if parsed_url.scheme != "http" or parsed_url.hostname != "127.0.0.1":
            parser.error("url must be an HTTP URL on the loopback IPv4 address 127.0.0.1")
        try:
            port = parsed_url.port
        except ValueError as error:
            parser.error(f"url contains an invalid port: {error}")
        if port == 0:
            parser.error("url port must be between 1 and 65535")
        if parsed_url.username is not None or parsed_url.password is not None:
            parser.error("url must not contain credentials")
    if not 1 <= args.seconds <= 300:
        parser.error("seconds must be in 1..300")
    if not 0 <= args.volume <= 100:
        parser.error("volume must be in 0..100 (default 0 keeps trials muted)")
    if not 1 <= args.repetitions <= 10:
        parser.error("repetitions must be in 1..10")
    if args.profile_a == args.profile_b:
        parser.error("profile-a and profile-b must name different profiles")
    if not args.cli.is_file():
        parser.error(f"CLI binary does not exist: {args.cli}")
    return args


def run_trial(args, trial, variant, profile):
    expected_prebuffer_ms = PROFILES[profile]
    command = [str(args.cli), "listen", args.station] if args.station else [
        str(args.cli),
        "stream-listen",
        args.url,
    ]
    command.extend([
        "--seconds",
        str(args.seconds),
        "--volume",
        str(args.volume),
        "--output",
        args.output,
        "--buffering",
        profile,
    ])
    try:
        result = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            timeout=args.seconds + 60,
        )
    except subprocess.TimeoutExpired as error:
        raise RuntimeError(f"trial {trial} {variant} timed out after {error.timeout}s") from error

    diagnostic_lines = [
        line.split("RUST RADIO DIAGNOSTICS ", 1)[1]
        for line in result.stdout.splitlines()
        if line.startswith("RUST RADIO DIAGNOSTICS ")
    ]
    if result.returncode or len(diagnostic_lines) != 1:
        raise RuntimeError(
            f"trial {trial} {variant} failed (exit {result.returncode});\n"
            f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    try:
        metrics = json.loads(diagnostic_lines[0])
    except json.JSONDecodeError as error:
        raise RuntimeError(f"trial {trial} {variant} emitted invalid diagnostics JSON") from error
    if metrics.get("output_prebuffer_ms") != expected_prebuffer_ms:
        raise RuntimeError(
            f"trial {trial} {variant} reported prebuffer "
            f"{metrics.get('output_prebuffer_ms')!r}, expected {expected_prebuffer_ms}ms"
        )
    required = {
        "buffer_ready_latency_observed_ms",
        "first_play_latency_observed_ms",
        "output_samples_consumed_current_attempt",
        "prebuffer_fill_latency_observed_ms",
        "underrun_samples",
    }
    missing = required.difference(metrics)
    if missing:
        raise RuntimeError(f"trial {trial} {variant} omitted diagnostics: {sorted(missing)}")
    return {
        "trial": trial,
        "variant": variant,
        "profile": profile,
        **metrics,
    }


def summarize(records, profiles):
    summaries = {}
    for variant, profile in zip(("A", "B"), profiles):
        group = [record for record in records if record["variant"] == variant]
        totals = {
            "output_samples_consumed_current_attempt": sum(
                record["output_samples_consumed_current_attempt"] for record in group
            ),
            "underrun_samples": sum(record["underrun_samples"] for record in group),
        }
        missing = totals["underrun_samples"]
        consumed = totals["output_samples_consumed_current_attempt"]
        summaries[variant] = {
            "profile": profile,
            "prebuffer_ms": PROFILES[profile],
            "trials": len(group),
            "mean_buffer_ready_latency_observed_ms": sum(
                record["buffer_ready_latency_observed_ms"] for record in group
            )
            / len(group),
            "mean_first_play_latency_observed_ms": sum(
                record["first_play_latency_observed_ms"] for record in group
            )
            / len(group),
            "mean_prebuffer_fill_latency_observed_ms": sum(
                record["prebuffer_fill_latency_observed_ms"] for record in group
            )
            / len(group),
            **totals,
            "missing_sample_share_percent": 100 * missing / (missing + consumed)
            if missing + consumed
            else None,
        }
    return summaries


def main():
    args = parse_args()
    started_at_utc = datetime.now(timezone.utc).isoformat(timespec="seconds")
    records = []
    selected_profiles = (args.profile_a, args.profile_b)
    for repetition in range(args.repetitions):
        order = (
            ("A", args.profile_a),
            ("B", args.profile_b),
            ("B", args.profile_b),
            ("A", args.profile_a),
        )
        for variant, profile in order:
            trial = len(records) + 1
            record = run_trial(args, trial, variant, profile)
            records.append(record)
            print(json.dumps(record), flush=True)

    result = {
        "started_at_utc": started_at_utc,
        "environment": platform.platform(),
        "url": args.url,
        "station": args.station,
        "seconds": args.seconds,
        "volume": args.volume,
        "output": args.output,
        "repetitions": args.repetitions,
        "profiles": {"A": args.profile_a, "B": args.profile_b},
        "trial_order": [record["variant"] for record in records],
        "records": records,
        "summary": summarize(records, selected_profiles),
    }
    serialized = json.dumps(result, indent=2) + "\n"
    if args.output_file:
        args.output_file.write_text(serialized)
    print(serialized, end="")


if __name__ == "__main__":
    main()
