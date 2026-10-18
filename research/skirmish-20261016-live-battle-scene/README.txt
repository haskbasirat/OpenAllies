Live native-world presentation bridge (bounded scope; not a playable-battle claim)
================================================================================

Production files
----------------
crates/aa-client/src/live_battle_scene.rs binds the actual staged ActorStore and
fully adopted managed World/SpatialWorld to the existing LiveScene update path.
It reads current authoritative pose, facing, scale, source/variant and the required
Presentation provider every collection. It does not simulate, manufacture units,
advance animation, or derive live transforms from extracted viewer placements.

BattleScene::from_staged rejects a World before the original load-time spatial
index has been completely adopted. It checks exact retained GTIDs, generation,
canonical proof, native storage class and rendered model/picking identity. Every
collect retains the same original ActorStore and SpatialWorld via opaque Weak
identity witnesses; their private Arc controlblocks prevent allocator-address
reuse after drop. Immutable SourceGeometry Arcs are retained and compared with
Arc::ptr_eq. No mutable placement/flag/pose owner is copied. Variant/source
changes require explicit model rebinding. Every apply also rechecks the sealed
rendered-map hash, collects/validates the entire snapshot, then delegates to the
existing transactional GPU/picking update API.

Rendered-map correspondence
---------------------------
rendered_map_source.rs checks the current raw RMP SHA256 against the recovered
extraction manifest, and compares six raw decoded chunks to both manifest hashes
and extracted chunk bytes. It checks actual native header/grid extents, tile IDs,
used palette identities, complete indexed GTIDs/class/name/span/version/base-word
and position/facing prefixes. Model, material and texture conversion are outside
this receipt. It is not certification that every native object has a model or
that renderer terrain arithmetic/pitch/roll/animation is bit-exact.

The strict check exposed a real old converter defect: reconstruction/tools/
map_catalog.py line68 round(x,5) discarded bits from HEIGHT_MAP and
MOUNTAIN_HEIGHT_MAP before JSON serialization. bind_exact replaces only the
in-memory PreparedScene sample arrays with the verified native raw binary32
values and rebuilds the actual PreparedTerrain before rendering. It validates
all other input correspondence before publishing either replacement. Original
RMP, extracted files, converted JSON, saves, profiles and release builds are not
modified. A missing/invalid receipt leaves inspection preview available with an
explicit warning and refuses native-battle binding. Model previews have no RMP
receipt and cannot bind as a battle map. NativeConditional terrain still uses the
existing approximate renderer calculation; exact sample input is the claim here.

Concrete visibility and replacement-color leaves
------------------------------------------------
live_battle_visibility.rs reads the actual class2/3/4/6/7/9 Object/Actor allocations,
Object+54 and current Kingdom+190/+228. It binds 4EEF70 and class+70/+74, with
Organization members in current native order and Element recursion. Null versus
missing owners remain distinct. CurrentView is mandatory and has no all-visible,
current-Kingdom, fog, session, thread or editor default. Positive Actor+C8 means
Flare slot26 (not Transport+C0); its 5733A0 is explicitly unbound. Cyclic parent
references require native reentrant support instead of silently skipping them.

NativeColors loads actual completed KingdomColor RGB words+18/+1C/+20 and manager
order from the retained canonical source registry. Each requested Kingdom color
rereads its actual mutable +1E8 reference. No translated-name relookup, clamping,
white inspector color or unverified constructor seed is substituted. Source PC
comes from the actual definition Bank receipt, distinct from a live operation PC.

Native evidence
---------------
native_visibility.py / native-visibility.json: 408 rows, 816 fresh original/v14
executions across x87 PC24/53/64, 247 executed instruction pins plus actual
class vtable pins. 4EEF70,4EA1D0,4D3B80,4D3C50,54EB40,54EB70,4364C0 execute.
Only OS thread identity is seamed. The other globals are explicit probe inputs;
this is not proof of their application startup producers. Includes lazy view
reads, null/nonmajor/shared masks, dead flag, thread/editor gates, Organization
null/duplicate slots and Element traversal. Full returns check stack/nonvolatile
registers and x87 balance. Immutable replay command: python native_visibility.py.

native_color_load.py / native-color-load.json: 21 rows, 42 fresh original/v14
executions across all PCs, 41 executed pins. Unchanged successful47E0F0 stores
all3 RGB words. Localized-name lookup/assignment and typed GetPoint3 are explicit
seams; inherited name is empty. Includes actual blue/red and arbitrary binary32
payloads. Full TGI parser, default color seed and render routing are not claimed.
Immutable replay command: python native_color_load.py.

Rust verification
-----------------
Scoped command (original local assets required):
  AA_GAME_ROOT=E:\Games\Axis and Alies
  CARGO_TARGET_DIR=target/s-client-dev09
  cargo test -p aa-client live_scene::battle --release --locked --offline -j4 -- --include-ignored

Tests distinguish native synthetic getter fixtures from actual-source tests.
The real source/model test uses original v14 source, real ActorStore constructors,
actual published SpatialWorld, recovered model and CPU picking binding. It reads
committed current pose/facing/scale, rejects foreign owners with repeated IDs,
and proves collection itself does not publish picking changes. Its explicit
Appearance probe is TEST ONLY; this does not claim whole-map managed startup.
The rendered-map test requires tutorial02, tests the old rounding refusal, exact
sample installation and actual terrain mesh rebuild, foreign map hashes and
in-memory stale sample/placement rejection. Other tests replay native visibility,
mutate actual shared-mask owners, compare real source RGB with47E0F0, and exercise
move/drop/replacement lifetime witnesses.

Remaining required integration
------------------------------
No main.rs battle hook was added. The actual managed map still must complete
startup/adoption. A real CurrentView implementation needs current player Kingdom
and its VWorldState, real fog variable, calling-thread identity, SessionE4 and
editor owners. LocalBootstrap765E10 replacement exists but its real BeginPlaying
caller must supply the current Kingdom; constructor-null is not a view default.
Complete remembered-model selection4EAB20, definition4EACC0, owner4EAD70 and
positive Flare5733A0 remain required before an automatic faithful Presentation
can replace the trait boundary. Merely using current visibility would incorrectly
hide memory ghosts, so there is no automatic all-visible/current-visible provider.
Actual managed-World-to-GPU frame execution and a controllable battle have not
been tested or claimed by this bounded change.

Final result: tests-05.log, six scoped Rust tests PASSED (6/6), including
original asset tests. Native immutable replays passed separately. All commands
reaped. Earlier failed logs are retained as diagnostics, not counted as passes.
