"""Build the actual application, retaining default release optimizations."""
from pathlib import Path
import hashlib, importlib.util, json, os, subprocess, sys, time
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/"qa-output/development10/hotfix02-build02"
OUT.mkdir(parents=True,exist_ok=True)
spec=importlib.util.spec_from_file_location("dev10_hotfix_package",ROOT/"research/development10/package_hotfix02.py")
package=importlib.util.module_from_spec(spec);spec.loader.exec_module(package)
def save(name,obj):
    with (OUT/name).open("x",encoding="utf-8") as f: json.dump(obj,f,indent=2);f.write("\n")
before=package.source_rows()
save("source-before.json",before)
env=os.environ.copy()
env["CARGO_TARGET_DIR"]="target"
for name in ("CARGO_PROFILE_RELEASE_LTO","CARGO_PROFILE_RELEASE_CODEGEN_UNITS"):
    env.pop(name,None)
command=["cargo","build","-p","aa-client","--bin","AxisAlliesRust","--target","x86_64-pc-windows-msvc","--release","--locked","--offline","-j","2"]
started=time.time()
with (OUT/"build.log").open("x",encoding="utf-8") as log:
    result=subprocess.run(command,cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
after=package.source_rows()
save("source-after.json",after)
exe=ROOT/"target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe"
evidence=[OUT/"build.log", OUT/"source-before.json", OUT/"source-after.json",
    ROOT/"research/development10/build_hotfix02_candidate02.py"]
record={"passed":result.returncode==0 and before==after and exe.is_file(),
    "source_unchanged_during_build":before==after,"exit_code":result.returncode,
    "target":"x86_64-pc-windows-msvc","profile":"release","profile_overrides":{},
    "command":command,"seconds":time.time()-started,
    "rustc_version":subprocess.check_output(["rustc","-vV"],cwd=ROOT,text=True).strip(),
    "source_files":after,"evidence_files":[package.record(p) for p in evidence]}
if exe.is_file():
    record["executable_sha256"]=package.sha(exe)
    record["candidate"]=str(exe)
save("build-record.json",record)
print(json.dumps({key:value for key,value in record.items() if key not in ("source_files","evidence_files")}))
sys.exit(0 if record["passed"] else 1)
