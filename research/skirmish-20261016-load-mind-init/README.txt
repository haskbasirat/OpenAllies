Held saved Mind InitExisting adapter
===================================

Production ownership
--------------------
crates/aa-runtime/src/load_mind_init_world.rs binds Element 529EC0 and Regiment
51D1B0 to the actual current live_object_init::Record. The owner remains Loading
in the real generation-bearing registry. Source definition, category, constructor
mask and source FP owner are checked against the installed Bank/Catalog.

The same RestoredMindParts allocation owns installed fields, command allocations,
the Mind core and embedded state allocations. A scoped Arena temporarily owns
the core and states. They return to the same component BEFORE any Actor callback
and are reborrowed afterward on success or error. No ActorStore, Organization,
replacement Mind, queue, RNG draw or default stack is fabricated.

The native 529EC0 empty/nonempty coordinator was extracted without behavioral
changes into mind_world::init_existing and is shared by existing live storage
and the new held-owner adapter. The saved_actor_state::set_fields helper similarly
splits the existing native state kernel's actual state/Thing owners; the old
ActorStateContext entry forwards it unchanged.

Concrete scope
--------------
The Element constant state+20 getters, Sleep's actual high-state bit predicate,
Regiment's constant getters and Sleep/Disable predicate are concrete. Regiment
Sleep51D690 returns17 unless actual Disable.status==2, which delegates to12. The
first native-backed Rust prefix replay caught and corrected an initial erroneous
constant12 interpretation; final-mind-replay.log retains that failed run. Unsupported
dynamic getters stop before their actual required service. Regiment 4D7DA0 reads
real Organization presence, current state bit4 and lazily Actor+118 before the
state40 call. State changes run the original aa_actors::state coordinator and
commit native prefixes before callbacks. Reentrant callback changes are reread.

The default action callback invokes saved_actor_actions::rebuild with the actual
held actor, inventory, components, source Bank and ability owner. It does not
invent an Organization. Actual source PC and live arithmetic PC remain separate.

RequiredEffects refuses actual entry/leave effects, default-reset services and
unsupported dynamic getters when reached. Empty Mind reset is NOT implemented
by this adapter. Aircraft Init517C60 is also explicitly unavailable. These are
real boundaries, not successful no-op virtual calls. AfterInit is the common
native 4B2830 leaf, with exact concrete vtable-entry pins in the native fixture.

Evidence and qualifications
---------------------------
actual-before-init.json is the historical first read-only capture; its whole
Mind fields were not a complete context receipt. The historical native fixture
native-mind-init-v1-context-probes.json used labelled-but-unverified extra context
values. It is retained as history, NOT the final native actual-context evidence.

actual-before-init-v2.json captures the first four actual stopped Kursk owners
including actual Organization presence, Actor+118 availability, all embedded
state statuses, stack, saved state bits, flags and command ownership. These are
Regiment herd_horse15283 and Element horse15284/15285/15286: animals, not military
combat units. The map is restored in diagnostic InspectPartial mode with verified
source setup. This is not proof of a completed native map load or complete Init.

native_mind_init.py executes original and v14 instructions independently at each
of Single24, Double53 and Extended64. 903 rows cover constructor-state offsets,
empty stacks, state/flag predicates, Organization/Actor+118 gates, Disable status
and four actual saved input projections. Child effects stop at their genuine
entry points. They are not mocked as successful effects. Unavailable Actor+118
bytes in an unreached actual context are poisoned; they are not assigned zero.
Address-byte pins distinguish executed instructions from stopped child entries.

The Rust actual four-owner test calls the REAL action child, preserves the same
allocation/core/states/commands, and compares all twelve resulting sync calls
(four actual owners x three live PCs). Source and construction are Double53 in
that test; live PC varies independently. It also replays all 903 native prefixes
on existing typed allocations with explicitly labelled test field/stack changes
and stops at the same child boundaries. Original allocations are restored before
the registry receives each held record. These probes do not represent full game
records with those edited scalar combinations.

actual-four-sync.json is historical execution-only capture before the corrected
Sleep getter. It is not final parity evidence. actual-four-sync-v2.json is the
corrected final actual sync capture, compared during immutable replay.

The direct split-state test replays all 4,044 existing native Actor-state rows.
Existing saved_actor_state tests cover the unchanged original entry point too.
SavedActions regression tests are separate proof of the concrete child; a Mind
native stop at 4D1350 alone does not prove that child's full behavior.

Replay commands (PowerShell, project root)
-----------------------------------------
$env:AA_GAME_ROOT='E:\Games\Axis and Alies'
$env:CARGO_TARGET_DIR='target/s-state-binding-review'
$env:AA_MIND_INIT_CAPTURE=$null
$env:AA_MIND_SYNC_CAPTURE=$null
python research/skirmish-20261016-load-mind-init/native_mind_init.py
cargo test -p aa-runtime --lib load_mind_init_world::tests --release --locked --offline -j4 -- --include-ignored

Normal native and Rust runs compare frozen evidence without rewriting it. Capture
paths use create_new. Native --write intentionally regenerates the native fixture.
See findings.json for final hashes, outcomes, exact commands and any pending
validation. No whole battle, multiplayer, gameplay-completeness or percentage
claim follows from this bounded adapter.
