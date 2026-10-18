"""Record this bounded integration's evidence without changing old research."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
QA = ROOT / "qa-output/resume-20261016"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def census(name):
    path = QA / name
    raw = json.loads(path.read_text(encoding="utf-8-sig"))
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "sha256": digest(path),
        "mode": raw["mode"],
        "map_count": raw["map_count"],
        "maps_without_failures": raw["maps_without_failures"],
        "failed_records": sum(group["count"] for group in raw["groups"]),
        "converted_records": raw["converted_records"],
        "skipped_records_by_cause": raw["skipped_records_by_cause"],
        "groups": [
            {"error": group["normalized_error"], "records": group["count"],
             "map_count": len(group["maps"]), "first": group["first"]}
            for group in raw["groups"]
        ],
        "post_load_by_outcome": raw["post_load_by_outcome"],
        "tutorials": [
            {"map": row["map"], "load_error": row.get("load_error"),
             "first_failure": next(iter(row.get("failures", [])), None),
             "post_load": row.get("post_load")}
            for row in raw["maps"] if "tutorial0" in row["map"]
        ],
    }


pinned = json.loads((HERE / "final-source-hashes-before-census.json").read_text(encoding="utf-8-sig"))
checked = []
for entry in pinned:
    path = Path(entry["Path"])
    after = digest(path)
    checked.append({
        "path": path.relative_to(ROOT).as_posix(),
        "before_census_sha256": entry["Hash"].lower(),
        "after_census_sha256": after,
        "unchanged_during_census": entry["Hash"].lower() == after,
    })

proofs = [
    "research/phase8-layouts/native-layout-data.json",
    "research/phase8-layouts/native-organization-component.json",
    "research/phase8-definitions/native-definition-registry.json",
    "research/phase7-properties/00501c90.asm.txt",
    "research/resume-20261016-morale/native-morale.json",
]
logs = [
    "before-inventory-test.log", "layout-bindings-test.log",
    "after-layout-inventory-test.log", "organization-modifier-tests.log",
    "layout-morale-integration-tests-final.log", "after-inventory-test.log",
]
findings = {
    "date": "2026-10-16",
    "scope": "Native Organization LayoutData, pointer identity, source morale and Properties adapter integration",
    "native": {
        "layout_load": "0x4C3610", "formation_load": "0x4C34A0",
        "layout_formation_load": "0x4C32B0", "layout_validation": "0x4C1BE0",
        "one_hop_find": "0x48D500", "organization_component": "0x466C4F..0x46707C",
        "element_link": "0x460000", "actor_has_property": "0x45EEF0",
        "organization_load": "0x57D130", "morale_recompute": "0x578300",
        "base_morale": "0x45D580", "properties_context": "0x4CF110",
        "additive_morale_modifier": "0x501C90",
    },
    "source_pins": checked,
    "native_evidence": [{"path": path, "sha256": digest(ROOT / path)} for path in proofs],
    "validation_logs": [{"path": (QA / log).relative_to(ROOT).as_posix(),
                         "sha256": digest(QA / log)} for log in logs],
    "tests": {
        "mapworld_properties_adapter": "2 passed; loading owner, member, nonzero deltas/floors, stale IDs, absence/wrong payload/owner",
        "runtime_layout_source_adapter": "5 passed; native fixture fields/handles, partial registration, one-hop translation, actual setting, all 192 organizations at PC24/53/64",
        "existing_tutorial_catalog_restoration": "1 passed; original assets and staged owner state",
        "final_original_map_census": "1 diagnostic test; 80 original RMP files; see final census for actual stops",
        "fixture_setup_correction": "First integration run's translation test lacked an earlier template file. Fixed fixture source ordering; production source unchanged by this repair; final 6/6 runtime tests passed.",
    },
    "before": census("before-inventory.json"),
    "preliminary_layout_only": census("after-layout-inventory.json"),
    "after": census("after-inventory.json"),
    "limitations": [
        "InspectPartial continues after failed records; later errors may be cascades. These are not playable-world tests.",
        "No original native map load was executed end-to-end; the adapters compose existing native-proven primitives and original-source data.",
        "Canonical ActorData identities shadowed by Thing translations are explicitly unsupported instead of silently retranslated.",
        "Property contents and remaining Organization world callbacks are not completed by this change.",
        "Clean restoration/PostLoad does not imply InitForLoadedMap, conversions, publication, battle simulation, campaign, original saves or multiplayer completion.",
        "The two pre-existing localization failures disappeared in concurrent work; they are not attributed to these layout changes.",
    ],
}
review = HERE / "independent-review.json"
if review.exists():
    findings["independent_review"] = {
        "path": review.relative_to(ROOT).as_posix(), "sha256": digest(review),
    }
(HERE / "findings.json").write_text(json.dumps(findings, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"before": findings["before"]["failed_records"],
                  "after": findings["after"]["failed_records"],
                  "clean_maps": findings["after"]["maps_without_failures"],
                  "source_hash_changes": [x["path"] for x in checked if not x["unchanged_during_census"]]}, indent=2))
