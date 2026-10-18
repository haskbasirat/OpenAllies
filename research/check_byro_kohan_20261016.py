"""Seal scoped source acceptance; never equate parser/tests with a full game."""
from datetime import datetime, timezone
from pathlib import Path
import hashlib
import importlib.util
import json
import zipfile

ROOT = Path(__file__).resolve().parents[1]
QA = ROOT / 'qa-output/byro-kohan-20261016'
spec = importlib.util.spec_from_file_location('checkpoint_helpers', ROOT / 'research/check_skirmish_state_leader_20261016.py')
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)
helper.QA = QA
sha = helper.sha


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def review(name, field, expected):
    path = ROOT / name
    data = read(path)
    assert data[field] == expected, path
    assert not data.get('blocking_findings', []), path
    assert not data.get('blockers', []), path
    pins = data.get('pins', data.get('files', {}))
    if isinstance(pins, dict):
        for relative, value in pins.items():
            digest = value.get('sha256') if isinstance(value, dict) else value
            if isinstance(digest, str) and len(digest) == 64:
                assert sha(ROOT / relative) == digest, relative
    return {'path': name, 'sha256': sha(path), 'accepted': True}


def main():
    before_path = ROOT / 'qa-output/public-build-options-20261016/managed-mode1-inventory.json'
    after_path = QA / 'managed-mode1-inventory.json'
    before, after = read(before_path), read(after_path)
    assert sorted(row['map'] for row in before['maps']) == sorted(row['map'] for row in after['maps'])
    for field in ['binding_context', 'mode', 'precision', 'native_load_mode', 'resource_source']:
        assert before[field] == after[field], field
    summaries = {'before': helper.census(before), 'after': helper.census(after)}
    assert summaries['after']['maps'] == 80
    assert summaries['after']['map_load_errors'] == 0
    assert summaries['after']['failure_boundaries'] == {'4618B0': 577, '5784C0': 219}
    for field in ['completed_immediate_init_maps', 'publication_ready_maps', 'battle_ready_maps']:
        assert summaries['after'][field] == 0, field
    comparison = {'schema': 1, 'scope': 'Managed mode1, PC53, explicit diagnostic clock0, InspectPartial records; not full map startup.',
                  **summaries, 'binding_context': after['binding_context'],
                  'sha256': {str(p.relative_to(ROOT)): sha(p) for p in [before_path, after_path]},
                  'caveat': 'Later failures can cascade from partial state. No immediate Init/PostCreate, publication or playable-battle acceptance.'}
    comparison_path = QA / 'managed-mode1-comparison.json'
    comparison_path.write_text(json.dumps(comparison, indent=2) + '\n')
    element_path = QA / 'element-attach-final.json'
    element = read(element_path)
    assert '5784C0 member added is unported' in element['first_failure']['error']
    assert element['first_failure']['saved_id'] == 4099
    assert element['first_failure']['source_ordinal'] == 434
    assert len(element['attachment_probe_outcomes']) == 8
    assert all(row['native_prefix_membership_compared'] for row in element['attachment_probe_outcomes'])
    check = QA / 'workspace-check-final.log'
    raw = check.read_text(encoding='utf-8-sig')
    assert 'Finished `release`' in raw and 'error: ' not in raw
    reviews = [
        review('research/skirmish-20261016-owner-actions/independent-review.json', 'status', 'accepted_for_bounded_saved_owner_4d1350_integration'),
        review('research/skirmish-20261016-combat-means/independent-review.json', 'status', 'accepted_bounded_borrowed_mean_producer'),
        review('research/kohan-leads-20261016/projection/independent-review.json', 'accepted', True),
        review('research/byroredux-20261016/independent-review/independent-review.json', 'accepted', True),
    ]
    corpus_folder = ROOT / 'research/byroredux-20261016/corpus'
    corpus_pins = read(corpus_folder / 'pins.json')
    for row in corpus_pins['files']:
        assert sha(corpus_folder / row['path']) == row['sha256'], row['path']
    production_pins = read(ROOT / 'research/byroredux-20261016/production-pins.json')
    for row in production_pins:
        assert sha(Path(row['path'])) == row['sha256'], row['path']
    semantic = read(corpus_folder / 'semantic-comparison.json')
    assert semantic['parser_outcomes'] == {'telemetry_clean': 15, 'partial_truncated': 1}
    assert semantic['comparison_counts']['alpha_exact_blocks'] == 95
    alpha = read(ROOT / 'research/byroredux-20261016/alpha-retention-fixtures.json')
    assert alpha['property_cases'] == 95
    inventory = read(corpus_folder / 'original-header-inventory.json')
    for row in inventory['records']:
        assert sha(Path(inventory['source_root']) / row['path']) == row['sha256'], row['path']
    prior = ROOT / 'backups/source-public-build-options-20261017-005510.zip'
    with zipfile.ZipFile(prior) as saved:
        for name in ['Cargo.toml', 'Cargo.lock']:
            assert hashlib.sha256(saved.read(name)).hexdigest() == sha(ROOT / name), name
    result = {
        'schema': 1, 'created_utc': datetime.now(timezone.utc).isoformat(),
        'scoped_validation_passed': True, 'new_executable': False,
        'playable_skirmish': False, 'playable_multiplayer': False,
        'priority': ['skirmish', 'multiplayer', 'campaign'],
        'workspace_check': {'path': str(check.relative_to(ROOT)), 'sha256': sha(check), 'passed': True,
                            'warnings': 'Existing client warnings remain; no whole-workspace warning-free claim.'},
        'root_tests': [helper.tests(name) for name in ['actor-storage-organization-final.log', 'runtime-regressions-final.log', 'managed-mode1-census.log']],
        'additional_test_logs': [helper.tests(str(ROOT / name)) for name in [
            'research/byroredux-20261016/aa-assets-tests.log',
            'research/byroredux-20261016/alpha-conflict-tests.log',
            'research/skirmish-20261016-combat-means/independent-rust-tests.log',
            'research/kohan-leads-20261016/rwd/tests.log']],
        'independent_reviews': reviews,
        'native_scope': {'owner_action_rows': 90, 'combat_mean_rows': 210, 'camera_projection_rows': 1536,
                         'total_original_v14_native_rows': 1836, 'precisions': ['PC24', 'PC53', 'PC64'],
                         'caveat': 'Bounded seams, supported numeric domains and stopped prefixes; not whole-game parity.'},
        'byroredux': {'commit': semantic['upstream_commit'], 'production_parser_replaced': False,
            'use': 'Actual isolated independent NIF decoder; validated optional raw-alpha retention and conflict refusal.',
            'sample_parsed': 16, 'telemetry_clean': 15, 'strict_eof_verified': 14, 'truncated': 1,
            'original_nif_hashes_reverified': len(inventory['records']),
            'semantic_comparison': {'path': str((corpus_folder / 'semantic-comparison.json').relative_to(ROOT)), 'sha256': sha(corpus_folder / 'semantic-comparison.json')},
            'representation_differences': {'path': str((corpus_folder / 'representation-differences.json').relative_to(ROOT)), 'sha256': sha(corpus_folder / 'representation-differences.json')},
            'exact_alpha_properties': 95, 'production_pins': production_pins,
            'legacy_water_footer_unmeasured': True, 'airplane_death_NiVisData': '16 of231 blocks; partial, not accepted as full model'},
        'map_comparison': {'path': str(comparison_path.relative_to(ROOT)), 'sha256': sha(comparison_path), **summaries},
        'element_checkpoint': {'path': str(element_path.relative_to(ROOT)), 'sha256': sha(element_path), 'historical_membership_probes': 8},
        'source_sha256': {str(path.relative_to(ROOT)): sha(path) for path in sorted((ROOT / 'crates').rglob('*'))
                          if path.is_file() and path.suffix in ['.rs', '.toml'] and 'target' not in path.parts},
        'converter_sha256': sha(ROOT.parent / 'reconstruction/tools/convert_models.py'),
        'unchanged_manifests_compared_to': str(prior.relative_to(ROOT)),
        'context': {'AA_GAME_ROOT': str(ROOT.parent), 'AA_METAGAME_EXPORT': '0 (disabled)',
                    'flags': '--release --locked --offline -j4; selected real-data ignored tests included'},
        'limitations': ['Only16 original NIFs parsed;1900 is the header/hash inventory, not parser coverage.',
            'No assets regenerated or shader/default changes; retention/conflict checks are not a visible transparency-fix claim.',
            'No upstream engine, vendor Gamebryo implementation or Kohan gameplay was integrated.',
            'Borrowed means need concrete completed shared graph/source context; ActorData+1E4 is still unavailable.',
            'Camera subroutine is not bound to the client, actual renderer/device, picking, FOV or culling.',
            'Full startup/world publication/live ticks, complete battles/saves and multiplayer remain unfinished.',
            'No two-PC/separate-Internet test, performance benchmark or new EXE; Development09 is unchanged.',
            'No messages sent to developers; official source-access claim does not establish public source availability.'],
        'retained_superseded_attempts': ['Initial actor command named nonexistent native_actor_storage test; corrected selection passes.',
            'Old element first-error assertion recorded the new required5784C0 stop; only that assertion changed.',
            'First runtime selection used a relative AA_ELEMENT_ATTACH_OUT and failed only while writing its report; full selection rerun with absolute path.',
            'Initial isolated cargo metadata offline resolution failure retained; isolated dependencies resolved without changing main workspace.'],
    }
    path = QA / 'acceptance.json'
    path.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'tests': result['root_tests'], **summaries, 'byro_sample': result['byroredux']['sample_parsed'], 'native_rows': 1836}, indent=2))


if __name__ == '__main__':
    main()
