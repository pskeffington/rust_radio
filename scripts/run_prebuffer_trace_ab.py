#!/usr/bin/env python3
"""Run muted native-output A/B trials against loopback or curated audio."""

import argparse
from datetime import datetime, timezone
import json
import math
from pathlib import Path
import platform
import statistics
import subprocess
from urllib.parse import urlsplit


PROFILES = {
    "low-latency": 200,
    "balanced": 350,
    "resilient": 500,
}
MAX_OUTPUT_RATE_HZ = 192_000
MAX_OUTPUT_CHANNELS = 8
REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_URL = "http://127.0.0.1:18743/live"
CATALOG_PATH = REPO_ROOT / "catalog/curated-stations.json"


def is_finite_nonnegative_number(value):
    if type(value) not in (int, float) or value < 0:
        return False
    try:
        return math.isfinite(value)
    except OverflowError:
        return False


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
            f"trial {trial} {variant} failed (exit {result.returncode}; "
            f"diagnostic records={len(diagnostic_lines)})"
        )
    try:
        metrics = json.loads(diagnostic_lines[0])
    except json.JSONDecodeError as error:
        raise RuntimeError(f"trial {trial} {variant} emitted invalid diagnostics JSON") from error
    expected_fields = {
        "output_prebuffer_ms",
        "buffer_ready_latency_observed_ms",
        "first_play_latency_observed_ms",
        "output_samples_consumed_current_attempt",
        "prebuffer_fill_latency_observed_ms",
        "underrun_samples",
    }
    if not isinstance(metrics, dict) or set(metrics) != expected_fields:
        raise RuntimeError(f"trial {trial} {variant} emitted an invalid diagnostics schema")
    if (
        type(metrics["output_prebuffer_ms"]) is not int
        or metrics["output_prebuffer_ms"] != expected_prebuffer_ms
    ):
        raise RuntimeError(
            f"trial {trial} {variant} reported an invalid prebuffer; "
            f"expected {expected_prebuffer_ms}ms"
        )
    latency_fields = {
        "buffer_ready_latency_observed_ms",
        "first_play_latency_observed_ms",
        "prebuffer_fill_latency_observed_ms",
    }
    if any(
        not is_finite_nonnegative_number(metrics[field])
        or metrics[field] > (args.seconds + 60) * 1_000
        for field in latency_fields
    ):
        raise RuntimeError(f"trial {trial} {variant} emitted invalid latency diagnostics")
    counter_fields = {
        "output_samples_consumed_current_attempt",
        "underrun_samples",
    }
    maximum_sample_count = (
        MAX_OUTPUT_RATE_HZ * MAX_OUTPUT_CHANNELS * (args.seconds + 60)
    )
    if any(
        type(metrics[field]) is not int
        or not 0 <= metrics[field] <= maximum_sample_count
        for field in counter_fields
    ):
        raise RuntimeError(f"trial {trial} {variant} emitted invalid sample counters")
    return {
        "trial": trial,
        "variant": variant,
        "profile": profile,
        **metrics,
    }


def validate_curated_station(args):
    command = [str(args.cli), "validate", args.station, "--json"]
    try:
        result = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            timeout=60,
        )
    except subprocess.TimeoutExpired as error:
        raise RuntimeError("curated station preflight timed out") from error

    try:
        report = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError("curated station preflight returned invalid JSON") from error
    if not isinstance(report, dict):
        raise RuntimeError("curated station preflight returned an invalid report")
    if (
        type(report.get("schema_version")) is not int
        or report["schema_version"] != 1
        or set(report) != {"schema_version", "checked_at_unix", "results"}
        or not isinstance(report.get("results"), list)
        or len(report["results"]) != 1
    ):
        raise RuntimeError("curated station preflight returned an invalid report")
    validation = report["results"][0]
    validation_fields = {
        "station_id",
        "attempts",
        "recovered",
        "status",
        "stage",
        "checked_at_unix",
        "content_type",
        "codec",
        "sample_rate",
        "channels",
        "decoded_frames",
        "decoded",
    }
    if not isinstance(validation, dict) or set(validation) != validation_fields:
        raise RuntimeError("curated station preflight returned invalid station fields")
    if (
        result.returncode != 0
        or validation["station_id"] != args.station
        or validation["status"] != "decoded"
        or validation["stage"] != "decoded_audio"
        or validation["decoded"] is not True
    ):
        raise RuntimeError(
            "curated station preflight did not verify decoded audio "
            f"(status={validation.get('status')!r}, "
            f"stage={validation.get('stage')!r})"
        )
    numeric_fields = (
        "checked_at_unix",
        "attempts",
        "sample_rate",
        "channels",
        "decoded_frames",
    )
    if any(
        not isinstance(validation[field], int)
        or isinstance(validation[field], bool)
        or validation[field] <= 0
        for field in numeric_fields
    ):
        raise RuntimeError("curated station preflight returned invalid metadata")
    if (
        type(report["checked_at_unix"]) is not int
        or report["checked_at_unix"] != validation["checked_at_unix"]
    ):
        raise RuntimeError("curated station preflight returned invalid timestamps")
    if not isinstance(validation["codec"], str) or not validation["codec"]:
        raise RuntimeError("curated station preflight returned invalid codec metadata")
    if validation["attempts"] != 1 or validation["recovered"] is not False:
        raise RuntimeError("curated station preflight returned invalid attempt metadata")
    return report


def summarize(records, profiles):
    latency_metrics = {
        "buffer_ready": "buffer_ready_latency_observed_ms",
        "first_play": "first_play_latency_observed_ms",
        "prebuffer_fill": "prebuffer_fill_latency_observed_ms",
    }

    def describe(values):
        return {
            "mean": sum(values) / len(values),
            "median": statistics.median(values),
            "min": min(values),
            "max": max(values),
        }

    summaries = {}
    for variant, profile in zip(("A", "B"), profiles):
        group = [record for record in records if record["variant"] == variant]
        if not group:
            raise ValueError(f"no trials for variant {variant}")
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
            "latency_ms": {
                label: describe([record[field] for record in group])
                for label, field in latency_metrics.items()
            },
        }

    paired_differences = {label: [] for label in latency_metrics}
    for index in range(0, len(records) - 1, 2):
        pair = records[index : index + 2]
        by_variant = {record["variant"]: record for record in pair}
        if set(by_variant) != {"A", "B"}:
            raise ValueError("each adjacent trial pair must contain one A and one B trial")
        for label, field in latency_metrics.items():
            paired_differences[label].append(
                by_variant["A"][field] - by_variant["B"][field]
            )

    summaries["paired_latency_difference_ms"] = {
        "definition": "A minus B; negative values mean A was faster",
        "pairs": len(next(iter(paired_differences.values()))),
        "metrics": {
            label: describe(values) for label, values in paired_differences.items()
        },
    }
    return summaries


def repetition_order(repetition, profile_a, profile_b):
    """Alternate ABBA/BAAB so the first trial is counterbalanced."""
    variants = ("A", "B", "B", "A") if repetition % 2 == 0 else ("B", "A", "A", "B")
    profiles = {"A": profile_a, "B": profile_b}
    return tuple((variant, profiles[variant]) for variant in variants)


def main():
    args = parse_args()
    station_validation = (
        validate_curated_station(args) if args.station is not None else None
    )
    started_at_utc = datetime.now(timezone.utc).isoformat(timespec="seconds")
    records = []
    selected_profiles = (args.profile_a, args.profile_b)
    for repetition in range(args.repetitions):
        for variant, profile in repetition_order(
            repetition, args.profile_a, args.profile_b
        ):
            trial = len(records) + 1
            record = run_trial(args, trial, variant, profile)
            records.append(record)
            print(json.dumps(record), flush=True)

    result = {
        "started_at_utc": started_at_utc,
        "environment": platform.platform(),
        "url": args.url,
        "station": args.station,
        "station_validation": station_validation,
        "seconds": args.seconds,
        "volume": args.volume,
        "output": args.output,
        "repetitions": args.repetitions,
        "order_scheme": "alternating_abba_baab",
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
