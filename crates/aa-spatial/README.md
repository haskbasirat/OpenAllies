# aa-spatial

Recovered native ground-position sequencing, with explicit world interfaces.
This crate does not provide a collision solver, tile-lock owner, pathfinder or
complete map heightfield implementation.

Operations:

- `query_actor_height`: original `4F0E50` height-source selection, cell clamping,
  eight-neighbor check, deterministic kind1 offset and binary32 store points.
- `snap_half_cell`: original `46EE00` snapping, including its different X/Y
  reciprocal precision. Positive finite cell sizes and i32 floor results only.
- `commit_position`: original `4EE5C0` shape snapshots, tile-update boundary,
  position store and ordered notifications, for a distinct candidate buffer.
- `move_ground`: original `4EEB40` composition of those operations. The height
  query uses the actor's **old** XY, before candidate snapping or position commit.

`Actor` and `Cell` store float32 bits as `u32`; use `f32::to_bits/from_bits` at
integration boundaries. `Precision` is always supplied explicitly. No game-wide
x87 precision default is assumed. Native map/definition data must supply the
`TerrainGrid`; it has no invented default cells or heights.

Implement `World` for the actual game world. Its required leaves are the native
heightfield samplers, shape getters, tile update, position notification and
presentation update. Height callbacks return `aa_numeric::Binary`, retaining the
native extended value until this crate's explicit store. Actor mutations are
re-read in native order; the grid/settings snapshot stays stable for one call.

The native bit12-without-bit21 branch reads uninitialized height. `move_ground`
rejects it explicitly. Invalid numeric/grid domains also return typed errors.
An error may follow earlier state changes or callbacks: these operations do not
roll back, and callers must not automatically retry a failed operation.

`aa_movement::GroundWorld` now propagates `Result` from every required boundary,
including `move_on_ground`, and stops later scheduling after a failure. A world
adapter must propagate this crate's errors; do not `unwrap`, discard the error,
or continue scheduling as though a partially applied position update succeeded.
Mapping motion fields to this crate's actor fields must also preserve the
native offsets and callback rereads. No direct command-to-movement connection is
supplied here.

`height_layers::HeightLayers` binds actual native base/overlay samplers while
preserving extended returns. `tile_walk::TileWalk` reproduces the original
candidate-cell order, floating-point boundary decisions and bounds filtering,
with caller-owned work budgets. It supplies line traversal for collision and
navigation implementations; it does not answer whether a cell is passable.

The 504 original-x86 fixtures span PC24/53/64, query flags and boundaries,
neighbor/own-cell distinctions, extended arithmetic, callback mutations,
shape snapshots, presentation suppression and half-cell floor behavior. The
original executable, native runtimes and assets are read-only inputs.

See `research/phase5-spatial/README.md` for addresses, byte provenance and the
precise validation boundary.

Phase 8: `overlaps` ports the all-overlaps query `0x555620` (shape tile iterator
`0x554dd0`/`0x5541f0`) and the actor overlapping-thing list refresh `0x4d67f0`.
Evidence: `research/phase8-movement/README.txt` (2304 queries, 576 refreshes).
