#!/usr/bin/env python3
"""Render a credential-safe GitHub Actions summary for catalog validation."""

from __future__ import annotations

import json
import re
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


_ID = re.compile(r"[a-z0-9_-]{1,64}\Z")
_STATUSES = {
    "decoded",
    "unverified",
    "unavailable",
    "invalid_stream",
    "unsupported",
    "cancelled",
    "check_error",
}
_STAGES = {
    "not_checked",
    "endpoint_reached",
    "stream_response",
    "supported_codec",
    "decoded_audio",
}


def _safe(value: Any, allowed: set[str], fallback: str = "unknown") -> str:
    return value if isinstance(value, str) and value in allowed else fallback


def _station_id(value: Any) -> str:
    return value if isinstance(value, str) and _ID.fullmatch(value) else "unknown"


def render_summary(report: dict[str, Any]) -> str:
    version = report.get("schema_version")
    if not isinstance(version, int) or isinstance(version, bool) or version != 1:
        raise ValueError("unsupported validation report schema version")
    results = report.get("results")
    if not isinstance(results, list):
        raise ValueError("validation report must contain a results array")

    statuses = Counter(
        _safe(result.get("status"), _STATUSES)
        if isinstance(result, dict)
        else "unknown"
        for result in results
    )
    decoded = statuses["decoded"]
    lines = ["## Curated station validation", "", f"Decoded: **{decoded}/{len(results)}**"]

    checked_at = report.get("checked_at_unix")
    if isinstance(checked_at, int) and not isinstance(checked_at, bool):
        try:
            when = datetime.fromtimestamp(checked_at, timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
        except (OverflowError, OSError, ValueError):
            pass
        else:
            lines.append(f"Checked: {when}")

    if statuses:
        counts = ", ".join(f"{status}: {count}" for status, count in sorted(statuses.items()))
        lines.extend([f"Outcomes: {counts}", ""])
    else:
        lines.append("")

    lines.extend(["| Station | Status | Stage | Attempts | Recovered |", "|---|---|---|---:|:---:|"])
    for result in results:
        if not isinstance(result, dict):
            result = {}
        station = _station_id(result.get("station_id"))
        status = _safe(result.get("status"), _STATUSES)
        stage = _safe(result.get("stage"), _STAGES)
        attempts = result.get("attempts")
        attempts = attempts if isinstance(attempts, int) and not isinstance(attempts, bool) else 0
        attempts = min(max(attempts, 0), 255)
        recovered = "yes" if result.get("recovered") is True else "no"
        lines.append(f"| {station} | {status} | {stage} | {attempts} | {recovered} |")

    return "\n".join(lines) + "\n"


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: summarize_station_validation.py REPORT.json", file=sys.stderr)
        return 2
    report = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    if not isinstance(report, dict):
        raise ValueError("validation report must be a JSON object")
    sys.stdout.write(render_summary(report))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
