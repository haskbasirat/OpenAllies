# Original animation recovery: first supported subset

This phase adds authored character animation to the isolated model inspector.
It does not animate battlefield actors according to invented combat or movement
rules. Root motion, game events and animation-state selection remain separate
reconstruction work.

The original data contains 581 `.kf` files, each parsed as NIF controller data by
the existing recovery tooling. Earlier conversions include rounded key JSON and
five 15-fps morph previews. Those previews approximated spline channels and are
**not** inputs to this implementation. New cache files come directly from the
original NIF/KF files and retain their read float32 values without six-decimal
rounding.

Supported assets:

| Model | Rig nodes | Visible meshes | Vertices | Original clips |
|---|---:|---:|---:|---|
| USAInfantry | 29 | 2 | 230 | Idle, Jog, Attack1, Die1 |
| GERInfantry | 29 | 2 | 259 | Same four shared infantry clips |
| GBRInfantry | 29 | 2 | 259 | Same four shared infantry clips |
| JPNInfantry | 29 | 2 | 259 | Same four shared infantry clips |
| RUSInfantry | 29 | 2 | 259 | Same four shared infantry clips |

Each clip is one authored second and has 23 `NiKeyframeController` tracks.
The deforming body uses 15 skin bones. The second mesh is the attached rifle.
The exporter preserves hierarchy, original mesh vertex order, triangle indices,
texture coordinates, materials, inverse-bind transforms, skin offsets, sparse
vertex weights and source hashes. Text events are retained as data only.

All moving translation, scale and quaternion channels in this subset have
linear key type 1. Playback uses linear vectors/scalars and shortest-arc
quaternion SLERP. The rifle uses constant Euler type-2 keys with zero tangents;
the exporter verifies their orientation against the actual bind matrix within
1e-6 and retains that matrix. Nonconstant Euler/spline channels, nondefault
frequency/phase and unresolved node links fail explicitly rather than being
silently approximated. The cache records a null sequence cycle type for these
10.1.0.0 files: that field is not serialized until version 10.1.0.106. Controller
flags 40 and 44 are preserved, but the inspector's Loop/Clamp choice is explicit.

One original `Attack1` left-calf final-key quaternion has squared length differing
from one by about 0.0007256. Clamped interval values, including that final key,
are retained rather than silently renormalized. Exact first/interior key times
still enter interpolation and normalization. This behavior is
source/reference grounded, not yet a proven match to the original Gamebryo
interpolator's treatment of that authored irregularity.

## Coordinate and skinning contract

Cache local coordinates remain native X/Y ground, Z up. NIF uses row-vector
matrices; their rows are supplied as columns to Rust's column-vector operations.
Parent world transforms multiply local transforms. For each weighted vertex,
the effective column-vector transform is:

`geometry_world * skin_offset * inverse(skeleton_world) * bone_world * inverse_bind`

Normals use the corresponding rotation after removing uniform scale, then
normalize after weighted accumulation. The scale decomposition follows the
reference determinant cube root. Final positions and normals are converted once
to `(native_x, native_z, -native_y)`. UV V is flipped once during recovery to
match the existing browser-oriented intermediate format. WGPU uploads PNG rows
without the browser's `flipY`, so the WGPU model-vertex boundary must recover
native V with `1 - cached_v`; terrain/generated UVs use a separate convention.
No original or existing converted asset is changed.

## API and validation

`aa_animation::AnimatedAsset::load(path)` validates the cache. `sample_bind()`
returns an authored pose. `sample(clip_index, seconds, PlaybackMode::Clamp/Loop)`
returns `SampledPose { time_seconds, meshes, bounds }`; every `PosedMesh` contains
render-space positions/normals corresponding to the static `RigMesh` UVs,
indices and material. Texture URLs use `AssetStore::resolve_url` as before.

Cache: `rust-rebuild/assets-cache/animation/<nation>Infantry.rig.json`.
Reproducible exporter: `crates/aa-animation/tools/export_original.py`.
The manifest pins original NIFs, KF hashes inside each clip, exporter, parser,
schema, material resolver and reference helper source. No timestamp is embedded.

Four Rust tests compare all five bind poses and four clips at five times each:
105 poses total. Positions and normals are compared to a separate Python/PyFFI
hierarchy and skinning implementation with a 1e-5 tolerance. Additional checks
prove visible authored vertex movement, stable topology, explicit loop/clamp
behavior, and rejection of malformed hierarchy/weights/nonlinear channels.

The reference tests do not execute the native game's Gamebryo animation code.
They prove consistency with parsed source data and an independent skinning
implementation, not full animation-engine parity. No animation blending,
visibility/material animation, animated LOD switching, gameplay events, network
synchronization or native root-motion behavior is claimed. GPU integration and
visual inspection are separate checks owned by the client.
