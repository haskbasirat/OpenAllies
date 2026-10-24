//! Original textures (as the converted PNGs the wgpu client also reads),
//! decoded in parallel with a box-filtered mip chain.
use aa_assets::AssetStore;
use anyhow::{Context, Result};
use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rayon::prelude::*;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::RwLock;

/// The current biome texture subfolder: the original's ResourceManager+80,
/// copied from the map's Biome texture_subfolder_name at world init
/// (43F0EE/5AE0A0). Every texture lookup tries `<folder>/<subfolder>/<file>`
/// before the named file (5A77E0), so Arctic, Desert and Tropical maps show
/// their texture variants.
static SUBFOLDER: RwLock<String> = RwLock::new(String::new());

/// Set at battle load, before any of the battle's textures are decoded.
pub fn set_biome_subfolder(subfolder: Option<&str>) {
    if let Ok(mut current) = SUBFOLDER.write() {
        *current = subfolder.unwrap_or_default().to_owned();
    }
}

/// The biome variant of `url`, when the current subfolder has one.
fn biome_variant(store: &AssetStore, url: &str) -> Option<PathBuf> {
    let subfolder = SUBFOLDER.read().ok()?;
    if subfolder.is_empty() {
        return None;
    }
    let (folder, file) = url.rsplit_once('/')?;
    store.resolve_url(&format!("{folder}/{subfolder}/{file}")).ok()
}

/// How a texture is sampled and what its alpha means.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Kind {
    /// Diffuse colour, repeating, alpha kept.
    Color,
    /// Diffuse colour whose alpha is the team-colour mask: the team colour is
    /// mixed in where alpha is low (scene.wgsl `mix(team, texel, texel.a)`).
    Team([u8; 3]),
    /// Terrain transition mask: linear, clamped, only alpha is read.
    Mask,
}

#[derive(Default)]
pub struct Requests(BTreeSet<(String, Kind)>);
impl Requests {
    pub fn add(&mut self, url: &str, kind: Kind) {
        self.0.insert((url.to_owned(), kind));
    }
}

pub struct Loaded {
    pub handles: HashMap<(String, Kind), Handle<Image>>,
    pub failures: Vec<String>,
}

pub fn decode(store: &AssetStore, requests: Requests) -> (Vec<((String, Kind), Image)>, Vec<String>) {
    let results: Vec<_> = requests
        .0
        .into_par_iter()
        .map(|(url, kind)| {
            let image = load(store, &url, kind).with_context(|| format!("texture {url}"));
            ((url, kind), image)
        })
        .collect();
    let mut images = Vec::new();
    let mut failures = Vec::new();
    for (key, result) in results {
        match result {
            Ok(image) => images.push((key, image)),
            Err(e) => failures.push(format!("{e:#}")),
        }
    }
    (images, failures)
}

pub fn upload(images: Vec<((String, Kind), Image)>, failures: Vec<String>, assets: &mut Assets<Image>) -> Loaded {
    let handles = images.into_iter().map(|(key, image)| (key, assets.add(image))).collect();
    Loaded { handles, failures }
}

pub fn load(store: &AssetStore, url: &str, kind: Kind) -> Result<Image> {
    let path = match biome_variant(store, url) {
        Some(path) => path,
        None => store.resolve_url(url)?,
    };
    let mut rgba = image::open(&path)?.to_rgba8();
    if let Kind::Team(team) = kind {
        for p in rgba.pixels_mut() {
            let a = p[3] as f32 / 255.0;
            for c in 0..3 {
                p[c] = (team[c] as f32 * (1.0 - a) + p[c] as f32 * a).round() as u8;
            }
            p[3] = 255;
        }
    }
    let (width, height) = rgba.dimensions();
    let mut data = rgba.into_raw();
    let mut levels = 1;
    let (mut w, mut h) = (width, height);
    let mut level = data.clone();
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    let mut sum = 0u32;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let sx = (x * 2 + dx).min(w - 1);
                        let sy = (y * 2 + dy).min(h - 1);
                        sum += level[((sy * w + sx) * 4 + c) as usize] as u32;
                    }
                    next[((y * nw + x) * 4 + c) as usize] = ((sum + 2) / 4) as u8;
                }
            }
        }
        data.extend_from_slice(&next);
        level = next;
        (w, h) = (nw, nh);
        levels += 1;
    }
    let format = if kind == Kind::Mask { TextureFormat::Rgba8Unorm } else { TextureFormat::Rgba8UnormSrgb };
    let mut image = Image::new(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data[..(width * height * 4) as usize].to_vec(),
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(data);
    image.texture_descriptor.mip_level_count = levels;
    let address = ImageAddressMode::Repeat;
    let filter = if kind == Kind::Mask { ImageFilterMode::Nearest } else { ImageFilterMode::Linear };
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: address,
        address_mode_v: address,
        address_mode_w: address,
        mag_filter: filter,
        min_filter: filter,
        mipmap_filter: filter,
        // Masks: native compositor point sampling, no mip filtering (8A0830(0)).
        lod_max_clamp: if kind == Kind::Mask { 0.0 } else { 32.0 },
        anisotropy_clamp: if kind == Kind::Mask { 1 } else { 4 },
        ..default()
    });
    Ok(image)
}
