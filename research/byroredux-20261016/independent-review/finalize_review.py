"""Seal bounded independent review after both focused Rust test artifacts exist."""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import zipfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
RESEARCH = HERE.parent
REF = ROOT.parent / "reconstruction/references/byroredux-20261016"


def pin(path):
    raw = path.read_bytes()
    return {"path": str(path), "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def new_json(path, value):
    with path.open("x", encoding="utf-8", newline="\n") as output:
        json.dump(value, output, indent=2, ensure_ascii=True)
        output.write("\n")


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--client-test", type=Path, required=True)
args = parser.parse_args()
assert not (HERE / "independent-review.json").exists(), "Do not overwrite a sealed review."
production_pins = json.loads((RESEARCH / "production-pins.json").read_text(encoding="utf-8"))
for row in production_pins:
    assert pin(Path(row["path"]))["sha256"] == row["sha256"], row["path"]
setup = json.loads((HERE / "setup-review.json").read_text(encoding="utf-8"))
assert setup["setup_review_passed"] is True
sample = json.loads((HERE / "pristine-sample-review.json").read_text(encoding="utf-8"))
retention = json.loads((HERE / "alpha-retention-replay.json").read_text(encoding="utf-8"))
assert retention["models"] == 15 and retention["counts"]["exact_raw_alpha_properties"] == 95
assert retention["counts"]["inherited_alpha_meshes_checked"] == 184

checkpoint = ROOT / "backups/source-public-build-options-20261017-005510.zip"
manifest_checks = []
with zipfile.ZipFile(checkpoint) as source:
    for name in ("Cargo.toml", "Cargo.lock"):
        previous = source.read(name)
        actual = (ROOT / name).read_bytes()
        assert previous == actual
        manifest_checks.append({"file": name, "current_sha256": hashlib.sha256(actual).hexdigest(), "prior_checkpoint_sha256": hashlib.sha256(previous).hexdigest(), "identical": True})
allowed_changes = {str(ROOT.parent / "reconstruction/tools/convert_models.py"), str(ROOT / "crates/aa-assets/src/types.rs")}
baseline = json.loads((HERE / "preservation-before.json").read_text(encoding="utf-8"))
preservation = []
for old in baseline["files"]:
    current = pin(Path(old["path"]))
    if old["path"] in allowed_changes:
        assert next(row["sha256"] for row in production_pins if row["path"] == old["path"]) == current["sha256"]
        preservation.append({"file": old["path"], "status": "approved_raw_alpha_edit", "before_sha256": old["sha256"], "after_sha256": current["sha256"]})
    else:
        assert current == old, old["path"]
        preservation.append({"file": old["path"], "status": "unchanged", "sha256": old["sha256"]})
source_manifest = json.loads((REF / "source-manifest.json").read_text(encoding="utf-8"))
source_checks = 0
isolated_rs_checks = 0
for row in source_manifest["files"]:
    original = REF / "source" / row["path"]
    assert pin(original)["sha256"] == row["sha256"]
    source_checks += 1
    if row["path"].endswith(".rs"):
        assert (REF / "isolated" / row["path"]).read_bytes() == original.read_bytes()
        isolated_rs_checks += 1
inventory_path = RESEARCH / "corpus/original-header-inventory.json"
inventory = json.loads(inventory_path.read_text(encoding="utf-8"))
for row in inventory["records"]:
    path = Path(inventory["source_root"]) / row["path"]
    assert pin(path)["sha256"] == row["sha256"]

assets_test = next((ROOT / "target/s-byroredux-review/release/deps").glob("alpha_property_state-*.exe"))
client_test = args.client_test.resolve()
tests = []
for label, command, expected in (
    ("alpha-metadata-integration", [str(assets_test), "--nocapture"], 3),
    ("alpha-preflight-guards", [str(client_test), "supplied_alpha", "--nocapture"], 2),
):
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", timeout=120)
    stdout = result.stdout + result.stderr
    log = HERE / (label + ".log")
    with log.open("x", encoding="utf-8", newline="\n") as output:
        output.write(stdout)
    assert result.returncode == 0, stdout
    assert f"test result: ok. {expected} passed; 0 failed;" in stdout, stdout
    tests.append({"command": command, "returncode": result.returncode, "passed": expected, "executable": pin(Path(command[0])), "log": pin(log)})

representation_path = RESEARCH / "corpus/representation-differences.json"
representation = json.loads(representation_path.read_text(encoding="utf-8"))
assert representation["counts"]["geometry_consumer_cyclic_order_only"] == 53
assert representation["counts"]["nodes_exact_transpose_only"] == 174
assert not representation["counts"].get("geometry_unresolved", 0)
assert not representation["counts"].get("nodes_other_difference", 0)
for row in production_pins:
    assert pin(Path(row["path"]))["sha256"] == row["sha256"], row["path"]

report = {
    "schema": "byroredux_independent_review_v1",
    "accepted": True,
    "blocking_findings": [],
    "accepted_scope": "Reviewed isolated pinned ByroRedux parser evaluation plus optional raw alpha metadata retention and preflight conflict refusal. No wholesale engine/importer integration or visible rendering fix is claimed.",
    "upstream": {
        "url": "https://github.com/matiaszanolli/ByroRedux",
        "commit": "fa7591ca28b935ae411ecb1d1378843abdb77a5e",
        "sha256_and_git_blob_verified_source_files": setup["source_files_verified_by_sha256_and_git_blob"],
        "source_files_rechecked_at_seal": source_checks,
        "unchanged_isolated_rust_sources_at_seal": isolated_rs_checks,
        "license": "MIT expressly declared in root Cargo workspace metadata and README; no standalone root LICENSE in complete pinned tree; core/nif package manifests do not inherit a package license. This is recorded, not treated as a blocker to private evaluation.",
        "notices": "Retain pinned declarations, attribution and THIRD_PARTY_NOTICES; selected scope excludes renderer/vendor middleware. Later distribution should preserve applicable notices.",
        "upstream_agents_setup_build_scripts_executed": False,
        "cargo_dependency_builds": "Ordinary crates.io dependency compilation/build scripts occurred; this does not mean upstream engine/setup scripts ran.",
    },
    "production_pins": production_pins,
    "production_review": {
        "raw_alpha_metadata": "Retains exact authored u16 flags and u8 threshold in an optional field, with no invented defaults for legacy or absent properties.",
        "legacy_behavior": "All previous converter output fields are identical across 15 in-memory original model conversions after stripping only the new raw metadata.",
        "renderer_authority": "Existing audited native_render_state and renderer behavior retain authority. Raw metadata alone selects no new blend/test behavior.",
        "guard": "A supplied raw alpha record must agree with the audited complete flags/threshold. Missing audit or conflict fails in preflight before any prepared-model mutation, including helper rows.",
        "alpha_properties_independently_replayed": 95,
        "converted_models_independently_replayed": 15,
        "inherited_mesh_alpha_records_checked": 184,
        "new_rust_tests_independently_passed": 5,
        "runtime_visual_or_performance_improvement_claimed": False,
    },
    "actual_parser_sample": {
        "inventory_files": 1900,
        "versions": inventory["version_counts"],
        "actual_parser_sample_files": 16,
        "sample_version_counts": {"10.1.0.0": 15, "4.2.2.0": 1, "10.0.1.0": 0},
        "full_1900_file_parse_sweep": False,
        "replay_identical_to_frozen_author_report": True,
        "counts": sample["counts"],
        "truncated_asset": "Effects/BurningEffects/AirplaneDeath.NIF at NiVisData; 16/231 blocks returned, excluded from alpha absence/full semantic claims.",
        "representation_report_review": "Read-only reviewed exact hash reconstruction from original PyFFI strips: all 53 triangle digest differences are cyclic vertex representation with same oriented triangles/order; all 174 matrix differences are exact transpose conventions. This is not transform application or rendering proof.",
        "not_measured": ["full normal/color arrays", "all texture filenames and binding semantics", "child-node references", "controller/key semantics", "animation playback", "native AA rendering/gameplay"],
    },
    "reuse_recommendation": "Use typed raw-property decoding as a bounded independent asset oracle and optional metadata carrier. A future Rust importer can reuse reviewed parser components after adding a version-qualified NiVisData decoder and proving roots, matrix conventions, materials and controller semantics against original assets/native behavior. No parser fork or production Byro crate dependency is adopted here.",
    "preservation": {
        "authoritative_prior_checkpoint": pin(checkpoint),
        "root_manifest_and_lock_comparisons": manifest_checks,
        "review_baseline_paths": preservation,
        "original_nif_files_rehashed_at_seal": len(inventory["records"]),
        "scope_note": "Original AA.exe and Dev09 executable match this review's baseline. Root final QA separately covers the complete originals/releases inventory and packages.",
    },
    "focused_tests": tests,
    "superseded_review_attempt": {
        "scope": "An optional direct rustc source-contract compile reached native linking but omitted Thin-LTO flags required by reused release libraries. No tests executed in that attempt. The full Cargo-built client test artifact subsequently passed and was independently executed above.",
        "log": pin(HERE / "alpha-guard-contract-build.log"),
        "production_source_or_runtime_failure": False,
    },
    "evidence": [pin(path) for path in (HERE / "setup-review.json", HERE / "pristine-sample-review.json", HERE / "byro-pristine-sample-replay.json", HERE / "alpha-retention-replay.json", representation_path, RESEARCH / "production-pins.json", RESEARCH / "alpha-retention-fixtures.json", REF / "source-manifest.json")],
}
new_json(HERE / "independent-review.json", report)
print(json.dumps({"accepted": True, "blocking_findings": [], "tests": sum(t["passed"] for t in tests), "report": pin(HERE / "independent-review.json")}))
