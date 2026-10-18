"""Exercise the staged client through its own egui test-input hook.

Screenshots are reviewed separately. Does not claim battle execution or
Windows scaling on a different PC. Preferences stay in isolated QA folders.
"""
from pathlib import Path
import json, os, subprocess, time
import package
OUT=package.ROOT/'qa-output/development10/acceptance02'
assert OUT.is_dir()
exe=package.RELEASE/package.EXE
staged=package.read(package.RELEASE/'STAGING.json')
assert package.sha(exe)==staged['executable_sha256']
env=os.environ.copy()
env['APPDATA']=str(OUT/'gui-appdata')
env['LOCALAPPDATA']=str(OUT/'gui-localappdata')
rows=[]
for name,steps in [
    ('main-menu',[{'wait':6,'quit':True}]),
    ('skirmish-setup',[{'wait':3,'click':[1266,276]},
        {'wait':0.5,'click':[1266,332]},
        {'wait':0.5,'click':[610,439]},
        {'wait':0.2,'text':'Development10 Tester'},
        {'wait':0.5,'quit':True}]),
]:
    script=OUT/(name+'-script.json');shot=OUT/(name+'.png')
    package.write_new(script,steps)
    args=[str(exe),'--ui-script',str(script),'--shot',str(shot)]
    start=time.monotonic()
    p=subprocess.run(args,cwd=OUT/'work',env=env,shell=False,capture_output=True,
        text=True,encoding='utf-8',errors='replace',timeout=60)
    for suffix,value in [('stdout.txt',p.stdout),('stderr.txt',p.stderr)]:
        with(OUT/(name+'.'+suffix)).open('x',encoding='utf-8')as f:f.write(value)
    rows.append(dict(name=name,arguments=args[1:],exit_code=p.returncode,
        image=shot.name,image_created=shot.is_file(),seconds=time.monotonic()-start))
package.write_new(OUT/'frontend-results.json',dict(executable_sha256=package.sha(exe),
    passed=all(r['exit_code']==0 and r['image_created']for r in rows),
    scope=__doc__,visual_review_required=True,checks=rows))
print(json.dumps(rows))
