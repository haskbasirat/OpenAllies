Saved Actor state coordinator, 2026-10-16
=========================================

Purpose
-------
Bind native GActor::SetState (004D64F0) to the actual temporarily-held
ActorShell and actual mutable Thing flags during saved Organization loading.
Organization 0057CBA0 asks for state 34 (unpacked) at its primary member and
state 37 (missing_primary) after member callbacks. This is a bounded restore
adapter, not a completed live Actor state-effects implementation.

Implementation
--------------
crates/aa-runtime/src/saved_actor_state.rs calls the existing native-proved
aa_actors::state::set coordinator. It does not replace SetState with a raw bit
toggle. Actor +100/+104 are the authoritative Components.state fields; those
offsets in the separate opaque words map are deliberately ignored.

The native unchanged-state branch returns before descriptor/source lookup or
any state effect. Construction Thing flag 31 suppresses enter/leave, but does
not suppress the exclusive branch's 004D1350 action-mask rebuild.

At every reached unsupported effect boundary, the adapter first commits the
native coordinator's completed state and Thing flag stores to the borrowed
real shell and then errors by original effect address. It commits again after
coordinator return, including errors. The coordinator's scalar Fields value
cannot hide a separately-mutating live actor: an exclusive mutable shell borrow
is held throughout, and its private Effects implementation only commits and
returns an error. It calls no external/reentrant world service. Supporting
real mutable enter/leave callbacks later requires deliberately preserving that
actual-state synchronization contract; a generic copy-out/copy-back wrapper
must not silently introduce separately-mutating callbacks.

Root owns the shared ActorStateContext, RestoreDefinitions::actor_set_state,
Catalog forwarding, and mutable Thing flag plumbing through Organization load.
This task does not own or claim those integration checks.

Evidence
--------
python research/skirmish-20261016-saved-actor-state/native_saved_actor_state.py

The producer executes original AA.exe and runtime-expanded-v14 separately at
Single24, Double53, Extended64. It executes actual 004D64F0, 004D07D0,
004D0820, native bit helpers and coordinator branches. The descriptor table
and group masks are populated from the previous native 00477FC0 registration
oracle, whose complete file hash is pinned. Heap/string registration transport
is not re-executed by this probe.

4044 comparisons: 2388 native returns and 1656 stops at the first required
004D4A90 enter, 004D16B0 leave, or 004D1350 action rebuild entry. No intercepted
effect returns success. State and flags are captured exactly at the native
required-effect entry. Native instruction bytes are recorded and required to
match both executable images. Page zero faults except legitimate FS:[0].

204 rows address the Organization call states 34 and 37. Twelve of these use
the actual previously-recorded Rust tutorial02 partial restore owner 4099 and
member 4100 state/flag input (its source report hash is pinned). This is not a
claim that a complete native map restore or omitted immediate Init ran. In
that observed prefix the owner already has unpacked set, primary member has
high state bit 2 set, and missing_primary is clear: state34=true and a later
state37=false request are source-free no-ops. The producer poisons the native
catalog pointer to zero on all unchanged requests, verifying no accidental
source lookup or callback. The later state37 request is a focused input case,
not proof that all preceding load callbacks have completed.

Focused Rust replay
-------------------
cargo test -p aa-runtime --lib saved_actor_state::tests --release --locked --offline -j4

The replay compares native complete/error boundary state and flags, retains
all unrelated Actor words, and verifies no component payload is manufactured.
A second test checks portable invalid-index refusal; malformed native table or
index behavior is outside the accepted domain and has no parity claim.

Open boundaries
---------------
Live authored enter/leave still explicitly reject 004D4A90 / 004D16B0. They
need canonical ActorState properties, Organization transition, Economy refresh,
Character/view transitions, property application/removal, FX emission, and
group removal as actually reached. Exclusive state changes still explicitly
reject 004D1350 here, even though other restore adapters provide portions of
action rebuilding in their own fuller contexts. No missing service is skipped
or converted into success. Complete Init/PostCreate, live publication, battles
and multiplayer are not provided by this adapter.

No SDK implementation was copied or integrated; original AA machine code and
the existing native-proved Rust coordinator are the authorities.
