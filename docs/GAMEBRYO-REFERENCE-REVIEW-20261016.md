# Gamebryo reference review — October 16, 2026

The matching evaluation headers and documentation are useful for identifying
scene objects, material state and animation interfaces. The review found real
data that the current ordinary-model path does not retain. These references
provide a much better investigation map; they do not supply the TimeGate game
source or establish native behavior by themselves.

This was a reference audit. SDK implementation was not added to Rust or any
release. No SDK installer, sample, exporter, library or engine binary was run.
Original AA.exe instructions and source-bound native tests remain authoritative.

## Which reference to use

| Reference | Observed version evidence | Use and limitation |
| --- | --- | --- |
| `Gamebryo-1.1.2-Evaluation.zip` | `NiVersion.h` internal `10.1.0.0`; AA.exe has matching NiSystem/NiMain/NiDX9/NiAnimation strings | Best available header/documentation reference. Exact compiler build, ABI and TimeGate changes remain unverified. This is an evaluation SDK, not full engine source. |
| `Gamebryo-1.2-Source.rar` | SDK `1.2.2.6`, separately declared NIF format `10.2.0.0` | Later source-layout and symbol leads only. The reviewed member/path inventory contains 1,553 CoreLibs cpp/h/inl paths; that count is not a completeness or compatibility certificate. Implementation algorithms were not adopted. |
| [Gamebryo 2.6 version header](https://raw.githubusercontent.com/sigmaco/gamebryo-v2.6/master/CoreLibs/NiSystem/NiVersion.h) | SDK `2.6.0.0`, separately declared NIF format `20.6.0.0` | Later architecture reference. Repository not cloned or integrated. DeepWiki could not be read through the web tool; its generated explanations were not used. |

SDK and NIF version numbers are different concepts. All 1,900 original NIF
files were read: 1,859 are `10.1.0.0`, 31 are `10.0.1.0`, and 10 are `4.2.2.0`.
The existing PyFFI reader parsed all 1,900 without errors. That validates this
bounded inventory, not original-engine stream/link or render parity. Current
Rust `aa-assets` loads converted mesh JSON; `aa-formats` handles RWD archives,
not a general native NIF scene graph.

The inspected evaluation NiMain/NiAnimation DLLs have x86 PE headers. They were
read as data, never loaded. They are not replacements for the x64 Rust client.

Both downloaded archives contain signed-agreement notices. No agreement or
reuse/distribution permission was verified. Reading the available references
does not establish an open-source license. Raw selected documentation extracts
are isolated under the original references folder; the Rust research reports
retain our own observations, metadata, hashes and citations.

## Concrete results and priorities

| Priority | Finding in the existing assets/code | Required next step |
| --- | --- | --- |
| 1. Camera-facing models | 354 `NiBillboardNode` objects in 108 original files. 105 of those source files, containing 340 nodes, occur in the 1,847-model converted manifest. The flattened ordinary-model representation loses node modes/render-time rotation. The lake-lily/fish example has nine billboard parents with quad geometry. Manifest membership alone does not prove every node becomes visible ordinary geometry. | Compare that source asset in AA across camera directions; locate the native billboard routine and prove each reached mode, parent transform order and update timing before retaining and implementing it. Keep the separate VFX path distinct. |
| 2. Texture sampling and apply mode | In 7,113 retained non-team mesh rows, 285 base maps clamp both axes, 165 use bilinear filtering without mips and one uses nearest without mips. The ordinary renderer instead uses a fixed repeating trilinear sampler. Also, 58 rows use REPLACE rather than MODULATE. The current model schema discards these choices. | Add original-renderer state/pixel fixtures, beginning with `IwoJimaBeachBunkerMG01/Plane04` for NEAREST, then clamped/BILERP textures and REPLACE. Retain authored per-map state after the native behavior is established. |
| 3. Building pack/unpack animation | The existing source-bound animation inventory has 119 packing/unpacking NIFs with varying Euler rotation. Twenty-six also contain alpha controllers, ten look-at and six path controllers. Current ordinary pose playback does not implement this complete combination. No visibility controller occurs in this subset. | Prove varying Euler plus property/path/look-at update order using actual building assets. Extending bone positions alone cannot complete these animations. |
| 4. Sparse clip transitions | Manifest-verified live infantry has no dropped populated channel across 60 ordered clip switches. The separate inspector families have 20 sparse-switch cases, all on `HiResUSABattleshipTexas` recoil clips. The sampler currently rebuilds missing channels from bind pose. | Execute native recoil A → B → stop/reactivate cases and compare target transforms. The SDK does not determine AA's reset/blend/deactivation policy. |
| 5. Property and effect animation | The existing 2,481-file NIF/KF catalog records alpha controllers in 179 files, UV-transform controllers in 42, material-color in five, flip in five, and visibility in one. The ordinary pose bridge only publishes positions/normals. The separate VFX extractor already handles several property-controller types. | Trace actual target binding/update order for the ordinary path; do not label all VFX behavior absent. Use the recorded tank-death Turret visibility controller as a specific test asset; its stored catalog omits the boolean key value, so visibility was not inferred. |

The material counts above refer specifically to retained **non-team ordinary
meshes**. Their new audit checked all 1,847 model source hashes and all 11,035
sidecar mesh-row identities; it is not a screenshot or native rendering test.
Animation-catalog controller counts were recomputed from existing inventories,
not from a new native animation run. Exact rows and denominators are preserved
in the linked subreports.

## Changes this review does not justify

* The complete current NIF inventory found no switch/LOD nodes and no unflagged
  triangle geometry beneath an app-culled ancestor. Those importer omissions
  remain coverage gaps, not demonstrated current serialized-corpus defects.
* Many retained meshes have authored vertex colors, but every present effective
  vertex-color source mode in the audited ordinary population is IGNORE.
  Multiplying all those colors into diffuse would not follow that state.
* The audited retained population contains no explicit CW-only/CCW-only draw
  modes, active stencil state or shade property. Generic SDK capabilities do
  not prove these explain the user's screenshots.
* Existing original-code alpha/depth and queue/sort fixtures are stronger than
  generic SDK prose. Preserve them, including cases where a broad documentation
  description does not settle AA's effective D3D9 behavior.
* Existing animation-clock evidence remains authoritative. SDK enum names do
  not invalidate AA's observed mode-3 result. Bit 5 is manager-update, so the
  current "Inactive source controller" message is misleading terminology;
  this review has not changed its runtime behavior.
* Degenerate triangle-strip connectors explain a large difference between raw
  primitive counts and expanded nondegenerate triangles. The current strip
  expansion was not demonstrated incorrect by this review.

## Important correction for battle-startup work

The Development-09 checkpoint's phrase "Settlement model-fit radius" must not
be read as a Gamebryo mesh-bound dependency. Direct original/v14 instruction
inspection shows `0x460940` fitting TimeGate gameplay `ObjectData+0x38` shapes
from WallSet segment/gate/corner definitions. Its `+0x48` input is a gameplay
half-width; the corner `+0x3C` is a gameplay radius. Existing
`aa-rules::objects::shape` and the completed shared definition mutations are
the relevant producer leads. `NiBound` is a different visual sphere.

Seven inspected native function byte ranges match original/v14. This is
static evidence, not a new arithmetic oracle. Exact fitting, shared mutations,
FSIN/FCOS and spill/order proof remain required. No raw wall diameter or mesh
sphere should replace the missing completed value. The sealed Development-09
checkpoint is preserved; this supplement clarifies that wording.

## Reproducible evidence and continuation

* [Models, scene graph and NIF inventory](../research/gamebryo-reference-review-20261016/models/REVIEW.md)
* [Materials and authored-state audit](../research/gamebryo-reference-review-20261016/materials/README.md)
* [Material native-validation queue](../research/gamebryo-reference-review-20261016/materials/validation-queue.md)
* [Animation and controller coverage](../research/gamebryo-reference-review-20261016/animation/README.md)
* [Native geometry / Settlement clarification](../research/gamebryo-reference-review-20261016/native-geometry/README.md)
* [Version/provenance evidence](../research/gamebryo-reference-review-20261016/reference-identity.json)
* `qa-output/gamebryo-reference-review-20261016/` records source/input/release
  preservation before/after this review. `HANDOFF.txt` links this supplement.

The reports contain exact archive/nested-CHM members, line/section references,
hashes, native addresses and read-only inventory scripts. Review observations
are explicitly separated from missing native differential tests. No new game
completion percentage, performance result or finished feature is established.
