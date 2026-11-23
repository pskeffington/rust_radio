#!/usr/bin/env python3
"""Fail-closed, read-only preflight for shipping the curated station bundle."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any
from urllib.parse import urlsplit

MAX_INPUT_BYTES = 256 * 1024
ID_PATTERN = re.compile(r"[a-z0-9][a-z0-9_-]{0,63}\Z")


class GateError(Exception):
    """An input did not meet the distribution gate."""


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    obj: dict[str, Any] = {}
    for key, value in pairs:
        if key in obj:
            raise GateError(f"duplicate JSON key: {key}")
        obj[key] = value
    return obj


def _reject_constant(value: str) -> None:
    raise GateError(f"invalid JSON constant: {value}")


def _read_json(path: Path, label: str) -> tuple[bytes, Any]:
    try:
        with path.open("rb") as stream:
            raw = stream.read(MAX_INPUT_BYTES + 1)
    except OSError as exc:
        raise GateError(f"cannot read {label}") from exc
    if not raw or len(raw) > MAX_INPUT_BYTES:
        raise GateError(f"{label} must be between 1 and {MAX_INPUT_BYTES} bytes")
    try:
        value = json.loads(
            raw,
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise GateError(f"{label} is not valid UTF-8 JSON") from exc
    return raw, value


def _runtime_stations(source: str) -> dict[str, dict[str, str]]:
    declaration = re.search(
        r"pub\s+const\s+CURATED_STATIONS\s*:\s*\[\s*InternetStation\s*;\s*(\d+)\s*\]\s*=\s*\[(.*?)\];",
        source,
        re.DOTALL,
    )
    if declaration is None:
        raise GateError("cannot identify the compiled CURATED_STATIONS declaration")
    declared_count = int(declaration.group(1))
    body = declaration.group(2)
    blocks = re.findall(r"\bInternetStation\s*\{(.*?)\n\s*\}", body, re.DOTALL)
    if len(blocks) != declared_count:
        raise GateError("compiled station declaration count does not match its entries")

    stations: dict[str, dict[str, str]] = {}
    for block in blocks:
        fields: dict[str, str] = {}
        for field in ("id", "broadcaster"):
            values = re.findall(rf"\b{field}\s*:\s*\"([^\"]*)\"", block)
            if len(values) != 1:
                raise GateError(f"compiled station entry must have one {field}")
            fields[field] = values[0]
        station_id = fields["id"]
        if not ID_PATTERN.fullmatch(station_id):
            raise GateError("compiled station ID has an invalid format")
        if station_id in stations:
            raise GateError(f"duplicate compiled station ID: {station_id}")
        if not fields["broadcaster"].strip():
            raise GateError(f"compiled station {station_id} has no publisher")
        stations[station_id] = fields
    return stations


def _valid_evidence_ref(value: Any) -> bool:
    if not isinstance(value, str) or not value or value != value.strip():
        return False
    if any(ord(char) < 0x20 for char in value):
        return False
    parsed = urlsplit(value)
    return (
        parsed.scheme == "https"
        and bool(parsed.hostname)
        and parsed.username is None
        and parsed.password is None
        and not parsed.query
        and not parsed.fragment
    )


def validate(root: Path, manifest_path: Path) -> None:
    root = root.resolve()
    catalog_path = root / "catalog/curated-stations.json"
    runtime_path = root / "crates/rust-radio-core/src/directory.rs"

    catalog_raw, catalog = _read_json(catalog_path, "curated catalog")
    if not isinstance(catalog, dict):
        raise GateError("curated catalog must be a JSON object")
    policy = catalog.get("policy")
    if not isinstance(policy, dict) or policy.get("redistribution") != "authorized":
        raise GateError("catalog redistribution policy is not exactly authorized")
    catalog_entries = catalog.get("stations")
    if not isinstance(catalog_entries, list):
        raise GateError("catalog stations must be a list")

    catalog_stations: dict[str, dict[str, str]] = {}
    for entry in catalog_entries:
        if not isinstance(entry, dict):
            raise GateError("catalog station entry must be an object")
        station_id = entry.get("id")
        broadcaster = entry.get("broadcaster")
        if not isinstance(station_id, str) or not ID_PATTERN.fullmatch(station_id):
            raise GateError("catalog station ID has an invalid format")
        if not isinstance(broadcaster, str) or not broadcaster.strip():
            raise GateError(f"catalog station {station_id} has no publisher")
        if station_id in catalog_stations:
            raise GateError(f"duplicate catalog station ID: {station_id}")
        catalog_stations[station_id] = {"id": station_id, "broadcaster": broadcaster}

    try:
        with runtime_path.open("rb") as stream:
            runtime_raw = stream.read(MAX_INPUT_BYTES + 1)
        runtime_source = runtime_raw.decode("utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        raise GateError("cannot read compiled station source") from exc
    if not runtime_raw or len(runtime_raw) > MAX_INPUT_BYTES:
        raise GateError("compiled station source is outside the allowed size")
    runtime_stations = _runtime_stations(runtime_source)
    if catalog_stations != runtime_stations:
        raise GateError("catalog station IDs and publishers do not match compiled stations")

    manifest_raw, manifest = _read_json(manifest_path, "approval manifest")
    del manifest_raw  # Manifest content is validated below; it contains no credentials.
    if not isinstance(manifest, dict) or set(manifest) != {
        "schema_version",
        "catalog_sha256",
        "runtime_source_sha256",
        "approvals",
    }:
        raise GateError("approval manifest has missing or unknown top-level fields")
    if type(manifest["schema_version"]) is not int or manifest["schema_version"] != 1:
        raise GateError("approval manifest schema_version must be 1")
    catalog_digest = hashlib.sha256(catalog_raw).hexdigest()
    runtime_digest = hashlib.sha256(runtime_raw).hexdigest()
    if manifest["catalog_sha256"] != catalog_digest:
        raise GateError("approval manifest catalog digest is stale or malformed")
    if manifest["runtime_source_sha256"] != runtime_digest:
        raise GateError("approval manifest runtime source digest is stale or malformed")
    approvals = manifest["approvals"]
    if not isinstance(approvals, list):
        raise GateError("approval manifest approvals must be a list")

    approved: set[str] = set()
    for item in approvals:
        if not isinstance(item, dict) or set(item) != {
            "station_id",
            "publisher",
            "scope",
            "evidence_ref",
        }:
            raise GateError("each approval must contain exactly station_id, publisher, scope, evidence_ref")
        station_id = item["station_id"]
        if not isinstance(station_id, str) or not ID_PATTERN.fullmatch(station_id):
            raise GateError("approval station ID has an invalid format")
        if station_id in approved:
            raise GateError(f"duplicate approval for station: {station_id}")
        if station_id not in runtime_stations:
            raise GateError(f"approval refers to unknown station: {station_id}")
        if item["publisher"] != runtime_stations[station_id]["broadcaster"]:
            raise GateError(f"approval publisher does not match station: {station_id}")
        if item["scope"] != "app_distribution":
            raise GateError(f"approval scope is not app_distribution: {station_id}")
        if not _valid_evidence_ref(item["evidence_ref"]):
            raise GateError(f"approval evidence_ref must be an HTTPS reference without query or fragment: {station_id}")
        approved.add(station_id)

    missing = sorted(runtime_stations.keys() - approved)
    if missing:
        raise GateError(f"approval manifest is missing station IDs: {', '.join(missing)}")
    if len(approved) != len(runtime_stations):
        raise GateError("approval manifest does not cover the compiled station set exactly")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args()
    try:
        validate(args.root, args.manifest)
    except GateError as exc:
        print(f"distribution rights preflight blocked: {exc}", file=sys.stderr)
        return 1
    print("distribution rights preflight passed structural checks; this is not a legal or publication approval")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
