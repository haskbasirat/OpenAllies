//! `KKC_RText` draw layout (`.\Source\Renderer\Text.cpp`): the rebuild
//! `0x735B90` called by the screen-text update `0x735EE0`, with
//! PreProcessText `0x734120` (row breaking, wrap insertion of 0x0D / 0x0E
//! into the processed text, row and glyph records, size and overflow
//! flags), GetRowStartingXPixels `0x730130`, Parse `0x733B10` (font,
//! colour and highlight markup, mixed-font vertical offsets,
//! `use_own_color`), the per-glyph quad builder `0x730F00` and the text
//! batch AddQuad `0x72FA00` (clamp to the screen, clip to the area rect,
//! floor to pixels, highlight quad, overflow tint).
//!
//! NATIVE-PROVEN: research/phase8-ui/native_draw.py runs the original
//! `0x735B90` under Unicorn on a `KKC_RScreenText` (vtable 0xA89B90) at
//! PC24 and PC53; tests/native_draw.rs replays every case
//! (`native-draw.json`: processed text, row records, size, flags, pen and
//! errors exactly, glyph records and quads by FNV-1a digest).
//!
//! The view element sets the area to the label's clipped outer rect, the
//! origin to (-clip left, -clip top) (`0x5E2C90`); there is no vertical
//! alignment. Native behaviour kept as is (all proven): the wrap test uses
//! the origin-shifted pen; when one glyph cannot fit after a forced break
//! the rest of its token is dropped; the insert / replace positions of
//! later wraps drift after an earlier insertion (they are raw positions of
//! the original text applied to the processed text) and can corrupt
//! markup; `<RC>` before any `<Color>` restores (0,0,0,0); rows advance by
//! the largest 'A' height of the fonts on the row; glyphs after a taller
//! font on the same row are bottom-aligned; quads are the advance wide (the
//! shadow column of the cell is not drawn).

use crate::layout::Precision;
use crate::rtext::{GlyphTable, TextEnv, f32r, msvc_atof, next_token, trim};

/// The `KKC_RText` fields the rebuild reads.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawParams {
    /// RText+0x44 (0 = the font system default per character).
    pub font: u32,
    /// RText+0x48 colour, +0x58 highlight colour.
    pub color: [f32; 4],
    pub highlight: [f32; 4],
    /// RText+0x74 / +0x78.
    pub origin: (f32, f32),
    /// RText+0xA8 area rect L, T, R, B (screen pixels).
    pub area: [f32; 4],
    /// RText+0x8C: 0 left, 1 centre, 2 right, else unsupported.
    pub justify: i32,
    /// RText+0x98 / +0x99.
    pub multiline: bool,
    pub fixed: bool,
    /// The overflow debug flag `*(*(0xB39940+0x1E8))`.
    pub debug_tint: bool,
}

/// Row record (0x14 bytes at RText+0x1C).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowRec {
    pub w: f32,
    pub h: f32,
    pub first: u32,
    /// Last glyph record (the break record included).
    pub last: u32,
    pub start_x: f32,
}

/// Glyph record (0x20 bytes at RText+0x28): caret and hit testing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphRec {
    pub c: u32,
    pub row: u32,
    pub idx: u32,
    /// Visible text index (-1 for inserted breaks).
    pub text_idx: i32,
    pub rect: [f32; 4],
}

/// One emitted quad: `geo` = x0, y0, x1, y1, u0, v0, u1, v1 (atlas pixels).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextQuad {
    /// The batch's +0x10 list (highlight background) instead of +0xC.
    pub highlight: bool,
    pub font: u32,
    /// Glyph index drawn (after stand-in and missing-glyph substitution).
    pub glyph: u32,
    pub geo: [f32; 8],
    pub color: [f32; 4],
}

/// The rebuilt text object.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DrawnText {
    /// RText+0xC (UTF-16 units, wraps inserted).
    pub processed: Vec<u32>,
    pub rows: Vec<RowRec>,
    pub glyphs: Vec<GlyphRec>,
    /// RText+0x84 / +0x88.
    pub size: [f32; 2],
    /// RText+0x9C (height overflow), +0x9D (any overflow).
    pub flags: [bool; 2],
    /// Final pen RText+0x68 / +0x6C.
    pub pen: [f32; 2],
    pub quads: Vec<TextQuad>,
    /// Logged errors (`0x404EA0`).
    pub errors: Vec<String>,
    /// The original would crash here (null font at the end of PreProcessText).
    pub unsupported: Option<String>,
}

const REMAP: [u32; 5] = [0x11, 0x12, 0x13, 0x14, 0x15];
const REMAP_TO: [u32; 5] = ['<' as u32, '>' as u32, '"' as u32, '\\' as u32, '&' as u32];
const DELIMS: [u32; 2] = ['<' as u32, '>' as u32];

struct St {
    font: u32,
    colour: [f64; 4],
    saved: [f64; 4],
    flag: bool,
    pen_x: f64,
    pen_y: f64,
}

pub struct RDraw<'a> {
    pub env: &'a TextEnv,
    pub table: &'a dyn GlyphTable,
}

impl RDraw<'_> {
    fn r(&self, x: f64) -> f64 {
        match self.env.precision {
            Precision::Pc24 => f32r(x),
            Precision::Pc53 => x,
        }
    }

    fn adv(&self, font: u32, c: u32) -> f64 {
        self.table.glyph(font, c).map(|g| g.advance()).unwrap_or(0) as f64
    }

    fn line_h(&self, font: u32) -> f64 {
        self.table.glyph(font, 65).map(|g| g.h as i32).unwrap_or(0) as f64
    }

    fn cell_h(&self, font: u32, c: u32) -> f64 {
        self.table.glyph(font, c).map(|g| g.h as i32).unwrap_or(0) as f64
    }

    fn has(&self, font: u32, c: u32) -> bool {
        self.table.glyph(font, c).is_some()
    }

    fn set_of(&self, c: u32) -> Option<(u32, u32)> {
        self.env.sets.iter().find(|s| s.0 <= c && c <= s.1).map(|s| (s.2, s.3))
    }

    fn glyph_for(&self, font: u32, c: u32) -> u32 {
        let mut g = c;
        if c == 10 || c == 13 || c == 14 {
            g = self.set_of(c).map(|s| s.1).unwrap_or(self.env.standin);
        }
        if !self.has(font, g) {
            g = self.set_of(g).map(|s| s.0).unwrap_or(self.env.undisplayable);
            if !self.has(font, g) {
                g = self.env.fallback;
            }
        }
        g
    }

    /// Markup `0x7335B0` / `0x7331E0`: returns the code of the last command.
    fn markup(&self, tok: &[u32], st: &mut St) -> i32 {
        let mut r = 7;
        let mut pos = 0;
        loop {
            let (sub, p) = next_token(tok, pos, &[';' as u32], true);
            pos = p;
            if sub.is_empty() {
                return r;
            }
            let (k, p2) = next_token(&sub, 0, &['=' as u32], true);
            let (v, _) = next_token(&sub, p2, &['=' as u32], true);
            let (k, v) = (trim(&k), trim(&v));
            match k.to_ascii_lowercase().as_str() {
                "h_on" => {
                    st.flag = true;
                    r = 0;
                }
                "h_off" => {
                    st.flag = false;
                    r = 1;
                }
                "font" => {
                    st.font = self.table.font_by_name(&v);
                    r = 2;
                }
                "rc" | "restorecolor" => {
                    st.colour = st.saved;
                    r = 4;
                }
                "color" => {
                    let vu: Vec<u32> = v.encode_utf16().map(|u| u as u32).collect();
                    let mut comps = [0.0f64; 4];
                    let mut p = 0;
                    for (n, c) in comps.iter_mut().enumerate() {
                        let (t, np) = next_token(&vu, p, &[',' as u32], true);
                        p = np;
                        let t = trim(&t);
                        *c = if t.is_empty() { if n == 3 { 1.0 } else { 0.0 } } else { f32r(msvc_atof(&t)) };
                    }
                    st.saved = st.colour;
                    let a = f32r(self.r(comps[3] * st.colour[3]));
                    st.colour = [comps[0], comps[1], comps[2], a];
                    r = 3;
                }
                "setx" => {
                    let mut x = msvc_atof(&v);
                    if x < st.pen_x {
                        let sp = if st.font != 0 { self.adv(st.font, 32) } else { 4.0 };
                        x = self.r(sp + st.pen_x);
                    }
                    st.pen_x = f32r(x);
                    r = 5;
                }
                "sety" => {
                    st.pen_y = f32r(msvc_atof(&v));
                    r = 6;
                }
                _ => r = 7,
            }
        }
    }

    /// The rebuild `0x735B90` of `text` (UTF-16 units of the label text).
    pub fn rebuild(&self, text: &str, p: &DrawParams) -> DrawnText {
        let units: Vec<u32> = text.encode_utf16().map(|u| u as u32).collect();
        let mut out = DrawnText::default();
        self.preprocess(&units, p, &mut out);
        let mut pen = [p.origin.0 as f64, p.origin.1 as f64];
        if !units.is_empty() && out.unsupported.is_none() {
            let s = self.row_start(p, &out.rows, 0, &mut out.errors);
            let processed = out.processed.clone();
            pen = self.parse(&processed, p, &mut out, f32r(s), p.origin.1 as f64);
        }
        out.pen = [pen[0] as f32, pen[1] as f32];
        out
    }

    /// PreProcessText `0x734120`.
    fn preprocess(&self, text: &[u32], p: &DrawParams, out: &mut DrawnText) {
        let r = |x: f64| self.r(x);
        let mut full: Vec<u32> = text.to_vec();
        let mut font = p.font;
        let (ox, oy) = (p.origin.0 as f64, p.origin.1 as f64);
        let (mut pen_x, mut pen_y) = (ox, oy);
        let mut size = [0.0f64; 2];
        let mut rows: Vec<RowRec> = Vec::new();
        let mut gl: Vec<GlyphRec> = Vec::new();
        let (mut row_w, mut row_h, mut trailing, mut w_sp) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        let (mut idx, mut row_first) = (0u32, 0u32);
        let mut last_space: i64 = -1;
        let mut retry = false;
        let mut vis: i64 = 0;
        let mut xstart = 0.0f64;
        let (mut has_xs, mut wrapped) = (false, false);
        let [l, t, rr, b] = p.area.map(|v| v as f64);
        let w_area = r(rr - l);
        let mut pos = 0usize;
        let mut in_markup = text.first() == Some(&('<' as u32));
        let rec = |c: u32, row: usize, idx: u32, ti: i64, x0: f64, y0: f64, x1: f64, y1: f64| GlyphRec {
            c,
            row: row as u32,
            idx,
            text_idx: ti as i32,
            rect: [x0 as f32, y0 as f32, x1 as f32, y1 as f32],
        };
        loop {
            let mut tok_start = pos as i64;
            let (tok, np) = next_token(text, pos, &DELIMS, false);
            pos = np;
            if tok.is_empty() {
                break;
            }
            if in_markup {
                let mut st = St { font, colour: [0.0; 4], saved: [0.0; 4], flag: false, pen_x, pen_y };
                let code = self.markup(&tok, &mut st);
                font = st.font;
                pen_x = st.pen_x;
                pen_y = st.pen_y;
                if code == 5 {
                    row_w = pen_x;
                    xstart = pen_x;
                }
                in_markup = text.get(pos) == Some(&('<' as u32));
                continue;
            }
            let flag = pos < text.len();
            let mut tok = tok;
            let mut i: i64 = 0;
            while (i as usize) < tok.len() {
                let mut c = tok[i as usize];
                if let Some(k) = REMAP.iter().position(|&x| x == c) {
                    c = REMAP_TO[k];
                }
                if font == 0 {
                    font = self.env.default_font;
                }
                let g = self.glyph_for(font, c);
                let adv = if p.fixed { self.table.fixed(font) as f64 } else { self.adv(font, g) };
                let h = self.line_h(font);
                if c == 10 || c == 13 || c == 14 {
                    if w_sp > size[0] {
                        size[0] = w_sp;
                    }
                    size[1] = f32r(r(row_h + size[1]));
                    if c == 10 {
                        gl.push(rec(c, rows.len(), idx, i + vis, pen_x, pen_y, f32r(r(pen_x + 1.0)), f32r(r(pen_y + h))));
                    } else {
                        gl.push(rec(c, rows.len(), idx, -1, w_sp, pen_y, w_sp, f32r(r(pen_y + h))));
                    }
                    rows.push(RowRec {
                        w: row_w as f32,
                        h: row_h as f32,
                        first: row_first,
                        last: gl.len() as u32 - 1,
                        start_x: if has_xs { xstart as f32 } else { 0.0 },
                    });
                    row_first = gl.len() as u32;
                    row_h = h;
                    idx = 0;
                    trailing = 0.0;
                    w_sp = 0.0;
                    if wrapped {
                        row_w = xstart;
                        has_xs = true;
                        pen_x = f32r(r(xstart + ox));
                    } else {
                        row_w = 0.0;
                        pen_x = ox;
                        has_xs = false;
                    }
                    pen_y = f32r(r(pen_y + h));
                    if !wrapped {
                        xstart = 0.0;
                    }
                    wrapped = false;
                    i += 1;
                    continue;
                }
                if c == 32 {
                    gl.push(rec(c, rows.len(), idx, i + vis, pen_x, pen_y, f32r(r(adv + pen_x)), f32r(r(pen_y + h))));
                    pen_x = f32r(r(adv + pen_x));
                    idx += 1;
                    last_space = i + tok_start;
                    retry = false;
                    w_sp = row_w;
                    trailing = f32r(r(adv + trailing));
                    i += 1;
                    continue;
                }
                if p.multiline && w_area < f32r(r(adv + pen_x)) {
                    if last_space == -1 {
                        if retry {
                            // 0x7347AC: the rest of the token is dropped
                            break;
                        }
                        tok.insert(i as usize, 13);
                        let at = ((i + tok_start).max(0) as usize).min(full.len());
                        full.insert(at, 13);
                        retry = true;
                        w_sp = row_w;
                        idx = 0;
                        continue;
                    }
                    let s = last_space - tok_start;
                    retry = false;
                    let n = if s < 0 { i + 1 } else { i - s };
                    for j in 0..n {
                        let k = gl.len() as i64 - 1 - j;
                        let gc = if k >= 0 { gl[k as usize].c } else { 0 };
                        row_w = f32r(r(row_w - self.adv(font, gc)));
                    }
                    if n > 0 {
                        let start = (gl.len() as i64 - n).max(0) as usize;
                        gl.truncate(start);
                    }
                    if s < 0 {
                        tok.insert(0, 13);
                        tok_start -= 1;
                        i = 0;
                    } else {
                        tok[s as usize] = 14;
                        i = s;
                    }
                    if let Some(x) = full.get_mut(last_space as usize) {
                        *x = 14;
                    }
                    wrapped = true;
                    last_space = -1;
                    continue;
                }
                gl.push(rec(c, rows.len(), idx, i + vis, pen_x, pen_y, f32r(r(adv + pen_x)), f32r(r(pen_y + h))));
                pen_x = f32r(r(adv + pen_x));
                idx += 1;
                retry = false;
                row_w = f32r(r(r(adv + trailing) + row_w));
                trailing = 0.0;
                if h > row_h {
                    row_h = h;
                }
                i += 1;
            }
            vis += tok.len() as i64;
            in_markup = flag;
        }
        if font == 0 {
            out.unsupported = Some("0x734120: null font at the end of the text (the original crashes)".into());
        }
        gl.push(rec(0, rows.len(), idx, vis, pen_x, pen_y, f32r(r(pen_x + 1.0)), f32r(r(self.line_h(font) + pen_y))));
        if row_w > 0.0 {
            if row_w > size[0] {
                size[0] = row_w;
            }
            rows.push(RowRec {
                w: row_w as f32,
                h: row_h as f32,
                first: row_first,
                last: gl.len() as u32 - 1,
                start_x: if has_xs { xstart as f32 } else { 0.0 },
            });
        }
        size[1] = f32r(r(row_h + size[1]));
        let h_area = r(b - t);
        out.flags = [h_area < size[1], w_area < size[0] || h_area < size[1]];
        out.size = [size[0] as f32, size[1] as f32];
        out.processed = full;
        out.rows = rows;
        out.glyphs = gl;
    }

    /// GetRowStartingXPixels `0x730130`.
    fn row_start(&self, p: &DrawParams, rows: &[RowRec], row: usize, errors: &mut Vec<String>) -> f64 {
        if row >= rows.len() {
            return 0.0;
        }
        let [l, _, rr, _] = p.area.map(|v| v as f64);
        let w = f32r(self.r(rr - l));
        let s = rows[row].start_x as f64;
        if s > 0.0 {
            return s;
        }
        let rw = rows[row].w as f64;
        match p.justify {
            0 => p.origin.0 as f64,
            1 => self.r(self.r(self.r(w - rw) * 0.5) + 0.5).floor(),
            2 => self.r(w - rw),
            _ => {
                errors.push("Unsupported justification mode.".into());
                0.0
            }
        }
    }

    /// Parse `0x733B10` with the quad builder `0x730F00`: returns the pen.
    fn parse(&self, full: &[u32], p: &DrawParams, out: &mut DrawnText, pen_x: f64, pen_y: f64) -> [f64; 2] {
        let r = |x: f64| self.r(x);
        let [l, t, _, b] = p.area.map(|v| v as f64);
        let mut font = p.font;
        let mut st = St {
            font,
            colour: p.color.map(|v| v as f64),
            saved: [0.0; 4],
            flag: false,
            pen_x,
            pen_y,
        };
        let ws = self.env.world_scale as f64;
        let bs = self.env.baseline_scale as f64;
        let (mut hmax, mut yacc, mut ysub) = (0.0f64, 0.0f64, 0.0f64);
        let mut base = 0.0f64;
        let mut row_idx = 1usize;
        let mut pos = 0usize;
        let mut in_markup = full.first() == Some(&('<' as u32));
        let overflow = out.flags[1];
        let rows = out.rows.clone();
        loop {
            let (tok, np) = next_token(full, pos, &DELIMS, false);
            pos = np;
            if tok.is_empty() {
                break;
            }
            if in_markup {
                st.font = font;
                self.markup(&tok, &mut st);
                font = st.font;
                in_markup = full.get(pos) == Some(&('<' as u32));
                continue;
            }
            in_markup = pos < full.len();
            for &ch in &tok {
                let mut c = ch;
                if let Some(k) = REMAP.iter().position(|&x| x == c) {
                    c = REMAP_TO[k];
                }
                if font == 0 {
                    font = self.env.default_font;
                }
                let g = self.glyph_for(font, c);
                let hh = self.line_h(font);
                if hh > hmax {
                    base = f32r(r(r(self.table.baseline(font) as f64 / bs) + base));
                    hmax = hh;
                } else if r(self.line_h(font) / ws) < hmax {
                    ysub = base;
                    yacc = f32r(r(hmax - hh));
                }
                let cv = if self.table.own_color(g) { [1.0; 4] } else { st.colour };
                let yoff = f32r(r(yacc - ysub));
                let adv = self.adv(font, g);
                let fixed_w = self.table.fixed(font) as f64;
                let fixoff = if p.fixed { f32r(r(r(fixed_w - adv) * 0.5)) } else { 0.0 };
                let cell_h = self.cell_h(font, g);
                let vc = f32r(r(r(hh - cell_h) * 0.5));
                if c == 10 || c == 13 || c == 14 {
                    let s = self.row_start(p, &rows, row_idx, &mut out.errors);
                    st.pen_x = f32r(s);
                    st.pen_y = f32r(r(hmax + st.pen_y));
                    row_idx += 1;
                    hmax = 0.0;
                    yacc = 0.0;
                    ysub = 0.0;
                    continue;
                }
                let y = f32r(r(r(t + st.pen_y) + yoff));
                if b > y {
                    let (ax, ay) = self.table.atlas(font, g);
                    let (x, yq) = if p.fixed {
                        (f32r(r(r(l + st.pen_x) + fixoff)), f32r(r(r(t + st.pen_y) + yoff)))
                    } else {
                        (f32r(r(l + st.pen_x)), f32r(r(r(r(t + st.pen_y) + vc) + yoff)))
                    };
                    self.add_quad(p, [x, yq, adv, cell_h, ax as f64, ay as f64], cv, st.flag, overflow, font, g, &mut out.quads);
                }
                st.pen_x = if p.fixed { f32r(r(fixed_w + st.pen_x)) } else { f32r(r(adv + st.pen_x)) };
            }
        }
        [st.pen_x, st.pen_y]
    }

    /// AddQuad `0x72FA00`: `q` = x, y, w, h, u, v.
    #[allow(clippy::too_many_arguments)]
    fn add_quad(&self, p: &DrawParams, q: [f64; 6], col: [f64; 4], hl_on: bool, overflow: bool, font: u32, g: u32, quads: &mut Vec<TextQuad>) {
        let r = |x: f64| self.r(x);
        let [x, y, w, h, mut u, mut v] = q;
        let [l, t, rr, b] = p.area.map(|v| v as f64);
        let clamp_floor = |val: f64, lim: f64| {
            let v = if val < 0.0 {
                0.0
            } else if val > lim {
                lim
            } else {
                val
            };
            v.floor()
        };
        let (sw, sh) = (self.env.screen.0 as f64, self.env.screen.1 as f64);
        let sh1 = f32r(r(sh - 1.0));
        let sw1 = f32r(r(sw - 1.0));
        let mut yq = f32r(clamp_floor(y, sh1));
        let tq = clamp_floor(t, sh1);
        let mut hq = h;
        if yq < tq {
            let d = r(tq - yq);
            if d > h {
                return;
            }
            yq = f32r(r(yq + d));
            v = f32r(r(v + d));
            hq = f32r(r(h - d));
        }
        let bq = clamp_floor(b, sh1);
        if r(hq + yq) > bq {
            let d = r(bq - yq);
            if d <= 0.0 {
                return;
            }
            hq = f32r(d);
        }
        let mut xq = f32r(clamp_floor(x, sw1));
        let lq = clamp_floor(l, sw1);
        let mut wq = w;
        if xq < lq {
            let d = r(lq - xq);
            if d > w {
                return;
            }
            xq = f32r(r(xq + d));
            u = f32r(r(u + d));
            wq = f32r(r(w - d));
        }
        let rq = clamp_floor(rr, sw1);
        if r(wq + xq) > rq {
            let d = r(rq - xq);
            if d <= 0.0 {
                return;
            }
            wq = f32r(d);
        }
        let geo = [xq, yq, f32r(r(wq + xq)), f32r(r(hq + yq)), u, v, f32r(r(wq + u)), f32r(r(hq + v))].map(|z| z as f32);
        let mut col = col;
        let hl = p.highlight.map(|z| z as f64);
        if hl_on {
            if col == hl {
                col = [f32r(r(1.0 - col[0])), f32r(r(1.0 - col[1])), f32r(r(1.0 - col[2])), col[3]];
            }
            quads.push(TextQuad { highlight: true, font, glyph: g, geo, color: p.highlight });
        }
        if overflow && p.debug_tint {
            col = [1.0, 0.0, 0.0, 1.0];
        }
        quads.push(TextQuad { highlight: false, font, glyph: g, geo, color: col.map(|z| z as f32) });
    }
}
