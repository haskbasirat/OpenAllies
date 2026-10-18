"""Package only the validated Development 03 executable, caches and notes."""
from pathlib import Path
import hashlib
import json
import re
import shutil
import struct
import zipfile

root = Path(__file__).resolve().parents[1]
qa = root / 'qa-output/phase3'
exe = root / 'target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe'
cache = root / 'assets-cache/animation'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


tests = (qa / 'workspace-tests-final.txt').read_text(encoding='utf-8-sig')
assert 'FAILED' not in tests
assert sum(map(int, re.findall(r'test result: ok\. (\d+) passed;', tests))) == 113
assert not re.search(r'; [1-9]\d* (?:failed|ignored);', tests)
clippy = (qa / 'workspace-clippy-final.txt').read_text(encoding='utf-8-sig')
assert 'Finished' in clippy and not re.search(r'^(?:warning|error)', clippy, re.M)
maps = read(qa / 'maps-indexed-final/report.json')
clips = read(qa / 'animations-indexed-final/report.json')
assert len(maps['maps']) == 80 and all(r['exit_code'] == 0 for r in maps['maps'])
assert len(clips['results']) == 85 and all(r['exit_code'] == 0 for r in clips['results'])
assert len({(r['rig'], r['clip_index']) for r in clips['results']}) == 85
assert maps['executable_sha256'] == clips['executable_sha256'] == sha(exe)
assert read(qa / 'actor-integration-final.json')['exe_sha256'] == sha(exe)
rigs = sorted(cache.glob('*.rig.json'))
assert len(rigs) == 46
assert sum(len(read(path)['clips']) for path in rigs) == 85
original_manifest = read(cache / 'manifest.json')
publication = read(cache / 'promoted-families.json')
assert publication['models'] == 41 and publication['clips'] == 65
approved = {row['file']: row['sha256'] for row in original_manifest['assets']}
approved.update({row['file']: row['sha256'] for row in publication['artifacts'] if row['file'].endswith('.rig.json')})
assert set(approved) == {p.name for p in rigs}
for path in rigs:
    assert sha(path) == approved[path.name]

reviews = [
    'phase3-orders/independent-review.json',
    'phase3-terrain/independent-review.json',
    'phase3-rules/independent-review.json',
    'phase3-animation/independent-review.json',
    'phase3-client/independent-client-review.json',
    'phase3-client/clip-selection-supplemental-review.json',
]
for relative in reviews:
    assert read(root / 'research' / relative)['passed']
data = exe.read_bytes()
pe = struct.unpack_from('<I', data, 0x3c)[0]
assert data[pe:pe + 4] == b'PE\0\0'
assert struct.unpack_from('<H', data, pe + 4)[0] == 0x8664
assert struct.unpack_from('<H', data, pe + 24)[0] == 0x20b
assert b'PerMonitorV2' in data and b'asInvoker' in data
release = root / 'release/Development-03'
release.mkdir(parents=True, exist_ok=False)


def copy(source, relative):
    destination = release / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


copy(exe, exe.name)
copy(root / 'README.txt', 'README.txt')
for name in ['DEVELOPMENT-03', 'VALIDATION-03', 'BASELINE', 'PARITY', 'SPEC', 'RULES', 'WORLD-ORDER']:
    copy(root / f'docs/{name}.txt', f'docs/{name}.txt')
copy(root / 'crates/aa-orders/README.md', 'docs/COMMAND-HISTORY.md')
copy(root / 'research/phase3-rules/README.txt', 'docs/AUTHORED-DEFINITIONS.txt')
copy(root / 'assets-cache/model-materials.json', 'assets-cache/model-materials.json')
for path in rigs:
    copy(path, f'assets-cache/animation/{path.name}')
for name in ['manifest.json', 'promoted-families.json']:
    copy(cache / name, f'assets-cache/animation/{name}')
for relative in reviews:
    copy(root / 'research' / relative, f'evidence/{relative}')
for name in ['animation-catalog.json', 'phase3-evidence.json']:
    copy(root / 'research/phase3-animation' / name, f'evidence/phase3-animation/{name}')
for source, name in [
    ('workspace-tests-final.txt', 'workspace-tests.txt'),
    ('workspace-clippy-final.txt', 'workspace-clippy.txt'),
    ('maps-indexed-final/report.json', 'map-render-report.json'),
    ('animations-indexed-final/report.json', 'animation-render-report.json'),
    ('actor-integration-final.json', 'actor-integration.json'),
    ('economy-integration-final.json', 'economy-integration.json'),
    ('ambiguous-clip-rejection.txt', 'ambiguous-clip-rejection.txt'),
]:
    copy(qa / source, f'evidence/{name}')
copy(qa / 'maps-indexed-final/c8f06979d0a4c712.png', 'Preview-Normandy.png')
tank = next(row for row in clips['results'] if row['rig'] == 'Vehicle_USATankMedium' and row['clip_index'] == 0)
copy(Path(tank['result']['image']), 'Preview-Tank.png')
manifest = {
    'status': 'Development 03; incomplete reconstruction; recovered original assets required',
    'standalone_game': False,
    'architecture': 'AMD64 PE32+',
    'pe_large_address_aware': bool(struct.unpack_from('<H', data, pe + 22)[0] & 0x20),
    'per_monitor_v2': True,
    'tests_passed': 113, 'strict_clippy_passed': True,
    'map_render_smoke_passed': 80, 'animation_model_count': 46,
    'animation_binding_render_smoke_passed': 85,
    'full_game_parity': False, 'files': [],
}
for file in sorted(release.rglob('*')):
    if file.is_file():
        manifest['files'].append({'path': file.relative_to(release).as_posix(), 'bytes': file.stat().st_size, 'sha256': sha(file)})
(release / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
archive_path = release / 'AxisAlliesRust-Development-03.zip'
with zipfile.ZipFile(archive_path, 'x', zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
    for file in sorted(release.rglob('*')):
        if file.is_file() and file != archive_path:
            archive.write(file, file.relative_to(release).as_posix())
with zipfile.ZipFile(archive_path) as archive:
    assert archive.testzip() is None
    for item in manifest['files']:
        assert hashlib.sha256(archive.read(item['path'])).hexdigest() == item['sha256']
    assert archive.read(exe.name) == data
result = {'folder': str(release), 'exe_sha256': sha(exe), 'zip_bytes': archive_path.stat().st_size,
          'zip_sha256': sha(archive_path), 'crc_and_all_payload_hashes_verified': True}
(qa / 'package-report.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
print(json.dumps(result))
