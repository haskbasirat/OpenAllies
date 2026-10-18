PHASE 6: ORIGINAL TERRAIN INPUT AND STATIC SIMULATION GRID

Production: crates/aa-mapworld/src/terrain.rs

The original TERRAIN chunk is GMap::Load557AD0 input. Its second u32 is the
constructed MapGTileResolutionBits, NOT a count of biome strings. All80 original
maps use resolution1, sourced independently from Game/SVars.tgi. The original
MAP_HEADER WorldCreator extents determine dimensions through5565E1..556668:
ceil(binary32 width/height), arithmetic shift by resolution, no extra corner.
Nonfinite, negative, zero-dimension and overflowing original allocation layouts
fail explicitly. This is original-format validation, not a global RAM/VRAM cap.

TerrainChunk::decode(&Chunk) validates identity, v6/v7, palette/count/record
extents and indices. Original maps contain48 v6 six-byte layouts and32 v7
eight-byte layouts, totaling2,960,384tiles. V6 conversion557C67..557C93 expands
the ID byte, flag byte, region u16 and byte2C into the native eight-byte record;
unused bytes cleared by the original conversion are still preserved separately
in serialized_tile_bytes. Source UTF16 IDs and raw source flags remain intact.

TerrainGridLayout::from_world_extents(extent_bits, evaluated_resolution)
TerrainChunk::bind(layout, &[Option<ResolvedVariation>])
TerrainChunk::bind_with_native_fallback(layout, palette, initial_variation)

ResolvedVariation is an explicit definition input: source ID, original native
terrain type, land/water bits and source identity. Neither type nor passability
is guessed from rendering or a name. None means a failed registry lookup. The
strict bind rejects a referenced missing definition. The native-fallback API
requires the authored biome default and follows557D4B..557E37: missing entries
carry the previous chosen variation across source-order cells, initially the
biome+78 terrain's first variation at+30. Missing source ordinals are reported.
Native localization of the first two error messages is outside this module.

BoundTerrain stores exact static flags20, byte2C, raw and effective palette
indices, saved region references and definition identities. Saved flags replace
the low byte; native terrain type overwrites bits4..6;552C50→54EE20 recomputes
the0x100 land /0x200 water blocking bits. Savedbit0 forces both. Dynamic object
membership is independent. Every original map's outer ring already hasbit0;
has_blocked_outer_ring verifies it without modifying any saved cell.

The source metadata provider is research/phase6-terrain/terrain-render-metadata
plus engine-baseline/terrain-variation-metadata. Original active translate.tgi
aliases and authored land/water defaults are pinned. A large historical block
in translate.tgi is commented out; those aliases are not reactivated. The source
catalog therefore causes463,557 carried-fallback cells across41 original maps,
including old developer maps. This is the observed source-catalog/native-loader
combination, not a claim that all missing historical IDs have been recovered.
Default biome/first variation order has separate native evidence in
research/phase6-terrain/native-defaults.json. Generic runtime data-registry
construction remains a required service; this component consumes typed inputs.

Native evidence:
- native_terrain.py / native-terrain.json:20 original layout fragments,
  32 original complete palette/tile-read loops,1048 passability inputs and8
  missing-definition sequences. The original557960 palette input loop executes.
  Allocation, CString storage, resolved registry lookup and biome reference
  transport are explicit seams. Stream game-mode59E520=false. Native missing
  diagnostics are intercepted after the original counter increments; chosen
  variation, branches, flag writes, cell order and counters execute unchanged.
- original_map_terrain.py / native-real-map-terrain.json: all80 raw layouts,
  source files, tile fields and static flag digests. Original constructor layout
  executes on every map. First9 unchanged source-order cells from every map run
  through557AD0 in a reduced3x3 fixture with the complete original palette and
  authored biome default:720 native sampled cells, not3million native executions.
- Five Rust tests cover the above, malformed/truncated input, undefined layouts,
  missing definitions and all80 actual maps/2,960,384cells. Strict Clippy passes.

Explicit remaining work:
- Original7AA9B0 auxiliary data and GMap+5C subtype loading follow suffix_offset.
  pending_auxiliary_bytes preserves these bytes. Water-cache recovery is being
  developed separately. This decoder does not publish them as runtime regions.
- Stream/game-mode-dependent region handle restoration54EA50, owning world
  registries, dynamic tile object lists and actor memberships are still required.
- No fake SearchWorld, movement collision callback, complete live map or playable
  match is supplied by this input adapter. The exact static tile data now feeds
  those separately recovered systems.
- Older terrain versions0..5 need their version-specific side effects; they are
  rejected explicitly rather than routed through the v6/v7 path.

Reproduce the source evidence and scoped tests:
  python research/phase6-spatial/record_terrain_evidence.py

Original game assets, binaries and saves remain read-only.
