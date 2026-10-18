Concrete scheduled component delivery, 2026-10-16

live_scheduler_dispatch::dispatch consumes actual Delivery owner tokens through
scheduler_owners::Owners and borrows the one ActorStore, Queues, NativeRng and
published Spatial owner. It does not construct a substitute clocked::Services,
copy component state out/back, or claim battle readiness.

Native entry selection matters: periodic50B330 calls Updatable virtual+14.
Deferred queue3 (GWorld+1E8) uses4F05C0 -> virtual+18; deferred queue4
(GWorld+208) uses4F05B0 -> virtual+1C. Queue4 is not another57F040 entry.
The adapter rejects incompatible queue/kind delivery metadata. Unsupported
targets, including the actual retained Sun singleton, error explicitly.

Implemented paths:
* Actual Recruit, Builder, MobileDeployment, Research and HQ periodic vtables
  point to4D1E10 (RET8). Their no-op does not validate/read unused float words.
* Performer queue3 entry57F040 checks its actual owning allocation, current
  Actor.active_component and then actual4D03D0 sources in native order.
*4D03D0 reads Thing+8 bit0 through current Spatial storage. That bit skips the
  Properties read. Otherwise a present actual Properties reads modifier.flags
  at+1C bit1, not its later modifier_flags field. Null is an actual absent slot.
* Paused deliveries call real Queues.add_after(queue3,owner,data,0x38D1B717).
  Native6A71D0 makes the Performer scheduling admission true. No RNG is drawn;
  payload bits are preserved, including negative signed events.
* Inactive, mismatched stage, signed negative/default switch events and cues
  with no actual Character return through their proven native paths.
* Stage0 writes actual turret_index=-1 and50B1C0(0,0) removes queue0/data0.
  These effects persist before the explicit required57ED20 begin error.
  This removal is not a cancellation of the deferred queue3 event.

First unbound callbacks remain errors: begin57ED20, use57E8C0, end ability
context4C8300/4C8850, actual Mind notification57DC90/+138, Animator563AA0.
Other periodic component updates and queue4 virtuals are also unbound.
Errors do not roll back earlier effects. The enclosing clocked runtime must
stop after a failed partially committed world operation; retry is not recovery.

native_dispatch.py executes original x86 at PC24/53/64 on actual native
Updatable vtables and queues. Native4D03D0/50ACC0/50B1C0 run unchanged.
Allocator/free/log are explicit fixture seams inherited from phase6 queues;
page0 is explicitly unmapped. Gameplay callbacks stop at their exact entry,
without fabricated successful returns.972 Performer rows reach756 bounded
returns and216 explicit first-unbound boundaries (36 each begin/use/end/finish,
72 Animator).135 additional periodic no-op rows preserve all component bytes,
queue state and RNG, including unused NaN elapsed words. Exact original/v14
instruction and vtable pins accompany the fixture.

Rust replays all972 outcomes against the actual original-source ActorStore,
actual current published placement, native-created Performer/Properties/
Character and real queue/RNG owners. Performer allocation address is stable.
The135 no-op class probes are explicitly isolated component constructors on
that source actor; they do not claim infantry naturally owns all those slots.
Further tests cover reached/unread corrupted Properties, current published
Thing flags, actual queued paused delivery, stale scheduler token retirement,
foreign Performer ownership and an explicit Sun required-update refusal.

An initial test setup incorrectly tried constructing isolated no-op classes
after spatial publication. ActorStore correctly rejected it. Its failed log
is preserved. Constructors now run before publication, without weakening that
production guard. Final logs/findings identify successful replay results.

No fresh authored floating field is read by these admitted prefix paths, so
they require no substitute source precision. Future data-dependent updates
must use the actual completed source Bank PC, separately from queue/simulation PC.

Commands from rust-rebuild:
$env:PYTHONPATH='E:/Games/Axis and Alies/reconstruction/tools/python-packages'
python -B research/skirmish-20261016-live-scheduler-dispatch/native_dispatch.py
$env:AA_GAME_ROOT='E:/Games/Axis and Alies'
$env:CARGO_TARGET_DIR='target/s-detection-mask'
cargo test -p aa-runtime --test live_scheduler_dispatch --release --locked --offline -j4 --config profile.release.lto=false --config profile.release.codegen-units=16 -- --include-ignored

No artificial limits were added or removed here. This component supplies real
scheduled behavior at proved boundaries, not movement/combat/economy success,
an end-to-end match test, original save compatibility or a final EXE validation.
