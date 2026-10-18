"""Seal scoped acceptance; record-codec progress is not battle completion."""
from pathlib import Path
from datetime import datetime, timezone
import hashlib, json, re

ROOT = Path(__file__).resolve().parents[2]
QA = ROOT / 'qa-output/economy-lifecycle-20261016'

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))

def tests(name):
    path = QA / name
    text = path.read_text(encoding='utf-8-sig')
    rows = re.findall(r'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored', text)
    assert rows and all(int(ignored) == 0 for _, ignored in rows), name
    assert 'test result: FAILED' not in text and 'error: could not compile' not in text, name
    return {'passed': sum(int(passed) for passed, _ in rows), 'sha256': sha(path)}

def main():
    result = {'schema': 1, 'created_utc': datetime.now(timezone.utc).isoformat(),
        'scope': 'Faithful source checkpoint, skirmish before multiplayer before campaign; no new executable',
        'selected_logs': {}, 'compatibility_logs': {}, 'diagnostic_contexts': {}}
    for name in ['accepted-actors.log', 'accepted-economy.log', 'accepted-mapworld.log',
                 'accepted-runtime-lib.log', 'accepted-runtime-remaining.log', 'accepted-element-attachment.log']:
        result['selected_logs'][name] = tests(name)
    partial = QA / 'accepted-runtime-integration.log'
    text = partial.read_text(encoding='utf-8-sig')
    assert 'test result: FAILED. 0 passed; 1 failed;' in text
    assert '5705A0 SAI observer lifecycle is not bound to this restore context' in text
    before_failure = sum(int(n) for n in re.findall(r'test result: ok\. (\d+) passed; 0 failed', text))
    assert before_failure == 7
    result['selected_logs'][partial.name] = {'passed': before_failure, 'sha256': sha(partial),
        'qualification': 'Two complete binaries passed7 tests before the stale Element diagnostic assertion failed. Its corrected source-bound context is separately accepted; failure retained.'}
    result['selected_tests_passed'] = sum(v['passed'] for v in result['selected_logs'].values())
    for name in ['rules-compatibility.log', 'metagame-compatibility.log']:
        result['compatibility_logs'][name] = tests(name)
    result['compatibility_export'] = 'disabled: existing native data compared, never rewritten'
    check = QA / 'accepted-workspace-final.log'
    text = check.read_text(encoding='utf-8-sig')
    assert 'Finished `release`' in text and 'error[' not in text and '\nerror:' not in text
    result['workspace_all_targets_compile'] = {'passed': True, 'sha256': sha(check),
        'existing_client_warnings': '13 for binary; 8 duplicate warnings for its test target'}
    result['diagnostic_context_config_tests'] = tests('inventory-context-tests.log')
    result['inventory_comparison_sha256'] = sha(QA / 'inventory-comparison.json')
    result['inventory_manifest_sha256'] = sha(QA / 'inventory-manifest.json')
    for context in ['standalone', 'resources', 'managed']:
        path = QA / f'{context}-mode1-inventory.json'
        doc = read(path)
        assert doc['map_count'] == 80 and doc['native_load_mode'] == 1
        assert doc['binding_context']['context'] == context
        result['diagnostic_contexts'][context] = {
            'sha256': sha(path), 'log': tests(f'{context}-mode1-inventory.log'),
            'inputs': doc['binding_context'], 'maps': 80,
            'maps_without_record_failures': doc['maps_without_failures'],
            'failed_records': sum(len(m['failures']) for m in doc['maps']),
            'converted_records': doc['converted_records'],
            'skipped_records_by_cause': doc['skipped_records_by_cause'],
            'error_groups': [{'count': g['count'], 'error': g['normalized_error']} for g in doc['groups']],
            'post_load_by_outcome': doc['post_load_by_outcome'],
            'qualification': 'Immediate per-record Init/PostCreate remains omitted; no map is a playable-world acceptance.'}
    audit = read(QA / 'inventory-source-drift-audit.json')
    assert not audit['full_source_unchanged'] and audit['production_prefix_unchanged']
    start = ROOT / 'research/skirmish-20261016-saved-economy-integration/census-start-source-reconstructed.rs'
    final = ROOT / audit['path']
    assert sha(start) == audit['start_sha256'] and sha(final) == audit['final_sha256']
    boundary = b'#[cfg(test)]\nmod tests'
    old = start.read_bytes()
    new = final.read_bytes()
    assert boundary in old and boundary in new
    assert old.split(boundary)[0] == new.split(boundary)[0]
    assert hashlib.sha256(old.split(boundary)[0]).hexdigest() == audit['production_prefix_sha256']
    result['inventory_source_drift'] = {'audit': audit,
        'sha256': sha(QA / 'inventory-source-drift-audit.json'),
        'runner_qualification': 'Whole-file source guard rejects the added tests; individual census tests pass. Independently verified production prefix is byte-identical.'}
    baseline = read(ROOT / 'qa-output/resume-20261016/continued-checkpoint-20261016-174134.json')['source_and_evidence_sha256']
    source = {str(p.relative_to(ROOT)).replace('\\', '/'): sha(p)
        for p in (ROOT / 'crates').rglob('*.rs')}
    result['changed_existing_rust_paths'] = sorted(name for name, digest in source.items()
        if name in baseline and baseline[name] != digest)
    result['new_rust_paths'] = sorted(set(source) - set(baseline))
    result['source_sha256'] = source
    for name in ['Cargo.toml', 'Cargo.lock']:
        assert sha(ROOT / name) == baseline[name], f'unexpected manifest change: {name}'
    result['workspace_manifest_and_lock_unchanged'] = True
    result['preserved_failed_attempt'] = {
        'path': 'managed-restore-final.log', 'sha256': sha(QA / 'managed-restore-final.log'),
        'cause': 'Test-only unwrap_err required Debug on a successful owner type; corrected to err().unwrap().',
        'superseded_by': 'managed-restore-accepted.log and accepted-runtime-remaining.log'}
    result['not_claimed'] = ['Complete native per-record Init/PostCreate or registration',
        'Playable skirmish, simulation startup, full conversion replacement drain',
        'LAN or separate-internet multiplayer battle, original save writing compatibility',
        'New release or executable', 'Whole-game completion percentage']
    output = QA / 'acceptance.json'
    with output.open('x', encoding='utf-8') as handle:
        json.dump(result, handle, indent=2)
        handle.write('\n')
    print(json.dumps({'selected_tests_passed': result['selected_tests_passed'],
        'compatibility': result['compatibility_logs'],
        'contexts': {k: {'maps_without_record_failures': v['maps_without_record_failures'],
            'failed_records': v['failed_records']} for k, v in result['diagnostic_contexts'].items()},
        'report_sha256': sha(output)}))

if __name__ == '__main__':
    main()
