# Fresh Denizen InitCreate

`map_denizen_init_create::initialize` now binds native `568750` to the same
staged ActorStore, Denizen Box, source bindings, queue owners, queues and RNG.
The dispatcher forwards only this newly implemented leaf. The existing
`component_virtuals::init_create` Denizen branch delegates to the same
extracted `init_create_denizen` kernel through a small `DenizenCreateWorld`
adapter; it supplies no unrelated successful services.

The kernel checks actual construction state and Params mode before the
Inventory loop. Each target's null-context `45D730` health is stored in its
current slot immediately. The actual finalized source guard range is read
before the source Inventory-head predicate. That head remains present for
authored zero/negative counts, even if there are no constructed slots. The
positive branch uses the existing concrete periodic queue/RNG kernel. Only
then is source `3A4` read and its negation stored in the same Denizen `34`.

The runtime validates the real Catalog binding allocation against the moved
source Bank. Target health comes from that binding's canonical health map;
no occupant, source or queue is fabricated. `3A4` has the actual constructor
zero from `4629F8`, overridden only by optional `DenizenComponent/Runway.delay`
at `46759C..4675A8`. Those producer windows, names and the constructor's EBX
clear are statically pinned in both images. They are not extra executed
instruction pins or a new whole source-construction claim.

The new native fixture executes actual `568750` and `45D730(0)` against both
images at all three precision modes: 195 paired rows, 390 executions, 53
decoded equal executed pins. Eight required read/queue cutoffs check already
committed health stores and unchanged final time. `50AC60` is an explicit
service seam; its production queue implementation is not replaced by the
fixture callback. The class's real Updatable acceptance slot is separately
pinned to `6A71D0` (true).

Both affected Rust tests pass: the new 195-row replay and the existing full
component-leaves replay. Target health is finite binary32; quiet NaN guard
comparison and raw source-time sign copies are covered. Signalling NaN CPU
exception behavior, invalid native links and unwinding are outside this
receipt. Inventory/source ownership is stable during the direct null-context
health reads; no application reentrancy is inferred from laboratory cutoffs.

The genuine predecessor is `playable-startup/attempt25.fresh-actor-prefix.json`
(SHA `0e35d35c5f2d0c50aa0d7ac5679f0234f736061ec097a8e522d6ce0deb87ffa4`):
ger_corps GTID 1688, key 2362, actual empty Inventory and initialized Denizen
time zero. That snapshot identifies the reached allocation; the production
constructor does not consume it as a source or default. The next complete
map run belongs to the startup runner. No StartedMission, activation or
playable-battle result is certified by these bounded tests alone.

```powershell
$env:PYTHONPATH='reconstruction/tools/python-packages'
python research/development10/denizen-init-create/native_denizen.py
python research/development10/denizen-init-create/audit_denizen.py
cargo test -p aa-actors --test native_denizen_init_create --test native_actor_creation_leaves --release --locked --offline -j4 --target-dir target/s-settlement-completion
```
