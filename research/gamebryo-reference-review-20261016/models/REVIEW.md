# Model reference review — 2026-10-16

The evaluation archive is a useful map of scene-graph concepts and API names.
It does not establish the AA executable's object layouts, exact streaming
implementation, numerical behavior, or TimeGate changes. The present Rust model
path consumes converted static JSON, so these findings primarily describe the
information a future native model path must preserve and the native checks needed
before changing it. No production code, converted assets, releases, dependencies,
or manifests were changed. No SDK program, installer, library, or sample was run.
No SDK implementation or documentation text was copied into this evidence.

## Evidence and confidence

`source-citations.json` identifies every cited complete local file or archive
member, SHA-256, original line ranges, and documentation section. `SDK-H*` means
an evaluation ZIP header; `SDK-D*` means an HTML document inside its CHM. CHM line
numbers refer to original HTML lines before text extraction. The complete archive
remains authoritative. Selected CHM entries were read to memory with the existing
7-Zip installation, without rendering scripts or saving documentation into this
rewrite. ZIP headers and the CHM member were read with Python `zipfile`.

- **Observed** means directly read native-file bytes or current rewrite code.
- **Reader interpretation** means the existing PyFFI 2.2.3 conversion reader,
  including its existing legacy UV-field shim, interpreted original asset data.
- **SDK reference** describes the evaluation headers/docs. It is not AA parity.
- **Native validation required** identifies the next proof using original
  AA.exe functions or a controlled native visual capture. None was run here.

The evaluation ZIP SHA-256 is
`171e36a2d6ed8008b9d59711ebcf5ca3d906f7175d5843f821d6f15dc0feaa87`.
The CHM member SHA-256 is
`2f02a4ffda66beac525fbf4d78bc2ab1697111b2522c444002c24e13c1f4453d`.

## 1. Version match is useful, but the native model corpus has three versions

**Observed:** `NiVersion.h` identifies engine 10.1.0.0, and the executable string
inventory contains matching NiMain/NiDX9/NiAnimation versions. This is a string
match only. A complete read-only census of the recovered original Data tree
found **1,900 NIF files**. Header strings and the following little-endian version
word agree:

| NIF file version | Files | Original example |
|---|---:|---|
| 10.1.0.0 | 1,859 | `Bridges/LondonBridge/LondonBridge.nif` |
| 10.0.1.0 | 31 | `Buildings/Campaign/BeachMinePole/BeachMinePole.nif` |
| 4.2.2.0 | 10 | `Doodads/Shared/Waves/Waves01.nif` |

`native-model-header-inventory.json` records relative path, length, full original
file SHA-256, text header, and version word for every file. The census is exact
for that recovered directory at review time; it is not a claim that every file
is loaded in normal gameplay. Reproduce it with `scan_native_model_headers.py`.

**Current path:** `aa-formats` reads RWD archives, not NIF objects. `aa-assets`
resolves model keys to converted JSON; `ModelAsset::prepare` validates the
intermediate schema, Y-up marker and mesh presence. `nif_version` is retained
metadata rather than a native NIF parser dispatch. The existing Python converter
already has a 4.2.2.0-specific UV field-width correction. [SDK-H1; NATIVE-1;
RUST-1, RUST-2, RUST-3; CONVERTER-1:25–44]

**SDK reference:** `NiStream` distinguishes the file version from a user-defined
version, resolves object links after reading, and registers loaders by type.
Neither its declarations nor C++ in-memory member types specify all on-disk
byte layouts. SDK product numbering also must not be treated as a NIF number.
[SDK-H9; SDK-D8 §Versioning Functions]

**Native validation required:** cover all three actual file versions, original
registered/custom object types, user-defined version checks and load/link error
paths. Compare source-bound decoded fields and original AA loader results before
claiming compatibility. A future 64-bit parser should keep original wire widths
and grow runtime storage; the SDK's 16-bit geometry counters are not a reason to
reintroduce a global runtime allocation cap. [SDK-H5, SDK-H6; RULES]

## 2. Billboard orientation is an actual static-conversion gap

**Reader interpretation:** the original
`Doodads/Temperate/TemperateLakeLilyFish/TemperateLakeLilyFish01.nif`
contains **nine NiBillboardNode objects**, each parenting an ordinary four-vertex,
two-triangle mesh. The source SHA-256 is
`3cdedf87250948b86f46a86db41f01d71f2a219e098dd65b3355a02d2de5669f`.
The smaller original Speaker model also contains a billboard, although its
gameplay placement visibility is handled separately by existing research.
`native-model-metadata-sample.json` records node identities and parent ordinals.

The complete existing-reader census parsed all **1,900 originals with zero read
errors** and found **354 billboard nodes in 108 files**. Of those, 105 files
(containing 340 source billboard nodes) are in the current 1,847-model converted
manifest. Manifest membership alone does not mean every source node is rendered;
helpers and other exclusions still apply. See `native-model-node-inventory.json`
and `billboard-conversion-coverage.json` for paths, counts and source hashes.

**Observed:** the converter treats billboard nodes as ordinary parents and bakes
their stored transform. The JSON/Rust mesh representation carries names and a
static pose but no billboard node class, mode or camera-dependent operation.
[CONVERTER-1:169–222; RUST-4:247–284]

**SDK reference:** billboard behavior is performed during display traversal; the
documentation distinguishes five rotation modes and the camera-facing rotation
from the node's stored local rotation. It also explicitly leaves world-up choice
to the application. Consequently, SDK camera defaults are not grounds to change
AA's already established native/render basis. [SDK-H10; SDK-D9 §§Description,
Billboard Rotation Modes, Billboard Nodes and Rotations]

**Native validation required:** use the lake-lily model as an actual native
fixture, sweep camera orientation, and identify the original display function,
mode mapping, parent-transform order and update timing. Preserve those fields in
a future source-bound scene representation before reproducing the behavior. The
SDK is a guide to locating the mechanism, not proof of the precise AA algorithm.

## 3. Scene visibility cannot be reduced to a geometry's own flag

**Observed:** `convert_models.py` computes the current node's bit-zero hidden
flag, checks it only when exporting triangle geometry, and continues descending
through a hidden non-geometry parent. It also traverses all ordinary children,
has no active-child handling for `NiSwitchNode`, and chooses only `children[:1]`
for `NiLODNode`. Rust receives no node flags, child slots, selected-child indices
or LOD data. [CONVERTER-1:169–222; RUST-4:247–284]

**SDK reference:** app-culling an object suppresses its child subtree. Switch
nodes draw one selected child or none; LOD nodes select through LOD data and
camera state. Node child arrays may contain null slots, so an active-child index
is not an index into a compacted non-null-child list. Range and screen-space
LOD data are separate mechanisms. [SDK-D2 §Culling Functions:938–950; SDK-D3
§Child Array Functions; SDK-D5; SDK-D6; SDK-H11–H14]

These are distinct from triangle face culling or stencil draw mode. They are
also distinct from the existing native Thing-level invisible flag handling.
An SDK-supported type need not occur in AA's serialized model corpus. Do not
promote switch/LOD work solely because the engine binary contains their names.

**Reader interpretation, full corpus:** all 1,900 original models parsed; there
were **zero NiSwitchNode objects, zero NiLODNode objects, and zero unflagged
triangle geometries beneath app-culled ancestors**. Those particular conversion
omissions therefore do **not** establish a present serialized-asset defect.
Runtime-created nodes and controller-driven changes are outside this census.
Billboards, by contrast, are a demonstrated input-representation gap.

**Native validation required:** prioritize actual corpus occurrences. For any
hidden parent, trace AA's app-cull traversal and game controller changes rather
than propagating an immutable authoring flag by assumption. For any switch/LOD
asset, preserve original child-slot identity, selected index and dynamic state,
then execute/capture its native selector. No switch or LOD example was present
in the complete parsed corpus. The inventory's type coverage is limited to
scene AVObject descendants; it is not a controller or all-NIF-block census.

## 4. Triangle-strip primitive counts include degenerate connectors

**SDK reference:** strips consist of an ordered list of per-strip lengths and
indices. Winding alternates across successive strip primitives. Source triangle
indices/counters use 16-bit elements in this API; that is separate from the Rust
intermediate's `u32` triangle-list indices. [SDK-H5; SDK-D4 §§Description,
NiTriStripsData Connectivity]

**Reader interpretation, actual 10.1.0.0 file:** original LondonBridge, SHA-256
`ef94aabe28ff0d851e0c73be1a1ccdb69bab1bde0a9b0740bd2bb5cb7d1b1e2c`,
contains these two strip geometries:

| Mesh | Strip length | Authored primitive count | Existing reader's nondegenerate triangles |
|---|---:|---:|---:|
| LondonBridge | 1,638 | 1,636 | 556 |
| Road | 723 | 721 | 350 |

BeachMinePole's visible strip similarly reports 122 primitives and yields 87
nondegenerate triangles. Its SelectionBox is a separate hidden helper, not the
visible-pole result. Waves01 supplies a four-vertex/two-triangle strip in the
4.2.2.0 format. [native-model-metadata-sample.json, source hashes per model]

Across the full existing-reader census there are **4,800 strip geometries**,
**1,005,230 reported strip primitives**, and **431,856 nondegenerate triangles**.
These numbers include helper and shadow geometry; they are not final submitted
draw totals. [native-model-node-inventory.json §summary]

**Observed:** the converter expands `meshdata.get_triangles()` into flat indices;
the existing PyFFI helper drops repeated-index triangles while maintaining strip
parity. Rust validates only the resulting triangle list. Therefore the above
count reduction is expected conversion behavior, not evidence of lost visible
faces. Original strip boundaries and primitive ordinals are absent from the
intermediate. The current Rust model fixture is one synthetic triangle and does
not prove original strip expansion. [CONVERTER-1:207–214; PYFFI-1, PYFFI-2;
RUST-3:34–62; RUST-6:43–53,114–131]

**Native validation required:** capture original AA strip-index queries or draw
submission for these source-bound cases. Cover multiple strips, parity across
degenerate connectors, face winding, native primitive ordinals and malformed
length/index failures. Compare visible triangle sequences separately from native
primitive counts. This review did not find an incorrect current expansion.

## 5. Static AABBs, authored render spheres and collision volumes are different

**Observed:** the intermediate's `Bounds` and prepared model bound are min/max
AABBs of converted vertex positions. The client already receives a source-bound
authored sphere **center** as `native_sort_center`; do not describe that existing
work as absent. It does not preserve a full model/world sphere or parent subtree
bound in `aa-assets`. [RUST-3:157–168; RUST-4:145–174,210–217; CONVERTER-2:207–210;
CLIENT-1:865–904]

**SDK reference:** `NiBound` exposes a sphere center and radius; geometry keeps a
model bound, AV objects keep world bounds, and nodes have subtree-bound update
operations. The collision library separately defines sphere, box, capsule,
lozenge, union and halfspace bounding-volume variants. Recomputing one static
AABB does not reproduce those representations or their merge/update timing.
The NiBound prose is more abstract than its concrete evaluation header, so the
header declaration and actual AA routines must settle representation questions.
[SDK-H7; SDK-H8:72–99; SDK-H2:87–88,229–245; SDK-H4:62–69,119–120;
SDK-D2 §§Update Functions, Bounding Volume Functions; SDK-D7]

**Native validation required:** identify AA's model-bound streaming, sphere
world update, node merge/rigid update and camera-plane culling routines. Exercise
source sphere centers/radii, nested scale/rotation, zero radius and boundary
contacts. Keep collision/picking behavior grounded in AA's own paths; neither
the current AABB nor the SDK's generic collision types prove gameplay collision
rules.

## 6. Transform conventions need source-bound numerical proof

**SDK reference:** a dynamic transform consists of an orthonormal rotation,
translation and uniform scalar scale, composed through the parent hierarchy.
The docs use column-vector notation. This conceptual model is compatible with
static hierarchy baking, but does not prove any specific PyFFI row-matrix
interpretation or AA floating-point order. The API's general `ApplyTransform`
can push nonuniform scale/shear into data rather than store it as the ordinary
dynamic transform. [SDK-H3; SDK-D1; SDK-D10; SDK-D2 §Transformation Functions]

**Observed:** current conversion bakes transform chains, converts native
`(x,y,z)` to render `(x,z,-y)`, rounds to six decimal places and drops the
original transforms. Normals follow an extracted rotation and are normalized
again by Rust. This cannot be claimed bit-identical to the original transform
execution, even when the visible pose looks correct. The existing native actor
placement-facing fixture proves a narrower placement path, not all NIF parent
composition. No transform-order defect was established in this review.
[CONVERTER-1:61–83,169–200; RUST-3:74–108; RUST-5:19–24; NATIVE-FACING]

**Native validation required:** include unequal parent/child rotations,
translation plus scale, very small/large values, multiple nesting levels and
the real source scales from the metadata sample. Record original binary32
components before JSON rounding; compare positions, normal direction and bounds
against the AA functions under its actual thread precision modes. Retain source
node IDs/transforms for animation, billboards, culling and attachments rather
than trying to recover them from flattened name paths.

## Reproduction and limits

Run from `rust-rebuild`, with Python's standard library and the already-vendored
conversion reader; no installation or build is required:

```powershell
python research/gamebryo-reference-review-20261016/models/scan_native_model_headers.py ../reconstruction/assets/original/Data research/gamebryo-reference-review-20261016/models/native-model-header-inventory.json
python research/gamebryo-reference-review-20261016/models/inspect_native_model_sample.py ../reconstruction research/gamebryo-reference-review-20261016/models/native-model-metadata-sample.json
python research/gamebryo-reference-review-20261016/models/scan_native_model_nodes.py ../reconstruction research/gamebryo-reference-review-20261016/models/native-model-header-inventory.json research/gamebryo-reference-review-20261016/models/native-model-node-inventory.json
```

The node census preserves source membership and hashes from the header census.
Metadata records are original observations, not distributed engine content.
No original executable oracle was executed and no rendering comparison was made.
No Rust test or build was needed for these research-only files. SDK 1.2/2.6
implementations were not used in this review; they remain later-version
hypotheses, not evidence for AA behavior. All native validation above remains
open until supported by original executable evidence under NATIVE-PORT-RULES.
