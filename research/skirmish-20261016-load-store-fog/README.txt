Mission global Fog over staged actors:551F90/551FD0
==================================================
load_store_fog::reveal_all binds the actual pre-adoption ActorStore, current
native index, same ZoneMap, retained Kingdom registry, Session phase and
source-field receipts. It does not reconstruct a RestoredRecord registry or
retain another mutable actor flag/detection-memory copy. RuntimeObject,
component and memory owners are the ones moved by ActorStore conversion.
The map's original load_zone_globals MapOwnerWitness is checked before stores.
The private root caller must supply this map's paired index; geometry checking
does not certify index identity or whole-battle readiness.

Native order:
-551F90 visits row-major tiles and calls54F400;551FD0 calls54F400 then54F4B0
 for each tile. Dimensions are reread after callbacks.
-54F400 sets tile explored low16=FFFF, then follows native actor nodes. It
 qualifies current actor flag bit20 OR(bit11&&!bit23), calls actual+B0, checks
 Object+50's generation-bearing current association, temporarily clears flag
 bit30 if required, calls+94 and restores bit30 only after successful return.
-54F4B0 sets visible high16=FFFF, then calls+98 on every node actor.
-+94/+98 are actual4EBFB0/4EBFD0 for all admitted constructor classes
 2/3/4/6/7/9. They store Object.word54 and invoke shared4EBC70 memory order.
 Source, kind, proof, current generation and stage flag owner are validated.
 Adopted placement is explicitly refused; its flags belong to SpatialWorld.

The memory adapter borrows the same ObjectFields.memory arena. The existing
refresh_inner coordinator and memory_eligibility predicate are reused.
Kingdom fields come from the actual ActorStore Special derived owner. Native
record allocation/count/time prefixes remain committed before an unavailable
positive virtual24. Container notifications, active/present observer children,
dead-object visibility and unsupported class virtuals remain required errors.
No empty result or source default replaces a missing service.

New native proof:
1,728 coordinator cases;3,456 separate original/v14 machines, all3PC. All six
actual vtable triples are pinned. Inputs cover explore eligibility, current vs
stale association, temporary bit30 on success/error, unqualified visible path,
live node-next/dimension mutation and failure prefixes.4EBC70 is explicitly
either a returning TEST seam or a required stopped entry in this fixture.
It does not certify memory implementation or a full mission. Those existing
kernel/owner proofs are reused separately.154 executed instruction addresses
plus one intercepted/stopped memory entry are pinned;16-bit visible shift is
read from the unchanged original/v14 data image (A11740=16).

Native immutable replay and the focused Rust replay passed. Current shared
memory2/2, Actor64 2/2 and predicate2/2 tests also passed after the narrow
storage-independent visibility/generic/source-definition extractions.
Actual source/ActorStore integration has a separate ignored test and artifact;
consult findings.json for its final status instead of inferring success from
the coordinator fixture.

Actual stopped-prefix capture passed56.93s; immutable replay passed61.72s:113 actual records moved into the
same ActorStore conversion. Global exploration updates741/16,384 tiles before
the real positive memory path for class9 GTID28(temperate_pine_trees). Its three
slot allocation and one unpublished record remain owned; timestamp0 is stored,
but name/position/definition/owner and the two uninitialized stat words retain
their constructor state at required4EC4D3. Source-snapshot and fog-tests.exe
preserve this exact bounded checkpoint while the next memory child is ported.

The actual-data test intentionally uses a genuine stopped immediate-load prefix
and moves its actual registry/abilities into ActorStore. It is a storage/binding
check. It cannot certify later records, scheduler/PostLoad, complete mission
startup, battle stepping, multiplayer or campaign.

Test-only profile: release/locked/offline,-j4,LTO=false,codegen-units=16. The
shipping release profile was not changed. Artifacts are frozen in findings.json.
Native replay: python research/skirmish-20261016-load-store-fog/native_store_fog.py
Rust: fog-tests.exe load_store_fog::tests::all_native_global_fog_rows_preserve_live_nodes_flags_and_failure_prefixes --exact --nocapture
Actual ignored test needs AA_GAME_ROOT. AA_CAPTURE_STORE_FOG must be unset for
immutable replay; use an absolute new output path only when generating evidence.
