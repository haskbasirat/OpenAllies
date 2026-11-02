//! The strategic map world in 3D: the map table props (mg_maptable.tgi, 939230:
//! at their `location`, z 2.0, facing (0, -1)), one territory doodad per MG
//! territory (955A40:
//! the definition's `object_ids` Thing at the territory `location`, z 2.0,
//! facing (0, -1)), drawn with its map tile textures, and one actor per army
//! at its display position (TerritoryPositionSolver 959420 on its slot, z 2.0,
//! facing (0.7071, -0.7071), 955B70), moved along the move waypoints
//! (962880: the link's intermediate points, then the solved slot) at the
//! army MoverComponent's `speed_max` 32 / `acceleration` 10.
//!
//! The camera is the world camera in metagame mode (700E30 / 6FEA20 with
//! 40CEE0): yaw `CameraMetaGameYaw` 0, pitch `CameraPitch` 40, height 100,
//! FOV 15, zoom `CameraZoomMetaGameMin..Max` 0.5..1.2 (default 1.2); it
//! scrolls and zooms like the battle camera and looks at the capital of the
//! kingdom whose turn starts (CenterOnCapital 933680 -> 6FF200).
//!
//! Not built: the territory and arrow VFX of the selected army (934D50, the
//! campaign's `territory_friendly_vfx` / `territory_enemy_vfx`), the VFX
//! object names, the focus and selection tints of the doodads' View, and the
//! army's move animation. Ownership is shown by the armies, the object display
//! and the politics display (not built), as nothing in the original recolours a
//! territory's model by its owner (its nodes are only hidden or shown, 93AE60,
//! by the debug commands `hide` / `show`).
use super::Game;
use crate::look::{self, OriginalMaterial};
use crate::textures::{self, Kind};
use aa_assets::{AssetStore, PreparedModel};
use aa_metagame::state::ArmyId;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

/// MetaGameArmy MoverComponent (mg_army.tgi).
const ARMY_SPEED_MAX: f32 = 32.0;
const ARMY_ACCELERATION: f32 = 10.0;
/// 955A40 / 955B70: territory and army z.
const MAP_Z: f32 = 2.0;
/// CameraZoomMetaGame* (401BD0 registrations: 0.5, 1.2, 1.2).
const ZOOM_MIN: f32 = 0.5;
const ZOOM_MAX: f32 = 1.2;
const ZOOM_DEFAULT: f32 = 1.2;

#[derive(Component)]
pub struct MapEntity;

#[derive(Component)]
pub struct MapCamera;

struct LoadedSet {
    models: HashMap<String, PreparedModel>,
    images: Vec<((String, Kind), Image)>,
    failures: Vec<String>,
}

struct TerritoryView {
    model: Option<String>,
}

struct ArmyView {
    entity: Entity,
    definition: usize,
    owner: u32,
    /// Native position and facing yaw (render).
    pos: Vec3,
    path: Vec<Vec3>,
    speed: f32,
    territory: Option<usize>,
}

#[derive(Resource, Default)]
pub struct MapScene {
    game: Option<u64>,
    loading: Option<Arc<Mutex<Option<LoadedSet>>>>,
    ready: bool,
    models: HashMap<String, PreparedModel>,
    textures: HashMap<(String, Kind), Handle<Image>>,
    territories: Vec<TerritoryView>,
    armies: BTreeMap<u32, ArmyView>,
    pub target: Vec2,
    pub zoom: f32,
    ambient_before: Option<(Color, f32)>,
}

fn to_render(p: Vec3) -> Vec3 {
    Vec3::new(p.x, p.z, -p.y)
}

fn store() -> Option<Arc<AssetStore>> {
    let root = crate::ui::reconstruction_root()?;
    crate::preload::store(&root).ok()
}

/// Every model the campaign's map needs: props, territory doodads, armies.
fn wanted(game: &Game) -> Vec<String> {
    let d = &game.data;
    let mut out: Vec<String> = Vec::new();
    let c = &d.meta.campaigns[game.campaign.state.campaign];
    if let Some(t) = d.view.tables.iter().find(|t| t.ids.eq_ignore_ascii_case(&c.map_table_ids)) {
        out.extend(t.props.iter().filter_map(|p| d.model(&p.object_ids).map(str::to_owned)));
    }
    if let Some(m) = c.map.and_then(|m| d.meta.maps.get(m)) {
        out.extend(m.territories.iter().filter_map(|t| d.model(&t.object_ids).map(str::to_owned)));
    }
    out.extend(d.meta.armies.iter().filter_map(|a| d.model(&a.ids).map(str::to_owned)));
    out.sort_by_key(|s| s.to_ascii_lowercase());
    out.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    out
}

fn team(color: [f32; 3]) -> [u8; 3] {
    color.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn start_loading(game: &Game) -> Arc<Mutex<Option<LoadedSet>>> {
    let cell = Arc::new(Mutex::new(None));
    let out = cell.clone();
    let paths = wanted(game);
    let colors: Vec<[u8; 3]> = game.data.view.colors.values().map(|c| team(*c)).collect();
    rayon::spawn(move || {
        let Some(store) = store() else { return };
        let mut models = HashMap::new();
        let mut failures = Vec::new();
        let mut requests = textures::Requests::default();
        for p in paths {
            match store.load_model(&p).and_then(|m| m.prepare()) {
                Ok(m) => {
                    for mesh in &m.meshes {
                        if let Some(t) = &mesh.material.texture {
                            if mesh.material.team_color_mask {
                                for c in &colors {
                                    requests.add(t, Kind::Team(*c));
                                }
                            } else {
                                requests.add(t, Kind::Color);
                            }
                        }
                    }
                    models.insert(p.to_ascii_lowercase(), m);
                }
                Err(e) => failures.push(format!("{p}: {e}")),
            }
        }
        let (images, mut f) = textures::decode(&store, requests);
        failures.append(&mut f);
        if let Ok(mut g) = out.lock() {
            *g = Some(LoadedSet { models, images, failures });
        }
    });
    cell
}

fn despawn_all(commands: &mut Commands, scene: &mut MapScene, query: &Query<Entity, With<MapEntity>>, ambient: &mut GlobalAmbientLight) {
    for e in query {
        commands.entity(e).despawn();
    }
    if let Some((c, b)) = scene.ambient_before.take() {
        ambient.color = c;
        ambient.brightness = b;
    }
    *scene = MapScene::default();
}

/// Spawn the meshes of a model under a parent.
fn spawn_model(
    commands: &mut Commands,
    parent: Entity,
    model: &PreparedModel,
    textures: &HashMap<(String, Kind), Handle<Image>>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<OriginalMaterial>,
    team_color: [u8; 3],
) {
    for m in model.meshes.iter().filter(|m| !m.indices.is_empty()) {
        let kind = crate::scene::kind(&m.material, team_color);
        let texture = m.material.texture.as_ref().and_then(|t| textures.get(&(t.clone(), kind)).cloned());
        let handle = materials.add(crate::scene::standard(&m.material, texture));
        let child = commands.spawn((Mesh3d(meshes.add(crate::scene::model_mesh(m))), MeshMaterial3d(handle), Transform::default())).id();
        commands.entity(parent).add_child(child);
    }
}

/// The kingdom's colour (Game/kingdom_colors.tgi), the team colour of its armies.
fn kingdom_color(game: &Game, k: usize) -> [f32; 3] {
    let c = &game.campaign.data.campaigns[game.campaign.state.campaign];
    let kd = game.campaign.state.kingdoms[k].data;
    game.data.color(&c.kingdoms[kd].color_ids)
}

/// The display position of an army at rest (959420 on its slot, z 2.0).
pub(super) fn rest_position(game: &Game, army: ArmyId) -> Option<Vec3> {
    let a = game.campaign.state.armies.get(army.0 as usize)?.as_ref()?;
    let t = a.territory?;
    let slot = aa_metagame::positions::army_slot(&game.campaign.state, t, army);
    let z = |_: usize| Some(MAP_Z.to_bits());
    let p = aa_metagame::positions::solve(&game.campaign.state, &game.campaign.data, t, slot, &z, aa_metagame::GAME_PRECISION).ok()?;
    Some(Vec3::new(f32::from_bits(p[0]), f32::from_bits(p[1]), MAP_Z))
}

/// Army facing (955B70: 0.7071, -0.7071) as a render yaw (placement basis:
/// models face native -Y at identity, yaw = atan2(x, -y)).
fn facing_yaw(dir: Vec2) -> f32 {
    dir.x.atan2(-dir.y)
}

#[allow(clippy::too_many_arguments)]
pub fn sync(
    mut commands: Commands,
    mut scene: ResMut<MapScene>,
    mut front: ResMut<crate::ui::front::Front>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OriginalMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    entities: Query<Entity, With<MapEntity>>,
    mut transforms: Query<&mut Transform>,
) {
    let front = &mut *front;
    let showing = front.page == crate::ui::front::Page::Campaign && front.campaign.screen == super::Screen::Map;
    let Some(game) = front.campaign.game.as_mut().filter(|_| showing) else {
        if scene.game.is_some() {
            despawn_all(&mut commands, &mut scene, &entities, &mut ambient);
        }
        return;
    };
    if scene.game != Some(game.id) {
        if scene.game.is_some() {
            despawn_all(&mut commands, &mut scene, &entities, &mut ambient);
        }
        scene.game = Some(game.id);
        scene.loading = Some(start_loading(game));
        scene.zoom = ZOOM_DEFAULT;
        // 700E30: the camera starts at the map centre
        let c = &game.campaign.data.campaigns[game.campaign.state.campaign];
        if let Some(m) = c.map.and_then(|m| game.campaign.data.maps.get(m)) {
            scene.target = Vec2::new(f32::from_bits(m.width_bits) * 0.5, f32::from_bits(m.height_bits) * 0.5);
        }
        // the world's sun (SVars, 4EE1F0) as the battle lights it (43A400)
        let sun = look::vars().sun;
        let scaled = |c: [f32; 3]| Color::linear_rgb(c[0] * sun.dimmer, c[1] * sun.dimmer, c[2] * sun.dimmer);
        scene.ambient_before = Some((ambient.color, ambient.brightness));
        ambient.color = scaled(sun.ambient);
        ambient.brightness = 1.0;
        commands.spawn((
            DirectionalLight { illuminance: 1.0, color: scaled(sun.diffuse), shadow_maps_enabled: false, ..default() },
            Transform::default().looking_to(-sun.to_light(), Vec3::Y),
            MapEntity,
        ));
        commands.spawn((
            Camera3d::default(),
            Camera { order: 0, ..default() },
            bevy::core_pipeline::tonemapping::Tonemapping::None,
            Projection::custom(crate::camera::NativeProjection::new()),
            Transform::default(),
            MapCamera,
            MapEntity,
        ));
    }
    // the background load
    if !scene.ready {
        let done = scene.loading.as_ref().and_then(|c| c.lock().ok().and_then(|mut g| g.take()));
        let Some(set) = done else { return };
        for f in set.failures.iter().take(10) {
            warn!("campaign map: {f}");
        }
        scene.textures = textures::upload(set.images, Vec::new(), &mut images).handles;
        scene.models = set.models;
        scene.ready = true;
        scene.loading = None;
        // the map table props
        let c = &game.campaign.data.campaigns[game.campaign.state.campaign];
        if let Some(t) = game.data.view.tables.iter().find(|t| t.ids.eq_ignore_ascii_case(&c.map_table_ids)) {
            for p in &t.props {
                let Some(model) = game.data.model(&p.object_ids).and_then(|m| scene.models.get(&m.to_ascii_lowercase())) else { continue };
                let e = commands.spawn((Transform::from_translation(to_render(Vec3::new(p.location[0], p.location[1], MAP_Z))), Visibility::default(), MapEntity)).id();
                spawn_model(&mut commands, e, model, &scene.textures, &mut meshes, &mut materials, [128; 3]);
            }
        }
        // territory doodads (955A40)
        let map = c.map.unwrap_or(0);
        for t in &game.campaign.state.territories {
            let def = &game.campaign.data.maps[map].territories[t.definition];
            let at = Vec3::new(f32::from_bits(def.location[0]), f32::from_bits(def.location[1]), MAP_Z);
            let e = commands.spawn((Transform::from_translation(to_render(at)), Visibility::default(), MapEntity)).id();
            let model_path = game.data.model(&def.object_ids).map(|m| m.to_ascii_lowercase());
            if let Some(model) = model_path.as_ref().and_then(|p| scene.models.get(p)) {
                spawn_model(&mut commands, e, model, &scene.textures, &mut meshes, &mut materials, [128; 3]);
            }
            scene.territories.push(TerritoryView { model: model_path });
        }
    }
    // armies: spawn, respawn on type or owner change, despawn the dead
    let live: Vec<(u32, usize, u32)> = game
        .campaign
        .state
        .armies
        .iter()
        .enumerate()
        .filter_map(|(i, a)| a.as_ref().map(|a| (i as u32, a.definition, a.owner.0)))
        .collect();
    let gone: Vec<u32> = scene.armies.keys().copied().filter(|k| !live.iter().any(|(i, _, _)| i == k)).collect();
    for k in gone {
        if let Some(v) = scene.armies.remove(&k) {
            commands.entity(v.entity).despawn();
        }
    }
    for (i, definition, owner) in live {
        let id = ArmyId(i);
        let changed = scene.armies.get(&i).is_some_and(|v| v.definition != definition || v.owner != owner);
        if changed {
            if let Some(v) = scene.armies.remove(&i) {
                commands.entity(v.entity).despawn();
            }
        }
        if !scene.armies.contains_key(&i) {
            let Some(pos) = rest_position(game, id) else { continue };
            let def = &game.campaign.data.armies[definition];
            let kd = game.campaign.kingdom_of_actor(aa_metagame::battle::KingdomActor(owner));
            let tc = kd.map_or([255; 3], |k| team(kingdom_color(game, k)));
            let yaw = facing_yaw(Vec2::new(f32::from_bits(aa_metagame::positions::PLACEMENT_FACING_BITS[0]), f32::from_bits(aa_metagame::positions::PLACEMENT_FACING_BITS[1])));
            let e = commands
                .spawn((Transform::from_translation(to_render(pos)).with_rotation(Quat::from_rotation_y(yaw)), Visibility::default(), MapEntity))
                .id();
            if let Some(model) = game.data.model(&def.ids).and_then(|m| scene.models.get(&m.to_ascii_lowercase())) {
                let model = model.clone();
                spawn_model(&mut commands, e, &model, &scene.textures, &mut meshes, &mut materials, tc);
            }
            let territory = game.campaign.state.army(id).territory;
            scene.armies.insert(i, ArmyView { entity: e, definition, owner, pos, path: Vec::new(), speed: 0.0, territory });
        }
    }
    // move visuals (962880): the link's intermediate points, then the slot
    for (army, from, to) in std::mem::take(&mut game.visual_moves) {
        let Some(v) = scene.armies.get_mut(&army.0) else {
            game.moving.remove(&army);
            continue;
        };
        let map = game.campaign.data.campaigns[game.campaign.state.campaign].map.unwrap_or(0);
        let defs = &game.campaign.data.maps[map].territories;
        let mut path: Vec<Vec3> = Vec::new();
        if let Some(f) = from {
            let fd = game.campaign.state.territories[f].definition;
            let td = game.campaign.state.territories[to].definition;
            if let Some(link) = defs[fd].links.iter().find(|l| l.target == td) {
                path.extend(link.intermediate_points.iter().map(|p| Vec3::new(f32::from_bits(p[0]), f32::from_bits(p[1]), MAP_Z)));
            }
        }
        if let Some(end) = rest_position(game, army) {
            path.push(end);
        }
        v.path = path;
        v.territory = Some(to);
    }
    // the movers
    let dt = time.delta_secs();
    for (&i, v) in scene.armies.iter_mut() {
        let id = ArmyId(i);
        if v.path.is_empty() {
            game.moving.remove(&id);
            // at rest on its slot (captures, realigns, respawns)
            if let Some(p) = rest_position(game, id)
                && (p - v.pos).length() > 0.01
                && game.campaign.state.army(id).waypoints.is_empty()
            {
                v.pos = p;
                if let Ok(mut t) = transforms.get_mut(v.entity) {
                    t.translation = to_render(p);
                }
            }
            continue;
        }
        game.moving.insert(id);
        v.speed = (v.speed + ARMY_ACCELERATION * dt).min(ARMY_SPEED_MAX);
        let mut step = v.speed * dt;
        let mut dir = Vec2::ZERO;
        while step > 0.0 && !v.path.is_empty() {
            let next = v.path[0];
            let d = next - v.pos;
            let len = d.truncate().length();
            if len <= step {
                v.pos = next;
                step -= len;
                v.path.remove(0);
            } else {
                dir = d.truncate() / len;
                v.pos += Vec3::new(dir.x, dir.y, 0.0) * step;
                step = 0.0;
            }
        }
        if v.path.is_empty() {
            v.speed = 0.0;
        }
        if let Ok(mut t) = transforms.get_mut(v.entity) {
            t.translation = to_render(v.pos);
            if dir != Vec2::ZERO {
                t.rotation = Quat::from_rotation_y(facing_yaw(dir));
            }
        }
    }
}

/// The metagame camera (see the module doc).
pub fn camera(
    mut scene: ResMut<MapScene>,
    mut front: ResMut<crate::ui::front::Front>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut cams: Query<&mut Transform, With<MapCamera>>,
) {
    if scene.game.is_none() {
        return;
    }
    let front = &mut *front;
    if let Some(p) = front.campaign.iface.look_at.take() {
        scene.target = Vec2::new(p[0], p[1]);
    }
    let speed = front.ui.uvars.get_f32("CameraScrollSpeed");
    let scale_pan = front.ui.uvars.get_f32("CameraScalePan");
    let dt = time.delta_secs().clamp(0.01, 0.1);
    let mut s = Vec2::ZERO;
    let dialog = front.ui.message_box.is_some();
    if !dialog {
        if keys.pressed(KeyCode::ArrowLeft) {
            s.x -= 1.0;
        }
        if keys.pressed(KeyCode::ArrowRight) {
            s.x += 1.0;
        }
        if keys.pressed(KeyCode::ArrowUp) {
            s.y += 1.0;
        }
        if keys.pressed(KeyCode::ArrowDown) {
            s.y -= 1.0;
        }
        if let (Ok(w), Some(c)) = (windows.single(), front.pointer.pos) {
            let (ww, wh) = (w.physical_width() as f32, w.physical_height() as f32);
            if c.x <= 0.0 {
                s.x = -1.0;
            }
            if c.x >= ww - 1.0 {
                s.x = 1.0;
            }
            if c.y <= 0.0 {
                s.y = 1.0;
            }
            if c.y >= wh - 1.0 {
                s.y = -1.0;
            }
        }
    }
    // yaw 0: screen right = native +x, screen up = native +y
    scene.target += s * scale_pan * speed * dt;
    if let Some(g) = &front.campaign.game {
        let c = &g.campaign.data.campaigns[g.campaign.state.campaign];
        if let Some(m) = c.map.and_then(|m| g.campaign.data.maps.get(m)) {
            let (w, h) = (f32::from_bits(m.width_bits), f32::from_bits(m.height_bits));
            scene.target = scene.target.clamp(Vec2::ZERO, Vec2::new(w, h));
        }
    }
    // the wheel over the world (one notch = 0.1)
    let over_world = super::map_pointer_over_world(&*front);
    if front.pointer.wheel != 0.0 && over_world && !dialog {
        scene.zoom = (scene.zoom - front.pointer.wheel * 0.1).clamp(ZOOM_MIN, ZOOM_MAX);
    }
    let pitch = crate::camera::PITCH_DEG.to_radians();
    // CameraMetaGameYaw 0: d = (sin 0 cos p, cos 0 cos p, -sin p)
    let d = Vec3::new(0.0, pitch.cos(), -pitch.sin());
    let target = Vec3::new(scene.target.x, scene.target.y, MAP_Z);
    let eye = target - d * (scene.zoom * crate::camera::HEIGHT / pitch.sin());
    for mut t in &mut cams {
        *t = Transform::from_translation(to_render(eye)).looking_at(to_render(target), Vec3::Y);
    }
}

/// World clicks. What is under the pointer stands for the world's hover
/// (4363B0): an army (within 1.0 of its position, an engine choice), else the
/// territory whose topography contains the ground point.
/// - the hot mode: a left click gives the waiting command the hovered
///   territory (956E40) and ends the mode whatever is hovered; a right click
///   only ends it (5CD28E);
/// - otherwise a left click selects what is hovered, a right click on a
///   territory runs 956C20.
pub fn pick(
    scene: Res<MapScene>,
    mut front: ResMut<crate::ui::front::Front>,
    cams: Query<(&Camera, &GlobalTransform), With<MapCamera>>,
) {
    if !scene.ready {
        return;
    }
    let front = &mut *front;
    let (left, right) = (front.pointer.left_pressed, front.pointer.right_pressed);
    // 957540 / 5CD190: only while the world is the input target ([B3653C]+60)
    if !(left || right) || !super::map_pointer_over_world(&*front) || front.ui.message_box.is_some() || front.campaign.iface.dialog.is_some() {
        return;
    }
    let Some(pos) = front.pointer.pos else { return };
    let Ok((camera, tf)) = cams.single() else { return };
    let Ok(ray) = camera.viewport_to_world(tf, pos) else { return };
    // the ground plane at the map height (render y = 2.0)
    if ray.direction.y.abs() < 1e-5 {
        return;
    }
    let t = (MAP_Z - ray.origin.y) / ray.direction.y;
    if t <= 0.0 {
        return;
    }
    let hit = ray.origin + *ray.direction * t;
    let ground = Vec2::new(hit.x, -hit.z);
    let ui = &mut front.ui;
    let super::CampaignUi { iface, game, .. } = &mut front.campaign;
    let Some(game) = game.as_mut() else { return };
    let army = scene
        .armies
        .iter()
        .map(|(&i, v)| (i, (v.pos.truncate() - ground).length()))
        .filter(|(_, d)| *d < 1.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| ArmyId(i));
    // the territory under the point: its topography triangles
    let map = game.campaign.data.campaigns[game.campaign.state.campaign].map.unwrap_or(0);
    let mut territory = None;
    if army.is_none() {
        for (ti, view) in scene.territories.iter().enumerate() {
            let Some(model) = view.model.as_ref().and_then(|m| scene.models.get(m)) else { continue };
            let def = &game.campaign.data.maps[map].territories[game.campaign.state.territories[ti].definition];
            let local = ground - Vec2::new(f32::from_bits(def.location[0]), f32::from_bits(def.location[1]));
            // model space is render space: (x, -y) on the ground
            let p = Vec2::new(local.x, -local.y);
            let inside = model.meshes.iter().filter(|m| m.hierarchy.iter().any(|h| h.eq_ignore_ascii_case("Topography"))).any(|m| {
                m.indices.chunks_exact(3).any(|tri| {
                    let v = |i: u32| {
                        let q = m.vertices[i as usize].position;
                        Vec2::new(q[0], q[2])
                    };
                    point_in_triangle(p, v(tri[0]), v(tri[1]), v(tri[2]))
                })
            });
            if inside {
                territory = Some(ti);
                break;
            }
        }
    }
    if let Some(cmd) = game.targeting.take() {
        if let (true, Some(t)) = (left, territory) {
            iface.hot_territory(ui, game, cmd, t);
        }
        return;
    }
    match (left, army, territory) {
        (true, Some(a), _) => {
            game.selected_army = Some(a);
            game.selected_territory = None;
        }
        (true, None, Some(t)) => {
            game.selected_army = None;
            game.selected_territory = Some(t);
        }
        (false, _, Some(t)) => iface.right_click(ui, game, t),
        _ => {}
    }
}

fn point_in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let s = |p1: Vec2, p2: Vec2, p3: Vec2| (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y);
    let d1 = s(p, a, b);
    let d2 = s(p, b, c);
    let d3 = s(p, c, a);
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}
