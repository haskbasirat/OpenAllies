"""Synthetic packaging safety checks; no real release, game, profile or GUI run."""
from pathlib import Path
import argparse
import json
import struct
import tempfile
import unittest
from unittest import mock
import zipfile

import package


def executable():
    raw = bytearray(512)
    raw[:2] = b"MZ"
    struct.pack_into("<I", raw, 60, 64)
    raw[64:68] = b"PE\0\0"
    struct.pack_into("<HH", raw, 68, 0x8664, 0)
    struct.pack_into("<HH", raw, 84, 240, 0x20)
    struct.pack_into("<H", raw, 88, 0x20B)
    raw[450:473] = b"PerMonitorV2 asInvoker\0"
    return raw


class Packaging(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="synthetic-", dir=package.HERE)
        self.root = Path(self.temp.name).resolve()
        # Verify the sole recursive cleanup target before TemporaryDirectory
        # cleanup; it is an owned generated child of this research directory.
        assert self.root.is_relative_to(package.HERE.resolve())
        self.release = self.root / "release/Development-10"
        self.patches = [mock.patch.object(package, "ROOT", self.root), mock.patch.object(package, "RELEASE", self.release)]
        for patch in self.patches:
            patch.start()

    def tearDown(self):
        for patch in reversed(self.patches):
            patch.stop()
        assert self.root.is_relative_to(package.HERE.resolve())
        self.temp.cleanup()

    def fixture(self):
        for relative, data in {
            "candidate.exe": executable(), "Cargo.toml": b"synthetic workspace", "Cargo.lock": b"synthetic lock",
            "crates/example.rs": b"// synthetic source", "build.log": b"synthetic successful build",
            "README.txt": b"Synthetic packaging fixture, not a game", "CHECKPOINT.txt": b"Synthetic checkpoint",
            "assets-cache/model-materials.json": b"{}", "release/Development-08/keep.zip": b"keep08",
            "release/Development-09/keep.zip": b"keep09",
        }.items():
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        old = [package.record(self.root / f"release/Development-{v}/keep.zip") for v in ("08", "09")]
        build = {"passed": True, "executable_sha256": package.sha(self.root / "candidate.exe"),
                 "target": "x86_64-pc-windows-msvc", "profile": "release", "command": ["cargo", "build"],
                 "rustc_version": "synthetic", "source_files": package.source_rows(),
                 "evidence_files": [package.record(self.root / "build.log")]}
        package.write_new(self.root / "build-record.json", build)
        package.write_new(self.root / "payload.json", {"cache_files": [package.record(self.root / "assets-cache/model-materials.json")],
                                                     "previous_release_files": old, "external_assets": ["synthetic external data"]})
        package.write_new(self.root / "launchers.json", [{"name": "Viewer.cmd", "arguments": ["--viewer"]}])
        return argparse.Namespace(exe=str(self.root / "candidate.exe"), sha256=build["executable_sha256"],
                                  build_record=str(self.root / "build-record.json"), payload_plan=str(self.root / "payload.json"),
                                  readme="README.txt", checkpoint="CHECKPOINT.txt", launchers=str(self.root / "launchers.json"))

    def test_existing_dev10_is_never_overwritten(self):
        self.release.mkdir(parents=True)
        keep = self.release / "keep"
        keep.write_bytes(b"untouched")
        with self.assertRaisesRegex(RuntimeError, "already exists"):
            package.stage(argparse.Namespace())
        self.assertEqual(keep.read_bytes(), b"untouched")

    def test_paths_launchers_and_case_collisions(self):
        for path in ("../outside", "C:drive", "a\\b", "a//b", "CON", "trailing.", "a/../b"):
            with self.subTest(path=path), self.assertRaises(RuntimeError):
                package.relative(path)
        for value in ("%APPDATA%", "x&command", 'bad"quote', "line\nnext"):
            with self.assertRaises(RuntimeError):
                package.launcher([value])
        file = self.root / "file"
        file.write_bytes(b"x")
        row = package.record(file)
        self.assertEqual(package.record(file.relative_to(Path.cwd())), row)
        with self.assertRaisesRegex(RuntimeError, "case-colliding"):
            package.verify_rows(self.root, [row, {**row, "path": "FILE"}])

    def test_candidate_hash_and_architecture(self):
        file = self.root / "candidate.exe"
        file.write_bytes(executable())
        self.assertEqual(package.pe_info(file, package.sha(file))["architecture"], "AMD64 PE32+")
        with self.assertRaisesRegex(RuntimeError, "SHA-256"):
            package.pe_info(file, "0" * 64)
        raw = executable()
        struct.pack_into("<H", raw, 68, 0x14C)
        file.write_bytes(raw)
        with self.assertRaisesRegex(RuntimeError, "AMD64"):
            package.pe_info(file, package.sha(file))

    def test_changed_source_refuses_before_release_creation(self):
        args = self.fixture()
        (self.root / "crates/example.rs").write_bytes(b"changed")
        with self.assertRaisesRegex(RuntimeError, "Source differs"):
            package.stage(args)
        self.assertFalse(self.release.exists())

    def test_stage_copies_exact_payload_and_keeps_old_releases(self):
        args = self.fixture()
        package.stage(args)
        self.assertEqual(package.sha(self.release / package.EXE), args.sha256)
        self.assertFalse((self.release / "manifest.json").exists())
        self.assertEqual((self.root / "release/Development-08/keep.zip").read_bytes(), b"keep08")
        self.assertEqual((self.root / "release/Development-09/keep.zip").read_bytes(), b"keep09")

    def test_diagnostic_success_cannot_seal_as_a_battle(self):
        args = self.fixture()
        package.stage(args)
        path = self.root / "diagnostic.json"
        package.write_new(path, {"passed": True, "executable_sha256": args.sha256,
                                 "playable_battle": False, "full_game_parity": False})
        with self.assertRaisesRegex(RuntimeError, "native-battle acceptance"):
            package.seal(argparse.Namespace(validation=str(path)))
        self.assertFalse((self.release / "manifest.json").exists())
        self.assertFalse((self.release / package.ARCHIVE).exists())

    def test_seal_checks_zip_bytes_and_preservation(self):
        args = self.fixture()
        package.stage(args)
        evidence = self.root / "synthetic-acceptance.txt"
        evidence.write_text("Synthetic fixture only; no game acceptance")
        path = self.root / "synthetic-validation.json"
        package.write_new(path, {"passed": True, "executable_sha256": args.sha256,
            "playable_battle": True, "full_game_parity": False, "limitations": ["synthetic only"], "not_tested": ["game"],
            "evidence_files": [package.record(evidence)], "checks": [
                {"name": name, "status": "passed", "evidence_files": ["synthetic-acceptance.txt"]}
                for name in ("native-battle", "client-controls", "render-review", "package-cli", "independent-review")]})
        package.seal(argparse.Namespace(validation=str(path)))
        with zipfile.ZipFile(self.release / package.ARCHIVE, "a") as archive:
            archive.writestr("unapproved.txt", b"not approved")
        with self.assertRaisesRegex(RuntimeError, "ZIP payload"):
            package.verify_package()

    def diagnostic_fixture(self):
        args = self.fixture()
        args.release_mode = "diagnostic"
        for name in ("README.txt", "CHECKPOINT.txt"):
            (self.root / name).write_text("DIAGNOSTIC / UNFINISHED\nBATTLE STATUS: UNAVAILABLE\nSynthetic fixture only; unfinished battle startup UI labelled.\n")
        return args

    def diagnostic_validation(self, args, name="diagnostic-validation.json", **overrides):
        evidence = self.root / "synthetic-diagnostic-acceptance.txt"
        evidence.write_text("Synthetic fixture only; architecture/assets/CLI/render/labelled unfinished battle UI/independent evidence")
        validation = {"passed": True, "executable_sha256": args.sha256,
            "release_mode": "diagnostic", "battle_status": "unavailable", "playable_battle": False,
            "full_game_parity": False, "unfinished_battle_ui_labelled": True, "claims_native_battle": False,
            "limitations": ["synthetic only; battle unavailable"], "not_tested": ["gameplay"],
            "evidence_files": [package.record(evidence)], "checks": [
                {"name": check, "status": "passed", "evidence_files": [evidence.name]}
                for check in ("architecture-dependencies", "asset-verification", "package-cli", "render-review", "unfinished-battle-ui", "independent-review")]}
        validation.update(overrides)
        path = self.root / name
        package.write_new(path, validation)
        return argparse.Namespace(validation=str(path))

    def test_diagnostic_stage_requires_exact_warning_documents(self):
        args = self.fixture(); args.release_mode = "diagnostic"
        with self.assertRaisesRegex(RuntimeError, "explicitly label"):
            package.stage(args)
        self.assertFalse(self.release.exists())

    def test_diagnostic_stage_cannot_use_playable_seal(self):
        args = self.diagnostic_fixture(); package.stage(args)
        with self.assertRaisesRegex(RuntimeError, "release mode differs"):
            package.seal(self.diagnostic_validation(args))
        self.assertFalse((self.release / "manifest.json").exists())

    def test_diagnostic_seal_refuses_gameplay_claim_or_missing_ui_check(self):
        args = self.diagnostic_fixture(); package.stage(args)
        with self.assertRaisesRegex(RuntimeError, "explicitly refuse"):
            package.seal_diagnostic(self.diagnostic_validation(args, "false-claim.json", playable_battle=True))
        valid = self.diagnostic_validation(args, "no-ui-evidence.json")
        data = package.read(valid.validation)
        data["checks"] = [c for c in data["checks"] if c["name"] != "unfinished-battle-ui"]
        Path(valid.validation).write_text(json.dumps(data))
        with self.assertRaisesRegex(RuntimeError, "required release acceptance"):
            package.seal_diagnostic(valid)
        self.assertFalse((self.release / "manifest.json").exists())

    def test_diagnostic_seals_truthful_distinct_archive_and_preserves_old_releases(self):
        args = self.diagnostic_fixture(); package.stage(args)
        package.seal_diagnostic(self.diagnostic_validation(args))
        manifest = package.read(self.release / "manifest.json")
        self.assertEqual(manifest["release_mode"], "diagnostic")
        self.assertEqual(manifest["battle_status"], "unavailable")
        self.assertIs(manifest["playable_battle"], False)
        self.assertIs(manifest["unfinished_battle_ui_labelled"], True)
        self.assertTrue((self.release / package.DIAGNOSTIC_ARCHIVE).is_file())
        self.assertFalse((self.release / package.ARCHIVE).exists())
        self.assertEqual((self.root / "release/Development-08/keep.zip").read_bytes(), b"keep08")
        self.assertEqual((self.root / "release/Development-09/keep.zip").read_bytes(), b"keep09")


if __name__ == "__main__":
    unittest.main()
