"""Package Development 24 (the Bevy game) from an already built executable.

Never builds or launches the game and never touches older releases.
Every output is exclusive-create: an existing release/Development-24 is refused.

    python research/development24/package.py stage --exe <path> --readme <path> [--eos-sdk <dir> --eos-config <dir>]
    python research/development24/package.py seal
    python research/development24/package.py verify
"""
from pathlib import Path
from datetime import datetime, timezone
import argparse, hashlib, json, shutil, subprocess, zipfile

ROOT = Path(__file__).resolve().parents[2]
RELEASE = ROOT / "release/Development-24"
EXE = "AxisAlliesBevy.exe"
ARCHIVE = "AxisAllies-Bevy-Development-24-TEST.zip"
# Caches built from the player's own installed game, bundled so the first
# start is fast; the game rebuilds a cache whose source key does not match.
# They are taken from CACHE_ROOT, a copy of assets-cache holding only the
# caches this build's source writes.
CACHE_ROOT = ROOT / "target/dev24-cache"
CACHE = ["assets-cache/animation", "assets-cache/video", "assets-cache/model-materials.json",
         "assets-cache/aa-bevy", "assets-cache/aa-play"]
# Online play (a build made with the client's eos feature): Epic's library beside the
# program, its end-user notice, and the product's settings in eos/. The settings are
# copied as bytes and never read here.
EOS_LIBRARY = "EOSSDK-Win64-Shipping.dll"
EOS_NOTICE = ROOT / "crates/aa-eos/END-USER-NOTICE.txt"
EOS_SETTINGS = ["openallies-ids.txt", "client-secret.txt"]
LAUNCHERS = {
    "Start.cmd": ['"%~dp0AxisAlliesBevy.exe"', "if errorlevel 1 pause"],
    # A second player on the same PC, to try LAN or online play: its own profile
    # (nickname, settings) in second-player\, and a new anonymous online player.
    "Start-Second-Player.cmd": ['set "APPDATA=%~dp0second-player"', 'if not exist "%APPDATA%" mkdir "%APPDATA%"',
                                "set AA_EOS_NEW_DEVICE_ID=1", '"%~dp0AxisAlliesBevy.exe"', "if errorlevel 1 pause"],
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
    need(not RELEASE.exists(), "release/Development-24 already exists; never overwritten")
    exe = Path(args.exe).resolve()
    need(exe.is_file() and exe.is_relative_to(ROOT.resolve()), "executable must be a workspace file")
    need(exe.read_bytes()[:2] == b"MZ", "not a PE executable")
    readme = Path(args.readme).read_text(encoding="utf-8")
    need((args.eos_sdk is None) == (args.eos_config is None), "--eos-sdk and --eos-config go together")
    eos = None
    if args.eos_sdk:
        library = Path(args.eos_sdk) / "SDK/Bin" / EOS_LIBRARY
        if not library.is_file():
            library = Path(args.eos_sdk) / "Bin" / EOS_LIBRARY
        need(library.is_file(), f"no {EOS_LIBRARY} in the SDK folder")
        need(library.read_bytes()[:2] == b"MZ", "the SDK library is not a PE file")
        settings = [Path(args.eos_config) / name for name in EOS_SETTINGS]
        need(all(p.is_file() and p.stat().st_size > 0 for p in settings), "missing online settings")
        need(EOS_NOTICE.is_file(), "missing the end-user notice")
        need(b"aa-eos" in exe.read_bytes() or b"EOS_Platform_Create" in exe.read_bytes(), "executable was built without the eos feature")
        eos = (library, settings)
    need("DIAGNOSTIC / UNFINISHED" in readme or "TEST BUILD" in readme, "README must label the build as a test build")
    RELEASE.mkdir(parents=True, exist_ok=False)
    copy_new(exe, RELEASE / EXE)
    copy_new(args.readme, RELEASE / "README.txt")
    for rel in CACHE:
        src = CACHE_ROOT / rel
        need(src.exists(), f"missing {rel}")
        files = [src] if src.is_file() else sorted(p for p in src.rglob("*") if p.is_file())
        for f in files:
            copy_new(f, RELEASE / f.relative_to(CACHE_ROOT))
    if eos:
        library, settings = eos
        copy_new(library, RELEASE / EOS_LIBRARY)
        copy_new(EOS_NOTICE, RELEASE / "EPIC-ONLINE-SERVICES-NOTICE.txt")
        for p in settings:
            copy_new(p, RELEASE / "eos" / p.name)
    for name, lines in LAUNCHERS.items():
        with (RELEASE / name).open("x", encoding="ascii", newline="") as stream:
            stream.write("@echo off\r\ncd /d \"%~dp0\"\r\n" + "".join(l + "\r\n" for l in lines))
    staged = {
        "schema": 1, "release": "Development-24", "status": "staged",
        "staged_utc": datetime.now(timezone.utc).isoformat(),
        "executable_sha256": sha(RELEASE / EXE),
        "source_commit": git("rev-parse", "HEAD"),
        "source_uncommitted_files": len([l for l in git("status", "--porcelain").splitlines() if l.strip()]),
        "online_play": bool(eos),
        "external_assets": [
            "Original Data.rwd in the game installation (<game root>)",
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
        "schema": 1, "release": "Development-24", "archive": ARCHIVE, "status": "TEST BUILD",
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
    p.add_argument("--eos-sdk")
    p.add_argument("--eos-config")
    sub.add_parser("seal")
    sub.add_parser("verify")
    args = parser.parse_args()
    {"stage": stage, "seal": seal, "verify": verify}[args.phase](args)


if __name__ == "__main__":
    main()
