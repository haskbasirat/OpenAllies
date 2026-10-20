//! The original battle effects, run from the original FX data (see
//! `data`): every `aa_play::Event::Fired` runs the weapon's AbilityDefinition
//! FXC lists (WhenStart and WhenDo: muzzle flash at the gun bone, tracers,
//! impacts on the target, the attack sound), projectiles fly with their
//! original model and run their WhenDestroyed list on landing (explosions,
//! scorch marks, explosion sound), deaths run the View WhenDestroyed list
//! (death sounds, building destruction), health thresholds run the
//! BodyComponent WhenHealth list (smoke, fire, debris, death explosions),
//! movers run WhenStartMoving / WhenStopMoving (dust, exhaust, engine
//! sounds), new actors their WhenCreated list, and dead actors with an
//! `aspect_death` model show it (wrecks, rubble) until removed.
//! Owner: effects and sound. Behaviour-faithful, not byte-exact: the FXC
//! semantics follow template_fx.tgi and the native FX graph ports in
//! aa-runtime (fx_graph*, view_fx_*); unclear points are marked "compare".
pub mod bench;
pub mod bones;
pub mod data;
pub mod decal;
pub mod nif;
pub mod player;
pub mod render;

use crate::audio::{Place, Sfx, SfxQueue};
use crate::battle_view::{ActorView, render_yaw, to_render};
use crate::look::OriginalMaterial;
use crate::session::{AppState, Battle};
use aa_play::{Activity, ActorId, Event, World};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use data::{Fxc, FxTables, VfxDef};
use nif::NifTable;
use player::{Player, Rng};
use render::{FxMaterial, Quad, View};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Rendered effect quads per frame.
const MAX_QUADS: usize = 24_000;
/// Live NIF effects (oldest temporary ones are dropped first).
const MAX_EFFECTS: usize = 1_500;
/// Newly emitted particles per frame.
const EMIT_BUDGET: usize = 6_000;
/// Temporary non-critical VFX farther than this from the camera target are
/// not created (the original discards off-screen temporary VFX unless
/// `never_discard_temporary`).
const DISCARD_DISTANCE: f32 = 220.0;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectsSet;

pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        render::add_render(app);
        app.add_plugins(crate::audio::SoundPlugin)
            .init_resource::<FxStats>()
            .add_systems(OnEnter(AppState::Battle), start_loading)
            .add_systems(OnExit(AppState::Battle), clear)
            .add_systems(Update, bench::spawn.run_if(in_state(AppState::Battle)))
            .add_systems(
                Update,
                (poll_loading, run)
                    .chain()
                    .in_set(EffectsSet)
                    .after(crate::session::step_battle)
                    .after(crate::battle_view::sync_actors)
                    .run_if(in_state(AppState::Battle)),
            );
    }
}

/// Frame cost of the effects (for the 100-soldier measurement).
#[derive(Resource, Default, Debug, Clone)]
pub struct FxStats {
    pub effects: usize,
    pub quads: usize,
    pub tracers: usize,
    pub micros: f32,
    pub peak_micros: f32,
    pub average_micros: f32,
    frames: u32,
    total: f32,
    window: (f32, u32, f32, f32),
    /// Window sums of the phases: scan, events, step, draw (us).
    phases: [f32; 4],
}

/// The original FX tables once loaded (shared with the sound side).
#[derive(Resource, Clone)]
pub struct FxData(pub Arc<FxTables>);

fn find_up(rel: &str) -> Option<PathBuf> {
    let mut starts = vec![];
    if let Ok(exe) = std::env::current_exe() {
        starts.push(exe);
    }
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    starts.iter().flat_map(|s| s.ancestors()).map(|a| a.join(rel)).find(|p| p.exists())
}

/// The reconstruction folder (extracted original files).
pub fn reconstruction_root() -> Option<PathBuf> {
    find_up("reconstruction/assets/converted").and_then(|p| p.parent()?.parent().map(PathBuf::from))
}

struct Loaded {
    tables: Arc<FxTables>,
    nif: &'static NifTable,
    layers: Vec<image::RgbaImage>,
    layer_index: HashMap<String, u16>,
    white: u16,
    reconstruction: PathBuf,
    bones: bones::Bones,
    notes: Vec<String>,
    seconds: f32,
}

#[derive(Resource)]
struct LoadTask(Arc<Mutex<Option<Result<Loaded, String>>>>);

fn start_loading(mut commands: Commands, battle: Option<Res<Battle>>, existing: Option<Res<FxState>>) {
    if existing.is_some() {
        return;
    }
    let Some(battle) = battle else {
        // The scene inserts Battle in its own OnEnter; try again next frame.
        commands.insert_resource(Retry);
        return;
    };
    spawn_loader(&mut commands, &battle);
}

#[derive(Resource)]
struct Retry;

fn spawn_loader(commands: &mut Commands, battle: &Battle) {
    let ids: Vec<String> = battle.world.data.defs.iter().map(|d| d.id.clone()).collect();
    let slot = Arc::new(Mutex::new(None));
    let out = slot.clone();
    std::thread::spawn(move || {
        let result = load(&ids).map_err(|e| format!("{e:#}"));
        *out.lock().unwrap() = Some(result);
    });
    commands.insert_resource(LoadTask(slot));
}

/// Texture keys of the effect table: converted PNG paths below
/// assets/converted/textures, `original:` Data paths, `embedded:` images the
/// extractor wrote next to the table.
fn load_layer(reconstruction: &Path, table_dir: &Path, key: &str) -> Option<image::RgbaImage> {
    let path = if let Some(name) = key.strip_prefix("embedded:") {
        Some(table_dir.join("textures").join(name))
    } else if let Some(rel) = key.strip_prefix("original:") {
        bones::find_file(&reconstruction.join("assets/original/Data"), rel)
    } else {
        let rel = key.replace('\\', "/");
        let stem = rel.rsplit_once('.').map_or(rel.as_str(), |(s, _)| s);
        bones::find_file(&reconstruction.join("assets/converted/textures"), &format!("{stem}.png"))
            .or_else(|| bones::find_file(&reconstruction.join("assets/converted/textures"), &rel))
            .or_else(|| bones::find_file(&reconstruction.join("assets/original/Data"), &rel))
    }?;
    image::open(path).ok().map(|i| i.to_rgba8())
}

fn load(ids: &[String]) -> anyhow::Result<Loaded> {
    let start = Instant::now();
    let reconstruction = reconstruction_root().ok_or_else(|| anyhow::anyhow!("the reconstruction folder was not found"))?;
    let install = reconstruction.parent().ok_or_else(|| anyhow::anyhow!("install root"))?.to_path_buf();
    let mut notes = Vec::new();
    let tables = Arc::new(data::load(&install, ids)?);
    // The wgpu client's extracted effect NIF table (see nif.rs).
    let table_path = find_up("assets-cache/aa-bevy/nif_effects.json").or_else(|| find_up("crates/aa-client/src/vfx/nif_effects.json"));
    let nif: NifTable = match table_path.as_ref().and_then(|p| std::fs::read(p).ok()) {
        Some(bytes) => serde_json::from_slice(&bytes)?,
        None => {
            notes.push("effect NIF table (nif_effects.json) not found: NIF effects are not drawn".into());
            NifTable { format: String::new(), textures: Vec::new(), effects: HashMap::new(), failures: HashMap::new() }
        }
    };
    let nif: &'static NifTable = Box::leak(Box::new(nif));
    let table_dir = table_path.as_ref().and_then(|p| p.parent()).map(Path::to_path_buf).unwrap_or_default();
    let mut layers = Vec::new();
    let mut layer_index = HashMap::new();
    for key in &nif.textures {
        let img = load_layer(&reconstruction, &table_dir, key).unwrap_or_else(|| {
            notes.push(format!("effect texture {key} not found"));
            image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 255, 255, 255]))
        });
        layer_index.insert(key.to_ascii_lowercase(), layers.len() as u16);
        layers.push(img);
    }
    // Projected textures (scorch marks, ground planes).
    for def in tables.vfx.values() {
        if def.category == "VFXPROJECTEDTEXTURE" && !def.texture.is_empty() {
            let key = def.texture.to_ascii_lowercase();
            if layer_index.contains_key(&key) {
                continue;
            }
            if let Some(img) = load_layer(&reconstruction, &table_dir, &def.texture) {
                layer_index.insert(key, layers.len() as u16);
                layers.push(img);
            }
        }
    }
    let white = layers.len() as u16;
    layers.push(image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 255, 255, 255])));
    let rig_dir = find_up("assets-cache/animation/RUSInfantry.rig.json").and_then(|p| p.parent().map(Path::to_path_buf));
    let mut bones = bones::Bones::new(rig_dir.as_deref(), reconstruction.join("assets/original/Data"));
    bones.preload_rigs();
    Ok(Loaded { tables, nif, layers, layer_index, white, reconstruction, bones, notes, seconds: start.elapsed().as_secs_f32() })
}

/// A target filter (`+infantry -building`) as required / forbidden bits.
fn parse_filter(text: &str, names: &[String]) -> (u32, u32) {
    let (mut req, mut forb) = (0u32, 0u32);
    for token in text.split([',', ' ', '\t']).filter(|t| !t.is_empty()) {
        let (neg, name) = match token.as_bytes()[0] {
            b'-' => (true, &token[1..]),
            b'+' => (false, &token[1..]),
            _ => (false, token),
        };
        if let Some(i) = names.iter().position(|n| n.eq_ignore_ascii_case(name)) {
            if neg {
                forb |= 1 << i;
            } else {
                req |= 1 << i;
            }
        }
    }
    (req, forb)
}

/// Where and on whom an FXC runs (the FX event parameters).
#[derive(Clone, Copy, Debug, Default)]
struct Ctx {
    actor: Option<ActorId>,
    target: Option<ActorId>,
    /// Render-space position of SELF when it is not an actor (a projectile).
    at: Option<Vec3>,
    /// Native position of the target at the event (when it is gone later).
    target_ground: Option<Vec3>,
    shooter_ground: Option<Vec3>,
}

/// Which list started a permanent effect (for stopping it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListKind {
    Other,
    Moving,
    Health,
    Created,
    /// WhenConstructed: stopped when the actor is destructed
    /// (FXC stop_on_destructed).
    Constructed,
}

struct LiveFx {
    player: Player,
    attach: Option<(ActorId, Option<String>)>,
    gyroscope: bool,
    owner: Option<ActorId>,
    list: ListKind,
    ids: String,
    permanent: bool,
    critical: bool,
    chain: Vec<Fxc>,
    ctx: Ctx,
}

struct Tracer {
    from: Vec3,
    dir: Vec3,
    dist: f32,
    speed: f32,
    length: f32,
    color: [f32; 4],
    t: f32,
}

struct Decal {
    /// Triangles lying on the drawn terrain (render space).
    verts: Vec<decal::Vert>,
    /// Texture layer (unlit decals) and Data path (lit decals).
    layer: u16,
    texture: String,
    /// VFXProjectedTextureLit: lit like the ground (the scene's material).
    lit: bool,
    /// decal_type order: terrain, scorch, track, generic, ui, shadow.
    order: u8,
    color: [f32; 3],
    additive: bool,
    age: f32,
    duration: f32,
    fade: [f32; 2],
    owner: Option<ActorId>,
    permanent: bool,
    list: ListKind,
}

/// Lit decals of one texture: one mesh drawn with the scene's terrain look.
struct LitBatch {
    entity: Entity,
    mesh: Handle<Mesh>,
}

fn decal_order(kind: &str) -> u8 {
    match kind {
        "terrain" => 0,
        "scorch" => 1,
        "track" => 2,
        "ui" => 4,
        "shadow" => 5,
        _ => 3,
    }
}

struct Delayed {
    at: f32,
    fxc: Fxc,
    ctx: Ctx,
    list: ListKind,
}

struct Flight {
    projectile: String,
    from: Vec3,
    to: Vec3,
    start: f32,
    end: f32,
    /// Peak height above the straight line (LOB projectiles).
    arc: f32,
    target: Option<ActorId>,
    shooter: ActorId,
    entity: Option<Entity>,
}

/// A static original model shown by the effects (wrecks, rubble, debris,
/// projectiles).
struct ModelFx {
    entity: Entity,
    owner: Option<ActorId>,
    /// Seconds left; None = until the owner is removed.
    life: Option<f32>,
    list: ListKind,
}

type Parts = Arc<Vec<(Handle<Mesh>, Handle<OriginalMaterial>)>>;

#[derive(Resource)]
pub struct FxState {
    pub tables: Arc<FxTables>,
    nif: &'static NifTable,
    layer_index: HashMap<String, u16>,
    white: u16,
    bones: bones::Bones,
    store: Option<aa_assets::AssetStore>,
    filters: HashMap<String, (u32, u32)>,
    time: f32,
    rng: Rng,
    seed: u32,
    effects: Vec<LiveFx>,
    tracers: Vec<Tracer>,
    decals: Vec<Decal>,
    delayed: Vec<Delayed>,
    flights: Vec<Flight>,
    models: Vec<ModelFx>,
    wrecks: HashMap<ActorId, Entity>,
    health: Vec<f32>,
    moving: Vec<bool>,
    seen: Vec<bool>,
    frequency: HashMap<(u32, usize), u32>,
    parts: HashMap<(String, [u8; 3]), Option<Parts>>,
    quads_mesh: Handle<Mesh>,
    decal_mesh: Handle<Mesh>,
    quads_entity: Entity,
    decal_entity: Entity,
    view: View,
    focus: Vec3,
    warned: std::collections::HashSet<String>,
    quads: Vec<Quad>,
    lit: HashMap<String, LitBatch>,
    /// The first battle frame: the map's own actors exist already built.
    first_frame: bool,
    /// Bone world matrices of this frame, by (actor, bone name hash).
    bone_cache: HashMap<(ActorId, u64), Mat4>,
    def_bones: Vec<Option<Arc<bones::Source>>>,
    def_fx: Vec<Option<Arc<data::ActorFx>>>,
    /// Decal meshes need rebuilding (set changed, fading, camera moved).
    decals_dirty: bool,
    decal_eye: Vec3,
    decal_time: f32,
}

fn poll_loading(
    mut commands: Commands,
    task: Option<Res<LoadTask>>,
    retry: Option<Res<Retry>>,
    battle: Option<Res<Battle>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<FxMaterial>>,
) {
    if retry.is_some() {
        if let Some(b) = battle.as_ref() {
            commands.remove_resource::<Retry>();
            spawn_loader(&mut commands, b);
        }
        return;
    }
    let Some(task) = task else { return };
    let Some(result) = task.0.lock().unwrap().take() else { return };
    commands.remove_resource::<LoadTask>();
    let loaded = match result {
        Ok(l) => l,
        Err(e) => {
            error!("effects and sound: the original FX data did not load: {e}");
            return;
        }
    };
    for n in &loaded.notes {
        warn!("effects: {n}");
    }
    info!(
        "effects: {} actors, {} VFX, {} audio, {} NIF effects, {} texture layers, loaded in {:.2} s",
        loaded.tables.actors.len(),
        loaded.tables.vfx.len(),
        loaded.tables.audio.len(),
        loaded.nif.effects.len(),
        loaded.layers.len(),
        loaded.seconds
    );
    let atlas = images.add(render::atlas_image(&loaded.layers));
    let quads_mesh = meshes.add(render::empty_mesh());
    let decal_mesh = meshes.add(render::empty_mesh());
    let quads_entity = commands
        .spawn((Mesh3d(quads_mesh.clone()), MeshMaterial3d(materials.add(FxMaterial { atlas: atlas.clone(), depth_bias: 0.0 })), Transform::default(), NoFrustumCulling, NotShadowCaster))
        .id();
    let decal_entity = commands
        .spawn((Mesh3d(decal_mesh.clone()), MeshMaterial3d(materials.add(FxMaterial { atlas, depth_bias: 0.0 })), Transform::default(), NoFrustumCulling, NotShadowCaster))
        .id();
    let store = aa_assets::AssetStore::open(&loaded.reconstruction).ok();
    commands.insert_resource(FxData(loaded.tables.clone()));
    commands.insert_resource(FxState {
        tables: loaded.tables,
        nif: loaded.nif,
        layer_index: loaded.layer_index,
        white: loaded.white,
        bones: loaded.bones,
        store,
        filters: HashMap::new(),
        time: 0.0,
        rng: Rng::new(0x5EED),
        seed: 1,
        effects: Vec::new(),
        tracers: Vec::new(),
        decals: Vec::new(),
        delayed: Vec::new(),
        flights: Vec::new(),
        models: Vec::new(),
        wrecks: HashMap::new(),
        health: Vec::new(),
        moving: Vec::new(),
        seen: Vec::new(),
        frequency: HashMap::new(),
        parts: HashMap::new(),
        quads_mesh,
        decal_mesh,
        quads_entity,
        decal_entity,
        view: View::default(),
        focus: Vec3::ZERO,
        warned: Default::default(),
        quads: Vec::new(),
        lit: HashMap::new(),
        first_frame: true,
        bone_cache: HashMap::new(),
        def_bones: Vec::new(),
        def_fx: Vec::new(),
        decals_dirty: true,
        decal_eye: Vec3::ZERO,
        decal_time: 0.0,
    });
}

fn clear(mut commands: Commands, state: Option<Res<FxState>>) {
    if let Some(s) = state {
        commands.entity(s.quads_entity).despawn();
        commands.entity(s.decal_entity).despawn();
        for m in &s.models {
            commands.entity(m.entity).despawn();
        }
        for f in &s.flights {
            if let Some(e) = f.entity {
                commands.entity(e).despawn();
            }
        }
        for b in s.lit.values() {
            commands.entity(b.entity).despawn();
        }
        for e in s.wrecks.values() {
            commands.entity(*e).despawn();
        }
    }
    commands.remove_resource::<FxState>();
    commands.remove_resource::<FxData>();
    commands.remove_resource::<LoadTask>();
}

/// Everything an FXC reads from the frame.
struct Frame<'a> {
    world: &'a World,
    /// The drawn terrain height (scene), render x/z -> y.
    ground: Option<&'a crate::camera::Ground>,
    /// Drawn actor transforms (battle_view), render space.
    drawn: &'a HashMap<ActorId, Mat4>,
    local: u32,
}

/// Bevy access for spawning model VFX.
struct Spawn<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<OriginalMaterial>,
    images: &'a mut Assets<Image>,
    sfx: &'a mut SfxQueue,
}

fn actor_matrix(frame: &Frame, id: ActorId) -> Option<Mat4> {
    if let Some(m) = frame.drawn.get(&id) {
        return Some(*m);
    }
    let a = frame.world.actor(id)?;
    Some(Mat4::from_rotation_translation(Quat::from_rotation_y(render_yaw(a.facing)), to_render(a.pos)))
}

fn death_clip(world: &World, id: ActorId) -> Option<&str> {
    let a = world.actors.get(id as usize)?;
    let index = world.combat.actor(id)?.death_clip?;
    world.data.def(a.def).death_clips.get(index).map(|c| c.id.as_str())
}

fn team_of(world: &World, a: &aa_play::Actor) -> [u8; 3] {
    a.kingdom.and_then(|k| world.kingdoms.get(k as usize)).filter(|k| !k.independent).map_or(crate::scene::NEUTRAL_TEAM, |k| k.color)
}

impl FxState {
    fn next_seed(&mut self) -> u32 {
        self.seed = self.seed.wrapping_add(0x9E37_79B9);
        self.seed
    }

    /// World matrix of an actor's bone (FXC `location`), or its origin.
    fn bone(&mut self, frame: &Frame, actor: ActorId, location: &str) -> Option<Mat4> {
        let base = actor_matrix(frame, actor)?;
        if location.is_empty() {
            return Some(base);
        }
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            location.hash(&mut h);
            (actor, h.finish())
        };
        if let Some(m) = self.bone_cache.get(&key) {
            return Some(*m);
        }
        let a = frame.world.actors.get(actor as usize)?;
        let def_id = a.def as usize;
        if self.def_bones.len() <= def_id {
            self.def_bones.resize(def_id + 1, None);
        }
        let source = match &self.def_bones[def_id] {
            Some(s) => s.clone(),
            None => {
                let s = match frame.world.data.def(a.def).model.as_deref() {
                    Some(model) => self.bones.source(model),
                    None => Arc::new(bones::Source::None),
                };
                self.def_bones[def_id] = Some(s.clone());
                s
            }
        };
        let tables = self.tables.clone();
        let name = tables.bones.get(location).map_or(location, String::as_str);
        let m = match source.node(name, actor, a.activity, a.activity_since, frame.world.time, death_clip(frame.world, actor)) {
            Some(m) => base * m,
            None => base,
        };
        self.bone_cache.insert(key, m);
        Some(m)
    }

    /// The FX lists of a def (cached by DefId).
    fn fx_of(&mut self, world: &World, def: aa_play::DefId) -> Option<Arc<data::ActorFx>> {
        let i = def as usize;
        if self.def_fx.len() <= i {
            self.def_fx.resize(i + 1, None);
        }
        if self.def_fx[i].is_none() {
            let fx = self.tables.actors.get(&world.data.def(def).id.to_ascii_lowercase()).cloned().unwrap_or_default();
            self.def_fx[i] = Some(Arc::new(fx));
        }
        self.def_fx[i].clone()
    }

    fn filter_ok(&mut self, fxc: &Fxc, frame: &Frame, target: Option<ActorId>) -> bool {
        if fxc.target_flags.is_empty() {
            return true;
        }
        let (req, forb) = match self.filters.get(&fxc.target_flags) {
            Some(f) => *f,
            None => {
                let f = parse_filter(&fxc.target_flags, &frame.world.data.target_types);
                self.filters.insert(fxc.target_flags.clone(), f);
                f
            }
        };
        let Some(t) = target.and_then(|t| frame.world.actors.get(t as usize)) else { return false };
        let flags = frame.world.data.def(t.def).target_flags;
        flags & req == req && flags & forb == 0
    }

    /// terrain_group of the cell under a render-space point.
    fn terrain_group(&self, world: &World, p: Vec3) -> Option<String> {
        let native = Vec2::new(p.x, -p.z);
        let (x, y) = world.map.cell_of(native)?;
        let v = *world.map.cell_variation.get((y * world.map.width + x) as usize)?;
        let id = world.map.variations.get(v as usize)?;
        let terrain = &world.data.terrain.find(id)?.terrain;
        self.tables.terrain_groups.get(&terrain.to_ascii_lowercase()).cloned()
    }

    /// The placement of `at` (render-space matrix) and what it attaches to.
    fn resolve(&mut self, fxc: &Fxc, ctx: &Ctx, frame: &Frame) -> Option<(Mat4, Option<(ActorId, Option<String>)>, bool)> {
        let ground = |p: Vec3| -> Vec3 {
            let n = Vec2::new(p.x, -p.z);
            Vec3::new(p.x, frame.world.map.height_at(n), p.z)
        };
        let loc = (!fxc.location.is_empty()).then(|| fxc.location.clone());
        match fxc.at.as_str() {
            "TARGET_THING" => {
                let t = ctx.target?;
                frame.world.actor(t)?;
                let m = self.bone(frame, t, &fxc.location)?;
                Some((m, fxc.attached.then_some((t, loc)), false))
            }
            "TARGET_GROUND" => {
                let p = match ctx.target.and_then(|t| actor_matrix(frame, t)) {
                    Some(m) => m.w_axis.truncate(),
                    None => to_render(ctx.target_ground?),
                };
                Some((Mat4::from_translation(ground(p)), None, true))
            }
            "SOURCE_GROUND" => {
                let p = match (ctx.at, ctx.actor.and_then(|a| actor_matrix(frame, a))) {
                    (Some(p), _) => p,
                    (None, Some(m)) => m.w_axis.truncate(),
                    _ => to_render(ctx.shooter_ground?),
                };
                Some((Mat4::from_translation(ground(p)), None, true))
            }
            "TARGET_HIT_POSITION" => {
                // compare: where the original puts the hit on the target's
                // bounds; here the target's centre height, toward the shooter.
                let tm = match ctx.target.and_then(|t| actor_matrix(frame, t)) {
                    Some(m) => m,
                    None => Mat4::from_translation(to_render(ctx.target_ground?)),
                };
                let mut p = tm.w_axis.truncate() + Vec3::Y * 1.0;
                let from = ctx.at.or_else(|| ctx.actor.and_then(|a| actor_matrix(frame, a)).map(|m| m.w_axis.truncate()));
                if let Some(f) = from {
                    let mut d = f - p;
                    d.y = 0.0;
                    p += d.normalize_or_zero() * 0.6;
                }
                Some((Mat4::from_translation(p), None, true))
            }
            "SOURCE_ABILITY_POSITION" => {
                let a = ctx.actor?;
                let m = self.bone(frame, a, "gun_ae_01")?;
                Some((Mat4::from_translation(m.w_axis.truncate()), None, true))
            }
            "MAP_CENTER" => {
                let w = frame.world.map.width as f32 * frame.world.map.cell_size * 0.5;
                let h = frame.world.map.height as f32 * frame.world.map.cell_size * 0.5;
                Some((Mat4::from_translation(to_render(frame.world.ground(Vec2::new(w, h)))), None, true))
            }
            _ => {
                // SELF
                if let Some(p) = ctx.at {
                    return Some((Mat4::from_translation(p), None, true));
                }
                let a = ctx.actor?;
                frame.world.actor(a)?;
                let m = self.bone(frame, a, &fxc.location)?;
                Some((m, fxc.attached.then_some((a, loc)), false))
            }
        }
    }

    /// Run one FXC (template_fx.tgi semantics).
    fn run_fxc(&mut self, fxc: &Fxc, ctx: Ctx, frame: &Frame, spawn: &mut Spawn, list: ListKind, now_delay: bool) {
        if fxc.frequency <= 0 {
            return;
        }
        if fxc.frequency > 1 {
            let key = (ctx.actor.unwrap_or(u32::MAX), fxc as *const Fxc as usize);
            let n = self.frequency.entry(key).or_insert(0);
            *n += 1;
            if *n % fxc.frequency as u32 != 0 {
                return;
            }
        }
        if !self.filter_ok(fxc, frame, ctx.target) {
            return;
        }
        if fxc.delay > 0.0 && !now_delay {
            self.delayed.push(Delayed { at: self.time + fxc.delay, fxc: fxc.clone(), ctx, list });
            return;
        }
        match fxc.kind.as_str() {
            "CREATE_VFX" => {
                if !fxc.terrain_group.is_empty() {
                    let Some((m, _, _)) = self.resolve(fxc, &ctx, frame) else { return };
                    let group = self.terrain_group(frame.world, m.w_axis.truncate());
                    if group.as_deref() != Some(fxc.terrain_group.as_str()) {
                        return;
                    }
                }
                self.create_vfx(fxc, ctx, frame, spawn, list);
            }
            "PLAY_SFX" => {
                if !fxc.terrain_group.is_empty() {
                    let Some((m, _, _)) = self.resolve(fxc, &ctx, frame) else { return };
                    if self.terrain_group(frame.world, m.w_axis.truncate()).as_deref() != Some(fxc.terrain_group.as_str()) {
                        return;
                    }
                }
                let owner = ctx.actor.and_then(|a| frame.world.actors.get(a as usize)).and_then(|a| a.kingdom);
                let place = match fxc.at.as_str() {
                    "SELF" | "TARGET_THING" if ctx.at.is_none() => {
                        let who = if fxc.at == "TARGET_THING" { ctx.target } else { ctx.actor };
                        match who {
                            Some(a) => Place::Object(a as u64),
                            None => return,
                        }
                    }
                    _ => match self.resolve(fxc, &ctx, frame) {
                        Some((m, _, _)) => {
                            let p = m.w_axis.truncate();
                            Place::Location(Vec3::new(p.x, -p.z, p.y))
                        }
                        None => return,
                    },
                };
                spawn.sfx.play.push(Sfx { ids: fxc.ids.clone(), place, owner, voice: ctx.actor.map(|a| a as u64), looping: fxc.looping });
            }
            "STOP_VFX" => {
                for e in self.effects.iter_mut().filter(|e| e.ids == fxc.ids && e.owner == ctx.actor) {
                    e.player.stop();
                    e.permanent = false;
                }
            }
            "STOP_SFX" => {
                spawn.sfx.stop.push((fxc.ids.clone(), ctx.actor.map(|a| Place::Object(a as u64))));
            }
            // PLAY_OBJECT_ANIMATION (recoil), EQUIPMENT, CREATE_RFX (radar
            // blips), CHANGE_ASPECT: drawing/UI-owned, not effects.
            _ => {}
        }
        for c in &fxc.chain_immediate {
            self.run_fxc(c, ctx, frame, spawn, list, false);
        }
    }

    fn create_vfx(&mut self, fxc: &Fxc, ctx: Ctx, frame: &Frame, spawn: &mut Spawn, list: ListKind) {
        let Some(def) = self.tables.vfx.get(&fxc.ids).cloned() else {
            if self.warned.insert(fxc.ids.clone()) {
                warn!("effects: VFX {} has no definition", fxc.ids);
            }
            return;
        };
        if def.nightonly {
            // compare: battle maps are daytime here (no day/night state yet).
            return;
        }
        let Some((mut placement, attach, position_only)) = self.resolve(fxc, &ctx, frame) else { return };
        let pos = placement.w_axis.truncate();
        if !def.permanent && !def.critical && pos.distance(self.focus) > DISCARD_DISTANCE {
            return;
        }
        let translation = Vec3::from(fxc.translation);
        if translation != Vec3::ZERO {
            placement *= Mat4::from_translation(Vec3::new(translation.x, translation.z, -translation.y));
        }
        let owner = match &attach {
            Some((a, _)) => Some(*a),
            None => ctx.actor,
        };
        let duration = if fxc.duration > 0.0 { fxc.duration } else { def.duration };
        match def.category.as_str() {
            "VFXMODEL" => self.model_vfx(&def, fxc, placement, attach, position_only, owner, duration, ctx, frame, spawn, list),
            "VFXTRACER" => {
                let Some(to) = self.tracer_target(&ctx, frame) else { return };
                let from = pos;
                let d = to - from;
                let dist = d.length();
                if dist < 0.5 {
                    return;
                }
                // compare: the original draws the tracer as a line of its
                // colour; alpha 1, additive like the client's streaks.
                self.tracers.push(Tracer {
                    from,
                    dir: d / dist,
                    dist,
                    speed: def.speed.max(1.0),
                    length: def.length.max(0.1),
                    color: [def.color[0], def.color[1], def.color[2], 1.0],
                    t: 0.0,
                });
            }
            "VFXPROJECTEDTEXTURE" => {
                if def.texture.is_empty() {
                    return;
                }
                let layer = self.layer_index.get(&def.texture.to_ascii_lowercase()).copied();
                if layer.is_none() && !def.respect_lighting {
                    return;
                }
                // Random, else oriented with the actor (template_vfx.tgi).
                let angle = if def.random_orientation {
                    self.rng.next() * std::f32::consts::TAU
                } else {
                    let x = placement.x_axis;
                    x.z.atan2(x.x)
                };
                let size = if def.texture_size > 0.0 { def.texture_size } else { 2.0 };
                let world = frame.world;
                let height = |x: f32, z: f32| match frame.ground {
                    Some(g) => (g.0)(x, z),
                    None => world.map.height_at(Vec2::new(x, -z)),
                };
                let verts = decal::build(pos, size, angle, world.map.cell_size, &height);
                // compare: decals of the map's own buildings start faded in
                // (their decal_fade_time ran while the original loaded).
                let age = if self.first_frame && list == ListKind::Constructed { def.decal_fade[0] } else { 0.0 };
                self.decals_dirty = true;
                self.decals.push(Decal {
                    verts,
                    layer: layer.unwrap_or(self.white),
                    texture: def.texture.clone(),
                    lit: def.respect_lighting,
                    order: decal_order(&def.decal_type),
                    color: def.color,
                    additive: def.additive,
                    age,
                    duration: if def.permanent { f32::INFINITY } else if duration > 0.0 { duration } else { 15.0 },
                    fade: def.decal_fade,
                    owner,
                    permanent: def.permanent,
                    list,
                });
            }
            // VFXLIGHT, VFXEXTRUSION (grenade streaks, contrails), VFXTEXT,
            // VFXOBJECTNAME, modifiers: not drawn yet (see the report).
            _ => {}
        }
    }

    fn tracer_target(&mut self, ctx: &Ctx, frame: &Frame) -> Option<Vec3> {
        let t = ctx.target?;
        let m = self.bone(frame, t, "spine").or_else(|| actor_matrix(frame, t))?;
        let mut p = m.w_axis.truncate();
        if frame.world.actors.get(t as usize).is_some_and(|a| frame.world.data.def(a.def).target_flags & self.infantry_bit(frame) == 0) {
            p += Vec3::Y * 1.0;
        }
        Some(p)
    }

    fn infantry_bit(&mut self, frame: &Frame) -> u32 {
        frame.world.data.target_types.iter().position(|n| n.eq_ignore_ascii_case("infantry")).map_or(0, |i| 1 << i)
    }

    #[allow(clippy::too_many_arguments)]
    fn model_vfx(
        &mut self,
        def: &VfxDef,
        fxc: &Fxc,
        placement: Mat4,
        attach: Option<(ActorId, Option<String>)>,
        position_only: bool,
        owner: Option<ActorId>,
        duration: f32,
        ctx: Ctx,
        frame: &Frame,
        spawn: &mut Spawn,
        list: ListKind,
    ) {
        let key = def.model.strip_prefix("effects/").unwrap_or(&def.model).to_owned();
        let pos = placement.w_axis.truncate();
        let rotation = if def.random_orientation {
            Quat::from_rotation_y(self.rng.next() * std::f32::consts::TAU)
        } else if position_only || (attach.is_some() && def.gyroscope) {
            Quat::IDENTITY
        } else {
            let (_, r, _) = placement.to_scale_rotation_translation();
            r
        };
        let place = Mat4::from_rotation_translation(rotation, pos);
        if let Some(fx) = self.nif.effects.get(&key) {
            if self.effects.len() >= MAX_EFFECTS {
                match self.effects.iter().position(|e| !e.permanent && !e.critical) {
                    Some(i) => {
                        self.effects.swap_remove(i);
                    }
                    None => return,
                }
            }
            let emit_until = if duration > 0.0 {
                duration
            } else if def.permanent {
                f32::INFINITY
            } else {
                // The NIF's length, but no longer than it shows anything.
                fx.active_span().min(fx.quiet_after()).max(0.05)
            };
            let seed = self.next_seed();
            let player = Player::new(fx, place, 1.0, emit_until, seed, self.white);
            self.effects.push(LiveFx {
                player,
                attach,
                gyroscope: def.gyroscope || def.random_orientation,
                owner,
                list,
                ids: fxc.ids.clone(),
                permanent: def.permanent && duration <= 0.0,
                critical: def.critical,
                chain: fxc.chain.clone(),
                ctx,
            });
            return;
        }
        // Mesh effects the particle table does not carry (death debris,
        // flags): the original model, still.
        // compare: the original plays their node keyframes (debris flying).
        if key.starts_with("deathdebris/") || key.starts_with("flags/") {
            let team = owner.and_then(|o| frame.world.actors.get(o as usize)).map_or(crate::scene::NEUTRAL_TEAM, |a| team_of(frame.world, a));
            if let Some(entity) = self.spawn_model(&def.model, team, Transform::from_matrix(place), spawn) {
                self.models.push(ModelFx { entity, owner, life: if def.permanent { None } else { Some(duration.max(5.0)) }, list });
            }
        } else if self.warned.insert(key.clone()) {
            warn!("effects: VFX {} model {} is not in the effect NIF table", fxc.ids, def.model);
        }
    }

    /// An original static model as entities (cached meshes/materials).
    fn spawn_model(&mut self, path: &str, team: [u8; 3], transform: Transform, spawn: &mut Spawn) -> Option<Entity> {
        let key = (path.to_ascii_lowercase(), team);
        let parts = match self.parts.get(&key) {
            Some(p) => p.clone(),
            None => {
                let loaded = self.store.as_ref().and_then(|store| {
                    let model = store.load_model(path).ok()?.prepare().ok()?;
                    let mut parts = Vec::new();
                    for mesh in model.meshes.iter().filter(|m| !m.indices.is_empty()) {
                        let texture = mesh.material.texture.as_ref().and_then(|t| {
                            crate::textures::load(store, t, crate::scene::kind(&mesh.material, team)).ok().map(|img| spawn.images.add(img))
                        });
                        parts.push((spawn.meshes.add(crate::scene::model_mesh(mesh)), spawn.materials.add(crate::look::model_material(&mesh.material, texture))));
                    }
                    Some(Arc::new(parts))
                });
                if loaded.is_none() && self.warned.insert(path.to_owned()) {
                    warn!("effects: model {path} did not load");
                }
                self.parts.insert(key, loaded.clone());
                loaded
            }
        }?;
        let parent = spawn.commands.spawn((transform, Visibility::default())).id();
        for (mesh, material) in parts.iter() {
            spawn.commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), ChildOf(parent)));
        }
        Some(parent)
    }

    fn run_list(&mut self, list: &[Fxc], ctx: Ctx, frame: &Frame, spawn: &mut Spawn, kind: ListKind) {
        for f in list {
            self.run_fxc(f, ctx, frame, spawn, kind, false);
        }
    }

    fn fired(&mut self, shooter: ActorId, target: ActorId, weapon: usize, frame: &Frame, spawn: &mut Spawn) {
        let tables = self.tables.clone();
        let Some(a) = frame.world.actors.get(shooter as usize) else { return };
        let def = frame.world.data.def(a.def);
        let Some(all) = self.fx_of(frame.world, a.def) else { return };
        let Some(fx) = all.weapons.get(weapon) else { return };
        let ctx = Ctx {
            actor: Some(shooter),
            target: Some(target),
            at: None,
            target_ground: frame.world.actors.get(target as usize).map(|t| t.pos),
            shooter_ground: Some(a.pos),
        };
        self.run_list(&fx.start, ctx, frame, spawn, ListKind::Other);
        self.run_list(&fx.do_, ctx, frame, spawn, ListKind::Other);
        self.run_list(&fx.per_target, ctx, frame, spawn, ListKind::Other);
        // The projectile in flight (the sim lands it after distance / speed).
        let w = def.weapons.get(weapon);
        if let (Some(p), Some(w)) = (fx.projectile.as_ref(), w)
            && w.projectile_speed > 0.0
            && let Some(t) = frame.world.actors.get(target as usize)
        {
            let flight = (t.pos.truncate() - a.pos.truncate()).length() / w.projectile_speed;
            let muzzle = ["cannon_ae_01", "gun_ae_01"]
                .iter()
                .find_map(|b| {
                    let m = self.bone(frame, shooter, b)?;
                    let base = actor_matrix(frame, shooter)?;
                    (m.w_axis != base.w_axis).then(|| m.w_axis.truncate())
                })
                .unwrap_or_else(|| to_render(a.pos) + Vec3::Y * 1.2);
            let to = actor_matrix(frame, target).map_or(to_render(t.pos), |m| m.w_axis.truncate()) + Vec3::Y * 0.5;
            // compare: LOB arc height from gravity over the flight time.
            let lob = w.projectile_type.as_deref().is_some_and(|k| k.eq_ignore_ascii_case("LOB"));
            let arc = if lob { 9.8 * flight * flight / 8.0 } else { 0.0 };
            let pfx = tables.actors.get(p);
            let model = pfx.and_then(|f| f.resource_prefix.clone()).map(|r| format!("{r}.nif"));
            let entity = model.and_then(|m| self.spawn_model(&m, crate::scene::NEUTRAL_TEAM, Transform::from_translation(muzzle), spawn));
            if let Some(pfx) = pfx {
                let pctx = Ctx { actor: None, target: Some(target), at: Some(muzzle), target_ground: Some(t.pos), shooter_ground: Some(a.pos) };
                let created = pfx.created.clone();
                self.run_list(&created, pctx, frame, spawn, ListKind::Other);
            }
            self.flights.push(Flight { projectile: p.clone(), from: muzzle, to, start: self.time, end: self.time + flight.max(0.05), arc, target: Some(target), shooter, entity });
        }
    }

    fn land(&mut self, f: &Flight, frame: &Frame, spawn: &mut Spawn) {
        let tables = self.tables.clone();
        if let Some(e) = f.entity {
            spawn.commands.entity(e).despawn();
        }
        let Some(pfx) = tables.actors.get(&f.projectile) else { return };
        let ground = Vec3::new(f.to.x, frame.world.map.height_at(Vec2::new(f.to.x, -f.to.z)), f.to.z);
        let ctx = Ctx {
            actor: None,
            target: f.target.filter(|t| frame.world.actor(*t).is_some()),
            at: Some(ground),
            target_ground: Some(Vec3::new(ground.x, -ground.z, ground.y)),
            shooter_ground: frame.world.actors.get(f.shooter as usize).map(|a| a.pos),
        };
        self.run_list(&pfx.destroyed, ctx, frame, spawn, ListKind::Other);
    }

    fn died(&mut self, actor: ActorId, frame: &Frame, spawn: &mut Spawn, views: &mut Query<(&ActorView, &mut Visibility)>) {
        let tables = self.tables.clone();
        let Some(a) = frame.world.actors.get(actor as usize) else { return };
        let def = frame.world.data.def(a.def);
        let _ = def;
        let Some(fx) = self.fx_of(frame.world, a.def) else { return };
        let ctx = Ctx { actor: Some(actor), target: None, at: None, target_ground: None, shooter_ground: Some(a.pos) };
        // WhenConstructed effects stop on destruction (stop_on_destructed).
        for e in self.effects.iter_mut().filter(|e| e.owner == Some(actor) && e.list == ListKind::Constructed) {
            e.player.stop();
            e.permanent = false;
            e.attach = None;
        }
        self.decals.retain(|d| !(d.owner == Some(actor) && d.list == ListKind::Constructed));
        self.decals_dirty = true;
        let commands = &mut *spawn.commands;
        self.models.retain(|m| {
            if m.owner == Some(actor) && m.list == ListKind::Constructed {
                commands.entity(m.entity).despawn();
                false
            } else {
                true
            }
        });
        self.run_list(&fx.destroyed, ctx, frame, spawn, ListKind::Other);
        // aspect_death: the wreck / rubble model replaces the actor's.
        if let Some(model) = fx.death_model.as_ref()
            && !self.wrecks.contains_key(&actor)
        {
            let Some(m) = actor_matrix(frame, actor) else { return };
            let team = team_of(frame.world, a);
            if let Some(entity) = self.spawn_model(model, team, Transform::from_matrix(m), spawn) {
                self.wrecks.insert(actor, entity);
                for (view, mut vis) in views.iter_mut() {
                    if view.id == actor {
                        *vis = Visibility::Hidden;
                    }
                }
            }
        }
    }

    /// Per-actor state changes: new actors (WhenCreated), health thresholds
    /// (WhenHealth), movement starts/stops, removal.
    fn scan(&mut self, frame: &Frame, spawn: &mut Spawn) {
        let tables = self.tables.clone();
        let n = frame.world.actors.len();
        if self.seen.len() < n {
            self.seen.resize(n, false);
            self.health.resize(n, 1.0);
            self.moving.resize(n, false);
        }
        for a in &frame.world.actors {
            let i = a.id as usize;
            if a.removed {
                if self.seen[i] {
                    self.seen[i] = false;
                    self.forget(a.id, spawn);
                }
                continue;
            }
            let def = frame.world.data.def(a.def);
            let fx = self.fx_of(frame.world, a.def);
            let fx = fx.as_deref();
            let ctx = Ctx { actor: Some(a.id), target: None, at: None, target_ground: None, shooter_ground: Some(a.pos) };
            if !self.seen[i] {
                self.seen[i] = true;
                self.health[i] = if def.health > 0.0 { a.health / def.health } else { 1.0 };
                self.moving[i] = a.activity == Activity::Moving;
                if let Some(fx) = fx {
                    self.run_list(&fx.created, ctx, frame, spawn, ListKind::Created);
                    // compare: WhenConstructed runs when the actor first
                    // exists built (map buildings, finished deployments).
                    if a.alive {
                        self.run_list(&fx.constructed, ctx, frame, spawn, ListKind::Constructed);
                    }
                }
                continue;
            }
            let Some(fx) = fx else { continue };
            if def.health > 0.0 && !fx.health.is_empty() {
                let cur = if a.alive { (a.health / def.health).max(0.0) } else { 0.0 };
                let prev = self.health[i];
                if cur != prev {
                    for f in &fx.health {
                        let h = f.health;
                        let crossed = match f.health_trigger.as_str() {
                            "ABSOLUTE_ABOVE" | "RELATIVE_INCREASE" => prev <= h && cur > h,
                            _ => (prev >= h && cur < h) || (h <= 0.0 && prev > 0.0 && cur <= 0.0),
                        };
                        if crossed {
                            self.run_fxc(f, ctx, frame, spawn, ListKind::Health, false);
                        }
                    }
                    self.health[i] = cur;
                }
            }
            let moving = a.alive && a.activity == Activity::Moving;
            if moving != self.moving[i] {
                self.moving[i] = moving;
                if moving {
                    self.run_list(&fx.start_moving, ctx, frame, spawn, ListKind::Moving);
                } else {
                    for e in self.effects.iter_mut().filter(|e| e.owner == Some(a.id) && e.list == ListKind::Moving) {
                        e.player.stop();
                        e.permanent = false;
                    }
                    self.run_list(&fx.stop_moving, ctx, frame, spawn, ListKind::Moving);
                }
            }
        }
    }

    /// The actor left the battle: its permanent effects end, its wreck goes.
    fn forget(&mut self, actor: ActorId, spawn: &mut Spawn) {
        for e in self.effects.iter_mut().filter(|e| e.owner == Some(actor) || e.attach.as_ref().is_some_and(|(a, _)| *a == actor)) {
            e.player.stop();
            e.permanent = false;
            e.attach = None;
        }
        self.decals.retain(|d| !(d.permanent && d.owner == Some(actor)));
        self.decals_dirty = true;
        if let Some(e) = self.wrecks.remove(&actor) {
            spawn.commands.entity(e).despawn();
        }
        let commands = &mut *spawn.commands;
        self.models.retain(|m| {
            if m.owner == Some(actor) && m.life.is_none() {
                commands.entity(m.entity).despawn();
                false
            } else {
                true
            }
        });
    }

    fn step(&mut self, dt: f32, frame: &Frame, spawn: &mut Spawn, transforms: &mut Query<&mut Transform, Without<ActorView>>) {
        self.time += dt;
        // Delayed FXCs.
        let now = self.time;
        let due: Vec<Delayed> = {
            let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.delayed).into_iter().partition(|d| d.at <= now);
            self.delayed = keep;
            due
        };
        for d in due {
            self.run_fxc(&d.fxc, d.ctx, frame, spawn, d.list, true);
        }
        // Projectiles.
        let mut landed = Vec::new();
        let mut i = 0;
        while i < self.flights.len() {
            let f = &self.flights[i];
            if now >= f.end {
                landed.push(self.flights.swap_remove(i));
                continue;
            }
            let s = ((now - f.start) / (f.end - f.start)).clamp(0.0, 1.0);
            let p = f.from.lerp(f.to, s) + Vec3::Y * (f.arc * 4.0 * s * (1.0 - s));
            let s2 = (s + 0.02).min(1.0);
            let p2 = f.from.lerp(f.to, s2) + Vec3::Y * (f.arc * 4.0 * s2 * (1.0 - s2));
            if let Some(e) = f.entity
                && let Ok(mut t) = transforms.get_mut(e)
            {
                t.translation = p;
                let d = p2 - p;
                if d.length_squared() > 1e-8 {
                    // compare: projectile models taken to face native -Y
                    // (render +Z) like units, turned along the flight.
                    t.rotation = Quat::from_rotation_arc(Vec3::Z, d.normalize());
                }
            }
            i += 1;
        }
        for f in landed {
            self.land(&f, frame, spawn);
        }
        // NIF effects follow their attachment.
        let mut chains = Vec::new();
        let mut budget = EMIT_BUDGET;
        let mut k = 0;
        while k < self.effects.len() {
            let attach = self.effects[k].attach.clone();
            if let Some((actor, bone)) = attach {
                if frame.world.actor(actor).is_some() {
                    if let Some(m) = self.bone(frame, actor, bone.as_deref().unwrap_or("")) {
                        let place = if self.effects[k].gyroscope { Mat4::from_translation(m.w_axis.truncate()) } else { m };
                        self.effects[k].player.set_placement(place);
                    }
                } else {
                    self.effects[k].player.stop();
                    self.effects[k].attach = None;
                }
            }
            let e = &mut self.effects[k];
            budget = budget.saturating_sub(e.player.update(dt, budget));
            if e.player.finished() {
                let e = self.effects.swap_remove(k);
                if !e.chain.is_empty() {
                    chains.push((e.chain, e.ctx));
                }
                continue;
            }
            k += 1;
        }
        for (chain, ctx) in chains {
            self.run_list(&chain, ctx, frame, spawn, ListKind::Other);
        }
        // Tracers.
        self.tracers.retain_mut(|t| {
            t.t += dt;
            t.t * t.speed < t.dist + t.length
        });
        // Decals.
        let before = self.decals.len();
        self.decals.retain_mut(|d| {
            d.age += dt;
            d.age < d.duration + d.fade[1]
        });
        if self.decals.len() != before {
            self.decals_dirty = true;
        }
        // Timed models.
        let commands = &mut *spawn.commands;
        self.models.retain_mut(|m| match m.life.as_mut() {
            Some(l) => {
                *l -= dt;
                if *l <= 0.0 {
                    commands.entity(m.entity).despawn();
                    false
                } else {
                    true
                }
            }
            None => true,
        });
    }

    fn draw(&mut self, spawn: &mut Spawn, transforms: &mut Query<&mut Transform, Without<ActorView>>) -> usize {
        let view = self.view;
        let mut quads = std::mem::take(&mut self.quads);
        quads.clear();
        for e in &self.effects {
            if quads.len() >= MAX_QUADS {
                break;
            }
            e.player.render(&view, &mut quads, MAX_QUADS);
        }
        for t in &self.tracers {
            let head_d = (t.t * t.speed).min(t.dist);
            let tail_d = (t.t * t.speed - t.length).max(0.0);
            if head_d <= tail_d {
                continue;
            }
            let head = t.from + t.dir * head_d;
            let tail = t.from + t.dir * tail_d;
            // About 1.5 pixels wide, like a line primitive.
            let width = view.eye.distance(head) * view.pixel * 1.5;
            quads.push(Quad::streak(&view, tail, head, width, t.color, self.white, 1));
        }
        let n = quads.len();
        // The particles sort after every other transparent object (their
        // entity sits at the eye); vertices are relative to it.
        let origin = view.eye - view.back * 0.5;
        if let Some(mut mesh) = spawn.meshes.get_mut(&self.quads_mesh) {
            for q in quads.iter_mut() {
                for c in q.corners.iter_mut() {
                    *c -= origin;
                }
            }
            render::write(&mut mesh, &quads, view.eye - origin, true);
        }
        if let Ok(mut t) = transforms.get_mut(self.quads_entity) {
            t.translation = origin;
        }
        self.quads = quads;
        self.draw_decals(view, spawn, transforms);
        n
    }

    fn draw_decals(&mut self, view: View, spawn: &mut Spawn, transforms: &mut Query<&mut Transform, Without<ActorView>>) {
        // Rebuilt only when needed: decals added or removed, fading, or the
        // camera far from where the meshes were last placed.
        let moved = view.eye.distance(self.decal_eye) > 50.0;
        let fading = self.decals.iter().any(|d| (d.fade[0] > 0.0 && d.age < d.fade[0]) || d.age > d.duration);
        // Fades alone refresh at 10 Hz.
        let fade_due = fading && self.time - self.decal_time >= 0.1;
        if !(self.decals_dirty || moved || fade_due) {
            return;
        }
        self.decal_time = self.time;
        self.decals_dirty = false;
        self.decal_eye = view.eye;
        // Unlit decals: drawn first among the transparent objects (their
        // entity sits far ahead along the view), in decal_type order.
        let far = view.eye - view.back * 2000.0;
        let mut order: Vec<usize> = (0..self.decals.len()).filter(|&i| !self.decals[i].lit).collect();
        order.sort_by_key(|&i| self.decals[i].order);
        let mut dq = Vec::new();
        for i in order {
            let d = &self.decals[i];
            let color = [d.color[0], d.color[1], d.color[2], decal_alpha(d)];
            for t in d.verts.chunks_exact(3) {
                dq.push(Quad {
                    corners: [t[0].pos - far, t[1].pos - far, t[2].pos - far, t[2].pos - far],
                    uv: [t[0].uv, t[1].uv, t[2].uv, t[2].uv],
                    color,
                    layer: d.layer,
                    blend: if d.additive { 1 } else { 0 },
                });
            }
        }
        if let Some(mut mesh) = spawn.meshes.get_mut(&self.decal_mesh) {
            render::write(&mut mesh, &dq, Vec3::ZERO, false);
        }
        if let Ok(mut t) = transforms.get_mut(self.decal_entity) {
            t.translation = far;
        }
        // Lit decals (building ground planes): the scene's terrain look,
        // alpha-blended, sorted before every other transparent object but
        // after the terrain layers (entity even farther ahead).
        let far_lit = view.eye - view.back * 3000.0;
        let mut by_texture: HashMap<String, (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<[f32; 4]>)> = HashMap::new();
        for d in self.decals.iter().filter(|d| d.lit) {
            let e = by_texture.entry(d.texture.to_ascii_lowercase()).or_default();
            let a = decal_alpha(d);
            for v in &d.verts {
                e.0.push((v.pos - far_lit).to_array());
                e.1.push(v.normal.to_array());
                e.2.push(v.uv);
                e.3.push([d.color[0], d.color[1], d.color[2], a]);
            }
        }
        for key in self.lit.keys().cloned().collect::<Vec<_>>() {
            by_texture.entry(key).or_default();
        }
        for (key, (pos, normals, uvs, colors)) in by_texture {
            if !self.lit.contains_key(&key) {
                let Some(batch) = self.lit_batch(&key, spawn) else { continue };
                self.lit.insert(key.clone(), batch);
            }
            let Some(batch) = self.lit.get(&key) else { continue };
            if let Some(mut mesh) = spawn.meshes.get_mut(&batch.mesh) {
                lit_mesh(&mut mesh, pos, normals, uvs, colors);
            }
            if let Ok(mut t) = transforms.get_mut(batch.entity) {
                t.translation = far_lit;
            }
        }
    }

    fn lit_batch(&mut self, texture: &str, spawn: &mut Spawn) -> Option<LitBatch> {
        let store = self.store.as_ref()?;
        let rel = texture.replace('\\', "/");
        let stem = rel.rsplit_once('.').map_or(rel.as_str(), |(s, _)| s);
        let url = format!("/assets/converted/textures/{stem}.png");
        let image = match crate::textures::load(store, &url, crate::textures::Kind::Color) {
            Ok(i) => spawn.images.add(i),
            Err(e) => {
                if self.warned.insert(url.clone()) {
                    warn!("effects: ground texture {url}: {e:#}");
                }
                return None;
            }
        };
        let mut material = crate::look::terrain_material(Some(image), [1.0; 4], None, 0);
        material.base.alpha_mode = AlphaMode::Blend;
        material.base.depth_bias = 0.0;
        let mut mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::RENDER_WORLD);
        lit_mesh(&mut mesh, Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mesh = spawn.meshes.add(mesh);
        let entity = spawn
            .commands
            .spawn((Mesh3d(mesh.clone()), MeshMaterial3d(spawn.materials.add(material)), Transform::default(), NoFrustumCulling, NotShadowCaster))
            .id();
        Some(LitBatch { entity, mesh })
    }
}

fn decal_alpha(d: &Decal) -> f32 {
    let mut alpha = 1.0;
    if d.fade[0] > 0.0 && d.age < d.fade[0] {
        alpha = d.age / d.fade[0];
    }
    if d.age > d.duration && d.fade[1] > 0.0 {
        alpha = (1.0 - (d.age - d.duration) / d.fade[1]).max(0.0);
    }
    alpha
}

fn lit_mesh(mesh: &mut Mesh, mut pos: Vec<[f32; 3]>, mut normals: Vec<[f32; 3]>, mut uvs: Vec<[f32; 2]>, mut colors: Vec<[f32; 4]>) {
    if pos.is_empty() {
        for _ in 0..3 {
            pos.push([0.0, -1.0e4, 0.0]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([0.0, 0.0]);
            colors.push([0.0; 4]);
        }
    }
    let n = pos.len() as u32;
    *mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(bevy::mesh::Indices::U32((0..n).collect()));
}

#[allow(clippy::too_many_arguments)]
fn run(
    mut commands: Commands,
    time: Res<Time>,
    battle: Option<Res<Battle>>,
    state: Option<ResMut<FxState>>,
    mut sfx: ResMut<SfxQueue>,
    mut stats: ResMut<FxStats>,
    cameras: Query<(&GlobalTransform, &crate::camera::NativeCamera)>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    drawn: Query<(&ActorView, &Transform)>,
    mut views: Query<(&ActorView, &mut Visibility)>,
    mut transforms: Query<&mut Transform, Without<ActorView>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OriginalMaterial>>,
    mut images: ResMut<Assets<Image>>,
    ground: Option<Res<crate::camera::Ground>>,
) {
    let (Some(battle), Some(mut state)) = (battle, state) else { return };
    let started = Instant::now();
    let state = &mut *state;
    if let Some((cam, native)) = cameras.iter().next() {
        let height = windows.iter().next().map_or(900.0, |w| w.physical_height().max(1) as f32);
        state.view = View {
            eye: cam.translation(),
            right: cam.right().as_vec3(),
            up: cam.up().as_vec3(),
            back: cam.back().as_vec3(),
            pixel: (crate::camera::FOV_DEG.to_radians() * 0.5).tan() * 2.0 / height,
        };
        state.focus = to_render(native.target);
    }
    let drawn_map: HashMap<ActorId, Mat4> = drawn.iter().map(|(v, t)| (v.id, t.to_matrix())).collect();
    state.bone_cache.clear();
    let frame = Frame { world: &battle.world, ground: ground.as_deref(), drawn: &drawn_map, local: battle.local };
    let _ = frame.local;
    let mut spawn = Spawn { commands: &mut commands, meshes: &mut meshes, materials: &mut materials, images: &mut images, sfx: &mut sfx };
    let t0 = Instant::now();
    state.scan(&frame, &mut spawn);
    let t1 = Instant::now();
    for ev in &battle.frame_events {
        match *ev {
            Event::Fired { shooter, target, weapon } => state.fired(shooter, target, weapon as usize, &frame, &mut spawn),
            Event::Died { actor } => state.died(actor, &frame, &mut spawn, &mut views),
            _ => {}
        }
    }
    let t2 = Instant::now();
    let dt = if battle.paused { 0.0 } else { time.delta_secs() * battle.speed };
    state.step(dt, &frame, &mut spawn, &mut transforms);
    let t3 = Instant::now();
    let quads = state.draw(&mut spawn, &mut transforms);
    let t4 = Instant::now();
    for (k, d) in [t1 - t0, t2 - t1, t3 - t2, t4 - t3].into_iter().enumerate() {
        stats.phases[k] += d.as_secs_f32() * 1e6;
    }
    state.first_frame = false;
    let micros = started.elapsed().as_secs_f32() * 1e6;
    stats.effects = state.effects.len();
    stats.quads = quads;
    stats.tracers = state.tracers.len();
    stats.micros = micros;
    stats.peak_micros = stats.peak_micros.max(micros);
    stats.frames += 1;
    stats.total += micros;
    stats.average_micros = stats.total / stats.frames as f32;
    // AA_BEVY_FX_STATS=1: the frame cost of effects every 2 s (measurement).
    let phases = stats.phases;
    let w = &mut stats.window;
    w.0 += micros;
    w.1 += 1;
    w.2 = w.2.max(micros);
    w.3 += time.delta_secs();
    if w.3 >= 2.0 {
        if std::env::var_os("AA_BEVY_FX_STATS").is_some() {
            let shooting = battle.world.actors.iter().filter(|a| a.alive && !a.removed && a.activity == Activity::Attacking).count();
            info!(
                "fx stats: {:.0} us/frame average (scan {:.0}, events {:.0}, step {:.0}, draw {:.0}), {:.0} us peak over {} frames; {} effects, {} quads, {} tracers, {} decals, {} attacking actors, {} alive actors",
                w.0 / w.1.max(1) as f32,
                phases[0] / w.1.max(1) as f32,
                phases[1] / w.1.max(1) as f32,
                phases[2] / w.1.max(1) as f32,
                phases[3] / w.1.max(1) as f32,
                w.2,
                w.1,
                state.effects.len(),
                quads,
                state.tracers.len(),
                state.decals.len(),
                shooting,
                battle.world.actors.iter().filter(|a| a.alive && !a.removed).count()
            );
        }
        *w = (0.0, 0, 0.0, 0.0);
        stats.phases = [0.0; 4];
    }
}
