"""Read-only headless smoke checks for the copied Development06 executable."""
from pathlib import Path
import hashlib, json, subprocess

root=Path(__file__).resolve().parents[1]
release=root/'release/Development-06'
exe=release/'AxisAlliesRust.exe'
qa=root/'qa-output/phase6'
out=qa/'package-smoke'
out.mkdir(parents=True,exist_ok=False)
read=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert sha(exe)==read(qa/'package-report.json')['exe_sha256']
rows=[]
for name,arguments in [
    ('world',['--world-check','--assets',str(out/'nonexistent-assets')]),
    ('mechanics',['--mechanics-check','--assets',str(out/'nonexistent-assets')]),
    ('catalog',['--verify-assets','--assets',str(root.parent/'reconstruction')]),
    ('render',['--backend','vulkan','--workers','4','--assets',str(root.parent/'reconstruction'),
               '--map','Campaign/axis01_crete','--render-check',str(out/'crete.png')]),
]:
    cmd=[str(exe),*arguments]
    proc=subprocess.run(cmd,cwd='E:/Games',capture_output=True,text=True,timeout=120)
    row={'name':name,'command':cmd,'exit_code':proc.returncode,'stderr':proc.stderr}
    assert proc.returncode==0,(name,proc.stdout,proc.stderr)
    result=json.loads(proc.stdout)
    if name in ('world','mechanics'):assert result['passed']
    if name=='render':
        assert not any('Original water source binding failed' in w for w in result.get('warnings',[]))
        assert sha(out/'crete.png')==sha(qa/'visuals-final/crete-vulkan.png')
    row['passed']=True
    rows.append(row)
manifest=read(release/'manifest.json')
for row in manifest['files']:assert sha(release/row['path'])==row['sha256']
report={'passed':True,'exe_sha256':sha(exe),'cwd':'E:/Games','checks':rows,
        'copied_map_png_identical_to_final_candidate':True,
        'manifest_payload_hashes_verified':len(manifest['files']),
        'scope':'Copied executable and shipped cache headless smoke; original assets still required, no full match/GUI/multiplayer claim'}
(qa/'package-smoke.json').write_text(json.dumps(report,indent=2))
print(json.dumps(report))
