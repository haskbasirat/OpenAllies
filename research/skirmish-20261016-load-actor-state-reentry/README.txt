Saved Actor state reentry4D4A90
==============================

The new runtime Context implements load_actor_registration::Reentry over the
same held Record, current generation registry and canonical Catalog/Bank. The
caller supplies the native indexed ActorState source, but the adapter verifies
its complete typed payload and provenance against a fresh canonical source
projection in the Bank's actual source precision. Runtime check precision is
separate. No active user profile, renderer, copied pose, or raw100/104 word map
supplies state; Components.state remains the authoritative saved bit owner.

This is a bounded concrete adapter, not full Actor state effects. Native4D4A90
first dispatches Organization57C400 and Economy570020, then its descriptor
Property list, stance selection45E620/4B4590, stance transition, world mode
4F0460 and WhenEnter4D0A00, then conditional group removal. Positive unbound
children refuse at the reached position; earlier successful stance15C stores
remain committed. There are no callback-only fake successful effects.

The supported source list proof is deliberately narrow. ActorData4624B0
initializes count18C=0 and constructs head/tail pairs194/19C through79E130.
464BD0 appends Stance and the ActorStateEvent lists. Shared mode1 reloading can
append, so a single canonical Thing declaration is required for the immutable
source projection. Absent source Stance then certifies count0. Both WhenEnter
AND WhenLeave must be empty:464FFB calls42B8E0, which can clone eligible leave
graphs into the enter list. A nonempty authored list requires its actual FXC
owner; it is not replaced by an empty runtime list.

For a nonempty Stance array and Body=NULL, actual4B4590 returns false before
reading rule fields. The reverse scan therefore returns-1. For an empty array,
45E620 returns-1 without reading Body. If the current15C differs, Character
transition is required when present; otherwise a valid old stance's null
property or absent Properties component permits the actual15C=-1 store.
Positive rule/health evaluation, Character, property removal/addition and
invalid-index diagnostics are explicit unsupported domains. The supported
branch performs no FP arithmetic; all three precision modes are still replayed.

4F0460 is reached even for a null FX head. The adapter requires the actual
retained MapResource mode supplied through Catalog; it does not infer zero
from missing state. Group removal uses actual Actor164 and generation-checked
current registry twice. Null/stale/empty references complete without a Group
call. Positive Group removal requires4DFF30 and refuses.

Proof and limits
----------------
native_state_entry.py runs full4D4A90 and concrete stance, world-mode, null-FX
and generation branches in separate original/v14 machines, without intercepted
bodies. It also executes the bounded462531..462543 constructor store interval
with explicit native register inputs and both actual79E130 constructors on
poisoned list words. This is not full4624B0/TLS/loader execution. Static original
and v14 instruction windows pin the constructor/loader, ActorStateEvent enum
and cross-list copying; static analysis is distinguished from executed pins.
The descriptor4742F6 zero-remove-byte store is also executed on a poisoned
byte.47DD32's static branch proves that only native group4 overwrites it from
Mind fields; the live binder refuses a typed Mind/native-group disagreement.
The final fixture has108 generated and3 actual saved-field rows,222 separate
original/v14 machines and137 executed image-address pins, with no body seams.

The actual3172 test uses the original tutorial02 saved Kingdom and its typed
source/Bank in all3PC. It is a diagnostic decoded-record check; immediate
Init/PostCreate and full registration/world stage acceptance belong to the
root integration. No playable battle, view/render improvement, native unwind
or general positive state-effect parity is claimed by this slice.
The actual record selects state27 structure_normal (low08000000/high0), has
zero Stance entries and retains15C=-1. actual-3172-frozen.json was captured
with source63e617... including the group4 guard; actual-3172-initial.json is
the earlier separate capture. The fresh native rows consume the frozen file's
actual component-presence, group reference, mode, state and stance fields;
unread flags in those native frames remain explicitly controlled fixture data.

Reproduce
---------
 python research/skirmish-20261016-load-actor-state-reentry/native_state_entry.py
 $env:CARGO_TARGET_DIR='target/s-graphics-reference'
 $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
 cargo test -p aa-runtime --lib load_actor_state_reentry --release --locked --offline -j4 -- --include-ignored --nocapture

The default Python command verifies the immutable fixture; --write is reserved
for deliberate author generation. Capture output uses create_new and must not
overwrite prior evidence. No root Cargo/lock or original asset/native binary,
save/profile/release is changed by this module.

Shared Mind-entry reuse (additive proof)
--------------------------------------
The inventory-borrowing Context can also execute the reached4D4A90 child after
the Mind coordinator commits actual Components.state. It does not write those
state bits itself. The current supported branches need no previous descriptor:
original/v14 instructions4D4A99..4D4AAC read/forward that argument only for the
Organization57C400 child, which the adapter still explicitly refuses first.
native_previous_pointer.py reruns all111 immutable generated/actual entry rows
with an unmapped nonnull previous pointer0xDEAD0001:222 separate original/v14
machines,137 executed pins, same outputs, no body seams. This proves the narrow
reuse boundary; positive Organization transition and4D16B0 remain unbound.
The original accepted state-entry fixture/report remains separately frozen.
The subsequently added shared positive Economy child is documented and tested
under skirmish-20261016-load-economy-refresh; the old report does not certify it.
