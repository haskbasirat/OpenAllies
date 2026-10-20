//! Projected textures (VFXProjectedTexture: building ground planes, scorch
//! marks) as geometry lying exactly on the drawn terrain: every terrain cell
//! triangle under the decal square (cells split along the same diagonal as
//! terrain.rs draws them, heights at the drawn corners) is clipped to the
//! square, so the decal follows every slope without floating or sinking.
use bevy::math::{Vec2, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Vert {
    pub pos: Vec3,
    pub uv: [f32; 2],
    pub normal: Vec3,
}

/// Lift above the terrain surface (render metres).
const LIFT: f32 = 0.015;

/// Triangles (render space) of a `size` x `size` square centred at
/// `center`, rotated by `angle` about the vertical, on the terrain whose
/// render height is `height(x, z)` with `cell`-metre cells.
pub fn build(center: Vec3, size: f32, angle: f32, cell: f32, height: &dyn Fn(f32, f32) -> f32) -> Vec<Vert> {
    let half = size * 0.5;
    let (s, c) = angle.sin_cos();
    // Square axes in render X/Z.
    let ax = Vec2::new(c, s);
    let az = Vec2::new(-s, c);
    let center2 = Vec2::new(center.x, center.z);
    let reach = half * std::f32::consts::SQRT_2;
    let cell = cell.max(0.1);
    // Native grid: x = render x, y = -render z.
    let (x0, x1) = (((center.x - reach) / cell).floor() as i32, ((center.x + reach) / cell).floor() as i32);
    let (y0, y1) = (((-center.z - reach) / cell).floor() as i32, ((-center.z + reach) / cell).floor() as i32);
    let mut out = Vec::new();
    let local = |p: Vec2| -> Vec2 {
        let d = p - center2;
        Vec2::new(d.dot(ax), d.dot(az))
    };
    for gy in y0.max(0)..=y1 {
        for gx in x0.max(0)..=x1 {
            // Cell corners 0 (x0,y0), 1 (x1,y0), 2 (x0,y1), 3 (x1,y1) in
            // native grid order; triangles (0,1,2) and (1,3,2).
            let corner = |i: i32| -> Vec3 {
                let nx = (gx + (i & 1)) as f32 * cell;
                let ny = (gy + (i >> 1)) as f32 * cell;
                Vec3::new(nx, height(nx, -ny), -ny)
            };
            let q = [corner(0), corner(1), corner(2), corner(3)];
            for tri in [[q[0], q[1], q[2]], [q[1], q[3], q[2]]] {
                let mut normal = (tri[1] - tri[0]).cross(tri[2] - tri[0]).normalize_or_zero();
                if normal.y < 0.0 {
                    normal = -normal;
                }
                // Clip in the square's local frame (Sutherland-Hodgman).
                let mut poly: Vec<(Vec2, f32)> = tri.iter().map(|p| (local(Vec2::new(p.x, p.z)), p.y)).collect();
                for (axis, sign) in [(0usize, 1.0f32), (0, -1.0), (1, 1.0), (1, -1.0)] {
                    if poly.is_empty() {
                        break;
                    }
                    let inside = |p: &(Vec2, f32)| sign * p.0[axis] <= half;
                    let mut next = Vec::with_capacity(poly.len() + 2);
                    for i in 0..poly.len() {
                        let a = poly[i];
                        let b = poly[(i + 1) % poly.len()];
                        let (ia, ib) = (inside(&a), inside(&b));
                        if ia {
                            next.push(a);
                        }
                        if ia != ib {
                            let da = sign * a.0[axis] - half;
                            let db = sign * b.0[axis] - half;
                            let t = da / (da - db);
                            next.push((a.0.lerp(b.0, t), a.1 + (b.1 - a.1) * t));
                        }
                    }
                    poly = next;
                }
                if poly.len() < 3 {
                    continue;
                }
                let vert = |p: &(Vec2, f32)| -> Vert {
                    let world = center2 + ax * p.0.x + az * p.0.y;
                    Vert {
                        pos: Vec3::new(world.x, p.1 + LIFT, world.y),
                        uv: [p.0.x / size + 0.5, p.0.y / size + 0.5],
                        normal,
                    }
                };
                let first = vert(&poly[0]);
                for i in 1..poly.len() - 1 {
                    out.push(first);
                    out.push(vert(&poly[i]));
                    out.push(vert(&poly[i + 1]));
                }
            }
        }
    }
    out
}
