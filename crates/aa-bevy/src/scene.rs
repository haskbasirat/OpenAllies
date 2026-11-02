//! Battle scene drawing: the original map, terrain layers, placed objects and
//! animation rigs from the user's install. Owner: lead (battle renderer).
//! The display-only HQ, soldiers and tank of the Development 12 test stay
//! until the aa-play world draws real units.
use crate::{camera, session, terrain, textures, units};

use aa_animation::AnimatedAsset;
use aa_parity::playback::FilmPlayer;
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
/// Team colours mixed into team-masked textures: the direct start's player
/// colour when the kingdom colour table is missing (display choice), and
/// the replacement colour of a thing without a kingdom colour: AVar 30
/// ReplacementColorDefault = 1,1,1 (Data/AVars.tgi; Game/kingdom_colors.tgi
/// lists the same white "default" last, for the code).
pub const PLAYER_TEAM: [u8; 3] = [178, 34, 30];
pub const NEUTRAL_TEAM: [u8; 3] = [255, 255, 255];


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
    /// The film being played on that world.
    film: Option<FilmPlayer>,
    /// The battle's sun: the map's saved SUN chunk, or the SVars sun of a
    /// new (random) world.
    sun: look::SceneSun,
    /// Every thing's View shadow type (0x49DC40).
    shadows: Option<Arc<crate::preload::ShadowTypes>>,
}

/// The saved Sun of an original map file (`<reconstruction>/assets/original/
/// Data/Maps/<id>.RMP`, chunk "SUN"), if the file has one.
fn map_sun(reconstruction: &std::path::Path, id: &str) -> Option<look::SceneSun> {
    let maps = reconstruction.join("assets/original/Data/Maps");
    let path = ["RMP", "rmp"].iter().map(|ext| maps.join(format!("{id}.{ext}"))).find(|p| p.exists())?;
    file_sun(&path)
}

/// The saved Sun of a map or saved game file, chunk "SUN".
fn file_sun(path: &std::path::Path) -> Option<look::SceneSun> {
    let archive = aa_mapworld::MapArchive::from_bytes(std::fs::read(path).ok()?).ok()?;
    let chunk = archive.decode_chunk("SUN", 1 << 16).ok()?;
    look::SceneSun::from_map_chunk(&chunk.bytes)
}

/// `KKC_GWorld::StartMission` 0x4FDCE0: a Sun world rule away from its default
/// sets the sun's colours (0x48D500 finds `Data/Sun/<ids>`, 0x4EDB10 with no
/// transition); at its default the map's or the SVars' sun stays. The rule is
/// its EnumValue IDS or, in a film's header, its value index (0x512420). Returns
/// the Sun's IDS when it applies, for the world's own Sun (`set_rule_sun`).
fn apply_sun_rule(sun: &mut look::SceneSun, reconstruction: &std::path::Path, ids: Option<&str>, index: Option<u32>) -> Option<String> {
    let data = aa_ui::fs::DataRoot::with_overlay(reconstruction.join("assets/original/Data"), reconstruction.join("native-game/runtime-expanded-v14/Data"));
    let rules = crate::ui::datafile::load(&data, "Game/world_rules_aa.tgi");
    let rule = rules.iter().find(|n| n.name.eq_ignore_ascii_case("WorldRule") && n.get("ids").is_some_and(|i| i.eq_ignore_ascii_case("Sun")))?;
    let values: Vec<&str> = rule.children_named("EnumValue").filter_map(|v| v.get("ids")).collect();
    let chosen = ids.or_else(|| values.get(index? as usize).copied());
    let chosen = chosen.filter(|c| !rule.get("default").is_some_and(|d| d.eq_ignore_ascii_case(c)))?;
    let file = crate::ui::datafile::load(&data, &format!("Sun/{chosen}.tgi"));
    let Some(found) = file.iter().find(|n| n.name.eq_ignore_ascii_case("Sun")) else {
        warn!("World rule specified sun '{chosen}' no found - using default");
        return None;
    };
    let triple = |key: &str, into: &mut [f32; 3]| {
        let parts: Vec<f32> = found.get(key).map(|v| v.split(',').filter_map(|p| p.trim().parse().ok()).collect()).unwrap_or_default();
        if let [a, b, c] = parts[..] {
            *into = [a, b, c];
        }
    };
    triple("Diffuse", &mut sun.diffuse);
    triple("Ambient", &mut sun.ambient);
    triple("Specular", &mut sun.specular);
    Some(chosen.to_owned())
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
    /// A random-map film: the generator's run its world's map comes from.
    Generated(Box<aa_play::mapgen::GeneratedMap>),
}

/// A generated map as a scene: its terrain plus every scenery model.
fn generated_scene(store: &AssetStore, mut generated: aa_play::mapgen::GeneratedMap) -> Result<PreparedScene> {
    // A map file's own palette names ids `Data/translate.tgi` renames.
    store.fill_palette(&mut generated.terrain);
    let terrain = generated.terrain.prepare(HeightMode::NativeConditional)?;
    // Every distinct scenery model, read and prepared in parallel.
    let mut unique: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for g in &generated.scenery {
        if let Some(model) = &g.model {
            unique.entry(model.replace('\\', "/").to_lowercase()).or_insert_with(|| model.clone());
        }
    }
    let loaded: Vec<(String, String, Result<aa_assets::PreparedModel>)> = {
        use rayon::prelude::*;
        unique
            .into_par_iter()
            .map(|(key, model)| {
                let prepared = store.load_model(&model).map_err(anyhow::Error::from).and_then(|m| m.prepare().map_err(anyhow::Error::from));
                (key, model, prepared)
            })
            .collect()
    };
    let mut models = std::collections::BTreeMap::new();
    let mut warnings = Vec::new();
    for (key, model, prepared) in loaded {
        match prepared {
            Ok(m) => {
                models.insert(key, m);
            }
            Err(e) => warnings.push(aa_assets::AssetWarning { source: model, message: format!("{e:#}") }),
        }
    }
    let mut instances = Vec::new();
    for (i, g) in generated.scenery.iter().enumerate() {
        let Some(model) = &g.model else { continue };
        let key = model.replace('\\', "/").to_lowercase();
        if !models.contains_key(&key) {
            continue;
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
    Ok(PreparedScene { map: aa_assets::MapAsset { terrain: generated.terrain, placements }, terrain, instances, models, warnings })
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

/// Loading step timings in the log (`load step <name>: <ms>`).
struct Steps(Instant);
impl Steps {
    fn mark(&mut self, name: &str) {
        info!("load step {name}: {:.0} ms", self.0.elapsed().as_secs_f64() * 1000.0);
        self.0 = Instant::now();
    }
}

#[allow(clippy::too_many_arguments)]
fn load(args: &Args, source: MapSource, setup: Option<aa_play::SkirmishSetup>, profiles: Vec<(usize, String)>, factions: Vec<(usize, Option<String>)>, film: Option<(aa_play::World, FilmPlayer)>, sun_rule: Option<String>) -> Result<Loaded> {
    let start = Instant::now();
    let mut steps = Steps(Instant::now());
    let store = (*crate::preload::store(&args.assets)?).clone();
    steps.mark("asset store");
    let root = args.assets.parent().context("the install root above the reconstruction folder")?;
    let mut map_id = None;
    let mut random_generals = None;
    let (mut scene, game_map) = match source {
        MapSource::File(id) => {
            let scene = store.load_scene(&id, HeightMode::NativeConditional)?;
            map_id = Some(id);
            (scene, None)
        }
        MapSource::Random(params) => {
            session::set_load_status("app_status_generating_map");
            // The battle starts as a film of it starts (0x513410): the creator's
            // lists, the kingdoms' picks on the world stream, then the generator.
            let maps = crate::preload::random_maps(root)?;
            let generated = match &setup {
                Some(setup) => {
                    let data = crate::preload::game_data(root)?;
                    let battle = aa_parity::recording::random_battle(&maps, &data, &params, setup, &factions)?;
                    random_generals = Some(battle.generals);
                    battle.generated
                }
                None => maps.generate(&params)?,
            };
            let map = generated.map.clone();
            let scene = generated_scene(&store, generated)?;
            session::set_load_status("app_status_verifying_map");
            (scene, Some(map))
        }
        MapSource::Generated(generated) => {
            session::set_load_status("app_status_verifying_map");
            (generated_scene(&store, *generated)?, None)
        }
    };
    steps.mark("map scene");
    // The audited original-NIF corrections (shadow helpers, team colour
    // flags, alpha states), exactly as the wgpu client applies them.
    let fixes = crate::preload::fixes(&args.assets)?;
    {
        use rayon::prelude::*;
        scene.models.par_iter_mut().for_each(|(key, model)| {
            if let Err(e) = fixes.apply(&store, model) {
                eprintln!("model fixes {key}: {e:#}");
            }
        });
    }
    steps.mark("model fixes");
    let terrain = terrain::build(&scene.map.terrain, &scene.terrain)?;
    steps.mark("terrain layers");
    let (mut world, film) = match film {
        // The film's own world (0x5A3980 made it from the film's map).
        Some((world, player)) => {
            session::set_load_status("app_status_loading_objects");
            (world, Some(player))
        }
        None => {
            // The battle world from the original rules and map.
            let data = crate::preload::game_data(root)?;
            steps.mark("game data");
            let random_map = game_map.is_some();
            let map_id = map_id.clone().unwrap_or_else(|| "random".to_owned());
            let world_map = match &game_map {
                Some(m) => m.clone(),
                None => aa_play::GameMap::load_with(root, &map_id, &data)?,
            };
            steps.mark("game map");
            let setup = setup.unwrap_or_else(|| default_setup(&map_id, &data));
            session::set_load_status("app_status_loading_objects");
            // WorldParameters 0x50E8B0 draws the general of a kingdom staging left open,
            // during the map's restore (a film's replay draws the same way).
            let chosen = |i: usize| factions.iter().any(|(p, f)| *p == i && f.is_some());
            let picks: Vec<aa_play::creation::StartPick> =
                (0..setup.players.len()).map(|i| aa_play::creation::StartPick { random_nation: false, random_general: !random_map && !chosen(i) }).collect();
            let resources = data.resources.len();
            let mut world = aa_play::World::new_skirmish_with(data, Arc::new(world_map), &setup, &picks)?;
            // A random map's creator drew its picks before the generator.
            for (k, general) in random_generals.into_iter().flatten().enumerate() {
                if general.is_some() {
                    world.books.kingdom_mut(k as u32, resources).general = general;
                }
            }
            // The film tables, on this thread rather than in the battle's first frame.
            if let Err(e) = aa_parity::registries::Registries::cached(root, None) {
                warn!("film registries: {e:#}");
            }
            steps.mark("new skirmish");
            // The original computer players (Data/SAI profiles); cached after the first load.
            aa_play::ai::attach(&mut world, crate::preload::ai_data(root)?);
            steps.mark("ai data");
            // The generals the setup screen chose for the computer slots (kingdom id = player index).
            for (player, profile) in &profiles {
                aa_play::ai::set_profile(&mut world, *player as u32, profile);
            }
            // The generals' special operations (Data/Generals; each kingdom's general
            // is set at battle start, ui/abilities.rs). Appended by the combat owner.
            match crate::preload::generals(root) {
                Ok(g) => aa_play::special_ops::attach(&mut world, g),
                Err(e) => warn!("generals: {e:#}"),
            }
            steps.mark("generals");
            (world, None)
        }
    };
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
    session::set_load_status("app_status_loading_resources");
    // The map's Biome texture subfolder (Arctic, Desert, Tropical) for every
    // texture of this battle, as the original sets it at world init.
    let subfolder = world.data.terrain.texture_subfolder(&world.map.biome);
    info!("biome {:?}: texture subfolder {subfolder:?}", world.map.biome);
    textures::set_biome_subfolder(subfolder);
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
    steps.mark("textures");
    // A loaded map keeps its saved Sun (4FC600 -> 4EDBD0); a generated world
    // has the SVars sun of 4EE1F0 (the RMCs list no suns for 5925E0).
    let source_file = film.as_ref().and_then(|p| p.scene().source_file.as_deref());
    let mut sun = map_id.as_deref().and_then(|id| map_sun(&args.assets, id)).or_else(|| source_file.and_then(file_sun)).unwrap_or_else(|| look::vars().sun);
    let film_rule = film.as_ref().and_then(|p| p.film().header.map_header.creator.rules.rules.iter().find(|r| String::from_utf16_lossy(&r.key).eq_ignore_ascii_case("Sun")).map(|r| r.value));
    if let Some(ids) = apply_sun_rule(&mut sun, &args.assets, sun_rule.as_deref(), film_rule) {
        aa_play::triggers::set_rule_sun(&mut world, &ids);
    }
    info!("battle sun: {sun:?}");
    let shadows = crate::preload::shadow_types(&args.assets).map_err(|e| warn!("shadow types: {e:#}")).ok();
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
        film,
        sun,
        shadows,
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
    generals: Option<Res<crate::ui::abilities::PendingGenerals>>,
    pending_film: Option<Res<session::PendingFilm>>,
    sun_rule: Option<Res<session::PendingSunRule>>,
    mut progress: ResMut<session::LoadProgress>,
) {
    let mut film = pending_film.and_then(|p| p.0.lock().ok().and_then(|mut f| f.take()));
    commands.remove_resource::<session::PendingFilm>();
    let generated = film.as_mut().and_then(|(_, player)| player.take_generated());
    let map = match (generated, &random, &setup) {
        (Some(g), _, _) => MapSource::Generated(Box::new(g)),
        (None, Some(r), _) => MapSource::Random(r.0.clone()),
        (None, None, Some(s)) => MapSource::File(s.0.map_id.clone()),
        (None, None, None) => MapSource::File(args.0.map.clone()),
    };
    let label = match &map {
        MapSource::File(id) => id.clone(),
        MapSource::Random(r) => format!("random map {}", r.rmc),
        MapSource::Generated(g) => format!("random map {}", g.params.rmc),
    };
    *progress = session::LoadProgress { fraction: 0.0, status: format!("Loading {label}"), error: None };
    session::set_load_status("app_status_loading_data");
    let slot = Arc::new(Mutex::new(None));
    let out = slot.clone();
    let args = args.0.clone();
    let setup = setup.map(|s| s.0.clone());
    let profiles = profiles.map(|p| p.0.clone()).unwrap_or_default();
    let factions: Vec<(usize, Option<String>)> = generals.map_or_else(Vec::new, |g| g.factions.clone());
    let sun_rule = sun_rule.map(|s| s.0.clone());
    std::thread::spawn(move || {
        let result = load(&args, map, setup, profiles, factions, film, sun_rule).map_err(|e| format!("{e:#}"));
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
        crate::preload::start(self.args.assets.clone());
        // the original lighting variables, before any material is made
        look::load_vars(&self.args.assets);
        app.add_plugins(look::LookPlugin)
            // Outside the map the original shows black (screenshot of
            // (2v2)Market_Garden at the map edge: (0,0,0); the map-edge skirt
            // Special/SkirtTexture.dds is near-black too).
            .insert_resource(ClearColor(Color::BLACK))
            // The fixed-function look takes the battle's sun at unit
            // intensity (look::apply_sun); the PBR comparison look
            // (AA_BEVY_PBR_LOOK) keeps the Development 12 tuning.
            .insert_resource(GlobalAmbientLight {
                color: Color::WHITE,
                brightness: if look::fixed_function() { 1.0 } else { env_f32("AA_BEVY_AMBIENT", 340.0) },
                ..default()
            })
            .insert_resource(bevy::light::DirectionalLightShadowMap { size: 4096 })
            .add_systems(Update, textures::stream)
            .insert_resource(SceneArgs(self.args.clone()))
            .init_resource::<session::LoadProgress>()
            .add_systems(OnEnter(session::AppState::Loading), start_loading)
            .add_systems(Update, poll_loading.run_if(in_state(session::AppState::Loading)))
            .add_systems(OnEnter(session::AppState::Battle), setup)
            .add_systems(OnExit(session::AppState::Battle), leave_battle)
            .add_plugins(crate::battle_view::BattleViewPlugin)
            .add_systems(Update, (hud, crate::debug_battle::spawn, stream_terrain, look::apply_sun).run_if(in_state(session::AppState::Battle)));
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

/// What `setup` and `stream_terrain` spawned: map objects, terrain layers,
/// the sun and the camera.
#[derive(Component)]
struct SceneEntity;

/// The battle ended (`RequestFinished` 0x6125A0, ui/front.rs): its drawing
/// and per-battle state go with its world, so the next launch (the staging's
/// Launch, the score screen's Replay) starts from nothing. The actor views
/// are battle_view's, the effects and sounds their owners'.
fn leave_battle(mut commands: Commands, scene: Query<Entity, With<SceneEntity>>) {
    for e in &scene {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<TerrainStream>();
    commands.remove_resource::<Loaded>();
    commands.remove_resource::<camera::Ground>();
    commands.remove_resource::<crate::battle_view::Library>();
    commands.remove_resource::<look::SceneSun>();
    commands.insert_resource(session::ObjectAnimations::default());
    commands.insert_resource(crate::input::Selecting::default());
    commands.insert_resource(camera::CameraRequests::default());
    commands.insert_resource(crate::ui::placement::Placement::default());
    commands.insert_resource(crate::ui::placement::Ghost::default());
    commands.insert_resource(crate::ui::order_marks::PointVfx::default());
    commands.insert_resource(crate::ui::order_marks::Issued::default());
    // A film on a profile's data has drawn its art (ui/front.rs launch_film).
    if let Some(store) = crate::ui::reconstruction_root().and_then(|r| crate::preload::store(&r).ok()) {
        store.set_art_overlay(None);
    }
}

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
    let setup_started = Instant::now();
    let decoded = loaded.images.take().unwrap_or_default();
    let tex = textures::upload(decoded, std::mem::take(&mut loaded.failures), &mut images);
    for f in tex.failures.iter().take(20) {
        warn!("{f}");
    }
    let tex = tex.handles;
    // Terrain layers stream in over the next frames (`stream_terrain`).
    if std::env::var_os("AA_BEVY_NO_TERRAIN").is_none() {
        commands.insert_resource(TerrainStream { textures: tex.clone(), next: 0, frames: 0 });
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
    let mut shadow_counts = std::collections::BTreeMap::new();
    let no_objects = std::env::var_os("AA_BEVY_NO_OBJECTS").is_some();
    for p in loaded.scene.instances.iter().filter(|_| !no_objects) {
        let category = loaded.scene.map.placements.types.get(&p.thing_id).and_then(|t| t.category.as_deref());
        // (AREA, CAMERA: Flags invisible, template_area / template_camera: the
        // script's markers show in the editor only)
        if matches!(category, Some("START_LOC" | "PLACEHOLDER" | "AREA" | "CAMERA")) {
            continue;
        }
        if not_scenery.contains(&key(&p.thing_id, p.position[0], -p.position[2])) {
            continue;
        }
        // Flags invisible: the audio doodads' speakers show in the editor only
        if world.data.find(&p.thing_id).is_some_and(|d| world.data.def(d).invisible) {
            continue;
        }
        let Some(list) = p.model_key.as_ref().and_then(|k| parts.get(k)) else { continue };
        let transform = Transform::from_translation(Vec3::from_array(p.position))
            .with_rotation(Quat::from_rotation_y(p.yaw_radians))
            .with_scale(Vec3::splat(p.scale));
        // Only things with a shadow cast one (0x49DC40): shadow_type NONE
        // (tall grass, logs, docks...) or no shadow_texture casts nothing.
        // The sun's shadow map stands in for STENCIL volumes and, until the
        // ground blobs are drawn, for TEXTURE / PROJECTED shadows.
        let shadow = loaded.shadows.as_ref().and_then(|s| s.get(&p.thing_id));
        *shadow_counts.entry(format!("{shadow:?}")).or_insert(0usize) += 1;
        let casts = shadow != Some(crate::preload::ShadowType::None);
        for (mesh, material) in list {
            let mut entity = commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), transform, SceneEntity));
            if !casts {
                entity.insert(bevy::light::NotShadowCaster);
            }
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
    // AA_BEVY_TERRAIN_CHECK=1: the drawn terrain against the world the units
    // stand on, and the scenery the world keeps against what is drawn (test runs).
    if std::env::var_os("AA_BEVY_TERRAIN_CHECK").is_some() {
        let mut worst = (0.0f32, 0u32, 0u32);
        let mut differ = 0u32;
        let mut kinds = std::collections::BTreeMap::<String, u32>::new();
        for y in 0..=gh {
            for x in 0..=gw {
                let drawn = corners[(y * (gw + 1) + x) as usize];
                let there = world.map.height_at(Vec2::new(x as f32 * cell, y as f32 * cell));
                let d = (drawn - there).abs();
                if d > 0.01 {
                    differ += 1;
                    let tile = map.tiles.get((y.min(gh - 1) * gw + x.min(gw - 1)) as usize);
                    let name = tile.and_then(|t| map.palette.get(t)).map_or("?".to_owned(), |p| p.id.clone());
                    *kinds.entry(name).or_default() += 1;
                }
                if d > worst.0 {
                    worst = (d, x, y);
                }
            }
        }
        info!(
            "terrain check: {}x{} cells of {cell} m, world {}x{} of {}, worst height difference {} at corner ({}, {}), {differ} corners differ, on {kinds:?}; {objects} objects drawn, {} placed on the world's map",
            gw, gh, world.map.width, world.map.height, world.map.cell_size, worst.0, worst.1, worst.2, world.map.placed.len()
        );
    }
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
    // The camera starts on the local kingdom's start location (4F61A0
    // StartMission: Kingdom+1F8 position -> World+22C, 43F0A0 LookAt), else
    // over the local player's first building (its Corps HQ). A film's viewer
    // has no kingdom: half the map's width and height ([O]: lookat=(160,160)
    // on a 320x320 map).
    let film_view = loaded.film.is_some().then(|| Vec2::new(world.map.width as f32 * world.map.cell_size * 0.5, world.map.height as f32 * world.map.cell_size * 0.5));
    let hq_at = film_view
        .or_else(|| camera::start_location(&world, 0))
        .or_else(|| {
            world
                .actors
                .iter()
                .find(|a| a.kingdom == Some(0) && !a.removed && world.data.def(a.def).class == Some(aa_play::DefClass::Building) && !world.data.def(a.def).invisible)
                .map(|a| a.pos.truncate())
        })
        .map(|p| Vec2::new(p.x, -p.y))
        .unwrap_or(hq_at);
    let hq_y = ground(hq_at.x, hq_at.y);
    let mut battle = session::Battle::new(world, 0);
    if let Some(player) = loaded.film.take() {
        battle.attach_film(player);
    }
    commands.insert_resource(battle);
    commands.insert_resource(crate::battle_view::Library::new(loaded.store.clone(), &loaded.rigs, loaded.fixes.clone()));
    // The battle's sun (look::apply_sun hands it to this light every frame).
    let sun = loaded.sun.clone();
    let to_light = if std::env::var_os("AA_BEVY_SUN_DIR").is_some() { sun_dir() } else { sun.to_light() };
    commands.insert_resource(look::SceneSun { direction: Vec3::new(-to_light.x, to_light.z, -to_light.y).normalize_or_zero(), ..sun });
    // Sun with cascaded shadows over the battlefield: Bevy's shadow map
    // stands in for the original's stencil shadow volumes.
    commands.spawn((
        DirectionalLight {
            illuminance: if look::fixed_function() { 1.0 } else { env_f32("AA_BEVY_SUN", 1300.0) },
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::default().looking_to(-to_light, Vec3::Y),
        // The original camera looks from about 155 m (CameraHeight 100 / sin 40 deg,
        // zoom 0.7..2.1 = 110..325 m): the sharp cascade must cover that range.
        CascadeShadowConfigBuilder { num_cascades: 2, minimum_distance: 40.0, first_cascade_far_bound: 230.0, maximum_distance: 420.0, ..default() }.build(),
        SceneEntity,
    ));
    let b = loaded.scene.terrain.bounds;
    let focus2 = loaded.focus.map(|f| Vec2::new(f.x, -f.y)).unwrap_or(hq_at);
    let focus = Vec3::new(focus2.x, ground(focus2.x, focus2.y), focus2.y);
    commands.spawn((
        Camera3d::default(),
        // The original is fixed-function lighting: keep authored colours.
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        // No multisampling: the original's edges are aliased (its
        // screenshots; dgVoodoo.conf Antialiasing "appdriven").
        Msaa::Off,
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
        SceneEntity,
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
            SceneEntity,
        ));
    }
    info!("object meshes by alpha mode: {modes:?}");
    info!("map objects by shadow type: {shadow_counts:?}");
    info!("load step battle scene setup: {:.0} ms", setup_started.elapsed().as_secs_f64() * 1000.0);
    info!("Bevy test: {objects} objects, {} terrain layers, {} textures, loaded in {:.1} s", loaded.terrain.len(), tex.len(), loaded.seconds);
}

/// Terrain layers still to be created. Every new mesh and material is
/// prepared by the renderer in the frame it appears; a big map has about
/// 2000 layers (Berlin 1976, ~0.1 ms each), so they are spread over the
/// frames the loading screen still covers instead of one 0.2 s frame.
#[derive(Resource)]
struct TerrainStream {
    textures: HashMap<(String, Kind), Handle<Image>>,
    next: usize,
    frames: u32,
}

/// Terrain layers created per frame (about 20 ms of renderer preparation).
const TERRAIN_LAYERS_PER_FRAME: usize = 200;

/// Terrain layers, later batches on top (drawn before other transparents).
fn stream_terrain(
    mut commands: Commands,
    loaded: Option<Res<Loaded>>,
    stream: Option<ResMut<TerrainStream>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OriginalMaterial>>,
) {
    let (Some(loaded), Some(mut stream)) = (loaded, stream) else { return };
    let end = (stream.next + TERRAIN_LAYERS_PER_FRAME).min(loaded.terrain.len());
    for (order, draw) in loaded.terrain.iter().enumerate().take(end).skip(stream.next) {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, draw.positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, draw.normals.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, draw.uvs.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, draw.cell_uvs.clone());
        mesh.insert_indices(Indices::U32(draw.indices.clone()));
        let texture = draw.texture.as_ref().and_then(|t| stream.textures.get(&(t.clone(), Kind::Color)).cloned());
        let mask = draw.mask.as_ref().and_then(|m| stream.textures.get(&(m.texture.clone(), Kind::Mask)).cloned());
        let mask = mask.zip(draw.mask.as_ref().map(|m| m.rows));
        let material = materials.add(look::terrain_material(texture, draw.color, mask, order));
        commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(material), bevy::light::NotShadowCaster, SceneEntity));
    }
    stream.next = end;
    stream.frames += 1;
    if end >= loaded.terrain.len() {
        info!("terrain: {} layers created over {} frames", loaded.terrain.len(), stream.frames);
        commands.remove_resource::<TerrainStream>();
    }
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

/// A tuning value from the environment (display experiments), else `default`.
fn env_f32(name: &str, default: f32) -> f32 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// Render-space direction to the sun: the original SunDirection, or
/// AA_BEVY_SUN_DIR="x,y,z" (native, the way the light travels) for tests.
fn sun_dir() -> Vec3 {
    std::env::var("AA_BEVY_SUN_DIR")
        .ok()
        .and_then(|v| {
            let p: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            (p.len() == 3).then(|| -Vec3::new(p[0], p[2], -p[1]).normalize())
        })
        .unwrap_or_else(look::sun_to_light)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    /// 0x4FDCE0: a Sun rule away from its default takes the colours of its
    /// `Data/Sun/*.tgi`, by IDS or by a film's value index; the direction and
    /// the dimmer stay, and at the default nothing changes.
    #[test]
    fn a_sun_rule_away_from_its_default_sets_the_colours() {
        let Some(root) = crate::ui::reconstruction_root() else { return };
        let start = look::SceneSun { direction: Vec3::X, diffuse: [0.1; 3], ambient: [0.2; 3], specular: [0.3; 3], dimmer: 0.5 };
        let same = |s: &look::SceneSun| s.diffuse == [0.1; 3] && s.ambient == [0.2; 3] && s.specular == [0.3; 3];
        let mut sun = start.clone();
        apply_sun_rule(&mut sun, &root, Some("sun_default"), None);
        assert!(same(&sun));
        apply_sun_rule(&mut sun, &root, None, Some(0));
        assert!(same(&sun));
        apply_sun_rule(&mut sun, &root, None, None);
        assert!(same(&sun));
        apply_sun_rule(&mut sun, &root, Some("sun_dusk"), None);
        assert_eq!((sun.diffuse, sun.ambient, sun.specular), ([1.0, 0.8, 0.55], [0.8, 0.6, 0.4], [0.2, 0.1, 0.05]));
        assert_eq!((sun.direction, sun.dimmer), (Vec3::X, 0.5));
        let mut indexed = start.clone();
        apply_sun_rule(&mut indexed, &root, None, Some(2));
        assert_eq!(indexed.diffuse, [1.0, 0.8, 0.55]);
    }

    #[test]
    fn leaving_a_battle_clears_the_scene_and_its_state() {
        let mut world = bevy::prelude::World::new();
        let scene = world.spawn(SceneEntity).id();
        let other = world.spawn_empty().id();
        world.insert_resource(camera::Ground(Box::new(|_, _| 0.0)));
        world.insert_resource(session::ObjectAnimations([(1, ("recoil1".to_string(), 0.0))].into_iter().collect()));
        world.run_system_once(leave_battle).unwrap();
        assert!(world.get_entity(scene).is_err());
        assert!(world.get_entity(other).is_ok());
        assert!(!world.contains_resource::<camera::Ground>());
        assert!(world.resource::<session::ObjectAnimations>().0.is_empty());
        assert!(world.contains_resource::<crate::input::Selecting>());
        assert!(world.contains_resource::<camera::CameraRequests>());
    }
}
