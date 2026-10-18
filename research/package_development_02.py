"""Package the verified Development 02 preview, leaving prior releases intact."""
from pathlib import Path
import hashlib
import json
import re
import shutil
import struct
import zipfile


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


root = Path(__file__).resolve().parents[1]
exe = root / 'target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe'
qa = root / 'qa-output/phase2'
tests = (qa / 'workspace-tests.txt').read_text(encoding='utf-8-sig')
assert 'FAILED' not in tests
assert sum(map(int, re.findall(r'test result: ok\. (\d+) passed;', tests))) == 85
assert not re.search(r'; [1-9]\d* (?:failed|ignored);', tests)
clippy = (qa / 'workspace-clippy.txt').read_text(encoding='utf-8-sig')
assert 'Finished' in clippy and not re.search(r'^(?:warning|error)', clippy, re.M)
maps = json.loads((qa / 'all-maps-final/report.json').read_text())
clips = json.loads((qa / 'animations-final/report.json').read_text())
assert len(maps['maps']) == 80 and all(m['exit_code'] == 0 for m in maps['maps'])
assert len(clips['results']) == 20 and all(m['exit_code'] == 0 for m in clips['results'])
assert maps['executable_sha256'] == clips['executable_sha256'] == sha256(exe)
audit_path = root / 'research/texture-phase2/model-materials-validation.json'
audit = json.loads(audit_path.read_text())
sidecar = root / 'assets-cache/model-materials.json'
assert audit['passed'] and audit['model_count'] == 1847
assert audit['sidecar']['sha256'] == sha256(sidecar)
assert audit['guard_refinement_models_equal_prior_complete_audit']
economy = json.loads((qa / 'economy-integration-final.json').read_text())
assert economy['shortage_money_per_minute'] == 7
assert economy['money_after_60_seconds'] == 1093

data = exe.read_bytes()
pe = struct.unpack_from('<I', data, 0x3c)[0]
assert data[pe:pe + 4] == b'PE\0\0'
assert struct.unpack_from('<H', data, pe + 4)[0] == 0x8664
assert struct.unpack_from('<H', data, pe + 24)[0] == 0x20b
assert b'PerMonitorV2' in data and b'asInvoker' in data

release = root / 'release/Development-02'
release.mkdir(parents=True, exist_ok=False)


def copy(source, relative):
    destination = release / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


copy(exe, exe.name)
copy(root / 'README.txt', 'README.txt')
for name in ['DEVELOPMENT-02', 'VALIDATION-02', 'BASELINE', 'PARITY', 'SPEC', 'RULES', 'WORLD-ORDER']:
    copy(root / f'docs/{name}.txt', f'docs/{name}.txt')
copy(sidecar, 'assets-cache/model-materials.json')
copy(root / 'assets-cache/animation/manifest.json', 'assets-cache/animation/manifest.json')
for country in ['USA', 'GER', 'GBR', 'JPN', 'RUS']:
    relative = f'assets-cache/animation/{country}Infantry.rig.json'
    copy(root / relative, relative)
copy(qa / 'corps-pack-final.png', 'Preview-Corps.png')
copy(qa / 'oil-final-dx12.png', 'Preview-Oil.png')
copy(audit_path, 'evidence/model-materials-validation.json')
for source, name in [
    ('workspace-tests.txt', 'workspace-tests.txt'),
    ('workspace-clippy.txt', 'workspace-clippy.txt'),
    ('all-maps-final/report.json', 'map-render-report.json'),
    ('animations-final/report.json', 'animation-render-report.json'),
    ('economy-integration-final.json', 'economy-integration.json'),
]:
    copy(qa / source, f'evidence/{name}')

manifest = {
    'status': 'Development 02; incomplete game; recovered original assets required',
    'standalone_game': False,
    'architecture': 'AMD64 PE32+',
    'pe_large_address_aware': bool(struct.unpack_from('<H', data, pe + 22)[0] & 0x20),
    'per_monitor_v2': True,
    'tests_passed': 85,
    'strict_clippy_passed': True,
    'map_render_smoke_passed': 80,
    'animation_render_smoke_passed': 20,
    'full_game_parity': False,
    'files': [],
}
for file in sorted(release.rglob('*')):
    if file.is_file():
        manifest['files'].append({
            'path': file.relative_to(release).as_posix(),
            'bytes': file.stat().st_size,
            'sha256': sha256(file),
        })
(release / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
zip_path = release / 'AxisAlliesRust-Development-02.zip'
with zipfile.ZipFile(zip_path, 'x', compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
    for file in sorted(release.rglob('*')):
        if file.is_file() and file != zip_path:
            archive.write(file, file.relative_to(release).as_posix())
with zipfile.ZipFile(zip_path) as archive:
    assert archive.testzip() is None
    for item in manifest['files']:
        assert hashlib.sha256(archive.read(item['path'])).hexdigest() == item['sha256']
    assert archive.read(exe.name) == data
result = {
    'folder': str(release), 'exe_sha256': sha256(exe),
    'zip_bytes': zip_path.stat().st_size, 'zip_sha256': sha256(zip_path),
    'material_sidecar_sha256': sha256(sidecar),
    'archive_crc_and_all_payload_hashes_verified': True,
}
(qa / 'package-report.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
print(json.dumps(result))
