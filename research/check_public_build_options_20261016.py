"""Validate the bounded continuation, without claiming complete map startup."""
from datetime import datetime, timezone
from pathlib import Path
import hashlib
import importlib.util
import json

ROOT = Path(__file__).resolve().parents[1]
QA = ROOT / 'qa-output/public-build-options-20261016'
REFERENCES = ROOT.parent / 'reconstruction/references/public-rewrite-20261016'
spec = importlib.util.spec_from_file_location('prior_check_helpers', ROOT / 'research/check_skirmish_state_leader_20261016.py')
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)
helper.QA = QA
sha = helper.sha


def accepted(path, field, expected):
    data = json.loads(path.read_text(encoding='utf-8-sig'))
    assert data.get('accepted') is not False, path
    if expected is True:
        assert data.get(field) is True, path
    else:
        assert data.get(field) == expected, path
    assert not data.get('blocking_findings', []), path
    assert not data.get('blockers', []), path
    return {'path': path.relative_to(ROOT).as_posix(), 'sha256': sha(path)}


def reference_manifest():
    checked = []
    for area in ['ai', 'fov', 'graphics', 'networking']:
        folder = REFERENCES / area
        path = folder / 'download-manifest.json'
        data = json.loads(path.read_text(encoding='utf-8-sig'))
        if area == 'graphics':
            rows = [(folder / repo['repository'].rsplit('/', 1)[-1] / row['path'], row)
                    for repo in data['repositories'] for row in repo['files']]
        else:
            rows = [(folder / row.get('relative_path', row.get('file', '')), row)
                    for row in data['files']]
        for current, row in rows:
            assert current.is_file(), current
            assert sha(current) == row['sha256'], current
            assert current.stat().st_size == row['bytes'], current
        checked.append({'area': area, 'download_manifest_sha256': sha(path),
                        'verified_entries': len(rows)})
    result = {
        'schema': 1, 'audit_date_local': '2026-10-16',
        'scope': 'Pinned selected public references; not complete buildable SDKs. No downloaded implementation executed or copied into production Rust.',
        'checked_child_manifests': checked,
        'files_sha256': {path.relative_to(REFERENCES).as_posix(): sha(path)
            for path in sorted(REFERENCES.rglob('*')) if path.is_file()
            and path.name != 'reference-index.json' and '__pycache__' not in path.parts},
    }
    path = REFERENCES / 'reference-index.json'
    path.write_text(json.dumps(result, indent=2) + '\n')
    return {'path': str(path), 'sha256': sha(path), 'areas': checked}


def main():
    before_path = ROOT / 'qa-output/skirmish-state-leader-20261016/managed-mode1-inventory.json'
    after_path = QA / 'managed-mode1-inventory.json'
    before = json.loads(before_path.read_text())
    after = json.loads(after_path.read_text())
    assert sorted(row['map'] for row in before['maps']) == sorted(row['map'] for row in after['maps'])
    for field in ['binding_context', 'mode', 'precision', 'native_load_mode', 'resource_source']:
        assert before[field] == after[field], field
    summaries = {'before': helper.census(before), 'after': helper.census(after)}
    assert summaries['after']['maps'] == 80
    for field in ['completed_immediate_init_maps', 'publication_ready_maps', 'battle_ready_maps']:
        assert summaries['after'][field] == 0, field
    comparison = {
        'schema': 1, 'scope': 'Managed mode1/PC53/clock0 InspectPartial record diagnostic, not a complete initialized or playable world.',
        **summaries,
        'inventory_sha256': {p.relative_to(ROOT).as_posix(): sha(p) for p in [before_path, after_path]},
        'binding_context': after['binding_context'],
        'caveat': 'Later errors can cascade from partial state. Immediate Init/PostCreate, full conversion/publication and live battle services are still required.',
    }
    comparison_path = QA / 'managed-mode1-comparison.json'
    comparison_path.write_text(json.dumps(comparison, indent=2) + '\n')
    check = QA / 'workspace-check-final.log'
    raw = check.read_text(encoding='utf-8-sig')
    assert 'Finished `release`' in raw and 'error: ' not in raw
    reviews = [
        (ROOT / 'research/skirmish-20261016-builder-data/independent-review.json',
         'status', 'accepted_bounded_source_projection'),
        (ROOT / 'research/skirmish-20261016-saved-actor-properties/independent-core-review.json',
         'result', 'accepted_borrowed_list_core'),
        (ROOT / 'research/skirmish-20261016-saved-actor-properties/independent-review.json',
         'result', 'accepted_bounded_saved_build_targets_and_owner_economy_dispatch'),
        (ROOT / 'research/skirmish-20261016-combat-source-context/independent-review.json',
         'status', 'accepted'),
        (ROOT / 'research/public-references-20261016/graphics/independent-review.json',
         'accepted', True),
    ]
    result = {
        'schema': 1, 'audit_date_local': '2026-10-16',
        'created_utc': datetime.now(timezone.utc).isoformat(),
        'scoped_validation_passed': True, 'new_executable': False,
        'playable_skirmish': False, 'playable_multiplayer': False,
        'priority': ['skirmish', 'multiplayer', 'campaign'],
        'workspace_check': {'path': check.relative_to(ROOT).as_posix(), 'sha256': sha(check),
                            'passed': True, 'warnings': 'Existing client warnings remain; no strict whole-workspace clean claim.'},
        'root_tests': [helper.tests(name) for name in ['actor-storage-organization.log',
            'runtime-regressions-final.log', 'rules-compatibility.log', 'metagame-compatibility.log',
            'managed-mode1-census.log']],
        'independent_reviews': [accepted(*review) for review in reviews],
        'references': reference_manifest(),
        'map_comparison': {'path': comparison_path.relative_to(ROOT).as_posix(),
                           'sha256': sha(comparison_path), **summaries},
        'source_sha256': {path.relative_to(ROOT).as_posix(): sha(path)
            for path in sorted((ROOT / 'crates').rglob('*')) if path.is_file()
            and path.suffix in ['.rs', '.toml'] and 'target' not in path.parts},
        'converter_sha256': sha(ROOT.parent / 'reconstruction/tools/convert_models.py'),
        'context': {'AA_GAME_ROOT': str(ROOT.parent), 'AA_METAGAME_EXPORT': '0 (disabled)',
            'flags': '--release --locked --offline -j4; selected real-data ignored tests included'},
        'limitations': ['Native proofs are bounded, with explicit seams and error prefixes.',
            'Builder projection does not certify all global deferred callbacks.',
            'FP source context is supplied diagnostic input, not observed native startup.',
            'No existing converted assets were regenerated; GPU samplers do not consume retained metadata yet.',
            'No downloaded source/DLL execution, account changes or public hosting occurred.',
            'No two-PC or separate-internet tests; no complete battle or native-save acceptance.',
            'Development09 remains the unchanged earlier EXE and ZIP.'],
        'retained_superseded_attempts': [
            'Old first-error assertion in element-observe.log; all eight historical membership-prefix checks retained.',
            'Builder synthetic fixtures initially omitted required list-template binding; assertions were retained.',
            'Initial Builder kind2-only producer corrected after native-derived RandomActor review.',
            'Owner-Economy initial test selected a nonexistent Organization+Economy combination; final test uses actual absent-Economy owners.',
            'Old context compile/private-method and CSV filename-collision attempts retained.',
            'runtime-regressions.log linked the earlier context library against the later empty-assignment regression while authors finalized it; final whole selection rerun after all source/test freezes.',
            'User-file restricted flags3 discovered and corrected with native proof before final acceptance.'],
    }
    path = QA / 'acceptance.json'
    path.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'root_tests': result['root_tests'], **summaries,
                      'references': result['references']['areas']}, indent=2))


if __name__ == '__main__':
    main()
