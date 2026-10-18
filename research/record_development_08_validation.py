"""Record completed Development08 checks; never infer full gameplay parity."""
from pathlib import Path
import hashlib
import json
import re
import struct

ROOT = Path(__file__).resolve().parents[1]
QA = ROOT / 'qa-output/build-08'
EXE = ROOT / 'release/Development-08/AxisAlliesRust.exe'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()


def tests(path):
    log = path.read_text(encoding='utf-8-sig')
    rows = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', log)
    assert rows and 'FAILED' not in log and not re.search(r'^error:', log, re.M), path
    counts = [sum(int(row[i]) for row in rows) for i in range(3)]
    assert counts[1] == 0
    return dict(zip(('passed', 'failed', 'ignored'), counts))


staged = read(QA / 'staged.json')
exe_hash = sha(EXE)
assert exe_hash == staged['exe_sha256']
cli, renders = [read(QA / p / 'report.json') for p in ['cli', 'renders']]
assert cli['passed'] and renders['passed']
assert cli['executable_sha256'] == renders['executable_sha256'] == exe_hash
ui = tests(QA / 'ui-tests.log')
assert ui == {'passed': 51, 'failed': 0, 'ignored': 0}
for log in [QA / 'build.log', ROOT / 'qa-output/resume-20261016/frozen-workspace-check.log']:
    content = log.read_text(encoding='utf-8-sig')
    assert 'Finished ' in content and not re.search(r'^error:', content, re.M)

images = []
for name in ['menu-initial', 'menu-next-frame', 'menu-fullscreen', 'menu-options',
             'menu-single-player', 'menu-credits', 'menu-editor']:
    path = QA / (name + '.png')
    data = path.read_bytes()
    assert data[:8] == b'\x89PNG\r\n\x1a\n'
    size = struct.unpack_from('>II', data, 16)
    assert size == ((2560, 1440) if name == 'menu-fullscreen' else (1440, 900))
    images.append({'path': path.relative_to(ROOT).as_posix(), 'sha256': sha(path), 'size': size})
assert images[0]['sha256'] != images[1]['sha256']
for name in ['menu-initial', 'menu-options', 'menu-single-player', 'menu-credits', 'menu-editor']:
    log = (QA / (name + '.log')).read_text(encoding='utf-8-sig')
    assert 'saved ' in log and 'screenshot failed' not in log and 'panicked' not in log
gui = {
    'passed': True, 'executable_sha256': exe_hash,
    'method': 'Five scripted eframe input runs, each exited 0; resulting screenshots inspected by the agent',
    'settings': 'Isolated qa-output/build-08/test-profile APPDATA; original/user profiles untouched',
    'observed': ['Main menu and changing movie frames', 'F11 enters 2560x1440 fullscreen and returns',
                 'Options page opens with visible controls', 'Single Player page opens with match buttons disabled',
                 'Credits page displays', 'Editor enters tutorial01 asset viewer with Development08 label'],
    'images': images,
    'limits': ['No actual Windows150% DPI change/test', 'No audio listening validation',
               'No full menu-option persistence or gameplay effect acceptance',
               'Static render/GUI smoke is not visual parity or gameplay FPS acceptance'],
}
(QA / 'gui.json').write_text(json.dumps(gui, indent=2), encoding='utf-8')

preserved = {
    '../AA.exe': '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26',
    '../reconstruction/native-game/runtime-expanded-v14/AA.exe': '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1',
    'release/Development-07/AxisAlliesRust.exe': 'c5bd663998bbbf1840f608a00ab84eea9a7cecd6e8b99e43f0576d9becafe55b',
    'release/Development-07/AxisAlliesRust-Development-07.zip': 'f31514f63cf0d06d70551676b47b0ca78b060211688aa6958272db094fe46ca0',
}
for name, expected in preserved.items():
    assert sha(ROOT / name) == expected, name

evidence = [p.relative_to(ROOT).as_posix() for p in sorted(QA.glob('menu-*')) if p.suffix in ['.png', '.json', '.log']]
evidence += ['qa-output/build-08/' + n for n in ['build.log', 'ui-tests.log', 'cli/report.json', 'renders/report.json', 'gui.json']]
evidence += ['qa-output/resume-20261016/' + n for n in ['frozen-workspace-check.log', 'final-comparison.json',
             'final-inventory-test.log', 'saved-command-ownership.log', 'mind-world-regression.log',
             'name-list-rules.log', 'name-list-metagame.log']]
evidence += ['docs/CONTINUATION-20261016.txt',
             'research/resume-20261016-mind-commands/runtime-integration-review.json',
             'research/resume-20261016-element-attach/independent-review.json']
record = {
    'passed': True, 'executable_sha256': exe_hash,
    'scope': 'Original front-end and asset-preview test build; not a completed game',
    'workspace_all_targets_check_passed': True, 'client_build_warnings': 13,
    'ui_tests': ui, 'cli_diagnostics': 85, 'map_inputs': 80,
    'render_smokes': len(renders['renders']), 'invalid_cli_inputs_rejected': len(renders['invalid_arguments_rejected']),
    'backends_rendered': ['Vulkan', 'Direct3D12'], 'gui': gui,
    'preserved_file_hashes': preserved, 'evidence_files': evidence,
    'limitations': ['No complete battles, AI/campaign, original saves or multiplayer in this EXE',
                    'Native backend libraries are not connected to this client',
                    'ZIP requires existing external recovered original assets',
                    'No second-PC or separate-internet multiplayer tests',
                    'No fresh actual Windows150% DPI test or audio listening test',
                    'Full workspace test suite was not run; scoped tests and all-target compilation were run',
                    'Skinned transparent sphere propagation, shadows/water/lighting and full visual parity remain incomplete'],
}
(QA / 'validation.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
(ROOT / 'docs/VALIDATION-08.txt').write_text(
    'DEVELOPMENT 08 VALIDATION -- 2026-10-16\n\nExecutable SHA256: ' + exe_hash + '\n\n'
    'Passed: Windows x64 release build; all-workspace/all-target compilation;\n'
    '51 aa-ui tests; 85 packaged-EXE diagnostics including all80 map inputs;\n'
    'six GPU render smoke cases across Vulkan and Direct3D12; four rejected\n'
    'invalid command lines. The release build retains13 dead-code warnings.\n\n'
    'Five GUI runs exited normally. Screenshots reviewed: original main menu,\n'
    'changing movie frames, F11 fullscreen at2560x1440, options, disabled\n'
    'Single Player match entries, credits, and Editor opening the asset viewer.\n'
    'GUI tests used an isolated APPDATA directory and did not change user settings.\n\n'
    'Native source integration and the80-map restoration census are documented\n'
    'in evidence/docs/CONTINUATION-20261016.txt. These backend systems are not\n'
    'connected as playable game modes in this EXE. Original and v14 AA.exe\n'
    'and Development07 EXE/ZIP SHA256 hashes remain unchanged.\n\n'
    'Not established: complete battles, AI/campaign, original save loading,\n'
    'multiplayer, full original visual parity, or performance improvement.\n'
    'Not retested: actual Windows150% DPI, audio listening, second-PC/internet.\n'
    'The ZIP requires the existing recovered original assets outside this folder.\n'
    'Manifest and evidence pin this exact build and the completed checks.\n', encoding='utf-8')
print(json.dumps({'passed': True, 'executable_sha256': exe_hash, 'ui': ui, 'renders': len(renders['renders'])}))
