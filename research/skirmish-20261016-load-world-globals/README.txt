Native startup globals for the first real Object::Init boundary, 2026-10-16

Delivered ownership and scope
-----------------------------
aa-runtime::load_world_globals::PointShape owns the six-word B3A8C4 shape
returned by native4E3E90. Its only constructor is
from_verified_startup_reference(&Catalog,&VerifiedStartupReceipt). It checks
the sealed point reference, the private source-owner guard, logical source
fingerprint, verified source/FP reference and current numeric target. It then
constructs from the six observed receipt words. There is no raw, Default,
Deserialize or zero-fallback constructor. validate_catalog must be called at
the reached shape-read boundary; it rejects another source target or numeric
settings changed after construction. Object/Zone authors call it lazily.

This is the native PE zero-fill STARTUP SEED carried through observed native
initialization and retained for the known ported load path. It is not a claim
that arbitrary later native code cannot mutate the global. New services that
could mutate that source must be proved before adding them to the admitted
path. Neither startup capture enters a map, Object::Init or a battle.

SessionPhase reads the actual retained KKC_Session, not a copied phase. The
aa-netcode::lobby::Session implementation reads field_f8 (+F8) and state (+E4)
on each call. The additive application_state method has an explicit unbound
error for phase-only hosts. KKC_Session is [B36550]; KKC_Sim timeline B36554 and
MapResource+A8 are different objects. Native76AC90 reads Session+F8. Native
54EF10 compares Session+E4 before World+48. Tentative76AC60 was checked and
rejected as an E4 getter: it reads [B36558]+8, and is not used here.

Native76F890 constructor has XOR EBX at76F8C7, stores +E4 at76FA53 and +F8 at
76FAE1. ChangeToState77A750 stores E4 at77A806 and F8 at77A81C from actual
launch state (kind6 takes its+18; other kinds take kind). Existing staging
ports own those state transitions. Its launch calls host.load BEFORE the
final ChangeToState, so the first map cannot infer F8 from final launch kind.
Root must retain and pass the actual session from the actual preceding path;
these getters do not fabricate that session or certify its whole constructor.

Native producer observations
----------------------------
capture_startup_globals.py is an independent copy of the frozen successful
capture03 debugger. Neither capture03 nor its script was changed. Both new
QA processes use separate copied runtimes, independent read-only RWDs and
verified Personal5 folder redirection before source_entry. All54 copied
runtime support files match source after each run. Native CV words, means,
FP controls/status and numeric settings match frozen capture03 exactly.

The PE static audit places all24 shape bytes in the .data zero-fill tail.
This static fact alone was not accepted. New Windows captures instrument all
created threads with four aligned4-byte WRITE data watches:

* capture-01: words0,1,2,3;62 threads armed; no native writes observed.
* capture-02: words2,3,4,5;62 threads armed; no native writes observed.

All six words were observed zero at process creation, source entry/return,
Session constructor phase store and constructor return. Session is actually
NULL through source return, then its real constructor stores F8=0. A null
Session is recorded as absent, never substituted with a phase value.

The real-observation cutoff is sealed BEFORE instrumentation calibration.
After that cutoff, all other threads are suspended and four same-value
isolated MOV [ESI+F8],EBX probes exercise every armed hardware slot. All eight
probes trap the expected slot and leave all six words unchanged. They execute
the exact pinned native instruction with redirected integer registers; they
are tracer probes, NOT native game producers. No next instruction executes
after the final pending debug stop; the owned process is terminated and drained.

SetContext uses CONTROL only for software stepping, DEBUG_REGISTERS only for
watches, and INTEGER|CONTROL only for isolated probes. No FP/SSE state is
written. Both captures have normal EXIT_PROCESS/wait0/exit0 cleanup receipts,
zero cleanup errors, unchanged original archive hashes and no owned process
left in the post-process inventory. The copied base retains CreateProcess's
two original handles until Python exits (owned duplicates are explicitly
closed); that limited handle lifetime does not outlive either script process.
Setup duplication/identity checks precede its main try/finally, an inherited
limitation; no successful receipt is accepted without normal final cleanup.

Windows APIs and processor-breakpoint details were checked against Microsoft:
https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-wow64_context
https://learn.microsoft.com/en-us/windows/win32/api/wow64apiset/nf-wow64apiset-wow64getthreadcontext
https://learn.microsoft.com/en-us/windows/win32/api/wow64apiset/nf-wow64apiset-wow64setthreadcontext
https://learn.microsoft.com/en-us/windows-hardware/drivers/debugger/processor-breakpoints---ba-breakpoints-
https://learn.microsoft.com/en-us/windows/win32/api/debugapi/nf-debugapi-continuedebugevent

Native executable evidence
--------------------------
native_globals_proof.py pins original/v14 Session constructor, transition,
phase getter, application predicate and point getter bytes. It executes936
phase/application-prefix rows,234 isolated constructor-store rows and3 point
getter rows at PC24/53/64. There are no decision hooks or dummy World+48/node
inputs: the application prefix stops before either can be read. Constructor
EBX=0 is an explicit isolated seam independently established by the actual
constructor instructions and the Windows trace, not a whole-constructor port.
Page0 is unmapped in the machine. Native read-only execution changes no asset.

Validation and reproducibility
------------------------------
Runtime tests check the sealed complete word coverage, explicit E4 failure,
actual source-bound point construction and rejection of stale destination
settings/foreign source owners. Netcode tests read actual Session fields
against every native getter row and existing staging fixtures. See final logs
and findings.json for actual results; earlier compilation with the pre-guard
module is preserved separately and is not a successful test claim.

Commands from rust-rebuild:
$env:PYTHONPATH='E:/Games/Axis and Alies/reconstruction/tools/python-packages'
python -B research/skirmish-20261016-load-world-globals/native_globals_proof.py
python -B research/skirmish-20261016-load-world-globals/seal_startup_reference.py
$env:AA_GAME_ROOT='E:/Games/Axis and Alies'
$env:CARGO_TARGET_DIR='target/s-detection-mask'
cargo test -p aa-runtime --test load_world_globals --release --locked --offline -j4 --config profile.release.lto=false --config profile.release.codegen-units=16 -- --include-ignored
cargo test -p aa-netcode --test session_owner_globals --test staging --release --locked --offline -j4 --config profile.release.lto=false --config profile.release.codegen-units=16 -- --include-ignored

The existing workspace serde_json dependency was promoted to production in
aa-runtime's manifest by root for the sealed receipt decoder. No new version
or lock update was needed. Tests use the stated release profile overrides;
the final executable's default release build belongs to root's integration.
No original executable, asset, profile, release package or old capture changed.
