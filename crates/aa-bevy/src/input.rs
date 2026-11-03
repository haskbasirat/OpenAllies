//! Battle input: selection and orders as the original's game interface
//! state machine (aa-runtime player_order_input: click 0x640CF0, add
//! 0x640E10, box 0x436460 / 0x4377B0, double click, groups 0x439E90 /
//! 0x4376C0, right-click command 0x620760), behaviour-faithful over Bevy
//! picking of the battle view's actor entities.
//!
//! * A left press on the world (the HUD's VWorld widget, aa-ui
//!   `press_target_5c3bc0`) is a click; moving 10 px (Manhattan,
//!   AVar CursorDragThreshold) turns it into a drag box.
//! * The selection follows the game UIWorldState (vtable 0xA61E40; port in
//!   aa-runtime `player_order_input::ui_world`): every object it takes
//!   passes the class rule `0x6406C0` against the selection's class
//!   (+0x54), at most 6 objects (`0x6409E0`), the newest at the head.
//!   - The class is the relation to the local kingdom (own 0x800, enemy
//!     0x400, allied 0x200, neutral 0x100) and the kind: an organization,
//!     a truck or another thing with a Mind that may move (actor vt+0x20,
//!     `0x4CF120`: no mover, or MobileDeployment / Recruit / Denizen) and is
//!     not static (`0x4D12C0`) 0x40 (then 0x80); a mover without those
//!     components 0x08 (then 0x10); a wall 0x04 (then 0x01); anything else
//!     (a building) 0x20, of which only one is taken. A lower relation or
//!     kind is refused; a higher one replaces the selection.
//! * Click (`0x640CF0` -> `0x437B20`): clear, then select the hit (a
//!   soldier stands for his regiment, `0x6409A0` / `0x640B40`); empty ground
//!   clears. Shift/Ctrl (`0x640E10`): a selected hit is dropped, another one
//!   added when its class fits. Double click with an own object at the
//!   selection's head: every visible object of the head's definition joins
//!   through the six-object cap (`0x640BF0` -> `0x640FA0`); Shift/Ctrl
//!   double click adds the hit's like (hit selected) or drops them.
//! * Box (`0x5CCB70` -> `0x436460`: clear, then `0x4377B0`; Shift/Ctrl
//!   keeps the selection): every visible object whose model bounds reach
//!   into the box (`0x4367A0` / `0x715D40`; here the projected bounds
//!   overlapping the rectangle) becomes a candidate through its regiment
//!   (`0x6409A0`) under the class rule (`0x640A40`); then `0x640FA0`: with
//!   more than six, the six nearest to the centroid of the selection (or
//!   of the candidates).
//! * What is under the cursor (`0x43D240` FindObjectAt): every drawn object
//!   the ray meets (its model bound and the model's box in model space,
//!   `0x454AC0` -> `0x715C10`); of those the nearest by the distance from the
//!   eye to the object's position (`0x436480`, `0x5E05D0`), not to the point
//!   the ray enters its box, so a soldier in front of a building's box stays
//!   the soldier. The hover (`0x436650` -> `0x43DC70`) keeps a single selected
//!   object while the cursor still meets it; a click on it again goes to the
//!   nearest object behind it (`0x437B20`).
//! * Right press with a selection (`0x620760`): the default order, no command
//!   named. The hovered object (through `0x6409A0`, a soldier stands for his
//!   regiment) is its target and it goes at once. Otherwise the ground point;
//!   when a selected object has a hot location command that takes a direction
//!   (`0x620A00`: definition +0x40 and +0x2C bit 5, move) the facing state
//!   (GameInterface+0xE4, `0x621230`) holds the order until the right button
//!   comes up: past the drag threshold (`0x688AA0`) `facing_arrow_vfx` stands at
//!   the press point and the direction follows the cursor (`0x688990`), and the
//!   release sends it with that direction, or zero (`0x6210B0`). Over the minimap
//!   (its widget takes only the left button, `0x5D77A0`) the right press reaches
//!   the world with the minimap's point (`0x6203F0` -> `0x5D74B0`): a zero
//!   direction at once. Each unit guesses its command on every machine (`0x4DA570`).
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
    /// The armed bombard cannot reach the ground point under the cursor
    /// (the cursor_invalid_ids cursor, `0x620C30`).
    pub bombard_invalid: bool,
    /// The facing state's order while the right button is down.
    facing: Option<FacingDrag>,
    /// The replicated selection (what the requests have said).
    replicated: Vec<Unit>,
    /// The one thing an armed order is for (a mini's button), not the selection.
    armed_unit: Option<Unit>,
    /// Tracker +0x14/+0x18: the last right press.
    right_press: Vec2,
    /// The dragged box's candidates (`0x4371A0`).
    pub box_candidates: Vec<Unit>,
}

/// The facing drag state GameInterface+0xE4 (ctor `0x6232F0`, vtable `0xA5C580`)
/// with the order `0x620A90` hands it (+0x38).
#[derive(Clone, Copy, Debug)]
struct FacingDrag {
    /// The order's location (VWorld+0x184 at the press, +0x1B4).
    at: Vec2,
    /// Input+0x28 at the press: Alt, and Shift or Ctrl.
    alt: bool,
    queue: bool,
    /// Tracker +0x14/+0x18: the last right press, which `0x688AA0` measures from
    /// whichever button holds the state.
    press: Vec2,
    /// +8: the drag passed the threshold (`0x688AA0`).
    dragging: bool,
    /// +0x0C: the direction (`0x688990`, `0x44B600`).
    aim: Vec2,
    /// +0x34: the left button holds it: an action button's order (`0x6229F0`).
    left: bool,
    /// The ActorCommand an action button named, and its waypoint flag.
    command: Option<(u32, &'static str)>,
}

/// What `0x620760` sends over the world or the minimap.
fn send_hot(b: &mut Battle, target: Option<Unit>, at: Option<Vec2>, aux: Option<Vec2>, alt: bool, queue: bool) {
    let units = b.selection.iter().rev().copied().collect();
    let kingdom = b.local;
    b.pending.push(Command::Hot { kingdom, units, command: None, target, at, aux, alt, queue });
}

/// 0x61E150 -> `0x436950`: some selected object of the local kingdom takes this
/// default order (`0x4CF430`: its guess `0x4DA570`, then vt+0x154).
fn can_send_hot(b: &Battle, target: Option<Unit>, at: bool, aux: bool, alt: bool) -> bool {
    let mut flags = 0x10;
    if at {
        flags |= 4;
    }
    if aux {
        flags |= 8;
    }
    if alt {
        flags |= 0x400;
    }
    let mut word = 0;
    if let Some(t) = target {
        let Some(g) = aa_play::player_orders::native::gtid_of_unit(&b.world, t) else { return false };
        flags |= 1;
        word = g;
    }
    let fields = aa_play::orders::guess::Fields { flags, target: word };
    b.selection.iter().any(|u| unit_kingdom(b, *u) == Some(b.local) && aa_play::orders::guess::guess(&b.world, *u, fields, false).is_some())
}

fn unit_kingdom(b: &Battle, u: Unit) -> Option<aa_play::KingdomId> {
    match u {
        Unit::Org(o) => b.world.orgs.get(o as usize).map(|o| o.kingdom),
        Unit::Actor(a) => b.world.actor(a).and_then(|a| a.kingdom),
    }
}

/// `0x620A00`: a selected object has a command that a location can start
/// (definition +0x40) and that takes a direction (+0x2C bit 5), so the right
/// press waits for its drag.
fn takes_direction(b: &Battle) -> bool {
    use aa_play::orders::guess::{DEFINITION_COUNT, action_bits, requirement, row};
    b.selection.iter().any(|u| {
        let bits = action_bits(&b.world, *u);
        (0..DEFINITION_COUNT).any(|i| bits >> i & 1 != 0 && row(i).hot_location && row(i).requirements & requirement::AUXILIARY != 0)
    })
}

/// A pickable actor this frame.
struct Pick {
    id: ActorId,
    /// The cursor ray meets the object (`0x454AC0`).
    hit: bool,
    /// The eye to the object's position (`0x436480`).
    dist: f32,
    /// Screen position (physical pixels) of the actor's origin.
    screen: Option<Vec2>,
    /// Screen rectangle (physical pixels) of its model bounds, when a box
    /// is released this frame.
    rect: Option<(Vec2, Vec2)>,
}

fn native(p: Vec3) -> Vec3 {
    Vec3::new(p.x, -p.z, p.y)
}

/// The relation bits of `0x6406C0` (view vt+0x78 / +0x84 / +0x7C / +0x80):
/// own 0x800, enemy 0x400, allied 0x200, neutral 0x100.
fn relation_bits(b: &Battle, u: Unit) -> u32 {
    let k = match u {
        Unit::Org(o) => b.world.orgs.get(o as usize).map(|o| o.kingdom),
        Unit::Actor(a) => b.world.actor(a).and_then(|a| a.kingdom),
    };
    match aa_play::setup::relation(&b.world, Some(b.local), k) {
        aa_play::books::SELF => 0x800,
        aa_play::books::ENEMY => 0x400,
        aa_play::books::FRIEND => 0x200,
        _ => 0x100,
    }
}

/// The kinds `0x6406C0` tells apart.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Actor vt+0x20 (`0x4CF120`) false: a mover without MobileDeployment,
    /// Recruit or Denizen.
    Mover,
    /// A Mind (+0x70: an organization, aircraft or element, `0x4CF480`)
    /// that is not disabled (`0x4D12C0`: Thing flag 29, static).
    Mind,
    /// A WallComponent (+0x90).
    Wall,
    Plain,
}

fn kind_of(b: &Battle, u: Unit) -> Kind {
    let w = &b.world;
    let Some(def) = (match u {
        Unit::Org(o) => w.orgs.get(o as usize).map(|o| o.def),
        Unit::Actor(a) => w.actor(a).map(|a| a.def),
    }) else {
        return Kind::Plain;
    };
    let d = w.data.def(def);
    let has = |bit: u32| d.components & (1 << bit) != 0;
    // aa-rules COMPONENTS: Projectile 1, Mover 3, Organization 7, Element
    // 8, Denizen 10, Wall 12, Recruit 16, MobileDeployment 19, Aircraft 23
    let mover = has(3);
    if mover && !(has(19) || has(16) || has(10)) {
        return Kind::Mover;
    }
    let mind = has(7) || has(23) || has(8);
    // ActorData 0x468995: bit 29 (static) stays unless a mover with speed,
    // an organization or a projectile
    let fixed = !((mover && d.speed_max > 0.0) || has(7) || has(1));
    if mind && !fixed {
        Kind::Mind
    } else if has(12) {
        Kind::Wall
    } else {
        Kind::Plain
    }
}

/// `0x6406C0(object, current)`: the object's class bits against the
/// current class, 0 when refused.
fn class_bits(b: &Battle, u: Unit, current: u32) -> u32 {
    let relation = relation_bits(b, u);
    let mut current = current;
    if current != 0 {
        if relation < current & 0xf00 {
            return 0;
        }
        if relation > current {
            current = 0;
        }
    }
    let pick = |kind: u32| if current > kind { 0 } else { kind };
    let next = |first: u32, again: u32, mask: u32| if current != 0 && current & mask != 0 { again } else { first };
    match kind_of(b, u) {
        Kind::Mover => pick(relation | next(0x08, 0x10, 0x18)),
        Kind::Mind => pick(relation | next(0x40, 0x80, 0xc0)),
        Kind::Wall => pick(relation | next(0x04, 0x01, 0x05)),
        // 0x20 first; again only while the current class has no other kind
        Kind::Plain => pick(relation | if current != 0 && current & 0xdd == 0 { 0x02 } else { 0x20 }),
    }
}

/// The selection's class (+0x54) as `0x6409E0` built it, newest last.
fn selection_class(b: &Battle) -> u32 {
    let mut class = 0;
    for u in b.selection.iter().rev() {
        let c = class_bits(b, *u, class);
        if c != 0 {
            class = c;
        }
    }
    class
}

/// `0x6409E0`: take `u` when there is room and its class fits; a higher
/// class (not one of the repeat kinds 0x93) replaces the selection.
fn select_one(b: &mut Battle, class: &mut u32, u: Unit) {
    if b.selection.len() >= MAX_SELECTION {
        return;
    }
    let c = class_bits(b, u, *class);
    if c == 0 {
        return;
    }
    if c > *class && c & 0x93 == 0 {
        b.selection.clear();
    }
    *class = c;
    if !b.selection.contains(&u) {
        b.selection.insert(0, u);
    }
}

/// `0x640B20`: the object's class fits the selection's.
fn compatible(b: &Battle, class: u32, u: Unit) -> bool {
    class == 0 || class_bits(b, u, class) != 0
}

/// `0x4367A0` over the release or the dragged rectangle: every drawn object whose model
/// reaches into it, through its regiment (`0x6409A0`), targetable (`0x5C19E0` ->
/// `0x45B470`), under the class rule (`0x640A40`), head-inserted. The hovered object
/// `0x4377B0` also takes is never there: the box state's vt+0x20 turns the hover off
/// (`0x5C1E70`).
fn box_candidates(b: &Battle, picks: &[Pick], lo: Vec2, hi: Vec2) -> Vec<Unit> {
    let mut inside: Vec<&Pick> = picks
        .iter()
        .filter(|pk| pk.rect.or(pk.screen.map(|s| (s, s))).is_some_and(|(rlo, rhi)| rlo.x <= hi.x && rlo.y <= hi.y && rhi.x >= lo.x && rhi.y >= lo.y))
        .collect();
    // the view list in creation order
    inside.sort_by_key(|pk| pk.id);
    let mut candidates: Vec<Unit> = Vec::new();
    let mut class = 0;
    for pk in inside {
        let Some(u) = unit_of(b, pk.id) else { continue };
        let targetable = match u {
            Unit::Org(o) => b.world.orgs.get(o as usize).is_some_and(|o| b.world.data.def(o.def).targetable),
            Unit::Actor(x) => b.world.actor(x).is_some_and(|x| b.world.data.def(x.def).targetable),
        };
        if candidates.contains(&u) || !targetable {
            continue;
        }
        let c = class_bits(b, u, class);
        if c == 0 {
            continue;
        }
        if c > class && c & 0x93 == 0 {
            candidates.clear();
        }
        class = c;
        candidates.insert(0, u);
    }
    candidates
}

/// `0x640FA0`: select a list under the six-object cap; with more than the
/// room left, the `6 - selected` compatible ones nearest (XY) to the
/// centroid of the selection (or of the list when nothing is selected).
fn select_list(b: &mut Battle, list: &[Unit]) {
    let mut class = selection_class(b);
    let count = b.selection.len();
    if count >= MAX_SELECTION {
        return;
    }
    let chosen: Vec<Unit> = if list.len() + count <= MAX_SELECTION {
        list.to_vec()
    } else {
        let source: Vec<Unit> = if count > 0 { b.selection.clone() } else { list.to_vec() };
        let pts: Vec<Vec2> = source.iter().filter_map(|u| unit_pos(b, *u)).collect();
        let c = pts.iter().copied().sum::<Vec2>() / pts.len().max(1) as f32;
        let slots = MAX_SELECTION - count;
        let mut best: Vec<(Unit, f32)> = Vec::with_capacity(slots);
        for u in list.iter().copied() {
            if b.selection.contains(&u) || !compatible(b, class, u) {
                continue;
            }
            let Some(p) = unit_pos(b, u) else { continue };
            let d2 = p.distance_squared(c);
            if best.len() < slots {
                best.push((u, d2));
                continue;
            }
            // the first strictly largest entry is replaced by a nearer one
            let mut worst = 0;
            for i in 1..slots {
                if best[i].1 > best[worst].1 {
                    worst = i;
                }
            }
            if d2 < best[worst].1 {
                best[worst] = (u, d2);
            }
        }
        best.iter().rev().map(|(u, _)| *u).collect()
    };
    // 0x4368C0
    for u in chosen {
        if !b.selection.contains(&u) && compatible(b, class, u) {
            select_one(b, &mut class, u);
        }
    }
}

/// `0x640BF0`: the visible objects of `u`'s definition (head-inserted, so
/// in reverse list order).
fn same_definition(b: &Battle, picks: &[Pick], u: Unit, on_screen: impl Fn(&Pick) -> bool) -> Vec<Unit> {
    let def = unit_def(b, u);
    let mut out: Vec<Unit> = Vec::new();
    for pk in picks.iter().filter(|pk| on_screen(pk)) {
        // the object's own view (a soldier's +0x10 is the soldier's)
        let x = Unit::Actor(pk.id);
        let x = if unit_def(b, x) == def { x } else { match unit_of(b, pk.id) { Some(o) if unit_def(b, o) == def => o, _ => continue } };
        if !out.contains(&x) {
            out.insert(0, x);
        }
    }
    out
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
fn cap(b: &Battle, units: Vec<Unit>) -> Vec<Unit> {
    let mut seen = Vec::new();
    let mut units: Vec<Unit> = units.into_iter().filter(|u| if seen.contains(u) { false } else { seen.push(*u); true }).collect();
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
            // the order gate: no selection or command from a viewer without a
            // kingdom (rule 0x766C70, FILM-PLAYBACK.md 8.1)
            if b.local_kingdom().is_none() && !viewer_action(a) {
                continue;
            }
            match a {
                // SelectObservingKingdom (0x5F4780 -> 0x5C0E90)
                GameAction::ObserveKingdom(k) => b.set_view_kingdom(*k),
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
                GameAction::Order(order) => {
                    // An escort order goes to the selected structure's Denizen.
                    let units = match order {
                        aa_play::orders::ActorOrder::Escort { .. } | aa_play::orders::ActorOrder::ConstructionCancel => b.selection.clone(),
                        _ => own_movable(&b),
                    };
                    if !units.is_empty() {
                        b.pending.push(Command::Order { kingdom: local, units, order: order.clone() });
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
                GameAction::CancelTarget => sel.armed = None,
                // a +SELF general ability: the order without a target
                GameAction::UseAbility(i) => b.pending.push(Command::UseAbility { kingdom: local, ability: *i as u32, target: None }),
                GameAction::MinimapJump(u, v) => {
                    let m = &b.world.map;
                    requests.look_at = Some(Vec2::new(u * m.width as f32 * m.cell_size, v * m.height as f32 * m.cell_size));
                }
                GameAction::SelectThing(gtid) => {
                    if let Some(u) = b.world.ids.thing(*gtid).and_then(aa_play::ids::unit) {
                        b.selection = vec![u];
                    }
                }
                GameAction::OrderFor { gtid, order } => {
                    if let Some(u) = b.world.ids.thing(*gtid).and_then(aa_play::ids::unit) {
                        b.pending.push(Command::Order { kingdom: local, units: vec![u], order: order.clone() });
                    }
                }
                GameAction::TargetFor { gtid, order } => {
                    sel.armed_unit = b.world.ids.thing(*gtid).and_then(aa_play::ids::unit);
                    sel.armed = Some(order.clone());
                }
                GameAction::TogglePause => b.paused = !b.paused,
                // (the menu's pause is a single player's: a session goes on, 0x76AEE0)
                GameAction::SetPaused(p) => {
                    if b.lan.is_none() {
                        b.paused = *p;
                    }
                }
                GameAction::SetSpeed(s) => b.set_speed_exponent(s.max(1e-6).log2()),
                GameAction::CameraZoom(z) => requests.zoom = Some(*z),
                GameAction::SpeedUp => b.speed_step(1.0),
                GameAction::SpeedDown => b.speed_step(-1.0),
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
                KeyCode::Minus | KeyCode::NumpadSubtract => b.speed_step(-1.0),
                KeyCode::Equal | KeyCode::NumpadAdd => b.speed_step(1.0),
                // backslash: "GameSpeed" without an argument (0x60D690):
                // exponent 0, 100%
                KeyCode::Backslash | KeyCode::IntlBackslash => b.speed = 1.0,
                _ => {}
            }
            if let Some(i) = digit(*k).filter(|_| b.local_kingdom().is_some()) {
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
    // FindObjectAt 0x43D240: every object the ray meets, the nearest by position
    let mut picks = Vec::new();
    let want_rects = sel.drag.is_some();
    for (view, tf, children) in &views {
        let Some(a) = b.world.actor(view.id) else { continue };
        // what the fog hides cannot be picked
        if !a.alive || !crate::ui::fog::drawn(&b, view.id) {
            continue;
        }
        // 0x715C10: the model's bound sphere, then its box in model space (the
        // parts' boxes as one: `(model+8)+0x4C` is the whole model's, 0x715D40)
        let mut hit_model = false;
        if let Some(r) = ray {
            let at = tf.translation();
            let mut radius = 0.0f32;
            for child in children.into_iter().flatten() {
                let Ok((aabb, ptf)) = parts.get(*child) else { continue };
                let world_centre = ptf.transform_point(Vec3::from(aabb.center));
                let reach = ptf.affine().matrix3.mul_vec3a(aabb.half_extents).length().max(ptf.affine().transform_vector3a(aabb.half_extents).length());
                radius = radius.max((world_centre - at).length() + reach * 1.8);
            }
            let to = at - r.origin;
            let along = to.dot(*r.direction);
            if radius > 0.0 && to.length_squared() - along * along <= radius * radius {
                let inv = tf.affine().inverse();
                let mut model: Option<(Vec3, Vec3)> = None;
                for child in children.into_iter().flatten() {
                    let Ok((aabb, ptf)) = parts.get(*child) else { continue };
                    let (c, h) = (Vec3::from(aabb.center), Vec3::from(aabb.half_extents));
                    for i in 0..8 {
                        let corner = c + h * Vec3::new(if i & 1 == 0 { -1.0 } else { 1.0 }, if i & 2 == 0 { -1.0 } else { 1.0 }, if i & 4 == 0 { -1.0 } else { 1.0 });
                        let q = inv.transform_point3(ptf.transform_point(corner));
                        model = Some(model.map_or((q, q), |(lo, hi)| (lo.min(q), hi.max(q))));
                    }
                }
                if let Some((lo, hi)) = model {
                    hit_model = slab(inv.transform_point3(r.origin), inv.transform_vector3(*r.direction), lo, hi).is_some();
                }
            }
        }
        // 0x5C19E0's tail (0x45B470): what is not targetable is not found
        let hit_model = hit_model && b.world.data.def(a.def).targetable;
        let dist = ray.map_or(f32::MAX, |r| (tf.translation() - r.origin).length());
        let screen = camera.world_to_viewport(cam_tf, tf.translation()).ok().map(|p| p * scale);
        let mut rect: Option<(Vec2, Vec2)> = None;
        if want_rects {
            for child in children.into_iter().flatten() {
                let Ok((aabb, ptf)) = parts.get(*child) else { continue };
                let (c, h) = (Vec3::from(aabb.center), Vec3::from(aabb.half_extents));
                for i in 0..8 {
                    let corner = c + h * Vec3::new(if i & 1 == 0 { -1.0 } else { 1.0 }, if i & 2 == 0 { -1.0 } else { 1.0 }, if i & 4 == 0 { -1.0 } else { 1.0 });
                    if let Ok(s) = camera.world_to_viewport(cam_tf, ptf.transform_point(corner)) {
                        let s = s * scale;
                        rect = Some(rect.map_or((s, s), |(lo, hi)| (lo.min(s), hi.max(s))));
                    }
                }
            }
        }
        picks.push(Pick { id: view.id, hit: hit_model, dist, screen, rect });
    }
    // the view list in creation order: 0x5E05D0, the later of two equally near
    picks.sort_by_key(|pk| pk.id);
    let mut hits: Vec<(ActorId, f32)> = picks.iter().filter(|p| p.hit).map(|p| (p.id, p.dist)).collect();
    let mut hit = hits.iter().copied().fold(None, |best: Option<(ActorId, f32)>, h| match best {
        Some(b) if b.1 < h.1 => Some(b),
        _ => Some(h),
    });
    // an organization's banner (its view) picks the organization: the
    // camera-facing square of the last drawn frame
    let mut on_banner = false;
    if let (Some(r), Some(bn)) = (ray, banners.as_deref()) {
        let (right, up) = (cam_tf.right(), cam_tf.up());
        for (o, centre, half) in &bn.placed {
            let t = (*centre - r.origin).dot(*r.direction);
            if t <= 0.0 {
                continue;
            }
            let local = r.origin + *r.direction * t - *centre;
            if local.dot(*right).abs() <= *half && local.dot(*up).abs() <= *half {
                let member = b.world.orgs.get(*o as usize).and_then(|g| g.members.iter().copied().find(|m| b.world.actor(*m).is_some_and(|a| a.alive)));
                if let Some(m) = member {
                    let d = (*centre - r.origin).length();
                    hits.push((m, d));
                    if !hit.is_some_and(|(_, best)| best < d) {
                        hit = Some((m, d));
                        on_banner = true;
                    }
                }
            }
        }
    }
    let hit = hit.map(|(id, _)| id);
    // 0x43DC70: a single selected object the ray still meets stays the hovered one
    let single = match b.selection.as_slice() {
        [u] => hits.iter().map(|h| h.0).find(|id| unit_of(&b, *id) == Some(*u) || Unit::Actor(*id) == *u),
        _ => None,
    };
    let hovered = single.or(hit);
    sel.hover = hovered.filter(|_| !front.pointer_over_ui);
    // the armed general ability's cursor (faction_ability_target_valid)
    sel.ability_valid = match sel.armed {
        Some(TargetOrder::Ability(i)) => crate::ui::abilities::valid_target(&b.world, local, i, sel.hover),
        _ => false,
    };
    sel.hover_banner = on_banner && sel.hover.is_some();
    // the order gate: a viewer without a kingdom selects nothing and gives no
    // order (SelectionSet and the world commands are rejected, 0x766C70); the
    // camera stays free
    if b.local_kingdom().is_none() {
        sel.armed = None;
        sel.press = None;
        sel.drag = None;
        sel.facing = None;
        sel.ability_valid = false;
        sel.bombard_invalid = false;
        requests.blocked = false;
        place.active = None;
        if *ghost != Ghost::default() {
            *ghost = Ghost::default();
        }
        if !b.selection.is_empty() {
            b.selection.clear();
        }
        return;
    }
    // the targeting state's cursor (0x620C30): bombard (command 1, which
    // has cursor_invalid_ids) shows its invalid cursor while the first
    // selected object relates to the ground point under the cursor
    // (0x6203F0) other than "in range" (0x4D5180(point, 1): the least of
    // an organization's members, 0x57EAF0 for an actor)
    sel.bombard_invalid = matches!(sel.armed, Some(TargetOrder::Bombard))
        && match (b.selection.first().copied(), ray.and_then(|r| ground_hit(&b, r))) {
            (Some(u), Some(g)) => bombard_relation(&b, u, g.truncate()) != 0,
            _ => false,
        };
    // the Unpack placement mode: the ghost at the cursor (or at the press
    // point, turned toward the cursor while dragged), tested every frame
    // (0x688860)
    // (0x612A70: Unpack with the truck's structure, a construct item
    // without a truck)
    let placing = match sel.armed {
        Some(TargetOrder::Deploy(truck)) => b.world.actor(truck).and_then(|a| b.world.data.def(a.def).unpacks_to).map(|d| (Some(truck), d)),
        Some(TargetOrder::Construct(def)) => Some((None, def)),
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
        // 0x6229F0: an action button's location order whose command takes a direction
        // (+0x2C bit 5) goes through the facing state, the left button holding it
        let held = match sel.armed {
            Some(TargetOrder::Move) => Some((5, "waypoint_move_actor")),
            Some(TargetOrder::Run) => Some((7, "waypoint_run_actor")),
            Some(TargetOrder::Guard) if hit.is_none() => Some((2, "waypoint_defend_actor")),
            Some(TargetOrder::Explore) => Some((9, "waypoint_explore_actor")),
            _ => None,
        }
        .filter(|(c, _)| aa_play::orders::guess::row(*c).requirements & aa_play::orders::guess::requirement::AUXILIARY != 0);
        if let (Some(command), Some(g), Some(_)) = (held, ray.and_then(|r| ground_hit(&b, r)), cursor) {
            let press = sel.right_press;
            sel.facing = Some(FacingDrag { at: g.truncate(), alt: false, queue: add, press, dragging: false, aim: Vec2::ZERO, left: true, command: Some(command) });
        } else if let Some(c) = cursor {
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
    // 0x437DC0 -> 0x4371A0: the candidates of the dragged box, every frame (view flag
    // 0x8000, the focus VFX)
    sel.box_candidates = match sel.drag {
        Some((p, c)) => box_candidates(&b, &picks, p.min(c), p.max(c)),
        None => Vec::new(),
    };
    if front.pointer.left_released {
        if let Some((p, _)) = sel.press.take() {
            if let Some((a, c)) = sel.drag.take() {
                // box select (0x5CCB70: 0x436460 clears, then 0x4377B0;
                // Shift/Ctrl keeps the selection)
                let (lo, hi) = (a.min(c), a.max(c));
                let candidates = box_candidates(&b, &picks, lo, hi);
                if !add {
                    b.selection.clear();
                }
                select_list(&mut b, &candidates);
            } else if let Some(order) = sel.armed.take() {
                // targeted order from an action button
                let target = hit.filter(|id| b.world.actor(*id).is_some_and(|a| a.kingdom.is_some() && a.kingdom != Some(local)));
                let ground = ray.and_then(|r| ground_hit(&b, r));
                match order {
                    TargetOrder::Attack => {
                        let units = own_movable(&b);
                        if !units.is_empty() {
                            // 0x527FB0: bombard when the layout fires indirect
                            // (0x578AC0, 0x57A5E0), at the thing or the location.
                            let (bombard, attack): (Vec<Unit>, Vec<Unit>) = units.into_iter().partition(|u| matches!(u, Unit::Org(o) if aa_play::orders::guess::bombard_layout(&b.world, *o)));
                            if let Some(t) = target {
                                if !bombard.is_empty() {
                                    b.pending.push(Command::Bombard { kingdom: local, units: bombard, target: aa_play::AbilityTarget::Actor(t) });
                                }
                                if !attack.is_empty() {
                                    b.pending.push(Command::Attack { kingdom: local, units: attack, target: t });
                                }
                            } else if let Some(g) = ground {
                                // kill at a location (actor_commands.tgi kill)
                                order_marks::move_command(&mut marks, g.truncate());
                                if !bombard.is_empty() {
                                    b.pending.push(Command::Bombard { kingdom: local, units: bombard, target: aa_play::AbilityTarget::Point(g.truncate()) });
                                }
                                if !attack.is_empty() {
                                    issued.give(&attack, g.truncate(), "waypoint_attack_actor");
                                    b.pending.push(Command::Move { kingdom: local, units: attack, to: g.truncate(), attack_move: true });
                                }
                            }
                        }
                    }
                    // the placement: where the press fixed the ghost, with the
                    // facing the drag gave it (native -Y without a drag)
                    TargetOrder::Deploy(truck) => {
                        if let Some(a) = place.active.take().filter(|a| a.truck == Some(truck) && a.press.is_some()) {
                            b.pending.push(Command::Deploy { kingdom: local, truck, at: a.at, facing: a.facing });
                        }
                    }
                    // move (5), Run (7), guard (2) and explore (9): a location,
                    // for guard also the clicked object
                    TargetOrder::Move | TargetOrder::Run | TargetOrder::Guard | TargetOrder::Explore => {
                        let units = match sel.armed_unit.take() {
                            Some(u) => vec![u],
                            None => own_movable(&b),
                        };
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
                                // retreat (7) 0x51D1E0, guard (2) 0x51F360 and
                                // explore (9) 0x5249A0 are their own Mind states
                                // (`aa_play::orders`).
                                let order = match order {
                                    TargetOrder::Run => Some(aa_play::orders::ActorOrder::Retreat { target: None, at: to }),
                                    TargetOrder::Guard => Some(aa_play::orders::ActorOrder::Guard { target: hit.filter(|_| object.is_some()).map(Unit::Actor), at: to, aux: None }),
                                    TargetOrder::Explore => Some(aa_play::orders::ActorOrder::Explore { at: to, scalar: None }),
                                    _ => None,
                                };
                                match order {
                                    Some(order) => b.pending.push(Command::Order { kingdom: local, units, order }),
                                    None => b.pending.push(Command::Move { kingdom: local, units, to, attack_move: false }),
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
                    // bombard (1) at the clicked enemy, else the ground point
                    TargetOrder::Bombard => {
                        let units = own_movable(&b);
                        let at = target.map(aa_play::AbilityTarget::Actor).or(ground.map(|g| aa_play::AbilityTarget::Point(g.truncate())));
                        if let (false, Some(target)) = (units.is_empty(), at) {
                            b.pending.push(Command::Bombard { kingdom: local, units, target });
                        }
                    }
                    // repair (3) of the clicked actor (the command takes an
                    // actor only: can_hot_command_location false)
                    TargetOrder::Repair => {
                        let units = own_movable(&b);
                        if let (false, Some(target)) = (units.is_empty(), hit) {
                            b.pending.push(Command::Repair { kingdom: local, units, target });
                        }
                    }
                    // a construct page structure: where the press fixed the
                    // ghost, with the facing the drag gave it (0x621350)
                    TargetOrder::Construct(def) => {
                        let units = own_movable(&b);
                        if let Some(a) = place.active.take().filter(|a| a.truck.is_none() && a.building == def && a.press.is_some()) {
                            if !units.is_empty() {
                                b.pending.push(Command::Construct { kingdom: local, units, def, at: a.at, facing: a.facing });
                            }
                        }
                    }
                    // an airfield's air attack (0x19, the clicked enemy or
                    // the ground point) / air recon (0x1A, the ground point)
                    TargetOrder::AirAttack(building) | TargetOrder::AirRecon(building) => {
                        let enemy = target.filter(|_| matches!(order, TargetOrder::AirAttack(_)));
                        let at = enemy.map(aa_play::AbilityTarget::Actor).or(ground.map(|g| aa_play::AbilityTarget::Point(g.truncate())));
                        if let Some(target) = at {
                            b.pending.push(match order {
                                TargetOrder::AirAttack(_) => Command::AirAttack { kingdom: local, building, target },
                                _ => Command::AirRecon { kingdom: local, building, target },
                            });
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
                let (pw, ph) = (window.physical_width() as f32, window.physical_height() as f32);
                let on_screen = |pk: &Pick| pk.screen.is_some_and(|s| s.x >= 0.0 && s.y >= 0.0 && s.x < pw && s.y < ph);
                let head_own = b.selection.first().is_some_and(|h| relation_bits(&b, *h) == 0x800);
                match (add, double) {
                    // 0x640CF0 double: the head's like, when the head is own
                    (false, true) if head_own => {
                        let head = b.selection[0];
                        let list = same_definition(&b, &picks, head, on_screen);
                        if !list.is_empty() {
                            select_list(&mut b, &list);
                        }
                    }
                    // 0x640CF0 -> 0x437B20: the hit alone (none on bare
                    // ground); a single selected object the ray meets again gives
                    // way to the nearest hit behind it
                    (false, _) => {
                        let behind = match b.selection.as_slice() {
                            [one] => hits.iter().find(|h| unit_of(&b, h.0) == Some(*one) || Unit::Actor(h.0) == *one).map(|s| s.1).and_then(|d| {
                                hits.iter().filter(|h| d < h.1 && unit_of(&b, h.0) != Some(*one)).fold(None, |best: Option<(ActorId, f32)>, h| match best {
                                    Some(x) if x.1 < h.1 => Some(x),
                                    _ => Some(*h),
                                })
                            }),
                            _ => None,
                        };
                        let pick = behind.and_then(|h| unit_of(&b, h.0)).or(unit);
                        b.selection.clear();
                        if let Some(u) = pick {
                            let mut class = 0;
                            select_one(&mut b, &mut class, u);
                        }
                    }
                    // 0x640E10: drop a selected hit, add another that fits
                    (true, false) => {
                        if let Some(u) = unit {
                            if let Some(i) = b.selection.iter().position(|x| *x == u) {
                                b.selection.remove(i);
                            } else {
                                let mut class = selection_class(&b);
                                if compatible(&b, class, u) {
                                    select_one(&mut b, &mut class, u);
                                }
                            }
                        }
                    }
                    // 0x640E10 double: the hit's like when the hit itself is
                    // selected, else drop every selected one of its kind
                    (true, true) => {
                        if let Some(h) = hit {
                            let raw = Unit::Actor(h);
                            if b.selection.contains(&raw) {
                                let list = same_definition(&b, &picks, raw, on_screen);
                                if !list.is_empty() {
                                    select_list(&mut b, &list);
                                }
                            } else {
                                let def = unit_def(&b, raw);
                                let keep: Vec<Unit> = b.selection.iter().copied().filter(|x| unit_def(&b, *x) != def).collect();
                                b.selection = keep;
                            }
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
            if !b.selection.is_empty() {
                let m = &b.world.map;
                let to = Vec2::new(u * m.width as f32 * m.cell_size, v * m.height as f32 * m.cell_size);
                let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
                // 0x620976: flag 8 with the zero vector at 0xB279C0, sent at once,
                // then the marker 0x43DB10.
                debug!("order: default at {to:?} (minimap)");
                order_marks::move_command(&mut marks, to);
                issued.give(&own_movable(&b), to, "waypoint_move_actor");
                send_hot(&mut b, None, Some(to), Some(Vec2::ZERO), alt, add);
            }
        }
    }
    // Tracker +0x14/+0x18 (0x5BF2F0)
    if front.pointer.right_pressed && let Some(c) = cursor {
        sel.right_press = c;
    }
    // the facing state (GameInterface+0xE4): a modal state, the press it holds
    // until its button comes up
    if let Some(mut f) = sel.facing {
        if let Some(c) = cursor {
            // 0x688AA0: past the threshold from the right press (tracker +0x14)
            if !f.dragging && (f.press - c).abs().element_sum() >= DRAG_THRESHOLD {
                f.dragging = true;
            }
            // 0x688990: from the press point to the pointer's, when it is not zero
            if f.dragging {
                if let Some(g) = ray.and_then(|r| ground_hit(&b, r)) {
                    let d = g.truncate() - f.at;
                    if d.x != 0.0 || d.y != 0.0 {
                        let l = d.length();
                        f.aim = if l <= 1e-6 { Vec2::ZERO } else { d / l };
                    }
                }
            }
        }
        sel.facing = Some(f);
        // 0x6886B0 -> 0x6210B0: flag 8 with the direction, or zero without a drag
        if f.left && front.pointer.left_released {
            sel.facing = None;
            sel.armed = None;
            let aim = if f.dragging { f.aim } else { Vec2::ZERO };
            let units = match sel.armed_unit.take() {
                Some(u) => vec![u],
                None => own_movable(&b),
            };
            if let (false, Some((command, vfx))) = (units.is_empty(), f.command) {
                order_marks::move_command(&mut marks, f.at);
                issued.give(&units, f.at, vfx);
                b.pending.push(Command::Hot { kingdom: local, units, command: Some(command), target: None, at: Some(f.at), aux: Some(aim), alt: false, queue: f.queue });
            }
        } else if !f.left && front.pointer.right_released {
            sel.facing = None;
            let aim = if f.dragging { f.aim } else { Vec2::ZERO };
            if can_send_hot(&b, None, true, true, f.alt) {
                debug!("order: default at {:?}, direction {aim:?}", f.at);
                order_marks::move_command(&mut marks, f.at);
                issued.give(&own_movable(&b), f.at, "waypoint_move_actor");
                send_hot(&mut b, None, Some(f.at), Some(aim), f.alt, f.queue);
            }
        }
    } else if front.pointer.right_pressed && !front.pointer_over_ui {
        if sel.armed.take().is_some() || sel.drag.take().is_some() {
            sel.press = None;
        } else if !b.selection.is_empty() {
            // 0x620760: the default order. Alt is 0x4D9B30's flag 0x400, which
            // 0x4DA570 reads as the hot command HotkeyAltRightClickCommand; Shift
            // or Ctrl is +0x3C.
            let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
            // 0x4363B0: the hovered object's vt+0x54, a soldier his regiment
            if let Some(t) = hovered.and_then(|id| unit_of(&b, id)) {
                // 0x61E150, then 0x61EEF0
                if can_send_hot(&b, Some(t), false, false, alt) {
                    debug!("order: default on {t:?}");
                    send_hot(&mut b, Some(t), None, None, alt, add);
                }
            } else if let (Some(g), Some(c)) = (ray.and_then(|r| ground_hit(&b, r)).map(|g| g.truncate()), cursor) {
                if takes_direction(&b) {
                    // 0x620A90 -> 0x621230: the state holds it
                    sel.facing = Some(FacingDrag { at: g, alt, queue: add, press: c, dragging: false, aim: Vec2::ZERO, left: false, command: None });
                } else {
                    // 0x620A51: at once, then the marker. A lone own structure's is
                    // set_rally_point (16, 0x14 and no direction).
                    debug!("order: default at {g:?}");
                    if can_send_hot(&b, None, true, false, alt) {
                        order_marks::move_command(&mut marks, g);
                        send_hot(&mut b, None, Some(g), None, alt, add);
                    }
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
    // 0x436AD0, 0x436B90, 0x436C50: the requests of a selection that changed. What
    // left it by dying or going out of sight sends nothing.
    let gone: Vec<Unit> = sel.replicated.iter().copied().filter(|u| !b.selection.contains(u) && match u {
        Unit::Org(o) => !b.world.orgs.get(*o as usize).is_some_and(|o| o.alive),
        Unit::Actor(a) => !b.world.actor(*a).is_some_and(|a| a.alive),
    }).collect();
    sel.replicated.retain(|u| !gone.contains(u));
    let oldest_first: Vec<Unit> = b.selection.iter().rev().copied().collect();
    if oldest_first != sel.replicated {
        sel.replicated = oldest_first.clone();
        b.pending.push(Command::Select { kingdom: local, units: oldest_first });
    }
    // the placement ends with its armed order (placed or cancelled)
    if !matches!(sel.armed, Some(TargetOrder::Deploy(_)) | Some(TargetOrder::Construct(_))) {
        place.active = None;
    }
    let mut g = placement::ghost_of(&b.world, local, &place);
    // 0x620230: VFXSelectFacing at the press point while a move's facing is dragged
    if let Some(f) = sel.facing.filter(|f| f.dragging) {
        g.arrow = Some((Vec3::new(f.at.x, f.at.y, b.world.map.height_at(f.at)), f.aim.y.atan2(f.aim.x)));
    }
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

/// `0x4D5180(point, 1)` for unit `u`: an organization's least member
/// relation (stopping at 0), an actor's own (core `point_relation`, the
/// Performer `0x57EAF0`: 0 in range, 1 out of range, 2 inside the minimum
/// range, 3 outside the turret arc, 4 no bombard ability); 6 without either.
fn bombard_relation(b: &Battle, u: Unit, point: Vec2) -> u8 {
    let w = &b.world;
    let alive = |a: ActorId| w.actor(a).is_some_and(|x| x.alive);
    match u {
        Unit::Org(o) => {
            let mut best = 6;
            for m in w.orgs.get(o as usize).map(|o| o.members.clone()).unwrap_or_default() {
                if alive(m) {
                    best = best.min(aa_play::combat::tasks::point_relation(w, m, point));
                    if best == 0 {
                        break;
                    }
                }
            }
            best
        }
        Unit::Actor(a) if alive(a) => aa_play::combat::tasks::point_relation(w, a, point),
        Unit::Actor(_) => 6,
    }
}

/// What a viewer without a kingdom may still do (the orders that have no
/// film check, FILM-PLAYBACK.md 8.1): speed, pause, the camera, the view.
fn viewer_action(a: &GameAction) -> bool {
    matches!(
        a,
        GameAction::ObserveKingdom(_)
            | GameAction::MinimapJump(..)
            | GameAction::TogglePause
            | GameAction::SetPaused(_)
            | GameAction::SetSpeed(_)
            | GameAction::CameraZoom(_)
            | GameAction::SpeedUp
            | GameAction::SpeedDown
    )
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
