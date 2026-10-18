Saved Object Init(NULL) coordinator, 2026-10-16
=============================================

API
---
aa_actors::object_init_null::initialize(actor, precision, &mut world)
  -> actor_creation::Result<bool>

World is a mandatory-service trait. It has no success/default implementations.
The Actor Init(NULL) coordinator may call this entry from its first Object seam.
It does not accept an ActorParams/GACD block and cannot silently route NULL
placement through the fresh-creation 4F3640 path. Actor/Kingdom/current-loading
record storage ownership remains the runtime host's responsibility.

Native control and numeric scope
-------------------------------
4EBE50, 4EE700, 4EFA40(NULL), 4EF6A0(NULL)'s placement-result control and
4F0460's optional map-mode test are ported. Virtual +4, +C and +8C are mandatory
actual-class host dispatches. The fixture executes their untouched native leaf
bodies (4D1E40, 4EA010 and 4EA3D0) using the original Object A2A2A0, Actor A273F0
and Kingdom A299D8 vtables. Their +A8 differs for Kingdom; the coordinator never
replaces that dispatch with Actor or Team behavior.

Native order is preserved:
* Multiply current scale +3C by definition +24 under caller-selected x87 PC;
  clear only byte +4C, then store the binary32 product at +38.
* Capture ftol(tile-size)*0.25 as binary32 before 4EE700. After its flags gate,
  reread tile-size and dispatch actual radius; tile integer <= radius*4 accepts.
  Dispatch shape separately twice when required, including the second call's
  kind/radius fallback and min(extent10,extent14) branch.
* If flag16 is set, capture tile before actual shape dispatch and invoke the
  required 46EE00 snap service on the live position. No parameter writebacks.
* NULL reaches 4F4590(actor,0,0,0,0). Nonzero result first emits its diagnostic,
  then checks current optional map mode; only mode2 and codes1/2 produce the map
  warning and continue. Other nonzero results return false with prior stores.
* Invoke 4EE4C0, then reread flags. Bit21 selects terrain flags123; otherwise
  clear bit12 selects122. Store returned extended terrain height as binary32.
* Invoke full 4EB6E0 and 4EA150 services. Capture X/Y before reading tile shift.
  Truncate both, apply signed arithmetic shift masked to31, then store44 and48.
* Invoke full 4F1F70, then actual +8C using current loaded instance state. If true,
  invoke 4EB490 and, if false, 4EBB80. Finally invoke 4EBC70.

Raw word/definition reads must access initialized owned storage, without calling
external services. Service callbacks can mutate owned state. There are no stale
initial flags/position/instance snapshots across the native service boundaries.
Absent loaded +58 is an error, not an invented zero; the entry never clears it.
Errors preserve previous writes and propagate to the caller. They do not claim
native exception/unwind implementation or rollback semantics.

Mandatory host boundaries, not implemented here
---------------------------------------------
46EE00 snap (actual shape context, precision and live position);
4F4590 placement with zero optional arguments;
404EA0/404F60 and localized5A1040 notifications (including real strings);
4EE4C0 spatial membership (actual +B0 and descriptors, not Team's false virtual);
4F0E50 terrain sampling;
4EB6E0 blocking notifications and its own bit10 gate;
4EA150 footprint and tile ownership;
4F1F70 world-list insertion and its own bit8 gate/backlinks;
4EB490 current retention predicate, including actual +A8;
4EBB80 instance cleanup/SAI notifications/storage ownership;
4EBC70 visibility refresh, majors and instance creation/destruction.

These services cannot be replaced with success/no-op callbacks in a real runtime
binding. The test fixture supplies explicit deterministic probe services only to
measure caller order and stores. It is not a working spatial host or a claim of
completed per-record runtime Init, publication, skirmish or multiplayer.

Numeric domain
--------------
aa_numeric::Binary provides finite x87 arithmetic with explicit Single24,
Double53 or Extended64, nearest/ties-even and binary32 stores. Nonfinite inputs,
extended overflow/underflow outside its supported domain and ftol outside signed
i32 return an error. Masked native exceptional arithmetic (indefinite integers,
NaN/Inf/exception flags) is not claimed equivalent. No native timing/bandwidth,
memory or gameplay constant has been changed.

Evidence and validation
-----------------------
native_object_init_null.py separately maps the pinned original AA.exe and v14
AA.exe. It runs 306 identical cases per image (612 native executions total),
covering all three x87 PCs, and verifies every native instruction byte against
both images. The saved fixture records 544 distinct instruction addresses and
the actual vtable entries. Inputs include gate combinations, all placement reason
branches, absent/0/1/2 map modes, snap, both terrain modes, negative coordinates,
shift masking, changed positions before/after capture, changed flags after index,
two tile reads with different values, changed shape kind on the second dispatch,
signed instance counts, list mutations, retention and 18 service-error prefixes.

CString allocation/reference/literal/localization transport uses the pinned
phase8-conversion/native_machine.py harness. Diagnostics/format strings are not
asserted as a new Rust string-format implementation. The probe snap writes
specified coordinates; terrain returns specified binary64 extended values;
cleanup zeroes its explicitly owned fixture count/storage. Those effects are
test services, not native descendant proofs. Error probes interrupt inside the
service; they do not emulate a native C++ exception or unwind.

crates/aa-actors/tests/object_init_null.rs replays all306 native rows and compares
service ordering, output fields and every CPU store to object storage. Its second
test removes saved +58 and requires an explicit error after the native prefix,
with no fabricated visibility success. Both tests pass. Scoped Clippy completes
with two pre-existing warnings in recruit_actions.rs:77 and actor_creation.rs:716;
strict -D warnings remains blocked by those unrelated files. No new-module lint
warnings remain, and those other source files were not edited.

Replay without modifying the fixture, from rust-rebuild in PowerShell:
  $env:PYTHONPATH='../reconstruction/tools/python-packages'
  python research/skirmish-20261016-object-init-null/native_object_init_null.py
  cargo test -p aa-actors --test object_init_null --release --offline --locked --target-dir target/s-object-init-null -j4
  cargo clippy -p aa-actors --test object_init_null --release --offline --locked --target-dir target/s-object-init-null -j4

Use --write only for intentional author regeneration. Production/test/oracle are
frozen for independent review. findings.json pins their hashes and this README.
backups/aa-actors-lib.rs.before preserves the shared export file before the one
new export. No fresh-creation helper, parser, manifest, original game or release
artifact was modified by this task.
