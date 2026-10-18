from pathlib import Path
import os,json,hashlib,subprocess,time
root=Path.cwd();out=root/'research/phase7-runtime/catalog-independent-output';out.mkdir(exist_ok=True);env=os.environ.copy();env['AA_GAME_ROOT']=str(root.parent)
cmd=['cargo','test','-p','aa-runtime','--test','catalog_restoration','--test','localization','--test','settings','--','--include-ignored','--nocapture']
t=time.monotonic();p=subprocess.run(cmd,cwd=root,env=env,capture_output=True,text=True);log=out/'tests.log';log.write_text(p.stdout+p.stderr,encoding='utf8');r={'command':cmd,'cwd':str(root),'AA_GAME_ROOT':str(root.parent),'exit_code':p.returncode,'seconds':round(time.monotonic()-t,3),'log':str(log.relative_to(root)),'sha256':hashlib.sha256(log.read_bytes()).hexdigest()};(out/'checks.json').write_text(json.dumps([r],indent=2));print(json.dumps(r));raise SystemExit(p.returncode)
