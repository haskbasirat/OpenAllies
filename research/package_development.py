"""Package only the separately identified development executable and its notes."""
from pathlib import Path
import hashlib
import json
import re
import shutil
import struct
import zipfile

root = Path(__file__).resolve().parents[1]
exe = root / 'target/release/AxisAlliesRust.exe'
tests = (root / 'qa-output/workspace-tests.txt').read_text(encoding='utf-8-sig')
assert 'FAILED' not in tests
assert sum(map(int, re.findall(r'test result: ok\. (\d+) passed;', tests))) == 62
matrix = json.loads((root / 'qa-output/all-maps-vulkan/report.json').read_text())
assert len(matrix['maps']) == 80 and all(m.get('exit_code') == 0 for m in matrix['maps'])
data = exe.read_bytes()
pe = struct.unpack_from('<I', data, 0x3c)[0]
assert struct.unpack_from('<H', data, pe+4)[0] == 0x8664
assert struct.unpack_from('<H', data, pe+24)[0] == 0x20b
assert b'PerMonitorV2' in data and b'asInvoker' in data
release = root / 'release/Development-01'
release.mkdir(parents=True, exist_ok=False)
shutil.copy2(exe, release / exe.name)
shutil.copy2(root / 'README.txt', release / 'README.txt')
with (release / 'README.txt').open('a', encoding='utf-8') as f:
    f.write('\nThis archive contains a development executable and notes only. The recovered\n'
            'original assets and source workspace are NOT bundled. On the development PC,\n'
            'leave this folder below rust-rebuild so automatic asset discovery works.\n')
(release / 'docs').mkdir()
for file in sorted((root / 'docs').glob('*.txt')):
    shutil.copy2(file, release / 'docs' / file.name)
shutil.copy2(root / 'qa-output/tutorial-vulkan-4.png', release / 'Preview-Tutorial.png')
manifest = {
    'status': 'development foundation; incomplete game; original assets required',
    'architecture': 'AMD64 PE32+',
    'pe_large_address_aware': bool(struct.unpack_from('<H', data, pe+22)[0] & 0x20),
    'per_monitor_v2': True,
    'tests_passed': 62,
    'map_render_smoke_passed': 80,
    'full_game_parity': False,
    'files': [],
}
for file in sorted(release.rglob('*')):
    if file.is_file():
        manifest['files'].append({'path': file.relative_to(release).as_posix(), 'bytes': file.stat().st_size,
                                  'sha256': hashlib.sha256(file.read_bytes()).hexdigest()})
(release / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
zip_path = release / 'AxisAlliesRust-Development-01.zip'
with zipfile.ZipFile(zip_path, 'x', compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
    for file in sorted(release.rglob('*')):
        if file.is_file() and file != zip_path:
            archive.write(file, file.relative_to(release).as_posix())
with zipfile.ZipFile(zip_path) as archive:
    assert archive.testzip() is None
    assert archive.read(exe.name) == data
print(json.dumps({'folder': str(release), 'exe_sha256': hashlib.sha256(data).hexdigest(),
                  'zip_bytes': zip_path.stat().st_size, 'zip_sha256': hashlib.sha256(zip_path.read_bytes()).hexdigest()}))
