"""Headless smoke checks for every shipped original-rig preview clip."""
from pathlib import Path
import argparse,hashlib,json,subprocess
root=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser()
parser.add_argument('--exe',type=Path,default=root/'target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe')
parser.add_argument('--output',type=Path,default=root/'qa-output/phase3/animations-final')
parser.add_argument('--backend',choices=['dx12','vulkan'],default='vulkan')
parser.add_argument('--expect-rigs',type=int)
parser.add_argument('--expect-clips',type=int)
args=parser.parse_args()
exe=args.exe.resolve()
out=args.output.resolve()
out.mkdir(parents=True,exist_ok=False)
paths=sorted((root/'assets-cache/animation').glob('*.rig.json'))
assert paths and (args.expect_rigs is None or len(paths)==args.expect_rigs)
rows=[]
for path in paths:
    rig=path.name.removesuffix('.rig.json')
    for clip_index,clip in enumerate(json.loads(path.read_text())['clips']):
        key=f'{rig}/{clip_index}/{clip["source"]}'
        png=out/(hashlib.sha256(key.encode()).hexdigest()[:16]+'.png')
        process=subprocess.run([str(exe),'--backend',args.backend,'--animated',rig,'--clip-index',str(clip_index),'--time','0.2',
                                '--render-check',str(png)],capture_output=True,text=True,timeout=60)
        row={'rig':rig,'clip':clip['name'],'clip_index':clip_index,'source':clip['source'],'exit_code':process.returncode,'stderr':process.stderr}
        if process.returncode==0:
            row['result']=json.loads(process.stdout)
            row['image_sha256']=hashlib.sha256(png.read_bytes()).hexdigest()
        else:row['stdout']=process.stdout
        rows.append(row)
        print(key,process.returncode,flush=True)
        report={'executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'rigs':len(paths),'backend':args.backend,'results':rows}
        (out/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
report={'executable_sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'rigs':len(paths),'backend':args.backend,'results':rows}
(out/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
assert (args.expect_clips is None or len(rows)==args.expect_clips) and all(r['exit_code']==0 for r in rows)
