//! Original static SAI region-topology builder `KKC_SAIMap::StaticMapAnalysis`
//! (`7ACB00`, `.\Source\Sai\SAIMap.cpp`) and its callees, producing the same
//! [`Topology`] representation that the saved-map loader (`7AA9B0`) produces.
//!
//! Native callers: `4FB9E0`, `4FBA10` and `4FC600` (GWorld load below world
//! version 17, "Old map that used niche filler - recomputing regions") and
//! `5925E0` (random map generator). All of them pass the SAI map at `B3652C`.
//!
//! Ported routines, in native call order:
//! * `7ACB00` square placement loop (`StaticMapStartingSquareSize`, with the
//!   `7A9B00` reduction), `7A9DF0` square test, `7A9EA0` alternating growth;
//! * `7ABD40` split into rectangles no wider than the `7AA180` width, tile
//!   publication, `7BAC90` rectangle construction, `7AACD0`/`7B9B80` side
//!   links, region creation (`7BA4F0`/`7B99B0`) or the small-rectangle list;
//! * `7AB7A0`/`7AA8B0` island flood (recursion replaced by an explicit stack
//!   with the same visiting order), `7AA2A0` primary island, the `7ACD43` map
//!   diagonal, `7AB320` small-rectangle merge, and `7BC590` region centers and
//!   the distance hash (`7BA3C0`, rehash `606750`).
//!
//! Native linked lists insert at the head (`7B9B80`, `7B99B0`); every output
//! vector is the live head-to-tail order of the freshly analysed SAI map, so
//! `Topology::prepare_graph` sees what original `7BCA60` sees after `7ACB00`.
//! A saved and reloaded map differs only in list orders the loader rebuilds:
//! `7AA9B0` re-links region rectangle lists in descending rectangle order, and
//! distance pairs follow hash-chain order at save time. Side lists, bounds,
//! centers and distances are identical (research/phase8-movement evidence).
//!
//! Not ported here (explicit inputs or caller duties):
//! * `7AA180` (max rectangle width) uses the CRT `_CIpow` (`96D890`, x87
//!   FYL2X/F2XM1), `floor` (`96D5E0`) and `_ftol2` (`96CC4C`). Its integer result
//!   is the explicit input [`StaticMapParameters::max_rectangle_width`].
//! * `7AC9A0` ClearStaticMapAnalysis is a precondition: this function starts
//!   from a cleared SAI map (no islands/regions/rectangles, every tile +10 null).
//!   Its external teardown (`79FF10` when SAI map +5 is set; `5D85C0(0)` when
//!   `B3974C` and its +18C are non-null) belongs to those subsystems' owners.
//! * After success native sets SAI map +4 and, when `B36518` is non-null, byte
//!   +101 of that object to 1; the caller owns those flags.
//!
//! Timing: `59AA70`/`59AB60`/`59ABA0` (QueryPerformanceCounter timer) and the
//! `404F60` log calls only format diagnostics. The builder never yields or
//! stops on time; there is no wall-clock budget to model. `404ED0` at
//! `7ACD88` is a warning when more than 0x1000 rectangles exist, not a cap; it
//! is reported as [`StaticAnalysis::rectangle_count_warning`].
use crate::topology::{GridLayout, PostLoad, Rectangle, SavedRegion, Side, StaticTile, Topology};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::BTreeSet};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    Topology(#[from] crate::topology::Error),
    #[error(transparent)]
    Allocation(#[from] std::collections::TryReserveError),
    #[error("invalid static map input: {0}")]
    Input(&'static str),
    /// The native integer division would raise #DE (zero divisor or overflow).
    #[error("native integer division fault in {0}")]
    DivideFault(&'static str),
    /// The native loop would never terminate for these inputs.
    #[error("native static analysis never terminates: {0}")]
    NonTerminating(&'static str),
    /// Native would dereference an invalid pointer, read an uninitialized
    /// local or raise an assertion dialog (`855D30`). No result is invented.
    #[error("native static analysis reaches an invalid state: {0}")]
    Native(&'static str),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Every value read by the builder through `4B2E60` SVar pointers (Data/Game/
/// SVars.tgi), plus the integer result of `7AA180`. No defaults are implied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticMapParameters {
    /// `StaticMapStartingSquareSize` (int, read at `7ACBD1`).
    pub starting_square_size: i32,
    /// `StaticMapUseFastSquareReduction` (flag byte, non-zero selects fast).
    pub use_fast_square_reduction: bool,
    /// `StaticMapFastSquareReductionMult` (int).
    pub fast_square_reduction_mult: i32,
    /// `StaticMapFastSquareReductionDiv` (int).
    pub fast_square_reduction_div: i32,
    /// `StaticMapMinRegionSize` (int, total tiles).
    pub min_region_size: i32,
    /// `StaticMapMinRegionWidth` (int, tiles along one edge).
    pub min_region_width: i32,
    /// `StaticMapMaxRegionSizeForExpansion` (int).
    pub max_region_size_for_expansion: i32,
    /// Integer returned by `7AA180` for this grid: trunc(floor(
    /// StaticMapMaxRectangleWidth * pow((H+W)*0.5/StaticMapBaseMapWidth,
    /// StaticMapMaxRectangleWidthExp) + 0.5)). Not computed here (see module).
    pub max_rectangle_width: i32,
}

/// The constructed GMap fields read by the builder (world +30 grid).
#[derive(Clone, Copy, Debug)]
pub struct StaticGrid<'a> {
    pub layout: GridLayout,
    /// Grid +8 binary32 cell size, used by the `54EDC0` tile centers.
    pub cell_size_bits: u32,
    /// Row-major tiles (+20 flags and effective definition type, as `54ECE0`).
    pub tiles: &'a [StaticTile],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Island {
    /// Island +4 region array, in native append order.
    pub regions: Vec<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StaticAnalysis {
    /// Enabled topology with live list orders and the native distance tables
    /// in bucket order (keys are unique, so `prepare_graph` is order-free).
    pub topology: Topology,
    /// SAI map +8 island objects (rebuilt differently by the loader).
    pub islands: Vec<Island>,
    /// SAI map +2C (island id) and +30, from `7AA2A0` and `7ACD43`. Note that
    /// `7AA2A0` runs before `7AB320`, so only pre-merge regions are counted.
    pub post_load: PostLoad,
    /// `7ACD7E`: more than 0x1000 rectangles makes native log a warning.
    pub rectangle_count_warning: bool,
}

/// Inline terrain type (`7A9AC0`/`7A9DF0`/`7A9EA0`/`7ACB00`): flag +20 bit 9
/// selects bit 8 as 0/1, bit 8 alone gives 2, else the definition type.
pub fn terrain_type(tile: &StaticTile) -> u32 {
    if tile.flags20 & 0x200 != 0 {
        u32::from(tile.flags20 & 0x100 != 0)
    } else if tile.flags20 & 0x100 != 0 {
        2
    } else {
        tile.definition_type
    }
}

fn idiv(a: i32, b: i32, site: &'static str) -> Result<i32> {
    a.checked_div(b).ok_or(Error::DivideFault(site))
}

/// `7A9B00`: next square size (`(mult*size)/div` with 32-bit wrapping imul
/// and truncating idiv, or `size-1`).
pub fn next_square_size(size: i32, params: &StaticMapParameters) -> Result<i32> {
    if params.use_fast_square_reduction {
        idiv(
            params.fast_square_reduction_mult.wrapping_mul(size),
            params.fast_square_reduction_div,
            "7A9B00",
        )
    } else {
        Ok(size.wrapping_sub(1))
    }
}

/// Native side-length helper `7B8E50`: +E - +C - +8 + +A + 1, as signed 16-bit.
fn side_length(side: &Side) -> i32 {
    let [b0, b1, b2, b3] = side.bounds;
    i32::from(
        b3.wrapping_sub(b2)
            .wrapping_sub(b0)
            .wrapping_add(b1)
            .wrapping_add(1) as i16,
    )
}

/// `(maxX-minX+1)*(maxY-minY+1)` with native 16-bit subtraction/increment.
fn rect_area(bounds: [i16; 4]) -> i32 {
    let [y0, x1, y1, x0] = bounds;
    let x = x1.wrapping_sub(x0).wrapping_add(1);
    let y = y1.wrapping_sub(y0).wrapping_add(1);
    i32::from(x).wrapping_mul(i32::from(y))
}

/// Native region distance hash (region +5C buckets, +60 count, +64 entries,
/// +68 auto-grow byte = 1 from `7BA4F0`). Each chain is stored head first.
#[derive(Clone, Debug)]
struct DistanceTable {
    buckets: Vec<Vec<(u32, u32)>>,
    len: u32,
}
impl DistanceTable {
    fn new() -> Self {
        Self {
            buckets: vec![Vec::new(); 11],
            len: 0,
        }
    }
    fn contains(&self, key: u32) -> bool {
        let b = (key % self.buckets.len() as u32) as usize;
        self.buckets[b].iter().any(|&(k, _)| k == key)
    }
    /// `7BA3C0`: grow when entries >= 2*buckets, then head-insert.
    fn insert(&mut self, key: u32, value: u32) -> Result<()> {
        let count = self.buckets.len() as u32;
        if self.len >= count.wrapping_mul(2) {
            // `606750`: 2n+1, skipping at most two multiples of 3 or 5.
            let skip = |m: u32| m.is_multiple_of(3) || m.is_multiple_of(5);
            let mut m = count.wrapping_mul(2).wrapping_add(1);
            if skip(m) {
                m += 2;
                if skip(m) {
                    m += 2;
                }
            }
            let mut grown: Vec<Vec<(u32, u32)>> = Vec::new();
            grown.try_reserve_exact(m as usize)?;
            grown.resize_with(m as usize, Vec::new);
            for chain in &self.buckets {
                for &entry in chain {
                    grown[(entry.0 % m) as usize].insert(0, entry);
                }
            }
            self.buckets = grown;
        }
        let b = (key % self.buckets.len() as u32) as usize;
        self.buckets[b].try_reserve(1)?;
        self.buckets[b].insert(0, (key, value));
        self.len = self.len.wrapping_add(1);
        Ok(())
    }
    fn pairs(&self) -> Vec<[u32; 2]> {
        self.buckets
            .iter()
            .flatten()
            .map(|&(k, v)| [k, v])
            .collect()
    }
}

/// Rectangle object (`7BAC90`, 0x34 bytes). Side lists are kept in insertion
/// order; the native live order (head first) is the reverse.
struct Rect {
    region: i32,
    island: i32,
    bounds: [i16; 4],
    sides: [Vec<Side>; 4],
}
/// Region object (`7BA4F0`, 0x78 bytes); rectangle list in insertion order.
struct Region {
    rects: Vec<u32>,
    center: [u32; 2],
    table: DistanceTable,
}

struct Builder<'a> {
    grid: &'a StaticGrid<'a>,
    params: &'a StaticMapParameters,
    p: Precision,
    width: i32,
    height: i32,
    tile_rects: Vec<Option<u32>>,
    rects: Vec<Rect>,
    regions: Vec<Region>,
    islands: Vec<Island>,
    /// `7ABD40` small-rectangle list, head first.
    small: Vec<u32>,
}

impl Builder<'_> {
    fn index(&self, x: i32, y: i32) -> Result<usize> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return Err(Error::Native("tile index outside the native grid"));
        }
        Ok(y as usize * self.width as usize + x as usize)
    }
    fn tile_type_at(&self, x: i32, y: i32) -> Result<u32> {
        Ok(terrain_type(&self.grid.tiles[self.index(x, y)?]))
    }
    fn rect_at(&self, x: i32, y: i32) -> Result<Option<u32>> {
        Ok(self.tile_rects[self.index(x, y)?])
    }
    /// `7B9320`: type of the rectangle's (minX, minY) tile.
    fn rect_type(&self, id: u32) -> Result<u32> {
        let b = self.rects[id as usize].bounds;
        self.tile_type_at(i32::from(b[3]), i32::from(b[0]))
    }
    /// `7B9900`: signed 32-bit sum of the region's rectangle areas.
    fn region_area(&self, region: u32) -> i32 {
        self.regions[region as usize]
            .rects
            .iter()
            .fold(0i32, |sum, &r| {
                sum.wrapping_add(rect_area(self.rects[r as usize].bounds))
            })
    }
    /// Live (head-first) region rectangle order.
    fn region_rects(&self, region: u32) -> Vec<u32> {
        self.regions[region as usize]
            .rects
            .iter()
            .rev()
            .copied()
            .collect()
    }
    /// Live (head-first) side entries of side lists 0..3 (`7BAEE0` iterator).
    fn side_entries(&self, id: u32) -> Vec<Side> {
        self.rects[id as usize]
            .sides
            .iter()
            .flat_map(|list| list.iter().rev().cloned())
            .collect()
    }

    /// `7A9DF0`: every tile of the s*s square is unassigned and of `kind`.
    fn square_free(&self, x: i32, y: i32, s: i32, kind: u32) -> Result<bool> {
        let (xe, ye) = (x.wrapping_add(s), y.wrapping_add(s));
        let mut i = x;
        while i < xe {
            let mut j = y;
            while j < ye {
                if self.rect_at(i, j)?.is_some() || self.tile_type_at(i, j)? != kind {
                    return Ok(false);
                }
                j += 1;
            }
            i += 1;
        }
        Ok(true)
    }

    /// `7A9EA0`: alternate one-column and one-row growth until both stop.
    fn grow(&self, x: i32, y: i32, s: i32) -> Result<(i32, i32)> {
        let kind = self.tile_type_at(x, y)?;
        let (mut w, mut h) = (s, s);
        let mut next_x = w.wrapping_add(x);
        let mut next_y = y.wrapping_add(s);
        let (mut done_x, mut done_y) = (false, false);
        loop {
            if done_x && done_y {
                return Ok((w, h));
            }
            if next_x >= self.width {
                done_x = true;
            }
            if next_y >= self.height {
                done_y = true;
            }
            if !done_x {
                let mut ok = true;
                let mut j = y;
                while j < next_y {
                    if self.rect_at(next_x, j)?.is_some() || self.tile_type_at(next_x, j)? != kind {
                        ok = false;
                        break;
                    }
                    j += 1;
                }
                if ok {
                    w = w.wrapping_add(1);
                    next_x = next_x.wrapping_add(1);
                } else {
                    done_x = true;
                }
            }
            if done_y {
                continue;
            }
            let mut ok = true;
            let mut i = x;
            while i < next_x {
                if self.rect_at(i, next_y)?.is_some() || self.tile_type_at(i, next_y)? != kind {
                    ok = false;
                    break;
                }
                i += 1;
            }
            if ok {
                h = h.wrapping_add(1);
                next_y = next_y.wrapping_add(1);
            } else {
                done_y = true;
            }
        }
    }

    /// `7B9B80` on `owner`: head-insert a side naming `other`.
    fn add_side(&mut self, owner: u32, other: u32, side: usize, b: [i32; 4]) -> Result<()> {
        let list = &mut self.rects[owner as usize].sides[side];
        list.try_reserve(1)?;
        list.push(Side {
            bounds: b.map(|v| v as u16),
            rectangle: other,
        });
        Ok(())
    }

    /// One `7AACD0` scan: maximal runs of equal non-null tile rectangles.
    #[allow(clippy::too_many_arguments)]
    fn link_run(
        &mut self,
        id: u32,
        horizontal: bool,
        fixed: i32,
        from: i32,
        to: i32,
        own_side: usize,
        own_line: i32,
        other_side: usize,
    ) -> Result<()> {
        let at = |b: &Self, v: i32| {
            if horizontal {
                b.rect_at(v, fixed)
            } else {
                b.rect_at(fixed, v)
            }
        };
        let mut v = from;
        loop {
            match at(self, v)? {
                None => v += 1,
                Some(other) => {
                    let start = v;
                    while v <= to && at(self, v)? == Some(other) {
                        v += 1;
                    }
                    let end = v - 1;
                    if horizontal {
                        self.add_side(id, other, own_side, [start, end, own_line, own_line])?;
                        self.add_side(other, id, other_side, [start, end, fixed, fixed])?;
                    } else {
                        self.add_side(id, other, own_side, [own_line, own_line, start, end])?;
                        self.add_side(other, id, other_side, [fixed, fixed, start, end])?;
                    }
                }
            }
            if v > to {
                return Ok(());
            }
        }
    }

    /// `7AACD0`: link sides above (0), below (2), left (3), right (1).
    fn link_sides(&mut self, id: u32, x0: i32, x1: i32, y0: i32, y1: i32) -> Result<()> {
        let above = y0.wrapping_sub(1);
        if above >= 0 && x0 <= x1 {
            self.link_run(id, true, above, x0, x1, 0, y0, 2)?;
        }
        let below = y1.wrapping_add(1);
        if below < self.height && x0 <= x1 {
            self.link_run(id, true, below, x0, x1, 2, y1, 0)?;
        }
        let left = x0.wrapping_sub(1);
        if left >= 0 && y0 <= y1 {
            self.link_run(id, false, left, y0, y1, 3, x0, 1)?;
        }
        let right = x1.wrapping_add(1);
        if right < self.width && y0 <= y1 {
            self.link_run(id, false, right, y0, y1, 1, x1, 3)?;
        }
        Ok(())
    }

    /// `7B99B0`: head-insert the rectangle and set rectangle +4.
    fn add_to_region(&mut self, region: u32, rect: u32) -> Result<()> {
        let list = &mut self.regions[region as usize].rects;
        list.try_reserve(1)?;
        list.push(rect);
        self.rects[rect as usize].region = region as i32;
        Ok(())
    }
    fn new_region(&mut self) -> Result<u32> {
        let id = u32::try_from(self.regions.len()).map_err(|_| Error::Input("region count"))?;
        self.regions.try_reserve(1)?;
        self.regions.push(Region {
            rects: Vec::new(),
            center: [0xbf80_0000; 2],
            table: DistanceTable::new(),
        });
        Ok(id)
    }

    /// `7ABD40`: split the grown square, publish tiles, link, classify.
    fn split(&mut self, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
        let width = self.params.max_rectangle_width;
        let nx = idiv(w.wrapping_sub(1), width, "7ABDFE")?.wrapping_add(1);
        let ny = idiv(h.wrapping_sub(1), width, "7ABE09")?.wrapping_add(1);
        if nx < 1 {
            return Ok(());
        }
        let mut x_start = x;
        let mut acc_w = w;
        for _ in 0..nx {
            let x1 = idiv(acc_w, nx, "7ABE35")?.wrapping_add(x).wrapping_sub(1);
            if ny >= 1 {
                let mut y_start = y;
                let mut acc_h = h;
                // Native reloads x from the last fill counter (+14) after the
                // row loop: x1+1 when the column range is non-empty.
                let mut next_x = x_start;
                for _ in 0..ny {
                    let y1 = idiv(acc_h, ny, "7ABE65")?.wrapping_add(y).wrapping_sub(1);
                    let id = u32::try_from(self.rects.len())
                        .map_err(|_| Error::Input("rectangle count"))?;
                    self.rects.try_reserve(1)?;
                    self.rects.push(Rect {
                        region: -1,
                        island: -1,
                        bounds: [y_start as i16, x1 as i16, y1 as i16, x_start as i16],
                        sides: std::array::from_fn(|_| Vec::new()),
                    });
                    let mut next_y = y_start;
                    let mut xx = x_start;
                    while xx <= x1 {
                        let mut yy = y_start;
                        while yy <= y1 {
                            let i = self.index(xx, yy)?;
                            self.tile_rects[i] = Some(id);
                            yy += 1;
                        }
                        next_y = yy;
                        xx += 1;
                    }
                    self.link_sides(id, x_start, x1, y_start, y1)?;
                    let b = self.rects[id as usize].bounds;
                    let rw = i32::from(b[1].wrapping_sub(b[3]).wrapping_add(1));
                    let rh = i32::from(b[2].wrapping_sub(b[0]).wrapping_add(1));
                    let p = self.params;
                    if rw * rh < p.min_region_size
                        || rh < p.min_region_width
                        || rw < p.min_region_width
                    {
                        self.small.try_reserve(1)?;
                        self.small.insert(0, id);
                    } else {
                        let region = self.new_region()?;
                        self.add_to_region(region, id)?;
                    }
                    acc_h = acc_h.wrapping_add(h);
                    y_start = next_y;
                    next_x = xx;
                }
                x_start = next_x;
            }
            acc_w = acc_w.wrapping_add(w);
        }
        Ok(())
    }

    /// `7ACB00` square loop.
    fn place_squares(&mut self) -> Result<()> {
        let mut remaining = self.width.wrapping_mul(self.height);
        let mut size = self.params.starting_square_size;
        let mut seen = BTreeSet::new();
        while size > 0 {
            if !seen.insert(size) {
                return Err(Error::NonTerminating("7A9B00 square size cycles"));
            }
            if size.wrapping_mul(size) <= remaining {
                let last_x = self.width.wrapping_sub(size);
                let last_y = self.height.wrapping_sub(size);
                let mut x = 0;
                while x <= last_x {
                    let mut y = 0;
                    while y <= last_y {
                        if self.rect_at(x, y)?.is_none() {
                            let kind = self.tile_type_at(x, y)?;
                            if self.square_free(x, y, size, kind)? {
                                let (w, h) = self.grow(x, y, size)?;
                                self.split(x, y, w, h)?;
                                remaining = remaining.wrapping_sub(w.wrapping_mul(h));
                            }
                        }
                        y += 1;
                    }
                    x += 1;
                }
            }
            size = next_square_size(size, self.params)?;
        }
        Ok(())
    }

    /// `7AB7A0` with `7AA8B0`; recursion replaced by an explicit stack.
    fn flood_islands(&mut self) -> Result<()> {
        let n = self.rects.len();
        let mut visited = vec![false; n];
        let mut count = 0usize;
        if n == 0 {
            return Ok(());
        }
        loop {
            let start = (0..n)
                .find(|&i| !visited[i])
                .ok_or(Error::Native("7AB837 scans past the visited array"))?;
            let island = self.islands.len();
            self.islands.try_reserve(1)?;
            self.islands.push(Island {
                regions: Vec::new(),
            });
            // Frame: rectangle type, live side entries, next entry.
            let mut stack: Vec<(u32, Vec<Side>, usize)> = Vec::new();
            stack.push(self.visit(start as u32, island, &mut visited, &mut count)?);
            while let Some(top) = stack.last_mut() {
                let Some(side) = top.1.get(top.2) else {
                    stack.pop();
                    continue;
                };
                top.2 += 1;
                let (other, kind) = (side.rectangle, top.0);
                // Tested when reached: deeper visits change `visited`.
                if !visited[other as usize] && self.rect_type(other)? == kind {
                    let frame = self.visit(other, island, &mut visited, &mut count)?;
                    stack.try_reserve(1)?;
                    stack.push(frame);
                }
            }
            if count >= n {
                return Ok(());
            }
        }
    }
    /// Body of `7AA8B0` before its side loop. A rectangle with a region marks
    /// every rectangle of that region, but only its own sides are explored.
    fn visit(
        &mut self,
        rect: u32,
        island: usize,
        visited: &mut [bool],
        count: &mut usize,
    ) -> Result<(u32, Vec<Side>, usize)> {
        let kind = self.rect_type(rect)?;
        let region = self.rects[rect as usize].region;
        if region >= 0 {
            self.islands[island].regions.try_reserve(1)?;
            self.islands[island].regions.push(region as u32);
            for r in self.region_rects(region as u32) {
                self.rects[r as usize].island = island as i32;
                *count += 1;
                visited[r as usize] = true;
            }
        } else {
            self.rects[rect as usize].island = island as i32;
            *count += 1;
            visited[rect as usize] = true;
        }
        Ok((kind, self.side_entries(rect), 0))
    }

    /// `7AA2A0`: largest land (type 0) island by binary32-stored area.
    fn primary_island(&self) -> Result<Option<u32>> {
        let p = self.p;
        let mut best = Binary::from_f32_bits(0)?;
        let mut primary = None;
        for (id, island) in self.islands.iter().enumerate() {
            let Some(&first) = island.regions.first() else {
                continue;
            };
            let head = *self
                .region_rects(first)
                .first()
                .ok_or(Error::Native("7AA314 dereferences an empty region list"))?;
            if self.rect_type(head)? != 0 {
                continue;
            }
            let mut area = Binary::from_f32_bits(0)?;
            for &region in &island.regions {
                for r in self.region_rects(region) {
                    area = area.add(
                        &Binary::from_i32(rect_area(self.rects[r as usize].bounds)),
                        p,
                    )?;
                }
            }
            if area.compare(&best) == Ordering::Greater {
                best = Binary::from_f32_bits(area.store_f32()?)?;
                primary = Some(id as u32);
            }
        }
        Ok(primary)
    }

    /// `7AB320`: merge the small-rectangle list until it is empty.
    fn merge_small(&mut self) -> Result<()> {
        let mut at = 0usize;
        let mut idle = 0usize;
        while !self.small.is_empty() {
            let rect = self.small[at];
            let kind = self.rect_type(rect)?;
            let mut candidates: Vec<u32> = Vec::new();
            let mut weights: Vec<i32> = Vec::new();
            let mut too_big = false;
            for side in self.side_entries(rect) {
                let other = side.rectangle;
                let region = self.rects[other as usize].region;
                if region < 0 || self.rect_type(other)? != kind {
                    continue;
                }
                let region = region as u32;
                if self.region_area(region) > self.params.max_region_size_for_expansion {
                    too_big = true;
                    continue;
                }
                let length = side_length(&side);
                match candidates.iter().rposition(|&r| r == region) {
                    Some(i) => weights[i] = weights[i].wrapping_add(length),
                    None => {
                        candidates.try_reserve(1)?;
                        weights.try_reserve(1)?;
                        candidates.push(region);
                        weights.push(length);
                    }
                }
            }
            let removed = if !candidates.is_empty() {
                let mut best_weight = -1i32;
                let mut best = None;
                let mut best_area: Option<i32> = None;
                for i in (0..candidates.len()).rev() {
                    let w = weights[i];
                    let take = if w > best_weight {
                        true
                    } else if w == best_weight {
                        let known = best_area
                            .ok_or(Error::Native("7AB590 compares an uninitialized area"))?;
                        self.region_area(candidates[i]) < known
                    } else {
                        false
                    };
                    if take {
                        best_weight = w;
                        best = Some(i);
                        best_area = Some(self.region_area(candidates[i]));
                    }
                }
                let i = best.ok_or(Error::Native("7AB5BC indexes past the candidates"))?;
                self.add_to_region(candidates[i], rect)?;
                true
            } else {
                let island = self.rects[rect as usize].island;
                let island = usize::try_from(island)
                    .ok()
                    .filter(|&i| i < self.islands.len())
                    .ok_or(Error::Native("7AB5E7 reads an invalid island"))?;
                if !too_big && !self.islands[island].regions.is_empty() {
                    false
                } else {
                    let region = self.new_region()?;
                    self.islands[island].regions.try_reserve(1)?;
                    self.islands[island].regions.push(region);
                    self.add_to_region(region, rect)?;
                    true
                }
            };
            if removed {
                self.small.remove(at);
                idle = 0;
            } else {
                at += 1;
                idle += 1;
                if idle >= self.small.len() {
                    return Err(Error::NonTerminating(
                        "7AB320 cycles over small rectangles without progress",
                    ));
                }
            }
            if at >= self.small.len() {
                at = 0;
            }
        }
        Ok(())
    }

    /// `54EDC0`: binary32 world center of tile (x, y).
    fn tile_center(&self, x: i32, y: i32) -> Result<[Binary; 2]> {
        let p = self.p;
        let shift = self.grid.layout.coordinate_shift;
        let half = Binary::from_f32_bits(self.grid.cell_size_bits)?
            .mul(&Binary::from_f32_bits(0x3f00_0000)?, p)?;
        let cx = Binary::from_i32(x.wrapping_shl(shift)).add(&half, p)?;
        let cy = Binary::from_i32(y.wrapping_shl(shift)).add(&half, p)?;
        Ok([
            Binary::from_f32_bits(cx.store_f32()?)?,
            Binary::from_f32_bits(cy.store_f32()?)?,
        ])
    }

    /// `7BC590`: area-weighted corner center, then neighbor distances.
    fn finish_region(&mut self, region: u32) -> Result<()> {
        let p = self.p;
        let mut sum = [Binary::from_f32_bits(0)?, Binary::from_f32_bits(0)?];
        let mut count = 0i32;
        for r in self.region_rects(region) {
            let [y0, x1, y1, x0] = self.rects[r as usize].bounds.map(i32::from);
            let corners = [
                self.tile_center(x1, y1)?,
                self.tile_center(x0, y1)?,
                self.tile_center(x1, y0)?,
                self.tile_center(x0, y0)?,
            ];
            let area = rect_area(self.rects[r as usize].bounds);
            count = count.wrapping_add(area.wrapping_mul(4));
            let weight = Binary::from_i32(area);
            for (axis, total) in sum.iter_mut().enumerate() {
                let mut s = corners[0][axis].add(&corners[1][axis], p)?;
                s = s.add(&corners[2][axis], p)?;
                s = s.add(&corners[3][axis], p)?;
                let next = s.mul(&weight, p)?.add(total, p)?;
                *total = Binary::from_f32_bits(next.store_f32()?)?;
            }
        }
        let inverse = Binary::from_f32_bits(0x3f80_0000)?.div(&Binary::from_i32(count), p)?;
        let center = [
            sum[0].mul(&inverse, p)?.store_f32()?,
            inverse.mul(&sum[1], p)?.store_f32()?,
        ];
        self.regions[region as usize].center = center;
        let zero = Binary::from_f32_bits(0)?;
        let (cx, cy) = (
            Binary::from_f32_bits(center[0])?,
            Binary::from_f32_bits(center[1])?,
        );
        for r in self.region_rects(region) {
            for side in self.side_entries(r) {
                let other = self.rects[side.rectangle as usize].region;
                let other = u32::try_from(other)
                    .map_err(|_| Error::Native("7BC776 dereferences a null neighbor region"))?;
                if other == region {
                    continue;
                }
                let o = self.regions[other as usize].center;
                let ox = Binary::from_f32_bits(o[0])?;
                if ox.compare(&zero) == Ordering::Less
                    || self.regions[region as usize].table.contains(other)
                {
                    continue;
                }
                let dx = Binary::from_f32_bits(cx.sub(&ox, p)?.store_f32()?)?;
                let dy = cy.sub(&Binary::from_f32_bits(o[1])?, p)?;
                let distance = dy
                    .mul(&dy, p)?
                    .add(&dx.mul(&dx, p)?, p)?
                    .sqrt(p)?
                    .store_f32()?;
                self.regions[region as usize]
                    .table
                    .insert(other, distance)?;
                self.regions[other as usize]
                    .table
                    .insert(region, distance)?;
            }
        }
        Ok(())
    }
}

fn validate(grid: &StaticGrid) -> Result<usize> {
    let l = grid.layout;
    if l.width <= 0 || l.height <= 0 || l.coordinate_shift > 30 {
        return Err(Error::Input("invalid grid dimensions"));
    }
    // Rectangle bounds are native signed 16-bit fields (retained format).
    if l.width > i32::from(i16::MAX) || l.height > i32::from(i16::MAX) {
        return Err(Error::Input(
            "grid exceeds native signed 16-bit rectangle bounds",
        ));
    }
    for (extent, cells) in l.extent_bits.into_iter().zip([l.width, l.height]) {
        let b = Binary::from_f32_bits(extent)?;
        if b.is_negative_nonzero() || b.truncate_i32()? >> l.coordinate_shift != cells {
            return Err(Error::Input("grid dimensions differ from stored extent"));
        }
    }
    Binary::from_f32_bits(grid.cell_size_bits)?;
    let count = (l.width as usize)
        .checked_mul(l.height as usize)
        .ok_or(Error::Input("grid extent overflow"))?;
    if grid.tiles.len() != count {
        return Err(Error::Input("tile count differs from grid"));
    }
    Ok(count)
}

/// Complete `7ACB00` on a cleared SAI map. The function has no external
/// effects; an error means native would fault, assert or never return, and no
/// topology is produced (the caller must not substitute one).
pub fn build(
    grid: &StaticGrid,
    params: &StaticMapParameters,
    precision: Precision,
) -> Result<StaticAnalysis> {
    let count = validate(grid)?;
    let mut tile_rects = Vec::new();
    tile_rects.try_reserve_exact(count)?;
    tile_rects.resize(count, None);
    let mut b = Builder {
        grid,
        params,
        p: precision,
        width: grid.layout.width,
        height: grid.layout.height,
        tile_rects,
        rects: Vec::new(),
        regions: Vec::new(),
        islands: Vec::new(),
        small: Vec::new(),
    };
    b.place_squares()?;
    b.flood_islands()?;
    let primary_island = b.primary_island()?;
    let mut topology = Topology {
        layout: grid.layout,
        enabled: true,
        regions: Vec::new(),
        rectangles: Vec::new(),
        region_rectangles: Vec::new(),
        tile_rectangles: Vec::new(),
    };
    let diagonal_bits = topology.diagonal_bits(precision)?;
    b.merge_small()?;
    for region in (0..b.regions.len()).rev() {
        b.finish_region(region as u32)?;
    }
    let rectangle_count_warning = b.rects.len() > 0x1000;
    topology.regions.try_reserve_exact(b.regions.len())?;
    topology
        .region_rectangles
        .try_reserve_exact(b.regions.len())?;
    for id in 0..b.regions.len() {
        topology.region_rectangles.push(b.region_rects(id as u32));
        let region = &b.regions[id];
        topology.regions.push(SavedRegion {
            position_bits: region.center,
            distance_pairs: region.table.pairs(),
        });
    }
    topology.rectangles.try_reserve_exact(b.rects.len())?;
    for rect in &b.rects {
        topology.rectangles.push(Rectangle {
            region: rect.region,
            island: rect.island,
            bounds: rect.bounds,
            sides: std::array::from_fn(|s| rect.sides[s].iter().rev().cloned().collect()),
        });
    }
    topology.tile_rectangles = b.tile_rects;
    Ok(StaticAnalysis {
        topology,
        islands: b.islands,
        post_load: PostLoad {
            primary_island,
            diagonal_bits,
        },
        rectangle_count_warning,
    })
}
