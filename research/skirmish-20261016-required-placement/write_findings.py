"""Seal bounded author results; does not regenerate native fixtures."""
from pathlib import Path
import hashlib
import json
import re

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def pin(path):
    raw = path.read_bytes()
    return {"path": path.relative_to(ROOT).as_posix(), "bytes": len(raw),
            "sha256": hashlib.sha256(raw).hexdigest()}


def results(path):
    text = path.read_text()
    rows = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored", text)
    assert rows, path
    counts = [sum(int(row[i]) for row in rows) for i in range(3)]
    assert counts[1] == 0
    return {"passed": counts[0], "failed": counts[1], "ignored": counts[2],
            "result_groups": len(rows), "log": pin(path)}


core = json.loads((HERE / "native-required-placement.json").read_text())
getters = json.loads((HERE / "native-shape-getters.json").read_text())
assert len(core["cases"]) == 177
assert len(getters["cases"]) == 108
source_files = [
    "crates/aa-spatial/src/required_placement.rs",
    "crates/aa-spatial/tests/required_placement_native.rs",
    "crates/aa-runtime/src/live_required_placement.rs",
    "crates/aa-runtime/src/live_required_placement_tests.rs",
]
artifacts = [
    "native_required_placement.py", "native-required-placement.json",
    "native_shape_getters.py", "native-shape-getters.json", "inspect_native.py",
    "native-instructions.log", "native-instructions-v2.log",
    "native-replay-final.log", "native-shape-replay-expanded.log", "README.txt",
]
doc = {
    "schema": 1,
    "status": "author source and evidence frozen; independent review separate",
    "scope": "Native4F1660 required-placement coordinator and current pre-staging saved-shell adapter; no complete Init or playable-battle claim",
    "native": {
        "coordinator_rows": 177, "coordinator_fresh_original_v14_executions": 354,
        "coordinator_executed_instruction_addresses": 397,
        "coordinator_intercepted_leaf_entries": 6,
        "getter_rows": 108, "getter_executed_instruction_addresses": 7,
        "getter_intercepted_leaf_entries": 0,
        "precision_modes": ["Single24", "Double53", "Extended64"],
        "runtime_classes_getter_proof": [2, 3, 4, 6, 7, 9],
        "proof_boundaries": [
            "Coordinator tile iterator, virtual results, zone lookup/overlap,46EF30 result,4D1120 and thread service are explicit seams",
            "Getter fixture executes actual0C/B0/B4 leaves and pins constructor vtable stores; it does not execute complete constructors",
            "Actual Berlin test verifies source ownership/refusals/metadata, not successful end-to-end placement using a fabricated completed manager",
        ],
    },
    "rust": {
        "aa_spatial_all": results(HERE / "spatial-all-tests.log"),
        "runtime_final": results(HERE / "rust-live-tests-final-v3.log"),
        "actual_map": "Maps/(2v2)Berlin.RMP",
        "terrain_metadata_cells_each_precision": 50176,
        "actual_source_precision": "Double53, independent of current query precision",
    },
    "binding": [
        "Catalog Bank-owned completed reference manager maximum74 and exact source identity guard",
        "NativeSourceRequirements whole receipt, canonical provenance and actual Bank source-PC equality",
        "Canonical current component mask/proof/category and actual LoadKind/Object/Actor storage guards",
        "Persistent generation mapping supplies node identity between callback-free load-field leases",
        "Current shell/registry supplies flags, pose, Object50 and generation checks",
        "Actual effective BattleTerrain Variation/TerrainData4C and live ZoneMap sets",
    ],
    "remaining": [
        "Positive Settlement464 source field remains unsupported by its bounded source producer",
        "Parent100/15C, mobility4D1120, hostility140, zone actors and actual app/thread/global modes remain mandatory services",
        "Caller must borrow same-world map/index/terrain/zone owners; equal dimensions are not identity authentication",
        "This adapter is pre-staging load-shell binding, not a post-staging gameplay query service",
        "No full map Init, tick, battle readiness or renderer parity certification",
    ],
    "preservation": "No Cargo/lock, original asset, save, profile, renderer or release edited. Only four new source/test files and module exports for this port.",
    "production_pins": [pin(ROOT / p) for p in source_files],
    "artifact_pins": [pin(HERE / p) for p in artifacts],
}
(HERE / "findings.json").write_text(json.dumps(doc, indent=2) + "\n")
print(json.dumps({"source_files": len(source_files), "rust": doc["rust"],
                  "findings": pin(HERE / "findings.json")}))
