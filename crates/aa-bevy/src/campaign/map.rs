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
//! A territory's meshes carry the RepColor tag (the audited model corrections,
//! `model_fixes`), so its doodad shows its owner's colour where the tile's alpha
//! leaves room: the coloured borders. The doodad is an actor of the owner
//! (45AFC0: the kingdom colour of the View, the thing's default, white, without
//! an owner); a captured territory changes it.
//!
//! The arrows of the selected army (934D50, with UVar MetaGameViewShowVFXArrows,
//! for an army of the kingdom whose turn it is and the local player): to every
//! territory of its move range (940470, 934AC0), the campaign's
//! `territory_enemy_arrow_vfx` (Arrows/AttackArrow.nif) to a territory hostile to
//! the army's owner (932B90, 954B30), else `territory_friendly_arrow_vfx`
//! (Arrows/MoveArrow.nif): from the army's slot (959420) over the mid point at
//! `arrow_middle` height (446390) to the territory (9552A0) in `move_duration`,
//! then at rest. Compare against the original (labelled rendition): the VFX's
//! Extrusion (its wings' ribbon, one second at full alpha, one fading) is one
//! flat ribbon along the path, from the wing colour at `node_alpha`.
//!
//! The double click (957540) selects an army and centres on it. Dragging an army
//! (957130) and letting go over a territory runs the right click's command. Compare
//! against the original (labelled rendition): the drag's ghost model at the pointer
//! (956F40 -> 693AE0) and its cursor states (9571D0) are not drawn.
//!
//! Not built: the VFX object names, the focus and selection tints of the doodads'
//! View (`territory_focus_vfx`, `territory_select_vfx`, the 932D80 highlights of
//! an alias territory's Thing), and the army's move animation.
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
    /// The doodad's entity once spawned, with the team colour it was spawned in.
    entity: Option<Entity>,
    team: [u8; 3],
}

/// A model waiting for its spawn: where, which model, and the territory it is
/// the doodad of.
struct ToSpawn {
    at: Vec3,
    path: String,
    territory: Option<usize>,
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

/// scene.rs: RenderAssetBytesPerFrame.
const UPLOAD_PER_FRAME: usize = 32 << 20;

#[derive(Resource, Default)]
pub struct MapScene {
    game: Option<u64>,
    loading: Option<Arc<Mutex<Option<LoadedSet>>>>,
    ready: bool,
    models: HashMap<String, PreparedModel>,
    textures: HashMap<(String, Kind), Handle<Image>>,
    territories: Vec<TerritoryView>,
    /// Frames left until the textures are on the GPU (RenderAssetBytesPerFrame
    /// spreads them out; a mesh spawned with a material whose texture is not
    /// ready can stay undrawn), then the models to spawn a slice per frame.
    spawn_wait: u32,
    to_spawn: std::collections::VecDeque<ToSpawn>,
    armies: BTreeMap<u32, ArmyView>,
    pub target: Vec2,
    pub zoom: f32,
    ambient_before: Option<(Color, f32)>,
    /// The minimap: the map's texture and the image made of it, with what it shows.
    minimap_source: Option<Vec<u8>>,
    minimap_image: Option<Handle<Image>>,
    minimap_shown: Vec<(Option<usize>, Option<[u32; 2]>)>,
    /// What the arrows are drawn for (934D50 on a change of selection): the army
    /// and the territory it stands on, with the arrows themselves.
    arrows_for: Option<(u32, Option<usize>, usize)>,
    arrows: Vec<MoveArrow>,
    ribbon_material: Option<Handle<StandardMaterial>>,
    /// A left press on an army (957130): where, which; and whether it became a drag.
    press: Option<(Vec2, ArmyId)>,
    dragging: bool,
    /// The last left click on an army, for the double click (957540).
    last_click: Option<(f32, ArmyId)>,
}

/// One arrow VFX of the selected army.
struct MoveArrow {
    head: Entity,
    ribbon: Entity,
    /// Render space: start, mid point, end.
    points: [Vec3; 3],
    start: f32,
    enemy: bool,
}

/// `move_duration`, the segment times and the wing colours of mg_vfx.tgi's arrows.
const ARROW_SECONDS: f32 = 0.5;
const ARROW_FULL: f32 = 1.0;
const ARROW_FADE: f32 = 1.0;
const ARROW_ALPHA: f32 = 0.5;
/// `arrow_middle` z.
const ARROW_HEIGHT: f32 = 5.0;
/// The ribbon's half width (labelled rendition: the distance of the head's wings).
const ARROW_HALF_WIDTH: f32 = 0.35;
const ARROW_MOVE_MODEL: &str = "MetaGame/Arrows/MoveArrow.nif";
const ARROW_ATTACK_MODEL: &str = "MetaGame/Arrows/AttackArrow.nif";

/// Windows' GetDoubleClickTime, as the front end's.
const DOUBLE_CLICK_SECONDS: f32 = 0.5;

/// The point on the path through the three points at `t` (0..=1).
fn arrow_point(p: &[Vec3; 3], t: f32) -> Vec3 {
    let u = 1.0 - t;
    // the quadratic that passes the mid point at one half
    let control = p[1] * 2.0 - (p[0] + p[2]) * 0.5;
    p[0] * (u * u) + control * (2.0 * u * t) + p[2] * (t * t)
}

/// 940470 (without aliases added): the territories of the army's move range,
/// breadth first from the first territory of its +24 list while the depth is within
/// the army type's moves (+5B8 +4); the start is the first of the list.
fn move_range(game: &Game, army: ArmyId) -> Vec<usize> {
    let state = &game.campaign.state;
    let a = state.army(army);
    let Some(start) = a.path.first().copied().or(a.territory) else { return Vec::new() };
    let limit = game.campaign.data.armies[a.definition].movement;
    let map = game.campaign.data.campaigns[state.campaign].map.unwrap_or(0);
    let defs = &game.campaign.data.maps[map].territories;
    let of_def = |d: usize| state.territories.iter().position(|t| t.definition == d);
    let mut seen = vec![start];
    let mut level = vec![start];
    let mut depth = 0;
    let mut out = vec![start];
    while depth < limit && !level.is_empty() {
        depth += 1;
        let mut next = Vec::new();
        for t in level {
            for l in &defs[state.territories[t].definition].links {
                if let Some(n) = of_def(l.target)
                    && !seen.contains(&n)
                {
                    seen.push(n);
                    next.push(n);
                    out.push(n);
                }
            }
        }
        level = next;
    }
    out
}

/// 934D50: the arrows the selected army should have: for each the target
/// territory and whether it is hostile to the army's owner.
fn wanted_arrows(game: &Game, show: bool) -> Vec<(usize, bool)> {
    let Some(army) = game.selected_army.filter(|_| show) else { return Vec::new() };
    let state = &game.campaign.state;
    let a = state.army(army);
    let current = state.current.and_then(|k| game.campaign.actor(k));
    if current != Some(a.owner) || !game.campaign.local_turn() {
        return Vec::new();
    }
    let map = game.campaign.data.campaigns[state.campaign].map.unwrap_or(0);
    let defs = &game.campaign.data.maps[map].territories;
    let start = a.path.first().copied().or(a.territory);
    let partner = start.and_then(|s| defs[state.territories[s].definition].alias_partner);
    move_range(game, army)
        .into_iter()
        .filter(|&t| Some(t) != start && Some(state.territories[t].definition) != partner)
        // 954B30: unowned, or the owner regards the army's owner as an enemy
        .map(|t| (t, state.owner_actor(t).is_none_or(|o| aa_metagame::battle::KingdomRelations::enemy(&game.campaign.relations, o, a.owner))))
        .collect()
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
    out.push(ARROW_MOVE_MODEL.to_string());
    out.push(ARROW_ATTACK_MODEL.to_string());
    out.sort_by_key(|s| s.to_ascii_lowercase());
    out.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    out
}

fn team(color: [f32; 3]) -> [u8; 3] {
    color.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// 45AFC0 without an owner: the thing's default View colour.
const NO_OWNER_TEAM: [u8; 3] = [255; 3];

fn start_loading(game: &Game) -> Arc<Mutex<Option<LoadedSet>>> {
    let cell = Arc::new(Mutex::new(None));
    let out = cell.clone();
    let paths = wanted(game);
    // the campaign's kingdoms: whose armies and territories there can be
    let c = &game.campaign.data.campaigns[game.campaign.state.campaign];
    let colors: Vec<[u8; 3]> = c.kingdoms.iter().map(|k| team(game.data.color(&k.color_ids))).collect();
    let fixes = crate::ui::reconstruction_root().and_then(|r| crate::preload::fixes(&r).ok());
    rayon::spawn(move || {
        let Some(store) = store() else { return };
        let mut models = HashMap::new();
        let mut failures = Vec::new();
        let mut requests = textures::Requests::default();
        for p in paths {
            match store.load_model(&p).and_then(|m| m.prepare()) {
                Ok(mut m) => {
                    // the audited RepColor tags and render states, as the battle's models
                    if let Some(f) = &fixes
                        && let Err(e) = f.apply(&store, &mut m)
                    {
                        failures.push(format!("model fixes {p}: {e:#}"));
                    }
                    for mesh in &m.meshes {
                        if let Some(t) = &mesh.material.texture {
                            if mesh.material.team_color_mask {
                                for c in &colors {
                                    requests.add(t, Kind::Team(*c));
                                }
                                requests.add(t, Kind::Team(NO_OWNER_TEAM));
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

/// The arrow's ribbon: flat quads along the path up to `t`, in the vertex colour.
fn ribbon_mesh(points: &[Vec3; 3], t: f32, color: [f32; 4]) -> Mesh {
    const SEGMENTS: usize = 20;
    let d = points[2] - points[0];
    let side = Vec3::new(-d.z, 0.0, d.x).normalize_or_zero() * ARROW_HALF_WIDTH;
    let mut positions = Vec::with_capacity((SEGMENTS + 1) * 2);
    let mut indices = Vec::with_capacity(SEGMENTS * 6);
    for i in 0..=SEGMENTS {
        let p = arrow_point(points, t * i as f32 / SEGMENTS as f32);
        positions.push((p - side).to_array());
        positions.push((p + side).to_array());
    }
    for i in 0..SEGMENTS as u32 {
        let a = i * 2;
        indices.extend_from_slice(&[a, a + 1, a + 2, a + 1, a + 3, a + 2]);
    }
    let n = positions.len();
    let mut mesh = Mesh::new(bevy::mesh::PrimitiveTopology::TriangleList, bevy::asset::RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; n]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![color; n]);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    mesh
}

/// The unlit, blended material of the ribbons. `floating_ui_vfx` of the arrow's data:
/// over the map; the blended territories would otherwise be drawn after it (the blended
/// phase sorts by the entity's origin, and a ribbon's is the map's corner).
fn materials_ribbon(materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial { base_color: Color::WHITE, unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, depth_bias: 1.0e6, ..default() })
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
        if m.material.texture.is_some() && texture.is_none() {
            warn!("campaign map: no texture {:?} {kind:?}", m.material.texture);
        }
        let handle = materials.add(crate::scene::standard(&m.material, texture));
        let child = commands.spawn((Mesh3d(meshes.add(crate::scene::model_mesh(m))), MeshMaterial3d(handle), Transform::default())).id();
        commands.entity(parent).add_child(child);
    }
}

/// `minimap_size` of the map's texture (Minimap.tga).
const MINIMAP_SIZE: (usize, usize) = (256, 128);

/// The strategic minimap: the widget's quad (5E8F90) over the map's
/// `minimap_texture`, whose alpha names each territory (`minimap_alpha`), and the
/// camera's view. Compare against the original (labelled rendition, the native
/// compositor is not found yet): a territory is its owner's colour mixed with the
/// texture by `minimap_blend_amount`, an army a square by its relation to the local
/// kingdom (0x454DC0's colours), as original captures show them.
pub fn minimap(
    mut scene: ResMut<MapScene>,
    front: Res<crate::ui::front::Front>,
    mut draw: ResMut<crate::ui::render::UiDraw>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<MapCamera>>,
) {
    let Some(game) = front.campaign.game.as_ref().filter(|_| front.page == crate::ui::front::Page::Campaign && front.campaign.screen == super::Screen::Map) else {
        return;
    };
    let rect = draw.list.commands.iter().find_map(|c| match &c.kind {
        aa_ui::draw::DrawKind::External { ve_type, rect, .. } if ve_type == "MiniMap" => Some(*rect),
        _ => None,
    });
    let Some(rect) = rect else {
        draw.minimap = None;
        return;
    };
    let c = &game.campaign.data.campaigns[game.campaign.state.campaign];
    let Some(map) = c.map.and_then(|m| game.campaign.data.maps.get(m)) else { return };
    if scene.minimap_source.is_none() {
        let found = front.ui.store.root.find(map.minimap_texture.trim_start_matches('/')).and_then(|p| image::open(&p).ok());
        scene.minimap_source = Some(found.map(|i| i.to_rgba8().into_raw()).unwrap_or_default());
    }
    let (mw, mh) = (f32::from_bits(map.width_bits), f32::from_bits(map.height_bits));
    // what the image shows: every territory's owner, every army's place
    let mut shown: Vec<(Option<usize>, Option<[u32; 2]>)> = game.campaign.state.territories.iter().map(|t| (t.owner, None)).collect();
    let local = game.campaign.local_actor();
    let mut squares: Vec<(Vec2, [u8; 4])> = Vec::new();
    for (i, a) in game.campaign.state.armies.iter().enumerate() {
        let Some(a) = a else { continue };
        let Some(p) = rest_position(game, ArmyId(i as u32)) else { continue };
        shown.push((Some(a.owner.0 as usize), Some([p.x.to_bits(), p.y.to_bits()])));
        use aa_metagame::battle::KingdomRelations;
        let color = match local {
            Some(l) if l == a.owner => [0, 255, 0, 255],
            Some(l) if game.campaign.relations.enemy(a.owner, l) => [255, 0, 0, 255],
            _ => [0, 0, 255, 255],
        };
        squares.push((Vec2::new(p.x, p.y), color));
    }
    if scene.minimap_image.is_none() || scene.minimap_shown != shown {
        let source = scene.minimap_source.clone().unwrap_or_default();
        let (w, h) = MINIMAP_SIZE;
        if source.len() == w * h * 4 {
            let blend = f32::from_bits(map.minimap_blend_amount_bits);
            let mut owner_of = [None::<[u8; 3]>; 256];
            for t in &game.campaign.state.territories {
                let alpha = map.territories[t.definition].minimap_alpha;
                if let (Ok(a), Some(k)) = (usize::try_from(alpha), t.owner)
                    && a < 256
                {
                    owner_of[a] = Some(team(kingdom_color(game, k)));
                }
            }
            let mut pixels = source;
            for px in pixels.chunks_exact_mut(4) {
                if let Some(o) = owner_of[px[3] as usize] {
                    for i in 0..3 {
                        px[i] = (f32::from(px[i]) * blend + f32::from(o[i]) * (1.0 - blend)) as u8;
                    }
                }
                px[3] = 255;
            }
            for (at, color) in &squares {
                let (x, y) = ((at.x / mw * w as f32) as i32, ((1.0 - at.y / mh) * h as f32) as i32);
                for dy in -2..=2i32 {
                    for dx in -2..=2i32 {
                        let (px, py) = (x + dx, y + dy);
                        if px >= 0 && py >= 0 && (px as usize) < w && (py as usize) < h {
                            let i = (py as usize * w + px as usize) * 4;
                            let edge = dx.abs() == 2 || dy.abs() == 2;
                            pixels[i..i + 4].copy_from_slice(if edge { &[0, 0, 0, 255] } else { color });
                        }
                    }
                }
            }
            let image = crate::ui::tex::make_image(w as u32, h as u32, pixels, crate::ui::tex::Variant::default(), false);
            scene.minimap_image = Some(images.add(image));
        }
        scene.minimap_shown = shown;
    }
    let widget = front.ui.widget("Root/MetaGame/MetaGameMiniMap");
    let m = match widget.and_then(|w| w.control.as_deref()) {
        Some(aa_ui::controls::Control::MiniMap(m)) => m.clone(),
        _ => aa_ui::minimap::MiniMap::default(),
    };
    let quad = aa_ui::minimap::quad(&m, rect);
    let [p0, p1, p2] = quad;
    let pt = |u: f32, v: f32| (p0.0 + u * (p2.0 - p1.0) + v * (p1.0 - p0.0), p0.1 + u * (p2.1 - p1.1) + v * (p1.1 - p0.1));
    let mut lines = Vec::new();
    if let (Ok(window), Ok((camera, tf))) = (windows.single(), cameras.single()) {
        let (w, h) = (window.width(), window.height());
        let mut corners = Vec::new();
        for (x, y) in [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)] {
            if let Ok(r) = camera.viewport_to_world(tf, Vec2::new(x, y))
                && r.direction.y.abs() > 1e-4
            {
                let t = (MAP_Z - r.origin.y) / r.direction.y;
                let p = r.origin + *r.direction * t;
                corners.push(Vec2::new(p.x, -p.z));
            }
        }
        if corners.len() == 4 {
            for i in 0..4 {
                let (a, e) = (corners[i], corners[(i + 1) % 4]);
                // kept inside the widget, as the original's draws it
                let uv = |p: Vec2| ((p.x / mw).clamp(0.0, 1.0), (1.0 - p.y / mh).clamp(0.0, 1.0));
                let (au, av) = uv(a);
                let (eu, ev) = uv(e);
                lines.push((pt(au, av), pt(eu, ev), [1.0, 1.0, 1.0, 1.0]));
            }
        }
    }
    draw.minimap_image = scene.minimap_image.clone();
    draw.minimap = Some(crate::ui::minimap::MinimapDraw { quad, lines, has_terrain: scene.minimap_image.is_some() });
}

/// A territory doodad's team colour: its owner's (45AFC0).
fn territory_team(game: &Game, t: usize) -> [u8; 3] {
    game.campaign.state.territories.get(t).and_then(|x| x.owner).map_or(NO_OWNER_TEAM, |k| team(kingdom_color(game, k)))
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
    mut std_materials: ResMut<Assets<StandardMaterial>>,
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
        let bytes: usize = set.images.iter().map(|(_, i)| i.data.as_ref().map_or(0, Vec::len)).sum();
        scene.textures = textures::upload(set.images, Vec::new(), &mut images).handles;
        scene.models = set.models;
        scene.ready = true;
        scene.loading = None;
        scene.spawn_wait = (bytes / UPLOAD_PER_FRAME) as u32 + 4;
        // the map table props
        let c = &game.campaign.data.campaigns[game.campaign.state.campaign];
        if let Some(t) = game.data.view.tables.iter().find(|t| t.ids.eq_ignore_ascii_case(&c.map_table_ids)) {
            for p in &t.props {
                let Some(path) = game.data.model(&p.object_ids).map(str::to_ascii_lowercase) else { continue };
                scene.to_spawn.push_back(ToSpawn { at: to_render(Vec3::new(p.location[0], p.location[1], MAP_Z)), path, territory: None });
            }
        }
        // territory doodads (955A40)
        let map = c.map.unwrap_or(0);
        for (i, t) in game.campaign.state.territories.iter().enumerate() {
            let def = &game.campaign.data.maps[map].territories[t.definition];
            let at = Vec3::new(f32::from_bits(def.location[0]), f32::from_bits(def.location[1]), MAP_Z);
            let model_path = game.data.model(&def.object_ids).map(|m| m.to_ascii_lowercase());
            if let Some(path) = &model_path {
                scene.to_spawn.push_back(ToSpawn { at: to_render(at), path: path.clone(), territory: Some(i) });
            }
            scene.territories.push(TerritoryView { model: model_path, entity: None, team: NO_OWNER_TEAM });
        }
    }
    if scene.spawn_wait > 0 {
        scene.spawn_wait -= 1;
    } else {
        let mut budget = UPLOAD_PER_FRAME / 2;
        while budget > 0 {
            let Some(next) = scene.to_spawn.pop_front() else { break };
            let color = next.territory.map_or(NO_OWNER_TEAM, |t| territory_team(game, t));
            let MapScene { models, textures, .. } = &*scene;
            let Some(model) = models.get(&next.path) else { continue };
            let e = commands.spawn((Transform::from_translation(next.at), Visibility::default(), MapEntity)).id();
            spawn_model(&mut commands, e, model, textures, &mut meshes, &mut materials, color);
            let size: usize = model.meshes.iter().map(|m| m.vertices.len() * 48 + m.indices.len() * 4).sum();
            budget = budget.saturating_sub(size.max(1 << 16));
            if let Some(t) = next.territory.and_then(|t| scene.territories.get_mut(t)) {
                t.entity = Some(e);
                t.team = color;
            }
        }
    }
    // a territory that changed hands: its doodad in the new owner's colour
    for i in 0..scene.territories.len() {
        let color = territory_team(game, i);
        let TerritoryView { entity, team, model: path } = &scene.territories[i];
        let (Some(e), true, Some(path)) = (*entity, *team != color, path.clone()) else { continue };
        let MapScene { models, textures, territories, .. } = &mut *scene;
        let Some(model) = models.get(&path) else { continue };
        commands.entity(e).despawn_related::<Children>();
        spawn_model(&mut commands, e, model, textures, &mut meshes, &mut materials, color);
        territories[i].team = color;
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
    // the ribbons' pipeline is made with the scene: a first selection would otherwise
    // not see its arrows' ribbons, gone again before the pipeline is compiled
    if scene.ribbon_material.is_none() {
        let material = materials_ribbon(&mut std_materials);
        commands.spawn((Mesh3d(meshes.add(ribbon_mesh(&[Vec3::ZERO; 3], 0.0, [0.0; 4]))), MeshMaterial3d(material.clone()), Transform::default(), MapEntity, bevy::camera::visibility::NoFrustumCulling));
        scene.ribbon_material = Some(material);
    }
    // 934D50: the arrows follow the selection
    let show_arrows = front.ui.uvars.get_bool("MetaGameViewShowVFXArrows");
    let now = time.elapsed_secs();
    let want = wanted_arrows(game, show_arrows);
    let key = game.selected_army.filter(|_| !want.is_empty()).map(|a| (a.0, game.campaign.state.army(a).territory, want.len()));
    if key != scene.arrows_for {
        scene.arrows_for = key;
        for a in std::mem::take(&mut scene.arrows) {
            commands.entity(a.head).despawn();
            commands.entity(a.ribbon).despawn();
        }
        if let Some(from) = game.selected_army.and_then(|a| rest_position(game, a)).filter(|_| !want.is_empty()) {
            let map = game.campaign.data.campaigns[game.campaign.state.campaign].map.unwrap_or(0);
            for (t, enemy) in &want {
                let def = &game.campaign.data.maps[map].territories[game.campaign.state.territories[*t].definition];
                let to = Vec3::new(f32::from_bits(def.location[0]), f32::from_bits(def.location[1]), MAP_Z);
                let mid = Vec3::new((from.x + to.x) * 0.5, (from.y + to.y) * 0.5, ARROW_HEIGHT);
                let points = [to_render(from), to_render(mid), to_render(to)];
                let model = scene.models.get(&(if *enemy { ARROW_ATTACK_MODEL } else { ARROW_MOVE_MODEL }).to_ascii_lowercase()).cloned();
                let head = commands.spawn((Transform::from_translation(points[0]), Visibility::default(), MapEntity)).id();
                if let Some(model) = model {
                    spawn_model(&mut commands, head, &model, &scene.textures, &mut meshes, &mut materials, NO_OWNER_TEAM);
                }
                // its mesh changes with the head: no bounds to cull by
                let ribbon = commands.spawn((Transform::default(), Visibility::default(), MapEntity, bevy::camera::visibility::NoFrustumCulling)).id();
                scene.arrows.push(MoveArrow { head, ribbon, points, start: now, enemy: *enemy });
            }
        }
    }
    if !scene.arrows.is_empty() {
        let MapScene { arrows, ribbon_material, .. } = &mut *scene;
        for a in arrows.iter() {
            let age = now - a.start;
            let t = (age / ARROW_SECONDS).clamp(0.0, 1.0);
            let at = arrow_point(&a.points, t);
            if let Ok(mut tr) = transforms.get_mut(a.head) {
                tr.translation = at;
                let ahead = arrow_point(&a.points, (t + 0.02).min(1.0)) - at;
                let d = a.points[2] - a.points[0];
                let dir = if ahead.length_squared() > 1e-6 { ahead } else { Vec3::new(d.x, 0.0, d.z) };
                // models face native -Y (render +Z)
                tr.rotation = Quat::from_rotation_arc(Vec3::Z, dir.normalize_or_zero());
            }
            // the ribbon behind the head while it is alive (segments 1 s, fading 1 s)
            if age <= ARROW_SECONDS + ARROW_FULL + ARROW_FADE + 0.1 {
                let fade = ((ARROW_SECONDS + ARROW_FULL + ARROW_FADE - age) / ARROW_FADE).clamp(0.0, 1.0);
                let alpha = ARROW_ALPHA * fade;
                let color = if a.enemy { [1.0, 0.0, 0.0, alpha] } else { [0.0, 1.0, 0.0, alpha] };
                let mesh = ribbon_mesh(&a.points, t, color);
                let material = ribbon_material.get_or_insert_with(|| materials_ribbon(&mut std_materials)).clone();
                commands.entity(a.ribbon).insert((Mesh3d(meshes.add(mesh)), MeshMaterial3d(material)));
            } else {
                commands.entity(a.ribbon).remove::<Mesh3d>();
            }
        }
    }
    // 933320, without UVar MetaGameUIFastMGAI: on a computer's turn the camera looks
    // at the current kingdom's army while it is the only one on the move
    if !game.campaign.options.fast_mgai && !game.campaign.local_turn() {
        let current = game.campaign.state.current.and_then(|k| game.campaign.actor(k));
        let moving: Vec<&ArmyView> = scene.armies.values().filter(|v| Some(aa_metagame::battle::KingdomActor(v.owner)) == current && !v.path.is_empty()).collect();
        if let [only] = moving[..] {
            scene.target = Vec2::new(only.pos.x, only.pos.y);
        }
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
    mut scene: ResMut<MapScene>,
    mut front: ResMut<crate::ui::front::Front>,
    cams: Query<(&Camera, &GlobalTransform), With<MapCamera>>,
    time: Res<Time>,
) {
    if !scene.ready {
        return;
    }
    let front = &mut *front;
    let (left, right) = (front.pointer.left_pressed, front.pointer.right_pressed);
    // 957130: the pointer moved far enough from the press: the army is dragged
    if let (Some((start, _)), Some(pos)) = (scene.press, front.pointer.pos)
        && !scene.dragging
        && (pos.x - start.x).abs() + (pos.y - start.y).abs() >= front.ui.avars.float("CursorDragThreshold").unwrap_or(8.0)
    {
        scene.dragging = true;
    }
    let released = front.pointer.left_released && scene.press.is_some();
    if released {
        scene.press = None;
    }
    let dropped = released && std::mem::take(&mut scene.dragging);
    if released && !dropped {
        return;
    }
    let left = left && !dropped;
    let right = right || dropped;
    // 957540 / 5CD190: only while the world is the input target ([B3653C]+60)
    if !(left || right) || !super::map_pointer_over_world(&*front) || front.ui.message_box.is_some() || front.campaign.iface.dialog.is_some() || front.campaign.iface.help.shown() {
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
        .map(|(i, _)| ArmyId(i))
        .filter(|_| !dropped);
    // 957540: a double click selects the army and centres on it
    if let (true, Some(a)) = (left, army) {
        let now = time.elapsed_secs();
        let double = scene.last_click.is_some_and(|(t, b)| b == a && now - t <= DOUBLE_CLICK_SECONDS);
        scene.last_click = Some((now, a));
        scene.press = Some((pos, a));
        if double {
            game.selected_army = Some(a);
            game.selected_territory = None;
            iface.look_at = super::interface::selection_point(game);
            return;
        }
    }
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
