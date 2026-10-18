# Native navigation operations, Phase 5

This phase ports owned paths and the complete prepared-node ActionGo operation.
It does not implement path production, formations, collision solvers, or the
outer UAI state machine. No command-5-to-Mover shortcut is introduced. Original
AA.exe and the accepted v14 executable remain read-only; all recorded instruction
windows are identical in both.

## Ownership and operations

UAI owns its MoveState at +0x7c. MoveState embeds a path collection at +0x58:
first/last links at collection+4/+8, signed type-1/type-2 counters at +0xc/+0x10,
dirty byte at +0x14 and retained word at +0x18. UAI+0xec, MoveState+0x70 and
path+0x18 are the same field; the path-event write of -1 is not a separate state.

Nodes contain eight u32 payload words, previous link +0x20 and next link +0x24.
ActionGo interprets words 0/1 as waypoint XY, 2/3 as direction XY, and word 4 as
node kind. Words 5..7 stay opaque and bit-exact, including NaN-looking patterns.

- `0x52d000` allocates/copies all eight words and assigns both links.
- `0x52d0f0` no-ops on identical source/destination; otherwise releases the old
  chain in forward order, copies source order into independently owned nodes,
  copies metadata and marks dirty. Rust's exclusive borrow excludes self-alias.
- `0x5189f0` clears only links/nodes. It does not reset counters or dirty state.
- `0x52ce00` repeatedly removes the tail while signed type-2 count is positive.
  Removed type-1/type-2 nodes decrement the corresponding counter; other kinds
  still disappear. This is not simply removal of a trailing run of type 2.

Rust owns nodes in a checked/fallibly allocated Vec; native pool addresses and
allocator timing are not reproduced. Before assignment allocation succeeds the
old path is retained, which is deliberate safe behavior where native allocation
failure would dereference null. Corrupt positive counts that outlive the chain
return an error; earlier removals are not rolled back.

## Activation and path events

`0x52df60` invokes the virtual eligibility callback only for activation modes
other than 0/1. After acceptance it clears target+0x10, writes destination+0x14,
auxiliary direction+0x1c, requested speed+0x24, mode+0x30 and phase+0x2c=0, then
calls native state activation `0x52a810`. A false final activation result does
not undo the preceding writes.

Events `0x532170` and `0x532310` reject actor+8 bits 0 or 29. Only the ordinary
event tests the separate UAI+0x48 active state via `0x52a000`. Both use the last
source waypoint, activation mode 2 and zero auxiliary vector; movement mode is
0 for ordinary or 3 for formation. After activation they set phase=1, own-copy
the path, perform counter-driven tail pruning and set path.word18=u32::MAX.
The remaining two native event argument words are unused in non-diagnostic
behavior. Empty input after eligibility checks is an invalid native pointer
domain and returns an explicit Rust error. Native path debug logging is disabled
in the fixtures and is not part of this simulation API.

## ActionGo arithmetic and ordering

`action_go` implements the full `0x5300a0` body with required leaf interfaces.
It preserves node-kind instant turn selection, extended FSQRT arrival comparison
(<= float32 0x38d1b717), direction projection <=0 arrival, the single-node placement
predicate, phase 2→1 transition, requested-speed fallback, formation continuation,
the related-handle/group-speed check, optional organization catch-up speed,
Mover dispatch and scheduling. There are no host sqrt/trig substitutions.

The formerly provisional "zone policy" label was not established: `0x4ce6c0`
selects actor or its leader and `0x44dea0` generation-validates that object's
handle at +0x164. The referenced object's semantic role is not guessed. The
callback is therefore named `actor_or_leader_handle_valid`.

Native settings are named by their registration, not inferred from usage:
`GroupSpeedMinimum` has ID 100/cache+0x190; `MoverCatchupSpeed` ID 18/cache+0x48.
The original SVars.tgi contains values 1 and 1.75 respectively; no defaults are
hardcoded into ActionContext. Catch-up computes factor×Mover+0x28 and stores f32.

The adjusted waypoint has a non-symmetric native store boundary: directionX×
offset is stored to f32 before subtracting from waypointX; the Y product remains
extended until the final subtraction/store. The implementation keeps this.

After calling `0x5773c0`, ActionGo ignores its bool result and makes exactly one
wide RNG draw via GWorld+0xd4 (`0x4f02b0→0x45d400→0x85d840`). The signed integer
is conditionally raised by double 2^32, multiplied by the original double
reciprocal of u32::MAX, then by f32 0.3. There is no intermediate f32 random store.
The current node kind is reread after the Mover and RNG callbacks; delay is
0.3+jitter for kind0 or 1+jitter otherwise, stored f32 once before the timer call.
Rejected/arrived paths consume no draw; empty/group-stopped paths schedule 1.0.

## Required boundaries and supported domain

NavigationWorld has no permissive defaults. Native UAI activation, node-reached
state changes, single-node turning geometry, actual moving/organization/formation
predicates, related-handle resolution, stop fallback, Mover operation, shared RNG
ownership and timer scheduling remain caller implementations. Their call order
and mutable MoveState/ActorState effects are retained. Settings and precision are
stable operation inputs, not mutable global callbacks. A valid native organization
component and Mover are required for a nonempty ActionGo path.

The full solver remains `GWorld+0x238→0x53e1f0`, with replan `0x53c170` and native
collision/region/tie-break policies unrecovered. A supplied path must come from
that eventual implementation, not an invented straight line. Outer scheduler,
arrival state machine and all native factory initialization also remain separate.

Nonfinite arithmetic, overflow, absent required nodes/components, and corrupt
counts produce errors. Errors can follow callbacks/state changes/random draws;
there is no rollback or safe automatic retry. All three explicit x87 precision
modes are tested, but no native game-thread precision default is claimed.

## Evidence and tests

`native_oracle.py` executes the original whole routines, plus real eligibility
check, active-state predicate, path-copy/list operations, owner helper, FSQRT,
catch-up multiply, and extended RNG normalization. Allocation/free and unresolved
world/UAI leaves are synthetic boundaries. It verifies return stack, all x86
nonvolatile registers, source immutability, list topology and freed payloads.

255 executions cover 85 cases in each of PC24/53/64: list lifecycle and metadata,
opaque payloads, both event types and all rejection branches, activation outcomes,
arrival thresholds/projections, kind/mode/speed conditions, random endpoints,
20 deterministic fractional cases and nine callback-mutation scenarios. The
Rust suite compares complete state/actor bits, native bool results and ordered
callbacks. Three self-copy rows validate the recorded native no-op against Rust's
non-aliasing contract rather than executing an unsafe Rust alias. The two other
tests verify independent copied ownership and explicit invalid-domain errors.

`navigation-evidence.json` binds final code, data, commands and dependency
proofs. Tests validate these bounded operations, not gameplay or pathfinding.

`client-samples.json` selects six named rows from that corpus and records the
source fixture/oracle hashes. `client_samples.rs` is a fixture-only diagnostic
adapter for activation success/failure, ordinary/formation path events, catch-up
speed, and the post-RNG node-kind reread. Its required world methods replay
explicit fixture responses; they must not be installed as gameplay defaults.
