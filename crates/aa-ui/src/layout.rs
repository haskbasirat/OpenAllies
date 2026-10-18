//! View-element bounds: the L/T/R/B unit grammar and the layout arithmetic of
//! the original UI (`TGC_ViewElement`, `.\Source\Ui\System\ViewElement.cpp`).
//!
//! Ported from AA.exe (image base 0x400000):
//! * `0x5E3150` bounds parse: four strings -> four floats + unit flags;
//! * `0x5E3370` bounds resolve against a parent rectangle;
//! * `0x5E2460` degenerate clamp and "inside parent" test;
//! * `0x5E1580` clip to parent, recording the clipped amount per side;
//! * `0x5E34B0` whole view-element layout (outer + client bounds);
//! * `0x5C1DB0` / `0x5C1DD0` UI scale = screen size / `InterfacePositioning
//!   Width|Height` (AVar slots 0x22/0x23, registered at 0x402C5A with the
//!   defaults 1024.0 / 768.0, data `Data/AVars.tgi`).
//!
//! The x87 code runs either at 24-bit precision (Direct3D 9 sets this on the
//! device thread unless `D3DCREATE_FPU_PRESERVE`, which the original never
//! requests; see research/phase4-orders/fpu-startup-static.json) or at the CRT
//! default of 53 bits. Both are reproduced bit-exactly ([`Precision`]) and
//! proven against the original instructions by the Unicorn fixtures in
//! `research/phase8-ui/native-layout.json`.

use serde::{Deserialize, Serialize};

/// x87 precision-control setting the original arithmetic ran under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Precision {
    /// PC = 24 bits (FPCW 0x007F): Direct3D 9 device thread default. Every
    /// x87 operation rounds to binary32.
    #[default]
    Pc24,
    /// PC = 53 bits (FPCW 0x027F): CRT startup default. Operations round to
    /// binary64; values are rounded to binary32 only where stored.
    Pc53,
}

impl Precision {
    #[inline]
    pub(crate) fn r(self, x: f64) -> f64 {
        match self {
            Precision::Pc24 => x as f32 as f64,
            Precision::Pc53 => x,
        }
    }
    #[inline]
    pub(crate) fn add(self, a: f64, b: f64) -> f64 {
        self.r(a + b)
    }
    #[inline]
    pub(crate) fn sub(self, a: f64, b: f64) -> f64 {
        self.r(a - b)
    }
    #[inline]
    pub(crate) fn mul(self, a: f64, b: f64) -> f64 {
        self.r(a * b)
    }
    #[inline]
    pub(crate) fn div(self, a: f64, b: f64) -> f64 {
        self.r(a / b)
    }
}

/// Unit flags produced by `0x5E3150` (stored at bounds+0x30).
pub mod unit {
    /// L is measured from the parent's right edge (`10r`).
    pub const L_FROM_RIGHT: u32 = 0x1;
    /// T is measured from the parent's bottom edge (`10b`).
    pub const T_FROM_BOTTOM: u32 = 0x2;
    /// R is measured from the parent's right edge.
    pub const R_FROM_RIGHT: u32 = 0x4;
    /// B is measured from the parent's bottom edge.
    pub const B_FROM_BOTTOM: u32 = 0x8;
    /// L is a percentage of the parent width.
    pub const L_PERCENT: u32 = 0x10;
    /// T is a percentage of the parent height.
    pub const T_PERCENT: u32 = 0x20;
    /// R is a percentage of the parent width.
    pub const R_PERCENT: u32 = 0x40;
    /// B is a percentage of the parent height.
    pub const B_PERCENT: u32 = 0x80;
    /// R is a width added to L (`+237`).
    pub const R_SIZE: u32 = 0x100;
    /// B is a height added to T (`+758`).
    pub const B_SIZE: u32 = 0x200;
}

/// A rectangle in screen pixels: left, top, right, bottom (y down).
#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Rect {
    pub l: f32,
    pub t: f32,
    pub r: f32,
    pub b: f32,
}

impl Rect {
    pub const fn new(l: f32, t: f32, r: f32, b: f32) -> Self {
        Self { l, t, r, b }
    }
    pub fn width(&self) -> f32 {
        self.r - self.l
    }
    pub fn height(&self) -> f32 {
        self.b - self.t
    }
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.l && x < self.r && y >= self.t && y < self.b
    }
    pub fn to_array(self) -> [f32; 4] {
        [self.l, self.t, self.r, self.b]
    }
    pub fn from_array(a: [f32; 4]) -> Self {
        Self::new(a[0], a[1], a[2], a[3])
    }
}

/// Native bounds object (`0x34` bytes at VE+0x10 and VE+0x44): the raw
/// authored values, the resolved rectangle, the per-side clipped amounts and
/// the unit flags.
#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Bounds {
    /// Raw authored L, T, R, B numbers (offset 0x00..0x0c).
    pub raw: [f32; 4],
    /// Resolved rectangle (offset 0x10..0x1c).
    pub rect: Rect,
    /// Amount cut off each side by clipping to the parent, in order
    /// left, top, right, bottom (offset 0x20..0x2c).
    pub clip: [f32; 4],
    /// Unit flags (offset 0x30), see [`unit`].
    pub flags: u32,
}

/// MSVC CRT `_wtof` prefix grammar as used by `0x5E3150` (CRT 0x96D3E8):
/// leading `iswspace` is skipped, then `[sign] digits [. digits]
/// [e|E|d|D [sign] digits]` is converted; anything else stops the scan and an
/// empty prefix gives 0.0. The double is then stored to a binary32 field.
pub fn crt_wtof(text: &str) -> f64 {
    let s = text.trim_start_matches([' ', '\t', '\n', '\u{0b}', '\u{0c}', '\r']);
    let b = s.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let digits_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let int_digits = i - digits_start;
    let mut frac_digits = 0;
    if i < b.len() && b[i] == b'.' {
        let dot = i;
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        frac_digits = i - dot - 1;
        if int_digits == 0 && frac_digits == 0 {
            i = dot;
        }
    }
    if int_digits == 0 && frac_digits == 0 {
        return 0.0;
    }
    let mantissa_end = i;
    let mut exponent = String::new();
    if i < b.len() && matches!(b[i], b'e' | b'E' | b'd' | b'D') {
        let mut j = i + 1;
        let mut e = String::from("e");
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            e.push(b[j] as char);
            j += 1;
        }
        let es = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > es {
            e.push_str(&s[es..j]);
            exponent = e;
        }
    }
    let mut number = s[..mantissa_end].to_string();
    if number.ends_with('.') {
        number.push('0');
    }
    number.push_str(&exponent);
    number.parse::<f64>().unwrap_or(0.0)
}

fn last_char(s: &str) -> Option<char> {
    s.chars().last()
}

/// `0x5E3150`: parse the four bound strings into `bounds`.
///
/// Mirrors the native side effects exactly: the flags are reset to zero, each
/// value is converted (and stored) before its suffix is validated, and parsing
/// stops at the first invalid value leaving later fields untouched. Returns
/// `false` for an empty value, `+`-prefixed L/T, `b` on L or R, `r` on T or B,
/// or a `+` size combined with `r`/`b`.
pub fn parse_bounds(bounds: &mut Bounds, l: &str, t: &str, r: &str, b: &str) -> bool {
    bounds.flags = 0;
    let is_r = |c: Option<char>| matches!(c, Some('r') | Some('R'));
    let is_b = |c: Option<char>| matches!(c, Some('b') | Some('B'));
    let is_pct = |c: Option<char>| c == Some('%');

    // L
    bounds.raw[0] = crt_wtof(l) as f32;
    if l.is_empty() {
        return false;
    }
    let c = last_char(l);
    if l.starts_with('+') || is_b(c) {
        return false;
    }
    if is_r(c) {
        bounds.flags += unit::L_FROM_RIGHT;
    }
    if is_pct(c) {
        bounds.flags += unit::L_PERCENT;
    }
    // T
    bounds.raw[1] = crt_wtof(t) as f32;
    if t.is_empty() {
        return false;
    }
    let c = last_char(t);
    if t.starts_with('+') || is_r(c) {
        return false;
    }
    if is_b(c) {
        bounds.flags += unit::T_FROM_BOTTOM;
    }
    if is_pct(c) {
        bounds.flags += unit::T_PERCENT;
    }
    // R
    bounds.raw[2] = crt_wtof(r) as f32;
    if r.is_empty() {
        return false;
    }
    let c = last_char(r);
    if r.starts_with('+') {
        if is_r(c) || is_b(c) {
            return false;
        }
        bounds.flags += unit::R_SIZE;
    } else {
        if is_r(c) {
            bounds.flags += unit::R_FROM_RIGHT;
        }
        if is_b(c) {
            return false;
        }
    }
    if is_pct(c) {
        bounds.flags += unit::R_PERCENT;
    }
    // B
    bounds.raw[3] = crt_wtof(b) as f32;
    if b.is_empty() {
        return false;
    }
    let c = last_char(b);
    if b.starts_with('+') {
        if is_r(c) || is_b(c) {
            return false;
        }
        bounds.flags += unit::B_SIZE;
    } else {
        if is_r(c) {
            return false;
        }
        if is_b(c) {
            bounds.flags += unit::B_FROM_BOTTOM;
        }
    }
    if is_pct(c) {
        bounds.flags += unit::B_PERCENT;
    }
    true
}

/// The binary32 constant 0.01 at 0xA03078 used for percentages.
const PERCENT: f32 = 0.01;

/// `0x5E3370`: resolve `bounds` against `parent` with the UI scale, then
/// (when `clip`) clamp and clip it to the parent (`0x5E2460`). Returns the
/// native boolean result (true unless clipping/clamping happened).
///
/// `parent` is passed by reference because the native clip helper reads it
/// through a pointer (it is never modified).
pub fn resolve_bounds(
    bounds: &mut Bounds,
    parent: &Rect,
    sx: f32,
    sy: f32,
    clip: bool,
    p: Precision,
) -> bool {
    let f = bounds.flags;
    let (pl, pt, pr, pb) = (parent.l as f64, parent.t as f64, parent.r as f64, parent.b as f64);
    let (sx, sy) = (sx as f64, sy as f64);
    let [rl, rt, rr, rb] = bounds.raw.map(|v| v as f64);
    let pc = PERCENT as f64;
    // fld [p+8]; fsub [p]; fstp [esp]  (and the same for the height)
    let w = p.sub(pr, pl) as f32 as f64;
    let h = p.sub(pb, pt) as f32 as f64;

    // L
    let l_reg = if f & unit::L_PERCENT != 0 {
        p.add(p.mul(p.mul(w, rl), pc), pl)
    } else if f & unit::L_FROM_RIGHT != 0 {
        p.sub(pr, p.mul(sx, rl))
    } else {
        p.add(p.mul(sx, rl), pl)
    };
    let l32 = l_reg as f32; // fst [esp+0xc]; fstp [ecx+0x10]
    bounds.rect.l = l32;

    // T (kept on the x87 stack: `fst dword ptr [ecx+0x14]`)
    let t_reg = if f & unit::T_PERCENT != 0 {
        p.add(p.mul(p.mul(h, rt), pc), pt)
    } else if f & unit::T_FROM_BOTTOM != 0 {
        p.sub(pb, p.mul(sy, rt))
    } else {
        p.add(p.mul(sy, rt), pt)
    };
    bounds.rect.t = t_reg as f32;

    // R
    let r_reg = if f & unit::R_SIZE != 0 {
        if f & unit::R_PERCENT != 0 {
            p.add(p.mul(p.mul(w, rr), pc), l32 as f64)
        } else {
            p.add(p.mul(sx, rr), l32 as f64)
        }
    } else if f & unit::R_PERCENT != 0 {
        p.add(p.mul(p.mul(w, rr), pc), pl)
    } else if f & unit::R_FROM_RIGHT != 0 {
        p.sub(pr, p.mul(sx, rr))
    } else {
        p.add(p.mul(sx, rr), pl)
    };
    bounds.rect.r = r_reg as f32;

    // B (a size adds to the T value still on the x87 stack)
    let b_reg = if f & unit::B_SIZE != 0 {
        if f & unit::B_PERCENT != 0 {
            p.add(t_reg, p.mul(p.mul(h, rb), pc))
        } else {
            p.add(t_reg, p.mul(sy, rb))
        }
    } else if f & unit::B_PERCENT != 0 {
        p.add(p.mul(p.mul(h, rb), pc), pt)
    } else if f & unit::B_FROM_BOTTOM != 0 {
        p.sub(pb, p.mul(sy, rb))
    } else {
        p.add(p.mul(sy, rb), pt)
    };
    bounds.rect.b = b_reg as f32;

    if clip {
        clamp_and_clip(bounds, parent)
    } else {
        true
    }
}

/// `0x5E2460`: make R >= L and B >= T; if the rectangle lies inside `parent`
/// clear the clip amounts, otherwise clip it (`0x5E1580`).
fn clamp_and_clip(bounds: &mut Bounds, parent: &Rect) -> bool {
    let rect = &mut bounds.rect;
    // `!(R < L)` with NaN treated as "not less" by the x87 compare sequence
    let r_ok = !(rect.r < rect.l);
    if !r_ok {
        rect.r = rect.l;
    }
    let b_ok = !(rect.b < rect.t);
    if !b_ok {
        rect.b = rect.t;
    }
    if !(rect.l < parent.l) && rect.r <= parent.r && !(rect.t < parent.t) && rect.b <= parent.b {
        bounds.clip = [0.0; 4];
        return b_ok && r_ok;
    }
    clip_to_parent(rect, parent, &mut bounds.clip);
    false
}

/// `0x5E1580`: clip `rect` to `parent`, recording the amount removed on each
/// side (left, top, right, bottom).
fn clip_to_parent(rect: &mut Rect, parent: &Rect, out: &mut [f32; 4]) {
    *out = [0.0; 4];
    if rect.l < parent.l {
        out[0] = parent.l - rect.l;
        rect.l = parent.l;
        if rect.r < rect.l {
            rect.r = rect.l;
        }
    }
    if parent.r < rect.r {
        out[2] = rect.r - parent.r;
        rect.r = parent.r;
        if rect.r < rect.l {
            rect.l = rect.r;
        }
    }
    if rect.t < parent.t {
        out[1] = parent.t - rect.t;
        rect.t = parent.t;
        if rect.b < rect.t {
            rect.b = rect.t;
        }
    }
    if parent.b < rect.b {
        out[3] = rect.b - parent.b;
        rect.b = parent.b;
        if rect.b < rect.t {
            rect.t = rect.b;
        }
    }
}

/// The interface reference size (AVars `InterfacePositioningWidth/Height`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reference {
    pub width: f32,
    pub height: f32,
}

impl Default for Reference {
    /// Defaults registered at 0x402C5A/0x402CA0 (0x44800000, 0x44400000) and
    /// authored in `Data/AVars.tgi`.
    fn default() -> Self {
        Self { width: 1024.0, height: 768.0 }
    }
}

/// Screen (back buffer) size in pixels, the ints at 0xB396D0/0xB396D4.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Screen {
    pub width: i32,
    pub height: i32,
}

/// `0x5C1DB0` / `0x5C1DD0`: `fild screen; fdiv reference` stored as binary32.
pub fn ui_scale(screen: Screen, reference: Reference, p: Precision) -> (f32, f32) {
    let sx = p.div(screen.width as f64, reference.width as f64) as f32;
    let sy = p.div(screen.height as f64, reference.height as f64) as f32;
    (sx, sy)
}

/// View-element flag bits used by the layout (VE+0x88).
pub mod ve_flag {
    /// Set after layout (`or 1`).
    pub const LAID_OUT: u32 = 0x1;
    /// Visible (set by the constructor `0x5E3BF0`, `SetVisible` `0x5E1970`).
    pub const VISIBLE: u32 = 0x4;
    /// Active (`0x5E1C70`, set recursively while the owning widget is
    /// active).
    pub const ACTIVE: u32 = 0x40;
    /// Set after layout (`or 0x20`), cleared by geometry rebuild.
    pub const GEOMETRY_DIRTY: u32 = 0x20;
    /// Loaded (set by `0x5E5780`).
    pub const LOADED: u32 = 0x80;
    /// Decal `tiled_x` (`0x5E81E0`).
    pub const TILED_X: u32 = 0x100;
    /// Decal `tiled_y`.
    pub const TILED_Y: u32 = 0x200;
    /// Outer rectangle has zero width or height.
    pub const EMPTY: u32 = 0x400;
    /// Clip to the parent rectangle (bit 12, passed to `0x5E3370`).
    pub const CLIP_TO_PARENT: u32 = 0x1000;
    /// Label `multiline` (`0x5E5BA0`).
    pub const MULTILINE: u32 = 0x2000;
}

/// Layout state of one view element: outer and client bounds plus flags.
#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct VeLayout {
    pub outer: Bounds,
    pub client: Bounds,
    pub flags: u32,
}

/// `0x5E34B0`: lay out a view element from its parent's already laid out
/// state (`None` for a root element, laid out against the whole screen).
pub fn layout_ve(
    ve: &mut VeLayout,
    parent: Option<&VeLayout>,
    screen: Screen,
    reference: Reference,
    p: Precision,
) {
    let clip = (ve.flags >> 12) & 1 != 0;
    let (sx, sy) = ui_scale(screen, reference, p);
    match parent {
        Some(parent) => {
            let parent_rect = parent.outer.rect;
            resolve_bounds(&mut ve.outer, &parent_rect, sx, sy, clip, p);
            for i in 0..4 {
                ve.outer.clip[i] = p.add(parent.outer.clip[i] as f64, ve.outer.clip[i] as f64) as f32;
            }
        }
        None => {
            let screen_rect = Rect::new(0.0, 0.0, screen.width as f32, screen.height as f32);
            resolve_bounds(&mut ve.outer, &screen_rect, sx, sy, clip, p);
        }
    }
    let own = ve.outer.rect;
    resolve_bounds(&mut ve.client, &own, sx, sy, clip, p);
    let h = (own.b - own.t).abs();
    let w = (own.r - own.l).abs();
    let mut flags = ve.flags;
    if h == 0.0 || w == 0.0 {
        flags |= ve_flag::EMPTY;
    } else {
        flags &= !ve_flag::EMPTY;
    }
    ve.flags = flags | ve_flag::LAID_OUT | ve_flag::GEOMETRY_DIRTY;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wtof_prefix_grammar() {
        assert_eq!(crt_wtof("26"), 26.0);
        assert_eq!(crt_wtof("+237"), 237.0);
        assert_eq!(crt_wtof("50%"), 50.0);
        assert_eq!(crt_wtof("13r"), 13.0);
        assert_eq!(crt_wtof(" 7"), 7.0);
        assert_eq!(crt_wtof("1e1"), 10.0);
        assert_eq!(crt_wtof("3.75b"), 3.75);
        assert_eq!(crt_wtof("abc"), 0.0);
        assert_eq!(crt_wtof("."), 0.0);
        assert_eq!(crt_wtof("5."), 5.0);
        assert_eq!(crt_wtof("2d2"), 200.0);
    }

    #[test]
    fn main_menu_panel_at_2560x1440() {
        let mut b = Bounds::default();
        assert!(parse_bounds(&mut b, "783", "8", "+237", "+758"));
        let mut ve = VeLayout { outer: b, ..Default::default() };
        assert!(parse_bounds(&mut ve.client, "0", "0", "0r", "0b"));
        layout_ve(&mut ve, None, Screen { width: 2560, height: 1440 }, Reference::default(), Precision::Pc24);
        assert_eq!(ve.outer.rect, Rect::new(1957.5, 15.0, 2550.0, 1436.25));
    }
}
