//! UI text: font definitions, the native pixel-size rule and TrueType
//! tables.
//!
//! Native sources (AA.exe):
//! * `KKC_FontData::LoadData` 0x493B20: `size`, `[CharacterSet]` entries
//!   (font, backup_font, backup_font_size, use_backup_font_when_smaller_than,
//!   flags antialias=1 bold=2 italic=4 outline=8 shadow=0x10,
//!   shadow_opacity/brightness = `_ftol(v * 255.0f)`).
//! * `KKC_RFont::SetupFont` 0x708B30 (0x708C03..0x708C99): pixel size
//!   `floor(size * screen_h / InterfacePositioningHeight + 0.5)`; if the
//!   character set has `use_backup_font_when_smaller_than` and the scaled size
//!   is below that (unscaled) value, the backup face and the scaled backup
//!   size are used. Rebuilt on every resolution change (`RebuildFonts`
//!   0x708F90 from `KKC_UI::Reskin` 0x5C3DC0).
//! * Face creation: a `.ttf` face goes through GDI+ `GdipCreateFont(emSize =
//!   px, UnitPoint)` (0x7089C4) - at the 96 dpi of a non-DPI-aware process the
//!   em is `px * 96 / 72` pixels; other faces use GDI `CreateFontW(-px, ...)`
//!   (em = px).
//! * Glyph cells (`AddGlyphs` 0x7072F0): measured width/height rounded up,
//!   +2 with outline else +1 with shadow; advance = cell width - 1 with shadow
//!   (0x7051A0); line height = cell height of 'A' (0x705580).
//! * Label placement and glyph quads: `rdraw.rs` (native-proven rebuild
//!   0x735B90); measuring: `rtext.rs`; atlas cells: `fonts.rs` (GDI+ cells
//!   from the TrueType tables, GDI cells from baked hinted metrics).

use serde::{Deserialize, Serialize};

/// One `[CharacterSet]` of a font (only the `base` set is used for Latin).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct CharacterSet {
    pub ids: String,
    pub font: String,
    pub backup_font: Option<String>,
    pub backup_font_size: i32,
    pub use_backup_font_when_smaller_than: i32,
    pub antialias: bool,
    pub bold: bool,
    pub italic: bool,
    pub outline: bool,
    pub shadow: bool,
    /// `_ftol(shadow_opacity * 255.0f)`.
    pub shadow_opacity255: i32,
    /// `_ftol(shadow_brightness * 255.0f)`.
    pub shadow_brightness255: i32,
}

/// A `[Font]` definition from `Data/Fonts/font_*.tgi`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct FontDef {
    pub ids: String,
    pub size: i32,
    pub sets: Vec<CharacterSet>,
}

/// The face chosen for a screen height.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FontFace {
    /// Font file (`/fonts/truetype/A&A.TTF`) or system face name (`Arial`).
    pub face: String,
    /// Scaled pixel size passed to CreateFont / GdipCreateFont.
    pub px: i32,
    /// Em height in screen pixels (GDI+ UnitPoint at 96 dpi: px*4/3).
    pub em_px: f32,
    /// True when the face is a TrueType file loaded through GDI+.
    pub gdiplus: bool,
    pub bold: bool,
    pub italic: bool,
    pub outline: bool,
    pub shadow: bool,
    pub shadow_opacity: f32,
    pub shadow_brightness: f32,
}

/// `0x708C03..0x708C99`: `floor(size * screen_h / iph + 0.5)` with x87
/// `fidivr`; for `iph` = 768 every fraction is a multiple of 1/768, so the
/// precision setting cannot change the result.
pub fn scaled_font_px(size: i32, screen_h: i32, interface_height: f32) -> i32 {
    ((size as f64 * screen_h as f64) / interface_height as f64 + 0.5).floor() as i32
}

pub fn ftol255(v: f32) -> i32 {
    (v * 255.0f32) as i32
}

impl FontDef {
    /// `SetupFont` face/size choice for the `base` character set.
    pub fn face_for(&self, screen_h: i32, interface_height: f32) -> Option<FontFace> {
        let set = self.sets.iter().find(|s| s.ids.eq_ignore_ascii_case("base")).or(self.sets.first())?;
        let mut px = scaled_font_px(self.size, screen_h, interface_height);
        let mut face = set.font.clone();
        if set.use_backup_font_when_smaller_than != 0 && px < set.use_backup_font_when_smaller_than {
            if let Some(b) = &set.backup_font {
                face = b.clone();
                px = scaled_font_px(set.backup_font_size, screen_h, interface_height);
            }
        }
        let gdiplus = face.to_ascii_lowercase().ends_with(".ttf");
        Some(FontFace {
            em_px: if gdiplus { px as f32 * 96.0 / 72.0 } else { px as f32 },
            face,
            px,
            gdiplus,
            bold: set.bold,
            italic: set.italic,
            outline: set.outline,
            shadow: set.shadow,
            shadow_opacity: set.shadow_opacity255 as f32 / 255.0,
            shadow_brightness: set.shadow_brightness255 as f32 / 255.0,
        })
    }
}

/// Unhinted TrueType metrics of a face file.
#[derive(Clone, Debug)]
pub struct TtfMetrics {
    pub units_per_em: u16,
    pub ascent: i16,
    pub descent: i16,
    /// `hhea` ascender, descender (negative) and line gap (the GDI+ cell
    /// height is their (asc - desc + gap) * em / upem).
    pub hhea_ascent: i16,
    pub hhea_descent: i16,
    pub hhea_gap: i16,
    advances: Vec<u16>,
    cmap: Vec<(u16, u16, i16, u16, usize)>,
    bytes: Vec<u8>,
}

fn be16(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(o..o + 2)?.try_into().ok()?))
}
fn be32(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

impl TtfMetrics {
    pub fn parse(bytes: Vec<u8>) -> Option<Self> {
        let n = be16(&bytes, 4)? as usize;
        let mut tables = std::collections::HashMap::new();
        for i in 0..n {
            let at = 12 + 16 * i;
            let tag = bytes.get(at..at + 4)?.to_vec();
            tables.insert(tag, (be32(&bytes, at + 8)? as usize, be32(&bytes, at + 12)? as usize));
        }
        let head = tables.get(b"head".as_slice())?.0;
        let units_per_em = be16(&bytes, head + 18)?;
        let hhea = tables.get(b"hhea".as_slice())?.0;
        let hhea_ascent = be16(&bytes, hhea + 4)? as i16;
        let hhea_descent = be16(&bytes, hhea + 6)? as i16;
        let hhea_gap = be16(&bytes, hhea + 8)? as i16;
        let mut ascent = hhea_ascent;
        let mut descent = -hhea_descent;
        let nhm = be16(&bytes, hhea + 34)? as usize;
        if let Some(&(os2, len)) = tables.get(b"OS/2".as_slice()) {
            if len >= 78 {
                // GDI/GDI+ cell metrics use usWinAscent/usWinDescent.
                ascent = be16(&bytes, os2 + 74)? as i16;
                descent = be16(&bytes, os2 + 76)? as i16;
            }
        }
        let hmtx = tables.get(b"hmtx".as_slice())?.0;
        let advances = (0..nhm).map(|g| be16(&bytes, hmtx + 4 * g).unwrap_or(0)).collect();
        let cmap_at = tables.get(b"cmap".as_slice())?.0;
        let ntab = be16(&bytes, cmap_at + 2)? as usize;
        let mut sub = None;
        for i in 0..ntab {
            let pid = be16(&bytes, cmap_at + 4 + 8 * i)?;
            let eid = be16(&bytes, cmap_at + 6 + 8 * i)?;
            let off = be32(&bytes, cmap_at + 8 + 8 * i)? as usize;
            if pid == 3 && (eid == 1 || eid == 0) {
                sub = Some(cmap_at + off);
            }
        }
        let sub = sub?;
        let mut cmap = Vec::new();
        if be16(&bytes, sub)? == 4 {
            let seg = be16(&bytes, sub + 6)? as usize / 2;
            for i in 0..seg {
                let end = be16(&bytes, sub + 14 + 2 * i)?;
                let start = be16(&bytes, sub + 16 + 2 * seg + 2 * i)?;
                let delta = be16(&bytes, sub + 16 + 4 * seg + 2 * i)? as i16;
                let roff_pos = sub + 16 + 6 * seg + 2 * i;
                let roff = be16(&bytes, roff_pos)?;
                cmap.push((start, end, delta, roff, roff_pos));
            }
        }
        Some(Self { units_per_em, ascent, descent, hhea_ascent, hhea_descent, hhea_gap, advances, cmap, bytes })
    }

    fn glyph(&self, c: char) -> u16 {
        let c = c as u32;
        if c > 0xffff {
            return 0;
        }
        let c = c as u16;
        for &(start, end, delta, roff, roff_pos) in &self.cmap {
            if start <= c && c <= end {
                if roff == 0 {
                    return (c as i32 + delta as i32) as u16;
                }
                let p = roff_pos + roff as usize + 2 * (c - start) as usize;
                let g = be16(&self.bytes, p).unwrap_or(0);
                return if g == 0 { 0 } else { (g as i32 + delta as i32) as u16 };
            }
        }
        0
    }

    /// The widest advance of any glyph (font units).
    pub fn max_advance(&self) -> u16 {
        self.advances.iter().copied().max().unwrap_or(0)
    }

    pub fn has_glyph(&self, c: char) -> bool {
        self.glyph(c) != 0
    }

    /// Advance in font units.
    pub fn advance_units(&self, c: char) -> u16 {
        let g = self.glyph(c) as usize;
        if self.advances.is_empty() {
            return 0;
        }
        self.advances[g.min(self.advances.len() - 1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn font_px_rule() {
        assert_eq!(scaled_font_px(16, 768, 768.0), 16);
        assert_eq!(scaled_font_px(16, 1440, 768.0), 30);
        assert_eq!(scaled_font_px(12, 1024, 768.0), 16);
        assert_eq!(scaled_font_px(16, 600, 768.0), 13);
        assert_eq!(ftol255(0.4), 102);
        assert_eq!(ftol255(0.8), 204);
    }
}

/// Colour markup around `text` (`0x731980` over the whole string):
/// `"%s%cColor=%g,%g,%g%c%s%cRC%c%s"` with the font system markup
/// characters '<' / '>' (font system +0x1C / +0x1E).
pub fn color_wrap(text: &str, rgb: [f32; 3]) -> String {
    let g = |v: f32| crate::staging::fmt_g(v as f64);
    format!("<Color={},{},{}>{}<RC>", g(rgb[0]), g(rgb[1]), g(rgb[2]), text)
}
