"""Save a source/evidence checkpoint after all writers and builds are frozen.

This backs up the local source and this continuation's evidence. Original game
assets and preexisting research fixtures are dependencies, not copied game data.
"""
from pathlib import Path
from datetime import datetime, timezone
import argparse,hashlib,json,zipfile

ROOT=Path(__file__).resolve().parents[2]
AREAS=['skirmish-20261016-group-classes','skirmish-20261016-owner-recruitment',
    'skirmish-20261016-research-actions','skirmish-20261016-structure-actions',
    'skirmish-20261016-denizen-actions','skirmish-20261016-saved-economy-entry',
    'skirmish-20261016-construction-actions','skirmish-20261016-start-bindings',
    'skirmish-20261016-regiment-context']
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--tag',required=True)
    args=ap.parse_args()
    if not args.tag.replace('-','').isalnum():raise ValueError('tag must be alphanumeric/hyphens')
    baseline=json.loads((ROOT/'qa-output/resume-20261016/skirmish-checkpoint-validation.json').read_text(encoding='utf-8-sig'))
    # Preserved executable/release pins are explicit rather than guessed from
    # a potentially changed baseline report schema.
    preserved={
        ROOT.parent/'AA.exe':'5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26',
        ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe':'3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1',
        ROOT/'release/Development-08/AxisAlliesRust.exe':'3d4e80d8aa8fcaf92915c5867b6e390551608f227e65047e12a7f5e3ad6f053e',
        ROOT/'release/Development-08/AxisAlliesRust-Development-08.zip':'9c0527cf7cb679054d7670d08f15d4ab53ff3783c44abb07d22dcf4f2a9071c0',
    }
    for p,expected in preserved.items():assert sha(p)==expected,str(p)
    files={p for p in (ROOT/'crates').rglob('*') if p.is_file() and p.suffix in ['.rs','.toml','.json','.wgsl']}
    files.update(p for p in (ROOT/'docs').rglob('*') if p.is_file())
    files.update(ROOT/p for p in ['Cargo.toml','Cargo.lock','HANDOFF.txt'])
    for area in AREAS:
        files.update(p for p in (ROOT/'research'/area).rglob('*') if p.is_file())
    files.update(p for p in (ROOT/'qa-output/resume-20261016').glob('continued-*') if p.is_file())
    files.add(Path(__file__).resolve())
    files.add(Path(__file__).with_name('validate_continued.ps1'))
    files.add(Path(__file__).with_name('summarize_continued.py'))
    files.add(ROOT/'research/resume-20261016-element-attach/observer/owner-binding-followup.json')
    pins={str(p.relative_to(ROOT)).replace('\\','/'):sha(p) for p in sorted(files)}
    archive=ROOT/'backups'/f'source-skirmish-continued-{args.tag}.zip'
    report=ROOT/'qa-output/resume-20261016'/f'continued-checkpoint-{args.tag}.json'
    assert not archive.exists() and not report.exists(),'checkpoint names must be new'
    with zipfile.ZipFile(archive,'x',zipfile.ZIP_DEFLATED,compresslevel=6) as z:
        for name in pins:z.write(ROOT/name,name)
    with zipfile.ZipFile(archive) as z:
        assert z.testzip() is None
        assert len(z.infolist())==len(pins)
        for name,expected in pins.items():assert hashlib.sha256(z.read(name)).hexdigest()==expected,name
    for name,expected in pins.items():assert sha(ROOT/name)==expected,'concurrent source/evidence mutation: '+name
    for p,expected in preserved.items():assert sha(p)==expected,str(p)
    result={'schema':1,'created_utc':datetime.now(timezone.utc).isoformat(),
        'scope':'source and new evidence checkpoint; not a new release or playable-battle acceptance',
        'baseline_validation_sha256':sha(ROOT/'qa-output/resume-20261016/skirmish-checkpoint-validation.json'),
        'baseline_validation_loaded':bool(baseline),'archive':str(archive),'archive_bytes':archive.stat().st_size,
        'archive_sha256':sha(archive),'files':len(pins),'source_and_evidence_sha256':pins,
        'preserved_sha256':{str(p):v for p,v in preserved.items()},
        'backup_limits':'Original assets and older research fixtures are external dependencies; no target binaries included.'}
    with report.open('x',encoding='utf-8') as f:json.dump(result,f,indent=2);f.write('\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ['source_and_evidence_sha256','preserved_sha256']}))
if __name__=='__main__':main()
