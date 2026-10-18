"""Run fresh Berlin/Firefight strict startup, retaining real failures as failures."""
from pathlib import Path
import argparse,json,os,subprocess,time
import package_hotfix02 as p

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument("map",choices=("berlin","firefight"));args=parser.parse_args()
base=p.ROOT/"qa-output/development10/hotfix02-native-startup02"
build=p.read(base/"build-record.json");assert build["passed"] is True
assert build["source_files"]==p.source_rows(),"Source drift since test compile"
exe=p.ROOT/build["executable"]["path"];assert p.sha(exe)==build["executable"]["sha256"]
out=base/args.map;out.mkdir(exist_ok=False)
for folder in ("work","appdata","localappdata"):(out/folder).mkdir()
env=os.environ.copy();env.update(AA_GAME_ROOT=str(p.ROOT.parent),
    AA_PLAYABLE_REPORT=str(out/"attempt"),
    AA_PLAYABLE_MAP=("Maps/(2v2)Berlin.RMP" if args.map=="berlin" else "Maps/DeveloperMaps/(3v3)Firefight.RMP"),
    APPDATA=str(out/"appdata"),LOCALAPPDATA=str(out/"localappdata"))
command=[str(exe),"actual_local_firefight_host_launch_and_same_resource_task_reach_started_mission",
    "--exact","--ignored","--nocapture","--test-threads=1"]
start=time.monotonic();timed_out=False
with (out/"stdout.txt").open("x",encoding="utf-8") as stdout,(out/"stderr.txt").open("x",encoding="utf-8") as stderr:
    child=subprocess.Popen(command,cwd=out/"work",env=env,stdout=stdout,stderr=stderr)
    p.write_new(out/"process.json",dict(pid=child.pid,command=command,map=env["AA_PLAYABLE_MAP"],executable=build["executable"]))
    print(json.dumps(dict(map=args.map,pid=child.pid,phase="running")),flush=True)
    try:code=child.wait(timeout=480)
    except subprocess.TimeoutExpired:
        timed_out=True;child.kill();code=child.wait()
worker=p.read(out/"attempt.worker.json") if (out/"attempt.worker.json").exists() else None
record=dict(map=args.map,executable=build["executable"],exit_code=code,timed_out=timed_out,
    seconds=time.monotonic()-start,strict_startup_passed=code==0 and not timed_out,
    native_failure=None if worker is None else worker.get("error"),retained_prefix=None if worker is None else worker.get("retained_prefix"),
    playable_battle=False,source_unchanged_since_build=build["source_files"]==p.source_rows(),
    evidence_files=[p.record(file) for file in sorted(out.glob("*")) if file.is_file()],
    scope="Actual selected-map Host/resource/mission startup only. Strict failure is not converted into a pass. Even StartedMission does not accept frontend activation/View/playable battle.")
p.write_new(out/"result.json",record)
print(json.dumps(record),flush=True)
