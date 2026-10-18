# Shortest truthful client-to-battle path and contextual Move binding

This work adds `aa-runtime::element_move_command`, the original Element Mind
contextual Move handler **538340** over the actual authoritative Rust stores.
It does not publish a loaded map or turn the asset viewer into a playable battle.
Fresh movement/attack state activations remain required live services; a test
recording a native activation seam is not a gameplay implementation.

## Executable path audited

1. `aa-client/src/tgi_frontend.rs` disables `RequestCustom`, campaign and
   multiplayer entries. `RequestSinglePlayer` opens the original single-player
   page, but there is no StartMatch action. `RequestEditor` opens the asset viewer.
   `frontend.rs::Screen` contains MainMenu, Loading and Viewer only.
2. `App` in `aa-client/src/main.rs` owns CPU/GPU scenes, picking, static map
   bindings and an optional isolated `live_preview::Active`. It does not own a
   battlefield `ActorStore`, `clocked::Runtime` or `simulation::Simulation`.
   Its frame callback ticks only preview animation/presentation. Right click
   currently requests viewport focus; picking explicitly does not issue orders.
3. `native_map_check::run` opens original Data.rwd plus v14 overrides and uses
   `Attempt::load_with_bindings` with native load mode1 and **InspectPartial**.
   Its managed SAI remains inactive; immediate Init, PostLoad, conversion and
   runtime publication are not executed. It discards its stopped attempt after
   reporting. Its `battle_ready` is explicitly false.
4. `restoration::Attempt::into_staged` transfers canonical registry/components,
   saved queue ownership, clock and RNG into ActorStore. Staged has no step API.
   `Staged::post_load` rejects any failed/incomplete record section. The later
   `init_for_loaded_map` still explicitly errors for **4F1E10 / 4F5290 / 4F1C20
   and the conversion replacement tail**. Separate native startup pieces exist;
   their presence does not mean this complete ordered coordinator is connected.
5. `SpatialWorld::publish_actor` transfers one actor's real placement into the
   spatial index; ActorStore's placement getters then read the spatial owner.
   Membership, derived classes and world lifecycle remain separate operations.
   Per-actor position publication is not battle publication.
6. `clocked::Runtime<S>` executes original world update4F9970 through required
   `Services`: fog/first-tick/manager/actor effects, predicates and real scheduled
   component deliveries. A partially failed step permanently stops that Runtime.
   No concrete complete client battlefield Services implementation was found.
7. `simulation::Simulation<S,N>` contains the native75CA70 frame driver, world
   tick and admin timeline. It requires concrete NetworkSeams for decree/admin
   processing, clock, replay/session/end checks and presentation seams. No client
   instance or complete local-session adapter was found. Replacing all required
   callbacks with success would fabricate the missing game.
8. Generic selection/player/decree control ports exist in `aa-orders`, and Mind
   dispatch/ownership in `aa-mind::command` and `aa-runtime::mind_world`. These
   require actual current player, permissions, canonical definitions and subtype
   handlers. The new module connects **one admitted subtype**; it does not bypass
   player/selection authorization, synthesize a local player or revive aa-game.
9. `live_scene::LiveScene` can apply accepted ActorVisual snapshots to GPU
   instances and picking. It does not simulate. `live_preview` owns identity1,
   a local clock, manual transforms and selected clips; none is a battlefield
   authority or suitable substitute for an ActorStore/Mind command.

## Implemented binding

`element_move_command::World` borrows one `mind_world::Context` and SpatialWorld:

- Real ActorAccess and Mind Arena determine the owner and exact embedded state
  identity/parent; only the concrete Element constructor is supported.
- Actual Commands hold the input and allow callback field mutation through the
  added `Commands::value_mut`. That method changes no identity or reference count
  and rejects absent/dead command IDs exactly like the existing `value` getter.
- The typed movement Runtime allocation owns Move target/destination. It is
  borrowed only on reached active-Move paths; no snapshot is copied back.
  Independent reentry while another callback holds that payload's mutable borrow
  remains an explicit error; this adapter does not manufacture a cached copy to
  bypass the existing movement ownership contract.
- Canonical placement supplies flags/position, including after spatial transfer.
  Generation-checked actor lookup supplies targets. Active means the actual Mind
  stack head equals the embedded Move state; status2 is not substituted.
- A duplicate active destination or target really queries Owners/Queues. If no
  update is pending it invokes existing native **529CD0** scheduling, including
  its actual Performer/Mover gates, cancellation and queue4 delay0.6. Installed
  `update_head` copies are not treated as current queue ownership.
- Performer57E280, target virtual+16C and Sleep/Move/Kill/Bombard/Follow
  activations are required methods receiving the same mutable Context. Reached
  failures propagate; committed state remains visible. There are no success
  defaults or direct position-assignment substitute for movement.

The reused `aa-mind::element_command` core gained backward-compatible granular
getters. The old aggregate-host defaults remain for existing tests. This adapter
overrides every granular read and refuses aggregate snapshots: it does not fill
unread unknown fields with zeros. Inactive actors return before command/state/
payload lookup; stale targets return before Move-state/payload reads; a fresh
point move need not read an existing Move payload. Native cached speed/auxiliary
parameters remain cached across target callbacks while current fields are read
again where the original reads them.

For integration, the admitted **move** definition is the compiled original
ActorCommandData entry: type5, component slot4, behavior flags304. Mind dispatcher
handler slot is **BC**, from HANDLER_SLOTS[5];538340 is its concrete Element
handler. It can be called from the real `mind_world::Services::invoke_command`
using that callback's Context and the caller's actual SpatialWorld. The driver,
permission boundary and fresh activation services must already be real.

## Validation and honest scope

`native_element_move.py` re-executes the original538340 machine code for all
**123 cases** at PC24, PC53 and PC64. It includes original ordered equality,
near-distance52A240, exact stack-head52A000 and generation lookup530F10. All
reached instructions match the v14 executable byte-for-byte. It records **zero
non-FS page-zero reads**, preventing synthetic SEH memory from hiding null reads.
It compares every result with the previous native fixture. This executes the
original binary; v14 equivalence here is reached-instruction-byte equality,
not a separately booted v14 battle.

These native functions are explicit oracle seams: Performer indirect fire,
target acceptance, Updatable Schedule and five activation functions. Their
recorded arguments/returns do not prove their whole implementations. In the Rust
adapter replay, duplicate-move scheduling instead uses actual Rust queue storage
and the existing separately proved scheduler port. Activation test services
remain deliberately named oracle leaves, not production game services.

The actual-store test uses original/v14 `usa_infantry` definitions, genuine
ActorStore construction/identity/component assembly and genuine Mind/Move/Queue
objects. Probe scalars and stack contents are supplied from each native row;
these are isolated state probes, not restored full battles. All123 cases include
all **three callback command-mutation probes**. Runtime IDs/state IDs are mapped
to fixture labels for trace comparison; native pointer values are not reused as
portable identities. Poisoned installed update_head and deliberately contrary
state status values check that actual queue links and stack head remain owners.

Six additional ownership probes cover inactive/immobile early returns with an
absent command and unused Move binding, stale target generation before unused
state access, fresh-move explicit activation refusal, a position changed in the
published SpatialWorld selecting Sleep rather than using the old position, and
a failing activation callback retaining its deliberate real Move-payload store.
The last partial-effect probe tests adapter ownership, not complete52DF60 parity.

## Next acceptance scenario

The first useful playable acceptance should be one original local skirmish map
loaded without restoration errors, all required Init/PostLoad/startup completed,
and its actual objects published. Select an owned infantry element; right-click
clear terrain; observe native command authorization/dispatch, Move-state/path
activation, scheduled movement updates, actual spatial positions, and matching
GPU/picking updates. Repeating the destination should retain the current path
and schedule only when genuinely not pending. Stop must clear actual movement.

Then add a real hostile target and prove target authorization, Kill/Bombard
activation, Performer updates, health/death and victory through native code.
Economy/recruitment uses the same actual queue/owner/resource state. None of these
later acceptance steps has been completed by this module. Multiplayer requires
the same deterministic complete battle authority before internet transport tests.

Existing native ports make these steps concrete, but the missing work is the
complete ordered lifecycle and actual component/service wiring. A button, a
preview animation, a fabricated player or a no-op service would not shorten
that path truthfully. No completion percentage or finish date follows from the
count of ported leaf functions.

Backups of shared files are in `backups/`. All source changes are limited to the
approved new adapter/tests, granular core getters, Commands::value_mut and one
runtime export. No original/release files, root Cargo files or dependencies were
changed by this task. Final hashes/test results are in `findings.json`.
