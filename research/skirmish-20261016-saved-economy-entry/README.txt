Saved Economy early-return bridge -- 2026-10-16

API/scope
---------
aa_runtime::saved_economy_refresh::refresh_member(&mut SavedActions, ActorId)
returns aa_combat::Result<()> and is crate-private. The restore_checkpoint
agent owns forwarding from AttachWorld::refresh_member. Research sources
remain frozen. Only this new runtime module, one appended lib export and new
tests/evidence were added; lib.rs was backed up first under backups/.

This is NOT a completed570020 resource refresh. It implements only:
 4CF0F0 reads Actor+84; absent Economy returns immediately at4CF0FF.
 Otherwise570020 reads actual Economy+4 component owner and dispatches the
 actor virtual+108. GActor4D1E80 returns its actual+E8 Kingdom pointer.
57004B branches to5701F6 return when that pointer is null. No component
 flags, resource vectors, source values or parent callbacks are read/written.
 Nonnull owner remains an explicit unbound error at the first required body
 boundary, before570051 reads further actor state. No fake Kingdom or zero
 resource arrays are created and no deferred success is reported.

The bridge checks actual stored Economy header.owner against the requested
actor before calling its owner getter. An identity mismatch is an explicit
Rust consistency error, not a claimed native malformed-pointer behavior.
Missing E8 remains an error; None component is the only absent-component case.
Wrong payload in the Economy slot also fails. All existing data stays owned
by the original SavedActions context/registry; no actor or component is removed
or cloned by production code. The helper signature can remain stable when
the real nonnull-owner resource service is implemented later.

Native evidence
---------------
native_entry.py independently loads original and acceptedv14 images, then
executes4CF0F0, actual570020 entry/return and actual4D1E80 getter. No getter
seam is used. The nonnull branch intentionally stops before570051 and is
named unported_nonnull_owner, not native failure or completed refresh.
36runs: absent/present Economy x owners0/17/FFFFFFFF x PC24/53/64 x2images.
Actual native field reads and unchanged actor/component byte snapshots are
recorded. The component contains initialized nonzero sentinel bytes to prove
the null-owner path leaves them unchanged. Every executed instruction is
checked against the other image. All image/function hashes are in JSON.

The prior full570020 oracle is reused and pinned:
 research/phase7-economy-refresh/native-refresh-fixtures.json
Its3 null_initial_owner rows (one perprecision) execute original570020 with
the declared virtual-owner seam, both flags set and nonzero resource vectors.
The new Rust regression compares every retained flag/vector and verifies the
old oracle's Kingdom outputs stay unchanged. The new leaf oracle replaces
that one getter seam with actual4D1E80 for this bounded scope.

Tests
-----
Three ordinary tests replay36entryrows and3priorrows, plus missing-E8 and
header-identity refusal and absent-component short circuit. The ignored test
loads actual tutorial02 mode1/PC53 under InspectPartial, constructs SavedActions
from real owned records and checks each restored actor's actual Economy/E8
entry path. It snapshots all component data and actor words and verifies no
mutation for both success and the explicit nonnull-owner stop. The census is
printed in tests.log. Additional explicit in-memory probe inputs check null,
missing E8, header mismatch and component absence through the real adapter.
No original map/save/profile is changed. InspectPartial is diagnostic; this
does not establish that the entire map can be restored or published.

Reproduce:
  $env:PYTHONPATH='E:\Games\Axis and Alies\reconstruction\tools\python-packages'
  python research/skirmish-20261016-saved-economy-entry/native_entry.py
  $env:CARGO_TARGET_DIR='target/s-research-actions'
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  cargo test -p aa-runtime --release --locked --offline -j4 --lib saved_economy_refresh::tests -- --include-ignored --nocapture
  python research/skirmish-20261016-saved-economy-entry/record_findings.py

Remaining real boundary
-----------------------
570020 nonnull owner still needs authoritative initialized Kingdom flow/
stockpile arrays and the existing native value helpers, removal/addition
arithmetic, ordered member caches and recursive parent refresh. Existing
economy_refresh.rs/economy_values.rs provide the algorithms, but saved
OrganizationAttachContext does not yet supply canonical aa_economy::Economy
ownership. Do not reset or manufacture flow arrays to bypass that boundary.
No full570020 integration, live economy, playable skirmish or multiplayer is
claimed; no artificial runtime limit is introduced or removed here.
