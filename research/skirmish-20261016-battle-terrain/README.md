# Owned battle terrain

`aa_runtime::battle_terrain` loads the actual original map archive into two
independent native heightfield allocations and the native terrain grid. It
provides a borrowed `GroundSurface` for Object Init, placement and required-space
services. Converted viewer maps and renderer metadata do not supply gameplay
height, mountain noise or passability.

## Source and storage

`Source::bind(&mut V14Sources, &ActorCatalog, &Settings, Precision)` binds the
actual immutable source registry owner. Both resolution SVars require unique
authored integer values. Terrain, Variation and Biome definitions use native
registry identity and source order. Relevant authored Translate files are pinned
too. Every retained source file is reread through V14Sources and checked against
its provenance hash; its complete byte length and hash become SourceIdentity.
The current source has144 Terrain definitions,410 Variation definitions and8
Biomes at every precision control.

`Source::source_precision()` and `BattleTerrain::source_precision()` retain
the immutable precision used by the source producer. The loaded terrain carries
that receipt with its allocations. Object/Team callers compare it against the
completed definition Bank's source PC; a separate live arithmetic PC does not
replace it. This receipt addition changes no height, grid, alias or sample value.

TerrainData+4C land/water bits come from required typed fields. MOUNTAIN alone
requires both authored noise words; no missing mountain noise becomes zero.
Nonmountain Cell noise payloads are unused zero storage, not a mountain default.
VariationData+1C retains its actual Terrain owner. Biome+78 retains the first
authored Terrain pointer and Terrain+30 retains its first Variation pointer.
These retained pointers are not subsequently translated through the final map.

`BattleTerrain::load(&MapArchive, &Source)` reads MAP_HEADER, RULES, HEIGHT_MAP,
MOUNTAIN_HEIGHT_MAP and TERRAIN. Layouts use the authored native resolution and
world extents. Serialized resolution/count are checked witnesses. The existing
proved height binding preserves raw samples, flags, limit/default words and
chunk tails. The base/overlay arrays are independently owned. No fixed runtime
map byte or sample-count budget is added. Checked arithmetic, allocation and
unsupported data still return errors. A map rule overriding either resolution
SVar is refused because the native override policy is not bound by this API;
none of the80 original maps has such an override.

Terrain binding uses the existing native streaming fallback: an unresolved
palette entry retains the prior effective Variation, starting with the Biome
default. `surface()` borrows these owned arrays and the same grid.
`terrain_definition_at(x,y)` and `passability_bits_at(x,y)` use private retained
per-cell Variation identities; they do not derive TerrainData+4C from flags20.
They explicitly reject an out-of-range cell or missing source owner.

## A corrected earlier reference

Native48D500 performs one translation, then direct4915F0 registry lookup. The
older renderer metadata recursively followed translation chains. The authored
chain `tropical_river03 -> tropical_river02 -> tropical_river01` therefore does
not resolve03 in native code when only01 is registered. The new runtime follows
the original instructions. It does not force the old reference's alias result.
Compared with that earlier reference, four maps have additional carried cells:

| Map | Old fallback cells | Native fallback cells | flags20 changed |
| --- | ---: | ---: | --- |
| DeveloperMaps/(4FFA)TropicalCruise.RMP |25938|26256|yes|
| DeveloperMaps/cutscene_short_02.rmp |34881|35127|yes|
| TrailerMaps/allies02_guadalcanal.RMP |17551|17633|no|
| TrailerMaps/axis12_hawaii.RMP |45451|45950|yes|

The other76 maps match the earlier static terrain flags/fallback reference.
All80 raw map hashes and160 raw height chunk hashes still match its pinned
original-data corpus. No original asset was edited.

## Evidence

The focused ignored Rust test independently enumerates all80 original .rmp
members through V14Sources, and loads each at Single24, Double53 and Extended64:
240 map loads. It verifies actual source identity, native lookup results, static
terrain binding, separate layer ownership, exact passability/type source access
and1968 fractional ground samples, including mountain and water where present.

- `native_source_find.py`:3702 actual lookup rows,1080 executed instruction
  pins. Original native manager construction, registry insertion, translation,
  hashing and48D500 lookup execute. The two PEs are separately loaded through
  the actual base Machine loader; a differing loaded section byte is checked
  for each of the six image/precision executions. Authored canonical and alias
  facts are explicit inputs; this is not a full native TGI loader proof.
- `native_source_fields.py`:912 rows and42 pins, independently executing
  original/v14 scalar passability/mountain-noise stores and retained first
  Terrain/Variation pointers at all three controls. Typed source getter results
  are explicit seams. Full texture/material/TGI loading is outside this scope.
- `native_battle_terrain.py`:3936 ground samples and383 instruction pins,
  separately executing original/v144F0E50 with real550190/550210 height samplers
  at all three controls. Height arrays come directly from hash-checked raw map
  chunks. No height result or mountain-noise answer is substituted. Actual
  source-bound neighboring cells are inputs; unrelated cells remain inaccessible
  null pointers. The fixture allocation extent is only an oracle harness limit.
- Existing aa-heightfield native_binding/native_fixtures/original_maps tests
  replay constructor, lossless binding, precision-sensitive native sampling and
  both height layers across all80 originals. Existing aa-mapworld terrain tests
  replay layout, palette carry, flags and required errors.

The native scripts replay immutably by default. `--write` is an explicit fixture
regeneration command. Rust capture uses `AA_BATTLE_TERRAIN_CAPTURE=1`; normal
verification requires that variable to be absent and reads frozen native rows.

```powershell
$env:CARGO_TARGET_DIR='target/s-owner-recruitment'
$env:AA_GAME_ROOT='E:\Games\Axis and Alies'
cargo test -p aa-runtime --lib battle_terrain::tests:: --release --locked --offline -j4 -- --include-ignored --nocapture
python research/skirmish-20261016-battle-terrain/native_source_find.py
python research/skirmish-20261016-battle-terrain/native_source_fields.py
python research/skirmish-20261016-battle-terrain/native_battle_terrain.py
cargo test -p aa-heightfield --test native_binding --test native_fixtures --test original_maps --release --locked --offline -j4 -- --include-ignored --nocapture
cargo test -p aa-mapworld --test terrain --release --locked --offline -j4
```

This is an owned ground-surface dependency, not a complete World startup,
Actor Init, dynamic terrain editing, pathfinding, battle, save migration,
multiplayer or gameplay-parity completion claim. Those callers must retain this
same terrain owner and execute their own required services in native order.
