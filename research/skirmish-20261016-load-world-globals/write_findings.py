"""Pin this completed bounded startup-global component, without rerunning captures."""
from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    files = [
        ROOT / 'crates/aa-runtime/src/load_world_globals.rs',
        ROOT / 'crates/aa-runtime/tests/load_world_globals.rs',
        ROOT / 'crates/aa-netcode/src/lobby.rs',
        ROOT / 'crates/aa-netcode/tests/session_owner_globals.rs',
    ]
    files += [HERE / name for name in [
        'capture_startup_globals.py', 'native_globals_proof.py',
        'native-instructions.json', 'seal_startup_reference.py',
        'point-startup-reference.json', 'post-process-inventory.json',
        'runtime-tests.log', 'netcode-tests.log', 'README.txt',
        'runtime-tests.before-validate-catalog-freeze.log',
        'independent-hardware-review.json',
        'capture-01/capture.json', 'capture-02/capture.json',
        'capture-01/runtime-files.json', 'capture-02/runtime-files.json',
    ]]
    review = ROOT / 'research/skirmish-20261016-load-zone-entry/point-binding-review.json'
    files.append(review)
    runtime = (HERE / 'runtime-tests.log').read_text()
    netcode = (HERE / 'netcode-tests.log').read_text()
    assert '3 passed; 0 failed; 0 ignored' in runtime
    assert netcode.count('1 passed; 0 failed; 0 ignored') == 2
    assert json.loads((HERE / 'post-process-inventory.json').read_text()) == []
    result = {
        'schema': 1,
        'status': 'frozen component; no first-map or playable-battle claim',
        'native_sources': {
            'point_getter': '4E3E90 -> B3A8C4 (six words)',
            'session_owner': '[B36550] KKC_Session, distinct timeline B36554',
            'phase_getter': '76AC90 reads actual Session+F8',
            'application_prefix': '54EF10 reads actual Session+E4 before World+48',
            'constructor': '76F890 EBX zero before E4/F8 stores; Windows observed constructor store/return',
            'state_transition': '77A750 actual retained Session; staging load precedes final change_to_state',
        },
        'production_contract': [
            'PointShape private provenance owner, constructed only from sealed observed startup receipt',
            'Current Catalog source owner, source fingerprint and sealed numeric target rechecked at construction and reached validate_catalog boundary',
            'SessionPhase reads actual authoritative Session.state/field_f8 on every call; no scalar copy/default',
            'Unbound application_state explicitly errors for phase-only implementations',
        ],
        'windows_observations': {
            'separate_processes': 2, 'threads_armed_per_process': [62, 62],
            'word_indices': [[0, 1, 2, 3], [2, 3, 4, 5]],
            'native_writes_before_cutoff': [0, 0],
            'post_cutoff_same_value_calibration_probes': [4, 4],
            'observed_point_words': [0, 0, 0, 0, 0, 0],
            'copied_support_files_checked': 108,
            'source_return_rows_compared_to_frozen_capture03': 1028,
            'source_cv_means_fp_numeric_match_capture03': True,
            'normal_cleanup_both_runs': True,
            'remaining_AA_processes': 0,
            'fp_sse_context_writes': 0,
        },
        'native_execution': {
            'original_v14_bytes_equal': True,
            'precisions': ['Single24', 'Double53', 'Extended64'],
            'phase_application_prefix_rows': 936,
            'isolated_constructor_store_rows': 234,
            'point_getter_rows': 3,
            'constructor_seam': 'isolated MOV with EBX=0 proved separately; not a complete constructor port',
            'application_scope': 'stops before World+48/node access; no dummy world values',
        },
        'validation': {
            'runtime_tests': {'passed': 3, 'failed': 0, 'ignored': 0},
            'actual_session_native_replay_tests': {'passed': 1, 'native_rows': 936},
            'existing_staging_tests': {'passed': 1, 'failed': 0},
            'profile': 'release --locked --offline -j4 --config profile.release.lto=false --config profile.release.codegen-units=16',
            'all_owned_commands_reaped': True,
            'earlier_pre_guard_compile_failure': 'preserved and excluded from successful test claims',
            'independent_hardware_receipt_review': 'accepted bounded observation; no tracer/sealer rerun',
            'independent_point_source_binding_review': 'accepted separately from hardware review',
        },
        'limits': [
            'Point is initial PE zero-fill seed carried through observed source completion and Session constructor; no perpetual native immutability claim',
            'Neither Windows capture reaches a map, Object::Init or a battle',
            'New services potentially mutating global must be proved before joining admitted ported load path',
            'Session getters do not fabricate startup Session or prove its full constructor',
            'Inherited debugger setup precedes main cleanup try; successful receipts require normal final cleanup',
            'CreateProcess original two handles live until bounded Python process exit; duplicates explicitly closed',
            'Focused tests only, not a full runtime/workspace or final EXE validation',
        ],
        'pins': {str(p.relative_to(ROOT)).replace('\\', '/'): sha(p) for p in files},
    }
    out = HERE / 'findings.json'
    out.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'findings_sha256': sha(out), 'pins': len(files)}))

if __name__ == '__main__':
    main()
