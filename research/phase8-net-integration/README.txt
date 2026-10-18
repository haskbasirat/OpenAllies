PHASE 8: NATIVE LAUNCH-SETTINGS / STAGING INTEGRATION
2026-10-16

Scope
-----
The existing network layer had native-proven ChangeToState (0x77A750),
but its launch-settings assignment (0x779F00) remained an external callback.
This work executes that assignment and its subordinate value copies in the
original code and supplies a production settings store to the staging port.
No rejected aa-game/aa-net code was used. No client, Cargo, wire codec,
original game file, profile, save or release package was changed.

Code
----
crates/aa-netcode/src/launch_assignment.rs:
  assign / CompleteSettings::assign_from: 0x779F00 value assignment.
  Retained / KingdomRetained: fields copied natively but absent from the
  existing network wire projection. Callers provide them explicitly; there
  is deliberately no Default implementation.
  SettingsStore::copy: the four assignments called by 0x77A750 / 0x77A6E0:
  Launch <- Launch88; Launch88 <- Launch; last single/multi <- Launch.
  Session remains the single owner of its live wire projections.

crates/aa-netcode/src/lobby_staging.rs:
  Staging implements staging::StagingHost, using the real SettingsStore for
  copy_launch_settings instead of asking the client to invent a copy.
  Snapshot implements lobby::SnapshotHost, calling the existing native
  ChangeToState port and this concrete store. Map readiness/checksum I/O is
  required through MapStatus; it cannot silently report success. UI event
  publication, countdown, world loading and timeline effects remain explicit.

Copy-only fields recovered
--------------------------
LaunchSettings +0x10 CString; GameSettings +0x14 word; random map +0x74
CString; map24 +0x50 word; metagame map +0x50 byte/+0x54/+0x58 words;
each KingdomSetup +0x0C CString and +0x68 byte. Launch+0x30 and LaunchState
+0x0C were already represented and are checked as well. CString values are
UTF-16 code units, retaining embedded NUL and unpaired surrogates.

Native functions executed
-------------------------
0x779F00 launch assignment; 0x5A46D0 game settings assignment; 0x5145E0
map settings assignment; 0x514240 named triples; 0x514430 kingdom vector;
0x4A4C00 kingdom entry; 0x41D130 rule vector; 0x5A12D0 game entry vector;
0x513690 random-map named-word vectors; 0x4145C0 launch-state assignment;
their actual native vector growth/element copy descendants; 0x77A750 and
0x77A6E0 session state transitions. Exact executed instruction windows and
hashes are recorded in both fixtures. No copy function is an oracle seam.

Evidence
--------
native_launch_assignment.py -> native-launch-assignment.json:
  32 complete value-state cases on EACH of original and v14 images (64
  executions); results agree. Nested empty/growing/shrinking lists, ordinary
  copies and 4 self-assignments per image, non-wire fields and UTF-16 cases.
  Return destination pointer and unchanged source checked. Both before and
  after values are captured from native memory, not calculated by Rust.

native_staging_assignment.py -> native-staging-assignment.json:
  48 native ChangeToState executions with 0x779F00 unhooked, covering four
  old states, three destination states, host/client/migrated/save-disabled
  branches. Complete settings slots, session flags and published event
  values are captured. Every executed original instruction window is
  byte-identical in v14; these 48 cases run on the original image.

Both harnesses reuse the phase8-net allocator, CString-manager and logging
seams. The staging harness intercepts only the external state event post in
the exercised transition path. Page-zero accesses fault unless they are the
native FS:[0] exception-chain access. No floating arithmetic or RNG is used
by the copied-value logic. Native allocation capacities/refcounts are memory
management details and are not reproduced as limits in growable Rust data.
Allocation-failure and exception-state equivalence are NOT established.

Tests
-----
tests/launch_assignment.rs replays all 32 native value outcomes and rejects
incomplete companion state without silently supplying missing records.
The original self-assignment executions alias their pointers; the safe Rust
API forbids simultaneous mutable/immutable aliasing, so their replay uses
distinct equal-valued inputs and establishes value idempotence only.
tests/lobby_staging.rs exercises the production Snapshot -> Staging ->
SettingsStore chain over all 48 native outcomes/events. It also checks that
a failed event publication retains only the already-committed transition
prefix and that unavailable map-status I/O propagates an error.
Validation commands and final results are in findings.json and validation-*.log.

Small existing-code cleanup (separate from assignment behavior): strict Clippy
found a collapsible nested condition at heartbeat.rs:1245. With lead approval,
it was collapsed into a Rust let-chain, preserving left-to-right short-circuit
evaluation and the GameSpyPlayerLeft callback. The native heartbeat/ClientLeave
tests were rerun after this edit. No other heartbeat behavior was changed.

Full snapshot companion binding (follow-up 2026-10-16)
----------------------------------------------------
lobby_staging::SnapshotOwner now owns the only mutable Session/settings pair
on this path. read_snapshot runs the unchanged wire decoder through real
ChangeToState and assignment, then reconstructs decoded kingdom companions.
The actual 0x5104A0 uses one 0x50F4C0(1) temporary and 0x4A4C00 copies it into
each decoded record: +0x0C is the static empty CString, and +0x68 is zero.
The old destination and the sender's private values must both be discarded,
even when record count is unchanged or names are replaced/reordered.
Other retained fields remain untouched by wire decoding (after any initial
ChangeToState restore). The complete final state is compared to original code.

native_snapshot_companions.py -> native-snapshot-companions.json executes
0x76D0B0 writer + 0x77AAC0 reader, real state/copy/header/kingdom routines,
on 12 cases for EACH original and v14 image. Both images agree; separate
0x50F4C0 constructor probes agree. Cases cover equal-sized replacements and
reordering, emptying, growth, and entry from states 0/1/2. No decoder,
settings-copy, header or kingdom routine is mocked in the oracle. Native
allocation/CString/logging, external definition lookups, map status and event
publication retain the explicitly documented harness seams.

On a truncated read, map-status error or failed staging callback, settings
ownership is invalidated. Partial Session/Admin/cursor writes remain visible;
the operation cannot be retried or copied as though it completed. Invalidation
is a safe Rust failure contract, NOT a claim that native malformed-stream
exception/partial-store behavior is identical. An invalidated owner requires
the session layer to discard/reload it from complete source state.
tests/snapshot_companions.rs compares complete session/companion/admin values,
cursor position, actual callback order and subsequent state-copy values. It
also checks truncated inputs and map-check failure cannot expose stale state.

SnapshotOwner::new requires complete native-constructed settings. It validates
lengths, not historical identity; callers must not seed it with already-stale
companions. The raw Snapshot trait adapter remains low-level: raw decoder
callers should migrate to SnapshotOwner::read_snapshot. No existing UDP
loopback mock was replaced with another mock to conceal outstanding services.

Follow-up backup (before source edits):
backups/native-snapshot-companions-before-4c4cad7c2422465da74deb4f14ac4ccd.zip
Release Development08 and earlier packages remain immutable.

Remaining integration boundary
------------------------------

Still required for a usable session: canonical initial settings construction,
binding SnapshotOwner into the real private-message session dispatcher,
real map-status/checksum requests, kingdom setters, client/session
ownership, world restoration/publication, actual decree game effects,
world loading, UI publication and client Simulation::step integration.
This work does NOT make matches, internet multiplayer or the client lobby
playable. No second-machine or separate-internet test was performed.

Reproduce (PowerShell, rust-rebuild directory)
-------------------------------------------
$env:PYTHONPATH='E:\Games\Axis and Alies\reconstruction\tools\python-packages'
python research/phase8-net-integration/native_launch_assignment.py
python research/phase8-net-integration/native_staging_assignment.py
$env:CARGO_TARGET_DIR='target/s-orders-adapter'
cargo test -p aa-netcode --release --locked --offline -j 4 --test launch_assignment --test lobby_staging --test staging --test join_resume --test send_selector --test heartbeat --test loopback
