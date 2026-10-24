"""Package Development 17 (the Bevy game) from an already built executable.

Never builds or launches the game and never touches older releases.
Every output is exclusive-create: an existing release/Development-17 is refused.

    python research/development17/package.py stage --exe <path> --readme <path>
    python research/development17/package.py seal
    python research/development17/package.py verify
"""
from pathlib import Path
from datetime import datetime, timezone
import argparse, hashlib, json, shutil, subprocess, zipfile

ROOT = Path(__file__).resolve().parents[2]
RELEASE = ROOT / "release/Development-17"
EXE = "AxisAlliesBevy.exe"
ARCHIVE = "AxisAllies-Bevy-Development-17-TEST.zip"
# Caches built from the player's own installed game, bundled so the first
# start is fast; the game rebuilds aa-play's caches if they are missing.
CACHE = ["assets-cache/animation", "assets-cache/video", "assets-cache/model-materials.json",
         "assets-cache/aa-bevy", "assets-cache/aa-play"]
LAUNCHERS = {
    "Start.cmd": ['"%~dp0AxisAlliesBevy.exe"', "if errorlevel 1 pause"],
}


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


def git(*args):
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True).stdout.strip()


def stage(args):
    need(not RELEASE.exists(), "release/Development-17 already exists; never overwritten")
    exe = Path(args.exe).resolve()
    need(exe.is_file() and exe.is_relative_to(ROOT.resolve()), "executable must be a workspace file")
    need(exe.read_bytes()[:2] == b"MZ", "not a PE executable")
    readme = Path(args.readme).read_text(encoding="utf-8")
    need("DIAGNOSTIC / UNFINISHED" in readme or "TEST BUILD" in readme, "README must label the build as a test build")
    RELEASE.mkdir(parents=True, exist_ok=False)
    copy_new(exe, RELEASE / EXE)
    copy_new(args.readme, RELEASE / "README.txt")
    for rel in CACHE:
        src = ROOT / rel
        need(src.exists(), f"missing {rel}")
        files = [src] if src.is_file() else sorted(p for p in src.rglob("*") if p.is_file())
        for f in files:
            copy_new(f, RELEASE / f.relative_to(ROOT))
    for name, lines in LAUNCHERS.items():
        with (RELEASE / name).open("x", encoding="ascii", newline="") as stream:
            stream.write("@echo off\r\ncd /d \"%~dp0\"\r\n" + "".join(l + "\r\n" for l in lines))
    staged = {
        "schema": 1, "release": "Development-17", "status": "staged",
        "staged_utc": datetime.now(timezone.utc).isoformat(),
        "executable_sha256": sha(RELEASE / EXE),
        "source_commit": git("rev-parse", "HEAD"),
        "source_uncommitted_files": len([l for l in git("status", "--porcelain").splitlines() if l.strip()]),
        "external_assets": [
            "Original Data.rwd in the game installation (E:/Games/Axis and Alies)",
            "reconstruction/ beside rust-rebuild (original Data, converted assets)",
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
        "schema": 1, "release": "Development-17", "archive": ARCHIVE, "status": "TEST BUILD",
        "standalone_game": False, "executable_sha256": staged["executable_sha256"],
        "source_commit": staged["source_commit"], "external_assets": staged["external_assets"],
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
    sub.add_parser("seal")
    sub.add_parser("verify")
    args = parser.parse_args()
    {"stage": stage, "seal": seal, "verify": verify}[args.phase](args)


if __name__ == "__main__":
    main()
