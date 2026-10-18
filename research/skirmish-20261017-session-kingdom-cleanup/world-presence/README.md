# Bind765E00 to the current World, independently of Sim-null

Actual765E00 reads globalWorld B36534 before raw Player+28. It does not test
Sim+3C and does not require later VWorld replacement to have completed.
The original4F7D00 constructor publishes B36534 at4F7D2F, while the outer
76AA90 stores its returned pointer in Sim+3C at76AB1C. A partial constructor
can therefore leave positive globalWorld while Sim+3C remains constructor-null.

native_global_world.py records12 original/v14/all-PC reached instruction-slice
cases proving that interval and the later Sim alias store. Both selected PE
windows are actually installed and executed. This is explicitly not execution
of the full World constructor, a successful application startup, or proof of
global absence/destructor behavior. native-global-world.json is immutable;
generation uses create-new and default execution independently replays it.

The Rust fix keeps a nonowning sealed world_revision::Witness, minted only
from a real ConstructedWorldRevision. It contains weak allocation identity and
numeric metadata, never another World or a readiness boolean. Sim's native
pointer alias likewise retains weak identity. Validation requires the exact
live allocation and revision. The Sim alias cannot keep a destroyed World
alive; equal scalar revisions from another application do not match.

Actual BeginPlaying, film capture and film preamble derive this witness from
the current validated WorldFrameInputs owner. They still borrow the same Sim,
Session and players. The witness can cross the disjoint mutable World loan
without aliasing or copying the World. Checks occur at the reached native
World getter, preserving earlier profile-Find/name/ID operations and prefixes.
Raw Player+28 is read only in this admitted positive-current-World path. The
separate stronger SessionRead complete-view guards remain unchanged.

The older generic film_roster(sim)/create_sai766960 forms now refuse when they
reach a World getter without the actual witness. They do not equate Sim-null
with native global-null. Name/ID/AI getters remain available. Existing fresh
constructor-only diagnostics were corrected to expect the missing owner,
rather than claim global absence from a different pointer. Actual native
no-World cases remain in the native kernels' fixtures; this adapter does not
implement that branch until its genuine global owner is supplied.

This change does not publish an arbitrary old World as current: concrete
callers derive the witness from the current private frame immediately before
the validated sibling loan. It does not add a full global-World lifecycle or
permit old-World replacement. Native constructor timing, session phase, clock,
RNG, gameplay values and source rules are unchanged. Focused compilation/test
results and their precise shared-source qualifications are in the logs and
checkpoint; a compile-only result is not a playable-battle claim.

Focused runtime validation passed3/3, including the prior48-row native revision
replay,12-row publication interval and weak identity/foreign/same-value/stale/
replaced checks. The subsequent public Witness::is_for method was additionally
compiled and exercised against the ACTUAL source module by
check_world_witness.rs. That isolated research harness includes the unchanged
module directly, so it can test private portable constructor metadata without
exporting a production scalar-token constructor or linking unrelated runtime
unit modules. It does not construct a native World/frame or replace Sim tests.
The same/metadata-clone/foreign-same-value/replaced/destroyed assertions pass.
Full netcode regression passed49/49 before the later current-Text/UI-owner
extraction; final cohort testing will cover those additive changes separately.
