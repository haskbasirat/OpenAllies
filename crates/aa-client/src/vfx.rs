//! Original visual effects (Data/Effects/*.tgi): muzzle flashes, impacts,
//! explosions, smoke, burning buildings. Contract used by the game screen:
//! - `VfxLibrary::load(root)` once per match (background thread is fine).
//! - `VfxLibrary::required_models()` -> converted model paths to preload into
//!   the dynamic layer (same keys as `game::view::ModelTable::by_key`, lowercase).
//! - `VfxSystem` lives in the GameScreen; spawn effects from world events,
//!   `update(dt)` each frame, then `model_instances` + `particles` are rendered.
//!
//! Effect definitions come from the original TGI tables (`[VFX Template=...]`
//! blocks resolved through `Data/Templates/template_vfx*.tgi`). Model effects
//! are played from the original NIFs: `vfx/nif_effects.json` holds their
//! node keyframes, billboards, sprite quads with alpha/colour/flip
//! controllers and NiPSys emitters/modifiers (extracted by
//! `vfx/extract_nif_effects.py`), simulated by `vfx/player.rs` and drawn as
//! camera-facing particles and quads by `gpu::particles` with the original
//! effect textures.
#![allow(dead_code)]

mod embedded_textures;
mod nif;
mod player;
mod tgi;

use crate::gpu::dynamic::{DynInstance, ModelHandle};
use crate::gpu::particles::{MODE_BILLBOARD, MODE_QUAD, MODE_STREAK, ParticleInstance};
use glam::{Quat, Vec3};
use nif::{NifEffect, NifTable};
use player::{Player, Rng};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, OnceLock};

/// Rendered particle/quad cap per frame.
pub const MAX_PARTICLES: usize = 8000;
/// Simultaneous NIF effect instances.
pub const MAX_EFFECTS: usize = 900;
const MAX_TRACERS: usize = 700;
const MAX_CHUNKS: usize = 900;
const MAX_DECALS: usize = 160;

// ---------------------------------------------------------------------------
// Effect texture atlas (shared with gpu::particles)

#[derive(Clone, Debug)]
pub enum AtlasSource {
    /// Relative to `assets/converted/textures`.
    Converted(String),
    /// Relative to `assets/original/Data` (PNG/JPG originals).
    Original(String),
    /// Texture stored inside an effect NIF (decoded by the extractor).
    Embedded(&'static [u8]),
    Procedural(Procedural),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Procedural {
    White,
    Streak,
    Glow,
}

/// Additional original textures used by composed effects (decals, debris).
const EXTRA_TEXTURES: &[&str] = &[
    "Effects/ScorchMarks/ScorchMarkS01.png",
    "Effects/ScorchMarks/ScorchMarkM01.png",
    "Effects/ScorchMarks/ScorchMarkL02.png",
    "Effects/ScorchMarks/ScorchMarkXL01.png",
    "Effects/SpecialOps/AtomicBomb/AtomicScorch.png",
    "Effects/CrushedDoodad/TemperateRocks.png",
    "Effects/CrushedDoodad/ArcticSnowChunks.png",
    "Shared/SharedTextures/Dust.png",
    "Shared/SharedTextures/DirtSmoke.png",
    "Shared/SharedTextures/TankDust.png",
    "Shared/SharedTextures/FIRE-HitSparks.png",
];

fn nif_table() -> &'static NifTable {
    static TABLE: OnceLock<NifTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        match serde_json::from_str::<NifTable>(include_str!("vfx/nif_effects.json")) {
            Ok(t) => t,
            Err(e) => {
                log::error!("Original effect table unreadable: {e}");
                NifTable {
                    format: String::new(),
                    textures: Vec::new(),
                    effects: HashMap::new(),
                    failures: HashMap::new(),
                }
            }
        }
    })
}

struct Atlas {
    sources: Vec<AtlasSource>,
    index: HashMap<String, u16>,
}

fn atlas() -> &'static Atlas {
    static ATLAS: OnceLock<Atlas> = OnceLock::new();
    ATLAS.get_or_init(|| {
        let mut sources = Vec::new();
        let mut index = HashMap::new();
        for key in &nif_table().textures {
            let source = if let Some(name) = key.strip_prefix("embedded:") {
                embedded_textures::EMBEDDED
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map_or(AtlasSource::Procedural(Procedural::White), |(_, b)| {
                        AtlasSource::Embedded(b)
                    })
            } else if let Some(rel) = key.strip_prefix("original:") {
                AtlasSource::Original(rel.to_owned())
            } else {
                AtlasSource::Converted(key.clone())
            };
            index.insert(key.to_ascii_lowercase(), sources.len() as u16);
            sources.push(source);
        }
        for rel in EXTRA_TEXTURES {
            let key = rel.to_ascii_lowercase();
            if !index.contains_key(&key) {
                index.insert(key, sources.len() as u16);
                sources.push(AtlasSource::Converted((*rel).to_owned()));
            }
        }
        for (name, p) in [
            ("proc:white", Procedural::White),
            ("proc:streak", Procedural::Streak),
            ("proc:glow", Procedural::Glow),
        ] {
            index.insert(name.to_owned(), sources.len() as u16);
            sources.push(AtlasSource::Procedural(p));
        }
        Atlas { sources, index }
    })
}

/// Ordered texture layers of the particle texture array.
pub fn atlas_sources() -> &'static [AtlasSource] {
    &atlas().sources
}

/// Atlas layer of a texture (converted-relative path, `proc:*`), any case.
pub fn atlas_layer(key: &str) -> Option<u16> {
    let key = key.replace('\\', "/").to_ascii_lowercase();
    let a = atlas();
    a.index.get(&key).copied().or_else(|| {
        // TGI texture names are relative to Data/Effects and may name .dds/.tga
        let stem = key.rsplit_once('.').map_or(key.as_str(), |(s, _)| s);
        let png = format!("{stem}.png");
        a.index
            .get(&png)
            .or_else(|| a.index.get(&format!("effects/{png}")))
            .copied()
    })
}

fn layer(key: &str) -> f32 {
    atlas_layer(key).or_else(|| atlas_layer("proc:white")).unwrap_or(0) as f32
}

// ---------------------------------------------------------------------------
// Definitions

#[derive(Clone, Debug)]
pub enum VfxKind {
    /// NIF model effect (`VFXModelTemporary` / `VFXModelPermanent`).
    Model {
        /// Lowercase path relative to `Data/Effects` (or to `Data` when the
        /// original value started with '/').
        model: String,
        permanent: bool,
        /// Seconds; 0 = use the NIF's own controller length.
        duration: f32,
        gyroscope: bool,
        random_orientation: bool,
    },
    Tracer {
        color: [f32; 3],
        speed: f32,
        length: f32,
        spread: [f32; 3],
        match_orientation: bool,
    },
    /// Projected ground texture (`VFXProjectedTexture`, scorch marks).
    Decal {
        texture: String,
        size: f32,
        duration: f32,
        fade: [f32; 2],
        color: [f32; 3],
        random_orientation: bool,
        permanent: bool,
        additive: bool,
    },
    Light {
        color: [f32; 3],
        radius: f32,
    },
    Text,
    Other(String),
}

#[derive(Clone, Debug)]
pub struct VfxDef {
    pub ids: String,
    pub template: String,
    pub source: String,
    pub kind: VfxKind,
}

#[derive(Default)]
pub struct VfxLibrary {
    pub defs: HashMap<String, VfxDef>,
    /// Lowercase converted model resource keys (`effects/...nif`) that exist.
    converted: HashSet<String>,
    pub files: usize,
    pub warnings: Vec<String>,
}

/// Model effects drawn through the dynamic model layer (real meshes).
const MODEL_EFFECTS: &[&str] = &[
    "armored_car_debris",
    "heavy_tank_debris",
    "light_tank_debris",
    "medium_tank_debris",
];

struct Template {
    parent: Option<String>,
    values: Vec<(String, String)>,
}

fn flag_values(block: &tgi::Block, out: &mut Vec<(String, String)>) {
    out.extend(block.values.iter().cloned());
    if let Some(flags) = block.child("flags") {
        out.extend(flags.values.iter().cloned());
    }
}

impl VfxLibrary {
    pub fn load(root: &Path) -> Self {
        let data = root.join("assets/original/Data");
        let mut lib = Self::default();
        let mut files: Vec<std::path::PathBuf> = Vec::new();
        for dir in ["Templates", "Effects", "Projectiles"] {
            collect_tgi(&data.join(dir), &mut files, dir != "Templates");
        }
        files.retain(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_ascii_lowercase();
            !p.parent()
                .and_then(|d| d.file_name())
                .is_some_and(|d| d.eq_ignore_ascii_case("Templates"))
                || name.starts_with("template_vfx")
        });
        let mut templates: HashMap<String, Template> = HashMap::new();
        let mut defs: Vec<(tgi::Block, String)> = Vec::new();
        for path in &files {
            let Ok(bytes) = std::fs::read(path) else {
                lib.warnings.push(format!("unreadable {}", path.display()));
                continue;
            };
            let text: String = bytes.iter().map(|&b| b as char).collect();
            lib.files += 1;
            let source = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_owned();
            let mut stack: Vec<tgi::Block> = tgi::parse(&text);
            while let Some(block) = stack.pop() {
                if block.kind == "template" {
                    if let Some(name) = block.words.first() {
                        let mut values = Vec::new();
                        flag_values(&block, &mut values);
                        templates.insert(
                            name.to_ascii_lowercase(),
                            Template {
                                parent: block.parent().map(|p| p.to_ascii_lowercase()),
                                values,
                            },
                        );
                    }
                } else if block.get("ids").is_some() && block.parent().is_some() {
                    defs.push((block, source.clone()));
                    continue;
                }
                // nested definitions (e.g. projectile files) are searched too
                if block.kind != "template" {
                    stack.extend(block.children.into_iter());
                }
            }
        }
        for (block, source) in defs {
            let Some(ids) = block.get("ids").map(|s| s.to_ascii_lowercase()) else { continue };
            let template = block.parent().unwrap_or("").to_ascii_lowercase();
            // template chain (nearest first) and merged values (root first)
            let mut chain = vec![template.clone()];
            let mut cursor = templates.get(&template);
            while let Some(t) = cursor {
                match &t.parent {
                    Some(p) if !chain.contains(p) && chain.len() < 16 => {
                        chain.push(p.clone());
                        cursor = templates.get(p);
                    }
                    _ => break,
                }
            }
            let mut values: HashMap<String, String> = HashMap::new();
            for name in chain.iter().rev() {
                if let Some(t) = templates.get(name) {
                    for (k, v) in &t.values {
                        values.insert(k.clone(), v.clone());
                    }
                }
            }
            let mut own = Vec::new();
            flag_values(&block, &mut own);
            for (k, v) in own {
                values.insert(k, v);
            }
            let kind = classify(&chain, &values);
            lib.defs.insert(
                ids.clone(),
                VfxDef {
                    ids,
                    template,
                    source,
                    kind,
                },
            );
        }
        // converted model availability (for meshes drawn by the model layer)
        #[derive(serde::Deserialize)]
        struct Manifest {
            #[serde(rename = "byPath", default)]
            by_path: HashMap<String, String>,
        }
        if let Ok(text) = std::fs::read_to_string(root.join("assets/converted/models/manifest.json"))
            && let Ok(m) = serde_json::from_str::<Manifest>(&text)
        {
            lib.converted = m
                .by_path
                .into_keys()
                .filter(|k| k.starts_with("effects/"))
                .collect();
        }
        log::info!(
            "Original VFX: {} definitions from {} files, {} NIF effects",
            lib.defs.len(),
            lib.files,
            nif_table().effects.len()
        );
        lib
    }

    pub fn get(&self, ids: &str) -> Option<&VfxDef> {
        self.defs.get(&ids.to_ascii_lowercase())
    }

    /// The original NIF animation/particle data of a model effect.
    pub fn nif(&self, ids: &str) -> Option<&'static NifEffect> {
        match &self.get(ids)?.kind {
            VfxKind::Model { model, .. } => nif_table().effects.get(model),
            _ => None,
        }
    }

    fn model_key(&self, ids: &str) -> Option<String> {
        match &self.get(ids)?.kind {
            VfxKind::Model { model, .. } => {
                let key = format!("effects/{model}");
                self.converted.contains(&key).then_some(key)
            }
            _ => None,
        }
    }

    pub fn required_models(&self) -> Vec<String> {
        MODEL_EFFECTS
            .iter()
            .filter_map(|ids| self.model_key(ids))
            .collect()
    }
}

fn collect_tgi(dir: &Path, out: &mut Vec<std::path::PathBuf>, recursive: bool) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            if recursive {
                collect_tgi(&p, out, recursive);
            }
        } else if p
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("tgi"))
        {
            out.push(p);
        }
    }
}

fn classify(chain: &[String], v: &HashMap<String, String>) -> VfxKind {
    let has = |name: &str| chain.iter().any(|c| c == name);
    let f = |k: &str, d: f32| {
        v.get(k)
            .and_then(|s| tgi::floats(s).first().copied())
            .unwrap_or(d)
    };
    let f3 = |k: &str, d: [f32; 3]| {
        let x = v.get(k).map(|s| tgi::floats(s)).unwrap_or_default();
        if x.len() >= 3 { [x[0], x[1], x[2]] } else { d }
    };
    let b = |k: &str, d: bool| v.get(k).and_then(|s| tgi::boolean(s)).unwrap_or(d);
    let any = |needle: &str| chain.iter().any(|c| c.contains(needle));
    if has("vfxtracer") || any("tracer") {
        return VfxKind::Tracer {
            color: f3("color", [0.7, 0.7, 0.4]),
            speed: f("speed", 100.0),
            length: f("length", 2.0),
            spread: f3("spread", [0.0; 3]),
            match_orientation: b("match_orientation", false),
        };
    }
    if has("vfxprojectedtexture") || any("projectedtexture") || any("scorch") || any("groundplane") {
        let fade = v.get("decal_fade_time").map(|s| tgi::floats(s)).unwrap_or_default();
        return VfxKind::Decal {
            texture: v.get("texture").cloned().unwrap_or_default(),
            size: f("texture_size", 2.0),
            duration: f("duration", 0.0),
            fade: [fade.first().copied().unwrap_or(0.0), fade.get(1).copied().unwrap_or(0.0)],
            color: f3("color", [1.0; 3]),
            random_orientation: b("random_orientation", false),
            permanent: b("permanent", true),
            additive: b("additive", false),
        };
    }
    if has("vfxlight") {
        return VfxKind::Light {
            color: f3("diffuse", [1.0; 3]),
            radius: f("visual_radius", 1.0),
        };
    }
    if has("vfxtext") || has("vfxobjectname") {
        return VfxKind::Text;
    }
    if let Some(model) = v.get("model").filter(|m| !m.is_empty()) {
        let raw = model.replace('\\', "/");
        let model = raw.trim_start_matches('/').to_ascii_lowercase();
        let permanent = if has("vfxmodelpermanent") || any("permanent") || has("vfxstatusbar") {
            true
        } else if has("vfxmodeltemporary") || any("temporary") {
            false
        } else {
            b("permanent", false)
        };
        return VfxKind::Model {
            model,
            permanent,
            duration: f("duration", 0.0),
            gyroscope: b("gyroscope", true),
            random_orientation: b("random_orientation", false),
        };
    }
    VfxKind::Other(chain.first().cloned().unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Runtime

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Rifle,
    MachineGun,
    HeavyMg,
    AntiAir,
    Aircraft,
    Cannon,
    Artillery,
    Mortar,
    Rocket,
    Flame,
}

fn classify_shot(muzzle: Option<&str>, impact: Option<&str>) -> Class {
    let m = muzzle.unwrap_or("").to_ascii_lowercase();
    let i = impact.unwrap_or("").to_ascii_lowercase();
    let either = |s: &str| m.contains(s) || i.contains(s);
    if m.starts_with("mg_heavy") {
        Class::HeavyMg
    } else if m.starts_with("mg_") || m.contains("tommy") {
        Class::MachineGun
    } else if m.starts_with("rifle") {
        Class::Rifle
    } else if m.starts_with("aa_shot") || i.starts_with("flak") {
        Class::AntiAir
    } else if m.starts_with("aircraft") {
        Class::Aircraft
    } else if m.starts_with("tank_shot") || i.starts_with("tank_shot") {
        Class::Cannon
    } else if m.contains("mortar") || i.starts_with("mortar") {
        Class::Mortar
    } else if m.contains("artillery") || m.contains("battleship") || i.contains("large_explosion") {
        Class::Artillery
    } else if m.contains("rpg") || m.contains("bazooka") || m.contains("rocket") {
        Class::Rocket
    } else if either("flame") {
        Class::Flame
    } else if i.starts_with("mg_") {
        Class::MachineGun
    } else if i.contains("explosion_1") {
        Class::Artillery
    } else {
        Class::Rifle
    }
}

struct Effect {
    player: Player,
    /// Small, frequent effects (muzzle flashes, hit puffs) are dropped first.
    minor: bool,
}

struct Tracer {
    from: Vec3,
    dir: Vec3,
    dist: f32,
    speed: f32,
    length: f32,
    width: f32,
    color: [f32; 4],
    t: f32,
}

struct Chunk {
    pos: Vec3,
    vel: Vec3,
    floor: f32,
    age: f32,
    life: f32,
    size: f32,
    angle: f32,
    spin: f32,
    layer: f32,
    color: [f32; 4],
    drag: f32,
    gravity: f32,
    rise: f32,
}

struct Decal {
    pos: Vec3,
    size: f32,
    angle: f32,
    layer: f32,
    color: [f32; 3],
    additive: bool,
    age: f32,
    duration: f32,
    fade: [f32; 2],
}

struct Delayed {
    at: f32,
    ids: String,
    pos: Vec3,
    toward: Option<Vec3>,
    scale: f32,
    lifetime: Option<f32>,
}

struct Pending {
    ids: String,
    to: Vec3,
    deadline: f32,
    scale: f32,
}

struct ModelFx {
    key: String,
    pos: Vec3,
    yaw: f32,
    scale: f32,
    age: f32,
    life: f32,
}

pub struct VfxSystem {
    lib: Arc<VfxLibrary>,
    effects: Vec<Effect>,
    tracers: Vec<Tracer>,
    chunks: Vec<Chunk>,
    decals: Vec<Decal>,
    delayed: Vec<Delayed>,
    pending: Vec<Pending>,
    /// Recently played explosions (position, radius, time) for de-duplication.
    recent: Vec<(Vec3, f32, f32)>,
    models: Vec<ModelFx>,
    time: f32,
    rng: Rng,
    seed: u32,
    eye: Option<Vec3>,
    focus: Option<Vec3>,
    climate: &'static str,
    white: u16,
    streak_layer: f32,
    rocks_layer: f32,
    dust_layer: f32,
    spark_layer: f32,
}

impl VfxSystem {
    pub fn new(lib: Arc<VfxLibrary>) -> Self {
        Self {
            lib,
            effects: Vec::new(),
            tracers: Vec::new(),
            chunks: Vec::new(),
            decals: Vec::new(),
            delayed: Vec::new(),
            pending: Vec::new(),
            recent: Vec::new(),
            models: Vec::new(),
            time: 0.0,
            rng: Rng::new(0x5EED),
            seed: 1,
            eye: None,
            focus: None,
            climate: "temperate",
            white: atlas_layer("proc:white").unwrap_or(0),
            streak_layer: layer("proc:streak"),
            rocks_layer: layer("effects/crusheddoodad/temperaterocks.png"),
            dust_layer: layer("shared/sharedtextures/dirtsmoke.png"),
            spark_layer: layer("shared/sharedtextures/fire-hitsparks.png"),
        }
    }

    /// Optional: camera eye/target (render space) for back-to-front sorting
    /// and for skipping minor effects far from the view.
    pub fn set_camera(&mut self, eye: [f32; 3], target: [f32; 3]) {
        self.eye = Some(Vec3::from(eye));
        self.focus = Some(Vec3::from(target));
    }

    /// Map climate for the original explosion variants: "temperate",
    /// "desert" or "arctic" (also accepts "snow"/"winter").
    pub fn set_climate(&mut self, climate: &str) {
        let c = climate.to_ascii_lowercase();
        self.climate = if c.contains("desert") || c.contains("sand") {
            "desert"
        } else if c.contains("arctic") || c.contains("snow") || c.contains("winter") {
            "arctic"
        } else {
            "temperate"
        };
        self.rocks_layer = if self.climate == "arctic" {
            layer("effects/crusheddoodad/arcticsnowchunks.png")
        } else {
            layer("effects/crusheddoodad/temperaterocks.png")
        };
    }

    pub fn active_effects(&self) -> usize {
        self.effects.len()
    }

    fn next_seed(&mut self) -> u32 {
        self.seed = self.seed.wrapping_add(0x9E37_79B9);
        self.seed
    }

    fn far(&self, at: Vec3) -> bool {
        self.focus.is_some_and(|f| {
            let d = at - f;
            d.x * d.x + d.z * d.z > 260.0 * 260.0
        })
    }

    /// Spawn an original VFX by IDS at a render-space position, optionally
    /// oriented toward `toward` (render space).
    pub fn spawn(&mut self, ids: &str, at: [f32; 3], toward: Option<[f32; 3]>, scale: f32) {
        self.spawn_effect(ids, Vec3::from(at), toward.map(Vec3::from), scale, None, false);
    }

    /// Returns false when the id has no playable effect.
    fn spawn_effect(
        &mut self,
        ids: &str,
        at: Vec3,
        toward: Option<Vec3>,
        scale: f32,
        lifetime: Option<f32>,
        minor: bool,
    ) -> bool {
        let lib = self.lib.clone();
        let Some(def) = lib.get(ids) else { return false };
        match &def.kind {
            VfxKind::Model {
                model,
                permanent,
                duration,
                gyroscope,
                random_orientation,
            } => {
                let Some(fx) = nif_table().effects.get(model) else {
                    return false;
                };
                if self.effects.len() >= MAX_EFFECTS {
                    if minor {
                        return false;
                    }
                    if let Some(i) = self.effects.iter().position(|e| e.minor) {
                        self.effects.swap_remove(i);
                    } else {
                        return false;
                    }
                }
                let rotation = match toward {
                    Some(t) if !*gyroscope || minor => {
                        let d = (t - at).normalize_or_zero();
                        if d == Vec3::ZERO {
                            Quat::IDENTITY
                        } else {
                            let yaw = d.x.atan2(d.z);
                            let pitch = -d.y.clamp(-1.0, 1.0).asin();
                            Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch)
                        }
                    }
                    Some(t) => {
                        let d = t - at;
                        Quat::from_rotation_y(d.x.atan2(d.z))
                    }
                    None if *random_orientation => {
                        Quat::from_rotation_y(self.rng.next() * std::f32::consts::TAU)
                    }
                    None => Quat::IDENTITY,
                };
                let emit_until = match lifetime {
                    Some(l) => l,
                    None if *duration > 0.0 => *duration,
                    None if *permanent => 30.0,
                    None => fx.active_span().max(0.1),
                };
                let seed = self.next_seed();
                let player = Player::new(fx, at, rotation, scale.max(0.05), emit_until, seed, self.white);
                self.effects.push(Effect { player, minor });
                true
            }
            VfxKind::Tracer {
                color,
                speed,
                length,
                ..
            } => {
                if let Some(t) = toward {
                    self.tracer(at, t, *color, *speed, *length, 0.05 * scale);
                }
                true
            }
            VfxKind::Decal {
                texture,
                size,
                duration,
                fade,
                color,
                random_orientation,
                additive,
                ..
            } => {
                let Some(l) = atlas_layer(texture) else { return false };
                if self.decals.len() >= MAX_DECALS {
                    self.decals.remove(0);
                }
                self.decals.push(Decal {
                    pos: at + Vec3::Y * 0.06,
                    size: size * scale,
                    angle: if *random_orientation {
                        self.rng.next() * std::f32::consts::TAU
                    } else {
                        0.0
                    },
                    layer: l as f32,
                    color: *color,
                    additive: *additive,
                    age: 0.0,
                    duration: if *duration > 0.0 { *duration } else { 15.0 },
                    fade: *fade,
                });
                true
            }
            _ => false,
        }
    }

    fn tracer(&mut self, from: Vec3, to: Vec3, color: [f32; 3], speed: f32, length: f32, width: f32) {
        if self.tracers.len() >= MAX_TRACERS {
            self.tracers.remove(0);
        }
        let d = to - from;
        let dist = d.length();
        if dist < 0.5 {
            return;
        }
        // The original tracer colour is dim (.7,.7,.4); additive brightening.
        let c = [
            (color[0] * 1.4).min(1.0),
            (color[1] * 1.35).min(1.0),
            (color[2] * 1.3).min(1.0),
            0.9,
        ];
        self.tracers.push(Tracer {
            from,
            dir: d / dist,
            dist,
            speed: speed.max(10.0),
            length: length.max(0.2),
            width,
            color: c,
            t: 0.0,
        });
    }

    /// Generic explosion of a given radius (metres) at a render position.
    pub fn explosion(&mut self, at: [f32; 3], radius: f32) {
        let at = Vec3::from(at);
        let radius = radius.max(0.5);
        // Already played (building destruction or a projectile impact).
        let now = self.time;
        if self
            .recent
            .iter()
            .any(|(p, r, t)| now - t < 1.5 && xz_dist(*p, at) < r + radius + 3.0)
        {
            return;
        }
        let pending = self
            .pending
            .iter()
            .enumerate()
            .filter(|(_, p)| xz_dist(p.to, at) < radius + 6.0)
            .min_by(|a, b| xz_dist(a.1.to, at).total_cmp(&xz_dist(b.1.to, at)))
            .map(|(i, _)| i);
        let ids = match pending {
            Some(i) => self.pending.swap_remove(i).ids,
            None => self.explosion_ids(radius),
        };
        self.play_explosion(&ids, at, radius);
    }

    fn explosion_ids(&self, radius: f32) -> String {
        let size = if radius < 2.5 {
            "small"
        } else if radius < 7.0 {
            "medium"
        } else {
            "large"
        };
        format!("{}_{}_explosion_1", self.climate, size)
    }

    /// Original explosion NIF plus scorch mark and thrown debris.
    fn play_explosion(&mut self, ids: &str, at: Vec3, radius: f32) {
        let ids = if self.lib.nif(ids).is_some() {
            ids.to_owned()
        } else {
            self.explosion_ids(radius)
        };
        let nominal = if ids.contains("small") {
            2.0
        } else if ids.contains("large") {
            7.0
        } else {
            4.0
        };
        let scale = (radius / nominal).clamp(0.75, 1.8);
        self.spawn_effect(&ids, at, None, scale, None, false);
        self.recent.push((at, radius, self.time));
        // scorch mark (original projectile WhenDestroyed decals)
        let scorch = if radius < 2.0 {
            "scorch_mark_small_vfx"
        } else if radius < 5.0 {
            "scorch_mark_medium_vfx"
        } else {
            "scorch_mark_large_vfx"
        };
        self.spawn_effect(scorch, at, None, (radius / 3.0).clamp(0.6, 1.6), None, false);
        self.debris(at, radius, ((radius * 4.0) as usize).clamp(6, 28));
    }

    /// Thrown dirt clods / stones (original crushed-doodad rock sprites).
    fn debris(&mut self, at: Vec3, radius: f32, count: usize) {
        for _ in 0..count {
            if self.chunks.len() >= MAX_CHUNKS {
                self.chunks.remove(0);
            }
            let a = self.rng.next() * std::f32::consts::TAU;
            let out = (2.0 + radius * 1.2) * (0.4 + self.rng.next());
            let up = (5.0 + radius * 1.6) * (0.6 + self.rng.next() * 0.8);
            let shade = 0.35 + self.rng.next() * 0.25;
            self.chunks.push(Chunk {
                pos: at + Vec3::new(a.cos(), 0.0, a.sin()) * self.rng.next() * radius * 0.3 + Vec3::Y * 0.3,
                vel: Vec3::new(a.cos() * out, up, a.sin() * out),
                floor: at.y,
                age: 0.0,
                life: 1.2 + self.rng.next() * 1.4,
                size: 0.25 + self.rng.next() * 0.35 * (radius / 3.0).clamp(0.7, 2.0),
                angle: self.rng.next() * 6.28,
                spin: self.rng.sym() * 6.0,
                layer: self.rocks_layer,
                color: [shade * 1.1, shade, shade * 0.85, 1.0],
                drag: 0.4,
                gravity: 14.0,
                rise: 0.0,
            });
        }
        // low dust ring hugging the ground
        let puffs = ((radius * 1.5) as usize).clamp(3, 10);
        for i in 0..puffs {
            if self.chunks.len() >= MAX_CHUNKS {
                break;
            }
            let a = i as f32 / puffs as f32 * std::f32::consts::TAU + self.rng.next();
            let speed = 1.5 + radius * 0.8;
            self.chunks.push(Chunk {
                pos: at + Vec3::Y * 0.6,
                vel: Vec3::new(a.cos() * speed, 0.3, a.sin() * speed),
                floor: at.y + 0.3,
                age: 0.0,
                life: 2.0 + self.rng.next() * 1.5,
                size: 0.8 + radius * 0.35,
                angle: self.rng.next() * 6.28,
                spin: self.rng.sym() * 0.4,
                layer: self.dust_layer,
                color: [0.55, 0.5, 0.42, 0.55],
                drag: 1.6,
                gravity: 0.0,
                rise: 0.35,
            });
        }
    }

    /// Weapon fired: muzzle effect at `from`, impact at `to` (render space).
    pub fn shot(&mut self, vfx: Option<&str>, impact: Option<&str>, from: [f32; 3], to: [f32; 3]) {
        let from = Vec3::from(from);
        let to = Vec3::from(to);
        let d = to - from;
        let dist = d.length();
        let dir = if dist > 1e-3 { d / dist } else { Vec3::Z };
        let class = classify_shot(vfx, impact);
        let far = self.far(from) && self.far(to);
        let muzzle_default = match class {
            Class::Rifle => "rifle_shot",
            Class::MachineGun => "mg_shot",
            Class::HeavyMg => "mg_heavy_shot",
            Class::AntiAir => "aa_shot1",
            Class::Aircraft => "aircraft_shot",
            Class::Cannon => "tank_shot",
            Class::Artillery => "artillery_shot",
            Class::Mortar => "mortar_shot",
            Class::Rocket => "rpg_shot",
            Class::Flame => "",
        };
        let muzzle = vfx.filter(|v| self.lib.get(v).is_some()).unwrap_or(muzzle_default);
        let barrel = match class {
            Class::Rifle | Class::MachineGun => 0.55,
            Class::HeavyMg => 0.8,
            Class::AntiAir | Class::Aircraft => 1.0,
            Class::Cannon => 2.4,
            Class::Artillery => 2.0,
            Class::Mortar | Class::Rocket => 0.6,
            Class::Flame => 0.6,
        };
        let horizontal = Vec3::new(dir.x, 0.0, dir.z).normalize_or(Vec3::Z);
        let muzzle_at = from + horizontal * barrel;
        let lobbed = matches!(class, Class::Artillery | Class::Mortar);
        if !far && !muzzle.is_empty() {
            // lobbed weapons fire upward along their arc
            let aim = if lobbed {
                muzzle_at + (horizontal + Vec3::Y * 0.9).normalize()
            } else {
                to
            };
            self.spawn_effect(muzzle, muzzle_at, Some(aim), 1.0, None, true);
            if matches!(class, Class::MachineGun | Class::HeavyMg) && self.rng.next() < 0.35 {
                let side = Vec3::new(horizontal.z, 0.0, -horizontal.x);
                self.spawn_effect("mg_shells", from + side * 0.25, Some(from + side + Vec3::Y * 0.5), 1.0, None, true);
            }
        }
        match class {
            Class::Rifle | Class::MachineGun | Class::HeavyMg | Class::AntiAir | Class::Aircraft => {
                let tracer_ids = match class {
                    Class::Rifle => "rifle_shot_tracer",
                    Class::Aircraft => "aircraft_shot_tracer",
                    _ => "mg_shot_tracer",
                };
                let (color, speed, length) = match self.lib.get(tracer_ids).map(|d| &d.kind) {
                    Some(VfxKind::Tracer { color, speed, length, .. }) => (*color, *speed, *length),
                    _ => ([0.7, 0.7, 0.4], 100.0, 2.0),
                };
                let width = match class {
                    Class::Rifle => 0.035,
                    Class::HeavyMg | Class::AntiAir => 0.06,
                    _ => 0.05,
                };
                if !far {
                    self.tracer(muzzle_at, to, color, speed, length, width);
                }
                let default_hit = match class {
                    Class::Rifle => "rifle_shot_explosion",
                    Class::AntiAir => "flak_shot_1",
                    _ => "mg_shot_explosion",
                };
                let hit = impact.filter(|v| self.lib.get(v).is_some()).unwrap_or(default_hit);
                if !self.far(to) {
                    let jitter = Vec3::new(self.rng.sym() * 0.35, self.rng.sym() * 0.3 - 0.25, self.rng.sym() * 0.35);
                    self.delayed.push(Delayed {
                        at: self.time + dist / speed.max(10.0),
                        ids: hit.to_owned(),
                        pos: to + jitter,
                        toward: Some(from),
                        scale: 1.0,
                        lifetime: None,
                    });
                }
            }
            Class::Flame => {
                if let Some(hit) = impact.filter(|v| self.lib.nif(v).is_some()) {
                    self.delayed.push(Delayed {
                        at: self.time + 0.25,
                        ids: hit.to_owned(),
                        pos: to,
                        toward: Some(from),
                        scale: 1.0,
                        lifetime: None,
                    });
                }
            }
            Class::Cannon | Class::Artillery | Class::Mortar | Class::Rocket => {
                let default_hit = match class {
                    Class::Cannon => "tank_shot_explosion".to_owned(),
                    Class::Mortar => "mortar_shot_explosion".to_owned(),
                    Class::Rocket => "tank_shot_explosion".to_owned(),
                    _ => self.explosion_ids(5.0),
                };
                let hit = impact
                    .filter(|v| self.lib.nif(v).is_some())
                    .map(str::to_owned)
                    .unwrap_or(default_hit);
                let flight = if lobbed {
                    (dist / 45.0).clamp(0.6, 6.5)
                } else {
                    (dist / 140.0).clamp(0.05, 2.0) + 0.15
                };
                if self.pending.len() > 400 {
                    self.pending.remove(0);
                }
                self.pending.push(Pending {
                    ids: hit,
                    to,
                    deadline: self.time + flight,
                    scale: 1.0,
                });
            }
        }
    }

    /// Building destroyed: original destruction smoke, a large explosion,
    /// debris and ~60 s of burning (rising flame + smoke), then lingering smoke.
    pub fn building_destroyed(&mut self, at: [f32; 3], radius: f32) {
        let at = Vec3::from(at);
        let radius = radius.max(2.0);
        let destruction = if radius < 6.0 { "building_destruction_small" } else { "building_destruction" };
        self.spawn_effect(destruction, at, None, (radius / 6.0).clamp(0.7, 2.0), None, false);
        let big = format!("{}_large_explosion_1", self.climate);
        self.spawn_effect(&big, at + Vec3::Y * 0.5, None, (radius / 7.0).clamp(0.8, 2.0), None, false);
        self.spawn_effect(
            "scorch_mark_extra_large_vfx",
            at,
            None,
            (radius / 10.0).clamp(0.6, 1.5),
            None,
            false,
        );
        self.recent.push((at, radius * 1.5, self.time));
        self.debris(at, radius, ((radius * 3.0) as usize).clamp(10, 40));
        let fires = ((radius / 4.0) as usize).clamp(2, 5);
        for i in 0..fires {
            let a = i as f32 / fires as f32 * std::f32::consts::TAU + self.rng.next() * 0.8;
            let r = radius * (0.15 + 0.4 * self.rng.next());
            let p = at + Vec3::new(a.cos() * r, 0.2, a.sin() * r);
            let life = 50.0 + self.rng.next() * 20.0;
            let scale = 0.9 + self.rng.next() * 0.6;
            self.delayed.push(Delayed {
                at: self.time + 0.3 + self.rng.next() * 1.5,
                ids: "rising_flame".into(),
                pos: p,
                toward: None,
                scale,
                lifetime: Some(life),
            });
        }
        self.delayed.push(Delayed {
            at: self.time + 1.0,
            ids: "smoke_after_explosion".into(),
            pos: at + Vec3::Y * 0.5,
            toward: None,
            scale: (radius / 6.0).clamp(0.7, 1.5),
            lifetime: Some(60.0),
        });
        self.delayed.push(Delayed {
            at: self.time + 55.0,
            ids: "lingering_smoke".into(),
            pos: at,
            toward: None,
            scale: (radius / 5.0).clamp(0.8, 2.0),
            lifetime: Some(30.0),
        });
    }

    /// Vehicle/unit destroyed (optional hook): tank fire + smoke for ~20 s and
    /// the original death debris model for vehicles.
    pub fn unit_destroyed(&mut self, at: [f32; 3], vehicle: bool, heavy: bool) {
        let at = Vec3::from(at);
        if !vehicle {
            return;
        }
        let boom = self.explosion_ids(1.5);
        self.spawn_effect(&boom, at, None, 0.8, None, false);
        self.spawn_effect("tank_death_fire", at + Vec3::Y * 0.3, None, 1.0, Some(18.0), false);
        self.spawn_effect("tank_death_smoke", at + Vec3::Y * 0.5, None, 1.0, Some(22.0), false);
        let ids = if heavy { "heavy_tank_debris" } else { "medium_tank_debris" };
        if let Some(key) = self.lib.model_key(ids) {
            self.models.push(ModelFx {
                key,
                pos: at,
                yaw: self.rng.next() * std::f32::consts::TAU,
                scale: 1.0,
                age: 0.0,
                life: 45.0,
            });
        }
    }

    pub fn update(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 0.25);
        self.time += dt;
        let now = self.time;
        // delayed spawns (impacts arriving with their tracer, staged fires)
        let mut due = Vec::new();
        self.delayed.retain(|d| {
            if d.at <= now {
                due.push((d.ids.clone(), d.pos, d.toward, d.scale, d.lifetime));
                false
            } else {
                true
            }
        });
        for (ids, pos, toward, scale, lifetime) in due {
            let minor = lifetime.is_none();
            self.spawn_effect(&ids, pos, toward, scale, lifetime, minor);
        }
        // projectile impacts whose explosion event never came
        let mut landed = Vec::new();
        self.pending.retain(|p| {
            if p.deadline <= now {
                landed.push((p.ids.clone(), p.to, p.scale));
                false
            } else {
                true
            }
        });
        for (ids, to, scale) in landed {
            if self.far(to) {
                continue;
            }
            self.spawn_effect(&ids, to, None, scale, None, false);
            self.recent.push((to, 1.5, now));
        }
        self.recent.retain(|(_, _, t)| now - t < 3.0);
        // effects
        let alive: usize = self.effects.iter().map(|e| e.player.particle_count()).sum();
        let mut budget = (MAX_PARTICLES + MAX_PARTICLES / 4).saturating_sub(alive);
        for e in &mut self.effects {
            budget = budget.saturating_sub(e.player.update(dt, budget));
        }
        self.effects.retain(|e| !e.player.finished());
        // tracers
        for t in &mut self.tracers {
            t.t += dt;
        }
        self.tracers
            .retain(|t| t.t * t.speed - t.length < t.dist);
        // debris and dust
        for c in &mut self.chunks {
            c.age += dt;
            c.vel.y -= c.gravity * dt;
            c.vel *= (-c.drag * dt).exp();
            c.vel.y += c.rise * dt;
            c.pos += c.vel * dt;
            c.angle += c.spin * dt;
            if c.pos.y < c.floor {
                c.pos.y = c.floor;
                c.vel.y = -c.vel.y * 0.25;
                c.vel.x *= 0.5;
                c.vel.z *= 0.5;
                c.spin *= 0.5;
            }
        }
        self.chunks.retain(|c| c.age < c.life);
        for d in &mut self.decals {
            d.age += dt;
        }
        self.decals.retain(|d| d.age < d.duration);
        for m in &mut self.models {
            m.age += dt;
        }
        self.models.retain(|m| m.age < m.life);
    }

    pub fn model_instances(&self, models: &HashMap<String, ModelHandle>) -> Vec<(ModelHandle, DynInstance)> {
        self.models
            .iter()
            .filter_map(|m| {
                let h = *models.get(&m.key)?;
                // sink into the ground over the last 5 seconds
                let sink = ((m.age - (m.life - 5.0)) / 5.0).clamp(0.0, 1.0) * 1.2;
                let model = glam::Mat4::from_scale_rotation_translation(
                    Vec3::splat(m.scale),
                    Quat::from_rotation_y(m.yaw),
                    m.pos - Vec3::Y * sink,
                );
                Some((
                    h,
                    DynInstance {
                        model: model.to_cols_array_2d(),
                        team_color: [0.0; 4],
                        anim: [0.0, 0.0, 0.0, -0.35],
                    },
                ))
            })
            .collect()
    }

    pub fn particles(&self) -> Vec<ParticleInstance> {
        let mut out: Vec<ParticleInstance> = Vec::with_capacity(2048);
        // ground decals first (flat, under everything else)
        for d in &self.decals {
            if out.len() >= MAX_PARTICLES {
                break;
            }
            let mut a = 1.0;
            if d.fade[0] > 0.0 && d.age < d.fade[0] {
                a = d.age / d.fade[0];
            }
            if d.fade[1] > 0.0 && d.age > d.duration - d.fade[1] {
                a = a.min((d.duration - d.age) / d.fade[1]);
            }
            let h = d.size * 0.5;
            let (s, c) = d.angle.sin_cos();
            let ax = Vec3::new(c, 0.0, s) * h;
            let az = Vec3::new(-s, 0.0, c) * h;
            let corner = |p: Vec3, u: f32| [p.x, p.y, p.z, u];
            out.push(ParticleInstance {
                position: d.pos.to_array(),
                size: 0.0,
                color: [d.color[0], d.color[1], d.color[2], a.clamp(0.0, 1.0)],
                params: [d.layer, 0.0, if d.additive { 1.0 } else { 0.0 }, MODE_QUAD],
                corners: [
                    corner(-ax - az, 0.0),
                    corner(ax - az, 1.0),
                    corner(-ax + az, 0.0),
                    corner(ax + az, 1.0),
                ],
                corner_v: [0.0, 0.0, 1.0, 1.0],
            });
        }
        let first_sorted = out.len();
        for e in &self.effects {
            if out.len() >= MAX_PARTICLES {
                break;
            }
            e.player.render(&mut out, MAX_PARTICLES);
        }
        for c in &self.chunks {
            if out.len() >= MAX_PARTICLES {
                break;
            }
            let fade = ((c.life - c.age) / 0.5).clamp(0.0, 1.0);
            let grow = if c.rise > 0.0 { 0.6 + 0.6 * (c.age / c.life) } else { 1.0 };
            out.push(ParticleInstance {
                position: c.pos.to_array(),
                size: c.size * grow,
                color: [c.color[0], c.color[1], c.color[2], c.color[3] * fade],
                params: [c.layer, c.angle, 0.0, MODE_BILLBOARD],
                ..Default::default()
            });
        }
        for t in &self.tracers {
            if out.len() >= MAX_PARTICLES {
                break;
            }
            let travelled = t.t * t.speed;
            let head = travelled.min(t.dist);
            let tail = (travelled - t.length).max(0.0);
            if head - tail < 0.05 {
                continue;
            }
            let head_p = t.from + t.dir * head;
            out.push(ParticleInstance {
                position: head_p.to_array(),
                size: t.width,
                color: t.color,
                params: [self.streak_layer, 0.0, 1.0, MODE_STREAK],
                corners: [
                    [-(t.dir.x) * (head - tail), -(t.dir.y) * (head - tail), -(t.dir.z) * (head - tail), 0.0],
                    [0.0; 4],
                    [0.0; 4],
                    [0.0; 4],
                ],
                corner_v: [0.0; 4],
            });
        }
        if let Some(eye) = self.eye {
            let rest = &mut out[first_sorted..];
            let mut keyed: Vec<(f32, ParticleInstance)> = rest
                .iter()
                .map(|p| (Vec3::from(p.position).distance_squared(eye), *p))
                .collect();
            keyed.sort_by(|a, b| b.0.total_cmp(&a.0));
            for (slot, (_, p)) in rest.iter_mut().zip(keyed) {
                *slot = p;
            }
        }
        out
    }
}

fn xz_dist(a: Vec3, b: Vec3) -> f32 {
    let d = a - b;
    (d.x * d.x + d.z * d.z).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> Option<std::path::PathBuf> {
        let candidates = [
            std::env::var("AA_RECONSTRUCTION").ok().map(std::path::PathBuf::from),
            Some(std::path::PathBuf::from("E:/Games/Axis and Alies/reconstruction")),
        ];
        candidates
            .into_iter()
            .flatten()
            .find(|p| p.join("assets/original/Data/Effects").is_dir())
    }

    #[test]
    fn nif_table_parses() {
        let t = nif_table();
        assert_eq!(t.format, "aa-nif-effects-v1");
        assert!(t.effects.len() >= 60, "{} effects", t.effects.len());
        let mg = &t.effects["gunshot/mgheavyshot.nif"];
        assert!(!mg.sprites.is_empty());
        assert!(!mg.emitters.is_empty());
        let boom = &t.effects["explosions/temperate/medium/temperatemed_01.nif"];
        assert!(boom.emitters.len() >= 3);
        // every texture index resolves to an atlas layer
        let n = atlas_sources().len();
        for fx in t.effects.values() {
            for s in &fx.sprites {
                assert!(s.material.tex < n as i32);
            }
        }
    }

    #[test]
    fn original_definitions_resolve() {
        let Some(root) = root() else {
            eprintln!("reconstruction data not found; skipping");
            return;
        };
        let lib = VfxLibrary::load(&root);
        assert!(lib.files >= 26, "{} files", lib.files);
        assert!(lib.defs.len() >= 200, "{} definitions", lib.defs.len());
        for ids in [
            "rifle_shot",
            "rifle_shot_explosion",
            "mg_shot",
            "mg_heavy_shot",
            "mg_shells",
            "mg_shot_explosion",
            "tank_shot",
            "tank_shot_explosion",
            "artillery_shot",
            "mortar_shot",
            "mortar_shot_explosion",
            "temperate_small_explosion_1",
            "temperate_medium_explosion_1",
            "temperate_large_explosion_1",
            "desert_medium_explosion_1",
            "arctic_large_explosion_1",
            "building_destruction",
            "rising_flame",
            "smoke_after_explosion",
            "lingering_smoke",
            "flak_shot_1",
        ] {
            assert!(lib.nif(ids).is_some(), "{ids} has no playable NIF");
        }
        match &lib.get("mg_heavy_shot").unwrap().kind {
            VfxKind::Model { model, gyroscope, permanent, .. } => {
                assert_eq!(model, "gunshot/mgheavyshot.nif");
                assert!(!gyroscope);
                assert!(!permanent);
            }
            k => panic!("mg_heavy_shot is {k:?}"),
        }
        match &lib.get("rising_flame").unwrap().kind {
            VfxKind::Model { permanent, .. } => assert!(*permanent),
            k => panic!("{k:?}"),
        }
        match &lib.get("lingering_smoke").unwrap().kind {
            VfxKind::Model { duration, .. } => assert_eq!(*duration, 30.0),
            k => panic!("{k:?}"),
        }
        match &lib.get("temperate_medium_explosion_1").unwrap().kind {
            VfxKind::Model { random_orientation, .. } => assert!(*random_orientation),
            k => panic!("{k:?}"),
        }
        match &lib.get("rifle_shot_tracer").unwrap().kind {
            VfxKind::Tracer { color, speed, length, .. } => {
                assert_eq!(*color, [0.7, 0.7, 0.4]);
                assert_eq!(*speed, 100.0);
                assert_eq!(*length, 2.0);
            }
            k => panic!("{k:?}"),
        }
        match &lib.get("scorch_mark_medium_vfx").unwrap().kind {
            VfxKind::Decal { size, duration, fade, random_orientation, .. } => {
                assert_eq!(*size, 7.0);
                assert_eq!(*duration, 15.0);
                assert_eq!(*fade, [0.0, 5.0]);
                assert!(*random_orientation);
            }
            k => panic!("{k:?}"),
        }
        assert!(atlas_layer("scorchmarks/ScorchMarkM01.DDS").is_some());
        let models = lib.required_models();
        assert!(models.iter().all(|m| m.starts_with("effects/deathdebris/")), "{models:?}");
    }

    /// `AA_VFX_DUMP=<ids> cargo test dump_effect -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn dump_effect() {
        let Some(root) = root() else { return };
        let lib = Arc::new(VfxLibrary::load(&root));
        let ids = std::env::var("AA_VFX_DUMP").unwrap_or_else(|_| "mg_heavy_shot".into());
        let mut sys = VfxSystem::new(lib);
        sys.spawn(&ids, [0.0, 0.0, 0.0], Some([10.0, 0.0, 0.0]), 1.0);
        let mut t = 0.0;
        for _ in 0..40 {
            sys.update(0.05);
            t += 0.05;
            let p = sys.particles();
            println!("t={t:.2} n={}", p.len());
            for i in p.iter().take(12) {
                println!(
                    "  mode={} layer={} blend={} pos=({:.2},{:.2},{:.2}) size={:.2} color=({:.2},{:.2},{:.2},{:.2}) c0=({:.2},{:.2},{:.2}) c3=({:.2},{:.2},{:.2})",
                    i.params[3], i.params[0], i.params[2], i.position[0], i.position[1], i.position[2], i.size,
                    i.color[0], i.color[1], i.color[2], i.color[3],
                    i.corners[0][0], i.corners[0][1], i.corners[0][2], i.corners[3][0], i.corners[3][1], i.corners[3][2]
                );
            }
        }
    }

    #[test]
    fn system_plays_combat_effects() {
        let Some(root) = root() else { return };
        let lib = Arc::new(VfxLibrary::load(&root));
        let mut sys = VfxSystem::new(lib);
        sys.shot(Some("mg_heavy_shot"), Some("mg_shot_explosion"), [0.0, 1.3, 0.0], [30.0, 1.0, 5.0]);
        sys.shot(Some("rifle_shot"), Some("rifle_shot_explosion"), [0.0, 1.3, 0.0], [20.0, 1.0, -5.0]);
        sys.shot(Some("tank_shot"), None, [0.0, 1.3, 0.0], [60.0, 1.0, 0.0]);
        sys.explosion([10.0, 0.0, 10.0], 4.0);
        sys.building_destroyed([40.0, 0.0, 40.0], 8.0);
        let mut peak = 0;
        let mut kinds = HashSet::new();
        for _ in 0..120 {
            sys.update(1.0 / 30.0);
            let p = sys.particles();
            peak = peak.max(p.len());
            for i in &p {
                kinds.insert(i.params[3] as u32);
                assert!(i.position.iter().all(|v| v.is_finite()));
                assert!(i.color.iter().all(|v| v.is_finite()));
            }
        }
        assert!(peak > 50, "peak {peak}");
        assert!(kinds.contains(&0) && kinds.contains(&2), "{kinds:?}");
        assert!(peak <= MAX_PARTICLES);
    }
}
