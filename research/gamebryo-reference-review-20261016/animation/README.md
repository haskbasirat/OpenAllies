# Animation reference review — 2026-10-16

The evaluation SDK helps identify the animation objects and coordinate spaces already present in AA. It does not establish exact engine, TimeGate, arithmetic, or gameplay parity. No runtime change follows from this review. The most useful next native investigation is the lifecycle from resource binding through sequence activation, clip replacement and missing-channel behavior; the existing clock oracle covers a narrower helper.

## Inputs and citation key

Read `docs/NATIVE-PORT-RULES.md` and the archive `README.txt`/`download-manifest.json`. The primary reference is `Gamebryo-1.1.2-Evaluation.zip`, SHA-256 `171e36a2d6ed8008b9d59711ebcf5ca3d906f7175d5843f821d6f15dc0feaa87`. H01 (`NiVersion.h`, lines 17–21) identifies **10.1.0.0**, matching the recorded AA library version strings, but not proving an exact build match. The 1.2 source RAR and 2.6 repository were not needed for these findings and supply no behavioral authority here.

`evidence-index.json` supplies each Hxx ZIP member path, each Dxx nested CHM path, full SHA-256, and physical line count. Thus `D05 §Notes, HTML lines 310–405` means that exact nested document, not a generated summary or a later SDK page. The outer CHM member is `Documentation/HTML/Gamebryo_1_1.chm`, SHA-256 `2f02a4ffda66beac525fbf4d78bc2ab1697111b2522c444002c24e13c1f4453d`. Rust references below are relative to `rust-rebuild/`; their current hashes are also indexed. Original paraphrases only are retained here.

Generic 7-Zip read the CHM in the isolated references directory. No SDK executable, sample, installer, exporter, game, native harness, build or test was run. This review wrote only these research artifacts; no runtime, cache, dependency, release or original game file changed. Existing oracle records were inspected, not revalidated by execution.

## Findings

### A01 — This is the keyframe-data/controller generation

The matching evaluation headers describe `NiKeyframeController` targeting `NiAVObject`, with independent position, quaternion/Euler rotation and scalar scale arrays in shared `NiKeyframeData`. Replacing keyframe data resets channel search indexes and recalculates time extrema. The evaluated include tree has no `NiTransformController.h` or `NiTransformInterpolator.h`. Do not substitute a later interpolator pipeline because a newer source tree documents it. [H04 lines 26–92; H05 lines 20–28,78–90; H06 lines 23–107; D03 §Description/Update, HTML lines 48–86.]

Current exporters explicitly accept `NiKeyframeController`/`NiKeyframeData` and resolve controlled target names; runtime loads their cached JSON projection, not general NIF/KF streams. That matches the object family but is intentionally narrower than the SDK. [Rust `crates/aa-animation/tools/export_original.py:69–103`; `crates/aa-animation/src/lib.rs:278–288,375–421`.]

### A02 — Flags have separate active and manager-update meanings

The SDK maps bit 0 to APP_TIME=0 / APP_INIT=1, cycle bits 1–2 to LOOP=0 / REVERSE=1 / CLAMP=2, bit 3 to active, bit 4 to backwards, and bit 5 to manager-update. APP_TIME takes caller time; APP_INIT has start/stop semantics. The manager-update flag controls whether a manager updates a controller. [H02 lines 38–96,175–195; H03 lines 15–55,130–155; D08 §Supported Member Functions, HTML lines 397–470.]

Current `ControllerClock` rejects bit 0 and uses native finite external-time behavior; retain that boundary. Its comments/error call bit 0 application-time, whereas the SDK's public enum name for value 1 is APP_INIT. Separately, `live_animation.rs:177–180` tests **bit 5**, but calls its rejection “Inactive source controller”. This is a diagnostic terminology mismatch: bit 5 means manager-update; it is not the active bit. Native `00883f50.c:24–45` supports the distinction by checking bit 5, temporarily setting bit 3 around the controller update, then clearing bit 3. No behavior correction is justified by this wording issue. [Rust `crates/aa-animation/src/controller_clock.rs:1–4,58–73`; `crates/aa-client/src/live_animation.rs:177–180`.]

### A03 — Preserve the native clock and its tested scope

The SDK gives useful names to frequency, phase, range, cycle and backwards, but does not settle AA's x87 stores or incremental frequency changes. The existing fixture records 72 ordered sequences / 1,152 calls across PC24/53/64; the Rust replay checks output bits and first/last/accumulated state. The fixture's native byte windows and seams are indexed here. This is evidence already present in the checkout, not a test pass from this review. [Rust `research/phase7-live-animation/native_controller_clock.py:1–15,17–34`; `crates/aa-animation/tests/controller_clock.rs:5–35`.]

The fixture deliberately covers mode 3 as well. The SDK enum defines only three cycle types; that absence must not override AA's observed clamp-like mode-3 result. Also, the base `DontDoUpdate` inline has active/repeated-input/scaled-time/forced-update guards, whereas the prior AA investigation identifies the keyframe constructor's separate scaled-time policy. Do not add a second clock query solely from the base helper. [H02 lines 60–67; H03 lines 130–145; native `008c0370.c:12–18`; Rust `research/phase7-live-animation/FINDINGS.txt:4–5`; `crates/aa-animation/src/controller_clock.rs:101–140`.]

### A04 — Controller reverse and sequence cycle policy are different layers

`NiTimeController` supports ping-pong REVERSE, while `NiControllerSequence::SetCycleType` only applies LOOP or CLAMP. A sequence setter also propagates frequency/phase/backwards to its controllers. Sequence offset is added before controller update. The asset inspector's explicit loop/clamp switch is therefore not native sequence initialization. [H18 lines 93–126; H19 lines 95–148; D04 §Cycle/Offset, HTML lines 224–297.]

Rust's live bridge uses each serialized controller's range/flags and replaces frequency with `(KF duration / gameplay length) * Character.rate`; its inspector wraps an explicit `[0,duration]` clock. Those are separate paths. AA `007170f0.c:80–106` writes wrapper loop policy, negative start into sequence offset, and controller frequency; `00883f50.c:16–20` supplies relative time. Trace the full resource/setup path before claiming serialized flags are the final game loop policy. [Rust `crates/aa-animation/src/runtime.rs:189–210`; `crates/aa-animation/src/controller_clock.rs:147–161`; `crates/aa-client/src/live_animation.rs:247–275`; `crates/aa-animation/src/lib.rs:426–444`; `crates/aa-client/src/animation_preview.rs:277–324`.]

### A05 — Keys: strict ordering agrees; endpoint completion and exact interpolation remain open

The SDK requires nonempty key arrays passed to interpolation, strictly increasing times, caller times within key range, and keys at controller start/end for every populated channel; exporter/streaming normally guarantees those boundaries. This confirms Rust's strict ordering check. Rust additionally clamps requests outside a channel's own first/last keys, and validates finite controller ranges without enforcing populated-channel boundary keys. For source-verified caches this is a bounded consumer choice, not proof of original streaming behavior or a general NIF validator. [D02 §Requirements, HTML lines 44–92; Rust `crates/aa-animation/src/lib.rs:166–187,375–396`.]

The SDK distinguishes no interpolation, linear, Bezier, TCB and Euler; Euler contains three independent float-key tracks and nine possible axis orderings. Current Rust accepts populated linear channels and bind-equivalent constant XYZ Euler only; the family exporter reduces only equal-valued, zero-tangent type-2 channels. Native interpolation, nonunit quaternion endpoints, sign choices, near-parallel threshold, rounding, varying splines and other Euler orders remain unproved. The SDK does not validate Rust's binary64 SLERP formula or its 0.9995 threshold. [H07 lines 38–45,70–82; H08 lines 29–42,62–70; D09 §Order, HTML lines 147–168; Rust `crates/aa-animation/src/lib.rs:195–223,399–418`; `crates/aa-animation/tools/export_families.py:18–24`; `research/animation-phase2/README.md:41–46,79–90`.]

The bounded cache audit found 410 live-infantry populated channels without keys at both controller endpoints; **every one has exactly one constant key**. No multi-key channel in either audited manifest lacks its endpoints, and the family caches have no endpoint mismatch. Therefore this is a serialized-cache/native-stream-fixup investigation, not an observed varying-channel fault. `cached-channel-audit.json` retains the exact hashed rig/clip/channel records; its script does not execute either loader.

### A06 — TRS and skin spaces are conceptually consistent

`NiTransform` holds a rotation matrix, translation and **uniform** scale, with a point rotated/scaled before translation. Skin data has a bind transform from root-bone-parent to skin plus a skin-to-bone transform per bone; the instance supplies current root-parent and bone objects. Its documented deformation combines those transforms, weights transformed bind vertices, removes scale for normals, and renormalizes normals after accumulation. Bone weights per vertex must sum to one. [H09 lines 20–33; H10 lines 15–28; H11 lines 38–75; H12 lines 100–142; H13 lines 31–62; D05 §Custom Skinning/Notes, HTML lines 240–405; D06 §BoneData/Weights, HTML lines 131–141,200–214,412–428.]

Rust's `skin_offset * inverse(root_global) * bone_global * inverse_bind` followed by geometry-global and the final `(x,z,-y)` render conversion agrees with those named spaces at the mathematical level. In particular the cache name `skeleton_root` corresponds to the SDK's **parent of the root bone**, not necessarily the first deforming bone. The source exporter uses that link. Rust keeps per-instance pose/clocks separate, while arrays of weights/transforms are source data. This is supporting reference evidence; exact native deformation order/rounding and CPU-vs-palette paths are not proven. The SDK's optional partitions have their own vertex remapping, palette, weight and triangle arrays; do not infer a universal fixed influence count. [H14 lines 45–72,91–110; Rust `crates/aa-animation/tools/export_original.py:57–62`; `crates/aa-animation/src/lib.rs:272–277,326–364,479–531`; `crates/aa-client/src/live_animation.rs:28–38`.]

### A07 — Sequence ownership/transition state is more than a selected clip

The evaluation already deprecates `NiSequence`/`NiKeyframeManager` in favor of `NiControllerSequence`/`NiControllerManager`; the old stream helper exists only for very old KF conversion. A sequence stores name/controller pairs, targets, an owner, offsets and transition/accumulation state. The new sequence's owner setter rejects assigning a nonnull owner when one already exists. Manager priority determines which competing controller updates a target. Blend snapshots, synchronized morphs, sums and cumulative root transforms are separate mechanisms. [H17 lines 34–58; H18 lines 131–159,191–240; H19 lines 79–93; H23 lines 20–21; D11 §Activation/Notes, HTML lines 207–220,455–519.]

The live bridge currently retains one clock vector, resets it when canonical animation identity changes, and samples each pose from fresh authored bind channels. An absent track/channel thus reverts to bind in this sampler. The SDK documents local-transform writes and last-controller-wins ordering but does **not** establish that AA's attach/detach, blend or deactivation resets absent channels to bind. Require a native sequential A → B → deactivate/rest oracle before changing or claiming this behavior. Native `007170f0.c:43–78` already shows distinct activation/morph/blend calls worth tracing. [D01 §Time Controllers, HTML lines 66–76; Rust `crates/aa-animation/src/lib.rs:247–263,446–448,473–483`; `crates/aa-client/src/live_animation.rs:255–260`; `research/phase7-live-animation/INTEGRATION.txt:10–12`.]

Read-only, manifest-hash-verified counts refine that follow-up:

| Current cache scope | Rig/clip bindings | Tracks | Empty channels on tracks | Ordered switches that drop a populated channel |
| --- | ---: | ---: | --- | ---: |
| Live infantry manifest | 5 / 20, sharing 4 source KFs | 460 | 420 scale; no translation/rotation | 0 of 60 |
| Separate inspector families | 41 / 65, 65 source KFs | 385 | 83 scale, 99 rotation | 20 of 124 |

All live infantry clips use the same populated target/channel sets; their six nodes without tracks are also shared across those clips. The concrete sparse-switch candidate is **HiResUSABattleshipTexas**, whose five recoil clips target different barrel translations. For example Recoil1 → Recoil2 drops Barrel1A/Barrel1B translation tracks, and the stateless sampler restores those nodes to bind. All 20 dropped-channel ordered switches in the family set belong to that rig. These are cache structural counts; no transition was executed and no AA result is inferred. Reproduce with `audit-cached-channels.py`; see `cached-channel-audit.json` for source hashes and every pair.

### A08 — KFM is a state-machine descriptor, not evidence of AA's game rules

The SDK's KFM tool describes model/root paths, event-code-to-KF/sequence-index mappings, transitions, text-key pairs, intermediate sequences and priority layers. Its optional ActorManager loads those resources and runs an event-code state machine. KF can contain multiple sequences; the SDK sequence loader accepts a stream index or all sequences. Current export's exactly-one-sequence restriction and live source-KF binding are narrower. [H21 lines 48–103,145–182; H22 lines 65–118; D04 §Creation, HTML lines 90–129; D16 §Description, HTML lines 44–84; Rust `crates/aa-animation/tools/export_original.py:69–71`; `crates/aa-client/src/live_animation.rs:151–195`.]

None of this proves AA uses the SDK ActorManager or the sample's event names/policy. AA's authored animation definitions, canonical aspect/set identity, gameplay length, cues, FX and scheduler remain game-specific native responsibilities. Keep the existing separate runtime/definition evidence. D15 also requires compatible model structures, unique controller target names, and APP_TIME for managed sequences; these are useful acquisition checks, not substitute gameplay logic. [D15 §Guidelines, HTML lines 62–107; Rust `crates/aa-animation/src/definitions.rs`; `research/phase7-animation-runtime/FINDINGS.txt:19–39`.]

### A09 — Geometry sharing and update optimization need their own evidence

`NiTriShape` is an unrestricted indexed triangle set; its shape data can be shared, and geometry carries its own optional skin instance. Exported 16-bit SDK counts/indices describe the original format, not a Rust allocation limit. Current Rust uses growable vectors/u32 topology and per-actor vertices; the skin/bind/math review does not prove animated LOD, visibility, material/effect controllers, or selective scene-update scheduling. The SDK says automatic selective-update flags depend on every animated local transform being represented by a transform controller; manual root updates are a special case. [H15 lines 21–43; H24 lines 67–84,137–138; D12 §Constructors, HTML lines 65–115; D10 §Requirements, HTML lines 44–76; Rust `crates/aa-animation/src/lib.rs:59–68`; `crates/aa-client/src/live_animation.rs:284–320`.]

### A10 — Actual AA assets require visibility/material/UV animation beyond bone pose

The SDK distinguishes these target effects: `NiVisController` changes `NiAVObject` app-culling through visibility keys; alpha controls a material property; texture-transform controls U/V translation, rotation and U/V scale of a texture map; material-color selects ambient/diffuse/specular/self-illumination; flip switches map textures over time. These are not changes to bone matrices. [D17 §Update, HTML lines 69–79; H28 lines 39–41; H30 lines 21–23; H31 lines 23–42; H32 lines 23–54; H33 lines 30–50.]

The existing phase3 corpus inventory (2,481 parsed files: 1,900 NIF / 581 KF, zero recorded parse errors) contains the following. Counts were recomputed from its stored rows, not by reparsing originals. `controller-coverage-audit.json` pins both source inventories, verifies the catalog's inventory hash, and preserves each selected class's filenames, source hashes and source-row line numbers.

| Controller | Instances | Files | Current pose bridge support |
| --- | ---: | ---: | --- |
| NiVisController | 1 | 1 | None |
| NiAlphaController | 665 | 179 | None |
| NiTextureTransformController | 56 | 42 | None |
| NiMaterialColorController | 14 | 5 | None |
| NiFlipController | 5 | 5 | None |

The single recorded visibility controller belongs to `Units/USA/USATankCO/USATankCODeath.nif`, target `Turret`, one `NiVisData` key, range 0–0, flags 44. The prior inventory does not retain that key's boolean value, so this review does not infer its resulting visibility. It does establish a concrete future visibility-oracle asset. [Rust `research/phase3-animation/source-inventory.json:796498`; `research/phase3-animation/animation-catalog.json:51909`; derived `controller-coverage-audit.json` §visibility_controller_details.]

The **119 building pack/unpack NIFs** in that catalog all contain varying Euler rotation, beyond the current constant-only Euler subset. Collectively they contain 4,407 keyframe, 39 alpha (26 files), 30 look-at (10 files), and 8 path controllers (6 files). None of these flagged pack/unpack rows contains NiVisController, texture-transform, material-color or flip. Thus NiVis is not demonstrated as the packing issue in this parsed set; varying Euler plus alpha/path/look-at are concrete unsupported features. Example: `GBRBrigadeArtilleryPack.nif` includes alpha plus varying Euler. [Rust `research/phase3-animation/animation-catalog.json:1088`; derived `controller-coverage-audit.json` §building_pack_unpack.]

The current KF exporter rejects non-keyframe controllers, and both `AnimationPreview::pose` and `ActorRig::pose` publish positions/normals only. Their tested bone/vertex poses do not establish culling, alpha, animated UV, texture selection or material-color playback. Trace native property/controller target binding and update ordering alongside the transform curves before implementing building pack/unpack or death effects. [Rust `crates/aa-animation/tools/export_original.py:73–81`; `crates/aa-animation/src/lib.rs:375–388`; `crates/aa-client/src/animation_preview.rs:286–311`; `crates/aa-client/src/live_animation.rs:284–320`.]

## Native follow-up order

1. Trace the AA resource/manager setup around `007170f0`, `00883f50`, and their activation/blend calls. Capture actual controller flags/times, loop override, ownership, target/priority order and first-frame setup for idle/move/attack/death.
2. Execute interpolation and deformation against actual AA routines with original source keys: exact endpoints, the irregular Attack1 quaternion, sign/near-parallel cases, distinct curve types, root-parent and bind transforms, two-plus influences and normals. Preserve x87/store/order details.
3. Execute sequential sparse-track A → B → stop/deactivate/reactivate cases, with/without blend and root accumulation. Compare local target transforms and resulting vertices, not just clip labels or screenshots.
4. Add native visibility/material/texture-controller fixtures using the concrete A10 assets. Building pack/unpack requires varying Euler and any accompanying alpha/path/look-at updates; bone-only rendering cannot establish its parity.
5. Only then extend resource coverage, nonlinear channels, dynamic aspect/model changes or root motion. The SDK's ability to do something is not evidence that AA chooses that behavior.

The only immediate discrepancy identified here is flag/clock terminology in diagnostics/comments. All other differences above are explicit scope limits, conceptual matches or proposed native investigations. This review grants no new runtime parity claim.
