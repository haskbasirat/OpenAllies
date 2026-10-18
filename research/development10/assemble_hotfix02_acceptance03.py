"""Candidate3 diagnostic evidence assembler. Root runs it after actual validation.

Never builds, launches, stages or seals. Candidate1 reports remain untouched.
An existing output is an error; no observation or native readiness is inferred.
"""
import argparse
from pathlib import Path

import package_hotfix02 as p


BASE = "qa-output/development10/"
OUT = BASE + "hotfix02-acceptance03/"
PLATFORM = BASE + "hotfix02-platform03/"
STARTUP = BASE + "hotfix02-native-startup03/"
RESULT_STARTUP = BASE + "hotfix02-native-startup02/"
GUI = BASE + "hotfix02-gui03/"
BUILD = BASE + "hotfix02-build03/"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--backup-index", required=True, help="Exact HF01 predecessor index for candidate3")
    parser.add_argument("--targeted-review", action="append", required=True,
                        help="Passed JSON review of a targeted fix; repeat for each new fix")
    parser.add_argument("--independent-review", action="append", required=True,
                        help="Passed candidate3/source/package review; repeat as needed")
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
           "Matching diagnostic candidate3 stage required")
    predecessor = p.read(p.ROOT / "backups/dev10-hotfix02-unsealed-candidate01/STAGING.json")
    p.need(digest != predecessor["executable_sha256"], "Candidate1 EXE cannot be relabelled as candidate3")
    build = passed(BUILD + "build-record.json", digest)
    copied_build = p.read(p.RELEASE / "evidence/build-record.json")
    p.need(build == copied_build and build.get("source_unchanged_during_build") is True,
           "Stage must contain the exact frozen candidate3 default-profile build")
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
           "Current QA03 compile and GUI03 build must share source, with distinct executable/profile scope")
    p.verify_rows(p.ROOT, [startup["executable"]])
    result_build = passed(RESULT_STARTUP + "build-record.json")
    p.need(result_build.get("source_unchanged_during_build") is True
           and result_build["executable"] == startup_build["executable"]
           and startup.get("strict_results_reused") is True
           and startup.get("strict_results_origin") == RESULT_STARTUP
           and startup.get("qa_import_dependencies_unchanged") is True,
           "Reuse must explicitly identify the identical original QA02 artifact/results")
    before = {row["path"]: row for row in result_build["source_files"]}
    after = {row["path"]: row for row in startup_build["source_files"]}
    reuse_delta = [name for name in sorted(before.keys() & after.keys()) if before[name] != after[name]]
    p.need(len(before) == len(result_build["source_files"])
           and len(after) == len(startup_build["source_files"])
           and before.keys() == after.keys()
           and reuse_delta == ["crates/aa-client/src/battle_notifications.rs"],
           "Reuse allows only the client notification literal change, with no Cargo/runtime/QA-source delta")
    dependency = startup["qa_dependency_manifest"]
    dependency_path = Path(startup_build["executable"]["path"]).with_suffix(".d").as_posix()
    p.need(dependency["path"] == dependency_path, "QA dependency manifest must belong to the current test artifact")
    p.verify_rows(p.ROOT, [dependency])
    p.need("battle_notifications.rs" not in p.inside(p.ROOT, dependency_path).read_text(encoding="utf-8").casefold(),
           "The changed client source must not be imported by the QA test")
    files.add(dependency_path)
    p.verify_rows(p.ROOT, startup["qa_dependency_inputs"])
    files.update(row["path"] for row in startup["qa_dependency_inputs"])
    checkpoint = startup["qa_external_fixture_checkpoint"]
    p.verify_rows(p.ROOT, [checkpoint])
    files.add(checkpoint["path"])
    old_startup = p.read(p.ROOT / "qa-output/development10/hotfix02-native-startup/build-record.json")
    p.need(startup["executable"]["sha256"] != old_startup["executable"]["sha256"],
           "Candidate1 QA artifact cannot be relabelled as the proven reused QA02 artifact")
    strict_results = []
    for name in ("berlin", "firefight"):
        result_name = RESULT_STARTUP + name + "/result.json"
        result = read(result_name)
        p.need(result.get("map") == name and result.get("strict_startup_passed") is True
               and result.get("source_unchanged_since_build") is True
               and result.get("exit_code") == 0 and result.get("timed_out") is False
               and result.get("playable_battle") is False and result["executable"] == startup["executable"],
               f"Missing original candidate2 strict mission result: {name}")
        p.verify_rows(p.ROOT, result["evidence_files"])
        files.update(row["path"] for row in result["evidence_files"])
        review_rows = [row for row in startup["maps"] if row["map"] == name]
        p.need(len(review_rows) == 1 and review_rows[0]["result"] == p.record(p.ROOT / result_name)
               and review_rows[0]["seconds"] == result["seconds"],
               "Startup review must retain the original result path/hash/timing without relabelling")
        strict_results.append(p.record(p.ROOT / result_name))
        for leaf in ("process.json", "attempt.main-launch.json", "attempt.worker.json", "stdout.txt", "stderr.txt"):
            files.add(RESULT_STARTUP + name + "/" + leaf)
        actual_gui = read(GUI + name + "/result.json")
        p.need(actual_gui.get("executable_sha256") == digest and actual_gui.get("exit_code") == 0
               and actual_gui.get("timed_out") is False, f"Incomplete fresh candidate3 GUI run: {name}")
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
           "Candidate3 index must cover every actual changed HF01 predecessor and new source exactly once")
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
    validation = dict(schema=2, candidate="hotfix02-candidate03", passed=True,
        release_mode="diagnostic", battle_status="unavailable", playable_battle=False,
        full_game_parity=False, claims_native_battle=False, unfinished_battle_ui_labelled=True,
        executable_sha256=digest, scope="Candidate3 exact-client GUI/CLI/static-render diagnostics. Original candidate2 strict mission results are reused only for the identical QA artifact, freshly compiled against candidate3 source. No rerun or playable-battle acceptance is claimed.",
        checks=[dict(name=name, status="passed", evidence_files=names) for name, names in checks],
        evidence_files=[p.record(p.inside(p.ROOT, name)) for name in sorted(files)],
        source_change_index=p.record(p.ROOT / args.backup_index),
        source_change_counts=dict(source_files=len(sources), changed_existing=len(changed), new_files=len(added)),
        strict_startup_results=strict_results,
        strict_startup_reuse=dict(reused=True, results_origin=RESULT_STARTUP,
            original_build=p.record(p.ROOT / (RESULT_STARTUP + "build-record.json")),
            current_compile=p.record(p.ROOT / (STARTUP + "build-record.json")),
            changed_source_paths=reuse_delta, qa_dependency_manifest=dependency,
            qa_dependency_inputs=startup["qa_dependency_inputs"],
            qa_external_fixture_checkpoint=checkpoint,
            qa_import_dependencies_unchanged=True),
        evidence_roots=dict(acceptance=OUT, gui=GUI, platform=PLATFORM, startup=RESULT_STARTUP, startup_compile=STARTUP, build=BUILD),
        limitations=[
            "Berlin/Firefight strict executions remain original candidate2 results/timings, not candidate3 reruns. QA03 compilation produced the exact same artifact; the sole tracked source delta is an unimported client notification literal. Cargo/crate comparisons and dep-info are not complete external compiler-input attestation.",
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
