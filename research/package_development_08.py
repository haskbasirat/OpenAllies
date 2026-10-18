"""Stage/seal Development 08 without replacing any previous release.

Run stage only after building the candidate, then validate the copied executable.
Root supplies qa-output/build-08/validation.json and docs/VALIDATION-08.txt before
seal. Neither phase builds the game or invents validation results.
"""
from pathlib import Path, PurePosixPath
import argparse
import hashlib
import json
import math
import shutil
import struct
import zipfile

ROOT = Path(__file__).resolve().parents[1]
RELEASE = ROOT / 'release/Development-08'
QA = ROOT / 'qa-output/build-08'
EXE = ROOT / 'target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe'
ARCHIVE = RELEASE / 'AxisAlliesRust-Development-08.zip'
PREVIOUS = ROOT / 'release/Development-07'


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')


def relative_path(name):
    path = PurePosixPath(name)
    require(not path.is_absolute() and '..' not in path.parts and ':' not in name
            and '\\' not in name, f'Expected a relative package path: {name}')
    return path


def copy(source, name):
    destination = RELEASE / relative_path(name)
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    require(sha(source) == sha(destination), f'Copy mismatch: {name}')


def record(path, base=ROOT):
    return {'path': path.relative_to(base).as_posix(),
            'bytes': path.stat().st_size, 'sha256': sha(path)}


def source_inputs():
    # This is a source snapshot at staging, not a claim that Cargo attested it.
    paths = [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock']
    paths.extend(path for path in (ROOT / 'crates').rglob('*')
                 if path.is_file() and 'target' not in path.relative_to(ROOT / 'crates').parts)
    return [record(path) for path in sorted(paths)]


def checked_cache_files():
    previous = read(PREVIOUS / 'manifest.json')
    caches = [row for row in previous['files'] if row['path'].startswith('assets-cache/')]
    require(len(caches) == 49, 'Expected all 49 audited Development-07 cache files')
    require(len({row['path'] for row in caches}) == len(caches), 'Duplicate cache path')
    for row in caches:
        source = ROOT / relative_path(row['path'])
        require(source.is_file() and sha(source) == row['sha256'],
                f'Original cache changed or missing: {row["path"]}')

    videos = []
    movie_summary = []
    for family in ['MainMenu', 'MainMenuGlobe']:
        folder = ROOT / 'assets-cache/video' / family
        index = read(folder / 'index.json')
        require(bool(index['movies']), f'Empty movie index: {folder}')
        names = set()
        for movie in index['movies']:
            name = movie['name']
            relative_path(name)
            require('/' not in name and name not in names, f'Invalid/repeated movie name: {name}')
            names.add(name)
            require(isinstance(movie['frames'], int) and movie['frames'] > 0
                    and math.isfinite(movie['fps']) and movie['fps'] > 0,
                    f'Invalid frame count/rate: {name}')
            for frame in range(movie['frames']):
                image = folder / name / f'{frame:04}.jpg'
                require(image.is_file(), f'Missing movie frame: {image}')
                with image.open('rb') as stream:
                    require(stream.read(3) == b'\xff\xd8\xff', f'Invalid JPEG header: {image}')
        rows = [record(path) for path in sorted(folder.rglob('*')) if path.is_file()]
        videos.extend(rows)
        movie_summary.append({'family': family, 'files': len(rows),
                              'bytes': sum(row['bytes'] for row in rows),
                              'movies': index['movies']})
    return caches, videos, movie_summary


def stage():
    require(not RELEASE.exists(), 'Development-08 already exists; stage never overwrites it')
    require(not (QA / 'staged.json').exists(), 'A Development-08 staging record already exists')
    data = EXE.read_bytes()
    pe = struct.unpack_from('<I', data, 0x3c)[0]
    require(data[:2] == b'MZ' and data[pe:pe + 4] == b'PE\0\0', 'Not a Windows PE executable')
    require(struct.unpack_from('<H', data, pe + 4)[0] == 0x8664
            and struct.unpack_from('<H', data, pe + 24)[0] == 0x20b, 'Expected AMD64 PE32+')
    require(b'PerMonitorV2' in data and b'asInvoker' in data, 'Expected DPI/privilege manifest')
    caches, videos, movie_summary = checked_cache_files()
    require((ROOT / 'docs/DEVELOPMENT-08.txt').is_file(), 'Development-08 README is missing')
    inputs = source_inputs()
    preserved = [record(PREVIOUS / name) for name in
                 ['AxisAlliesRust.exe', 'AxisAlliesRust-Development-07.zip', 'manifest.json']]
    candidate_sha = hashlib.sha256(data).hexdigest()

    RELEASE.mkdir(parents=True, exist_ok=False)
    QA.mkdir(parents=True, exist_ok=True)
    copy(EXE, EXE.name)
    copy(ROOT / 'docs/DEVELOPMENT-08.txt', 'README.txt')
    for row in caches + videos:
        copy(ROOT / row['path'], row['path'])
    for name, arguments in [('Start Asset Viewer.cmd', '--viewer'),
                            ('Start Live Preview.cmd', '--live-preview usa_infantry')]:
        (RELEASE / name).write_text(
            '@echo off\ncd /d "%~dp0"\n"%~dp0AxisAlliesRust.exe" ' + arguments + '\n',
            encoding='ascii', newline='\r\n')
    require(sha(EXE) == sha(RELEASE / EXE.name) == candidate_sha,
            'Candidate changed while staging')
    write_json(QA / 'source-snapshot.json', {
        'scope': 'Source files present at staging; no independent Cargo build attestation',
        'files': inputs,
    })
    write_json(QA / 'staged.json', {
        'exe_sha256': candidate_sha, 'architecture': 'AMD64 PE32+',
        'per_monitor_v2': True,
        'large_address_aware': bool(struct.unpack_from('<H', data, pe + 22)[0] & 0x20),
        'cache_files': len(caches), 'movie_files': len(videos), 'movies': movie_summary,
        'folder': str(RELEASE), 'preserved_files': preserved,
        'source_snapshot_sha256': sha(QA / 'source-snapshot.json'),
        'staged_files': [record(path, RELEASE) for path in sorted(RELEASE.rglob('*')) if path.is_file()],
    })
    print(str(RELEASE))


def seal():
    require(RELEASE.is_dir(), 'Stage Development-08 first')
    require(not ARCHIVE.exists() and not (RELEASE / 'manifest.json').exists(),
            'Package already sealed or has a manifest; refusing to overwrite')
    staged = read(QA / 'staged.json')
    validation = read(QA / 'validation.json')
    require(validation.get('passed') is True, 'Validation did not pass')
    expected = staged['exe_sha256']
    require(sha(EXE) == sha(RELEASE / EXE.name) == expected, 'Candidate/copied executable changed')
    keys = [key for key in ['exe_sha256', 'executable_sha256'] if key in validation]
    require(bool(keys) and all(validation[key] == expected for key in keys),
            'Validation refers to a different executable')
    require(sha(QA / 'source-snapshot.json') == staged['source_snapshot_sha256'],
            'Source staging record changed')
    require(read(QA / 'source-snapshot.json')['files'] == source_inputs(),
            'Source files changed after staging; rebuild/revalidate before packaging')
    for row in staged['staged_files']:
        require(sha(RELEASE / row['path']) == row['sha256'], f'Staged payload changed: {row["path"]}')
    for row in staged['preserved_files']:
        require(sha(ROOT / row['path']) == row['sha256'], f'Previous release changed: {row["path"]}')
    require((ROOT / 'docs/VALIDATION-08.txt').is_file(), 'Development-08 validation notes missing')
    evidence = validation.get('evidence_files')
    require(isinstance(evidence, list) and bool(evidence), 'Validation must list actual evidence files')
    for name in evidence:
        source = ROOT / relative_path(name)
        require(source.is_file(), f'Missing validation evidence: {name}')
        require(source.resolve().is_relative_to(ROOT), f'Evidence outside workspace: {name}')

    copy(QA / 'validation.json', 'evidence/validation.json')
    copy(QA / 'staged.json', 'evidence/staged.json')
    copy(QA / 'source-snapshot.json', 'evidence/source-snapshot.json')
    for name in evidence:
        copy(ROOT / name, 'evidence/' + name)
    copy(ROOT / 'docs/VALIDATION-08.txt', 'VALIDATION.txt')
    manifest = {
        'status': 'Development08: original front end and asset preview; unfinished native reconstruction',
        'standalone_game': False, 'full_game_parity': False, 'playable_complete_match': False,
        'native_runtime_connected_to_client': False,
        'architecture': staged['architecture'], 'per_monitor_v2': True,
        'large_address_aware': staged['large_address_aware'],
        'default_mode': 'Original TGI front end; gameplay launch requests remain disabled',
        'viewer_modes': ['Maps', 'Models', 'Animations', 'Five-faction isolated infantry Live preview'],
        'external_assets_required': [
            'Original Data.rwd alongside reconstruction',
            'reconstruction/game/data/maps/index.json and recovered map/model/texture/animation assets',
            'reconstruction/assets/original/Data including UI/localization/fonts',
            'reconstruction/native-game/runtime-expanded-v14',
            'Recovered Skins/Music/Speech assets for original menu art and audio',
        ],
        'settings_location': '%APPDATA%/AxisAlliesRust/settings.json',
        'validation': 'evidence/validation.json',
        'source_snapshot': 'evidence/source-snapshot.json',
        'files': [],
    }
    for path in sorted(RELEASE.rglob('*')):
        if path.is_file():
            require(path.suffix.lower() != '.zip', 'Unexpected ZIP already in release')
            manifest['files'].append(record(path, RELEASE))
    write_json(RELEASE / 'manifest.json', manifest)
    payload = sorted(path for path in RELEASE.rglob('*') if path.is_file())
    expected_files = {path.relative_to(RELEASE).as_posix(): sha(path) for path in payload}
    with zipfile.ZipFile(ARCHIVE, 'x', zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for path in payload:
            archive.write(path, path.relative_to(RELEASE).as_posix())
    with zipfile.ZipFile(ARCHIVE) as archive:
        require(archive.testzip() is None, 'ZIP CRC verification failed')
        require(len(archive.namelist()) == len(expected_files)
                and set(archive.namelist()) == set(expected_files), 'ZIP payload list differs')
        for name, expected_hash in expected_files.items():
            require(hashlib.sha256(archive.read(name)).hexdigest() == expected_hash,
                    f'ZIP payload hash mismatch: {name}')
    result = {'folder': str(RELEASE), 'exe_sha256': expected, 'zip_sha256': sha(ARCHIVE),
              'zip_bytes': ARCHIVE.stat().st_size, 'payload_files': len(expected_files),
              'all_payload_hashes_and_crc_verified': True}
    write_json(QA / 'package-report.json', result)
    print(json.dumps(result))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['stage', 'seal'])
    options = parser.parse_args()
    stage() if options.phase == 'stage' else seal()
