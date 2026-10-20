//! The UI font atlas (`KKC_RFont`, SetupFont 0x708B30, AddGlyphs 0x7072F0)
//! rebuilt with a TrueType rasteriser.
//!
//! aa-ui lays out every label with the native-proven rebuild (glyph quads
//! with their cell offsets, `aa_ui::rdraw`); this module only supplies the
//! cell pixels. A cell is drawn like the original atlas: the glyph pen at
//! the cell's top-left (GDI+ `DrawString` / GDI `TextOut` with top
//! alignment: baseline at the ascent), white with antialiased coverage, and
//! the one-pixel drop shadow at (+1, +1) in `shadow_brightness` grey with
//! `shadow_opacity` (padding +1), or an outline around the glyph (padding
//! +2, glyph at (1, 1)). The quad modulates the cell with its vertex
//! colour.
//!
//! Faces: `.ttf` data files through GDI+ (em = px * 96 / 72; A&A.ttf is a
//! bold-flagged face, so GDI+ synthesises nothing), other faces are GDI
//! system fonts from %WINDIR%\Fonts (em = px; Arial / Arial Bold).
//! Behaviour-faithful: the rasteriser is not GDI's, so hinting and
//! antialiasing differ slightly.
use super::tex::{Variant, make_image};
use ab_glyph::{Font, FontArc, PxScale, point};
use aa_ui::text::FontFace;
use bevy::prelude::*;
use std::collections::HashMap;

pub const PAGE: u32 = 1024;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct CellKey {
    face: String,
    px: i32,
    gdiplus: bool,
    bold: bool,
    italic: bool,
    outline: bool,
    shadow: bool,
    opacity: u8,
    brightness: u8,
    c: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub page: usize,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

struct Page {
    image: Handle<Image>,
    pixels: Vec<u8>,
    x: u32,
    y: u32,
    row: u32,
    dirty: bool,
}

pub struct GlyphAtlas {
    data: aa_ui::fs::DataRoot,
    fonts: HashMap<(String, bool, bool), Option<FontArc>>,
    cells: HashMap<CellKey, Option<Cell>>,
    pages: Vec<Page>,
}

impl GlyphAtlas {
    pub fn new(data: aa_ui::fs::DataRoot) -> Self {
        Self { data, fonts: HashMap::new(), cells: HashMap::new(), pages: Vec::new() }
    }

    pub fn page_image(&self, page: usize) -> Option<Handle<Image>> {
        self.pages.get(page).map(|p| p.image.clone())
    }

    fn font(&mut self, face: &FontFace) -> Option<FontArc> {
        let key = (face.face.to_ascii_lowercase(), face.bold, face.italic);
        if let Some(f) = self.fonts.get(&key) {
            return f.clone();
        }
        let bytes = if face.gdiplus {
            self.data.read(face.face.trim_start_matches('/')).ok()
        } else {
            let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
            let dir = std::path::Path::new(&windir).join("Fonts");
            let base = face.face.to_ascii_lowercase().replace(' ', "");
            // GDI face name -> file (the faces the font TGIs name)
            let stem = match base.as_str() {
                "arial" => "arial",
                "gulimche" => "gulim",
                "mingliu" => "mingliu",
                "msgothic" => "msgothic",
                other => other,
            };
            let suffix = match (face.bold, face.italic) {
                (true, true) => "bi",
                (true, false) => "bd",
                (false, true) => "i",
                _ => "",
            };
            [format!("{stem}{suffix}.ttf"), format!("{stem}.ttf"), format!("{stem}.ttc"), "arial.ttf".to_string()]
                .iter()
                .find_map(|n| std::fs::read(dir.join(n)).ok())
        };
        let font = bytes.and_then(|b| FontArc::try_from_vec(b).ok());
        if font.is_none() {
            warn!("UI font face not found: {}", face.face);
        }
        self.fonts.insert(key, font.clone());
        font
    }

    /// The atlas cell of a glyph, rendered on first use.
    pub fn cell(&mut self, face: &FontFace, c: u32, images: &mut Assets<Image>) -> Option<Cell> {
        let key = CellKey {
            face: face.face.to_ascii_lowercase(),
            px: face.px,
            gdiplus: face.gdiplus,
            bold: face.bold,
            italic: face.italic,
            outline: face.outline,
            shadow: face.shadow,
            opacity: (face.shadow_opacity * 255.0) as u8,
            brightness: (face.shadow_brightness * 255.0) as u8,
            c,
        };
        if let Some(cell) = self.cells.get(&key) {
            return *cell;
        }
        let rendered = self.font(face).and_then(|f| render(&f, face, c));
        let cell = rendered.map(|(w, h, rgba)| self.place(w, h, &rgba, images));
        self.cells.insert(key, cell);
        cell
    }

    fn place(&mut self, w: u32, h: u32, rgba: &[u8], images: &mut Assets<Image>) -> Cell {
        let (w, h) = (w.min(PAGE), h.min(PAGE));
        let fits = |p: &Page| {
            let (x, y) = if p.x + w + 1 > PAGE { (0, p.y + p.row + 1) } else { (p.x, p.y) };
            y + h + 1 <= PAGE && x + w + 1 <= PAGE
        };
        if !self.pages.last().is_some_and(fits) {
            let pixels = vec![0u8; (PAGE * PAGE * 4) as usize];
            let image = images.add(make_image(PAGE, PAGE, pixels.clone(), Variant::default(), true));
            self.pages.push(Page { image, pixels, x: 0, y: 0, row: 0, dirty: false });
        }
        let index = self.pages.len() - 1;
        let p = self.pages.last_mut().unwrap();
        if p.x + w + 1 > PAGE {
            p.x = 0;
            p.y += p.row + 1;
            p.row = 0;
        }
        let (x, y) = (p.x, p.y);
        for row in 0..h {
            let src = (row * w * 4) as usize;
            let dst = (((y + row) * PAGE + x) * 4) as usize;
            p.pixels[dst..dst + (w * 4) as usize].copy_from_slice(&rgba[src..src + (w * 4) as usize]);
        }
        p.x += w + 1;
        p.row = p.row.max(h);
        p.dirty = true;
        Cell { page: index, x, y, w, h }
    }

    /// Upload the pages that received glyphs this frame.
    pub fn flush(&mut self, images: &mut Assets<Image>) {
        for p in self.pages.iter_mut().filter(|p| p.dirty) {
            if let Some(mut img) = images.get_mut(&p.image) {
                img.data = Some(p.pixels.clone());
            }
            p.dirty = false;
        }
    }
}

/// Rasterise one atlas cell: (width, height, RGBA).
fn render(font: &FontArc, face: &FontFace, c: u32) -> Option<(u32, u32, Vec<u8>)> {
    let ch = char::from_u32(c)?;
    let upem = font.units_per_em().unwrap_or(1000.0);
    let em = if face.gdiplus { (face.px as f32 * 96.0) / 72.0 } else { face.px as f32 };
    let k = em / upem;
    let id = font.glyph_id(ch);
    let advance = font.h_advance_unscaled(id) * k;
    let pad = if face.outline { 2 } else if face.shadow { 1 } else { 0 };
    let ascent = font.ascent_unscaled() * k;
    let line = (font.ascent_unscaled() - font.descent_unscaled() + font.line_gap_unscaled()) * k;
    // the cell plus room for ink past the advance (cut by the quad anyway)
    let w = (advance.ceil() as u32 + pad + 4).max(1);
    let h = (line.ceil() as u32 + pad + 2).max(1);
    let mut cov = vec![0f32; (w * h) as usize];
    let origin = if face.outline { 1.0 } else { 0.0 };
    let scale = PxScale::from(em * font.height_unscaled() / upem);
    let glyph = id.with_scale_and_position(scale, point(origin, origin + ascent));
    if let Some(outlined) = font.outline_glyph(glyph) {
        let b = outlined.px_bounds();
        let (bx, by) = (b.min.x as i32, b.min.y as i32);
        outlined.draw(|x, y, v| {
            let (px, py) = (bx + x as i32, by + y as i32);
            if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                let i = (py as u32 * w + px as u32) as usize;
                cov[i] = (cov[i] + v).min(1.0);
            }
        });
    }
    let at = |x: i32, y: i32| -> f32 {
        if x < 0 || y < 0 || x as u32 >= w || y as u32 >= h { 0.0 } else { cov[(y as u32 * w + x as u32) as usize] }
    };
    let grey = face.shadow_brightness.clamp(0.0, 1.0);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let g = at(x, y);
            let s = if face.outline {
                let mut m = 0f32;
                for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                    m = m.max(at(x + dx, y + dy));
                }
                m * face.shadow_opacity
            } else if face.shadow {
                at(x - 1, y - 1) * face.shadow_opacity
            } else {
                0.0
            };
            // glyph (white) over the shadow / outline
            let a = g + s * (1.0 - g);
            let rgb = if a > 0.0 { (g + grey * s * (1.0 - g)) / a } else { 0.0 };
            let i = ((y as u32 * w + x as u32) * 4) as usize;
            let v = (rgb * 255.0 + 0.5) as u8;
            rgba[i] = v;
            rgba[i + 1] = v;
            rgba[i + 2] = v;
            rgba[i + 3] = (a * 255.0 + 0.5) as u8;
        }
    }
    Some((w, h, rgba))
}
