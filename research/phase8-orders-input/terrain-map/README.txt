TERRAINMAP (DAT_00B36518) CONSTRUCTORS FOR 0x71D840: RUST PORT AND NATIVE ORACLE (2026-10-19)

Code
  crates/aa-runtime/src/player_order_input/terrain_map.rs        (port, impl terrain_ray::Terrain)
  crates/aa-runtime/src/player_order_input/terrain_map_tests.rs  (replays every oracle row)
  crates/aa-runtime/src/player_order_input/mod.rs                (+1 line: pub mod terrain_map;)
Oracle
  native_terrain_map.py      producer (research/phase8-ai/native_machine.py, both pinned images,
                             m.verify_v14(), PC24/PC53/PC64)
  native-terrain-map.json    57 rows (19 per precision), 795 operations with full map/block
                             state after each, 1440 native 0x71D840 rays (480 per precision)
  generate01.log = replay01.log (byte-identical replay)

STATUS
  The Rust TerrainMap is built with the original constructor arithmetic and replays every row
  with no mismatch at PC24, PC53 and PC64:
    - construction: every map field and every block field 71D840 reads (16 OK rows per
      precision), plus 3 rows per precision where the native code asserts (855D30 lines
      0x95C/0x960, 0x95F) or raises #DE (0x72425C). Rust returns an error naming the first
      native assertion line or the #DE site.
    - 265 operations per precision (load 720AD0, release 721030, 725340 recompute,
      7208D0 water rebuild, 71FFA0 invalidate, lab sample/tile rewrites). After each one,
      the whole map/block state is compared, including the loaded block's stride^2 vertex words.
    - end to end: native-constructed map + native 71D840 vs Rust-constructed map + Rust
      intersect_71d840, 480/480 rays per precision (302 AL=1, 321 with secondary pointer),
      comparing AL, primary words, secondary words and the ordered store list.
  Test command (13 passed, 0 failed: the 4 terrain_map tests plus the existing
  player_order_input tests; no warnings in terrain_map*.rs; per precision: constructions 16,
  construction errors 3, operations 265, rays 480):
    CARGO_TARGET_DIR=target/agent-orders-terrainmap cargo test -p aa-runtime --lib --release
      --locked --offline -j 6 player_order_input

NATIVE ADDRESSES (image base 0x400000)
  43FD80..43FE26  43F0A0 builds VWorld+BC (431A90/431BC0), then 721330(VWorld+BC,
          [[GWorld+30]+24], min(UVar0, AVar0), AVar0), publishes B36518 (43FE21), calls 723E00.
  721330  +04/+08/+0C up (0,0,1), +10..+18 X normal (1,0,0), +1C..+24 Y normal (0,1,0);
          +6C heightfield, +70 config, +8C/+90 = heightfield +20/+24 (authored limits),
          +A0 = block arg, +AC = quad arg, +A8 0, +98/+9C holders 0, +94/+BC/+C0 bytes 0.
          No later store to +04..+24 (static scan of 723E00 and the terrain module, oracle).
  723E00  7241CE/7241E9 +74/+78 = _ftol(CRT ceil(World+3C/+40)) (96D4C0 SSE2 path, 96CC4C);
          724205 +B0 = _ftol([GWorld+30]+8) (GMap gtile size); 72421C +A0 = AVar0
          TerrainBlockSize; 72422B +AC = UVar0 TerrainQuadSize (raw, not the 43FD0F clamp);
          72423C +B4 = +B0 / +AC (#DE if +AC is 0); 72424B assertion 0x95B (stale +A8 0 % +B4;
          #DE at 72425C if +B4 is 0); 0x95C width/height % quad; 0x95D all > 0;
          0x95E width,height >= block >= quad; 0x95F block % quad; 0x960 width/height % block;
          724402..724455 +7C/+80 = width/height / quad, +A8 = block/quad (quads per block),
          +84/+88 = width/height / block, +A4 = quads+1 (stride), +A0 = quads*quad;
          724558 cell array (by*bx*4); 724657 +8C/+90 = FLT_MAX / -FLT_MAX (the 721330 limits
          are discarded); 72464B..7246A9 row-major 723D30(map, x, y) -> 723960;
          7246AB..7252AF Ni scene tail (stores only map +34..+68); 7252AF +BC = 1.
  723D30  "Dynamic/Terrain x%d y%d" resource: operator new(0x150), 723960, 5ACEC0 register.
  723960  +40 owner, +44 0, +80/+84 = FLT_MAX/-FLT_MAX, 736E20 water holder at +8C
          (+90 0, +98 1, +9C byte 0), +AC/+B0 index, +C8 = map+A4, +CC = map+A8,
          +D0 = map+A8/map+B4, +D4 = _ftol(fild map+AC), +14E..+14F = 1, 71AFD0 bounding
          sphere (not read by 71D840), 71B8F0.
  71B8F0  block min/max over the stride^2 samples [[+40]+6C]+04[cols*(j+row*q)+k+col*q]
          (j outer). min is replaced unless min < h, and max unless max > h, so ties take the
          later sample's bits.
  720AD0  block mesh load: +80/+84 reset, 720130 pooled NiTriShape (stride^2 vertices),
          71B610 vertices: x0/y0 = fstp(fild(index*map+A0)), x/y advance by fstp(fadd(heightfield
          +10 step)) at the run's PC, z = raw 4316A0 sample word, with the same min/max fold, then
          71A690.
  71A690  map fold: the min holder (+9C) is cleared when its min rose above +8C, and the max
          holder (+98) when its max fell below +90; +8C/+90 take strict improvements (+94 = 1);
          either holder null -> +C0 = 1.
  71A750  recompute: +8C/+90 = +-FLT_MAX, every cell in order, strict improvements move the
          holders (they are not cleared first). 725340 runs it at 725429 when +C0 is set, then
          sets +C0 = 0.
  721030  release: +44 = 0 (mesh back to 720330's pool). Map extremes/holders unchanged.
  7208D0  per visible (state 2) block each frame; at 720A93, while +14F: +14F = 0, 737470
          clears water, 738280 sets +9C if any gtile (gx,gy) of the block (g = map+A8/map+B4)
          or one of its 8 in-bounds non-null neighbours (B322E0 offsets, AB7804 order) has
          GMap class [[tile+0]+1C]+1C == 2; the centre tile is read unchecked; 7377F0 builds
          the water geometry.
  71FFA0  sets +14F when a tile change involves water.

BLOCK LOADING ANSWER (+44 mesh, +BC loaded byte)
  +BC is written once, synchronously, at the end of 723E00 (7252AF) during map publication
  (43FE26); 71F750 clears it at teardown (71FA9C). Meshes are lazy and depend on the camera:
  43EC60 (VWorld update) -> 4312F0 -> 430E50 rasterizes the camera-frustum footprint over the
  block grid (430710; callbacks 430080 state 2, 4300D0 state 1, test 42F1F0) into the block
  +BC request; 42FF50 applies it (720080: +B8 = +BC), queues changes (42F980), then 42FE10:
  new state 2 -> Request 721180 + Load 7212D0 -> 720AD0 when +44 is null (synchronous, main
  thread); leaving state 2 -> 71CD00 + Release 7211F0 -> 721030 when the resource count
  (+4) reaches zero. Then 7208D0 runs for every state-2 block (water). 725340 follows at
  43ED15 (71A750 when +C0). So the 71D840 answer depends on render state: which blocks have a
  mesh (71B9B0 triangle walk vs 71CD30 flat fallback) and the map +8C/+90 extremes. Those
  extremes are the 71A690 fold over blocks loaded so far, not over the whole map, and they
  set the top/bottom planes and therefore the walked segment. This follows the camera
  history, not World data. It is not ported.
  The Rust port therefore starts every block as Residency::Unreported. has_mesh() is an
  error for such a block, and the map min_height()/max_height() are errors while any block
  is unreported. The live owner (a port of the visibility pass, or the renderer's own
  frustum culling once it is proven equal to 430E50/430710) must report, per frame and in
  42FE10 queue order:
    load_block(world, height, x, y, pc)   new state 2 (720AD0 if absent + 7208D0 water)
    release_block(x, y)                    left state 2 and the resource count reached 0
    report_absent(x, y)                    never in state 2 since 723E00
    update_725340()                        once per VWorld update after the pass
  and invalidate_water_71ffa0 / rebuild_water when GMap tile classes change.

PUBLIC API (terrain_map.rs)
  TerrainMap::for_world(world: &live_world_frame::World, height: &view_heightfield::State,
      grid: &view_grid_variables::State, quad: impl FnOnce() -> Result<i32, String>)
      -> Result<TerrainMap, String>
    Borrows: World +3C/+40 (presentation_parts().creator_extents), the GMap gtile size
    (presentation_parts().terrain.grid.cell_size, i.e. [GWorld+30]+8), the SAME completed
    VWorld+BC heightfield (samples_for_world; a SampleWitness and World witness are retained,
    and later loads must supply the same allocation), AVar0 TerrainBlockSize
    (grid.terrain_block_size()) and UVar0 TerrainQuadSize (quad, e.g. aa-ui
    UiVars::terrain_quad_size_current, read after AVar0 as at 72422B).
  impl terrain_ray::Terrain for TerrainMap (Block: terrain_ray::TerrainBlock), so 59A6E0/43C4B0
    become intersect_71d840(&map, eye, dir, None, 0, pc)?.apply_to(point).
  load_block / release_block / report_absent / update_725340 / invalidate_water_71ffa0 /
  rebuild_water / recompute_pending / blocks_xy / block_at(x, y).residency().
  The query is a pure function of &TerrainMap.

REMAINING EXPLICIT ERRORS (and who must supply the data)
  - Unreported block residency: has_mesh, and map +8C/+90 while any block is unreported.
    Supplier: the camera visibility owner (4312F0/430E50/430710/42FF50/42FE10, not ported).
  - Water triangles of a block whose +9C is 1 (water_index_count/triangle/vertex). Supplier:
    a port of 7377F0 (TERRAIN-cache water geometry per block); aa-client water_geometry.rs
    decodes the same cache but has no per-block u16 index list proof.
  - 738280 centre tile outside the GMap allocation or a null centre variation (native
    out-of-bounds read / null dereference).
  - 723E00 assertions 0x95B..0x960 (the 855D30 handler is not ported), #DE at 72423C, 72425C
    and the other idiv sites, nonfinite/out-of-range ceil/_ftol, a cell array whose size
    wraps 32 bits.
  - Heightfield sample indices outside the allocation (71B8F0 / 4316A0) and nonfinite
    samples (x87 NaN quieting/unordered folds and infinities are outside the proved domain).
  - 71CFC0 (vertex refresh after a 431390 heightfield update) is not ported. view_heightfield
    has no update path, so the samples never change after 431BC0.
  Not kept (not read by 71D840): map +7C/+80/+B0, block +D0, the +90 water holder list, the
  71AFD0 bounding sphere, mesh normals/colours/UVs/indices, +104..+14E.

ORACLE METHOD AND SEAMS
  Ran natively: 721330, 723E00 (to 7246AB, then 7252AF), 723D30, 723960, 736E20, 71AB60,
  71AFD0, 71B8F0, 40D770, 96D4C0 (SSE2 path: B4B340 = 1, MXCSR 0x1F80), 96CC4C, 720AD0
  (to 720C79), 71B610, 4316A0, 42D750, 71A690, 71A750, 721030, 737470, 738280, 738200,
  737310/737500/45D8A0/4849F0/96BE33/736DD0 (water bookkeeping), 71D840 and all its children.
  Seams (Python, recorded call counts in the JSON): operator new/delete, eh vector ctor
  96D18D (element ctors 4DD140/71AAF0/736E00 modelled), sub-object ctors 72B160/72B040/
  728DF0/86B240/86CCC0/867690, 4B6AA0 / 4B2E60 / 405370 var lookups, 72EC90 zone manager
  (64x64 table read at 723CB0), 40E970/59AB60/59ABA0/59AAD0 profiling, strings 410CB0/
  40F9D0/401B00/856370/405D60, 5A77E0 textures, 5A63D0/709B50/71AE00/43B5B0, 72D320, 5ACEC0,
  5AA890, 5AA270, 4F0E50 (71AFD0 heights), 720130 mesh pool, 8704C0/89F5F0/43C3B0/71FD20/
  71FB20/6FF7D0/86E8D0/6FF850/8677A0/86F090/720330, 7377F0, 855D30 (records the line).
  Skips by explicit seam: 723E00 7246AB..7252AF and 720AD0 after 720C79. The static scan
  shows that only map +34..+68 / block +124..+14E are stored there, that the map pointer
  does not escape, and that no B36518 user appears in either direct-call closure.
  Lab inputs (explicit, not mission values): 16 specs, including retail 16x16 (block 8,
  quad 2, gtile 2), non-square, block 12, quad 1/4, ceil extents, signed zeros/ties,
  full-mantissa floats, fractional and 3.3e6 steps (PC-dependent 71B610 accumulation),
  flat, recompute-tail (71A750 remainder loop), sample rewrites that clear holders (+C0),
  water GMaps (neighbour-only water, null and out-of-bounds neighbours, wrapped centre rows,
  repaint + 71FFA0 + rebuild) and the 3 assertion/#DE specs.

COVERAGE (executed / instructions, all 3 precisions together)
  71B8F0 61/61, 71B610 205/205, 71A690 45/45, 71A750 120/121 (alignment lea), 723960 229/252,
  723D30 60/65, 721330 327/354, 723E00 prefix 580/679, 720AD0..720C79 117/125, 736E20 9/9,
  71AB60 30/30, 71AFD0 287/287, 721030 52/113, 737470 32/32, 738280 251/278, 738200 50/53,
  4316A0 109/127, 96D4C0 39/63, 96CC4C 14/14 (5045 executed pins, identical in both images).
  The unexecuted lines are smart-pointer/string release paths that the seams make unreachable,
  allocation-failure paths, 4316A0 normal clamps, and ceil branches for values that are
  negative, below 1, or 2^52 and above.

REPLAY
  python research/phase8-orders-input/terrain-map/native_terrain_map.py   (byte-identical)
  --write refuses to overwrite; --dry prints coverage.
  sha256 native-terrain-map.json 1bf78e7015adefa91e46686b90b0e0ea96a89627dd1caa1297a0243c1106452c
         native_terrain_map.py   de7dfa540f5136f3207de63e11bc6fa50d173d798b36202aba844287248ff9c1
         native_machine.py       1aba789e7e72dc2d016623caa22456181c03fcedc3f40dc57bff7fb7bde68c00
         generate01.log = replay01.log fd9aa033dbcd4830a89b6a8e80a9557d5027f4e201f1bfcfd7bd7ff73e53f5e7

NOT DONE / UNTESTED
  for_world/load_block against a live World are compiled but not run (no live World in unit
  tests). The production lab-free path is the same code as the replayed internal functions.
  No mutation suite was run. Nothing wires the map into event::Services::terrain_71d840 yet
  (battle_controller/tgi_frontend owners).
