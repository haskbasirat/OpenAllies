"""Verified source/evidence backup. Does not create or modify a game release."""
from datetime import datetime, timezone
from pathlib import Path
import hashlib
import json
import zipfile

ROOT = Path(__file__).resolve().parents[1]
QA = ROOT / 'qa-output/byro-kohan-20261016'


def sha(path):
    with path.open('rb') as handle:
        return hashlib.file_digest(handle, 'sha256').hexdigest()


def main():
    acceptance = json.loads((QA / 'acceptance.json').read_text())
    assert acceptance['scoped_validation_passed'] is True
    for relative, digest in acceptance['source_sha256'].items():
        assert sha(ROOT / relative) == digest, relative
    assert sha(ROOT.parent / 'reconstruction/tools/convert_models.py') == acceptance['converter_sha256']
    preserved = {
        ROOT.parent / 'AA.exe': '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26',
        ROOT.parent / 'reconstruction/native-game/runtime-expanded-v14/AA.exe': '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1',
        ROOT / 'release/Development-09/AxisAlliesRust.exe': 'db8ceac200346462571bb5135948feff86c5f296e4f020c01e3ed743612a9131',
        ROOT / 'release/Development-09/AxisAlliesRust-Development-09.zip': 'dab3ee9c3f3c99583b05f41cb7ef6784c04247ae765aaa5ff2e8786bb726561e',
        ROOT / 'release/Development-08/AxisAlliesRust.exe': '3d4e80d8aa8fcaf92915c5867b6e390551608f227e65047e12a7f5e3ad6f053e',
        ROOT / 'release/Development-08/AxisAlliesRust-Development-08.zip': '9c0527cf7cb679054d7670d08f15d4ab53ff3783c44abb07d22dcf4f2a9071c0',
    }
    for path, digest in preserved.items():
        assert sha(path) == digest, path
    prior = ROOT / 'backups/source-public-build-options-20261017-005510.zip'
    assert sha(prior) == '33c3bbee1d427ef8b64820383b9ce1ea9c161d5d323edfb1adcb7afd8a5cb5e5'
    with zipfile.ZipFile(prior) as saved:
        for name in ['Cargo.toml', 'Cargo.lock']:
            assert hashlib.sha256(saved.read(name)).hexdigest() == sha(ROOT / name), name
    files = {}
    folders = [ROOT / 'crates', ROOT / 'docs', QA,
        ROOT / 'research/byroredux-20261016', ROOT / 'research/kohan-leads-20261016',
        ROOT / 'research/skirmish-20261016-owner-actions', ROOT / 'research/skirmish-20261016-owner-action-gaps',
        ROOT / 'research/skirmish-20261016-combat-means', ROOT / 'backups/before-kohan-owner-actions-20261016']
    for folder in folders:
        assert folder.is_dir(), folder
        for path in folder.rglob('*'):
            if path.is_file() and not {'target', '__pycache__'}.intersection(path.relative_to(ROOT).parts) and path.suffix.lower() not in ['.o', '.obj', '.pdb', '.exp', '.lib'] and not path.name.startswith('source-backup-'):
                files[path.relative_to(ROOT).as_posix()] = path
    for name in ['Cargo.toml', 'Cargo.lock', 'HANDOFF.txt',
                 'research/check_byro_kohan_20261016.py', 'research/backup_byro_kohan_20261016.py',
                 'research/check_skirmish_state_leader_20261016.py']:
        files[name] = ROOT / name
    for folder in [ROOT.parent / 'reconstruction/references/byroredux-20261016', ROOT.parent / 'reconstruction/references/kohan-leads-20261016']:
        assert folder.is_dir(), folder
        for path in folder.rglob('*'):
            if path.is_file() and not {'target', '__pycache__'}.intersection(path.relative_to(folder).parts):
                files['external/' + path.relative_to(ROOT.parent).as_posix()] = path
    converter = ROOT.parent / 'reconstruction/tools/convert_models.py'
    files['external/' + converter.relative_to(ROOT.parent).as_posix()] = converter
    pins = {name: sha(path) for name, path in sorted(files.items())}
    tag = datetime.now(timezone.utc).strftime('%Y%m%d-%H%M%S')
    archive = ROOT / 'backups' / f'source-byro-kohan-{tag}.zip'
    with zipfile.ZipFile(archive, 'x', zipfile.ZIP_DEFLATED, compresslevel=6) as output:
        for name, path in sorted(files.items()):
            output.write(path, name)
    with zipfile.ZipFile(archive) as saved:
        assert saved.testzip() is None and len(saved.infolist()) == len(pins)
        for name, digest in pins.items():
            assert hashlib.sha256(saved.read(name)).hexdigest() == digest, name
    for name, path in files.items():
        assert sha(path) == pins[name], 'Concurrent source mutation: ' + name
    for path, digest in preserved.items():
        assert sha(path) == digest, path
    result = {'schema': 1, 'passed': True, 'archive': str(archive.relative_to(ROOT)),
        'archive_bytes': archive.stat().st_size, 'archive_sha256': sha(archive),
        'file_count': len(pins), 'sha256': pins,
        'scope': 'Current source, dated evidence, selected public source and isolated parser. Not a game release/full historical fixture archive; original assets/vendor SDK excluded.',
        'preserved_sha256': {str(path): digest for path, digest in preserved.items()},
        'unchanged_manifests_compared_to': str(prior.relative_to(ROOT))}
    report = QA / f'source-backup-{tag}.json'
    with report.open('x', encoding='utf-8') as handle:
        json.dump(result, handle, indent=2)
        handle.write('\n')
    print(json.dumps({key: value for key, value in result.items() if key not in ['sha256', 'preserved_sha256']}))


if __name__ == '__main__':
    main()
