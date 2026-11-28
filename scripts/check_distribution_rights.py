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
STATION_STRING_FIELDS = (
    "id",
    "name",
    "broadcaster",
    "genre",
    "language",
    "country",
    "category",
    "playlist_url",
    "publisher_reference_url",
    "rights_reviewed_date",
    "format",
)
STATION_BOOL_FIELDS = ("third_party_player_support",)
STATION_FIELDS = (
    "id",
    "name",
    "broadcaster",
    "genre",
    "language",
    "country",
    "category",
    "playlist_url",
    "publisher_reference_url",
    "third_party_player_support",
    "rights_reviewed_date",
    "nominal_kbps",
    "format",
)
STATION_FIELD_SET = frozenset(STATION_FIELDS)


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


def _rust_tokens(source: str) -> list[tuple[str, str, bool]]:
    """Tokenize the Rust syntax used by the curated station declaration.

    This is deliberately not a general Rust parser. It skips nested comments,
    keeps string contents atomic, and fails closed when the declaration uses
    metadata expressions this preflight cannot safely interpret.
    """
    tokens: list[tuple[str, str, bool]] = []
    index = 0

    def quoted(start: int, quote: str) -> tuple[str, bool, int]:
        cursor = start + 1
        value: list[str] = []
        escaped = False
        while cursor < len(source):
            char = source[cursor]
            if char == "\\":
                escaped = True
                if cursor + 1 >= len(source):
                    raise GateError("unterminated Rust string or character literal")
                value.append(source[cursor + 1])
                cursor += 2
            elif char == quote:
                return "".join(value), escaped, cursor + 1
            else:
                value.append(char)
                cursor += 1
        raise GateError("unterminated Rust string or character literal")

    while index < len(source):
        char = source[index]
        if char.isspace():
            index += 1
            continue

        if source.startswith("//", index):
            newline = source.find("\n", index + 2)
            index = len(source) if newline < 0 else newline + 1
            continue

        if source.startswith("/*", index):
            depth = 1
            index += 2
            while index < len(source) and depth:
                if source.startswith("/*", index):
                    depth += 1
                    index += 2
                elif source.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            if depth:
                raise GateError("unterminated Rust block comment")
            continue

        # Rust raw strings can contain quotes and comment markers verbatim.
        raw_prefix = "br" if source.startswith("br", index) else "r" if char == "r" else None
        if raw_prefix is not None:
            cursor = index + len(raw_prefix)
            while cursor < len(source) and source[cursor] == "#":
                cursor += 1
            if cursor < len(source) and source[cursor] == '"':
                hashes = source[index + len(raw_prefix) : cursor]
                terminator = '"' + hashes
                end = source.find(terminator, cursor + 1)
                if end < 0:
                    raise GateError("unterminated Rust raw string literal")
                kind = "byte_string" if raw_prefix == "br" else "string"
                tokens.append((kind, source[cursor + 1 : end], False))
                index = end + len(terminator)
                continue

        if source.startswith('b"', index):
            value, escaped, index = quoted(index + 1, '"')
            tokens.append(("byte_string", value, escaped))
            continue

        if char == '"':
            value, escaped, index = quoted(index, '"')
            tokens.append(("string", value, escaped))
            continue

        if char == "'":
            # Rust lifetimes have no closing apostrophe; character literals do.
            cursor = index + 1
            if cursor < len(source) and (source[cursor].isalpha() or source[cursor] == "_"):
                cursor += 1
                while cursor < len(source) and (source[cursor].isalnum() or source[cursor] == "_"):
                    cursor += 1
                if cursor >= len(source) or source[cursor] != "'":
                    tokens.append(("lifetime", source[index + 1 : cursor], False))
                    index = cursor
                    continue
            _, _, index = quoted(index, "'")
            tokens.append(("character", "", False))
            continue

        if char.isalpha() or char == "_":
            cursor = index + 1
            while cursor < len(source) and (source[cursor].isalnum() or source[cursor] == "_"):
                cursor += 1
            tokens.append(("identifier", source[index:cursor], False))
            index = cursor
            continue

        if char.isdigit():
            cursor = index + 1
            while cursor < len(source) and (source[cursor].isdigit() or source[cursor] == "_"):
                cursor += 1
            tokens.append(("number", source[index:cursor], False))
            index = cursor
            continue

        tokens.append(("punctuation", char, False))
        index += 1

    return tokens


def _matches(tokens: list[tuple[str, str, bool]], start: int, pattern: tuple[tuple[str, str], ...]) -> bool:
    return start + len(pattern) <= len(tokens) and all(
        tokens[start + offset][:2] == expected for offset, expected in enumerate(pattern)
    )


def _runtime_stations(source: str) -> dict[str, dict[str, str | int]]:
    tokens = _rust_tokens(source)
    declaration_prefix = (
        ("identifier", "pub"),
        ("identifier", "const"),
        ("identifier", "CURATED_STATIONS"),
        ("punctuation", ":"),
        ("punctuation", "["),
        ("identifier", "InternetStation"),
        ("punctuation", ";"),
    )
    declarations = [
        index
        for index in range(len(tokens))
        if _matches(tokens, index, declaration_prefix)
    ]
    if len(declarations) != 1:
        raise GateError("cannot uniquely identify the compiled CURATED_STATIONS declaration")

    declaration = declarations[0]
    count_token = tokens[declaration + len(declaration_prefix)]
    suffix_start = declaration + len(declaration_prefix) + 1
    if count_token[0] != "number" or not re.fullmatch(r"[0-9]+", count_token[1]):
        raise GateError("compiled station declaration has an unsupported entry count")
    declared_count = int(count_token[1])
    if not _matches(
        tokens,
        suffix_start,
        (("punctuation", "]"), ("punctuation", "="), ("punctuation", "[")),
    ):
        raise GateError("compiled station declaration has an unsupported array form")

    cursor = suffix_start + 3
    stations: dict[str, dict[str, str | int | bool]] = {}
    string_fields = frozenset(STATION_STRING_FIELDS)

    while cursor < len(tokens) and tokens[cursor][:2] != ("punctuation", "]"):
        if not _matches(tokens, cursor, (("identifier", "InternetStation"), ("punctuation", "{"))):
            raise GateError("compiled station array contains an unsupported entry")
        cursor += 2
        fields: dict[str, str | int | bool] = {}

        while cursor < len(tokens) and tokens[cursor][:2] != ("punctuation", "}"):
            field_token = tokens[cursor]
            if field_token[0] != "identifier" or cursor + 1 >= len(tokens) or tokens[cursor + 1][:2] != ("punctuation", ":"):
                raise GateError("compiled station entry has an unsupported field form")
            field_name = field_token[1]
            if field_name not in STATION_FIELD_SET:
                raise GateError("compiled station entry has an unknown field")
            if field_name in fields:
                raise GateError(f"compiled station entry has duplicate {field_name} fields")
            cursor += 2

            if field_name in string_fields:
                value_token = tokens[cursor] if cursor < len(tokens) else None
                if value_token is None or value_token[0] != "string" or value_token[2]:
                    raise GateError(f"compiled station field {field_name} must be a direct string literal")
                if cursor + 1 >= len(tokens) or tokens[cursor + 1][:2] not in (
                    ("punctuation", ","),
                    ("punctuation", "}"),
                ):
                    raise GateError(f"compiled station field {field_name} must be a direct string literal")
                field_value = value_token[1]
                cursor += 1
            elif field_name in STATION_BOOL_FIELDS:
                value_token = tokens[cursor] if cursor < len(tokens) else None
                if value_token is None or value_token[0] != "identifier" or value_token[1] not in ("true", "false"):
                    raise GateError(f"compiled station field {field_name} must be a boolean literal")
                field_value = value_token[1] == "true"
                if cursor + 1 >= len(tokens) or tokens[cursor + 1][:2] not in (
                    ("punctuation", ","),
                    ("punctuation", "}"),
                ):
                    raise GateError(f"compiled station field {field_name} must be a boolean literal")
                cursor += 1
            else:
                value_token = tokens[cursor] if cursor < len(tokens) else None
                if (
                    value_token is None
                    or value_token[0] != "number"
                    or not re.fullmatch(r"[0-9]+(?:_[0-9]+)*", value_token[1])
                ):
                    raise GateError("compiled station field nominal_kbps must be an integer literal")
                field_value = int(value_token[1].replace("_", ""))
                if field_value > 65_535:
                    raise GateError("compiled station field nominal_kbps exceeds u16")
                if cursor + 1 >= len(tokens) or tokens[cursor + 1][:2] not in (
                    ("punctuation", ","),
                    ("punctuation", "}"),
                ):
                    raise GateError("compiled station field nominal_kbps must be a direct integer literal")
                cursor += 1

            fields[field_name] = field_value

            if cursor >= len(tokens):
                raise GateError("compiled station entry is unterminated")
            if tokens[cursor][:2] == ("punctuation", ","):
                cursor += 1
                if cursor < len(tokens) and tokens[cursor][:2] == ("punctuation", "}"):
                    break  # Rust permits a trailing struct-field comma.
            elif tokens[cursor][:2] != ("punctuation", "}"):
                raise GateError("compiled station entry has an unsupported field separator")

        if cursor >= len(tokens) or tokens[cursor][:2] != ("punctuation", "}"):
            raise GateError("compiled station entry is unterminated")
        cursor += 1

        for field in STATION_FIELDS:
            if field not in fields:
                raise GateError(f"compiled station entry must have one {field}")
        station_id = fields["id"]
        if not isinstance(station_id, str):
            raise GateError("compiled station ID must be a string literal")
        if not ID_PATTERN.fullmatch(station_id):
            raise GateError("compiled station ID has an invalid format")
        if station_id in stations:
            raise GateError(f"duplicate compiled station ID: {station_id}")
        broadcaster = fields["broadcaster"]
        if not isinstance(broadcaster, str) or not broadcaster.strip():
            raise GateError(f"compiled station {station_id} has no publisher")
        stations[station_id] = fields

        if cursor < len(tokens) and tokens[cursor][:2] == ("punctuation", ","):
            cursor += 1
        elif cursor >= len(tokens) or tokens[cursor][:2] != ("punctuation", "]"):
            raise GateError("compiled station array has an unsupported entry separator")

    if cursor >= len(tokens) or not _matches(
        tokens, cursor, (("punctuation", "]"), ("punctuation", ";"))
    ):
        raise GateError("compiled station array is unterminated")
    if len(stations) != declared_count:
        raise GateError("compiled station declaration count does not match its entries")
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

    catalog_stations: dict[str, dict[str, str | int]] = {}
    for entry in catalog_entries:
        if not isinstance(entry, dict) or set(entry) != STATION_FIELD_SET:
            raise GateError("catalog station entry must be an object")
        station = {field: entry[field] for field in STATION_FIELDS}
        station_id = station["id"]
        broadcaster = station["broadcaster"]
        if not isinstance(station_id, str) or not ID_PATTERN.fullmatch(station_id):
            raise GateError("catalog station ID has an invalid format")
        if not isinstance(broadcaster, str) or not broadcaster.strip():
            raise GateError(f"catalog station {station_id} has no publisher")
        if any(not isinstance(station[field], str) for field in STATION_STRING_FIELDS):
            raise GateError(f"catalog station {station_id} has a non-string metadata field")
        if type(station["third_party_player_support"]) is not bool:
            raise GateError(f"catalog station {station_id} has invalid player-guidance metadata")
        if type(station["nominal_kbps"]) is not int or not 0 <= station["nominal_kbps"] <= 65_535:
            raise GateError(f"catalog station {station_id} has an invalid nominal bitrate")
        if station_id in catalog_stations:
            raise GateError(f"duplicate catalog station ID: {station_id}")
        catalog_stations[station_id] = station

    try:
        with runtime_path.open("rb") as stream:
            runtime_raw = stream.read(MAX_INPUT_BYTES + 1)
        runtime_source = runtime_raw.decode("utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        raise GateError("cannot read compiled station source") from exc
    if not runtime_raw or len(runtime_raw) > MAX_INPUT_BYTES:
        raise GateError("compiled station source is outside the allowed size")
    runtime_stations = _runtime_stations(runtime_source)
    if list(catalog_stations) != list(runtime_stations) or catalog_stations != runtime_stations:
        raise GateError("ordered catalog station metadata does not match compiled stations")

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
