//! Draws the aa-play battle world: every actor with its original model, the
//! original animation clip for what it is doing, its kingdom's team colour,
//! and smooth motion between the 16 Hz ticks. Owner: lead.
//!
//! Animation frames are sampled once per (rig, clip, frame) at FRAME_RATE and
//! cached, so any number of soldiers playing the same clip share meshes.
//!
//! The original's view animation on top (effects/view.rs; effects and sound
//! owner): the ground tilt of terrain-conforming things
//! (aa_play::movement::orientation, interpolated between ticks), tank treads
//! and tires turning with the distance moved, the towed trailers of trucks
//! (KKC_VTrailer), crew equipment shown and hidden by FXC_Equipment, and the
//! buildings' aspects: the packing, unpacking and death models with their
//! own timelines and the normal model's own animation.
use crate::effects::view::{self, AnimModel, Running, Trailer};
use crate::look::OriginalMaterial;
use crate::model_fixes::ModelFixes;
use crate::scene::{NEUTRAL_TEAM, kind, model_mesh, standard};
use crate::session::{Battle, tick_fraction};
use crate::textures::{self, Kind};
use aa_animation::{AnimatedAsset, PlaybackMode};
use aa_assets::AssetStore;
use aa_play::{Activity, ActorId, DefClass, World};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Animation sampling rate for the frame cache.
const FRAME_RATE: f32 = 20.0;

pub struct BattleViewPlugin;
impl Plugin for BattleViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_actors.run_if(in_state(crate::session::AppState::Battle)));
        app.add_systems(Update, draw_ghost.after(crate::input::battle_input).run_if(in_state(crate::session::AppState::Battle)));
    }
}

type Parts = Arc<Vec<(Handle<Mesh>, Handle<OriginalMaterial>)>>;

/// What the original's view animates on a model (effects/view.rs): its
/// parts in the original scene graph and the tread meshes at every
/// texture offset.
pub struct Extras {
    pub info: view::ModelInfo,
    pub treads: Vec<(usize, Arc<Vec<Handle<Mesh>>>, view::Tread)>,
}

/// Models, rigs, textures and materials loaded on first use.
#[derive(Resource)]
pub struct Library {
    store: Arc<AssetStore>,
    fixes: Arc<ModelFixes>,
    /// Lower-case original NIF path -> rig file.
    rig_files: HashMap<String, PathBuf>,
    models: HashMap<(String, [u8; 3]), Option<Parts>>,
    rigs: HashMap<(String, [u8; 3]), Option<Arc<Rig>>>,
    textures: HashMap<(String, Kind), Option<Handle<Image>>>,
    extras: HashMap<String, Option<Arc<Extras>>>,
    /// KF clips baked on the models (the character animations a rig lacks).
    kf: HashMap<(String, String, bool), KfSlot>,
    /// Aspect models with their own timelines (buildings).
    pub animated: view::AnimatedModels,
    data_root: PathBuf,
    pub failures: Vec<String>,
    /// Per static model: each drawn part's texture path and material (the
    /// aspect skins replace the texture).
    parts_src: HashMap<(String, [u8; 3]), Arc<Vec<(Option<String>, aa_assets::Material)>>>,
    /// (model, team, skin) -> the parts' materials with that skin.
    skins: HashMap<(String, [u8; 3], String), Arc<Vec<Handle<OriginalMaterial>>>>,
    /// Def IDS -> its Character data.
    characters: HashMap<String, Arc<CharacterDef>>,
}

pub struct Rig {
    asset: AnimatedAsset,
    materials: Vec<Handle<OriginalMaterial>>,
    /// clip index -> frames -> one mesh per rig mesh.
    frames: std::sync::Mutex<HashMap<(usize, u32), Arc<Vec<Handle<Mesh>>>>>,
    idle: Option<usize>,
    moving: Option<usize>,
    attack: Option<usize>,
    deaths: Vec<usize>,
    extras: Option<Arc<Extras>>,
}

impl Library {
    /// `rig_dir` is assets-cache/animation (it holds game/*.rig.json too).
    pub fn new(store: AssetStore, rig_dir: &Path, fixes: Arc<ModelFixes>) -> Self {
        let mut rig_files = HashMap::new();
        for dir in [rig_dir.to_path_buf(), rig_dir.join("game")] {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.to_string_lossy().ends_with(".rig.json") {
                    continue;
                }
                // Only the source field is needed for the index.
                let Ok(text) = std::fs::read_to_string(&path) else { continue };
                if let Some(source) = rig_source(&text) {
                    // game/ rigs carry idle/move/attack/die clips; prefer them.
                    let key = source.to_ascii_lowercase().replace('\\', "/");
                    let is_game = dir.ends_with("game");
                    if is_game || !rig_files.contains_key(&key) {
                        rig_files.insert(key, path);
                    }
                }
            }
        }
        let data_root = crate::effects::reconstruction_root().map(|r| r.join("assets/original/Data")).unwrap_or_default();
        let store = Arc::new(store);
        let animated = view::AnimatedModels::new(store.clone(), fixes.clone());
        Self {
            store,
            fixes,
            rig_files,
            models: HashMap::new(),
            rigs: HashMap::new(),
            textures: HashMap::new(),
            extras: HashMap::new(),
            kf: HashMap::new(),
            animated,
            data_root,
            failures: Vec::new(),
            parts_src: HashMap::new(),
            skins: HashMap::new(),
            characters: HashMap::new(),
        }
    }

    /// The materials of a static model's parts with an aspect skin (View
    /// Aspect `skin`, e.g. aspect_disabled's %sDisabledBase.dds): the parts
    /// textured with the normal aspect's skin take it, the others keep
    /// theirs. None until the model is loaded.
    fn skinned(&mut self, path: &str, team: [u8; 3], normal_skin: &str, skin: &str, materials: &mut Assets<OriginalMaterial>, images: &mut Assets<Image>) -> Option<Arc<Vec<Handle<OriginalMaterial>>>> {
        let key = (path.to_ascii_lowercase(), team, skin.to_ascii_lowercase());
        if let Some(s) = self.skins.get(&key) {
            return Some(s.clone());
        }
        let src = self.parts_src.get(&(key.0.clone(), team))?.clone();
        let parts = self.models.get(&(key.0.clone(), team)).cloned().flatten()?;
        let from = texture_stem(normal_skin);
        let to = texture_stem(skin);
        let mut list = Vec::with_capacity(src.len());
        for ((url, material), (_, original)) in src.iter().zip(parts.iter()) {
            let handle = match url.as_deref().filter(|u| texture_stem(u) == from) {
                Some(u) => match self.texture(&with_stem(u, &to), kind(material, team), images) {
                    Some(t) => materials.add(standard(material, Some(t))),
                    None => original.clone(),
                },
                None => original.clone(),
            };
            list.push(handle);
        }
        let list = Arc::new(list);
        self.skins.insert(key, list.clone());
        Some(list)
    }

    /// The same for an aspect model's parts (an HQ's normal aspect after it
    /// unpacked).
    fn skinned_anim(&mut self, model: &Arc<AnimModel>, team: [u8; 3], normal_skin: &str, skin: &str, materials: &mut Assets<OriginalMaterial>, images: &mut Assets<Image>) -> Arc<Vec<Handle<OriginalMaterial>>> {
        let key = (format!("anim:{:p}", Arc::as_ptr(model)), team, skin.to_ascii_lowercase());
        if let Some(s) = self.skins.get(&key) {
            return s.clone();
        }
        let from = texture_stem(normal_skin);
        let to = texture_stem(skin);
        let mut list = Vec::with_capacity(model.parts.len());
        for ((url, material), (_, original)) in model.src.iter().zip(model.parts.iter()) {
            let handle = match url.as_deref().filter(|u| texture_stem(u) == from) {
                Some(u) => match self.texture(&with_stem(u, &to), kind(material, team), images) {
                    Some(t) => materials.add(standard(material, Some(t))),
                    None => original.clone(),
                },
                None => original.clone(),
            };
            list.push(handle);
        }
        let list = Arc::new(list);
        self.skins.insert(key, list.clone());
        list
    }

    fn texture(&mut self, url: &str, kind: Kind, images: &mut Assets<Image>) -> Option<Handle<Image>> {
        let key = (url.to_owned(), kind);
        if let Some(h) = self.textures.get(&key) {
            return h.clone();
        }
        let handle = match textures::load(&self.store, url, kind) {
            Ok(image) => Some(images.add(image)),
            Err(e) => {
                self.failures.push(format!("texture {url}: {e:#}"));
                None
            }
        };
        self.textures.insert(key, handle.clone());
        handle
    }

    /// The model's view-animation data, from its original NIF (once per path).
    fn make_extras(&mut self, path: &str, meshes: &[&aa_assets::PreparedMesh], assets: &mut Assets<Mesh>) -> Option<Arc<Extras>> {
        let key = path.to_ascii_lowercase().replace('\\', "/");
        if let Some(e) = self.extras.get(&key) {
            return e.clone();
        }
        let nif = crate::effects::bones::find_file(&self.data_root, path).and_then(|p| std::fs::read(p).ok()).and_then(|b| aa_formats::nif::load(&b).ok());
        let extras = nif.map(|nif| {
            let info = view::model_info(&nif, meshes);
            let treads = info.parts.iter().enumerate().filter_map(|(i, p)| p.tread.map(|t| (i, view::tread_variants(meshes[i], t, assets), t))).collect();
            Arc::new(Extras { info, treads })
        });
        self.extras.insert(key, extras.clone());
        extras
    }

    /// A character animation's KF clip baked on `model` (effects/anim.rs
    /// bake_clip) on a loader thread: None while baking, Some(None) if the
    /// clip cannot be played. `all_meshes`: the parts are every converted
    /// mesh (a rig's), else the drawn ones.
    fn kf_clip(&mut self, model: &str, kf: &str, looping: bool, all_meshes: bool, meshes: &mut Assets<Mesh>) -> Option<Option<Arc<KfClip>>> {
        let key = (model.to_ascii_lowercase(), kf.to_ascii_lowercase(), all_meshes);
        if !self.kf.contains_key(&key) {
            let slot = Arc::new(std::sync::Mutex::new(None));
            let out = slot.clone();
            let (store, fixes, root, model, kf) = (self.store.clone(), self.fixes.clone(), self.data_root.clone(), model.to_owned(), kf.to_owned());
            rayon::spawn(move || {
                let decoded = (|| -> Option<KfDecoded> {
                    let mut prepared = store.load_model(&model).ok()?.prepare().ok()?;
                    let _ = fixes.apply(&store, &mut prepared);
                    let parts: Vec<&aa_assets::PreparedMesh> = prepared.meshes.iter().filter(|m| all_meshes || !m.indices.is_empty()).collect();
                    let converted: Vec<(String, usize)> = parts.iter().map(|m| (m.name.clone(), m.vertices.len())).collect();
                    let read = |rel: &str| crate::effects::bones::find_file(&root, rel).and_then(|p| std::fs::read(p).ok()).and_then(|b| aa_formats::nif::load(&b).ok());
                    let file = read(&model)?;
                    let kff = read(&kf)?;
                    let scene = crate::effects::extract::scene(&file);
                    let bind = |name: &str| scene.nodes.iter().find(|n| n.name.eq_ignore_ascii_case(name)).map_or(Mat3::IDENTITY, |n| Mat3::from_quat(Quat::from_xyzw(n.q[0], n.q[1], n.q[2], n.q[3]).normalize()));
                    // KF sequences store the quaternions one way (checked
                    // against the exported rigs: identical poses).
                    let (tracks, length) = crate::effects::extract::kf_tracks(&kff, &bind, Some(1.0));
                    let baked = crate::effects::anim::bake_clip(&file, &tracks, length, looping, &converted)?;
                    let mut frames = Vec::with_capacity(baked.frames);
                    for f in 0..baked.frames {
                        let mut list = Vec::with_capacity(parts.len());
                        for (i, mesh) in parts.iter().enumerate() {
                            let mut m = model_mesh(mesh);
                            if let Some((pos, nrm)) = baked.parts.get(i).and_then(|p| p.frames.as_ref()).and_then(|fr| fr.get(f)) {
                                m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos.clone());
                                m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, nrm.clone());
                            }
                            list.push(m);
                        }
                        frames.push(list);
                    }
                    Some((frames, length))
                })();
                *out.lock().unwrap() = Some(decoded);
            });
            self.kf.insert(key.clone(), KfSlot::Loading(slot));
        }
        let decoded = match self.kf.get(&key)? {
            KfSlot::Ready(c) => return Some(c.clone()),
            KfSlot::Loading(slot) => slot.lock().unwrap().take()?,
        };
        let clip = decoded.map(|(frames, duration)| Arc::new(KfClip { frames: frames.into_iter().map(|list| list.into_iter().map(|m| meshes.add(m)).collect()).collect(), duration }));
        if clip.is_none() {
            self.failures.push(format!("animation {kf} on {model} could not be baked"));
        }
        self.kf.insert(key, KfSlot::Ready(clip.clone()));
        Some(clip)
    }

    fn extras_of(&self, path: &str) -> Option<Arc<Extras>> {
        self.extras.get(&path.to_ascii_lowercase().replace('\\', "/")).cloned().flatten()
    }

    fn static_model(&mut self, path: &str, team: [u8; 3], assets: &mut Assets<Mesh>, materials: &mut Assets<OriginalMaterial>, images: &mut Assets<Image>) -> Option<Parts> {
        let key = (path.to_ascii_lowercase(), team);
        if let Some(m) = self.models.get(&key) {
            return m.clone();
        }
        let loaded = self.store.load_model(path).map_err(|e| e.to_string()).and_then(|m| m.prepare().map_err(|e| e.to_string()));
        let parts = match loaded {
            Ok(mut model) => {
                if let Err(e) = self.fixes.apply(&self.store, &mut model) {
                    self.failures.push(format!("model fixes {path}: {e:#}"));
                }
                let mut parts = Vec::new();
                let mut src = Vec::new();
                for mesh in model.meshes.iter().filter(|m| !m.indices.is_empty()) {
                    let texture = mesh.material.texture.clone().and_then(|t| self.texture(&t, kind(&mesh.material, team), images));
                    parts.push((assets.add(model_mesh(mesh)), materials.add(standard(&mesh.material, texture))));
                    src.push((mesh.material.texture.clone(), mesh.material.clone()));
                }
                self.parts_src.insert(key.clone(), Arc::new(src));
                let drawn: Vec<&aa_assets::PreparedMesh> = model.meshes.iter().filter(|m| !m.indices.is_empty()).collect();
                self.make_extras(path, &drawn, assets);
                Some(Arc::new(parts))
            }
            Err(e) => {
                self.failures.push(format!("model {path}: {e}"));
                None
            }
        };
        self.models.insert(key, parts.clone());
        parts
    }

    fn rig(&mut self, path: &str, team: [u8; 3], assets: &mut Assets<Mesh>, materials: &mut Assets<OriginalMaterial>, images: &mut Assets<Image>) -> Option<Arc<Rig>> {
        let key = (path.to_ascii_lowercase().replace('\\', "/"), team);
        if let Some(r) = self.rigs.get(&key) {
            return r.clone();
        }
        let rig = self.rig_files.get(&key.0).cloned().and_then(|file| match AnimatedAsset::load(&file) {
            Ok(mut asset) => {
                // Same material source as the client's animation path: the static model.
                let mut extras = None;
                if let Ok(mut model) = self.store.load_model(&asset.source).map_err(|e| e.to_string()).and_then(|m| m.prepare().map_err(|e| e.to_string())) {
                    let _ = self.fixes.apply(&self.store, &mut model);
                    if model.meshes.len() == asset.meshes.len() {
                        for (rig, mesh) in asset.meshes.iter_mut().zip(&model.meshes) {
                            rig.material = mesh.material.clone();
                        }
                        // The rig meshes are the model's: the same view data.
                        if asset.meshes.iter().zip(&model.meshes).all(|(r, m)| r.positions.len() == m.vertices.len()) {
                            let all: Vec<&aa_assets::PreparedMesh> = model.meshes.iter().collect();
                            extras = self.make_extras(path, &all, assets);
                        }
                    }
                }
                let mats = asset
                    .meshes
                    .iter()
                    .map(|m| {
                        let texture = m.material.texture.clone().and_then(|t| self.texture(&t, kind(&m.material, team), images));
                        materials.add(standard(&m.material, texture))
                    })
                    .collect();
                let find = |names: &[&str]| asset.clips.iter().position(|c| names.iter().any(|n| c.name.eq_ignore_ascii_case(n)));
                let idle = find(&["idle", "InfantryIdle"]);
                let moving = find(&["move", "InfantryJog", "InfantryWalk"]);
                let attack = find(&["attack", "InfantryAttack1"]);
                let deaths = asset
                    .clips
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.name.to_ascii_lowercase().starts_with("die") || c.name.contains("Die"))
                    .map(|(i, _)| i)
                    .collect();
                Some(Arc::new(Rig { asset, materials: mats, frames: Default::default(), idle, moving, attack, deaths, extras }))
            }
            Err(e) => {
                self.failures.push(format!("rig {}: {e:#}", file.display()));
                None
            }
        });
        self.rigs.insert(key, rig.clone());
        rig
    }
}

fn rig_source(text: &str) -> Option<String> {
    let at = text.find("\"source\"")?;
    let rest = &text[at + 8..];
    let start = rest.find('"')? + 1;
    let end = rest[start..].find('"')? + start;
    Some(rest[start..end].to_owned())
}

impl Rig {
    /// The clip and playback time for an activity.
    /// `death` is the original death clip combat chose (Def::death_clips id).
    /// `object` is an object animation an FX command started (e.g. a tank's
    /// `recoil1` on firing): (clip IDS, start time); it plays once, then the
    /// normal clip returns.
    fn clip_for(&self, actor: ActorId, activity: Activity, since: f32, now: f32, death: Option<&str>, object: Option<(&str, f32)>) -> Option<(usize, f32, PlaybackMode)> {
        let phase = (actor as f32 * 0.618_034).fract() * 4.0;
        if let Some((name, start)) = object {
            if let Some(c) = self.asset.clips.iter().position(|c| c.name.eq_ignore_ascii_case(name)) {
                let t = now - start;
                if (0.0..self.asset.clips[c].duration).contains(&t) && !matches!(activity, Activity::Dying | Activity::Dead) {
                    return Some((c, t, PlaybackMode::Clamp));
                }
            }
        }
        // Rigs with only object clips (tank recoil): rest on their first frame.
        let rest = || (self.idle.is_none() && self.moving.is_none()).then(|| self.asset.clips.iter().position(|c| c.name.to_ascii_lowercase().starts_with("recoil"))).flatten().map(|c| (c, 0.0, PlaybackMode::Clamp));
        let normal = match activity {
            Activity::Moving => self.moving.or(self.idle).map(|c| (c, now + phase, PlaybackMode::Loop)),
            Activity::Attacking => self.attack.or(self.idle).map(|c| (c, now - since, PlaybackMode::Loop)),
            Activity::Dying | Activity::Dead => {
                let chosen = death.and_then(|name| self.asset.clips.iter().position(|c| c.name.eq_ignore_ascii_case(name)));
                let clip = match chosen {
                    Some(c) => c,
                    None => *self.deaths.get(actor as usize % self.deaths.len().max(1))?,
                };
                let t = if activity == Activity::Dead { f32::MAX / 4.0 } else { now - since };
                Some((clip, t, PlaybackMode::Clamp))
            }
            _ => self.idle.map(|c| (c, now + phase, PlaybackMode::Loop)),
        };
        normal.or_else(rest)
    }

    fn frame(&self, clip: usize, time: f32, mode: PlaybackMode, meshes: &mut Assets<Mesh>) -> Option<Arc<Vec<Handle<Mesh>>>> {
        let duration = self.asset.clips.get(clip)?.duration.max(1.0 / FRAME_RATE);
        let t = match mode {
            PlaybackMode::Loop => time.rem_euclid(duration),
            PlaybackMode::Clamp => time.clamp(0.0, duration),
        };
        let index = (t * FRAME_RATE).floor() as u32;
        let mut frames = self.frames.lock().unwrap();
        if let Some(f) = frames.get(&(clip, index)) {
            return Some(f.clone());
        }
        let pose = self.asset.sample(clip, index as f32 / FRAME_RATE, PlaybackMode::Clamp).ok()?;
        let handles: Vec<_> = self
            .asset
            .meshes
            .iter()
            .zip(pose.meshes)
            .map(|(rig, posed)| {
                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, posed.positions);
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, posed.normals);
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, rig.uvs.iter().map(|uv| crate::scene::model_uv(*uv)).collect::<Vec<_>>());
                mesh.insert_indices(Indices::U32(rig.indices.clone()));
                meshes.add(mesh)
            })
            .collect();
        let handles = Arc::new(handles);
        frames.insert((clip, index), handles.clone());
        Some(handles)
    }
}

/// A KF clip baked on a model: per frame (20 Hz) one mesh per part.
struct KfClip {
    frames: Vec<Vec<Handle<Mesh>>>,
    duration: f32,
}
type KfDecoded = (Vec<Vec<Mesh>>, f32);
enum KfSlot {
    Loading(Arc<std::sync::Mutex<Option<Option<KfDecoded>>>>),
    Ready(Option<Arc<KfClip>>),
}

use crate::effects::data::{AbilityAnims, AnimFx, StanceFx};

/// One animation of a Character's sets (shared by every actor of a def).
type Anim = Arc<AnimFx>;

/// A def's Character data, built once per def and shared by its actors:
/// the base set, the aspect sets, the Stance list, the abilities'
/// animations and View idle_loops.
pub struct CharacterDef {
    base: Vec<Anim>,
    sets: Vec<(String, Vec<Anim>)>,
    stances: Vec<StanceFx>,
    abilities: Vec<AbilityAnims>,
    idle_loops: [i32; 2],
}

impl CharacterDef {
    fn new(fx: &crate::effects::data::ActorFx) -> Self {
        let list = |l: &[AnimFx]| l.iter().cloned().map(Arc::new).collect::<Vec<_>>();
        CharacterDef {
            base: list(&fx.view.animations),
            sets: fx.view.anim_sets.iter().map(|(a, l)| (a.clone(), list(l))).collect(),
            stances: fx.view.stances.clone(),
            abilities: fx.view.abilities.clone(),
            idle_loops: fx.view.idle_loops,
        }
    }
}

/// What the Character is playing (its state +28 and the event it waits for).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    None,
    /// The idle, repeated idle_loops times (563BA0), then a fidget.
    Idle,
    /// A fidget (563CA0), then the idle again.
    Fidget,
    /// The move of the current speed (5774A0 -> 45DA00).
    Move,
    /// An ability's animation, once per use (57ED20).
    Ability,
    /// The ability's idle animation between uses (563BA0, Performer busy).
    AbilityIdle,
    Die,
}

/// The original Character of a biped (KKC_CharacterComponent): its
/// animation sets (464BD0: the base set and one per AspectAnimationSet,
/// merged by 463F10), the set of the active stance (45E620, 45E690), what
/// plays for what the actor does, its playback rate (44F7B0: KF length /
/// gameplay length; a move also speed / walk_distance, 4DB170) and its
/// FXC_Equipment lists.
struct Character {
    def: Arc<CharacterDef>,
    /// The set in use (None: the base set) and the animation playing.
    set: Option<usize>,
    current: Option<Anim>,
    mode: Mode,
    /// When the idle repetitions, the fidget or the ability use end.
    until: f32,
    started: f32,
    /// A move's KF time, accumulated with the speed.
    phase: f32,
    last: f32,
    cues: usize,
    /// The game RNG stand-in for 49B4D0 / 45DA40 / 4BE5C0 (85D840 LCG).
    rng: u32,
    /// A KF frame has been drawn (later clips keep the last frame while
    /// they bake instead of falling back to the rig).
    drawn: bool,
    weapon: Option<u32>,
}

impl Character {
    fn new(def: Arc<CharacterDef>, actor: ActorId, now: f32) -> Self {
        Character {
            def,
            set: None,
            current: None,
            mode: Mode::None,
            until: 0.0,
            started: now,
            phase: 0.0,
            last: now,
            cues: 0,
            rng: (actor as u32).wrapping_mul(2_654_435_761) ^ 0x5eed,
            drawn: false,
            weapon: None,
        }
    }

    /// 85D840.
    fn next(&mut self) -> u32 {
        self.rng = self.rng.wrapping_mul(0x41c6_4e6d).wrapping_add(0x3039);
        self.rng >> 16
    }

    fn list(&self) -> &[Anim] {
        self.set.map_or(&self.def.base, |i| &self.def.sets[i].1)
    }

    /// 45EC00: the animation of this IDS and type in the set.
    fn lookup(&self, ids: &str, kind: &str) -> Option<Anim> {
        self.list().iter().find(|a| a.kind == kind && a.ids == ids).cloned()
    }

    fn first(&self, kind: &str) -> Option<Anim> {
        self.list().iter().find(|a| a.kind == kind).cloned()
    }

    /// 45DA00 over the thresholds of 461190: each move of the set in order
    /// takes the speeds up to the mean of its walk_distance and the next
    /// one's; the last takes the rest.
    fn move_for(&self, speed: f32) -> Option<Anim> {
        let moves: Vec<&Anim> = self.list().iter().filter(|a| a.kind == "MOVE").collect();
        for (i, m) in moves.iter().enumerate() {
            let threshold = moves.get(i + 1).map_or(f32::MAX, |n| (m.walk_distance + n.walk_distance) * 0.5);
            if speed <= threshold {
                return Some((*m).clone());
            }
        }
        None
    }

    /// 45DA40 weighted: a random point in the total weight, the first
    /// animation whose weight takes it to zero.
    fn fidget(&mut self) -> Option<Anim> {
        let fidgets: Vec<Anim> = self.list().iter().filter(|a| a.kind == "FIDGET").cloned().collect();
        if fidgets.len() <= 1 {
            return fidgets.into_iter().next();
        }
        let total: f32 = fidgets.iter().map(|f| f.weight).sum();
        let r = self.next() as f64 / 65536.0 * total as f64;
        let mut left = r;
        for f in &fidgets {
            left -= f.weight as f64;
            if left <= 0.0 {
                return Some(f.clone());
            }
        }
        None
    }

    /// 49B4D0: idle repetitions in [idle_loops_min, idle_loops_max).
    fn idle_loops(&mut self) -> f32 {
        let [min, max] = self.def.idle_loops;
        let width = max.wrapping_sub(min).max(1) as u32;
        (self.next() % width) as f32 + min as f32
    }

    /// The stance's aspect picks the animation set (45E690: none -> the
    /// base set).
    fn stance_set(&self, world: &World, actor: &aa_play::Actor) -> Option<usize> {
        let aspect = stance_aspect(&self.def.stances, world, actor)?;
        self.def.sets.iter().position(|(a, _)| a == aspect)
    }

    /// Decide what plays now; returns whether the animation (re)started.
    fn update(&mut self, actor: &aa_play::Actor, world: &World, fired: Option<u32>, death: Option<&str>, hidden: &mut HashSet<String>) -> bool {
        let now = world.time;
        // The death plays in the set of the stance the actor died in.
        let set = if matches!(actor.activity, Activity::Dying | Activity::Dead) { self.set } else { self.stance_set(world, actor) };
        let mut restart = false;
        if set != self.set {
            self.set = set;
            // 5638B0: a looping animation is looked up again in the new set
            // at once (by IDS, else the set's own animation of that type).
            if let Some(c) = self.current.clone()
                && matches!(c.kind.as_str(), "IDLE" | "MOVE")
            {
                let again = self.lookup(&c.ids, &c.kind).or_else(|| self.first(&c.kind));
                if again.as_ref().is_some_and(|a| a.kf != c.kf) {
                    self.switch(again, self.mode, now, hidden);
                    restart = true;
                }
            }
        }
        if fired.is_some() {
            self.weapon = fired;
        }
        let (want, mode): (Option<Anim>, Mode) = match actor.activity {
            Activity::Dying | Activity::Dead => {
                if self.mode == Mode::Die {
                    return restart;
                }
                let a = death.and_then(|d| self.lookup(&d.to_ascii_lowercase(), "DIE")).or_else(|| self.first("DIE"));
                (a, Mode::Die)
            }
            Activity::Moving => (self.move_for(actor.motion.speed.max(0.0)), Mode::Move),
            Activity::Attacking => {
                let ability = self.weapon.and_then(|w| self.def.abilities.get(w as usize)).cloned().unwrap_or_default();
                if fired.is_some() {
                    // 57ED20 / 4BE5C0: a random one of the ability's animations.
                    let found: Vec<Anim> = ability.animations.iter().filter_map(|i| self.lookup(i, "ABILITY")).collect();
                    let pick = (!found.is_empty()).then(|| {
                        let n = found.len() as u32;
                        found[(self.next() % n) as usize].clone()
                    });
                    match pick {
                        Some(a) => {
                            self.until = now + a.length.max(0.05);
                            self.switch(Some(a), Mode::Ability, now, hidden);
                            return true;
                        }
                        None => (ability.idle.as_ref().and_then(|i| self.lookup(i, "IDLE")).or_else(|| self.first("IDLE")), Mode::AbilityIdle),
                    }
                } else if self.mode == Mode::Ability && now < self.until {
                    return restart;
                } else {
                    (ability.idle.as_ref().and_then(|i| self.lookup(i, "IDLE")).or_else(|| self.first("IDLE")), Mode::AbilityIdle)
                }
            }
            _ => match self.mode {
                Mode::Idle | Mode::Fidget if now < self.until => return restart,
                Mode::Idle => match self.fidget() {
                    // 563CA0: the idle repetitions are over: a fidget.
                    Some(f) => {
                        self.until = now + f.length.max(0.05);
                        self.switch(Some(f), Mode::Fidget, now, hidden);
                        return true;
                    }
                    None => {
                        let a = self.first("IDLE");
                        self.until = now + self.idle_loops() * a.as_ref().map_or(3.0, |a| a.length);
                        (a, Mode::Idle)
                    }
                },
                _ => {
                    // 563BA0: the set's first idle, idle_loops times.
                    let a = self.first("IDLE");
                    self.until = now + self.idle_loops() * a.as_ref().map_or(3.0, |a| a.length);
                    (a, Mode::Idle)
                }
            },
        };
        let same = match (&self.current, &want) {
            (Some(c), Some(w)) => c.kf == w.kf && c.ids == w.ids,
            (None, None) => true,
            _ => false,
        };
        if same && self.mode == mode {
            return restart;
        }
        // 4DB120 / 4DB170: the same looping animation keeps its start time.
        let keep = same && matches!(mode, Mode::Idle | Mode::Move | Mode::AbilityIdle);
        if keep {
            self.mode = mode;
            return restart;
        }
        self.switch(want, mode, now, hidden);
        true
    }

    fn switch(&mut self, want: Option<Anim>, mode: Mode, now: f32, hidden: &mut HashSet<String>) {
        if let Some(c) = &self.current {
            equipment(&c.end, hidden);
        }
        if let Some(w) = &want {
            equipment(&w.start, hidden);
        }
        let keep_phase = self.current.as_ref().is_some_and(|c| c.kind == "MOVE") && want.as_ref().is_some_and(|w| w.kind == "MOVE");
        if !keep_phase {
            self.phase = 0.0;
        }
        self.current = want;
        self.mode = mode;
        self.started = now;
        self.cues = 0;
    }
}

/// FXC_Equipment HIDE / UNHIDE of one list.
fn equipment(list: &[crate::effects::data::Fxc], hidden: &mut HashSet<String>) {
    for f in list.iter().filter(|f| f.kind == "EQUIPMENT") {
        match f.action.as_str() {
            "HIDE" => {
                hidden.insert(f.ids.clone());
            }
            "UNHIDE" => {
                hidden.remove(&f.ids);
            }
            _ => {}
        }
    }
}

/// A truck's towed trailer, drawn as its own model.
struct TrailerView {
    entity: Entity,
    parts: Vec<Entity>,
    state: Trailer,
    running: Option<Running>,
    /// The truck model's Hitch node (model space, NIF frame).
    truck_hitch: Vec3,
}

/// A building aspect being drawn from its own model and timeline.
struct AspectPlay {
    ids: String,
    model: Arc<AnimModel>,
    start: f32,
    looping: bool,
    shown: usize,
}

/// The original view-animation state of one drawn actor.
#[derive(Default)]
struct Look {
    setup: bool,
    model: String,
    team: [u8; 3],
    extras: Option<Arc<Extras>>,
    running: Option<Running>,
    /// Parts the rig frames leave alone (the tread meshes).
    own_mesh: Vec<bool>,
    /// Per part: its equipment type.
    equipment: Vec<Option<String>>,
    hidden: HashSet<String>,
    applied: Option<HashSet<String>>,
    trailer: Option<TrailerView>,
    aspect: Option<AspectPlay>,
    building: bool,
    character: Option<Character>,
    /// The aspect skin the parts wear (None: the model's own textures).
    skin: Option<String>,
    /// Per Performer turret: its node's parts, turned by the sim's yaw.
    turrets: Vec<Option<view::TurretView>>,
    /// An aircraft waiting in its hangar (parts hidden).
    in_hangar: bool,
}

/// An aircraft created in its airfield's or carrier's hangar waits there
/// unseen until the runway releases it (CreateDenizen 0x56CC10 sets actor
/// +8 flag 0x40000000 and virtual +0x50(0) through 0x4D04B0; the release
/// 0x5695B0 / 0x4D7EA0 shows it again): no model, so no shadow either.
fn in_hangar(world: &World, id: ActorId) -> bool {
    world.books.actor(id).and_then(|b| b.aircraft.as_ref()).is_some_and(|x| x.hidden)
}

/// One drawn actor: a parent with its transform and one child per model mesh.
#[derive(Component)]
pub struct ActorView {
    pub id: ActorId,
    rig: Option<Arc<Rig>>,
    parts: Vec<Entity>,
    prev: (Vec3, f32),
    cur: (Vec3, f32),
    /// The ground placement (movement::orientation) at the last two ticks:
    /// model origin and up vector.
    origin: (Vec3, Vec3),
    up: (Vec3, Vec3),
    tick: u64,
    look: Look,
}

/// Native (x, y ground, z up) -> render (x, z, -y).
pub fn to_render(p: Vec3) -> Vec3 {
    Vec3::new(p.x, p.z, -p.y)
}

/// Render yaw for a native facing angle (0 = native +X); models face native -Y.
pub fn render_yaw(facing: f32) -> f32 {
    facing + std::f32::consts::FRAC_PI_2
}

/// The death clip combat chose for this actor, by its original clip id.
fn death_clip<'w>(world: &'w World, actor: &aa_play::Actor) -> Option<&'w str> {
    let index = world.combat.actor(actor.id)?.death_clip?;
    world.data.def(actor.def).death_clips.get(index).map(|c| c.id.as_str())
}

fn team_of(world: &World, actor: &aa_play::Actor) -> [u8; 3] {
    // A map's independent kingdom (cities, farms) is drawn neutral.
    actor.kingdom.and_then(|k| world.kingdoms.get(k as usize)).filter(|k| !k.independent).map_or(NEUTRAL_TEAM, |k| k.color)
}

/// The ground placement of an actor now: model origin and up.
fn placement(world: &World, actor: &aa_play::Actor) -> (Vec3, Vec3) {
    match aa_play::movement::orientation(world, actor.id) {
        Some(o) => (o.position, o.up.normalize_or(Vec3::Z)),
        None => (actor.pos, Vec3::Z),
    }
}

/// 45E620 (4D0050 / 4D16B0 re-evaluate it as the actor changes): the last
/// stance of the list whose health and actor state apply; its aspect (None:
/// the normal aspect, or none applies).
fn stance_aspect<'a>(stances: &'a [StanceFx], world: &World, actor: &aa_play::Actor) -> Option<&'a str> {
    let max = world.combat.actor(actor.id).map_or(0.0, |c| c.max_health);
    let max = if max > 0.0 { max } else { world.data.def(actor.def).health.max(1.0) };
    let health = actor.health / max;
    let org = actor.org.and_then(|o| world.combat.org(o));
    let dead = !actor.alive || matches!(actor.activity, Activity::Dying | Activity::Dead);
    let state = |s: &str| match s {
        // The organization's Mind state (actor_states_aa.tgi group 4).
        "mind_engaged" => org.map_or(actor.combat.target.is_some(), |o| o.routing.is_none() && !o.exhausted && o.engaging()),
        "dead" => dead,
        "structure_packing" => actor.activity == Activity::Packing,
        "structure_unpacking" => actor.activity == Activity::Unpacking,
        _ => false,
    };
    // A dead actor's health is 0, under every stance's health level; only
    // the stances that name its state apply then.
    let stance = stances.iter().rev().find(|s| (health <= s.health + 1e-6 || dead) && if s.states.is_empty() { !dead } else { s.states.iter().any(|x| state(x)) })?;
    stance.aspect.as_deref()
}

/// The file name of a texture path without its extension, lower case.
fn texture_stem(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.rsplit_once('.').map_or(name, |(s, _)| s).to_ascii_lowercase()
}

/// The same path with another file name (its folder and extension kept).
fn with_stem(path: &str, stem: &str) -> String {
    let (folder, name) = path.rsplit_once(['/', '\\']).map_or(("", path), |(f, n)| (f, n));
    let ext = name.rsplit_once('.').map_or("", |(_, e)| e);
    let file = if ext.is_empty() { stem.to_owned() } else { format!("{stem}.{ext}") };
    if folder.is_empty() { file } else { format!("{folder}/{file}") }
}

/// The aspect a building shows for what it is doing (StancePacking,
/// StanceUnpacking, StanceDeath; else AspectNormal), for data without a
/// stance list.
fn building_aspect(activity: Activity) -> &'static str {
    match activity {
        Activity::Packing => "aspect_packing",
        Activity::Unpacking => "aspect_unpacking",
        Activity::Dying | Activity::Dead => "aspect_death",
        _ => "aspect_normal",
    }
}

/// The model of an aspect (FxTables view aspects), Data-relative.
fn aspect_model(fx: &crate::effects::data::ActorFx, ids: &str) -> Option<String> {
    fx.view.aspects.iter().find(|a| a.ids == ids).and_then(|a| a.model.clone())
}

/// Once the effect tables are loaded: the view data of a drawn actor
/// (tires and treads, equipment, the truck's trailer).
#[allow(clippy::too_many_arguments)]
fn set_up(
    look: &mut Look,
    actor: &aa_play::Actor,
    world: &World,
    tables: &crate::effects::data::FxTables,
    library: &mut Library,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<OriginalMaterial>,
    images: &mut Assets<Image>,
    rig_meshes: bool,
) {
    look.setup = true;
    let def = world.data.def(actor.def);
    look.building = def.class == Some(DefClass::Building);
    let Some(fx) = tables.actors.get(&def.id.to_ascii_lowercase()) else { return };
    let node = |bone: &str| tables.bones.get(bone).map(|n| n.to_ascii_lowercase()).unwrap_or_else(|| bone.to_owned());
    if let Some(extras) = look.extras.clone() {
        let tires: Vec<(String, bool)> = fx.view.tires.iter().map(|(b, front)| (node(b), *front)).collect();
        look.running = Running::new(&extras.info, &tires, fx.view.tire_circumference, extras.treads.clone());
        look.own_mesh = (0..extras.info.parts.len()).map(|i| extras.treads.iter().any(|(p, _, _)| *p == i)).collect();
        look.equipment = extras.info.parts.iter().map(|p| p.equipment.clone()).collect();
        // The Performer turrets (turret_ids -> Bone node, 453BF0).
        look.turrets = fx
            .turrets
            .iter()
            .enumerate()
            .map(|(i, ids)| {
                let mut t = view::TurretView::new(&extras.info, &node(ids))?;
                t.pitch = def.turrets.get(i).map_or(0.0, |d| d.pitch_start);
                Some(t)
            })
            .collect();
        // View WhenCreated FXC_Equipment (e.g. a mortar team carries its tube).
        for f in fx.created.iter().filter(|f| f.kind == "EQUIPMENT") {
            match f.action.as_str() {
                "HIDE" => {
                    look.hidden.insert(f.ids.clone());
                }
                "UNHIDE" => {
                    look.hidden.remove(&f.ids);
                }
                _ => {}
            }
        }
        // The towed trailer (MoverComponent trailer_actor, 4D6F10).
        if let (Some(tid), Some((hitch_bone, truck_offset))) = (fx.view.trailer.as_ref(), fx.view.hitch.as_ref())
            && let Some(tfx) = tables.actors.get(tid)
            && let Some(prefix) = tfx.resource_prefix.as_ref()
            && let Some(truck_hitch) = extras.info.nodes.get(&node(hitch_bone)).map(|b| b.global.w_axis.truncate())
        {
            let path = format!("{prefix}.nif");
            if let Some(parts) = library.static_model(&path, look.team, meshes, materials, images)
                && let Some(textras) = library.extras_of(&path)
            {
                let trailer_bone = tfx.view.hitch.as_ref().map_or("hitch".to_owned(), |(b, _)| node(b));
                let trailer_offset = tfx.view.hitch.as_ref().map_or([0.0; 3], |(_, o)| *o);
                let hitch = textras.info.nodes.get(&trailer_bone).map_or(Vec3::new(0.0, -1.0, 0.5), |b| b.global.w_axis.truncate());
                let f = Vec2::new(actor.facing.cos(), actor.facing.sin());
                let state = Trailer::spawn(actor.pos, f, *truck_offset, trailer_offset, hitch, [Vec2::from(tfx.view.rear[0]), Vec2::from(tfx.view.rear[1])]);
                let tires: Vec<(String, bool)> = tfx.view.tires.iter().map(|(b, front)| (node(b), *front)).collect();
                let running = Running::new(&textras.info, &tires, tfx.view.tire_circumference, textras.treads.clone());
                let entity = commands.spawn((view::render_transform(state.matrix()), Visibility::default())).id();
                let parts: Vec<Entity> = parts.iter().map(|(mesh, material)| commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), ChildOf(entity))).id()).collect();
                look.trailer = Some(TrailerView { entity, parts, state, running, truck_hitch });
            }
        }
    }
    if std::env::var_os("AA_BEVY_FX_STATS").is_some() && (look.running.is_some() || look.trailer.is_some() || look.equipment.iter().any(Option::is_some)) {
        info!(
            "view: actor {} {} ({}): {} tires, {} treads, equipment {:?} hidden {:?}, trailer {}",
            actor.id,
            def.id,
            look.model,
            fx.view.tires.len(),
            look.extras.as_ref().map_or(0, |e| e.treads.len()),
            look.equipment.iter().flatten().collect::<Vec<_>>(),
            look.hidden,
            look.trailer.is_some()
        );
    }
    if !look.building && !fx.view.animations.is_empty() {
        let key = def.id.to_ascii_lowercase();
        let cdef = library.characters.entry(key).or_insert_with(|| Arc::new(CharacterDef::new(fx))).clone();
        let ch = Character::new(cdef, actor.id, world.time);
        // Start baking what a soldier plays first: each set's first idle,
        // its moves and its abilities' animations.
        let all_meshes = rig_meshes;
        let mut wanted: Vec<(String, bool)> = Vec::new();
        for list in std::iter::once(&ch.def.base).chain(ch.def.sets.iter().map(|(_, l)| l)) {
            let first_idle = list.iter().find(|a| a.kind == "IDLE");
            let named = |ids: &str| list.iter().find(|a| a.ids == ids);
            let abilities = ch.def.abilities.iter().flat_map(|ab| ab.animations.iter().chain(ab.idle.iter())).filter_map(|i| named(i));
            for a in first_idle.into_iter().chain(list.iter().filter(|a| a.kind == "MOVE")).chain(abilities) {
                let item = (a.kf.clone(), matches!(a.kind.as_str(), "MOVE" | "IDLE"));
                if !a.kf.is_empty() && !wanted.contains(&item) {
                    wanted.push(item);
                }
            }
        }
        for (kf, looping) in wanted {
            let _ = library.kf_clip(&look.model, &kf, looping, all_meshes, meshes);
        }
        look.character = Some(ch);
    }
    // A truck that unpacks: its building's unpacking aspect is decoded now,
    // so the camp starts from its first frame when the truck deploys.
    if let Some(target) = def.unpacks_to
        && let Some(tfx) = tables.actors.get(&world.data.def(target).id.to_ascii_lowercase())
        && let Some(m) = aspect_model(tfx, "aspect_unpacking")
    {
        library.animated.request(&m, look.team);
    }
    // Buildings: start decoding the aspects they may show.
    if look.building {
        for ids in ["aspect_normal", "aspect_packing", "aspect_unpacking", "aspect_death"] {
            if let Some(m) = aspect_model(fx, ids) {
                library.animated.request(&m, look.team);
            }
        }
    }
}

/// Play the Character's animation on the parts: the animation's own KF
/// (shared or the unit's) baked on the model. Returns whether the parts
/// show the Character (else the rig's clip is drawn).
#[allow(clippy::too_many_arguments)]
fn animate_character(
    ch: &mut Character,
    actor: &aa_play::Actor,
    world: &World,
    fired: Option<u32>,
    log: bool,
    hidden: &mut HashSet<String>,
    rig: Option<&Rig>,
    model: &str,
    parts: &[Entity],
    own_mesh: &[bool],
    library: &mut Library,
    meshes: &mut Assets<Mesh>,
    parts_q: &mut Query<(&mut Mesh3d, &mut Transform, &mut Visibility), Without<ActorView>>,
) -> bool {
    let now = world.time;
    let dt = (now - ch.last).max(0.0);
    ch.last = now;
    let death = death_clip(world, actor);
    if ch.update(actor, world, fired, death, hidden) {
        if ch.mode == Mode::Die {
            ch.started = actor.activity_since;
        }
        if log {
            let c = ch.current.as_ref();
            info!(
                "character: actor {} {} [{}] {:?} speed {:.2} -> {:?} {} {} ({}) length {}",
                actor.id,
                world.data.def(actor.def).id,
                ch.set.map_or("normal", |i| ch.def.sets[i].0.as_str()),
                actor.activity,
                actor.motion.speed,
                ch.mode,
                c.map_or("-", |a| a.kind.as_str()),
                c.map_or("-", |a| a.ids.as_str()),
                c.map_or("-", |a| a.kf.as_str()),
                c.map_or(0.0, |a| a.length)
            );
        }
    }
    let Some(a) = ch.current.clone() else { return false };
    // WhenDoAtTime cues (gameplay seconds into the animation).
    let elapsed = now - ch.started;
    let mut cues: Vec<&crate::effects::data::Fxc> = a.at_time.iter().collect();
    cues.sort_by(|x, y| x.time.total_cmp(&y.time));
    while ch.cues < cues.len() && elapsed >= cues[ch.cues].time {
        equipment(std::slice::from_ref(cues[ch.cues]), hidden);
        ch.cues += 1;
    }
    // 499FF0: idle and move loop, the other types play once.
    let looping = matches!(a.kind.as_str(), "MOVE" | "IDLE");
    let length = a.length.max(0.05);
    // 4DB170: a move's rate is speed / walk_distance.
    let rate_move = if a.kind == "MOVE" { actor.motion.speed.max(0.0) / a.walk_distance.max(0.05) } else { 1.0 };
    let frame: Option<Arc<Vec<Handle<Mesh>>>> = if a.kf.is_empty() {
        None
    } else {
        match library.kf_clip(model, &a.kf, looping, rig.is_some(), meshes) {
            Some(Some(clip)) => {
                // 44F7B0: KF time = gameplay time x KF length / length.
                let d = clip.duration.max(1.0 / FRAME_RATE);
                if a.kind == "MOVE" {
                    ch.phase += dt * rate_move * d / length;
                }
                let t = if a.kind == "MOVE" { ch.phase } else { elapsed * d / length };
                let f = crate::effects::anim::frame_at(clip.frames.len(), looping, t);
                clip.frames.get(f).map(|l| Arc::new(l.clone()))
            }
            _ => None,
        }
    };
    let Some(frame) = frame else { return ch.drawn };
    ch.drawn = true;
    for (i, (part, mesh)) in parts.iter().zip(frame.iter()).enumerate() {
        if own_mesh.get(i).copied().unwrap_or(false) {
            continue;
        }
        if let Ok((mut m, _, _)) = parts_q.get_mut(*part)
            && m.0 != *mesh
        {
            m.0 = mesh.clone();
        }
    }
    true
}

#[allow(clippy::too_many_arguments)]
pub fn sync_actors(
    mut commands: Commands,
    battle: Option<Res<Battle>>,
    library: Option<ResMut<Library>>,
    fx: Option<Res<crate::effects::FxData>>,
    mut views: Query<(Entity, &mut ActorView, &mut Transform, &Visibility)>,
    mut parts_q: Query<(&mut Mesh3d, &mut Transform, &mut Visibility), Without<ActorView>>,
    mut trailers: Query<(&mut Transform, &mut Visibility), (Without<ActorView>, Without<Mesh3d>)>,
    mut mats_q: Query<&mut MeshMaterial3d<OriginalMaterial>, Without<ActorView>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OriginalMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut known: Local<HashMap<ActorId, Entity>>,
    object_animations: Option<Res<crate::session::ObjectAnimations>>,
    time: Res<Time>,
    mut log_timer: Local<f32>,
) {
    *log_timer += time.delta_secs();
    let log_now = *log_timer >= 2.0 && std::env::var_os("AA_BEVY_FX_STATS").is_some();
    if *log_timer >= 2.0 {
        *log_timer = 0.0;
    }
    let (Some(battle), Some(mut library)) = (battle, library) else { return };
    let world = &battle.world;
    let object = |id: ActorId| object_animations.as_ref().and_then(|o| o.0.get(&id)).map(|(n, t)| (n.as_str(), *t));
    let alpha = tick_fraction(&battle);
    let tables = fx.as_ref().map(|f| f.0.clone());
    // An ability used at an actor or at a ground point (a bombard) plays the
    // same fire animation.
    let fired: HashMap<ActorId, u32> = battle
        .frame_events
        .iter()
        .filter_map(|e| match e {
            aa_play::Event::Fired { shooter, weapon, .. } | aa_play::Event::FiredAt { shooter, weapon, .. } => Some((*shooter, *weapon)),
            _ => None,
        })
        .collect();
    let log_characters = std::env::var_os("AA_BEVY_FX_STATS").is_some();
    // Update and remove existing views.
    for (entity, mut view, mut transform, visibility) in &mut views {
        let Some(actor) = world.actors.get(view.id as usize).filter(|a| !a.removed) else {
            if let Some(t) = view.look.trailer.take() {
                commands.entity(t.entity).despawn();
            }
            commands.entity(entity).despawn();
            known.remove(&view.id);
            continue;
        };
        let view = &mut *view;
        if !view.look.setup
            && let Some(tables) = tables.as_deref()
        {
            set_up(&mut view.look, actor, world, tables, &mut library, &mut commands, &mut meshes, &mut materials, &mut images, view.rig.is_some());
        }
        if view.tick != world.tick {
            view.prev = view.cur;
            view.cur = (actor.pos, actor.facing);
            let (origin, up) = placement(world, actor);
            view.origin = (view.origin.1, origin);
            view.up = (view.up.1, up);
            view.tick = world.tick;
            if let Some(r) = view.look.running.as_mut() {
                r.tick(origin);
            }
        }
        let mut turn = (view.cur.1 - view.prev.1).rem_euclid(std::f32::consts::TAU);
        if turn > std::f32::consts::PI {
            turn -= std::f32::consts::TAU;
        }
        let facing = view.prev.1 + turn * alpha;
        // The ground tilt, interpolated like position and facing.
        let origin = view.origin.0.lerp(view.origin.1, alpha);
        let up = view.up.0.lerp(view.up.1, alpha).normalize_or(Vec3::Z);
        let flat = Vec3::new(facing.cos(), facing.sin(), 0.0);
        let forward = (flat - up * flat.dot(up)).normalize_or(flat);
        let model = view::model_matrix(origin, forward, up);
        *transform = view::render_transform(model);
        let drawn_by_character = match view.look.character.as_mut() {
            Some(ch) => animate_character(ch, actor, world, fired.get(&actor.id).copied(), log_characters, &mut view.look.hidden, view.rig.as_deref(), &view.look.model, &view.parts, &view.look.own_mesh, &mut library, &mut meshes, &mut parts_q),
            None => false,
        };
        if !drawn_by_character && let Some(rig) = &view.rig {
            if let Some((clip, t, mode)) = rig.clip_for(actor.id, actor.activity, actor.activity_since, world.time, death_clip(world, actor), object(actor.id)) {
                if let Some(frame) = rig.frame(clip, t, mode, &mut meshes) {
                    for (i, (part, mesh)) in view.parts.iter().zip(frame.iter()).enumerate() {
                        if view.look.own_mesh.get(i).copied().unwrap_or(false) {
                            continue;
                        }
                        if let Ok((mut m, _, _)) = parts_q.get_mut(*part) {
                            if m.0 != *mesh {
                                m.0 = mesh.clone();
                            }
                        }
                    }
                }
            }
        }
        // Buildings: the aspect for what they do, from its own model.
        if view.look.building
            && let Some(tables) = tables.as_deref()
            && let Some(fx) = tables.actors.get(&world.data.def(actor.def).id.to_ascii_lowercase())
        {
            // The aspect of the active stance (45E620); one with the normal
            // model and only a skin (aspect_disabled) keeps the normal parts
            // and changes their texture.
            let stance = if fx.view.stances.is_empty() { Some(building_aspect(actor.activity)) } else { stance_aspect(&fx.view.stances, world, actor) };
            let stance = stance.unwrap_or("aspect_normal");
            let normal_model = aspect_model(fx, "aspect_normal");
            let skin = fx.view.aspects.iter().find(|a| a.ids == stance && a.ids != "aspect_normal").filter(|a| a.model.is_none() || a.model == normal_model).and_then(|a| a.skin.clone());
            let want = if skin.is_some() || fx.view.aspects.iter().all(|a| a.ids != stance) { "aspect_normal" } else { stance };
            let normal_parts = view.look.aspect.as_ref().is_none_or(|a| a.ids == "aspect_normal");
            if normal_parts && skin != view.look.skin {
                let normal_skin = fx.view.aspects.iter().find(|a| a.ids == "aspect_normal").and_then(|a| a.skin.clone());
                let team = view.look.team;
                let list = match (&skin, &normal_skin, view.look.aspect.as_ref()) {
                    (Some(s), Some(n), Some(a)) => Some(library.skinned_anim(&a.model, team, n, s, &mut materials, &mut images)),
                    (Some(s), Some(n), None) => library.skinned(&view.look.model, team, n, s, &mut materials, &mut images),
                    (_, _, Some(a)) => Some(Arc::new(a.model.parts.iter().map(|(_, m)| m.clone()).collect())),
                    (_, _, None) => library.models.get(&(view.look.model.to_ascii_lowercase(), team)).cloned().flatten().map(|p| Arc::new(p.iter().map(|(_, m)| m.clone()).collect())),
                };
                if let Some(list) = list {
                    for (part, m) in view.parts.iter().zip(list.iter()) {
                        if let Ok(mut mm) = mats_q.get_mut(*part) {
                            mm.0 = m.clone();
                        }
                    }
                    view.look.skin = skin;
                }
            }
            let path = aspect_model(fx, want).or_else(|| (want != "aspect_normal").then(|| aspect_model(fx, "aspect_normal")).flatten());
            // An aspect with its own timeline (unpacking, packing, death) that
            // is still decoding: nothing is drawn meanwhile, not the normal
            // model (the finished camp would stand there before the unpack).
            let own_model = want != "aspect_normal" && aspect_model(fx, want).is_some_and(|m| Some(m) != aspect_model(fx, "aspect_normal"));
            if own_model && view.look.aspect.as_ref().is_none_or(|a| a.ids != want) {
                for p in &view.parts {
                    if let Ok((_, _, mut v)) = parts_q.get_mut(*p)
                        && *v != Visibility::Hidden
                    {
                        *v = Visibility::Hidden;
                    }
                }
            }
            if view.look.aspect.as_ref().is_none_or(|a| a.ids != want)
                && let Some(path) = path
                && let Some(Some(m)) = library.animated.get(&path, view.look.team, &mut meshes, &mut materials, &mut images)
                && (want != "aspect_normal" || m.frames > 1 || view.look.aspect.is_some())
            {
                for p in view.parts.drain(..) {
                    commands.entity(p).despawn();
                }
                for (frames, material) in m.parts.iter() {
                    if let Some(first) = frames.first() {
                        view.parts.push(commands.spawn((Mesh3d(first.clone()), MeshMaterial3d(material.clone()), ChildOf(entity))).id());
                    }
                }
                let normal = want == "aspect_normal";
                if log_characters {
                    info!("aspect: actor {} {} {:?} -> {want} ({path}) {} frames, start {:.2} now {:.2}, at {:?}", actor.id, world.data.def(actor.def).id, actor.activity, m.frames, actor.activity_since, world.time, actor.pos);
                }
                view.look.aspect = Some(AspectPlay { ids: want.to_owned(), start: if normal { 0.0 } else { actor.activity_since }, looping: normal && m.looping, model: m, shown: 0 });
                view.look.running = None;
                view.look.equipment.clear();
                // The new parts wear their own textures.
                view.look.skin = None;
            }
            if let Some(a) = view.look.aspect.as_mut() {
                let f = crate::effects::anim::frame_at(a.model.frames, a.looping, world.time - a.start);
                if f != a.shown {
                    a.shown = f;
                    for (part, (frames, _)) in view.parts.iter().zip(a.model.parts.iter()) {
                        if let Some(h) = frames.get(f).or(frames.last())
                            && let Ok((mut m, _, _)) = parts_q.get_mut(*part)
                        {
                            m.0 = h.clone();
                        }
                    }
                }
            }
        }
        // Tires and treads (453600 / 73D1E0).
        if let Some(r) = view.look.running.as_mut() {
            let changes = r.update(model);
            if log_now && actor.activity == Activity::Moving {
                info!("view: actor {} moving: {}", actor.id, r.state());
            }
            for (i, tf) in changes.transforms {
                if let Some(&part) = view.parts.get(i)
                    && let Ok((_, mut t, _)) = parts_q.get_mut(part)
                {
                    *t = tf;
                }
            }
            for (i, h) in changes.meshes {
                if let Some(&part) = view.parts.get(i)
                    && let Ok((mut m, _, _)) = parts_q.get_mut(part)
                {
                    m.0 = h;
                }
            }
        }
        // Turrets turned to the sim's yaw, raised by the view's own pitch
        // (453130 / 4521D0).
        let frame_dt = if battle.paused { 0.0 } else { time.delta_secs() * battle.speed };
        let tdefs = &world.data.def(actor.def).turrets;
        for (i, t) in view.look.turrets.iter_mut().enumerate() {
            let Some(t) = t else { continue };
            let rel = view::turret_turn(world, actor.id, i, alpha).unwrap_or(0.0);
            if let Some(td) = tdefs.get(i) {
                // The pitch toward the combat target from the turret's pivot.
                let want = actor.combat.target.and_then(|tg| world.actors.get(tg as usize)).map(|tg| {
                    let pivot = model.transform_point3(t.pivot());
                    let d = tg.pos - pivot;
                    d.z.atan2(d.truncate().length().max(1e-4))
                });
                t.step_pitch(td, want, frame_dt);
            }
            let pitch_moved = (t.pitch - t.shown_pitch).abs() > 1e-4;
            if log_now && actor.combat.target.is_some() {
                let target = actor.combat.target.and_then(|t| world.actors.get(t as usize)).map(|t| t.pos.truncate() - actor.pos.truncate());
                let want = target.map(|d| (d.y.atan2(d.x) - actor.facing).to_degrees().rem_euclid(360.0));
                info!("turret: actor {} {} turret {i} at {:.1} deg from the hull, target at {:?} deg, {:?}", actor.id, world.data.def(actor.def).id, rel.to_degrees().rem_euclid(360.0), want, actor.activity);
            }
            if (rel - t.shown).abs() > 1e-4 || t.shown.is_nan() || pitch_moved {
                t.shown = rel;
                t.shown_pitch = t.pitch;
                let tf = t.transform(rel, t.pitch);
                for &p in &t.parts {
                    if let Some(&part) = view.parts.get(p)
                        && let Ok((_, mut pt, _)) = parts_q.get_mut(part)
                    {
                        *pt = tf;
                    }
                }
            }
        }
        // An aircraft in its hangar: every part hidden (the fog system owns
        // the root's visibility); released, the parts come back and the
        // equipment state below is applied again.
        let hangar = in_hangar(world, actor.id);
        if hangar || view.look.in_hangar {
            for p in &view.parts {
                if let Ok((_, _, mut v)) = parts_q.get_mut(*p) {
                    let want = if hangar { Visibility::Hidden } else { Visibility::Inherited };
                    if *v != want {
                        *v = want;
                    }
                }
            }
            if !hangar {
                view.look.applied = None;
            }
            view.look.in_hangar = hangar;
        }
        // Equipment shown / hidden (FXC_Equipment).
        if !view.look.in_hangar && view.look.applied.as_ref() != Some(&view.look.hidden) && !view.look.equipment.is_empty() {
            for (i, eq) in view.look.equipment.iter().enumerate() {
                let Some(eq) = eq else { continue };
                if let Some(&part) = view.parts.get(i)
                    && let Ok((_, _, mut v)) = parts_q.get_mut(part)
                {
                    *v = if view.look.hidden.contains(eq) { Visibility::Hidden } else { Visibility::Inherited };
                }
            }
            view.look.applied = Some(view.look.hidden.clone());
        }
        // The towed trailer follows the truck's Hitch (45CC40).
        if let Some(t) = view.look.trailer.as_mut() {
            let h = model.transform_point3(t.truck_hitch);
            let moved = t.state.mirror(h, flat.truncate(), |p| world.map.height_at(p), battle.paused);
            let m = t.state.matrix();
            if let Ok((mut tf, mut vis)) = trailers.get_mut(t.entity) {
                *tf = view::render_transform(m);
                let dead = matches!(actor.activity, Activity::Dying | Activity::Dead);
                *vis = if dead { Visibility::Hidden } else { *visibility };
            }
            if moved && let Some(r) = t.running.as_mut() {
                let changes = r.update(m);
                for (i, tf) in changes.transforms {
                    if let Some(&part) = t.parts.get(i)
                        && let Ok((_, mut pt, _)) = parts_q.get_mut(part)
                    {
                        *pt = tf;
                    }
                }
            }
        }
    }
    // Spawn views for new actors.
    let spawn_started = std::time::Instant::now();
    let new: Vec<ActorId> = world.actors.iter().filter(|a| !a.removed && !known.contains_key(&a.id)).map(|a| a.id).collect();
    let new_count = new.len();
    for (n, actor) in new.iter().map(|&id| &world.actors[id as usize]).enumerate() {
        // Loading a model is slow the first time: past a frame budget the
        // rest wait for the next frames (a battle's first actors appear
        // behind the loading screen).
        if n > 0 && spawn_started.elapsed().as_secs_f32() > 0.008 {
            break;
        }
        let def = world.data.def(actor.def);
        if def.invisible {
            continue;
        }
        let Some(model) = def.model.clone() else { continue };
        let team = team_of(world, actor);
        let (origin, up) = placement(world, actor);
        let flat = Vec3::new(actor.facing.cos(), actor.facing.sin(), 0.0);
        let forward = (flat - up * flat.dot(up)).normalize_or(flat);
        let transform = view::render_transform(view::model_matrix(origin, forward, up));
        let rig = library.rig(&model, team, &mut meshes, &mut materials, &mut images);
        let mut parts = Vec::new();
        let parent = commands.spawn((transform, Visibility::default())).id();
        let mut extras = None;
        if let Some(rig) = &rig {
            let (clip, t, mode) = rig.clip_for(actor.id, actor.activity, actor.activity_since, world.time, death_clip(world, actor), object(actor.id)).unwrap_or((0, 0.0, PlaybackMode::Clamp));
            if let Some(frame) = rig.frame(clip, t, mode, &mut meshes) {
                for (mesh, material) in frame.iter().zip(&rig.materials) {
                    parts.push(commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), ChildOf(parent))).id());
                }
            }
            extras = rig.extras.clone();
        } else if let Some(list) = library.static_model(&model, team, &mut meshes, &mut materials, &mut images) {
            for (mesh, material) in list.iter() {
                parts.push(commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), ChildOf(parent))).id());
            }
            extras = library.extras_of(&model);
        }
        let look = Look { model: model.clone(), team, extras, ..Default::default() };
        commands.entity(parent).insert(ActorView {
            id: actor.id,
            rig,
            parts,
            prev: (actor.pos, actor.facing),
            cur: (actor.pos, actor.facing),
            origin: (origin, origin),
            up: (up, up),
            tick: world.tick,
            look,
        });
        known.insert(actor.id, parent);
    }
    let spawn_ms = spawn_started.elapsed().as_secs_f32() * 1000.0;
    if spawn_ms > 20.0 && std::env::var_os("AA_BEVY_FX_FRAMES").is_some() {
        info!("battle_view: {new_count} new actor views in {spawn_ms:.0} ms");
    }
    for f in library.failures.drain(..) {
        warn!("{f}");
    }
    for f in library.animated.failures.drain(..) {
        warn!("{f}");
    }
}

/// The Unpack placement ghost (SharedUIStates placement 0x6895B0; ghost
/// list 0x693AE0 / 0x693D10): the building's model in its kingdom's colour,
/// drawn through AVar VFXGhosting `ghost_vfx` (VFXAlphaModifier alpha 0.5),
/// and while the spot is illegal also AVar VFXIllegalPlacement
/// `illegal_placement_vfx` (VFXTintModifier colour 1,0,0, instantaneous,
/// 0x693A10 / 0x693A80). A preview: no shadow, no fog, no ActorView (not
/// picked).
#[derive(Default)]
pub struct GhostView {
    key: Option<(aa_play::DefId, [u8; 3])>,
    entity: Option<Entity>,
    parts: Vec<Entity>,
    /// Per part: the ghost material and the tinted one.
    plain: Vec<Handle<OriginalMaterial>>,
    tinted: Vec<Handle<OriginalMaterial>>,
    shown_tint: bool,
    shown: Option<(Vec3, f32)>,
}

#[allow(clippy::too_many_arguments)]
pub fn draw_ghost(
    mut commands: Commands,
    ghost: Option<Res<crate::ui::placement::Ghost>>,
    battle: Option<Res<Battle>>,
    library: Option<ResMut<Library>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OriginalMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut state: Local<GhostView>,
    mut roots: Query<(&mut Transform, &mut Visibility), Without<ActorView>>,
    mut mats: Query<&mut MeshMaterial3d<OriginalMaterial>, Without<ActorView>>,
) {
    let (Some(battle), Some(mut library)) = (battle, library) else { return };
    if battle.is_added() {
        // A new battle: the last one's ghost went with its world.
        *state = GhostView::default();
    }
    let want = ghost.as_ref().and_then(|g| g.model.clone());
    let Some(g) = want else {
        if let Some(e) = state.entity
            && let Ok((_, mut v)) = roots.get_mut(e)
            && *v != Visibility::Hidden
        {
            *v = Visibility::Hidden;
        }
        return;
    };
    let world = &battle.world;
    let team = world.kingdoms.get(g.kingdom as usize).filter(|k| !k.independent).map_or(NEUTRAL_TEAM, |k| k.color);
    let key = (g.def, team);
    if state.key != Some(key) {
        if let Some(e) = state.entity.take() {
            commands.entity(e).despawn();
        }
        state.parts.clear();
        state.plain.clear();
        state.tinted.clear();
        state.shown = None;
        state.key = None;
        let Some(model) = world.data.def(g.def).model.clone() else { return };
        let Some(parts) = library.static_model(&model, team, &mut meshes, &mut materials, &mut images) else { return };
        let parent = commands.spawn((Transform::default(), Visibility::Hidden)).id();
        for (mesh, material) in parts.iter() {
            let Some(base) = materials.get(material).cloned() else { continue };
            // ghost_vfx: the whole model at alpha 0.5; no fog over a preview.
            let mut plain = base;
            plain.base.alpha_mode = AlphaMode::Blend;
            let a = plain.base.base_color.alpha();
            plain.base.base_color.set_alpha(a * g.alpha);
            plain.extension.params.sun.w = 1.0;
            // illegal_placement_vfx: TINT colour 1,0,0 (KKC_VFXMaterialModifier
            // 0x449920 -> 0x717E40, lirp off, pct 1: factor = lerp(white,
            // colour, pct); 0x716620 multiplies every material's ambient,
            // diffuse, specular and emissive by it). A multiply, not an
            // added glow: only the red channel is left, alpha unchanged.
            const TINT: Vec4 = Vec4::new(1.0, 0.0, 0.0, 1.0);
            let mut tinted = plain.clone();
            let c = tinted.base.base_color.to_linear();
            tinted.base.base_color = Color::LinearRgba(LinearRgba::new(c.red * TINT.x, c.green * TINT.y, c.blue * TINT.z, c.alpha));
            let e = tinted.base.emissive;
            tinted.base.emissive = LinearRgba::new(e.red * TINT.x, e.green * TINT.y, e.blue * TINT.z, e.alpha);
            let p = &mut tinted.extension.params;
            p.ambient = Vec4::new(p.ambient.x * TINT.x, p.ambient.y * TINT.y, p.ambient.z * TINT.z, p.ambient.w);
            p.diffuse = Vec4::new(p.diffuse.x * TINT.x, p.diffuse.y * TINT.y, p.diffuse.z * TINT.z, p.diffuse.w);
            p.emissive = Vec4::new(p.emissive.x * TINT.x, p.emissive.y * TINT.y, p.emissive.z * TINT.z, p.emissive.w);
            let plain = materials.add(plain);
            let tinted = materials.add(tinted);
            let child = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(plain.clone()), bevy::light::NotShadowCaster, ChildOf(parent))).id();
            state.parts.push(child);
            state.plain.push(plain);
            state.tinted.push(tinted);
        }
        state.entity = Some(parent);
        state.key = Some(key);
        state.shown_tint = false;
    }
    let Some(e) = state.entity else { return };
    if state.shown != Some((g.at, g.facing)) {
        let flat = Vec3::new(g.facing.cos(), g.facing.sin(), 0.0);
        let tf = view::render_transform(view::model_matrix(g.at, flat, Vec3::Z));
        if let Ok((mut t, mut v)) = roots.get_mut(e) {
            *t = tf;
            *v = Visibility::Inherited;
            state.shown = Some((g.at, g.facing));
        }
    } else if let Ok((_, mut v)) = roots.get_mut(e)
        && *v == Visibility::Hidden
    {
        *v = Visibility::Inherited;
    }
    if state.shown_tint != g.illegal {
        let list = if g.illegal { &state.tinted } else { &state.plain };
        for (part, m) in state.parts.iter().zip(list.iter()) {
            if let Ok(mut mm) = mats.get_mut(*part) {
                mm.0 = m.clone();
            }
        }
        state.shown_tint = g.illegal;
    }
}