"""Exercise actual menu Start with isolated profiles; captures require visual review.

Early diagnostic termination is allowed only after a genuine logged source
failure and its GUI capture. It kills only this helper's own child, is recorded
as such, and can never be treated as a successful map load.
"""
import argparse
import os
import subprocess
import time
import common as p


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", required=True)
    parser.add_argument("--name", required=True)
    parser.add_argument("--map", choices=("berlin", "firefight"), default="berlin")
    parser.add_argument("--seconds", type=int, default=480)
    parser.add_argument("--early-source-error", action="store_true")
    args = parser.parse_args()
    if not args.name.replace("-", "").isalnum() or not 30 <= args.seconds <= 900:
        raise RuntimeError("Simple output name and bounded duration required")
    build = p.read(p.ROOT / args.build)
    if not build["passed"]:
        raise RuntimeError("Use a successful frozen build")
    exe = p.ROOT / build["executable"]["path"]
    if p.sha(exe) != build["executable_sha256"]:
        raise RuntimeError("Candidate identity changed")
    out = p.QA / args.name
    out.mkdir(parents=True, exist_ok=False)
    for folder in ("work", "appdata", "localappdata"):
        (out / folder).mkdir()
    env = os.environ.copy()
    env["APPDATA"], env["LOCALAPPDATA"] = str(out / "appdata"), str(out / "localappdata")
    env["RUST_LOG"] = "info"
    steps = [dict(wait=2, click=[1266,276]), dict(wait=.4, click=[1266,332]),
        dict(wait=.3, click=[610,439]), dict(text="Map Load Tester"), dict(wait=.2, click=[625,418])]
    if args.map == "berlin":
        steps.append(dict(wait=.2, click=[600,442]))
    else:
        steps.extend([dict(move=[625,600]), dict(scroll=-84), dict(wait=.6, click=[600,611])])
    steps.extend([dict(wait=.3, shot=str(out / "before-start.png")),
        dict(wait=.2, log="Click actual Test skirmish startup", click=[482,549])])
    times = sorted({1, 3, 10, *range(30, args.seconds + 1, 30), args.seconds})
    prior = 0
    for seconds in times:
        steps.append(dict(wait=seconds-prior, shot=str(out / f"after-{seconds:03d}s.png"), log=f"Map load capture +{seconds}s"))
        prior = seconds
    steps.append(dict(wait=1, quit=True))
    p.write_new(out / "script.json", steps)
    command = [str(exe), "--assets", str(p.ROOT.parent / "reconstruction"), "--ui-script", str(out / "script.json")]
    started = time.monotonic()
    reason = None
    with (out / "stdout.txt").open("x", encoding="utf-8") as stdout, (out / "stderr.txt").open("x", encoding="utf-8") as stderr:
        process = subprocess.Popen(command, cwd=out / "work", env=env, stdout=stdout, stderr=stderr, shell=False)
        p.write_new(out / "process.json", dict(pid=process.pid, command=command, candidate_sha256=build["executable_sha256"]))
        print(dict(pid=process.pid, output=str(out)), flush=True)
        while process.poll() is None:
            if time.monotonic() - started > args.seconds + 90:
                reason = "bounded_timeout"
            elif args.early_source_error and (out / "after-010s.png").is_file():
                errors = (out / "stderr.txt").read_text(encoding="utf-8", errors="replace")
                if "native pre-World source preparation failed:" in errors:
                    reason = "reproduced_source_failure_after_gui_capture"
            if reason:
                process.kill()
                break
            time.sleep(.2)
        code = process.wait()
    p.write_new(out / "result.json", dict(candidate_sha256=build["executable_sha256"], map=args.map,
        seconds=time.monotonic()-started, command=command, exit_code=code, termination_reason=reason,
        screenshots=[p.record(path) for path in sorted(out.glob("*.png"))],
        first_battle_frame_marker="actual battle native frame presented" in (out / "stderr.txt").read_text(encoding="utf-8",errors="replace"),
        semantic_review_required=True, map_load_success_claimed=False, playable_battle=False, scope=__doc__))
    print(out / "result.json", flush=True)


if __name__ == "__main__":
    main()
