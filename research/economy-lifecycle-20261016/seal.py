"""Source/evidence backup only. Original assets and older oracle dependencies stay external."""
from pathlib import Path
from datetime import datetime, timezone
import argparse, hashlib, json, zipfile

ROOT = Path(__file__).resolve().parents[2]
QA = ROOT / 'qa-output/economy-lifecycle-20261016'
AREAS = ['borrowed-flows', 'saved-flows', 'saved-economy-values', 'saved-economy-integration',
         'restore-lifecycle', 'recruit-backrefs', 'map-replacement-queue', 'kingdom-init', 'team-init',
         'managed-attachment-regression']

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--tag', required=True)
    args = parser.parse_args()
    if not args.tag.replace('-', '').isalnum():
        raise ValueError('tag must contain letters, numbers and hyphens only')
    preserved = {
        ROOT.parent / 'AA.exe': '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26',
        ROOT.parent / 'reconstruction/native-game/runtime-expanded-v14/AA.exe': '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1',
        ROOT / 'release/Development-08/AxisAlliesRust.exe': '3d4e80d8aa8fcaf92915c5867b6e390551608f227e65047e12a7f5e3ad6f053e',
        ROOT / 'release/Development-08/AxisAlliesRust-Development-08.zip': '9c0527cf7cb679054d7670d08f15d4ab53ff3783c44abb07d22dcf4f2a9071c0'}
    for path, digest in preserved.items():
        assert sha(path) == digest, path
    assert (QA / 'acceptance.json').is_file(), 'run acceptance summary first'
    files = {p for p in (ROOT / 'crates').rglob('*') if p.is_file()
        and p.suffix in ['.rs', '.toml', '.json', '.wgsl']}
    files.update(p for p in (ROOT / 'docs').rglob('*') if p.is_file())
    files.update(ROOT / name for name in ['Cargo.toml', 'Cargo.lock', 'HANDOFF.txt'])
    for area in AREAS:
        files.update(p for p in (ROOT / 'research' / f'skirmish-20261016-{area}').rglob('*')
            if p.is_file() and '__pycache__' not in p.parts)
    files.update(p for p in Path(__file__).parent.rglob('*') if p.is_file() and '__pycache__' not in p.parts)
    files.update(p for p in QA.rglob('*') if p.is_file())
    pins = {str(p.relative_to(ROOT)).replace('\\', '/'): sha(p) for p in sorted(files)}
    archive = ROOT / 'backups' / f'source-skirmish-economy-lifecycle-{args.tag}.zip'
    report = QA / f'checkpoint-{args.tag}.json'
    assert not archive.exists() and not report.exists(), 'checkpoint paths must be new'
    with zipfile.ZipFile(archive, 'x', zipfile.ZIP_DEFLATED, compresslevel=6) as handle:
        for name in pins:
            handle.write(ROOT / name, name)
    with zipfile.ZipFile(archive) as handle:
        assert handle.testzip() is None
        assert len(handle.infolist()) == len(pins)
        for name, digest in pins.items():
            assert hashlib.sha256(handle.read(name)).hexdigest() == digest, name
    for name, digest in pins.items():
        assert sha(ROOT / name) == digest, 'concurrent mutation: ' + name
    for path, digest in preserved.items():
        assert sha(path) == digest, path
    result = {'schema': 1, 'created_utc': datetime.now(timezone.utc).isoformat(),
        'scope': 'Source/evidence backup; no new release or battle acceptance',
        'archive': str(archive), 'archive_bytes': archive.stat().st_size, 'archive_sha256': sha(archive),
        'files': len(pins), 'source_and_evidence_sha256': pins,
        'preserved_sha256': {str(p): digest for p, digest in preserved.items()},
        'backup_limits': 'Original assets and older research/oracle fixtures are external dependencies. No target binaries included.'}
    with report.open('x', encoding='utf-8') as handle:
        json.dump(result, handle, indent=2)
        handle.write('\n')
    print(json.dumps({k: v for k, v in result.items()
        if k not in ['source_and_evidence_sha256', 'preserved_sha256']}))

if __name__ == '__main__':
    main()
