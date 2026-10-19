TERRAIN VISIBILITY PASS 0x4312F0 (BLOCK RESIDENCY OWNER): RUST PORT AND NATIVE ORACLE (2026-10-19)

Code
  crates/aa-runtime/src/terrain_visibility.rs         (port; lib.rs: pub mod terrain_visibility;)
  crates/aa-runtime/src/terrain_visibility_tests.rs   (replays every oracle frame and ray)
  crates/aa-runtime/src/player_order_input/terrain_map.rs   (+71AFD0 block sphere, +14E/+14D flags,
                                                       crate hooks for this owner)
  crates/aa-runtime/src/player_order_input/terrain_ray.rs   (minimal: Plane and plane_42ed60 made
                                                       pub(crate) for 42EED0's eight 42ED60 casts)
Oracle
  native_terrain_visibility.py     producer (native_machine.py, both pinned images, verify_v14, PC24/53/64;
                                   reuses ../terrain-map/native_terrain_map.py's Lab and seams)
  native-terrain-visibility.json   9 rows (3 maps x 3 precisions), 90 frames, 270 end-to-end rays
  generate01.log = replay01.log    (byte-identical replay)

STATUS
  Every frame replays with no mismatch at PC24, PC53 and PC64. Per precision: 30 frames, 30 rays,
  1179 resource requests/releases in total over all rows.
  Per frame the test compares:
    - every block's +BC request, +B8 state, +B4 previous state, +140 active flag, resource +4
      count, +44 mesh presence, +9C water byte, +14F and +14E;
    - the active-list order;
    - the eight base-height hit points vis+28..+84;
    - the VWorld+144..+170 footprint (four 71D840 casts);
    - the map extremes, both after the pass and after the 725340 +C0 branch;
    - the exact 42FAD0/42F670 view-cell hook call sequence.
  At the end, the native 71D840 on the natively built memory matches the Rust 71D840 on the
  Rust-built map, with residency driven by this owner: 90/90 rays per precision (264/270 AL=1).
  The 71AFD0 spheres of every block also match: 64+24+9 blocks per precision, laboratory 4F0E50
  heights.
  Test command (CARGO_TARGET_DIR=target/agent-orders-terrainmap cargo test -p aa-runtime --lib
  --release --locked --offline -j 6 -- player_order_input terrain_visibility): 20 passed, 1 failed.
  All terrain_map, terrain_ray and terrain_visibility tests pass, with 823 view-cell hook calls per
  precision. The failure is player_order_input::states::right_click_matches_native_rows: another
  developer's in-progress states.rs/states_tests.rs, modified at 11:13-11:17, after the earlier
  13/13 run.
  Footprint: in the first frame of each case the map extremes are still +-FLT_MAX. There the Rust
  71D840 refuses the binary32 overflow, and the test checks that the native wrote an infinity
  (3 frames per precision). The other 27 frames per precision match word for word.

NATIVE ADDRESSES PORTED
  43EC60  VWorld update order: 50C750, 43A400, 4318E0 (heightfield update, not ported),
          4312F0 at 43ED0A (this owner), 725340 at 43ED15 (TerrainMap::update_725340), then
          the VWorld+208 request segment and the +E0 list (ported elsewhere).
  430290  VWorld+94 ctor (lists, planes); TerrainVisibility::for_map pairs it with the map
          built by 723E00: no 42FE10 pass has run, so every block +44 is reported null.
  4312F0  42EED0(*[B40048]); 430E50; if TerrainMap+BC: 42FF50; jmp 42F670.
  42EED0  camera +90 eye, +114 direction and +19C..+1F8 six planes copied to vis+88/+94/+A0.
          While +BC is set, 71D840 runs on rays +2EC/+2D4/+2E0/+2F8 into VWorld+144/+150/+15C/+168
          (43B2F0). 869C90 builds up-normal planes at the base heightfield [GWorld+34]+48 and +4C,
          and 42ED60 (t<0 accepted) stores the hits at vis+58/+64/+70/+7C (min) and vis+28/+34/+40/+4C
          (max). A parallel ray leaves the old point.
  430E50  6 points max2EC, min2EC, min2D4, min2E0, min2F8, max2F8. x,y are scaled by
          fstp(1/fstp(fild +A0)); z (unused) by the register reciprocal for the first point.
          Min/max chains (ties take the later operand), CRT floor 96D5E0 / ceil 96D4C0, then _ftol.
          Bounds [floor-1, ceil+1]. 430130 copies the polygon; 430710 runs with callbacks
          (430080, vis, 0, 0, 42F1F0, vis, 430080, vis, 4300D0, vis).
  430710  The grid has (H+1)(W+1) entries but is indexed with row stride W = |x1-x0|; this
          native quirk is kept. Pass 1, per row: 42F330 against every polygon edge gives tmin/tmax
          and their crossing x. With at least 2 hits, the span floor(xmin)..ceil(xmax) gets 430080.
          A cell with an unmarked cell above is sphere-tested; if it passes, 430080 runs on the cell
          again and the cells above it are tested and marked. If it fails, the cell gets 4300D0 and
          its grid entry is cleared. The left neighbour gets the same treatment (no repeated mark
          on the cell itself). Pass 2, bottom-right first: the same growth downward and to the right.
  42F330  segment crossing: dx and the edge dx are stored binary32, the denominator is stored and
          tested against 0, and s,u are stored and must lie in [0,1]; the point is a + s(b-a).
  42F1F0  in grid and 42ED10 not outside any of the 6 planes, using the block's 71AFD0 sphere.
          42ED10 counts a sphere as outside when (nz*cz + ny*cy) + nx*cx - k <= -r (ordered).
  430080 / 4300D0  in-grid -> 42FE80(vis, block, 2 / 1); 71A230 stores +BC = 1|2, and a block
          that is not yet active is pushed at the front of vis+4 (+140 = 1).
  42FF50  for each active block in list order: 42FAD0 (view-cell hook), then 720080, which on a
          change sets +B4 = +B8, +B8 = +BC and queues the block (42F980: state 2 is appended through
          42F7D0, other states are pushed in front); +BC = 0. A block whose +B8 is 0 is unlinked
          (42FED0, +140 = 0). Then 42FE10 and 42FCD0. Then 7208D0 for every active block.
  42FE10  queue order. State 2: Request vt+10 721180 (720670 when +88 is null; 5AA3F0(1,4):
          count+1), Load vt+18 7212D0 (720AD0 when +44 is null), 71CCB0. States 0/1 with +B4 == 2:
          71CD00, Release vt+14 7211F0 (5AA550: count-1; 721030 when the count reaches 0 or below).
  7208D0  state-2 blocks: the 720A93 water rebuild while +14F (737470 + 738280). Its other
          children (71B500, 7203F0, 71C290, 71CE70, 4325C0, 71F3B0, 71C470) write no field that
          71D840 or this pass reads (two-level static store scan).
  725340  only the +C0 branch matters for 71D840 (71A750); its other children store no
          71D840/terrain-block field (two-level static store scan).
  71AFD0  (TerrainMap, at 723960) block sphere. The four corners (index+{0,1})*fstp(fild +A0)
          each get a 4F0E50 height at flags 0x122 and 0x123; every result except the last 0x123
          is stored binary32. The sums and differences follow the exact x87 store/register pattern.
          Maxima: ties take the second operand, and one compare uses the register value. The
          radius is sqrt at PC.

LIVE API
  TerrainVisibility::for_map(&mut TerrainMap)        right after TerrainMap::for_world (723E00)
  CameraWords::current(&native_camera::directory::Directory<K,R>)  (or CameraWords::read(&State))
  vis.update_4312f0(&mut map, &World, &view_heightfield::State, &CameraWords, &mut impl ViewCellHooks, pc)
      then map.update_725340()  (43ED15)
  trait ViewCellHooks { sweep_42fad0(index, block_size_a0, state_b8); traverse_42f670() }
  Read views: block(cell) -> BlockVisibility, active(), footprint(), hits().
  TerrainMap::for_world now takes `pc` (71AFD0 arithmetic and the 4F0E50 GroundSurface samples
  at flags 0x122/0x123).

SPLIT WITH THE VIEW-CELL OWNER (coordinator note)
  This module: 4312F0 dispatch, 42EED0, 430E50, 430710 and its callbacks 430080/4300D0/42F1F0,
  42FE80/71A230, and the 42FF50 dispatcher with the block half of it (720080, 42F980, 42FED0,
  42FE10 load/release, 7208D0 water).
  View module (through ViewCellHooks): 42FAD0 per active block, called in list order right before
  that block's 720080 with the block's (+AC,+B0), TerrainMap+A0 and pre-apply +B8; and 42F670 as
  the tail of every 4312F0, also while TerrainMap+BC is clear. Their state (vis+0..+24 VGrid from
  42F540, vis+100..+10C lists) is theirs. The oracle records both seam call sequences and the test
  checks the order.

RESOURCE REQUEST PATH
  Resources::request_0c / release_14 cannot be used here. Terrain blocks are dynamic resources
  ("Dynamic/Terrain x%d y%d", registered by 723D30 -> 5ACEC0), and the Rust ResourceManager has
  no such Kind (resource_objects::Kind is Texture/Model/Sequence/Sound only; Handle::new needs a
  catalog SourceFile).
  So this module keeps the 5AA3F0/5AA550 count +4 itself: +1 on entering state 2 (mode 4), -1 on
  leaving it, and 721030 runs when it reaches <= 0. 42FE10 is the only requester found. The
  5B0270/5AEE70 manager queue/unload calls are not modelled.
  Registering blocks in the ResourceManager would need a resource-owner change; it was not made.

INTEGRATION (crates/aa-client/src/battle_presentation.rs; owners: camera dev; view dev for the
43EC60 queue segment in battle_view_startup.rs / view_directory.rs)
  Construction: right after the visual heightfield and TerrainMap publication (43FE0F/43FE26):
    let mut map = TerrainMap::for_world(world, height, grid, || ui.uvars.terrain_quad_size_current(), Precision::Double53)?;
    let mut vis = TerrainVisibility::for_map(&mut map)?;
  Per UI frame, in the submit_activated job, while vworld_ad, IMMEDIATELY BEFORE
  startup.update_view_requests_43ec60(...) (native 43EC60 order: ...4312F0 at 43ED0A, 725340 at
  43ED15, then the VWorld+208 segment):
    let (camera, _) = loans.application_views.camera_and_visual_map();
    let words = CameraWords::current(camera.directory())?;
    vis.update_4312f0(&mut map, loans.session.world(), <VWorld+BC heightfield>, &words, &mut <view-cell hooks>, Precision::Double53)?;
    map.update_725340()?;
  The selection/order input earlier in the same job (native: WndProc before the next 43EC60) then
  sees the previous frame's residency, as natively. Pass Live.terrain = Some(&mut TerrainRay(&map))
  and Live.terrain_ready = (map +BC == 1, true after for_world) to battle_orders_input.
  50C750, 43A400 and 4318E0 (visual heightfield update) are not ported. Until 4318E0 exists,
  view_heightfield is immutable and blocks never get +14E/+14D.

REMAINING EXPLICIT ERRORS
  - 430E50 on a hit point never written (a parallel corner ray in every past frame).
  - Nonfinite or out-of-range floor/ceil/_ftol bounds, and a 430710 grid size that overflows.
  - 720080 +14E vertex refresh (71CFC0) and +14D recompute: set only by 71CC00/73D7F0, which are
    not ported (unreachable in Rust).
  - Footprint 71D840 errors are stored in footprint() and do not abort the pass. Native stores
    infinities there.
  - Water triangles (7377F0). 7377F0 builds (x, y, 0) vertices and u16 triangles from the
    7371B0 body polygon of the block's water gtiles, then registers 73E300(736D50)/73BA00/736AB0
    effects, which are possibly per-frame animation and are unverified. Not ported, so a
    +9C = 1 block query stays an error. The oracle uses land/mountain-only GMaps.

COVERAGE (executed / instructions, all precisions)
  4312F0 18/18, 42EED0 258/258, 430E50 357/357, 430710 524/565, 42F330 93/93, 42F1F0 48/48,
  42ED10 28/28, 869C90 25/25, 430080 24/24, 4300D0 24/24, 42FE80 28/28, 71A230 8/8,
  42FF50 80/80, 42F980 28/28, 42F7D0 36/36, 42FED0 46/48, 42FE10 45/45, 720080 27/48 (the
  unreachable +14E/+14D branches), 721180 28/39, 7212D0 18/30, 7211F0 55/77 (dependency loops and
  the mesh RTTI walk), 7208D0 60/138 (fade/texture branches), 43B2F0 35/35, 430290 47/47,
  71AFD0 380/380, 96D5E0 44/58. 6646 executed pins, identical in both images.
  430710 misses: negative widths, CArray growth branches and the null param_4 path (430E50
  passes 0).

REPLAY
  python research/phase8-orders-input/terrain-visibility/native_terrain_visibility.py   (byte-identical)
