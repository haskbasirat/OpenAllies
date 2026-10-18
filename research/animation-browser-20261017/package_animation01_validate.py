"""Root-invoked animation CLI/render/GUI capture; never accepts a battle.

Each invocation owns one isolated QA process/profile. GUI and GPU captures
require separate human semantic review; screenshots alone are not a pass.
No import-time work, build, stage, seal or desktop-wide user input occurs.
"""
import argparse
import json
import math
import os
from pathlib import Path
import subprocess
import time

import package_animation01 as p


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("catalog", "render", "gui"))
    parser.add_argument("--exe", default=str(p.RELEASE / p.EXE))
    parser.add_argument("--sha256", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--cases", help="Render JSON list: name,model,clip_index,time,backend")
    parser.add_argument("--script", help="Root-authored actual UI hook script; all shots stay in --output")
    parser.add_argument("--timeout", type=float, default=480)
    args = parser.parse_args()
    out, exe = Path(args.output).resolve(), Path(args.exe).resolve()
    p.need(out.is_relative_to(p.QA.resolve()) and out != p.QA.resolve(), "Use a new animation QA child")
    p.need(exe.is_relative_to(p.ROOT) and exe.is_file(), "Use the built workspace client")
    p.pe_info(exe, args.sha256)
    p.need(math.isfinite(args.timeout) and args.timeout > 0, "Positive finite timeout required")
    manifest_file = exe.parent / p.ANIMATION_DESTINATION / "animation-browser-manifest.json"
    p.need(manifest_file.is_file(), "Validate the installed package closure; do not silently use legacy developer caches")
    manifest = p.read(manifest_file)
    expected = {row["source"].casefold(): row for row in manifest["rigs"]}
    p.need(len(expected) == len(manifest["rigs"]) and expected, "Duplicate/empty installed browser manifest")
    out.mkdir(parents=True, exist_ok=False)
    for name in ("work", "appdata", "localappdata"):
        (out / name).mkdir()
    env = os.environ.copy()
    env["APPDATA"], env["LOCALAPPDATA"] = str(out / "appdata"), str(out / "localappdata")
    assets = str(p.ROOT.parent / "reconstruction")

    def run(name, arguments):
        name = str(p.relative(name))
        p.need("/" not in name, "Simple case name required")
        command = [str(exe), "--assets", assets, *arguments]
        started = time.monotonic()
        timed_out = False
        with (out / (name + ".stdout.txt")).open("x", encoding="utf-8") as stdout, (out / (name + ".stderr.txt")).open("x", encoding="utf-8") as stderr:
            process = subprocess.Popen(command, cwd=out / "work", env=env, stdout=stdout, stderr=stderr, shell=False)
            p.write_new(out / (name + ".process.json"), dict(pid=process.pid, command=command, candidate_sha256=args.sha256))
            try:
                code = process.wait(timeout=args.timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
                process.kill()  # Only the exact child process launched above.
                code = process.wait()
        result = dict(name=name, candidate_sha256=args.sha256, command=command,
            exit_code=code, timed_out=timed_out, seconds=time.monotonic() - started,
            scope="Isolated animation browser process/capture only; no battle/native full-FX acceptance")
        p.write_new(out / (name + ".process-result.json"), result)
        p.need(not timed_out and code == 0 and p.sha(exe) == args.sha256, "Client failed/timed out/changed; raw outcome retained")
        return result

    observations = []
    if args.phase == "catalog":
        result = run("catalog", ["--list-animations"])
        catalog = p.read(out / "catalog.stdout.txt")
        actual = {row["source"].casefold(): row for row in catalog["rigs"]}
        p.need(actual.keys() == expected.keys() and len(actual) == len(catalog["rigs"]), "CLI did not discover the exact installed full browser closure")
        for name, rig in expected.items():
            row = actual[name]
            p.need(row["sha256"] == rig["sha256"] and row["clips"] == [clip["name"] for clip in rig["clips"]], "CLI clip/file identity differs from installed manifest")
            #Rust canonicalize uses Windows' extended-length \\?\ namespace;
            #compare actual file identity rather than those spelling variants.
            p.need(Path(row["path"]).samefile(manifest_file.parent / p.relative(rig["file"])), "CLI resolved a different/legacy cache")
        unavailable = {row["source"].casefold(): row["reason"] for row in catalog["unavailable_sources"]}
        unsupported = {row["source"].casefold(): row["catalog_only_reason"] for row in manifest["sources"]
            if not row["in_exported_timeline"] and (row["kind"] == "kf" or row["has_embedded_controllers"])}
        p.need(unavailable == unsupported and len(unavailable) == len(catalog["unavailable_sources"]), "CLI omitted or altered explicit unavailable animation sources")
        observations.append(dict(process=result, rigs=len(actual), clips=sum(len(row["clips"]) for row in actual.values()), unavailable=len(unavailable)))
        p.write_new(out / "catalog.json", catalog)
    elif args.phase == "render":
        p.need(args.cases is not None, "Supply actual representative render cases")
        cases_path = Path(args.cases).resolve()
        p.need(cases_path.is_relative_to(p.ROOT), "Cases must stay in workspace")
        cases = p.read(cases_path)
        p.need(isinstance(cases, list) and cases, "Nonempty representative cases required")
        for case in cases:
            rig = expected.get(case["model"].casefold())
            p.need(rig is not None and type(case["clip_index"]) is int and 0 <= case["clip_index"] < len(rig["clips"])
                   and math.isfinite(case["time"]) and case["time"] >= 0 and case["backend"] in ("vulkan", "dx12"), "Case must select a real installed model/clip/time/backend")
            image = out / (str(p.relative(case["name"])) + ".png")
            result = run(case["name"], ["--animated", rig["source"], "--clip-index", str(case["clip_index"]),
                "--time", str(case["time"]), "--backend", case["backend"], "--render-check", str(image), "--render-frames", "2"])
            report = p.read(out / (case["name"] + ".stdout.txt"))
            p.need(image.is_file() and report["triangles"] > 0 and report["render_frames"] == 2, "Render process did not produce the requested geometry/image")
            p.write_new(out / (case["name"] + ".json"), report)
            observations.append(dict(case=case, process=result, image=p.record(image), semantic_review_required=True))
    else:
        p.need(args.script is not None, "Root supplies actual observed UI coordinates/script")
        script_path = Path(args.script).resolve()
        p.need(script_path.is_relative_to(p.ROOT), "UI script must stay in workspace")
        script = p.read(script_path)
        p.need(isinstance(script, list) and script, "Actual UI script required")
        shots = []
        for step in script:
            if "shot" in step:
                shot = Path(step["shot"]).resolve()
                p.need(shot.is_relative_to(out) and shot.suffix.lower() == ".png", "UI shots must stay in this owned QA directory")
                shots.append(shot)
        p.need(shots, "Capture actual GUI selection/playback for semantic review")
        #Script input belongs to Shell, which continues forwarding real egui
        #events after Editor opens App. Direct --viewer has no script hook.
        result = run("gui", ["--ui-script", str(script_path)])
        p.need(all(path.is_file() for path in shots), "GUI did not process every requested capture")
        observations.append(dict(process=result, script=p.record(script_path), screenshots=[p.record(path) for path in shots], semantic_review_required=True))
    p.write_new(out / "capture-summary.json", dict(candidate_sha256=args.sha256,
        animation_manifest_sha256=p.sha(manifest_file), phase=args.phase, observations=observations,
        process_checks_completed=True, semantic_review_required=args.phase != "catalog",
        playable_battle=False, full_game_parity=False, all_animations_playable_claimed=False))
    print(out / "capture-summary.json")


if __name__ == "__main__":
    main()
