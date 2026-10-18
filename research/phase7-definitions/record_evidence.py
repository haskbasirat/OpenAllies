from pathlib import Path
import os,json,subprocess,hashlib,time
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def h(p):return hashlib.sha256(p.read_bytes()).hexdigest()
commands=[['cargo','test','-p','aa-rules'],['cargo','test','-p','aa-rules','--test','native_ground','--test','native_doodads','--test','native_source_extras','--test','native_init_inputs','--test','native_property_inputs','--test','native_denizen','--test','native_construction','--','--ignored'],['cargo','clippy','-p','aa-rules','--all-targets','--','-D','warnings']]
env=os.environ.copy();env['AA_GAME_ROOT']=str(ROOT.parent)
checks=[]
for i,cmd in enumerate(commands):
 t=time.monotonic();p=subprocess.run(cmd,cwd=ROOT,env=env,capture_output=True,text=True);out=HERE/f'verification-{i+1}.txt';out.write_text(p.stdout+p.stderr,encoding='utf8');checks.append({'command':cmd,'cwd':str(ROOT),'environment':{'AA_GAME_ROOT':str(ROOT.parent)},'exit_code':p.returncode,'seconds':round(time.monotonic()-t,3),'output':out.name,'output_sha256':h(out)});print(i+1,p.returncode,flush=True)
 if p.returncode:raise SystemExit(p.returncode)
paths=list((ROOT/'crates/aa-rules').rglob('*.rs'))+list(HERE.glob('*.json'))+list(HERE.glob('*.py'))+[HERE/'README.txt']
paths=[p for p in paths if p.name not in ['evidence.json','independent-review.json']]
d={'schema':1,'passed':True,'scope':'Phase7 typed ground source bindings and bounded simulation Ability materialization; explicit exclusions in README.txt','producer_sha256':h(Path(__file__)),'checks':checks,'files':{str(p.relative_to(ROOT)).replace('\\','/'):h(p)for p in sorted(paths)}}
(HERE/'evidence.json').write_text(json.dumps(d,indent=2));print('evidence',h(HERE/'evidence.json'))
