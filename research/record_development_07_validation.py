"""Record this build's actual checks; historical parity ledgers are not release gates."""
from pathlib import Path
import hashlib
import json
import re
import shutil

ROOT = Path(__file__).resolve().parents[1]
QA = ROOT / 'qa-output/build-07'
read = lambda p: json.loads(p.read_text(encoding='utf-8-sig'))
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()


def tests(path):
    text = path.read_text(encoding='utf-8-sig')
    rows = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
    assert rows and 'FAILED' not in text and not re.search(r'^error:', text, re.M)
    counts = [sum(int(row[i]) for row in rows) for i in range(3)]
    assert counts[1] == 0
    return dict(zip(('passed', 'failed', 'ignored'), counts))


staged = read(QA / 'staged.json')
exe_hash = staged['exe_sha256']
exe = ROOT / 'release/Development-07/AxisAlliesRust.exe'
assert sha(exe) == exe_hash
cli = read(QA / 'cli/report.json')
renders = read(QA / 'renders/report.json')
gui = read(QA / 'gui.json')
for report in [cli, renders, gui]:
    assert report['executable_sha256'] == exe_hash and report['passed']
agent = ROOT / 'research/phase7-live-preview'
ledger = read(agent / 'validation-record.json')
for name, expected in ledger['source_files'].items():
    assert sha(ROOT / name) == expected, name
workspace = tests(ROOT / 'qa-output/build-07-workspace-tests.log')
client = tests(agent / 'client-default-tests.log')
hardware = tests(agent / 'focused-test.log')
assert hardware['passed'] == 1 and ledger['clippy_exit'] == 0
for name in ['validation-record.json', 'focused-test.log', 'clippy-final.log', 'client-default-tests.log']:
    shutil.copy2(agent / name, QA / ('preview-' + name))
inputs = [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock']
inputs += sorted((ROOT / 'crates').rglob('*.rs'))
inputs += sorted((ROOT / 'crates').rglob('Cargo.toml'))
(QA / 'build-inputs.json').write_text(json.dumps({p.relative_to(ROOT).as_posix(): sha(p) for p in inputs}, indent=2), encoding='utf-8')
preserved = {
    '../AA.exe': '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26',
    '../reconstruction/native-game/runtime-expanded-v14/AA.exe': '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1',
    'release/Development-06/AxisAlliesRust.exe': '38cbb9d231fe5b0e56847b24101cc59465d86bd317ce609d2feaa5d2189d9a9d',
    'release/Development-06/AxisAlliesRust-Development-06.zip': 'e4aeae97b86f98801d693e93ece37dbc485974ab680746d1bdefb20e7b442aa2'
}
for name, expected in preserved.items():
    assert sha(ROOT / name) == expected, name
evidence = ['qa-output/build-07-workspace-tests.log', 'qa-output/build-07-candidate-build.log']
evidence += ['qa-output/build-07/' + n for n in ['staged.json', 'cli/report.json', 'renders/report.json', 'gui.json', 'build-inputs.json',
             'preview-validation-record.json', 'preview-focused-test.log', 'preview-clippy-final.log', 'preview-client-default-tests.log']]
record = {'passed': True, 'executable_sha256': exe_hash, 'scope': 'Development preview build, not a completed game',
          'workspace_default_before_final_gui_delta': workspace, 'final_client_default': client,
          'preview_source_gpu_test': hardware, 'strict_client_test_target_clippy_passed': True,
          'cli_diagnostics': len(cli['checks']), 'original_map_inputs_decoded': len(cli['maps']),
          'packaged_executable_render_smokes': len(renders['renders']),
          'gui': gui, 'preserved_file_hashes': preserved, 'evidence_files': evidence,
          'limitations': ['No full battles, orders, AI, campaign, original-save loading, or multiplayer mode',
                          'Full original visual parity has not been established',
                          'Hardware/source tests skipped by default were not all rerun',
                          'No new Windows 150% DPI or second-PC/internet validation this build',
                          'Original installation assets remain required outside this ZIP']}
(QA / 'validation.json').write_text(json.dumps(record, indent=2), encoding='utf-8')
(ROOT / 'docs/VALIDATION-07.txt').write_text(
    'DEVELOPMENT 07 VALIDATION\n\nExecutable SHA-256: ' + exe_hash + '\n\n'
    'Passed: workspace default release tests (505 passed, 62 ignored) before the\n'
    'final preview UI changes; final client defaults (43 passed, 15 ignored);\n'
    'one source/GPU integration test spanning five factions and four clips each;\n'
    'strict client test-target Clippy; 85 executable diagnostics including 80\n'
    'original map inputs; six packaged-EXE render smoke tests across Vulkan/DX12.\n'
    'Four invalid CLI inputs were rejected. GUI checks are detailed in\n'
    'evidence/build-07/gui.json. These checks do not establish full game parity.\n\n'
    'The source/GPU test checks pose changes, pause/restart equality, visibility,\n'
    'manual transforms and stable resident allocation size. Render-frame timings\n'
    'are not gameplay FPS benchmarks. Native game and Development06 hashes match.\n\n'
    'Not available: full battles, AI, campaign, original save loading, multiplayer.\n'
    'Not retested: all ignored hardware/source cases, 150% Windows DPI, second-PC\n'
    'or internet play. The ZIP requires external recovered game assets.\n', encoding='utf-8')
print(json.dumps({'passed': True, 'executable_sha256': exe_hash, 'workspace': workspace, 'client': client}))
