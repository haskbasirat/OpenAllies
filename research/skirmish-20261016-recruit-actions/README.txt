Recruit action contributor 582C40 (2026-10-16)

Implemented API
---------------
aa_actors::recruit_actions::contribute(world, recruit_storage_owner)
The World trait supplies actual component/source fields and mandatory
eligibility5827B0. A later approved source/saved-restore adapter is described
below. No Cargo configuration, release08, original game or save was changed.
The contributor module and its test are new. The existing aa-actors
lib.rs was backed up before adding its single module export:
backups/aa-actors-lib-before-export.rs.

Native behavior ported
---------------------
Full582C40 runs in original order:
  - Native group13 selection4D14C0/4D0820 on the Recruit component owner.
  - ActorStateData+28 equal to1 is blocked. The group number at+2C is a
    different field. The owner's state bit20 is sampled at this point.
  - Unless already blocked, a nonnull Settlement+14 causes group13/type
    selection on that referenced actor.
  - Commission bit14 is allowed when not blocked (or selected state29),
    not under siege, and the current owner has a Settlement component.
    Native517FA0 writes this bit before any eligibility calls.
  - When not blocked, separately read ActorData+4EC list presence and
    Recruit+11 (ordinary), then list presence and Recruit+10 (location).
    These bytes are tested as nonzero, not normalized stored booleans.
  - If either path is enabled, walk the live linked list in order. Snapshot
    each entry ActorData+184 classification and+2F0 LayoutParameters, then
    call5827B0(definition,layout,1,0). Only result2 excludes an entry;
    all other words count. Counters retain native wrapping signed32 adds.
    The next link is read after the callback, so mutations are observable.
  - Set ordinary recruit bit15, then location recruit bit17, then rally
    bit16. Rally requires not blocked (or state29), and commission or
    ordinary recruitment. Location recruitment alone does not allow rally.
  - Every write rereads the current Recruit owner, preserves unrelated
    action bits, and leaves upper32 bits unchanged. Eligibility callbacks
    can rebind the component owner; final writes use the current owner.

Required binding fields
-----------------------
World::recruit_owner: current Recruit header+4; the input identity addresses
  stable component storage even if its owner changes.
World::fields/set_action_bits: actor state+100/+104 and actions+F0/+F4.
World::state_type: actual loaded ActorStateData+28 (not an invented default).
World::settlement_reference: absent component distinguished from null+14.
World::recruit_enabled: raw byte+10 or+11, on the same Recruit storage.
World::recruit_list_head: current owner's ActorData+4EC, separately read at
  each native gate and immediately before traversal.
World::recruit_entry: definition identity, +184 !=0 class, +2F0 layout.
World::next_recruit_entry: current next pointer read after eligibility.
World::eligibility: the concrete5827B0 query with check_requirements=1 and
  no diagnostic destination. An unavailable service returns an error;
  it must not infer eligibility from presentation data or return success.

Proof
-----
native_recruit_actions.py independently loads and runs original AA.exe
and accepted v14 AA.exe. At PC24/53/64 it executes full582C40 plus actual
4D14C0,4D0820,529C30 and517FA0. Every executed native byte is checked
against both images by the established economy harness. Function windows
and executable hashes are retained. Group13 mask comes from the pinned
previous original477FC0 state-registration oracle; no new mask is guessed.
State types and source lists are explicit input facts.5827B0 alone is the
gameplay seam; arguments and call order are recorded.

906 runs =151 inputs x3 precision modes x2 separately loaded images.
Cases vary group states25..31 and multibit selection, type0/1/2, settlement
absence/null/target, siege bit, empty/mixed lists, zero/nonzero enable
bytes, eligibility0/1/2/3/FFFFFFFF, both classification values, high action
bits, null/non-null layout words, next-link mutation, classification change
inside eligibility, action-mask mutation, and owner rebinding.
Injected eligibility dependency failure is explicitly labeled; the native
function stops at that seam. It proves the earlier bit14 write remains,
not that the game itself throws this injected error.

Native action-memory writes are recorded once per low-word write together
with the current unchanged high word. The native high-word self-writes
have no intervening callback and are not counted as separate Rust events.
Rust replays exact masks, callbacks and their order for every row.

Validation: 2/2 focused tests pass; tests.log.
  contributor_matches_original_and_v14_fields_call_order_and_failure_prefixes
  cyclic_source_list_is_explicitly_unsupported_after_earlier_writes
No floating-point arithmetic is added, so Precision is not an API argument.
The oracle still verifies all three native control-word modes.

Reproduce from rust-rebuild:
  $env:PYTHONPATH='E:\Games\Axis and Alies\reconstruction\tools\python-packages'
  python research/skirmish-20261016-recruit-actions/native_recruit_actions.py
  $env:CARGO_TARGET_DIR='target/s-layouts'
  cargo test -p aa-actors --release --locked --offline -j4 --test native_recruit_actions -- --nocapture
  python research/skirmish-20261016-recruit-actions/record_findings.py

Explicit limits
---------------
Recruitment availability5827B0, requirements/economy and live action
publication are separate required dependencies. This is not
unit production, completed recruitment, a playable skirmish or multiplayer.
A selected group13 of-1 is rejected as an invalid descriptor lookup; the
controlled native fixture observes a guarded null access, not a claim
that every real invalid lookup must fault identically. A cyclic source
list would not terminate natively; Rust returns explicit unsupported after
the already committed prefix. No arbitrary iteration or allocation cap is
introduced; the visited set grows. Allocator exhaustion is not covered.

Source projection and saved-restore binding
------------------------------------------
aa_rules::native::recruit_data exports:
  actor_recruit_targets(catalog, canonical_actor_id) -> RecruitTargets
  actor_transport_class(catalog, canonical_actor_id) -> bool
  actor_state_type(catalog, compiled_state_name, precision) -> u32
This uses the existing typed ActorCatalog and ResolvedNode resolvers. It is
not a generic ActorData loader. Canonical ActorData or ActorState names
shadowed by a translation are explicitly rejected; they cannot silently
resolve to a different definition. Deferred target lookups are performed
once using the original authored lookup and retain the canonical result.

Native source paths:
  - Recruit fragment467C89..467DB6 in ActorData464BD0 tests actual component
    presence, sets bit16, and requires Structure bit13 for nonempty lists.
    GetString(actor_ids),473AD0 prepends GameData+824 deferred records.
  -475230 visits those records last-created first. VtableA03470 dispatches
    461810 ->460310, which validates ActorData via4BDBA0(category2B), then
    prepends owner+4EC/+4F0 targets and target+17C/+180 backreferences.
    Two reversals preserve authored order and duplicates on success.
  -4608C0 resolves transport_actor. Empty CString stores null+184; nonempty
    requires actual ActorData with TransportComponent bit24 or throws.
  - CRT9CC560 ->48A630(B3AD80,1) registers NONE0/BUSY1/QUEUE2. State loader
    47DC50 fragment47DCB3..47DD32 gets type, calls enum lookup497220,
    stores+28, and checks invalid values. The type is separate from group+2C.

native_recruit_source.py runs the original TGI parser with the exact
template_recruit.tgi bytes, the producer fragments, deferred callbacks,
full460310 for simple target backrefs, full4608C0, and state enum/type-store
code.4BDBA0 is an explicit typed registry seam with fixture ActorData;
405D60 diagnostic formatting and atexit are infrastructure seams. Target
fixture masks deliberately exclude organization backreference recursion;
that branch is outside this bounded list projection. It checks the native
instruction windows against accepted v14. Source fixtures contain45runs:
7list cases +4transport cases +4state cases, each at PC24/53/64. Failed
load rows prove rejection; Rust does not claim to expose the original
partially consolidated source registry after a failed definition load.

native_recruit_data.rs replays all45rows. The ignored real-source test
visits every standard ActorData in the native-order catalog and all42
ActorState definitions (at all3 precisions);46nonempty Recruit lists and
44transport-linked actors resolve. source-census.json includes exact
source file hashes. source-tests.log records2/2passing tests.

aa-runtime/src/saved_recruit_actions.rs implements the contributor World
trait on the existing authoritative SavedActions adapter. It reads the
actual Recruit header owner and raw byte10/11, real actor state/actions,
actual canonical owner definitions, compiled ActorState names, source
targets/classification and existing native default-layout projection.
The source list is immutable during saved restoration; its cursor is
(canonical source ID, index), and every head/entry/next query reads that
list. A real Settlement component requires an actual+14 payload; it is
explicitly unsupported until that payload is owned. Missing component and
present-with-null payload are never conflated.

Recruit constructor581E40 initializes byte10/11 to0; saved loader583C80
does not restore or change them, as already proven by the Recruit state
fixtures. The adapter reads those stored bytes; it does not supply zero
when a payload is absent. Lists are still resolved even with those bytes
zero. An enabled path that reaches5827B0 returns an explicit missing
requirements/eligibility-service error after earlier action writes.
The surrounding action dispatcher is owned by the restore agent.

The ignored saved_recruit_actions unit test constructs SavedActions from
actual tutorial02 restored records and a real nonempty source list. It
supplies explicit normal-state/action-mask probe inputs, checks the actual
saved byte10/11 zero path, then enables byte11 and checks the5827B0 failure
and committed bit14 prefix. This is a local adapter boundary test, not a
full saved-world completion or production execution test. adapter-tests.log
records its result; source-adapter-findings.json pins files and evidence.

Reproduce source/adapter checks:
  python research/skirmish-20261016-recruit-actions/native_recruit_source.py
  $env:CARGO_TARGET_DIR='target/s-layouts'
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  cargo test -p aa-rules --release --locked --offline -j4 --test native_recruit_data -- --include-ignored --nocapture
  cargo test -p aa-runtime --release --locked --offline -j4 --lib saved_recruit_actions::tests -- --ignored --nocapture
Broad aa-rules/metagame compatibility runs are coordinated by root once
all shared definition-loading work freezes; they are not claimed here.
The prior shared exports were backed up under backups/ before editing.
