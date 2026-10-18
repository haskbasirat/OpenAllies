Owner Economy dispatch integration (2026-10-16)

Scope
  NEW crates/aa-runtime/src/saved_owner_economy.rs exposes
  refresh(&Catalog, OrganizationAttachContext) -> Result<(),String>.
  It constructs the existing SavedActions on the actual owner, with
  active_mind=None, then calls saved_economy_refresh::refresh_member(owner).
  Existing570020 refresh semantics, resource/source readers, borrowed Kingdom
  flow callbacks, and allocation ownership are unchanged. No substitute
  Economy, flow reset, vector resize, rollback or initialized-world claim.

Native4CF0F0
  4CF0F0 reads Actor+84, tests the Economy pointer and returns when absent.
  When present it jumps570020 with ECX equal to the actual Economy pointer.
  The new Python oracle executes the unchanged thunk on separately mapped
  original and accepted v14 at PC24/53/64:9rows,18nativeexecutions,26visitedpins.
  Absent Economy performs no owner lookup. Present Economy with null owner
  executes actual570020 and its actual SEH-return sequence, with explicit
  owner virtual+108 result0. A separate present case stops at the required
 570020 entry; that row is deliberately not reported as refresh success.
  Native component bytes remain unchanged in all nine rows. Instruction bytes
  visited (including the replaced entry's pin) match across both executables.
  Native service seams: owner virtual+108, and explicit dependency-stop entry.
  No native nonnull Kingdom flow subgraph is newly certified by this thunk test.

Real saved caller test
  crates/aa-runtime/tests/saved_owner_economy.rs reloads actual tutorial02
  records with real source actors/resources/settings, fresh managed inactive
  scope, explicit constructor clock0 and InspectPartial. This is a diagnostic,
  not complete immediate Init/PostCreate or published battle startup.
  Selects an actual Organization owner WITHOUT Economy. Temporarily removes
  that same actor's E8 word, then invokes the new wrapper at all three PCs.
  It must return without reading E8, without inserting Economy or changing the
  current Organization, and without changing any other real Economy payload
  or its allocation address. The original E8 and Organization are restored.
  No Kingdom flow vectors are initialized or fabricated by this test.

Rejected test assumption retained
  The initial tests.log and initial-test-rejected-assumption.rs.txt record an
  overly broad test selector that expected an actual Organization+Economy with
  nonnull owner in tutorial02. No such actor was found. This was a test
  assumption failure; the production wrapper was unchanged. The final focused
  test uses a real absent-Economy Organization rather than inventing one.

Reused existing present-Economy evidence (not new counts)
  research/skirmish-20261016-saved-economy-integration/README.txt and its pinned
  findings/native-natural-hq-refresh.json/native-initialized-refresh.json
  cover the exact unchanged saved_economy_refresh coordinator being invoked.
  That earlier diagnostic observed29 actual Economies:27 reached required
  missing Kingdom1B8[0]; two linked HQ gated refreshes completed and reached
  their absent-Economy parent4CF0F0. The earlier explicitly initialized fixture
  is separate from natural loading; production does not call its reset.
  Existing present/null/missing-owner, identity, same-allocation, partial flow
  failure and nested-parent tests remain the authority for that implementation.
  This tiny owner adapter does not enlarge their proven numeric domain.

Remaining limits
  Real per-record Kingdom Init/flow setup, Actor Init, PostCreate, publication,
  active world services, complete battle/skirmish and multiplayer remain
  required work. Their current explicit errors are preserved. Original game
  files, saves, profiles, dependency manifests and releases were not changed.
  No SDK implementation was consulted or integrated.

Reproduce (PowerShell, from rust-rebuild)
  python research/skirmish-20261016-owner-economy-refresh/native_owner_economy.py
  $env:AA_GAME_ROOT='E:/Games/Axis and Alies'
  $env:CARGO_TARGET_DIR='target/s-borrowed-leader'
  cargo test -p aa-runtime --test saved_owner_economy --release --locked --offline -j4 -- --include-ignored --nocapture
  Normal Python invocation replays/asserts the immutable fixture. --write is
  reserved for deliberate regeneration by the evidence owner.

Final observed result
  tests-second.log passes1actual-data integration test /3PCcalls. Actual owner
 4099 is regiment_usa_hq_infantry_unpacked; both restored Organization owners
 4099 and8130 have no Economy. All29other actual Economy payloads and their
  allocation addresses are retained. All child commands have completed.
