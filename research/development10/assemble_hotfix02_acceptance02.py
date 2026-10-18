"""Candidate2 diagnostic evidence assembler. Root runs it after actual validation.

Never builds, launches, stages or seals. Candidate1 reports remain untouched.
An existing output is an error; no observation or native readiness is inferred.
"""
import argparse
from pathlib import Path

import package_hotfix02 as p


BASE = "qa-output/development10/"
OUT = BASE + "hotfix02-acceptance02/"
PLATFORM = BASE + "hotfix02-platform02/"
STARTUP = BASE + "hotfix02-native-startup02/"
GUI = BASE + "hotfix02-gui02/"
BUILD = BASE + "hotfix02-build02/"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--backup-index", required=True, help="Exact HF01 predecessor index for candidate2")
    parser.add_argument("--targeted-review", action="append", required=True,
                        help="Passed JSON review of a targeted fix; repeat for each new fix")
    parser.add_argument("--independent-review", action="append", required=True,
                        help="Passed candidate2/source/package review; repeat as needed")
    parser.add_argument("--evidence", action="append", default=[], help="Additional bounded proof file")
    parser.add_argument("--independent-final", help="Passed final review; selects final validation.json")
    args = parser.parse_args()
    files = set()

    def read(name):
        path = p.inside(p.ROOT, name)
        files.add(name)
        return p.read(path)

    def passed(name, executable=None):
        value = read(name)
        p.need(value.get("passed") is True, f"No explicit passed review: {name}")
        if executable is not None:
            p.need(value.get("executable_sha256") == executable, f"Wrong client EXE: {name}")
        return value

    staged = p.read(p.RELEASE / "STAGING.json")
    digest = p.sha(p.RELEASE / p.EXE)
    p.need(staged["executable_sha256"] == digest and staged.get("release_mode") == "diagnostic",
           "Matching diagnostic candidate2 stage required")
    predecessor = p.read(p.ROOT / "backups/dev10-hotfix02-unsealed-candidate01/STAGING.json")
    p.need(digest != predecessor["executable_sha256"], "Candidate1 EXE cannot be relabelled as candidate2")
    build = passed(BUILD + "build-record.json", digest)
    copied_build = p.read(p.RELEASE / "evidence/build-record.json")
    p.need(build == copied_build and build.get("source_unchanged_during_build") is True,
           "Stage must contain the exact frozen candidate2 default-profile build")
    p.verify_rows(p.ROOT, build["evidence_files"])
    files.update(row["path"] for row in build["evidence_files"])

    cli = passed(OUT + "cli-results.json", digest)
    expected_cli = {"help", "asset-verification", "mechanics-fragments", "world-fragments", "record-loading",
                    "hq-vulkan", "map-vulkan", "infantry-dx12"}
    p.need(len(cli["checks"]) == 8 and {row["name"] for row in cli["checks"]} == expected_cli
           and all(row.get("passed") is True for row in cli["checks"]),
           "All eight exact-client CLI checks are required")
    for row in cli["checks"]:
        p.need(read(OUT + row["name"] + "-result.json") == row, "CLI result differs from final summary")
        files.update(OUT + row["name"] + suffix for suffix in (".stdout.txt", ".stderr.txt"))
    visual = passed(OUT + "visual-review.json", digest)
    for image in visual["images"]:
        for key in ("image", "report"):
            p.need(image[key]["path"].startswith(OUT), "Old visual evidence cannot be relabelled")
            p.verify_rows(p.ROOT, [image[key]])
            files.add(image[key]["path"])
    gui = passed(OUT + "gui-review.json", digest)
    p.need("remaining_blocker" in gui and (gui["remaining_blocker"] is None
           or isinstance(gui["remaining_blocker"], str) and bool(gui["remaining_blocker"].strip())),
           "GUI review must name the observed blocker, or explicitly use null")
    p.need(isinstance(gui.get("observations", gui.get("maps")), (dict, list))
           and bool(gui.get("observations", gui.get("maps"))) and bool(gui.get("limitations")),
           "Final actual GUI observations and limitations are required")
    p.verify_rows(p.ROOT, gui["evidence_files"])
    files.update(row["path"] for row in gui["evidence_files"])

    startup = passed(OUT + "startup-review.json")
    startup_build = passed(STARTUP + "build-record.json")
    p.need(startup.get("source_files_match_gui_build") is True and startup.get("playable_battle") is False
           and startup_build.get("source_unchanged_during_build") is True
           and startup_build["source_files"] == build["source_files"]
           and startup["executable"] == startup_build["executable"]
           and startup["executable"]["sha256"] != digest,
           "Fresh candidate2 strict QA build must share source, with distinct executable/profile scope")
    p.verify_rows(p.ROOT, [startup["executable"]])
    old_startup = p.read(p.ROOT / "qa-output/development10/hotfix02-native-startup/build-record.json")
    p.need(startup["executable"]["sha256"] != old_startup["executable"]["sha256"],
           "Candidate1 QA artifact cannot be relabelled as candidate2")
    strict_results = []
    for name in ("berlin", "firefight"):
        result_name = STARTUP + name + "/result.json"
        result = read(result_name)
        p.need(result.get("map") == name and result.get("strict_startup_passed") is True
               and result.get("source_unchanged_since_build") is True
               and result.get("exit_code") == 0 and result.get("timed_out") is False
               and result.get("playable_battle") is False and result["executable"] == startup["executable"],
               f"Missing actual candidate2 strict mission result: {name}")
        strict_results.append(p.record(p.ROOT / result_name))
        for leaf in ("process.json", "attempt.main-launch.json", "attempt.worker.json", "stdout.txt", "stderr.txt"):
            files.add(STARTUP + name + "/" + leaf)
        actual_gui = read(GUI + name + "/result.json")
        p.need(actual_gui.get("executable_sha256") == digest and actual_gui.get("exit_code") == 0
               and actual_gui.get("timed_out") is False, f"Incomplete actual candidate2 GUI run: {name}")
        for leaf in ("script.json", "process.json", "stdout.txt", "stderr.txt"):
            files.add(GUI + name + "/" + leaf)

    passed(PLATFORM + "architecture.json", digest)
    passed(PLATFORM + "native-executables-preserved.json")
    passed(PLATFORM + "results.json", digest)
    passed(PLATFORM + "system-dll-validation.json", digest)
    passed(BASE + "hotfix02-package-tests/preparation-report.json")

    backup = read(args.backup_index)
    sources = {row["path"]: row for row in build["source_files"]}
    old = p.read(p.ROOT / "release/Development-10-Hotfix-01/evidence/build-record.json")
    old_sources = {row["path"]: row for row in old["source_files"]}
    changed = {name for name in sources.keys() & old_sources.keys() if sources[name] != old_sources[name]}
    added = sources.keys() - old_sources.keys()
    p.need(len(sources) == len(build["source_files"]) and not old_sources.keys() - sources.keys()
           and {row["source"] for row in backup["files"]} == changed
           and len(backup["files"]) == len(changed)
           and set(backup["new_files"]) == added and len(backup["new_files"]) == len(added),
           "Candidate2 index must cover every actual changed HF01 predecessor and new source exactly once")
    for row in backup["files"]:
        p.need(row["sealed_predecessor"] == old_sources[row["source"]]
               and row["backup"]["sha256"] == row["sealed_predecessor"]["sha256"]
               and row["backup"]["bytes"] == row["sealed_predecessor"]["bytes"], "Wrong HF01 predecessor")
        p.verify_rows(p.ROOT, [row["backup"]])
    for name in args.targeted_review + args.independent_review:
        passed(name)
    if args.independent_final:
        passed(args.independent_final)
    files.update(args.evidence)
    files.add(Path(__file__).resolve().relative_to(p.ROOT).as_posix())
    for name in ("asset-verification.json", "mechanics-fragments.json", "world-fragments.json", "record-loading.json",
                 "hq-vulkan.png", "map-vulkan.png", "infantry-dx12.png"):
        files.add(OUT + name)
    checks = [
        ("architecture-dependencies", [PLATFORM + name for name in ("architecture.json", "native-executables-preserved.json", "system-dll-validation.json", "results.json")]),
        ("asset-verification", [OUT + "asset-verification.json"]),
        ("package-cli", [OUT + name for name in ("cli-results.json", "mechanics-fragments.json", "world-fragments.json", "record-loading.json")]),
        ("render-review", [OUT + "visual-review.json", OUT + "hq-vulkan.png", OUT + "map-vulkan.png", OUT + "infantry-dx12.png"]),
        ("unfinished-battle-ui", [OUT + "gui-review.json"]),
        ("targeted-startup-fixes", [OUT + "startup-review.json"] + args.targeted_review),
        ("independent-review", args.independent_review + ([args.independent_final] if args.independent_final else [])),
    ]
    validation = dict(schema=2, candidate="hotfix02-candidate02", passed=True,
        release_mode="diagnostic", battle_status="unavailable", playable_battle=False,
        full_game_parity=False, claims_native_battle=False, unfinished_battle_ui_labelled=True,
        executable_sha256=digest, scope="Candidate2 exact-client diagnostic GUI/CLI/static-render acceptance; separate strict mission startup only. No playable battle or full-game acceptance.",
        checks=[dict(name=name, status="passed", evidence_files=names) for name, names in checks],
        evidence_files=[p.record(p.inside(p.ROOT, name)) for name in sorted(files)],
        source_change_index=p.record(p.ROOT / args.backup_index),
        source_change_counts=dict(source_files=len(sources), changed_existing=len(changed), new_files=len(added)),
        strict_startup_results=strict_results, evidence_roots=dict(acceptance=OUT, gui=GUI, platform=PLATFORM, startup=STARTUP, build=BUILD),
        limitations=[
            "Strict Host/resource/StartedMission results do not imply frontend activation, native View/task readiness, control or scheduled gameplay.",
            "Actual GUI outcomes and remaining blockers are recorded in the final semantic review. Missing callbacks are not treated as successful.",
            "Targeted native/source owner proofs establish only their stated domains; static previews do not establish whole native Model/View parity.",
            "Native worker cleanup can wait. No interruptible loader, hard latency or performance guarantee.",
            "Requires the original Data.rwd/reconstruction assets, x64 MSVC runtime and appropriate drivers; not standalone.",
            "Cargo/crate fingerprints do not attest every external compiler include; startup QA and default-profile GUI executables remain separate.",
        ] + gui["limitations"],
        not_tested=["Complete playable battle, native orders, full movement/combat/AI/victory", "Whole-game original saves and campaign", "Real second-PC/internet multiplayer", "Clean-PC standalone deployment or 150% laptop DPI", "Every original map or performance benchmark", "Full client unit-test suite"])
    destination = p.ROOT / OUT / ("validation.json" if args.independent_final else "validation-draft.json")
    p.write_new(destination, validation)
    print(destination)


if __name__ == "__main__":
    main()
