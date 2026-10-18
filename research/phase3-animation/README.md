# Phase 3 original animation families

This phase adds 41 models with 65 source-authored recoil clips to the isolated
asset inspector. These cover tanks and self-propelled guns from all five nations,
five halftrack artillery models, four national battleships, the high-resolution
USS Texas model and four artillery bunkers. The five earlier infantry rigs and
their 20 model-clip bindings remain byte-identical. The resulting selectable set
is 46 models / 85 model-clip bindings, using 69 distinct original KF files.

The new families contain 1,390 transform nodes, 298 retained visible meshes and
32,399 vertices. Each published model includes all resolved, selected View-bound
KF clips; unsupported curves or unresolved track targets reject its entire
resolved set. This is not a claim that every authored actor/aspect declaration
resolves. The German artillery bunker's disabled-aspect KF is missing from the
recovered original files; its available enabled recoil remains supported. That
gap and the other missing declarations are explicit in animation-catalog.json.
This is animation inspection, not authoritative battlefield animation state,
blending, root motion, firing events, turret targeting or movement-driven wheels.

## Complete source inventory and explicit gaps

`source-inventory.json` parses all 1,900 original NIFs and 581 KF files without
parse failures. It retains source hashes, controller classes, timing, flags,
key-type summaries, sequence targets/events and raw TGI declarations with source
lines. There are 743 NIFs with embedded controllers and 119 building Pack/Unpack
NIFs. `animation-catalog.json` indexes every source and records 3,580 resolved
candidate KF declarations, 16 unresolved declarations, and references to 500 of
the 581 KF files in the recovered TGI projection.

These association records are explicitly qualified: the older recovered catalog
is useful evidence for candidate model/clip pairs, but is not proof of every
native template/list merge or runtime state-selection rule. Raw declarations are
retained independently. Default CharacterComponent animations do not receive an
invented aspect assignment. TGI `length`, `key_ratio` and `walk_distance` remain
separate from the original controller range; the inspector plays authored clip
seconds and does not silently apply gameplay timing.

Thirteen selected families are rejected: ten command-vehicle portrait models
with varying spline/Euler channels, the Japanese battleship with type-3 channels,
and the Japanese/Russian artillery bunkers with unresolved controller targets.
The exact source files and reasons are in `family-coverage.json`.

Pack/Unpack models are inventoried but not published as partially correct clips.
For example, USACorpsPack/UnPack each contain 44 keyframe and two alpha controllers
over 0..10 authored seconds, including varying Euler rotations. Native Hermite/
TCB interpolation, alpha/look-at/path/particle/texture-transform/flip/material
controllers and their scheduling have not been established for playback. A truck
without authored movement tracks does not receive invented wheel or motion curves.

## Source and rendering contract

New cache files retain original float values and use the existing
`aa-original-rig-v1` schema. Two backward-compatible fields bind new exports:

- `AnimatedAsset.static_model_sha256`: exact bytes of the old converted static
  JSON, separate from `source_sha256`, which identifies the original NIF.
- `RigMesh.source_mesh_index`: original flattened-converter mesh index. The full
  ordered list equals the native-audited retained indices in model-materials.json.

Repeated node labels are valid in original vehicle hierarchies; tracks use
explicit node indices. Export rejects a KF string target when the source name is
ambiguous or absent. Rust still validates parent-before-child order, node bounds,
matching track labels and unique track/mesh identities. No global name-based
retargeting or basename model matching is performed.

The material audit controls helper removal and team-color flags. Naively
propagating every ancestor Sg flag was rejected because native context behavior
retains one such RUSTankDestroyer mesh. The final output follows the audited
per-mesh decision and strict original name/index/vertex-count checks.

Three original untextured meshes have no UV array: RUSTankLight's Turret_sg and
BarrelSg, and USABattleship's Editable Mesh. They receive explicit zero UV
placeholders, matching static RawMesh preparation for constant-color geometry;
`untextured_uv_placeholder=true` records this. A textured mesh without UVs is
rejected. No texture mapping is invented. All 43 referenced PNG resources exist.

Local coordinates, hierarchy/skinning math and browser-oriented intermediate UVs
retain the Phase 2 contract. The WGPU consumer reverses the intermediate V flip
at its vertex boundary. Original NIFs, textures and the earlier converted JSONs
remain unchanged. These artwork declarations do not replace the modified v14
gameplay rules used as the rebuild's behavioral baseline.

## Validation and publication

Six Rust tests pass, including 105 unchanged infantry reference poses and 366
new-family poses. Every new clip is checked at five authored times plus each
model's bind pose. Maximum new position error against the separate Python/PyFFI
reference is 9.536743e-7; maximum normal error is 1.1920929e-7. All 65 new clips
produce visible vertex displacement in those samples. This is differential
asset-math validation, not executed-native Gamebryo interpolation parity.

Three Python tests reject varying quadratic values, nonzero tangents and type-3
reduction, while checking preserved constant-channel metadata. The final byte
audit verifies 1,390 source/hash/topology/resource conditions and the unchanged
Phase 2 cache. The family test also checks exact position and normal lengths.
Independent review and client GPU acceptance are separate evidence.

`assets-cache/animation/families-manifest.json` pins the staged source export.
`promoted-families.json` records exact copied top-level files for browser discovery;
the staging directory remains provenance, not an additional selectable family.
Only explicit manifest-listed files are published, with collision/hash checks.
No old cache file is overwritten. Root owns client GPU validation and release
packaging; their results must not be inferred from this CPU/source audit.

Reproduction:

1. `python crates/aa-animation/tools/inventory_phase3.py`
2. `python crates/aa-animation/tools/export_families.py`
3. `python crates/aa-animation/tools/build_phase3_catalog.py`
4. `python crates/aa-animation/tools/test_phase3_export.py`
5. `cargo test -p aa-animation -- --include-ignored --nocapture`
6. `python crates/aa-animation/tools/validate_phase3.py --promote`

The inventory and caches contain no timestamps. Do not rerun exporters against
changed material/parser sources while describing their output as the frozen set.
