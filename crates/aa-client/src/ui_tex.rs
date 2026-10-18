//! Original UI artwork (icons, panels, menu art) as egui textures.
//! Original PNGs are read from `assets/original/Data`; TGAs come from the
//! converted PNG copies under `assets/converted/textures`.
//!
//! Nation skins (`Skins/<NATION>/...`, the original `depot_name` overlay)
//! are tried first by [`UiTextures::get_skinned`]; aa-ui resolves the
//! resolution folders (`/UI/%s/...`) before a path reaches this cache.

use eframe::egui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// How a texture is prepared for drawing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Variant {
    Plain,
    /// Desaturated and darkened (original disabled buttons).
    Gray,
    /// Point-sampled with clamp addressing, as the original UI samplers
    /// (AA.exe 0x742200); used for the TGI-driven front end.
    Nearest,
    /// [`Variant::Nearest`] with alpha reduced to 0/255: the original draws
    /// non-blended UI sprites with ALPHATEST > 0 and no blending.
    NearestAlphaTest,
}

pub struct UiTextures {
    root: PathBuf,
    cache: HashMap<String, Option<egui::TextureHandle>>,
}

impl UiTextures {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_owned(),
            cache: HashMap::new(),
        }
    }

    fn png_name(rel: &str) -> String {
        match rel.rsplit_once('.') {
            Some((stem, _)) => format!("{stem}.png"),
            None => format!("{rel}.png"),
        }
    }

    fn candidates(&self, path: &str) -> Vec<PathBuf> {
        let rel = path.trim_start_matches('/').replace('\\', "/");
        // Explicit skin paths: "skin:USA/UI/1280/Game/..." (original Skins/<NATION>/...).
        if let Some(rest) = rel.strip_prefix("skin:") {
            return vec![
                self.root.join("assets/original/Skins").join(rest),
                self.root.join("assets/converted/skins").join(Self::png_name(rest)),
                self.root.join("assets/original/Skins").join(Self::png_name(rest)),
            ];
        }
        let original = self.root.join("assets/original/Data").join(&rel);
        let png = Self::png_name(&rel);
        let mut out = vec![original];
        out.push(self.root.join("assets/converted/textures").join(&png));
        out.push(self.root.join("assets/original/Data").join(&png));
        // UI art also exists in resolution-specific folders.
        for res in ["1280/Default", "800/Default"] {
            if let Some(r) = rel.strip_prefix("UI/") {
                out.push(self.root.join("assets/converted/textures/UI").join(res).join(Self::png_name(r)));
                out.push(self.root.join("assets/original/Data/UI").join(res).join(r));
            }
        }
        out
    }

    fn exists(&self, path: &str) -> bool {
        self.candidates(path)
            .iter()
            .any(|c| c.is_file() && c.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
    }

    fn load(&self, ctx: &egui::Context, path: &str, variant: Variant) -> Option<egui::TextureHandle> {
        for candidate in self.candidates(path) {
            if !candidate.is_file() {
                continue;
            }
            // TGAs are read from their converted PNG copies.
            if !candidate.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) {
                continue;
            }
            let Ok(bytes) = std::fs::read(&candidate) else { continue };
            let Ok(img) = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png) else { continue };
            let mut rgba = img.to_rgba8();
            if variant == Variant::NearestAlphaTest {
                for p in rgba.pixels_mut() {
                    p[3] = if p[3] > 0 { 255 } else { 0 };
                }
            }
            if variant == Variant::Gray {
                for p in rgba.pixels_mut() {
                    let l = (0.30 * p[0] as f32 + 0.59 * p[1] as f32 + 0.11 * p[2] as f32) * 0.55;
                    let l = l.min(255.0) as u8;
                    p[0] = l;
                    p[1] = l;
                    p[2] = l;
                }
            }
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            let options = match variant {
                Variant::Nearest | Variant::NearestAlphaTest => egui::TextureOptions {
                    wrap_mode: egui::TextureWrapMode::ClampToEdge,
                    ..egui::TextureOptions::NEAREST
                },
                _ => egui::TextureOptions::LINEAR,
            };
            let name = Self::key(path, variant);
            return Some(ctx.load_texture(name, color, options));
        }
        None
    }

    fn key(path: &str, variant: Variant) -> String {
        match variant {
            Variant::Plain => path.to_owned(),
            Variant::Gray => format!("gray:{path}"),
            Variant::Nearest => format!("nearest:{path}"),
            Variant::NearestAlphaTest => format!("nearest-at:{path}"),
        }
    }

    /// A data path as resolved by aa-ui (`%s` already substituted), from the
    /// nation skin first when one is active (`Skins/<NATION>/...`).
    pub fn get_skinned(&mut self, ctx: &egui::Context, path: &str, skin: Option<&str>, variant: Variant) -> Option<egui::TextureHandle> {
        if let Some(s) = skin {
            let skinned = format!("skin:{s}/{}", path.trim_start_matches('/'));
            if self.exists(&skinned) {
                return self.get_variant(ctx, &skinned, variant);
            }
        }
        self.get_variant(ctx, path, variant)
    }

    /// Load (once) an original UI image by its resource path, e.g.
    /// "/UI/Game/RegimentButtons/IconInfantry.png".
    pub fn get(&mut self, ctx: &egui::Context, path: &str) -> Option<egui::TextureHandle> {
        self.get_variant(ctx, path, Variant::Plain)
    }

    /// A prepared variant (gray / point-sampled) of an image.
    pub fn get_variant(&mut self, ctx: &egui::Context, path: &str, variant: Variant) -> Option<egui::TextureHandle> {
        if path.is_empty() {
            return None;
        }
        let key = Self::key(path, variant);
        if let Some(t) = self.cache.get(&key) {
            return t.clone();
        }
        let found = self.load(ctx, path, variant);
        if found.is_none() {
            log::debug!("UI image not found: {path}");
        }
        self.cache.insert(key, found.clone());
        found
    }
}
