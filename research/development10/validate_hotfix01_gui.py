"""Click the real startup path; capture GUI frames while work is pending.

Uses the application's egui input/screenshot hook, not desktop-wide input.
Kills only this helper's captured subprocess on an explicit bounded timeout.
Screenshot existence alone is not a semantic pass; inspect the images/logs.
"""
from pathlib import Path
import argparse, json, os, subprocess, time
import package_hotfix01 as package

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument("map", choices=("berlin","firefight"))
args=parser.parse_args()
OUT=package.ROOT/"qa-output/development10/hotfix01-gui"/args.map
OUT.mkdir(parents=True,exist_ok=False)
for folder in ("work","appdata","localappdata"):
    (OUT/folder).mkdir()
env=os.environ.copy()
env["APPDATA"]=str(OUT/"appdata")
env["LOCALAPPDATA"]=str(OUT/"localappdata")
exe=package.RELEASE/package.EXE
staged=package.read(package.RELEASE/"STAGING.json")
assert package.sha(exe)==staged["executable_sha256"]
steps=[{"wait":2,"click":[1266,276]}, {"wait":.4,"click":[1266,332]},
       {"wait":.3,"click":[610,439]}, {"text":"Regression Tester"},
       {"wait":.2,"click":[625,418]}]
if args.map=="berlin":
    steps += [{"wait":.2,"click":[600,442]}]
else:
    steps += [{"move":[625,600]}, {"scroll":-84},
              {"wait":.6,"click":[600,611]}]
steps += [{"wait":.3,"shot":str(OUT/"before-start.png")},
          {"wait":.2,"log":"Click actual Test skirmish startup","click":[482,549]}]
last=0
capture_times=(1,3,10,30,60,90,125,155) if args.map=="berlin" else (1,3,10,30,60,90,125,155,190,220)
for seconds in capture_times:
    steps.append({"wait":seconds-last,"shot":str(OUT/f"after-{seconds:03d}s.png"),
                  "log":f"GUI processed screenshot step at target +{seconds}s"})
    if args.map=="firefight" and seconds==10:
        #Session button coordinates observed in the preceding Berlin run.
        #Leaving the page must preserve the same in-flight attempt.
        steps += [{"wait":.2,"click":[419,519]},
                  {"wait":.3,"shot":str(OUT/"menu-during-loading.png")},
                  {"wait":.2,"click":[1266,276]},
                  {"wait":.3,"click":[1266,332]},
                  {"wait":.3,"shot":str(OUT/"resumed-same-loading.png")}]
    last=seconds
steps.append({"wait":1,"log":"Request application close after diagnostic capture","quit":True})
package.write_new(OUT/"script.json",steps)
command=[str(exe),"--ui-script",str(OUT/"script.json")]
started=time.monotonic();timed_out=False;code=None
with (OUT/"stdout.txt").open("x",encoding="utf-8") as stdout, (OUT/"stderr.txt").open("x",encoding="utf-8") as stderr:
    process=subprocess.Popen(command,cwd=OUT/"work",env=env,stdout=stdout,stderr=stderr,shell=False)
    package.write_new(OUT/"process.json",dict(pid=process.pid,command=command,executable_sha256=package.sha(exe)))
    print(json.dumps(dict(map=args.map,pid=process.pid,phase="running")),flush=True)
    try:
        code=process.wait(timeout=360 if args.map=="firefight" else 240)
    except subprocess.TimeoutExpired:
        timed_out=True
        process.kill()
        code=process.wait()
shots=[package.record(p) for p in sorted(OUT.glob("*.png"))]
result=dict(map=args.map,executable_sha256=package.sha(exe),pid=process.pid,
    command=command,seconds=time.monotonic()-started,exit_code=code,timed_out=timed_out,
    screenshot_count=len(shots),screenshots=shots,semantic_review_required=True,
    playable_battle=False,scope=__doc__)
package.write_new(OUT/"result.json",result)
print(json.dumps(result),flush=True)
