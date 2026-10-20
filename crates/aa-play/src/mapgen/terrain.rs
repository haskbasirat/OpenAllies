//! KKC_RMGTerrainStage (RMGTerrainStage.cpp, 0x599420): normalized brush
//! weights (BuildTerrainWeights 0x598260), per-brush variation tile budgets
//! (0x5990D0), blob painting (0x598B00) with the best-first blob solver
//! (0x539990 / 0x539650, score 0x5392C0, epoch counter 0x5394D0), micro-blobs,
//! attach positions (0x596AE0), streaks (DoStreaks 0x597530), and the
//! blocking painter's mountain/water heights (0x598600 / 0x598950 / 0x5972F0).
use super::data::{BiomeDef, BlobDef, BrushDef, BrushKind, RmcDef, RmgData, StreakDef};
use super::grid::{Batch, NONE, NEIGHBORS, Relax, Slope, World};
use super::rng::Rng;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// One variation entry of a brush (0x5990D0 builds them, 16 bytes each).
struct Entry {
    var: u32,
    fraction: f32,
    count: i32,
}

/// BuildTerrainWeights (0x598260): slider values normalized, the water and
/// mountain shares capped at max_percent_water/mountain (RMC, else SVars) and
/// the excess handed to the land brushes.
pub(super) fn normalized_weights(data: &RmgData, rmc: &RmcDef, values: &[f32]) -> Vec<f32> {
    let kind = |b: &BrushDef| data.terrains[b.terrain].kind;
    let (mut land, mut mountain, mut water, mut total) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for (b, &v) in rmc.brushes.iter().zip(values) {
        match kind(b) {
            1 => mountain += v,
            2 => water += v,
            _ => land += v,
        }
        total += v;
    }
    if total <= 0.0 {
        return vec![0.0; rmc.brushes.len()];
    }
    let inv = 1.0 / total;
    let (water, mountain) = (inv * water, inv * mountain);
    let max_water = if rmc.max_percent_water >= 0.0 { rmc.max_percent_water } else { data.svars.max_percent_water };
    let max_mountain = if rmc.max_percent_mountain >= 0.0 { rmc.max_percent_mountain } else { data.svars.max_percent_mountain };
    let excess_water = if water <= max_water { 0.0 } else { water - max_water };
    let excess_mountain = if max_mountain < mountain { mountain - max_mountain } else { 0.0 };
    let excess = excess_water + excess_mountain;
    let scale_water = if water <= 0.0 { 1.0 } else { 1.0 - excess_water / water };
    let scale_mountain = if mountain <= 0.0 { 1.0 } else { 1.0 - excess_mountain / mountain };
    let scale_land = if land * inv <= 0.0 { 1.0 } else { excess / (land * inv) + 1.0 };
    rmc.brushes
        .iter()
        .zip(values)
        .map(|(b, &v)| {
            inv * v
                * match kind(b) {
                    1 => scale_mountain,
                    2 => scale_water,
                    _ => scale_land,
                }
        })
        .collect()
}

/// 0x596AE0: a start point for a brush attached to a map region.
fn attach_point(world: &World, rng: &mut Rng, dir: u8, r: f32) -> [f32; 2] {
    let (mut u, mut v) = (rng.unit(), rng.unit());
    let (w, h) = (world.width, world.height);
    let d = r + r;
    let dir = if dir == 10 { (rng.below(9) + 1) as u8 } else { dir };
    let (x, y) = match dir {
        0 => (w * u, h * v),
        1 => (d * u, v * d),
        2 => (d * u, h * v),
        // NORTHWEST uses the full range as x (as the original does).
        3 => (d, h - v * d),
        4 => (w * u, h - v * d),
        5 => (w - d * u, h - v * d),
        6 => (w - d * u, h * v),
        7 => (w - d * u, v * d),
        8 => (w * u, v * d),
        _ => {
            u = w * 0.5 + d * u - r;
            v = h * 0.5 + d * v - r;
            (u, v)
        }
    };
    [x.clamp(1e-4, w - 1e-4), y.clamp(1e-4, h - 1e-4)]
}
/// 0x5969D0: a point in the square of half-size `r` around `c`.
fn point_near(world: &World, rng: &mut Rng, c: [f32; 2], r: f32) -> [f32; 2] {
    let x = (rng.unit() - 0.5) * r;
    let x = x + x + c[0];
    let y = (rng.unit() - 0.5) * r;
    let y = y + y + c[1];
    [x.clamp(1e-4, world.width - 1e-4), y.clamp(1e-4, world.height - 1e-4)]
}
fn pick_attach(rng: &mut Rng, attach: &[u8]) -> u8 {
    match attach.len() {
        0 => 0,
        1 => attach[0],
        n => attach[rng.below(n as u32) as usize],
    }
}
/// PickRandVariation (0x596D20): weighted by the remaining tile counts.
fn pick_entry(entries: &[Entry], total: i32, start: usize, rng: &mut Rng) -> Option<usize> {
    if total <= 0 {
        return None;
    }
    let r = (rng.next_u32() % total as u32) as i32;
    let mut sum = 0;
    for (i, e) in entries.iter().enumerate().skip(start) {
        sum += e.count;
        if r <= sum - 1 {
            return Some(i);
        }
    }
    None
}

#[derive(PartialEq)]
struct Open {
    cost: f32,
    seq: u32,
    tile: usize,
}
impl Eq for Open {}
impl Ord for Open {
    fn cmp(&self, o: &Self) -> Ordering {
        // Min-heap on cost, then first pushed first.
        o.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal).then(o.seq.cmp(&self.seq))
    }
}
impl PartialOrd for Open {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// Blob shape parameters the solver goal copies from the brush (0x539990).
#[derive(Clone, Copy)]
struct Shape {
    proximity: f32,
    consistency: f32,
    fuzzy: f32,
    dir_bias: [f32; 2],
    dir_bias_factor: f32,
}

/// Scratch reused by every blob solve.
pub(super) struct Solver {
    visit: Vec<u32>,
    stamp: u32,
    heap: BinaryHeap<Open>,
}
impl Solver {
    pub(super) fn new(world: &World) -> Self {
        Self { visit: vec![0; world.tw * world.th], stamp: 0, heap: BinaryHeap::new() }
    }
}

/// What a painted tile does besides recording its variation.
pub(super) struct Painter {
    pub blocking: bool,
    pub water: bool,
    pub affect_height: bool,
    pub terrain: usize,
    /// Tiles painted by this brush, in order (land: 0x597EF0 list).
    pub painted: Vec<(usize, u32)>,
    /// Blocking: vertex -> (weight, target) of the current blob (0x598600).
    heights: HashMap<usize, (f32, f32)>,
    height_order: Vec<usize>,
    pub slope: Relax,
}

impl Painter {
    pub(super) fn new(data: &RmgData, terrain: usize, blocking: bool) -> Self {
        let t = &data.terrains[terrain];
        Self {
            blocking,
            water: t.kind == 2,
            affect_height: true,
            terrain,
            painted: Vec::new(),
            heights: HashMap::new(),
            height_order: Vec::new(),
            slope: Relax::new(t.max_slope),
        }
    }
    /// Goal vt[2]: 0x598530 (land) / 0x598600 (blocking).
    fn paint_tile(&mut self, world: &mut World, data: &RmgData, t: usize, var: u32, radius: f32) {
        if self.blocking && self.affect_height {
            let old_kind = data.terrains[data.variations[world.var[t] as usize].terrain].kind;
            // A mountain painted over water (or water over a mountain) starts from 0.
            let from_zero = if self.water { old_kind == 1 } else { old_kind == 2 };
            let tdef = &data.terrains[self.terrain];
            let offset = if self.water { -tdef.max_depth } else { tdef.max_slope * radius };
            let [tx, ty] = world.tile_xy(t);
            let scale = world.tile / world.spacing;
            let (x0, y0) = ((tx as f32 * scale) as i32, (ty as f32 * scale) as i32);
            let x1 = (((tx + 1) as f32 * scale) as i32).min(world.hw as i32 - 1);
            let y1 = (((ty + 1) as f32 * scale) as i32).min(world.hh as i32 - 1);
            for vy in y0..=y1 {
                for vx in x0..=x1 {
                    let Some(v) = world.vertex(vx, vy) else { continue };
                    let base = if from_zero { 0.0 } else { self.slope.get(&world.overlay, v) };
                    let h = (base + offset).clamp(world.limits[0], world.limits[1]);
                    let e = self.heights.entry(v).or_insert_with(|| {
                        self.height_order.push(v);
                        (0.0, 0.0)
                    });
                    if e.0 < 1.0 {
                        e.0 += 0.25;
                        e.1 = h;
                    }
                }
            }
        }
        world.pending[t] = var;
        self.painted.push((t, var));
    }
    /// 0x598950: after each blob the accumulated vertex targets go to the
    /// slope-limited overlay setter.
    fn after_blob(&mut self) {
        for v in self.height_order.drain(..) {
            let (w, h) = self.heights[&v];
            if w != 0.0 {
                self.slope.seed(v, h * w);
            }
        }
        self.heights.clear();
    }
}

/// The blob solver: best-first growth from the centre tile until `n` tiles
/// are accepted. Cost = 10 * epoch - score (0x5392C0).
#[allow(clippy::too_many_arguments)]
fn grow_blob(
    world: &mut World,
    data: &RmgData,
    solver: &mut Solver,
    painter: &mut Painter,
    rng: &mut Rng,
    center: [f32; 2],
    radius: f32,
    n: i32,
    var: u32,
    shape: Shape,
) {
    grow_blob_raw(world, solver, rng, center, radius, n, var, shape, &mut |w, t, v| painter.paint_tile(w, data, t, v, radius));
}

#[allow(clippy::too_many_arguments)]
fn grow_blob_raw(
    world: &mut World,
    solver: &mut Solver,
    rng: &mut Rng,
    center: [f32; 2],
    radius: f32,
    n: i32,
    var: u32,
    shape: Shape,
    paint: &mut dyn FnMut(&mut World, usize, u32),
) {
    let Some(start) = world.tile_of(center) else { return };
    solver.stamp = solver.stamp.wrapping_add(1);
    if solver.stamp == 0 {
        solver.visit.iter_mut().for_each(|v| *v = 0);
        solver.stamp = 1;
    }
    let stamp = solver.stamp;
    solver.heap.clear();
    let mut seq = 0u32;
    let mut epoch = 0i32;
    let (mut counter, mut state) = (0i32, 0u8);
    let r_tiles = (radius / world.tile).ceil() as i32;
    let mut remaining = n;
    let score = |world: &World, rng: &mut Rng, t: usize, epoch: i32| -> f32 {
        let p = world.tile_center(t);
        let (mut dx, mut dy) = (p[0] - center[0], p[1] - center[1]);
        let d = (dx * dx + dy * dy).sqrt();
        if d != 0.0 {
            dx /= d;
            dy /= d;
        }
        let fuzzy = if shape.fuzzy != 0.0 { rng.unit() * shape.fuzzy } else { 0.0 };
        let consistency = if shape.consistency != 0.0 {
            let [x, y] = world.tile_xy(t);
            let same = NEIGHBORS
                .iter()
                .filter(|o| world.tile_at(x + o[0], y + o[1]).is_some_and(|n| world.effective(n) == var))
                .count();
            same as f32 * 0.125 * shape.consistency
        } else {
            0.0
        };
        let proximity = if shape.proximity != 0.0 { d * shape.proximity / (radius * 1.5) } else { 0.0 };
        let dir = if shape.dir_bias_factor != 0.0 && d != 0.0 {
            (dy * shape.dir_bias[1] + dx * shape.dir_bias[0]).abs() * shape.dir_bias_factor
        } else {
            0.0
        };
        (10 * epoch) as f32 - (dir + consistency + fuzzy - proximity)
    };
    let c = score(world, rng, start, epoch);
    solver.visit[start] = stamp;
    solver.heap.push(Open { cost: c, seq, tile: start });
    seq += 1;
    while let Some(Open { tile, .. }) = solver.heap.pop() {
        // 0x5394D0: the epoch counter, then the goal paints the tile.
        counter += 1;
        if state == 0 {
            if counter > r_tiles {
                epoch += 1;
                state = 1;
                counter = 0;
            }
        } else if counter > 4 * r_tiles {
            state = 0;
            counter = 0;
        }
        paint(world, tile, var);
        remaining -= 1;
        if remaining <= 0 {
            break;
        }
        let [x, y] = world.tile_xy(tile);
        for d in NEIGHBORS {
            let Some(nb) = world.tile_at(x + d[0], y + d[1]) else { continue };
            if solver.visit[nb] == stamp {
                continue;
            }
            solver.visit[nb] = stamp;
            let c = score(world, rng, nb, epoch);
            solver.heap.push(Open { cost: c, seq, tile: nb });
            seq += 1;
        }
    }
}

/// 0x597090: tiles in a blob of radius `r`, at least 12, at most the entry's.
fn blob_tiles(world: &World, r: f32, available: i32) -> i32 {
    let n = (r * r * std::f32::consts::PI / (world.tile * world.tile)) as i32;
    if n < 12 { 12 } else { n.min(available) }
}

/// Random unit vector (the 0xB39AF8 direction table spans 2*pi).
fn random_dir(rng: &mut Rng) -> [f32; 2] {
    let a = rng.unit() * std::f32::consts::TAU;
    [a.cos(), a.sin()]
}

fn shape_of(blob: &BlobDef, rng: &mut Rng) -> Shape {
    let mut dir = blob.dir_bias;
    if dir[0] == 0.0 && dir[1] == 0.0 {
        dir = random_dir(rng);
    }
    Shape { proximity: blob.proximity, consistency: blob.consistency, fuzzy: blob.fuzzy, dir_bias: dir, dir_bias_factor: blob.dir_bias_factor }
}

/// 0x598B00 for one blob brush.
fn paint_blobs(world: &mut World, data: &RmgData, biome: &BiomeDef, solver: &mut Solver, rng: &mut Rng, brush: &BrushDef, blob: &BlobDef, entries: &mut [Entry], mut total: i32) {
    let blocking = data.terrains[brush.terrain].kind != 0;
    let mut painter = Painter::new(data, brush.terrain, blocking);
    let tile_area = world.tile * world.tile;
    let micro = blob.micro.as_ref().filter(|_| entries.len() >= 2);
    while total > 0 {
        let (index, var) = if micro.is_some() {
            (0usize, entries[0].var)
        } else {
            let Some(i) = pick_entry(entries, total, 0, rng) else { break };
            (i, entries[i].var)
        };
        let radius = rng.range(blob.size);
        let attach_r = if brush.attach_range >= 0.0 { brush.attach_range } else { radius };
        let dir = pick_attach(rng, &brush.attach);
        let center = attach_point(world, rng, dir, attach_r);
        let n = blob_tiles(world, radius, entries[index].count);
        let shape = shape_of(blob, rng);
        grow_blob(world, data, solver, &mut painter, rng, center, radius, n, var, shape);
        painter.after_blob();
        entries[index].count -= n;
        total -= n;
        if let Some(micro) = micro {
            // Micro-blobs of the other variations inside the main blob's circle.
            let circle = (n as f32 * tile_area * std::f32::consts::FRAC_1_PI).sqrt();
            let micro_tiles = (rng.range(micro.ratio) * n as f32) as i32;
            let mut sum = 0;
            for e in entries.iter_mut().skip(1) {
                e.count = (micro_tiles as f32 * e.fraction) as i32;
                sum += e.count;
            }
            if blocking {
                painter.affect_height = micro.affect_height;
            }
            while sum > 0 {
                let Some(i) = pick_entry(entries, sum, 1, rng) else { break };
                let r = rng.range(micro.size);
                let c = point_near(world, rng, center, circle);
                let m = blob_tiles(world, r, entries[i].count);
                let shape = shape_of(blob, rng);
                grow_blob(world, data, solver, &mut painter, rng, c, r, m, entries[i].var, shape);
                painter.after_blob();
                entries[i].count -= m;
                sum -= m;
            }
            painter.affect_height = true;
        }
    }
    finish(world, data, biome, painter);
}

/// Painter finish: 0x597F50 (land) / 0x5972F0 (blocking: twice if the batch
/// did not settle, then the overlay slope pass).
fn finish(world: &mut World, data: &RmgData, biome: &BiomeDef, mut painter: Painter) {
    let passes = if painter.blocking { 2 } else { 1 };
    let painted = std::mem::take(&mut painter.painted);
    for _ in 0..passes {
        let mut batch = Batch::new(data, biome);
        for &(t, v) in &painted {
            batch.paint(world, t, v);
        }
        if batch.commit(world) {
            break;
        }
    }
    for &(t, _) in &painted {
        world.pending[t] = NONE;
    }
    if painter.blocking && !painter.slope.is_empty() {
        painter.slope.finish(world);
    }
}

/// DoStreaks (0x597530): continuous strokes of a circle brush.
fn paint_streaks(world: &mut World, data: &RmgData, biome: &BiomeDef, rng: &mut Rng, brush: &BrushDef, streak: &StreakDef, entries: &mut [Entry], mut total: i32) {
    let tdef = &data.terrains[brush.terrain];
    let blocking = tdef.kind != 0;
    let tile_area = world.tile * world.tile;
    while total > 0 {
        let Some(index) = pick_entry(entries, total, 0, rng) else { break };
        let var = entries[index].var;
        let wspan = streak.width[1] - streak.width[0];
        let r = if brush.attach_range < 0.0 { wspan * 0.5 } else { brush.attach_range };
        let dir = pick_attach(rng, &brush.attach);
        let origin = attach_point(world, rng, dir, r);
        let count = streak.streaks[0] + (rng.unit() * (streak.streaks[1] - streak.streaks[0]) as f32) as i32;
        let mut dir_v = [rng.signed(), rng.signed()];
        let len = (dir_v[0] * dir_v[0] + dir_v[1] * dir_v[1]).sqrt();
        if len <= 1e-4 {
            dir_v = [0.0, 0.0];
        } else {
            dir_v = [dir_v[0] / len, dir_v[1] / len];
        }
        let first_dir = dir_v;
        let mut start = origin;
        let mut restarts = 0;
        let mut carry = 0.0f32;
        let mut slope = Slope::new(tdef.max_slope, None);
        let mut painted_any = false;
        for _ in 0..count.max(0) {
            if entries[index].count < 1 || restarts > 1 {
                break;
            }
            let width = rng.range(streak.width);
            let radius = width * 0.5;
            let length = rng.range(streak.length);
            let mut end = [length * dir_v[0] + start[0], length * dir_v[1] + start[1]];
            end = [end[0].clamp(0.0, world.width), end[1].clamp(0.0, world.height)];
            let actual = ((start[1] - end[1]).powi(2) + (start[0] - end[0]).powi(2)).sqrt();
            let short = length - actual;
            // 0x5559F0: paint every tile whose centre is within the brush radius of the segment.
            let mut batch = Batch::new(data, biome);
            paint_segment(world, start, end, radius, |t| batch.paint(world, t, var));
            batch.commit(world);
            painted_any = true;
            if blocking {
                // Raise (or lower) the overlay along the stroke to max_slope * width / 2.
                let target = if tdef.kind == 2 { -(tdef.max_slope * width * 0.5) } else { tdef.max_slope * width * 0.5 };
                let steps = (actual / world.spacing).ceil().max(1.0) as i32;
                for s in 0..=steps {
                    let f = s as f32 / steps as f32;
                    let p = [start[0] + (end[0] - start[0]) * f, start[1] + (end[1] - start[1]) * f];
                    let (vx, vy) = ((p[0] / world.spacing + 0.5) as i32, (p[1] / world.spacing + 0.5) as i32);
                    let Some(v) = world.vertex(vx, vy) else { continue };
                    let cur = slope.get(&world.overlay, v);
                    let mut h = if target <= 0.0 { cur.min(target) } else { cur.max(target) };
                    h = h.clamp(world.limits[0], world.limits[1]);
                    // 0x597B9x: never deeper than the terrain's max_depth.
                    if h < -tdef.max_depth {
                        h = -tdef.max_depth;
                    }
                    slope.seed(v, h);
                }
            }
            let tiles = carry + width * actual / tile_area;
            let whole = tiles as i32;
            carry = tiles - whole as f32;
            entries[index].count -= whole;
            total -= whole;
            if short <= 1e-4 {
                start = end;
                let a = rng.range(streak.angle).to_radians();
                let (s, c) = a.sin_cos();
                let nx = c * dir_v[0] + dir_v[1] * s;
                let ny = c * dir_v[1] - s * dir_v[0];
                let l = (nx * nx + ny * ny).sqrt();
                dir_v = if l > 1e-4 { [nx / l, ny / l] } else { [0.0, 0.0] };
            } else {
                dir_v = [-first_dir[0], -first_dir[1]];
                start = origin;
                restarts += 1;
            }
        }
        if blocking && !slope.is_empty() {
            slope.finish(world, data, true);
            slope.commit(world, true);
        }
        if !painted_any {
            // Nothing could be painted from this entry: drop it.
            total -= entries[index].count.max(0);
            entries[index].count = 0;
        }
    }
}

fn paint_segment(world: &World, a: [f32; 2], b: [f32; 2], r: f32, mut f: impl FnMut(usize)) {
    let (minx, maxx) = (a[0].min(b[0]) - r, a[0].max(b[0]) + r);
    let (miny, maxy) = (a[1].min(b[1]) - r, a[1].max(b[1]) + r);
    let tx0 = ((minx / world.tile).floor() as i32).max(0);
    let ty0 = ((miny / world.tile).floor() as i32).max(0);
    let tx1 = ((maxx / world.tile).floor() as i32).min(world.tw as i32 - 1);
    let ty1 = ((maxy / world.tile).floor() as i32).min(world.th as i32 - 1);
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    for ty in ty0..=ty1 {
        for tx in tx0..=tx1 {
            let t = ty as usize * world.tw + tx as usize;
            let p = world.tile_center(t);
            let s = if len2 > 0.0 { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
            let (qx, qy) = (a[0] + dx * s - p[0], a[1] + dy * s - p[1]);
            if qx * qx + qy * qy <= r * r {
                f(t);
            }
        }
    }
}

/// The whole terrain stage (0x599420 / 0x5990D0).
pub(super) fn run(world: &mut World, data: &RmgData, rmc: &RmcDef, biome: &BiomeDef, values: &[f32], rng: &mut Rng) {
    let weights = normalized_weights(data, rmc, values);
    let tiles = (world.tw * world.th) as f32;
    let mut solver = Solver::new(world);
    let mut acc = 0.0f32;
    for (brush, &w) in rmc.brushes.iter().zip(&weights) {
        acc += w;
        // Later brushes paint over earlier ones; earlier ones get more.
        let w = (1.0 - acc) * w + w;
        let terrain = &data.terrains[brush.terrain];
        let micro = matches!(&brush.kind, BrushKind::Blob(b) if b.micro.is_some());
        let mut entries = Vec::new();
        let mut total = 0;
        for (i, &var) in terrain.variations.iter().enumerate() {
            let vw = data.variations[var].weight;
            let (fraction, count) = if micro {
                if i == 0 {
                    (1.0, (tiles * w) as i32)
                } else {
                    let first = data.variations[terrain.variations[0]].weight;
                    (vw / (terrain.total_weight - first), 0)
                }
            } else if brush.paint_first_variation_only {
                (1.0, (tiles * w) as i32)
            } else {
                let f = vw / terrain.total_weight;
                (f, (tiles * f * w) as i32)
            };
            entries.push(Entry { var: var as u32, fraction, count });
            total += count;
            if brush.paint_first_variation_only && !micro {
                break;
            }
        }
        if total <= 0 {
            continue;
        }
        match &brush.kind {
            BrushKind::Blob(blob) => paint_blobs(world, data, biome, &mut solver, rng, brush, blob, &mut entries, total),
            BrushKind::Streak(s) => paint_streaks(world, data, biome, rng, brush, s, &mut entries, total),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// A lone blob on an empty grid (shape check, printed with --nocapture).
    #[test]
    fn blob_shape() {
        let (tw, th) = (64usize, 64usize);
        let mut world = World {
            width: 128.0,
            height: 128.0,
            tile: 2.0,
            tile_bits: 1,
            tw,
            th,
            var: vec![0; tw * th],
            pending: vec![NONE; tw * th],
            hw: tw + 1,
            hh: th + 1,
            spacing: 2.0,
            base: vec![2.0; (tw + 1) * (th + 1)],
            overlay: vec![0.0; (tw + 1) * (th + 1)],
            limits: [-10.0, 14.0],
        };
        let mut solver = Solver { visit: vec![0; tw * th], stamp: 0, heap: BinaryHeap::new() };
        let mut rng = Rng::new(3);
        let radius = 20.0;
        let n = blob_tiles(&world, radius, 10_000);
        let shape = Shape { proximity: 0.9, consistency: 0.5, fuzzy: 0.9, dir_bias: [1.0, 0.0], dir_bias_factor: 0.0 };
        grow_blob_raw(&mut world, &mut solver, &mut rng, [64.0, 64.0], radius, n, 7, shape, &mut |w, t, v| w.pending[t] = v);
        let mut s = String::new();
        for y in 0..th {
            for x in 0..tw {
                s.push(if world.pending[y * tw + x] == 7 { 'X' } else { '-' });
            }
            s.push('\n');
        }
        eprintln!("n={n}\n{s}");
    }
}
