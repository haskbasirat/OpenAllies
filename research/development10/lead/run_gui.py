"""Click the real Test skirmish startup path in a freshly built client and
capture screenshots plus logs while startup work is pending.

Same menu steps and coordinates as validate_hotfix02_gui03.py, but it runs the
current build in target/ instead of a sealed release, and writes to a new
output folder each run. Uses the client's own --ui-script hook (no desktop
input). Kills only its own child process, after a bounded timeout.
Screenshots are evidence to look at, not a pass by themselves.
"""
from pathlib import Path
import argparse, hashlib, json, os, subprocess, time

ROOT = Path(__file__).resolve().parents[3]
EXE = ROOT / "target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe"

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("map", choices=("berlin", "firefight"))
parser.add_argument("tag", help="new run folder name, e.g. run01")
parser.add_argument("--seconds", type=int, default=240)
parser.add_argument("--exe", help="client executable (default: the release build in target/)")
parser.add_argument("--census", action="store_true",
                    help="collect every failing definition into census.json instead of stopping at the first")
parser.add_argument("--look-ahead", action="store_true",
                    help="with --census, continue startup past failed definitions (diagnostic only)")
args = parser.parse_args()
if args.exe:
    EXE = Path(args.exe).resolve()

out = ROOT / "qa-output/development10/lead-gui" / args.tag / args.map
out.mkdir(parents=True, exist_ok=False)
for folder in ("work", "appdata", "localappdata"):
    (out / folder).mkdir()

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

env = os.environ.copy()
env["APPDATA"] = str(out / "appdata")
env["LOCALAPPDATA"] = str(out / "localappdata")
if args.census:
    env["AA_STARTUP_CENSUS"] = str(out / "census.json")
    if args.look_ahead:
        env["AA_STARTUP_CENSUS_CONTINUE"] = "1"
env.setdefault("RUST_LOG", "info,wgpu_core=warn,wgpu_hal=warn,naga=warn,eframe=warn,egui_wgpu=warn")

steps = [{"wait": 2, "click": [1266, 276]}, {"wait": .4, "click": [1266, 332]},
         {"wait": .3, "click": [610, 439]}, {"text": "Regression Tester"},
         {"wait": .2, "click": [625, 418]}]
if args.map == "berlin":
    steps += [{"wait": .2, "click": [600, 442]}]
else:
    steps += [{"move": [625, 600]}, {"scroll": -84}, {"wait": .6, "click": [600, 611]}]
steps += [{"wait": .3, "shot": str(out / "before-start.png")},
          {"wait": .2, "log": "Click actual Test skirmish startup", "click": [482, 549]}]
last = 0
for seconds in (1, 3, 10, 30, 60, 90, 125, 155, 190, 240, 300, 360, 420, 600, 900, 1200, 1500, 1800, 2400, 3000, 3600):
    if seconds > args.seconds:
        break
    steps.append({"wait": seconds - last, "shot": str(out / f"after-{seconds:03d}s.png"),
                  "log": f"GUI screenshot step at target +{seconds}s"})
    last = seconds
steps.append({"wait": 1, "log": "Request application close after diagnostic capture", "quit": True})
(out / "script.json").write_text(json.dumps(steps, indent=1), encoding="utf-8")

# Run a private copy so a rebuild is never blocked by this window. The PDB
# path embedded in the exe still points at the original build output.
import shutil
run_exe = out / "bin" / EXE.name
run_exe.parent.mkdir()
shutil.copy2(EXE, run_exe)
command = [str(run_exe), "--ui-script", str(out / "script.json")]
started = time.monotonic(); timed_out = False
with (out / "stdout.txt").open("x", encoding="utf-8") as stdout, \
     (out / "stderr.txt").open("x", encoding="utf-8") as stderr:
    process = subprocess.Popen(command, cwd=out / "work", env=env, stdout=stdout, stderr=stderr)
    print(json.dumps(dict(map=args.map, pid=process.pid, phase="running")), flush=True)
    try:
        code = process.wait(timeout=args.seconds + 120)
    except subprocess.TimeoutExpired:
        timed_out = True
        process.kill()
        code = process.wait()
result = dict(map=args.map, executable=str(EXE), executable_sha256=sha(EXE), pid=process.pid,
              seconds=time.monotonic() - started, exit_code=code, timed_out=timed_out,
              screenshots=[p.name for p in sorted(out.glob("*.png"))], playable_battle=False,
              scope="Startup observation only. Inspect screenshots and logs.")
(out / "result.json").write_text(json.dumps(result, indent=1), encoding="utf-8")
print(json.dumps(result), flush=True)
