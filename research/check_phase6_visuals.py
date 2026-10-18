"""Final executable render/cache probes; these are not FPS or native screenshot parity."""
from pathlib import Path
import hashlib, json, subprocess

root=Path(__file__).resolve().parents[1]
exe=root/'target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe'
out=root/'qa-output/phase6/visuals-final'
out.mkdir(parents=True,exist_ok=False)
rows=[]
for name,backend,selection,frames in [
    ('crete-vulkan','vulkan',['--map','Campaign/axis01_crete'],61),
    ('crete-dx12','dx12',['--map','Campaign/axis01_crete'],3),
    ('stalingrad','vulkan',['--map','Campaign/allies03_stalingrad'],3),
    ('corps','vulkan',['--model','Buildings/GBR/GBRCorps/GBRCorps.nif'],3),
    ('oil','vulkan',['--model','Buildings/GBR/GBRDepotOil/GBRDepotOil.nif'],3),
]:
    cmd=[str(exe),'--assets',str(root.parent/'reconstruction'),'--workers','4','--backend',backend,
         *selection,'--render-check',str(out/(name+'.png')),'--render-frames',str(frames)]
    proc=subprocess.run(cmd,cwd='E:/Games',capture_output=True,text=True,timeout=180)
    row={'name':name,'command':cmd,'exit_code':proc.returncode,'stderr':proc.stderr}
    if proc.returncode==0:
        row['result']=json.loads(proc.stdout)
        row['passed']=row['result']['draw_order_rebuilds']==1 and not any(
            'Original water source binding failed' in warning for warning in row['result'].get('warnings',[]))
        row['png_sha256']=hashlib.sha256((out/(name+'.png')).read_bytes()).hexdigest()
    else:
        row.update(passed=False,stdout=proc.stdout)
    rows.append(row)
    report={'passed':all(r['passed'] for r in rows),'exe_sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),
            'scope':'Headless current-asset rendering, backend and draw-cache probes; no original-pixel or game-FPS parity claim',
            'results':rows}
    (out/'report.json').write_text(json.dumps(report,indent=2))
    print(name,row['passed'],flush=True)
assert len(rows)==5 and report['passed']
