//! Original water-area flood fill (`KKC_FloodFillWaterAreaGoal`,
//! `.\Source\Sim\Solver\FloodFill.cpp`) and the GMap water-area painting that
//! drives it (`KKC_GMap::PaintTerrain`, `.\Source\Sim\Map\Map.cpp`).
//!
//! Native objects: GWorld (global `0xB36534`) +0x30 GMap, +0x34 base
//! heightfield, +0x240 flood solver (`0x539D60`, vtable `0xA36048`, embedded
//! FIFO open list at +0x34, vtable `0xA3602C`). The goal (`0x539E30`, vtable
//! `0xA36058`) lives on the caller's stack and holds the start tile and the
//! water-area record; it keeps no state between solves.
//!
//! Persistent state written (and nothing else; the oracle checks every other
//! tile byte is untouched):
//! - GMap+0x48 visit stamp (`0x552440`, which clears every tile stamp and
//!   restarts at 0 once the incremented value reaches 0x7FFFFFFF unsigned);
//! - per reached tile: +0x28 visit stamp, +0x2D reached-from direction,
//!   +0x24 open-list link (cleared on reach, set when the next tile is queued);
//! - per painted tile: +0x2C water-area id; the old record's +4 count is
//!   decremented and the new record's incremented (wrapping i32);
//! - heightfield +0x44 per-vertex water-area bytes (`0x550F20`/`0x5507C0`:
//!   0 none, an id, or 0xFF where different areas meet);
//! - the FIFO head/tail and the process-wide solver-busy byte `0xB3D43D`.
//!
//! Caps: area ids are one byte in the tile (+0x2C), in the saved TERRAIN
//! record and in the vertex bytes, where 0xFF marks a shared vertex. The GMap
//! constructor (`0x556570`) builds 255 table entries (entry 0 null, records
//! 1..=254) and `0x552C90` searches ids 1..=254. This is data semantics, so the
//! 254-area limit is kept (see [`AREA_TABLE_LEN`]). No engine buffer caps are
//! involved: the native FIFO is an intrusive tile list, ported as such.
//!
//! Precision is an explicit input. Native stores are ported at the same
//! points; heights compare in x87 order including unordered results.
//!
//! Callers and inputs: `0x5559F0` ([`stroke_and_flood`], reached from
//! `0x555C10` [`paint_terrain`] and from the random-map routine `0x597530`,
//! not ported) floods GMap+0x50\[area\] from the tile of the stroke start.
//! The random-map caller `0x5972F0` (not ported) floods, in list order and
//! when its +0xD4 flag is set, every record `{u8 area +0, i32 x +4, i32 y +8}`
//! of its +0x108 list (+0x10C count) with GMap+0x50\[area\]. The terrain and
//! height brush routines (`0x5526C0`, `0x559FB0`, `0x5528F0`, `0x551140`,
//! `0x551B40`, `0x551F60`) are boundaries ([`StrokeWorld`],
//! [`PaintTerrainStages`]).
//!
//! [`FloodSolver::busy`] models the process-wide byte `0xB3D43D` that every
//! user of the generic solver `0x4F4050` (path solvers included) tests and
//! sets; an integration must keep a single such byte for all of them.
use aa_heightfield::Heightfield;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    Heightfield(#[from] aa_heightfield::Error),
    #[error("water map, area table or vertex grid does not describe a native layout")]
    InvalidLayout,
    /// The original would address memory outside the GMap tile allocation.
    #[error("tile index {0} lies outside the native tile allocation")]
    TileOutside(i64),
    /// The original would address memory outside the heightfield +0x44 bytes.
    #[error("vertex index {0} lies outside the native vertex-area allocation")]
    VertexOutside(i64),
    /// The original would read past the 255-entry GMap+0x50 table.
    #[error("area table index {0} lies outside the native 255-entry table")]
    AreaTableIndex(usize),
    /// The original dereferences this null GMap+0x50 entry (access violation).
    #[error("null water-area record {0} is dereferenced")]
    NullAreaRecord(usize),
    /// The original dereferences a null tile variation/type chain.
    #[error("tile {0} has a null terrain variation chain")]
    NullTerrain(usize),
    /// A reached tile is not adjacent: the original would index past its
    /// 3x3 direction tables (unreachable from the ported callers).
    #[error("solver direction delta outside the native 3x3 tables")]
    DirectionDelta,
    #[error("non-finite value outside the ported domain: {0}")]
    NonFinite(&'static str),
    #[error("boundary failed: {0}")]
    Boundary(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Terrain class (`[[tile+0]+0x1C]+0x1C`) that `0x539C40` lets the fill enter.
pub const WATER_CLASS: i32 = 2;
/// GMap+0x50 table size built by `0x556570`: entry 0 is null, 1..=254 are
/// records. Kept: ids are bytes and 0xFF is the shared-vertex marker.
pub const AREA_TABLE_LEN: usize = 255;
/// Vertex byte written where two different areas meet (`0x550F20`/`0x5507C0`).
pub const SHARED_VERTEX: u8 = 0xFF;
/// `0x9FE0C0`: binary32 0.1, the largest |height - base| a fill may cross into
/// a tile that already belongs to another area.
pub const SPREAD_THRESHOLD_BITS: u32 = 0x3DCC_CCCD;
/// `0x552440` restart bound (unsigned compare of the incremented stamp).
pub const STAMP_RESTART: u32 = 0x7FFF_FFFF;

/// `0xB322E0`: (dx, dy) per direction 0..=8.
const DELTA: [[i32; 2]; 9] = [
    [0, 0],
    [-1, -1],
    [-1, 0],
    [-1, 1],
    [0, 1],
    [1, 1],
    [1, 0],
    [1, -1],
    [0, -1],
];
/// `0xAB7804` next, `0xAB7828` previous, `0xAB784C` next-but-one,
/// `0xAB7870` previous-but-one, `0xAB7894` opposite direction.
const NEXT: [u32; 9] = [0, 2, 3, 4, 5, 6, 7, 8, 1];
const PREVIOUS: [u32; 9] = [0, 8, 1, 2, 3, 4, 5, 6, 7];
const NEXT2: [u32; 9] = [0, 3, 4, 5, 6, 7, 8, 1, 2];
const PREVIOUS2: [u32; 9] = [0, 7, 8, 1, 2, 3, 4, 5, 6];
const OPPOSITE: [u32; 9] = [0, 5, 6, 7, 8, 1, 2, 3, 4];
/// `*(0xB322D8)` = `0xAB78BC` and `*(0xB322DC)` = `0xAB78C8`, indexed by
/// dx/dy in -1..=1, and `0xAB78D0` mapping their OR to a direction.
const DX_MASK: [u32; 3] = [1, 0, 4];
const DY_MASK: [u32; 3] = [2, 0, 8];
const MASK_DIRECTION: [u8; 13] = [0, 2, 8, 1, 6, 0, 7, 0, 4, 3, 0, 0, 5];
/// `0x9FBDB8` 0.5, `0x9F32E0` 1.0, `0xA3ACC8` and `0xA3AA10` (water margin).
const HALF_BITS: u32 = 0x3F00_0000;
const ONE_BITS: u32 = 0x3F80_0000;
const MARGIN_SCALE_BITS: u32 = 0x40B5_04F3;
const MARGIN_BIAS_BITS: u32 = 0x38D1_B717;

/// One 0x30-byte GMap tile, reduced to the fields this code reads or writes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaterTile {
    /// `[[tile+0x00]+0x1C]+0x1C`: class of the tile's terrain variation.
    /// None is a null variation/type pointer (dereferenced by `0x539C40`).
    pub terrain_class: Option<i32>,
    /// +0x24 intrusive open-list link (native tile pointer as tile index).
    pub link: Option<u32>,
    /// +0x28 solver visit stamp.
    pub visit_stamp: u32,
    /// +0x2C water-area id; 0 is none.
    pub area: u8,
    /// +0x2D direction (0..=8) from which the last solve reached this tile.
    pub reached_from: u8,
}

/// GMap+0x50 entry: an 8-byte heap record, id byte +0 and tile count +4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AreaRecord {
    pub id: u8,
    pub count: i32,
}

/// The GMap fields used by water-area painting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaterMap {
    /// +0x1C / +0x20 tile counts.
    pub width: i32,
    pub height: i32,
    /// +0x04 tile shift (MapGTileResolutionBits) and +0x08 binary32 cell size.
    pub coordinate_shift: u32,
    pub cell_size_bits: u32,
    /// +0x0C / +0x10 binary32 world extents (`0x555C10` bounds test).
    pub extent_bits: [u32; 2],
    /// +0x48 visit stamp shared by every GMap solver.
    pub visit_stamp: u32,
    /// +0x28 row-major tiles.
    pub tiles: Vec<WaterTile>,
    /// +0x50 table of [`AREA_TABLE_LEN`] entries; None is a null pointer.
    pub areas: Vec<Option<AreaRecord>>,
}

/// Base heightfield (GWorld+0x34) +0x44 per-vertex water-area bytes, with the
/// heightfield's +0x0C width, +0x10 height and +0x14 shift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VertexAreas {
    pub width: i32,
    pub height: i32,
    pub coordinate_shift: u32,
    pub bytes: Vec<u8>,
}

/// Flood solver GWorld+0x240 open list (+0x3C head, +0x38 tail) and the
/// process-wide busy byte `0xB3D43D` shared by every `0x4F4050` solver.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloodSolver {
    pub busy: bool,
    pub head: Option<u32>,
    pub tail: Option<u32>,
}

/// Recorded `0x404F60` diagnostics. They have no state effect; they are
/// returned so callers can log them in native order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Diagnostic {
    /// FloodFill.cpp line 0x6F: "flood area %d not spreading to nbr
    /// %.1f,%.1f area %d height %f (base %f)" (binary32 bits of each value).
    SpreadBlocked {
        area: u8,
        x_bits: u32,
        y_bits: u32,
        neighbour_area: u8,
        height_bits: u32,
        base_bits: u32,
    },
    /// Map.cpp line 0x68A: "paint water aborted - no more available areas".
    NoFreeArea,
}

/// The GWorld objects one solve touches.
pub struct WaterWorld<'a> {
    pub map: &'a mut WaterMap,
    /// GWorld+0x34 samples for `0x550190`.
    pub heights: &'a Heightfield,
    /// GWorld+0x34 +0x44 bytes of the same heightfield.
    pub vertices: &'a mut VertexAreas,
    pub solver: &'a mut FloodSolver,
}

fn b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn is_nan(bits: u32) -> bool {
    bits & 0x7F80_0000 == 0x7F80_0000 && bits & 0x007F_FFFF != 0
}
fn is_infinite(bits: u32) -> bool {
    bits & 0x7FFF_FFFF == 0x7F80_0000
}

/// x87 FCOM of two binary32 values: None is unordered.
fn compare_bits(left: u32, right: u32) -> Result<Option<Ordering>> {
    if is_nan(left) || is_nan(right) {
        return Ok(None);
    }
    let rank = |bits: u32| -> i32 {
        if !is_infinite(bits) {
            0
        } else if bits >> 31 != 0 {
            -1
        } else {
            1
        }
    };
    let (l, r) = (rank(left), rank(right));
    if l != 0 || r != 0 {
        return Ok(Some(l.cmp(&r)));
    }
    Ok(Some(b(left)?.compare(&b(right)?)))
}

impl WaterMap {
    fn tile_count(&self) -> Result<usize> {
        let count = self.width.checked_mul(self.height).filter(|n| *n > 0);
        match count {
            Some(n) if self.width > 0 && self.height > 0 && n as usize == self.tiles.len() => {
                Ok(n as usize)
            }
            _ => Err(Error::InvalidLayout),
        }
    }
    pub fn validate(&self) -> Result<()> {
        let n = self.tile_count()?;
        if self.areas.len() != AREA_TABLE_LEN
            || self
                .tiles
                .iter()
                .any(|t| t.link.is_some_and(|l| l as usize >= n))
        {
            return Err(Error::InvalidLayout);
        }
        Ok(())
    }
    /// Native `(width * y + x) * 0x30 + tiles` with 32-bit wrapping arithmetic.
    pub fn linear_index(&self, cell: [i32; 2]) -> Result<usize> {
        let index = self.width.wrapping_mul(cell[1]).wrapping_add(cell[0]);
        if index < 0 || index as usize >= self.tiles.len() {
            return Err(Error::TileOutside(i64::from(index)));
        }
        Ok(index as usize)
    }
    /// `0x539DF0` / `0x54EDC0` pointer-to-cell division.
    pub fn cell_of(&self, index: usize) -> [i32; 2] {
        let i = index as i32;
        [i % self.width, i / self.width]
    }
    fn record(&self, index: usize) -> Result<AreaRecord> {
        self.areas
            .get(index)
            .ok_or(Error::AreaTableIndex(index))?
            .ok_or(Error::NullAreaRecord(index))
    }
    fn record_mut(&mut self, index: usize) -> Result<&mut AreaRecord> {
        self.areas
            .get_mut(index)
            .ok_or(Error::AreaTableIndex(index))?
            .as_mut()
            .ok_or(Error::NullAreaRecord(index))
    }
}

impl VertexAreas {
    pub fn validate(&self) -> Result<()> {
        let count = self.width.checked_mul(self.height).filter(|n| *n > 0);
        if self.width <= 0 || self.height <= 0 || count.map(|n| n as usize) != Some(self.bytes.len())
        {
            return Err(Error::InvalidLayout);
        }
        Ok(())
    }
    /// Native `width * y + bytes + x` with 32-bit wrapping arithmetic.
    fn index(&self, x: i32, y: i32) -> Result<usize> {
        let index = self.width.wrapping_mul(y).wrapping_add(x);
        if index < 0 || index as usize >= self.bytes.len() {
            return Err(Error::VertexOutside(i64::from(index)));
        }
        Ok(index as usize)
    }
}

impl WaterWorld<'_> {
    pub fn validate(&self) -> Result<()> {
        self.map.validate()?;
        self.vertices.validate()?;
        self.heights.validate()?;
        if self.heights.width != self.vertices.width
            || self.heights.height != self.vertices.height
            || self.heights.coordinate_shift != self.vertices.coordinate_shift
        {
            return Err(Error::InvalidLayout);
        }
        let n = self.map.tiles.len();
        if [self.solver.head, self.solver.tail]
            .iter()
            .any(|v| v.is_some_and(|i| i as usize >= n))
        {
            return Err(Error::InvalidLayout);
        }
        Ok(())
    }
}

/// Complete `0x552440`: advance GMap+0x48. When the incremented value is not
/// below 0x7FFFFFFF (unsigned) the stamp restarts at 0 and every tile's +0x28
/// stamp is cleared row by row, so all tiles then read as visited.
pub fn advance_visit_stamp(map: &mut WaterMap) -> Result<u32> {
    map.tile_count()?;
    let next = map.visit_stamp.wrapping_add(1);
    map.visit_stamp = next;
    if next < STAMP_RESTART {
        return Ok(next);
    }
    map.visit_stamp = 0;
    for tile in &mut map.tiles {
        tile.visit_stamp = 0;
    }
    Ok(0)
}

/// Complete `0x552C90`: the first id in 1..=254 whose record count is zero,
/// or 0 when every record is in use. A null record faults in the original.
pub fn free_area_id(map: &WaterMap) -> Result<u8> {
    if map.areas.len() != AREA_TABLE_LEN {
        return Err(Error::InvalidLayout);
    }
    for id in 1..AREA_TABLE_LEN {
        if map.record(id)?.count == 0 {
            return Ok(id as u8);
        }
    }
    Ok(0)
}

/// Complete `0x42A530`: tile containing a binary32 point (truncation, then
/// arithmetic shift; no bounds clamp).
pub fn tile_at_point(map: &WaterMap, point_bits: [u32; 2]) -> Result<usize> {
    let shift = map.coordinate_shift & 31;
    let x = b(point_bits[0])?.truncate_i32()? >> shift;
    let y = b(point_bits[1])?.truncate_i32()? >> shift;
    map.linear_index([x, y])
}

/// Cell centre as in `0x539B70`/`0x54EDC0`: `fild(c << shift) + cell * 0.5`
/// at `precision`, each stored to binary32.
fn cell_centre(map: &WaterMap, cell: [i32; 2], precision: Precision) -> Result<[u32; 2]> {
    let half = b(map.cell_size_bits)?.mul(&b(HALF_BITS)?, precision)?;
    let shift = map.coordinate_shift & 31;
    let mut out = [0; 2];
    for (slot, c) in out.iter_mut().zip(cell) {
        *slot = Binary::from_i32(c.wrapping_shl(shift))
            .add(&half, precision)?
            .store_f32()?;
    }
    Ok(out)
}

/// `0x550190` index selection (as `Heightfield::sample_nearest`), returning
/// the stored binary32 bits so non-finite samples keep x87 semantics.
fn nearest_height_bits(heights: &Heightfield, position_bits: [u32; 2]) -> Result<u32> {
    heights.validate()?;
    let shift = heights.coordinate_shift & 31;
    let x = (b(position_bits[0])?
        .truncate_i32()?
        .wrapping_add(heights.index_bias)
        >> shift)
        .clamp(0, heights.width - 1);
    let y = (b(position_bits[1])?
        .truncate_i32()?
        .wrapping_add(heights.index_bias)
        >> shift)
        .clamp(0, heights.height - 1);
    Ok(heights.height_bits[(y * heights.width + x) as usize])
}

/// `0x539C40` height test: `d = h - base` (FSUB at `precision`), FCHS when
/// FCOM reports below zero, blocked only when |d| compares greater than
/// 0.1f. Unordered results (NaN, inf - inf) never block.
fn exceeds_spread_threshold(height: u32, base: u32, precision: Precision) -> Result<bool> {
    if is_nan(height) || is_nan(base) {
        return Ok(false);
    }
    if is_infinite(height) || is_infinite(base) {
        // inf - inf of equal sign is the masked-invalid default NaN.
        return Ok(!(is_infinite(height) && is_infinite(base) && (height ^ base) >> 31 == 0));
    }
    let mut d = b(height)?.sub(&b(base)?, precision)?;
    if d.is_negative_nonzero() {
        d = d.negated();
    }
    Ok(d.compare(&b(SPREAD_THRESHOLD_BITS)?) == Ordering::Greater)
}

/// Complete `0x5507C0`: rebuild one vertex byte from the up to four tiles
/// around it (x offset outer, y offset inner, each -1 then 0). The vertex X
/// is converted exactly; Y passes through a binary32 store first.
fn recompute_vertex(world: &mut WaterWorld<'_>, vx: i32, vy: i32) -> Result<()> {
    let index = world.vertices.index(vx, vy)?;
    let hs = world.vertices.coordinate_shift & 31;
    let gs = world.map.coordinate_shift & 31;
    let tx = vx.wrapping_shl(hs) >> gs;
    let ty = b(Binary::from_i32(vy.wrapping_shl(hs)).store_f32()?)?.truncate_i32()? >> gs;
    world.vertices.bytes[index] = 0;
    for ox in [-1, 0] {
        for oy in [-1, 0] {
            let (x, y) = (tx.wrapping_add(ox), ty.wrapping_add(oy));
            if x < 0 || x >= world.map.width || y < 0 || y >= world.map.height {
                continue;
            }
            let area = world.map.tiles[(world.map.width * y + x) as usize].area;
            let byte = &mut world.vertices.bytes[index];
            if area != 0 && *byte != area {
                if *byte != 0 {
                    *byte = SHARED_VERTEX;
                    return Ok(());
                }
                *byte = area;
            }
        }
    }
    Ok(())
}

/// Complete `0x550F20`: the four vertices (x,y), (x+1,y), (x+1,y+1), (x,y+1)
/// of the vertex containing `centre`. Adding marks 0 -> id and any other
/// different id -> 0xFF; removing rebuilds each vertex with `0x5507C0`.
fn update_vertices(world: &mut WaterWorld<'_>, centre: [u32; 2], id: u8, add: bool) -> Result<()> {
    let shift = world.vertices.coordinate_shift & 31;
    let vx = b(centre[0])?.truncate_i32()? >> shift;
    let vy = b(centre[1])?.truncate_i32()? >> shift;
    for [dx, dy] in [[0, 0], [1, 0], [1, 1], [0, 1]] {
        let (x, y) = (vx.wrapping_add(dx), vy.wrapping_add(dy));
        if add {
            let index = world.vertices.index(x, y)?;
            let byte = &mut world.vertices.bytes[index];
            if *byte != id {
                *byte = if *byte == 0 { id } else { SHARED_VERTEX };
            }
        } else {
            recompute_vertex(world, x, y)?;
        }
    }
    Ok(())
}

/// Complete `0x552110` (`record` = GMap+0x50 index): set the tile's area to
/// the record id, mark its vertices and increment the record count.
pub fn add_tile_to_area(
    world: &mut WaterWorld<'_>,
    record: usize,
    tile: usize,
    precision: Precision,
) -> Result<()> {
    world.validate()?;
    add_tile(world, record, tile, precision)
}

/// Complete `0x552150`: clear the tile's area (before the record is read),
/// rebuild its vertices without it and decrement the record count.
pub fn remove_tile_from_area(
    world: &mut WaterWorld<'_>,
    record: usize,
    tile: usize,
    precision: Precision,
) -> Result<()> {
    world.validate()?;
    remove_tile(world, record, tile, precision)
}

fn checked_tile(world: &WaterWorld<'_>, tile: usize) -> Result<()> {
    if tile >= world.map.tiles.len() {
        return Err(Error::TileOutside(tile as i64));
    }
    Ok(())
}

fn add_tile(world: &mut WaterWorld<'_>, record: usize, tile: usize, precision: Precision) -> Result<()> {
    checked_tile(world, tile)?;
    let id = world.map.record(record)?.id;
    world.map.tiles[tile].area = id;
    let centre = cell_centre(world.map, world.map.cell_of(tile), precision)?;
    update_vertices(world, centre, id, true)?;
    let r = world.map.record_mut(record)?;
    r.count = r.count.wrapping_add(1);
    Ok(())
}

fn remove_tile(world: &mut WaterWorld<'_>, record: usize, tile: usize, precision: Precision) -> Result<()> {
    checked_tile(world, tile)?;
    if record >= world.map.areas.len() {
        return Err(Error::AreaTableIndex(record));
    }
    world.map.tiles[tile].area = 0;
    let id = world.map.record(record)?.id;
    let centre = cell_centre(world.map, world.map.cell_of(tile), precision)?;
    update_vertices(world, centre, id, false)?;
    let r = world.map.record_mut(record)?;
    r.count = r.count.wrapping_sub(1);
    Ok(())
}

/// The stack goal of `0x539E30` (+0x0C/+0x10 start, +0x14 record, +0x18
/// stored base height).
struct Goal {
    start: [i32; 2],
    record: usize,
    base_bits: u32,
}

/// `0x539C40` IsIndexBlocked.
fn is_blocked(
    world: &WaterWorld<'_>,
    goal: &Goal,
    tile: usize,
    precision: Precision,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<bool> {
    let t = &world.map.tiles[tile];
    if t.terrain_class.ok_or(Error::NullTerrain(tile))? != WATER_CLASS {
        return Ok(true);
    }
    let other = t.area;
    if other == 0 {
        return Ok(false);
    }
    let id = world.map.record(goal.record)?.id;
    if other == id {
        return Ok(false);
    }
    let centre = cell_centre(world.map, world.map.cell_of(tile), precision)?;
    let height = nearest_height_bits(world.heights, centre)?;
    if !exceeds_spread_threshold(height, goal.base_bits, precision)? {
        return Ok(false);
    }
    diagnostics.push(Diagnostic::SpreadBlocked {
        area: id,
        x_bits: centre[0],
        y_bits: centre[1],
        neighbour_area: other,
        height_bits: height,
        base_bits: goal.base_bits,
    });
    Ok(true)
}

/// `0x539C00`: move the tile into the goal's area unless it already has it.
fn paint(world: &mut WaterWorld<'_>, goal: &Goal, tile: usize, precision: Precision) -> Result<()> {
    let id = world.map.record(goal.record)?.id;
    let current = world.map.tiles[tile].area;
    if current == id {
        return Ok(());
    }
    if current != 0 {
        remove_tile(world, usize::from(current), tile, precision)?;
    }
    add_tile(world, goal.record, tile, precision)
}

/// `0x53C630` (reached via solver +8 `0x4F0550` and goal +0x14 `0x539B10`):
/// stamp, reached-from direction, then the +0x24 link is cleared.
fn reach(map: &mut WaterMap, from: [i32; 2], to: [i32; 2], tile: usize) -> Result<()> {
    map.tiles[tile].visit_stamp = map.visit_stamp;
    let delta = [to[0].wrapping_sub(from[0]), to[1].wrapping_sub(from[1])];
    if delta.iter().any(|d| !(-1..=1).contains(d)) {
        return Err(Error::DirectionDelta);
    }
    let mask = DX_MASK[(delta[0] + 1) as usize] | DY_MASK[(delta[1] + 1) as usize];
    let t = &mut map.tiles[tile];
    t.reached_from = MASK_DIRECTION[mask as usize];
    t.link = None;
    Ok(())
}

/// `0x539DB0` / `0x539D20` FIFO append through the tile +0x24 links.
fn push(world: &mut WaterWorld<'_>, tile: usize) {
    if let Some(tail) = world.solver.tail {
        world.map.tiles[tail as usize].link = Some(tile as u32);
    }
    world.solver.tail = Some(tile as u32);
    if world.solver.head.is_none() {
        world.solver.head = Some(tile as u32);
    }
}

/// `0x53F5D0` FIFO pop.
fn pop(world: &mut WaterWorld<'_>) -> Option<usize> {
    let head = world.solver.head?;
    let next = world.map.tiles[head as usize].link;
    world.solver.head = next;
    if next.is_none() {
        world.solver.tail = None;
    }
    Some(head as usize)
}

/// `0x554ED0` iterator words +0/+4 origin, +8 reached-from, +C current and
/// +10 last direction (+14 goal, +18 GMap are implicit).
struct Neighbours {
    origin: [i32; 2],
    came: u32,
    dir: u32,
    end: u32,
}

/// `0x4F2000` for this goal: never back along the reached-from direction,
/// inside the GMap (goal +0x40 is true), then not IsIndexBlocked (goal byte
/// +8 is clear); goal +0x38 is false so no corner test follows.
fn neighbour_valid(
    world: &WaterWorld<'_>,
    goal: &Goal,
    it: &Neighbours,
    precision: Precision,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<bool> {
    if OPPOSITE[it.came as usize] == it.dir {
        return Ok(false);
    }
    let d = DELTA[it.dir as usize];
    let n = [it.origin[0] + d[0], it.origin[1] + d[1]];
    if n[0] < 0 || n[0] >= world.map.width || n[1] < 0 || n[1] >= world.map.height {
        return Ok(false);
    }
    let tile = world.map.linear_index(n)?;
    Ok(!is_blocked(world, goal, tile, precision, diagnostics)?)
}

/// `0x4F2760`: step forward until a valid direction, the last direction or
/// a full turn; then the iterator is exhausted (0, 0).
fn next_neighbour(
    world: &WaterWorld<'_>,
    goal: &Goal,
    it: &mut Neighbours,
    precision: Precision,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<()> {
    let first = it.dir;
    if first == 0 {
        return Ok(());
    }
    while it.dir != it.end {
        it.dir = NEXT[it.dir as usize];
        if it.dir == first {
            break;
        }
        if neighbour_valid(world, goal, it, precision, diagnostics)? {
            return Ok(());
        }
    }
    it.dir = 0;
    it.end = 0;
    Ok(())
}

/// `0x554ED0` for this goal (+0x40 true, +0x34 false, +0x3C true): from the
/// start all eight directions 1..=8; after a diagonal arrival the five around
/// it; after an orthogonal arrival the three ahead.
fn first_neighbour(
    world: &WaterWorld<'_>,
    goal: &Goal,
    origin: [i32; 2],
    precision: Precision,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Neighbours> {
    let came = u32::from(world.map.tiles[world.map.linear_index(origin)?].reached_from);
    let (dir, end) = match came {
        0 => (1, 8),
        1 | 3 | 5 | 7 => (PREVIOUS2[came as usize], NEXT2[came as usize]),
        2 | 4 | 6 | 8 => (PREVIOUS[came as usize], NEXT[came as usize]),
        _ => (0, 0),
    };
    let mut it = Neighbours {
        origin,
        came,
        dir,
        end,
    };
    if it.dir != 0 && !neighbour_valid(world, goal, &it, precision, diagnostics)? {
        next_neighbour(world, goal, &mut it, precision, diagnostics)?;
    }
    Ok(it)
}

/// Entire `0x539E30` (goal constructor `0x53C570` + solver `0x4F4050`) with
/// `record` the GMap+0x50 index the caller read. Returns the native EAX: 0 if
/// the busy byte was set (it is cleared and nothing else happens), else 1
/// once the FIFO is exhausted.
///
/// Breadth-first: the start is reached and queued unconditionally and painted
/// when popped (even when it is not water). Each popped tile visits its
/// direction subset in table order; an unvisited neighbour is reached
/// (stamped), and unless IsIndexBlocked it is painted then queued. Tiles of
/// another area are entered when |height - base| <= 0.1 at the tiles' centre
/// samples, so separate water bodies at other levels keep their ids.
///
/// Errors are non-transactional: effects up to the failing native access
/// remain, including the busy byte (the original faults while it is set).
pub fn flood_fill_water_area(
    world: &mut WaterWorld<'_>,
    start: [i32; 2],
    record: usize,
    precision: Precision,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<u32> {
    world.validate()?;
    if record >= AREA_TABLE_LEN {
        return Err(Error::AreaTableIndex(record));
    }
    if world.solver.busy {
        world.solver.busy = false;
        return Ok(0);
    }
    world.solver.head = None;
    world.solver.tail = None;
    advance_visit_stamp(world.map)?;
    let centre = cell_centre(world.map, start, precision)?;
    let mut base_bits = nearest_height_bits(world.heights, centre)?;
    if is_nan(base_bits) {
        // FLD/FSTP quiets a signalling NaN; only unorderedness is observable.
        base_bits |= 0x0040_0000;
    }
    let goal = Goal {
        start,
        record,
        base_bits,
    };
    world.solver.busy = true;
    let start_tile = world.map.linear_index(start)?;
    reach(world.map, start, start, start_tile)?;
    push(world, start_tile);
    while let Some(current) = pop(world) {
        let origin = world.map.cell_of(current);
        if origin == goal.start {
            paint(world, &goal, current, precision)?;
        }
        let mut it = first_neighbour(world, &goal, origin, precision, diagnostics)?;
        while it.dir != 0 {
            let d = DELTA[it.dir as usize];
            let n = [origin[0] + d[0], origin[1] + d[1]];
            let tile = world.map.linear_index(n)?;
            if world.map.tiles[tile].visit_stamp != world.map.visit_stamp {
                reach(world.map, origin, n, tile)?;
                if !is_blocked(world, &goal, tile, precision, diagnostics)? {
                    paint(world, &goal, tile, precision)?;
                    push(world, tile);
                }
            }
            next_neighbour(world, &goal, &mut it, precision, diagnostics)?;
        }
    }
    world.solver.busy = false;
    Ok(1)
}

/// `0x5559F0` brush fields: +0x00 kind, +0x04 size, +0x10/+0x14 extents
/// (kinds 2 and 3 step by the smaller extent, others by the size).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrokeBrush {
    pub kind: i32,
    pub size_bits: u32,
    pub extent_bits: [u32; 2],
}

/// The `0x559980` change batch as seen by its consumers: +0x04 visit stamp
/// taken from `0x552440` and the +0x0A flag (the caller's ninth argument).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrokeBatch {
    pub visit_stamp: u32,
    pub flag: u8,
}

/// Terrain-brush subsystem boundaries of `0x5559F0` (not ported here). The
/// pass-through arguments (brush pointer, arguments 2, 5, 6 and 8) belong to
/// the implementation.
pub trait StrokeWorld {
    /// `0x5526C0`: stamp the brush at the binary32 cursor into the batch.
    fn stamp(&mut self, map: &mut WaterMap, batch: &StrokeBatch, point_bits: [u32; 2]) -> Result<()>;
    /// `0x559FB0`: apply the batch to the map.
    fn commit(&mut self, map: &mut WaterMap, batch: &StrokeBatch) -> Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrokeRequest {
    pub brush: StrokeBrush,
    pub from_bits: [u32; 2],
    pub to_bits: [u32; 2],
    /// Argument 7 (byte): water-area id to flood from `from`, 0 for none.
    pub area: u8,
    /// Argument 9 (byte) copied to batch +0x0A.
    pub batch_flag: u8,
}

/// Entire `0x5559F0` around its boundaries: open the batch (`0x559980`,
/// which advances the visit stamp), stamp along the segment, commit, then
/// flood `area` (GMap+0x50 entry, via the GWorld+0x240 solver) from the tile
/// of `from`; `0x554480` only releases batch lists. The segment walk keeps
/// the native mix of extended and binary32 values: direction from extended
/// dx and stored dy, step = max(brush step, binary32(trunc(cell) * 0.5)),
/// cursor stores after each stamp, the end point stamped exactly once.
pub fn stroke_and_flood<S: StrokeWorld>(
    world: &mut WaterWorld<'_>,
    stroke: &mut S,
    request: &StrokeRequest,
    precision: Precision,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<()> {
    world.validate()?;
    let p = precision;
    let batch = StrokeBatch {
        visit_stamp: advance_visit_stamp(world.map)?,
        flag: request.batch_flag,
    };
    let zero = Binary::from_i32(0);
    let dy32 = b(b(request.to_bits[1])?
        .sub(&b(request.from_bits[1])?, p)?
        .store_f32()?)?;
    let dx = b(request.to_bits[0])?.sub(&b(request.from_bits[0])?, p)?;
    let length = dx.mul(&dx, p)?.add(&dy32.mul(&dy32, p)?, p)?.sqrt(p)?;
    let mut length32 = length.store_f32()?;
    let (mut x_dir, mut y_dir) = (dx, dy32);
    if length.compare(&zero) == Ordering::Greater {
        let reciprocal = b(ONE_BITS)?.div(&b(length32)?, p)?;
        x_dir = x_dir.mul(&reciprocal, p)?;
        y_dir = reciprocal.mul(&y_dir, p)?;
    }
    let brush = &request.brush;
    let step_bits = if brush.kind == 3 || brush.kind == 2 {
        match compare_bits(brush.extent_bits[0], brush.extent_bits[1])? {
            Some(Ordering::Greater) | None => brush.extent_bits[1],
            _ => brush.extent_bits[0],
        }
    } else {
        brush.size_bits
    };
    if is_nan(step_bits) || is_infinite(step_bits) {
        // The original continues with a non-finite step (a NaN step never ends).
        return Err(Error::NonFinite("stroke step"));
    }
    let cell = b(world.map.cell_size_bits)?.truncate_i32()?;
    let minimum = Binary::from_i32(cell).mul(&b(HALF_BITS)?, p)?.store_f32()?;
    let mut step32 = step_bits;
    if b(step_bits)?.compare(&b(minimum)?) != Ordering::Greater {
        step32 = minimum;
    }
    let mut cursor = request.from_bits;
    if b(length32)?.compare(&zero) != Ordering::Less {
        let step = b(step32)?;
        let sy = y_dir.mul(&step, p)?.store_f32()?;
        let sx = x_dir.mul(&step, p)?.store_f32()?;
        loop {
            stroke.stamp(world.map, &batch, cursor)?;
            cursor[0] = b(sx)?.add(&b(cursor[0])?, p)?.store_f32()?;
            cursor[1] = b(cursor[1])?.add(&b(sy)?, p)?.store_f32()?;
            if b(length32)?.compare(&zero) != Ordering::Greater {
                break;
            }
            let rest = b(length32)?.sub(&step, p)?;
            length32 = rest.store_f32()?;
            if rest.compare(&zero) != Ordering::Greater {
                cursor = request.to_bits;
                length32 = 0;
            }
        }
    }
    stroke.commit(world.map, &batch)?;
    if request.area != 0 {
        let shift = world.map.coordinate_shift & 31;
        let y = b(request.from_bits[1])?.truncate_i32()? >> shift;
        let x = b(request.from_bits[0])?.truncate_i32()? >> shift;
        flood_fill_water_area(world, [x, y], usize::from(request.area), precision, diagnostics)?;
    }
    Ok(())
}

/// Heightfield-brush boundaries of `0x555C10` (not ported here).
pub trait PaintTerrainStages {
    /// Class 1: `0x551B40` on GWorld+0x38 (land height brush).
    fn land_height(&mut self, map: &mut WaterMap) -> Result<()>;
    /// Class 2: `0x5528F0` widening the copied brush by `margin_bits`.
    fn water_margin(&mut self, margin_bits: u32) -> Result<()>;
    /// Class 2: `0x551140` on GWorld+0x34 with descriptor +0x20 = `area`;
    /// false ends the paint before any stroke.
    fn water_surface(&mut self, map: &mut WaterMap, area: u8) -> Result<bool>;
    /// Class 2: `0x551F60` descriptor then `0x551B40` on GWorld+0x38.
    fn water_bed(&mut self, map: &mut WaterMap) -> Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaintTerrainRequest {
    pub brush: StrokeBrush,
    /// Argument 3: stroke start and the water-area probe point.
    pub point_bits: [u32; 2],
    /// Argument 4: stroke end.
    pub end_bits: [u32; 2],
    /// Argument 5's `[[variation]+0x1C]+0x1C` class.
    pub terrain_class: i32,
    /// Argument 11 (byte), the stroke batch flag.
    pub batch_flag: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaintTerrainOutcome {
    /// The point is not inside `[0, extent)` on both axes (or unordered).
    OutsideWorld,
    /// Diagnostic logged; nothing else changed.
    NoFreeArea,
    /// `0x551140` returned false.
    SurfaceRejected,
    /// `0x5559F0` ran with this area (0 unless water).
    Stroked { area: u8 },
}

/// Entire `0x555C10` around its brush boundaries. Water paints reuse the
/// area under the point or take the first free id (`0x552C90`); none left
/// logs "no more available areas" and stops. The margin is
/// `binary32(cell * 5.656854 + 0.0001)` at `precision`.
pub fn paint_terrain<S: StrokeWorld, T: PaintTerrainStages>(
    world: &mut WaterWorld<'_>,
    stroke: &mut S,
    stages: &mut T,
    request: &PaintTerrainRequest,
    precision: Precision,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<PaintTerrainOutcome> {
    world.validate()?;
    for (v, extent) in request.point_bits.into_iter().zip(world.map.extent_bits) {
        if !matches!(compare_bits(v, 0)?, Some(Ordering::Greater | Ordering::Equal))
            || compare_bits(v, extent)? != Some(Ordering::Less)
        {
            return Ok(PaintTerrainOutcome::OutsideWorld);
        }
    }
    let mut area = 0;
    if request.terrain_class == 1 {
        stages.land_height(world.map)?;
    } else if request.terrain_class == WATER_CLASS {
        area = world.map.tiles[tile_at_point(world.map, request.point_bits)?].area;
        if area == 0 {
            area = free_area_id(world.map)?;
            if area == 0 {
                diagnostics.push(Diagnostic::NoFreeArea);
                return Ok(PaintTerrainOutcome::NoFreeArea);
            }
        }
        let margin = b(world.map.cell_size_bits)?
            .mul(&b(MARGIN_SCALE_BITS)?, precision)?
            .add(&b(MARGIN_BIAS_BITS)?, precision)?
            .store_f32()?;
        stages.water_margin(margin)?;
        if !stages.water_surface(world.map, area)? {
            return Ok(PaintTerrainOutcome::SurfaceRejected);
        }
        stages.water_bed(world.map)?;
    }
    let stroke_request = StrokeRequest {
        brush: request.brush,
        from_bits: request.point_bits,
        to_bits: request.end_bits,
        area,
        batch_flag: request.batch_flag,
    };
    stroke_and_flood(world, stroke, &stroke_request, precision, diagnostics)?;
    Ok(PaintTerrainOutcome::Stroked { area })
}

#[cfg(test)]
#[path = "water_areas_tests.rs"]
mod tests;
