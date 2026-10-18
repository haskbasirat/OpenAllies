"""Run the staged Dev10's documented CLI previews with isolated preferences.

Original assets are read from the existing installation. This cannot accept a
playable battle, full native rendering parity or a machine without those assets.
"""
from pathlib import Path
import json, os, subprocess, time, hashlib
import package

ROOT = package.ROOT
OUT = ROOT / "qa-output/development10/acceptance01"
OUT.mkdir(exist_ok=False)
for folder in ("work", "appdata", "localappdata"):
    (OUT / folder).mkdir()
env = os.environ.copy()
env["APPDATA"] = str(OUT / "appdata")
env["LOCALAPPDATA"] = str(OUT / "localappdata")
exe = package.RELEASE / package.EXE
staged = package.read(package.RELEASE / "STAGING.json")
assert package.sha(exe) == staged["executable_sha256"]
rows = []

def run(name, args, assertion, timeout=300):
    start = time.monotonic()
    stdout, stderr, code, failure, document = "", "", None, None, None
    try:
        p = subprocess.run([str(exe), *args], cwd=OUT / "work", env=env,
            capture_output=True, text=True, encoding="utf-8", errors="replace",
            timeout=timeout, shell=False)
        stdout, stderr, code = p.stdout, p.stderr, p.returncode
        if code != 0:
            raise RuntimeError(f"exit code {code}")
        if name != "help":
            document = json.loads(stdout)
        if not assertion(stdout, document):
            raise RuntimeError("positive output assertion failed")
    except Exception as e:
        failure = str(e)
    (OUT / (name + ".stdout.txt")).write_text(stdout, encoding="utf-8")
    (OUT / (name + ".stderr.txt")).write_text(stderr, encoding="utf-8")
    if document is not None:
        package.write_new(OUT / (name + ".json"), document)
    row = dict(name=name, arguments=args, exit_code=code, passed=failure is None,
        error=failure, seconds=time.monotonic()-start)
    rows.append(row)
    package.write_new(OUT / (name + "-result.json"), row)
    print(json.dumps(row), flush=True)

run("help", ["--help"], lambda text, _: "Rust Development 10" in text)
run("asset-verification", ["--verify-assets"], lambda _, d:
    d["maps"] > 0 and d["placements"] > 0 and d["model_catalog"] > 0
    and "not gameplay parity" in d["status"])
run("mechanics-fragments", ["--mechanics-check"], lambda _, d: d["passed"] is True)
run("world-fragments", ["--world-check"], lambda _, d: d["passed"] is True)
run("record-loading", ["--map-restore-check", "DeveloperMaps/(3v3)Firefight"],
    lambda _, d: d["diagnostic_completed"] is True and d["battle_ready"] is False
    and isinstance(d["first_unsupported_step"], str), 480)
for name, args in (
    ("hq-vulkan", ["--model", "Buildings/GBR/GBRCorps/GBRCorps.nif", "--backend", "vulkan"]),
    ("map-vulkan", ["--map", "Campaign/axis01_crete", "--backend", "vulkan"]),
    ("infantry-dx12", ["--live-preview", "usa_infantry", "--backend", "dx12"]),
):
    image = OUT / (name + ".png")
    run(name, [*args, "--render-check", str(image), "--render-frames", "2"],
        lambda _, d: image.is_file() and d["triangles"] > 0
        and d["render_frames"] == 2 and "no gameplay parity claim" in d["status"], 480)
package.write_new(OUT / "cli-results.json", dict(executable_sha256=package.sha(exe),
    passed=all(r["passed"] for r in rows), checks=rows,
    scope=__doc__, existing_installation_required=True))
