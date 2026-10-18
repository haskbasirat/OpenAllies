Live saved-object placement dependency slice (2026-10-16)

Production
  aa-spatial::creation_placement is the original GWorld4F3640 coordinator,
  distinct from movement4F3C40. aa-runtime::live_placement binds the saved
  NULL-input 4F4590 path to the actual borrowed LoadShell and Catalog source.
  Neither module publishes an ActorStore object or uses viewer placements.
  Native executables, saves, source assets, release folders and SDK files were
  not modified. Only new Rust modules/tests and module exports were added.

What is concretely implemented
  Current shell definition/pose/flags/owner reads, canonical source identity,
  native source flags/mask/shape reads, GameData+44 expansion arithmetic,
  constructor/copy/snap/outside placement operations, current ZoneMap lookup,
  ordered tile geometry and the 4F3640 branch/error-code coordinator.
  Source virtual+30/+38 EAX predicates and +2C/+3C byte predicates retain the
  original call/read order. Descriptor +2C is hostility context; +34 is custom
  filter. Argument6 sets ignore_all byte+30, not ignore_moving byte+31.
  A late current-definition+38 read is mandatory even when a last actor exists.

  candidate_tiles executes mode0 geometry using the actual full-cell shape.
  occupancy_tiles executes mode1 point sampling using an explicitly supplied
  global B3A8C4 shape. It does not construct a default global or prove that
  global's startup producer/lifetime. Returned coordinates are local geometry,
  never copied runtime tile/zone storage. Cells retain native row-major order
  and the forced center-cell rule.

Source precision and ownership
  Source values use the immutable Bank combat-source precision and full source
  identity receipt. Placement arithmetic independently uses the caller's PC.
  The current source was loaded at PC53; all three placement PCs are checked.
  A source-field arena at the placement PC instead of the actual source PC is
  rejected. Held RuntimeDefinition flags and component mask must equal their
  canonical source values; public runtime fields cannot assert unrelated words.
  Source-field tokens retain canonical registration provenance; the adapter
  verifies their owner and complete writer universe. Missing arena is an error.
  Global settings distinguish Unbound, actual Null and Present(mode). Unbound
  fails at the reached native read; it is not recast as a null/local setting.
  Reached virtual+B4 validates complete current shell/source proof and actual
  constructor/storage: class3/4 requires Actor+Object, class2/9 Object alone.
  Six malformed-shell regressions cover wrong kind/missing storage/provenance.

  Source final-flag verification is deliberately narrow: random source kinds,
  Placeholder replacement flags and non-Actor ownable repair require their
  completed postpass storage and fail explicitly when reached. This module
  does not claim all flags/postpasses were completed. Source requirements in
  the subdirectory are a separate author-owned guarded dependency slice.

Native proofs
  All fixture rows execute separate original and v14 machines at PC24/53/64;
  executed bytes are checked against both images. Page-zero access is guarded
  except actual FS:[0], x87 stack balance is checked, and original files stay
  read-only. Native allocator/string seams are explicit, not native heap runs.

  native_creation_placement.py / native-creation-placement.json:
    363 rows, 726 paired executions, 734 instruction pins. Untouched4F3640,
    actual553090/46E0A0/552250/553370; controlled source/child/iterator services
    exercise branch results and service failure prefixes. Four preparatory
    original runs enumerate reached service stops in addition to these rows.
    This proves coordinator ordering, not the supplied child implementations.

  native_placement_tiles.py / native-placement-tiles.json:
    432 rows, 864 executions, 1282 pins. Actual554DD0/554330/5541F0 plus native
    shape primitives for both modes. Global point-shape bytes are explicit
    probe inputs, not claimed as actual startup ownership.

  native_creation_shape.py / native-creation-shape.json:
    72 rows, 144 executions, 128 pins. Actual49BD10 source expansion branch
    with explicit GameData+44 inputs and binary32/x87 store ordering.

  native_actual_placement.py / native-actual-placement.json:
    9 captured contexts, 18 executions, 397 pins. Actual saved first Kingdom
    of tutorial02, Firefight and Berlin through full original4F4590->4F3640,
    all PCs, actual source/object vtables and getters, current owner word/pose.
    Native string allocation is an explicitly unique empty-name service;
    actual release/refcount body executes and final references are zero.
    All five child services are forbidden and confirmed unreachable. Result0,
    unchanged defined GACD words/current source and shell buffers, balanced
    x87 and correct native return stack are checked. These are isolated actual
    placement compositions; earlier ObjectInit/lifecycle is not inferred.

Rust validation
  creation_placement_native: two tests replay all363 coordinator and432 tile
    rows (both tile modes).
  live_placement: source expansion replays72 rows; the ignored original-asset
    test borrows actual first Kingdom records in all three maps and checks all
    PCs, missing arena, wrong source-PC arena and forged held flags/masks.
    It confirms no child callback is reached and actual shell state survives.
  AA_GAME_ROOT is required for the original-asset test. AA_WRITE_CAPTURE=1 is
    an explicit author capture mode; normal runs compare the retained capture.
  Logs retain exact commands/results. aa-spatial scoped strict Clippy passes;
    aa-runtime scoped Clippy retains pre-existing warnings outside these files.

Remaining world/battle gaps
  Required4F1660, zone-actor4F0AE0 positive-path world services, structure4F2220,
  collision4F0A70 policy/index lease and footprint4F0B70 remain mandatory
  reached services. The separate required_placement/live_required_placement
  author owns the real4F1660 child; this adapter does not claim its integration.
  Positive4F0AE0 uses current shell/ZoneMap and requires completed Bank mean
  radius, but an actual positive-zone runtime case is not yet in this fixture.
  prepare_collision/collision helpers bind existing native query code and
  preserve the post-query+38 read. A safe pre-staging query lease over current
  shells plus mutating policy callbacks is still required; no empty query or
  stale index snapshot is supplied to hide it.
  Ground/visibility membership, global point producer and full per-object
  Init/PostCreate/registration sequencing are owned by other work. This slice
  is not a playable battle or a whole-map startup completion claim.
  Nonfinite numeric inputs, overflowing binary32 temporary results, native
  heap failures, SEH/unwind parity and arbitrary reentrant host callbacks are
  outside the bounded finite arithmetic/callback domain of these fixtures.

Reproduction from rust-rebuild
  python research/skirmish-20261016-live-placement/native_creation_placement.py
  python research/skirmish-20261016-live-placement/native_placement_tiles.py
  python research/skirmish-20261016-live-placement/native_creation_shape.py
  python research/skirmish-20261016-live-placement/native_actual_placement.py
  Each command defaults to read-only fixture comparison; --write regenerates.
  cargo test -p aa-spatial --test creation_placement_native --release --locked
    --offline -j4 --target-dir target/s-borrowed-leader
  Set AA_GAME_ROOT=E:\Games\Axis and Alies, then cargo test -p aa-runtime
    --test live_placement --release --locked --offline -j4
    --target-dir target/s-borrowed-leader -- --include-ignored
