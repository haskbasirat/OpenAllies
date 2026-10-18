"""Assemble scoped evidence only after root reviews actual final-executable captures."""
import argparse
from pathlib import Path
import package_animation01 as p


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--observations", required=True)
    args = parser.parse_args()
    observation_path = p.ROOT / args.observations
    seen = p.read(observation_path)
    p.need(seen.get("passed") is True and seen["gui"]["passed"] is True and seen["gpu"]["passed"] is True,
           "Root must inspect actual GUI/GPU results first")
    exe = p.RELEASE / p.EXE
    digest = p.sha(exe)
    p.need(seen["candidate_sha256"] == digest, "Review belongs to another executable")
    out = p.QA / "acceptance01"
    out.mkdir(parents=True, exist_ok=False)
    rows = []

    def report(name, data, evidence, limitations):
        doc = dict(passed=True, candidate_sha256=digest,
            evidence_files=[p.record(p.ROOT / path) for path in sorted(set(evidence))], limitations=limitations, **data)
        target = out / (name + ".json")
        p.write_new(target, doc)
        rows.append(dict(name=name, report=target.relative_to(p.ROOT).as_posix()))

    catalog_folder = seen["catalog_capture"]
    render_folder = seen["render_capture"]
    catalog = p.read(p.ROOT / catalog_folder / "capture-summary.json")
    renders = p.read(p.ROOT / render_folder / "capture-summary.json")
    for capture in (catalog, renders):
        p.need(capture["candidate_sha256"] == digest and capture["process_checks_completed"] is True, "Wrong or failed capture")
    manifest = p.read(p.RELEASE / p.ANIMATION_DESTINATION / "animation-browser-manifest.json")
    manifest_sha = p.sha(p.RELEASE / p.ANIMATION_DESTINATION / "animation-browser-manifest.json")
    cpu_path = "research/animation-browser-20261017/review/catalog-check02.json"
    cpu = p.read(p.ROOT / cpu_path)
    p.need(cpu["passed"] is True and cpu["loaded_rigs"] == len(manifest["rigs"])
           and cpu["sampled_clips"] == sum(len(r["clips"]) for r in manifest["rigs"]) and not cpu["errors"], "CPU corpus incomplete")
    export_path = "research/animation-browser-20261017/export/validation02.json"
    export = p.read(p.ROOT / export_path)
    p.need(export["passed"] is True and export["manifest_sha256"] == manifest_sha, "Wrong export acceptance")
    staged = p.read(p.RELEASE / "STAGING.json")
    common_limit = "Inspector acceptance only; no native whole-effect, gameplay, battle, campaign or multiplayer parity claim."
    report("architecture-dependencies", dict(pe=p.pe_info(exe, digest), tested_machine="Current Windows x64 machine; Vulkan and DX12 representative captures"),
        ["qa-output/animation-browser-20261017/build01/build-record.json", render_folder + "/capture-summary.json"],
        ["Static PE imports and local execution only; clean-PC deployment and all dynamically loaded DLLs are not established.", common_limit])
    native = []
    for path, expected in [(p.ROOT.parent / "AA.exe", "5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26"),
        (p.ROOT.parent / "reconstruction/native-game/runtime-expanded-v14/AA.exe", "3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1")]:
        actual = p.sha(path)
        p.need(actual == expected, "Protected original executable changed")
        native.append(dict(path=str(path.resolve()), bytes=path.stat().st_size, sha256=actual, baseline_sha256=expected))
    report("native-executable-preservation", dict(native_files=native, previous_release_files=len(staged["previous_release_files"])),
        ["research/animation-browser-20261017/payload-plan01.json", export_path],
        ["Native executable hashes, original source inventory hashes and sealed-release payload hashes checked; user profiles are not part of this payload."])
    implicit = p.read(p.ROOT / seen["implicit_launch_receipt"])
    p.need(implicit["passed"] is True and implicit["candidate_sha256"] == digest, "Implicit launcher discovery failed")
    report("package-cli", dict(observations=catalog["observations"], implicit_launch=implicit),
        [catalog_folder + "/capture-summary.json", catalog_folder + "/catalog.json", catalog_folder + "/catalog.process-result.json", seen["implicit_launch_receipt"], *[r["path"] for r in implicit["evidence_files"]]],
        ["Exact installed catalog discovery/clip identity checked through --list-animations; this is not execution of every GUI path.", common_limit])
    report("animation-cpu-corpus", dict(animation_manifest_sha256=manifest_sha, source_catalog_sha256=staged["source_catalog"]["sha256"],
        source_assets=2481, rigs_checked=cpu["loaded_rigs"], clips_checked=cpu["sampled_clips"], sampled_poses=cpu["sampled_poses"],
        unsupported_coverage_retained=True, all_animations_playable_claimed=False),
        [cpu_path, "research/animation-browser-20261017/review/catalog-check02.log", export_path,
         "research/animation-browser-20261017/export/tcb-successor-validation.json"],
        ["Five authored timestamps per published clip; finite geometry/alpha and source/topology checks do not prove every intervening frame or every native effect.",
         "240 animated sources remain unavailable; 47 nonlinear quaternion clips are quarantined; 139 published rigs retain explicit unsupported controllers.", common_limit])
    gui = seen["gui"]
    report("animation-gui-selection-playback", dict(selection_and_playback_observed=gui["selection_and_playback_observed"],
        hq_death_selected=gui["hq_death_selected"], unsupported_entries_visible=gui["unsupported_entries_visible"], observations=gui["observations"]),
        [args.observations, *gui["evidence"]], ["Actual scripted egui events and screenshots reviewed on this machine; representative selections only.", common_limit])
    gpu = seen["gpu"]
    render_evidence = [render_folder + "/capture-summary.json", args.observations, *gpu["evidence"]]
    for row in renders["observations"]:
        render_evidence += [row["image"]["path"], render_folder + "/" + row["case"]["name"] + ".json"]
    report("animation-gpu-review", dict(hq_death_reviewed=gpu["hq_death_reviewed"], representatives=gpu["representatives"], observations=gpu["observations"]),
        render_evidence, ["Representative original geometry inspected at selected times on Vulkan/DX12; no all-frame, all-hardware or pixel-perfect native-render comparison.", common_limit])
    unit_folder = "qa-output/animation-browser-20261017/unit-final01"
    unit = p.read(p.ROOT / unit_folder / "receipt.json")
    p.need(unit["passed"] is True, "Unit executable failed")
    cat_unit = "qa-output/animation-browser-20261017/catalog-unit03"
    cat_tests = p.read(p.ROOT / cat_unit / "receipt.json")
    p.need(cat_tests["passed"] is True, "Catalog unit tests failed")
    report("animation-unit-tests", dict(tested_source_files=unit["tested_source_files"] + cat_tests["tested_source_files"], test_binary=unit["test_binary"],
        test_commands=[unit["command"], p.read(p.ROOT / cat_unit / "receipt.json")["test_command"]]),
        [unit_folder + "/receipt.json", unit_folder + "/unit.log", cat_unit + "/receipt.json", cat_unit + "/test.log",
         "research/animation-browser-20261017/review/animation-tests04.log", "research/animation-browser-20261017/review/hq25-tests05.log",
         "research/animation-browser-20261017/client-adapter/findings.json", "research/animation-browser-20261017/client-adapter/exact-cpu-tests.log"],
        ["Seven exact sampler unit tests plus two exact catalog tests executed; prior 17 legacy regression tests and six adapter helper tests have separate receipts.",
         "Catalog unit harness stubs the unused cache locator; actual discovery is separately exercised by the release EXE. The saved GPU unit test was typechecked, not executed.", common_limit])
    independent = ["research/animation-browser-20261017/inventory/findings.json", "research/animation-browser-20261017/native-curves/findings.json",
        export_path, cpu_path, "research/animation-browser-20261017/client-adapter/findings.json"]
    for path in independent:
        doc = p.read(p.ROOT / path)
        if "passed" in doc:
            p.need(doc["passed"] is True, "A boundary review failed")
    if seen.get("independent_client_review"):
        independent.append(seen["independent_client_review"])
        p.need(p.read(p.ROOT / seen["independent_client_review"])["passed"] is True, "Client review failed")
    report("independent-review", dict(scope="Separate agent inventory/native-curve investigation, source exporter and independently implemented Rust sampler cross-checks; root final integration review.",
        reviewers=["sdk_native_geometry_review", "state_binding_review", "builder_source_review", "leader_binding_review", "saved_leader_adapter"],
        client_integration_review=seen["client_review"]), independent + [args.observations],
        ["Module boundary/source references have independent authors. This is not an independent rebuild attestation or an assertion that every client line was reviewed by a second person.", common_limit])
    p.write_new(out / "reports.json", rows)
    print(out / "reports.json")


if __name__ == "__main__":
    main()
