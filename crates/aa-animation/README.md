# aa-animation

Original NIF skeleton/skin and KF keyframe playback for the asset inspector.

```rust,no_run
use aa_animation::{AnimatedAsset, PlaybackMode};
let asset = AnimatedAsset::load("assets-cache/animation/USAInfantry.rig.json")?;
let pose = asset.sample(1, 0.25, PlaybackMode::Loop)?;
// Update mesh vertex positions/normals from pose.meshes; retain asset.meshes'
// indices, UV coordinates and original material texture URLs.
# Ok::<(), aa_animation::AnimationError>(())
```

The five infantry rigs retain their four shared clips. Phase 3 adds 41 vehicle,
ship and artillery models with 65 authored recoil clips. New exports retain an
explicit original converted-mesh index and static-model hash; the client checks
these against the native-audited visible mesh inventory before using materials.
The complete selectable cache contains 46 models / 85 model-clip bindings (69
distinct source KF files).

Linear curves, quaternion SLERP and verified constant Euler bind orientation
are supported. A quadratic scalar/vector channel may be reduced only when all
values are equal and every tangent is zero; its source type/tangents are retained
in the export. Varying spline and effect-controller animation remains unsupported,
including the building pack/unpack sequences. Output is render Y-up `(x,z,-y)`;
cache skeleton data is original native Z-up. This crate does not advance the
simulation, retime clips according to gameplay, or choose animations for actors.

Run `cargo test -p aa-animation -- --include-ignored` to include all 471 original
and new reference poses. `tools/export_original.py` reproduces the five Phase 2
rigs; `tools/export_families.py` stages Phase 3 families separately. Publication
uses the byte/source audit in `tools/validate_phase3.py --promote` after reference
tests pass. Frozen cache bytes are not rewritten by tests.

The complete scope, provenance and limitations are in
`research/animation-phase2/README.md` and `research/phase3-animation/README.md`.
