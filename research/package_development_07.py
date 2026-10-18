"""Stage and seal the bounded Development-07 preview build; preserve old releases."""
from pathlib import Path
import argparse
import hashlib
import json
import shutil
import struct
import zipfile

ROOT = Path(__file__).resolve().parents[1]
RELEASE = ROOT / 'release/Development-07'
QA = ROOT / 'qa-output/build-07'
EXE = ROOT / 'target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def copy(source, name):
    destination = RELEASE / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def stage():
    data = EXE.read_bytes()
    pe = struct.unpack_from('<I', data, 0x3c)[0]
    assert data[pe:pe + 4] == b'PE\0\0'
    assert struct.unpack_from('<H', data, pe + 4)[0] == 0x8664
    assert struct.unpack_from('<H', data, pe + 24)[0] == 0x20b
    assert b'PerMonitorV2' in data and b'asInvoker' in data
    previous = read(ROOT / 'release/Development-06/manifest.json')
    caches = [row for row in previous['files'] if row['path'].startswith('assets-cache/')]
    assert len(caches) == 49
    for row in caches:
        assert sha(ROOT / row['path']) == row['sha256'], row['path']
    RELEASE.mkdir(parents=True, exist_ok=False)
    copy(EXE, EXE.name)
    copy(ROOT / 'docs/DEVELOPMENT-07.txt', 'README.txt')
    for row in caches:
        copy(ROOT / row['path'], row['path'])
    (RELEASE / 'Start Live Preview.cmd').write_text(
        '@echo off\ncd /d "%~dp0"\n"%~dp0AxisAlliesRust.exe" --live-preview usa_infantry\n',
        encoding='ascii', newline='\r\n')
    QA.mkdir(parents=True, exist_ok=True)
    (QA / 'staged.json').write_text(json.dumps({
        'exe_sha256': sha(EXE), 'architecture': 'AMD64 PE32+',
        'per_monitor_v2': True,
        'large_address_aware': bool(struct.unpack_from('<H', data, pe + 22)[0] & 0x20),
        'cache_files': len(caches), 'folder': str(RELEASE)
    }, indent=2), encoding='utf-8')
    print(str(RELEASE))


def seal():
    staged = read(QA / 'staged.json')
    validation = read(QA / 'validation.json')
    assert validation['passed']
    assert sha(EXE) == sha(RELEASE / EXE.name) == staged['exe_sha256']
    assert validation['executable_sha256'] == staged['exe_sha256']
    copy(QA / 'validation.json', 'evidence/validation.json')
    for relative in validation['evidence_files']:
        source = ROOT / relative
        assert source.is_file(), relative
        copy(source, 'evidence/' + source.relative_to(ROOT / 'qa-output').as_posix())
    copy(ROOT / 'docs/VALIDATION-07.txt', 'VALIDATION.txt')
    manifest = {
        'status': 'Development07: unfinished reconstruction and isolated live preview',
        'standalone_game': False, 'full_game_parity': False, 'playable_complete_match': False,
        'architecture': staged['architecture'], 'per_monitor_v2': True,
        'large_address_aware': staged['large_address_aware'],
        'new_user_feature': 'Five-faction isolated infantry preview with four original clips',
        'external_assets_required': ['../Data.rwd relative to reconstruction',
                                     'reconstruction including recovered assets and runtime-expanded-v14'],
        'validation': 'evidence/validation.json', 'files': []
    }
    for path in sorted(RELEASE.rglob('*')):
        if path.is_file():
            assert path.suffix.lower() != '.zip', 'Package already sealed'
            manifest['files'].append({'path': path.relative_to(RELEASE).as_posix(),
                                      'bytes': path.stat().st_size, 'sha256': sha(path)})
    (RELEASE / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    archive = RELEASE / 'AxisAlliesRust-Development-07.zip'
    with zipfile.ZipFile(archive, 'x', zipfile.ZIP_DEFLATED, compresslevel=6) as z:
        for path in sorted(RELEASE.rglob('*')):
            if path.is_file() and path != archive:
                z.write(path, path.relative_to(RELEASE).as_posix())
    with zipfile.ZipFile(archive) as z:
        assert z.testzip() is None
        for row in manifest['files']:
            assert hashlib.sha256(z.read(row['path'])).hexdigest() == row['sha256']
        assert hashlib.sha256(z.read(EXE.name)).hexdigest() == staged['exe_sha256']
    result = {'folder': str(RELEASE), 'exe_sha256': sha(EXE), 'zip_sha256': sha(archive),
              'zip_bytes': archive.stat().st_size, 'all_payload_hashes_and_crc_verified': True}
    (QA / 'package-report.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
    print(json.dumps(result))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('phase', choices=['stage', 'seal'])
    options = parser.parse_args()
    stage() if options.phase == 'stage' else seal()
