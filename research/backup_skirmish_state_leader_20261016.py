"""Seal this continuation without changing previous releases or backups."""
from datetime import datetime, timezone
from pathlib import Path
import hashlib
import json
import zipfile

ROOT = Path(__file__).resolve().parents[1]
AREAS = [
    'saved-actor-state', 'borrowed-leader', 'saved-leader',
    'saved-organization-position', 'loaded-placement',
    'settlement-completion', 'startup-critical-path',
]
QA = ROOT / 'qa-output/skirmish-state-leader-20261016'


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    tag = datetime.now(timezone.utc).strftime('%Y%m%d-%H%M%S')
    archive = ROOT / 'backups' / f'source-skirmish-state-leader-{tag}.zip'
    report = QA / f'source-backup-{tag}.json'
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
    prior_source = ROOT / 'backups/source-development09-20261016-230859.zip'
    with zipfile.ZipFile(prior_source) as saved:
        for name in ['Cargo.toml', 'Cargo.lock']:
            assert hashlib.sha256(saved.read(name)).hexdigest() == sha(ROOT / name), name
    files = set()
    folders = [ROOT / 'crates', ROOT / 'docs', QA,
               *[ROOT / 'research' / f'skirmish-20261016-{area}' for area in AREAS]]
    for folder in folders:
        assert folder.is_dir(), folder
        files.update(path for path in folder.rglob('*') if path.is_file()
                     and not {'target', '__pycache__'}.intersection(path.relative_to(ROOT).parts)
                     and not path.name.startswith('source-backup-'))
    files.update(ROOT / name for name in ['Cargo.toml', 'Cargo.lock', 'HANDOFF.txt'])
    files.add(Path(__file__).resolve())
    files.add(ROOT / 'research/check_skirmish_state_leader_20261016.py')
    pins = {p.relative_to(ROOT).as_posix(): sha(p) for p in sorted(files)}
    with zipfile.ZipFile(archive, 'x', zipfile.ZIP_DEFLATED, compresslevel=6) as output:
        for name in pins:
            output.write(ROOT / name, name)
    with zipfile.ZipFile(archive) as saved:
        assert saved.testzip() is None and len(saved.infolist()) == len(pins)
        for name, digest in pins.items():
            assert hashlib.sha256(saved.read(name)).hexdigest() == digest, name
    for name, digest in pins.items():
        assert sha(ROOT / name) == digest, 'Concurrent mutation: ' + name
    for path, digest in preserved.items():
        assert sha(path) == digest, path
    result = {
        'schema': 1, 'passed': True,
        'created_utc': datetime.now(timezone.utc).isoformat(),
        'scope': 'Source and current continuation evidence; not a game release. Original assets and older proof fixtures remain external.',
        'archive': archive.relative_to(ROOT).as_posix(),
        'archive_bytes': archive.stat().st_size, 'archive_sha256': sha(archive),
        'file_count': len(pins), 'source_and_evidence_sha256': pins,
        'preserved_sha256': {str(p): d for p, d in preserved.items()},
        'unchanged_manifests_compared_to': str(prior_source.relative_to(ROOT)),
    }
    with report.open('x', encoding='utf-8') as handle:
        json.dump(result, handle, indent=2)
        handle.write('\n')
    print(json.dumps({k: v for k, v in result.items()
                      if k not in ['source_and_evidence_sha256', 'preserved_sha256']}))


if __name__ == '__main__':
    main()
