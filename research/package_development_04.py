"""Package validated Development04 into its own new release folder."""
from pathlib import Path
import hashlib
import json
import re
import shutil
import struct
import zipfile

root = Path(__file__).resolve().parents[1]
qa = root / 'qa-output/phase4'
exe = root / 'target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe'
previous = root / 'release/Development-03'
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
read = lambda path: json.loads(path.read_text(encoding='utf-8-sig'))

tests = (qa / 'workspace-tests-final.txt').read_text(encoding='utf-8-sig')
test_count = sum(map(int, re.findall(r'test result: ok\. (\d+) passed;', tests)))
assert test_count >= 140 and 'FAILED' not in tests
assert not re.search(r'; [1-9]\d* (?:failed|ignored);', tests)
clippy = (qa / 'workspace-clippy-final.txt').read_text(encoding='utf-8-sig')
assert 'Finished' in clippy and not re.search(r'^(?:warning|error)', clippy, re.M)
integration = read(qa / 'executable-check/report.json')
assert integration['passed'] and integration['executable_sha256'] == sha(exe)
assert len(integration['render_pairs']) == 6
assert all(row['pixel_file_identical'] for row in integration['render_pairs'])

reviews = [
    'phase4-orders/independent-review.json',
    'phase4-orders/actors-independent-review.json',
    'phase4-orders/body-independent-review.json',
    'phase4-orders/numeric-independent-review.json',
    'phase4-orders/rng-independent-review.json',
    'phase4-trig/lookup-independent-review.json',
    'phase4-trig-angle/independent-review.json',
    'phase4-movement/independent-review.json',
    'phase4-client/independent-review.json',
]
for relative in reviews:
    review = read(root / 'research' / relative)
    assert review['passed'], relative
    for field in ['files', 'source_pins_verified']:
        for path, expected in review.get(field, {}).items():
            assert sha(root / path) == expected, f'reviewed file changed: {path}'
    for field in ['pins', 'pins_verified']:
        for pin in review.get(field, []):
            assert sha(root / pin['path']) == pin['sha256'], f'reviewed file changed: {pin["path"]}'

# Carry forward only reviewed asset caches, verifying every copy against the
# preserved prior release manifest. Do not silently package unreviewed assets.
prior = read(previous / 'manifest.json')
cache_rows = [row for row in prior['files'] if row['path'].startswith('assets-cache/')]
assert len(cache_rows) == 49
for row in cache_rows:
    assert sha(previous / row['path']) == row['sha256']
    assert sha(root / row['path']) == row['sha256']

data = exe.read_bytes()
pe = struct.unpack_from('<I', data, 0x3c)[0]
assert data[pe:pe+4] == b'PE\0\0'
assert struct.unpack_from('<H', data, pe+4)[0] == 0x8664
assert struct.unpack_from('<H', data, pe+24)[0] == 0x20b
assert b'PerMonitorV2' in data and b'asInvoker' in data
release = root / 'release/Development-04'
release.mkdir(parents=True, exist_ok=False)


def copy(source, relative):
    destination = release / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


copy(exe, exe.name)
copy(root / 'README.txt', 'README.txt')
for name in ['DEVELOPMENT-04', 'VALIDATION-04', 'NEXT-ENGINE-STEPS', 'BASELINE', 'PARITY', 'SPEC', 'RULES', 'WORLD-ORDER']:
    copy(root / f'docs/{name}.txt', f'docs/{name}.txt')
for name, suffix in [('aa-actors','md'), ('aa-body','md'), ('aa-movement','md'), ('aa-orders','md'), ('aa-trig','txt')]:
    copy(root / f'crates/{name}/README.{suffix}', f'docs/{name}.{suffix}')
for row in cache_rows:
    copy(previous / row['path'], row['path'])
for relative in reviews:
    copy(root / 'research' / relative, f'evidence/{relative}')
for name in ['workspace-tests-final.txt', 'workspace-clippy-final.txt', 'executable-check/report.json']:
    copy(qa / name, f'evidence/{name}')
copy(root / 'research/phase4-trig-angle/acceptance.json', 'evidence/phase4-trig-angle/acceptance.json')
copy(qa / 'executable-check/corps-current.png', 'Preview-Corps.png')
manifest = {
    'status': 'Development04; incomplete reconstruction; recovered original assets required for renderer',
    'standalone_game': False, 'full_game_parity': False, 'architecture': 'AMD64 PE32+',
    'pe_large_address_aware': bool(struct.unpack_from('<H', data, pe+22)[0] & 0x20),
    'per_monitor_v2': True, 'tests_passed': test_count, 'strict_clippy_passed': True,
    'mechanics_check_requires_assets': False,
    'render_regression_pairs': 6, 'render_regression_pixel_files_identical': True,
    'prior_release_full_render_matrix_rerun': False,
    'animation_models_preserved': 46, 'animation_bindings_preserved': 85,
    'files': [],
}
for file in sorted(release.rglob('*')):
    if file.is_file():
        manifest['files'].append({'path': file.relative_to(release).as_posix(),
                                  'bytes': file.stat().st_size, 'sha256': sha(file)})
(release / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
archive_path = release / 'AxisAlliesRust-Development-04.zip'
with zipfile.ZipFile(archive_path, 'x', zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
    for file in sorted(release.rglob('*')):
        if file.is_file() and file != archive_path:
            archive.write(file, file.relative_to(release).as_posix())
with zipfile.ZipFile(archive_path) as archive:
    assert archive.testzip() is None
    for row in manifest['files']:
        assert hashlib.sha256(archive.read(row['path'])).hexdigest() == row['sha256']
    assert archive.read(exe.name) == data
result = {'folder': str(release), 'exe_sha256': sha(exe), 'zip_bytes': archive_path.stat().st_size,
          'zip_sha256': sha(archive_path), 'crc_and_all_payload_hashes_verified': True}
(qa / 'package-report.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
print(json.dumps(result))
