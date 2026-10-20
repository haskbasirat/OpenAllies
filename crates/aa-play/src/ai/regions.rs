//! The SAI map: regions, their neighbours and the dynamic influence analysis.
//!
//! Native: `KKC_SAIMap::StaticMapAnalysis` (`7ACB00`) partitions the land into
//! rectangles grown from squares of `StaticMapStartingSquareSize` tiles and
//! groups them into regions; `DynamicMapAnalysis` (`7AC0D0`, ported exactly in
//! `aa_mind::sai::map_analysis`) spreads every live actor's combat value over
//! the regions every `dynamic_map_analysis_think_frequency` seconds:
//! mobile actors within `min(speed, DynamicMapAnalysisSpeedCap) *
//! cv_influence_max_travel_time` metres, static actors within their
//! settlement range (times `settlement_cv_range_fudge_factor`) and within the
//! ego `border_influence_range`, with the factor
//! `max(1 - d/range*(1-min_cv_propogation_factor), min)^cv_influence_exponent`.
//!
//! Behaviour-faithful simplification (listed under "compare against the
//! original"): regions are the square blocks of `StaticMapStartingSquareSize`
//! tiles (no rectangle growth/merging), neighbours are the 8 surrounding
//! blocks, and the spread uses straight-line distances between region
//! centres instead of the neighbour-graph recursion `7AA620`.
use crate::map::{BLOCKED, GameMap, NOT_LAND_PASSABLE};
use glam::Vec2;

#[derive(Clone, Debug, Default)]
pub struct Regions {
    /// Region side in metres.
    pub size: f32,
    pub nx: u32,
    pub ny: u32,
    /// Representative land point of each region (a passable cell centre
    /// nearest the block centre, or the block centre).
    pub centers: Vec<Vec2>,
    pub passable: Vec<bool>,
    /// Land-connected component of each region (`u32::MAX` = no land).
    pub island: Vec<u32>,
    /// Number of land cells (the region area `+C`).
    pub area: Vec<u32>,
}

impl Regions {
    pub fn build(map: &GameMap, tiles_per_region: u32) -> Self {
        let tiles = tiles_per_region.max(1);
        let cell = if map.cell_size > 0.0 { map.cell_size } else { 1.0 };
        let (w, h) = (map.width.max(1), map.height.max(1));
        let nx = w.div_ceil(tiles);
        let ny = h.div_ceil(tiles);
        let count = (nx * ny) as usize;
        let mut centers = vec![Vec2::ZERO; count];
        let mut passable = vec![false; count];
        let mut area = vec![0u32; count];
        let land = |x: u32, y: u32| -> bool {
            if map.flags.is_empty() {
                return true;
            }
            let f = map.flags[(y * map.width + x) as usize];
            f & (BLOCKED | NOT_LAND_PASSABLE) == 0
        };
        for ry in 0..ny {
            for rx in 0..nx {
                let i = (ry * nx + rx) as usize;
                let (x0, y0) = (rx * tiles, ry * tiles);
                let (x1, y1) = ((x0 + tiles).min(w), (y0 + tiles).min(h));
                let mid = Vec2::new((x0 + x1) as f32 * 0.5 * cell, (y0 + y1) as f32 * 0.5 * cell);
                let mut best: Option<(f32, Vec2)> = None;
                for y in y0..y1 {
                    for x in x0..x1 {
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
        let mut regions = Regions { size: tiles as f32 * cell, nx, ny, centers, passable, island: vec![u32::MAX; count], area };
        regions.flood_islands();
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
                let around: Vec<u32> = self.neighbors(r).collect();
                for n in around {
                    if self.passable[n as usize] && self.island[n as usize] == u32::MAX {
                        self.island[n as usize] = next;
                        stack.push(n);
                    }
                }
            }
            next += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.centers.len()
    }
    pub fn is_empty(&self) -> bool {
        self.centers.is_empty()
    }
    pub fn coords(&self, r: u32) -> (i32, i32) {
        ((r % self.nx) as i32, (r / self.nx) as i32)
    }
    pub fn region_at(&self, p: Vec2) -> u32 {
        let x = ((p.x / self.size).floor() as i64).clamp(0, self.nx as i64 - 1) as u32;
        let y = ((p.y / self.size).floor() as i64).clamp(0, self.ny as i64 - 1) as u32;
        y * self.nx + x
    }
    pub fn neighbors(&self, r: u32) -> impl Iterator<Item = u32> + '_ {
        let (x, y) = self.coords(r);
        (-1..=1).flat_map(move |dy| (-1..=1).map(move |dx| (dx, dy))).filter_map(move |(dx, dy)| {
            if dx == 0 && dy == 0 {
                return None;
            }
            let (nx, ny) = (x + dx, y + dy);
            (nx >= 0 && ny >= 0 && nx < self.nx as i32 && ny < self.ny as i32).then(|| ny as u32 * self.nx + nx as u32)
        })
    }
    /// Region hops (8-connected grid distance).
    pub fn hops(&self, a: u32, b: u32) -> u32 {
        let (ax, ay) = self.coords(a);
        let (bx, by) = self.coords(b);
        (ax - bx).unsigned_abs().max((ay - by).unsigned_abs())
    }
    /// Passable regions within `depth` hops of `r` (`7BB1C0`), nearest first.
    pub fn within(&self, r: u32, depth: u32) -> Vec<u32> {
        let (x, y) = self.coords(r);
        let d = depth as i32;
        let mut out = Vec::new();
        for ring in 0..=d {
            for dy in -ring..=ring {
                for dx in -ring..=ring {
                    if dx.abs().max(dy.abs()) != ring {
                        continue;
                    }
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && ny >= 0 && nx < self.nx as i32 && ny < self.ny as i32 {
                        let n = ny as u32 * self.nx + nx as u32;
                        if self.passable[n as usize] {
                            out.push(n);
                        }
                    }
                }
            }
        }
        out
    }
    pub fn same_island(&self, a: u32, b: u32) -> bool {
        let (ia, ib) = (self.island[a as usize], self.island[b as usize]);
        ia != u32::MAX && ia == ib
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
        for src in sources {
            if src.cv <= 0.0 || src.owner >= kingdoms {
                continue;
            }
            let home = regions.region_at(src.pos);
            if src.mobile {
                spread(regions, home, src, src.range, s, &mut inf.mobile, kingdoms);
            } else {
                spread(regions, home, src, src.range * s.settlement_fudge, s, &mut inf.settlement, kingdoms);
                spread(regions, home, src, s.border_range, s, &mut inf.border, kingdoms);
            }
        }
        inf
    }
}

fn spread(regions: &Regions, home: u32, src: &Source, range: f32, s: SpreadSettings, out: &mut [f32], kingdoms: usize) {
    let range = range.max(0.0);
    let depth = (range / regions.size.max(1e-3)).ceil() as u32;
    for r in regions.within(home, depth) {
        if !regions.same_island(home, r) && r != home {
            continue;
        }
        let d = regions.centers[r as usize].distance(src.pos);
        if d > range && r != home {
            continue;
        }
        let factor = if range > 0.0 { (1.0 - d / range * (1.0 - s.min_factor)).max(s.min_factor) } else { 1.0 };
        out[r as usize * kingdoms + src.owner] += src.cv * factor.powf(s.exponent);
    }
}
