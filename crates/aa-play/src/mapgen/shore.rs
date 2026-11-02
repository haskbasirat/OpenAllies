//! The water doodads (KKC_GMap's water edge, 0x555F20 -> 0x552010): the
//! water-level contour of the surface around every water tile (0x55C670 with
//! its triangle cut 0x55BB60), welded into polylines (0x55BF70, walked by
//! 0x55A470) and a doodad line along each one (0x55B4E0: normals 0x55B0F0 /
//! 0x55A8F0, the side test 0x55B010, a doodad every
//! water_doodad_offset_parallel at water_doodad_offset_perpendicular from the
//! line).
//!
//! The native also keeps the polylines for the water renderer (GMap+0x5C,
//! 0x55D790); that part has no effect on the world.
//!
//! Arithmetic: the x87 register at PC53 with the original's binary32 stores.
use super::super::height::surface;
use super::super::rng::Rng;
use super::{Placement, World};
use std::collections::HashMap;

/// `0xA3AF18`, `0xA3AF34`: +-0.0001; `0xA21CCC`: binary32 1.0001; `0xA3AF30`: 1/64.
const EPS: f32 = f32::from_bits(0x38D1_B717);
const LOW: f32 = f32::from_bits(0xB8D1_B717);
const HIGH: f32 = f32::from_bits(0x3F80_0347);
const WELD: f32 = f32::from_bits(0x3C80_0000);
/// `0xA363AC`.
const SQRT2: f32 = f32::from_bits(0x3FB5_04F3);

/// One end of a contour segment (0x18 bytes: +0 point, +0xC the welded end of
/// the next segment, +0x10 the other end of this one, +0x14 used).
struct End {
    p: [f32; 3],
    link: Option<usize>,
    other: usize,
    done: bool,
}

/// One edge of 0x55BB60: the level's crossing from `a` to `b`, if any.
fn crossing(a: &[f32; 3], b: &[f32; 3], level: f32) -> Option<[f32; 2]> {
    let dx = (f64::from(b[0]) - f64::from(a[0])) as f32;
    let dy = (f64::from(b[1]) - f64::from(a[1])) as f32;
    let dz = (f64::from(b[2]) - f64::from(a[2])) as f32;
    if !(f64::from(dz).abs() > f64::from(EPS)) {
        return None;
    }
    let mut t = (f64::from(level) - f64::from(a[2])) / f64::from(dz);
    if !(t > f64::from(LOW)) || !(t < f64::from(HIGH)) {
        return None;
    }
    if t < 0.0 {
        t = 0.0;
    } else if t > 1.0 {
        t = 1.0;
    }
    Some([(f64::from(dx) * t + f64::from(a[0])) as f32, (f64::from(dy) * t + f64::from(a[1])) as f32])
}

/// 0x55BB60: the segment where the level cuts the triangle.
fn cut(ends: &mut Vec<End>, a: &[f32; 3], b: &[f32; 3], c: &[f32; 3], level: f32) {
    let mut found: Vec<[f32; 2]> = Vec::with_capacity(3);
    found.extend(crossing(a, b, level));
    found.extend(crossing(b, c, level));
    found.extend(crossing(c, a, level));
    let close = |p: [f32; 2], q: [f32; 2]| {
        let x = f64::from(p[0]) - f64::from(q[0]);
        let y = f64::from(p[1]) - f64::from(q[1]);
        (y * y + x * x).sqrt() < f64::from(WELD)
    };
    let (p, mut q) = match found.len() {
        2 | 3 => (found[0], found[1]),
        _ => return,
    };
    if found.len() == 2 {
        if close(p, q) {
            return;
        }
    } else if close(p, q) {
        // Here the second distance goes through binary32 stores.
        let x = (f64::from(found[1][0]) - f64::from(found[2][0])) as f32;
        let y = (f64::from(found[1][1]) - f64::from(found[2][1])) as f32;
        if (f64::from(y) * f64::from(y) + f64::from(x) * f64::from(x)).sqrt() < f64::from(WELD) {
            return;
        }
        q = found[2];
    }
    let i = ends.len();
    ends.push(End { p: [p[0], p[1], level], link: None, other: i + 1, done: false });
    ends.push(End { p: [q[0], q[1], level], link: None, other: i, done: false });
}

/// 0x55AA70 and its inline copies: the key of a point at `precision` cells
/// per metre.
fn key(p: &[f32; 3], precision: u32, width: f32) -> u32 {
    let f = precision as f32;
    let x = ((f64::from(f) * f64::from(p[0]) + 0.5).floor()) as i32;
    let y = ((f64::from(f) * f64::from(p[1]) + 0.5).floor()) as i32;
    let w = (f64::from(f) * f64::from(width)) as i32;
    w.wrapping_mul(y).wrapping_add(x) as u32
}

/// 0x55BF70: weld the segment ends. First, at 64 cells per metre, a segment
/// doubling another is taken out; then the ends meet, the unmatched ones again
/// at half the precision until none is left.
fn weld(ends: &mut [End], width: f32) {
    let mut table: HashMap<u32, usize> = HashMap::new();
    for i in 0..ends.len() {
        let k = key(&ends[i].p, 0x40, width);
        match table.get(&k).copied() {
            Some(found) => {
                let partner = ends[i].other;
                if found != i && found != partner && key(&ends[partner].p, 0x40, width) != key(&ends[ends[found].other].p, 0x40, width) {
                    continue;
                }
                ends[i].done = true;
                ends[partner].done = true;
                let k2 = key(&ends[partner].p, 0x40, width);
                if table.get(&k2) == Some(&partner) {
                    table.remove(&k2);
                }
            }
            None => {
                table.insert(k, i);
            }
        }
    }
    let mut precision = 0x40u32;
    loop {
        let mut any = false;
        table.clear();
        for i in 0..ends.len() {
            if ends[i].link.is_some() || ends[i].done {
                continue;
            }
            any = true;
            let k = key(&ends[i].p, precision, width);
            match table.get(&k).copied() {
                Some(found) => {
                    if ends[found].link.is_none() {
                        ends[found].link = Some(i);
                        ends[i].link = Some(found);
                        continue;
                    }
                    let partner = ends[i].other;
                    if let Some(l) = ends[partner].link {
                        ends[l].link = None;
                        ends[partner].link = None;
                    }
                    ends[i].done = true;
                    ends[partner].done = true;
                    let k2 = key(&ends[partner].p, precision, width);
                    if table.get(&k2) == Some(&partner) {
                        table.remove(&k2);
                    }
                }
                None => {
                    table.insert(k, i);
                }
            }
        }
        precision >>= 1;
        if !any || precision == 0 {
            break;
        }
    }
}

/// 0x4453A0.
fn normalized(v: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = v.map(f64::from);
    let len = (z * z + y * y + x * x).sqrt();
    if len > f64::from(EPS) {
        let inv = 1.0 / len;
        [(inv * x) as f32, (inv * y) as f32, (inv * z) as f32]
    } else {
        [0.0; 3]
    }
}

fn sub(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [(f64::from(a[0]) - f64::from(b[0])) as f32, (f64::from(a[1]) - f64::from(b[1])) as f32, (f64::from(a[2]) - f64::from(b[2])) as f32]
}

/// 0x55A8F0: the normal at `b` between its neighbours `a` and `c`.
fn corner(a: &[f32; 3], b: &[f32; 3], c: &[f32; 3]) -> [f32; 3] {
    let e1 = normalized(sub(b, a));
    let e2 = normalized(sub(c, b));
    let sy = (f64::from(e2[1]) + f64::from(e1[1])) as f32;
    let sx = (-f64::from(e2[0])) + (-f64::from(e1[0]));
    let n = normalized([(f64::from(sy) * 0.5) as f32, (sx * 0.5) as f32, 0.0]);
    let [x, y, z] = n.map(f64::from);
    if (z * z + y * y + x * x).sqrt() == 0.0 { e1 } else { n }
}

impl Placement<'_> {
    /// 0x55A3B0: the terrain of the tile under `p` when it is water, else of
    /// its first water neighbour.
    fn water_near(&self, world: &World, p: [f32; 2]) -> Option<usize> {
        let (x, y) = ((p[0] as i32) >> world.tile_bits, (p[1] as i32) >> world.tile_bits);
        let terrain = |cx: i32, cy: i32| -> Option<usize> {
            let t = world.tile_at(cx, cy)?;
            let tr = self.data.variations[world.var[t] as usize].terrain;
            (self.data.terrains[tr].kind == 2).then_some(tr)
        };
        if let Some(t) = terrain(x, y) {
            return Some(t);
        }
        super::super::grid::NEIGHBORS.iter().find_map(|d| terrain(x + d[0], y + d[1]))
    }

    /// 0x55B4E0: the doodad line along one polyline.
    fn doodad_line(&mut self, world: &mut World, rng: &mut Rng, pts: &[[f32; 3]]) {
        let n = pts.len();
        let mut normals = vec![[0.0f32; 3]; n];
        // 0x55B0F0.
        if n == 2 {
            let d = normalized(sub(&pts[1], &pts[0]));
            normals[0] = [d[1], -d[0], 0.0];
            normals[1] = normals[0];
        } else {
            for i in 0..n {
                let prev = pts[if i == 0 { n - 1 } else { i - 1 }];
                let next = pts[if i + 1 == n { 0 } else { i + 1 }];
                normals[i] = corner(&prev, &pts[i], &next);
            }
        }
        let diagonal = (f64::from(world.tile) * f64::from(SQRT2)) as f32;
        // 0x55B010: the normals point to the higher ground.
        let half = f64::from(diagonal) * 0.5;
        let mut score = 0i32;
        for (p, m) in pts.iter().zip(&normals) {
            let q = [(half * f64::from(m[0]) + f64::from(p[0])) as f32, (half * f64::from(m[1]) + f64::from(p[1])) as f32];
            if surface(world, self.data, q, 0x122) <= f64::from(p[2]) {
                score -= 1;
            } else {
                score += 1;
            }
        }
        if score > 0 {
            for m in &mut normals {
                m[0] = -m[0];
                m[1] = -m[1];
            }
        }
        // 0x55B590: the walk.
        let limit = f64::from(diagonal) * f64::from(SQRT2);
        let mut carry = 0.0f64;
        let mut prev = pts[n - 1];
        let mut prev_n = normals[n - 1];
        let (mut parallel, mut perpendicular) = (2.0f32, 2.0f32);
        let mut last: Option<usize> = None;
        let mut doodad: Option<usize> = None;
        for i in 0..n {
            let (cur, cur_n) = (pts[i], normals[i]);
            let dx = f64::from(cur[0]) - f64::from(prev[0]);
            let dy = (f64::from(cur[1]) - f64::from(prev[1])) as f32;
            let dz = f64::from(cur[2]) - f64::from(prev[2]);
            let len = ((dz * dz + f64::from(dy) * f64::from(dy)) + dx * dx).sqrt();
            let len32 = len as f32;
            let (ux, uy, uz) = if len > f64::from(EPS) {
                let inv = 1.0 / len;
                ((dx * inv) as f32, (f64::from(dy) * inv) as f32, (dz * inv) as f32)
            } else {
                (0.0, 0.0, 0.0)
            };
            if limit < f64::from(len32) {
                prev = cur;
                prev_n = cur_n;
                continue;
            }
            let mut t = 0.0f32;
            if carry < f64::from(len32) {
                let inv32 = (1.0 / f64::from(len32)) as f32;
                loop {
                    t = (f64::from(t) + carry) as f32;
                    let f = f64::from(inv32) * f64::from(t);
                    let g = 1.0 - f;
                    let a0 = (f64::from(prev_n[0]) * g) as f32;
                    let a1 = (f64::from(prev_n[1]) * g) as f32;
                    let gz = g * f64::from(prev_n[2]);
                    let b0 = f64::from(cur_n[0]) * f;
                    let b1 = (f64::from(cur_n[1]) * f) as f32;
                    let b2 = (f64::from(cur_n[2]) * f) as f32;
                    let mut nx = (b0 + f64::from(a0)) as f32;
                    let mut ny = (f64::from(b1) + f64::from(a1)) as f32;
                    let nz = gz + f64::from(b2);
                    let l = ((nz * nz + f64::from(ny) * f64::from(ny)) + f64::from(nx) * f64::from(nx)).sqrt();
                    if l > f64::from(EPS) {
                        let inv = 1.0 / l;
                        nx = (f64::from(nx) * inv) as f32;
                        ny = (inv * f64::from(ny)) as f32;
                    } else {
                        nx = 0.0;
                        ny = 0.0;
                    }
                    let px = (f64::from(ux) * f64::from(t) + f64::from(prev[0])) as f32;
                    let py = (f64::from(uy) * f64::from(t) + f64::from(prev[1])) as f32;
                    let uzt = (f64::from(uz) * f64::from(t)) as f32;
                    let pz = (f64::from(uzt) + f64::from(prev[2])) as f32;
                    if let Some(terrain) = self.water_near(world, [px, py]) {
                        if last != Some(terrain) {
                            last = Some(terrain);
                            let def = &self.data.terrains[terrain];
                            doodad = def.water_doodad.as_ref().and_then(|d| self.data.thing_by_id.get(&d.to_ascii_lowercase()).copied());
                            parallel = def.water_doodad_parallel;
                            perpendicular = def.water_doodad_perpendicular;
                        }
                    }
                    if let Some(thing) = doodad {
                        let x = (f64::from(nx) * f64::from(perpendicular) + f64::from(px)) as f32;
                        let y = (f64::from(ny) * f64::from(perpendicular) + f64::from(py)) as f32;
                        let request = super::Create { thing, pos: [x, y], z: pz, facing: super::DEFAULT_FACING, ..Default::default() };
                        if let Some(index) = self.create_gthing(world, rng, request) {
                            // 0x4EEBE0 (unless the thing's facing is locked).
                            if self.data.things[thing].flags & (1 << 17) == 0 {
                                self.objects[index].facing = [nx, ny];
                            }
                            // 0x4EE930(1): gone when its tile's terrain changes.
                            self.objects[index].auto = true;
                        }
                    }
                    carry = f64::from(parallel);
                    if !(f64::from(t) + f64::from(parallel) < f64::from(len32)) {
                        break;
                    }
                }
            }
            carry -= f64::from(len32) - f64::from(t);
            prev = cur;
            prev_n = cur_n;
        }
    }

    /// 0x555F20: the water doodads of the map. The things of an earlier try
    /// are gone with their world (0x593890), so nothing is left to clear.
    pub(in crate::mapgen) fn place_water_doodads(&mut self, world: &mut World, rng: &mut Rng) {
        let data = self.data;
        // 0x55C670.
        let tile_i = world.tile as i32;
        let cell = tile_i as f32;
        let k = (f64::from(world.tile) / (f64::from(1i32 << world.height_bits) * 0.5)) as i32;
        let m = (k + 1) as usize;
        let mut grid = vec![[0.0f32; 3]; m * m];
        let half = f64::from(world.tile) * 0.5;
        let mut ends: Vec<End> = Vec::new();
        for ty in 0..world.th as i32 {
            for tx in 0..world.tw as i32 {
                // 0x552AD0(cell, 1): a water tile a boat can cross, or beside one.
                let crossable = |x: i32, y: i32| -> bool {
                    world.tile_at(x, y).is_some_and(|t| {
                        let tr = &data.terrains[data.variations[world.var[t] as usize].terrain];
                        tr.kind == 2 && tr.water
                    })
                };
                if !(crossable(tx, ty) || super::super::grid::NEIGHBORS.iter().any(|d| crossable(tx + d[0], ty + d[1]))) {
                    continue;
                }
                let centre = [(f64::from(tx << world.tile_bits) + half) as f32, (f64::from(ty << world.tile_bits) + half) as f32];
                let level = surface(world, data, centre, 0x127) as f32;
                let x0 = (f64::from(tx) * f64::from(cell)) as f32;
                let y0 = (f64::from(ty) * f64::from(cell)) as f32;
                let kf = k as f32;
                for j in 0..m {
                    let y = (f64::from(j as i32) / f64::from(kf) * f64::from(cell) + f64::from(y0)) as f32;
                    for i in 0..m {
                        let x = (f64::from(i as i32) / f64::from(kf) * f64::from(cell) + f64::from(x0)) as f32;
                        grid[j * m + i] = [x, y, surface(world, data, [x, y], 0x122) as f32];
                    }
                }
                for row in 0..k as usize {
                    for col in 0..k as usize {
                        let a = grid[row * m + col];
                        let b = grid[row * m + col + 1];
                        let c = grid[(row + 1) * m + col];
                        let d = grid[(row + 1) * m + col + 1];
                        cut(&mut ends, &a, &b, &c, level);
                        cut(&mut ends, &d, &c, &b, level);
                    }
                }
            }
        }
        if ends.is_empty() {
            return;
        }
        weld(&mut ends, world.width);
        // 0x55A470.
        let mut i = 0;
        while i < ends.len() {
            if ends[i].done {
                i += 1;
                continue;
            }
            let mut start = i;
            if let Some(l) = ends[i].link {
                let mut e = ends[l].other;
                while e != i {
                    match ends[e].link {
                        Some(n) => e = ends[n].other,
                        None => break,
                    }
                }
                start = e;
            }
            let mut next = ends[start].other;
            let was_done = ends[start].done;
            ends[start].done = true;
            if ends[next].done {
                // The native walk would spin here on a chain closed by a used
                // end; the end that led to it is spent instead.
                if was_done {
                    ends[i].done = true;
                }
                continue;
            }
            let mut line: Vec<[f32; 3]> = Vec::new();
            loop {
                ends[next].done = true;
                if line.is_empty() {
                    line.push(ends[start].p);
                }
                line.push(ends[next].p);
                let Some(l) = ends[next].link else { break };
                if ends[l].done {
                    break;
                }
                ends[l].done = true;
                next = ends[l].other;
                if ends[next].done {
                    break;
                }
            }
            self.doodad_line(world, rng, &line);
        }
    }
}
