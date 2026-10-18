Saved Organization member state leaf 4D1080
==========================================

Changed only crates/aa-mapworld/src/organization_restore.rs: private
restored_member_flag_4d1080 helper, OrganizationWorld callback, two tests.
The file was backed up before modification. Combat-value forwarding and all
other callbacks were left untouched. In particular4D64F0 owner-state mutation
still returns its explicit unported error.

Native4D1080..4D1097 reads Actor+104, masks4, and returns EAX0 or1. No writes,
source definition lookup, MemberFacts computation, modifier, Body, or component
payload access is part of this leaf. The implementation reads only the actual
ActorShell.components.state.high. The loading owner comes directly from its
active shell; other members use IdentityRegistry.get(full GTID) followed by
the restored Actor variant and shell. Missing, stale, loading, non-Actor and
missing-shell states are explicit errors, never false defaults.

Prior source proof is retained in phase8-combat/native_morale.py,
native-morale-fixtures.json and asm/004d1080.asm.txt. This task additionally
executes the full real leaf directly on both original/v14 images at all three
precision modes, using37 high-word patterns (all single bits and mixed words).
All222 rows check the low state word is irrelevant and actor bytes are unchanged.
The native generator uses no call or arithmetic seams and pins both images,
the complete function range, generator and shared harness.

The new private tests replay each native row against both actual owner shell
and registered member shell, without populating unrelated component payloads.
They also supply contradictory unused words/owner state, exercise same-slot
wrong-generation IDs, missing/null IDs, Loading, wrong record variant, missing
Actor shell and removed records. Both tests passed:
  CARGO_TARGET_DIR=target/s-borrowed-combat-value
  cargo test -p aa-mapworld --lib member_flag_4d1080 --release --locked --offline -j4

This is one exact read-only restore leaf. It is not a completed Organization
restore, Init(NULL), battle or runtime release. Root owns the actual Tutorial02
continuation and next-blocker test. No original game/save/release was changed.
