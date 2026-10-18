from pathlib import Path
from hashlib import sha256
import json
from datetime import datetime, timezone

out = Path(__file__).resolve().parent
root = out.parents[1]
def pin(relative):
    p = root / relative
    return {"path": relative, "sha256": sha256(p.read_bytes()).hexdigest(), "bytes": p.stat().st_size}
rows = json.loads((out / "source-summary.json").read_text())["rows"]
ids = [r["id"] for r in rows]
assert ids == ["air_support", "armorhqproduction", "mechhqproduction", "mg", "money", "economic_building", "nation_ger", "corpshq", "nation_ger", "mobile_building", "building"]
assert all(r["bias"] == 0 and r["fx_counts"] == {"WhenAdded": 0, "WhenRemoved": 0} for r in rows)
assert all(not r["modifiers"] for r in rows[:-1])
report = {
    "created_utc": datetime.now(timezone.utc).isoformat(),
    "scope": "Actual ger_corps initial11 source and existing runtime binding audit before natural PostCreate",
    "result": "No new positive-child implementation required for this exact source list; natural execution remains pending",
    "evidence": [pin("research/skirmish-20261017-ger-corps-initial-properties/source_probe.rs"), pin("research/skirmish-20261017-ger-corps-initial-properties/source-probe-v2.exe"), pin("research/skirmish-20261017-ger-corps-initial-properties/actual-source.json"), pin("research/skirmish-20261017-ger-corps-initial-properties/source-summary.json")],
    "source_probe": "PASS5.38s; actual V14 sources, validated completed source Bank moved once from Catalog, actual native_actor_properties and registered State definitions; no Actor/World or instance insertion",
    "initial_order": ids,
    "findings": [
        "Same registered Bank Property tokens retained; duplicate nation_ger remains in source order",
        "First10 entries have no numeric/flag/table modifiers; all11 have no WhenAdded/WhenRemoved FX and zero performer CV bias",
        "building alone has supported Resistance armor_piercing amount0x3e4ccccd and Immunity ench_type_radiation table modifiers",
        "Current mask7974753 omits Organization, Settlement, Mover and Character, so their absent-component branches skip natively",
        "Initial5817A0 grants intentionally bypass4CEF10 admission; the first3 required corpshq values are not a reason to reject these initial grants",
        "Duplicate nation_ger appends an actual disabled instance through503680; it is not deduplicated away",
        "Same-owner Performer refresh remains the common enabled-instance tail; no second current State/Bank is made"
    ],
    "audited_current_sources": [pin("crates/aa-runtime/src/fresh_post_create_world.rs"), pin("crates/aa-runtime/src/staged_property_instances.rs"), pin("crates/aa-runtime/src/staged_property_admission.rs"), pin("crates/aa-runtime/src/stored_performer_refresh.rs")],
    "limits": [
        "No production changes made for this audit",
        "No retained original mutable replacement Params property count exists in actual39; unknown is not asserted empty",
        "Constructed17CREATE_VFX is a separate conditional prefix before PostCreate",
        "Positive ground deformation, City, Organization, Settlement, numeric modifiers and nonempty Property FX outside this exact source domain retain their own required children",
        "Actual40 stopped earlier at DetectionAfter Zone6; natural initial11 has not executed",
        "Direct probe is pinned by source/executable/output; the build dependency directory later evolved, so no retrospective frozen whole-rlib claim"
    ]
}
path = out / "findings.json"
if path.exists():
    raise RuntimeError("refusing to overwrite evidence")
path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
print(sha256(path.read_bytes()).hexdigest())
