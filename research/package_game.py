"""Package the playable Rust game into a new release folder.

Usage: python research/package_game.py --exe target/release/AxisAlliesRust.exe --out release/Game-1.0
Copies the executable, the asset cache it needs (material sidecar + animation rigs),
documentation and a launcher, then writes a SHA-256 manifest. Never overwrites an
existing release folder.
"""
import argparse
import hashlib
import json
import os
import shutil
import sys
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--exe", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--version", default="1.0")
    ap.add_argument("--zip", action="store_true")
    args = ap.parse_args()
    exe = os.path.join(ROOT, args.exe) if not os.path.isabs(args.exe) else args.exe
    out = os.path.join(ROOT, args.out) if not os.path.isabs(args.out) else args.out
    if os.path.exists(out):
        sys.exit(f"{out} already exists; choose a new release folder")
    os.makedirs(out)
    shutil.copy2(exe, os.path.join(out, "AxisAlliesRust.exe"))
    cache_src = os.path.join(ROOT, "assets-cache")
    cache_dst = os.path.join(out, "assets-cache")
    os.makedirs(cache_dst)
    shutil.copy2(os.path.join(cache_src, "model-materials.json"), cache_dst)
    shutil.copytree(os.path.join(cache_src, "animation"), os.path.join(cache_dst, "animation"))
    for doc in ["docs/PLAYING.md", "docs/GAME-DESIGN.md"]:
        shutil.copy2(os.path.join(ROOT, doc), out)
    with open(os.path.join(out, "Play Axis & Allies.cmd"), "w", newline="\r\n") as f:
        f.write('@echo off\r\ncd /d "%~dp0"\r\nstart "" "%~dp0AxisAlliesRust.exe" %*\r\n')
    with open(os.path.join(out, "README.txt"), "w", newline="\r\n") as f:
        f.write(f"""AXIS & ALLIES - RUST EDITION {args.version}

Double-click "Play Axis & Allies.cmd" (or AxisAlliesRust.exe).

This folder must stay inside the Axis and Alies installation (it reads the original
game assets from ..\\..\\..\\reconstruction and the original Data.rwd-derived files).
Keep assets-cache beside the executable.

See PLAYING.md for how to play and the controls.
Saves/settings: %APPDATA%\\AxisAlliesRust
Command line: AxisAlliesRust.exe --help
""")
    manifest = []
    for base, _, files in os.walk(out):
        for name in sorted(files):
            p = os.path.join(base, name)
            manifest.append({"path": os.path.relpath(p, out).replace("\\", "/"), "size": os.path.getsize(p), "sha256": sha256(p)})
    with open(os.path.join(out, "manifest.json"), "w") as f:
        json.dump({"version": args.version, "files": manifest}, f, indent=1)
    if args.zip:
        z = out.rstrip("\\/") + ".zip"
        with zipfile.ZipFile(z, "w", zipfile.ZIP_DEFLATED) as zf:
            for base, _, files in os.walk(out):
                for name in files:
                    p = os.path.join(base, name)
                    zf.write(p, os.path.join(os.path.basename(out), os.path.relpath(p, out)))
        print("zip", z, os.path.getsize(z))
    print("packaged", out, len(manifest), "files")


if __name__ == "__main__":
    main()
