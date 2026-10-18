"""Pin frozen, bounded author evidence without regenerating the fixtures."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def pin(path):
    raw = path.read_bytes()
    return {"path":path.relative_to(ROOT).as_posix(), "bytes":len(raw),
            "sha256":hashlib.sha256(raw).hexdigest()}


assert "test result: ok. 3 passed; 0 failed; 0 ignored" in (HERE/'rust-tests-final-v3.log').read_text()
final_log = ROOT/'research/skirmish-20261016-load-zone-entry/rust-tests-frozen.log'
assert "test result: ok. 4 passed; 0 failed; 0 ignored" in final_log.read_text()
initial = json.loads((HERE/'native-initial-zones.json').read_text())
current = json.loads((HERE/'native-current-services.json').read_text())
assert len(initial['executed_address_byte_pins']) == 217
assert len(current['cases']) == 30 and len(current['executed_address_byte_pins']) == 364
doc = {
    "schema":1, "status":"author source/evidence frozen; independent review separate",
    "scope":"Actual pre-first-record mode0 ZoneMap factory and reached saved Object footprint/memory services; no complete initialization or playable battle claim",
    "native_initial":{
        "machines":6, "mode_rows":126, "zero_and_actual_variation_binding_rows":12,
        "executed_pins":217, "intercepted_leaf_entries":2,
        "executed":"4981A0 payload stores,59E520/511860,557E6E..91/54EA50,5566AA..C6 zero fill and54FDC0",
        "seams":"allocation and mode registration storage; not whole enum intern/CString or full GWorld/GMap loader proof",
    },
    "native_current":{
        "rows":30, "fresh_original_v14_machines":60, "executed_pins":364,
        "seams":[],
        "executed":"4EA150/554DD0/554330/4CF210/ordinary shape getters,4EB490/4EA350/76AC90/4EBC70",
        "controlled_inputs":"actual stopped first Kingdom field-shaped pose/source shape/flags, explicit fixture zero point, two component masks and five Session phases",
        "not_claimed":"Captured actual mask/startup phase, global point producer, complete Object Init or positive detection-memory branches",
    },
    "rust":{
        "passed":3,"failed":0,"ignored":0,
        "actual_map":"Campaign/tutorial02.RMP", "tile_count_each_precision":16384,
        "precision_modes":["Single24","Double53","Extended64"],
        "combined_final_passed":4,
        "guards":["saved mask is not current mode0 fog","mode/grid/static flags mismatch","fractional creator extent uses native ceil","runtime constructor storage refusal","reached private PointShape validates destination Catalog and current numeric source bindings"],
    },
    "concrete_owners":[
        "Same actual BoundTerrain, header-derived TerrainGridLayout and persistent SpatialIndex feed the initial ZoneMap",
        "Private PointShape and actual borrowed SessionPhase; no PE-zero construction or cached phase",
        "Current shell/source-bound geometry for occupancy and actual live ZoneMap sets/entry order",
        "Reached supported actorE8 and actual generation-checked Kingdom18C/190 fog-prefix reads",
    ],
    "remaining":[
        "PointShape startup provenance is a separate global-owner proof; native fixture point is explicit controlled input",
        "World owns stage correctness and same-map association; equal dimensions cannot authenticate foreign maps",
        "Zone entry actor identity uses ActorId.raw domain, distinct SpatialIndex keys need explicit mapping",
        "Positive blocking query, type1 reveal mask/kingdom vector, container100, memory/SAI and source16D children refuse when reached",
        "No complete world load, map Init, actor registration, tick, battle or renderer parity certification",
    ],
    "production_pins":[pin(ROOT/p) for p in ['crates/aa-runtime/src/load_zone_services.rs','crates/aa-runtime/src/load_zone_services_tests.rs']],
    "evidence_pins":[pin(HERE/p) for p in [
        'native_initial_zones.py','native-initial-zones.json','native-initial-replay.log',
        'native_current_services.py','native-current-services.json','native-current-replay.log',
        'inspect_native.py','native-instructions.log','native-load-order.log',
        'world-mode-table-xrefs.log','world-mode-owner-xrefs.log','world-mode-ctor-neighbors.log',
        'rust-tests-final-v3.log','README.txt']]+[pin(final_log)],
    "preservation":"No Cargo/lock, original native files/assets/save/profile/release edited; only new runtime module/test and shared module export for this slice",
}
(HERE/'findings.json').write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps(pin(HERE/'findings.json')))
