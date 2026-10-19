//! The native TerrainMap at `DAT_00B36518` and its TerrainBlocks, reduced to
//! exactly the fields 0x71D840 ([`super::terrain_ray`]) reads, built by the
//! original constructors' arithmetic (AA.exe, image base 0x400000):
//!
//! * 0x721330 (thiscall `(heightfield, config, quad, block)`, `ret 0x10`,
//!   called at 0x43FE0F with VWorld+BC): +04 up (0,0,1), +10 X normal
//!   (1,0,0), +1C Y normal (0,1,0) (0x72134E..0x721371, never rewritten),
//!   +6C the VWorld+BC heightfield, +8C/+90 its limits (overwritten by
//!   0x723E00), +94/+98/+9C/+BC/+C0 zero.
//! * 0x723E00 (called at 0x43FE26 after B36518 is published): 0x7241CE..
//!   0x724202 width/height = `_ftol(ceil(World+3C/+40))`; 0x724205 +B0 =
//!   `_ftol([GWorld+30]+8)` (GMap gtile size); 0x72421C +A0 = AVar0
//!   TerrainBlockSize; 0x72422B +AC = UVar0 TerrainQuadSize (raw, not the
//!   43FD0F clamp); +B4 = +B0 / +AC; assertions 0x95B..0x960 (855D30);
//!   0x724402..0x724455 +7C/+80 quads, +A8 quads per block, +84/+88 block
//!   counts, +A4 stride, +A0 = quads * quad; 0x724657 +8C/+90 = +-FLT_MAX;
//!   0x72464B..0x7246A9 row-major 0x723D30 -> 0x723960 block ctor; 0x7252AF
//!   +BC = 1.
//! * 0x723960 block: +40 owner, +44 mesh 0, +80/+84 = +-FLT_MAX, 0x736E20
//!   water holder (+90 0, +9C 0), +AC/+B0 index, +C8/+CC/+D0/+D4 from the
//!   owner, +14F water rebuild flag 1, then 0x71B8F0 block min/max over the
//!   heightfield samples `[+6C]+04 [cols*(j+row*q)+k+col*q]`.
//!
//! Block meshes are NOT built at map load. The camera-frustum visibility
//! pass 0x4312F0 (called at 0x43ED0A each VWorld update, 0x43EC60) rasterizes
//! the frustum footprint over the block grid (0x430E50 -> 0x430710; 0x430080
//! marks state 2, 0x4300D0 state 1), 0x42FF50 applies the states (0x720080)
//! and 0x42FE10 processes the change queue: state 2 -> resource Request
//! (0x721180) + Load (0x7212D0) -> 0x720AD0 when +44 is null (mesh from
//! 0x720130's pool, 0x71B610 vertices/min/max, 0x71A690 map fold); leaving
//! state 2 -> Release (0x7211F0) -> 0x721030 (+44 = 0) when the resource
//! count reaches zero. Map +8C/+90 are therefore the 0x71A690 fold over the
//! blocks loaded so far (plus the 0x71A750 recompute that 0x725340 runs at
//! 0x725429 when +C0 is set). The 71D840 answer depends on that render
//! history. It is camera state, not World data, and the visibility pass is
//! not ported: every block starts [`Residency::Unreported`]. A visibility
//! owner must report each block's state through [`TerrainMap::load_block`],
//! [`TerrainMap::release_block`] and [`TerrainMap::report_absent`], in
//! 0x42FE10 queue order. Until then, the block's `has_mesh` and the map
//! extremes are errors.
//!
//! The water byte +9C is rebuilt by 0x7208D0 at 0x720A93 (state-2 blocks,
//! +14F set) through 0x737470 + 0x738280, which sets it when any gtile of the
//! block, or one of its 8 in-bounds neighbours, has GMap terrain class 2.
//! The water triangle lists built by 0x7377F0 are not ported, so the
//! triangle queries of a water block are errors.
use super::terrain_ray::{Terrain, TerrainBlock};
use crate::{live_world_frame::World, view_grid_variables, view_heightfield, world_revision};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

type Result<T> = std::result::Result<T, String>;

const ONE: u32 = 0x3f80_0000;
const FLT_MAX: u32 = 0x7f7f_ffff; // 0x723A2B / 0x724657 / 0x720AF2
const NEG_FLT_MAX: u32 = 0xff7f_ffff; // 0x723A35 / 0x724661 / 0x720AFC
/// 0x721354..0x721371: +04 up, +10 X normal, +1C Y normal.
const UP: [u32; 3] = [0, 0, ONE];
const X_NORMAL: [u32; 3] = [ONE, 0, 0];
const Y_NORMAL: [u32; 3] = [0, ONE, 0];
/// 0xB322E0 (dx, dy) by direction, visited 1..8 through 0xAB7804.
const NEIGHBOURS: [[i32; 2]; 9] = [[0, 0], [-1, -1], [-1, 0], [-1, 1], [0, 1], [1, 1], [1, 0], [1, -1], [0, -1]];
const NEXT_DIRECTION: [usize; 9] = [0, 2, 3, 4, 5, 6, 7, 8, 1];
const WATER_CLASS: i32 = 2; // [[tile+0]+1C]+1C == 2 at 0x738340

fn b(word: u32) -> Result<Binary> {
    Binary::from_f32_bits(word).map_err(|e| format!("TerrainMap binary32 {word:#010x}: {e}"))
}
fn less(a: u32, c: u32) -> Result<bool> {
    Ok(b(a)?.compare(&b(c)?) == Ordering::Less)
}
fn greater(a: u32, c: u32) -> Result<bool> {
    Ok(b(a)?.compare(&b(c)?) == Ordering::Greater)
}
/// `cdq; idiv` at `site`: #DE for a zero divisor or INT_MIN / -1.
fn idiv(value: i32, divisor: i32, site: &str) -> Result<(i32, i32)> {
    match (value.checked_div(divisor), value.checked_rem(divisor)) {
        (Some(q), Some(r)) => Ok((q, r)),
        _ => Err(format!("723E00 idiv {value} / {divisor} at {site} raises #DE")),
    }
}
/// 0x96D4C0 CRT ceil of the exactly promoted binary32, then 0x96CC4C _ftol.
fn ceil_ftol(word: u32, field: &str) -> Result<i32> {
    let value = f64::from(f32::from_bits(word));
    if !value.is_finite() {
        return Err(format!("723E00 {field} {word:#010x}: native nonfinite ceil/_ftol is outside the proved domain"));
    }
    let ceiling = value.ceil();
    if ceiling < f64::from(i32::MIN) || ceiling > f64::from(i32::MAX) {
        return Err(format!("723E00 {field} {word:#010x}: _ftol integer indefinite is outside the proved domain"));
    }
    Ok(ceiling as i32)
}

/// One native heightfield (VWorld+BC: +04 samples, +10 step, +18 columns,
/// +1C rows), read by 0x71B8F0 and 0x71B610/0x4316A0.
#[derive(Clone, Copy)]
struct Heights<'a> {
    columns: i32,
    rows: i32,
    step_bits: u32,
    samples: &'a [u32],
}
impl Heights<'_> {
    /// `[+04 + 4 * (columns * y + x)]` with 32-bit wrapping index arithmetic.
    fn at(&self, x: i32, y: i32, site: &str) -> Result<u32> {
        let index = self.columns.wrapping_mul(y).wrapping_add(x);
        usize::try_from(index)
            .ok()
            .and_then(|i| self.samples.get(i).copied())
            .ok_or_else(|| format!("{site} reads heightfield sample {index} outside the {}x{} allocation", self.columns, self.rows))
    }
}

/// The native words 0x721330/0x723E00 read (production: [`TerrainMap::for_world`]).
struct Source<'a> {
    /// World +0x3C / +0x40.
    extents_bits: [u32; 2],
    /// `[GWorld+0x30]+0x08` GMap gtile size.
    gtile_bits: u32,
    /// `[[B39990]]` AVar0 TerrainBlockSize.
    terrain_block_size: i32,
    /// `[[B39940]]` UVar0 TerrainQuadSize.
    terrain_quad_size: i32,
    heights: Heights<'a>,
}

/// GMap tile classes for the 0x738280 water rule: `+1C`/`+20` tile counts and
/// the class `[[tile+0]+1C]+1C` of a linear tile index (`None` is a null
/// variation pointer).
trait Tiles {
    fn size(&self) -> [i32; 2];
    fn class(&self, index: i32) -> Result<Option<i32>>;
}

/// Mesh state of one block (+0x44), as reported by the visibility owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Residency {
    /// No visibility owner has reported this block since 0x723E00.
    Unreported,
    /// +0x44 is null: never in visibility state 2, or released by 0x721030.
    Absent,
    /// +0x44 holds a 0x720AD0 mesh; `[[+44]+CC]+20` vertex words (stride^2).
    Resident(Vec<[u32; 3]>),
}

/// One TerrainBlock (`[TerrainMap+B8][cell]`).
#[derive(Clone, Debug)]
pub struct Block {
    /// Owner +04/+10/+1C, read through +0x40.
    owner: [[u32; 3]; 3],
    residency: Residency,
    /// +0x80 / +0x84.
    min: u32,
    max: u32,
    /// +0x9C water byte and the error of a failed 0x738280 rebuild.
    water: u8,
    water_error: Option<String>,
    /// +0x14F: 0x7208D0 rebuilds the water state while set.
    water_dirty: bool,
    /// +0xAC / +0xB0.
    index: [i32; 2],
    /// +0xC8 / +0xCC / +0xD4 (+0xD0 = quads / +B4 is not read here).
    stride: i32,
    quads: i32,
    quad_size: i32,
}
impl Block {
    pub fn residency(&self) -> &Residency {
        &self.residency
    }
}

struct Binding {
    world: world_revision::Witness,
    height: view_heightfield::SampleWitness,
}

/// The TerrainMap fields 0x71D840 and its block queries read.
pub struct TerrainMap {
    up: [u32; 3],
    x_normal: [u32; 3],
    y_normal: [u32; 3],
    /// +0x74 / +0x78.
    size: [i32; 2],
    /// +0x84 / +0x88.
    blocks_xy: [i32; 2],
    /// +0x8C / +0x90.
    min: u32,
    max: u32,
    /// +0x94 changed byte, +0x98 max holder, +0x9C min holder (cell index).
    changed: bool,
    max_holder: Option<usize>,
    min_holder: Option<usize>,
    /// +0xA0 block size, +0xA4 stride, +0xA8 quads per block, +0xAC quad
    /// size, +0xB4 quads per gtile (= +0xB0 gtile size / +0xAC).
    block_size: i32,
    stride: i32,
    quads: i32,
    quad_size: i32,
    quads_per_gtile: i32,
    /// +0xBC.
    loaded: u8,
    /// +0xC0: 0x725340 runs 0x71A750 while set.
    recompute: bool,
    blocks: Vec<Block>,
    binding: Option<Binding>,
}

impl TerrainMap {
    /// 0x721330 + 0x723E00 over the live owners: World +3C/+40 (the World
    /// creator extents), the GMap gtile size and tile classes (the World's
    /// BattleTerrain grid, `[GWorld+30]`), the SAME completed VWorld+BC
    /// heightfield the view samples, AVar0 TerrainBlockSize (`grid`) and
    /// UVar0 TerrainQuadSize (`quad`, read after AVar0 as at 0x72422B).
    pub fn for_world(
        world: &World<'_>,
        height: &view_heightfield::State,
        grid: &view_grid_variables::State,
        quad: impl FnOnce() -> Result<i32>,
    ) -> Result<Self> {
        let parts = world.presentation_parts()?;
        let extents = parts.creator_extents.ok_or("723E00 requires the actual World +3C/+40 extent fields")?.bits();
        let samples = height.samples_for_world(parts.revision)?;
        let heights = Heights {
            columns: i32::try_from(samples.columns).map_err(|_| "heightfield +18 columns exceed the native integer")?,
            rows: i32::try_from(samples.rows).map_err(|_| "heightfield +1C rows exceed the native integer")?,
            step_bits: samples.step_bits,
            samples: samples.heights,
        };
        let source = Source {
            extents_bits: extents,
            gtile_bits: parts.terrain.grid.cell_size,
            terrain_block_size: grid.terrain_block_size(),
            terrain_quad_size: quad()?,
            heights,
        };
        let mut map = Self::construct(&source)?;
        map.binding = Some(Binding { world: parts.revision.witness(), height: height.witness_for_world(parts.revision)? });
        Ok(map)
    }

    fn bound<'h>(&self, world: &World<'_>, height: &'h view_heightfield::State) -> Result<Heights<'h>> {
        let binding = self.binding.as_ref().ok_or("TerrainMap was not built from a live World")?;
        let parts = world.presentation_parts()?;
        if !binding.world.is_for(parts.revision) || !binding.height.is_for(height, parts.revision) {
            return Err("TerrainMap belongs to another World or heightfield allocation (map +6C)".into());
        }
        let samples = height.samples_for_world(parts.revision)?;
        Ok(Heights {
            columns: samples.columns as i32,
            rows: samples.rows as i32,
            step_bits: samples.step_bits,
            samples: samples.heights,
        })
    }

    /// Visibility state 2 for the block at (column, row), as 0x42FE10 then
    /// 0x7208D0 apply it in one 0x42FF50 pass: 0x7212D0 builds the mesh with
    /// 0x720AD0 only when +44 is null, then 0x720A93 rebuilds the water byte
    /// (0x737470 + 0x738280) while +14F is set.
    pub fn load_block(&mut self, world: &World<'_>, height: &view_heightfield::State, column: i32, row: i32, pc: Precision) -> Result<()> {
        let heights = self.bound(world, height)?;
        let cell = self.cell_index(column, row)?;
        if !matches!(self.blocks[cell].residency, Residency::Resident(_)) {
            self.load_720ad0(heights, cell, pc)?;
        }
        if self.blocks[cell].water_dirty {
            let terrain = world.presentation_parts()?.terrain;
            self.water_7208d0(cell, &WorldTiles(terrain))?;
        }
        Ok(())
    }
    /// Leaving visibility state 2 when the block's resource count reaches
    /// zero (0x42FE10 -> 0x7211F0 -> 0x721030): +44 becomes null. The map
    /// extremes and holders are unchanged.
    pub fn release_block(&mut self, column: i32, row: i32) -> Result<()> {
        let cell = self.cell_index(column, row)?;
        self.release_721030(cell)
    }
    /// The visibility owner knows the block has had no mesh since 0x723E00.
    pub fn report_absent(&mut self, column: i32, row: i32) -> Result<()> {
        let cell = self.cell_index(column, row)?;
        match self.blocks[cell].residency {
            Residency::Resident(_) => Err(format!("block ({column},{row}) is resident; only 0x721030 removes its mesh")),
            _ => {
                self.blocks[cell].residency = Residency::Absent;
                Ok(())
            }
        }
    }
    /// 0x725340 at 0x725429..0x725438 (once per VWorld update, after the
    /// visibility pass): 0x71A750 when +C0 is set, then +C0 = 0. Returns
    /// whether the recompute ran.
    pub fn update_725340(&mut self) -> Result<bool> {
        if !self.recompute {
            return Ok(false);
        }
        self.recompute_71a750()?;
        self.recompute = false;
        Ok(true)
    }
    /// 0x71FFA0 sets +14F when a GMap tile change involves water.
    pub fn invalidate_water_71ffa0(&mut self, column: i32, row: i32) -> Result<()> {
        let cell = self.cell_index(column, row)?;
        self.blocks[cell].water_dirty = true;
        Ok(())
    }
    /// 0x7208D0's water rebuild for a resident block whose +14F is set.
    pub fn rebuild_water(&mut self, world: &World<'_>, column: i32, row: i32) -> Result<()> {
        let cell = self.cell_index(column, row)?;
        if !matches!(self.blocks[cell].residency, Residency::Resident(_)) {
            return Err("7208D0 rebuilds water only for visibility state-2 blocks".into());
        }
        if self.blocks[cell].water_dirty {
            let terrain = world.presentation_parts()?.terrain;
            self.water_7208d0(cell, &WorldTiles(terrain))?;
        }
        Ok(())
    }
    pub fn blocks_xy(&self) -> [i32; 2] {
        self.blocks_xy
    }
    pub fn block_at(&self, column: i32, row: i32) -> Result<&Block> {
        Ok(&self.blocks[self.cell_index(column, row)?])
    }
    /// +C0 (0x71A690 lost an extreme holder).
    pub fn recompute_pending(&self) -> bool {
        self.recompute
    }

    fn cell_index(&self, column: i32, row: i32) -> Result<usize> {
        if column < 0 || row < 0 || column >= self.blocks_xy[0] || row >= self.blocks_xy[1] {
            return Err(format!("block ({column},{row}) is outside the {}x{} block grid", self.blocks_xy[0], self.blocks_xy[1]));
        }
        Ok((row * self.blocks_xy[0] + column) as usize)
    }

    /// 0x721330 + 0x723E00 (+ 0x723D30/0x723960/0x71B8F0 per block).
    fn construct(source: &Source<'_>) -> Result<Self> {
        // 0x7241CE..0x724202: World +3C then +40.
        let width = ceil_ftol(source.extents_bits[0], "World+3C")?;
        let height = ceil_ftol(source.extents_bits[1], "World+40")?;
        // 0x724205: fld [GMap+8]; _ftol (truncation of the binary32 value).
        let gtile = b(source.gtile_bits)?
            .truncate_i32()
            .map_err(|e| format!("723E00 GMap+8 gtile {:#010x}: {e}", source.gtile_bits))?;
        let block_in = source.terrain_block_size;
        let quad = source.terrain_quad_size;
        // 0x72423C: +B4 = +B0 / +AC.
        let (quads_per_gtile, _) = idiv(gtile, quad, "0x72423C")?;
        // 0x72424B (assertion 0x95B): stale +A8 (0 from 0x7213FC) % +B4.
        let (_, stale) = idiv(0, quads_per_gtile, "0x72425C")?;
        if stale != 0 {
            return Err("723E00 assertion 0x95B m_quads_per_block_axis % m_quads_per_gtile".into());
        }
        let assertion = |line: u32, text: &str| Err(format!("723E00 assertion {line:#x} ({text}) fails: the 855D30 handler is not ported"));
        // 0x72428C (0x95C).
        if idiv(width, quad, "0x72429F")?.1 != 0 || idiv(height, quad, "0x7242A9")?.1 != 0 {
            return assertion(0x95c, "m_terrain_width % m_quad_size == 0 && m_terrain_height % m_quad_size == 0");
        }
        // 0x7242D9 (0x95D).
        if width < 1 || height < 1 || quad < 1 || block_in < 1 {
            return assertion(0x95d, "m_terrain_width > 0 && m_terrain_height > 0 && m_quad_size > 0 && m_block_size > 0");
        }
        // 0x724326 (0x95E).
        if width < block_in || height < block_in || block_in < quad {
            return assertion(0x95e, "m_terrain_width >= m_block_size && m_terrain_height >= m_block_size && m_block_size >= m_quad_size");
        }
        // 0x724371 (0x95F).
        if idiv(block_in, quad, "0x724381")?.1 != 0 {
            return assertion(0x95f, "m_block_size % m_quad_size == 0");
        }
        // 0x7243B5 (0x960).
        if idiv(width, block_in, "0x7243C8")?.1 != 0 || idiv(height, block_in, "0x7243D2")?.1 != 0 {
            return assertion(0x960, "m_terrain_width % m_block_size == 0 && m_terrain_height % m_block_size == 0");
        }
        // 0x724402..0x724455 (all operands positive after the assertions).
        let quads = block_in / quad;
        let blocks_xy = [width / block_in, height / block_in];
        let stride = quads + 1;
        let block_size = quads.wrapping_mul(quad);
        // 0x724558..0x724565: imul +88,+84 then shl 2 for operator new.
        let count = blocks_xy[1]
            .checked_mul(blocks_xy[0])
            .filter(|n| (*n as u32).checked_mul(4).is_some())
            .ok_or("723E00 cell array size wraps 32 bits at 0x72455E..0x724565")?;
        let mut blocks = Vec::new();
        blocks.try_reserve_exact(count as usize).map_err(|_| "723E00 cell array allocation failed")?;
        let mut map = Self {
            up: UP,
            x_normal: X_NORMAL,
            y_normal: Y_NORMAL,
            size: [width, height],
            blocks_xy,
            min: FLT_MAX,
            max: NEG_FLT_MAX,
            changed: false,
            max_holder: None,
            min_holder: None,
            block_size,
            stride,
            quads,
            quad_size: quad,
            quads_per_gtile,
            loaded: 0,
            recompute: false,
            blocks,
            binding: None,
        };
        // 0x72464B..0x7246A9: rows outer, columns inner, 0x723D30(map, x, y).
        for row in 0..blocks_xy[1] {
            for column in 0..blocks_xy[0] {
                let block = map.block_723960(column, row, source.heights)?;
                map.blocks.push(block);
            }
        }
        map.loaded = 1; // 0x7252AF
        Ok(map)
    }

    /// 0x723960 fields + 0x71B8F0.
    fn block_723960(&self, column: i32, row: i32, heights: Heights<'_>) -> Result<Block> {
        // 0x723AB2 +D0 = +A8 / +B4 (not read by 71D840; +B4 != 0 here).
        let mut block = Block {
            owner: [self.up, self.x_normal, self.y_normal],
            residency: Residency::Unreported,
            min: FLT_MAX,
            max: NEG_FLT_MAX,
            water: 0,
            water_error: None,
            water_dirty: true,
            index: [column, row],
            stride: self.stride,
            quads: self.quads,
            quad_size: self.quad_size, // 0x723ABE fild +AC; _ftol (exact)
        };
        // 0x71B8F0: j outer, k inner over stride^2 samples.
        let q = block.stride.wrapping_sub(1);
        for j in 0..block.stride.max(0) {
            for k in 0..block.stride {
                let h = heights.at(k.wrapping_add(column.wrapping_mul(q)), j.wrapping_add(row.wrapping_mul(q)), "71B8F0")?;
                fold_block(&mut block, h)?;
            }
        }
        Ok(block)
    }

    /// 0x720AD0 through 0x720C79: +80/+84 reset, 0x720130 mesh, 0x71B610
    /// vertices and min/max, 0x71A690 map fold.
    fn load_720ad0(&mut self, heights: Heights<'_>, cell: usize, pc: Precision) -> Result<()> {
        let block_size = self.block_size;
        let block = &mut self.blocks[cell];
        block.min = FLT_MAX;
        block.max = NEG_FLT_MAX;
        let stride = block.stride;
        let q = stride.wrapping_sub(1);
        let count = usize::try_from(stride).ok().and_then(|s| s.checked_mul(s)).ok_or("720130 vertex count overflows")?;
        let mut vertices = Vec::new();
        vertices.try_reserve_exact(count).map_err(|_| "720130 vertex allocation failed")?;
        let store = |v: Binary| v.store_f32().map_err(|e| format!("71B610 binary32 store: {e}"));
        let step = b(heights.step_bits)?;
        // 0x71B629/0x71B64E: fild (index * +A0); fstp m32.
        let x0 = store(Binary::from_i32(block.index[0].wrapping_mul(block_size)))?;
        let mut y = store(Binary::from_i32(block.index[1].wrapping_mul(block_size)))?;
        let (column_base, row_base) = (q.wrapping_mul(block.index[0]), block.index[1].wrapping_mul(q));
        for j in 0..stride.max(0) {
            let mut x = x0;
            for k in 0..stride {
                // 0x4316A0: the raw sample word (normals are not read by 71D840).
                let h = heights.at(k.wrapping_add(column_base), j.wrapping_add(row_base), "4316A0")?;
                vertices.push([x, y, h]);
                fold_block(block, h)?;
                // 0x71B881: fld x; fadd step; fstp x.
                x = store(b(x)?.add(&step, pc).map_err(|e| e.to_string())?)?;
            }
            // 0x71B8C2: fld y; fadd step; fstp y.
            y = store(b(y)?.add(&step, pc).map_err(|e| e.to_string())?)?;
        }
        block.residency = Residency::Resident(vertices);
        self.fold_71a690(cell)
    }

    /// 0x721030: +44 = 0.
    fn release_721030(&mut self, cell: usize) -> Result<()> {
        let block = &mut self.blocks[cell];
        if !matches!(block.residency, Residency::Resident(_)) {
            return Err("721030 releases only a resident block mesh (+44)".into());
        }
        block.residency = Residency::Absent;
        Ok(())
    }

    /// 0x71A690(block): extreme holders and the map fold.
    fn fold_71a690(&mut self, cell: usize) -> Result<()> {
        let (min, max) = (self.blocks[cell].min, self.blocks[cell].max);
        // 0x71A6A0: the min holder loses the minimum when its min rose.
        if self.min_holder == Some(cell) && greater(min, self.min)? {
            self.min_holder = None;
        }
        // 0x71A6C1: the max holder loses the maximum when its max fell.
        if self.max_holder == Some(cell) && less(max, self.max)? {
            self.max_holder = None;
        }
        if less(min, self.min)? {
            self.min = min;
            self.min_holder = Some(cell);
            self.changed = true;
        }
        if greater(max, self.max)? {
            self.max = max;
            self.max_holder = Some(cell);
            self.changed = true;
        }
        if self.min_holder.is_none() || self.max_holder.is_none() {
            self.recompute = true; // 0x71A73E
        }
        Ok(())
    }

    /// 0x71A750: +8C/+90 = +-FLT_MAX, then every cell in order; holders
    /// move only on a strict improvement.
    fn recompute_71a750(&mut self) -> Result<()> {
        self.min = FLT_MAX;
        self.max = NEG_FLT_MAX;
        for cell in 0..self.blocks.len() {
            let (min, max) = (self.blocks[cell].min, self.blocks[cell].max);
            if less(min, self.min)? {
                self.min = min;
                self.min_holder = Some(cell);
            }
            if greater(max, self.max)? {
                self.max = max;
                self.max_holder = Some(cell);
            }
        }
        Ok(())
    }

    /// 0x7208D0 at 0x720A93: +14F = 0, 0x737470 clears the water state
    /// (+9C = 0), 0x738280 sets +9C when the block's gtiles or their
    /// in-bounds neighbours include terrain class 2.
    fn water_7208d0(&mut self, cell: usize, tiles: &impl Tiles) -> Result<()> {
        let g = idiv(self.quads, self.quads_per_gtile, "0x7382B7")?.0;
        let block = &mut self.blocks[cell];
        block.water_dirty = false;
        block.water = 0;
        block.water_error = None;
        match water_738280(block.index, g, tiles) {
            Ok(water) => {
                block.water = u8::from(water);
                Ok(())
            }
            Err(e) => {
                block.water_error = Some(e.clone());
                Err(e)
            }
        }
    }
}

/// 0x71B8F0 / 0x71B610 block folds: the minimum is replaced unless it is
/// below the sample, the maximum unless it is above it (ties take the later
/// sample's bits).
fn fold_block(block: &mut Block, h: u32) -> Result<()> {
    if b(h).is_err() {
        return Err(format!(
            "nonfinite heightfield sample {h:#010x}: x87 unordered (NaN quieting) and infinite folds are outside the proved domain"
        ));
    }
    if !less(block.min, h)? {
        block.min = h;
    }
    if !greater(block.max, h)? {
        block.max = h;
    }
    Ok(())
}

/// 0x738280's flag: some gtile (gx, gy) of the block, or one of its eight
/// in-bounds non-null neighbours (directions 1..8), has class 2. The centre
/// tile is read without a bounds or null check.
fn water_738280(index: [i32; 2], g: i32, tiles: &impl Tiles) -> Result<bool> {
    let [w, h] = tiles.size();
    let mut any = false;
    for gy in 0..g.max(0) {
        let y = gy.wrapping_add(index[1].wrapping_mul(g));
        for gx in 0..g {
            let x = gx.wrapping_add(index[0].wrapping_mul(g));
            let centre = w.wrapping_mul(y).wrapping_add(x);
            if centre < 0 || centre >= w.wrapping_mul(h) {
                return Err(format!("738280 reads GMap tile {centre} outside the {w}x{h} tile allocation"));
            }
            let class = tiles.class(centre)?.ok_or("738280 dereferences a null centre tile variation")?;
            let mut water = class == WATER_CLASS;
            let mut direction = 1;
            for _ in 0..8 {
                if water {
                    break;
                }
                let [dx, dy] = NEIGHBOURS[direction];
                let (nx, ny) = (dx.wrapping_add(x), y.wrapping_add(dy));
                if nx >= 0 && nx < w && ny >= 0 && ny < h {
                    if let Some(c) = tiles.class(w.wrapping_mul(ny).wrapping_add(nx))? {
                        water = c == WATER_CLASS;
                    }
                }
                direction = NEXT_DIRECTION[direction];
            }
            any |= water;
        }
    }
    Ok(any)
}

/// The World's GMap tiles: BattleTerrain grid size and TerrainData class.
struct WorldTiles<'a>(&'a crate::battle_terrain::BattleTerrain);
impl Tiles for WorldTiles<'_> {
    fn size(&self) -> [i32; 2] {
        [self.0.grid.width, self.0.grid.height]
    }
    fn class(&self, index: i32) -> Result<Option<i32>> {
        let w = self.0.grid.width;
        let class = self.0.terrain_definition_at(index % w, index / w)?.native_type;
        i32::try_from(class).map(Some).map_err(|_| "GMap terrain class exceeds the native integer".into())
    }
}

fn unreported(cell: usize) -> String {
    format!(
        "TerrainBlock {cell} +44 mesh residency is unreported: 0x42FE10 -> 0x7212D0/0x720AD0 (load) and 0x7211F0/0x721030 (release) follow the camera visibility pass 0x4312F0, which is not ported"
    )
}

impl Terrain for TerrainMap {
    type Block = Block;
    fn up_normal(&self) -> Result<[u32; 3]> {
        Ok(self.up)
    }
    fn width(&self) -> Result<i32> {
        Ok(self.size[0])
    }
    fn height(&self) -> Result<i32> {
        Ok(self.size[1])
    }
    fn blocks_x(&self) -> Result<i32> {
        Ok(self.blocks_xy[0])
    }
    fn blocks_y(&self) -> Result<i32> {
        Ok(self.blocks_xy[1])
    }
    fn min_height(&self) -> Result<u32> {
        match self.blocks.iter().position(|b| b.residency == Residency::Unreported) {
            Some(cell) => Err(format!("TerrainMap +8C is the 0x71A690 fold over loaded blocks; {}", unreported(cell))),
            None => Ok(self.min),
        }
    }
    fn max_height(&self) -> Result<u32> {
        match self.blocks.iter().position(|b| b.residency == Residency::Unreported) {
            Some(cell) => Err(format!("TerrainMap +90 is the 0x71A690 fold over loaded blocks; {}", unreported(cell))),
            None => Ok(self.max),
        }
    }
    fn block_size(&self) -> Result<i32> {
        Ok(self.block_size)
    }
    fn loaded(&self) -> Result<u8> {
        Ok(self.loaded)
    }
    fn block(&self, cell: i32) -> Result<&Block> {
        usize::try_from(cell)
            .ok()
            .and_then(|i| self.blocks.get(i))
            .ok_or_else(|| format!("TerrainMap cell {cell} is outside the cell array (0x724569 allocation)"))
    }
}

impl TerrainBlock for Block {
    fn owner_up_normal(&self) -> Result<[u32; 3]> {
        Ok(self.owner[0])
    }
    fn owner_x_normal(&self) -> Result<[u32; 3]> {
        Ok(self.owner[1])
    }
    fn owner_y_normal(&self) -> Result<[u32; 3]> {
        Ok(self.owner[2])
    }
    fn has_mesh(&self) -> Result<bool> {
        match &self.residency {
            Residency::Unreported => Err(format!(
                "block ({},{}) +44 mesh residency is unreported (visibility pass 0x4312F0 / 0x42FE10 not ported)",
                self.index[0], self.index[1]
            )),
            Residency::Absent => Ok(false),
            Residency::Resident(_) => Ok(true),
        }
    }
    fn mesh_vertex(&self, index: i32) -> Result<[u32; 3]> {
        match &self.residency {
            Residency::Resident(vertices) => usize::try_from(index)
                .ok()
                .and_then(|i| vertices.get(i).copied())
                .ok_or_else(|| format!("71B9B0 vertex {index} is outside the 720130 stride^2 allocation")),
            _ => Err("71B9B0 reads the mesh of a block without +44".into()),
        }
    }
    fn min_height(&self) -> Result<u32> {
        Ok(self.min)
    }
    fn max_height(&self) -> Result<u32> {
        Ok(self.max)
    }
    fn water(&self) -> Result<u8> {
        if let Some(e) = &self.water_error {
            return Err(format!("block +9C water rebuild failed: {e}"));
        }
        Ok(self.water)
    }
    fn water_index_count(&self) -> Result<u32> {
        Err(format!(
            "block ({},{}) water triangles: the 0x7377F0 water geometry (D+40/+44/+20) is not ported",
            self.index[0], self.index[1]
        ))
    }
    fn water_triangle(&self, _triangle: u32) -> Result<[u16; 3]> {
        self.water_index_count().map(|_| [0; 3])
    }
    fn water_vertex(&self, _index: u16) -> Result<[u32; 3]> {
        self.water_index_count().map(|_| [0; 3])
    }
    fn index_x(&self) -> Result<i32> {
        Ok(self.index[0])
    }
    fn index_y(&self) -> Result<i32> {
        Ok(self.index[1])
    }
    fn vertex_stride(&self) -> Result<i32> {
        Ok(self.stride)
    }
    fn quads(&self) -> Result<i32> {
        Ok(self.quads)
    }
    fn quad_size(&self) -> Result<i32> {
        Ok(self.quad_size)
    }
}

#[cfg(test)]
#[path = "terrain_map_tests.rs"]
mod tests;
