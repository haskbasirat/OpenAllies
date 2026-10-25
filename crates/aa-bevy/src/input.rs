//! Battle input: selection and orders as the original's game interface
//! state machine (aa-runtime player_order_input: click 0x640CF0, add
//! 0x640E10, box 0x436460 / 0x4377B0, double click, groups 0x439E90 /
//! 0x4376C0, right-click command 0x620760), behaviour-faithful over Bevy
//! picking of the battle view's actor entities.
//!
//! * A left press on the world (the HUD's VWorld widget, aa-ui
//!   `press_target_5c3bc0`) is a click; moving 10 px (Manhattan,
//!   AVar CursorDragThreshold) turns it into a drag box.
//! * Click: select the hit (a soldier selects his regiment, `Unit::Org`;
//!   buildings and trucks are `Unit::Actor`); empty ground clears.
//!   Shift/Ctrl toggles the hit. Double click: every visible object of the
//!   same definition as the hit (own objects only).
//! * Box: own regiments first, else own buildings (class rule 0x6406C0);
//!   at most 6 objects, the nearest to the centroid (0x640FA0).
//! * Right click with own movable units selected: an actor under the
//!   cursor (any owner) is followed (move with a target,
//!   `Command::Follow`), otherwise move to the ground point. Over the
//!   minimap (its widget takes only the left button, `0x5D77A0`) the
//!   right press reaches the world with the minimap's point (`0x6203F0` ->
//!   `0x5D74B0`): a move there.
//! * Order feedback (`0x6229F0`): an accepted location order plays
//!   `move_vfx` at its point (`crate::ui::order_marks`); the unit's
//!   waypoint flag shows the command given (`order_marks::Issued`).
//! * Unpack: the placement mode (`crate::ui::placement`): the ghost follows
//!   the cursor, a press at a legal spot fixes it, dragging turns it, the
//!   release places the building with that facing (default native -Y).
//! * Groups: Ctrl+digit saves, digit selects, a second tap within
//!   UICenterGroupTimeMS (1000 ms) centres the camera on the group.
//! * Keys: Pause = gamepause, +/- = SpeedUp/SpeedDown (hotkeys_core.txt).
use crate::battle_view::ActorView;
use crate::camera::{CameraRequests, NativeCamera};
use crate::session::{AppState, Battle};
use crate::ui::hud::{GameAction, GameInput, TargetOrder};
use crate::ui::order_marks::{self, Issued, PointVfx};
use crate::ui::placement::{self, Ghost, Placement};
use aa_play::{ActorId, Command, DefClass, Unit};
use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// The original selection limit (select 0x6409E0).
pub const MAX_SELECTION: usize = 6;
/// AVar CursorDragThreshold.
pub const DRAG_THRESHOLD: f32 = 10.0;
const DOUBLE_CLICK_S: f64 = 0.5;

#[derive(Resource, Default)]
pub struct Selecting {
    /// Left press on the world: physical pixel, time.
    press: Option<(Vec2, f64)>,
    /// Drag box corners (physical pixels) while dragging.
    pub drag: Option<(Vec2, Vec2)>,
    /// An action button armed a targeted order.
    pub armed: Option<TargetOrder>,
    groups: Vec<Vec<Unit>>,
    last_group: Option<(usize, f64)>,
    /// The idle buttons' cycle position per kind.
    idle_cycle: [usize; 3],
    last_click: Option<(Vec2, f64)>,
    /// The actor under the cursor (for the focused VFX).
    pub hover: Option<ActorId>,
    /// An armed general ability would be admitted where the cursor is
    /// (the faction_ability_target_valid cursor; combat owner).
    pub ability_valid: bool,
    /// The hover came from an organization's banner (the organization is
    /// focused, not the member).
    pub hover_banner: bool,
}

/// A pickable actor this frame.
struct Pick {
    id: ActorId,
    /// Distance along the cursor ray (hit) or None.
    dist: Option<f32>,
    /// Screen position (physical pixels) of the actor's origin.
    screen: Option<Vec2>,
}

fn native(p: Vec3) -> Vec3 {
    Vec3::new(p.x, -p.z, p.y)
}

/// Relation class (0x6406C0): own regiments 3, own buildings 2, own
/// others 1, foreign 0.
fn class(b: &Battle, u: Unit) -> u8 {
    let w = &b.world;
    match u {
        Unit::Org(o) => {
            if w.orgs.get(o as usize).is_some_and(|o| o.kingdom == b.local) {
                3
            } else {
                0
            }
        }
        Unit::Actor(a) => match w.actor(a) {
            Some(a) if a.kingdom == Some(b.local) => match w.data.def(a.def).class {
                Some(DefClass::Building) => 2,
                _ => 1,
            },
            _ => 0,
        },
    }
}

fn unit_of(b: &Battle, id: ActorId) -> Option<Unit> {
    let a = b.world.actor(id)?;
    if !a.alive {
        return None;
    }
    Some(match a.org {
        Some(o) if b.world.orgs.get(o as usize).is_some_and(|o| o.alive) => Unit::Org(o),
        _ => Unit::Actor(id),
    })
}

/// Where a unit is (native ground point).
fn unit_pos(b: &Battle, u: Unit) -> Option<Vec2> {
    match u {
        Unit::Org(o) => b.world.orgs.get(o as usize).map(|o| o.anchor),
        Unit::Actor(a) => b.world.actor(a).map(|a| a.pos.truncate()),
    }
}

fn unit_def(b: &Battle, u: Unit) -> Option<aa_play::DefId> {
    match u {
        Unit::Org(o) => b.world.orgs.get(o as usize).map(|o| o.def),
        Unit::Actor(a) => b.world.actor(a).map(|a| a.def),
    }
}

/// Keep at most 6, the nearest to the centroid (0x640FA0).
fn cap(b: &Battle, mut units: Vec<Unit>) -> Vec<Unit> {
    units.dedup();
    if units.len() <= MAX_SELECTION {
        return units;
    }
    let pts: Vec<Vec2> = units.iter().filter_map(|u| unit_pos(b, *u)).collect();
    let c = pts.iter().copied().sum::<Vec2>() / pts.len().max(1) as f32;
    units.sort_by(|x, y| {
        let dx = unit_pos(b, *x).map_or(f32::MAX, |p| p.distance_squared(c));
        let dy = unit_pos(b, *y).map_or(f32::MAX, |p| p.distance_squared(c));
        dx.total_cmp(&dy)
    });
    units.truncate(MAX_SELECTION);
    units
}

/// Ray / ground intersection over the battle heightfield (native point).
fn ground_hit(b: &Battle, ray: Ray3d) -> Option<Vec3> {
    let o = native(ray.origin);
    let d = native(*ray.direction);
    let above = |p: Vec3| p.z - b.world.map.height_at(p.truncate());
    let mut t0 = 0.0;
    let step = 2.0;
    let mut t = step;
    while t < 2000.0 {
        if above(o + d * t) < 0.0 {
            let (mut lo, mut hi) = (t0, t);
            for _ in 0..24 {
                let m = (lo + hi) * 0.5;
                if above(o + d * m) < 0.0 { hi = m } else { lo = m }
            }
            return Some(o + d * hi);
        }
        t0 = t;
        t += step;
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub fn battle_input(
    mut sel: ResMut<Selecting>,
    battle: Option<ResMut<Battle>>,
    front: Res<crate::ui::front::Front>,
    game: Option<Res<GameInput>>,
    mut requests: ResMut<CameraRequests>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<NativeCamera>>,
    views: Query<(&ActorView, &GlobalTransform, Option<&Children>)>,
    parts: Query<(&Aabb, &GlobalTransform)>,
    mut commands: Commands,
    banners: Option<Res<crate::ui::banners::BannerArt>>,
    (mut place, mut ghost, mut marks, mut issued): (ResMut<Placement>, ResMut<Ghost>, ResMut<PointVfx>, ResMut<Issued>),
) {
    let Some(mut b) = battle else {
        place.active = None;
        if *ghost != Ghost::default() {
            *ghost = Ghost::default();
        }
        return;
    };
    if game.is_some() {
        commands.remove_resource::<GameInput>();
    }
    let now = time.elapsed_secs_f64();
    let local = b.local;
    // HUD actions first
    if let Some(g) = &game {
        for a in &g.actions {
            match a {
                GameAction::Stop => {
                    let units = own_movable(&b);
                    if !units.is_empty() {
                        b.pending.push(Command::Stop { kingdom: local, units });
                    }
                }
                GameAction::Recruit { building, def } => b.pending.push(Command::Recruit { kingdom: local, building: *building, def: *def }),
                GameAction::Research { building, index } => {
                    let property = b.world.actor(*building).and_then(|a| b.world.data.def(a.def).research.get(*index)).map(|r| r.property.clone());
                    if let Some(property) = property {
                        b.pending.push(Command::Research { kingdom: local, building: *building, property });
                    }
                }
                GameAction::CancelRecruit { building, index } => b.pending.push(Command::CancelRecruit { kingdom: local, building: *building, index: *index }),
                GameAction::Pack { building } => b.pending.push(Command::Pack { kingdom: local, building: *building }),
                GameAction::Sell { building } => b.pending.push(Command::Sell { kingdom: local, building: *building }),
                GameAction::Detach { org } => b.pending.push(Command::Detach { kingdom: local, org: *org }),
                GameAction::SetFormation(i) => {
                    let orgs: Vec<aa_play::OrgId> = b
                        .selection
                        .iter()
                        .filter_map(|u| match u {
                            Unit::Org(o) if b.world.orgs.get(*o as usize).is_some_and(|x| x.kingdom == local) => Some(*o),
                            _ => None,
                        })
                        .collect();
                    if !orgs.is_empty() {
                        b.pending.push(Command::SetFormation { kingdom: local, orgs, formation: *i });
                    }
                }
                GameAction::Disband => {
                    let units = own_movable(&b);
                    if !units.is_empty() {
                        b.pending.push(Command::Disband { kingdom: local, units });
                    }
                }
                GameAction::CycleIdle(kind) => {
                    // the next idle unit of the kind, selected and centred
                    let list = crate::ui::hud::idle_units(&b, *kind);
                    if !list.is_empty() {
                        let k = (*kind as usize).min(2);
                        let i = sel.idle_cycle[k] % list.len();
                        sel.idle_cycle[k] = i + 1;
                        let u = list[i];
                        b.selection = vec![u];
                        if let Some(p) = unit_pos(&b, u) {
                            requests.look_at = Some(p);
                        }
                    }
                }
                GameAction::CommandAll { building } => {
                    // 0x60CCD0: every regiment attached to the HQ
                    let w = &b.world;
                    let hq_members: Vec<ActorId> = aa_play::production::hq_org(w, *building).map(|o| w.orgs[o as usize].members.clone()).unwrap_or_default();
                    let attached: Vec<Unit> = w
                        .orgs
                        .iter()
                        .filter(|o| o.alive && o.kingdom == local && o.hq.is_some_and(|h| h == *building || hq_members.contains(&h)))
                        .map(|o| Unit::Org(o.id))
                        .collect();
                    if !attached.is_empty() {
                        b.selection = cap(&b, attached);
                    }
                }
                GameAction::Target(t) => sel.armed = Some(t.clone()),
                // a +SELF general ability: the order without a target
                GameAction::UseAbility(i) => b.pending.push(Command::UseAbility { kingdom: local, ability: *i as u32, target: None }),
                GameAction::MinimapJump(u, v) => {
                    let m = &b.world.map;
                    requests.look_at = Some(Vec2::new(u * m.width as f32 * m.cell_size, v * m.height as f32 * m.cell_size));
                }
                GameAction::TogglePause => b.paused = !b.paused,
                GameAction::SetPaused(p) => b.paused = *p,
                GameAction::SetSpeed(s) => b.speed = s.clamp(0.25, 4.0),
                GameAction::CameraZoom(z) => requests.zoom = Some(*z),
                GameAction::SpeedUp => b.speed = speed_step(b.speed, 1.0),
                GameAction::SpeedDown => b.speed = speed_step(b.speed, -1.0),
            }
        }
    }
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let add = ctrl || keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    // keys the UI did not take (hotkeys_core / hotkeys_groups)
    if let Some(g) = &game {
        for k in &g.keys {
            match k {
                KeyCode::Pause => b.paused = !b.paused,
                KeyCode::Minus | KeyCode::NumpadSubtract => b.speed = speed_step(b.speed, -1.0),
                KeyCode::Equal | KeyCode::NumpadAdd => b.speed = speed_step(b.speed, 1.0),
                // backslash: "GameSpeed" without an argument (0x60D690):
                // exponent 0, 100%
                KeyCode::Backslash | KeyCode::IntlBackslash => b.speed = 1.0,
                _ => {}
            }
            if let Some(i) = digit(*k) {
                if sel.groups.len() < 10 {
                    sel.groups.resize(10, Vec::new());
                }
                if ctrl {
                    sel.groups[i] = b.selection.clone();
                } else {
                    let group: Vec<Unit> = sel.groups[i].iter().copied().filter(|u| unit_pos(&b, *u).is_some()).collect();
                    let again = sel.last_group.is_some_and(|(g, t)| g == i && now - t < 1.0);
                    if again && !group.is_empty() {
                        let pts: Vec<Vec2> = group.iter().filter_map(|u| unit_pos(&b, *u)).collect();
                        requests.look_at = Some(pts.iter().copied().sum::<Vec2>() / pts.len() as f32);
                    }
                    b.selection = group;
                    sel.last_group = Some((i, now));
                }
            }
        }
    }
    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_tf)) = cameras.single() else { return };
    let scale = window.scale_factor();
    // the UI's pointer (the same events the HUD saw, incl. scripted ones)
    let cursor = front.pointer.pos;
    let _ = &buttons;
    let ray = cursor.and_then(|c| camera.viewport_to_world(cam_tf, c / scale).ok());
    // pick: the nearest actor whose model bounds the ray crosses
    let mut picks = Vec::new();
    for (view, tf, children) in &views {
        let Some(a) = b.world.actor(view.id) else { continue };
        // what the fog hides cannot be picked
        if !a.alive || !crate::ui::fog::drawn(&b, view.id) {
            continue;
        }
        let mut dist: Option<f32> = None;
        if let Some(r) = ray {
            for child in children.into_iter().flatten() {
                let Ok((aabb, ptf)) = parts.get(*child) else { continue };
                let inv = ptf.affine().inverse();
                let o = inv.transform_point3(r.origin);
                let d = inv.transform_vector3(*r.direction);
                let (min, max) = (Vec3::from(aabb.center - aabb.half_extents), Vec3::from(aabb.center + aabb.half_extents));
                if let Some(t) = slab(o, d, min, max) {
                    let world_t = (ptf.transform_point(o + d * t) - r.origin).length();
                    dist = Some(dist.map_or(world_t, |x: f32| x.min(world_t)));
                }
            }
        }
        let screen = camera.world_to_viewport(cam_tf, tf.translation()).ok().map(|p| p * scale);
        picks.push(Pick { id: view.id, dist, screen });
    }
    let mut hit = picks.iter().filter(|p| p.dist.is_some()).min_by(|x, y| x.dist.unwrap().total_cmp(&y.dist.unwrap())).map(|p| (p.id, p.dist.unwrap()));
    // an organization's banner (its view) picks the organization: the
    // camera-facing square of the last drawn frame
    let mut on_banner = false;
    if let (Some(r), Some(bn)) = (ray, banners.as_deref()) {
        let (right, up) = (cam_tf.right(), cam_tf.up());
        for (o, centre, half) in &bn.placed {
            let t = (*centre - r.origin).dot(*r.direction);
            if t <= 0.0 || hit.is_some_and(|(_, d)| d <= t) {
                continue;
            }
            let local = r.origin + *r.direction * t - *centre;
            if local.dot(*right).abs() <= *half && local.dot(*up).abs() <= *half {
                let member = b.world.orgs.get(*o as usize).and_then(|g| g.members.iter().copied().find(|m| b.world.actor(*m).is_some_and(|a| a.alive)));
                if let Some(m) = member {
                    hit = Some((m, t));
                    on_banner = true;
                }
            }
        }
    }
    let hit = hit.map(|(id, _)| id);
    sel.hover = hit.filter(|_| !front.pointer_over_ui);
    // the armed general ability's cursor (faction_ability_target_valid)
    sel.ability_valid = match sel.armed {
        Some(TargetOrder::Ability(i)) => crate::ui::abilities::valid_target(&b.world, local, i, sel.hover),
        _ => false,
    };
    sel.hover_banner = on_banner && sel.hover.is_some();
    // the Unpack placement mode: the ghost at the cursor (or at the press
    // point, turned toward the cursor while dragged), tested every frame
    // (0x688860)
    let placing = match sel.armed {
        Some(TargetOrder::Deploy(truck)) => b.world.actor(truck).and_then(|a| b.world.data.def(a.def).unpacks_to).map(|d| (truck, d)),
        _ => None,
    };
    match placing {
        Some((truck, building)) => {
            if place.active.as_ref().is_none_or(|a| a.truck != truck || a.building != building) {
                place.active = Some(placement::Active { truck, building, at: Vec2::ZERO, facing: placement::DEFAULT_FACING, legal: false, problem: None, press: None, turning: false });
            }
            let g = ray.and_then(|r| ground_hit(&b, r)).map(|g| g.truncate()).filter(|_| !front.pointer_over_ui);
            let a = place.active.as_mut().expect("set");
            match a.press {
                None => {
                    if let Some(g) = g {
                        a.at = g;
                    }
                    a.facing = placement::DEFAULT_FACING;
                }
                Some((sp, gp)) => {
                    a.at = gp;
                    if let (Some(c), Some(g)) = (cursor, g) {
                        // 0x688AA0: past the drag threshold the facing follows
                        if !a.turning && (c - sp).abs().element_sum() >= DRAG_THRESHOLD {
                            a.turning = true;
                        }
                        let d = g - gp;
                        if a.turning && d != Vec2::ZERO {
                            a.facing = d.y.atan2(d.x);
                        }
                    }
                }
            }
            let (at, facing) = aa_play::deploy::placement(&b.world, building, a.at, a.facing);
            a.at = at;
            a.facing = facing;
            a.problem = placement::problem(&b.world, truck, building, at);
            a.legal = a.problem.is_none();
            // the placement state has no hover focus (that is the default
            // state's)
            sel.hover = None;
            sel.hover_banner = false;
        }
        None => place.active = None,
    }
    // left button on the world
    if front.pointer.left_pressed && !front.press_on_ui {
        if let Some(c) = cursor {
            match place.active.as_mut() {
                // 0x6887C0: an illegal spot does not place (the reason is
                // reported); a legal one fixes the ghost at the press point
                Some(a) if !a.legal => {
                    info!("placement: {}", a.problem.as_deref().unwrap_or("illegal spot"));
                }
                Some(a) => {
                    a.press = Some((c, a.at));
                    sel.press = Some((c, now));
                }
                None => sel.press = Some((c, now)),
            }
        }
    }
    if let (Some((p, _)), Some(c)) = (sel.press, cursor) {
        if front.left_held && sel.armed.is_none() && ((c - p).abs().element_sum() >= DRAG_THRESHOLD || sel.drag.is_some()) {
            sel.drag = Some((p, c));
        }
    }
    requests.blocked = sel.drag.is_some();
    if front.pointer.left_released {
        if let Some((p, _)) = sel.press.take() {
            if let Some((a, c)) = sel.drag.take() {
                // box select (0x436460 / 0x4377B0)
                let (lo, hi) = (a.min(c), a.max(c));
                let mut units: Vec<Unit> = picks
                    .iter()
                    .filter(|pk| pk.screen.is_some_and(|s| s.x >= lo.x && s.y >= lo.y && s.x <= hi.x && s.y <= hi.y))
                    .filter_map(|pk| unit_of(&b, pk.id))
                    .collect();
                units.sort_by_key(|u| std::cmp::Reverse(class(&b, *u)));
                let best = units.first().map(|u| class(&b, *u)).unwrap_or(0);
                // only own objects are box-selected; regiments beat buildings
                units.retain(|u| class(&b, *u) == best && best >= 2);
                let mut next = if add { b.selection.clone() } else { Vec::new() };
                if add && next.first().is_some_and(|u| class(&b, *u) != best) {
                    next.clear();
                }
                next.extend(units);
                if !next.is_empty() || !add {
                    b.selection = cap(&b, next);
                }
            } else if let Some(order) = sel.armed.take() {
                // targeted order from an action button
                let target = hit.filter(|id| b.world.actor(*id).is_some_and(|a| a.kingdom.is_some() && a.kingdom != Some(local)));
                let ground = ray.and_then(|r| ground_hit(&b, r));
                match order {
                    TargetOrder::Attack => {
                        let units = own_movable(&b);
                        if !units.is_empty() {
                            if let Some(t) = target {
                                b.pending.push(Command::Attack { kingdom: local, units, target: t });
                            } else if let Some(g) = ground {
                                // kill at a location (actor_commands.tgi kill)
                                order_marks::move_command(&mut marks, g.truncate());
                                issued.give(&units, g.truncate(), "waypoint_attack_actor");
                                b.pending.push(Command::Move { kingdom: local, units, to: g.truncate(), attack_move: true });
                            }
                        }
                    }
                    // the placement: where the press fixed the ghost, with the
                    // facing the drag gave it (native -Y without a drag)
                    TargetOrder::Deploy(truck) => {
                        if let Some(a) = place.active.take().filter(|a| a.truck == truck && a.press.is_some()) {
                            b.pending.push(Command::Deploy { kingdom: local, truck, at: a.at, facing: a.facing });
                        }
                    }
                    // move (5) and Run (7): a plain (passive) move; guard (2)
                    // and explore (9) move there engaging on the way
                    TargetOrder::Move | TargetOrder::Run | TargetOrder::Guard | TargetOrder::Explore => {
                        let units = own_movable(&b);
                        let engage = matches!(order, TargetOrder::Guard | TargetOrder::Explore);
                        // guard an object: its position
                        let object = hit.filter(|_| matches!(order, TargetOrder::Guard)).and_then(|id| b.world.actor(id)).map(|a| a.pos.truncate());
                        if let Some(to) = object.or(ground.map(|g| g.truncate())) {
                            if !units.is_empty() {
                                // a location order plays the move VFX; the
                                // waypoint shows the command
                                // (actor_commands*.tgi command_vfx_ids_actor)
                                if object.is_none() {
                                    order_marks::move_command(&mut marks, to);
                                }
                                let vfx = match order {
                                    TargetOrder::Run => "waypoint_run_actor",
                                    TargetOrder::Guard => "waypoint_defend_actor",
                                    TargetOrder::Explore => "waypoint_explore_actor",
                                    _ => "waypoint_move_actor",
                                };
                                issued.give(&units, to, vfx);
                                b.pending.push(Command::Move { kingdom: local, units, to, attack_move: engage });
                            }
                        }
                    }
                    TargetOrder::AttachHq => {
                        let hq = hit.filter(|id| b.world.actor(*id).is_some_and(|a| a.kingdom == Some(local)) && aa_play::production::hq_org(&b.world, *id).is_some());
                        if let Some(hq) = hq {
                            for u in b.selection.clone() {
                                if let Unit::Org(org) = u {
                                    b.pending.push(Command::Attach { kingdom: local, org, hq });
                                }
                            }
                        }
                    }
                    TargetOrder::Rally(building) => {
                        if let Some(g) = ground {
                            order_marks::move_command(&mut marks, g.truncate());
                            b.pending.push(Command::SetRallyPoint { kingdom: local, building, at: g.truncate() });
                        }
                    }
                    // a general ability: the clicked actor, else the ground
                    // point, when admitted; otherwise the targeting stays
                    TargetOrder::Ability(i) => {
                        let at = hit.map(aa_play::AbilityTarget::Actor).or(ground.map(|g| aa_play::AbilityTarget::Point(g.truncate())));
                        if at.is_some() && crate::ui::abilities::valid_target(&b.world, local, i, hit) {
                            b.pending.push(Command::UseAbility { kingdom: local, ability: i as u32, target: at });
                        } else {
                            sel.armed = Some(TargetOrder::Ability(i));
                        }
                    }
                }
            } else {
                // click (0x640CF0) / add (0x640E10) / double click
                let double = sel.last_click.is_some_and(|(q, t)| now - t <= DOUBLE_CLICK_S && (q - p).abs().max_element() <= 2.0);
                sel.last_click = Some((p, now));
                let unit = hit.and_then(|id| unit_of(&b, id));
                match unit {
                    None => {
                        if !add {
                            b.selection.clear();
                        }
                    }
                    Some(u) if double && class(&b, u) >= 1 => {
                        let def = unit_def(&b, u);
                        let mut same: Vec<Unit> = picks
                            .iter()
                            .filter(|pk| pk.screen.is_some_and(|s| s.x >= 0.0 && s.y >= 0.0 && s.x < window.physical_width() as f32 && s.y < window.physical_height() as f32))
                            .filter_map(|pk| unit_of(&b, pk.id))
                            .filter(|x| unit_def(&b, *x) == def && class(&b, *x) == class(&b, u))
                            .collect();
                        same.insert(0, u);
                        let mut next = if add { b.selection.clone() } else { Vec::new() };
                        next.extend(same);
                        b.selection = cap(&b, next);
                    }
                    Some(u) => {
                        if add {
                            if let Some(i) = b.selection.iter().position(|x| *x == u) {
                                b.selection.remove(i);
                            } else if b.selection.first().is_none_or(|f| class(&b, *f) == class(&b, u) && class(&b, u) >= 1) {
                                if b.selection.len() < MAX_SELECTION {
                                    b.selection.push(u);
                                }
                            }
                        } else {
                            b.selection = vec![u];
                        }
                    }
                }
            }
        }
    }
    // right click: cancel an armed order / box, else the hot command; over
    // the minimap the hot command's point is the minimap's (no actor there)
    let on_minimap = cursor.filter(|_| front.pointer.right_pressed && front.pointer_over_ui).and_then(|c| match front.ui.press_target_5c3bc0(c.x as i16, c.y as i16) {
        aa_ui::press_pick::PressTarget::MiniMap { path, .. } => front.ui.minimap_uv(&path, c.x, c.y),
        _ => None,
    });
    if front.pointer.right_pressed && on_minimap.is_some() {
        if sel.armed.take().is_some() || sel.drag.take().is_some() {
            sel.press = None;
        } else if let Some((u, v)) = on_minimap.filter(|(u, v)| (0.0..=1.0).contains(u) && (0.0..=1.0).contains(v)) {
            let units = own_movable(&b);
            if !units.is_empty() {
                let m = &b.world.map;
                let to = Vec2::new(u * m.width as f32 * m.cell_size, v * m.height as f32 * m.cell_size);
                debug!("order: move {units:?} to {to:?} (minimap)");
                order_marks::move_command(&mut marks, to);
                issued.give(&units, to, "waypoint_move_actor");
                b.pending.push(Command::Move { kingdom: local, units, to, attack_move: false });
            }
        }
    }
    if front.pointer.right_pressed && !front.pointer_over_ui {
        if sel.armed.take().is_some() || sel.drag.take().is_some() {
            sel.press = None;
        } else {
            let units = own_movable(&b);
            if !units.is_empty() {
                // 0x620760 -> GuessCommandData 0x4DA570: over a pickable
                // actor (own, allied, enemy, neutral, a building) only the
                // target is sent, and the first command whose
                // can_hot_command_actor allows it is move (kill's is
                // switched off in the data): the selection follows it. An
                // actor order plays no move VFX; Alt+right (HotkeyAltRight-
                // ClickCommand retreat) has no command of its own here.
                if let Some(t) = hit {
                    debug!("order: follow {t} with {units:?}");
                    b.pending.push(Command::Follow { kingdom: local, units, target: t });
                } else if let Some(g) = ray.and_then(|r| ground_hit(&b, r)) {
                    debug!("order: move {units:?} to {g:?}");
                    order_marks::move_command(&mut marks, g.truncate());
                    issued.give(&units, g.truncate(), "waypoint_move_actor");
                    b.pending.push(Command::Move { kingdom: local, units, to: g.truncate(), attack_move: false });
                }
            }
        }
    }
    // the selection keeps only living objects the local kingdom sees
    let alive: Vec<Unit> = b.selection.iter().copied().filter(|u| unit_pos(&b, *u).is_some() && match u {
        Unit::Org(o) => b.world.orgs.get(*o as usize).is_some_and(|o| o.alive && aa_play::fog::org_visible(&b.world, o.id, Some(b.local))),
        Unit::Actor(a) => b.world.actor(*a).is_some_and(|a| a.alive) && crate::ui::fog::drawn(&b, *a),
    }).collect();
    if alive.len() != b.selection.len() {
        b.selection = alive;
    }
    // the placement ends with its armed order (placed or cancelled)
    if !matches!(sel.armed, Some(TargetOrder::Deploy(_))) {
        place.active = None;
    }
    let g = placement::ghost_of(&b.world, local, &place);
    if *ghost != g {
        *ghost = g;
    }
}

/// The selected own units that take move / attack / stop orders:
/// regiments and single actors that are not buildings.
fn own_movable(b: &Battle) -> Vec<Unit> {
    b.selection
        .iter()
        .copied()
        .filter(|u| match u {
            Unit::Org(o) => b.world.orgs.get(*o as usize).is_some_and(|o| o.kingdom == b.local),
            Unit::Actor(a) => b.world.actor(*a).is_some_and(|a| a.kingdom == Some(b.local) && b.world.data.def(a.def).class != Some(DefClass::Building)),
        })
        .collect()
}

/// SpeedUp / SpeedDown: the GVar GameSpeed exponent v (speed 2^v) moved
/// by half a step, clamped to SVars GameSpeedMin / GameSpeedMax (-2 / +2:
/// 25% .. 400%), so 100% -> 141% -> 200%.
fn speed_step(speed: f32, dir: f32) -> f32 {
    let v = (speed.max(1e-3).log2() * 2.0).round() / 2.0 + 0.5 * dir;
    2f32.powf(v.clamp(-2.0, 2.0))
}

fn digit(k: KeyCode) -> Option<usize> {
    Some(match k {
        KeyCode::Digit1 => 0,
        KeyCode::Digit2 => 1,
        KeyCode::Digit3 => 2,
        KeyCode::Digit4 => 3,
        KeyCode::Digit5 => 4,
        KeyCode::Digit6 => 5,
        KeyCode::Digit7 => 6,
        KeyCode::Digit8 => 7,
        KeyCode::Digit9 => 8,
        KeyCode::Digit0 => 9,
        _ => return None,
    })
}

/// Ray / box slab test: entry distance (or 0 inside).
fn slab(o: Vec3, d: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let mut t0 = 0.0f32;
    let mut t1 = f32::MAX;
    for i in 0..3 {
        if d[i].abs() < 1e-8 {
            if o[i] < min[i] || o[i] > max[i] {
                return None;
            }
        } else {
            let inv = 1.0 / d[i];
            let (mut a, mut c) = ((min[i] - o[i]) * inv, (max[i] - o[i]) * inv);
            if a > c {
                std::mem::swap(&mut a, &mut c);
            }
            t0 = t0.max(a);
            t1 = t1.min(c);
            if t0 > t1 {
                return None;
            }
        }
    }
    Some(t0)
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Selecting>().init_resource::<CameraRequests>().init_resource::<Placement>().init_resource::<Ghost>().init_resource::<PointVfx>().init_resource::<Issued>().add_systems(
            Update,
            (crate::camera::attach, battle_input, crate::camera::update)
                .chain()
                .after(crate::ui::front::frame)
                .run_if(in_state(AppState::Battle)),
        );
    }
}
