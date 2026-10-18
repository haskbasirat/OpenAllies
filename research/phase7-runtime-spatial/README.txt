Phase7 concrete ActorStore spatial ownership and source shape adapter

New aa-runtime/src/spatial_world.rs composes ActorStore with the real
aa_spatial::SpatialIndex. It binds ordinary runtime class3/ActorData definitions
through ActorCatalog.native_object_definition and the shared ActorDefinition
token bank. It verifies actor/source provenance and rejects derived classes,
conflicting definition tokens, stale GTIDs, missing required Wall construction
and missing/uninitialized placement cache data. Shapes use source-native words;
renderer scale/bounds are never involved.

Publication is deliberately two ordered operations. publish_actor prepares
immutable source geometry then ActorStore.publish_placement transfers flags,
XYZ, facing and cached tile to SpatialIndex.register. Register validates before
accepting ownership. No recoverable operation after register may return staged
ownership. refresh_membership subsequently performs actual4EE4C0, with the
ordinary class3 virtual+B0=true proved by the original vtable. If this later
operation fails, ActorStore already reads the authoritative index and cannot
fall back to a stale staged pose. A source member claiming existing bit26 tile
membership before actual insertion is rejected, not silently corrected.

ActorStore ObjectKey remains64-bit; an explicit separate spatial token maps
objects/GTIDs to the existing index API without casts between identity domains.
Geometry is immutable source definition state. Transform/flags/cached_tile have
one mutable owner. In coordination with actor_creation, PlacementState now moves
Option cached_tile from ObjectFields44/48 and no longer leaves a second cache
in RuntimeObject.object. Non-Object GThing has None, never invented zero cells.

SoftView implements the real soft_range World using ActorStore::placement and
retained source definition/shape handles. Ordinary virtual+10 is exactly
definition+50; target override is irrelevant there. The index's target descriptor
separately honors the original override+68 or fallback+50. Source-first capture
and late shape access remain in the independently tested geometry wrapper.
ActorReader supports both full ActorStore and its disjoint ActorAccess borrow.
TargetView separately implements actual4EE800/4EE7C0 target-shape access through
live generation-bearing ActorIds; it is not used for Mover5774A0 soft shapes.
split_index returns mutable index and immutable SpatialBindings references to
the disjoint identity/source maps, allowing collision policies to inspect source
definitions without cloning mutable state or borrowing the entire world.

commit_position/set_facing/move_ground delegate to original-sequence index APIs
with mandatory TransformCallbacks. No default/successful empty callback exists.
Actual Actor+ C8 component/attachment/history/presentation effects and collision
policies still need the operational world adapter; this file does not certify
those services or a complete match.

Validation:20 original instruction executions confirm the actual Actor vtable
blocking/soft/target getters and+B0/+B4 booleans under four override bytes. The
load constructor vtable assignment and all relevant ranges match original/v14.
Two source-backed Rust tests construct12 actors from the five faction infantry
definitions and one regiment definition, publish membership, compare soft shapes,
and prove changed renderer scale has no geometry effect. A deliberately failing
position callback demonstrates real committed pose/membership remains visible
through ActorStore and later range checks. Rejection tests preserve staged
ownership; repeated publication and retired GTIDs fail explicitly. These tests
also exercise ActorAccess soft/target views and the split identity/source map.
These tests
exercise geometry integration, not completed actor InitCreate/AI/combat.

Commands:
python research/phase7-runtime-spatial/native_bindings.py
AA_GAME_ROOT=<original game folder> cargo test -p aa-runtime --test spatial_world -- --include-ignored
cargo clippy -p aa-runtime --test spatial_world --lib --no-deps -- -D warnings
AA_SPATIAL_SOURCE_OUTPUT can direct the actual source test's JSON report to a
separate evidence file. The pinned report here is original-actor-bindings.json.
