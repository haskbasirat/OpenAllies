"""Seal the bounded reviewer checkpoint after the loading-flag test is copied.

Only this review directory is written. Production/native fixtures stay immutable.
"""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    loading_log = HERE / "loading-flag-rust-replay.log"
    assert loading_log.exists(), "execute the retained loading-flag regression first"
    log = loading_log.read_text(encoding="utf-8-sig")
    assert "1 passed; 0 failed" in log and "exact_native_loading_flag_store" in log
    frozen_binary = HERE / "loading-flag-tests.exe"
    assert frozen_binary.is_file()
    historical = HERE / "reviewed-map-actor-null-only.rs"
    assert sha(historical) == "4e91d9faab83d729384fe9440441c3db9a836f9f34bc946930f0fd8cfa0f3a1c"
    corrected = ROOT / "crates/aa-runtime/src/stored_world_registration.rs"
    assert sha(corrected) == "0b5dc7729e76e48aa78b40f42204730929fb9a8632bf1c4c17d7e19ce4742fa1"
    local_paths = [
        "native-caller-disassembly.txt", "replay_empty_fx.py", "native-empty-fx-immutable.log",
        "native_null_flag_zero.py", "native-null-flag-zero.json", "native-null-immutable.log",
        "native-loading-flag-replay.log", "loading-flag-rust-replay.log", "loading-flag-tests.exe",
        "registration-kernel-tests.log", "publication-kernel-tests.log",
        "reviewed-map-actor-null-only.rs", "null-only-reconstruction.json", "source-review-pins.json",
        "actual_ger_corps_fx.rs", "actual-ger-corps-fx.exe", "actual-source-run.log",
        "actual_ger_corps_fx_v2.rs", "actual-ger-corps-fx-v2.exe", "actual-ger-corps-source-v2.json",
        "actual_ger_corps_fx_v3.rs", "actual-ger-corps-fx-v3.exe", "actual-ger-corps-source-v3.json",
        "actual-ger-corps-type-summary.json",
    ]
    root_paths = [
        "crates/aa-runtime/src/stored_world_registration.rs",
        "crates/aa-runtime/src/stored_scene_publication.rs",
        "crates/aa-runtime/src/map_actor_creation.rs",
        "crates/aa-runtime/src/definition_fx.rs",
        "crates/aa-spatial/src/index.rs",
        "crates/aa-spatial/src/index_loading_flag_tests.rs",
        "crates/aa-runtime/src/spatial_world.rs",
        "research/development10/fresh-world-registration/native_loading_flag.py",
        "research/development10/fresh-world-registration/native-loading-flag.json",
        "qa-output/development10/runtime-tests53.exe",
        "target/native-runtime/release/deps/libaa_runtime-4004dde273efdec2.rlib",
        "target/native-runtime/release/deps/libaa_rules-01028b5152c9e32e.rlib",
        "target/native-runtime/release/deps/libaa_numeric-55927aa222c0a1ba.rlib",
        "target/native-runtime/release/deps/libserde_json-3f0b3bb60c709793.rlib",
    ]
    pins = {str((HERE / path).relative_to(ROOT)).replace("\\", "/"): sha(HERE / path) for path in local_paths}
    pins.update({path: sha(ROOT / path) for path in root_paths})
    result = {
        "accepted": True,
        "blocking_findings": [],
        "scope": "Independent review of Root's fresh4F9510 composition, native50C220 early publication gates and the exact historical null-only42A590 forwarding checkpoint. Acceptance includes the dedicated loading-flag correction; it excludes the positive FX successor.",
        "reviewer_relationship": "Reviewer authored some reused Actor-registration/publication kernels. This is an independent review of Root's new composition, not independent acceptance of every reused kernel or the reviewer's own stored_actor_registration adapter.",
        "resolved_findings": [{
            "severity": "blocking",
            "problem": "Fresh4F9510 initially called clear_registration_flags(key,1<<31), whose existing API deliberately accepts only bits27/28; every reached fresh registration would therefore fail.",
            "resolution": "A dedicated clear_loading_flag primitive applies only flags&0x7fffffff to the same member. SpatialWorld resolves its same key and Root's wrapper now calls that primitive. The old bits27/28 API remains restricted.",
            "verification": "Fresh immutable native4F953C..4F954C replay and the exact actual-index regression both pass. The regression checks same member address, every other serialized field, tiles/references/events, invalid-key retention and the old API's bit31 refusal.",
        }],
        "source_audit": [
            "Same World registration lists and actual ActorStore ActorAccess/Arena are lent disjointly; no copied/restored shell, take/default lists or detached placement authority.",
            "Actual completed moved Bank/source/GTID/class and published spatial precision are validated. Current class/generation/source are reread after Actor registration even on child error; prior stores are retained.",
            "Native4F9510 name capture and flag store precede downstream name/class/state effects. Capture cloning models the native captured CString, not a gameplay-state copy.",
            "Source virtual+24 used by4F4C40 is distinct from source+2C45DB10. Class3 runtime+B0 is the actual constant true leaf.",
            "Native4F9FC7 uses Data+BC Created;4D755F uses Data+E4 Constructed, both with actual byte0. Null forwarding copies raw XYZ twice before429080 and returns only on a same-Bank certified empty source list.",
            "The null-only checkpoint retains precise errors for positive FX. Scene publication reads actual WorldD8 and actual attached-view owner; positive client factory remains required rather than inferring readiness from process VWorld/AD or constructed-World metadata.",
        ],
        "executions": {
            "native_null_byte_zero": {"rows": 12, "executions": 24, "executed_pins": 60, "child_seams": 0, "result": "PASS"},
            "native_empty_source": {"executions": 6, "result": "PASS", "qualification": "Original/v14 allPC; source count and enum sink are explicitly named child seams, not successful positive FX execution."},
            "native_loading_flag": {"pairs": 216, "executions": 432, "executed_pins": 4, "result": "PASS"},
            "actual_index_loading_flag": {"tests": 1, "result": "PASS", "binary": "loading-flag-tests.exe"},
            "retained_root53_actor_registration_kernel": {"tests": 1, "result": "PASS"},
            "retained_root53_publication_kernel": {"tests": 1, "result": "PASS"},
            "source_lab_v3": {"result": "PASS", "qualification": "Direct rustc against pinned root53 no-LTO production rlibs. Actual V14 source resolution and completed Bank move only; no Actor allocation, graph compilation/execution, full Attempt or World."},
        },
        "actual_ger_corps_source": {
            "native_class": 3, "component_mask": 7974753, "source_pc": "Double53",
            "canonical_source": "Buildings/GER/ger_corps.tgi",
            "canonical_source_sha256": "1df68a3b479fadad7cbc7edade480779a3c556642fda5a1ee76d10bc03855b2b",
            "counts": {"WhenCreated": 0, "WhenDestroyed": 1, "WhenSelected": 2, "WhenCommanded": 0, "WhenCommandedAggressive": 0, "WhenConstructed": 17},
            "types": "WhenDestroyed:CREATE_VFX; WhenSelected:PLAY_OBJECT_ANIMATION,PLAY_SFX; all17 WhenConstructed:CREATE_VFX. All20 nodes have no Chain/ChainImmediate children.",
            "order": "Resolved source order only; not compiled native intrusive execution order.",
            "failed_probe_preserved": "The v1 source lab incorrectly asserted both Created and Constructed empty and failed. Its executable/source/failure log are retained. v2/v3 remove the invalid assertion and report actual source counts; no production defect was inferred from the failed lab assumption.",
            "implication": "Created is certified null for this source. Constructed requires a positive FX implementation if the native state/descriptor trigger is reached; this lab does not establish that the natural trigger has run.",
        },
        "historical_source": {
            "null_only_sha256": sha(historical),
            "reconstruction": "Exactly three incoming positive-publication edits were reversed in a review-only copy; the resulting full-file SHA equals Root's original manifest. No production source was changed by the reviewer.",
            "excluded_successor": "Current stored_creation_fx positive Graph compile/construct_existing/worker/pending queue successor and all subsequent integration are outside this acceptance.",
        },
        "limits": [
            "No naturally reached fresh4F9510 composition or whole mission/battle acceptance. The supplied actual37 record stopped earlier at Mind Recruit eligibility5827B0.",
            "Root53 binary/rlibs are frozen historical dependencies. Reused kernel/source-producer results do not execute the new full wrapper or its positive successor.",
            "Portable binding/source/session/spatial preflights are stronger typed-domain guards; no invalid-pointer, allocator failure, SEH or native crash parity is claimed.",
            "Positive City/control/guard/discovery and attached-client factory child requirements remain explicit. No full native renderer, View or Model readiness is minted.",
            "No original assets/executables/saves/releases or manifest/lock files were modified by this review.",
        ],
        "pins": pins,
    }
    out = HERE / "independent-review.json"
    assert not out.exists(), "sealed review already exists"
    out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"accepted": True, "blocking_findings": [], "path": str(out.relative_to(ROOT)), "sha256": sha(out)}))


if __name__ == "__main__":
    main()
