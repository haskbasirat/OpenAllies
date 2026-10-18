Load-time native spatial membership and same-index staging
========================================================

Scope
-----
NEW aa-runtime::load_spatial binds the actual held LoadShell,
RuntimeDefinition, generation-bearing identity registry, canonical source Bank
and BoundTerrain to existing native spatial membership. It does not complete
map Init, collision placement, actor callbacks, renderer setup or battle startup.

Native authority
----------------
4EE4C0 evaluates virtual+B0 then Thing.flags bit26. The actual admitted
Object/Actor/Kingdom vtables return true from+B0. Missing membership calls
4CF210 (blocking descriptor),4EA0F0 (soft descriptor),5536B0 and the concrete
54F6F0 native linked-list insertion. The original nested order is X then Y;
new nodes become the head and the original tail remains. Thing.flags bit26 is
stored after list insertions.4EE700 decides blocker updates afterward.

The actual stopped tutorial02 first Kingdom has flags A0C46900, has no Wall
component, and does not satisfy the blocker flag gate. Its admitted insertion
therefore requires only the native allocation seam. The present-bit
second4EE4C0 call is a native no-op. Native actual source shape getters are
shared across the admitted classes; native-vtable-audit.json separately records
original/v14 vtable bytes. It is static evidence, not an execution oracle.

Ownership
---------
During record Load, mutable flags/XYZ/facing/cached tile stay in the actual held
shell or current registry record. LoadSpatial permanently owns only the
generation/key map, native tile/node/reference cache, immutable canonical source
geometry and ordered binding metadata. A synchronous, callback-free membership
operation leases actual fields into the index, commits changed current flags
back even when the kernel returns an error, then drops every temporary member
field binding. A member pose read outside the lease explicitly fails. Persistent
reference_for_key resolves generation-bearing identity without needing pose.

The ordinary source geometry and flags use the actual Bank source FP precision;
the membership arithmetic has its own explicit live precision. Source identities,
canonical provenance/template chain, full actual category, final source flags
and component mask are checked. An absent source FP owner is not substituted
with restore/index precision. Invalid provenance, unknown class, stale generation
and invalid finite geometry are portable refusal boundaries, not claimed native
corrupt-pointer behavior.

SpatialWorld::adopt_loaded first owns the SAME index. It moves the actual staged
ActorStore placement into its existing reference/key identity once, attaches the
fields, and retains native lists/nodes without reinsertion. It returns an owning
world plus completed/pending/error status after partial transfer. A failed
transfer never discards earlier native cache effects. The successful-first,
failed-second transfer test uses an actual second staged Kingdom with explicitly
corrupt stale-generation metadata. It is a portable ownership failure probe,
not a native gameplay input.

Tests and reproduction
----------------------
Use the repository's scoped release target and no more than four cargo jobs.
The actual test requires original/v14 data and the existing verified capture03
receipts; AA_GAME_ROOT is E:\Games\Axis and Alies. Do not modify native files.

  $env:CARGO_TARGET_DIR='target/s-state-binding-review'
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  cargo test -p aa-runtime --lib load_spatial::tests --release --locked --offline -j4 -- --include-ignored
  python research/skirmish-20261016-load-spatial/native_load_spatial.py

The Python command defaults to immutable replay. --write is used only to create
the author's fixture before freezing. Its sole seams supply native8-byte list
node allocation/deletion; no complete membership or geometry body is mocked.
Original and v14 execute in separate fresh machine images for every row and PC.
Page-zero and unseamed allocator guards are inherited from phase8 harness.
All tile flags are compared by SHA256 over little-endian u32 words; every
nonempty list/primary reference and native head/tail consistency is checked.

AA_LOAD_SPATIAL_CAPTURE is an AUTHOR-ONLY absolute create-new output path for
actual capture generation; normal replay must leave it unset. Prior failed
writer/initial proof logs are retained and are not final source-PC validation.

Explicit gaps
-------------
Blocking Actors still need pre-membership collision/footprint/zone services.
No generic mutable-query lease is exposed: QueryPolicy callbacks may mutate
pose/flags or read outer shell state, so they require deliberate routing and
writeback beyond this callback-free membership scope. Derived virtual classes
outside2/3/4 remain unsupported. Saved payload decoding, complete immediate
Init/PostCreate/registration, renderer/UI callbacks, manager publication, world
start, battle tick and real multiplayer are separate work. Nothing here certifies
full map readiness, gameplay completion or an increased completion percentage.

Final validation:9 native rows (3 actual Kingdom and6 labelled class probes),
18 separate original/v14 machines,36 native refresh calls;308 address-byte pins
comprise307 executed addresses and1 intercepted malloc entry. Author focused
Rust3/3 passed20.23sec. Independent fresh native replay and Rust3/3 passed16.53sec;
review accepted. See independent-review/independent-review.json and findings.json.
Existing ordered membership/transform regression tests also passed3/3 after the
unrelated shared saved_hq module export finished (existing-index-tests-second.log).
