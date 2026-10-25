//! The generator's working world: the GMap tile grid (tile variations), the
//! two heightmaps (base HEIGHT_MAP at world+34, mountain/water overlay
//! MOUNTAIN_HEIGHT_MAP at world+38; 0x4F27E0 creates them with the SVars
//! Elevation limits, base default ElevationDefault, overlay default 0), the
//! slope-limited height setter (0x558CE0/0x558D40 + 0x559470, the same
//! algorithm aa-heightfield::brush ports byte-exactly) and the terrain change
//! batch that enforces the biome Transition table (0x559980 / 0x5599F0 /
//! 0x559FB0 / 0x559A70).
use super::data::{BiomeDef, RmgData};
use std::collections::HashMap;

pub(super) const NONE: u32 = u32::MAX;
/// 8-neighbour order of the native tables (B322E0 / B3D610 / aa-heightfield).
pub(super) const NEIGHBORS: [[i32; 2]; 8] = [[-1, -1], [-1, 0], [-1, 1], [0, 1], [1, 1], [1, 0], [1, -1], [0, -1]];

pub(super) struct World {
    pub width: f32,
    pub height: f32,
    pub tile: f32,
    pub tile_bits: u32,
    pub tw: usize,
    pub th: usize,
    pub var: Vec<u32>,
    /// Variation painted by the current brush but not committed yet.
    pub pending: Vec<u32>,
    pub hw: usize,
    pub hh: usize,
    pub spacing: f32,
    pub base: Vec<f32>,
    pub overlay: Vec<f32>,
    pub limits: [f32; 2],
}

impl World {
    pub(super) fn new(width: f32, height: f32, data: &RmgData, initial: u32) -> Self {
        let tile_bits = data.svars.tile_bits.max(1);
        let height_bits = data.svars.height_bits.max(1);
        let ext = [width.ceil() as i32, height.ceil() as i32];
        let (tw, th) = ((ext[0] >> tile_bits) as usize, (ext[1] >> tile_bits) as usize);
        let (hw, hh) = (((ext[0] >> height_bits) + 1) as usize, ((ext[1] >> height_bits) + 1) as usize);
        let e = data.svars.elevation;
        Self {
            width,
            height,
            tile: (1u32 << tile_bits) as f32,
            tile_bits,
            tw,
            th,
            var: vec![initial; tw * th],
            pending: vec![NONE; tw * th],
            hw,
            hh,
            spacing: (1u32 << height_bits) as f32,
            base: vec![e[2]; hw * hh],
            overlay: vec![0.0; hw * hh],
            limits: [e[0], e[1]],
        }
    }
    pub(super) fn tile_of(&self, p: [f32; 2]) -> Option<usize> {
        let (x, y) = ((p[0] as i32) >> self.tile_bits, (p[1] as i32) >> self.tile_bits);
        (p[0] >= 0.0 && p[1] >= 0.0 && (x as usize) < self.tw && (y as usize) < self.th).then(|| y as usize * self.tw + x as usize)
    }
    pub(super) fn tile_xy(&self, t: usize) -> [i32; 2] {
        [(t % self.tw) as i32, (t / self.tw) as i32]
    }
    pub(super) fn tile_at(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && (x as usize) < self.tw && (y as usize) < self.th).then(|| y as usize * self.tw + x as usize)
    }
    /// Tile centre (0x54EDC0).
    pub(super) fn tile_center(&self, t: usize) -> [f32; 2] {
        let [x, y] = self.tile_xy(t);
        [x as f32 * self.tile + self.tile * 0.5, y as f32 * self.tile + self.tile * 0.5]
    }
    pub(super) fn effective(&self, t: usize) -> u32 {
        if self.pending[t] != NONE { self.pending[t] } else { self.var[t] }
    }
    pub(super) fn vertex(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && (x as usize) < self.hw && (y as usize) < self.hh).then(|| y as usize * self.hw + x as usize)
    }
    pub(super) fn vertex_xy(&self, v: usize) -> [f32; 2] {
        [(v % self.hw) as f32 * self.spacing, (v / self.hw) as f32 * self.spacing]
    }
    /// Terrain of the tile under a ground point (0x558A70 reads its +5C).
    pub(super) fn terrain_at(&self, data: &RmgData, p: [f32; 2]) -> Option<usize> {
        let t = self.tile_of(p)?;
        Some(data.variations[self.effective(t) as usize].terrain)
    }
}

/// Slope-limited height setter on one heightmap: pending rows are seeded,
/// then 0x559190 pulls every 8-neighbour to within `slope` (or the cell
/// terrain's max_slope beyond the seeds) of the row it is reached from,
/// breadth first; 0x550A10 commits the rows clamped to the limits.
pub(super) struct Slope {
    pub slope: f32,
    /// (centre, distance): the slope grows linearly from 0 at the centre to
    /// full at `distance` (settings+1C); None = constant slope.
    pub falloff: Option<([f32; 2], f32)>,
    rows: Vec<(usize, f32, u32)>,
    keys: HashMap<usize, usize>,
}

impl Slope {
    pub(super) fn new(slope: f32, falloff: Option<([f32; 2], f32)>) -> Self {
        Self { slope, falloff, rows: Vec::new(), keys: HashMap::new() }
    }
    pub(super) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub(super) fn rows(&self) -> &[(usize, f32, u32)] {
        &self.rows
    }
    fn set(&mut self, v: usize, h: f32, depth: u32) {
        if let Some(&i) = self.keys.get(&v) {
            self.rows[i].1 = h;
            return;
        }
        self.keys.insert(v, self.rows.len());
        self.rows.push((v, h, depth));
    }
    pub(super) fn seed(&mut self, v: usize, h: f32) {
        self.set(v, h, 0);
    }
    /// 0x558D90: the pending height, else the field's.
    pub(super) fn get(&self, field: &[f32], v: usize) -> f32 {
        self.keys.get(&v).map_or(field[v], |&i| self.rows[i].1)
    }
    pub(super) fn finish(&mut self, world: &World, data: &RmgData, overlay: bool) {
        let diagonal = world.spacing * std::f32::consts::SQRT_2;
        let mut cursor = 0;
        while cursor < self.rows.len() {
            let (v, h, depth) = self.rows[cursor];
            cursor += 1;
            let p = world.vertex_xy(v);
            let mut limit = self.slope;
            if depth > 0 {
                if let Some(t) = world.terrain_at(data, p) {
                    let s = data.terrains[t].max_slope;
                    if s != 0.0 {
                        limit = s;
                    }
                }
            }
            if let Some((c, d)) = self.falloff {
                if d > 0.0 {
                    let dist = ((c[0] - p[0]).powi(2) + (c[1] - p[1]).powi(2)).sqrt() / d;
                    limit *= dist.clamp(0.0, 1.0);
                }
            }
            let (vx, vy) = ((v % world.hw) as i32, (v / world.hw) as i32);
            for d in NEIGHBORS {
                let Some(n) = world.vertex(vx + d[0], vy + d[1]) else { continue };
                let old = match self.keys.get(&n) {
                    Some(&i) => {
                        if self.rows[i].2 < depth {
                            continue;
                        }
                        self.rows[i].1
                    }
                    None => {
                        if overlay {
                            world.overlay[n]
                        } else {
                            world.base[n]
                        }
                    }
                };
                let diff = old - h;
                let dist = if d[0] == 0 || d[1] == 0 { world.spacing } else { diagonal };
                if limit + 1e-4 < (diff / dist).abs() {
                    let value = diff.signum() * dist * limit + h;
                    self.set(n, value, depth + 1);
                }
            }
        }
    }
    pub(super) fn commit(&mut self, world: &mut World, overlay: bool) {
        let [lo, hi] = world.limits;
        let field = if overlay { &mut world.overlay } else { &mut world.base };
        for &(v, h, _) in &self.rows {
            field[v] = h.clamp(lo, hi);
        }
        self.rows.clear();
        self.keys.clear();
    }
}

/// The blocking painter's overlay setter (0x558D40: sink 0x5593D0, finish
/// 0x558FC0 / 0x558DE0): seeded vertices are relaxed from the outside in,
/// ring by ring; each ring vertex is pulled to within `slope` of its outer
/// neighbours (the average of the limited values) and committed, so painted
/// mountains and lakes ramp from the surrounding ground at the terrain's
/// max_slope.
pub(super) struct Relax {
    pub slope: f32,
    rows: Vec<(usize, f32, i32)>,
    keys: HashMap<usize, usize>,
}

impl Relax {
    pub(super) fn new(slope: f32) -> Self {
        Self { slope, rows: Vec::new(), keys: HashMap::new() }
    }
    pub(super) fn seed(&mut self, v: usize, h: f32) {
        if let Some(&i) = self.keys.get(&v) {
            self.rows[i].1 = h;
            return;
        }
        self.keys.insert(v, self.rows.len());
        self.rows.push((v, h, -1));
    }
    pub(super) fn get(&self, field: &[f32], v: usize) -> f32 {
        self.keys.get(&v).map_or(field[v], |&i| self.rows[i].1)
    }
    pub(super) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub(super) fn finish(&mut self, world: &mut World) {
        let diagonal = world.spacing * std::f32::consts::SQRT_2;
        let [lo, hi] = world.limits;
        let mut pass = 0i32;
        loop {
            let mut left = 0;
            for i in 0..self.rows.len() {
                if self.rows[i].2 != -1 {
                    continue;
                }
                let (v, h, _) = self.rows[i];
                let (vx, vy) = ((v % world.hw) as i32, (v / world.hw) as i32);
                let (mut sum, mut count) = (0.0f32, 0);
                for d in NEIGHBORS {
                    let Some(n) = world.vertex(vx + d[0], vy + d[1]) else { continue };
                    let nh = world.overlay[n];
                    let level = self.keys.get(&n).map_or(0, |&j| self.rows[j].2);
                    if level == pass {
                        self.rows[i].2 = pass + 1;
                    }
                    if self.rows[i].2 == pass + 1 && level == pass {
                        let diff = nh - h;
                        let dist = if d[0] == 0 || d[1] == 0 { world.spacing } else { diagonal };
                        if self.slope + 1e-4 < (diff / dist).abs() {
                            let sign = if diff < 0.0 { -1.0 } else { 1.0 };
                            sum += nh - sign * dist * self.slope;
                            count += 1;
                        }
                    }
                }
                if count > 0 {
                    self.rows[i].1 = sum / count as f32;
                }
                world.overlay[v] = self.rows[i].1.clamp(lo, hi);
                if self.rows[i].2 == -1 {
                    left += 1;
                }
            }
            pass += 1;
            if left == 0 || pass > (world.hw + world.hh) as i32 {
                break;
            }
        }
        self.rows.clear();
        self.keys.clear();
    }
}

/// 0xB3D610: the eight neighbour offsets of the batch, in its order.
const BATCH_NEIGHBORS: [[i32; 2]; 8] = [[-1, -1], [-1, 0], [-1, 1], [0, 1], [1, 1], [1, 0], [1, -1], [0, -1]];

/// One record of a batch (0x5598B0, 0x14 bytes: tile, x, y, variation,
/// state 0 untouched / 1 queued / 2 being processed / 3 settled).
struct Rec {
    tile: usize,
    x: i32,
    y: i32,
    var: u32,
    state: u8,
}

/// One terrain change batch (KKC_GMap change set: constructor 0x559980,
/// paint 0x5599F0, commit 0x559FB0, per-tile transitions 0x559A70): painted
/// tiles plus the neighbours the biome transitions convert. The batch's
/// stamp (GMap +0x48, advanced by the constructor) ties a tile to its
/// record through tile +0x24/+0x28; here a map does. Records are kept in
/// creation order (the +0x14 list the commit applies); the work queue
/// (+0xC) is prepended to and walked from its head.
pub(super) struct Batch<'a> {
    biome: &'a BiomeDef,
    data: &'a RmgData,
    /// +0x1C: a painted variation is water (0x5596B0 allows water
    /// transitions only then).
    water: bool,
    recs: Vec<Rec>,
    index: HashMap<usize, usize>,
    queue: std::collections::VecDeque<usize>,
}

impl<'a> Batch<'a> {
    pub(super) fn new(data: &'a RmgData, biome: &'a BiomeDef) -> Self {
        Self { biome, data, water: false, recs: Vec::new(), index: HashMap::new(), queue: std::collections::VecDeque::new() }
    }
    fn terrain(&self, var: u32) -> usize {
        self.data.variations[var as usize].terrain
    }
    /// 0x5598B0: the record of cell (x, y), created from the tile when the
    /// batch has none; None outside the map.
    fn record(&mut self, world: &World, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= world.tw as i32 || y >= world.th as i32 {
            return None;
        }
        let tile = y as usize * world.tw + x as usize;
        if let Some(&r) = self.index.get(&tile) {
            return Some(r);
        }
        let r = self.recs.len();
        self.recs.push(Rec { tile, x, y, var: world.var[tile], state: 0 });
        self.index.insert(tile, r);
        Some(r)
    }
    /// 0x5599F0 (the protect flag +0xA is clear for every RMG batch).
    pub(super) fn paint(&mut self, world: &World, t: usize, var: u32) {
        let [x, y] = world.tile_xy(t);
        if self.data.terrains[self.terrain(var)].kind == 2 {
            self.water = true;
        }
        let Some(r) = self.record(world, x, y) else { return };
        if self.recs[r].state == 0 {
            self.queue.push_front(r);
        }
        self.recs[r].var = var;
        self.recs[r].state = 1;
    }
    /// 0x5596B0: the transition variation to put next to `a` when `b` is
    /// painted beside it, following the biome's chain (0x4B6D90 per step);
    /// a water one only when this batch paints water.
    fn chain(&self, a: u32, b: u32) -> Option<u32> {
        let tb = self.terrain(b);
        let step = |v: u32| self.biome.transitions.get(&(self.terrain(v), tb)).map(|&t| t as u32);
        let mut prev = a;
        let mut t = step(a);
        while let Some(v) = t {
            if v == prev || v == b {
                break;
            }
            prev = v;
            t = step(v);
        }
        if self.data.terrains[self.terrain(prev)].kind == 2 && !self.water {
            return None;
        }
        (prev != a).then_some(prev)
    }
    /// 0x5597D0: qsort the eight candidates by pointer (null first; here the
    /// variation index stands for its address), then the longest run, the
    /// first one on ties.
    fn most_common(trans: &[Option<u32>; 8]) -> Option<u32> {
        let mut p: [u64; 8] = [0; 8];
        for (slot, t) in p.iter_mut().zip(trans) {
            *slot = t.map_or(0, |v| u64::from(v) + 1);
        }
        p.sort_unstable();
        let (mut cur, mut run, mut best, mut best_v) = (p[0], 1, 0, 0u64);
        for &v in &p[1..] {
            if v == 0 {
                continue;
            }
            if v == cur {
                run += 1;
            } else {
                if best < run && cur != 0 {
                    best = run;
                    best_v = cur;
                }
                run = 1;
                cur = v;
            }
        }
        let r = if run <= best || cur == 0 { best_v } else { cur };
        (r != 0).then(|| (r - 1) as u32)
    }
    /// 0x559A70 for one record.
    fn process(&mut self, world: &World, r: usize) {
        let (x, y, pvar) = (self.recs[r].x, self.recs[r].y, self.recs[r].var);
        let mut mode = 1u8;
        let mut conflict = false;
        let mut target: Option<u32> = 'first: {
            let mut trans: [Option<u32>; 8] = [None; 8];
            for (i, d) in BATCH_NEIGHBORS.iter().enumerate() {
                let Some(n) = self.record(world, x + d[0], y + d[1]) else { continue };
                if self.recs[n].state != 0 {
                    if self.chain(self.recs[n].var, pvar).is_some() {
                        mode = 3;
                        conflict = true;
                        break 'first Some(pvar);
                    }
                    continue;
                }
                trans[i] = self.chain(self.recs[n].var, pvar);
            }
            Self::most_common(&trans)
        };
        if target.is_none() {
            mode = 0;
        }
        if let Some(t) = target {
            let pterrain = self.terrain(pvar);
            let mut i = 0usize;
            while i < 8 {
                let d = BATCH_NEIGHBORS[i];
                if let Some(n) = self.record(world, x + d[0], y + d[1]) {
                    let cur = world.var[self.recs[n].tile];
                    if self.recs[n].state == 0 {
                        if mode == 3 {
                            self.recs[n].var = cur;
                        } else if self.terrain(cur) != pterrain {
                            self.recs[n].var = t;
                        }
                    } else if mode != 0 && mode != 3 && self.recs[n].state != 3 && self.chain(self.recs[n].var, t).is_some() {
                        mode = 3;
                        conflict = true;
                        i = 0;
                        continue;
                    }
                }
                i += 1;
            }
            target = Some(t);
        }
        let _ = target;
        if conflict {
            self.recs[r].state = 1;
            self.queue.push_front(r);
        } else if mode != 0 {
            self.recs[r].state = 3;
            for d in BATCH_NEIGHBORS {
                let Some(n) = self.record(world, x + d[0], y + d[1]) else { continue };
                if self.recs[n].state == 0 {
                    self.recs[n].state = 1;
                    if self.recs[n].var != world.var[self.recs[n].tile] {
                        self.queue.push_front(n);
                    }
                }
            }
        }
    }
    /// 0x559FB0(0): up to 16 passes over the queue (records whose variation
    /// differs from the tile's), then every changed record is applied
    /// (0x54FDC0). Returns false when the queue was not empty at the end
    /// (the batch +8 "unsettled" byte).
    pub(super) fn commit(mut self, world: &mut World) -> bool {
        for _ in 0..16 {
            if self.queue.is_empty() {
                break;
            }
            let list: Vec<usize> = self.queue.drain(..).filter(|&r| self.recs[r].var != world.var[self.recs[r].tile]).collect();
            for &r in &list {
                self.recs[r].state = 2;
            }
            for r in list {
                self.process(world, r);
            }
        }
        let settled = self.queue.is_empty();
        for rec in &self.recs {
            if world.var[rec.tile] != rec.var {
                world.var[rec.tile] = rec.var;
            }
        }
        settled
    }
}
