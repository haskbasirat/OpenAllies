//! KKC_RMGTerrainStage (RMGTerrainStage.cpp, Run 0x599420): normalized
//! brush weights (0x598260), per-brush variation entries (0x5990D0), blob
//! painting (0x598B00) with its micro-blobs, one blob (0x597090, the blob
//! solver in `blob.rs`), attach and scatter points (0x4A04D0, 0x596AE0,
//! 0x5969D0), the weighted entry pick (0x596D20) and streaks (DoStreaks
//! 0x597530, the stroke 0x5559F0 and its stamp 0x5526C0).
//!
//! Arithmetic: the original evaluates on the x87 register at PC53, which is
//! binary64 here; every binary32 store it makes is kept (`as f32`), and
//! `_ftol` (0x96CC4C) truncates.
//!
//! Not exact yet: the blocking painter's heights (0x598600 / 0x598950 and
//! the slope setter 0x558D40) and the streak heights (0x558CE0 / 0x5590F0 /
//! 0x559470) use the behaviour-level setters of `grid.rs`; water areas
//! (0x552C90, 0x539E30) are not tracked.
use super::blob::{self, Shape, Solver, TilePainter, ftol, ftol_d};
use super::data::{BiomeDef, BlobDef, BrushDef, BrushKind, RmcDef, RmgData, StreakDef};
use super::grid::{Batch, Relax, Slope, World};
use super::rng::Rng;
use std::collections::HashMap;

/// `0x9FC0BC`, binary32 0.0001.
const EPS: f32 = f32::from_bits(0x38D1_B717);

/// One variation entry of a brush (0x5990D0: var, fraction, count, brush).
struct Entry {
    var: u32,
    fraction: f32,
    count: i32,
}

/// BuildTerrainWeights (0x598260): slider values normalized, the water and
/// mountain shares capped at max_percent_water/mountain (RMC, else SVars) and
/// the excess handed to the land brushes. Sums and products stay on the
/// register; the shares and scales are binary32 locals.
pub(super) fn normalized_weights(data: &RmgData, rmc: &RmcDef, values: &[f32]) -> Vec<f32> {
    let kind = |b: &BrushDef| data.terrains[b.terrain].kind;
    let (mut land, mut total, mut mountain, mut water) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for (b, &v) in rmc.brushes.iter().zip(values) {
        let v = f64::from(v);
        match kind(b) {
            0 => land += v,
            1 => mountain += v,
            2 => water += v,
            _ => {}
        }
        total += v;
    }
    let inv_d = 1.0 / total;
    let inv = inv_d as f32;
    let water_n = (inv_d * water) as f32;
    let mountain_n = (f64::from(inv) * mountain) as f32;
    let land_n = land * f64::from(inv);
    let max_water = if rmc.max_percent_water >= 0.0 { rmc.max_percent_water } else { data.svars.max_percent_water };
    let max_mountain = if rmc.max_percent_mountain >= 0.0 { rmc.max_percent_mountain } else { data.svars.max_percent_mountain };
    let excess_water = if water_n <= max_water { 0.0 } else { (f64::from(water_n) - f64::from(max_water)) as f32 };
    let excess_mountain = if mountain_n <= max_mountain { 0.0 } else { f64::from(mountain_n) - f64::from(max_mountain) };
    let excess = (f64::from(excess_water) + excess_mountain) as f32;
    let scale_water = if water_n <= 0.0 { 1.0 } else { (1.0 - f64::from(excess_water) / f64::from(water_n)) as f32 };
    let scale_mountain = if mountain_n <= 0.0 { 1.0 } else { (1.0 - excess_mountain / f64::from(mountain_n)) as f32 };
    let scale_land = if land_n <= 0.0 { 1.0 } else { (f64::from(excess) / land_n + 1.0) as f32 };
    // A brush of another terrain kind keeps the previous value (the native
    // local is not rewritten), initially the water share.
    let mut w = water_n;
    rmc.brushes
        .iter()
        .zip(values)
        .map(|(b, &v)| {
            let scale = match kind(b) {
                0 => Some(scale_land),
                1 => Some(scale_mountain),
                2 => Some(scale_water),
                _ => None,
            };
            if let Some(s) = scale {
                w = (f64::from(inv) * f64::from(v) * f64::from(s)) as f32;
            }
            w
        })
        .collect()
}

/// 0x4A04D0 on the brush's attach list, after the caller's getter: no
/// draw for zero or one entry.
fn attach_dir(rng: &mut Rng, attach: &[u8]) -> u8 {
    match attach.len() {
        0 => {
            rng.getter();
            0
        }
        1 => {
            rng.getter();
            attach[0]
        }
        n => attach[(rng.at(0x4A04FB).next_u32() % n as u32) as usize],
    }
}

/// 0x596AE0: a start point for a brush attached to a map region; `r` is
/// the attach range (twice it is the band width).
fn attach_point(world: &World, rng: &mut Rng, dir: u8, r: f32) -> [f32; 2] {
    let u = rng.at(0x596AF4).unit_d() as f32;
    let mut v = rng.at(0x596B24).unit_d();
    let v32 = v as f32;
    let (w, h) = (f64::from(world.width), f64::from(world.height));
    let d = (f64::from(r) + f64::from(r)) as f32;
    let dir = if dir == 10 {
        let k = rng.at(0x596B79).next_u32() % 9 + 1;
        v = f64::from(v32);
        k as u8
    } else {
        dir
    };
    let (u, d) = (f64::from(u), f64::from(d));
    let (x, y): (f32, f64) = match dir {
        0 => ((w * u) as f32, v * h),
        1 => ((d * u) as f32, v * d),
        2 => ((d * u) as f32, v * h),
        3 => (d as f32, h - v * d),
        4 => ((w * u) as f32, h - v * d),
        5 => ((w - d * u) as f32, h - v * d),
        6 => ((w - d * u) as f32, v * h),
        7 => ((w - d * u) as f32, v * d),
        8 => ((w * u) as f32, v * d),
        9 => (((d * u + w * 0.5) - f64::from(r)) as f32, (v * d + h * 0.5) - f64::from(r)),
        _ => (u as f32, v),
    };
    clamp_point(world, x, y)
}

/// The shared tail of 0x596AE0 / 0x5969D0: x against [0.0001, width -
/// 0.0001] (the upper bound on the register), y against [0.0001,
/// binary32(height - 0.0001)].
fn clamp_point(world: &World, x: f32, y: f64) -> [f32; 2] {
    let eps = f64::from(EPS);
    let lim_x = f64::from(world.width) - eps;
    let x = if f64::from(x) < eps {
        EPS
    } else if f64::from(x) > lim_x {
        lim_x as f32
    } else {
        x
    };
    let lim_y = (f64::from(world.height) - eps) as f32;
    let y = if y < eps {
        EPS
    } else if y > f64::from(lim_y) {
        lim_y
    } else {
        y as f32
    };
    [x, y]
}

/// 0x5969D0: a point in the square of half-size `r` around `c`.
fn point_near(world: &World, rng: &mut Rng, c: [f32; 2], r: f32) -> [f32; 2] {
    let a = (rng.at(0x5969E5).unit_d() - 0.5) * f64::from(r);
    let x = ((a + a) + f64::from(c[0])) as f32;
    let b = (rng.at(0x596A27).unit_d() - 0.5) * f64::from(r);
    let y = (b + b) + f64::from(c[1]);
    clamp_point(world, x, y)
}

/// PickRandVariation (0x596D20): `draw % total`, then the first entry from
/// `start` whose running count passes it.
fn pick_entry(entries: &[Entry], total: i32, start: usize, rng: &mut Rng) -> Option<usize> {
    let r = rng.at(0x596D47).next_u32() % total as u32;
    let mut sum = 0i32;
    for (i, e) in entries.iter().enumerate().skip(start) {
        sum = sum.wrapping_add(e.count);
        if r as i32 <= sum - 1 {
            return Some(i);
        }
    }
    None
}

/// The painters (vtables 0xA4269C land, 0xA4268C blocking): vtable +8 paints
/// one blob (0x5971E0 / 0x599020), +0xC finishes the brush (0x597F50 /
/// 0x5972F0); the tile painter at +0xB8 is what the solver calls.
trait Painter: TilePainter {
    /// Set by 0x598B00 around micro-blobs of a blocking brush (+0xD6).
    fn set_affect_height(&mut self, _on: bool) {}
    /// After one blob's solve (0x599020: heights 0x598950).
    fn after_blob(&mut self, _world: &mut World) {}
    fn finish(&mut self, world: &mut World);
}

/// 0x598200: the land painter. Its tile painter (0x598530) sets tile +0x24
/// and appends (tile, var); 0x597F50 paints the list into one batch.
struct LandPainter<'a> {
    data: &'a RmgData,
    biome: &'a BiomeDef,
    list: Vec<(usize, u32)>,
}

impl TilePainter for LandPainter<'_> {
    fn paint(&mut self, _world: &mut World, tile: usize, var: u32) -> bool {
        self.list.push((tile, var));
        true
    }
}

impl Painter for LandPainter<'_> {
    fn finish(&mut self, world: &mut World) {
        let mut batch = Batch::new(self.data, self.biome);
        for &(t, v) in &self.list {
            batch.paint(world, t, v);
        }
        batch.commit(world);
        self.list.clear();
    }
}

/// 0x5980B0: the blocking (mountain / water) painter. Tiles as the land
/// painter (0x598600 appends to +0x114); 0x5972F0 paints the list into a
/// batch, once more when the first did not settle.
struct BlockingPainter<'a> {
    data: &'a RmgData,
    biome: &'a BiomeDef,
    terrain: usize,
    water: bool,
    affect_height: bool,
    list: Vec<(usize, u32)>,
    /// Blob heights (approximate, see the module notes).
    heights: HashMap<usize, (f32, f32)>,
    height_order: Vec<usize>,
    slope: Relax,
    radius: f32,
}

impl TilePainter for BlockingPainter<'_> {
    fn begin(&mut self, _var: u32, _center: [f32; 2], radius: f32) {
        self.radius = radius;
    }
    fn paint(&mut self, world: &mut World, t: usize, var: u32) -> bool {
        self.list.push((t, var));
        if self.affect_height {
            let data = self.data;
            let old_kind = data.terrains[data.variations[world.var[t] as usize].terrain].kind;
            let from_zero = if self.water { old_kind == 1 } else { old_kind == 2 };
            let tdef = &data.terrains[self.terrain];
            let offset = if self.water { -tdef.max_depth } else { tdef.max_slope * self.radius };
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
        true
    }
}

impl Painter for BlockingPainter<'_> {
    fn set_affect_height(&mut self, on: bool) {
        self.affect_height = on;
    }
    fn after_blob(&mut self, _world: &mut World) {
        for v in self.height_order.drain(..) {
            let (w, h) = self.heights[&v];
            if w != 0.0 {
                self.slope.seed(v, h * w);
            }
        }
        self.heights.clear();
    }
    fn finish(&mut self, world: &mut World) {
        for _ in 0..2 {
            let mut batch = Batch::new(self.data, self.biome);
            for &(t, v) in &self.list {
                batch.paint(world, t, v);
            }
            if batch.commit(world) {
                break;
            }
        }
        self.list.clear();
        if !self.slope.is_empty() {
            self.slope.finish(world);
        }
    }
}

/// 0x597090 (through the painter's blob method): one blob of `entry`'s
/// variation of `radius` tiles' area around `center`; returns the tile
/// count it was asked for (at least 12, at most the entry's count).
#[allow(clippy::too_many_arguments)]
fn paint_blob(world: &mut World, solver: &mut Solver, painter: &mut dyn Painter, rng: &mut Rng, center: [f32; 2], radius: f32, entry: &Entry, blob: &BlobDef) -> i32 {
    let tile = f64::from(world.tile);
    let r = f64::from(radius);
    let pi = f64::from(f32::from_bits(0x4049_0FDB));
    let n = ftol_d(r * r * pi / (tile * tile));
    let n = if n < 12 { 12 } else if entry.count < n { entry.count } else { n };
    let mut dir = blob.dir_bias;
    if dir[0] == 0.0 && dir[1] == 0.0 {
        let table = blob::direction_table();
        dir = table[(rng.at(0x59717E).next_u32() % table.len() as u32) as usize];
    }
    let shape = Shape { proximity: blob.proximity, consistency: blob.consistency, fuzzy: blob.fuzzy, dir_bias: dir, dir_bias_factor: blob.dir_bias_factor };
    solver.solve(world, painter, rng, center, n, radius, entry.var, shape);
    painter.after_blob(world);
    n
}

/// 0x598B00 for one blob brush (`land`: the land painter, else blocking).
#[allow(clippy::too_many_arguments)]
fn paint_blobs(world: &mut World, solver: &mut Solver, painter: &mut dyn Painter, rng: &mut Rng, brush: &BrushDef, blob: &BlobDef, entries: &mut [Entry], total: &mut i32, land: bool) {
    let tile_i = ftol(world.tile);
    let tile_area = (f64::from(tile_i) * f64::from(tile_i)) as f32;
    let attach_r = |radius: f32| if brush.attach_range < 0.0 { radius } else { brush.attach_range };
    match blob.micro.as_ref().filter(|_| entries.len() > 1) {
        Some(micro) => {
            while *total > 0 {
                let [mn, mx] = blob.size;
                let radius = ((f64::from(mx) - f64::from(mn)) * rng.at(0x598C19).unit_d() + f64::from(mn)) as f32;
                let r = attach_r(radius);
                let dir = attach_dir(rng, &brush.attach);
                let center = attach_point(world, rng, dir, r);
                let n = paint_blob(world, solver, painter, rng, center, radius, &entries[0], blob);
                entries[0].count -= n;
                *total -= n;
                let n_f = n as f32;
                let circle = (f64::from(n_f) * f64::from(tile_area) * f64::from(f32::from_bits(0x3EA2_F983))).sqrt() as f32;
                let [rmin, rmax] = micro.ratio;
                let micro_tiles = ftol_d(((f64::from(rmax) - f64::from(rmin)) * rng.at(0x598CF4).unit_d() + f64::from(rmin)) * f64::from(n_f));
                let mut sum = 0i32;
                for e in entries.iter_mut().skip(1) {
                    e.count = ftol_d(f64::from(micro_tiles) * f64::from(e.fraction));
                    sum = sum.wrapping_add(e.count);
                }
                if !land {
                    painter.set_affect_height(micro.affect_height);
                }
                while sum > 0 {
                    let Some(i) = pick_entry(entries, sum, 1, rng) else { break };
                    let [smin, smax] = micro.size;
                    let r = ((f64::from(smax) - f64::from(smin)) * rng.at(0x598DDE).unit_d() + f64::from(smin)) as f32;
                    let c = point_near(world, rng, center, circle);
                    let m = paint_blob(world, solver, painter, rng, c, r, &entries[i], blob);
                    entries[i].count -= m;
                    sum -= m;
                }
                if !land {
                    painter.set_affect_height(true);
                }
            }
        }
        None => {
            while *total > 0 {
                let Some(i) = pick_entry(entries, *total, 0, rng) else { break };
                let [mn, mx] = blob.size;
                let radius = ((f64::from(mx) - f64::from(mn)) * rng.at(0x598EAD).unit_d() + f64::from(mn)) as f32;
                let r = attach_r(radius);
                let dir = attach_dir(rng, &brush.attach);
                let center = attach_point(world, rng, dir, r);
                let n = paint_blob(world, solver, painter, rng, center, radius, &entries[i], blob);
                *total -= n;
                entries[i].count -= n;
            }
        }
    }
    painter.finish(world);
}

/// 0x5526C0: every tile of the GMap whose square meets the circle of radius
/// `r` around `p` (0x46EF30 -> 0x46ED10 -> 0x46E400 with the tile shape
/// 0x46E0E0) goes into `batch` with `var`; rows outer, columns inner.
fn stamp(world: &World, batch: &mut Batch<'_>, p: [f32; 2], r: f32, var: u32) {
    let inv = 1.0 / f64::from(world.tile);
    let (px, py, rr) = (f64::from(p[0]), f64::from(p[1]), f64::from(r));
    let clamp = |v: i32, hi: usize| v.clamp(0, hi as i32 - 1);
    let x0 = clamp(ftol_d((px - rr) * inv), world.tw);
    let y0 = clamp(ftol_d((py - rr) * inv), world.th);
    let x1 = clamp(ftol_d((rr + px) * inv), world.tw);
    let y1 = clamp(ftol_d((rr + py) * inv), world.th);
    let half = (f64::from(world.tile) * 0.5) as f32;
    // The tile square's bounds (0x46E0E0: +-0.5 * tile).
    let hi = (f64::from(world.tile) * 0.5) as f32;
    let lo = (f64::from(world.tile) * -0.5) as f32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let cx = (f64::from(x << world.tile_bits) + f64::from(half)) as f32;
            let cy = (f64::from(y << world.tile_bits) + f64::from(half)) as f32;
            // 0x46EF30: the circle's centre in the tile's frame (identity
            // orientation), stored binary32.
            let dx = (f64::from(p[0]) - f64::from(cx)) as f32;
            let dy = (f64::from(p[1]) - f64::from(cy)) as f32;
            if circle_meets_square(r, [dx, dy], lo, hi) {
                batch.paint(world, y as usize * world.tw + x as usize, var);
            }
        }
    }
}

/// 0x46E400: a circle of radius `r` at `d` against the square [lo, hi]^2.
fn circle_meets_square(r: f32, d: [f32; 2], lo: f32, hi: f32) -> bool {
    let (px, py) = (f64::from(d[0]), f64::from(d[1]));
    let (lo, hi, r) = (f64::from(lo), f64::from(hi), f64::from(r));
    let corner = |ex: f64, ey: f64| r * r > ey * ey + ex * ex;
    if px < lo {
        let ex = px - lo;
        if py > hi {
            corner(ex, py - hi)
        } else if py < lo {
            corner(ex, py - lo)
        } else {
            -ex < r
        }
    } else if px > hi {
        let ex = px - hi;
        if py > hi {
            corner(ex, py - hi)
        } else if py < lo {
            corner(ex, py - lo)
        } else {
            ex < r
        }
    } else if py < lo {
        -(py - lo) < r
    } else if py > hi {
        py - hi < r
    } else {
        true
    }
}

/// 0x5559F0 without its water-area flood: a batch, the stamps along the
/// segment (step = the brush size, at least binary32(trunc(tile) * 0.5);
/// the cursor stores after each stamp; the end point stamped once), commit.
fn stroke(world: &mut World, data: &RmgData, biome: &BiomeDef, from: [f32; 2], to: [f32; 2], size: f32, var: u32) {
    let mut batch = Batch::new(data, biome);
    let dy32 = (f64::from(to[1]) - f64::from(from[1])) as f32;
    let dx = f64::from(to[0]) - f64::from(from[0]);
    let length = (dx * dx + f64::from(dy32) * f64::from(dy32)).sqrt();
    let mut length32 = length as f32;
    let (mut x_dir, mut y_dir) = (dx, f64::from(dy32));
    if length > 0.0 {
        let recip = 1.0 / f64::from(length32);
        x_dir *= recip;
        y_dir = recip * y_dir;
    }
    let minimum = (f64::from(ftol(world.tile)) * 0.5) as f32;
    let step = if size <= minimum { minimum } else { size };
    let mut cursor = from;
    if length32 >= 0.0 {
        let sy = (y_dir * f64::from(step)) as f32;
        let sx = (x_dir * f64::from(step)) as f32;
        loop {
            stamp(world, &mut batch, cursor, size, var);
            cursor[0] = (f64::from(sx) + f64::from(cursor[0])) as f32;
            cursor[1] = (f64::from(cursor[1]) + f64::from(sy)) as f32;
            if length32 <= 0.0 {
                break;
            }
            let rest = f64::from(length32) - f64::from(step);
            length32 = rest as f32;
            if rest <= 0.0 {
                cursor = to;
                length32 = 0.0;
            }
        }
    }
    batch.commit(world);
}

/// DoStreaks (0x597530): continuous strokes of a circle brush.
fn paint_streaks(world: &mut World, data: &RmgData, biome: &BiomeDef, rng: &mut Rng, brush: &BrushDef, streak: &StreakDef, entries: &mut [Entry], total: &mut i32) {
    let tile_i = ftol(world.tile);
    let tile_area = (f64::from(tile_i) * f64::from(tile_i)) as f32;
    let first_kind = data.terrains[data.variations[entries[0].var as usize].terrain].kind;
    let land = first_kind == 0;
    while *total > 0 {
        let Some(index) = pick_entry(entries, *total, 0, rng) else { break };
        let var = entries[index].var;
        let tdef = &data.terrains[data.variations[var as usize].terrain];
        let wspan = (f64::from(streak.width[1]) - f64::from(streak.width[0])) as f32;
        let count_span = streak.streaks[1] - streak.streaks[0];
        let lspan = (f64::from(streak.length[1]) - f64::from(streak.length[0])) as f32;
        let aspan = (f64::from(streak.angle[1]) - f64::from(streak.angle[0])) as f32;
        let r = if brush.attach_range < 0.0 { (f64::from(wspan) * 0.5) as f32 } else { brush.attach_range };
        let dir = attach_dir(rng, &brush.attach);
        let origin = attach_point(world, rng, dir, r);
        let mut start = origin;
        let count = streak.streaks[0] - ftol_d(rng.at(0x5976F7).unit_d() * -1.0 * f64::from(count_span));
        let mut restarts = 0;
        let dx = (rng.at(0x597743).unit_d() * 2.0 - 1.0) as f32;
        let dy = rng.at(0x59777B).unit_d() * 2.0 - 1.0;
        let len = (dy * dy + f64::from(dx) * f64::from(dx)).sqrt();
        let mut dir_v = if len > f64::from(EPS) {
            let inv = 1.0 / len;
            [(f64::from(dx) * inv) as f32, (inv * dy) as f32]
        } else {
            [0.0, 0.0]
        };
        let first_dir = dir_v;
        let mut slope = Slope::new(tdef.max_slope, None);
        let mut carry = 0.0f32;
        let mut i = 0;
        while i < count {
            if entries[index].count <= 0 || restarts >= 2 {
                break;
            }
            let width = (rng.at(0x597874).unit_d() * f64::from(wspan) + f64::from(streak.width[0])) as f32;
            let half_w = (f64::from(width) * 0.5) as f32;
            let length_d = rng.at(0x5978E0).unit_d() * f64::from(lspan) + f64::from(streak.length[0]);
            let length = length_d as f32;
            let mut end = [(length_d * f64::from(dir_v[0]) + f64::from(start[0])) as f32, (f64::from(length) * f64::from(dir_v[1]) + f64::from(start[1])) as f32];
            // 0x596920 on both points against [0, width] x [0, height].
            for p in [&mut start, &mut end] {
                p[0] = if p[0] < 0.0 { 0.0 } else if p[0] > world.width { world.width } else { p[0] };
                p[1] = if p[1] < 0.0 { 0.0 } else if p[1] > world.height { world.height } else { p[1] };
            }
            let ddy = f64::from(start[1]) - f64::from(end[1]);
            let ddx = f64::from(start[0]) - f64::from(end[0]);
            let actual = (ddx * ddx + ddy * ddy).sqrt() as f32;
            let short = (f64::from(length) - f64::from(actual)) as f32;
            stroke(world, data, biome, start, end, half_w, var);
            if !land {
                // The stroke's heights (approximate, see the module notes).
                let mut target = (f64::from(tdef.max_slope) * f64::from(width) * 0.5) as f32;
                if tdef.kind == 2 {
                    target = -target;
                }
                let steps = (actual / world.spacing).ceil().max(1.0) as i32;
                for s in 0..=steps {
                    let f = s as f32 / steps as f32;
                    let p = [start[0] + (end[0] - start[0]) * f, start[1] + (end[1] - start[1]) * f];
                    let (vx, vy) = ((p[0] / world.spacing + 0.5) as i32, (p[1] / world.spacing + 0.5) as i32);
                    let Some(v) = world.vertex(vx, vy) else { continue };
                    let cur = slope.get(&world.overlay, v);
                    let mut h = if target <= 0.0 { cur.min(target) } else { cur.max(target) };
                    h = h.clamp(world.limits[0], world.limits[1]);
                    if h < -tdef.max_depth {
                        h = -tdef.max_depth;
                    }
                    slope.seed(v, h);
                }
            }
            // Tiles covered: the round cap on the first stroke, then the
            // swept band, plus the carried fraction.
            let cap = if i == 0 { f64::from(half_w) * f64::from(half_w) * f64::from(f32::from_bits(0x4049_0FDB)) } else { 0.0 };
            let t = (cap + f64::from(actual) * f64::from(width)) / f64::from(tile_area) + f64::from(carry);
            let whole = ftol_d(t);
            entries[index].count -= whole;
            *total -= whole;
            carry = (t - f64::from(whole)) as f32;
            if short <= EPS {
                start = end;
                let a = (rng.at(0x597D8C).unit_d() * f64::from(aspan) + f64::from(streak.angle[0])) * f64::from(f32::from_bits(0x3C8E_FA35));
                let (s, c) = a.sin_cos();
                let nx = (f64::from(dir_v[1]) * s + c * f64::from(dir_v[0])) as f32;
                let ny_d = c * f64::from(dir_v[1]) - s * f64::from(dir_v[0]);
                let ny = ny_d as f32;
                let l = (ny_d * f64::from(ny) + f64::from(nx) * f64::from(nx)).sqrt();
                dir_v = if l > f64::from(EPS) {
                    let inv = (1.0 / l) as f32;
                    [(f64::from(inv) * f64::from(nx)) as f32, (f64::from(inv) * f64::from(ny)) as f32]
                } else {
                    [0.0, 0.0]
                };
            } else {
                dir_v = [-first_dir[0], -first_dir[1]];
                start = origin;
                restarts += 1;
            }
            i += 1;
        }
        if !land && !slope.is_empty() {
            slope.finish(world, data, true);
            slope.commit(world, true);
        }
    }
}

/// The whole terrain stage (0x599420 / 0x5990D0).
pub(super) fn run(world: &mut World, data: &RmgData, rmc: &RmcDef, biome: &BiomeDef, values: &[f32], rng: &mut Rng) {
    let weights = normalized_weights(data, rmc, values);
    let tiles = (world.tw * world.th) as i32;
    let mut solver = Solver::new(world);
    let mut acc = 0.0f32;
    for (brush, &w) in rmc.brushes.iter().zip(&weights) {
        acc = (f64::from(w) + f64::from(acc)) as f32;
        let w2 = ((1.0 - f64::from(acc)) * f64::from(w) + f64::from(w)) as f32;
        let terrain = &data.terrains[brush.terrain];
        let micro = matches!(&brush.kind, BrushKind::Blob(b) if b.micro.is_some());
        let mut entries = Vec::new();
        let mut total = 0i32;
        let first_weight = terrain.variations.first().map_or(0.0, |&v| data.variations[v].weight);
        for (i, &var) in terrain.variations.iter().enumerate() {
            let vw = data.variations[var].weight;
            let (fraction, count, stop) = if micro && i > 0 {
                ((f64::from(vw) / (f64::from(terrain.total_weight) - f64::from(first_weight))) as f32, 0, false)
            } else if micro {
                (1.0, ftol_d(f64::from(tiles) * f64::from(w2)), false)
            } else if brush.paint_first_variation_only {
                (1.0, ftol_d(f64::from(tiles) * f64::from(w2)), true)
            } else {
                let f = (f64::from(vw) / f64::from(terrain.total_weight)) as f32;
                (f, ftol_d(f64::from(tiles) * f64::from(f) * f64::from(w2)), false)
            };
            entries.push(Entry { var: var as u32, fraction, count });
            total = total.wrapping_add(count);
            if stop {
                break;
            }
        }
        match &brush.kind {
            BrushKind::Blob(blob) => {
                if terrain.kind != 0 {
                    let mut painter = BlockingPainter {
                        data,
                        biome,
                        terrain: brush.terrain,
                        water: terrain.kind == 2,
                        affect_height: true,
                        list: Vec::new(),
                        heights: HashMap::new(),
                        height_order: Vec::new(),
                        slope: Relax::new(terrain.max_slope),
                        radius: 0.0,
                    };
                    paint_blobs(world, &mut solver, &mut painter, rng, brush, blob, &mut entries, &mut total, false);
                } else {
                    let mut painter = LandPainter { data, biome, list: Vec::new() };
                    paint_blobs(world, &mut solver, &mut painter, rng, brush, blob, &mut entries, &mut total, true);
                }
            }
            BrushKind::Streak(s) => paint_streaks(world, data, biome, rng, brush, s, &mut entries, &mut total),
        }
    }
}
