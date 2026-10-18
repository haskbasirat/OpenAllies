"""Record reproducible final validation of this bounded source continuation."""
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
import hashlib
import json
import re

ROOT = Path(__file__).resolve().parents[1]
QA = ROOT / 'qa-output/skirmish-state-leader-20261016'


def sha(path):
    with path.open('rb') as handle:
        return hashlib.file_digest(handle, 'sha256').hexdigest()


def tests(name):
    path = QA / name
    raw = path.read_text(encoding='utf-8-sig')
    assert 'FAILED' not in raw and 'error: ' not in raw, path
    rows = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', raw)
    assert rows, path
    return {'log': str(path.relative_to(ROOT)), 'sha256': sha(path),
            'passed': sum(int(row[0]) for row in rows),
            'failed': sum(int(row[1]) for row in rows),
            'ignored': sum(int(row[2]) for row in rows),
            'test_executables_with_summary': len(rows)}


def census(data):
    boundary = Counter()
    for record in data['maps']:
        for error in record['failures']:
            match = re.search(r'\b[0-9A-F]{6}\b', error['error'])
            boundary[match.group() if match else error['error']] += 1
    return {
        'maps': data['map_count'], 'record_clean_maps': data['maps_without_failures'],
        'map_load_errors': sum(record['load_error'] is not None for record in data['maps']),
        'record_failures': sum(len(record['failures']) for record in data['maps']),
        'failure_boundaries': dict(boundary.most_common()),
        'completed_immediate_init_maps': sum(record['lifecycle_coverage']['immediate_init_complete'] for record in data['maps']),
        'publication_ready_maps': sum(record['lifecycle_coverage']['runtime_publication_ready'] for record in data['maps']),
        'battle_ready_maps': sum(record['lifecycle_coverage']['battle_ready'] for record in data['maps']),
    }


def main():
    before_path = ROOT / 'qa-output/development09/native/managed-mode1-inventory.json'
    after_path = QA / 'managed-mode1-inventory.json'
    before = json.loads(before_path.read_text())
    after = json.loads(after_path.read_text())
    assert sorted(row['map'] for row in before['maps']) == sorted(row['map'] for row in after['maps'])
    for field in ['binding_context', 'mode', 'precision', 'native_load_mode']:
        assert before[field] == after[field], field
    summaries = {'before': census(before), 'after': census(after)}
    clean_before = {row['map'] for row in before['maps'] if not row['failures'] and row['load_error'] is None}
    clean_after = {row['map'] for row in after['maps'] if not row['failures'] and row['load_error'] is None}
    assert len(clean_before) == summaries['before']['record_clean_maps']
    assert len(clean_after) == summaries['after']['record_clean_maps']
    assert summaries['after']['maps'] == 80
    assert summaries['after']['completed_immediate_init_maps'] == 0
    assert summaries['after']['publication_ready_maps'] == 0
    assert summaries['after']['battle_ready_maps'] == 0
    comparison = {
        'schema': 1, 'scope': 'Managed mode1/PC53/clock0 InspectPartial record diagnostic; NOT complete native map initialization or battle acceptance.',
        'inputs': {str(p.relative_to(ROOT)): sha(p) for p in [before_path, after_path]},
        'binding_context': after['binding_context'], **summaries,
        'clean_maps_added': sorted(clean_after - clean_before),
        'clean_maps_removed': sorted(clean_before - clean_after),
        'caveat': 'Later errors can be cascades of earlier partial state. Raw source-pinned inventories retain the exact errors and record identities.',
    }
    comparison_path = QA / 'managed-mode1-comparison.json'
    comparison_path.write_text(json.dumps(comparison, indent=2) + '\n')
    check_path = QA / 'workspace-check-final.log'
    check_text = check_path.read_text(encoding='utf-8-sig')
    assert 'Finished `release`' in check_text and 'error: ' not in check_text
    reviews = [
        'saved-actor-state', 'saved-leader', 'saved-organization-position',
        'loaded-placement', 'settlement-completion',
    ]
    evidence = {}
    for area in reviews:
        for name in ['findings.json', 'independent-review.json']:
            path = ROOT / 'research' / f'skirmish-20261016-{area}' / name
            report = json.loads(path.read_text(encoding='utf-8-sig'))
            if name == 'independent-review.json':
                if area in ['saved-actor-state', 'saved-organization-position']:
                    assert report['result'] == 'accepted_bounded_restore_adapter', path
                else:
                    assert report['accepted'] is True, path
                assert not report.get('blocking_findings', []), path
            evidence[str(path.relative_to(ROOT))] = sha(path)
    for area in ['borrowed-leader', 'startup-critical-path']:
        path = ROOT / 'research' / f'skirmish-20261016-{area}' / 'findings.json'
        json.loads(path.read_text(encoding='utf-8-sig'))
        evidence[str(path.relative_to(ROOT))] = sha(path)
    sources = [
        'crates/aa-mapworld/src/restore.rs', 'crates/aa-mapworld/src/organization_restore.rs',
        'crates/aa-runtime/src/catalog.rs', 'crates/aa-runtime/src/lib.rs',
        'crates/aa-runtime/src/saved_actor_state.rs',
        'crates/aa-runtime/src/saved_organization_leader.rs',
        'crates/aa-runtime/src/saved_organization_position.rs',
        'crates/aa-runtime/tests/element_attach_restore.rs',
        'crates/aa-movement/src/lib.rs', 'crates/aa-movement/src/borrowed_formation.rs',
        'crates/aa-rules/src/lib.rs', 'crates/aa-rules/src/settlement_completion.rs',
        'crates/aa-spatial/src/lib.rs', 'crates/aa-spatial/src/loaded_object_placement.rs',
    ]
    acceptance = {
        'schema': 1, 'created_utc': datetime.now(timezone.utc).isoformat(),
        'scoped_validation_passed': True, 'new_release_created': False,
        'playable_skirmish': False, 'playable_multiplayer': False,
        'priority': ['skirmish', 'multiplayer', 'campaign'],
        'scope': 'Bounded native source integration after Development09. Source tests do not validate the unchanged Development09 executable.',
        'workspace_check': {'log': str(check_path.relative_to(ROOT)), 'sha256': sha(check_path),
                            'result': 'passed; existing client warnings remain'},
        'tests': [tests(name) for name in ['runtime-regressions-final.log',
            'rules-compatibility.log', 'metagame-compatibility.log', 'managed-mode1-census.log']],
        'real_data_context': {'AA_GAME_ROOT': str(ROOT.parent), 'AA_METAGAME_EXPORT': '0 (exports disabled)',
            'flags': '--release --locked --offline -j4; real-data ignored tests explicitly included'},
        'map_comparison': {'path': str(comparison_path.relative_to(ROOT)), 'sha256': sha(comparison_path), **summaries},
        'evidence_sha256': evidence,
        'source_sha256': {name: sha(ROOT / name) for name in sources},
        'retained_superseded_attempts': {
            'element-observe.log': 'Old 4D64F0 checkpoint expectation; observed next 57C800 boundary.',
            'element-position-observe.log': 'Old 57C800 checkpoint expectation; observed next 4D4CF0 boundary. Only expected first error changed; all original membership-prefix checks retained.',
            'runtime-regressions.log': '31 library passes and one invalid fixture-selection assumption requiring an actual Mind-bearing Organization. Final tests separate actual no-Mind contexts from labeled native probes; no fake Mind success.'},
        'remaining': ['completed canonical ActorData combat values and real source context',
            'required changed-state effects and concrete Mind callbacks',
            'full 4F3640 placement and real world services',
            'immediate per-record Init/PostCreate, conversions, mission startup and publication',
            'deterministic live ticks and complete human-versus-AI match',
            'multiplayer integration; separate-PC/internet acceptance currently unavailable',
            'campaign and WWII progression'],
        'schedule': 'No reliable percentage or completion date. Multi-month scope is a planning assessment, not a delivery commitment.',
    }
    (QA / 'acceptance.json').write_text(json.dumps(acceptance, indent=2) + '\n')
    print(json.dumps({'tests': acceptance['tests'], **summaries}, indent=2))


if __name__ == '__main__':
    main()
