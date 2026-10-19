"""Package Development 11 from an already built client executable.

Never builds or launches the client and never touches older releases.
Every output is exclusive-create: an existing release/Development-11 is refused.

    python research/development11/package.py stage --exe <path> --readme <path> --checkpoint <path>
    python research/development11/package.py seal
    python research/development11/package.py verify

The runtime cache set is the same closure Development 10 shipped
(research/development10/package.py cache_closure), copied byte for byte.
"""
from pathlib import Path, PurePosixPath
from datetime import datetime, timezone
import argparse, hashlib, importlib.util, json, shutil, subprocess, sys, zipfile

ROOT = Path(__file__).resolve().parents[2]
RELEASE = ROOT / "release/Development-11"
EXE = "AxisAlliesRust.exe"
ARCHIVE = "AxisAlliesRust-Development-11-DIAGNOSTIC.zip"
MARKERS = ("DIAGNOSTIC / UNFINISHED", "BATTLE STATUS:")


def need(ok, message):
    if not ok:
        raise SystemExit(f"refused: {message}")


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def record(path, base):
    path = Path(path).resolve()
    return {"path": path.relative_to(Path(base).resolve()).as_posix(), "bytes": path.stat().st_size, "sha256": sha(path)}


def copy_new(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    with Path(source).open("rb") as src, target.open("xb") as dst:
        shutil.copyfileobj(src, dst)
    need(sha(source) == sha(target), f"copy hash mismatch: {target}")


def dev10_package():
    path = ROOT / "research/development10/package.py"
    spec = importlib.util.spec_from_file_location("dev10_package", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def launcher(lines):
    return "@echo off\r\ncd /d \"%~dp0\"\r\n" + "".join(line + "\r\n" for line in lines)


LAUNCHERS = {
    "Start.cmd": ['"%~dp0AxisAlliesRust.exe"'],
    # Diagnostic look-ahead: startup records every unported gap instead of
    # stopping at the first, so later stages (the battle frame) can be seen.
    "Start Diagnostic Look-Ahead.cmd": [
        'set "AA_STARTUP_CENSUS=%TEMP%\\AxisAlliesRust-startup-census.json"',
        "set AA_STARTUP_CENSUS_CONTINUE=1",
        '"%~dp0AxisAlliesRust.exe"',
        "echo Startup census written to %TEMP%\\AxisAlliesRust-startup-census.json",
        "pause",
    ],
}


def git(*args):
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True).stdout.strip()


def stage(args):
    need(not RELEASE.exists(), "release/Development-11 already exists; never overwritten")
    exe = Path(args.exe).resolve()
    need(exe.is_file() and exe.is_relative_to(ROOT.resolve()), "executable must be a workspace file")
    data = exe.read_bytes()
    need(data[:2] == b"MZ", "not a PE executable")
    docs = []
    for path, name in ((args.readme, "README.txt"), (args.checkpoint, "CHECKPOINT.txt")):
        text = Path(path).read_text(encoding="utf-8")
        need(all(m in text for m in MARKERS), f"{name} must label the build DIAGNOSTIC / UNFINISHED with its BATTLE STATUS")
        docs.append((Path(path), name))
    cache = dev10_package().cache_closure()
    RELEASE.mkdir(parents=True, exist_ok=False)
    copy_new(exe, RELEASE / EXE)
    for path, name in docs:
        copy_new(path, RELEASE / name)
    for row in cache:
        need(row["path"].startswith("assets-cache/"), "only runtime cache files belong in the payload")
        copy_new(ROOT / row["path"], RELEASE / PurePosixPath(row["path"]))
    for name, lines in LAUNCHERS.items():
        with (RELEASE / name).open("x", encoding="ascii", newline="") as stream:
            stream.write(launcher(lines))
    staged = {
        "schema": 1, "release": "Development-11", "status": "staged",
        "staged_utc": datetime.now(timezone.utc).isoformat(),
        "executable_sha256": sha(RELEASE / EXE),
        "source_commit": git("rev-parse", "HEAD"),
        "source_uncommitted_files": len([l for l in git("status", "--porcelain").splitlines() if l.strip()]),
        "build": {"command": args.build_command, "target": "x86_64-pc-windows-msvc", "profile": "release"},
        "external_assets": [
            "Original Data.rwd in the game installation (E:/Games/Axis and Alies)",
            "reconstruction/ beside rust-rebuild (original Data, converted assets, runtime-expanded-v14)",
        ],
        "files": [record(p, RELEASE) for p in sorted(RELEASE.rglob("*")) if p.is_file()],
    }
    with (RELEASE / "STAGING.json").open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(staged, stream, indent=2)
    print(json.dumps({"staged": str(RELEASE), "files": len(staged["files"]), "exe_sha256": staged["executable_sha256"]}))


def seal(_args):
    need(RELEASE.is_dir() and not (RELEASE / "manifest.json").exists(), "missing stage or already sealed")
    staged = json.loads((RELEASE / "STAGING.json").read_text(encoding="utf-8"))
    for row in staged["files"]:
        need(sha(RELEASE / row["path"]) == row["sha256"], f"changed after staging: {row['path']}")
    manifest = {
        "schema": 1, "release": "Development-11", "archive": ARCHIVE,
        "status": "DIAGNOSTIC / UNFINISHED",
        "standalone_game": False, "playable_battle": False, "full_game_parity": False,
        "executable_sha256": staged["executable_sha256"], "source_commit": staged["source_commit"],
        "external_assets": staged["external_assets"],
        "settings_location": "%APPDATA%/AxisAlliesRust/settings.json",
        "files": [record(p, RELEASE) for p in sorted(RELEASE.rglob("*")) if p.is_file()],
    }
    with (RELEASE / "manifest.json").open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(manifest, stream, indent=2)
    with zipfile.ZipFile(RELEASE / ARCHIVE, "x", zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for path in sorted(RELEASE.rglob("*")):
            if path.is_file() and path.name != ARCHIVE:
                archive.write(path, path.relative_to(RELEASE).as_posix())
    verify(None)


def verify(_args):
    manifest = json.loads((RELEASE / "manifest.json").read_text(encoding="utf-8"))
    for row in manifest["files"]:
        need(sha(RELEASE / row["path"]) == row["sha256"], f"changed: {row['path']}")
    expected = {r["path"] for r in manifest["files"]} | {"manifest.json"}
    with zipfile.ZipFile(RELEASE / ARCHIVE) as archive:
        names = archive.namelist()
        need(set(names) == expected and len(names) == len(set(names)), "wrong ZIP payload")
        need(archive.testzip() is None, "ZIP CRC failure")
    print(json.dumps({"verified": True, "files": len(manifest["files"]), "zip_sha256": sha(RELEASE / ARCHIVE),
                      "zip_bytes": (RELEASE / ARCHIVE).stat().st_size}))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="phase", required=True)
    p = sub.add_parser("stage")
    p.add_argument("--exe", required=True)
    p.add_argument("--readme", required=True)
    p.add_argument("--checkpoint", required=True)
    p.add_argument("--build-command", default="")
    sub.add_parser("seal")
    sub.add_parser("verify")
    args = parser.parse_args()
    {"stage": stage, "seal": seal, "verify": verify}[args.phase](args)


if __name__ == "__main__":
    main()
