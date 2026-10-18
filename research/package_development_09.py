"""Prepare Development-09 once, then seal only its explicitly validated payload.

No build, executable launch, original-asset edit, or previous-release mutation
is performed. Root must supply the candidate path/hash and approved documents.
Staging and sealing refuse to overwrite existing release/evidence products.
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
RELEASE = ROOT / 'release/Development-09'
PREVIOUS = ROOT / 'release/Development-08'
QA = ROOT / 'qa-output/development09'
ARCHIVE_NAME = 'AxisAlliesRust-Development-09.zip'
EXE_NAME = 'AxisAlliesRust.exe'
ARCHIVE = RELEASE / ARCHIVE_NAME


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(Path(path).read_text(encoding='utf-8-sig'))


def write_json(path, value):
    path = Path(path)
    require(not path.exists(), f'Refusing to replace evidence: {path}')
    path.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')


def relative_path(name):
    require(isinstance(name, str) and bool(name), 'Empty/non-string relative path')
    path = PurePosixPath(name)
    require(not path.is_absolute() and '..' not in path.parts and '.' not in path.parts
            and ':' not in name and '\\' not in name and '\x00' not in name
            and str(path) == name and name != '.', f'Unsafe relative path: {name!r}')
    return path


def workspace_path(name):
    path = (ROOT / relative_path(name)).resolve()
    require(path.is_relative_to(ROOT.resolve()) and path.is_file(),
            f'Missing/outside workspace input: {name}')
    return path


def record(path, base=ROOT):
    path = Path(path)
    return {'path': path.relative_to(base).as_posix(),
            'bytes': path.stat().st_size, 'sha256': sha(path)}


def verified_rows(base, rows):
    require(isinstance(rows, list), 'Expected file-record list')
    names = set()
    folded = set()
    for row in rows:
        name = str(relative_path(row['path']))
        require(name not in names and name.casefold() not in folded,
                f'Duplicate/case-colliding path: {name}')
        names.add(name)
        folded.add(name.casefold())
        path = base / name
        require(path.resolve().is_relative_to(base.resolve()) and path.is_file(),
                f'Missing/outside payload: {name}')
        require(path.stat().st_size == row['bytes'] and sha(path) == row['sha256'],
                f'Changed payload: {name}')
    return names


def source_inputs():
    paths = [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock']
    paths.extend(path for path in (ROOT / 'crates').rglob('*')
                 if path.is_file() and 'target' not in path.relative_to(ROOT / 'crates').parts)
    return [record(path) for path in sorted(paths)]


def inspect_pe(path, expected_hash):
    require(isinstance(expected_hash, str) and len(expected_hash) == 64
            and all(c in '0123456789abcdef' for c in expected_hash), 'Expected lowercase SHA256')
    data = path.read_bytes()
    require(hashlib.sha256(data).hexdigest() == expected_hash, 'Candidate hash differs from approved hash')
    require(len(data) >= 64 and data[:2] == b'MZ', 'Not a Windows executable')
    offset = struct.unpack_from('<I', data, 0x3c)[0]
    require(offset <= len(data) - 26 and data[offset:offset+4] == b'PE\0\0', 'Invalid PE header')
    require(struct.unpack_from('<H', data, offset+4)[0] == 0x8664
            and struct.unpack_from('<H', data, offset+24)[0] == 0x20b, 'Expected AMD64 PE32+')
    require(struct.unpack_from('<H',data,offset+22)[0] & 0x20 != 0,
            'Expected large-address-aware Windows x64 executable')
    require(b'PerMonitorV2' in data and b'asInvoker' in data, 'Expected DPI/privilege manifest markers')
    return {'architecture': 'AMD64 PE32+', 'per_monitor_v2_manifest_marker': True,
            'as_invoker_manifest_marker': True,
            'large_address_aware': bool(struct.unpack_from('<H', data, offset+22)[0] & 0x20)}


def checked_document(name):
    path = workspace_path(name)
    text = path.read_text(encoding='utf-8-sig')
    require(text.strip() and '{{' not in text and '<FILL_' not in text,
            f'Incomplete document template: {name}')
    return path


def previous_payload():
    manifest = read(PREVIOUS / 'manifest.json')
    verified_rows(PREVIOUS, manifest['files'])
    previous_files = [record(path) for path in sorted(PREVIOUS.rglob('*')) if path.is_file()]
    require(any(row['path'].endswith('AxisAlliesRust-Development-08.zip') for row in previous_files),
            'The immutable Development-08 archive is missing')
    caches = [row for row in manifest['files'] if row['path'].startswith('assets-cache/')]
    nonvideo = [row for row in caches if not row['path'].startswith('assets-cache/video/')]
    require(len(nonvideo) == 49, 'Expected Development-08\'s 49 audited non-video cache files')
    verified_rows(ROOT, caches)
    movies = []
    cache_names = {row['path'] for row in caches}
    for family in ['MainMenu', 'MainMenuGlobe']:
        folder = f'assets-cache/video/{family}'
        require(folder + '/index.json' in cache_names, 'Missing audited movie index')
        index = read(ROOT / folder / 'index.json')
        require(bool(index.get('movies')), 'Empty movie index')
        names = set()
        for movie in index['movies']:
            name = str(relative_path(movie['name']))
            require('/' not in name and name.casefold() not in names, 'Invalid movie directory')
            names.add(name.casefold())
            require(type(movie['frames']) is int and movie['frames'] > 0
                    and math.isfinite(movie['fps']) and movie['fps'] > 0, 'Invalid movie frame count/rate')
            for number in range(movie['frames']):
                name = f'{folder}/{movie["name"]}/{number:04}.jpg'
                require(name in cache_names, f'Movie frame is not in audited payload: {name}')
                with (ROOT / name).open('rb') as stream:
                    require(stream.read(3) == b'\xff\xd8\xff', f'Invalid JPEG header: {name}')
        movies.append({'family': family, 'movies': index['movies']})
    return caches, movies, previous_files


def copy(source, name):
    destination = RELEASE / relative_path(name)
    require(not destination.exists(), f'Refusing to replace staged file: {name}')
    require(destination.resolve().is_relative_to(RELEASE.resolve()), f'Outside release: {name}')
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    require(sha(source) == sha(destination), f'Copy mismatch: {name}')


def launcher_text(arguments, pause_after_exit=False):
    require(isinstance(arguments, list) and all(isinstance(v, str) for v in arguments),
            'Launcher arguments must be a string list')
    # Batch launchers are commands, so reject expansion/redirection syntax.
    require(all(not any(c in value for c in '"&|<>^%!\r\n\x00') for value in arguments),
            'Unsupported batch metacharacter in launcher argument')
    require(all(value.isascii() for value in arguments) and type(pause_after_exit) is bool,
            'Launchers require ASCII arguments and a Boolean pause flag')
    args = ''.join(' "' + value + '"' for value in arguments)
    return '@echo off\ncd /d "%~dp0"\n"%~dp0AxisAlliesRust.exe"' + args + '\n' + ('pause\n' if pause_after_exit else '')


def stage(options):
    require(not RELEASE.exists(), 'Development-09 already exists; staging never overwrites it')
    require(not (QA / 'staged.json').exists() and not (QA / 'source-snapshot.json').exists(),
            'Staging evidence already exists')
    require(RELEASE.parent.resolve().is_relative_to(ROOT.resolve()), 'Release parent escapes workspace')
    candidate = Path(options.exe).resolve()
    require(candidate.is_relative_to(ROOT.resolve()) and candidate.is_file(), 'Candidate must be a workspace file')
    pe = inspect_pe(candidate, options.sha256)
    readme = checked_document(options.readme)
    checkpoint = checked_document(options.checkpoint)
    launchers_path = workspace_path(options.launchers)
    launchers = read(launchers_path)
    require(isinstance(launchers, list) and bool(launchers), 'Provide approved launcher definitions')
    launcher_names = set()
    for launcher in launchers:
        name = str(relative_path(launcher['name']))
        require('/' not in name and name.endswith('.cmd') and name.casefold() not in launcher_names,
                'Invalid/duplicate launcher filename')
        launcher_names.add(name.casefold())
        require(name.isascii(), 'Launchers require ASCII filenames')
        launcher_text(launcher['arguments'],launcher.get('pause_after_exit',False))
    caches, movies, preserved = previous_payload()
    inputs = source_inputs()
    RELEASE.mkdir(parents=False, exist_ok=False)
    QA.mkdir(parents=True, exist_ok=True)
    copy(candidate, EXE_NAME)
    copy(readme, 'README.txt')
    copy(checkpoint, 'CHECKPOINT.txt')
    for row in caches:
        copy(ROOT / row['path'], row['path'])
    for launcher in launchers:
        (RELEASE / launcher['name']).write_text(launcher_text(launcher['arguments'],launcher.get('pause_after_exit',False)),
                                               encoding='ascii', newline='\r\n')
    require(sha(candidate) == sha(RELEASE / EXE_NAME) == options.sha256, 'Candidate changed while staging')
    write_json(QA / 'source-snapshot.json', {
        'scope': 'Workspace files at staging; this is not independent Cargo build attestation', 'files': inputs})
    write_json(QA / 'staged.json', {
        'schema': 1, 'exe_sha256': options.sha256, 'candidate_path': candidate.relative_to(ROOT).as_posix(),
        **pe, 'cache_files': len(caches), 'movies': movies, 'launchers': launchers,
        'folder': str(RELEASE), 'preserved_files': preserved,
        'input_documents': [record(p) for p in [readme, checkpoint, launchers_path]],
        'packaging_inputs': [record(Path(__file__).resolve()), record(ROOT/'research/check_development_09_package.py')],
        'source_snapshot_sha256': sha(QA / 'source-snapshot.json'),
        'staged_files': [record(p, RELEASE) for p in sorted(RELEASE.rglob('*')) if p.is_file()]})
    print(json.dumps({'folder': str(RELEASE), 'executable_sha256': options.sha256,
                      'phase': 'staged; exact copied executable still requires validation'}))


def seal(options):
    require(RELEASE.is_dir(), 'Stage Development-09 first')
    require(not ARCHIVE.exists() and not (RELEASE / 'manifest.json').exists(),
            'Package already sealed/partially sealed; refusing to overwrite')
    require(not (QA / 'package-report.json').exists(), 'Package report already exists')
    staged = read(QA / 'staged.json')
    validation_path = workspace_path(options.validation)
    validation = read(validation_path)
    require('<FILL_' not in json.dumps(validation) and '{{' not in json.dumps(validation),
            'Validation still contains template placeholders')
    require(validation.get('passed') is True, 'Validation did not pass')
    expected = staged['exe_sha256']
    require(validation.get('executable_sha256') == expected, 'Validation names a different executable')
    require(validation.get('native_map_load_diagnostic') is True
            and validation.get('native_runtime_diagnostics_connected') is True
            and validation.get('playable_runtime') is False
            and validation.get('playable_skirmish') is False
            and validation.get('full_game_parity') is False, 'Validation must distinguish diagnostic from gameplay')
    checks = validation.get('checks')
    require(isinstance(checks, list) and bool(checks) and all(
        isinstance(row, dict) and isinstance(row.get('name'),str) and bool(row['name'])
        and row.get('status') == 'passed' and isinstance(row.get('evidence_files'),list)
        and bool(row['evidence_files']) for row in checks),
        'Validation needs named completed checks and their evidence')
    require(isinstance(validation.get('not_tested'), list) and bool(validation['not_tested'])
            and isinstance(validation.get('limitations'), list) and bool(validation['limitations']),
            'Validation must explicitly state remaining tests/limitations')
    require(sha(workspace_path(staged['candidate_path'])) == sha(RELEASE / EXE_NAME) == expected,
            'Built/staged executable changed')
    require(sha(QA / 'source-snapshot.json') == staged['source_snapshot_sha256']
            and read(QA / 'source-snapshot.json')['files'] == source_inputs(),
            'Source changed after staging; rebuild/revalidate before release')
    verified_rows(ROOT, staged['preserved_files'])
    verified_rows(ROOT, staged['input_documents'])
    verified_rows(ROOT, staged['packaging_inputs'])
    names = verified_rows(RELEASE, staged['staged_files'])
    require({p.relative_to(RELEASE).as_posix() for p in RELEASE.rglob('*') if p.is_file()} == names,
            'Staged folder contains unexpected files; test profiles/logs belong in QA')
    notes = checked_document(options.validation_notes)
    evidence = validation.get('evidence_files')
    require(isinstance(evidence, list) and bool(evidence) and len(set(evidence)) == len(evidence),
            'Validation must enumerate unique evidence files')
    for check in checks:
        require(all(name in evidence for name in check['evidence_files']), 'Check references unlisted evidence')
    for name in evidence:
        path = workspace_path(name)
        require(not path.is_relative_to(RELEASE.resolve()), 'Evidence may not recursively include the package')
    copy(validation_path, 'evidence/validation.json')
    copy(QA / 'staged.json', 'evidence/staged.json')
    copy(QA / 'source-snapshot.json', 'evidence/source-snapshot.json')
    for name in evidence:
        copy(workspace_path(name), 'evidence/' + name)
    copy(notes, 'VALIDATION.txt')
    manifest = {
        'schema': 1, 'status': 'Development09: native map-load diagnostic and original asset previews; game unfinished',
        'standalone_game': False, 'full_game_parity': False, 'playable_skirmish': False,
        'playable_complete_match': False, 'native_runtime_diagnostics_connected': True,
        'playable_runtime': False,
        'native_map_load_diagnostic': True, 'architecture': staged['architecture'],
        'per_monitor_v2_manifest_marker': staged['per_monitor_v2_manifest_marker'],
        'large_address_aware': staged['large_address_aware'],
        'settings_location': '%APPDATA%/AxisAlliesRust/settings.json',
        'external_assets_required': [
            'Original Data.rwd',
            'reconstruction/game/data/maps/index.json and recovered map/model/texture/animation assets',
            'reconstruction/assets/original/Data including original UI/localization/fonts',
            'reconstruction/native-game/runtime-expanded-v14 source overlays',
            'Recovered Skins/Music/Speech assets for original menu art and audio'],
        'included_assets': 'Audited Development08 assets-cache payload including MainMenu/MainMenuGlobe frame caches; original game data is external',
        'workspace_source_tree_included': False,
        'checkpoint': 'CHECKPOINT.txt', 'validation': 'evidence/validation.json',
        'source_snapshot': 'evidence/source-snapshot.json',
        'files': [record(p, RELEASE) for p in sorted(RELEASE.rglob('*')) if p.is_file()]}
    require(all(not row['path'].endswith('.zip') or row['path'].startswith('evidence/') for row in manifest['files']),
            'Unexpected archive outside evidence')
    write_json(RELEASE / 'manifest.json', manifest)
    payload = sorted(p for p in RELEASE.rglob('*') if p.is_file())
    with zipfile.ZipFile(ARCHIVE, 'x', zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for path in payload:
            archive.write(path, path.relative_to(RELEASE).as_posix())
    # Use the separate checker so sealing and independent checking share the
    # public on-disk contract, not an unchecked "ZIP created" assertion.
    from check_development_09_package import check_sealed
    checked = check_sealed(RELEASE, staged)
    result = {'folder': str(RELEASE), 'exe_sha256': expected, 'zip_sha256': sha(ARCHIVE),
              'zip_bytes': ARCHIVE.stat().st_size, **checked}
    write_json(QA / 'package-report.json', result)
    print(json.dumps(result))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='phase', required=True)
    staging = sub.add_parser('stage')
    staging.add_argument('--exe', required=True)
    staging.add_argument('--sha256', required=True)
    staging.add_argument('--readme', required=True, help='Workspace-relative approved README')
    staging.add_argument('--checkpoint', required=True, help='Workspace-relative current handoff/checkpoint')
    staging.add_argument('--launchers', required=True, help='Workspace-relative approved JSON list')
    sealing = sub.add_parser('seal')
    sealing.add_argument('--validation', default='qa-output/development09/validation.json')
    sealing.add_argument('--validation-notes', required=True)
    options = parser.parse_args()
    stage(options) if options.phase == 'stage' else seal(options)


if __name__ == '__main__':
    main()
