"""Independent bounded review record; does not edit author fixtures/sources."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
AUTHOR = HERE.parent


def pin(path):
    raw = path.read_bytes()
    return {"path": path.relative_to(ROOT).as_posix(), "bytes": len(raw),
            "sha256": hashlib.sha256(raw).hexdigest()}


log = (HERE / "rust-tests.log").read_text()
assert "test result: ok. 3 passed; 0 failed; 0 ignored" in log
fixture = json.loads((AUTHOR / "native-load-spatial.json").read_text())
capture = json.loads((AUTHOR / "actual-first-kingdom.json").read_text())
pins = fixture["address_byte_pins"]
leaves = [v for v in pins if int(v["va"], 16) in (0x6f7810, 0x6f7430)]
assert len(pins) == 308 and len(leaves) == 1 and int(leaves[0]["va"], 16) == 0x6f7810
assert len(fixture["cases"]) == 9 and len(capture["cases"]) == 3
doc = {
    "schema": 1,
    "status": "accepted within bounded load-time membership/storage scope",
    "scope": "Read-only independent source/ownership/error-prefix review of LoadSpatial, minimal load-field/index-reference APIs and same-index SpatialWorld adoption",
    "native_replay": {
        "fresh_immutable_replay": True, "rows": 9,
        "original_v14_fresh_machines": 18, "refresh_calls": 36,
        "address_byte_pins": 308, "executed_addresses": 307,
        "intercepted_leaf_entries": ["0x6f7810"],
        "actual_first_kingdom_rows": 3,
        "explicit_class2_3_vtable_probes": 6,
        "seams": "native list-node allocator/deallocator only; delete not reached",
        "actual_capture_scope": "Stopped tutorial02 first Kingdom with verified source-completion diagnostic, not full map Init",
    },
    "rust_replay": {"passed": 3, "failed": 0, "ignored": 0,
                    "actual_precision_modes": ["Single24", "Double53", "Extended64"]},
    "accepted": [
        "Actual Bank source identities and source precision govern immutable source geometry/flags/component-mask checks, independently of membership arithmetic PC",
        "Canonical category/template/source proof and actual LoadKind/Object/Actor storage are validated; immutable Binding proof defeats two consistently corrupted mutable copies",
        "All live snapshots validate before installing the callback-free field lease; current mutated flags commit on success/error and Drop clears every temporary field",
        "Persistent native list nodes, tile flags and generation references remain in the same index; reference_for_key reads identity without stale pose",
        "Adoption owns the same index before fallible work, attaches placement fields without node reinsertion, and preserves successful-prefix ownership plus pending/error metadata",
        "Actual Single24 probe transfers first Kingdom then refuses a second actual staged Kingdom with corrupt stale-generation transfer metadata; second staged placement and cache snapshot remain unchanged",
    ],
    "review_fixes_verified": [
        "Source-PC authority separated from live/query precision",
        "Canonical full category/source flags/component-mask guards",
        "Actual constructor storage checks and private immutable source certificate",
        "Successful-first/failed-second actual adoption storage regression",
        "Executed pins separated from intercepted allocator entry",
    ],
    "limits": [
        "No generic mutable query lease, post-staging gameplay collision readiness, derived runtime classes outside2/3/4 or complete Init is certified",
        "Corrupt/stale metadata probes are portable ownership/refusal tests, not claimed native corrupt-pointer semantics",
        "No renderer, complete battle startup, tick or playability claim",
        "Reviewer made no production/source/fixture edits",
    ],
    "production_pins": [pin(ROOT / p) for p in [
        "crates/aa-runtime/src/load_spatial.rs", "crates/aa-runtime/src/load_spatial_tests.rs",
        "crates/aa-spatial/src/index.rs", "crates/aa-runtime/src/spatial_world.rs"]],
    "evidence_pins": [pin(p) for p in [
        AUTHOR / "native_load_spatial.py", AUTHOR / "native-load-spatial.json",
        AUTHOR / "actual-first-kingdom.json", HERE / "native-replay.log", HERE / "rust-tests.log"]],
}
(HERE / "independent-review.json").write_text(json.dumps(doc, indent=2) + "\n")
print(json.dumps(pin(HERE / "independent-review.json")))
