"""Pin the final bounded integration sources/evidence and exact checks."""
from pathlib import Path
import hashlib, json, re
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
def entry(path):
    p = ROOT / path
    return {'path': path, 'sha256': hashlib.sha256(p.read_bytes()).hexdigest(), 'bytes': p.stat().st_size}

paths = ['crates/aa-netcode/src/lib.rs', 'crates/aa-netcode/src/launch_assignment.rs',
         'crates/aa-netcode/src/lobby_staging.rs', 'crates/aa-netcode/src/heartbeat.rs',
         'crates/aa-netcode/src/lobby.rs', 'crates/aa-netcode/src/staging.rs',
         'crates/aa-netcode/tests/launch_assignment.rs', 'crates/aa-netcode/tests/lobby_staging.rs',
         'crates/aa-netcode/tests/heartbeat.rs', 'crates/aa-netcode/tests/staging.rs',
         'crates/aa-netcode/tests/join_resume.rs', 'crates/aa-netcode/tests/send_selector.rs',
         'crates/aa-netcode/tests/loopback.rs', 'Cargo.toml', 'Cargo.lock']
evidence = ['native_launch_assignment.py', 'native_staging_assignment.py',
            'native-launch-assignment.json', 'native-staging-assignment.json', 'README.txt']
checks = []
for name in ('validation-assignment.log', 'validation-integration.log', 'validation-heartbeat-cleanup.log', 'validation-clippy-final.log'):
    p = HERE / name
    if not p.exists(): raise RuntimeError('missing final validation: ' + name)
    text = p.read_text(encoding='utf-8-sig')
    if 'validation-clippy' in name:
        assert 'Finished' in text and 'error:' not in text, name
        checks.append({'file': name, 'result': 'passed', 'scope': 'aa-netcode lib and two new integration test targets, --no-deps, -D warnings'})
    else:
        rows = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
        assert rows and 'test result: FAILED' not in text and 'error:' not in text, name
        checks.append({'file': name, 'result': 'passed', 'passed': sum(int(r[0]) for r in rows),
                       'ignored': sum(int(r[2]) for r in rows), 'note': 'runs overlap; do not sum as distinct tests'})
assignment = json.loads((HERE / 'native-launch-assignment.json').read_text())
report = {
    'schema': 1, 'date': '2026-10-16', 'scope': 'Native launch-settings assignment and staging settings-store composition',
    'original_sha256': assignment['original_sha256'], 'v14_sha256': assignment['v14_sha256'],
    'sources': [entry(p) for p in paths],
    'evidence': [entry('research/phase8-net-integration/' + p) for p in evidence],
    'oracle_runs': {'assignment': {'cases_per_image': 32, 'images': ['original', 'v14'], 'copy_seams': False},
                    'staging_with_real_assignment': {'cases': 48, 'image': 'original', 'executed_windows_identical_in_v14': True, 'copy_seams': False}},
    'validation': checks,
    'earlier_strict_clippy_failure': {'file': 'validation-clippy.log', 'reason': 'pre-existing heartbeat.rs:1245 collapsible_if; equivalent let-chain approved and final check rerun'},
    'concurrent_edit_retry': {'file': 'validation-heartbeat-concurrent-edit.log', 'reason': 'aa-runtime catalog trait methods temporarily preceded RestoreDefinitions additions; unrelated shared edit completed before retry'},
    'independent_review': {'agent': 'network_assignment_review', 'result': 'no blocking findings within complete-settings successful-copy contract',
                           'verification': 'reviewer independently regenerated both native harness outputs and compared existing JSON; outputs identical',
                           'caveats': ['safe Rust self-assignment replay uses distinct equal-valued inputs', 'allocation failure / within-copy partial-store parity unproven', 'decoder companion binding remains open']},
    'production_integration': ['SettingsStore supplies native 0x779F00 copies to existing StagingHost',
                               'Snapshot state-transition seam routes through original ChangeToState and real settings store'],
    'unchanged': ['native game and prior releases', 'network wire codecs', 'client/Cargo/runtime integration', 'existing loopback test source'],
    'open': ['wire snapshot reader must reconstruct copy-only companions when kingdom records are replaced/reordered; count validation is insufficient for identity',
             'complete constructor-bound settings state and decoder integration',
             'map readiness/checksum transport and UI/world effects are still required explicit leaves',
             'client has no native runtime/session frame-loop integration',
             'allocation failure and native exception-state equivalence',
             'two-machine/separate-internet multiplayer, full matches, save compatibility']}
(HERE / 'findings.json').write_text(json.dumps(report, indent=2) + '\n')
print('Recorded', len(paths), 'source pins and', len(checks), 'validation records')
