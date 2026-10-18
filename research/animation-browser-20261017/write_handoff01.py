"""Write the completed animation task handoff, preserving prior text byte-for-byte."""
import package_animation01 as p


def main():
    manifest = p.read(p.RELEASE / "manifest.json")
    p.need(manifest["animation_preview_validated"] is True and manifest["playable_battle"] is False, "Expected sealed animation preview")
    files = {name: p.record(p.RELEASE / name) for name in (p.EXE, p.DIAGNOSTIC_ARCHIVE, "manifest.json", "evidence/validation.json")}
    staged = p.read(p.RELEASE / "STAGING.json")
    index = p.read(p.ROOT / "backups/source-checkpoint-20261017-animations01/pre-animation-source-index-v2.json")
    doc = f"""# Development-10 Animations-01 — 2026-10-18

DIAGNOSTIC / UNFINISHED. BATTLE STATUS: UNAVAILABLE.

## Delivered

New folder: `release/Development-10-Animations-01`. Run `Animation Browser.cmd`
to open a British infantry HQ death, or EXE → Editor → Animations.
The existing installation/reconstruction assets remain required; this is not
a standalone game. All {len(staged['previous_release_files']):,} files in earlier sealed
Development08/09/10/HF01/HF02 releases, plus both native executables, were verified
unchanged. Original NIF/KF assets were byte-checked against Data.rwd by the inventory audit.

- EXE: {files[p.EXE]['bytes']} bytes; SHA-256 `{files[p.EXE]['sha256']}`.
- ZIP: {files[p.DIAGNOSTIC_ARCHIVE]['bytes']} bytes; SHA-256 `{files[p.DIAGNOSTIC_ARCHIVE]['sha256']}`.
- Manifest SHA-256: `{files['manifest.json']['sha256']}`.
- Validation SHA-256: `{files['evidence/validation.json']['sha256']}`.
- Final browser export: `research/animation-browser-20261017/export/cache-v2-final/animation-browser-manifest.json`,
  SHA-256 `a2f450c66d921c4007293ac3e0bda26a2aa87eada255865ffb5fc10958f3f2df`.

## Changes

The old browser scanned only 46 root-cache rigs (85 model/clip bindings).
The new manifest publishes 786 original model rigs and 3,316 model/clip bindings.
It includes all 20 HQ deaths, five Corps deaths, 77 building deaths and 119
building Pack/UnPack source models (these groups overlap). The full original
census remains 1,900 NIF +581 KF =2,481 files.

`aa-client/animation_catalog.rs` performs catalog discovery, path containment,
load-time rig SHA checks, source/clip filtering and explicit unavailable-source
listing. Clicking a rig queues/loads it automatically; current selection and
actually loaded identity are separate. Old clip controls stay disabled while
another selection loads. Latest queued selection wins after an in-flight load.

Inspector-only `aa-animation::load_preview/sample_preview` support sourced
Hermite scalar/vector channels, Euler orders and nonuniform scalar/vector TCB
curves. Native instruction probes established tangent ordering, endpoint and
scale semantics; PyFFI raw-source geometry checks independently exercise HQs.
Alpha-controller values replace material opacity; zero-scale geometry is hidden
without singular normal transforms. The GPU uploads geometry/alpha/visibility
together. Legacy/live gameplay sampling retains its strict domain.

Production delta relative to sealed HF02: {index['counts']['changed_existing']} changed existing files,
{index['counts']['new_files']} new files, no deletions, {index['counts']['current_source_files']} source pins.
Exact predecessor copies are in `backups/source-checkpoint-20261017-animations01/`;
`pre-animation-source-index-v2.json` pins them. No restricted SDK source was copied.
Original/export caches and rejected successor attempts remain immutable; only
final-manifest-listed browser files were added to this new release.

## Actually checked

- Default Windows x64 release build (`thin` LTO, one codegen unit, no profile
  overrides) passed in 508.765 seconds; all source pins unchanged during build.
- All 786 final rigs loaded and 3,316 clips sampled at five authored timestamps:
  16,580 finite CPU poses, zero refusals. Export audit verifies 8,094 mesh
  topologies, all 2,481 source hashes, original controller/status census and old
  cache receipts. Original raw TCB fields were repaired in a separate successor;
  91 channels/392 keys checked, rather than inventing missing zeros.
- Seven final sampler unit tests passed, including 170 reference poses across
  all 25 HQ/Corps death models (max position error 2.861023e-6; normal 4.172325e-6).
  Seventeen existing animation regressions passed separately. Two exact catalog
  unit tests and six exact adapter helper tests passed; their narrow seams are
  documented. Saved GPU unit test was typechecked, **not executed**.
- Exact packaged EXE: CLI discovers the installed 786-rig/3,316-clip catalog and
  240 unavailable animated sources. Short-name launch and implicit asset
  discovery also rendered successfully without `--assets`.
- Twelve actual headless GPU captures: British HQ/Corps deaths and US Corps
  Pack at three timestamps on Vulkan; US Infantry Jog pair and German bunker
  artillery on DX12. Root and independent reviewer inspected actual images.
- Actual main menu → Editor → Animations, filtering, automatic HQ loading,
  progressing 0.8301→2.8346 second death poses, pause, slider, Loop toggle,
  Pack/Unpack and latest rapid selection tested through the existing real egui
  event hook. Paused viewport remained pixel-identical across one second.
  Expanded unavailable-source entries were visibly inspected.
- Producer seal verified every ZIP entry/CRC/SHA and exact manifest membership.
  Independent post-seal audit, when present, is recorded separately at
  `qa-output/animation-browser-20261017/sealed-package-review.json`.

Complete scoped acceptance: `qa-output/animation-browser-20261017/validation01.json`.
Root actual observations: `qa-output/animation-browser-20261017/root-observations01.json`.
Independent source/render reviews: `research/animation-browser-20261017/review-client/`.
The first catalog test helper compared Windows extended-length and ordinary
path spelling as unequal; `samefile` proved identical files. The helper was
corrected and the full installed catalog recheck passed. Two initial standalone
catalog test-link attempts lacked host dependency/LTO flags; the final exact
module test harness passed. Neither required a product source change.

## Limits and continuation

Source-list completeness is separate from playback support. Of 1,324 animated
source files, 1,084 have at least one exported timeline and 240 remain explicitly
unavailable. Forty-seven nonlinear quaternion clips were quarantined; six models
then had no supported clips and were not published. 139 published rigs retain
unsupported controller records. Particles, UV/texture transforms, look-at/path,
lights/colors and some alpha/visibility effects remain incomplete. Clip markers
do not execute sounds, effects or gameplay events. Posed-vertex transparency
bounds do not establish native skinned world-sphere or whole-render parity.
Some displaced packing geometry extends beyond the initial-pose camera frame.

There is no new playable battle, original-save, AI, campaign or multiplayer
acceptance and no overall completion percentage or finish-time estimate.
No skirmish tests were rerun for this inspector task. HF02 battle blockers remain:
Berlin needs actual source Aspect preparation; Firefight needs per-tile native
terrain Variation carry-forward without conflating serialized palette slots.
Read `docs/DEVELOPMENT-10-HOTFIX-02-20261017.md` and
`docs/NATIVE-PORT-RULES.md` before resuming those separate children. Do not weaken
readiness/source identity checks or reinterpret the new animations as battle completion.

Native curve proof: `research/animation-browser-20261017/native-curves/findings.json`.
Quaternion-3 successor investigation is unfinished research; current sampler
and final export deliberately keep those clips unsupported. No late production
changes may be applied to this sealed build.
"""
    target = p.ROOT / "docs/DEVELOPMENT-10-ANIMATIONS-01-20261018.md"
    with target.open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(doc)
    checkpoint = p.ROOT / "backups/source-checkpoint-20261017-animations01/document-before"
    history_rows = []
    for name in ("HANDOFF.txt", "research/development10/ACTIVE-SESSION-20261017.md"):
        source = p.ROOT / name
        previous = p.record(source)
        backup = checkpoint / name
        p.copy_new(source, backup)
        old_bytes = source.read_bytes()
        note = ("\n\n## Development-10 Animations-01 SEALED — 2026-10-18\n"
            "Current inspector task completed in release/Development-10-Animations-01. "
            "786 rigs /3316 clips; all20HQ+5Corps/77building deaths represented. "
            "240 animated sources and47 nonlinear quaternion clips remain unsupported. "
            "GUI/CPU/GPU/package checks passed; battle UNAVAILABLE, nonstandalone.\n"
            f"EXE {files[p.EXE]['sha256']}; ZIP {files[p.DIAGNOSTIC_ARCHIVE]['sha256']}.\n"
            "Read docs/DEVELOPMENT-10-ANIMATIONS-01-20261018.md for exact scope/evidence/continuation. "
            "Previous historical notes below retain their original dated state; "
            "HF02 current battle blockers are unchanged.\n\n")
        source.write_bytes((note.encode("utf-8") + old_bytes) if name == "HANDOFF.txt" else (old_bytes + note.encode("utf-8")))
        history_rows.append(dict(before=previous, backup=p.record(backup), after=p.record(source)))
    p.write_new(p.ROOT / "backups/source-checkpoint-20261017-animations01/document-index.json", history_rows)
    p.write_new(p.QA / "release-result.json", dict(passed=True, release=str(p.RELEASE), files=files,
        validation=p.record(p.QA / "validation01.json"), handoff=p.record(target),
        rigs=786, clips=3316, original_sources=2481, unavailable_animated_sources=240,
        previous_release_files=len(staged["previous_release_files"]), playable_battle=False,
        standalone_game=False, full_game_parity=False, all_animations_playable_claimed=False))
    print(target)


if __name__ == "__main__":
    main()
