"""Record exact QA02 artifact reuse after Root starts fresh CLI03 validation.

No build or map test runs. No output directory is created: CLI03 owns it.
The only output is a new startup-review.json; existing observations stay put.
"""
from pathlib import Path
import package_hotfix02 as p


BASE = "qa-output/development10/"
ORIGIN = BASE + "hotfix02-native-startup02/"
CURRENT = BASE + "hotfix02-native-startup03/"


def main():
    out = p.ROOT / (BASE + "hotfix02-acceptance03")
    p.need(out.is_dir() and (out / "work").is_dir(), "Wait for Root's CLI03 validator to create its output directory")
    old = p.read(p.ROOT / (ORIGIN + "build-record.json"))
    current = p.read(p.ROOT / (CURRENT + "build-record.json"))
    gui = p.read(p.ROOT / (BASE + "hotfix02-build03/build-record.json"))
    p.need(old.get("passed") is True and current.get("passed") is True and gui.get("passed") is True
           and current.get("source_unchanged_during_build") is True
           and gui.get("source_unchanged_during_build") is True
           and current["source_files"] == gui["source_files"]
           and old["executable"] == current["executable"], "Fresh compile/source/artifact equality failed")
    p.verify_rows(p.ROOT, [current["executable"]])
    before = {row["path"]: row for row in old["source_files"]}
    after = {row["path"]: row for row in current["source_files"]}
    changed = [name for name in sorted(before.keys() & after.keys()) if before[name] != after[name]]
    p.need(before.keys() == after.keys() and changed == ["crates/aa-client/src/battle_notifications.rs"],
           "Only the unimported client notification source may differ")
    dep = p.ROOT / Path(current["executable"]["path"]).with_suffix(".d")
    text = dep.read_text(encoding="utf-8")
    p.need("battle_notifications.rs" not in text.casefold(), "Changed client source appears in QA imports")
    # Exact rustc dep-info domain: four relative, space-free phony target lines.
    # Resolve the authored .. segments against the workspace, not by guessing imports.
    inputs = set()
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("crates\\") and line.endswith(":"):
            path = (p.ROOT / line[:-1].replace("\\", "/")).resolve()
            p.need(path.is_relative_to(p.ROOT.resolve()) and path.is_file(), "QA import leaves workspace")
            inputs.add(path.relative_to(p.ROOT).as_posix())
    fixture = "research/skirmish-20261016-local-bootstrap/native-timer.json"
    expected = {"crates/aa-runtime/tests/playable_battle_startup.rs", "crates/aa-client/src/runtime_fp.rs",
                "crates/aa-client/src/windows_reset_os.rs", fixture}
    p.need(inputs == expected, "Unexpected QA direct import closure; review before reuse")
    checkpoint_name = "research/skirmish-20261016-local-bootstrap/checkpoint01.json"
    checkpoint = p.read(p.ROOT / checkpoint_name)
    rows = []
    baselines = []
    for name in sorted(inputs):
        row = p.record(p.ROOT / name)
        if name in before:
            p.need(row == before[name] == after[name], "QA direct source import changed")
            baseline = "QA02 and QA03 Cargo/crate source manifests"
        else:
            p.need(name == fixture and row["sha256"] == checkpoint["sha256"][name],
                   "External timer fixture differs from its historical checkpoint")
            baseline = checkpoint_name
        rows.append(row)
        baselines.append(dict(path=name, baseline=baseline))
    maps = []
    for name in ("berlin", "firefight"):
        result_path = p.ROOT / (ORIGIN + name + "/result.json")
        result = p.read(result_path)
        p.need(result.get("map") == name and result.get("strict_startup_passed") is True
               and result.get("source_unchanged_since_build") is True and result.get("exit_code") == 0
               and result.get("timed_out") is False and result.get("playable_battle") is False
               and result["executable"] == current["executable"], "Original strict result mismatch")
        p.verify_rows(p.ROOT, result["evidence_files"])
        maps.append(dict(map=name, result=p.record(result_path), seconds=result["seconds"], strict_startup_passed=True))
    review = dict(passed=True, executable=current["executable"], source_files_match_gui_build=True,
        strict_results_reused=True, strict_results_origin=ORIGIN, qa_import_dependencies_unchanged=True,
        qa_dependency_manifest=p.record(dep), qa_dependency_inputs=rows, qa_dependency_baselines=baselines,
        qa_external_fixture_checkpoint=p.record(p.ROOT / checkpoint_name),
        current_compile=p.record(p.ROOT / (CURRENT + "build-record.json")),
        original_result_build=p.record(p.ROOT / (ORIGIN + "build-record.json")),
        source_delta_from_result_build=changed, maps=maps, playable_battle=False,
        scope="Original QA02 Berlin/Firefight Host/resource/StartedMission results reused for the identical 509 QA artifact. QA03 no-run compile and GUI03 fingerprints match. The sole changed tracked source is an unimported client notification literal. No new strict execution, GUI activation or battle acceptance.",
        limitations=["Strict results and elapsed times remain original02 observations; they were not rerun or relabelled.",
            "Direct rustc dep-info imports are hashed against QA02/03 source manifests or the historical external timer checkpoint. This is not complete transitive/external compiler-input attestation.",
            "StartedMission does not establish frontend activation, native View/task readiness, controls or playable battle."])
    destination = out / "startup-review.json"
    p.write_new(destination, review)
    print(p.record(destination))


if __name__ == "__main__":
    main()
