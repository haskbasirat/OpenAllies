Concrete saved Object Init composition (2026-10-16)

Owned new module: crates/aa-runtime/src/load_object_init_world.rs.
It is exported after its actual global-owner types and zone-service APIs landed.
The scoped actual release tests pass (actual-tests-final.log): one compound
test exercises the actual saved first Kingdom781 at live PC24/53/64 with the
same sourcePC53 terrain, PointShape, Session, zone/index and object-list owners.
The final JSON captures every outcome in actual-first-kingdom.json. A previous
run reached all assertions and failed only its optional relative output path;
the final rerun uses an absolute capture path and passes.

API
  initialize(&Catalog, &mut live_object_init::Record, &mut Context, Precision)
    -> actor_creation::Result<bool>
  placement_null(...) -> Result<u32>
  spatial_membership(...) -> Result<()>

Context borrows the actual BattleTerrain, MapHeader, LoadSpatial, ZoneMap,
WorldObjectList<ActorId>, NativeSourceRequirements, actual mutable local mode,
explicit global-mode binding, placement clearance owner and load_zone_services
context. It owns no second actor pose/flags/memory list. Full shell and source
identity guards stay in the reused binders.

MapHeader world extents and original Terrain resolution produce the layout;
terrain, spatial and zone dimensions/shift/cell size/extents/precision must
agree. GMap extent words use the native binary32 store of ceil(header extent),
not the fractional raw WorldCreator input (556653..68). QueryGeometry derives
from that same index. Terrain's retained source precision must equal the
immutable Bank source PC; it is independent of this live Init PC. The
immutable Source is
privately rebound through live_object_init::Source::bind. The current Record
is only reborrowed into the existing exact Object Init coordinator.

Concrete delegates
  4F4590/4F3640 -> live_placement::check_restored
  4EE4C0/5536B0 -> LoadSpatial::refresh_membership
  4F0E50 -> real BattleTerrain GroundSurface in live_object_init
  4EA150/4EBC70/4EB6E0 -> concrete load_zone_services methods
  4F1F70 -> original WorldObjectList owner in live_object_init

The reached placement callback borrows the actual current mode owner. Both
local reads and the proof-backed same-World global alias read the current word;
the global capability validates the stable Box owner's address. No captured
local scalar authorizes the late global read. The root retains Box and
capability together across Catalog/Attempt/Staged moves. This remains the
bounded one-World scope: a later native child creating/replacing GWorld must
retire/rebind that capability before its next use.

Validated scope and still required
  VerifiedStartupReceipt constructs the source-owned PointShape. The actual
  aa_netcode::Session is borrowed directly for F8; this focused test initializes
  that field from the separately proved constructor store, and does not claim
  equivalence of all unread Session fields or its complete constructor.
  First Kingdom Object base succeeds, updates native membership bit/cache,
  samples real GroundSurface height and inserts one real world-list node.
  Reached positive placement children, diagnostics and memory destruction/SAI
  operations remain explicit errors; they never become empty/clear successes.
  Team shape/vtable admission is proved separately in live-placement/team-
  admission and its actual owned Point test passes. Team's native membership
  flag gate remains the Team coordinator's responsibility; this composition
  does not invent Team Object storage or skip that gate.

Evidence dependency references
  ../skirmish-20261016-live-placement/
  ../skirmish-20261016-live-map-startup/
  ../skirmish-20261016-object-init-null/
  LoadSpatial, BattleTerrain and load_zone_services have their own proofs.

This isolated composed Object-base test is distinct from the full derived
Kingdom Init, PostCreate and registration integration performed by the root.
No playable battle or complete whole-map startup is established by this test.
