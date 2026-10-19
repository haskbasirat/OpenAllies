"""Real GUI run of the selection/order input on the battle frame (copy of
run_camera_gui.py with mouse steps instead of camera keys).

After the skirmish start it repeatedly (at --first and then every --every
seconds) captures a screenshot, left-clicks the battle frame centre, drags a
left-button box across it (one pointer event per UI frame, as the --ui-script
hook injects them), shift-drags, right-clicks, and screenshots again. The
client logs every orders UI frame that carried button messages ("native
orders frame: ...") and each distinct refusal once. Kills only its own child.
"""
from pathlib import Path
import argparse, hashlib, json, os, subprocess, time, shutil

ROOT = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("map", choices=("berlin", "firefight"))
parser.add_argument("tag")
parser.add_argument("--exe", required=True)
parser.add_argument("--seconds", type=int, default=1500)
parser.add_argument("--first", type=int, default=600)
parser.add_argument("--every", type=int, default=150)
parser.add_argument("--presses", type=int, default=40)
parser.add_argument("--census", action="store_true")
parser.add_argument("--look-ahead", action="store_true")
args = parser.parse_args()
EXE = Path(args.exe).resolve()
out = ROOT / "qa-output/phase8-orders-camera" / args.tag / args.map
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

steps = [{"wait": 2, "click": [1266, 276]}, {"wait": .4, "click": [1266, 332]},
         {"wait": .3, "click": [610, 439]}, {"text": "Regression Tester"},
         {"wait": .2, "click": [625, 418]}]
if args.map == "berlin":
    steps += [{"wait": .2, "click": [600, 442]}]
else:
    steps += [{"move": [625, 600]}, {"scroll": -84}, {"wait": .6, "click": [600, 611]}]
steps += [{"wait": .3, "shot": str(out / "before-start.png")},
          {"wait": .2, "log": "Click actual Test skirmish startup", "click": [482, 549]},
          {"wait": 2, "move": [640, 360]}]
elapsed = 2.0
t = args.first
n = 0
while t + 30 < args.seconds:
    steps.append({"wait": max(t - elapsed, 0), "shot": str(out / f"ord{n:02d}-a-before.png"),
                  "log": f"orders sequence {n} at +{t}s: before"})
    steps.append({"wait": 0.5, "click": [640, 360], "log": f"orders sequence {n}: left click (640,360)"})
    steps.append({"wait": 1.0, "drag": [480, 260, 820, 480], "log": f"orders sequence {n}: left drag (480,260)-(820,480)"})
    steps.append({"wait": 1.0, "shot": str(out / f"ord{n:02d}-b-after-drag.png"), "log": f"orders sequence {n}: after drag"})
    steps.append({"wait": 0.5, "drag": [700, 300, 560, 420], "mods": "shift", "log": f"orders sequence {n}: shift drag (700,300)-(560,420)"})
    steps.append({"wait": 1.0, "click": [640, 360], "button": "right", "log": f"orders sequence {n}: right click (640,360)"})
    steps.append({"wait": 1.0, "shot": str(out / f"ord{n:02d}-c-after-right.png"), "log": f"orders sequence {n}: after right click"})
    elapsed = t + 6.5
    t += args.every
    n += 1
steps.append({"wait": max(args.seconds - elapsed, 1), "shot": str(out / "final.png"), "log": "final screenshot"})
steps.append({"wait": 1, "log": "Request application close after camera capture", "quit": True})
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
        code = process.wait(timeout=args.seconds + 300)
    except subprocess.TimeoutExpired:
        timed_out = True
        process.kill()
        code = process.wait()
result = dict(map=args.map, executable=str(EXE), executable_sha256=hashlib.sha256(EXE.read_bytes()).hexdigest(),
              seconds=time.monotonic() - started, exit_code=code, timed_out=timed_out,
              screenshots=[p.name for p in sorted(out.glob("*.png"))])
(out / "result.json").write_text(json.dumps(result, indent=1), encoding="utf-8")
print(json.dumps(result), flush=True)
