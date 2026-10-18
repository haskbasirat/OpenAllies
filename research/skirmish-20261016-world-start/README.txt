WORLD-START LIVE STATISTICS AND SNAPSHOTS -- 2026-10-16

Purpose
Replace the missing game-side 58F010 classification bookkeeping and 58F530
snapshot services used by world start4F1E10. This is a backend prerequisite
for skirmish; it does not itself expose a playable match or validate multiplayer.

Production code
crates/aa-economy/src/statistics_live.rs
  TrackingFacts identifies exact original object offsets. track_actor executes
  the sequential category changes, retains Actor+13C and mission score effects,
  calls the supplied Actor virtual+100 at the original points, and maintains
  peak score33. remove_actor also supplies the kingdom-unregister58EFE0 helper.
  snapshot takes the actual Statistics, resource registry, kingdom+1A0/+1B8
  arrays, signed+218 field, live relation identity and actual derived-stat rows.
  Formula terms must be in native linked-list order, not sorted/source order.
  The snapshot owns its ordinary counters but references the live relation
  statistics object by identity, matching the native pointer assignment.

crates/aa-economy/src/world_start.rs
  OwnedWorldStart and world_start_owned provide the same4F1E10 order without
  requiring mutable statistics to be borrowed simultaneously through callbacks
  and an external StartingKingdom slice. Every operation borrows the real owner
  independently. The existing standalone API and fixtures remain preserved.

Native proof
native_statistics_live.py runs original and v14 images separately, verifying
every executed instruction matches both accepted image byte streams.
  110 tracking inputs x3precisions x2images =660 executions.
  44 complete snapshot inputs x3precisions x2images =264 executions.
  Total924 executions,462 result fixtures,736 distinct pinned instructions.
  Precisions: PC24,PC53,PC64, nearest-even.
Only seam: Actor virtual+100, whose two possible sequential returns and call
order are recorded. There are no arithmetic, classification or derived-formula
seams. 58EE00,45ECA0,58E8B0,58EBA0,58E890,4CEEC0,4CEED0,58F420 and58E860
execute unchanged alongside the two entrypoints and58EFE0.
The fixture includes an instruction manifest and both image SHA256 values.
instructions.txt is a disassembly aid; the fixture instruction manifest is the
executed-byte record. The exported pseudocode's stale image metadata is not used.

Tests
native_statistics_live.rs replays all462 fixtures and additionally checks
17 resources/19 derived rows (safe storage expansion) and partial effects on a
missing required component/virtual service. The latter are explicit host-error
contracts, not a claim of matching native access-violation behavior.
native_world_start_owned.rs replays all360 existing original4F1E10 fixtures,
including their explicitly seamed tracking/snapshot observations, through the
new owned API and verifies the second call performs no further work.
The existing world-start/statistics/load regressions also pass.
clippy.log records strict aa-economy all-targets Clippy results.
tests.log records the full aa-economy test run for this source checkpoint.

Removed storage limits
58F530 caps resource snapshot rows at10. Its native snapshot has10 fixed
derived-score slots and58F420 writes registry-count rows without its own check.
Rust uses growable vectors for both. Native57..61 index aliases address the
first five derived slots; at least five initialized slots remain for those
aliases even if fewer derived definitions exist. These are storage changes,
not changes to configured gameplay caps or the deterministic calculation order.

Limits / integration requirements
The runtime must bind actual Actor/Kingdom ownership, virtual+100, live
statistics and correctly sourced StatData rows. An absent formula catalog must
not be turned into an empty successful catalog. Source bindings are documented
in this directory separately. Neither function draws RNG. All arithmetic uses
the existing finite-input aa-economy precision contract; NaN/Inf arithmetic is
not claimed. Raw copied snapshot fields may be preserved until arithmetic uses
them. Original serialized snapshot wire support, native allocation failures,
complete world publication, battle ticks and player-facing battles remain
separate work. No new release or original executable was changed here.

Backups
backups/world-start-statistics-before-20261016 contains priorlib.rs/world_start.rs.
