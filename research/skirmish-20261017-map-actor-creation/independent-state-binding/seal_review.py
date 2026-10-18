"""Seal read-only independent review of the author's explicit v1 checkpoint."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
AUTHOR = HERE.parent
H = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
freeze = json.loads((AUTHOR / "freeze.json").read_text())
addendum = json.loads((AUTHOR / "evidence-addendum.json").read_text())
assert addendum["historical_freeze_sha256"] == H(AUTHOR / "freeze.json")
for group in ["historical_docs", "corrected_docs"]:
    for name, expected in addendum[group].items():
        assert H(ROOT / name) == expected
pins = []
for item in freeze["pins"]:
    path = ROOT / item["path"]
    actual = H(path)
    entry = dict(path=item["path"], sha256=actual)
    if actual != item["sha256"]:
        assert item["path"] == "research/skirmish-20261017-map-actor-creation/README.txt"
        historical = AUTHOR / "source-v1/README.txt"
        assert H(historical) == item["sha256"]
        entry["historical_sha256"] = item["sha256"]
        entry["qualification"] = "documented pin-count wording correction only, historical document retained and evidence-addendum verified"
    if "snapshot" in item:
        snapshot = ROOT / item["snapshot"]
        assert H(snapshot) == actual
        entry["snapshot"] = item["snapshot"]
    pins.append(entry)
audit = {"checked": len(pins), "changed_production_tests_fixtures_binaries": [], "approved_doc_addenda": ["README.txt"], "bad_snapshots": [], "pins": pins}
(HERE / "pin-audit.json").write_text(json.dumps(audit, indent=2) + "\n")
binary = ROOT / "target/s-owner-recruitment/release/deps/map_actor_creation-b275c3e451b3d8f8.exe"
assert H(binary) == "c87465db5056bf1b21e47c8329ee2afbdcaa4f035074a1b70da97ced056e2664"
logs = ["native_constructor-replay.log", "native_defender-replay.log", "native_layout_slots-replay.log", "native_requirements-replay.log", "native_construction_actions-replay.log", "rust-replay.log"]
assert "3 passed; 0 failed" in (HERE / "rust-replay.log").read_text()
report = {
    "schema": 1,
    "reviewer": "/root/state_binding_review",
    "status": "accepted_within_explicit_five_constituent_scope",
    "blocking_findings": [],
    "author_freeze_sha256": H(AUTHOR / "freeze.json"),
    "pin_audit": {"path": "pin-audit.json", "sha256": H(HERE / "pin-audit.json"), "checked_pins": len(pins), "unchanged_non_document_pins": len(pins) - 1, "document_addendum_sha256": H(AUTHOR / "evidence-addendum.json")},
    "validation": {
        "fresh_immutable_native": {"producers": 5, "rows": 261, "separate_original_v14_executions": 522, "precisions": ["Single24", "Double53", "Extended64"], "author_fixtures_written": False},
        "frozen_rust": {"path": str(binary.relative_to(ROOT)), "sha256": H(binary), "passed": 3, "failed": 0, "seconds": 3.36, "command": "AA_GAME_ROOT=<actual game root>; AA_CREATION_CONSTRUCTOR_CAPTURE unset; exact binary --include-ignored --nocapture"},
        "logs": [{"path": name, "sha256": H(HERE / name)} for name in logs],
    },
    "accepted_constituents": [
        "Direct unowned actual-source 4D6BB0 constructor plus 4E9FC0 identity-before-scale values; exact Catalog cached Arc and independent source/live/startup precision inputs",
        "Actual 567CD0 Defender constructor with actual completed local/shared source identities/powers and same ActorStore Ability arena; explicit fresh laboratory Bank",
        "Source-aware early 4D1350 -> 565140/565090 and 4CFCC0 contributor/publication while only Construction is installed; lazy foreign-Catalog error preserves prior action-word clears",
        "4C2BC0 primary/fallback/empty organization slot selection on valid captured and synthetic arrays",
        "Native component +20 requirement leaves, including 589090 real current Structure owner state read and Transport 7C7840 native slot3 Mover",
    ],
    "source_precision_qualification": {
        "direct_lab": "construct_new_bound certifies the same Catalog cache Arc at binding.precision. It does not by itself certify an installed or moved Bank source PC. The fixture uses explicit Double53 source and Single24 direction startup; live arithmetic varies independently.",
        "live_entry": "Source::bind separately validates the actual moved Staged Bank source PC/source identities before binding. That live path is compiled, not established by this checkpoint's direct-lab replay.",
    },
    "evidence_wording_addendum": {
        "constructor_pins": {"address_byte_pins": 533, "executed_image_instruction_addresses": 530, "intercepted_entries": ["0x77da70", "0x77dc50", "0x856370"]},
        "basis": "phase8-conversion inherited hook records original-image bytes before intercepting seams; independent replay and constructor-pin-classification.json distinguish them. Author corrected README/findings with verified evidence-addendum and preserved historical documents; frozen native fixture unchanged.",
        "other_producers": "phase8-ai hook intercepts seams before pinning; Defender161, Construction349, Layout52, Requirements46 are executed image instruction addresses",
    },
    "host_domain_guards": [
        "Forged equal Arc and foreign Catalog refusals are portable source authority guards, not native invalid-pointer failure equivalence",
        "Array bounds, missing current Structure state and absent services remain explicit errors rather than guessed defaults",
    ],
    "not_accepted_by_this_review": [
        "Complete live map_actor_creation suffix and map_component_init_create dispatch",
        "Selected naturally initialized Staged fresh creation/replacement run",
        "Full specialized InitCreate, nonNULL InitExisting, PostCreate, placement/registration/FX/failure destruction",
        "Complete Organization leader/member/hero creation",
        "Native allocation failure, SEH/unwind, CString/allocator/profiler transport equivalence",
        "Full mission activation, playable battle, campaign or multiplayer",
    ],
    "production_edits_by_reviewer": False,
    "author_fixtures_modified_by_reviewer": False,
    "game_complete_claimed": False,
}
(HERE / "independent-review.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps({"checked_pins": len(pins), "review_sha256": H(HERE / "independent-review.json"), "status": report["status"]}))
