Actor terrain refresh (native4D4130), 2026-10-16

Production: crates/aa-runtime/src/load_actor_terrain_refresh.rs
API: refresh(Catalog, actual held Record, actual BattleTerrain, live PC, Effects).
This is a concrete held-record binding, not a completed battle or Actor Init.

The current Actor's source identity, component mask and Loading owner are
checked by the existing Actor Init guard. Definition reads use that same
completed Bank's source PC. Runtime callbacks retain the separately supplied
live PC. Actual State lives in components.state, not a raw Actor+100 mirror.

Native early gates precede all terrain lookup. An active ready Element may
retain its actual parent F8; otherwise cached Object44/48 select the actual
BattleTerrain Variation whose TerrainData pointer is retained at Variation+1C.
This does not infer absent TerrainData or an empty Property list from zones.
Unique source-manager entries, source hashes and source PC are checked before
the canonical TerrainData identity is interned in the same definition Bank.

The actual held F8 is stored before any Organization579EB0 or Properties
effects. Equality returns immediately. Real Properties is component slot6,
Actor+78. Its absence is read from the real component arena. Old and new
TerrainData+B0 lists use reverse authored Property order: native4B8C8E..B9
prepends each resolved reference. Positive references resolve through the same
cached NativeLayoutData Property manager at their actual native load position;
this metadata does not pretend to be compiled criteria or a live instance.

Positive Organization refresh, Property removal581810, criteria5035E0,
admission/effects5817A0, and null-terrain diagnostics remain required callbacks.
If a diagnostic returns, the later native null+B0 dereference remains an
explicit failure. Earlier F8/callback effects survive every later Result error.
No dummy Organization, empty Properties instance arena, or permissive callback
was added. The default RequiredEffects refuses each reached unported effect.

Evidence (all original/v14, all three x87 precision controls):
- native-actor-terrain-refresh.json:186 paired cases/372 fresh executions,
  native4D4130,4D10A0,570560,4D12C0,4EA200,54ECE0.162 executed address pins and
  three stopped required-entry pins; no successful body interception.
- native-actor-terrain-null.json:6 paired/12 executions, actual F8=null store
  retained before required4D4211 diagnostic. This does not execute logging.
- native-terrain-property-source.json:12 paired rows/24 fresh machines,
  actual TerrainData constructor4B7870, count-zero load fragment, and positive
  head-prepend fragment.147 observed address pins; CString/allocator and source
  getter/reference inputs are explicitly supplied seams. This is not a complete
  TGI parser, Property compiler, or modifier/application proof.
- Fresh read-only replays preserve all three frozen JSON files.

Historical native refresh fixture key `weapon` and result label
`required_Weapon581810` are MISNAMED. They address actual Actor+78 Properties;
the actual bytes, address and production typed slot are correct. The frozen
fixture is retained rather than silently relabeling historical evidence.

Rust immutable replay:2/2 tests pass. One compares the exact gate prefix of
all186 refresh rows; it does not claim the entire fixture is a Rust source
manager replay. The actual-source test loads original Kursk plus verified
capture03 source completion, then uses real held allocations and components:
- GTID11632 eeurope_villageblock12x12_01: actual temperate_grass Property list is
  empty; the transition and subsequent equality fastpath complete at all PCs.
- GTID15283 herd_horse: actual temperate_crops F8 store survives required579EB0
  at all PCs. This is an animal Organization, not a military combat unit.
The test EXPLICITLY controls active gate bits and seeds cached cell44/48 from
the real saved pose because diagnostic Load omits Object Init. It does not
claim these are untouched full-ActorInit calls. Same allocation pointers are
retained; an unbound Catalog refuses before any further F8 write.

Frozen executable and exact source/evidence pins are in findings.json. Root
integration tests are separate; this leaf does not certify PostLoad, WorldStart,
an ordered simulation tick, combat, skirmish, multiplayer, or campaign.
