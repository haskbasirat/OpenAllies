SCOPED FRESH MANAGED LOAD / SAI LIFECYCLE -- 2026-10-16

BOUNDARY
restore_lifecycle.rs owns a NEW stopped application loading scope. Default is
Unknown. It never infers SAI state from MapResource mode, a map inventory, the
absence of a supplied callback, or the user's running original game.

The supported path is the first managed LoadGame of this fresh scope:
  7A0AA0 constructor -> present SAI, byte+20=0;
  76F240 constructor's Sim+3C store -> no previous GWorld;
  59E6F0/5B17F0 task construction;
  5A4280 ->76AB80 empty-world branch ->4F7910 ->5A3* ->5A2B90.
The token stops BEFORE GWorld parsing/task completion/StartMission/activation.
The parent entrypoint owns actual map restoration and keeps this boundary.

EXISTING WORLD IS NOT SUPPORTED BY THIS FACTORY
An existing world's native teardown must be connected to its actual owner.
No caller can attach an old world or set a guessed inactive flag here.
The earlier static lifecycle chain remains:
76AB80 ->4F8690 ->555030 ->7ACF30 ->7ACE50 ->7AC9A0 ->79FF10.
SAIMap+5 gates79FF10; activation7A0D40 sets that flag through7ABC30 before
setting SAI+20. Ordinary old-world destruction clears+20 and preserves the
application-owned SAI singleton. The complete old-world teardown is NOT
executed or implemented by this checkpoint. It is not replaced by a no-op.

CONSTRUCTOR SCOPE AND SOURCE
Native7A0AA0 is executed in full by the new oracle, including its call to
857D60/857910/857840 waiter/event construction, two79E130 empty-list initializers,
85D8A0 private clock-seed encoding, and7A5BF0/856DF0/4B2E60 debug-file guard.
CreateEvent is an explicit successful OS leaf, initially nonsignaled and
auto-reset. Allocator/CString storage and59ABA0 clock input are explicit leaves.
The Rust stopped owner allocates synchronization storage and growable empty
collections and retains the proven private seed. It does not run SAI waiters,
fibers, agents, goals, classifications, or AI. The original initial capacities
16 and10 are allocation policies and are not imposed as engine limits.

The complete Sim/thread/network constructor is not ported by this module.
Only its owned null-world field, which76AB80 actually queries, is projected;
the oracle executes76F2C5..76F2D7's native field stores before the real destroy
coordinator. None of the other Sim fields is accessed on that empty branch.

419C76 loads root UVars*.tgi. from_sources reads the actual V14Sources catalog,
hash-checked source bytes, and rejects additional GoalPriorityCSV declarations.
from_uvars is the explicit single-source factory, requiring the native assigned
string GoalPriorityCSV. Only unquoted null or quoted empty is supported.
Missing, duplicate, wrong-type, changed-byte and nonempty debug-file inputs
return errors. Source overrides need their real native override owner.
This is a fresh source-default scope and deliberately imports NO existing
original-game user profile; it is not a whole application/profile constructor.
The caller supplies actual application elapsed milliseconds (59ABA0 input),
not world time or simulation RNG. Tests supply known clock inputs.

OWNERSHIP / API
ScopedManagedLoad::default() is Unknown and begin() errors.
ScopedManagedLoad::from_sources(&mut V14Sources, elapsed_millis) creates the
fresh supported owner. .begin(TaskKind) consumes it and yields InactiveLoad.
TaskKind::Normal/Editor/Alternate are task wrapper choices, mapping respectively
to MapResource load modes1/2/4. Task+20 uses normal/1/2, a different numbering.
This mapping does not establish SAI state; prior constructor ownership does.

InactiveLoad is non-Clone with private fields and no public state setter.
Move it Attempt->Staged and borrow it from Catalog. observer_present returns
true, since native5705C5 checks presence. notify_observer performs only the
native7A0C20 inactive branch, which reads no actor input. No false absence.
The mutable request_world_ready boundary invalidates the capability and returns
an explicit unbound7757F0/7A0D40 error. It does not publish task success or claim
to activate AI. Borrowing/non-Clone ownership prevents a stale copied witness;
the caller cannot mutate the token while its Catalog borrow is active.
Standalone restoration must retain None/Unknown; parent owns that threading.

NEW EXECUTION EVIDENCE
native_lifecycle.py -> native-lifecycle.json contains54 successful cases:
6 clock values x3 task wrappers xPC24/PC53/PC64. Every visited instruction is
matched against unchanged original and v14 PE bytes. The full SAI constructor
publishes the same singleton and leaves+20=0. Native task prefixes record lock,
76AB80, nested lock/unlock,4F7910, selected wrapper, and5A2B90 in that order.
The empty Sim world remains null;4F8690 is guarded to fail if unexpectedly
called. No activation or task-success callback executes. Task state remains
its constructed value0 because the external worker scheduler is outside scope.

Explicit seams: memory/CString allocation; OS event creation and synchronization
calls; elapsed clock; GoalPriorityCSV typed registry cell; MapResource selection
5A28B0 supplies the existing fixture map object. Execution stops at5A2B90.
The editor fixture is an ordinary resource (byte+84=0), not a converted test map.
Pagezero ordinary reads/writes fail; only FS-prefixed SEH accesses are allowed.

REUSED EXECUTION EVIDENCE
research/resume-20261016-element-attach/observer/inactive-oracle.json has42
original/v14 executions of7A0C20 with+20=0 across seven actor pointers and
PC24/53/64. There are zero actor reads, no callback/allocator seams, and seven
executed instructions. Null and unmapped actor values still return normally.
The old lifecycle-static.json and owner-binding-followup.json contain the
full teardown/task-completion static chains; they are not new full execution.

REPRODUCE
python research/skirmish-20261016-restore-lifecycle/native_lifecycle.py
$env:CARGO_TARGET_DIR='target/s-restore-lifecycle'
$env:AA_GAME_ROOT='E:\Games\Axis and Alies'
cargo test -p aa-runtime --release --locked --offline -j4 --test restore_lifecycle_native -- --include-ignored

No production file existed at either new module/test path before this task.
No shared files, original EXEs, saves, profiles or release artifacts were edited
by this agent. Parent owns module export and lifecycle threading. Final source
hashes and test outcomes are recorded in findings.json once validation finishes.
