Native required-space placement child4F1660 -- 2026-10-16

Added aa-spatial::required_placement and aa-runtime::live_required_placement.
The source/APIs are frozen pending review/check defects. This is not a complete
battle or a claim that the live startup caller has reached this child.

Native behavior
 4F1660 constructs a circle from ThingDataManager+74 and the requested XY,
 facing(0,-1), then calls553370. Outside returns1. It selects the optional
 ActorData required definition immediately; Settlement bit14 follows +464->+4
 before reading +4E0. Object/Actor views, parent15C and generation-checked
 Object+50 lookup precede mode-zero tile iteration554DD0/554330.

 For each tile, the live Object+50 generation lookup can suppress zone query.
 Otherwise54EF80(type2, excluded actor, excluded definition) queries the current
 ZoneMap. App mode3 on the simulation thread clears only the query's definition
 exclusion. Overlap4F0AE0 returns2. With argument7 low byte zero,54ECE0 returns
 TerrainData via tile[0] VariationData+1C: +4C bit0 clear returns3. This is the
 authored land flag written by4B87A0, not runtime tile flags20.

 Tile+8 linked objects are then read in native order. B4 dispatch happens before
 pointer/definition exclusions. Bit10, ActorData+1A8,4CF210/46EF30, parent100,
4D1120, ActorData+430, bit11, bit29 and140/global-mode tests retain original
 short-circuit order. Bit11 is captured before4D1120; bit29 is captured before
140. Blocker returns4, hostile bit29 returns5, exhaustion returns0. Duplicate
 tile membership is not globally deduplicated. Required service errors retain
 preceding callbacks and are never changed to clear placement.

Proof
 native_required_placement.py separately maps unchanged original/v14 and runs
4F1660 at PC24/53/64:177 fixture rows,354 executions,397 executed image
 instruction addresses. Six intercepted image leaf entries are stored separately.
 Native46E080,553370,4CF210,54ECE0, generation-table reads, source/tile/list reads
 and branching execute. Explicit seams are definition/actor virtual results,
 tile iterator stream, zone lookup/overlap,46EF30's result,4D1120 and thread ID.
 These seams are not proofs of those child implementations. Synthetic callbacks
 change live flags and GTIDs to verify later reads/cached fields; failures check
 reached-service prefixes. Frozen replay is immutable by default; --write is
 deliberate fixture regeneration. native-instructions.log pins bounded original/
 v14 windows for4F1660,54ECE0,54EF80 and4B87A0.

 native_shape_getters.py separately runs the actual runtime vtable+0C/+B0/+B4
 leaves for classes2/3/4/6/7/9 at all3PC in both images:108 rows,7 distinct
 executed instruction addresses, no virtual getter seams. Actual constructor
 vtable stores and slot words are pinned. All +0C slots target4EA010, returning
 [this+4]+38; all B0 are true, B4 is true for classes3/4 only. Constructor bodies
 themselves are not newly executed by this small getter proof.

Concrete runtime binding
 - Catalog's same Bank-owned combat State supplies completed_maximum_74();
   actual source identities must match and the State's completed-reference guard
   remains in force. No diagnostic/zero radius or UI source is substituted.
 - NativeSourceRequirements supplies +430/+1A8/+4E0 from its private typed owner.
   Its whole source receipt and canonical root provenance are checked. Its source
   precision must equal the actual Bank's recorded source precision, independently
   of current placement precision. Canonical component-mask and blocking-shape
   projections use that source precision. Current RuntimeDefinition mask/proof/
   class are checked against the actual source-PC projection and held shell.
   The positive Settlement+464 path remains unsupported by that bounded producer.
 - The current Record shell/IdentityRegistry remains borrowed. Generation lookup,
   current flags/Object+50, source category/provenance and source blocking shape
   are concrete. Geometry uses current shell pose, not renderer mesh bounds.
 - SpatialIndex supplies live native-order tile links and reference_for_key reads
   persistent generation mapping even between field leases. Identity lookup never
   requires an unbound member or exposes a stale copied pose. ZoneMap supplies the actual
   current sets. BattleTerrain's retained effective Variation->Terrain owner
   supplies +4C, including palette fallback, independently of tile flags20.
 - Map dimensions/shifts/extents/precision are checked across these views. The
   caller must borrow the same world's owners; identical dimensions do not by
   themselves authenticate an unrelated map. ZoneActors keys must match that
   ZoneMap, and word4 must use this Catalog Bank's definition-token domain.
 - Only supported definition/object kinds1/2/20/21 and explicitly pinned runtime
   classes2/3/4/6/7/9 are bound here. Actual LoadKind/Object/Actor storage and held
   RuntimeDefinition proof are checked before virtual dispatch. Unsupported
   dispatch, missing source fields and unavailable storage return errors.
 - Parent100/15C,4D1120 mobility,140 hostility, zone actor/ZoneActors and actual
   app/thread/global modes remain mandatory services from their actual owners.
   This adapter has no permissive callback defaults and does not certify them.

Validation
 The actual-map adapter test loads Maps/(2v2)Berlin.RMP from original/v14 data.
 At each PC it checks all50,176 exact terrain metadata cells and witnesses that
 those bits differ from flags20, generation mismatch, current-shell mutation,
 actual Object+50, grid-owner mismatch and missing-manager refusal. A diagnostic
 Bound source owner atPC53 tests source/placement precision separation, foreign
 source-PC refusal, current mask/proof tampering and incomplete-manager refusal.
 Malformed shell kind/storage and callback-substituted source class are refused.
 Persistent tile-node identity is checked with member fields intentionally unbound.
 It does NOT
 supply a fake completed manager or claim successful end-to-end placement.
 Full/focused final results and source/artifact pins are in findings.json.
 An initial own test compile typo and initial unmapped test-stub attempt are
 retained in their earlier logs; the corrected successful runs are explicit.

Reproduce from rust-rebuild (PowerShell)
 $env:CARGO_TARGET_DIR='target/s-graphics-reference'
 $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
 python research/skirmish-20261016-required-placement/native_required_placement.py
 cargo test -p aa-spatial --test required_placement_native --release --locked --offline -j4
 cargo test -p aa-runtime --lib live_required_placement --release --locked --offline -j4 -- --include-ignored --nocapture

Only the four new Rust source/test files and appended module exports were
changed for this port. No Cargo/lock, original asset, save, profile, renderer or
release was edited. Independent review is reported separately by its reviewer.
