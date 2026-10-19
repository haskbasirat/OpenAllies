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
//! 0x738280 groups those gtiles into bodies by the water tile's variation
//! (0x738200) and builds one water object per body (0x7377F0): the saved
//! GMap+5C triangle lists (0x55B9E0) or 0x7376B0's regular quads, through
//! the 0x737600/0x7372A0 vertex hash, as (x, y, 0) vertices and u16
//! triangles. 0x736AB0 then sets every vertex z to the 0x550210 base height
//! minus 0.5 and widens the block min/max (+80/+84) with it. 0x71D840 reads the
//! first object only (0x736E70). The 0x73E300(0x736D50) effect is a
//! texture-coordinate transform (TerrainMap+50/+54 scroll); with 0x73BA00 it
//! is a render state, not geometry.
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
pub(crate) struct Heights<'a> {
    pub(crate) columns: i32,
    pub(crate) rows: i32,
    pub(crate) step_bits: u32,
    pub(crate) samples: &'a [u32],
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
pub(crate) struct Source<'a> {
    /// World +0x3C / +0x40.
    pub(crate) extents_bits: [u32; 2],
    /// `[GWorld+0x30]+0x08` GMap gtile size.
    pub(crate) gtile_bits: u32,
    /// `[[B39990]]` AVar0 TerrainBlockSize.
    pub(crate) terrain_block_size: i32,
    /// `[[B39940]]` UVar0 TerrainQuadSize.
    pub(crate) terrain_quad_size: i32,
    pub(crate) heights: Heights<'a>,
    /// `0x4F0E50(&xy, flags)` on GWorld (0x71AFD0 reads flags 0x122 and
    /// 0x123 at the four block corners); the extended ST0 result.
    pub(crate) ground: &'a dyn Fn([u32; 2], u32) -> Result<Binary>,
}

/// GMap tile classes for the 0x738280 water rule: `+1C`/`+20` tile counts and
/// the class `[[tile+0]+1C]+1C` of a linear tile index (`None` is a null
/// variation pointer).
pub(crate) trait Tiles {
    fn size(&self) -> [i32; 2];
    /// `[tile+0]` variation identity (0x738200's body key) and its
    /// `[[+0]+1C]+1C` class; `None` is a null variation pointer.
    fn tile(&self, index: i32) -> Result<Option<(usize, i32)>>;
    /// GMap+8 gtile size and GMap+C extent x (binary32 words).
    fn gmap_words(&self) -> Result<[u32; 2]>;
    /// 0x55B9E0 on GMap+5C and the 0x7378E0 loop: the cached points of an
    /// absolute gtile, `None` for a missing cell.
    fn cached_cell(&self, cell: [i32; 2]) -> Result<Option<Vec<[u32; 2]>>>;
    /// 0x550210 on GWorld+34 (the extended result).
    fn base_height(&self, xy: [u32; 2], p: Precision) -> Result<Binary>;
}

/// One 0x738280 water object (`[block+90][k]`, built by 0x7377F0): the
/// NiTriShapeData D+20 vertices, D+44 u16 triangles, D+3C u16 count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WaterObject {
    pub(crate) vertices: Vec<[u32; 3]>,
    pub(crate) triangles: Vec<u16>,
    pub(crate) triangle_count: u16,
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
    /// +0x9C water byte, the +90 water objects and the error of a failed
    /// 0x738280 rebuild.
    water: u8,
    water_objects: Vec<WaterObject>,
    water_error: Option<String>,
    /// +0x14F: 0x7208D0 rebuilds the water state while set.
    water_dirty: bool,
    /// +0xAC / +0xB0.
    index: [i32; 2],
    /// +0xC8 / +0xCC / +0xD4 (+0xD0 = quads / +B4 is not read here).
    stride: i32,
    quads: i32,
    quad_size: i32,
    /// +0xE8..+0xF4 0x71AFD0 bounding sphere (centre, radius), read by the
    /// visibility test 0x42F1F0 -> 0x42ED10.
    sphere: [u32; 4],
    /// +0x14E (0x720080 vertex refresh 0x71CFC0 while a mesh exists) and
    /// +0x14D (0x720080 0x71B8F0 recompute): set only by 0x71CC00/0x73D7F0.
    refresh_14e: bool,
    minmax_14d: bool,
}
impl Block {
    pub fn residency(&self) -> &Residency {
        &self.residency
    }
    fn first_water(&self) -> Result<&WaterObject> {
        self.water_objects.first().ok_or_else(|| {
            format!("block ({},{}) has no 0x7377F0 water object (+90 holder list empty)", self.index[0], self.index[1])
        })
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
    identity: std::sync::Arc<()>,
    /// The World's GMap+5C cache, decoded once from the TERRAIN suffix.
    water_cache: Option<std::result::Result<std::sync::Arc<crate::terrain_water_cache::WaterCache>, String>>,
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
        pc: Precision,
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
        let surface = parts.terrain.surface().map_err(|e| e.to_string())?;
        let ground = |xy: [u32; 2], flags: u32| surface.sample(xy, flags, pc).map_err(|e| format!("4F0E50: {e}"));
        let source = Source {
            extents_bits: extents,
            gtile_bits: parts.terrain.grid.cell_size,
            terrain_block_size: grid.terrain_block_size(),
            terrain_quad_size: quad()?,
            heights,
            ground: &ground,
        };
        let mut map = Self::construct(&source, pc)?;
        map.binding = Some(Binding { world: parts.revision.witness(), height: height.witness_for_world(parts.revision)? });
        let grid = &parts.terrain.grid;
        map.water_cache = Some(
            crate::terrain_water_cache::decode_suffix(
                &parts.terrain.terrain.pending_auxiliary_bytes,
                grid.width as u32,
                grid.height as u32,
            )
            .map(std::sync::Arc::new),
        );
        Ok(map)
    }

    pub(crate) fn bound<'h>(&self, world: &World<'_>, height: &'h view_heightfield::State) -> Result<Heights<'h>> {
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
            let tiles = self.world_tiles(world)?;
            self.water_7208d0(cell, &tiles, pc)?;
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
    pub fn rebuild_water(&mut self, world: &World<'_>, column: i32, row: i32, pc: Precision) -> Result<()> {
        let cell = self.cell_index(column, row)?;
        if !matches!(self.blocks[cell].residency, Residency::Resident(_)) {
            return Err("7208D0 rebuilds water only for visibility state-2 blocks".into());
        }
        if self.blocks[cell].water_dirty {
            let tiles = self.world_tiles(world)?;
            self.water_7208d0(cell, &tiles, pc)?;
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

    /// +8C, +90, +9C min holder, +98 max holder, +C0, +94 (oracle views).
    #[cfg(test)]
    pub(crate) fn extremes(&self) -> (u32, u32, Option<usize>, Option<usize>, bool, bool) {
        (self.min, self.max, self.min_holder, self.max_holder, self.recompute, self.changed)
    }
    #[cfg(test)]
    pub(crate) fn water_byte(&self, cell: usize) -> u8 {
        self.blocks[cell].water
    }
    pub(crate) fn identity(&self) -> &std::sync::Arc<()> {
        &self.identity
    }
    pub(crate) fn up_words(&self) -> [u32; 3] {
        self.up
    }
    pub(crate) fn loaded_byte(&self) -> u8 {
        self.loaded
    }
    pub(crate) fn block_size_a0(&self) -> i32 {
        self.block_size
    }
    pub(crate) fn block_count(&self) -> usize {
        self.blocks.len()
    }
    pub(crate) fn block_index(&self, cell: usize) -> [i32; 2] {
        self.blocks[cell].index
    }
    pub(crate) fn sphere_e8(&self, cell: usize) -> [u32; 4] {
        self.blocks[cell].sphere
    }
    pub(crate) fn resident(&self, cell: usize) -> bool {
        matches!(self.blocks[cell].residency, Residency::Resident(_))
    }
    pub(crate) fn water_dirty_14f(&self, cell: usize) -> bool {
        self.blocks[cell].water_dirty
    }
    pub(crate) fn refresh_flags(&self, cell: usize) -> (bool, bool) {
        (self.blocks[cell].refresh_14e, self.blocks[cell].minmax_14d)
    }
    /// The visibility owner's construction-time report: +44 has been null
    /// for every block since 0x723E00 (no 0x42FE10 pass ran yet).
    pub(crate) fn report_all_absent_after_723e00(&mut self) -> Result<()> {
        if self.blocks.iter().any(|b| b.residency != Residency::Unreported) {
            return Err("TerrainMap residency was already reported by another owner".into());
        }
        for block in &mut self.blocks {
            block.residency = Residency::Absent;
        }
        Ok(())
    }
    pub(crate) fn world_tiles<'w>(&self, world: &'w World<'_>) -> Result<impl Tiles + 'w> {
        let cache = match &self.water_cache {
            Some(Ok(cache)) => Ok(std::sync::Arc::clone(cache)),
            Some(Err(e)) => Err(format!("GMap+5C water cache: {e}")),
            None => Err("TerrainMap was not built from a live World (GMap+5C water cache)".into()),
        };
        Ok(WorldTiles { terrain: world.presentation_parts()?.terrain, cache })
    }
    #[cfg(test)]
    pub(crate) fn water_objects(&self, cell: usize) -> &[WaterObject] {
        &self.blocks[cell].water_objects
    }

    fn cell_index(&self, column: i32, row: i32) -> Result<usize> {
        if column < 0 || row < 0 || column >= self.blocks_xy[0] || row >= self.blocks_xy[1] {
            return Err(format!("block ({column},{row}) is outside the {}x{} block grid", self.blocks_xy[0], self.blocks_xy[1]));
        }
        Ok((row * self.blocks_xy[0] + column) as usize)
    }

    /// 0x721330 + 0x723E00 (+ 0x723D30/0x723960/0x71B8F0 per block).
    pub(crate) fn construct(source: &Source<'_>, pc: Precision) -> Result<Self> {
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
            identity: std::sync::Arc::new(()),
            water_cache: None,
        };
        // 0x72464B..0x7246A9: rows outer, columns inner, 0x723D30(map, x, y).
        for row in 0..blocks_xy[1] {
            for column in 0..blocks_xy[0] {
                let block = map.block_723960(column, row, source, pc)?;
                map.blocks.push(block);
            }
        }
        map.loaded = 1; // 0x7252AF
        Ok(map)
    }

    /// 0x723960 fields, 0x71AFD0 sphere, 0x71B8F0 min/max.
    fn block_723960(&self, column: i32, row: i32, source: &Source<'_>, pc: Precision) -> Result<Block> {
        let heights = source.heights;
        // 0x723AB2 +D0 = +A8 / +B4 (not read by 71D840; +B4 != 0 here).
        let mut block = Block {
            owner: [self.up, self.x_normal, self.y_normal],
            residency: Residency::Unreported,
            min: FLT_MAX,
            max: NEG_FLT_MAX,
            water: 0,
            water_objects: Vec::new(),
            water_error: None,
            water_dirty: true,
            index: [column, row],
            stride: self.stride,
            quads: self.quads,
            quad_size: self.quad_size, // 0x723ABE fild +AC; _ftol (exact)
            sphere: sphere_71afd0([column, row], self.block_size, source.ground, pc)?,
            refresh_14e: true, // 0x723BA7
            minmax_14d: false, // 0x723BA1
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
    pub(crate) fn load_720ad0(&mut self, heights: Heights<'_>, cell: usize, pc: Precision) -> Result<()> {
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
        block.refresh_14e = false; // 0x721017 (after the 720C79 tail)
        self.fold_71a690(cell)
    }

    /// 0x721030: +44 = 0.
    pub(crate) fn release_721030(&mut self, cell: usize) -> Result<()> {
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
    pub(crate) fn water_7208d0(&mut self, cell: usize, tiles: &impl Tiles, p: Precision) -> Result<()> {
        let g = idiv(self.quads, self.quads_per_gtile, "0x7382B7")?.0;
        let block = &mut self.blocks[cell];
        block.water_dirty = false;
        block.water = 0;
        block.water_objects.clear();
        block.water_error = None;
        let result = build_water(block, g, tiles, true, p);
        if let Err(e) = &result {
            block.water_error = Some(e.clone());
        }
        result
    }
    /// The 0x738280 flag with 0x7377F0 seamed out, as research/
    /// phase8-orders-input/terrain-map's oracle runs it (no geometry, no
    /// 0x736AB0 min/max widening).
    #[cfg(test)]
    pub(crate) fn water_7208d0_without_7377f0(&mut self, cell: usize, tiles: &impl Tiles) -> Result<()> {
        let g = idiv(self.quads, self.quads_per_gtile, "0x7382B7")?.0;
        let block = &mut self.blocks[cell];
        block.water_dirty = false;
        block.water = 0;
        block.water_objects.clear();
        block.water_error = None;
        let result = build_water(block, g, tiles, false, Precision::Double53);
        if let Err(e) = &result {
            block.water_error = Some(e.clone());
        }
        result
    }
}

/// 0x71AFD0: the block bounding sphere +E8/+EC/+F0 centre, +F4 radius, from
/// the four corners `(index + {0,1}) * fstp(fild +A0)` and their 0x4F0E50
/// heights at flags 0x122 and 0x123, with the native x87 register/store
/// pattern (the last 0x123 height stays extended; several sums and
/// differences are stored binary32, others stay in registers).
fn sphere_71afd0(
    index: [i32; 2],
    block_size: i32,
    ground: &dyn Fn([u32; 2], u32) -> Result<Binary>,
    p: Precision,
) -> Result<[u32; 4]> {
    let e = |r: aa_numeric::Result<Binary>| r.map_err(|e| format!("71AFD0: {e}"));
    let st = |v: &Binary| v.store_f32().map_err(|e| format!("71AFD0 binary32 store: {e}"));
    let add = |a: &Binary, c: &Binary| e(a.add(c, p));
    let sub = |a: &Binary, c: &Binary| e(a.sub(c, p));
    let mul = |a: &Binary, c: &Binary| e(a.mul(c, p));
    let bs = b(st(&Binary::from_i32(block_size))?)?; // 0x71AFDC fild; 0x71AFEC fstp
    let one = b(ONE)?;
    let (ix, iy) = (Binary::from_i32(index[0]), Binary::from_i32(index[1]));
    let (ix1, iy1) = (add(&ix, &one)?, add(&iy, &one)?);
    let corner = |x: &Binary, y: &Binary| -> Result<[u32; 2]> { Ok([st(&mul(x, &bs)?)?, st(&mul(y, &bs)?)?]) };
    let corners = [corner(&ix, &iy)?, corner(&ix1, &iy)?, corner(&ix, &iy1)?, corner(&ix1, &iy1)?];
    // Calls in order: corner k at 0x122 then 0x123; every result but the
    // last is stored binary32 (0x71B026..0x71B17F).
    let mut za = [0u32; 4];
    let mut zb = [0u32; 3];
    let mut last = None;
    for (k, xy) in corners.iter().enumerate() {
        za[k] = st(&ground(*xy, 0x122)?)?;
        let h = ground(*xy, 0x123)?;
        if k < 3 {
            zb[k] = st(&h)?;
        } else {
            last = Some(h);
        }
    }
    let h3b = last.ok_or("71AFD0 corner heights")?;
    let [x0, y0] = [b(corners[0][0])?, b(corners[0][1])?];
    let [x1, y1] = [b(corners[1][0])?, b(corners[1][1])?];
    let [x2, y2] = [b(corners[2][0])?, b(corners[2][1])?];
    let [x3, y3] = [b(corners[3][0])?, b(corners[3][1])?];
    let (z0a, z1a, z2a, z3a) = (b(za[0])?, b(za[1])?, b(za[2])?, b(za[3])?);
    let (z0b, z1b, z2b) = (b(zb[0])?, b(zb[1])?, b(zb[2])?);
    // 0x71B1A4..0x71B25B: the eight-term sums.
    let sx2 = b(st(&add(&add(&x1, &x0)?, &x2)?)?)?;
    let sx3 = b(st(&add(&sx2, &x3)?)?)?;
    let sx6 = b(st(&add(&add(&add(&sx3, &x0)?, &x1)?, &x2)?)?)?;
    let sx7 = b(st(&add(&sx6, &x3)?)?)?;
    let sy3 = b(st(&add(&add(&add(&y1, &y0)?, &y2)?, &y3)?)?)?;
    let sy4 = b(st(&add(&sy3, &y0)?)?)?;
    let sy7 = b(st(&add(&add(&add(&sy4, &y1)?, &y2)?, &y3)?)?)?;
    let sz1 = b(st(&add(&z1a, &z0a)?)?)?;
    let sz4 = b(st(&add(&add(&add(&sz1, &z2a)?, &z3a)?, &z0b)?)?)?;
    let sz5 = b(st(&add(&sz4, &z1b)?)?)?;
    let sz7 = add(&add(&sz5, &z2b)?, &h3b)?;
    let eighth = b(0x3e00_0000)?; // 0xA35EE0
    let cx = st(&mul(&sx7, &eighth)?)?;
    let cy = st(&mul(&sy7, &eighth)?)?;
    let cz = st(&mul(&sz7, &eighth)?)?;
    let (c_x, c_y, c_z) = (b(cx)?, b(cy)?, b(cz)?);
    let sq = |v: &Binary| mul(v, v);
    // 0x71B293..0x71B2DB: corner 0, first height (x stored, y register).
    let dx0 = sub(&x0, &c_x)?;
    let dx0f = b(st(&dx0)?)?;
    let dy0 = sub(&y0, &c_y)?;
    let dz0a = sub(&z0a, &c_z)?;
    let d0a = st(&add(&add(&sq(&dz0a)?, &sq(&dy0)?)?, &sq(&dx0f)?)?)?;
    // 0x71B2E3..0x71B38F: corners 1..3, first height, register dx/dy; the
    // binary32 copies feed the second-height distances.
    let mut da = [d0a, 0, 0, 0];
    let mut stored = [[0u32; 2]; 4];
    for (k, (x, y, z)) in [(&x1, &y1, &z1a), (&x2, &y2, &z2a), (&x3, &y3, &z3a)].into_iter().enumerate() {
        let dx = sub(x, &c_x)?;
        let dy = sub(y, &c_y)?;
        stored[k + 1] = [st(&dx)?, st(&dy)?];
        let dz = sub(z, &c_z)?;
        da[k + 1] = st(&add(&add(&sq(&dz)?, &sq(&dy)?)?, &sq(&dx)?)?)?;
    }
    // 0x71B397..0x71B412: second heights.
    let dz0b = sub(&z0b, &c_z)?;
    let d0b = st(&add(&add(&sq(&dy0)?, &sq(&dx0f)?)?, &sq(&dz0b)?)?)?;
    let mut db = [d0b, 0, 0];
    for (k, z) in [(1usize, &z1b), (2, &z2b)] {
        let (dx, dy) = (b(stored[k][0])?, b(stored[k][1])?);
        let dz = sub(z, &c_z)?;
        db[k] = st(&add(&add(&sq(&dx)?, &sq(&dz)?)?, &sq(&dy)?)?)?;
    }
    let (dx3f, dy3f) = (b(stored[3][0])?, b(stored[3][1])?);
    let dz3b = sub(&h3b, &c_z)?;
    let d3b = add(&add(&sq(&dx3f)?, &sq(&dz3b)?)?, &sq(&dy3f)?)?;
    // 0x71B43E..0x71B4EB: maxima (ties take the second operand), sqrt.
    let pick = |a: u32, c: u32| -> Result<u32> { Ok(if greater(a, c)? { a } else { c }) };
    let m1 = pick(da[2], da[3])?;
    let m2 = pick(pick(da[0], da[1])?, m1)?;
    let m3 = if b(db[2])?.compare(&d3b) == Ordering::Greater { db[2] } else { st(&d3b)? };
    let u = pick(pick(pick(db[0], db[1])?, m3)?, m2)?;
    let radius = st(&e(b(u)?.sqrt(p))?)?;
    Ok([cx, cy, cz, radius])
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

/// 0x738280 (with 0x737470 already applied by the caller): the water
/// bodies of the block, 0x7377F0 per body (`geometry`), the +9C flag.
fn build_water(block: &mut Block, g: i32, tiles: &impl Tiles, geometry: bool, p: Precision) -> Result<()> {
    let bodies = water_bodies_738280(block.index, g, tiles)?;
    for (_, cells) in &bodies {
        if geometry {
            let mut object = geometry_7377f0(block.index, g, cells, tiles, p)?;
            heights_736ab0(block, &mut object, tiles, p)?;
            block.water_objects.push(object);
        }
        block.water = 1; // 0x738532 / 0x73856B
    }
    Ok(())
}

/// 0x738280's gtile walk: for gtile (gx, gy) of the block (row-major),
/// the centre tile if its class is 2, else the first in-bounds non-null
/// neighbour of class 2 (directions 1..8), keys a body (0x738200: linear
/// search by the tile's variation, appended when new) that receives the
/// local (gx, gy). The centre tile is read without a bounds or null check.
fn water_bodies_738280(index: [i32; 2], g: i32, tiles: &impl Tiles) -> Result<Vec<(usize, Vec<[i32; 2]>)>> {
    let [w, h] = tiles.size();
    let mut bodies: Vec<(usize, Vec<[i32; 2]>)> = Vec::new();
    for gy in 0..g.max(0) {
        let y = gy.wrapping_add(index[1].wrapping_mul(g));
        for gx in 0..g {
            let x = gx.wrapping_add(index[0].wrapping_mul(g));
            let centre = w.wrapping_mul(y).wrapping_add(x);
            if centre < 0 || centre >= w.wrapping_mul(h) {
                return Err(format!("738280 reads GMap tile {centre} outside the {w}x{h} tile allocation"));
            }
            let (key, class) = tiles.tile(centre)?.ok_or("738280 dereferences a null centre tile variation")?;
            let mut found = (class == WATER_CLASS).then_some(key);
            let mut direction = 1;
            for _ in 0..8 {
                if found.is_some() {
                    break;
                }
                let [dx, dy] = NEIGHBOURS[direction];
                let (nx, ny) = (dx.wrapping_add(x), y.wrapping_add(dy));
                if nx >= 0 && nx < w && ny >= 0 && ny < h {
                    if let Some((k, c)) = tiles.tile(w.wrapping_mul(ny).wrapping_add(nx))? {
                        if c == WATER_CLASS {
                            found = Some(k);
                        }
                    }
                }
                direction = NEXT_DIRECTION[direction];
            }
            if let Some(key) = found {
                match bodies.iter_mut().find(|(k, _)| *k == key) {
                    Some((_, cells)) => cells.push([gx, gy]),
                    None => bodies.push((key, vec![[gx, gy]])),
                }
            }
        }
    }
    Ok(bodies)
}

/// 0x7372A0: `floor(fstp64(v*64 + 0.5))` for x and y, then
/// `hx - _ftol(GMap+C * -64) * hy` (32-bit wrapping).
fn hash_7372a0(point: [u32; 2], extent_x: u32, p: Precision) -> Result<i32> {
    let e = |r: aa_numeric::Result<Binary>| r.map_err(|e| format!("7372A0: {e}"));
    let axis = |v: u32| -> Result<i32> {
        let t = e(e(b(v)?.mul(&b(0x4280_0000)?, p))?.add(&b(0x3f00_0000)?, p))?;
        let floor = binary64_of(&t)?.floor();
        if !(f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&floor) {
            return Err("7372A0 _ftol integer indefinite".into());
        }
        Ok(floor as i32)
    };
    let hx = axis(point[0])?;
    let hy = axis(point[1])?;
    let w = e(b(extent_x)?.mul(&b(0xc280_0000)?, p))?
        .truncate_i32()
        .map_err(|e| format!("7372A0 _ftol: {e}"))?;
    Ok(hx.wrapping_sub(w.wrapping_mul(hy)))
}

/// `fstp qword`: the x87 register rounded to binary64 (nearest-even).
fn binary64_of(v: &Binary) -> Result<f64> {
    let bits = v.to_x87_extended_bits().ok_or("7372A0 value outside the x87 extended domain")?;
    let negative = (bits >> 79) & 1 != 0;
    let exponent = ((bits >> 64) & 0x7fff) as i32;
    let significand = bits as u64;
    if significand == 0 {
        return Ok(if negative { -0.0 } else { 0.0 });
    }
    let mut m = significand >> 11;
    let rest = significand & 0x7ff;
    if rest > 0x400 || (rest == 0x400 && m & 1 == 1) {
        m += 1;
    }
    let e2 = exponent - 16383 - 52;
    if !(-1000..=960).contains(&e2) || exponent == 0 {
        return Err("7372A0 binary64 store outside the proved normal range".into());
    }
    let value = (m as f64) * 2f64.powi(e2);
    Ok(if negative { -value } else { value })
}

/// 0x7377F0 for one body: cached cells (0x55B9E0) or 0x7376B0 quads,
/// deduplicated by 0x737600 (hash key only, first point kept), vertices
/// (x, y, 0), `count / 3` triangles stored as u16.
fn geometry_7377f0(index: [i32; 2], g: i32, cells: &[[i32; 2]], tiles: &impl Tiles, p: Precision) -> Result<WaterObject> {
    let origin = [index[0].wrapping_mul(g), index[1].wrapping_mul(g)];
    let [cell_bits, extent_x] = tiles.gmap_words()?;
    let mut points: Vec<[u32; 2]> = Vec::new();
    let mut lookup: std::collections::HashMap<i32, u32> = std::collections::HashMap::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut dedup = |point: [u32; 2]| -> Result<u32> {
        let hash = hash_7372a0(point, extent_x, p)?;
        if let Some(i) = lookup.get(&hash) {
            return Ok(*i);
        }
        let i = points.len() as u32;
        points.push(point);
        lookup.insert(hash, i);
        Ok(i)
    };
    let e = |r: aa_numeric::Result<Binary>| r.map_err(|e| format!("7376B0: {e}"));
    let st = |v: &Binary| v.store_f32().map_err(|e| format!("7376B0 binary32 store: {e}"));
    for local in cells {
        let cell = [local[0].wrapping_add(origin[0]), local[1].wrapping_add(origin[1])];
        match tiles.cached_cell(cell)? {
            Some(list) => {
                for point in list {
                    indices.push(dedup(point)?);
                }
            }
            None => {
                // 0x7376B0: P0 (x0,y0), P1 (x0,y1), P2 (x1,y1), P3 (x1,y0);
                // triangles P0 P3 P1 and P2 P1 P3.
                let s = b(cell_bits)?;
                let x0 = st(&e(Binary::from_i32(cell[0]).mul(&s, p))?)?;
                let y0 = st(&e(Binary::from_i32(cell[1]).mul(&s, p))?)?;
                let i0 = dedup([x0, y0])?;
                let y1 = st(&e(b(y0)?.add(&s, p))?)?;
                let i1 = dedup([x0, y1])?;
                let x1 = st(&e(b(x0)?.add(&s, p))?)?;
                let i2 = dedup([x1, y1])?;
                let i3 = dedup([x1, y0])?;
                indices.extend([i0, i3, i1, i2, i1, i3]);
            }
        }
    }
    let count = i32::try_from(indices.len()).map_err(|_| "7377F0 index count exceeds the native integer")?;
    let triangles = count / 3;
    Ok(WaterObject {
        vertices: points.iter().map(|q| [q[0], q[1], 0]).collect(),
        triangles: indices[..3 * triangles as usize].iter().map(|i| *i as u16).collect(),
        triangle_count: triangles as u16,
    })
}

/// 0x736AB0 for the first `u16 D+C` vertices: z = fst(550210(x, y) - 0.5);
/// the register value lowers +80 when below it, else raises +84 when above.
fn heights_736ab0(block: &mut Block, object: &mut WaterObject, tiles: &impl Tiles, p: Precision) -> Result<()> {
    let count = object.vertices.len() as u16 as usize;
    for vertex in object.vertices.iter_mut().take(count) {
        let height = tiles.base_height([vertex[0], vertex[1]], p)?;
        let z = height.sub(&b(0x3f00_0000)?, p).map_err(|e| format!("736AB0: {e}"))?;
        let stored = z.store_f32().map_err(|e| format!("736AB0 binary32 store: {e}"))?;
        vertex[2] = stored;
        if z.compare(&b(block.min)?) == Ordering::Less {
            block.min = stored;
        } else if z.compare(&b(block.max)?) == Ordering::Greater {
            block.max = stored;
        }
    }
    Ok(())
}

/// The World's GMap ([GWorld+30]): BattleTerrain grid, tile VariationData
/// identity and TerrainData class, the decoded GMap+5C cache and the
/// GWorld+34 base heightfield.
struct WorldTiles<'a> {
    terrain: &'a crate::battle_terrain::BattleTerrain,
    cache: std::result::Result<std::sync::Arc<crate::terrain_water_cache::WaterCache>, String>,
}
impl Tiles for WorldTiles<'_> {
    fn size(&self) -> [i32; 2] {
        [self.terrain.grid.width, self.terrain.grid.height]
    }
    fn tile(&self, index: i32) -> Result<Option<(usize, i32)>> {
        let w = self.terrain.grid.width;
        let (x, y) = (index % w, index / w);
        let key = self.terrain.variation_definition_at(x, y)? as *const _ as usize;
        let class = self.terrain.terrain_definition_at(x, y)?.native_type;
        let class = i32::try_from(class).map_err(|_| "GMap terrain class exceeds the native integer")?;
        Ok(Some((key, class)))
    }
    fn gmap_words(&self) -> Result<[u32; 2]> {
        // GMap+8 and the ceiled integer extent stored binary32 at 0x556653.
        Ok([self.terrain.grid.cell_size, (self.terrain.bound.layout.extent_i32[0] as f32).to_bits()])
    }
    fn cached_cell(&self, cell: [i32; 2]) -> Result<Option<Vec<[u32; 2]>>> {
        self.cache.as_ref().map_err(Clone::clone)?.cell_points(cell)
    }
    fn base_height(&self, xy: [u32; 2], p: Precision) -> Result<Binary> {
        self.terrain.base.state.field.sample_interpolated(xy, p).map_err(|e| format!("550210: {e}"))
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
    /// 0x736E70: the first water object; D+40 = 3 * D+3C (0x8D05A2).
    fn water_index_count(&self) -> Result<u32> {
        Ok(3 * u32::from(self.first_water()?.triangle_count))
    }
    fn water_triangle(&self, triangle: u32) -> Result<[u16; 3]> {
        let object = self.first_water()?;
        let at = 3 * triangle as usize;
        object
            .triangles
            .get(at..at + 3)
            .map(|t| [t[0], t[1], t[2]])
            .ok_or_else(|| "736C60 reads beyond the D+44 triangle list".into())
    }
    fn water_vertex(&self, index: u16) -> Result<[u32; 3]> {
        self.first_water()?
            .vertices
            .get(index as usize)
            .copied()
            .ok_or_else(|| "736C60 reads beyond the D+20 vertex array".into())
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
