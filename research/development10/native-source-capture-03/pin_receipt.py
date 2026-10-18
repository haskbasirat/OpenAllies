from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    capture = json.loads((HERE / 'capture.json').read_text())
    provenance = json.loads((HERE / 'runtime-files.json').read_text())
    process = json.loads((HERE / 'post-process-inventory.json').read_text(encoding='utf-8-sig'))
    assert capture['completed_source_return'] and capture['original_error'] is None
    assert capture['cleanup']['wait_result'] == 0 and not capture['cleanup']['errors']
    assert capture['cleanup']['exit_process_event_observed']
    assert capture['original_archives_preserved'] and capture['folder_redirection_observed']
    assert len(capture['captures']) == 4 and len(capture['captures'][-1]['actors']) == 1028
    assert all(row['legacy_fxsave_match'] for row in capture['captures'])
    assert all(row['legacy_x87_control'] == 0x27f for row in capture['captures'])
    assert all(row['legacy_x87_status'] & 0x20 for row in capture['captures'])
    assert all(row['mxcsr'] == 0x1fbd and row['crt_sse2_global'] == 1 for row in capture['captures'])
    assert provenance['all_copies_independent'] and provenance['all_source_copy_bytes_equal_after_capture']
    assert provenance['all_rwd_copies_readonly'] and len(provenance['records']) == 54
    assert not process['owned_capture_pid_present'] and not process['aa_processes']
    script = HERE.parent / 'capture_native_source.py'
    assert sha(script) == capture['script_sha256']
    paths = [HERE/'capture.json', HERE/'runtime-files.json', HERE/'verify_capture.py', HERE/'README.md',
             HERE/'post-process-inventory.json', HERE/'pin_receipt.py', script,
             HERE.parent/'capture_native_source.attempt02-preserved.py',
             HERE.parent/'native-source-capture-02/FAILED-ATTEMPT.txt',
             HERE.parent/'native-source-capture-01/FAILED-ATTEMPT.txt']
    report = {'schema': 1, 'status': 'actual_windows_source_capture_complete', 'battle_test': False,
              'receipt_files': [{'path': str(p.resolve()), 'sha256': sha(p), 'bytes': p.stat().st_size} for p in paths],
              'profile': {'x87_control': 0x27f, 'precision': 'Double53', 'precision_exception_pending_at_all_points': True,
                          'mxcsr': 0x1fbd, 'crt_sse2_global': 1},
              'actor_records': 1028, 'means': 8, 'runtime_files': 54,
              'pending_event_cleanup_successful': True, 'original_archives_preserved': True,
              'owned_process_reaped': True, 'all_commands_reaped': True,
              'excluded_invalid_evidence': ['runtime-files.invalid-path.json (initial zero-file wrong-root validator)'],
              'attempt02_original_exception': 'unknown; old cleanup masked it',
              'attempt01_profile_isolation': False}
    out = HERE / 'receipt-pins.json'
    with out.open('x', encoding='utf-8') as stream:
        json.dump(report, stream, indent=2)
        stream.write('\n')
    print(json.dumps({'sha256': sha(out), 'captures': 4, 'actors': 1028, 'profile': report['profile']}))


if __name__ == '__main__':
    main()
