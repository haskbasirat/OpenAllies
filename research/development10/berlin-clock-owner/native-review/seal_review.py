"""Seal the independent read-only native/source clock diagnosis."""
from pathlib import Path
import hashlib
import json
from datetime import datetime, timezone

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def pin(path):
    p = ROOT / path
    return {"path": path, "bytes": p.stat().st_size, "sha256": hashlib.sha256(p.read_bytes()).hexdigest()}


def main():
    prefix = "research/development10/berlin-clock-owner/caller-review-source/"
    sources = [prefix + "crates/aa-runtime/src/" + name + ".rs" for name in [
        "catalog", "restoration", "load_store_memory_refresh", "load_store_fog",
        "load_mission_start_world", "stored_actor_reveal", "map_actor_footprint", "live_world_frame",
    ]]
    sources += [prefix + "crates/aa-netcode/src/local_skirmish_worker.rs"]
    other = [
        "crates/aa-mapworld/src/world.rs", "crates/aa-simulation/src/arithmetic.rs",
        "crates/aa-runtime/src/initial_world_sync.rs", "crates/aa-runtime/src/load_detection_memory.rs",
        "research/skirmish-20261016-load-detection-memory/native_memory.py",
        "research/skirmish-20261016-load-detection-memory/native-memory.json",
        "research/skirmish-20261016-store-memory-refresh/native_memory_refresh.py",
        "research/skirmish-20261016-store-memory-refresh/native-memory-refresh.json",
        "research/skirmish-20261016-store-memory-refresh/native_memory_publication.py",
        "research/skirmish-20261016-store-memory-refresh/native-memory-publication.json",
        "research/development10/berlin-clock-owner/caller-review.json",
        "research/development10/berlin-clock-owner/native-review/probe_clock_read.py",
        "research/development10/berlin-clock-owner/native-review/native-clock-read-probe-v2.json",
        "research/development10/berlin-clock-owner/native-review/native-owner-dispatch.json",
    ]
    doc = {
        "schema": 1, "utc": datetime.now(timezone.utc).isoformat(),
        "reviewer": "/root/state_binding_review", "implementation_owner": "/root/builder_source_review",
        "status": "Native read/source lifetime diagnosis complete; no implementation or Berlin execution acceptance",
        "observed_error": "4EC4C8 actual GWorld+E0 clock owner",
        "conclusion": "Before activation the actual clock authority is the same restored Staged.clock. Lend it explicitly through all reached fresh/published memory entrances, and read begin.0 only at the existing first refresh getter. The readonly mission Catalog intentionally owns source capabilities but has lost its immediate-load runtime clock loan.",
        "native_sites": [
            {"site": "4F7D2F", "fact": "GWorld constructor publishes its actual THIS to process global B36534; no thread-local or timer substitute is indicated."},
            {"site": "5A302A/5A3031", "fact": "MapResource worker loads that same B36534 in ECX and calls actual GWorld Load4FC600."},
            {"site": "4FC6E4/4FC6F3/4FC702/4FC711", "fact": "World Load passes actual DC/E0/E4/E8 destination addresses to stream virtual0; each transfer is four bytes. Rust GWORLD decode and restored clock field order match."},
            {"site": "4F2871", "fact": "Fresh Init zeroes E0 before load; this constructor fact cannot replace the subsequently loaded or current E0."},
            {"site": "4EC4AE/4EC4BD/4EC4C8/4EC4D3", "fact": "Refresh reads process GWorld, reads raw E0, stores Record[0], then invokes actual name virtual24. It performs no clock calculation or float conversion."},
            {"site": "4F9A0D/4F9A1A/4F9A35", "fact": "Live step copies previous E4 into current E0, then computes/stores new E4. After activation authoritative ownership moves into the same ClockedState World clock."},
        ],
        "source_lifetime": [
            "WorldInventory::decode verifies source GWORLD chunk and reads tick/begin/end/step in native DC/E0/E4/E8 order.",
            "restoration.rs577 constructs NativeClock from those actual saved fields, validates finite values and positive step, then628 loans &clock into immediate Catalog.",
            "catalog.into_load_parts retires scoped clock references; Attempt::into_staged moves the same clock field. SourceData correctly does not retain it.",
            "local_skirmish_worker.rs189 constructs a fresh readonly Catalog after post_load, binding Resource/source graphs/Session/execution/settings but no clock; same Staged enters finish_load and start_mission.",
            "New Catalog None is a missing loan, not evidence that current World time is zero or that no World exists.",
            "map preparation/startup current source contains no write/reset/advance of Staged.clock. Genuine activation moves that field into aa_world::World, later ClockedState.",
        ],
        "lazy_gates_and_prefix": [
            "4EBC70 skips when neither keeps nor eligible; skips null Kingdom slots; reads nonmajor/current existing slots/shared visibility+exploration/alive in native order.",
            "Timestamp is unnecessary on skipped eligibility, existing-memory retention, deletion, empty Kingdom traversal, or unremembered branches. Do not require clock at public entry.",
            "4ECA80 guards null/nonmajor Kingdom, alive eligibility or reached dead potential-visible child, and already-populated slot before allocation/refresh.",
            "If needed, slot table allocation precedes fresh 0x38-byte Record allocation/constructor; then4EC4A0 reads time. Missing time must preserve this allocated but unlinked Record.",
            "Successful time read/store is first record field mutation; later name/pose/definition/owner/aspect/stats errors preserve timestamp and every earlier field store.",
            "After successful full refresh, native rereads Kingdom index and publishes the Record into that actual slot, then checks active observer. No earlier slot publication or observer callback.",
            "Actor64 show/explore and global Fog temporary bit30 clear may already be committed before memory-create/clock failure; no cleanup rollback or extra reads.",
        ],
        "required_binding": [
            "Clock loan must come from the same Staged used by fresh actor creation/footprint/Zone/published refresh and global Fog; carry through recursive Actor64 without a scalar snapshot.",
            "Keep old immediate Catalog loan behavior where retained. Prefer explicit optional &NativeClock in the shared memory carrier; absence errors at native time read after allocation, not eagerly.",
            "Avoid externally borrowing a child clock across a broad mutable &mut Loaded/Staged call. Thread the disjoint clock field at its actual caller alongside actors/Bank/World.",
            "No queue clock, source inventory reread, stock clock, cloned NativeClock, captured timestamp, PerformanceTimer, QPC, Session time, map mode, or zero fallback.",
            "Do not add runtime clock to SourceData/source-transfer cache: transfer deliberately retires the immediate runtime loan before the owning clock moves.",
            "Existing same source/Bank/PC/proof/generation/layout/published placement preflights remain mandatory portability guards; they do not establish native invalid-pointer fault equivalence.",
        ],
        "fresh_execution": {
            "accepted_fixture": "native-clock-read-probe-v2.json", "paired_rows": 21, "native_executions": 42,
            "executed_instruction_pins": 17, "required_name_entry_pins": 1,
            "matrix": "Original/v14 x PC24/53/64 x seven explicit finite current E0 words; current E0 changed after ctor, DC/E4/E8 deliberately distinct.",
            "assertions": "Timestamp equals current E0 bit-for-bit (including signed zero and low bits); required real name entry stops; prior old name, position and definition stay untouched.",
            "earlier_probe": "An additional42 native executions produced identical numeric rows in v1; two static disassembly starts were found mid-instruction and corrected before accepted v2. No native row or pin was weakened.",
            "rust_builds_or_runs": 0, "actual_berlin_runs": 0, "crate_edits": 0,
        },
        "existing_evidence_audited_not_reexecuted": [
            "native-memory.json: actual4EBC70 iterator/gates and partial4ECA80 create prefix; eligibility/create/delete callbacks explicitly labelled test seams.",
            "native-memory-refresh.json:252paired/504machines; six actual class vtables/current fields; required name/aspect/stat stops.",
            "native-memory-publication.json:18paired/36machines; real allocation/refresh/slot publication with explicit eligibility seam, observer stops and later Kingdom index reread probe.",
        ],
        "limits": [
            "No exact Berlin caller/actor/time value is derivable from the shared leaf error alone. Earlier fresh MapPrepare/Zone/footprint can reach it before global Fog.",
            "New probe uses explicit lab fields and real native instructions, not actual map captured current values or whole worker execution.",
            "Whole load/init/live-step/worker lifecycle facts are static byte-equal PE/source audits and existing bounded evidence, not newly executed end-to-end.",
            "Builder owns code changes/checks. This report does not certify the fix, startup completion, gameplay, readiness, or a release binary.",
        ],
        "pins": [pin(p) for p in sources + other],
    }
    target = HERE / "findings.json"
    with target.open("x", encoding="utf-8", newline="\n") as out:
        json.dump(doc, out, indent=2)
        out.write("\n")
    print(json.dumps({"path": str(target), "sha256": hashlib.sha256(target.read_bytes()).hexdigest()}))


if __name__ == "__main__":
    main()
