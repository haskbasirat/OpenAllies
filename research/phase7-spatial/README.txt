Phase7: original live spatial membership and ordinary actor transforms

Implemented APIs
- aa_spatial::geometry::Placement uses original gameplay shape words, facing
  and position. world_bounds/tile_rectangle port46E540/5534F0/552550, including
  cardinal shortcuts, exact x87 store points, upper-edge epsilon and clamping.
- aa_spatial::index::SpatialIndex owns generation-checked actor references,
  head/tail tile lists, actor transforms, native blocker bits and primary Wall
  references. register does not publish membership; insert/remove/update_tiles
  execute native5536B0/553830/5539B0 ordering. Reclaiming an actor still in lists
  is rejected. Null identity/GTID slot is rejected.
  Diagnostic Event capture is off by default; set_event_capture(true) enables
  native-write traces for tests. Runtime does not accumulate an unused trace
  queue. Actual component/presentation effects use required callbacks instead.
- Member blocking/soft/target shapes must come from ObjectData definitions.
  Ordinary virtual+C returns definition+38; +10 returns+50; +14 returns+68 if
  definition+80 is set, otherwise+50. No renderer bounds or visual-scale
  multiplication is used. height_bits is position Z, not the Size.height field.
- commit_position retains the old authoritative pose through membership work,
  commits XYZ, invokes required virtual+C8, then reads presentation gates.
  set_facing writes the new facing before membership work, preserving the
  original asymmetric ordering. Native normalization4EE9F0/4453A0 is explicit
  PC24/53/64 arithmetic. Bit17 bypass and bit12 zero-vector branch are retained.
- index_world::GroundSurface combines the exact raw height samplers with the
  actual TerrainGrid. SpatialIndex::move_ground samples old XY as4EEB40 does,
  preserves snapping, and commits through the owned index.
- CollisionView supplies actual tiles, linked lists, GTID resolution, flags,
  ordinary category and shape getters to the native54F370/54F190 collision
  routines. CollisionPolicy requires external definition, organization,
  allegiance, hostile/custom-filter and outer-actor behavior. No clear-cell or
  friendly default is supplied. Mandatory callbacks may fail.

Native ordering and limits
Full insertion/removal uses X outer/Y inner loops and push-front lists. The
non-solid overlapping rectangle update uses Y outer/X inner delta traversal.
Touching inclusive rectangles still take the native full remove/insert path.
Large-solid eligibility is flags&0x20000400 ==0x20000400 and radius*4 greater
than OR EQUAL to truncated cell size. Primary tile references additionally need
the actual Actor category and instantiated Wall slot12, then exact shape/probe
overlap. A stale generation handle is not an actor pointer.

TransformCallbacks.position_changed is required actual virtual+C8 (4D0EE0).
The new position::actor_position_changed supplies its complete outer call
sequence, rather than an empty callback. It executes4EB820 flag3 clear and
possible membership refresh,4EB500 branch gates, unclamped cached-tile writes,
Organization5782D0 saved-position copies, follower5771B0 target calculation,
and position-history50A710/50A970 cursor/sample writes. Member.cached_tile is
Object+44/+48, initialized from actual create/load shell [0,0]. Footprint
expansion5528F0 is native shape arithmetic, not a renderer-scale approximation.

position::World still REQUIRES actual551140 height brush,4F5560 doodad purge,
4EA260 biome/parent variation lookup,555C10 terrain paint, virtual+124 cell/fog/
terrain/region effects, diagnostic sinks, Element organization movement,
Structure/Settlement virtual predicates, recursive4EEB40 linked actor movement
and577240 follower transform. Gates preserve those effects; no branch is
silently dropped. Root must bind real implementations or return an error.
The new aa-actors::attachment helper supplies the ordinary Structure/Settlement
predicate and is reviewed separately. Element behavior has a separate owner.
Organization+7C is not Mind. Actor+144 is a position-history ring, not a generic
property controller. History admission preserves signed wrapping frame adds,
reset-latch transitions, exact raw position/time samples and modulo cursors.
Source operations that need an unsupported service stop with earlier effects
intact; the callback implementation is not yet a complete live-world backend.
Presentation sinks are43B760/43B7D0 after native50C250/50C270 gating.

These APIs support ordinary immutable definition shape getters. Arbitrary
virtual getter mutation, live definition replacement and derived shape classes
are separate. Callbacks must not reclaim/reuse the active actor or traversed
list node. Completed effects survive errors; retrying blindly is unsafe. The
finite arithmetic API reports the native2D zero-facing NaN domain after the
initial facing stores; it does not manufacture a usable direction. Object
scale, rendering animation bounds, complete collision descriptors, full
component initialization and playable-world startup are not claimed here.
Opaque Rust node handles need not equal original heap addresses. No synthetic
bandwidth/SSD/memory quota was added; source identity representations and real
allocation limits still exist.

Evidence
original_geometry.py:792 original x86 geometry cases, no decision hooks.
original_index.py:48 original sequences/432 steps. Only malloc/free intercepted.
All list order, head/tail, tile flags, primary references and actor flag writes
are compared. Prepared index-update cases explicitly apply caller pose state
after5539B0; they do not execute the actor position callback.
original_transforms.py:96 sequences/288 steps execute4EE5C0/4EEBE0 and actual
membership/normalization bodies. Required virtual+C8 and presentation sinks
are intercepted with captured pose and explicit phase mutation, so these
fixtures prove outer order/gates, not the internals of those callbacks.
original_position.py:192 sequences/576 steps execute4D0EE0/4EB820/4EB500,
4EE930/4EE4C0 membership branches,5528F0 footprint expansion,5782D0 savedXY,
5771B0 offset arithmetic and50A710/50A970 history. It covers indexed/nonindexed,
height/doodad/paint gates, attachment/follower branches, repeated notifications,
history delay/reset/wrap and signed frame rollover in all three PC modes.
The fixture explicitly intercepts the required world/component services above;
native-position.json enumerates every seam. A separate error test proves cached
cells/membership survive a required124 failure and later actor work is skipped.
Executed-instruction windows are original/v14 byte-identical; seam entries
are excluded before recording. All three x87 precision modes are exercised.

Rust native tests compare every fixture state and ordered effect. A separate
failure test proves committed pose/membership survive component failure while
later presentation is skipped. live_maps.rs loads all80 original RMPs, native
raw terrain and both height chunks; it derives seven original/v14 ground-unit
shapes and flags through ActorCatalog, then checks560 prepared move/rotate/
remove sequences and all cell memberships. Its component callback is explicitly
a test boundary, and its prepared actors/poses are not saved actors or startup.
Source hashes bind original RMP/TGI metadata. Original files are read-only.

Reproduce native evidence:
 python research/phase7-spatial/original_geometry.py
 python research/phase7-spatial/original_index.py
 python research/phase7-spatial/original_transforms.py
 python research/phase7-spatial/original_position.py
Reproduce Rust:
 cargo test -p aa-spatial -- --include-ignored --nocapture
 cargo clippy -p aa-spatial --all-targets --no-deps -- -D warnings
The complete crate currently has26 tests, including previous geometry work
and range.rs owned/reviewed separately. This ledger specifically owns the new
geometry/index/index_world/position modules and seven new native/composition/
error tests.
Independent review is a separate peer gate; this document is producer evidence.

Prepared live queries (new scope)
query.rs ports552DD0/5531F0/5522D0 preparation,552250 position snapping,
553370 world bounds and553BA0/558040/558280 first-hit scanning. It uses
the live SpatialIndex lists, GTID generations, actual blocking shapes and
shape_overlap's separately reviewed46EF30 intersections. The GMap+30 cell
shape is the native full square, not the quarter-circle tile prefilter.
QueryGeometry requires actual stored GMap+C/+10 extents plus explicit
constructor precision; a grid-dimension product cannot replace those extents.

PreparedQuery::for_actor(index,key,true,0x400,policy) prepares the same probe
used by57C080. set_position then first_hit(...).blocked() runs each candidate.
The selected actor must exist:5531F0 immediately dereferences actor+4. A missing
selected actor is an error, not permission to substitute an empty descriptor.
Queries preserve Y-major/X-minor scan order, temporary mask400 removal,
source/custom filter order, first valid reference selection, resnapping when
copying the descriptor and intrusive visited marks. Nested queries see outer
marks. Normal/error exits clear only the query's own marks. An actor marked by
an active query cannot be reclaimed; callbacks retain the existing list/node
lifetime restrictions. Policy errors preserve earlier world effects.

QueryPolicy extends the required CollisionPolicy with the actual definition
virtual+30 ObjectData-view result. Source virtual+30 actor exclusion4D5600,
custom predicates, allegiance/hostility, moving state and ownership remain
required services. This port does not guess their answers. Ordinary immutable
shape getters and finite kinds0..3 remain the supported domain. POINT/POINT
overlap uses the separately documented PE zero-origin assumption.

original_query.py executes384 original sequences and2304 candidate queries
across all three x87 precisions. It compares descriptor fields, exact results,
policy trace and final intrusive marks. Original geometry and iterator bodies
execute unchanged; allocation/free, source/custom policy and definition-view
results are explicit fixture seams. The malformed-policy Rust test checks
that error cleanup preserves marks held by an outer scan. Existing all80-map
tests validate membership/height composition; these new query cases are
prepared native fixtures, not full restored maps or a playable-world claim.

 python research/phase7-spatial/original_query.py
Final combined26-test and strict Clippy logs are query-integration-tests.log,
query-integration-clippy.log and query-validation.json. Query evidence remains
producer evidence; independent-query-review.json is a separate peer gate.

Terrain name correction
SerializedTile.explored_mask and TerrainCell.saved_explored_mask now name the
saved low16 tile+14 field correctly. It is unrelated to tile+10 topology
rectangle pointers. Existing serialized debug JSON accepts the old key as an
alias. Raw bytes and binding arithmetic are unchanged. The native mask setter
proof and exact naming-only file changes are in terrain-mask-rename.json.

Query repair after sweep call-site review: source definition bit10 clear sets
large-only at553BE2..BF8. Actual4EE700 now filters actors before intrusive
marking, matching5583F1..558407. Expanded fixtures cover both flag branches.
