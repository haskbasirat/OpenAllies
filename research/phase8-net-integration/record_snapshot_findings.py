"""Record the complete-snapshot follow-up independently of earlier checkpoints."""
from pathlib import Path
import hashlib, json, re
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
def pin(path):
    p = ROOT / path
    return {'path': path, 'bytes': p.stat().st_size, 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()}
paths = ['crates/aa-netcode/src/launch_assignment.rs', 'crates/aa-netcode/src/lobby_staging.rs',
         'crates/aa-netcode/src/lobby.rs', 'crates/aa-netcode/src/staging.rs',
         'crates/aa-netcode/tests/snapshot_companions.rs', 'crates/aa-netcode/tests/loopback.rs',
         'research/phase8-net-integration/native_snapshot_companions.py',
         'research/phase8-net-integration/native-snapshot-companions.json',
         'research/phase8-net-integration/README.txt', 'Cargo.toml', 'Cargo.lock']
test_log = ROOT / 'qa-output/resume-20261016/snapshot-companions.log'
clippy_log = ROOT / 'qa-output/resume-20261016/snapshot-companions-clippy.log'
text = test_log.read_text(encoding='utf-8-sig')
counts = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
assert counts and 'error:' not in text and 'test result: FAILED' not in text
clippy = clippy_log.read_text(encoding='utf-8-sig')
assert 'Finished' in clippy and 'error:' not in clippy
fixture = json.loads((HERE / 'native-snapshot-companions.json').read_text())
report = {
    'schema': 1, 'date': '2026-10-16',
    'scope': 'Owned full session snapshot decoding and native retained kingdom reconstruction',
    'backup': pin('backups/native-snapshot-companions-before-4c4cad7c2422465da74deb4f14ac4ccd.zip'),
    'sources_and_evidence': [pin(p) for p in paths],
    'original_sha256': fixture['original_sha256'], 'v14_sha256': fixture['v14_sha256'],
    'native': {'full_snapshots_per_image': 12, 'images': ['original', 'v14'], 'constructor_probes': 2,
               'images_agree': True, 'routines': ['0x76D0B0', '0x77AAC0', '0x77A750', '0x779F00', '0x76EBD0', '0x5104A0', '0x50F4C0', '0x4A4C00'],
               'retained_kingdom_constructor': fixture['constructor'],
               'no_mocked_routines': ['settings assignment', 'state transition', 'wire decoder', 'header/kingdom decoder', 'temporary kingdom constructor']},
    'validation': {'test_log': str(test_log.relative_to(ROOT)).replace('\\', '/'),
                   'passed': sum(int(r[0]) for r in counts), 'ignored': sum(int(r[2]) for r in counts),
                   'clippy': str(clippy_log.relative_to(ROOT)).replace('\\', '/'), 'strict_clippy_passed': True,
                   'udp_scope': 'existing host/client loopback direct and lossy/reordering relay, 127.0.0.1 only'},
    'safety_contract': 'Failed snapshot/staging operations preserve partial Session/Admin/cursor writes but invalidate settings ownership; no retry/copy of stale companions',
    'not_native_failure_parity': ['truncated-stream exception/within-reader partial-store equivalence', 'allocation failure'],
    'untouched': ['wire formats', 'original executables/assets/saves/profiles', 'Development08 and older releases', 'root Cargo configuration', 'existing UDP loopback test source'],
    'remaining': ['canonical native initial session/settings construction',
                  'wire SnapshotOwner into the actual private-message session dispatcher/client',
                  'real map checksum/readiness requests and other external effects',
                  'kingdom setters, restored world publication, match launch and real decree effects',
                  'separate machines/internet multiplayer, unavailable and not claimed']}
(HERE / 'findings-snapshot.json').write_text(json.dumps(report, indent=2) + '\n')
print('Recorded snapshot follow-up:', report['validation']['passed'], 'tests')
