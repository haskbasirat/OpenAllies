"""Root-invoked default-profile client build; importing this module does nothing."""
import argparse
import os
from pathlib import Path
import subprocess
import time

import package_animation01 as p


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", default=str(p.QA / "build01"))
    args = parser.parse_args()
    out = Path(args.output).resolve()
    p.need(out.is_relative_to(p.QA.resolve()) and out != p.QA.resolve(), "Build evidence must stay in a new animation QA child")
    out.mkdir(parents=True, exist_ok=False)
    before = p.source_rows()
    p.write_new(out / "source-before.json", before)
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(p.ROOT / "target")
    for name in ("CARGO_PROFILE_RELEASE_LTO", "CARGO_PROFILE_RELEASE_CODEGEN_UNITS"):
        env.pop(name, None)
    command = ["cargo", "build", "-p", "aa-client", "--bin", "AxisAlliesRust", "--target",
               "x86_64-pc-windows-msvc", "--release", "--locked", "--offline", "-j", "2"]
    started = time.monotonic()
    with (out / "build.log").open("x", encoding="utf-8") as log:
        result = subprocess.run(command, cwd=p.ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, shell=False)
    seconds = time.monotonic() - started
    after = p.source_rows()
    p.write_new(out / "source-after.json", after)
    exe = p.ROOT / "target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe"
    evidence = [out / name for name in ("source-before.json", "source-after.json", "build.log")]
    evidence.append(Path(__file__).resolve())
    record = dict(passed=result.returncode == 0 and before == after and exe.is_file(),
        source_unchanged_during_build=before == after, exit_code=result.returncode, seconds=seconds,
        target="x86_64-pc-windows-msvc", profile="release", profile_overrides={}, command=command,
        rustc_version=subprocess.check_output(["rustc", "-vV"], cwd=p.ROOT, text=True).strip(),
        source_files=after, evidence_files=[p.record(path) for path in evidence])
    if exe.is_file():
        record.update(candidate=str(exe), executable_sha256=p.sha(exe))
    p.write_new(out / "build-record.json", record)
    print({key: value for key, value in record.items() if key not in ("source_files", "evidence_files")})
    raise SystemExit(0 if record["passed"] else 1)


if __name__ == "__main__":
    main()
