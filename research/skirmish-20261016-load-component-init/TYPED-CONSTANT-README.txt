Actual constant component InitExisting on held saved allocations
================================================================

Scope
-----
load_constant_component_init admits Character2, Structure13, Research21,
Transport24, City29 and HQ30 only through their actual typed restored states.
The source Bank supplies definition-load FP precision and provenance; live PC
is separate and these constant leaves perform no arithmetic. The held Loading
registry identity, actual Actor constructor storage, proof/category/native
flags/source component mask and allocation's owner are checked before dispatch.
There is no duplicate state owner and no component is removed or cloned.

Each +8 is native6A71D0 (mov al,1; ret). Five +14 entries are4B2830 (ret).
City+14 is563FE0, which requires source5B4 and owned zone8 effects; after_init
explicitly fails at that service boundary. It does not turn a source radius
into an unowned zone or a mocked success. Projectile1 and Portal28 also have
native constant +8 entries, but this runtime has no actual typed saved owner
for them, so they remain unsupported. Other component states remain outside
this module, even when they share a constant AfterInit leaf.

Native proof
------------
native_typed_constant_init.py executes actual vtable entries separately in
original/v14 at all3 PCs:18 rows,36 fresh machines,66 leaf calls,3 executed
address-byte pins and12 vtable-entry pins. City563FE0 is pinned as its actual
vtable value but is NOT executed and no success is claimed. There are no
service seams, component field reads/writes or allocations. Opaque component
bytes are poisoned/unread; they are not represented as a loaded native record.

Actual saved-state evidence
---------------------------
Actual map census: Character10 (firstGTID15284 horse), Structure35 andCity5
(firstGTID11542 eastern_europe_city_center02_small). Research/Transport/HQ
are absent from this map and have no actual saved runtime call claimed.

The focused ignored test loads the actual Kursk diagnostic using verified
source capture03 and real source completion, selects the first actual held
allocation for each admitted slot present in that map, and calls both hooks
at all3 live PCs. Actual absent components are recorded as absent rather than
constructed. It checks the identical Box allocation and full shell retained,
plus missing source Bank and false source-mask refusal. The diagnostic is not
a successful whole-record Init or a playable battle.

Reproduce
---------
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  $env:CARGO_TARGET_DIR='target/s-state-binding-review'
  cargo test -p aa-runtime --lib load_constant_component_init::tests --release --locked --offline -j4 -- --include-ignored
  python research/skirmish-20261016-load-component-init/native_typed_constant_init.py

Leave AA_TYPED_CONSTANT_CAPTURE unset for immutable replay. The author-only
variable takes an absolute create-new destination. Native --write generates
before freeze; normal invocation compares the immutable fixture.

Validation/freeze hashes: typed-constant-findings.json (when capture completes).
