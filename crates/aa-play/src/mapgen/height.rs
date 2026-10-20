//! StageAddNoise (0x5906C0 -> AddTerrainNoise 0x551360 per RMC TerrainNoise on
//! the base heightmap) and the native render height (query 0x122: base, plus
//! the overlay where the cell or a neighbour is mountain/water, plus the
//! mountain cell noise), as aa-assets `native_height_at_grid` computes it.
use super::data::{NoiseDef, RmgData};
use super::grid::{Slope, World};
use super::rng::Rng;

/// Vertices next to water carry a water-area id; height brushes with the
/// protect flag may not change them (0x550660 / 0x559190 "protected area").
fn protected(world: &World, data: &RmgData) -> Vec<bool> {
    let mut out = vec![false; world.hw * world.hh];
    for t in 0..world.tw * world.th {
        if data.terrains[data.variations[world.var[t] as usize].terrain].kind != 2 {
            continue;
        }
        let [x, y] = world.tile_xy(t);
        let scale = (world.tile / world.spacing) as i32;
        for vy in y * scale..=(y + 1) * scale {
            for vx in x * scale..=(x + 1) * scale {
                if let Some(v) = world.vertex(vx, vy) {
                    out[v] = true;
                }
            }
        }
    }
    out
}

/// AddTerrainNoise (0x551360).
pub(super) fn add_noise(world: &mut World, data: &RmgData, noise: &NoiseDef, rng: &mut Rng) {
    let inv = 1.0 / noise.reference_size;
    let area = world.height * world.width * inv * inv;
    let n = (rng.unit() * (noise.num[1] - noise.num[0]) as f32 + noise.num[0] as f32) * noise.area_factor.powf(area - 1.0) * area;
    let count = n as i32;
    let guard = protected(world, data);
    for _ in 0..count.max(0) {
        let x = rng.unit() * world.width;
        let y = rng.unit() * world.height;
        let radius = rng.unit() * (noise.size[1] - noise.size[0]) + noise.size[0];
        let sign = if rng.unit() < noise.up_frequency { 1.0 } else { -1.0 };
        let lo = sign * noise.magnitude[0];
        let magnitude = (sign * noise.magnitude[1] - lo) * rng.unit() + lo;
        let slope = (magnitude / (radius * 0.5)).abs();
        // Round brush of radius 1 m, relative mode, strength 1 (0x550B00).
        let s = world.spacing;
        let (x0, x1) = (((x - 1.0) / s).floor() as i32, ((x + 1.0) / s).ceil() as i32);
        let (y0, y1) = (((y - 1.0) / s).floor() as i32, ((y + 1.0) / s).ceil() as i32);
        let mut brush = Slope::new(slope, Some(([x, y], noise.size[1])));
        let mut blocked = false;
        for vy in y0.max(0)..=y1.min(world.hh as i32 - 1) {
            for vx in x0.max(0)..=x1.min(world.hw as i32 - 1) {
                let Some(v) = world.vertex(vx, vy) else { continue };
                let p = world.vertex_xy(v);
                let d = ((x - p[0]).powi(2) + (y - p[1]).powi(2)).sqrt();
                let strength = 1.0 - d * d;
                if strength <= 0.0 {
                    continue;
                }
                let old = world.base[v];
                let delta = old + magnitude - old;
                if delta.abs() <= 1e-4 {
                    continue;
                }
                if guard[v] {
                    blocked = true;
                    break;
                }
                brush.seed(v, delta * strength + old);
            }
        }
        if blocked || brush.is_empty() {
            continue;
        }
        brush.finish(world, data, false);
        if brush_touches(&brush, &guard) {
            continue;
        }
        brush.commit(world, false);
    }
}

fn brush_touches(brush: &Slope, guard: &[bool]) -> bool {
    brush.rows().iter().any(|&(v, _, _)| guard[v])
}

/// Flatten the base heightmap under a footprint to its mean (flatten_terrain,
/// and the RMG "make terrain conforming" pass for required objects).
pub(super) fn flatten(world: &mut World, center: [f32; 2], half: [f32; 2]) {
    let s = world.spacing;
    let (x0, x1) = (((center[0] - half[0]) / s).floor() as i32, ((center[0] + half[0]) / s).ceil() as i32);
    let (y0, y1) = (((center[1] - half[1]) / s).floor() as i32, ((center[1] + half[1]) / s).ceil() as i32);
    let mut sum = 0.0;
    let mut cells = Vec::new();
    for vy in y0..=y1 {
        for vx in x0..=x1 {
            if let Some(v) = world.vertex(vx, vy) {
                sum += world.base[v];
                cells.push(v);
            }
        }
    }
    if cells.is_empty() {
        return;
    }
    let mean = sum / cells.len() as f32;
    for v in cells {
        world.base[v] = mean;
    }
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
