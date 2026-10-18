"""Verify and back up current source/evidence/public references; no releases."""
from datetime import datetime, timezone
from pathlib import Path
import hashlib
import json
import zipfile

ROOT = Path(__file__).resolve().parents[1]
QA = ROOT / 'qa-output/public-build-options-20261016'
REFERENCES = ROOT.parent / 'reconstruction/references/public-rewrite-20261016'


def sha(path):
    with path.open('rb') as handle:
        return hashlib.file_digest(handle, 'sha256').hexdigest()


def main():
    acceptance = json.loads((QA / 'acceptance.json').read_text())
    assert acceptance['scoped_validation_passed'] is True
    for name, digest in acceptance['source_sha256'].items():
        assert sha(ROOT / name) == digest, name
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
    prior = ROOT / 'backups/source-skirmish-state-leader-20261017-001749.zip'
    with zipfile.ZipFile(prior) as saved:
        for name in ['Cargo.toml', 'Cargo.lock']:
            assert hashlib.sha256(saved.read(name)).hexdigest() == sha(ROOT / name), name
    files = {}
    areas = ['actor-property-core', 'saved-actor-properties', 'builder-data',
             'owner-economy-refresh', 'combat-source-context']
    folders = [ROOT / 'crates', ROOT / 'docs', QA,
               ROOT / 'research/public-references-20261016',
               *[ROOT / 'research' / f'skirmish-20261016-{area}' for area in areas]]
    for folder in folders:
        assert folder.is_dir(), folder
        for path in folder.rglob('*'):
            if path.is_file() and not {'target', '__pycache__'}.intersection(path.relative_to(ROOT).parts) and not path.name.startswith('source-backup-'):
                files[path.relative_to(ROOT).as_posix()] = path
    for name in ['Cargo.toml', 'Cargo.lock', 'HANDOFF.txt',
                 'research/check_public_build_options_20261016.py',
                 'research/check_skirmish_state_leader_20261016.py',
                 'research/backup_public_build_options_20261016.py']:
        files[name] = ROOT / name
    external = [*REFERENCES.rglob('*'), ROOT.parent / 'reconstruction/tools/convert_models.py']
    for path in external:
        if path.is_file() and '__pycache__' not in path.parts:
            files['external/' + path.relative_to(ROOT.parent).as_posix()] = path
    pins = {name: sha(path) for name, path in sorted(files.items())}
    tag = datetime.now(timezone.utc).strftime('%Y%m%d-%H%M%S')
    archive = ROOT / 'backups' / f'source-public-build-options-{tag}.zip'
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
    result = {
        'schema': 1, 'passed': True, 'audit_date_local': '2026-10-16',
        'archive': archive.relative_to(ROOT).as_posix(), 'archive_bytes': archive.stat().st_size,
        'archive_sha256': sha(archive), 'file_count': len(pins), 'sha256': pins,
        'scope': 'Current source, continuation evidence and selected public references. Not a game release or complete historical fixture archive. No vendor SDK archives included.',
        'preserved_sha256': {str(path): digest for path, digest in preserved.items()},
        'unchanged_manifests_compared_to': prior.relative_to(ROOT).as_posix(),
    }
    report = QA / f'source-backup-{tag}.json'
    with report.open('x', encoding='utf-8') as handle:
        json.dump(result, handle, indent=2)
        handle.write('\n')
    print(json.dumps({key: value for key, value in result.items() if key not in ['sha256', 'preserved_sha256']}))


if __name__ == '__main__':
    main()
