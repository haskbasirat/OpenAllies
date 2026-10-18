Actual ResourceManager queue / UI View task checkpoint

This change connects actual5C3000 task publication to the SAME retained
ResourceManager TaskQueue and counter used by current Resource tasks. One
5B17F0 Header owns references4/priority8/sequenceC/queue10/state14/thread18 for
both typed payloads. It does not clone task status into a view-only queue.
The shared enqueue algorithm, signed-priority/unsigned-sequence ordering and
actual tail pop execute before native state1/thread capture. Both admitted
native vtables have vf10/vf14=false, so the manager+4 recursive lock is used.
The same Task18 is borrowed for guard validation, with no second thread lookup.

The private View callback retains only weak loans of the same World/VWorld AC
and Application Loading17 allocation. It clearsAC, setsLoading17, then stores
Task14=2 preserving18. Actual final-reference View destruction executes the
original5C38A0->5B18B0/free path in the oracle. The production UI transfers its
only Task owner after actual enqueue; TaskWitness is nonowning and normally
retires after the successful final release. DispatchReport contains only
identity/status metadata read after dispatch; it cannot write AD or activate a
battle. The legacy production direct-callback facade was removed; direct
complete/flag-writer APIs exist only under cfg(test) for bounded old fixtures.

Application publication requires canonical source validation, matching Bank
source identities, same Task/World/VWorld witness and actual AD already true.
The concrete caller obtains World and Application from one ActualActivatedSession.
After the true World/Session lock ends, the controller dispatches the actual
queue in its native priority/order until that exact View header is seen.
A resource error retains its popped allocation and state/reference prefix,
leavesAC/Loading17 unchanged, releases host lock loans and refuses a retry.
Queue empty is an error, never a completion signal. Cancellation state4 is
reread after diagnostic and produces5 without the completion callback.

Modern execution boundary: this closed dispatcher runs on the existing retained
SimulationExecution with its genuine typed thread identity while the UI pumps
real resource requests. It is a deliberate modern backend scheduling choice,
not a claim of original ResourceWorker bootstrap, manager shutdown/cache-timing
or C++ exception/unwind equivalence. Running/failed storage preserves refused
prefixes. Constructor-empty waiter128/12C is proved separately; positive
5B1FD0/waiter insertion remains required. Resource final0 destructor remains
required; ordinary admitted entries retain the actual resource-owned reference.
Unsupported decoder/model/audio children cannot be passed by this barrier.

Validation
- Exact current production Header/queue/typed Resource identity/World witness/
  weak callbacks plus existing tests:17/17 pass in exact-queue-tests-final-v2.exe.
- New native queue51 rows:102 separate original/v14 executions, all3PC;
  509 recorded pins,508 executed unique pins plus observed terminal5AE022.
  Resource vf1C is an explicitly named child seam; actual View vf1C5B3550,
  actual40D970 getter, task status/thread, enqueue/pop/refcount/deleting
  destructor and empty waiter traversal execute. Alloc/free, atomics, thread
  registry, synchronization and diagnostics are declared transports.
- Actual waiter constructor six rows:12 native executions,3 executed pins.
- Fresh immutable native replays and original/v14 pin verification pass.
- Existing Rust regressions cover36 queue rows,66 wrappers,6 queue-signal rows,
  18+360 VWorld constructor/prefix rows,1320 Loading rows,180 UI Init rows,
  finalAD store and weak ownership/current-thread/recursive-guard checks.
- The isolated host mechanically includes exact Handle and World-revision
  production fragments; unrelated Resource Entry/decoder payloads and World
  integration tests are omitted. No replacement behavior modules are used.
  Exact metadata/pins are in exact-host-source-pins.json.
- Native thread pointer12340000 is a lab registry proxy only. Rust compares its
  presence to a genuine retained ThreadHandle(OS17), never casts OS IDs into
  native pointers. AC comparison uses its public boolean getter; seeded raw
  sentinel values are preserved/cleared by the same actual atomic field.

Commands (working directory rust-rebuild; no game process)
  python research/skirmish-20261017-view-task-queue/native_view_task_queue.py
  python research/skirmish-20261017-view-task-queue/native_queue_waiter_constructor.py
  python research/skirmish-20261017-view-task-queue/verify_native_pins.py
  research/skirmish-20261017-view-task-queue/exact-queue-tests-final-v2.exe --nocapture
  cargo check -p aa-client --release --locked --offline -j2 --target-dir target/s-graphics-review

The complete client production check is a typecheck, not a natural activated
mission/render/frame/playability acceptance. This checkpoint does not certify
304/315 completion, Loading notification0/1 consumption, actual UI state enter/
leave callbacks, or rendering/picking. The root agent owns that next notification
and application-state composition. No battle-ready bit is introduced here.

Recoverability
before-source-pins.json records contemporaneous backups for the first eight
runtime source/test files. before-remove-direct-view-callback.rs is the genuine
pre-removal facade. Current shared integration files are checkpoint snapshots,
not labeled pre-edit backups; legitimate successor changes must be scoped apart.
The earlier prototype fixture and logs are retained. Only the final named fixture
is authoritative for the final typed dispatcher. No original game file changed.
