"""Build the existing strict startup integration target after the source freeze."""
from pathlib import Path
import json,os,subprocess,time
import package_hotfix02 as p

out=p.ROOT/"qa-output/development10/hotfix02-native-startup02"
out.mkdir(parents=True,exist_ok=False)
before=p.source_rows();p.write_new(out/"source-before.json",before)
env=os.environ.copy();env["CARGO_TARGET_DIR"]="target/s-local-bootstrap"
command=["cargo","test","-p","aa-runtime","--test","playable_battle_startup","--no-run","--release",
    "--config","profile.release.lto=false","--config","profile.release.codegen-units=16",
    "--locked","--offline","-j","2","--message-format=json"]
start=time.monotonic()
with (out/"build.stdout.jsonl").open("x",encoding="utf-8") as stdout,(out/"build.stderr.txt").open("x",encoding="utf-8") as stderr:
    result=subprocess.run(command,cwd=p.ROOT,env=env,stdout=stdout,stderr=stderr)
after=p.source_rows();p.write_new(out/"source-after.json",after)
artifacts=[]
for line in (out/"build.stdout.jsonl").read_text(encoding="utf-8").splitlines():
    try:row=json.loads(line)
    except json.JSONDecodeError:continue
    if row.get("reason")=="compiler-artifact" and row.get("target",{}).get("name")=="playable_battle_startup" and row.get("executable"):
        artifacts.append(Path(row["executable"]))
assert len(artifacts)==1,(result.returncode,artifacts)
exe=artifacts[0]
record=dict(passed=result.returncode==0 and before==after,exit_code=result.returncode,
    source_unchanged_during_build=before==after,seconds=time.monotonic()-start,command=command,
    executable=p.record(exe),source_files=after,
    scope="Strict same-owner startup integration test executable; distinct from the default-profile packaged GUI client. No gameplay acceptance from compilation.")
p.write_new(out/"build-record.json",record)
print(json.dumps({key:value for key,value in record.items() if key!="source_files"}),flush=True)
raise SystemExit(0 if record["passed"] else 1)
