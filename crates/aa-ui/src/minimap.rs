//! The minimap widget (`0x5D71A0`, vtable 0xA4ED20) and its MiniMapVE
//! geometry (VE type 9, ctor `0x5E8C90`).
//!
//! * LoadData `0x5D7230`: `dynamic_position`, `minimap_size` (truncated,
//!   sizes the renderer's minimap target), the `MiniMapVE` child
//!   (`orientation` normalised by `0x409230`, `minimap_is_square`).
//! * GetQuad `0x5E9270`: for a square VE the side is `0x5E9030` (the
//!   distance to the VE edges along the orientation rotated by -pi/4 with
//!   the 4096-entry sin/cos tables, times 1.41421354f); `0x46DD10` builds
//!   P0 = c - A - E, P1 = c + A - E, P2 = c + A + E with A = h/2 * o and
//!   E = w/2 * (-oy, ox): the world square drawn as a diamond.
//! * Point `0x5D72C0` maps (u, v) = world / size into the quad; Inverse
//!   `0x5D73E0` projects back; pick `0x5E9400` only inside.
//! * Input `0x5D77A0`: a left press or double click jumps the camera to the
//!   point and starts a drag (`0x5D7830`, jumps only while inside
//!   [0,1)x[0,1)); a right press is swallowed. The camera is the world's:
//!   reported as [`crate::ui::UiEvent::Minimap`].
//!
//! The map image, units, fog and view lines are drawn by the renderer
//! (`KKC_RMinimap`) into this quad; the draw list carries the VE rect.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MiniMap {
    pub dynamic_position: bool,
    pub size: (i32, i32),
    /// Normalised orientation (`+0xC8`).
    pub orientation: (f32, f32),
    pub square: bool,
}

impl Default for MiniMap {
    fn default() -> Self {
        MiniMap { dynamic_position: false, size: (172, 172), orientation: (1.0, -1.0), square: true }
    }
}

/// `0x409230`.
pub fn normalize(x: f32, y: f32) -> (f32, f32) {
    let len = (x * x + y * y).sqrt();
    if len <= 1.0e-4 {
        (0.0, 0.0)
    } else {
        let k = 1.0 / len;
        (x * k, y * k)
    }
}

/// The 4096-entry sin/cos tables (`0x85DA10`: index = trunc(frac(x/2pi) *
/// 4096), no interpolation).
fn table(x: f64, cos: bool) -> f32 {
    let f = (x / std::f64::consts::TAU).rem_euclid(1.0);
    let i = (f * 4096.0) as usize % 4096;
    let a = i as f64 * std::f64::consts::TAU / 4096.0;
    (if cos { a.cos() } else { a.sin() }) as f32
}

/// Line through `p` along `d` meets segment a-b (`0x5E8910`): distance.
fn hit(p: (f32, f32), d: (f32, f32), a: (f32, f32), b: (f32, f32)) -> Option<f32> {
    let e = (b.0 - a.0, b.1 - a.1);
    let den = d.0 * e.1 - d.1 * e.0;
    if den == 0.0 {
        return None;
    }
    let w = (a.0 - p.0, a.1 - p.1);
    let t = (w.0 * e.1 - w.1 * e.0) / den;
    let s = (w.0 * d.1 - w.1 * d.0) / den;
    if !(0.0..=1.0).contains(&s) {
        return None;
    }
    Some((t * d.0).hypot(t * d.1))
}

/// The quad (P0, P1, P2) for the VE outer pixel rect (`0x5E9270`).
pub fn quad(m: &MiniMap, rect: crate::layout::Rect) -> [(f32, f32); 3] {
    let mut h = (rect.b - rect.t).abs();
    let mut w = (rect.r - rect.l).abs();
    let c = (w * 0.5 + rect.l, h * 0.5 + rect.t);
    let (ox, oy) = m.orientation;
    if m.square {
        // 0x5E9030: the orientation rotated by -pi/4
        let (s, co) = (table(-std::f64::consts::FRAC_PI_4, false), table(-std::f64::consts::FRAC_PI_4, true));
        let d = (ox * co - oy * s, s * ox + oy * co);
        let right = (-d.1, d.0);
        let top = ((rect.l, rect.t), (rect.r, rect.t));
        let side = ((rect.r, rect.t), (rect.r, rect.b));
        let mut best = f32::MAX;
        for dir in [d, right] {
            for (a, b) in [top, side] {
                if let Some(dist) = hit(c, dir, a, b) {
                    best = best.min(dist);
                }
            }
        }
        let sz = if best == f32::MAX { 0.0 } else { best * 1.414_213_5f32 };
        w = sz;
        h = sz;
    }
    let a = (h * ox * 0.5, h * oy * 0.5);
    let e = (-oy * w * 0.5, ox * w * 0.5);
    let p0 = (c.0 - a.0 - e.0, c.1 - a.1 - e.1);
    let p1 = (c.0 + a.0 - e.0, c.1 + a.1 - e.1);
    let p2 = (c.0 + a.0 + e.0, c.1 + a.1 + e.1);
    [p0, p1, p2]
}

/// Point `0x5D72C0`: (u, v) = (world x / W, world y / H) to pixels.
pub fn point(q: &[(f32, f32); 3], u: f32, v: f32) -> (f32, f32) {
    let [p0, p1, p2] = *q;
    let x = ((p2.0 - p1.0) * u + (p1.0 - p0.0) * v) + p0.0;
    let y = ((p2.1 - p1.1) * u + (p1.1 - p0.1) * v) + p0.1;
    (x, y)
}

/// Inverse `0x5D73E0` (projections `0x5D7330`): pixels to (u, v).
pub fn inverse(q: &[(f32, f32); 3], p: (f32, f32)) -> (f32, f32) {
    let [p0, p1, p2] = *q;
    let proj = |a: (f32, f32), b: (f32, f32)| {
        let d = (b.0 - a.0, b.1 - a.1);
        let len = (d.0 * d.0 + d.1 * d.1).sqrt();
        ((p.0 - a.0) * d.0 + (p.1 - a.1) * d.1) / (len * len)
    };
    (proj(p1, p2), proj(p0, p1))
}

/// Pick test `0x5E8E10`: both projections in [0, 1].
pub fn inside(q: &[(f32, f32); 3], p: (f32, f32)) -> bool {
    let (u, v) = inverse(q, p);
    (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_minimap_is_a_diamond_on_the_edge_midpoints() {
        let o = normalize(1.0, -1.0);
        let m = MiniMap { orientation: o, ..Default::default() };
        let r = crate::layout::Rect::new(100.0, 50.0, 300.0, 250.0);
        let q = quad(&m, r);
        let near = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01;
        assert!(near(q[0], (100.0, 150.0)), "{q:?}");
        assert!(near(q[1], (200.0, 50.0)), "{q:?}");
        assert!(near(q[2], (300.0, 150.0)), "{q:?}");
        // world (0,0) left, (0,H) top, (W,H) right, (W,0) bottom
        assert!(near(point(&q, 1.0, 0.0), (200.0, 250.0)));
        let (u, v) = inverse(&q, (200.0, 150.0));
        assert!((u - 0.5).abs() < 1e-4 && (v - 0.5).abs() < 1e-4);
        assert!(inside(&q, (200.0, 150.0)));
        assert!(!inside(&q, (105.0, 55.0)));
    }
}
