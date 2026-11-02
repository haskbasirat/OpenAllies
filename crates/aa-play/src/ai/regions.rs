//! The SAI map: regions, their neighbours and the dynamic influence analysis.
//!
//! Native: `KKC_SAIMap::StaticMapAnalysis` (`7ACB00`, ported exactly in
//! `aa_navigation::topology_builder`) partitions the tiles into rectangles
//! grown from squares of `StaticMapStartingSquareSize` tiles, groups them
//! into regions and islands, and gives every region its centre and the
//! distances to its neighbours (`7BC590`, the region `+5C` hash).
//! `DynamicMapAnalysis` (`7AC0D0`, ported exactly in
//! `aa_mind::sai::map_analysis`) spreads every live actor's combat value
//! over the regions every `dynamic_map_analysis_think_frequency` seconds
//! with the recursive propagation `7AA620` over those neighbour distances,
//! through land regions only (`45D720` of the first rectangle's terrain):
//! mobile actors within `min(speed, DynamicMapAnalysisSpeedCap) *
//! cv_influence_max_travel_time` metres, static actors within their
//! settlement range (times `settlement_cv_range_fudge_factor`, never past
//! the unfudged range) and within the ego `border_influence_range`, with
//! the factor `max(1 - d/range*(1-min_cv_propogation_factor),
//! min)^cv_influence_exponent`; the source region takes the whole value.
//!
//! The spread here is that algorithm in plain `f32` on the exact topology.
//! A map the builder refuses (synthetic test maps, whose cell size is no
//! power of two) falls back to square blocks of the starting square size with
//! 8-neighbour links; [`Regions::exact`] tells them apart.
use crate::map::{BLOCKED, GameMap, NOT_LAND_PASSABLE};
use glam::Vec2;
use std::collections::VecDeque;

/// The SVars `7ACB00` reads (Data/Game/SVars.tgi), as the random map
/// generator's analysis reads them.
#[derive(Clone, Copy, Debug)]
pub struct StaticSvars {
    pub starting_square_size: i32,
    pub use_fast_square_reduction: bool,
    pub fast_square_reduction_mult: i32,
    pub fast_square_reduction_div: i32,
    pub min_region_size: i32,
    pub min_region_width: i32,
    pub max_region_size_for_expansion: i32,
    pub max_rectangle_width: i32,
    pub base_map_width: i32,
    pub max_rectangle_width_exp: f32,
}

impl Default for StaticSvars {
    /// The shipped SVars.tgi values.
    fn default() -> Self {
        StaticSvars {
            starting_square_size: 10,
            use_fast_square_reduction: true,
            fast_square_reduction_mult: 3,
            fast_square_reduction_div: 4,
            min_region_size: 9,
            min_region_width: 3,
            max_region_size_for_expansion: 1000,
            max_rectangle_width: 12,
            base_map_width: 64,
            max_rectangle_width_exp: f32::from_bits(0x3eaa_a64c),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Regions {
    /// The nominal region side in metres (starting square size times the
    /// tile edge), for distance tolerances.
    pub size: f32,
    /// Region centre (`7BC590`, region +4/+8).
    pub centers: Vec<Vec2>,
    /// Land region (terrain type 0 of its first rectangle, `7B9320`).
    pub passable: Vec<bool>,
    /// Island of each region (`u32::MAX` = none).
    pub island: Vec<u32>,
    /// The primary island (`[B3652C]+2C`, `7AA2A0`).
    pub primary: Option<u32>,
    /// `7ACB00` built this; false for the square-block fallback.
    pub exact: bool,
    /// Number of tiles (the region area `+C`).
    pub area: Vec<u32>,
    /// Neighbour distances (region `+5C`), in the hash's bucket order.
    pub neighbours: Vec<Vec<(u32, f32)>>,
    /// The region's rectangles as tile bounds (min x, min y, max x, max y),
    /// inclusive.
    pub rects: Vec<Vec<[i32; 4]>>,
    /// Region of every tile; tiles without a rectangle take the nearest
    /// region's (multi-source breadth-first fill).
    tile_region: Vec<u32>,
    width: u32,
    height: u32,
    cell: f32,
    /// Hop counts between regions over the neighbour graph (`u16::MAX` =
    /// not connected).
    hops: Vec<u16>,
}

impl Regions {
    /// `7ACB00` on the map's tile flags.
    pub fn build(map: &GameMap, sv: StaticSvars) -> Self {
        match Self::analyse(map, sv) {
            Some(r) => r,
            None => Self::squares(map, sv.starting_square_size.max(1) as u32),
        }
    }

    fn analyse(map: &GameMap, sv: StaticSvars) -> Option<Self> {
        use aa_navigation::topology::{GridLayout, StaticTile};
        use aa_navigation::topology_builder::{StaticGrid, StaticMapParameters, build, terrain_type};
        let (w, h) = (map.width, map.height);
        if w == 0 || h == 0 || map.flags.len() != (w * h) as usize {
            return None;
        }
        let cell = if map.cell_size > 0.0 { map.cell_size } else { return None };
        let shift = cell.log2();
        if shift.fract() != 0.0 || shift < 0.0 {
            return None;
        }
        let shift = shift as u32;
        let tiles: Vec<StaticTile> = map
            .flags
            .iter()
            .map(|&f| StaticTile { flags20: f, definition_type: (f & crate::map::TERRAIN_TYPE_MASK) >> crate::map::TERRAIN_TYPE_SHIFT })
            .collect();
        let extent = [(w as f32 * cell).to_bits(), (h as f32 * cell).to_bits()];
        let layout = GridLayout { width: w as i32, height: h as i32, coordinate_shift: shift, extent_bits: extent };
        // 7AA180: trunc(floor(max * pow((W + H) * 0.5 / base, exp) + 0.5)).
        let mean = (f64::from(w) + f64::from(h)) * 0.5 / f64::from(sv.base_map_width.max(1) as f32);
        let width = (mean.powf(f64::from(sv.max_rectangle_width_exp)) * f64::from(sv.max_rectangle_width as f32) + 0.5).floor() as i32;
        let params = StaticMapParameters {
            starting_square_size: sv.starting_square_size,
            use_fast_square_reduction: sv.use_fast_square_reduction,
            fast_square_reduction_mult: sv.fast_square_reduction_mult,
            fast_square_reduction_div: sv.fast_square_reduction_div,
            min_region_size: sv.min_region_size,
            min_region_width: sv.min_region_width,
            max_region_size_for_expansion: sv.max_region_size_for_expansion,
            max_rectangle_width: width,
        };
        let grid = StaticGrid { layout, cell_size_bits: cell.to_bits(), tiles: &tiles };
        let a = build(&grid, &params, aa_numeric::Precision::Double53).ok()?;
        let t = &a.topology;
        let n = t.regions.len();
        if n == 0 {
            return None;
        }
        let mut rects = vec![Vec::new(); n];
        let mut island = vec![u32::MAX; n];
        let mut area = vec![0u32; n];
        let mut passable = vec![false; n];
        for (r, list) in t.region_rectangles.iter().enumerate() {
            for (k, &ri) in list.iter().enumerate() {
                let rect = &t.rectangles[ri as usize];
                // Native +C,+E,+10,+12: minY, maxX, maxY, minX.
                let b = [i32::from(rect.bounds[3]), i32::from(rect.bounds[0]), i32::from(rect.bounds[1]), i32::from(rect.bounds[2])];
                area[r] += ((b[2] - b[0] + 1).max(0) * (b[3] - b[1] + 1).max(0)) as u32;
                if rect.island >= 0 {
                    island[r] = rect.island as u32;
                }
                if k == 0 {
                    // 7B9320: the terrain of the first rectangle's (min X,
                    // min Y) tile; 45D720: land is type 0.
                    let ti = (b[1].max(0) as u32 * w + b[0].max(0) as u32) as usize;
                    passable[r] = tiles.get(ti).is_some_and(|s| terrain_type(s) == 0);
                }
                rects[r].push(b);
            }
        }
        let centers: Vec<Vec2> = t.regions.iter().map(|r| Vec2::new(f32::from_bits(r.position_bits[0]), f32::from_bits(r.position_bits[1]))).collect();
        let neighbours: Vec<Vec<(u32, f32)>> =
            t.regions.iter().map(|r| r.distance_pairs.iter().filter(|p| (p[0] as usize) < n).map(|p| (p[0], f32::from_bits(p[1]))).collect()).collect();
        let mut tile_region = vec![u32::MAX; (w * h) as usize];
        for (ti, rect) in t.tile_rectangles.iter().enumerate() {
            if let Some(ri) = rect {
                let reg = t.rectangles[*ri as usize].region;
                if reg >= 0 && (reg as usize) < n {
                    tile_region[ti] = reg as u32;
                }
            }
        }
        let mut regions = Regions {
            size: sv.starting_square_size.max(1) as f32 * cell,
            centers,
            passable,
            island,
            primary: a.post_load.primary_island,
            exact: true,
            area,
            neighbours,
            rects,
            tile_region,
            width: w,
            height: h,
            cell,
            hops: Vec::new(),
        };
        regions.fill_tiles();
        regions.hop_table();
        Some(regions)
    }

    /// The fallback partition: square blocks of `tiles` tiles, 8-neighbour
    /// links at centre distances, land-connected islands.
    pub fn squares(map: &GameMap, tiles: u32) -> Self {
        let tiles = tiles.max(1);
        let cell = if map.cell_size > 0.0 { map.cell_size } else { 1.0 };
        let (w, h) = (map.width.max(1), map.height.max(1));
        let nx = w.div_ceil(tiles);
        let ny = h.div_ceil(tiles);
        let count = (nx * ny) as usize;
        let mut centers = vec![Vec2::ZERO; count];
        let mut passable = vec![false; count];
        let mut area = vec![0u32; count];
        let mut rects = vec![Vec::new(); count];
        let land = |x: u32, y: u32| -> bool {
            if map.flags.is_empty() {
                return true;
            }
            let f = map.flags[(y * map.width + x) as usize];
            f & (BLOCKED | NOT_LAND_PASSABLE) == 0
        };
        let mut tile_region = vec![0u32; (w * h) as usize];
        for ry in 0..ny {
            for rx in 0..nx {
                let i = (ry * nx + rx) as usize;
                let (x0, y0) = (rx * tiles, ry * tiles);
                let (x1, y1) = ((x0 + tiles).min(w), (y0 + tiles).min(h));
                rects[i].push([x0 as i32, y0 as i32, x1 as i32 - 1, y1 as i32 - 1]);
                let mid = Vec2::new((x0 + x1) as f32 * 0.5 * cell, (y0 + y1) as f32 * 0.5 * cell);
                let mut best: Option<(f32, Vec2)> = None;
                for y in y0..y1 {
                    for x in x0..x1 {
                        tile_region[(y * w + x) as usize] = i as u32;
                        if x < map.width && y < map.height && land(x, y) {
                            area[i] += 1;
                            let c = Vec2::new((x as f32 + 0.5) * cell, (y as f32 + 0.5) * cell);
                            let d = c.distance_squared(mid);
                            if best.is_none_or(|(bd, _)| d < bd) {
                                best = Some((d, c));
                            }
                        }
                    }
                }
                passable[i] = area[i] > 0;
                centers[i] = best.map_or(mid, |(_, c)| c);
            }
        }
        let mut neighbours = vec![Vec::new(); count];
        for r in 0..count as u32 {
            let (x, y) = ((r % nx) as i32, (r / nx) as i32);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (qx, qy) = (x + dx, y + dy);
                    if (dx, dy) == (0, 0) || qx < 0 || qy < 0 || qx >= nx as i32 || qy >= ny as i32 {
                        continue;
                    }
                    let q = qy as u32 * nx + qx as u32;
                    neighbours[r as usize].push((q, centers[r as usize].distance(centers[q as usize])));
                }
            }
        }
        let mut regions = Regions {
            size: tiles as f32 * cell,
            centers,
            passable,
            island: vec![u32::MAX; count],
            primary: None,
            exact: false,
            area,
            neighbours,
            rects,
            tile_region,
            width: w,
            height: h,
            cell,
            hops: Vec::new(),
        };
        regions.flood_islands();
        regions.hop_table();
        // The land island of the largest area.
        let mut size: std::collections::BTreeMap<u32, u64> = std::collections::BTreeMap::new();
        for r in 0..count {
            if regions.island[r] != u32::MAX {
                *size.entry(regions.island[r]).or_default() += u64::from(regions.area[r]);
            }
        }
        let mut best: Option<(u32, u64)> = None;
        for (&island, &a) in &size {
            if best.is_none_or(|(_, b)| a > b) {
                best = Some((island, a));
            }
        }
        regions.primary = best.map(|b| b.0);
        regions
    }

    fn flood_islands(&mut self) {
        let mut next = 0;
        let mut stack = Vec::new();
        for start in 0..self.centers.len() {
            if !self.passable[start] || self.island[start] != u32::MAX {
                continue;
            }
            self.island[start] = next;
            stack.push(start as u32);
            while let Some(r) = stack.pop() {
                for &(n, _) in &self.neighbours[r as usize] {
                    if self.passable[n as usize] && self.island[n as usize] == u32::MAX {
                        self.island[n as usize] = next;
                        stack.push(n);
                    }
                }
            }
            next += 1;
        }
    }

    /// Tiles outside every rectangle (blocked tiles) take the region of the
    /// nearest tile that has one.
    fn fill_tiles(&mut self) {
        let (w, h) = (self.width as i32, self.height as i32);
        let mut queue: VecDeque<usize> = (0..self.tile_region.len()).filter(|&t| self.tile_region[t] != u32::MAX).collect();
        while let Some(t) = queue.pop_front() {
            let (x, y) = ((t as i32) % w, (t as i32) / w);
            let r = self.tile_region[t];
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (qx, qy) = (x + dx, y + dy);
                if qx < 0 || qy < 0 || qx >= w || qy >= h {
                    continue;
                }
                let q = (qy * w + qx) as usize;
                if self.tile_region[q] == u32::MAX {
                    self.tile_region[q] = r;
                    queue.push_back(q);
                }
            }
        }
        for t in self.tile_region.iter_mut() {
            if *t == u32::MAX {
                *t = 0;
            }
        }
    }

    /// Breadth-first hop counts between every pair of regions.
    fn hop_table(&mut self) {
        let n = self.centers.len();
        let mut hops = vec![u16::MAX; n * n];
        let mut queue = VecDeque::new();
        for s in 0..n {
            hops[s * n + s] = 0;
            queue.clear();
            queue.push_back(s);
            while let Some(r) = queue.pop_front() {
                let d = hops[s * n + r];
                for &(q, _) in &self.neighbours[r] {
                    let q = q as usize;
                    if hops[s * n + q] == u16::MAX {
                        hops[s * n + q] = d.saturating_add(1);
                        queue.push_back(q);
                    }
                }
            }
        }
        self.hops = hops;
    }

    pub fn len(&self) -> usize {
        self.centers.len()
    }
    pub fn is_empty(&self) -> bool {
        self.centers.is_empty()
    }
    /// `7AA3D0`: the region of the tile under a point (points off the map
    /// are clamped onto it).
    pub fn region_at(&self, p: Vec2) -> u32 {
        if self.tile_region.is_empty() {
            return 0;
        }
        let x = ((p.x / self.cell).floor() as i64).clamp(0, self.width as i64 - 1) as u32;
        let y = ((p.y / self.cell).floor() as i64).clamp(0, self.height as i64 - 1) as u32;
        self.tile_region[(y * self.width + x) as usize]
    }
    pub fn neighbors(&self, r: u32) -> impl Iterator<Item = u32> + '_ {
        self.neighbours.get(r as usize).into_iter().flatten().map(|&(q, _)| q)
    }
    /// Region hops over the neighbour graph (65535 when not connected).
    pub fn hops(&self, a: u32, b: u32) -> u32 {
        let n = self.centers.len();
        u32::from(self.hops.get(a as usize * n + b as usize).copied().unwrap_or(u16::MAX))
    }
    /// Land regions within `depth` hops of `r` (`7BB1C0`), nearest first.
    pub fn within(&self, r: u32, depth: u32) -> Vec<u32> {
        let n = self.centers.len();
        if r as usize >= n {
            return Vec::new();
        }
        let mut seen = vec![false; n];
        let mut out = Vec::new();
        let mut queue = VecDeque::new();
        seen[r as usize] = true;
        queue.push_back((r, 0u32));
        while let Some((q, d)) = queue.pop_front() {
            if self.passable[q as usize] {
                out.push(q);
            }
            if d >= depth {
                continue;
            }
            for &(k, _) in &self.neighbours[q as usize] {
                if !seen[k as usize] {
                    seen[k as usize] = true;
                    queue.push_back((k, d + 1));
                }
            }
        }
        out
    }
    pub fn same_island(&self, a: u32, b: u32) -> bool {
        let (ia, ib) = (self.island.get(a as usize).copied().unwrap_or(u32::MAX), self.island.get(b as usize).copied().unwrap_or(u32::MAX));
        ia != u32::MAX && ia == ib
    }
    /// The tile edge in metres.
    pub fn cell(&self) -> f32 {
        self.cell
    }
}

/// One influence source of the dynamic analysis.
#[derive(Clone, Copy, Debug)]
pub struct Source {
    /// Index into the kingdom list of the world.
    pub owner: usize,
    pub pos: Vec2,
    pub cv: f32,
    /// Mobile spread range in metres, or the static settlement range.
    pub range: f32,
    pub mobile: bool,
}

/// Per region and kingdom index: mobile (+8), settlement (+14) and border
/// (+20) influence.
#[derive(Clone, Debug, Default)]
pub struct Influence {
    pub kingdoms: usize,
    pub mobile: Vec<f32>,
    pub settlement: Vec<f32>,
    pub border: Vec<f32>,
    /// Battle time of the analysis.
    pub time: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct SpreadSettings {
    pub min_factor: f32,
    pub exponent: f32,
    pub border_range: f32,
    pub settlement_fudge: f32,
}

impl Influence {
    pub fn at(&self, values: &[f32], region: u32, kingdom: usize) -> f32 {
        values.get(region as usize * self.kingdoms + kingdom).copied().unwrap_or(0.0)
    }
    pub fn compute(regions: &Regions, kingdoms: usize, sources: &[Source], s: SpreadSettings, time: f32) -> Self {
        let n = regions.len() * kingdoms;
        let mut inf = Influence { kingdoms, mobile: vec![0.0; n], settlement: vec![0.0; n], border: vec![0.0; n], time };
        let mut sp = Spread { dist: Vec::new(), val: Vec::new() };
        for src in sources {
            // 7AC0D0: only a positive combat value spreads.
            if src.cv <= 0.0 || src.owner >= kingdoms {
                continue;
            }
            let home = regions.region_at(src.pos);
            if src.mobile {
                // 7AB940
                sp.run(regions, home, src.cv, src.range, 0.0, true, s);
                sp.add(&mut inf.mobile, kingdoms, src.owner);
            } else {
                // 7ABA70: the fudged settlement range, never past the
                // unfudged one; then the ego border range.
                sp.run(regions, home, src.cv, src.range * s.settlement_fudge, src.range, false, s);
                sp.add(&mut inf.settlement, kingdoms, src.owner);
                sp.run(regions, home, src.cv, s.border_range, 0.0, true, s);
                sp.add(&mut inf.border, kingdoms, src.owner);
            }
        }
        inf
    }
}

/// `4A8700` binary32 1e6: the "unreached" distance.
const FAR: f32 = 1.0e6;

/// The spread vectors of one source (`7AA620` distance and value).
struct Spread {
    dist: Vec<f32>,
    val: Vec<f32>,
}

impl Spread {
    /// The seed (`4A8700`/`45F180`) and the first ring of `7ABA70`/`7AB940`,
    /// then `7AA620` from each admitted neighbour.
    #[allow(clippy::too_many_arguments)]
    fn run(&mut self, regions: &Regions, home: u32, value: f32, range: f32, raw: f32, mobile: bool, s: SpreadSettings) {
        let n = regions.len();
        self.dist.clear();
        self.dist.resize(n, FAR);
        self.val.clear();
        self.val.resize(n, 0.0);
        if home as usize >= n {
            return;
        }
        self.dist[home as usize] = 0.0;
        self.val[home as usize] = value;
        for &(key, step) in &regions.neighbours[home as usize] {
            let within = mobile || step <= raw;
            if step < range && regions.passable[key as usize] && within {
                self.propagate(regions, key, step, value, range, raw, mobile, s);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn propagate(&mut self, regions: &Regions, index: u32, distance: f32, value: f32, range: f32, raw: f32, mobile: bool, s: SpreadSettings) {
        let m = s.min_factor;
        let t = 1.0 - distance / range * (1.0 - m);
        let factor = if t < m { m } else { t.min(1.0) };
        self.dist[index as usize] = distance;
        self.val[index as usize] = factor.powf(s.exponent) * value;
        for &(key, step) in &regions.neighbours[index as usize] {
            let d2 = distance + step;
            let within = mobile || d2 <= raw;
            if d2 < self.dist[key as usize] && d2 < range && regions.passable[key as usize] && within {
                self.propagate(regions, key, d2, value, range, raw, mobile, s);
            }
        }
    }

    /// `7B9050` + `7BB780`/`7BB820`: every positive value into the source
    /// owner's column.
    fn add(&self, out: &mut [f32], kingdoms: usize, owner: usize) {
        for (r, &v) in self.val.iter().enumerate() {
            if v > 0.0 {
                out[r * kingdoms + owner] += v;
            }
        }
    }
}
