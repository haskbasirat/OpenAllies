//! The two heightmaps of the generated world as the original writes them:
//! - the pending height setters 0x558CE0 (slope spread 0x559470 / 0x559190)
//!   and 0x558D40 (relax 0x558FC0 / 0x558DE0), with their row list, in
//!   insertion order, and the limit 0x558A70;
//! - the height brush 0x551140 with its raster 0x550B00 and functor 0x550660
//!   (modes 0 and 1, one point, the round mode of AddTerrainNoise);
//! - AddTerrainNoise 0x551360 on the base map, and the tile reset 0x54F040 a
//!   mountain or water tile runs when it changes kind;
//! - the render height (query 0x122).
//!
//! Arithmetic is the x87 register at PC53 (binary64 here; every value stays
//! far inside the normal range) with the original's binary32 stores
//! (`as f32`), as `aa_heightfield::brush` proves on its fixtures. That crate
//! keeps the proven soft-float version; this one is the generator's hot path.
use super::data::{NoiseDef, RmgData};
use super::grid::{NEIGHBORS, World};
use super::rng::Rng;
use std::collections::HashMap;

/// `0x9FC0BC`, binary32 0.0001.
const EPS: f32 = f32::from_bits(0x38D1_B717);
/// `0xA363AC`, binary32 sqrt(2).
const SQRT2: f32 = f32::from_bits(0x3FB5_04F3);

/// GWorld+0x34 (HEIGHT_MAP) or GWorld+0x38 (MOUNTAIN_HEIGHT_MAP).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Map {
    Base,
    Overlay,
}

impl Map {
    fn field(self, world: &World) -> &[f32] {
        match self {
            Map::Base => &world.base,
            Map::Overlay => &world.overlay,
        }
    }
    fn field_mut(self, world: &mut World) -> &mut [f32] {
        match self {
            Map::Base => &mut world.base,
            Map::Overlay => &mut world.overlay,
        }
    }
}

/// One pending row (16 bytes natively: cell, height, depth).
#[derive(Clone, Copy, Debug)]
struct Row {
    x: i32,
    y: i32,
    h: f32,
    depth: i32,
}

/// The pending setter of one height operation (0x558CE0 / 0x558D40: +4 the
/// map, +8 the rows, +0x14 their hash, +0x28/+0x2C the falloff centre, +0x30
/// its distance, +0x34 the slope, +0x38 / +0x3C spacing and diagonal, +0x40
/// the protect byte or the relax pass, +0x41 blocked, +0x42 the area).
pub(super) struct Setter {
    map: Map,
    slope: f32,
    falloff: Option<([f32; 2], f32)>,
    protect: bool,
    area: u8,
    blocked: bool,
    rows: Vec<Row>,
    keys: HashMap<(i32, i32), usize>,
}

impl Setter {
    /// 0x558CE0.
    pub(super) fn new(map: Map, slope: f32, protect: bool, area: u8) -> Self {
        Self { map, slope, falloff: None, protect, area, blocked: false, rows: Vec::new(), keys: HashMap::new() }
    }
    /// 0x558D40 (no protect, no falloff).
    pub(super) fn relax(map: Map, slope: f32) -> Self {
        Self::new(map, slope, false, 0)
    }
    /// 0x558900.
    fn set_falloff(&mut self, center: [f32; 2], distance: f32) {
        self.falloff = (distance > 0.0).then_some((center, distance));
    }
    pub(super) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    fn put(&mut self, x: i32, y: i32, h: f32, depth: i32) {
        if let Some(&i) = self.keys.get(&(x, y)) {
            self.rows[i].h = h;
            return;
        }
        self.keys.insert((x, y), self.rows.len());
        self.rows.push(Row { x, y, h, depth });
    }
    /// 0x5590F0: a new row at depth 0, or the row's height.
    pub(super) fn set(&mut self, x: i32, y: i32, h: f32) {
        self.put(x, y, h, 0);
    }
    /// 0x5593D0: a new row at depth -1, or the row's height.
    pub(super) fn set_relax(&mut self, x: i32, y: i32, h: f32) {
        self.put(x, y, h, -1);
    }
    /// 0x558D90: the pending height, else the map's.
    pub(super) fn get(&self, world: &World, x: i32, y: i32) -> f32 {
        self.keys.get(&(x, y)).map_or_else(|| self.map.field(world)[y as usize * world.hw + x as usize], |&i| self.rows[i].h)
    }

    /// 0x558A70: +0x34, or the terrain's max_slope (+0x5C) under the vertex
    /// when asked and not zero, then scaled by the falloff.
    fn limit(&self, world: &World, data: &RmgData, x: i32, y: i32, use_terrain: bool) -> f64 {
        let mut value = f64::from(self.slope);
        if use_terrain {
            let px = (x << world.height_bits) as f32;
            let py = (y << world.height_bits) as f32;
            // 0x4282A0: 0 <= v < extent, then the tile.
            if px >= 0.0 && px < world.width && py >= 0.0 && py < world.height {
                let (tx, ty) = ((px as i32) >> world.tile_bits, (py as i32) >> world.tile_bits);
                if tx >= 0 && ty >= 0 && (tx as usize) < world.tw && (ty as usize) < world.th {
                    let v = world.var[ty as usize * world.tw + tx as usize];
                    let s = data.terrains[data.variations[v as usize].terrain].max_slope;
                    value = if s == 0.0 { f64::from(self.slope) } else { f64::from(s) };
                }
            }
        }
        if let Some((c, d)) = self.falloff {
            let s = f64::from(world.spacing);
            let dx = (f64::from(c[0]) - f64::from(x) * s) as f32;
            let dy = f64::from(c[1]) - f64::from(y) * s;
            let r = (f64::from(dx) * f64::from(dx) + dy * dy).sqrt() / f64::from(d);
            if r < 0.0 {
                return 0.0 * value;
            }
            value *= if r > 1.0 { 1.0 } else { r };
        }
        value
    }

    /// 0x559190: pull the eight neighbours of a row towards it.
    fn spread(&mut self, world: &World, data: &RmgData, row: Row) {
        let limit = self.limit(world, data, row.x, row.y, row.depth > 0) as f32;
        let field = self.map.field(world);
        let diagonal = (f64::from(world.spacing) * f64::from(SQRT2)) as f32;
        for d in NEIGHBORS {
            let (nx, ny) = (row.x + d[0], row.y + d[1]);
            if nx < 0 || ny < 0 || nx as usize >= world.hw || ny as usize >= world.hh {
                continue;
            }
            let existing = self.keys.get(&(nx, ny)).copied();
            let old = match existing {
                Some(i) => {
                    if self.rows[i].depth < row.depth {
                        continue;
                    }
                    self.rows[i].h
                }
                None => field[ny as usize * world.hw + nx as usize],
            };
            let diff = (f64::from(old) - f64::from(row.h)) as f32;
            let dist = if d[0] == 0 || d[1] == 0 { world.spacing } else { diagonal };
            let ratio = (f64::from(diff) / f64::from(dist)).abs();
            if !(f64::from(limit) + f64::from(EPS) < ratio) {
                continue;
            }
            if self.protect {
                // 0x550010: the vertex carries another water area.
                let a = world.vertex_areas(self.map)[ny as usize * world.hw + nx as usize];
                if a != 0 && a != self.area {
                    self.blocked = true;
                    return;
                }
            }
            let sign = if diff < 0.0 { -1.0f64 } else { 1.0 };
            let value = (sign * f64::from(dist) * f64::from(limit) + f64::from(row.h)) as f32;
            match existing {
                Some(i) => self.rows[i].h = value,
                None => {
                    self.keys.insert((nx, ny), self.rows.len());
                    self.rows.push(Row { x: nx, y: ny, h: value, depth: row.depth + 1 });
                }
            }
        }
    }

    /// 0x559470: spread every row in list order (the list grows), then
    /// commit them; nothing commits once a protected vertex was met.
    pub(super) fn finish(&mut self, world: &mut World, data: &RmgData) -> bool {
        self.blocked = false;
        let mut cursor = 0;
        while cursor < self.rows.len() {
            if self.blocked {
                return false;
            }
            let row = self.rows[cursor];
            cursor += 1;
            self.spread(world, data, row);
        }
        if self.blocked {
            return false;
        }
        let map = self.map;
        for r in &self.rows {
            commit(world, map, r.x, r.y, r.h);
        }
        true
    }

    /// 0x558DE0: one row of relax pass `pass`.
    fn relax_row(&mut self, world: &mut World, index: usize, pass: i32) {
        let Row { x, y, h, .. } = self.rows[index];
        // 0x558A70(row, 0): +0x34 (no falloff on these setters).
        let limit = self.slope;
        let diagonal = (f64::from(world.spacing) * f64::from(SQRT2)) as f32;
        let mut sum = 0.0f64;
        let mut count = 0;
        for d in NEIGHBORS {
            let (nx, ny) = (x + d[0], y + d[1]);
            if nx < 0 || ny < 0 || nx as usize >= world.hw || ny as usize >= world.hh {
                continue;
            }
            let field = self.map.field(world)[ny as usize * world.hw + nx as usize];
            let level = self.keys.get(&(nx, ny)).map_or(0, |&i| self.rows[i].depth);
            if level == pass {
                self.rows[index].depth = pass + 1;
            }
            if self.rows[index].depth == pass + 1 && level == pass {
                let diff = (f64::from(field) - f64::from(h)) as f32;
                let dist = if d[0] == 0 || d[1] == 0 { world.spacing } else { diagonal };
                let ratio = (f64::from(diff) / f64::from(dist)).abs();
                if f64::from(limit) + f64::from(EPS) < ratio {
                    let sign = if diff < 0.0 { -1.0f64 } else { 1.0 };
                    count += 1;
                    sum += f64::from(field) - sign * f64::from(dist) * f64::from(limit);
                }
            }
        }
        if count > 0 {
            self.rows[index].h = (sum / f64::from(count)) as f32;
        }
        let map = self.map;
        commit(world, map, x, y, self.rows[index].h);
    }

    /// 0x558FC0: passes over the rows still at depth -1, from the outside
    /// in, until none is left; every such row commits on every pass.
    pub(super) fn finish_relax(&mut self, world: &mut World) {
        let mut pass = 0;
        loop {
            let mut left = 0;
            let mut cursor = 0;
            while cursor < self.rows.len() {
                let i = cursor;
                cursor += 1;
                if self.rows[i].depth == -1 {
                    self.relax_row(world, i, pass);
                }
                if self.rows[i].depth == -1 {
                    left += 1;
                }
            }
            pass += 1;
            // A set of rows no outside vertex touches never ends natively.
            if left == 0 || pass > (world.hw + world.hh) as i32 {
                break;
            }
        }
    }
}

/// 0x550A10: the height clamped to the map's limits (0x401290).
fn commit(world: &mut World, map: Map, x: i32, y: i32, h: f32) {
    let [lo, hi] = world.limits;
    let v = if h < lo {
        lo
    } else if h > hi {
        hi
    } else {
        h
    };
    let at = y as usize * world.hw + x as usize;
    map.field_mut(world)[at] = v;
}

/// The settings block (0x4EB500 layout) of one brush.
#[derive(Clone, Copy, Debug)]
pub(super) struct Brush {
    /// +0: 0 sets `height`, 1 adds it.
    pub mode: u32,
    pub height: f32,
    pub slope: f32,
    pub min: f32,
    pub max: f32,
    pub protect: bool,
    pub smooth_edge: bool,
    pub strength: f32,
    pub falloff: f32,
    pub area: u8,
}

/// The parity probe (`AA_RMG_BR` names a file): every height brush with the
/// FNV hashes of both maps at its entry, as the RmgProbe sampler logs the
/// original's (BR lines).
fn probe_brush(world: &World, data: &RmgData, map: Map, radius: f32, pos: [f32; 2], b: &Brush) {
    static PATH: std::sync::OnceLock<Option<std::path::PathBuf>> = std::sync::OnceLock::new();
    let Some(path) = PATH.get_or_init(|| std::env::var_os("AA_RMG_BR").map(Into::into)) else { return };
    let hash = |v: &[f32]| {
        let mut h = 14695981039346656037u64;
        for f in v {
            h ^= u64::from(f.to_bits());
            h = h.wrapping_mul(1099511628211);
        }
        h
    };
    use std::io::Write;
    static INDEX: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(-1);
    let index = INDEX.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(
            f,
            "BR {} {:08x} from {:08x},{:08x} mode {} h {:08x} slope {:08x} fall {:08x} hb {:016x} ho {:016x}",
            if map == Map::Base { "base" } else { "overlay" },
            radius.to_bits(),
            pos[0].to_bits(),
            pos[1].to_bits(),
            b.mode,
            b.height.to_bits(),
            b.slope.to_bits(),
            b.falloff.to_bits(),
            hash(&world.base),
            hash(&world.overlay)
        );
    }
}

/// One bound of 0x550B00: floor or ceil of (p -+ r) / spacing (the CRT on a
/// double), times the spacing, truncated, biased, shifted, clamped.
fn raster_bound(world: &World, p: f64, limit: usize, upper: bool) -> i32 {
    let s = f64::from(world.spacing);
    let q = p / s;
    let rounded = if upper { q.ceil() } else { q.floor() };
    let v = (rounded * s).trunc() as i32;
    let bias = 1 << (world.height_bits - 1);
    ((v + bias) >> world.height_bits).clamp(0, limit as i32 - 1)
}

/// 0x551140 with equal end points, a circle of `radius` at `pos`: 0x550B00
/// (the functor 0x550660) then 0x559470. `round` is map +0x20, set only by
/// AddTerrainNoise. Returns false when a protected vertex stopped it.
pub(super) fn brush(world: &mut World, data: &RmgData, map: Map, round: bool, radius: f32, pos: [f32; 2], b: &Brush) -> bool {
    brush_region(world, data, map, round, radius, pos, b, &|x, y| contains_circle(radius, pos, x, y), &|x, y| distance_circle(radius, pos, x, y))
}

#[allow(clippy::too_many_arguments)]
fn brush_region(world: &mut World, data: &RmgData, map: Map, round: bool, radius: f32, pos: [f32; 2], b: &Brush, contains: &dyn Fn(f32, f32) -> bool, distance: &dyn Fn(f32, f32) -> f64) -> bool {
    probe_brush(world, data, map, radius, pos, b);
    let mut setter = Setter::new(map, b.slope, b.protect, b.area);
    let (px, py, r) = (f64::from(pos[0]), f64::from(pos[1]), f64::from(radius));
    let x0 = raster_bound(world, px - r, world.hw, false);
    let y0 = raster_bound(world, py - r, world.hh, false);
    let x1 = raster_bound(world, px + r, world.hw, true);
    let y1 = raster_bound(world, py + r, world.hh, true);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let vx = (x << world.height_bits) as f32;
            let vy = (y << world.height_bits) as f32;
            let strength = if round {
                let dx = (px - f64::from(vx)) as f32;
                let dy = py - f64::from(vy);
                let d = ((dy * dy + f64::from(dx) * f64::from(dx)).sqrt()) / r;
                let s = (1.0 - d * d) as f32;
                if !(s >= 0.0) {
                    continue;
                }
                if s > 1.0 {
                    1.0
                } else if s <= 0.0 {
                    continue;
                } else {
                    s
                }
            } else if contains(vx, vy) {
                1.0
            } else {
                if !b.smooth_edge {
                    continue;
                }
                let d = distance(vx, vy);
                let reach = f64::from(world.spacing) * f64::from(SQRT2);
                if !(d <= reach) {
                    continue;
                }
                let s = ((reach - d) / reach) as f32;
                if s <= 0.0 {
                    continue;
                }
                s
            };
            // 0x550660.
            let old = world.height_at(map, x, y);
            let target = match b.mode {
                0 => b.height,
                _ => (f64::from(old) + f64::from(b.height)) as f32,
            };
            let target = if target < b.min {
                b.min
            } else if target > b.max {
                b.max
            } else {
                target
            };
            let delta = f64::from(target) - f64::from(old);
            if !(delta.abs() > f64::from(EPS)) {
                continue;
            }
            if b.protect {
                let a = world.vertex_areas(map)[y as usize * world.hw + x as usize];
                if a != 0 && a != b.area {
                    return false;
                }
            }
            setter.set(x, y, (delta * f64::from(b.strength) * f64::from(strength) + f64::from(old)) as f32);
        }
    }
    setter.set_falloff(pos, b.falloff);
    setter.finish(world, data)
}

/// 0x46E150 + 0x46F280 for a box at its facing: the distance from a vertex to
/// the closest point of the box, 0 inside (binary32 stores as the original).
fn distance_box(place: &aa_spatial::geometry::Placement, x: f32, y: f32) -> f64 {
    let f = |b: u32| f32::from_bits(b);
    let (px, py) = (f(place.position[0]), f(place.position[1]));
    let (fx, fy) = (f64::from(f(place.facing[0])), f64::from(f(place.facing[1])));
    let [min_x, min_y, max_x, max_y] = place.shape.bounds.map(f);
    let dx = (f64::from(x) - f64::from(px)) as f32;
    let dy = (f64::from(y) - f64::from(py)) as f32;
    let (ddx, ddy) = (f64::from(dx), f64::from(dy));
    let mut lx = (fy * ddx - ddy * fx) as f32;
    let mut ly = fy * ddy + fx * ddx;
    if lx < min_x {
        lx = min_x;
    } else if lx > max_x {
        lx = max_x;
    }
    if ly < f64::from(min_y) {
        ly = f64::from(min_y);
    } else if ly > f64::from(max_y) {
        ly = f64::from(max_y);
    }
    let qx = (fx * ly + fy * f64::from(lx)) as f32;
    let qy = (ly * fy - fx * f64::from(lx)) as f32;
    let ex = ((f64::from(px) + f64::from(qx)) - f64::from(x)) as f32;
    let ey = (f64::from(qy) + f64::from(py)) - f64::from(y);
    let d = (ey * ey + f64::from(ex) * f64::from(ex)).sqrt() as f32;
    if d < f(place.shape.radius_bits) {
        let ix = (ddx * fy - ddy * fx) as f32;
        let iy = (ddx * fx + ddy * fy) as f32;
        if ix >= min_x && ix <= max_x && iy >= min_y && iy <= max_y {
            return 0.0;
        }
    }
    f64::from(d)
}

/// 0x551140 with equal end points, any placed shape: 0x550B00 (the functor
/// 0x550660) then 0x559470. A point stops there; a circle is [`brush`].
pub(super) fn brush_shape(world: &mut World, data: &RmgData, map: Map, place: &aa_spatial::geometry::Placement, b: &Brush) -> bool {
    let pos = place.position.map(f32::from_bits);
    let radius = f32::from_bits(place.shape.radius_bits);
    match place.shape.kind {
        1 => brush(world, data, map, false, radius, pos, b),
        2 | 3 => {
            let point = aa_spatial::geometry::Placement { shape: aa_spatial::collision::Shape { kind: 0, radius_bits: 0, bounds: [0; 4] }, facing: [0, 0xbf80_0000], position: [0, 0] };
            let contains = |x: f32, y: f32| {
                let vertex = aa_spatial::geometry::Placement { position: [x.to_bits(), y.to_bits()], ..point.clone() };
                aa_spatial::shape_overlap::overlaps(&vertex, place, aa_numeric::Precision::Double53).unwrap_or(false)
            };
            let distance = |x: f32, y: f32| distance_box(place, x, y);
            brush_region(world, data, map, false, radius, pos, b, &contains, &distance)
        }
        _ => true,
    }
}

/// 0x46EF30 for a circle against a point.
fn contains_circle(radius: f32, pos: [f32; 2], x: f32, y: f32) -> bool {
    let dx = (f64::from(x) - f64::from(pos[0])) as f32;
    let dy = (f64::from(y) - f64::from(pos[1])) as f32;
    let r = f64::from(radius);
    f64::from(dx) * f64::from(dx) + f64::from(dy) * f64::from(dy) + f64::from(EPS) <= r * r
}

/// 0x46F280 for a circle at the default facing: the distance to the closest
/// point of the circle (0x46E150 through 0x409230), stored binary32.
fn distance_circle(radius: f32, pos: [f32; 2], x: f32, y: f32) -> f64 {
    let mut ux = (f64::from(x) - f64::from(pos[0])) as f32;
    let mut uy = (f64::from(y) - f64::from(pos[1])) as f32;
    let len = (f64::from(ux) * f64::from(ux) + f64::from(uy) * f64::from(uy)).sqrt();
    if len > f64::from(EPS) {
        let inv = 1.0 / len;
        ux = (inv * f64::from(ux)) as f32;
        uy = (inv * f64::from(uy)) as f32;
    } else {
        ux = 0.0;
        uy = 0.0;
    }
    let ox = (f64::from(ux) * f64::from(radius)) as f32;
    let oy = (f64::from(uy) * f64::from(radius)) as f32;
    let fx = ((f64::from(pos[0]) + f64::from(ox)) - f64::from(x)) as f32;
    let fy = (f64::from(oy) + f64::from(pos[1])) - f64::from(y);
    let d = (fy * fy + f64::from(fx) * f64::from(fx)).sqrt() as f32;
    if d < radius && contains_circle(radius, pos, x, y) {
        return 0.0;
    }
    f64::from(d)
}

/// 0x54F040: a mountain or water tile changing kind brushes the overlay back
/// to 0 under it (mode 0, a circle of one tile's size, smooth edge, the old
/// terrain's max_slope, no falloff).
pub(super) fn reset_tile_overlay(world: &mut World, data: &RmgData, tile: usize, old_slope: f32) {
    let half = f64::from(world.tile) * 0.5;
    let [tx, ty] = world.tile_xy(tile);
    let pos = [(f64::from(tx << world.tile_bits) + half) as f32, (f64::from(ty << world.tile_bits) + half) as f32];
    let b = Brush { mode: 0, height: 0.0, slope: old_slope, min: f32::MIN, max: f32::MAX, protect: false, smooth_edge: true, strength: 1.0, falloff: -1.0, area: 0 };
    let radius = world.tile;
    brush(world, data, Map::Overlay, false, radius, pos, &b);
}

/// AddTerrainNoise (0x551360): the blob count (CRT pow 0x96D890 of the area
/// factor), then per blob x, y, radius, sign and magnitude, then the height
/// brush 0x551140: a round brush of radius 1 m adding the magnitude, slope
/// |magnitude / (radius / 2)|, protect on, falloff TerrainNoise size max.
pub(super) fn add_noise(world: &mut World, data: &RmgData, noise: &NoiseDef, rng: &mut Rng) {
    let inv = 1.0 / f64::from(noise.reference_size);
    let area = (f64::from(world.height) * f64::from(world.width) * inv * inv) as f32;
    let u = rng.at(0x5513AA).unit_d();
    let base = u * f64::from(noise.num[1].wrapping_sub(noise.num[0])) + f64::from(noise.num[0]);
    let p = aa_numeric::crt_pow::ci_pow(f64::from(noise.area_factor), f64::from(area) - 1.0, aa_numeric::crt_pow::CrtEnvironment::SSE2_DOUBLE53)
        .map(|o| o.value())
        .unwrap_or_else(|_| f64::from(noise.area_factor).powf(f64::from(area) - 1.0));
    let count = super::blob::ftol_d(base * (p * f64::from(area)));
    let size_span = (f64::from(noise.size[1]) - f64::from(noise.size[0])) as f32;
    for _ in 0..count.max(0) {
        let x = (rng.at(0x5514BB).unit_d() * f64::from(world.width)) as f32;
        let y = (rng.at(0x5514F9).unit_d() * f64::from(world.height)) as f32;
        let radius = (rng.at(0x55152D).unit_d() * f64::from(size_span) + f64::from(noise.size[0])) as f32;
        let sign: f32 = if rng.at(0x551564).unit_d() < f64::from(noise.up_frequency) { 1.0 } else { -1.0 };
        let lo = (f64::from(sign) * f64::from(noise.magnitude[0])) as f32;
        let u = rng.at(0x5515B5).unit_d();
        let magnitude = ((f64::from(sign) * f64::from(noise.magnitude[1]) - f64::from(lo)) * u + f64::from(lo)) as f32;
        let slope = (f64::from(magnitude) / (f64::from(radius) * 0.5)).abs() as f32;
        let b = Brush { mode: 1, height: magnitude, slope, min: f32::MIN, max: f32::MAX, protect: true, smooth_edge: true, strength: 1.0, falloff: noise.size[1], area: 0 };
        brush(world, data, Map::Base, true, 1.0, [x, y], &b);
    }
}

/// 0x550190: the nearest sample of a map.
fn nearest(world: &World, map: Map, p: [f32; 2]) -> f32 {
    let bias = 1 << (world.height_bits - 1);
    let x = (((p[0] as i32).wrapping_add(bias)) >> world.height_bits).clamp(0, world.hw as i32 - 1);
    let y = (((p[1] as i32).wrapping_add(bias)) >> world.height_bits).clamp(0, world.hh as i32 - 1);
    world.height_at(map, x, y)
}

/// 0x550210: the height of a map at a point: the sample on a lattice point,
/// the middle of two on a lattice line, else bilinear (register result).
pub(super) fn bilinear(world: &World, map: Map, p: [f32; 2]) -> f64 {
    let s = f64::from(world.spacing);
    let eps = f64::from(EPS);
    let rx = (f64::from(p[0]) % s) as f32;
    let ry = f64::from(p[1]) % s;
    if f64::from(rx) < eps {
        if ry < eps {
            return f64::from(nearest(world, map, p));
        }
        let half = (s * 0.5) as f32;
        let a = nearest(world, map, [p[0], (f64::from(p[1]) - f64::from(half)) as f32]);
        let b = nearest(world, map, [p[0], (f64::from(half) + f64::from(p[1])) as f32]);
        return (f64::from(b) + f64::from(a)) * 0.5;
    }
    if ry < eps {
        let half = (s * 0.5) as f32;
        let a = nearest(world, map, [(f64::from(half) + f64::from(p[0])) as f32, p[1]]);
        let b = nearest(world, map, [(f64::from(p[0]) - f64::from(half)) as f32, p[1]]);
        return (f64::from(b) + f64::from(a)) * 0.5;
    }
    let inv = 1.0 / s;
    let fx = (f64::from(rx) * inv) as f32;
    let fy = (inv * ry) as f32;
    let (fx, fy) = (f64::from(fx), f64::from(fy));
    let (w, h) = (world.hw as i32, world.hh as i32);
    let mut ix = ((p[0] as i32) >> world.height_bits).max(0);
    let mut iy = ((p[1] as i32) >> world.height_bits).max(0);
    if ix >= w - 1 {
        ix = w - 1;
    }
    if iy >= h - 1 {
        iy = h - 1;
    }
    let at = |x: i32, y: i32| f64::from(world.height_at(map, x, y));
    let ox = 1.0 - fx;
    let oy = 1.0 - fy;
    let w1 = f64::from((fy * ox) as f32);
    let w2 = f64::from((fx * oy) as f32);
    let here = at(ix, iy);
    let (more_x, more_y) = (ix + 1 < w, iy + 1 < h);
    let mut v = ox * oy * here;
    v += w2 * if more_x { at(ix + 1, iy) } else { here };
    v += w1 * if more_y { at(ix, iy + 1) } else { here };
    v + fy * fx * if more_x && more_y { at(ix + 1, iy + 1) } else { here }
}

/// GWorld::GetHeight 0x4F0E50: flag 0x10 the nearest base sample instead of
/// the bilinear one; 1 with 4 the base less half a metre; 2 the overlay where
/// the tile or one of its neighbours is mountain or water, plus the mountain
/// noise; 1 the result at least the base less half a metre.
pub(super) fn surface(world: &World, data: &RmgData, p: [f32; 2], flags: u32) -> f64 {
    let base = if flags & 0x10 == 0 { bilinear(world, Map::Base, p) as f32 } else { nearest(world, Map::Base, p) };
    if flags & 1 != 0 && flags & 4 != 0 {
        return f64::from(base) - 0.5;
    }
    let mut over = 0.0f64;
    if flags & 2 != 0 {
        let x = ((p[0] as i32) >> world.tile_bits).clamp(0, world.tw as i32 - 1);
        let y = ((p[1] as i32) >> world.tile_bits).clamp(0, world.th as i32 - 1);
        let terrain = |x: i32, y: i32| &data.terrains[data.variations[world.var[y as usize * world.tw + x as usize] as usize].terrain];
        let t = terrain(x, y);
        let lifted = matches!(t.kind, 1 | 2)
            || NEIGHBORS.iter().any(|d| {
                let (nx, ny) = (x + d[0], y + d[1]);
                nx >= 0 && ny >= 0 && (nx as usize) < world.tw && (ny as usize) < world.th && matches!(terrain(nx, ny).kind, 1 | 2)
            });
        if lifted {
            over = bilinear(world, Map::Overlay, p);
            if t.kind == 1 {
                let index = (world.tw as i32 * y + x).wrapping_mul(0x1273).wrapping_add(0x18B3);
                let hash = (index % 0x1B55) % 1000;
                let span = f64::from(t.noise[1]) - f64::from(t.noise[0]);
                over += f64::from(hash) * span * f64::from(f32::from_bits(0x3A83_126F)) + f64::from(t.noise[0]);
            }
        }
    }
    let sum = (over + f64::from(base)) as f32;
    if flags & 1 != 0 {
        let low = f64::from(base) - 0.5;
        if low > f64::from(sum) {
            return low;
        }
    }
    f64::from(sum)
}

/// Native height of a lattice corner (query 0x122 without the low clamp).
pub(super) fn native_corner(world: &World, data: &RmgData, x: usize, y: usize) -> f32 {
    let v = y * world.hw + x;
    let base = world.base[v];
    let scale = (world.tile / world.spacing).max(1.0) as usize;
    let tx = (x / scale).min(world.tw - 1);
    let ty = (y / scale).min(world.th - 1);
    let kind = |cx: usize, cy: usize| data.terrains[data.variations[world.var[cy * world.tw + cx] as usize].terrain].kind;
    let current = data.variations[world.var[ty * world.tw + tx] as usize].terrain;
    let mut add = matches!(kind(tx, ty), 1 | 2);
    for cy in ty.saturating_sub(1)..=(ty + 1).min(world.th - 1) {
        for cx in tx.saturating_sub(1)..=(tx + 1).min(world.tw - 1) {
            add |= matches!(kind(cx, cy), 1 | 2);
        }
    }
    if !add {
        return base;
    }
    let t = &data.terrains[current];
    let noise = if t.kind == 1 {
        let index = (ty * world.tw + tx) as i32;
        let hash = index.wrapping_mul(4723).wrapping_add(6323);
        let factor = (hash % 6997) % 1000;
        (t.noise[1] - t.noise[0]) * factor as f32 * 0.001 + t.noise[0]
    } else {
        0.0
    };
    base + world.overlay[v] + noise
}
