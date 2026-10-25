//! Decal geometry: the UI sprite grid and the decal view-element pieces.
//!
//! Sprite pipeline (AA.exe, renderer layer 0x73E000..0x745000):
//! * `0x740900`: every rect edge `v` with `v == trunc(v)` is nudged to
//!   `v - 0.1`, then divided by the screen size; the projection (manager
//!   +0x168, set at 0x7445F8) has no half-pixel term, so D3D9 samples pixel
//!   `i` at coordinate `i`. A renderer that samples at `i + 0.5` reproduces
//!   coverage and texel choice with `x' = adj(x) + 0.5` ([`raster`]).
//! * Samplers (`0x742200`): clamp, point min/mag filtering, no mips.
//! * Tiling (`0x73F750` counts, `0x73FA20` cells, `0x73F1B0` positions,
//!   `0x73F290` UVs): no hardware wrap; per axis in mode 1 (tiled) the span
//!   in design pixels `span = trunc(|R_n - L_n| * IPW + 0.5)` is divided by
//!   the texture size `tex = max(32, trunc(gpu_w * |u1 - u0|))` (unsigned
//!   division `q`), the rect is cut into `q + 1` uniform cells, cells
//!   `0..q-1` show `u0..u1` and the last shows
//!   `u0 .. u0 + |u1-u0| * (IPW * cell_w_n) / tex`. Mode 3: one quad.
//! * UV rect field order passed by decal code: `(u_left, v_bottom,
//!   u_right, v_top)` with v = 0 at the image top; the sprite stores
//!   `(u0, v0, u1, v1) = (in0, in3, in2, in1)` (`0x73FD90`, `0x73EF70`).
//! * Images are padded (not scaled) to a power of two (`0x5A6A10`,
//!   `0x87C280`) at the top-left with transparent texels; UVs here are
//!   normalised to the original image, so `gpu_w * |u1-u0|` is the image
//!   width for a full-image UV.
//!
//! Decal pieces:
//! * DecalVE (`0x5E7830`, fit `0x5E7000` when `stretch = false`);
//! * DecalVETiled 9-slice (`0x5E7480`, piece rects `0x5E6AB0`): corner,
//!   border and centre sizes are raw texture pixels (not scaled);
//! * DecalVETiledDirectional (`0x5E7EA0`, rects `0x5E7790` ->
//!   `0x5E6D40`/`0x5E6DC0`): the end half-size is always the ends texture
//!   width; vertical end caps are swapped (texture top half at the bottom).

use crate::draw::{Address, Blend, Quad};
use crate::layout::{Precision, Rect, Reference, Screen};

/// `0x740900` nudge.
pub fn adj(v: f32) -> f32 {
    if v == v.trunc() { v - 0.1 } else { v }
}

/// Map a native screen coordinate to the equivalent coordinate for a
/// renderer that samples pixel centres at `i + 0.5`.
pub fn raster(v: f32) -> f32 {
    adj(v) + 0.5
}

/// Inputs shared by every piece of one decal view element.
#[derive(Clone, Debug)]
pub struct PieceStyle {
    pub color: [f32; 4],
    pub blend: Blend,
}

/// Power of two at or above `n` (`0x43A920`).
pub fn pot(n: u32) -> u32 {
    if n == 0 { 0 } else { n.next_power_of_two() }
}

/// Emit the quads of one sprite piece. `uv_in` is in the native decal order
/// `(u_left, v_bottom, u_right, v_top)` over the image; `tex_size` the
/// original image size. The image is padded to a power of two
/// (`0x5A6A10`, sub-rect (0, 0, w/pot, h/pot)), the UVs mapped into it
/// (`0x5A6030`), and the vertex grid built by the native sprite code
/// ([`sprite_grid`], NATIVE-PROVEN); each cell becomes one quad with
/// image-relative UVs for the renderer.
#[allow(clippy::too_many_arguments)]
pub fn piece_quads(
    out: &mut Vec<Quad>,
    texture: Option<&str>,
    tex_size: Option<(u32, u32)>,
    rect: Rect,
    uv_in: [f32; 4],
    tile: (bool, bool),
    style: &PieceStyle,
    screen: Screen,
    reference: Reference,
    p: Precision,
) {
    let (iw, ih) = tex_size.unwrap_or((32, 32));
    let (iw, ih) = if iw == 0 || ih == 0 { (16, 16) } else { (iw, ih) };
    let (pw, ph) = (pot(iw), pot(ih));
    // 0x5A6A10: +0x4C = w / pot, +0x50 = h / pot (f32)
    let su = p.div(iw as f64, pw as f64) as f32 as f64;
    let sv = p.div(ih as f64, ph as f64) as f32 as f64;
    // 0x5A6030 over the sub-rect (0, 0, su, sv)
    let gpu = [
        p.mul(su, uv_in[0] as f64) as f32,
        p.mul(sv, uv_in[1] as f64) as f32,
        p.mul(su, uv_in[2] as f64) as f32,
        p.mul(sv, uv_in[3] as f64) as f32,
    ];
    let modes = [if tile.0 { 1 } else { 3 }, if tile.1 { 1 } else { 3 }];
    let input = SpriteInput {
        rect: [rect.l, rect.t, rect.r, rect.b],
        uv: if texture.is_some() { gpu } else { uv_in },
        tex: texture.map(|_| (pw, ph)),
        modes,
        layer: 0,
        color: style.color,
        color2: [style.color[0], style.color[1], style.color[2]],
        screen: (screen.width as f32, screen.height as f32),
        ip: (reference.width, reference.height),
        far: 1.0,
        zstep: 0.0,
    };
    let ((nx, ny), verts) = sprite_grid(&input, p);
    // cells: mode 1 pairs (2k, 2k+1), otherwise (k, k+1)
    let cells = |n: u32, mode: u32| -> Vec<(u32, u32)> {
        if mode == 1 { (0..n / 2).map(|k| (2 * k, 2 * k + 1)).collect() } else { (0..n.saturating_sub(1)).map(|k| (k, k + 1)).collect() }
    };
    let back = |u: f32, s: f64| if texture.is_some() && s != 0.0 { (u as f64 / s) as f32 } else { u };
    let (sw, sh) = (screen.width as f32, screen.height as f32);
    for (r0, r1) in cells(ny, modes[1]) {
        for (c0, c1) in cells(nx, modes[0]) {
            let a = verts[(r0 * nx + c0) as usize];
            let b = verts[(r1 * nx + c1) as usize];
            out.push(Quad {
                texture: texture.map(str::to_string),
                // native normalised coordinates to pixels, moved to the
                // i+0.5 convention
                rect: Rect::new(a.pos[0] * sw + 0.5, a.pos[1] * sh + 0.5, b.pos[0] * sw + 0.5, b.pos[1] * sh + 0.5),
                uv: [back(a.uv[0], su), back(a.uv[1], sv), back(b.uv[0], su), back(b.uv[1], sv)],
                color: style.color,
                blend: style.blend,
                address: [Address::Clamp, Address::Clamp],
                clip: None,
                effect: 0,
        tint: [0.0; 3],
            });
        }
    }
}

/// `0x5E7000`: decal UV rect (native field order `u_left, v_bottom,
/// u_right, v_top`). `stretch = true` (or tiling on both axes) gives the
/// full image; otherwise the image is fitted to the rect's aspect: a wider
/// rect (when not tiled in x) pillar-boxes with clamped edge texels, else
/// (when not tiled in y) the image letter-boxes, or crops when the rect is
/// wider and tiled in x. `at = tex_w / tex_h` stays on the x87 stack.
pub fn decal_fit(rect: Rect, tex: (u32, u32), tiled_x: bool, tiled_y: bool, stretch: bool, p: Precision) -> [f32; 4] {
    if stretch {
        return [0.0, 1.0, 1.0, 0.0];
    }
    let w = p.sub(rect.r as f64, rect.l as f64).abs();
    let h = p.sub(rect.b as f64, rect.t as f64).abs();
    let ar = p.div(w, h) as f32 as f64;
    let at = p.div(tex.0 as f32 as f64, tex.1 as f64);
    let pillar = !tiled_x && !(ar < at || ar == at || ar.is_nan() || at.is_nan());
    if pillar {
        let k = p.mul(p.sub(1.0, p.div(at, ar)), 0.5);
        [(-k) as f32, 1.0, p.add(k, 1.0) as f32, 0.0]
    } else if tiled_y {
        [0.0, 1.0, 1.0, 0.0]
    } else {
        let k = p.mul(p.sub(1.0, p.div(ar, at)), 0.5);
        [0.0, p.add(1.0, k) as f32, 1.0, (-k) as f32]
    }
}

/// `0x5E6AB0`: rects of the nine pieces of a tiled decal (order: top,
/// bottom, left, right, TL, TR, BL, BR, centre). `sizes` are the image
/// sizes of corners, left, right, top, bottom, centre.
pub fn nine_slice_rects(r: Rect, sizes: [Option<(u32, u32)>; 6]) -> [Rect; 9] {
    let (cw, ch) = match sizes[0] {
        Some((w, h)) => (w as f32 * 0.5, h as f32 * 0.5),
        None => (0.0, 0.0),
    };
    let w_of = |i: usize| sizes[i].map(|s| s.0 as f32);
    let h_of = |i: usize| sizes[i].map(|s| s.1 as f32);
    let top = Rect::new(r.l + cw, r.t, r.r - cw, h_of(3).map_or(r.b, |h| r.t + h));
    let bottom = Rect::new(r.l + cw, h_of(4).map_or(r.t, |h| r.b - h), r.r - cw, r.b);
    let left = Rect::new(r.l, r.t + ch, w_of(1).map_or(r.r, |w| r.l + w), r.b - ch);
    let right = Rect::new(w_of(2).map_or(r.l, |w| r.r - w), r.t + ch, r.r, r.b - ch);
    [
        top,
        bottom,
        left,
        right,
        Rect::new(r.l, r.t, r.l + cw, r.t + ch),
        Rect::new(r.r - cw, r.t, r.r, r.t + ch),
        Rect::new(r.l, r.b - ch, r.l + cw, r.b),
        Rect::new(r.r - cw, r.b - ch, r.r, r.b),
        Rect::new(r.l + cw, r.t + ch, r.r - cw, r.b - ch),
    ]
}

/// `0x5E7480` piece table: (texture index, uv_in, tile_x, tile_y).
pub const NINE_SLICE_PIECES: [(usize, [f32; 4], bool, bool); 9] = [
    (3, [0.0, 1.0, 1.0, 0.0], true, false),
    (4, [0.0, 1.0, 1.0, 0.0], true, false),
    (1, [0.0, 1.0, 1.0, 0.0], false, true),
    (2, [0.0, 1.0, 1.0, 0.0], false, true),
    (0, [0.0, 0.5, 0.5, 0.0], false, false),
    (0, [0.5, 0.5, 1.0, 0.0], false, false),
    (0, [0.0, 1.0, 0.5, 0.5], false, false),
    (0, [0.5, 1.0, 1.0, 0.5], false, false),
    (5, [0.0, 1.0, 1.0, 0.0], true, true),
];

/// `0x5E7790` (`0x5E6D40` horizontal / `0x5E6DC0` vertical) piece rects and
/// `0x5E7EA0` UVs: (texture index, rect, uv_in, tile_x, tile_y).
pub fn directional_pieces(r: Rect, ends: Option<(u32, u32)>, horizontal: bool) -> [(usize, Rect, [f32; 4], bool, bool); 3] {
    let h = ends.map_or(0.0, |(w, _)| w as f32 * 0.5);
    if horizontal {
        [
            (0, Rect::new(r.l, r.t, r.l + h, r.b), [0.0, 1.0, 0.5, 0.0], false, false),
            (0, Rect::new(r.r - h, r.t, r.r, r.b), [0.5, 1.0, 1.0, 0.0], false, false),
            (1, Rect::new(r.l + h, r.t, r.r - h, r.b), [0.0, 1.0, 1.0, 0.0], true, false),
        ]
    } else {
        [
            (0, Rect::new(r.l, r.b - h, r.r, r.b), [0.0, 0.5, 1.0, 0.0], false, false),
            (0, Rect::new(r.l, r.t, r.r, r.t + h), [0.0, 1.0, 1.0, 0.5], false, false),
            (1, Rect::new(r.l, r.t + h, r.r, r.b - h), [0.0, 1.0, 1.0, 0.0], false, true),
        ]
    }
}

/// Blend selection (`0x73F660`/`0x73FEF0`, `0x743750`): only
/// `alpha_blending` changes the result; otherwise texels with alpha 0 are
/// discarded (ALPHATEST GREATER 0) and the rest drawn opaque.
pub fn blend_of(alpha_blending: bool) -> Blend {
    if alpha_blending { Blend::Alpha } else { Blend::Opaque }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tiled_piece_cells() {
        // a 300 px span over a 100 px (padded 128) image at 1024 wide:
        // cells of max(32, trunc(128 * 100/128)) = 100 design px, the
        // last one partial
        let mut q = Vec::new();
        let style = PieceStyle { color: [1.0; 4], blend: Blend::Opaque };
        let screen = Screen { width: 1024, height: 768 };
        let reference = Reference { width: 1024.0, height: 768.0 };
        piece_quads(&mut q, Some("t"), Some((100, 100)), Rect::new(100.0, 50.0, 400.0, 90.0), [0.0, 1.0, 1.0, 0.0], (true, false), &style, screen, reference, Precision::Pc24);
        assert_eq!(q.len(), 4);
        assert!((q[0].rect.r - q[0].rect.l - 75.0).abs() < 0.2, "{:?}", q[0].rect);
        assert!((q[2].uv[2] - 1.0).abs() < 1e-6);
        assert!((q[3].uv[2] - 0.75).abs() < 1e-3, "{:?}", q[3].uv);
    }
}

/// One vertex of the native sprite grid (`0x73FF60`, 32 bytes: x, y, z
/// normalised, colour 0xAARRGGBB, colour 0xFFRRGGBB, u, v).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteVertex {
    pub pos: [f32; 3],
    pub c0: u32,
    pub c1: u32,
    pub uv: [f32; 2],
}

/// The sprite fields the vertex build reads (sprite vtable 0xA8ADC8).
#[derive(Clone, Debug, PartialEq)]
pub struct SpriteInput {
    /// Pixel rect L, T, R, B (SetRect `0x73EED0`).
    pub rect: [f32; 4],
    /// UV in the decal order (in0, in1, in2, in3) (SetUV `0x73EF70`): GPU
    /// texture coordinates.
    pub uv: [f32; 4],
    /// GPU texture size (the image padded to a power of two), `None` = no
    /// texture.
    pub tex: Option<(u32, u32)>,
    /// Address modes per axis (+0x50 / +0x54): 0 scaled, 1 tiled with
    /// duplicated seam vertices, 2 tiled shared, 3 stretched.
    pub modes: [u32; 2],
    /// +0x58.
    pub layer: u32,
    /// +0x20..+0x2C (r, g, b, a) and +0x30..+0x38.
    pub color: [f32; 4],
    pub color2: [f32; 3],
    /// Sprite manager screen size (+0x1B0 / +0x1B4).
    pub screen: (f32, f32),
    /// AVars InterfacePositioningWidth / Height.
    pub ip: (f32, f32),
    /// AVar +0x20 (far z) and manager +0x1A8 (z step per layer).
    pub far: f32,
    pub zstep: f32,
}

/// The native sprite grid: vertex counts per axis and the vertices in row
/// order (`0x73F750` / `0x73FA20` / `0x73FF60` with `0x73F1B0` and
/// `0x73F290`; rect nudge `0x740900`). NATIVE-PROVEN by
/// research/phase8-ui/native-sprite.json.
pub fn sprite_grid(s: &SpriteInput, p: Precision) -> ((u32, u32), Vec<SpriteVertex>) {
    let f = |x: f64| x as f32 as f64;
    let ftol = |x: f64| x.trunc() as i64;
    // 0x740900: integral coordinates nudged by -0.1f, then divided
    let nudge = |v: f32, size: f32| -> f64 {
        let mut v = v as f64;
        if ftol(v) as f64 == v {
            v = f(p.sub(v, 0.1f32 as f64));
        }
        f(p.div(v, size as f64))
    };
    let x0 = nudge(s.rect[0], s.screen.0);
    let y0 = nudge(s.rect[1], s.screen.1);
    let x1 = nudge(s.rect[2], s.screen.0);
    let y1 = nudge(s.rect[3], s.screen.1);
    // 0x73EF70: +0x40 = in0, +0x44 = in3, +0x48 = in2, +0x4C = in1
    let (u0, v0, u1, v1) = (s.uv[0] as f64, s.uv[3] as f64, s.uv[2] as f64, s.uv[1] as f64);
    let ipw = s.ip.0 as f64;
    let iph = s.ip.1 as f64;
    let du = p.sub(u1, u0).abs();
    let dv = p.sub(v1, v0).abs();
    // 0x73F000 / 0x73F050: ftol(gpu size * |span|), callers clamp >= 32
    let tex_x = |w: u32| (ftol(p.mul(w as f64, du)) as u32).max(32);
    let tex_y = |h: u32| (ftol(p.mul(h as f64, dv)) as u32).max(32);
    // 0x73F180 / 0x73F150: span in design pixels
    let span_x = ftol(p.add(p.mul(p.sub(x1, x0).abs(), ipw), 0.5)) as u32;
    let span_y = ftol(p.add(p.mul(p.sub(y1, y0).abs(), iph), 0.5)) as u32;
    // 0x73F750 vertex counts, 0x73FA20 cell counts
    let (mut nx, mut ny, mut cx, mut cy) = (2u32, 2u32, 1u32, 1u32);
    if let Some((tw, th)) = s.tex {
        match s.modes[0] {
            1 => {
                let q = span_x / tex_x(tw);
                nx = q * 2 + 2;
                cx = q + 1;
            }
            2 => {
                let q = span_x / tex_x(tw);
                nx = q + 1;
                cx = q + 1;
            }
            _ => {}
        }
        match s.modes[1] {
            1 => {
                let q = span_y / tex_y(th);
                ny = q * 2 + 2;
                cy = q + 1;
            }
            2 => {
                let q = span_y / tex_y(th);
                ny = q + 1;
                cy = q + 1;
            }
            _ => {}
        }
    }
    // 0x73FF60: packed colours, w / h, then the grid
    let b = |c: f32| ftol(p.mul(c as f64, 255.0)) as u32;
    let c0 = (((b(s.color[3]) << 8 | b(s.color[0])) << 8 | b(s.color[1])) << 8) | b(s.color[2]);
    let c1 = (((b(s.color2[0]) | 0xFFFF_FF00) << 8 | b(s.color2[1])) << 8) | b(s.color2[2]);
    let w = f(p.sub(x1, x0).abs());
    let h = f(p.sub(y1, y0).abs());
    let z = f(p.sub(s.far as f64, p.mul(s.layer as f64, s.zstep as f64))) as f32;
    let frac = |mode: u32, k: u32, n: u32| -> f64 {
        if mode == 0 || mode == 2 || mode == 3 {
            p.div(k as f64, n as f64)
        } else {
            let a = (k * 2 + 2) & !3;
            let b = (n * 2 + 2) & !3;
            p.div(a as i32 as f64, b as i32 as f64)
        }
    };
    let mut out: Vec<SpriteVertex> = Vec::with_capacity((nx * ny) as usize);
    for row in 0..ny {
        for col in 0..nx {
            // 0x73F1B0 (sprite +8 / +0xC offsets are 0 for UI sprites)
            let x = f(p.add(p.mul(frac(s.modes[0], col, nx - 1), w), x0));
            let y = f(p.add(p.mul(frac(s.modes[1], row, ny - 1), h), y0));
            let x = f(p.add(0.0, x));
            let y = f(p.add(0.0, y));
            let dx = if col > 0 { f(p.sub(x, out[(row * nx + col - 1) as usize].pos[0] as f64)) } else { 0.0 };
            let dy = if row > 0 { f(p.sub(y, out[((row - 1) * nx + col) as usize].pos[1] as f64)) } else { 0.0 };
            // 0x73F290
            let u = if col == 0 {
                u0
            } else {
                match s.tex {
                    None => u1,
                    Some((tw, _)) => match s.modes[0] {
                        0 => {
                            let k = f(p.div(f(span_x as f64), tw as f64));
                            f(p.add(p.mul(du, k), u0))
                        }
                        1 | 2 => {
                            let last = if s.modes[0] == 1 { col + 1 == cx * 2 } else { col == cx };
                            if !last {
                                if col % 2 == 0 { u0 } else { u1 }
                            } else {
                                let t = tex_x(tw) as f64;
                                let k = p.mul(p.div(p.mul(ipw, dx), t), du);
                                if col % 2 == 0 { f(p.sub(u1, k)) } else { f(p.add(u0, k)) }
                            }
                        }
                        _ => u1,
                    },
                }
            };
            let v = if row == 0 {
                v0
            } else {
                match s.tex {
                    // native quirk: |v1 - v0| without a texture
                    None => f(dv),
                    Some((_, th)) => match s.modes[1] {
                        0 => {
                            let k = f(p.div(f(span_y as f64), th as f64));
                            f(p.add(p.mul(dv, k), v0))
                        }
                        1 | 2 => {
                            let last = if s.modes[1] == 1 { row + 1 == cy * 2 } else { row == cy };
                            if !last {
                                if row % 2 == 0 { v0 } else { v1 }
                            } else {
                                let t = tex_y(th) as f64;
                                let k = p.mul(p.div(p.mul(iph, dy), t), dv);
                                if row % 2 == 0 { f(p.sub(v1, k)) } else { f(p.add(v0, k)) }
                            }
                        }
                        _ => v1,
                    },
                }
            };
            out.push(SpriteVertex { pos: [x as f32, y as f32, z], c0, c1, uv: [u as f32, v as f32] });
        }
    }
    ((nx, ny), out)
}
