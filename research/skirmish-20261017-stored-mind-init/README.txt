Fresh Mind InitExisting and retained reset runtime (bounded checkpoint)
====================================================================

Production entry: aa_runtime::stored_mind_init_world::init_existing and
after_init. The existing native529EC0 coordinator receives the SAME stored
Mind Arena, ActorAccess, World.mind_runtime, queues, scheduler owner map,
clock, variables, source Bank, RNG, and adopted spatial world. No LoadShell,
Arena, command directory, state payload or Runtime is cloned or substituted.

The root retains one Runtime from the fresh interactive application interval,
moves saved commands once after ActorStore conversion and before fresh reset,
then lends the same runtime to frame orders. Its latch zero values are PE
zero-fill observations, not invented stores by a World constructor. Replacing
a World in an existing application must reuse this runtime; this checkpoint
does not establish that later replacement path.

Native proof (original and v14, each PC24/PC53/PC64):
* native_regiment_reset.py: 864 parent-protocol rows, 1728 executions,
  74 executed address/byte pins. It executes51E3A0, real NULL52A660 and
  generation lookup4D11A0. Resume, Join, Denizen, state activation and
  scheduling are explicitly supplied child contracts. Failure cases stop at
  their actual native entries before child effects. Activate AL is ignored.
* native_element_reset.py: 96 parent-protocol rows, 192 executions,
  26 executed pins. It executes52D7C0 with explicit resume/activation
  child contracts and real NULL command clear. The one shared Rust kernel
  now retains guard1 on a reached child error and clears it only after a
  successful resume/activation, before final52A660, exactly like this prefix.
* native_reset_guards.py: both PE initial bytes and nine literal references
  in a skipdata linear .text audit. This does not exclude arbitrary computed
  pointer corruption and does not call the process globals World fields.

Default Python commands replay immutable fixtures. Only --write authors them.
The old native-regiment-author.log records a rejected wrong scheduler seam;
the corrected final oracle intercepts actual50AD80, not51F2F0.

movement_world::PayloadHandle now retains one Arc<RwLock<Payload>> allocation
instead of Rc<RefCell<Payload>>. Nonblocking try_read/try_write preserve
multiple-reader/exclusive-writer and reentry errors; a poisoned lock returns
its existing guard and retained writes, with no reset. This makes the actual
canonical runtime Send; it introduces no concurrent simulation updates.

Validation available at this checkpoint:
* production-check-final.log: source check passed31.08s (the later home read
  correction is covered by test-source-check-final-02.log compilation).
* test-source-check-final-02.log: no new Mind/payload source error, but the
  combined lib-test check stops at a foreign missing canonical_svars fixture.
* native Regiment/Element immutable logs passed; guard audit replay passed.
* Rust execution is assigned to the root's next coherent runtime cohort:
  stored_mind_init_world::tests:: (3 tests)
  movement_world::payload_tests:: (2 tests)
  Do not label these tests executed until that cohort reports success.

New callbacks bind actual split-owner4D1350 and repeated Character stance.
Sleep/Rampage constant state fields, pending-command resume, canonical source
Rampages and queue4 schedule are concrete. Aircraft flight, live Home Join,
Denizen, other active state metadata, positive queued commands/facing/group
effects and Actor4D4A90/4D16B0 children retain reached required errors.
The last two are the next assigned successor, outside this checkpoint.

There is no actual initialized fresh-actor acceptance or playable-battle claim
here. Parent-protocol child contracts and controlled constructor fixtures are
explicitly different from full native child/world execution. Heap failure,
native SEH unwind and later application-replacement globals are not proved.

source-v1 retains source snapshots; freeze.json pins inputs and fixtures.
