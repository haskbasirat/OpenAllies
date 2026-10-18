# Development-10 Animations-01 — 2026-10-18

DIAGNOSTIC / UNFINISHED. BATTLE STATUS: UNAVAILABLE.

## Delivered

New folder: `release/Development-10-Animations-01`. Run `Animation Browser.cmd`
to open a British infantry HQ death, or EXE → Editor → Animations.
The existing installation/reconstruction assets remain required; this is not
a standalone game. All 14,386 files in earlier sealed
Development08/09/10/HF01/HF02 releases, plus both native executables, were verified
unchanged. Original NIF/KF assets were byte-checked against Data.rwd by the inventory audit.

- EXE: 34931200 bytes; SHA-256 `4587442c60b609eddd84deb2d4aaff7d5eb0f312882207e7a2a7a92ba286930e`.
- ZIP: 262956065 bytes; SHA-256 `f9fa046ad4e1237029bc34384e043e8410dd26495301934c785dfff1057b85bb`.
- Manifest SHA-256: `6653f5414e05cfec62e95bc1b0d9d2ceba22d4d97ade406b610c9bfcb2805798`.
- Validation SHA-256: `14462e181fb6d56db4d4cf1b2b3303615dab061cc407d5b586895e7fb26d90ff`.
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

Production delta relative to sealed HF02: 4 changed existing files,
3 new files, no deletions, 2079 source pins.
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
