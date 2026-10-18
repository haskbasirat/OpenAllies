//! `KKC_RText` measuring (`.\Source\Renderer\Text.cpp`): GetTextExtent
//! `0x7352C0`, its reference-unit wrapper `0x735C00`, the world-unit wrapper
//! `0x735D20` and the ellipsis truncation `0x735D50` with the markup-aware
//! helpers `0x732C60` (visible length) and `0x732E60` (Left).
//!
//! NATIVE-PROVEN: research/phase8-ui/native_text.py runs the original code
//! under Unicorn over packed glyph tables (format of `0x705020`) at PC24
//! and PC53; tests/native_text.rs replays every case (`native-text.json`).
//!
//! Glyph cells come from a [`GlyphTable`] (font atlas `font+0x30`): the
//! advance is the cell width minus 1 when the shadow bit is set
//! (`0x7051A0`), the line height is the cell height of 'A' (`0x705580`).
//! Missing glyphs take the character set's `undisplayable_character`
//! (`0x705840`), else 149, else '#'; 10/13/14 measure as the set's
//! `spacing_standin` (`0x705870`), else 't'.
//!
//! Native behaviour kept as is (all proven): each wrap counts two rows;
//! the width of a wrapped measure includes the partial word moved to the
//! next row; `\r` measures as 't'; `trail` adds only the last row's
//! trailing spaces. One native behaviour is NOT reproduced: when a glyph is
//! wider than the wrap width the original loops forever inserting
//! newlines; this port returns [`Measured::Hang`] instead.

use crate::layout::Precision;

/// One atlas cell (`0x705020`: w 7 bits, h 7 bits, shadow, outline).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Glyph {
    pub w: u16,
    pub h: u16,
    pub shadow: bool,
    pub outline: bool,
}

impl Glyph {
    /// `0x7051A0`.
    pub fn advance(&self) -> i32 {
        if self.shadow { self.w as i32 - 1 } else { self.w as i32 }
    }
}

/// The font atlas tables the measure reads. Font id 0 = "no font".
pub trait GlyphTable {
    fn glyph(&self, font: u32, c: u32) -> Option<Glyph>;
    /// `font+0xC`: the widest padded cell + 2 (fixed-width advance).
    fn fixed(&self, font: u32) -> i32;
    /// `0x709280` (`<Font=name>`), 0 when unknown.
    fn font_by_name(&self, name: &str) -> u32;
    /// Atlas cell position (`0x705160` / `0x705180`), (0, 0) when absent.
    fn atlas(&self, _font: u32, _c: u32) -> (u16, u16) {
        (0, 0)
    }
    /// `font+8` (the baseline byte read by Parse `0x733B10`).
    fn baseline(&self, _font: u32) -> i32 {
        0
    }
    /// Special glyph `use_own_color` (`0x705510`: the global special glyph
    /// map at `[0xB36530]+0x3BC`, keyed by glyph index).
    fn own_color(&self, _c: u32) -> bool {
        false
    }
}

/// Font-system constants and character sets.
#[derive(Clone, Debug)]
pub struct TextEnv {
    pub precision: Precision,
    /// fontsys+0x10.
    pub default_font: u32,
    /// fontsys+0x20 (149), +0x22 ('#'), +0x24 ('t').
    pub undisplayable: u32,
    pub fallback: u32,
    pub standin: u32,
    /// Character sets: (min, max, undisplayable_character, spacing_standin).
    pub sets: Vec<(u32, u32, u32, u32)>,
    /// AVar TextWorldScaleFactor (`[0xB39990]+0xA0`).
    pub world_scale: f32,
    /// AVar TextBaselineScaleFactor (`[0xB39990]+0xA4`).
    pub baseline_scale: f32,
    /// Screen size and InterfacePositioningWidth/Height (`0x5C1DB0`/`0x5C1DD0`).
    pub screen: (f32, f32),
    pub reference: (f32, f32),
    /// Localized `grammar_ellipsis`.
    pub ellipsis: String,
}

/// Result of a measure.
#[derive(Clone, Debug, PartialEq)]
pub enum Measured {
    /// `ret`, size, rows (`None` when the clip mode ended the call early:
    /// the rows out-parameter is not written).
    Done { count: i32, size: [f32; 2], rows: Option<i32> },
    /// The original never returns for this input (see the module doc).
    Hang,
}

pub(crate) fn f32r(x: f64) -> f64 {
    x as f32 as f64
}

const WS: [u32; 6] = [0x20, 0x09, 0x0A, 0x0B, 0x0C, 0x0D];
const GROWTH_LIMIT: usize = 4000;

/// Tokenizer `0x732720` (delimiters skipped, the ending delimiter consumed).
pub(crate) fn next_token(text: &[u32], mut pos: usize, delims: &[u32], skip_ws: bool) -> (Vec<u32>, usize) {
    let n = text.len();
    if pos >= n {
        return (Vec::new(), pos);
    }
    let mut c = text[pos];
    if skip_ws {
        while WS.contains(&c) {
            pos += 1;
            if pos >= n {
                return (Vec::new(), pos);
            }
            c = text[pos];
        }
    }
    while delims.contains(&c) {
        pos += 1;
        if pos >= n {
            return (Vec::new(), pos);
        }
        c = text[pos];
    }
    let mut tok = Vec::new();
    loop {
        tok.push(c);
        pos += 1;
        c = if pos < n { text[pos] } else { 0 };
        if c != 0 && delims.contains(&c) {
            return (tok, pos + 1);
        }
        if pos >= n {
            return (tok, pos);
        }
    }
}

pub(crate) fn trim(v: &[u32]) -> String {
    let s: String = v.iter().filter_map(|&c| char::from_u32(c)).collect();
    s.trim().to_string()
}

/// MSVC `_wtof` prefix grammar (the oracle's seam model).
pub fn msvc_atof(text: &str) -> f64 {
    let t = text.trim_start_matches([' ', '\t', '\n', '\x0b', '\x0c', '\r']);
    let b = t.as_bytes();
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let ds = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let mut digits = i > ds;
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let fs = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        digits |= i > fs;
    }
    if !digits {
        return 0.0;
    }
    let mut end = i;
    if i < b.len() && matches!(b[i], b'e' | b'E' | b'd' | b'D') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let es = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > es {
            end = j;
        }
    }
    t[..end].replace(['d', 'D'], "e").parse::<f64>().unwrap_or(0.0)
}

struct MarkupState {
    font: u32,
    pen: f64,
    yacc: f64,
}

pub struct RText<'a> {
    pub env: &'a TextEnv,
    pub table: &'a dyn GlyphTable,
}

impl RText<'_> {
    fn r(&self, x: f64) -> f64 {
        match self.env.precision {
            Precision::Pc24 => f32r(x),
            Precision::Pc53 => x,
        }
    }

    fn has(&self, font: u32, c: u32) -> bool {
        self.table.glyph(font, c).is_some()
    }

    fn adv(&self, font: u32, c: u32) -> i32 {
        self.table.glyph(font, c).map(|g| g.advance()).unwrap_or(0)
    }

    fn line_h(&self, font: u32) -> i32 {
        self.table.glyph(font, 65).map(|g| g.h as i32).unwrap_or(0)
    }

    fn set_of(&self, c: u32) -> Option<(u32, u32)> {
        self.env.sets.iter().find(|s| s.0 <= c && c <= s.1).map(|s| (s.2, s.3))
    }

    /// Markup command token (`0x7335B0` / `0x7331E0`): returns the code of
    /// the last command.
    fn markup(&self, tok: &[u32], st: &mut MarkupState) -> i32 {
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
                "h_on" => r = 0,
                "h_off" => r = 1,
                "font" => {
                    st.font = self.table.font_by_name(&v);
                    r = 2;
                }
                "rc" | "restorecolor" => r = 4,
                "color" => r = 3,
                "setx" => {
                    let mut x = msvc_atof(&v);
                    if x < st.pen {
                        let sp = if st.font != 0 { self.adv(st.font, 32) as f64 } else { 4.0 };
                        x = self.r(sp + st.pen);
                    }
                    st.pen = f32r(x);
                    r = 5;
                }
                "sety" => {
                    st.yacc = f32r(msvc_atof(&v));
                    r = 6;
                }
                _ => r = 7,
            }
        }
    }

    /// GetTextExtent `0x7352C0`. `size_in` is the clip box (clip mode only);
    /// `px` selects atlas pixels (else world units).
    #[allow(clippy::too_many_arguments)]
    pub fn extent(&self, text: &str, size_in: (f32, f32), font: u32, px: bool, fixed: bool, trail: bool, wrap: f32, clip: bool) -> Measured {
        let text: Vec<u32> = text.encode_utf16().map(|u| u as u32).collect();
        self.extent_units(&text, size_in, font, px, fixed, trail, wrap, clip)
    }

    #[allow(clippy::too_many_arguments)]
    fn extent_units(&self, text: &[u32], size_in: (f32, f32), mut font: u32, px: bool, fixed: bool, trail: bool, wrap: f32, clip: bool) -> Measured {
        let (ow, oh) = (size_in.0 as f64, size_in.1 as f64);
        let wrap = wrap as f64;
        let mut sx = 0.0f64;
        let mut sy = 0.0f64;
        let mut rows = 0i32;
        let mut new_row = true;
        let mut pen = 0.0f64;
        let mut yacc = 0.0f64;
        let mut row_w = 0.0f64;
        let mut row_h = 0.0f64;
        let mut trailing = 0.0f64;
        let mut xstart = 0.0f64;
        let mut last_space: i64 = -1;
        let mut wrapped = false;
        let mut count = 0i32;
        let mut pos = 0usize;
        let mut in_markup = text.first() == Some(&('<' as u32));
        let delims = ['<' as u32, '>' as u32];
        let remap = [0x11u32, 0x12, 0x13, 0x14, 0x15];
        let remap_to = ['<' as u32, '>' as u32, '"' as u32, '\\' as u32, '&' as u32];
        let scale = self.env.world_scale as f64;
        let mut early = false;
        'outer: loop {
            let mut tok_start = pos as i64;
            let (tok, p) = next_token(text, pos, &delims, false);
            pos = p;
            if tok.is_empty() {
                break;
            }
            if in_markup {
                let mut st = MarkupState { font, pen, yacc };
                let r = self.markup(&tok, &mut st);
                font = st.font;
                pen = st.pen;
                yacc = st.yacc;
                if r == 5 {
                    row_w = pen;
                    xstart = pen;
                }
                in_markup = text.get(pos) == Some(&('<' as u32));
                continue;
            }
            in_markup = pos < text.len();
            let mut tok = tok;
            let mut i: i64 = 0;
            while (i as usize) < tok.len() {
                let mut ch = tok[i as usize];
                if let Some(k) = remap.iter().position(|&r| r == ch) {
                    ch = remap_to[k];
                }
                if font == 0 {
                    font = self.env.default_font;
                }
                let mut g = ch;
                if ch == 10 || ch == 13 || ch == 14 {
                    g = self.set_of(ch).map(|s| s.1).unwrap_or(self.env.standin);
                }
                if new_row {
                    rows += 1;
                    new_row = false;
                }
                if !self.has(font, g) {
                    g = self.set_of(g).map(|s| s.0).unwrap_or(self.env.undisplayable);
                    if !self.has(font, g) {
                        g = self.env.fallback;
                    }
                }
                let (adv, h) = if px {
                    let a = if fixed { self.table.fixed(font) as f64 } else { self.adv(font, g) as f64 };
                    (a, self.line_h(font) as f64)
                } else {
                    let a = if fixed {
                        f32r(self.r(self.table.fixed(font) as f64 / scale))
                    } else {
                        f32r(self.r(self.adv(font, g) as f64 / scale))
                    };
                    (a, self.r(self.line_h(font) as f64 / scale))
                };
                if ch == 10 {
                    if clip && wrap == 0.0 {
                        if pen > sx {
                            sx = pen;
                        }
                        early = true;
                        break 'outer;
                    }
                    if row_w > sx {
                        sx = row_w;
                    }
                    sy = f32r(self.r((if row_h == 0.0 { h } else { row_h }) + sy));
                    trailing = 0.0;
                    if !wrapped {
                        xstart = 0.0;
                    }
                    row_w = xstart;
                    pen = xstart;
                    wrapped = false;
                    new_row = true;
                    yacc = f32r(self.r(yacc + h));
                    if clip && self.r(yacc + row_h) > oh {
                        early = true;
                        break 'outer;
                    }
                    row_h = 0.0;
                    i += 1;
                    continue;
                }
                if ch == 32 {
                    if clip && self.r(adv + pen) > ow {
                        if pen > sx {
                            sx = pen;
                        }
                        if wrap == 0.0 || self.r(yacc + row_h) > oh {
                            early = true;
                            break 'outer;
                        }
                    }
                    pen = f32r(self.r(adv + pen));
                    count += 1;
                    last_space = i + tok_start;
                    trailing = f32r(self.r(adv + trailing));
                    i += 1;
                    continue;
                }
                if wrap != 0.0 && self.r(adv + pen) > wrap {
                    new_row = true;
                    if last_space == -1 {
                        if tok.len() > GROWTH_LIMIT {
                            return Measured::Hang;
                        }
                        tok.insert(i as usize, 10);
                        i -= 1;
                    } else {
                        let s = last_space - tok_start;
                        if s < 0 {
                            count -= i as i32;
                            tok.insert(0, 10);
                            tok_start -= 1;
                            i = -1;
                            wrapped = true;
                            last_space = -1;
                        } else {
                            tok[s as usize] = 10;
                            count += (s - i) as i32 + 1;
                            i = s - 1;
                            wrapped = true;
                            last_space = -1;
                        }
                    }
                    i += 1;
                    continue;
                }
                if clip && self.r(adv + pen) > ow {
                    if pen > sx {
                        sx = pen;
                    }
                    if wrap == 0.0 || self.r(yacc + row_h) > oh {
                        early = true;
                        break 'outer;
                    }
                }
                pen = f32r(self.r(adv + pen));
                count += 1;
                row_w = f32r(self.r(self.r(adv + trailing) + row_w));
                trailing = 0.0;
                if h > row_h {
                    row_h = f32r(h);
                }
                i += 1;
            }
        }
        if early {
            return Measured::Done { count, size: [sx as f32, sy as f32], rows: None };
        }
        if row_w > 0.0 && row_w > sx {
            sx = row_w;
        }
        sy = f32r(self.r(row_h + sy));
        if trail {
            sx = f32r(self.r(trailing + sx));
        }
        Measured::Done { count, size: [sx as f32, sy as f32], rows: Some(rows) }
    }

    /// `0x735C00`: wrap width in reference units (`* sx`), result divided by
    /// sx / sy.
    pub fn ref_extent(&self, text: &str, font: u32, fixed: bool, trail: bool, width: f32) -> Measured {
        let sxs = self.r(self.env.screen.0 as f64 / self.env.reference.0 as f64);
        let sys = self.r(self.env.screen.1 as f64 / self.env.reference.1 as f64);
        let w = f32r(self.r(sxs * width as f64));
        match self.extent(text, (0.0, 0.0), font, true, fixed, trail, w as f32, false) {
            Measured::Done { count, size, rows } => Measured::Done {
                count,
                size: [f32r(self.r(size[0] as f64 / sxs)) as f32, f32r(self.r(size[1] as f64 / sys)) as f32],
                rows,
            },
            Measured::Hang => Measured::Hang,
        }
    }

    /// `0x735D20`: world units, no rows.
    pub fn world_extent(&self, text: &str, font: u32, fixed: bool, trail: bool, wrap: f32) -> Measured {
        match self.extent(text, (0.0, 0.0), font, false, fixed, trail, wrap, false) {
            Measured::Done { count, size, .. } => Measured::Done { count, size, rows: None },
            Measured::Hang => Measured::Hang,
        }
    }

    /// `0x732C60`: characters outside markup tokens.
    pub fn visible_len(text: &str) -> usize {
        let t: Vec<u32> = text.encode_utf16().map(|u| u as u32).collect();
        let delims = ['<' as u32, '>' as u32];
        let (mut pos, mut n, mut in_m) = (0, 0, t.first() == Some(&('<' as u32)));
        loop {
            let (tok, p) = next_token(&t, pos, &delims, false);
            pos = p;
            if tok.is_empty() {
                return n;
            }
            if in_m {
                in_m = t.get(pos) == Some(&('<' as u32));
            } else {
                in_m = pos < t.len();
                n += tok.len();
            }
        }
    }

    /// `0x732E60(.., 1)`: the first `cnt` visible characters with the markup
    /// tokens before the cut.
    pub fn left(text: &str, cnt: usize) -> String {
        let t: Vec<u32> = text.encode_utf16().map(|u| u as u32).collect();
        let delims = ['<' as u32, '>' as u32];
        let mut out: Vec<u32> = Vec::new();
        let (mut acc, mut pos, mut in_m) = (0usize, 0usize, t.first() == Some(&('<' as u32)));
        let finish = |v: Vec<u32>| -> String {
            let u: Vec<u16> = v.iter().map(|&c| c as u16).collect();
            String::from_utf16_lossy(&u)
        };
        loop {
            let (tok, p) = next_token(&t, pos, &delims, false);
            pos = p;
            if tok.is_empty() {
                return finish(out);
            }
            if !in_m {
                in_m = pos < t.len();
                if cnt <= tok.len() + acc {
                    out.extend_from_slice(&tok[..cnt.saturating_sub(acc)]);
                    return finish(out);
                }
                out.extend_from_slice(&tok);
                acc += tok.len();
            } else {
                out.push('<' as u32);
                out.extend_from_slice(&tok);
                out.push('>' as u32);
                in_m = t.get(pos) == Some(&('<' as u32));
            }
        }
    }

    /// Ellipsis width of a font (`0x7080B0`; the original caches it in
    /// font+0x10/+0x14).
    pub fn ellipsis_size(&self, font: u32) -> [f32; 2] {
        match self.extent(&self.env.ellipsis, (0.0, 0.0), font, true, false, false, 0.0, false) {
            Measured::Done { size, .. } => size,
            Measured::Hang => [0.0, 0.0],
        }
    }

    /// `0x735D50`: (truncated?, text) fitting `w` x `h` pixels.
    pub fn ellipsis(&self, text: &str, w: f32, h: f32, font: u32, cache: &mut Option<[f32; 2]>) -> (bool, String) {
        let n = match self.extent(text, (w, h), font, true, false, false, 0.0, true) {
            Measured::Done { count, .. } => count,
            Measured::Hang => return (false, text.to_string()),
        };
        if n as usize == Self::visible_len(text) {
            return (false, text.to_string());
        }
        let e = match cache {
            Some(e) if e[0] != 0.0 => *e,
            _ => {
                let e = self.ellipsis_size(font);
                *cache = Some(e);
                e
            }
        };
        let w2 = self.r(w as f64 - e[0] as f64);
        if !(w2 > 0.0) {
            return (true, text.to_string());
        }
        let n2 = match self.extent(text, (w2 as f32, h), font, true, false, false, 0.0, true) {
            Measured::Done { count, .. } => count,
            Measured::Hang => 0,
        };
        (true, Self::left(text, n2.max(0) as usize) + &self.env.ellipsis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atof_prefix() {
        assert_eq!(msvc_atof(" 12.5x"), 12.5);
        assert_eq!(msvc_atof("abc"), 0.0);
        assert_eq!(msvc_atof("1e1"), 10.0);
        assert_eq!(msvc_atof("-3"), -3.0);
        assert_eq!(msvc_atof("2e"), 2.0);
    }
}
