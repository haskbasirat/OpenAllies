Organization member creation: native57D950

Portable kernel: crates/aa-actors/src/organization_spawn.rs.
Exact test: crates/aa-actors/tests/native_organization_spawn.rs.

The original and v14 immutable executables each execute84 generated cases at
all3 x87 precision settings:252 paired cases,504 executions,159 observed code
pins (including5 declared seam entries, not their bodies). Each case checks all21 creation words, low-byte50/51 values, callback
order, early occupied/failed-creation branches and the CString refcount tail.
The same owner's GTID is changed by the virtual108 seam before native rereads
it, including a branch where that seam makes the current owner NULL.

Seams: CString manager and its nil string, virtual108,4F9C10,4CEA60,
57CBA0 and578040. The property-named input field in the frozen fixture is
actually Organization+34's recruit-item pointer: native4CEA60 is called on
the new member with that item, pairs it, and applies saved health/experience.
The initial decompiler-only 'remove property' reading was corrected against
the real push/ECX/callee instructions BEFORE this fixture or passing test.
The failed producer console output was not accepted evidence.

The two Rust tests pass:504 exact fixture replays and every ordered child
failure prefix. A callback error retains preceding current-owner mutations.
Fixture SHA256 de594cd6d7a17cfa8e6d020967610cd3aee15d3cd37c3df1096abaf505000ddb.

This proves the descriptor/coordinator slice, not the positive child bodies,
whole Organization runtime adapter, gameplay, or a completed battle. Rust's
owned empty string replaces the native CString ABI; the fixture checks its
logical emptiness and the native cleanup independently. No native address is
used as an actual game-side handle. The concrete same-owner adapter is next.

Replay:
python research/development10/organization-spawn/native_create_member.py --check
cargo test -p aa-actors --test native_organization_spawn --release --locked --offline
