"""Read-only replay of frozen hardware-watch receipts; never runs the tracer/sealer."""
from pathlib import Path
import hashlib, json
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
AREA = ROOT / 'research/skirmish-20261016-load-world-globals'
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def doc(p): return json.loads(p.read_bytes())
expected = ['85e3fcd580f29ce046ac432209ed84e0a64ff6ed3d138d91a01fa2ddbf6f16bc',
            '2f4091d42fafc58a10f6c13a8d22b644764b9d4bba9d21f7a4e425dc67cad1b6']
reference = AREA / 'point-startup-reference.json'
assert sha(reference) == 'd50ac9ddb3d22d77c933068ebedb0d77fe700e24ec3511141f618ad3b7d15700'
ref = doc(reference)
coverage, rows, support_count = set(), [], 0
for index, words in enumerate(([0,1,2,3], [2,3,4,5]), 1):
    folder = AREA / f'capture-0{index}'
    path = folder / 'capture.json'
    assert sha(path) == expected[index-1]
    c = doc(path)
    assert c['original_error'] is None
    assert c['completed_source_return'] and c['completed_session_constructor']
    assert c['cleanup']['wait_result'] == c['cleanup']['exit_code'] == 0
    assert c['cleanup']['exit_process_event_observed'] and not c['cleanup']['errors']
    assert c['folder_redirection_observed'] and c['original_archives_preserved']
    assert c['script_sha256'] == sha(AREA / 'capture_startup_globals.py')
    assert c['set_context_flags'] == 0x10001
    assert c['debug_register_set_context_flags'] == 0x10010
    assert c['probe_integer_control_set_context_flags'] == 0x10003
    assert c['fp_state_never_written_by_debugger']
    assert c['watched_word_indices'] == words
    assert len(c['watch_arms']) == 62
    for arm in c['watch_arms']:
        assert arm['addresses'] == [0xb3a8c4 + 4*w for w in words]
        assert arm['observed_dr7'] & 255 == 0x55
        for slot in range(4):
            assert (arm['observed_dr7'] >> (16 + 4*slot)) & 15 == 0xd
        assert arm['requested_dr7'] & 0xffff00ff == arm['observed_dr7'] & 0xffff00ff
    assert c['observed_native_point_writes'] == []
    assert c['observation_cutoff']['point_words'] == [0]*6
    assert all(row['point_words'] == [0]*6 for row in c['global_captures'])
    probes = c['isolated_post_cutoff_watch_calibration_probes']
    assert len(probes) == 4
    for slot, p in enumerate(probes):
        assert p['index'] == slot and p['address'] == 0xb3a8c4 + 4*words[slot]
        assert p['dr6'] & 15 == 1 << slot
        assert p['eip'] == 0x76fae7 and p['value'] == p['observed_bits'] == 0
    manifest = folder / 'runtime-files.json'
    assert sha(manifest) == ref['captures'][index-1]['runtime_manifest_sha256']
    for f in doc(manifest)['files']:
        assert sha(folder / 'runtime' / f['path']) == f['source_sha256'] == f['copy_sha256']
        support_count += 1
    coverage.update(words)
    rows.append({'capture': str(path.relative_to(ROOT)), 'sha256': sha(path),
                 'armed_threads': 62, 'pre_cutoff_native_point_writes': 0,
                 'separately_labeled_post_cutoff_calibration_probes': 4,
                 'cleanup_successful': True})
assert coverage == set(range(6)) and support_count == 108
assert doc(AREA / 'post-process-inventory.json') == []
result = {'schema': 1, 'accepted': True, 'blocking_findings': [],
          'scope': 'Independent script/read-only receipt review of the bounded v14 Windows hardware observations only; SDK author separately reviews the Rust binder.',
          'reference_sha256': sha(reference), 'captures': rows,
          'verified_copied_support_files': support_count, 'support_files_per_capture': 54,
          'limits': ['Initial PE-zero seed through source completion and Session construction only.',
                     'No first-map, battle, internet, whole-engine or perpetual global-immutability claim.',
                     'Four same-value MOV calibration probes are after the sealed observation cutoff and are not game producers.',
                     'Tracer initial CreateProcess handles are inherited until bounded Python exit; final normal debugger cleanup receipts and empty owned-game process inventory are required.'],
          'review_script_sha256': sha(Path(__file__))}
target = AREA / 'independent-hardware-review.json'
if target.exists():
    assert doc(target) == result
else:
    target.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'accepted': True, 'captures': 2, 'support_files': support_count,
                  'report_sha256': sha(target)}))
