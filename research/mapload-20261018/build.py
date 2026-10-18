"""Build one frozen application candidate; fast diagnostic profile is explicit."""
import argparse
import os
import subprocess
import time
import common as p


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--name", required=True)
    parser.add_argument("--final", action="store_true")
    args = parser.parse_args()
    if not args.name.replace("-", "").isalnum():
        raise RuntimeError("Simple output name required")
    out = p.QA / args.name
    out.mkdir(parents=True, exist_ok=False)
    before = p.sources()
    p.write_new(out / "source-before.json", before)
    env = os.environ.copy()
    command = ["cargo", "build", "-p", "aa-client", "--bin", "AxisAlliesRust", "--release", "--locked", "--offline", "-j", "2"]
    if args.final:
        env["CARGO_TARGET_DIR"] = str(p.ROOT / "target")
        env.pop("CARGO_PROFILE_RELEASE_LTO", None)
        env.pop("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", None)
        command.extend(["--target", "x86_64-pc-windows-msvc"])
        exe = p.ROOT / "target/x86_64-pc-windows-msvc/release/AxisAlliesRust.exe"
        overrides = {}
    else:
        env["CARGO_TARGET_DIR"] = str(p.ROOT / "target/s-client-dev09")
        env["CARGO_PROFILE_RELEASE_LTO"] = "false"
        env["CARGO_PROFILE_RELEASE_CODEGEN_UNITS"] = "16"
        exe = p.ROOT / "target/s-client-dev09/release/AxisAlliesRust.exe"
        overrides = dict(lto=False, codegen_units=16)
    start = time.monotonic()
    with (out / "build.log").open("x", encoding="utf-8") as log:
        result = subprocess.run(command, cwd=p.ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
    after = p.sources()
    p.write_new(out / "source-after.json", after)
    record = dict(passed=result.returncode == 0 and before == after and exe.is_file(),
        source_unchanged_during_build=before == after, command=command,
        seconds=time.monotonic() - start, exit_code=result.returncode, source_files=after,
        profile_overrides=overrides, target="x86_64-pc-windows-msvc" if args.final else "host-x64",
        scope="Final default release" if args.final else "Fast diagnostic candidate, separately identified; no release acceptance inferred",
        rustc_version=subprocess.check_output(["rustc", "-vV"], cwd=p.ROOT, text=True).strip())
    if exe.is_file():
        #Copy the exact candidate so a later iteration cannot replace a running QA input.
        destination = out / "AxisAlliesRust.exe"
        with exe.open("rb") as source, destination.open("xb") as target:
            import shutil
            shutil.copyfileobj(source, target)
        record.update(candidate=str(destination), executable_sha256=p.sha(destination), executable=p.record(destination))
    p.write_new(out / "build-record.json", record)
    print({key: value for key, value in record.items() if key != "source_files"})
    raise SystemExit(0 if record["passed"] else 1)


if __name__ == "__main__":
    main()
