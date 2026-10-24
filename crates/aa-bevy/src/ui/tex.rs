//! Original UI artwork as Bevy images.
//!
//! Paths arrive from aa-ui's draw list already resolved (`/UI/%s/...` ->
//! the resolution folder). Lookup order follows the egui client
//! (aa-client `ui_tex.rs`): the nation skin (`Skins/<NATION>/...`) first
//! when the draw list names one, then the original Data file, its converted
//! PNG copy (TGAs), and the 1280/800 resolution folders.
//!
//! Sampling is POINT with clamp addressing like the original UI samplers
//! (AA.exe 0x742200); `Address::Wrap` quads get repeat addressing. Quads
//! drawn without `alpha_blending` use the ALPHATEST variant: texels with
//! alpha 0 are discarded and the rest drawn opaque (0x742CC0 / 0x743750),
//! which a 0/255 alpha drawn with blending reproduces exactly.
use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Variant {
    /// Alpha reduced to 0/255 (non-blended UI sprites).
    pub alpha_test: bool,
    pub wrap_u: bool,
    pub wrap_v: bool,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Key {
    pub path: String,
    pub skin: Option<String>,
    pub variant: Variant,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub image: Handle<Image>,
    pub size: [u32; 2],
}

pub struct UiTextures {
    root: PathBuf,
    entries: HashMap<Key, Option<Entry>>,
    missing_logged: std::collections::HashSet<String>,
}

fn png_name(rel: &str) -> String {
    match rel.rsplit_once('.') {
        Some((stem, _)) => format!("{stem}.png"),
        None => format!("{rel}.png"),
    }
}

impl UiTextures {
    /// `root` is the reconstruction folder.
    pub fn new(root: &Path) -> Self {
        Self { root: root.to_path_buf(), entries: HashMap::new(), missing_logged: Default::default() }
    }

    fn candidates(root: &Path, path: &str, skin: Option<&str>) -> Vec<PathBuf> {
        let rel = path.trim_start_matches('/').replace('\\', "/");
        let mut out = Vec::new();
        if let Some(s) = skin {
            out.push(root.join("assets/original/Skins").join(s).join(&rel));
            out.push(root.join("assets/converted/skins").join(s).join(png_name(&rel)));
            out.push(root.join("assets/original/Skins").join(s).join(png_name(&rel)));
        }
        let png = png_name(&rel);
        // a PNG is read as shipped; a TGA / DDS from its converted copy
        // (the converted folder holds PNG conversions of TGAs that can share
        // a shipped PNG's name, e.g. SliderBar/GripUnpressed.tga/.png)
        if rel.to_ascii_lowercase().ends_with(".png") {
            out.push(root.join("assets/original/Data").join(&rel));
            out.push(root.join("assets/converted/textures").join(&png));
        } else {
            out.push(root.join("assets/converted/textures").join(&png));
            out.push(root.join("assets/original/Data").join(&rel));
            out.push(root.join("assets/original/Data").join(&png));
        }
        if let Some(r) = rel.strip_prefix("UI/") {
            for res in ["1280/Default", "800/Default"] {
                out.push(root.join("assets/converted/textures/UI").join(res).join(png_name(r)));
                out.push(root.join("assets/original/Data/UI").join(res).join(r));
            }
        }
        out
    }

    fn decode(root: &Path, key: &Key) -> Option<(u32, u32, Vec<u8>)> {
        // a skinned path only when the skin has the file (aa-client get_skinned)
        let mut lists = Vec::new();
        if key.skin.is_some() {
            lists.push(Self::candidates(root, &key.path, key.skin.as_deref()).into_iter().take(3).collect::<Vec<_>>());
        }
        lists.push(Self::candidates(root, &key.path, None));
        for list in lists {
            for c in list {
                if !c.is_file() {
                    continue;
                }
                let Ok(bytes) = std::fs::read(&c) else { continue };
                let Ok(img) = image::load_from_memory(&bytes) else { continue };
                let mut rgba = img.to_rgba8();
                if key.variant.alpha_test {
                    for p in rgba.pixels_mut() {
                        p[3] = if p[3] > 0 { 255 } else { 0 };
                    }
                }
                return Some((rgba.width(), rgba.height(), rgba.into_raw()));
            }
        }
        None
    }

    /// Decode every key not seen yet (in parallel) and upload it.
    pub fn prepare(&mut self, keys: impl IntoIterator<Item = Key>, images: &mut Assets<Image>) {
        let mut missing: Vec<Key> = keys.into_iter().filter(|k| !self.entries.contains_key(k)).collect();
        missing.sort_by(|a, b| a.path.cmp(&b.path));
        missing.dedup();
        if missing.is_empty() {
            return;
        }
        let root = self.root.clone();
        let t0 = std::time::Instant::now();
        let count = missing.len();
        let decoded: Vec<(Key, Option<(u32, u32, Vec<u8>)>)> =
            missing.into_par_iter().map(|k| {
                let d = Self::decode(&root, &k);
                (k, d)
            }).collect();
        for (key, d) in decoded {
            let entry = d.map(|(w, h, data)| {
                let image = images.add(make_image(w, h, data, key.variant, false));
                Entry { image, size: [w, h] }
            });
            if entry.is_none() && self.missing_logged.insert(key.path.clone()) {
                warn!("UI image not found: {}", key.path);
            }
            self.entries.insert(key, entry);
        }
        let ms = t0.elapsed().as_secs_f32() * 1000.0;
        if ms > 20.0 {
            info!("UI textures: {count} decoded in {ms:.0} ms");
        }
    }

    pub fn get(&self, key: &Key) -> Option<&Entry> {
        self.entries.get(key).and_then(|e| e.as_ref())
    }
}

pub fn sampler(variant: Variant) -> ImageSampler {
    let mode = |w: bool| if w { ImageAddressMode::Repeat } else { ImageAddressMode::ClampToEdge };
    ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: mode(variant.wrap_u),
        address_mode_v: mode(variant.wrap_v),
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        mipmap_filter: ImageFilterMode::Nearest,
        ..default()
    })
}

/// `dynamic` keeps the pixels in the main world so they can be rewritten
/// (glyph atlas pages, video frames).
pub fn make_image(w: u32, h: u32, data: Vec<u8>, variant: Variant, dynamic: bool) -> Image {
    let mut image = Image::new(
        Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        if dynamic { RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD } else { RenderAssetUsages::RENDER_WORLD },
    );
    image.sampler = sampler(variant);
    image
}
