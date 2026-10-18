//! The UI font atlas metrics (`KKC_RFont`, SetupFont `0x708B30`, AddGlyphs
//! `0x7072F0`) as a [`GlyphTable`] for the text measure (`rtext.rs`).
//!
//! * TrueType faces (`*.TTF`, GDI+ private collection, `GdipCreateFont(px,
//!   UnitPoint)`, `GdipMeasureString` with GenericTypographic | 0x800):
//!   cell = (ceil(adv * em / upem), ceil((hhea asc - desc + gap) * em /
//!   upem)) with em = px * 96 / 72 in f32 steps. The research subagent
//!   checked this formula against live GDI+ for every character 32..383 at
//!   8..48 px (one difference: U+00AD). Every code point has a cell (no
//!   coverage check in the GDI+ path).
//! * Other faces (Arial: GDI `CreateFontW(-px ...)`,
//!   `GetTextExtentPoint32W`): hinted cells baked from GDI on the build
//!   machine (research/phase8-ui/gdi_metrics.py ->
//!   crates/aa-ui/data/gdi-metrics.json, 4..64 px, code points up to
//!   0x24F); only cmap-covered characters have cells.
//! * Padding +2 with outline, else +1 with shadow; the shadow bit makes the
//!   advance one pixel shorter (`0x7051A0`).
//! * Special glyphs (`special_glyphs*.tgi`, index 128..159): an image
//!   `Fonts/<font ids>/<screen width>/<display name without spaces>.png`,
//!   else `Fonts/<font ids>/<name>.png`, is the cell (image size, no
//!   padding, the font's shadow bit kept). Without an image the character
//!   is measured as a normal glyph.

use crate::rtext::{Glyph, GlyphTable};
use crate::text::{FontDef, FontFace, TtfMetrics};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

/// A special glyph definition (`Glyph` template, loader `0x493500`).
#[derive(Clone, Debug, PartialEq)]
pub struct SpecialGlyph {
    pub ids: String,
    pub display_name: String,
    pub index: u32,
    /// Default true: the image keeps its own colours (vertex colour white).
    pub use_own_color: bool,
}

/// An image glyph of one font.
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphImage {
    pub path: String,
    pub w: u16,
    pub h: u16,
    pub use_own_color: bool,
}

#[derive(Debug)]
struct GdiFace {
    cy: u16,
    widest: u16,
    cx: HashMap<u32, u16>,
}

fn gdi_table() -> &'static HashMap<(bool, i32), GdiFace> {
    static T: OnceLock<HashMap<(bool, i32), GdiFace>> = OnceLock::new();
    T.get_or_init(|| {
        let v: serde_json::Value = serde_json::from_str(include_str!("../data/gdi-metrics.json")).unwrap_or_default();
        let mut out = HashMap::new();
        for f in v["faces"].as_array().into_iter().flatten() {
            let bold = f["bold"].as_bool().unwrap_or(false);
            let px = f["px"].as_i64().unwrap_or(0) as i32;
            let cx = f["cx"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|p| Some((p[0].as_u64()? as u32, p[1].as_u64()? as u16)))
                .collect();
            out.insert(
                (bold, px),
                GdiFace { cy: f["cy"].as_u64().unwrap_or(0) as u16, widest: f["widest"].as_u64().unwrap_or(0) as u16, cx },
            );
        }
        out
    })
}

#[derive(Debug)]
enum Kind {
    GdiPlus(Arc<TtfMetrics>),
    Gdi { bold: bool, px: i32 },
    Missing,
}

/// One built font.
#[derive(Debug)]
pub struct UiFont {
    pub ids: String,
    pub face: FontFace,
    kind: Kind,
    pub images: HashMap<u32, GlyphImage>,
    fixed: i32,
}

impl UiFont {
    fn pad(&self) -> u16 {
        if self.face.outline {
            2
        } else if self.face.shadow {
            1
        } else {
            0
        }
    }

    fn cell(&self, c: u32) -> Option<Glyph> {
        let shadow = self.face.shadow;
        let outline = self.face.outline;
        if let Some(img) = self.images.get(&c) {
            return Some(Glyph { w: img.w, h: img.h, shadow, outline });
        }
        let pad = self.pad();
        match &self.kind {
            Kind::GdiPlus(m) => {
                if c > 0xFFFA {
                    return None;
                }
                let ch = char::from_u32(c)?;
                let em = (self.face.px as f32 * 96.0f32) / 72.0f32;
                let upem = m.units_per_em as f32;
                let w = ((m.advance_units(ch) as f32 * em) / upem).ceil() as u16;
                let h = (((m.hhea_ascent as f32 - m.hhea_descent as f32 + m.hhea_gap as f32) * em) / upem).ceil() as u16;
                Some(Glyph { w: w + pad, h: h + pad, shadow, outline })
            }
            Kind::Gdi { bold, px } => {
                let f = gdi_table().get(&(*bold, *px))?;
                let w = *f.cx.get(&c)?;
                Some(Glyph { w: w + pad, h: f.cy + pad, shadow, outline })
            }
            Kind::Missing => None,
        }
    }
}

/// The fonts of one screen size, by id (index + 1).
#[derive(Debug, Default)]
pub struct UiFonts {
    pub fonts: Vec<UiFont>,
    by_ids: HashMap<String, u32>,
    /// The global special glyph map (`[0xB36530]+0x3BC`): index ->
    /// use_own_color (first definition of an index).
    own: HashMap<u32, bool>,
    pub diagnostics: Vec<String>,
}

impl UiFonts {
    /// Build every font of the font TGIs for `screen` (`0x708B30`).
    pub fn build(
        defs: &HashMap<String, FontDef>,
        specials: &[SpecialGlyph],
        screen: (i32, i32),
        interface_height: f32,
        root: &crate::fs::DataRoot,
        ttf: &mut dyn FnMut(&FontFace) -> Option<Arc<TtfMetrics>>,
    ) -> Self {
        let mut out = UiFonts::default();
        for g in specials {
            out.own.entry(g.index).or_insert(g.use_own_color);
        }
        let mut names: Vec<&String> = defs.keys().collect();
        names.sort();
        for ids in names {
            let def = &defs[ids];
            let Some(face) = def.face_for(screen.1, interface_height) else { continue };
            let kind = if face.gdiplus {
                match ttf(&face) {
                    Some(m) => Kind::GdiPlus(m),
                    None => {
                        out.diagnostics.push(format!("font {ids}: {} not readable", face.face));
                        Kind::Missing
                    }
                }
            } else if face.face.eq_ignore_ascii_case("arial") {
                if gdi_table().contains_key(&(face.bold, face.px)) {
                    Kind::Gdi { bold: face.bold, px: face.px }
                } else {
                    out.diagnostics.push(format!("font {ids}: no GDI metrics for Arial {} px", face.px));
                    Kind::Missing
                }
            } else {
                out.diagnostics.push(format!("font {ids}: GDI face {} has no baked metrics", face.face));
                Kind::Missing
            };
            // special glyph images (AddGlyphs checks them before the font)
            let mut images = HashMap::new();
            for g in specials {
                let name: String = g.display_name.chars().filter(|c| *c != ' ').collect();
                for path in [format!("/Fonts/{}/{}/{}.png", def.ids, screen.0, name), format!("/Fonts/{}/{}.png", def.ids, name)] {
                    if let Some(canon) = root.canonical(&path) {
                        if let Some((w, h)) = root.read(&canon).ok().and_then(|b| crate::fs::image_size(&b)) {
                            images.insert(g.index, GlyphImage { path: canon, w: w as u16, h: h as u16, use_own_color: g.use_own_color });
                            break;
                        }
                    }
                }
            }
            let mut font = UiFont { ids: def.ids.clone(), face, kind, images, fixed: 0 };
            // font+0xC: widest padded cell + 2 (0x707833, 0x708F66)
            let widest = match &font.kind {
                Kind::GdiPlus(m) => {
                    let em = (font.face.px as f32 * 96.0f32) / 72.0f32;
                    ((m.max_advance() as f32 * em) / m.units_per_em as f32).ceil() as i32 + font.pad() as i32
                }
                Kind::Gdi { bold, px } => gdi_table().get(&(*bold, *px)).map(|f| f.widest as i32 + font.pad() as i32).unwrap_or(0),
                Kind::Missing => 0,
            };
            let img_w = font.images.values().map(|i| i.w as i32).max().unwrap_or(0);
            font.fixed = widest.max(img_w) + 2;
            out.by_ids.insert(def.ids.to_ascii_lowercase(), out.fonts.len() as u32 + 1);
            out.fonts.push(font);
        }
        out
    }

    pub fn id(&self, ids: &str) -> u32 {
        self.by_ids.get(&ids.to_ascii_lowercase()).copied().unwrap_or(0)
    }

    pub fn font(&self, id: u32) -> Option<&UiFont> {
        self.fonts.get((id as usize).checked_sub(1)?)
    }
}

impl GlyphTable for UiFonts {
    fn glyph(&self, font: u32, c: u32) -> Option<Glyph> {
        self.font(font)?.cell(c)
    }
    fn fixed(&self, font: u32) -> i32 {
        self.font(font).map(|f| f.fixed).unwrap_or(0)
    }
    fn font_by_name(&self, name: &str) -> u32 {
        self.id(name)
    }
    fn own_color(&self, c: u32) -> bool {
        self.own.get(&c).copied().unwrap_or(false)
    }
}
