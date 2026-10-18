"""HF02 packaging regressions in isolated synthetic trees; no game/build launch."""
from contextlib import redirect_stderr, redirect_stdout
from datetime import datetime, timezone
from pathlib import Path
import argparse
import inspect
import json
import tempfile
import unittest
from unittest import mock

import package_hotfix01 as predecessor
import package_hotfix02 as package
import test_package as base


WORKSPACE = package.ROOT.resolve()
OUTPUT = WORKSPACE / "qa-output/development10/hotfix02-package-tests"
PROTECTED = ("Development-08", "Development-09", "Development-10", "Development-10-Hotfix-01")
base.package = package


class Hotfix02Packaging(base.Packaging):
    def setUp(self):
        OUTPUT.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix="synthetic-hf02-", dir=OUTPUT)
        self.root = Path(self.temp.name).resolve()
        assert self.root.is_relative_to(OUTPUT.resolve())
        self.release = self.root / "release/Development-10-Hotfix-02"
        self.patches = [mock.patch.object(package, "ROOT", self.root),
                        mock.patch.object(package, "HERE", self.root / "research/development10"),
                        mock.patch.object(package, "RELEASE", self.release)]
        for patch in self.patches:
            patch.start()

    def tearDown(self):
        for patch in reversed(self.patches):
            patch.stop()
        # The only recursive cleanup target is our verified generated QA child.
        assert self.root.is_relative_to(OUTPUT.resolve())
        self.temp.cleanup()

    def fixture(self):
        args = super().fixture()
        for version in PROTECTED:
            folder = self.root / "release" / version
            folder.mkdir(parents=True, exist_ok=True)
            keep = folder / "keep.zip"
            if not keep.exists():
                keep.write_bytes(("synthetic sealed " + version).encode("ascii"))
            package.write_new(folder / "manifest.json", {"files": [package.record(keep, folder)]})
        payload = package.read(args.payload_plan)
        payload["previous_release_files"] = package.protected_rows()
        Path(args.payload_plan).write_text(json.dumps(payload), encoding="utf-8")
        return args

    def test_hf01_protected_rows_include_manifest_and_archive(self):
        self.fixture()
        rows = package.protected_rows()
        self.assertEqual({row["path"].split("/")[1] for row in rows}, set(PROTECTED))
        self.assertEqual(len(rows), 8)
        for version in PROTECTED:
            self.assertEqual({row["path"] for row in rows if row["path"].startswith(f"release/{version}/")},
                             {f"release/{version}/keep.zip", f"release/{version}/manifest.json"})

    def test_each_protected_release_change_refuses_before_creation(self):
        args = self.fixture()
        for version in PROTECTED:
            with self.subTest(version=version):
                keep = self.root / "release" / version / "keep.zip"
                original = keep.read_bytes()
                try:
                    keep.write_bytes(original + b"changed")
                    with self.assertRaisesRegex(RuntimeError, "Changed file"):
                        package.stage(args)
                    self.assertFalse(self.release.exists())
                finally:
                    keep.write_bytes(original)

    def test_diagnostic_preserves_all_four_previous_releases(self):
        args = self.diagnostic_fixture()
        rows = package.protected_rows()
        package.stage(args)
        package.seal_diagnostic(self.diagnostic_validation(args))
        package.verify_rows(self.root, rows)
        manifest = package.read(self.release / "manifest.json")
        self.assertEqual(manifest["archive"], "AxisAlliesRust-Development-10-Hotfix-02-DIAGNOSTIC.zip")
        self.assertIn("Development-10-Hotfix-02 DIAGNOSTIC / UNFINISHED", manifest["status"])
        self.assertIs(manifest["playable_battle"], False)
        self.assertEqual(manifest["battle_status"], "unavailable")

    def test_diagnostic_refuses_native_or_parity_claims(self):
        args = self.diagnostic_fixture()
        package.stage(args)
        for key in ("claims_native_battle", "full_game_parity"):
            with self.subTest(key=key):
                validation = self.diagnostic_validation(args, f"false-{key}.json", **{key: True})
                with self.assertRaisesRegex(RuntimeError, "explicitly refuse"):
                    package.seal_diagnostic(validation)
                self.assertFalse((self.release / "manifest.json").exists())

    def test_cache_closure_and_entire_helper_delta_are_exact(self):
        self.assertEqual(inspect.getsource(package.cache_closure), inspect.getsource(predecessor.cache_closure))
        original = Path(predecessor.__file__).read_bytes()
        expected = original.replace(b"Development-10-Hotfix-01", b"Development-10-Hotfix-02")
        old = b'for version in ("Development-08", "Development-09", "Development-10"):'
        new = b'for version in ("Development-08", "Development-09", "Development-10", "Development-10-Hotfix-01"):'
        self.assertEqual(expected.count(old), 1)
        expected = expected.replace(old, new)
        self.assertEqual(Path(package.__file__).read_bytes(), expected)


def main():
    global OUTPUT
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    # Synthetic fixtures and logs must stay in the dedicated QA directory.
    destination = args.output.resolve()
    expected = WORKSPACE / "qa-output/development10/hotfix02-package-tests"
    if destination != expected.resolve():
        parser.error("output must be the dedicated HF02 QA directory")
    OUTPUT = destination
    OUTPUT.mkdir(parents=True, exist_ok=True)
    pins = [package.record(Path(module.__file__), WORKSPACE) for module in (predecessor, package, base)]
    pins.append(package.record(Path(__file__), WORKSPACE))
    with (OUTPUT / "synthetic-tests.log").open("x", encoding="utf-8") as log:
        with redirect_stdout(log), redirect_stderr(log):
            result = unittest.TextTestRunner(stream=log, verbosity=2).run(
                unittest.defaultTestLoader.loadTestsFromTestCase(Hotfix02Packaging))
    package.verify_rows(WORKSPACE, pins)
    report = {"schema": 1, "utc": datetime.now(timezone.utc).isoformat(),
              "passed": result.wasSuccessful(), "tests_run": result.testsRun,
              "failures": len(result.failures), "errors": len(result.errors),
              "scope": "Isolated synthetic package safety only. No real release staged or sealed; no client/build/native process launched; no gameplay acceptance.",
              "protected_versions": list(PROTECTED), "pins": pins,
              "log": package.record(OUTPUT / "synthetic-tests.log", WORKSPACE)}
    package.write_new(OUTPUT / "synthetic-tests.json", report)
    print(json.dumps({"passed": report["passed"], "tests_run": result.testsRun,
                      "report": str(OUTPUT / "synthetic-tests.json")}))
    raise SystemExit(0 if result.wasSuccessful() else 1)


if __name__ == "__main__":
    main()
