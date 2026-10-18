Ability FX request/owned instance handoff

Original4C8500 ->4294E0 ->428F40 ->428570 is implemented by aa-combat::fx::emit.
It selects the authored start/use/end/hit reference, honors absent definition
and suppression, captures context fallback vectors, derives allegiance from
source virtual classification/owner, then reads current actor positions and
GTIDs. It selects the original rendering-thread clock or world clock and creates
all semantic Instance fields with native defaults. Local ability data and use
count are retained. Native vtables, callback self pointer, and allocator address
are portable identity details, not values copied into serialized Rust state.

52 native cases make208 calls/emissions including absent actors/owners, mutable
allegiance getters, clock branches, suppression, absent references and count wrap.
Full original constructor45C700 runs. Only actor virtuals, GetCurrentThreadId,
allocator and final50C800 submission are intercepted. Copy-only FX arithmetic is
run with native Extended64; no all-precision FX arithmetic claim is needed.

The manager's queue/immediate-registration path, effect-graph execution, particle
simulation, sound and GPU rendering are NOT implemented by this module. World::submit
must consume this owned Instance into the actual manager or return an error;
a successful diagnostic event log is not production behavior. This work provides
the precise request at that boundary, not evidence of visible effects in a game.
Context and actor/definition identities remain valid during dispatch; arbitrary
recursive deletion is outside the contract. No original assets/saves/exes changed.

One native differential Rust test passed. Commands:
python research/phase7-fx/native_fx.py
cargo test -p aa-combat --test native_fx
cargo clippy -p aa-combat --all-targets -- -D warnings
