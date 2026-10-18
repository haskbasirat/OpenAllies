"""Independent read-only provenance, isolation and preservation review.

Does not import or run downloaded upstream source. Writes this directory only.
"""
from pathlib import Path
from collections import Counter
import datetime
import hashlib
import json
import tomllib

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
REF = ROOT.parent / "reconstruction/references/byroredux-20261016"
SOURCE = REF / "source"
ISOLATED = REF / "isolated"


def hash_file(path):
    raw = path.read_bytes()
    return {"path": str(path), "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def new_json(path, value):
    with path.open("x", encoding="utf-8", newline="\n") as out:
        json.dump(value, out, indent=2, ensure_ascii=True)
        out.write("\n")


manifest = json.loads((REF / "source-manifest.json").read_text(encoding="utf-8"))
meta = json.loads((HERE.parent / "github-metadata.json").read_text(encoding="utf-8"))
assert manifest["commit"] == meta["commit"]["sha"] == "fa7591ca28b935ae411ecb1d1378843abdb77a5e"
assert meta["tree"]["truncated"] is False
assert manifest["count"] == len(manifest["files"])
assert len({row["path"] for row in manifest["files"]}) == len(manifest["files"])
tree = {row["path"]: row for row in meta["tree"]["tree"] if row["type"] == "blob"}
for row in manifest["files"]:
    path = SOURCE / row["path"]
    raw = path.read_bytes()
    assert len(raw) == row["bytes"] == tree[row["path"]]["size"], path
    assert hashlib.sha256(raw).hexdigest() == row["sha256"], path
    assert hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest() == row["git_blob"] == tree[row["path"]]["sha"], path
actual = {path.relative_to(SOURCE).as_posix() for path in SOURCE.rglob("*") if path.is_file()}
assert actual == {row["path"] for row in manifest["files"]}
selected_scripts = [name for name in actual if name.startswith(".agents/") or name.rsplit("/", 1)[-1] in {"AGENTS.md", "build.rs"} or name.startswith("scripts/")]
assert selected_scripts == []
root_toml = tomllib.loads((SOURCE / "Cargo.toml").read_text(encoding="utf-8"))
iso_toml = tomllib.loads((ISOLATED / "Cargo.toml").read_text(encoding="utf-8"))
assert root_toml["workspace"]["package"]["license"] == "MIT"
readme = (SOURCE / "README.md").read_text(encoding="utf-8")
assert "## License\n\nMIT." in readme
root_license_files = [name for name in tree if "/" not in name and name.lower().startswith(("license", "licence", "copying"))]
assert root_license_files == []
assert iso_toml["workspace"]["members"] == ["crates/core", "crates/nif"]
assert not (ISOLATED / "rust-toolchain.toml").exists()
assert not (ISOLATED / ".cargo").exists()
rust_files = sorted(p for p in ISOLATED.rglob("*.rs") if "target" not in p.relative_to(ISOLATED).parts)
same = []
changed = []
for path in rust_files:
    relative = path.relative_to(ISOLATED).as_posix()
    original = SOURCE / relative
    if original.is_file() and original.read_bytes() == path.read_bytes():
        same.append(relative)
    else:
        changed.append(relative)

preserve_paths = [ROOT / "Cargo.toml", ROOT / "Cargo.lock", ROOT.parent / "AA.exe",
                  ROOT.parent / "reconstruction/tools/convert_models.py",
                  ROOT / "release/Development-09/AxisAlliesRust.exe"]
preserve_paths += sorted(ROOT.glob("crates/*/Cargo.toml"))
preserve_paths += sorted((ROOT / "crates/aa-assets/src").rglob("*.rs"))
preserve_paths += sorted((ROOT / "crates/aa-formats/src").rglob("*.rs"))
preservation = [hash_file(p) for p in preserve_paths if p.is_file()]
before = HERE / "preservation-before.json"
if not before.exists():
    new_json(before, {"scope": "Review-time baseline; not proof of state before the other agent began setup.", "files": preservation})
else:
    old = json.loads(before.read_text(encoding="utf-8"))["files"]
    assert old == preservation, "Review-time protected paths changed; inspect before claiming preservation."

inventory = json.loads((HERE.parent / "corpus/original-header-inventory.json").read_text(encoding="utf-8"))
versions = Counter()
for row in inventory["records"]:
    p = Path(inventory["source_root"]) / row["path"]
    raw = p.read_bytes()
    assert len(raw) == row["bytes"] and hashlib.sha256(raw).hexdigest() == row["sha256"], p
    versions[row["binary_version"]] += 1
assert len(inventory["records"]) == inventory["count"] == 1900
assert {f"0x{k:08x}": v for k, v in versions.items()} == inventory["version_counts"]

report = {
    "schema": "byroredux_independent_setup_review_v1",
    "reviewed_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "upstream_pin": manifest["commit"],
    "source_files_verified_by_sha256_and_git_blob": len(actual),
    "github_complete_tree_entries": len(meta["tree"]["tree"]),
    "github_tree_truncated": False,
    "source_manifest": hash_file(REF / "source-manifest.json"),
    "upstream_metadata": hash_file(HERE.parent / "github-metadata.json"),
    "upstream_root_cargo_workspace_license": "MIT",
    "readme_license_declaration": "MIT",
    "root_license_files_in_complete_tree": root_license_files,
    "license_comment": "MIT is expressly declared in Cargo workspace metadata and README. Missing standalone LICENSE is recorded, not a blocker to private isolated parser evaluation; retain upstream declarations and attribution and review notices for later distribution.",
    "crate_package_license_inheritance": {name: tomllib.loads((SOURCE / f"crates/{name}/Cargo.toml").read_text(encoding="utf-8"))["package"].get("license") for name in ("core", "nif")},
    "isolation": {
        "members": iso_toml["workspace"]["members"],
        "same_rust_sources": len(same),
        "changed_or_added_rust_sources_at_review": changed,
        "downloaded_upstream_agent_setup_build_script_paths": selected_scripts,
        "toolchain_file_copied_to_isolated": False,
        "upstream_programs_executed_by_this_review": False,
        "dependency_build_scripts_note": "Cargo may execute ordinary crates.io dependency build scripts. No engine/renderer/upstream setup build script is selected here; do not describe dependency compilation as no code execution at all.",
    },
    "original_corpus_independently_rehashed": {
        "count": inventory["count"],
        "versions": {f"0x{k:08x}": v for k, v in sorted(versions.items())},
        "inventory": hash_file(HERE.parent / "corpus/original-header-inventory.json"),
    },
    "protected_review_baseline_files": len(preservation),
    "no_root_workspace_updates_observed_by_review": True,
    "setup_review_passed": not changed,
    "parser_coverage_or_semantic_parity_not_claimed_by_setup": True,
}
new_json(HERE / "setup-review.json", report)
print(json.dumps({"source_files": len(actual), "unchanged_isolated_rs": len(same), "changed_isolated_rs": changed, "corpus_count": inventory["count"], "versions": report["original_corpus_independently_rehashed"]["versions"], "report_sha256": hash_file(HERE / "setup-review.json")["sha256"]}))
