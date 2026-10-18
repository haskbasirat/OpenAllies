"""Task-scoped animation acceptance/doc templates; never runs a client or builds.

Root supplies real observations. `requirements` prints the contract only;
`assemble` exclusively writes a new validation after every supplied check passes.
No battle tests are required or accepted as animation-browser evidence.
"""
import argparse
from pathlib import Path
import json

import package_animation01 as p


README = """AXIS & ALLIES RUST - DEVELOPMENT 10 ANIMATIONS 01
DIAGNOSTIC / UNFINISHED
BATTLE STATUS: UNAVAILABLE

This preview is for browsing and playing recovered source animations,
including embedded building/HQ timelines. Source inventory completeness is
separate from playback support. Unsupported controllers, unbound source
clips and diagnostic errors must remain explicit; this is not a claim that
every original effect, animation or whole-game behavior is reconstructed.

Open the Animation Browser, choose a source/model and clip, and use its
playback controls. The final observed GUI selections, representative HQ
death renders, CPU corpus checks and remaining limitations are recorded in
evidence/validation.json. Static previews do not run a playable battle.

Keep this EXE and assets-cache together. Existing original Data.rwd and the
reconstruction assets are required; this ZIP is not a standalone game.
Windows x64, the x64 MSVC runtime and compatible graphics drivers are needed.
Earlier Development08/09/10/HF01/HF02 packages are preserved.
"""

CHECKPOINT = """DEVELOPMENT 10 ANIMATIONS 01 - CHECKPOINT
DIAGNOSTIC / UNFINISHED
BATTLE STATUS: UNAVAILABLE

Read docs/NATIVE-PORT-RULES.md first. This task extends an animation asset
browser, not native battle readiness. Preserve source identities, authored
curve/controller semantics, explicit unsupported cases and source coverage.
Do not silently drop controller children or equate a complete source list
with supported playback. Original source NIF/KF timelines and current game
animation role bindings are distinct provenance.

The package retains the complete legacy game-rig/menu/model-sidecar cache
closure and adds only manifest-listed browser files under
assets-cache/animation/browser. It does not overwrite earlier caches or
sealed releases. All stage/build/evidence/source and ZIP guards remain
exclusive and hash checked. Detailed actual observations belong in
evidence/validation.json; no unexecuted test result is asserted here.

Continue remaining animation/renderer support from explicit source/status
reports. Whole playable battle, native controls/movement/combat/AI/victory,
multiplayer, campaign and full-game parity remain separate unaccepted work.
No performance guarantee or overall completion percentage is implied.
"""


def requirements():
    return dict(required_checks=sorted(p.REQUIRED_CHECKS),
        all_reports=dict(passed=True, candidate_sha256="exact staged EXE SHA", evidence_files="nonempty workspace file records",
                         limitations="nonempty truthful limits"),
        special_fields={
            "animation-gui-selection-playback": ["selection_and_playback_observed=true", "hq_death_selected=true", "unsupported_entries_visible=true", "observations"],
            "animation-cpu-corpus": ["animation_manifest_sha256", "source_catalog_sha256", "source_assets=full catalog size", "rigs_checked=manifest rig count", "unsupported_coverage_retained=true", "all_animations_playable_claimed=false"],
            "animation-gpu-review": ["hq_death_reviewed=true", "representatives (nonempty)", "actual screenshots/pose reports among evidence files"],
            "animation-unit-tests": ["tested_source_files (exact subset of release build source pins)", "test_commands (actually executed)", "test_binary (separate unit executable record)"],
            "native-executable-preservation": ["native_files (both original/v14 AA.exe path/bytes/sha256/baseline_sha256)"],
        },
        report_manifest="JSON list of {name, report} workspace-relative paths; one per required check",
        output_scope="Only animation preview acceptance; diagnostic/unavailable/nonstandalone/full-parity-false",
        docs=dict(README=README, CHECKPOINT=CHECKPOINT),
        no_claim="Templates/check requirements are not executed observations, approved stage docs or gameplay acceptance")


def assemble(args):
    staged = p.read(p.RELEASE / "STAGING.json")
    digest = p.sha(p.RELEASE / p.EXE)
    p.need(digest == staged["executable_sha256"] and staged.get("release_mode") == "diagnostic", "Exact diagnostic animation stage required")
    build = p.read(p.RELEASE / "evidence/build-record.json")
    build_sources = {row["path"]: row for row in build["source_files"]}
    animation = p.read(p.RELEASE / p.ANIMATION_DESTINATION / "animation-browser-manifest.json")
    animation_sha = p.sha(p.RELEASE / p.ANIMATION_DESTINATION / "animation-browser-manifest.json")
    catalog = p.read(p.inside(p.ROOT, staged["source_catalog"]["path"]))
    manifest_path = Path(args.reports).resolve()
    p.need(manifest_path.is_relative_to(p.ROOT), "Report manifest must stay in workspace")
    manifest = p.read(manifest_path)
    names = [row["name"] for row in manifest]
    p.need(len(names) == len(set(names)) and set(names) == p.REQUIRED_CHECKS, "Every animation acceptance check is required exactly once")
    files, checks, limits = {p.record(manifest_path)["path"]}, [], []
    reports = {}
    for row in manifest:
        report_path = str(p.relative(row["report"]))
        report = p.read(p.inside(p.ROOT, report_path))
        p.need(report.get("passed") is True and report.get("candidate_sha256") == digest
               and report.get("evidence_files") and report.get("limitations"), "Missing actual matching check/evidence/limits")
        p.verify_rows(p.ROOT, report["evidence_files"])
        files.add(report_path)
        files.update(item["path"] for item in report["evidence_files"])
        checks.append(dict(name=row["name"], status="passed", evidence_files=[report_path]))
        limits.extend(report["limitations"])
        reports[row["name"]] = report
    gui = reports["animation-gui-selection-playback"]
    p.need(gui.get("selection_and_playback_observed") is True and gui.get("hq_death_selected") is True
           and gui.get("unsupported_entries_visible") is True and gui.get("observations"), "Actual browser selections/playback and visible unsupported entries required")
    cpu = reports["animation-cpu-corpus"]
    p.need(cpu.get("animation_manifest_sha256") == animation_sha
           and cpu.get("source_catalog_sha256") == staged["source_catalog"]["sha256"]
           and cpu.get("source_assets") == len(catalog["assets"])
           and cpu.get("rigs_checked") == len(animation["rigs"])
           and cpu.get("unsupported_coverage_retained") is True
           and cpu.get("all_animations_playable_claimed") is False, "Full source/status and every exported rig must be checked without inflated playback claims")
    gpu = reports["animation-gpu-review"]
    p.need(gpu.get("hq_death_reviewed") is True and gpu.get("representatives"), "Actual representative GPU review including HQ death required")
    unit = reports["animation-unit-tests"]
    p.need(unit.get("tested_source_files") and unit.get("test_commands") and unit.get("test_binary"), "Actual focused animation tests and source/binary provenance required")
    p.verify_rows(p.ROOT, unit["tested_source_files"] + [unit["test_binary"]])
    p.need(all(build_sources.get(row["path"]) == row for row in unit["tested_source_files"]), "Animation tests covered different sources from the release build")
    files.add(unit["test_binary"]["path"])
    native = reports["native-executable-preservation"]
    expected_native = {str((p.ROOT.parent / "AA.exe").resolve()): "5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26",
        str((p.ROOT.parent / "reconstruction/native-game/runtime-expanded-v14/AA.exe").resolve()): "3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1"}
    p.need(len(native["native_files"]) == 2 and {str(Path(row["path"]).resolve()) for row in native["native_files"]} == set(expected_native), "Both original native EXEs must be preserved")
    for row in native["native_files"]:
        path = Path(row["path"]).resolve()
        p.need(row["sha256"] == row["baseline_sha256"] == expected_native[str(path)]
               and path.stat().st_size == row["bytes"] and p.sha(path) == row["sha256"], "Original executable preservation failed")
    for row in [staged["source_catalog"], staged["animation_manifest"]]:
        p.verify_rows(p.ROOT, [row]); files.add(row["path"])
    files.add(Path(__file__).resolve().relative_to(p.ROOT).as_posix())
    validation = dict(schema=1, passed=True, executable_sha256=digest, release_mode="diagnostic",
        battle_status="unavailable", playable_battle=False, full_game_parity=False, claims_native_battle=False,
        standalone_game=False, unfinished_battle_ui_labelled=True, animation_preview_validated=True,
        source_coverage_reported=True, all_animations_playable_claimed=False,
        scope="Task-scoped actual animation-browser GUI/CPU/GPU/unit/package acceptance; no battle/startup/full-game acceptance.",
        animation_manifest_sha256=animation_sha, source_catalog=staged["source_catalog"],
        checks=checks, evidence_files=[p.record(p.inside(p.ROOT, name)) for name in sorted(files)],
        limitations=list(dict.fromkeys(limits)),
        not_tested=["Whole native animation/effect/Model/View parity", "Complete playable battle, controls, movement/combat/AI/victory", "Campaign/multiplayer/full saves", "Clean-PC standalone deployment or performance guarantees"])
    output = Path(args.output).resolve()
    p.need(output.is_relative_to(p.QA.resolve()) and not output.is_relative_to(p.RELEASE.resolve()), "Validation output must stay in new animation QA directory")
    p.write_new(output, validation)
    print(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="phase", required=True)
    sub.add_parser("requirements")
    command = sub.add_parser("assemble")
    command.add_argument("--reports", required=True); command.add_argument("--output", required=True)
    args = parser.parse_args()
    if args.phase == "requirements":
        print(json.dumps(requirements(), indent=2))
    else:
        assemble(args)


if __name__ == "__main__":
    main()
