"""Draft Dev10 packaging. Never builds/launches a client or changes old releases.

Only `plan` is authorized during the initial audit. Root must approve and supply
a built candidate, documents and acceptance evidence before `stage` / `seal`.
Every output is exclusive-create; a partial failed stage is preserved for review.
"""
from pathlib import Path, PurePosixPath
from datetime import datetime, timezone
import argparse
import hashlib
import json
import math
import shutil
import struct
import zipfile

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
RELEASE = ROOT / "release/Development-10-Hotfix-01"
QA = ROOT / "qa-output/development10"
EXE = "AxisAlliesRust.exe"
ARCHIVE = "AxisAlliesRust-Development-10-Hotfix-01.zip"
DIAGNOSTIC_ARCHIVE = "AxisAlliesRust-Development-10-Hotfix-01-DIAGNOSTIC.zip"
DIAGNOSTIC_MARKERS = ("DIAGNOSTIC / UNFINISHED", "BATTLE STATUS: UNAVAILABLE")


def need(ok, message):
    if not ok:
        raise RuntimeError(message)


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def write_new(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")


def relative(name):
    need(isinstance(name, str) and bool(name), "Missing relative path")
    path = PurePosixPath(name)
    need(not path.is_absolute() and str(path) == name and name != "."
         and not any(c in name for c in "\\:\0")
         and all(p not in (".", "..") and not p.endswith((".", " ")) for p in path.parts),
         f"Unsafe relative path: {name!r}")
    reserved = {"CON", "PRN", "AUX", "NUL"} | {f"{p}{i}" for p in ("COM", "LPT") for i in range(1, 10)}
    need(all(p.split(".")[0].upper() not in reserved for p in path.parts), "Windows device alias")
    return path


def inside(base, name):
    path = base / relative(name)
    need(path.resolve().is_relative_to(base.resolve()) and path.is_file(), f"Missing/outside file: {name}")
    return path


def record(path, base=None):
    if base is None:
        base = ROOT
    path, base = Path(path).resolve(), Path(base).resolve()
    return {"path": path.relative_to(base).as_posix(), "bytes": path.stat().st_size, "sha256": sha(path)}


def verify_rows(base, rows):
    names = set()
    for row in rows:
        name = str(relative(row["path"]))
        need(name.casefold() not in names, f"Duplicate/case-colliding path: {name}")
        names.add(name.casefold())
        path = inside(base, name)
        need(path.stat().st_size == row["bytes"] and sha(path) == row["sha256"], f"Changed file: {name}")


def source_rows():
    files = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    files += [p for p in (ROOT / "crates").rglob("*") if p.is_file() and "target" not in p.relative_to(ROOT / "crates").parts]
    return [record(p) for p in sorted(files)]


def pe_info(path, expected):
    data = Path(path).read_bytes()
    need(isinstance(expected, str) and len(expected) == 64 and expected == hashlib.sha256(data).hexdigest(), "Candidate SHA-256 mismatch")
    need(len(data) >= 64 and data[:2] == b"MZ", "Not a PE executable")
    at = struct.unpack_from("<I", data, 60)[0]
    need(at + 264 <= len(data) and data[at:at+4] == b"PE\0\0", "Truncated/invalid PE")
    machine, sections = struct.unpack_from("<HH", data, at + 4)
    optional_size, characteristics = struct.unpack_from("<HH", data, at + 20)
    opt = at + 24
    need(machine == 0x8664 and struct.unpack_from("<H", data, opt)[0] == 0x20B, "Expected AMD64 PE32+")
    need(characteristics & 0x20 and b"PerMonitorV2" in data and b"asInvoker" in data, "Missing LAA/DPI/privilege markers")
    table = opt + optional_size

    def offset(rva):
        for n in range(sections):
            s = table + n * 40
            need(s + 40 <= len(data), "Truncated PE section table")
            virtual_size, va, raw_size, raw = struct.unpack_from("<IIII", data, s + 8)
            if va <= rva < va + max(virtual_size, raw_size):
                pos = raw + rva - va
                need(pos < len(data), "PE RVA outside file")
                return pos
        raise RuntimeError("Unmapped PE import RVA")

    imports = []
    rva, length = struct.unpack_from("<II", data, opt + 120)
    if rva:
        pos = offset(rva)
        end = pos + length
        need(end <= len(data), "PE import table outside file")
        while pos + 20 <= end:
            fields = struct.unpack_from("<IIIII", data, pos)
            if fields == (0, 0, 0, 0, 0):
                break
            name = offset(fields[3])
            stop = data.find(b"\0", name)
            need(stop >= name, "Unterminated import name")
            imports.append(data[name:stop].decode("ascii"))
            pos += 20
    return {"architecture": "AMD64 PE32+", "large_address_aware": True,
            "per_monitor_v2": True, "as_invoker": True, "static_imports": sorted(set(imports)),
            "import_scope": "Static PE imports only; graphics/audio may load OS/driver DLLs dynamically."}


def cache_closure():
    """Runtime candidates; exclude reference JSON, export logs and staging copies."""
    cache = ROOT / "assets-cache"
    names = {"model-materials.json", "animation/manifest.json", "animation/game-rigs.json"}
    names.update(p.relative_to(cache).as_posix() for p in (cache / "animation").glob("*.rig.json"))
    game = read(cache / "animation/game-rigs.json")
    need(game.get("format") == "aa-game-rig-manifest-v1" and bool(game.get("rigs")), "Invalid game rig manifest")
    for rig in game["rigs"]:
        names.add("animation/" + str(relative(rig["file"])))
    for family in ("MainMenu", "MainMenuGlobe"):
        folder = f"video/{family}"
        names.add(folder + "/index.json")
        index = read(cache / folder / "index.json")
        need(bool(index.get("movies")), "Empty video index")
        for movie in index["movies"]:
            name = str(relative(movie["name"]))
            need("/" not in name and type(movie["frames"]) is int and movie["frames"] > 0
                 and math.isfinite(movie["fps"]) and movie["fps"] > 0, "Invalid video metadata")
            names.update(f"{folder}/{name}/{n:04}.jpg" for n in range(movie["frames"]))
    # These source-bound sidecars are opened at runtime by Model loading.
    # Include every canonical index reference, not just the older viewer cache.
    for family in ("model-native-graphs", "model-native-fields"):
        names.add(family + "/index.json")
        index = read(cache / family / "index.json")
        need(index.get("schema") == 2 and bool(index.get("by_source")), "Invalid model sidecar index")
        for table in ("graphs", "by_source"):
            for row in index.get(table, {}).values():
                name = str(relative(row["artifact"]))
                need("/" not in name, "Nested model sidecar artifact")
                artifact = inside(cache, family + "/" + name)
                need(sha(artifact) == row["sha256"], "Model sidecar hash differs from index")
                names.add(family + "/" + name)
    return [record(inside(cache, name)) for name in sorted(names)]


def protected_rows():
    rows = []
    for version in ("Development-08", "Development-09", "Development-10"):
        folder = ROOT / "release" / version
        manifest = read(folder / "manifest.json")
        verify_rows(folder, manifest["files"])
        rows.extend(record(p) for p in sorted(folder.rglob("*")) if p.is_file())
    return rows


def plan(output):
    need(not RELEASE.exists(), "Development-10-Hotfix-01 already exists; this helper never replaces it")
    output = Path(output).resolve()
    need(output.is_relative_to(HERE.resolve()) and output != HERE.resolve(), "Plan output must stay in this new research directory")
    payload = cache_closure()
    old = protected_rows()
    report = {"schema": 1, "status": "draft runtime payload plan; no release created", "generated_utc": datetime.now(timezone.utc).isoformat(),
              "release": str(RELEASE), "previous_release_files": old, "cache_files": payload,
              "external_assets": ["Original Data.rwd in the game installation", "reconstruction/game (maps and recovered metadata)",
                                  "reconstruction/assets/original/Data (rules, UI, fonts, NIFs, KF, localization)",
                                  "reconstruction/assets/converted (models, textures and terrain)",
                                  "reconstruction/native-game/runtime-expanded-v14 (the matching source overlay)",
                                  "Recovered Skins/Music/Speech under the same reconstruction tree"],
              "asset_discovery": "Use --assets <reconstruction>; implicit discovery searches CWD and EXE ancestors. Package alone is not standalone.",
              "battle_status": "No battle acceptance or launch flag inferred from this plan.",
              "audit_source_files": [record(ROOT / p) for p in ["crates/aa-client/src/main.rs", "crates/aa-client/src/live_scene.rs", "crates/aa-client/src/frontend.rs", "crates/aa-client/src/anim_bake.rs", "crates/aa-client/src/model_fixes.rs"]]}
    write_new(output, report)
    print(json.dumps({"cache_files": len(payload), "cache_bytes": sum(r["bytes"] for r in payload), "preserved_release_files": len(old), "release_created": False}))


def copy_new(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    with Path(source).open("rb") as src, target.open("xb") as dst:
        shutil.copyfileobj(src, dst)
    need(sha(source) == sha(target), "Copy hash mismatch")


def launcher(arguments, pause_after_exit=False):
    need(isinstance(arguments, list) and all(isinstance(a, str) and a.isascii()
         and not any(c in a for c in '"&|<>^%!\r\n\0') for a in arguments), "Unsafe launcher argument")
    need(type(pause_after_exit) is bool, "Invalid launcher pause flag")
    return '@echo off\r\ncd /d "%~dp0"\r\n"%~dp0AxisAlliesRust.exe"' + ''.join(' "' + a + '"' for a in arguments) + '\r\n' + ('pause\r\n' if pause_after_exit else '')


def stage(args):
    need(not RELEASE.exists(), "Development-10-Hotfix-01 already exists; staging never overwrites it")
    need(RELEASE.parent.resolve().is_relative_to(ROOT.resolve()), "Release parent escapes workspace")
    candidate = Path(args.exe).resolve()
    need(candidate.is_relative_to(ROOT.resolve()) and candidate.is_file(), "Candidate must be a workspace file")
    pe = pe_info(candidate, args.sha256)
    build = read(args.build_record)
    need(build.get("passed") is True and build.get("executable_sha256") == args.sha256
         and build.get("target") == "x86_64-pc-windows-msvc" and build.get("profile") == "release", "Wrong/unapproved build record")
    need(isinstance(build.get("command"), list) and bool(build["command"]) and bool(build.get("rustc_version")), "Missing build provenance")
    verify_rows(ROOT, build["evidence_files"])
    need(build["source_files"] == source_rows(), "Source differs from build record; freeze, rebuild and revalidate")
    payload = read(args.payload_plan)
    verify_rows(ROOT, payload["cache_files"])
    verify_rows(ROOT, payload["previous_release_files"])
    mode = getattr(args, "release_mode", "playable")
    need(mode in ("playable", "diagnostic"), "Unknown release mode")
    docs = [(inside(ROOT, args.readme), "README.txt"), (inside(ROOT, args.checkpoint), "CHECKPOINT.txt")]
    for path, _ in docs:
        text = path.read_text(encoding="utf-8-sig")
        need(text.strip() and "<FILL_" not in text and "{{" not in text, "Incomplete approved document")
        if mode == "diagnostic":
            need(all(marker in text for marker in DIAGNOSTIC_MARKERS), "Diagnostic README/checkpoint must explicitly label unfinished battle-unavailable status")
    launches = read(args.launchers)
    need(isinstance(launches, list), "Launchers must be a JSON list")
    seen = set()
    for row in launches:
        name = str(relative(row["name"]))
        need("/" not in name and name.endswith(".cmd") and name.casefold() not in seen, "Bad/duplicate launcher")
        seen.add(name.casefold()); launcher(row["arguments"], row.get("pause_after_exit", False))
    RELEASE.mkdir(exist_ok=False)
    copy_new(candidate, RELEASE / EXE)
    for path, name in docs:
        copy_new(path, RELEASE / name)
    for row in payload["cache_files"]:
        need(row["path"].startswith("assets-cache/"), "Only runtime cache files belong in the shared payload")
        copy_new(inside(ROOT, row["path"]), RELEASE / relative(row["path"]))
    for row in launches:
        with (RELEASE / row["name"]).open("x", encoding="ascii", newline="") as stream:
            stream.write(launcher(row["arguments"], row.get("pause_after_exit", False)))
    write_new(RELEASE / "evidence/build-record.json", build)
    write_new(RELEASE / "evidence/source-snapshot.json", {"files": build["source_files"], "scope": "Producer source fingerprint, not independent build attestation"})
    staged = {"schema": 1, "status": "staged-unvalidated", "release_mode": mode,
              "battle_status": "unavailable" if mode == "diagnostic" else "not-yet-accepted",
              "executable_sha256": args.sha256, "pe": pe,
              "packaging_helper_sha256": sha(__file__), "previous_release_files": payload["previous_release_files"],
              "source_files": build["source_files"], "external_assets": payload["external_assets"],
              "files": [record(p, RELEASE) for p in sorted(RELEASE.rglob("*")) if p.is_file()]}
    write_new(RELEASE / "STAGING.json", staged)
    need(sha(candidate) == sha(RELEASE / EXE) == args.sha256, "Candidate changed while copying")
    verify_rows(ROOT, staged["previous_release_files"])
    print(json.dumps({"phase": "staged-unvalidated", "executable_sha256": args.sha256, "folder": str(RELEASE)}))


def seal(args):
    seal_mode(args, diagnostic=False)


def seal_diagnostic(args):
    seal_mode(args, diagnostic=True)


def seal_mode(args, diagnostic):
    need(RELEASE.is_dir() and not (RELEASE / "manifest.json").exists()
         and not (RELEASE / ARCHIVE).exists() and not (RELEASE / DIAGNOSTIC_ARCHIVE).exists(), "Missing stage or existing seal")
    staged = read(RELEASE / "STAGING.json")
    mode = "diagnostic" if diagnostic else "playable"
    need(staged.get("release_mode", "playable") == mode, "Staged release mode differs; diagnostic and playable seals are separate")
    need(staged["packaging_helper_sha256"] == sha(__file__), "Packaging helper changed after staging")
    verify_rows(RELEASE, staged["files"])
    need({p.relative_to(RELEASE).as_posix() for p in RELEASE.rglob("*") if p.is_file()}
         == {r["path"] for r in staged["files"]} | {"STAGING.json"}, "Unexpected staged payload/profile/log")
    verify_rows(ROOT, staged["previous_release_files"])
    need(staged["source_files"] == source_rows(), "Source changed after staging")
    validation = read(args.validation)
    if diagnostic:
        need(validation.get("passed") is True and validation.get("executable_sha256") == staged["executable_sha256"]
             and validation.get("release_mode") == "diagnostic" and validation.get("battle_status") == "unavailable"
             and validation.get("playable_battle") is False and validation.get("full_game_parity") is False
             and validation.get("unfinished_battle_ui_labelled") is True
             and validation.get("claims_native_battle") is False, "Diagnostic acceptance must explicitly refuse playable/native-battle claims and clearly label unfinished battle startup UI")
        for name in ("README.txt", "CHECKPOINT.txt"):
            need(all(marker in (RELEASE / name).read_text(encoding="utf-8-sig") for marker in DIAGNOSTIC_MARKERS), "Missing diagnostic release warnings")
    else:
        need(validation.get("passed") is True and validation.get("executable_sha256") == staged["executable_sha256"]
             and validation.get("playable_battle") is True and validation.get("full_game_parity") is False, "No matching actual native-battle acceptance")
    checks = validation.get("checks", [])
    required = ({"architecture-dependencies", "asset-verification", "package-cli", "render-review", "unfinished-battle-ui", "independent-review"}
                if diagnostic else {"native-battle", "client-controls", "render-review", "package-cli", "independent-review"})
    need(required <= {r.get("name") for r in checks}, "Missing required release acceptance checks")
    evidence = validation.get("evidence_files", [])
    need(evidence and validation.get("limitations") and validation.get("not_tested"), "Missing evidence or truthful limits")
    verify_rows(ROOT, evidence)
    for row in checks:
        need(row.get("status") == "passed" and row.get("evidence_files")
             and all(name in {e["path"] for e in evidence} for name in row["evidence_files"]), "Unproven acceptance check")
    copy_new(args.validation, RELEASE / "evidence/validation.json")
    for row in evidence:
        need(not inside(ROOT, row["path"]).is_relative_to(RELEASE.resolve()), "Recursive package evidence")
        copy_new(inside(ROOT, row["path"]), RELEASE / "evidence" / relative(row["path"]))
    archive_name = DIAGNOSTIC_ARCHIVE if diagnostic else ARCHIVE
    manifest = {"schema": 1, "status": ("Development-10-Hotfix-01 DIAGNOSTIC / UNFINISHED; BATTLE STATUS: UNAVAILABLE" if diagnostic
                                         else "Development-10-Hotfix-01: accepted native battle scope in evidence/validation.json"),
                "release_mode": mode, "archive": archive_name,
                "battle_status": "unavailable" if diagnostic else "accepted-bounded-scope",
                "standalone_game": False, "full_game_parity": False, "playable_battle": not diagnostic,
                "unfinished_battle_ui_labelled": True if diagnostic else None,
                "executable_sha256": staged["executable_sha256"], "pe": staged["pe"],
                "external_assets": staged["external_assets"], "settings_location": "%APPDATA%/AxisAlliesRust/settings.json",
                "files": [record(p, RELEASE) for p in sorted(RELEASE.rglob("*")) if p.is_file()]}
    write_new(RELEASE / "manifest.json", manifest)
    with zipfile.ZipFile(RELEASE / archive_name, "x", zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for path in sorted(RELEASE.rglob("*")):
            if path.is_file() and path.name != archive_name:
                archive.write(path, path.relative_to(RELEASE).as_posix())
    verify_package()


def verify_package():
    manifest = read(RELEASE / "manifest.json")
    mode = manifest.get("release_mode", "playable")
    archive_name = manifest.get("archive", ARCHIVE)
    need(mode in ("diagnostic", "playable") and archive_name == (DIAGNOSTIC_ARCHIVE if mode == "diagnostic" else ARCHIVE), "Wrong release mode/archive label")
    if mode == "diagnostic":
        need(manifest.get("battle_status") == "unavailable" and manifest.get("playable_battle") is False
             and manifest.get("full_game_parity") is False and manifest.get("unfinished_battle_ui_labelled") is True,
             "Diagnostic manifest must retain unfinished/no-battle status")
    verify_rows(RELEASE, manifest["files"])
    expected = {r["path"] for r in manifest["files"]} | {"manifest.json"}
    need({p.relative_to(RELEASE).as_posix() for p in RELEASE.rglob("*") if p.is_file()} == expected | {archive_name}, "Unexpected sealed file")
    with zipfile.ZipFile(RELEASE / archive_name) as archive:
        names = archive.namelist()
        need(len(names) == len(set(names)) == len({n.casefold() for n in names}) and set(names) == expected, "Wrong ZIP payload")
        need(archive.testzip() is None, "ZIP CRC failure")
        for name in names:
            relative(name)
            with archive.open(name) as stream:
                need(hashlib.file_digest(stream, "sha256").hexdigest() == sha(RELEASE / name), "ZIP SHA-256 failure")
    verify_rows(ROOT, read(RELEASE / "STAGING.json")["previous_release_files"])
    print(json.dumps({"passed": True, "release_mode": mode, "battle_status": manifest.get("battle_status"),
                      "scope": "Package integrity only; semantic acceptance is separately recorded", "zip_sha256": sha(RELEASE / archive_name)}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="phase", required=True)
    p = sub.add_parser("plan"); p.add_argument("--output", required=True)
    p = sub.add_parser("stage")
    for name in ("exe", "sha256", "build-record", "payload-plan", "readme", "checkpoint", "launchers"):
        p.add_argument("--" + name, required=True)
    p.add_argument("--release-mode", choices=("playable", "diagnostic"), default="playable")
    p = sub.add_parser("seal"); p.add_argument("--validation", required=True)
    p = sub.add_parser("seal-diagnostic"); p.add_argument("--validation", required=True)
    sub.add_parser("verify")
    args = parser.parse_args()
    if args.phase == "plan": plan(args.output)
    elif args.phase == "stage": stage(args)
    elif args.phase == "seal": seal(args)
    elif args.phase == "seal-diagnostic": seal_diagnostic(args)
    else: verify_package()


if __name__ == "__main__":
    main()
