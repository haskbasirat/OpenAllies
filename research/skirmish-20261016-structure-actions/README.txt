Structure command availability (589A60), 2026-10-16

Scope
Ports the Structure contributor and its ordinary GActor virtual predicates.
This does not implement queued command execution or Settlement simulation.
No ActorData field is read by these functions, so no aa-rules provider was
added. Existing original/v14 sources and Development08 remain unchanged.

Native order and storage
589A60 calls virtual+158 (4CE980 -> 588990), then virtual+15C (4CE9A0),
which repeats +158 and rereads Structure+10. 588990 compares the parent's
Settlement+14 with the queried Structure's header owner, not a guessed ID.
For a nonnull center parent, 4D1350(parent) completes synchronously. Then
589A60 clears owner action bit13 and ORs the parent's current full mask.
The merge may restore bit13. Afterward bit18 reflects Structure+18 != 0.
The component header owner is reread after the callback and between writes;
the queue read uses stable component storage, including callback changes.
Parent callback failure retains its already-applied side effects. A later
queue dependency error retains the completed clear/merge prefix.
An absent parent Settlement is a native null dereference, exposed as an
explicit Rust dependency error. Null/other Settlement center returns false.
No RNG, floating-point arithmetic, new engine cap or work reordering occurs.

Runtime integration API
  crate::saved_structure_actions::contribute(&mut SavedActions, actor)
Implements the pure World on the existing SavedActions owner. It reads the
actual restored Structure, actor action words and queue Vec, and writes only
action words through the established setter. Parent rebuild uses flags::rebuild.
No cloned actor/component state or successful fallback callback is used.
Forwarding in saved_organization_actions.rs belongs to the runtime integrator.

Existing saved 58A770 discards the serialized Structure+10 reference and
sets it to null; later runtime parent references must be typed actor handles.
Its queue loop appends only while the signed saved count is positive; the
accepted empty queue path is read from actual storage, not assumed by this
adapter. Nonempty queue item codecs/execution are still unimplemented.
The shared component payload enum currently has no concrete Settlement.
Any present unavailable Settlement payload remains an explicit error.

Proof and test results
native_structure_actions.py independently loads original AA.exe and v14,
and executes 589A60, 4CE980, 588990 and 4CE9A0. Only parent 4D1350 is a
gameplay seam; its mutations and injected dependency failures are explicit
fixture inputs. Such an injected failure is not claimed as a native exception.
The fixture has 498 cases: 83 x PC24/53/64 x original/v14. It covers absent
components, null/different centers, raw queue counts including FFFFFFFF,
owner/queue/parent/action mutations, callback failure and ordered full-mask
writes. The native harness guards page-zero access. All four native function
windows and both complete image hashes are recorded in the fixture.
Default oracle replay compared the existing fixture successfully; --write
is the explicit regeneration option.

2 pure Rust tests pass, replaying all498 cases and a missing-queue prefix.
1 actual-source adapter test passes on DeveloperMaps/(3v3)Firefight.RMP,
after a successful StopOnError load. It uses a real owned restored Structure,
checks its parent/queue, allocation identity and action preservation, then
checks the explicit missing-Settlement boundary. An unused Organization
context is explicitly constructed only to satisfy SavedActions' attach
context; no Organization query is reached or claimed by this test.

Commands (release, --locked --offline -j4, target/s-structure-actions)
  cargo test -p aa-actors --test native_structure_actions
  AA_GAME_ROOT=<game root> cargo test -p aa-runtime --lib saved_structure_actions::tests -- --ignored --nocapture
  PYTHONPATH=<game root>/reconstruction/tools/python-packages python research/skirmish-20261016-structure-actions/native_structure_actions.py
Full commands and results: pure-tests.log, adapter-tests.log, native-replay.log.
No broad restoration census, complete match or release test is claimed.

Preservation
Backup before edits: backups/source-before-structure-actions-20261016.zip.
Changes are two new production modules, their exports, one pure test and
this evidence folder. No source parser, Research contributor, original asset,
save/profile, manifest/dependency or release file was changed by this work.
