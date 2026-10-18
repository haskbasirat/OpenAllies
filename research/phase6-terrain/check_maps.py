"""Headless shipped-map regression across biome families, preserving both captures."""
from pathlib import Path
import argparse, hashlib, json, subprocess, time
ROOT=Path(__file__).resolve().parents[2]
parser=argparse.ArgumentParser()
parser.add_argument('--exe',type=Path,default=ROOT/'target/debug/AxisAlliesRust.exe')
parser.add_argument('--output',type=Path,default=ROOT/'qa-output/phase6/terrain-maps')
args=parser.parse_args();exe=args.exe.resolve();output=args.output.resolve();output.mkdir(exist_ok=False,parents=True)
assets=ROOT.parent/'reconstruction'
manifest=json.loads((assets/'game/data/maps/index.json').read_text())
chosen={'chaparral':'Campaign/axis01_crete','desert':'Campaign/allies01_el_alamein','arctic_tundra':'Campaign/tutorial01'}
for row in manifest:
    for biome in row['biomes']:
        chosen.setdefault(biome,row['id'])
identities=list(dict.fromkeys(chosen.values()))
report={'exe':str(exe),'exe_sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'scope':'Headless asset/render regression; not native screenshot or gameplay parity','biomes':chosen,'maps':[]}
for ident in identities:
    name=hashlib.sha256(ident.encode()).hexdigest()[:16]+'.png';start=time.monotonic()
    run=subprocess.run([str(exe),'--assets',str(assets),'--backend','vulkan','--workers','4','--map',ident,'--render-check',str(output/name)],capture_output=True,text=True,encoding='utf8',timeout=120)
    row={'id':ident,'exit_code':run.returncode,'wall_seconds':time.monotonic()-start,'stderr':run.stderr}
    if run.returncode==0:
        row['result']=json.loads(run.stdout);row['image']=name;row['image_sha256']=hashlib.sha256((output/name).read_bytes()).hexdigest()
    else:row['stdout']=run.stdout
    report['maps'].append(row);(output/'report.json').write_text(json.dumps(report,indent=2))
    print(ident,run.returncode,flush=True)
assert all(row['exit_code']==0 for row in report['maps'])
print(json.dumps({'passed':len(identities),'biomes':len(chosen),'report':str(output/'report.json')}))
