566DB0 fresh stored Defender InitExisting checkpoint
===================================================

The production coordinator is crates/aa-actors/src/defender_init.rs. The
pre-existing ComponentWorld566DB0 arm now calls that same coordinator through
a narrow adapter. Fresh map_actor_init_existing dispatches the actual Defender
slot to stored_defender_init_world, which borrows the installed component and
the same Staged actors, moved Bank, World Zone4 identities/map and variables.
No LoadShell conversion, component extraction, copied zone or queue is used.

The source-PC receipt comes from stored_component_init_source::checked and
the immutable Bank combat source. Source174 is the canonical component mask.
45D6C0(NULL) reads the exact DefenderComponent fixed control_range produced at
4657A0..465937 (+218);45D4C0(NULL) reads the completed canonical Organization
control_range (+2FC). These source reads use source PC; zone arithmetic and
Performer evaluation use live PC. The range getter does not rebuild unrelated
Ability children. FLD32/FSTP32 preserves ordinary bits and quiets signalling
NaNs; the latter hardware rule is not claimed from this Unicorn fixture.

Native proof: native_defender.py executes original and configured v14 images
at PC24/53/64:1050 paired rows,2100 executions,48 executed basic-block pins.
It runs actual566DB0,529C30,45D6C0,45D4C0,4D69A0,516640 and515940, including
new/preexisting Zone4, each Organization branch, Aircraft/Performer present
or absent, zero/signed-zero/subnormal/normal/extreme/infinite range words,
and a stop before every reached declared operation. Defender+2C, slot count,
Zone4 identity presence, flags and state retain their exact reached prefix.
515F40 solver and57E430 Performer are explicitly intercepted child transports;
heap/vector allocation uses the existing Loader transports. No actual zone
geometry, full Performer computation, malloc failure or dead-actor diagnostic
completion is certified here. Signalling NaNs are excluded per emulator rules.

Fresh immutable native replay passed with unchanged fixture
b38775db244248cfbd6e1cafc7c348febc6d673163310d973626efe9ed07ae8d.

Rust replay: exact production coordinator and exact test source were compiled
by replay-defender.ps1 using existing actual aa_actors Error and serde_json
archives;2/2 tests pass (1050 fixture rows and a source-unread existing-zone
guard). The fixture retains complete EAX. Native writes AL=1 and leaves the
upper EAX clobbered, so the public Boolean replay compares AL only. The first
test attempt incorrectly compared full EAX; rust-core01.log preserves that
test expectation failure. No production value changed to fix the comparison.

The runtime adapter invokes actual create_slot_fields on same ActorZones and
World.zone_identities, validates Zone4 before515940, then consumes State's
stored_zone_activation::operate_slot for real movement. Positive solver,
SAI/reveal/component children outside that adapter's accepted scope remain
explicit required errors after preceding stores. A present Performer reaches
stored_performer_refresh::refresh_value_on_owners (direct57E430, without an
extra57F1E0 gate), same current variables/Ability/Property owners and lazy
actual runtime environment sampler. Missing environments are required only
if a reached evaluation needs the sampler. No scheduler/RNG draw was added.

Scoped aa-runtime --lib release offline locked check passed. The tiny replay
does not execute the runtime Catalog/ActorStore adapter or prove actual
ger_corps1688 startup completion; that requires the parent's strict runner.
No whole runtime link, GPU/view/battle or mission-ready claim is made.

Root batch filter (aa-actors): component_virtuals::defender::tests
Commands and exact pinned sources/artifact/dependencies: findings.json.
