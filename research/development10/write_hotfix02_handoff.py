"""Write HF02 continuation records only after Root seals the diagnostic package.

Preparation is safe to inspect/import: all reads and writes are inside main().
Root must serialize this final writer with other durable-journal writers.
Existing outputs are never replaced; a partial write is retained for review.
No compiler, client, stage/seal operation, crate or original-game write occurs.
"""
from datetime import datetime, timezone
import json
import os

import package_hotfix02 as p


ACCEPTANCE = "qa-output/development10/hotfix02-acceptance/"
HEADING = "## Development-10 Hotfix-02 SEALED (2026-10-17)"


def rendered(value):
    return json.dumps(value, indent=2, ensure_ascii=False)


def main():
    manifest_path = p.RELEASE / "manifest.json"
    p.need(manifest_path.is_file(), "Root must seal the final HF02 manifest first")
    manifest = p.read(manifest_path)
    p.need(manifest.get("release_mode") == "diagnostic"
           and manifest.get("battle_status") == "unavailable"
           and manifest.get("playable_battle") is False
           and manifest.get("full_game_parity") is False
           and manifest.get("standalone_game") is False
           and manifest.get("unfinished_battle_ui_labelled") is True,
           "Handoff requires the diagnostic/unavailable/non-standalone seal")
    p.need(manifest.get("archive") == p.DIAGNOSTIC_ARCHIVE,
           "Wrong final HF02 diagnostic ZIP label")
    archive = p.RELEASE / manifest["archive"]
    exe = p.RELEASE / p.EXE
    p.need(archive.is_file() and exe.is_file(), "Final ZIP and packaged EXE must exist")
    p.need(p.sha(exe) == manifest["executable_sha256"], "Packaged EXE changed")
    # Read-only full payload/ZIP/CRC/prior-release verification; this cannot seal.
    p.verify_package()

    validation_path = p.RELEASE / "evidence/validation.json"
    validation = p.read(validation_path)
    digest = manifest["executable_sha256"]
    p.need(validation.get("passed") is True
           and validation.get("executable_sha256") == digest
           and validation.get("release_mode") == "diagnostic"
           and validation.get("battle_status") == "unavailable"
           and validation.get("playable_battle") is False
           and validation.get("full_game_parity") is False
           and validation.get("claims_native_battle") is False,
           "Final matching diagnostic validation is required")
    evidence = validation["evidence_files"]
    p.verify_rows(p.ROOT, evidence)
    evidence_by_name = {row["path"]: row for row in evidence}
    p.need(len(evidence_by_name) == len(evidence), "Duplicate validation evidence")

    def accepted(name):
        p.need(name in evidence_by_name, f"Final validation does not include {name}")
        copy = p.inside(p.RELEASE, "evidence/" + name)
        row = evidence_by_name[name]
        p.need(copy.stat().st_size == row["bytes"] and p.sha(copy) == row["sha256"],
               f"Sealed evidence differs: {name}")
        return p.read(copy)

    gui = accepted(ACCEPTANCE + "gui-review.json")
    p.need(gui.get("passed") is True and gui.get("executable_sha256") == digest,
           "Final observed GUI review must match the packaged EXE")
    # A missing observation is never converted to a pending/error/success claim.
    p.need("remaining_blocker" in gui,
           "GUI review must explicitly name the observed remaining blocker, or use null")
    blocker = gui["remaining_blocker"]
    p.need(blocker is None or (isinstance(blocker, str) and bool(blocker.strip())),
           "Observed GUI blocker must be a nonempty string or explicit null")
    observations = gui.get("observations", gui.get("maps"))
    p.need(isinstance(observations, (dict, list)) and bool(observations),
           "Final per-map GUI observations are required")
    p.need(isinstance(gui.get("limitations"), list) and bool(gui["limitations"]),
           "Final GUI review must retain its observed limitations")

    startup = accepted(ACCEPTANCE + "startup-review.json")
    p.need(startup.get("passed") is True
           and startup.get("source_files_match_gui_build") is True
           and startup.get("playable_battle") is False,
           "Separate strict-startup evidence must retain its bounded scope")
    strict_results = []
    for name in ("berlin", "firefight"):
        native = accepted(f"qa-output/development10/hotfix02-native-startup/{name}/result.json")
        p.need(native.get("map") == name and native.get("strict_startup_passed") is True
               and native.get("exit_code") == 0 and native.get("timed_out") is False
               and native.get("source_unchanged_since_build") is True
               and native.get("playable_battle") is False
               and native["executable"] == startup["executable"],
               "Both actual strict mission-start results must match the separate QA EXE")
        strict_results.append(native)

    build = p.read(p.RELEASE / "evidence/build-record.json")
    p.need(build.get("passed") is True and build.get("source_unchanged_during_build") is True
           and build.get("executable_sha256") == digest
           and build.get("target") == "x86_64-pc-windows-msvc"
           and build.get("profile") == "release", "Matching frozen default-profile build required")
    p.need(startup["executable"]["sha256"] != digest,
           "QA startup and default-profile GUI executables must stay distinguished")
    sources = {row["path"]: row for row in build["source_files"]}
    p.need(len(sources) == len(build["source_files"]) == 2076, "Unexpected HF02 source inventory")

    backup_name = "backups/source-checkpoint-20261017-hotfix02/pre-hotfix-source-index.json"
    backup = accepted(backup_name)
    old_build = p.read(p.ROOT / "release/Development-10-Hotfix-01/evidence/build-record.json")
    old_sources = {row["path"]: row for row in old_build["source_files"]}
    changed = {name for name in sources.keys() & old_sources.keys() if sources[name] != old_sources[name]}
    new_sources = sources.keys() - old_sources.keys()
    p.need(not old_sources.keys() - sources.keys(), "Unexpected removed predecessor source")
    p.need(len(changed) == 18 and len(new_sources) == 1
           and {row["source"] for row in backup["files"]} == changed
           and set(backup["new_files"]) == new_sources, "Exact 18 predecessor backups plus one new file required")
    for row in backup["files"]:
        p.need(row["sealed_predecessor"] == old_sources[row["source"]]
               and row["backup"]["bytes"] == row["sealed_predecessor"]["bytes"]
               and row["backup"]["sha256"] == row["sealed_predecessor"]["sha256"],
               "Backup does not match the sealed HF01 source")
        p.verify_rows(p.ROOT, [row["backup"]])

    protected = p.read(p.RELEASE / "STAGING.json")["previous_release_files"]
    versions = {row["path"].split("/")[1] for row in protected}
    p.need(len(protected) == 10775 and versions == {
        "Development-08", "Development-09", "Development-10", "Development-10-Hotfix-01"},
        "Unexpected protected prior-release inventory")
    cli = accepted(ACCEPTANCE + "cli-results.json")
    visual = accepted(ACCEPTANCE + "visual-review.json")
    p.need(cli.get("passed") is True and cli.get("executable_sha256") == digest
           and all(row.get("passed") is True for row in cli["checks"])
           and visual.get("passed") is True and visual.get("executable_sha256") == digest,
           "Exact-client CLI and static-render observations must pass")

    result_path = p.ROOT / "qa-output/development10/hotfix02-release-result.json"
    doc_path = p.ROOT / "docs/DEVELOPMENT-10-HOTFIX-02-20261017.md"
    journal_path = p.ROOT / "research/development10/ACTIVE-SESSION-20261017.md"
    p.need(not result_path.exists() and not doc_path.exists(), "Handoff outputs already exist; never replace them")
    p.need(journal_path.is_file() and HEADING not in journal_path.read_text(encoding="utf-8"),
           "Durable journal missing or already contains this handoff")
    blocker_text = (blocker if blocker is not None else
                    "The final GUI review identifies no remaining blocker within its bounded observations; this does not accept a playable battle.")
    fixed = [
        "Visibility-memory callbacks borrow the actual retained Staged.clock explicitly. The reached 4EC4C8 timestamp read uses the same GWorld+E0 owner; no default clock, copied timestamp or new World substitutes for it.",
        "WorldStart 4F1E10 admits actual Source::New class-3 Actors through the original vtable+B4 constant AL=true after current identity/storage/source-proof checks. Saved/Special and Group8 behavior remain unchanged.",
    ]
    result = dict(schema=1, utc=datetime.now(timezone.utc).isoformat(),
        status="sealed diagnostic clock-owner and fresh-Actor-classifier hotfix",
        release_mode="diagnostic", battle_status="unavailable", playable_battle=False,
        full_game_parity=False, standalone_game=False,
        executable=p.record(exe), archive=p.record(archive), manifest=p.record(manifest_path),
        validation=p.record(validation_path), writer=p.record(__file__),
        gui_review=evidence_by_name[ACCEPTANCE + "gui-review.json"],
        gui_observations=observations, remaining_gui_blocker=blocker,
        gui_limitations=gui["limitations"], strict_startup_results=strict_results,
        strict_startup_scope=startup["scope"], source_files=len(sources),
        changed_existing_source_files=len(changed), new_source_files=len(new_sources),
        backup=evidence_by_name[backup_name], previous_releases_preserved=len(protected),
        protected_versions=sorted(versions), cli_checks=len(cli["checks"]),
        fixes=fixed, original_game_or_profile_writes_by_this_script=False,
        limitations=validation["limitations"], not_tested=validation["not_tested"])
    strict_table = "\n".join(
        f"| {row['map']} | Passed strict Host/resource/StartedMission | {row['seconds']:.3f} s |"
        for row in strict_results)
    doc = f"""# Development-10 Hotfix-02 (2026-10-17)

DIAGNOSTIC / UNFINISHED. BATTLE STATUS: UNAVAILABLE.

This hotfix addresses the reported Berlin `4EC4C8 actual GWorld+E0 clock
owner` refusal and the separate Source::New WorldStart classifier refusal.
No complete playable Rust battle is accepted.

## Delivery

- Folder: `release/Development-10-Hotfix-02`.
- EXE SHA-256: `{p.sha(exe)}` ({exe.stat().st_size} bytes).
- ZIP: `{archive.name}` ({archive.stat().st_size} bytes).
- ZIP SHA-256: `{p.sha(archive)}`.
- All {len(protected):,} previous Dev08/09/10/HF01 files verify unchanged.

The final seal verifies every payload hash, exact ZIP membership and CRC.
Existing original Data.rwd/reconstruction assets, x64 MSVC runtime and
compatible graphics drivers remain required. This is not a standalone
whole-game package; full-game parity and playable-battle acceptance are false.

## Fixes

{fixed[0]}

{fixed[1]}

Memory allocation and preceding native effects remain in their original
order on a reached failure. Failed mission graphs are retained; no reset,
retry, synthetic readiness, viewer-as-battle or original-engine fallback is
used to claim completion.

## Actual observations

The following strict tests use the separate QA executable
`{startup['executable']['sha256']}`. They share the frozen source with the
default-profile GUI client above, but are distinct executables and profiles.

| Fresh selected map | Bounded result | Elapsed |
|---|---|---|
{strict_table}

StartedMission verifies this startup path. It does not establish frontend
activation, native View/task readiness, first rendered battle frame, native
selection/orders, scheduled simulation or a playable battle. Timings are
diagnostics on this PC, not a performance guarantee.

Final packaged-EXE GUI observations, read from the sealed semantic review:

```json
{rendered(observations)}
```

Observed remaining GUI blocker: {blocker_text}

GUI review limitations:

{chr(10).join('- ' + str(value) for value in gui['limitations'])}

The exact client passed {len(cli['checks'])}/{len(cli['checks'])} recorded CLI
checks. New static Vulkan/DX12 previews were inspected under the bounded
visual-review scope; static assets are not original whole Model/View or
gameplay acceptance. Detailed validation, warnings and untested domains are
retained in `evidence/validation.json`.

## Evidence and continuation

Read `docs/NATIVE-PORT-RULES.md` first, then this handoff,
`research/development10/ACTIVE-SESSION-20261017.md` and the latest actual
startup/GUI observations. Preserve native order, same-owner lifetimes and
partial failures; continue from the observed blocker above without marking
missing children successful. Then finish actual World/Session/View activation,
native picking and orders, scheduled movement, combat, AI and victory before
accepting a playable match. Skirmish and multiplayer remain ahead of campaign.

- Final validation: `qa-output/development10/hotfix02-acceptance/validation.json`.
- GUI review/scripts/logs/screenshots: `qa-output/development10/hotfix02-acceptance/gui-review.json`
  and `qa-output/development10/hotfix02-gui`.
- Separate strict results: `qa-output/development10/hotfix02-native-startup`.
- Default-profile build: `qa-output/development10/hotfix02-build/build-record.json`
  ({build['seconds']:.3f} s; {len(sources):,} Cargo/crate source pins unchanged).
- Clock source/native proofs: `research/development10/berlin-clock-owner`.
- Fresh Actor predicate proof: `research/development10/world-start-fresh-classification`.
- Exact HF01 predecessors for all {len(changed)} changed existing source files:
  `{backup_name}`; {len(new_sources)} new source file is listed separately.

Cargo/crate fingerprints do not attest every external compiler input. No
full client unit suite, clean-PC deployment, every-map compatibility, 150%
laptop DPI, performance benchmark or real second-PC/internet multiplayer
acceptance is inferred. Native worker shutdown can still wait for cleanup;
no interruptible-loader or hard latency claim is made. Whole-game original
saves, campaign and full rendering/gameplay parity remain unfinished. No
defensible overall completion percentage or finish date has been established.

This handoff writes only its new result, this document and the continuation
journal. Sealed releases and original game/profile data are not modified.
"""
    journal = f"""

{HEADING}
DIAGNOSTIC / UNFINISHED; BATTLE STATUS: UNAVAILABLE; playable_battle=false,
standalone_game=false, full_game_parity=false. EXE {p.sha(exe)};
ZIP {archive.name}, {archive.stat().st_size} bytes, SHA {p.sha(archive)}.
Payload/ZIP/CRC and all {len(protected):,} prior Dev08/09/10/HF01 files verified.
Default-profile build {build['seconds']:.3f}s, {len(sources)} unchanged Cargo/crate
source pins. Exact HF01 backups: {len(changed)} changed existing files plus
{len(new_sources)} new file, {backup_name}.
Clock E0 uses actual retained Staged.clock; WorldStart Source::New ordinary
Actor uses proven class3 vtable+B4. No default/copy/readiness bypass.
Both fresh strict selected-map mission tests passed on separate QA EXE
{startup['executable']['sha256']}: Berlin {strict_results[0]['seconds']:.3f}s,
Firefight {strict_results[1]['seconds']:.3f}s. This is StartedMission scope,
not frontend activation/View/first frame/controls/gameplay acceptance.
Actual packaged-GUI observations: {json.dumps(observations, ensure_ascii=False)}
Observed remaining GUI blocker: {blocker_text}
Final validation {ACCEPTANCE}validation.json; final GUI review
{ACCEPTANCE}gui-review.json. Continuation docs/{doc_path.name}.
Read docs/NATIVE-PORT-RULES.md; resolve actual remaining children then verify
live battle, native selection/orders, scheduled movement/combat/AI/victory.
No original game/profile write by handoff; sealed earlier releases preserved.
"""
    p.write_new(result_path, result)
    with doc_path.open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(doc)
    with journal_path.open("a", encoding="utf-8", newline="\n") as stream:
        stream.write(journal)
        stream.flush()
        os.fsync(stream.fileno())
    print(rendered({"result": p.record(result_path), "document": p.record(doc_path),
                    "journal_appended": True, "playable_battle": False}))


if __name__ == "__main__":
    main()
