71D840 TERRAIN RAY QUERY: RUST PORT, NATIVE REPLAY, LIVE ADAPTER PLAN (2026-10-19)

Code
  crates/aa-runtime/src/player_order_input/terrain_ray.rs        (port)
  crates/aa-runtime/src/player_order_input/terrain_ray_tests.rs  (replays)

STATUS
  Ported instruction by instruction from the AA.exe disassembly (Capstone):
  every reached child, x87 operation order, PC rounding of register values and
  binary32 rounding exactly where the native code stores. Both native oracles
  replay with no mismatch at PC24, PC53 and PC64:
    v3 (accepted, research/skirmish-20261017-terrain-ray/native-ray-v3.json):
        480/480 rows (Single24 160/160, Double53 160/160, Extended64 160/160)
    v4 (new, this directory, native-terrain-ray-v4.json):
        3276/3276 rows (Single24 1092/1092, Double53 1092/1092, Extended64 1092/1092)
        including 27 rows where the native run faults (a negative cell index
        reads below the cell array). Rust returns an error for each of them.
  For every row the test checks AL, the primary point words (an unwritten word
  must stay at the 0xCDCDCDCD fill), the secondary words (null or non-null
  pointer; unwritten words must stay 0xABABABAB) and the exact ordered list of
  stores through both pointers.
  Verified with the official command (4 passed, 0 failed, no warnings in
  terrain_ray*.rs):
    CARGO_TARGET_DIR=target/agent-orders-ray cargo test -p aa-runtime --release
      --locked --offline -j 6 --lib player_order_input::terrain_ray
  Earlier attempts were blocked by other developers' in-progress files. In
  the meantime a scratch crate (target/agent-orders-ray-crate, now removed)
  included the unchanged terrain_ray.rs and its tests through #[path]. It also
  ran the mutation suite below.

NATIVE ADDRESSES PORTED (image base 0x400000)
  71D840  thiscall(eye*, dirX, dirY, dirZ, point*, secondary*, margin), ret 1C
  42D750  normalize (reused: crate::native_camera::ni::normalize, pub(crate))
  869C60 / 869C30  NiPlane ctors (copy normal+constant / zero-normal default)
  42ED60  plane intersection (AL=0 with no store on zero dot; flag accepts t<0)
  71C930  margin rectangle clip; 53A3A0 segment parameter (reused:
          aa_navigation::low_path::segment_parameter_53a3a0, checked against
          the disassembly); static epsilon B40420 = 0x38D1B717 (guarded once)
  71AB10  clamp; 53A7D0 rectangle containment
  42EE20 / 42EE70 block column/row (+ inline copies 71DDE2..71DE71),
          860680 shift-based binary32 truncation (exact shr-cl wrap)
  71ADC0  block at point; CRT _CIfmod 96DB7A via _trandisp2 9791C0/table
          B350F0 (valid/valid -> exact FPREM loop 96DB84; zero dividend ->
          FLDZ 977754; zero divisor -> domain error, Rust error)
  5E8910  block-edge crossing (AL only is used by 71D840)
  71D570  block query: 71A320 side-plane gate (jump table 71A478: 0 Y high,
          1 Y low, 2 X low, 3 X high); 71B9B0 mesh triangle walk with
          7151C0 / 436310 triangle plane and 875610 containment (eps AB7800);
          water child 736E70 -> 736C60 (first containing water triangle;
          water replaces the land hit when strictly higher); 71CD30
          absent-mesh flat-block fallback (eps A88AC8, clamps after the store)
  Quirks kept: the walker converts the major coordinate with 42EE20 and the
  minor with 42EE70 even when Y is major. Index clamps are upper-only, and
  values outside +-2^31 become 0. With a zero height range, the miss path's
  fraction is the averaged height itself. The miss path writes Z, X, Y. When
  no block answers and the top hit is inside the rectangle, the fallback
  writes the bottom hit (otherwise the top hit), with Z the mid height of the
  clamped block. dx is stored binary32 in 71C930 while dy stays in st0. The
  secondary point receives 71B9B0's land hit before the primary store.

COVERAGE (executed / instructions in v4, int3 excluded)
  71D840 615/618 (v3 alone: 437/618), 71C930 264/264, 71CD30 97/97, 71B9B0 203/205,
  71A320 93/93, 71D570 51/51, 5E8910 62/65, 53A3A0 81/81, 53A7D0 27/27,
  42ED60 65/65, 42EE20 25/26, 42EE70 25/26, 71AB10 32/32, 71ADC0 22/22,
  7151C0 51/51, 436310 54/54, 875610 76/76, 736C60 87/87, 736E70 3/3,
  860680 20/20, 42D750 30/37.
  Not executed: 71DC89/71BA08/71BA0F alignment padding; 71DE14/71DE5E/42EE55/
  42EEA5 (coordinate beyond +-2^31 -> index 0, unreachable for clamped/clipped
  map points); 5E8954.. (zero 2D determinant, unreachable after the degenerate
  check); 42D79A.. (zero-length direction, which makes 71D840 read undefined
  locals, a Rust error).
  Observed native reads (all covered by the traits): TerrainMap +04..+24
  (three vectors), +74 +78 +84 +88 +8C +90 +A0 +B8 +BC; cell +40 +44 +80 +84
  +90 +9C +AC +B0 +C8 +CC +D4; water holder +0, object +4, geometry +CC,
  data +20 +40 +44.

MUTATION SENSITIVITY (scratch only; 25 single-line mutants of the port)
  Detected by the oracles (18): plane Z store skipped, walker major via rows,
  water tie preferring water, 436310 Y kept in register, containment eps,
  71CD30 eps, zero-range fraction, fallback choice, two-crossing clip using
  tmin, side 0 constant, minor-exit counter check (non-termination), miss
  average order, secondary dropped, second-triangle winding, side gate
  rejecting t<0, rectangle inset, normalization skipped, miss mix at PC24.
  Not discriminated (7). Equivalent in this domain: fmod zero sign (x - +-0),
  END major/minor order (both query the same block; the gate has no side
  effects), the forward edge sum at PC24 and the minor start stored (both
  exact integers). Not reached by any row: length exactly 1e-6 (436310
  threshold equality). Masked by later binary32 stores: 71C930 dy stored,
  875610 r0 stored. The port follows the disassembly in each case.

PUBLIC API (terrain_ray.rs)
  pub trait Terrain { type Block: TerrainBlock;
    up_normal (+04), width (+74), height (+78), blocks_x (+84), blocks_y (+88),
    min_height (+8C), max_height (+90), block_size (+A0), loaded (+BC byte),
    block(cell: i32) -> &Block   ([+B8][blocks_x*row+col], wrapping) }
  pub trait TerrainBlock {
    owner_up_normal / owner_x_normal / owner_y_normal ([+40]+04/+10/+1C),
    has_mesh (+44 != 0), mesh_vertex(i32) ([[[+44]+CC]+20]+12i),
    min_height (+80), max_height (+84), water (+9C byte),
    water_index_count / water_triangle(u32) -> [u16;3] / water_vertex(u16)
      (D = [[[[+90]]+4]+CC]: +40 count, +44 u16 indices, +20 vertices),
    index_x (+AC), index_y (+B0), vertex_stride (+C8), quads (+CC),
    quad_size (+D4) }
  Every method returns Result<_, String>: missing data must be an error.
  pub fn intersect_71d840<T: Terrain>(terrain: &T, eye: [u32;3],
      direction: [u32;3], secondary: Option<&mut [u32;3]>, margin: i32,
      p: aa_numeric::Precision) -> Result<Outcome, String>
  pub struct Outcome { al: bool, point: [Option<u32>;3], writes: Vec<Write> }
    point_words() -> Result<[u32;3]> (error if any word was unwritten)
    apply_to(&mut [u32;3]) (stores only written words; 59A6E0 contract)
  pub struct Write { target: Target (Point | Secondary), offset, value }
  pub enum Side { YHigh=0, YLow=1, XLow=2, XHigh=3 }
  The query is a pure function of &Terrain, so concurrent queries are safe.
  Errors: parallel ray (71D8BD would read unwritten 42ED60 locals), 71CD30
  parallel clamp of an unwritten caller point, idiv #DE, fmod by zero,
  nonfinite or out-of-range numerics, missing terrain data, and a cell outside
  the array (the 27 native-fault rows).
  Caller: player_order_input::event::Services::terrain_71d840 (59A6E0: NULL
  secondary, margin 0, AL ignored, untouched unwritten words) maps to
    intersect_71d840(&map, eye, dir, None, 0, pc)?.apply_to(point).
  43C4B0 also passes margin 0 (push 0 at 43C4E8).

BUILDING A LIVE Terrain (read-only investigation; nothing implemented)
  No production owner yet holds a native TerrainMap/cell structure (723E00,
  723960, 71B8F0, 71B610 are not ported). An adapter needs these sources:
  - Heights: view_heightfield::State::samples_for_world(rev) -> Samples{
    step_bits, columns, rows, heights: &[u32]} (431BC0 storage, row-major
    y*columns+x, binary32 words, sample (x,y) at (x*step, y*step)). Owner:
    view_init_world::State.height (heightfield()). Per the source notes, 71B8F0/
    71B610 build block bounds/vertices from this same storage. Their port must
    fix the exact vertex arithmetic and the block min/max. Lab equivalent:
    vertex (xx,yy) of block (bx,by) = (bx*q+xx)*qs, (by*q+yy)*qs,
    height[(by*q+yy)*columns + bx*q+xx], stride q+1.
  - Block size (+A0): view_grid_variables::State::terrain_block_size()
    (TerrainBlockSize AVar; real data 8). Quad size: TerrainQuadSize UVar
    (aa-ui uvars terrain_quad_size_current(), default 2; view_grid_variables
    metrics() min(quad, block)). Quads per block (+CC) and the
    blocks_x/blocks_y split (+84/+88) come from 723E00 (not ported).
  - Width/height (+74/+78): view_visual_map::Global::dimensions_for_world
    (7241CE ceil of World+3C/+40). Owner: aa-client view_world_owner. It must
    be confirmed as the same value 723960 retains.
  - Min/max (+8C/+90): only the private view_heightfield Fields.limits (the
    authored HEIGHT_MAP limits). Whether the TerrainMap uses those or the
    computed extremes is unproven.
  - Loaded (+BC), mesh presence (+44): async native block loading. No Rust
    owner exists. Do not manufacture Resource flags or assume blocks loaded.
    The absent-mesh 71CD30 branch is ported and proven, so an unloaded or
    mesh-less block is a valid input once its state is known.
  - Up/X/Y normals (+04/+10/+1C): not present in Rust. The lab uses (0,0,1),
    (1,0,0), (0,1,0). They must come from the TerrainMap constructor port.
  - Water (+9C/+90): only preview/decode code exists. aa-client
    water_geometry.rs WaterGeometry{vertices [f32;2], indices u32, cells}
    decodes the TERRAIN cache (7AA9B0/7376B0/7377F0); water_render.rs is
    inspection-only. A per-block u16 index list plus water Z must be proven
    before water blocks can answer.
  Until those constructors are ported, adapter methods for the unproven
  fields must return Err rather than lab defaults.

OPEN GAPS
  - Live adapter: 723E00/723960/71B8F0/71B610 constructors, block loading
    (+BC/+44), owner normals and water child construction. The query itself
    is complete.
  - Not executed: the +-2^31 index-zero paths (logic identical to the
    executed shapes) and the zero-determinant 5E8910 exit (unreachable).
  - Outside the finite domain: nonfinite inputs, binary32 overflow and a
    parallel ray against the up plane are errors. Native behaviour for them
    is not modelled. Negative margins are native-defined except where the
    walk reads below the cell array.
  - Unicorn FPREM runs through binary64 (QEMU). This is exact for
    f32 / small-integer operands, so the oracle is valid for block sizes 8 and
    12. Rust models hardware FPREM (remainder_trunc).
  - Laboratory words are explicit oracle inputs, not native constructor
    receipts or current mission values.

REPLAY
  python research/phase8-orders-input/terrain-ray/native_terrain_ray_v4.py
    (byte-identical check; --write refuses to overwrite; --dry prints
    coverage). PYTHONPATH is not needed: it uses research/phase8-movement/harness.py.
  sha256: native-terrain-ray-v4.json 83e761b2524746a03c7d123295ac36cbf3abcbd4cc142ea5996dadeb9b6a1fc4
          native_terrain_ray_v4.py   0a3d8642a692b4c6fa531ea8e36c1224f582581418800ef01fa6b3e2427a15fb
          probe_terrain_ray_v4.py    59484b766fcd05f2b6f854daa6600b0079e3ebc13551a76f2bb88647e97661b1
          native-generate01.log = native-replay01.log e0941f42f1662550b58994f8e10a1d43b74c78e0b08fd6199f97ed2ce03b2b8a
