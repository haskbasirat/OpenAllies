//! Battle scene drawing: the original map, terrain layers, placed objects and
//! animation rigs from the user's install. Owner: lead (battle renderer).
//! The display-only HQ, soldiers and tank of the Development 12 test stay
//! until the aa-play world draws real units.
use crate::{camera, session, terrain, textures, units};

use aa_animation::AnimatedAsset;
use aa_assets::{AssetStore, HeightMode, PreparedModel, PreparedScene};
use anyhow::{Context, Result};
use bevy::asset::{RenderAssetUsages, uuid_handle};
use bevy::light::CascadeShadowConfigBuilder;
use bevy::mesh::{Indices, PrimitiveTopology};
use crate::look::{self, OriginalMaterial};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Face, ShaderType};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::shader::{Shader, ShaderRef};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use textures::Kind;
use std::sync::Mutex;

pub const TITLE: &str = "Axis & Allies - Bevy test (Development 12)";
/// Team colours mixed into team-masked textures (display choice).
pub const PLAYER_TEAM: [u8; 3] = [178, 34, 30];
pub const NEUTRAL_TEAM: [u8; 3] = [120, 120, 112];


#[derive(Clone)]
struct Args {
    assets: PathBuf,
    rigs: PathBuf,
    map: String,
    shot: Option<(PathBuf, f32)>,
    focus: Option<Vec2>,
}

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

fn parse_args() -> Result<Args> {
    let mut argv = std::env::args().skip(1);
    let (mut assets, mut rigs, mut map, mut shot, mut focus) = (None, None, "(2v2)Berlin".to_owned(), None, None);
    while let Some(a) = argv.next() {
        let mut value = || argv.next().with_context(|| format!("{a} needs a value"));
        match a.as_str() {
            "--assets" => assets = Some(PathBuf::from(value()?)),
            "--rigs" => rigs = Some(PathBuf::from(value()?)),
            "--map" => map = value()?,
            "--screenshot" => {
                let path = PathBuf::from(value()?);
                let seconds: f32 = value()?.parse()?;
                shot = Some((path, seconds));
            }
            "--focus" => {
                let x: f32 = value()?.parse()?;
                let y: f32 = value()?.parse()?;
                focus = Some(Vec2::new(x, y));
            }
            other => anyhow::bail!("unknown argument {other}"),
        }
    }
    let assets = match assets {
        Some(a) => a,
        None => find_up("reconstruction/assets/converted")
            .and_then(|p| p.parent()?.parent().map(PathBuf::from))
            .context("the reconstruction folder (original converted assets) was not found; pass --assets")?,
    };
    let rigs = match rigs {
        Some(r) => r,
        None => find_up("assets-cache/animation/RUSInfantry.rig.json")
            .and_then(|p| p.parent().map(PathBuf::from))
            .context("assets-cache/animation was not found; pass --rigs")?,
    };
    Ok(Args { assets, rigs, map, shot, focus })
}

/// Everything decoded before the window opens.
#[derive(Resource)]
struct Loaded {
    store: AssetStore,
    fixes: Arc<crate::model_fixes::ModelFixes>,
    rigs: PathBuf,
    scene: PreparedScene,
    /// The generator's game map for a random map (an original map loads its own).
    game_map: Option<aa_play::GameMap>,
    terrain: Vec<terrain::Draw>,
    images: Option<Vec<((String, Kind), Image)>>,
    failures: Vec<String>,
    hq_at: Vec2,
    focus: Option<Vec2>,
    seconds: f32,
    /// The battle world (aa-play) for this map and setup.
    world: Option<aa_play::World>,
}

fn model(store: &AssetStore, path: &str) -> Result<PreparedModel> {
    Ok(store.load_model(path)?.prepare()?)
}

pub fn kind(m: &aa_assets::Material, team: [u8; 3]) -> Kind {
    if m.team_color_mask { Kind::Team(team) } else { Kind::Color }
}

/// Where the battle map comes from.
#[derive(Clone)]
enum MapSource {
    /// An original map, e.g. "(2v2)Berlin".
    File(String),
    /// The original random map generator with the setup screen's options.
    Random(aa_play::mapgen::RandomMapParams),
}

/// A generated map as a scene: its terrain plus every scenery model.
fn generated_scene(args: &Args, store: &AssetStore, params: &aa_play::mapgen::RandomMapParams) -> Result<(PreparedScene, aa_play::GameMap)> {
    let root = args.assets.parent().context("the install root above the reconstruction folder")?;
    let generated = aa_play::mapgen::generate(params, root)?;
    let terrain = generated.terrain.prepare(HeightMode::NativeConditional)?;
    let mut models = std::collections::BTreeMap::new();
    let mut instances = Vec::new();
    let mut warnings = Vec::new();
    for (i, g) in generated.scenery.iter().enumerate() {
        let Some(model) = &g.model else { continue };
        let key = model.replace('\\', "/").to_lowercase();
        if !models.contains_key(&key) {
            match store.load_model(model).map_err(anyhow::Error::from).and_then(|m| m.prepare().map_err(anyhow::Error::from)) {
                Ok(prepared) => {
                    models.insert(key.clone(), prepared);
                }
                Err(e) => {
                    warnings.push(aa_assets::AssetWarning { source: model.clone(), message: format!("{e:#}") });
                    continue;
                }
            }
        }
        instances.push(aa_assets::PreparedInstance {
            source_index: i,
            instance_id: i as u32,
            thing_id: g.thing_id.clone(),
            position: aa_assets::native_to_render(g.position),
            yaw_radians: g.yaw,
            scale: g.scale,
            model_key: Some(key),
        });
    }
    let placements = aa_assets::PlacementMap {
        id: "random".into(),
        source: "generated".into(),
        expected_records: 0,
        actors: Vec::new(),
        types: std::collections::BTreeMap::new(),
        errors: Vec::new(),
        limitations: Vec::new(),
    };
    let scene = PreparedScene { map: aa_assets::MapAsset { terrain: generated.terrain, placements }, terrain, instances, models, warnings };
    Ok((scene, generated.map))
}

/// Direct start without the menus: you as Russia against a German computer
/// player on the first two start locations, original kingdom colours.
fn default_setup(map_id: &str, data: &aa_play::GameData) -> aa_play::SkirmishSetup {
    let colour = |i: usize, fallback: [u8; 3]| {
        data.kingdom_colors.get(i).map_or(fallback, |(_, c)| [(c[0] * 255.0).round() as u8, (c[1] * 255.0).round() as u8, (c[2] * 255.0).round() as u8])
    };
    aa_play::SkirmishSetup {
        map_id: map_id.to_owned(),
        players: vec![
            aa_play::PlayerSetup { name: "Player".into(), nation: "rus".into(), team: 0, color: colour(0, PLAYER_TEAM), controller: aa_play::Controller::Human, start: 0 },
            aa_play::PlayerSetup { name: "Computer".into(), nation: "ger".into(), team: 1, color: colour(1, NEUTRAL_TEAM), controller: aa_play::Controller::Computer(1), start: 1 },
        ],
        starting_resources: None,
        seed: 1,
    }
}

fn load(args: &Args, source: &MapSource, setup: Option<aa_play::SkirmishSetup>, profiles: Vec<(usize, String)>) -> Result<Loaded> {
    let start = Instant::now();
    let store = AssetStore::open(&args.assets)?;
    let (mut scene, game_map) = match source {
        MapSource::File(id) => (store.load_scene(id, HeightMode::NativeConditional)?, None),
        MapSource::Random(params) => {
            let (scene, map) = generated_scene(args, &store, params)?;
            (scene, Some(map))
        }
    };
    // The audited original-NIF corrections (shadow helpers, team colour
    // flags, alpha states), exactly as the wgpu client applies them.
    let fixes = Arc::new(crate::model_fixes::ModelFixes::discover(&store)?);
    for (key, model) in scene.models.iter_mut() {
        if let Err(e) = fixes.apply(&store, model) {
            eprintln!("model fixes {key}: {e:#}");
        }
    }
    let terrain = terrain::build(&scene.map.terrain, &scene.terrain)?;
    // The battle world from the original rules and map.
    let root = args.assets.parent().context("the install root above the reconstruction folder")?;
    let data = Arc::new(aa_play::GameData::load(root)?);
    let map_id = match source {
        MapSource::File(id) => id.clone(),
        MapSource::Random(_) => "random".to_owned(),
    };
    let world_map = match &game_map {
        Some(m) => m.clone(),
        None => aa_play::GameMap::load_with(root, &map_id, &data)?,
    };
    let setup = setup.unwrap_or_else(|| default_setup(&map_id, &data));
    let mut world = aa_play::World::new_skirmish(data, Arc::new(world_map), &setup)?;
    // The original computer players (Data/SAI profiles); cached after the first load.
    aa_play::ai::attach(&mut world, Arc::new(aa_play::ai::AiData::load(root)?));
    // The generals the setup screen chose for the computer slots (kingdom id = player index).
    for (player, profile) in &profiles {
        aa_play::ai::set_profile(&mut world, *player as u32, profile);
    }
    // First start location: where the Test skirmish put the local HQ.
    let hq_at = scene
        .map
        .placements
        .actors
        .iter()
        .find(|p| p.thing_id == "start_loc")
        .map(|p| Vec2::new(p.render_position()[0], p.render_position()[2]))
        .or_else(|| game_map.as_ref().and_then(|m| m.start_locations.first()).map(|p| Vec2::new(p.x, -p.y)))
        .unwrap_or(Vec2::new(scene.terrain.bounds.max[0] * 0.5, scene.terrain.bounds.min[2] * 0.5));
    let mut requests = textures::Requests::default();
    for draw in &terrain {
        if let Some(t) = &draw.texture {
            requests.add(t, Kind::Color);
        }
        if let Some(m) = &draw.mask {
            requests.add(&m.texture, Kind::Mask);
        }
    }
    for (key, m) in &scene.models {
        let _ = key;
        for mesh in &m.meshes {
            if let Some(t) = &mesh.material.texture {
                requests.add(t, kind(&mesh.material, NEUTRAL_TEAM));
            }
        }
    }
    let (images, failures) = textures::decode(&store, requests);
    Ok(Loaded {
        store: store.clone(),
        fixes,
        rigs: args.rigs.clone(),
        scene,
        game_map,
        terrain,
        images: Some(images),
        failures,
        hq_at,
        focus: args.focus,
        seconds: start.elapsed().as_secs_f32(),
        world: Some(world),
    })
}

/// The battle scene: loads the map in AppState::Loading (background thread)
/// and draws it in AppState::Battle.
pub struct ScenePlugin {
    args: Args,
}

/// Parse the command line (the map itself loads in AppState::Loading).
pub fn prepare() -> Result<ScenePlugin> {
    Ok(ScenePlugin { args: parse_args()? })
}

#[derive(Resource, Clone)]
struct SceneArgs(Args);

/// The background load started on entering AppState::Loading.
#[derive(Resource)]
struct LoadTask(Arc<Mutex<Option<std::result::Result<Loaded, String>>>>);

fn start_loading(
    mut commands: Commands,
    args: Res<SceneArgs>,
    setup: Option<Res<session::PendingSetup>>,
    random: Option<Res<session::PendingRandomMap>>,
    profiles: Option<Res<session::PendingAiProfiles>>,
    mut progress: ResMut<session::LoadProgress>,
) {
    let map = match (&random, &setup) {
        (Some(r), _) => MapSource::Random(r.0.clone()),
        (None, Some(s)) => MapSource::File(s.0.map_id.clone()),
        (None, None) => MapSource::File(args.0.map.clone()),
    };
    let label = match &map {
        MapSource::File(id) => id.clone(),
        MapSource::Random(r) => format!("random map {}", r.rmc),
    };
    *progress = session::LoadProgress { fraction: 0.0, status: format!("Loading {label}"), error: None };
    let slot = Arc::new(Mutex::new(None));
    let out = slot.clone();
    let args = args.0.clone();
    let setup = setup.map(|s| s.0.clone());
    let profiles = profiles.map(|p| p.0.clone()).unwrap_or_default();
    std::thread::spawn(move || {
        let result = load(&args, &map, setup, profiles).map_err(|e| format!("{e:#}"));
        *out.lock().unwrap() = Some(result);
    });
    commands.insert_resource(LoadTask(slot));
}

fn poll_loading(mut commands: Commands, task: Option<Res<LoadTask>>, mut progress: ResMut<session::LoadProgress>, mut next: ResMut<NextState<session::AppState>>) {
    let Some(task) = task else { return };
    let Some(result) = task.0.lock().unwrap().take() else { return };
    commands.remove_resource::<LoadTask>();
    match result {
        Ok(loaded) => {
            progress.fraction = 1.0;
            commands.insert_resource(loaded);
            next.set(session::AppState::Battle);
        }
        Err(e) => {
            error!("battle load failed: {e}");
            progress.error = Some(e);
            next.set(session::AppState::Menu);
        }
    }
}

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(look::LookPlugin)
            .insert_resource(ClearColor(Color::srgb(0.62, 0.68, 0.74)))
            .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 420.0, ..default() })
            .insert_resource(SceneArgs(self.args.clone()))
            .init_resource::<session::LoadProgress>()
            .add_systems(OnEnter(session::AppState::Loading), start_loading)
            .add_systems(Update, poll_loading.run_if(in_state(session::AppState::Loading)))
            .add_systems(OnEnter(session::AppState::Battle), setup)
            .add_plugins(crate::battle_view::BattleViewPlugin)
            .add_systems(Update, (hud, crate::debug_battle::spawn).run_if(in_state(session::AppState::Battle)));
        if let Some((path, seconds)) = self.args.shot.clone() {
            app.insert_resource(ShotPlan { path, seconds, taken: false, frames: 0 }).add_systems(Update, screenshot);
        }
    }
}

#[derive(Resource)]
struct ShotPlan {
    path: PathBuf,
    seconds: f32,
    taken: bool,
    frames: u32,
}

fn screenshot(mut commands: Commands, time: Res<Time>, mut plan: ResMut<ShotPlan>, mut exit: MessageWriter<AppExit>) {
    if !plan.taken && time.elapsed_secs() >= plan.seconds {
        plan.taken = true;
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(plan.path.clone()));
    } else if plan.taken {
        plan.frames += 1;
        if plan.frames > 30 {
            exit.write(AppExit::Success);
        }
    }
}

#[derive(Component)]
struct HudText;

pub fn standard(m: &aa_assets::Material, texture: Option<Handle<Image>>) -> OriginalMaterial {
    look::model_material(m, texture)
}

/// The recovered mesh files store (u, 1 - native v) for Three.js; restore the
/// native Direct3D coordinates, like the wgpu client's `loading::model_uv`.
pub fn model_uv(uv: [f32; 2]) -> [f32; 2] {
    [uv[0], 1.0 - uv[1]]
}

pub fn model_mesh(mesh: &aa_assets::PreparedMesh) -> Mesh {
    let mut out = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    out.insert_attribute(Mesh::ATTRIBUTE_POSITION, mesh.vertices.iter().map(|v| v.position).collect::<Vec<_>>());
    out.insert_attribute(Mesh::ATTRIBUTE_NORMAL, mesh.vertices.iter().map(|v| v.normal).collect::<Vec<_>>());
    out.insert_attribute(Mesh::ATTRIBUTE_UV_0, mesh.vertices.iter().map(|v| model_uv(v.uv)).collect::<Vec<_>>());
    out.insert_indices(Indices::U32(mesh.indices.clone()));
    out
}

/// Mesh and material handles for every mesh of a model.
fn model_parts(
    model: &PreparedModel,
    team: [u8; 3],
    tex: &HashMap<(String, Kind), Handle<Image>>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<OriginalMaterial>,
) -> Vec<(Handle<Mesh>, Handle<OriginalMaterial>)> {
    model
        .meshes
        .iter()
        .filter(|m| !m.indices.is_empty())
        .map(|m| {
            let texture = m.material.texture.as_ref().and_then(|t| tex.get(&(t.clone(), kind(&m.material, team))).cloned());
            (meshes.add(model_mesh(m)), materials.add(standard(&m.material, texture)))
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    mut loaded: ResMut<Loaded>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OriginalMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let decoded = loaded.images.take().unwrap_or_default();
    let tex = textures::upload(decoded, std::mem::take(&mut loaded.failures), &mut images);
    for f in tex.failures.iter().take(20) {
        warn!("{f}");
    }
    let tex = tex.handles;
    // Terrain layers, later batches on top (drawn before other transparents).
    let no_terrain = std::env::var_os("AA_BEVY_NO_TERRAIN").is_some();
    for (order, draw) in loaded.terrain.iter().enumerate().filter(|_| !no_terrain) {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, draw.positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, draw.normals.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, draw.uvs.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, draw.cell_uvs.clone());
        mesh.insert_indices(Indices::U32(draw.indices.clone()));
        let texture = draw.texture.as_ref().and_then(|t| tex.get(&(t.clone(), Kind::Color)).cloned());
        let mask = draw.mask.as_ref().and_then(|m| tex.get(&(m.texture.clone(), Kind::Mask)).cloned());
        let mask = mask.zip(draw.mask.as_ref().map(|m| m.rows));
        let material = materials.add(look::terrain_material(texture, draw.color, mask, order));
        commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(material), bevy::light::NotShadowCaster));
    }
    // Every placed map object with a resolved model.
    let mut parts = HashMap::new();
    for (key, model) in &loaded.scene.models {
        parts.insert(key.clone(), model_parts(model, NEUTRAL_TEAM, &tex, &mut meshes, &mut materials));
    }
    let mut objects = 0usize;
    // Placements the world turned into actors (or removed with unused slots).
    let world = loaded.world.take().expect("battle world");
    let scenery: std::collections::HashSet<usize> = world.books.scenery.iter().copied().collect();
    let key = |thing: &str, x: f32, y: f32| (thing.to_ascii_lowercase(), (x * 100.0).round() as i64, (y * 100.0).round() as i64);
    let not_scenery: std::collections::HashSet<_> = world
        .map
        .placed
        .iter()
        .enumerate()
        .filter(|(i, _)| !scenery.contains(i))
        .map(|(_, p)| key(&p.thing_id, p.position[0], p.position[1]))
        .collect();
    let mut modes = std::collections::BTreeMap::new();
    let no_objects = std::env::var_os("AA_BEVY_NO_OBJECTS").is_some();
    for p in loaded.scene.instances.iter().filter(|_| !no_objects) {
        let category = loaded.scene.map.placements.types.get(&p.thing_id).and_then(|t| t.category.as_deref());
        if matches!(category, Some("START_LOC" | "PLACEHOLDER")) {
            continue;
        }
        if not_scenery.contains(&key(&p.thing_id, p.position[0], -p.position[2])) {
            continue;
        }
        let Some(list) = p.model_key.as_ref().and_then(|k| parts.get(k)) else { continue };
        let transform = Transform::from_translation(Vec3::from_array(p.position))
            .with_rotation(Quat::from_rotation_y(p.yaw_radians))
            .with_scale(Vec3::splat(p.scale));
        for (mesh, material) in list {
            commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), transform));
            let mode = materials.get(material).map(|m| format!("{:?}", m.base.alpha_mode)).unwrap_or_default();
            *modes.entry(mode).or_insert(0usize) += 1;
        }
        objects += 1;
    }
    // Ground height in render space for the camera and units.
    // Corner heights once (the per-query metadata parse is too slow per frame),
    // then the same bilinear sampling as `TerrainMap::height_at_native`.
    let map = &loaded.scene.map.terrain;
    let metadata = aa_assets::TerrainMetadata::bundled().ok();
    let (gw, gh, cell) = (map.width, map.height, map.cell_size);
    let corners: Vec<f32> = (0..=gh)
        .flat_map(|y| (0..=gw).map(move |x| (x, y)))
        .map(|(x, y)| {
            metadata
                .as_ref()
                .and_then(|m| map.native_height_at_grid(x, y, m).map(|h| h.0))
                .or_else(|| map.height_at_grid(x, y, HeightMode::BaseOnly))
                .unwrap_or(0.0)
        })
        .collect();
    let world_heights = corners.clone();
    let ground = move |x: f32, z: f32| {
        let (gx, gy) = ((x / cell).clamp(0.0, gw as f32), (-z / cell).clamp(0.0, gh as f32));
        let (x0, y0) = (gx.floor() as u32, gy.floor() as u32);
        let (x1, y1) = ((x0 + 1).min(gw), (y0 + 1).min(gh));
        let (tx, ty) = (gx - x0 as f32, gy - y0 as f32);
        let h = |a: u32, b: u32| corners[(b * (gw + 1) + a) as usize];
        (h(x0, y0) * (1.0 - tx) + h(x1, y0) * tx) * (1.0 - ty) + (h(x0, y1) * (1.0 - tx) + h(x1, y1) * tx) * ty
    };
    let hq_at = loaded.hq_at;
    let hq_y = ground(hq_at.x, hq_at.y);
    // Display test world: aa-play actors drawn by battle_view (no gameplay yet).
    let _ = world_heights;
    // The camera starts over the local player's first building (its Corps HQ).
    let hq_at = world
        .actors
        .iter()
        .find(|a| a.kingdom == Some(0) && !a.removed && world.data.def(a.def).class == Some(aa_play::DefClass::Building) && !world.data.def(a.def).invisible)
        .map(|a| Vec2::new(a.pos.x, -a.pos.y))
        .unwrap_or(hq_at);
    let hq_y = ground(hq_at.x, hq_at.y);
    commands.insert_resource(session::Battle::new(world, 0));
    commands.insert_resource(crate::battle_view::Library::new(loaded.store.clone(), &loaded.rigs, loaded.fixes.clone()));
    // Sun with cascaded shadows over the battlefield.
    commands.spawn((
        // Shadows on, as in Development 12 (off with the fixed-function look).
        DirectionalLight { illuminance: 2400.0, shadow_maps_enabled: !look::fixed_function(), ..default() },
        Transform::default().looking_to(-look::sun_to_light(), Vec3::Y),
        CascadeShadowConfigBuilder { num_cascades: 2, first_cascade_far_bound: 60.0, maximum_distance: 220.0, ..default() }.build(),
    ));
    let b = loaded.scene.terrain.bounds;
    let focus2 = loaded.focus.map(|f| Vec2::new(f.x, -f.y)).unwrap_or(hq_at);
    let focus = Vec3::new(focus2.x, ground(focus2.x, focus2.y), focus2.y);
    commands.spawn((
        Camera3d::default(),
        // The original is fixed-function lighting: keep authored colours.
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Transform::default(),
        camera::RtsCamera {
            focus,
            home: Vec3::new(hq_at.x, hq_y, hq_at.y),
            yaw: 0.6,
            pitch: 0.78,
            distance: 42.0,
            min: Vec2::new(b.min[0], b.min[2]),
            max: Vec2::new(b.max[0], b.max[2]),
        },
    ));
    commands.insert_resource(camera::Ground(Box::new(ground)));
    // Debug overlay only on request (AA_BEVY_DEBUG): the original shows no such text.
    if std::env::var_os("AA_BEVY_DEBUG").is_some() {
        commands.spawn((
            Text::new(format!("{} - {objects} map objects, {} terrain layers", loaded.scene.map.terrain.name, loaded.terrain.len())),
            TextFont { font_size: FontSize::Px(15.0), ..default() },
            TextColor(Color::WHITE),
            Node { position_type: PositionType::Absolute, top: Val::Px(8.0), left: Val::Px(10.0), ..default() },
            HudText,
        ));
    }
    info!("object meshes by alpha mode: {modes:?}");
    info!("Bevy test: {objects} objects, {} terrain layers, {} textures, loaded in {:.1} s", loaded.terrain.len(), tex.len(), loaded.seconds);
}

fn hud(time: Res<Time>, mut frames: Local<(f32, u32, f32)>, mut text: Query<&mut Text, With<HudText>>) {
    frames.0 += time.delta_secs();
    frames.1 += 1;
    if frames.0 >= 1.0 {
        frames.2 = frames.1 as f32 / frames.0;
        *frames = (0.0, 0, frames.2);
        info!("FPS {:.1} at {:.0} s", frames.2, time.elapsed_secs());
        for mut t in &mut text {
            let base = t.0.split("\nFPS").next().unwrap_or("").to_owned();
            t.0 = format!("{base}\nFPS {:.0}", frames.2);
        }
    }
}
