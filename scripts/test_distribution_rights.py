#!/usr/bin/env python3
"""Offline regression coverage for the macOS distribution rights preflight."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any

SCRIPTS = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS))

import check_distribution_rights as gate  # noqa: E402


class DistributionRightsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="rust-radio-rights-")
        self.root = Path(self.temp.name) / "repo"
        self.catalog_path = self.root / "catalog/curated-stations.json"
        self.runtime_path = self.root / "crates/rust-radio-core/src/directory.rs"
        self.root.mkdir()
        self.catalog_path.parent.mkdir(parents=True)
        self.runtime_path.parent.mkdir(parents=True)
        self.write_runtime([("alpha", "Alpha Radio"), ("beta", "Beta Radio")])
        self.write_catalog([("alpha", "Alpha Radio"), ("beta", "Beta Radio")])
        self.manifest_path = Path(self.temp.name) / "approvals.json"
        self.write_manifest(self.approvals())

    def tearDown(self) -> None:
        self.temp.cleanup()

    def station_record(self, station_id: str, publisher: str) -> dict[str, Any]:
        return {
            "id": station_id,
            "name": station_id.title(),
            "broadcaster": publisher,
            "genre": "Ambient",
            "language": "en",
            "country": "US",
            "category": "CHILL",
            "playlist_url": f"https://radio.example.test/{station_id}",
            "publisher_reference_url": f"https://publisher.example.test/{station_id}",
            "nominal_kbps": 128,
            "format": "MP3",
        }

    def write_runtime(
        self,
        stations: list[tuple[str, str]],
        runtime_overrides: dict[str, dict[str, Any]] | None = None,
    ) -> None:
        runtime_overrides = runtime_overrides or {}
        entries = []
        for station_id, publisher in stations:
            station = self.station_record(station_id, publisher)
            station.update(runtime_overrides.get(station_id, {}))
            fields = []
            for field in gate.STATION_FIELDS:
                value = station[field]
                if field == "nominal_kbps":
                    fields.append(f"        {field}: {value},")
                else:
                    fields.append(f'        {field}: "{value}",')
            entries.append("    InternetStation {\n" + "\n".join(fields) + "\n    }")
        body = ",\n".join(entries)
        self.runtime_path.write_text(
            f"pub const CURATED_STATIONS: [InternetStation; {len(stations)}] = [\n{body}\n];\n",
            encoding="utf-8",
        )

    def write_catalog(
        self, stations: list[tuple[str, str]], redistribution: Any = "authorized", include_policy: bool = True
    ) -> None:
        catalog: dict[str, Any] = {
            "schema_version": 1,
            "stations": [self.station_record(station_id, publisher) for station_id, publisher in stations],
        }
        if include_policy:
            catalog["policy"] = {"redistribution": redistribution}
        self.catalog_path.write_text(json.dumps(catalog, sort_keys=True, indent=2) + "\n", encoding="utf-8")

    def approvals(self) -> list[dict[str, str]]:
        return [
            {
                "station_id": station_id,
                "publisher": publisher,
                "scope": "app_distribution",
                "evidence_ref": f"https://rights.example.test/{station_id}",
            }
            for station_id, publisher in (("alpha", "Alpha Radio"), ("beta", "Beta Radio"))
        ]

    def manifest(self, approvals: list[dict[str, str]]) -> dict[str, Any]:
        return {
            "schema_version": 1,
            "catalog_sha256": hashlib.sha256(self.catalog_path.read_bytes()).hexdigest(),
            "runtime_source_sha256": hashlib.sha256(self.runtime_path.read_bytes()).hexdigest(),
            "approvals": approvals,
        }

    def write_manifest(self, approvals: list[dict[str, str]]) -> None:
        self.manifest_path.write_text(json.dumps(self.manifest(approvals), sort_keys=True) + "\n", encoding="utf-8")

    def assert_blocked(self) -> None:
        with self.assertRaises(gate.GateError):
            gate.validate(self.root, self.manifest_path)

    def test_complete_current_approval_fixture_passes(self) -> None:
        gate.validate(self.root, self.manifest_path)

    def test_missing_unknown_and_denied_redistribution_policy_fail(self) -> None:
        for value, include in ((None, False), ("pending", True), ("not_authorized", True)):
            with self.subTest(policy=value):
                self.write_catalog(
                    [("alpha", "Alpha Radio"), ("beta", "Beta Radio")],
                    redistribution=value,
                    include_policy=include,
                )
                self.write_manifest(self.approvals())
                self.assert_blocked()

    def test_missing_manifest_fails(self) -> None:
        with self.assertRaises(gate.GateError):
            gate.validate(self.root, self.manifest_path.with_name("absent.json"))

    def test_partial_duplicate_and_extra_approval_sets_fail(self) -> None:
        cases = (
            self.approvals()[:1],
            self.approvals() + [self.approvals()[0]],
            self.approvals() + [{**self.approvals()[0], "station_id": "extra"}],
        )
        for approvals in cases:
            with self.subTest(approvals=approvals):
                self.write_manifest(approvals)
                self.assert_blocked()

    def test_malformed_approval_and_invalid_evidence_reference_fail(self) -> None:
        malformed = self.approvals()
        malformed[0]["unreviewed"] = "extra"
        self.write_manifest(malformed)
        self.assert_blocked()
        malformed = self.approvals()
        malformed[0]["evidence_ref"] = "https://rights.example.test/alpha?token=secret"
        self.write_manifest(malformed)
        self.assert_blocked()

    def test_publisher_scope_and_current_source_digests_are_enforced(self) -> None:
        approvals = self.approvals()
        approvals[0]["publisher"] = "Different Publisher"
        self.write_manifest(approvals)
        self.assert_blocked()
        approvals = self.approvals()
        approvals[0]["scope"] = "personal_listening"
        self.write_manifest(approvals)
        self.assert_blocked()
        self.write_manifest(self.approvals())
        self.runtime_path.write_text(self.runtime_path.read_text(encoding="utf-8") + "// changed\n", encoding="utf-8")
        self.assert_blocked()

    def test_catalog_and_compiled_station_mirror_must_match(self) -> None:
        self.write_catalog([("alpha", "Alpha Radio"), ("different", "Beta Radio")])
        self.write_manifest(self.approvals())
        self.assert_blocked()

    def test_catalog_station_order_must_match_runtime_order(self) -> None:
        self.write_catalog([("beta", "Beta Radio"), ("alpha", "Alpha Radio")])
        self.write_manifest(self.approvals())
        self.assert_blocked()

    def test_commented_station_values_cannot_mask_runtime_constants(self) -> None:
        self.runtime_path.write_text(
            'const ACTUAL_ALPHA: &str = "not-in-catalog";\n'
            'const ACTUAL_PUBLISHER: &str = "Unapproved Publisher";\n'
            "pub const CURATED_STATIONS: [InternetStation; 2] = [\n"
            "    InternetStation {\n"
            '        // id: "alpha",\n'
            "        id: ACTUAL_ALPHA,\n"
            '        /* broadcaster: "Alpha Radio" */\n'
            "        broadcaster: ACTUAL_PUBLISHER,\n"
            "    },\n"
            "    InternetStation {\n"
            '        id: "beta",\n'
            '        broadcaster: "Beta Radio",\n'
            "    },\n"
            "];\n",
            encoding="utf-8",
        )
        self.write_manifest(self.approvals())
        with self.assertRaises(gate.GateError):
            gate._runtime_stations(self.runtime_path.read_text(encoding="utf-8"))

    def test_raw_string_contents_cannot_satisfy_runtime_metadata_fields(self) -> None:
        self.runtime_path.write_text(
            'const ACTUAL_PUBLISHER: &str = "Unapproved Publisher";\n'
            "pub const CURATED_STATIONS: [InternetStation; 2] = [\n"
            "    InternetStation {\n"
            '        id: "alpha",\n'
            '        name: r#"broadcaster: "Alpha Radio""#,\n'
            "        broadcaster: ACTUAL_PUBLISHER,\n"
            '        genre: "Ambient",\n'
            '        language: "en",\n'
            '        country: "US",\n'
            '        category: "CHILL",\n'
            '        playlist_url: "https://radio.example.test/a",\n'
            '        publisher_reference_url: "https://publisher.example.test/a",\n'
            "        nominal_kbps: 128,\n"
            '        format: "MP3",\n'
            "    },\n"
            "    InternetStation {\n"
            '        id: "beta",\n'
            '        name: "Beta",\n'
            '        broadcaster: "Beta Radio",\n'
            '        genre: "Classical",\n'
            '        language: "en",\n'
            '        country: "US",\n'
            '        category: "CLASSICAL",\n'
            '        playlist_url: "https://radio.example.test/b",\n'
            '        publisher_reference_url: "https://publisher.example.test/b",\n'
            "        nominal_kbps: 128,\n"
            '        format: "AAC",\n'
            "    },\n"
            "];\n",
            encoding="utf-8",
        )
        self.write_manifest(self.approvals())
        with self.assertRaises(gate.GateError):
            gate._runtime_stations(self.runtime_path.read_text(encoding="utf-8"))

    def test_runtime_metadata_requires_exact_fields_and_literal_values(self) -> None:
        valid_source = self.runtime_path.read_text(encoding="utf-8")
        malformed_sources = (
            valid_source.replace('        name: "Alpha",\n', "", 1),
            valid_source.replace(
                "    InternetStation {\n",
                '    InternetStation {\n        unexpected: "field",\n',
                1,
            ),
            valid_source.replace(
                '        name: "Alpha",\n',
                '        name: "Alpha",\n        name: "Shadow",\n',
                1,
            ),
            valid_source.replace('        genre: "Ambient",\n', "        genre: GENRE,\n", 1),
            valid_source.replace("        nominal_kbps: 128,\n", "        nominal_kbps: RATE,\n", 1),
        )
        for malformed in malformed_sources:
            with self.subTest(source=malformed):
                self.runtime_path.write_text(malformed, encoding="utf-8")
                self.write_manifest(self.approvals())
                self.assert_blocked()

    def test_runtime_playlist_url_must_match_even_with_fresh_source_digest(self) -> None:
        self.write_runtime(
            [("alpha", "Alpha Radio"), ("beta", "Beta Radio")],
            runtime_overrides={"alpha": {"playlist_url": "https://radio.example.test/changed"}},
        )
        self.write_manifest(self.approvals())
        manifest = self.manifest(self.approvals())
        self.assertEqual(
            manifest["runtime_source_sha256"],
            hashlib.sha256(self.runtime_path.read_bytes()).hexdigest(),
        )
        self.assert_blocked()

    def test_repository_catalog_matches_ordered_compiled_station_metadata(self) -> None:
        project_root = SCRIPTS.parent
        catalog = json.loads((project_root / "catalog/curated-stations.json").read_text(encoding="utf-8"))
        runtime = (project_root / "crates/rust-radio-core/src/directory.rs").read_text(encoding="utf-8")
        catalog_stations = [
            {field: item[field] for field in gate.STATION_FIELDS}
            for item in catalog["stations"]
        ]
        runtime_stations = gate._runtime_stations(runtime)
        self.assertEqual(len(runtime_stations), 25)
        self.assertEqual(catalog_stations, list(runtime_stations.values()))

    def test_duplicate_json_keys_fail_closed(self) -> None:
        self.manifest_path.write_text(
            '{"schema_version":1,"schema_version":1,"catalog_sha256":"x",'
            '"runtime_source_sha256":"y","approvals":[]}\n',
            encoding="utf-8",
        )
        self.assert_blocked()

    def test_rejected_packaging_does_not_run_cargo_or_mutate_target(self) -> None:
        package_script = SCRIPTS / "package_macos_app.sh"
        with tempfile.TemporaryDirectory(prefix="rust-radio-package-smoke-") as temp_dir:
            temp = Path(temp_dir)
            fake_bin = temp / "bin"
            fake_bin.mkdir()
            cargo_marker = temp / "cargo-invoked"
            fake_cargo = fake_bin / "cargo"
            fake_cargo.write_text(f"#!/bin/sh\nprintf invoked > {cargo_marker}\n", encoding="utf-8")
            fake_cargo.chmod(0o755)
            env = os.environ.copy()
            env["PATH"] = f"{fake_bin}{os.pathsep}{env.get('PATH', '')}"

            missing_target = temp / "missing-target"
            env["CARGO_TARGET_DIR"] = str(missing_target)
            rejected = subprocess.run(
                ["bash", str(package_script), "distribution", str(temp / "unused-approvals.json")],
                env=env,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertNotEqual(rejected.returncode, 0)
            self.assertFalse(missing_target.exists())
            self.assertFalse(cargo_marker.exists())

            existing_target = temp / "existing-target"
            app_payload = existing_target / "release/Rust Radio.app/Contents/app-payload"
            app_payload.parent.mkdir(parents=True)
            app_payload.write_bytes(b"existing-app-must-remain-byte-identical")
            before = app_payload.read_bytes()
            env["CARGO_TARGET_DIR"] = str(existing_target)
            rejected = subprocess.run(
                ["bash", str(package_script), "distribution", str(temp / "unused-approvals.json")],
                env=env,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertNotEqual(rejected.returncode, 0)
            self.assertEqual(app_payload.read_bytes(), before)
            self.assertFalse(cargo_marker.exists())


if __name__ == "__main__":
    unittest.main()
