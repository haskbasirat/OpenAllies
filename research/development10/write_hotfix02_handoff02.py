"""Root-only candidate2 handoff writer; execute only after the final HF02 seal.

No work occurs on import. No build, client, stage/seal or original-game write.
Root must serialize the durable journal append with other journal writers.
Existing result/doc files are never replaced; partial outputs remain for review.
"""
from datetime import datetime, timezone
import json
import os
from pathlib import Path

import package_hotfix02 as p


ACCEPTANCE = "qa-output/development10/hotfix02-acceptance02/"
HEADING = "## Development-10 Hotfix-02 SEALED (2026-10-17)"


def main():
    manifest_path = p.RELEASE / "manifest.json"
    p.need(manifest_path.is_file(), "Root must seal the final HF02 package first")
    manifest = p.read(manifest_path)
    p.need(manifest.get("release_mode") == "diagnostic" and manifest.get("battle_status") == "unavailable"
           and manifest.get("playable_battle") is False and manifest.get("full_game_parity") is False
           and manifest.get("standalone_game") is False and manifest.get("unfinished_battle_ui_labelled") is True
           and manifest.get("archive") == p.DIAGNOSTIC_ARCHIVE,
           "A final diagnostic/non-standalone/no-battle seal is required")
    archive, exe = p.RELEASE / manifest["archive"], p.RELEASE / p.EXE
    p.need(archive.is_file() and exe.is_file() and p.sha(exe) == manifest["executable_sha256"],
           "Final matching EXE and diagnostic ZIP must exist")
    p.verify_package()  # Read-only payload/ZIP/CRC/previous-release verification.
    validation_path = p.RELEASE / "evidence/validation.json"
    validation = p.read(validation_path)
    digest = manifest["executable_sha256"]
    p.need(validation.get("passed") is True and validation.get("candidate") == "hotfix02-candidate02"
           and validation.get("executable_sha256") == digest and validation.get("release_mode") == "diagnostic"
           and validation.get("battle_status") == "unavailable" and validation.get("playable_battle") is False
           and validation.get("claims_native_battle") is False and validation.get("full_game_parity") is False,
           "Final matching candidate2 diagnostic validation required")
    evidence = {row["path"]: row for row in validation["evidence_files"]}
    p.need(len(evidence) == len(validation["evidence_files"]), "Duplicate evidence")
    p.verify_rows(p.ROOT, validation["evidence_files"])

    def accepted(name):
        p.need(name in evidence, f"Missing final accepted evidence: {name}")
        path = p.inside(p.RELEASE, "evidence/" + name)
        p.need(path.stat().st_size == evidence[name]["bytes"] and p.sha(path) == evidence[name]["sha256"],
               f"Sealed evidence differs: {name}")
        return p.read(path)

    writer = p.record(Path(__file__).resolve())
    p.need(evidence.get(writer["path"]) == writer, "Final validation must pin this exact handoff writer")
    gui = accepted(ACCEPTANCE + "gui-review.json")
    p.need(gui.get("passed") is True and gui.get("executable_sha256") == digest
           and "remaining_blocker" in gui, "Final exact-client GUI review and explicit blocker required")
    blocker = gui["remaining_blocker"]
    observations = gui.get("observations", gui.get("maps"))
    p.need((blocker is None or isinstance(blocker, str) and bool(blocker.strip()))
           and isinstance(observations, (dict, list)) and bool(observations) and bool(gui.get("limitations")),
           "Final observed GUI results/limits must be explicit, never inferred")
    startup = accepted(ACCEPTANCE + "startup-review.json")
    p.need(startup.get("passed") is True and startup.get("source_files_match_gui_build") is True
           and startup.get("playable_battle") is False, "Bounded strict-startup review required")
    strict = []
    for row in validation["strict_startup_results"]:
        p.need(evidence.get(row["path"]) == row, "Strict result must be final pinned evidence")
        value = accepted(row["path"])
        p.need(value.get("strict_startup_passed") is True and value.get("source_unchanged_since_build") is True
               and value.get("playable_battle") is False and value["executable"] == startup["executable"],
               "Strict result must retain the separate QA executable/scope")
        strict.append(value)
    p.need(len(strict) == 2 and {row["map"] for row in strict} == {"berlin", "firefight"},
           "Both actual candidate2 strict map results required")
    build = p.read(p.RELEASE / "evidence/build-record.json")
    p.need(build.get("passed") is True and build.get("source_unchanged_during_build") is True
           and build.get("executable_sha256") == digest and build.get("profile") == "release"
           and startup["executable"]["sha256"] != digest, "Frozen separate GUI/default and QA builds required")

    index = validation["source_change_index"]
    p.need(evidence.get(index["path"]) == index, "Final validation must pin the final backup index")
    backup = accepted(index["path"])
    sources = {row["path"]: row for row in build["source_files"]}
    old = p.read(p.ROOT / "release/Development-10-Hotfix-01/evidence/build-record.json")
    old_sources = {row["path"]: row for row in old["source_files"]}
    changed = {name for name in sources.keys() & old_sources.keys() if sources[name] != old_sources[name]}
    added = sources.keys() - old_sources.keys()
    counts = dict(source_files=len(sources), changed_existing=len(changed), new_files=len(added))
    p.need(counts == validation["source_change_counts"] and len(sources) == len(build["source_files"])
           and not old_sources.keys() - sources.keys()
           and len(backup["files"]) == len(changed) and {row["source"] for row in backup["files"]} == changed
           and len(backup["new_files"]) == len(added) and set(backup["new_files"]) == added,
           "Final dynamic source counts and complete predecessor coverage differ")
    for row in backup["files"]:
        p.need(row["sealed_predecessor"] == old_sources[row["source"]]
               and row["backup"]["bytes"] == row["sealed_predecessor"]["bytes"]
               and row["backup"]["sha256"] == row["sealed_predecessor"]["sha256"], "Wrong HF01 predecessor")
        p.verify_rows(p.ROOT, [row["backup"]])
    protected = p.read(p.RELEASE / "STAGING.json")["previous_release_files"]
    p.need(len(protected) == 10775 and {row["path"].split("/")[1] for row in protected}
           == {"Development-08", "Development-09", "Development-10", "Development-10-Hotfix-01"},
           "Prior-release protection differs")
    cli = accepted(ACCEPTANCE + "cli-results.json")
    result_path = p.ROOT / "qa-output/development10/hotfix02-release-result.json"
    doc_path = p.ROOT / "docs/DEVELOPMENT-10-HOTFIX-02-20261017.md"
    journal_path = p.ROOT / "research/development10/ACTIVE-SESSION-20261017.md"
    p.need(not result_path.exists() and not doc_path.exists(), "Never replace an existing handoff")
    p.need(journal_path.is_file() and HEADING not in journal_path.read_text(encoding="utf-8"),
           "Journal missing or this handoff already appended")
    blocker_text = blocker if blocker is not None else (
        "The final GUI review explicitly identifies no remaining blocker within its bounded observations; this is not playable-battle acceptance.")
    result = dict(schema=2, candidate="hotfix02-candidate02", utc=datetime.now(timezone.utc).isoformat(),
        status="sealed diagnostic HF02; final GUI observations recorded separately from strict mission startup",
        release_mode="diagnostic", battle_status="unavailable", playable_battle=False,
        full_game_parity=False, standalone_game=False, executable=p.record(exe), archive=p.record(archive),
        manifest=p.record(manifest_path), validation=p.record(validation_path), writer=writer,
        gui_review=evidence[ACCEPTANCE + "gui-review.json"], gui_observations=observations,
        remaining_gui_blocker=blocker, gui_limitations=gui["limitations"], strict_startup_results=strict,
        strict_startup_scope=startup["scope"], source_change_index=index, source_change_counts=counts,
        previous_releases_preserved=len(protected), cli_checks=len(cli["checks"]),
        original_game_or_profile_writes_by_this_script=False,
        limitations=validation["limitations"], not_tested=validation["not_tested"])
    strict_table = "\n".join(f"| {row['map']} | Strict Host/resource/StartedMission passed | {row['seconds']:.3f} s |" for row in strict)
    text = f"""# Development-10 Hotfix-02 (2026-10-17)

DIAGNOSTIC / UNFINISHED. BATTLE STATUS: UNAVAILABLE.

## Delivery

- Folder: `release/Development-10-Hotfix-02`.
- EXE SHA-256: `{p.sha(exe)}` ({exe.stat().st_size} bytes).
- ZIP: `{archive.name}` ({archive.stat().st_size} bytes).
- ZIP SHA-256: `{p.sha(archive)}`.
- All {len(protected):,} previous Dev08/09/10/HF01 files verify unchanged.

Payload hashes, exact ZIP membership and CRC are verified. Original
Data.rwd/reconstruction assets, x64 MSVC runtime and compatible graphics
drivers remain required. This package is not standalone or a complete game.

## Fixes and evidence

The reported visibility-memory timestamp failure now receives the actual
retained Staged.clock owner explicitly; no copied/default time substitutes
for GWorld+E0. WorldStart uses the proven Source::New class3 Actor vtable+B4
result after actual identity/storage/source checks, preserving saved/Group8
behavior. Candidate2 also forwards the captured MapResource load argument
through activation and lends the actual retained current MapResource mode
through explicit FX callback context. Targeted reviews in final validation define the exact
accepted domains. Prior native stores and partial failures are preserved.

Unsealed candidate1 is retained in
`backups/dev10-hotfix02-unsealed-candidate01`; its GUI observations and EXE
are historical and are not candidate2 acceptance.

Default-profile GUI build: {build['seconds']:.3f}s, {len(sources):,} unchanged
Cargo/crate source pins. Every one of {len(changed)} changed existing files
has an exact sealed-HF01 predecessor; {len(added)} new source file(s) are
listed separately in `{index['path']}`. These fingerprints do not attest
every external compiler include.

## Actual observations

Separate strict QA executable: `{startup['executable']['sha256']}`.

| Map | Bounded result | Elapsed |
|---|---|---|
{strict_table}

StartedMission is startup scope. It does not establish frontend activation,
native View/task readiness, first battle frame, native controls or scheduled
movement/combat/AI/victory. These timings are not a performance guarantee.

Final packaged-EXE GUI observations, from the sealed semantic review:

```json
{json.dumps(observations, indent=2, ensure_ascii=False)}
```

Observed remaining GUI blocker: {blocker_text}

{chr(10).join('- ' + str(value) for value in gui['limitations'])}

The exact client passed the {len(cli['checks'])} recorded CLI checks.
Static Vulkan/DX12 preview observations retain all visual-review warnings;
they are not native whole-Model/View or gameplay acceptance. Detailed checks,
observed limits and untested domains are in `evidence/validation.json`.

## Continuation

Read `docs/NATIVE-PORT-RULES.md` first, then
`research/development10/ACTIVE-SESSION-20261017.md` and the latest actual
observations. Continue from the explicit GUI blocker above, preserving
same-owner lifetimes, native callback order and failure prefixes. Do not
reset a failed graph, treat a missing child as success or use a viewer as
a battle. Finish actual World/Session/View activation, native picking and
orders, scheduled movement, combat, AI and victory before accepting a match.

- Final evidence: `{ACCEPTANCE}validation.json` and `gui-review.json`.
- GUI artifacts: `{validation['evidence_roots']['gui']}`.
- Separate strict startup: `{validation['evidence_roots']['startup']}`.
- Default-profile build: `{validation['evidence_roots']['build']}build-record.json`.
- Source-change index: `{index['path']}`.

No whole-game saves/campaign, real second-PC/internet multiplayer, clean-PC
standalone deployment, 150% laptop DPI, every-map or performance acceptance
is inferred. Native cleanup may still wait; no interruptible-loader claim.
Skirmish and multiplayer remain ahead of campaign. No defensible overall
completion percentage or delivery date has been established.

This writer changes only its new result/document and continuation journal.
Original game/profile data and all sealed releases remain untouched.
"""
    journal = f"""

{HEADING}
Candidate2 diagnostic seal; battle unavailable, playable/full-parity/standalone
false. EXE {p.sha(exe)}; ZIP {archive.name}, SHA {p.sha(archive)}.
All {len(protected)} prior Dev08/09/10/HF01 files verify unchanged. Source
counts {json.dumps(counts)}; exact HF01 index {index['path']}.
Separate strict startup QA {startup['executable']['sha256']}; observations
{json.dumps([{'map': row['map'], 'seconds': row['seconds'], 'strict_startup_passed': row['strict_startup_passed']} for row in strict])}.
No activation/View/first-frame/controls/gameplay readiness inferred.
Actual final GUI observations: {json.dumps(observations, ensure_ascii=False)}
Observed remaining GUI blocker: {blocker_text}
Final validation {ACCEPTANCE}validation.json. Continuation docs/{doc_path.name}.
Read docs/NATIVE-PORT-RULES.md; finish reached actual children, then live
native selection/orders and scheduled movement/combat/AI/victory acceptance.
"""
    p.write_new(result_path, result)
    with doc_path.open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(text)
    with journal_path.open("a", encoding="utf-8", newline="\n") as stream:
        stream.write(journal)
        stream.flush()
        os.fsync(stream.fileno())
    print(json.dumps(dict(result=p.record(result_path), document=p.record(doc_path), journal_appended=True)))


if __name__ == "__main__":
    main()
