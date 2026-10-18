Native live dispatcher binding (bounded; not a playable-battle certificate)

The existing aa-world4F9970/4F65B0 kernels and clocked Runtime retain their
laboratory defaults. Five typed field/list hooks let live_clocked::Services use
the same actual fog fields, auxiliary pointer, Body owner, placement flags and
registration lists. No scan constructs a second mutable flags/classification
table. Published removal flags remain in the same SpatialIndex Member; staged
flags remain in the same ActorStore placement. Only native masks2/81 are allowed.
Registry removals still require their actual lifetime services.

DispatcherState owns native constructor phase0 and empty linked-list headers
without a clock. Root keeps it during loading, then World::from_loaded_dispatcher
moves those owners beside the single moved actual load clock. Runtime::from_parts
moves the existing World/Queues/RNG/services. It checks raw clock/precision
coherence only; a failed bind returns all original parts. It does not initialize
missing queues or certify map startup. Existing Runtime::new laboratory behavior
is retained. Failed live operations stop Runtime after their committed prefix.
ClockedState permanently owns World/Queues/RNG and the same failure latch while
Services is borrowed per frame. Runtime::into_state transfers its existing latch,
including a stopped one. ClockedState::borrow exposes the existing WorldSeams
interface to Driver without self-referential owners or an inferred timer.

Concrete Services borrows ActorStore, scheduler Owners, SpatialWorld,
live_registration::State, actual Synchronizer A/B3655C, actual TriggerManager and
persistent CoreProgram. Scheduled deliveries reuse live_scheduler_dispatch.
Manager updates reuse54C280/5411D0 and the existing manager-local opcode kernels;
other opcodes stay required. The actual sync thread/application gate is called
only after enabled && suspend<1. Its Boolean output is the existing SyncLog
77D9D0 input, not a rendering operation. The same retained log is mutated.
The original direct SyncLog API remains, inside SyncBinding::Direct. Shared
binding borrows the actual SyncA RefCell only during the Fog callback, releasing
it before FirstTick/queues, so NetworkSeams can loan that same owner later.
Borrow conflicts are required errors; no duplicated or always-enabled log exists.

FirstTick uses the narrow FirstTick trait shared with the separate concrete
4F96D0 music/Player/Nation owner. UnboundFirstTick errors. This service does not
infer Player+28 from a Kingdom name. World+4 distinguishes Unbound, Null and a
borrowed real counter; zero is not guessed. Body561C20, zone5147C0, available-city
effects, GTID50C1B0 unlink and Actor lifecycle virtuals remain precise required
errors. No wall clock, Driver, source completion or whole-game success is minted.

native_live_prefix.py independently loads unchanged original/v14 images, verifies
their hashes and matching executed instruction bytes, and runs PC24/53/64.
216 executions cover SVar/World fog branch choices, null/8/u32MAX counters and
disabled/suspended/reached-required-thread-gate logs. Actual77D9D0 and phase
transition execute. 41C0E0 profiling instrumentation is an explicit RET4 seam.
FirstTick4F96D0 and Windows GetCurrentThreadId are required stops; neither body
is answered by successful invented behavior. 78 genuinely executed native
addresses are pinned separately from those boundaries. The native memory-read
trace proves a true SVar fog value skips the World+275 read. Enabled successful
logging is inherited from phase8-net/native_sync.py, not newly claimed here.

Default native command is immutable replay:
  python research/skirmish-20261016-live-clocked/native_live_prefix.py

Seven focused Rust tests replay all216 native prefix outputs, same-owner lazy
gate behavior, exact first-tick partial errors, a later explicit empty-manager
fixture, and move-binding failure/success preserving existing queue node IDs and
RNG. Empty-manager success is a labelled synthetic bounded fixture, not an actual
map or gameplay readiness claim. Native queue/world legacy suites are separately
replayed after these hook changes; final test results are in findings.json.

No game executable/profile/archive/release files were changed or launched.
