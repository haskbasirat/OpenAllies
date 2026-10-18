Actor registration: actual zone-grid half
=========================================

NEW aa-runtime::load_zone_entry::grid_register binds4CFEA0 through native
554CC0/554330 and54EF10. The4D62F0 state table/reentry and full registration
coordinator belong to the separate registration author. This module does not
publish fake zones or certify a complete Actor Register/battle.

554CC0 is not interchangeable with Object footprint554DD0(mode1). It calls
4EE700 on the actual flags and ordinary source radius: flags20000400 and
radius*4>=ftol(cellsize) choose5534F0 shape bounds. Otherwise it starts from
the actual cached Object+44/+48, clamped by552550. Its center cell comes from
the current descriptor position when in map extents, otherwise rectangle min.
The mode1 point overlap selects other cells. Consequently mismatched cached
tile/pose may produce NO cells, or an out-of-map pose retains its edge center;
neither is replaced by a more convenient occupancy list.

The implementation reuses existing native-proved marks_blocker, tile_rectangle
and shape_overlap arithmetic, with explicit precision and the same immutable
descriptor snapshot. Each selected cell dispatches before computing/visiting
the next. A callback error retains its exact prior prefix; there is no eager
list of future callback results.

The public adapter validates the private source against the held Record and a
fresh canonical source binding in its actual Catalog/Bank source precision.
Actual Actor/Kingdom constructor storage and persistent GTID generation mapping
are required. It reads cached coordinates/pose/flags from the held shell, never
from an unbound index Member or renderer. The same persistent ZoneMap and
LoadSpatial geometry/PC are checked. Caller must retain the same actual map;
dimensions alone cannot authenticate an unrelated owner.
The reached point read also calls PointShape::validate_catalog, rejecting a
foreign or subsequently edited destination source/Settings owner.

54EF10 reads actual SessionPhase::application_state at EVERY selected cell.
Only when E4==3 does it read the supplied actual saved WorldState.word_48; an
absent reached word is an error. E4 is distinct from Session+F8 and MapResource
mode. Current tile sets and their native-order entries are read when reached.
Entry.actor uses ActorId.raw(), not SpatialIndex's separate internal key domain.

Concrete515620 no-effect prefixes are supported: flags0/1/2/64, combinations
without effect bits, and type2 when the actual Actor has no Detection component.
Guard/control/other component zone-slot getters and positive marker/discovery
owners are explicit unsupported children at their native positions. No unknown
query becomes an empty candidate set or an implicit success. Positive child
effects, including possible zone mutation, remain outside this bounded proof.

Proof
-----
native_grid_registration.py executes actual4CFEA0,4CF210, ordinary runtime
getters,4EE700,554CC0,5534F0/552550,5541F0,554330,54EF10 and actual515620 leaves
in separate original/v14 machines at PC24/53/64.300 fixture rows use600 fresh
machines,932 executed image-address pins and NO intercepted bodies. Inputs
include point/circle/rotated rectangle, multi-tile flag gates, stale cached
coordinates, outside positions, both E4/World48 gates and five leaf flags.
The global point words are an explicit fixture input; actual startup provenance
is supplied by the separately verified PointShape owner, not inferred here.

The Rust replay compares every selected-cell order, enabled leaf-call order and
retained flags, and tests a first-callback refusal prefix. Native instruction
windows are pinned separately in native-instructions.log. This is not a native
exception/unwind proof. Whole shared-world/registration acceptance belongs to
the caller's integration test with the actual persistent world.

Reproduce
---------
 python research/skirmish-20261016-load-zone-entry/native_grid_registration.py
 $env:CARGO_TARGET_DIR='target/s-graphics-reference'
 cargo test -p aa-runtime --lib load_zone_entry --release --locked --offline -j4 -- --nocapture

Only new source/test files and one module export were added for this slice.
No Cargo/lock, original native binary/assets/save/profile/release was changed.
Final pins/results are in findings.json; independent review is separate.
