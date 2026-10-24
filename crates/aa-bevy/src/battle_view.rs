//! Draws the aa-play battle world: every actor with its original model, the
//! original animation clip for what it is doing, its kingdom's team colour,
//! and smooth motion between the 16 Hz ticks. Owner: lead.
//!
//! Animation frames are sampled once per (rig, clip, frame) at FRAME_RATE and
//! cached, so any number of soldiers playing the same clip share meshes.
use crate::scene::{NEUTRAL_TEAM, kind, model_mesh, standard};
use crate::look::OriginalMaterial;
use crate::model_fixes::ModelFixes;
use crate::session::{Battle, tick_fraction};
use crate::textures::{self, Kind};
use aa_animation::{AnimatedAsset, PlaybackMode};
use aa_assets::AssetStore;
use aa_play::{Activity, ActorId, World};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Animation sampling rate for the frame cache.
const FRAME_RATE: f32 = 20.0;

pub struct BattleViewPlugin;
impl Plugin for BattleViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_actors.run_if(in_state(crate::session::AppState::Battle)));
    }
}

/// Models, rigs, textures and materials loaded on first use.
#[derive(Resource)]
pub struct Library {
    store: AssetStore,
    fixes: Arc<ModelFixes>,
    /// Lower-case original NIF path -> rig file.
    rig_files: HashMap<String, PathBuf>,
    models: HashMap<(String, [u8; 3]), Option<Arc<Vec<(Handle<Mesh>, Handle<OriginalMaterial>)>>>>,
    rigs: HashMap<(String, [u8; 3]), Option<Arc<Rig>>>,
    textures: HashMap<(String, Kind), Option<Handle<Image>>>,
    pub failures: Vec<String>,
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
        Self { store, fixes, rig_files, models: HashMap::new(), rigs: HashMap::new(), textures: HashMap::new(), failures: Vec::new() }
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

    fn static_model(
        &mut self,
        path: &str,
        team: [u8; 3],
        assets: &mut Assets<Mesh>,
        materials: &mut Assets<OriginalMaterial>,
        images: &mut Assets<Image>,
    ) -> Option<Arc<Vec<(Handle<Mesh>, Handle<OriginalMaterial>)>>> {
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
                for mesh in model.meshes.iter().filter(|m| !m.indices.is_empty()) {
                    let texture = mesh.material.texture.clone().and_then(|t| self.texture(&t, kind(&mesh.material, team), images));
                    parts.push((assets.add(model_mesh(mesh)), materials.add(standard(&mesh.material, texture))));
                }
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

    fn rig(&mut self, path: &str, team: [u8; 3], materials: &mut Assets<OriginalMaterial>, images: &mut Assets<Image>) -> Option<Arc<Rig>> {
        let key = (path.to_ascii_lowercase().replace('\\', "/"), team);
        if let Some(r) = self.rigs.get(&key) {
            return r.clone();
        }
        let rig = self.rig_files.get(&key.0).cloned().and_then(|file| match AnimatedAsset::load(&file) {
            Ok(mut asset) => {
                // Same material source as the client's animation path: the static model.
                if let Ok(mut model) = self.store.load_model(&asset.source).map_err(|e| e.to_string()).and_then(|m| m.prepare().map_err(|e| e.to_string())) {
                    let _ = self.fixes.apply(&self.store, &mut model);
                    if model.meshes.len() == asset.meshes.len() {
                        for (rig, mesh) in asset.meshes.iter_mut().zip(&model.meshes) {
                            rig.material = mesh.material.clone();
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
                let find = |names: &[&str]| {
                    asset.clips.iter().position(|c| names.iter().any(|n| c.name.eq_ignore_ascii_case(n)))
                };
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
                Some(Arc::new(Rig { asset, materials: mats, frames: Default::default(), idle, moving, attack, deaths }))
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

/// One drawn actor: a parent with its transform and one child per model mesh.
#[derive(Component)]
pub struct ActorView {
    pub id: ActorId,
    rig: Option<Arc<Rig>>,
    parts: Vec<Entity>,
    prev: (Vec3, f32),
    cur: (Vec3, f32),
    tick: u64,
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

#[allow(clippy::too_many_arguments)]
pub fn sync_actors(
    mut commands: Commands,
    battle: Option<Res<Battle>>,
    library: Option<ResMut<Library>>,
    mut views: Query<(Entity, &mut ActorView, &mut Transform)>,
    mut part_meshes: Query<&mut Mesh3d>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OriginalMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut known: Local<HashMap<ActorId, Entity>>,
    object_animations: Option<Res<crate::session::ObjectAnimations>>,
) {
    let (Some(battle), Some(mut library)) = (battle, library) else { return };
    let world = &battle.world;
    let object = |id: ActorId| object_animations.as_ref().and_then(|o| o.0.get(&id)).map(|(n, t)| (n.as_str(), *t));
    let alpha = tick_fraction(&battle);
    // Update and remove existing views.
    for (entity, mut view, mut transform) in &mut views {
        let Some(actor) = world.actors.get(view.id as usize).filter(|a| !a.removed) else {
            commands.entity(entity).despawn();
            known.remove(&view.id);
            continue;
        };
        if view.tick != world.tick {
            view.prev = view.cur;
            view.cur = (actor.pos, actor.facing);
            view.tick = world.tick;
        }
        let pos = view.prev.0.lerp(view.cur.0, alpha);
        let mut turn = (view.cur.1 - view.prev.1).rem_euclid(std::f32::consts::TAU);
        if turn > std::f32::consts::PI {
            turn -= std::f32::consts::TAU;
        }
        let facing = view.prev.1 + turn * alpha;
        transform.translation = to_render(pos);
        transform.rotation = Quat::from_rotation_y(render_yaw(facing));
        if let Some(rig) = &view.rig {
            if let Some((clip, t, mode)) = rig.clip_for(actor.id, actor.activity, actor.activity_since, world.time, death_clip(world, actor), object(actor.id)) {
                if let Some(frame) = rig.frame(clip, t, mode, &mut meshes) {
                    for (part, mesh) in view.parts.iter().zip(frame.iter()) {
                        if let Ok(mut m) = part_meshes.get_mut(*part) {
                            if m.0 != *mesh {
                                m.0 = mesh.clone();
                            }
                        }
                    }
                }
            }
        }
    }
    // Spawn views for new actors.
    let new: Vec<ActorId> = world.actors.iter().filter(|a| !a.removed && !known.contains_key(&a.id)).map(|a| a.id).collect();
    for actor in new.iter().map(|&id| &world.actors[id as usize]) {
        let def = world.data.def(actor.def);
        if def.invisible {
            continue;
        }
        let Some(model) = def.model.clone() else { continue };
        let team = team_of(world, actor);
        let transform = Transform::from_translation(to_render(actor.pos)).with_rotation(Quat::from_rotation_y(render_yaw(actor.facing)));
        let rig = library.rig(&model, team, &mut materials, &mut images);
        let mut parts = Vec::new();
        let parent = commands.spawn((transform, Visibility::default())).id();
        if let Some(rig) = &rig {
            let (clip, t, mode) = rig.clip_for(actor.id, actor.activity, actor.activity_since, world.time, death_clip(world, actor), object(actor.id)).unwrap_or((0, 0.0, PlaybackMode::Clamp));
            if let Some(frame) = rig.frame(clip, t, mode, &mut meshes) {
                for (mesh, material) in frame.iter().zip(&rig.materials) {
                    parts.push(commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), ChildOf(parent))).id());
                }
            }
        } else if let Some(list) = library.static_model(&model, team, &mut meshes, &mut materials, &mut images) {
            for (mesh, material) in list.iter() {
                parts.push(commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), ChildOf(parent))).id());
            }
        }
        commands.entity(parent).insert(ActorView { id: actor.id, rig, parts, prev: (actor.pos, actor.facing), cur: (actor.pos, actor.facing), tick: world.tick });
        known.insert(actor.id, parent);
    }
    for f in library.failures.drain(..) {
        warn!("{f}");
    }
}
