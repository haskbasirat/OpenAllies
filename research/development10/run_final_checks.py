"""Root's bounded final source/build checks. Logs stay outside the release."""
from pathlib import Path
import hashlib, json, os, subprocess, sys, time
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "qa-output/development10/final"
OUT.mkdir(parents=True, exist_ok=True)
def sha(path):
    with path.open("rb") as f: return hashlib.file_digest(f,"sha256").hexdigest()
def sources():
    paths = [ROOT/"Cargo.toml", ROOT/"Cargo.lock"]
    paths += [p for p in (ROOT/"crates").rglob("*") if p.is_file() and "target" not in p.relative_to(ROOT/"crates").parts]
    return [{"path":p.relative_to(ROOT).as_posix(),"bytes":p.stat().st_size,"sha256":sha(p)} for p in sorted(paths)]
def save(name,obj):
    with (OUT/name).open("x",encoding="utf-8") as f: json.dump(obj,f,indent=2); f.write("\n")
before = sources()
save("source-before.json",before)
env=os.environ.copy()
env.update(CARGO_TARGET_DIR="target/native-runtime", AA_GAME_ROOT=str(ROOT.parent), AA_METAGAME_EXPORT="0",
           CARGO_PROFILE_RELEASE_LTO="false", CARGO_PROFILE_RELEASE_CODEGEN_UNITS="16")
commands=[
    ("workspace-all-targets",["cargo","check","--workspace","--all-targets","--release","--locked","--offline","-j","4"]),
    ("runtime-lib",["cargo","test","-p","aa-runtime","--lib","--release","--locked","--offline","-j","4"]),
    ("mapworld-lib",["cargo","test","-p","aa-mapworld","--lib","--release","--locked","--offline","-j","4"]),
    ("spatial-lib",["cargo","test","-p","aa-spatial","--lib","--release","--locked","--offline","-j","4"]),
    ("actors-state",["cargo","test","-p","aa-actors","--lib","--release","--locked","--offline","-j","4","state::"]),
    ("source-record-integration",["cargo","test","-p","aa-runtime","--test","source_completed_restoration","--test","element_attach_restore","--release","--locked","--offline","-j","4","--","--include-ignored"]),
]
results=[]
for label, command in commands:
    started=time.time()
    with (OUT/(label+".log")).open("x",encoding="utf-8") as log:
        p=subprocess.run(command,cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
    results.append({"name":label,"command":command,"exit_code":p.returncode,"seconds":time.time()-started,
                    "log":str((OUT/(label+".log")).relative_to(ROOT)).replace("\\","/")})
    print(json.dumps(results[-1]),flush=True)
    if p.returncode: break
after=sources()
save("source-after.json",after)
save("report.json",{"passed":all(r["exit_code"]==0 for r in results) and len(results)==len(commands) and before==after,
    "test_profile_overrides":{"release.lto":False,"release.codegen-units":16},
    "source_unchanged":before==after,"checks":results,"scope":"Compilation and bounded native/storage regressions; not battle acceptance"})
sys.exit(0 if len(results)==len(commands) and all(r["exit_code"]==0 for r in results) and before==after else 1)
