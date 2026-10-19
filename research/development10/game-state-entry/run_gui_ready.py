"""Real GUI run through the menus (same steps as research/development10/lead/
run_gui.py) that also presses the original LoadingScreen ReadyButton.

The ReadyButton (UI/Loading/loading_screen.tgi, View L=802 T=690, template
MenuPushButtonWidget) is shown by the LoadingScreen update 6E8C00 once the
Loading resource flag (+17) is set and UserReady (+16) is not. Its command
"UserReady" runs 6E80F0. The script cannot see the button, so it clicks the
button's centre at a fixed interval after --ready-after seconds; a click while
the button is hidden lands on no widget. Rect at 1440x900 (layout scales the
1024x768 reference by W/1024, H/768): L 1127.8 T 808.6 R 1383.8 B 862.5.
Uses the client's --ui-script hook. Kills only its own child process.
Screenshots are evidence to look at, not a pass by themselves.
"""
from pathlib import Path
import argparse, hashlib, json, os, shutil, subprocess, time

ROOT = Path(__file__).resolve().parents[3]

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("map", choices=("berlin", "firefight"))
parser.add_argument("tag", help="new run folder name")
parser.add_argument("--exe", required=True)
parser.add_argument("--seconds", type=int, default=2400)
parser.add_argument("--ready-after", type=int, default=600)
parser.add_argument("--ready-every", type=int, default=30)
parser.add_argument("--shot-every", type=int, default=60)
parser.add_argument("--census", action="store_true")
parser.add_argument("--look-ahead", action="store_true")
args = parser.parse_args()
EXE = Path(args.exe).resolve()

out = ROOT / "qa-output/development10/game-state-entry" / args.tag / args.map
out.mkdir(parents=True, exist_ok=False)
for folder in ("work", "appdata", "localappdata"):
    (out / folder).mkdir()

env = os.environ.copy()
env["APPDATA"] = str(out / "appdata")
env["LOCALAPPDATA"] = str(out / "localappdata")
if args.census:
    env["AA_STARTUP_CENSUS"] = str(out / "census.json")
    if args.look_ahead:
        env["AA_STARTUP_CENSUS_CONTINUE"] = "1"
env.setdefault("RUST_LOG", "info,wgpu_core=warn,wgpu_hal=warn,naga=warn,eframe=warn,egui_wgpu=warn")

READY = [1256, 835]
steps = [{"wait": 2, "click": [1266, 276]}, {"wait": .4, "click": [1266, 332]},
         {"wait": .3, "click": [610, 439]}, {"text": "Regression Tester"},
         {"wait": .2, "click": [625, 418]}]
if args.map == "berlin":
    steps += [{"wait": .2, "click": [600, 442]}]
else:
    steps += [{"move": [625, 600]}, {"scroll": -84}, {"wait": .6, "click": [600, 611]}]
steps += [{"wait": .3, "shot": str(out / "before-start.png")},
          {"wait": .2, "log": "Click actual Test skirmish startup", "click": [482, 549]}]
events = {}
for t in (1, 10, 30, 60, 120, 240, 360, 480):
    events.setdefault(t, []).append("shot")
for t in range(args.shot_every, args.seconds + 1, args.shot_every):
    events.setdefault(t, []).append("shot")
for t in range(args.ready_after, args.seconds + 1, args.ready_every):
    events.setdefault(t, []).append("ready")
last = 0
for t in sorted(events):
    if t > args.seconds:
        break
    first = True
    for kind in sorted(events[t]):
        wait = (t - last) if first else 0.2
        first = False
        if kind == "ready":
            steps.append({"wait": wait, "move": READY})
            steps.append({"wait": 0.1, "log": f"Press LoadingScreen ReadyButton at +{t}s", "click": READY})
            steps.append({"wait": 0.3, "move": [720, 450]})
        else:
            steps.append({"wait": wait, "shot": str(out / f"after-{t:04d}s.png"), "log": f"GUI screenshot at +{t}s"})
    last = t
steps.append({"wait": 1, "log": "Request application close after diagnostic capture", "quit": True})
(out / "script.json").write_text(json.dumps(steps, indent=1), encoding="utf-8")

run_exe = out / "bin" / EXE.name
run_exe.parent.mkdir()
shutil.copy2(EXE, run_exe)
command = [str(run_exe), "--ui-script", str(out / "script.json")]
started = time.monotonic(); timed_out = False
with (out / "stdout.txt").open("x", encoding="utf-8") as stdout, \
     (out / "stderr.txt").open("x", encoding="utf-8") as stderr:
    process = subprocess.Popen(command, cwd=out / "work", env=env, stdout=stdout, stderr=stderr)
    print(json.dumps(dict(map=args.map, pid=process.pid, phase="running", out=str(out))), flush=True)
    try:
        code = process.wait(timeout=args.seconds + 240)
    except subprocess.TimeoutExpired:
        timed_out = True
        process.kill()
        code = process.wait()
result = dict(map=args.map, executable=str(EXE), executable_sha256=hashlib.sha256(EXE.read_bytes()).hexdigest(),
              pid=process.pid, seconds=time.monotonic() - started, exit_code=code, timed_out=timed_out,
              screenshots=[p.name for p in sorted(out.glob("*.png"))], playable_battle=False,
              scope="Startup observation only. Inspect screenshots and logs.")
(out / "result.json").write_text(json.dumps(result, indent=1), encoding="utf-8")
print(json.dumps(result), flush=True)
