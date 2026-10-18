"""Pin the frozen saved Organization combat adapter and its scoped evidence."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent


def pin(relative):
    raw = (ROOT / relative).read_bytes()
    return {"path": relative, "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def main():
    native = json.loads((HERE / "native-saved-org-combat.json").read_text())
    inputs = json.loads((HERE / "actual-saved-inputs.json").read_text())
    actual = [row for row in native["cases"] if row["input"]["name"].startswith("kursk_")]
    probes = [row for row in native["cases"] if row["input"]["name"].startswith("probe_")]
    assert len(inputs) == 33 and len(actual) == 36 and len(probes) == 30
    assert all(m is None or m["performer"] is None for row in actual for m in row["input"]["members"])
    test_log = (HERE / "saved-adapter-tests.log").read_text(encoding="utf-8-sig")
    assert "1 passed; 0 failed" in test_log
    own = [
        "crates/aa-runtime/src/saved_organization_combat.rs",
        "crates/aa-runtime/src/saved_organization_combat_tests.rs",
        *[str((HERE / name).relative_to(ROOT)).replace("\\", "/") for name in [
            "native_saved_org_combat.py", "actual-saved-inputs.json", "native-saved-org-combat.json",
            "README.txt", "saved-adapter-tests.log", "scoped-clippy.log",
            "scoped-clippy-existing-allowances.log", "runtime-clippy.log", "pin_findings.py",
        ]],
    ]
    dependencies = [
        "crates/aa-mapworld/src/restore.rs",
        "crates/aa-mapworld/src/organization_restore.rs",
        "crates/aa-runtime/src/catalog.rs",
        "crates/aa-combat/src/organization_combat_value.rs",
        "research/skirmish-20261016-borrowed-combat-value/findings.json",
        "research/phase8-economy/harness.py",
    ]
    source_map = GAME / "reconstruction/assets/original/Data/Maps/(2v2)Kursk.RMP"
    doc = {
        "schema": 1,
        "date": "2026-10-16",
        "status": "frozen scoped adapter; focused Rust and independent native replay passed",
        "api": "aa_runtime::saved_organization_combat::recompute(&Catalog, OrganizationCombatContext)",
        "native_addresses": ["5788B0", "57DC40"],
        "original_sha256": native["original_sha256"],
        "v14_sha256": native["v14_sha256"],
        "native_windows": native["windows"],
        "native_executions": len(native["cases"]),
        "unmodified_partial_saved_executions": len(actual),
        "explicitly_substituted_storage_executions": len(probes),
        "precisions": sorted({row["precision"] for row in native["cases"]}),
        "source_map": {"path": str(source_map), "sha256": hashlib.sha256(source_map.read_bytes()).hexdigest()},
        "unmodified_saved_inputs_have_concrete_performers": False,
        "test_storage_inputs": {
            "performer_value": 10, "body_health": 3, "body_maximum": 4,
            "not_an_observed_native_initialization_receipt": True,
            "earlier_nonzero_current_on_required_source_error": 7.5,
            "earlier_nonzero_maximum_on_required_source_error": 10,
            "checks": ["Body absent", "Body present", "late unavailable source1E4", "null vacancy", "bit5-clear vacancy"],
        },
        "portable_identity_probe_cases": 12,
        "portable_identity_checks": [
            "later stale-generation member retains prior nonzero totals",
            "wrong Performer owner refuses after reset",
            "wrong Body owner refuses after reset",
            "wrong active Organization owner refuses before native stores",
        ],
        "tests": {
            "focused_rust": "1 passed, 0 failed; every frozen native row and 12 portable identity cases",
            "independent_native_replay": "66/66 passed in independent-saved-replay.json",
            "strict_clippy": "blocked by existing dependency lints; no pass claimed",
            "runtime_no_deps_clippy": "blocked by existing unrelated aa-runtime source/test lints; no new adapter/test diagnostic",
        },
        "binding_contract": [
            "Actual exclusive Organization storage with immediate68/6C stores on every reached path",
            "Generation-checked member GTIDs; actual active owner shell while outside registry",
            "Actual Performer word60 and enforced Performer-owner Body identity",
            "Current canonical layout slot groups and parameters, without Translate reapplication",
            "Narrow completed canonical174 mask getter; source1E4 read lazy on bit5 only",
            "Member/parameter/component containers stable under exclusive Organization and readonly registry; no reentry",
        ],
        "required_source_interruption": {
            "address": "57891A", "before": "FLD ActorData+1E4",
            "native_cpp_exception_claimed": False,
            "completed_source_value_supplied": False,
        },
        "limits": [
            "Completed canonical ActorData+1E4 producer remains explicitly unavailable",
            "Current partial Kursk prefix has no concrete Performer; nonzero tests are explicit substituted inputs",
            "No native immediate per-record Actor Init/PostCreate/publication completion claim",
            "Fresh managed stopped SAI/application mode1 clock0 context is a diagnostic fixture",
            "Typed ownership and canonical checks are portable diagnostics, not corrupt-pointer fault/exception parity",
            "Finite supported numeric inputs and nonzero Body divisor; no arbitrary reentry/container replacement",
            "No battle completion, original/save/profile mutation, release update or gameplay success claim",
        ],
        "backups": "backups/before-saved-org-combat-20261016-183833",
        "owned_files": [pin(p) for p in own],
        "shared_dependency_snapshots": [pin(p) for p in dependencies],
    }
    path = HERE / "findings.json"
    path.write_text(json.dumps(doc, indent=2) + "\n")
    print(json.dumps({"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}))


if __name__ == "__main__":
    main()
