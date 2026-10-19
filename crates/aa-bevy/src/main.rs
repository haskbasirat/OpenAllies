//! Axis & Allies Bevy test (Development 12).
//!
//! A presentation experiment: the original map, terrain layers, placed
//! objects and animation rigs from the user's install, drawn with Bevy so the
//! engine can be judged. No gameplay runs here. The HQ, soldiers and tank are
//! placed for viewing only.
mod camera;
mod terrain;
mod textures;
mod units;

use aa_animation::AnimatedAsset;
use aa_assets::{AssetStore, HeightMode, PreparedModel, PreparedScene};
use anyhow::{Context, Result};
use bevy::asset::{RenderAssetUsages, uuid_handle};
use bevy::light::CascadeShadowConfigBuilder;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::{DistanceFog, ExtendedMaterial, FogFalloff, MaterialExtension, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Face, ShaderType};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::shader::{Shader, ShaderRef};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use textures::Kind;

const TITLE: &str = "Axis & Allies - Bevy test (Development 12)";
const TERRAIN_SHADER: Handle<Shader> = uuid_handle!("6f1d2c4b-8a3e-4f57-9b1c-2e7d5a9c0b13");
/// Team colours mixed into team-masked textures (display choice).
const PLAYER_TEAM: [u8; 3] = [178, 34, 30];
const NEUTRAL_TEAM: [u8; 3] = [120, 120, 112];

type TerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainMask>;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct TerrainMask {
    #[uniform(100)]
    params: MaskParams,
    #[texture(101)]
    #[sampler(102)]
    mask: Option<Handle<Image>>,
}
#[derive(ShaderType, Debug, Clone)]
struct MaskParams {
    mask_u: Vec4,
    mask_v: Vec4,
    flags: Vec4,
}
impl MaterialExtension for TerrainMask {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(TERRAIN_SHADER)
    }
}

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
    scene: PreparedScene,
    terrain: Vec<terrain::Draw>,
    hq: Option<PreparedModel>,
    tank: Option<PreparedModel>,
    infantry: Option<Arc<AnimatedAsset>>,
    images: Option<Vec<((String, Kind), Image)>>,
    failures: Vec<String>,
    hq_at: Vec2,
    focus: Option<Vec2>,
    seconds: f32,
}

fn model(store: &AssetStore, path: &str) -> Result<PreparedModel> {
    Ok(store.load_model(path)?.prepare()?)
}

fn kind(m: &aa_assets::Material, team: [u8; 3]) -> Kind {
    if m.team_color_mask { Kind::Team(team) } else { Kind::Color }
}

fn load(args: &Args) -> Result<Loaded> {
    let start = Instant::now();
    let store = AssetStore::open(&args.assets)?;
    let scene = store.load_scene(&args.map, HeightMode::NativeConditional)?;
    let terrain = terrain::build(&scene.map.terrain, &scene.terrain)?;
    let hq = model(&store, "Buildings/RUS/RUSCorps/RUSCorps.nif").map_err(|e| eprintln!("HQ model: {e:#}")).ok();
    let tank = model(&store, "Units/RUS/RUSTankMedium/RUSTankMedium.nif").map_err(|e| eprintln!("tank model: {e:#}")).ok();
    let infantry = match AnimatedAsset::load(args.rigs.join("RUSInfantry.rig.json")) {
        Ok(mut asset) => {
            // Same material source as the client's animation path: the static model.
            if let Ok(model) = model(&store, &asset.source) {
                if model.meshes.len() == asset.meshes.len() {
                    for (rig, mesh) in asset.meshes.iter_mut().zip(&model.meshes) {
                        rig.material = mesh.material.clone();
                    }
                }
            }
            Some(Arc::new(asset))
        }
        Err(e) => {
            eprintln!("infantry rig: {e:#}");
            None
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
    for m in hq.iter().chain(tank.iter()) {
        for mesh in &m.meshes {
            if let Some(t) = &mesh.material.texture {
                requests.add(t, kind(&mesh.material, PLAYER_TEAM));
            }
        }
    }
    if let Some(a) = &infantry {
        for mesh in &a.meshes {
            if let Some(t) = &mesh.material.texture {
                requests.add(t, kind(&mesh.material, PLAYER_TEAM));
            }
        }
    }
    let (images, failures) = textures::decode(&store, requests);
    Ok(Loaded {
        scene,
        terrain,
        hq,
        tank,
        infantry,
        images: Some(images),
        failures,
        hq_at,
        focus: args.focus,
        seconds: start.elapsed().as_secs_f32(),
    })
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let loaded = load(&args)?;
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: TITLE.into(),
                resolution: (1600u32, 900u32).into(),
                present_mode: if std::env::var_os("AA_BEVY_VSYNC").is_some() { bevy::window::PresentMode::AutoVsync } else { bevy::window::PresentMode::AutoNoVsync },
                ..default()
            }),
            ..default()
        }),
    )
    .add_plugins(MaterialPlugin::<TerrainMaterial>::default())
    .insert_resource(ClearColor(Color::srgb(0.62, 0.68, 0.74)))
    .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 420.0, ..default() })
    .insert_resource(loaded)
    .add_systems(Startup, setup)
    .add_systems(Update, (camera::update, units::animate, units::walk, hud));
    let _ = app.world_mut().resource_mut::<Assets<Shader>>().insert(
        TERRAIN_SHADER.id(),
        Shader::from_wgsl(include_str!("terrain_mask.wgsl"), "aa_bevy/terrain_mask.wgsl"),
    );
    if let Some((path, seconds)) = args.shot {
        app.insert_resource(ShotPlan { path, seconds, taken: false, frames: 0 }).add_systems(Update, screenshot);
    }
    app.run();
    Ok(())
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

fn standard(m: &aa_assets::Material, texture: Option<Handle<Image>>) -> StandardMaterial {
    // Same alpha decisions as the client's `loading::material`.
    let blend = m.native_render_state.map_or(m.alpha_blend || m.opacity < 1.0, |s| s.alpha_flags & 1 != 0);
    let cutoff = if m.team_color_mask { 0.0 } else { m.alpha_test };
    let alpha_mode = if m.team_color_mask {
        AlphaMode::Opaque
    } else if blend {
        if std::env::var_os("AA_BEVY_A2C").is_some() { AlphaMode::AlphaToCoverage } else { AlphaMode::Blend }
    } else if cutoff > 0.0 {
        AlphaMode::Mask(cutoff)
    } else {
        AlphaMode::Opaque
    };
    StandardMaterial {
        base_color: Color::srgba(m.color[0], m.color[1], m.color[2], m.opacity),
        base_color_texture: texture,
        alpha_mode,
        double_sided: m.double_sided,
        cull_mode: (!m.double_sided).then_some(Face::Back),
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        metallic: 0.0,
        ..default()
    }
}

fn model_mesh(mesh: &aa_assets::PreparedMesh) -> Mesh {
    let mut out = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    out.insert_attribute(Mesh::ATTRIBUTE_POSITION, mesh.vertices.iter().map(|v| v.position).collect::<Vec<_>>());
    out.insert_attribute(Mesh::ATTRIBUTE_NORMAL, mesh.vertices.iter().map(|v| v.normal).collect::<Vec<_>>());
    out.insert_attribute(Mesh::ATTRIBUTE_UV_0, mesh.vertices.iter().map(|v| v.uv).collect::<Vec<_>>());
    out.insert_attribute(Mesh::ATTRIBUTE_COLOR, mesh.vertices.iter().map(|v| v.color).collect::<Vec<_>>());
    out.insert_indices(Indices::U32(mesh.indices.clone()));
    out
}

/// Mesh and material handles for every mesh of a model.
fn model_parts(
    model: &PreparedModel,
    team: [u8; 3],
    tex: &HashMap<(String, Kind), Handle<Image>>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Vec<(Handle<Mesh>, Handle<StandardMaterial>)> {
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
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
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
        let overlay = draw.mask.is_some();
        let rows = draw.mask.as_ref().map_or([[0.0; 4]; 2], |m| m.rows);
        let material = terrain_materials.add(TerrainMaterial {
            base: StandardMaterial {
                base_color: Color::srgba(draw.color[0], draw.color[1], draw.color[2], 1.0),
                base_color_texture: texture,
                alpha_mode: if overlay { AlphaMode::Blend } else { AlphaMode::Opaque },
                depth_bias: -1.0e8 + (order as f32) * 1000.0,
                perceptual_roughness: 1.0,
                reflectance: 0.0,
                ..default()
            },
            extension: TerrainMask {
                params: MaskParams {
                    mask_u: Vec4::from_array(rows[0]),
                    mask_v: Vec4::from_array(rows[1]),
                    flags: Vec4::new(if overlay && mask.is_some() { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0),
                },
                mask,
            },
        });
        commands.spawn((Mesh3d(meshes.add(mesh)), MeshMaterial3d(material), bevy::light::NotShadowCaster));
    }
    // Every placed map object with a resolved model.
    let mut parts = HashMap::new();
    for (key, model) in &loaded.scene.models {
        parts.insert(key.clone(), model_parts(model, NEUTRAL_TEAM, &tex, &mut meshes, &mut materials));
    }
    let mut objects = 0usize;
    let mut modes = std::collections::BTreeMap::new();
    let no_objects = std::env::var_os("AA_BEVY_NO_OBJECTS").is_some();
    for p in loaded.scene.instances.iter().filter(|_| !no_objects) {
        let category = loaded.scene.map.placements.types.get(&p.thing_id).and_then(|t| t.category.as_deref());
        if matches!(category, Some("START_LOC" | "PLACEHOLDER")) {
            continue;
        }
        let Some(list) = p.model_key.as_ref().and_then(|k| parts.get(k)) else { continue };
        let transform = Transform::from_translation(Vec3::from_array(p.position))
            .with_rotation(Quat::from_rotation_y(p.yaw_radians))
            .with_scale(Vec3::splat(p.scale));
        for (mesh, material) in list {
            commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), transform));
            let mode = materials.get(material).map(|m| format!("{:?}", m.alpha_mode)).unwrap_or_default();
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
    // Display-only HQ at the first start location, with soldiers and a tank.
    if let Some(hq) = &loaded.hq {
        for (mesh, material) in model_parts(hq, PLAYER_TEAM, &tex, &mut meshes, &mut materials) {
            commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::from_xyz(hq_at.x, hq_y, hq_at.y)));
        }
    }
    if let Some(asset) = &loaded.infantry {
        let mats: Vec<_> = asset
            .meshes
            .iter()
            .map(|m| {
                let t = m.material.texture.as_ref().and_then(|t| tex.get(&(t.clone(), kind(&m.material, PLAYER_TEAM))).cloned());
                materials.add(standard(&m.material, t))
            })
            .collect();
        let idle = units::clip_index(asset, "InfantryIdle").unwrap_or(0);
        let jog = units::clip_index(asset, "InfantryJog").unwrap_or(0);
        let attack = units::clip_index(asset, "InfantryAttack1").unwrap_or(idle);
        // Idle squad in a line in front of the HQ.
        for (clip, row, phase) in [(idle, 0.0, 0.0), (attack, 1.6, 0.4)] {
            let Ok(handles) = units::group_meshes(asset, &mut meshes) else { break };
            commands.spawn(units::PoseGroup { asset: asset.clone(), clip, phase, meshes: handles.clone() });
            for i in 0..6 {
                let x = hq_at.x - 6.0 + i as f32 * 2.0;
                let z = hq_at.y + 12.0 + row;
                let y = ground(x, z);
                let transform = Transform::from_xyz(x, y, z);
                for (mesh, material) in handles.iter().zip(&mats) {
                    commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone()), transform));
                }
            }
        }
        // Jogging squad circling the HQ.
        if let Ok(handles) = units::group_meshes(asset, &mut meshes) {
            commands.spawn(units::PoseGroup { asset: asset.clone(), clip: jog, phase: 0.0, meshes: handles.clone() });
            for i in 0..8 {
                let offset = Vec2::new((i % 2) as f32 * 1.6 - 0.8, (i / 2) as f32 * 1.8);
                for (mesh, material) in handles.iter().zip(&mats) {
                    commands.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::default(),
                        units::LoopPath { center: hq_at, radius: 16.0, speed: 3.5, angle: 0.0, offset },
                    ));
                }
            }
        }
    }
    if let Some(tank) = &loaded.tank {
        for (mesh, material) in model_parts(tank, PLAYER_TEAM, &tex, &mut meshes, &mut materials) {
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::default(),
                units::LoopPath { center: hq_at, radius: 26.0, speed: 6.0, angle: 2.0, offset: Vec2::ZERO },
            ));
        }
    }
    // Sun with cascaded shadows over the battlefield.
    commands.spawn((
        DirectionalLight { illuminance: 2400.0, shadow_maps_enabled: std::env::var_os("AA_BEVY_NO_SHADOWS").is_none(), ..default() },
        Transform::from_xyz(-60.0, 120.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
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
        DistanceFog {
            color: Color::srgb(0.62, 0.68, 0.74),
            falloff: FogFalloff::Linear { start: 220.0, end: 520.0 },
            ..default()
        },
    ));
    commands.insert_resource(camera::Ground(Box::new(ground)));
    commands.spawn((
        Text::new(format!(
            "{TITLE}\n{} - {objects} map objects, {} terrain layers, loaded in {:.1} s\n\
             Arrows/WASD or screen edges: pan   Wheel or PgUp/PgDn: zoom   Q/E: rotate   R/F: tilt   Home: HQ\n\
             Display only: the HQ, soldiers and tank are placed to show the engine; no gameplay runs here.",
            loaded.scene.map.terrain.name,
            loaded.terrain.len(),
            loaded.seconds
        )),
        TextFont { font_size: FontSize::Px(15.0), ..default() },
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, top: Val::Px(8.0), left: Val::Px(10.0), ..default() },
        HudText,
    ));
    info!("object meshes by alpha mode: {modes:?}");
    info!("Bevy test: {objects} objects, {} terrain layers, {} textures, loaded in {:.1} s", loaded.terrain.len(), tex.len(), loaded.seconds);
}

fn hud(time: Res<Time>, mut frames: Local<(f32, u32, f32)>, mut text: Query<&mut Text, With<HudText>>) {
    frames.0 += time.delta_secs();
    frames.1 += 1;
    if frames.0 >= 1.0 {
        frames.2 = frames.1 as f32 / frames.0;
        *frames = (0.0, 0, frames.2);
        for mut t in &mut text {
            let base = t.0.split("\nFPS").next().unwrap_or("").to_owned();
            t.0 = format!("{base}\nFPS {:.0}", frames.2);
            info!("FPS {:.1} at {:.0} s", frames.2, time.elapsed_secs());
        }
    }
}
