from pathlib import Path
import hashlib
import json
import re

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def pin(path):
    raw = path.read_bytes()
    return {'path': path.relative_to(ROOT).as_posix(), 'bytes': len(raw), 'sha256': hashlib.sha256(raw).hexdigest()}


def test_log(name):
    path = HERE / name
    text = path.read_text()
    results = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed;', text)
    return {**pin(path), 'passed_test_suites': [{'passed': int(a), 'failed': int(b)} for a, b in results],
            'compile_error': 'could not compile' in text}


def main():
    fixture = json.loads((HERE / 'native-element-move.json').read_text())
    final = test_log('final-adapter-tests-after-shared-fix.log')
    assert final['passed_test_suites'] == [{'passed': 2, 'failed': 0}] and not final['compile_error']
    files = ['crates/aa-runtime/src/element_move_command.rs', 'crates/aa-runtime/tests/element_move_command.rs',
             'crates/aa-mind/src/element_command.rs', 'crates/aa-runtime/src/mind_world.rs',
             'research/skirmish-20261016-playable-path/native_element_move.py',
             'research/skirmish-20261016-playable-path/native-element-move.json',
             'research/skirmish-20261016-playable-path/README.md',
             'research/skirmish-20261016-playable-path/write_findings.py']
    findings = {
        'schema': 1, 'status': 'source_frozen_focused_validation_passed',
        'battle_ready': False, 'new_client_battle_owner': False,
        'component': 'actual538340 contextual Element Move binding using existing native core',
        'native_addresses': {'handler': '538340', 'near_point': '52A240', 'stack_head': '52A000',
                             'generation_lookup': '530F10', 'schedule': '529CD0',
                             'fresh_point_activation': '52DF60', 'sleep_activation': '52A810',
                             'kill_activation': '5351C0', 'bombard_activation': '538120', 'follow_activation': '532930'},
        'native_oracle': {'actually_executed': 'original AA machine code under Unicorn',
                          'cases': len(fixture['cases']), 'precisions': ['Single24', 'Double53', 'Extended64'],
                          'original_sha256': fixture['original_sha256'], 'v14_sha256': fixture['v14_sha256'],
                          'v14_proof': 'every reached instruction byte equals v14; not separate full-battle execution',
                          'non_fs_page_zero_reads': len(fixture['page_zero_non_fs_reads']),
                          'effect_seams': fixture['effect_seams']},
        'actual_state_validation': {'source': 'original Data.rwd plus v14 overrides; usa_infantry',
                                    'native_rows_replayed': 123, 'mutation_probes_included': 3,
                                    'real_wakeup_schedule_probes': 6, 'extra_ownership_probes': 6,
                                    'all_are_isolated_state_probes_not_saved_battles': True,
                                    'stores': ['ActorStore/ActorAccess', 'Mind Arena', 'Commands', 'typed Move payload', 'Owners', 'Queues', 'SpatialWorld'],
                                    'no_snapshot_copyback': True, 'no_invented_unknown_fields': True},
        'tests': [test_log('actual-store-replay-v2.log'), test_log('core-replay.log'), final],
        'retained_failure_log': {**test_log('final-adapter-tests.log'),
                                'qualification': 'concurrent live_registration.rs E0283 prevented compilation; owner fixed it, then final rerun passed'},
        'files': [pin(ROOT / name) for name in files],
        'runtime_export': 'pub mod element_move_command;',
        'approved_shared_edits': ['aa-mind element_command granular getters and matching native branch reads',
                                  'mind_world Commands::value_mut only', 'runtime lib export only'],
        'limits': ['No client StartMatch/authorization/decree integration added here.',
                   'No complete map Init/PostLoad/world-start publication or concrete complete clocked/Network services.',
                   'Fresh activation, target acceptance and indirect fire remain required live services; recorder seams are tests only.',
                   'Active Move payload independent reentry during an existing mutable borrow explicitly refuses; no cached bypass.',
                   'No full movement/attack/economy battle, campaign, multiplayer or internet test occurred.',
                   'No completion percentage or finish date can be inferred from leaf counts.'],
        'target_dir': 'target/s-saved-leader', 'author_commands_reaped': True,
        'original_assets_modified': False, 'release_modified': False,
        'root_manifests_modified': False, 'new_external_dependencies': False,
    }
    target = HERE / 'findings.json'
    target.write_text(json.dumps(findings, indent=2) + '\n')
    print(json.dumps({'findings_sha256': hashlib.sha256(target.read_bytes()).hexdigest(), 'source_status': findings['status']}))


if __name__ == '__main__':
    main()
