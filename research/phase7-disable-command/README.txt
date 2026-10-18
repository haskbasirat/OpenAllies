Element disable/stop command6 handler52D910

Implemented in aa-mind::disable_command. The real installed Element Mind owns
its embedded Sleep state at native offset68. The handler calls the actual
reconstructed activate(mode2) /52A810, including admission, leave, stack replace,
scheduling, state entry and change callbacks. A false admission returns false
without leader/facing lookup. After activation, the actor is read again because
synchronous callbacks can mutate the live Arena. Actor+80 Element and its+10
leader are optional native branches. A present leader supplies current XY facing
for the required4EEBE0 actor/spatial service.

The production set_actor_facing callback must bind the existing concrete
SpatialIndex::set_facing with actual memberships and presentation callbacks;
a UI-only orientation write is not this service. Command admission4D9B60 and
Mind dispatch remain upstream; this handler does not bypass them. Wrong kind,
missing Sleep68 or mismatched parent binding return an explicit error.

52 original native cases execute52D910 and actual52A810/stack lifecycle. State
virtual admission/entry/exit, scheduler and final4EEBE0 are explicit fixture
services. The already-proven spatial setter is not re-proven by interception.
Fixtures cover absent/empty/present Element leader, state stacks, denied admission,
actor replacement during activation, changed parent/head and entry mutation.
Every state/event trace and return value matches. Native ABI checks apply to
the oracle only; no native ABI or complete tutorial/world initialization claim.

Two Rust tests passed. The second returns an error at facing after successful
Sleep transition and verifies no false rollback or reported successful command.
Strict scoped Clippy passed. No existing original game/assets/saves were edited.

Commands:
python research/phase7-disable-command/native_disable.py
cargo test -p aa-mind --test disable_command_native
cargo clippy -p aa-mind --all-targets --no-deps -- -D warnings
