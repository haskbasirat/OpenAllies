//! `KKC_GGroup` (Source/Sim/GGroup.cpp, thing vtable 0xA29490, Updatable +180
//! vtable 0xA29464) and the selection's path of a command, 0x4ED360. Census X2.
//!
//! With the group behaviour of a selection on (0x4ED360 cases 0x25 / 0x26:
//! selection settings +18, the boolean into +20), a command to more than one
//! thing makes a new group (`CreateGGroup` 0x4FBE10, then 0x4E0940) and goes to
//! it (0x4DFE20). The group gives the command to each member (0x4DF840 -> 0x4DF100
//! -> 0x4DE7F0), and a member's Move takes its place in the group's formation from
//! the group (0x51E9B0 -> 0x51D840 -> 0x4DE6C0). The group then thinks once a second
//! on the deferred queue (0x4E0880):
//!
//! | +1B8 | state | Update |
//! |---|---|---|
//! | 0 | idle | 0x4DFFF0: a member attacked turns it to state 2; a queued command runs |
//! | 1 | commanded | 0x4E0330: all members resting ends it; otherwise it evens out their speeds so they arrive together |
//! | 2 | under attack | 0x4E00A0: a free member guards one that fights; no fight left runs the command again |
//!
//! Without the behaviour each thing of a selection takes the command itself, the
//! position moved by where it stands in the selection (0x4ED360 lower path, 0x4ED280,
//! 0x4DD860), for a command with `apply_group_offset`.
//!
//! **What is exact and what is a rendition.** The state machine, the formation
//! placement 0x4DF600, the membership and the arithmetic are the native order. Not
//! fixture-proven: there is no oracle yet, and the checks against the original are
//! the film's group things and the mind-state comparison of `order_fights`. These
//! read aa-play's state in place of the native fields, and are marked where used:
//! the zone of slot 7 (0x515F40, `KKC_Zone` of radius +1B4) is a circle over open
//! tiles, not a flood; the actor state bits come from `resting`, `reforming`,
//! `under_attack`; the command mask is `guess::action_bits`, itself a rendition of
//! 0x4D1350.
use super::guess;
use crate::combat::TargetRef;
use crate::ids::Gtid;
use crate::scheduler::Owner;
use crate::world::{AbilityTarget, Command, KingdomId, Order, Unit, World};
use aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS as COMMANDS;
use glam::Vec2;
use std::collections::{BTreeMap, VecDeque};

/// 1.0 and 1e-4 (0x3F800000, 0x38D1B717): the group's reschedules.
const ONE_SECOND: f32 = 1.0;
const AT_ONCE: f32 = f32::from_bits(0x38d1_b717);
/// 0x9F3228: below this 0x409230 gives the zero vector.
const NO_DIRECTION: f32 = 1e-4;

/// `+188`, one 16-byte entry per member.
#[derive(Clone, Debug)]
pub struct Entry {
    pub unit: Unit,
    /// `+4`, `+8`: the place in the group's formation.
    offset: Vec2,
    /// `+C`, 0x4DDB60: the member's Move takes this place.
    in_formation: bool,
    /// `+D`, 0x4DF290: within the group's radius of its centre.
    near: bool,
}

/// `+1B8`. The constructor's 3 (0x4DE350) is only until the creation record's
/// vt+E4 = 0x4DF0C0 runs 0x4DEE40: a group starts idle.
const IDLE: u8 = 0;
const COMMANDED: u8 = 1;
const ATTACKED: u8 = 2;

/// One GGroup.
#[derive(Clone, Debug)]
pub struct Group {
    pub id: Gtid,
    kingdom: KingdomId,
    /// Thing +20, +24.
    position: Vec2,
    pub members: Vec<Entry>,
    /// `+194`: the first near member with the command (0x4DEDB0).
    leader: Option<Unit>,
    /// `+198`: the queued commands (command +3C; no film has one).
    queue: VecDeque<Request>,
    /// `+1A4`.
    command: Option<Request>,
    /// `+1A8`, `+1AC`.
    facing: Vec2,
    /// `+1B0`, the group formation of the members' common formation (0x4DEC30):
    /// its name and positions.
    formation: Option<(String, Vec<[f32; 2]>)>,
    /// `+1B4`, 0x4DECA0.
    radius: f32,
    state: u8,
    /// `+1BC`, the boolean of group_behavior_enable: members outside the radius
    /// are evened out too.
    together: bool,
    /// Zone of slot 7 (`+148[7]`): centre and radius; None is radius -1.
    zone: Option<(Vec2, f32)>,
}

/// A selection's settings (selection +24; +18 the behaviour, +1C its group, +20
/// the boolean). The original keeps them per player; here per kingdom.
#[derive(Clone, Copy, Debug, Default)]
pub struct Settings {
    pub on: bool,
    pub group: Option<u32>,
    pub together: bool,
}

/// The groups (`World::orders`).
#[derive(Clone, Debug, Default)]
pub struct Groups {
    pub live: BTreeMap<u32, Group>,
    next: u32,
    pub settings: BTreeMap<KingdomId, Settings>,
    /// A group is handing out a command: the member does not leave it (0x4ED360
    /// only takes a thing from its group for a command given to the thing).
    delivering: bool,
}

/// The fields of a generic command the group reads, and the command itself.
#[derive(Clone, Debug)]
pub struct Request {
    /// ActorCommand index, `+8` -> `+C`.
    kind: u32,
    kingdom: KingdomId,
    units: Vec<Unit>,
    /// Flag bit 0, `+10`.
    target: Option<Unit>,
    /// Flag bit 2, `+18`.
    position: Option<Vec2>,
    /// Flag bit 3, `+20`.
    aux: Option<Vec2>,
    command: Command,
}

impl Request {
    /// The command as 0x4ED360 sees it; None for one that names no selection.
    pub fn of(command: &Command) -> Option<Self> {
        use super::ActorOrder as A;
        let none = (None, None, None);
        let (kingdom, units, kind, (target, position, aux)) = match command {
            Command::Move { kingdom, units, to, attack_move } => (*kingdom, units, if *attack_move { 0 } else { 5 }, (None, Some(*to), None)),
            Command::Attack { kingdom, units, target } => (*kingdom, units, 0, (Some(Unit::Actor(*target)), None, None)),
            Command::Follow { kingdom, units, target } => (*kingdom, units, 5, (Some(Unit::Actor(*target)), None, None)),
            Command::Stop { kingdom, units } => (*kingdom, units, 6, none),
            Command::Disband { kingdom, units } => (*kingdom, units, 10, none),
            Command::Repair { kingdom, units, target } => (*kingdom, units, 3, (Some(Unit::Actor(*target)), None, None)),
            Command::Construct { kingdom, units, at, .. } => (*kingdom, units, 4, (None, Some(*at), None)),
            Command::Bombard { kingdom, units, target } => (
                *kingdom,
                units,
                1,
                match target {
                    AbilityTarget::Actor(t) => (Some(Unit::Actor(*t)), None, None),
                    AbilityTarget::Point(p) => (None, Some(*p), None),
                },
            ),
            Command::Order { kingdom, units, order } => {
                let (kind, fields) = match order {
                    A::Guard { target: Some(t), aux, .. } => (2, (Some(*t), None, *aux)),
                    A::Guard { target: None, at, aux } => (2, (None, Some(*at), *aux)),
                    A::Retreat { target: Some(t), .. } => (7, (Some(Unit::Actor(*t)), None, None)),
                    A::Retreat { target: None, at } => (7, (None, Some(*at), None)),
                    A::Rout => (8, none),
                    A::Explore { at, .. } => (9, (None, Some(*at), None)),
                    A::PackOrganization => (12, none),
                    A::Unpack { at, .. } => (11, (None, Some(*at), None)),
                    _ => return None,
                };
                (*kingdom, units, kind, fields)
            }
            _ => return None,
        };
        Some(Self { kind, kingdom, units: units.clone(), target, position, aux, command: command.clone() })
    }

    /// The same command to one thing, its position moved by `offset`.
    fn to_one(&self, unit: Unit, offset: Vec2) -> Command {
        let mut c = self.command.clone();
        match &mut c {
            Command::Move { units, to, .. } => {
                *units = vec![unit];
                *to += offset;
            }
            Command::Attack { units, .. } | Command::Follow { units, .. } | Command::Stop { units, .. } | Command::Disband { units, .. } | Command::Repair { units, .. } => {
                *units = vec![unit];
            }
            Command::Construct { units, at, .. } => {
                *units = vec![unit];
                *at += offset;
            }
            Command::Bombard { units, target, .. } => {
                *units = vec![unit];
                if let AbilityTarget::Point(p) = target {
                    *p += offset;
                }
            }
            Command::Order { units, order, .. } => {
                *units = vec![unit];
                use super::ActorOrder as A;
                match order {
                    A::Guard { target: None, at, .. } | A::Retreat { target: None, at } | A::Explore { at, .. } | A::Unpack { at, .. } => *at += offset,
                    _ => {}
                }
            }
            _ => {}
        }
        c
    }
    fn wants_position(&self) -> bool {
        COMMANDS.get(self.kind as usize).is_some_and(|d| d.behavior_flags & 0xf0 != 0)
    }
    fn wants_target(&self) -> bool {
        COMMANDS.get(self.kind as usize).is_some_and(|d| d.behavior_flags >> 8 & 0xff != 0)
    }
    /// `apply_group_offset` (+3D): false for kill, bombard and air_attack.
    fn applies_offset(&self) -> bool {
        !matches!(self.kind, 0 | 1 | 25)
    }
}

// ---- reading a thing ------------------------------------------------------

fn alive(world: &World, unit: Unit) -> bool {
    match unit {
        Unit::Org(o) => world.orgs.get(o as usize).is_some_and(|x| x.alive),
        Unit::Actor(a) => world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed),
    }
}
fn position(world: &World, unit: Unit) -> Option<Vec2> {
    super::guard::position(world, unit)
}
/// vt+150(kingdom, command, 1) through vt+154: the owner, then the command mask.
fn accepts(world: &World, kingdom: KingdomId, unit: Unit, kind: u32) -> bool {
    let owner = match unit {
        Unit::Org(o) => world.orgs.get(o as usize).filter(|x| x.alive).map(|x| x.kingdom),
        Unit::Actor(a) => world.actors.get(a as usize).filter(|x| x.alive && !x.removed).and_then(|x| x.kingdom),
    };
    owner == Some(kingdom) && has_command(world, unit, kind)
}
/// vt+38 -> vt+34: the command mask alone.
fn has_command(world: &World, unit: Unit, kind: u32) -> bool {
    guess::action_bits(world, unit) >> kind & 1 != 0
}
/// Actor state 12 mind_resting: nothing running in place of the sleep state.
fn resting(world: &World, unit: Unit) -> bool {
    match unit {
        Unit::Org(o) => {
            let Some(org) = world.orgs.get(o as usize) else { return true };
            let free = world.combat.org(o).is_none_or(|c| c.routing.is_none() && c.auto_order.is_none() && c.task.is_none() && !c.exhausted_state);
            org.order == Order::Idle && free && !world.orders.orgs.contains_key(&o)
        }
        Unit::Actor(a) => world.actors.get(a as usize).is_none_or(|x| x.order == Order::Idle),
    }
}
/// Actor state 15 mind_reforming.
fn reforming(world: &World, unit: Unit) -> bool {
    matches!(unit, Unit::Org(o) if world.combat.org(o).is_some_and(|c| c.reform.is_some()))
}
/// Actor state 4 mind_engaged or 20 under_siege, for 0x4DED00.
fn under_attack(world: &World, unit: Unit) -> bool {
    match unit {
        Unit::Org(o) => world.combat.org(o).is_some_and(|c| c.auto_order.is_some()),
        Unit::Actor(a) => crate::supply::under_siege(world, a),
    }
}
/// The speed a member's time is measured at: Organization +D8, or the
/// definition's +24.
fn top_speed(world: &mut World, unit: Unit) -> f32 {
    match unit {
        Unit::Org(o) => crate::movement::org_top_speed(world, o),
        Unit::Actor(a) => world.actors.get(a as usize).map_or(0.0, |x| world.data.def(x.def).speed_max),
    }
}

impl Group {
    /// 0x4DD590 over the living members: the mean, then the farthest thrown out
    /// while it is more than GroupZoneRangeMin from the mean. +20 when none lives
    /// (0x4DF1C0).
    fn center(&self, world: &World) -> Vec2 {
        let points: Vec<Vec2> = self.members.iter().filter_map(|m| position(world, m.unit)).collect();
        if points.is_empty() {
            return self.position;
        }
        robust_center(points, range_min(world))
    }
    /// 0x4DEC30: all members with an organization on one formation: its group
    /// formation. 0x4DECA0: the largest zone radius of them, at least
    /// GroupZoneRangeMin.
    fn read_members(&mut self, world: &World) {
        self.formation = self.common_formation(world);
        self.radius = self.largest_radius(world);
    }
    fn common_formation(&self, world: &World) -> Option<(String, Vec<[f32; 2]>)> {
        let mut seen: Option<(String, Vec<[f32; 2]>)> = None;
        let mut any = false;
        for m in &self.members {
            let Unit::Org(o) = m.unit else { continue };
            let Some(org) = world.orgs.get(o as usize).filter(|x| x.alive) else { continue };
            let formation = crate::player_orders::formation(world, o).and_then(|f| world.data.def(org.def).formations.get(f));
            let here = formation.filter(|f| !f.group_positions.is_empty()).map(|f| (f.id.clone(), f.group_positions.clone()));
            if any && seen.as_ref().map(|s| &s.0) != here.as_ref().map(|s| &s.0) {
                return None;
            }
            any = true;
            seen = here;
        }
        seen
    }
    fn largest_radius(&self, world: &World) -> f32 {
        let mut r = range_min(world);
        for m in &self.members {
            let Unit::Org(o) = m.unit else { continue };
            if let Some(org) = world.orgs.get(o as usize).filter(|x| x.alive) {
                let v = world.data.def(org.def).control_range;
                if r < v {
                    r = v;
                }
            }
        }
        r
    }
    /// 0x4DF290: `near` of each member (`within` false: all near).
    fn set_near(&mut self, world: &World, center: Vec2, within: bool) {
        let r2 = self.radius * self.radius;
        for m in &mut self.members {
            if let Some(p) = position(world, m.unit) {
                let d = p - center;
                m.near = d.y * d.y + d.x * d.x <= r2 || !within;
            }
        }
    }
    /// 0x4DF600: the places: the formation's own for as many as it has, the
    /// others where they stand (turned out of `facing`), less their mean.
    fn place(&mut self, world: &World, center: Vec2, facing: Vec2) {
        let count = self.members.len();
        let r = range_min(world);
        let positions = self.formation.as_ref().map_or(&[][..], |f| f.1.as_slice());
        let mut sum = Vec2::ZERO;
        for (i, m) in self.members.iter_mut().enumerate() {
            let Some(p) = position(world, m.unit) else { continue };
            let o = if i < positions.len() {
                Vec2::from(positions[i])
            } else {
                let mut l = p - center;
                if !m.near {
                    l = normalize(l) * r;
                }
                Vec2::new(l.x * facing.y - l.y * facing.x, l.x * facing.x + l.y * facing.y)
            };
            m.offset = o;
            sum += o;
        }
        let k = 1.0 / count as f32;
        let mean = sum * k;
        for m in &mut self.members {
            m.offset -= mean;
        }
    }
    /// 0x4DE020: the member's place at `point`: its offset turned by the facing,
    /// and back along the line to the point to the first tile in the zone.
    fn offset_at(&self, world: &World, entry: &Entry, point: Vec2) -> Vec2 {
        let (o, f) = (entry.offset, self.facing);
        let turned = Vec2::new(f.y * o.x + f.x * o.y, o.y * f.y - f.x * o.x);
        let Some(zone) = self.zone else { return turned };
        let at = clamp_525d0(world, point + turned);
        if in_zone(world, zone, at) {
            return turned;
        }
        // 0x4DD030 / 0x4DDDE0: the tile walk to the point.
        let cell = world.map.cell_size;
        let tile = |v: Vec2| ((v.x / cell).floor() as i32, (v.y / cell).floor() as i32);
        let (mut x, mut y) = tile(at);
        let (ex, ey) = tile(point);
        let mut last = at;
        while (x, y) != (ex, ey) {
            if x != ex {
                x += (ex - x).signum();
            }
            if y != ey {
                y += (ey - y).signum();
            }
            last = Vec2::new(x as f32 * cell + cell * 0.5, y as f32 * cell + cell * 0.5);
            if in_zone(world, zone, last) {
                break;
            }
        }
        last - point
    }
    fn entry(&self, unit: Unit) -> Option<&Entry> {
        self.members.iter().find(|m| m.unit == unit)
    }
    /// 0x4DDB60: whether a member's Move takes its place in the formation.
    fn formation_flag(&self, world: &World, unit: Unit, request: &Request) -> bool {
        if !alive(world, unit) {
            return true;
        }
        if self.state != COMMANDED {
            return false;
        }
        if request.kind == 5 {
            let Some(target) = request.target else { return true };
            let Some(rep) = crate::ids::target_actor(world, target) else { return true };
            if !super::hostile_to(world, self.kingdom, rep) {
                return true;
            }
        }
        if !accepts(world, self.kingdom, unit, request.kind) {
            return true;
        }
        // 0x4DCEB0: bombard, or the kill of a layout firing indirect.
        let bombards = request.kind == 1 || (request.kind == 0 && matches!(unit, Unit::Org(o) if guess::bombard_layout(world, o)));
        if request.position.is_some() && !bombards {
            return true;
        }
        if let Some(zone) = self.zone {
            let inside = position(world, unit).is_some_and(|p| in_zone(world, zone, p));
            if !inside && (request.target.is_some() || bombards) {
                return true;
            }
        }
        false
    }
}

/// `[+18C]` GroupZoneRangeMin.
fn range_min(world: &World) -> f32 {
    world.data.rule_ci("GroupZoneRangeMin").unwrap_or(0.0)
}
/// 0x409230.
fn normalize(v: Vec2) -> Vec2 {
    let l = (v.y as f64 * v.y as f64 + v.x as f64 * v.x as f64).sqrt();
    if l > NO_DIRECTION as f64 {
        let k = 1.0 / l;
        Vec2::new((k * v.x as f64) as f32, (k * v.y as f64) as f32)
    } else {
        Vec2::ZERO
    }
}
/// 0x5525D0: 0 to the far edge less 0xA3AA10.
fn clamp_525d0(world: &World, p: Vec2) -> Vec2 {
    crate::movement::clamp_extent(world, p)
}
/// 0x552640: one tile in from the near edge, one tile and 0xA3AA10 from the far.
fn clamp_552640(world: &World, p: Vec2) -> Vec2 {
    let cell = world.map.cell_size;
    let far = |tiles: i32| tiles as f32 * cell - cell - 1e-4;
    let one = |v: f32, far: f32| if v < cell { cell } else if far < v { far } else { v };
    Vec2::new(one(p.x, far(world.map.width as i32)), one(p.y, far(world.map.height as i32)))
}
/// 0x4DD590.
fn robust_center(mut points: Vec<Vec2>, range: f32) -> Vec2 {
    let n = points.len();
    let mut c = points.iter().copied().fold(Vec2::ZERO, |a, b| a + b) * (1.0 / n as f32);
    let r2 = range * range;
    loop {
        let (mut far, mut at) = (0.0f32, None);
        for (i, p) in points.iter().enumerate() {
            let d = *p - c;
            let v = d.x * d.x + d.y * d.y;
            if far <= v {
                far = v;
                at = Some(i);
            }
        }
        if far <= r2 {
            return c;
        }
        let Some(i) = at else { return c };
        let k = points.len() as f32;
        let p = points.remove(i);
        let inverse = 1.0 / points.len() as f32;
        c = Vec2::new((c.x * k - p.x) * inverse, inverse * (k * c.y - p.y));
    }
}
/// 0x4F2420 on the group's zone, as a circle over open tiles (a rendition of the
/// flood 0x540860).
fn in_zone(world: &World, zone: (Vec2, f32), p: Vec2) -> bool {
    p.distance_squared(zone.0) <= zone.1 * zone.1 && world.map.cell_flags(p) & (crate::map::BLOCKED | crate::map::NOT_LAND_PASSABLE) == 0
}

// ---- the group ------------------------------------------------------------

/// `f` on the group with the world to read from.
fn with_group<R>(world: &mut World, key: u32, f: impl FnOnce(&mut Group, &World) -> R) -> Option<R> {
    let mut g = world.orders.groups.live.remove(&key)?;
    let r = f(&mut g, world);
    world.orders.groups.live.insert(key, g);
    Some(r)
}

fn schedule(world: &mut World, key: u32, delay: f32) {
    // vt+4 = 0x50AD80: vt+10 0x529CD0 cancels what is pending (0x50B250).
    world.unschedule(Owner::Group(key));
    world.schedule_after(crate::scheduler::DEFERRED, Owner::Group(key), 0, delay);
}

/// 0x4DFF30: `unit` leaves its group; the last one out destroys it (0x4F6760).
pub(super) fn leave(world: &mut World, unit: Unit) {
    let Some(key) = world.orders.groups.live.iter().find(|(_, g)| g.entry(unit).is_some()).map(|(k, _)| *k) else { return };
    let (empty, idle) = {
        let g = world.orders.groups.live.get_mut(&key).expect("found");
        g.members.retain(|m| m.unit != unit);
        if g.leader == Some(unit) {
            g.leader = None;
        }
        (g.members.is_empty(), g.state == IDLE)
    };
    if empty {
        destroy(world, key);
    } else if idle {
        let g = &world.orders.groups.live[&key];
        let at = g.center(world);
        world.orders.groups.live.get_mut(&key).expect("found").position = at;
    }
}
fn destroy(world: &mut World, key: u32) {
    if let Some(g) = world.orders.groups.live.remove(&key) {
        world.ids.removed_group(g.id);
        world.unschedule(Owner::Group(key));
        for s in world.orders.groups.settings.values_mut() {
            if s.group == Some(key) {
                s.group = None;
            }
        }
    }
}

/// 0x4FBE10, 0x4E0940 and 0x4E2940: the selection becomes a new group; the
/// facing of the group it had goes to the new one. The id and its scale draw
/// (0x49B440); no other draw shows in the Berlin film.
fn form(world: &mut World, kingdom: KingdomId, units: &[Unit]) -> Option<u32> {
    let settings = world.orders.groups.settings.get(&kingdom).copied().unwrap_or_default();
    world.orders.groups.next += 1;
    let key = world.orders.groups.next;
    let id = world.ids.created_group(key)?;
    world.random_u32_at(crate::trace::sites::RANDOM_SCALE);
    let facing = settings.group.and_then(|k| world.orders.groups.live.get(&k)).map_or(Vec2::new(0.0, -1.0), |g| g.facing);
    let mut members = Vec::with_capacity(units.len());
    for &u in units {
        if alive(world, u) {
            leave(world, u);
            members.push(Entry { unit: u, offset: Vec2::ZERO, in_formation: false, near: true });
        }
    }
    let mut g = Group {
        id,
        kingdom,
        position: Vec2::ZERO,
        members,
        leader: None,
        queue: VecDeque::new(),
        command: None,
        facing,
        formation: None,
        radius: range_min(world),
        state: IDLE,
        together: settings.together,
        zone: None,
    };
    g.position = g.center(world);
    g.read_members(world);
    world.orders.groups.live.insert(key, g);
    world.orders.groups.settings.entry(kingdom).or_default().group = Some(key);
    schedule(world, key, AT_ONCE);
    Some(key)
}

/// The group thing alone, for a command that names no selection here.
pub(super) fn formed(world: &mut World, kingdom: KingdomId, units: &[Unit]) {
    form(world, kingdom, units);
}

/// 0x4DF840.
fn run(world: &mut World, key: u32, request: Request) -> bool {
    let (center, had_command) = {
        let g = &world.orders.groups.live[&key];
        (if g.command.is_some() { g.center(world) } else { g.position }, g.command.is_some())
    };
    let same_command = with_group(world, key, |g, w| {
        g.set_near(w, center, true);
        g.command.as_ref().is_some_and(|c| c.kind == request.kind && c.target == request.target && c.position == request.position)
    })
    .unwrap_or(false);
    // 0x4DEDB0.
    let leader_ok = {
        let g = &world.orders.groups.live[&key];
        had_command && g.leader.is_some_and(|l| alive(world, l))
    };
    if !leader_ok {
        let g = &world.orders.groups.live[&key];
        let leader = g.members.iter().find(|m| m.near && alive(world, m.unit) && has_command(world, m.unit, request.kind)).map(|m| m.unit);
        world.orders.groups.live.get_mut(&key).expect("live").leader = leader;
        if leader.is_none() {
            let g = world.orders.groups.live.get_mut(&key).expect("live");
            if g.zone.is_some() {
                g.zone = Some((center, g.radius));
            }
            g.command = None;
            finish(world, key, center, true);
            return false;
        }
    }
    let wants = request.wants_position() || request.wants_target();
    let facing_used;
    {
        let g = world.orders.groups.live.get_mut(&key).expect("live");
        g.command = Some(request.clone());
        g.state = COMMANDED;
    }
    if wants {
        // 0x4DDCD0.
        let destination = match request.position {
            Some(p) => Some(clamp_552640(world, p)),
            None => request.target.and_then(|t| position(world, t)).map(|p| clamp_552640(world, p)),
        };
        let Some(destination) = destination else {
            let g = world.orders.groups.live.get_mut(&key).expect("live");
            g.command = None;
            finish(world, key, center, true);
            return false;
        };
        let direction = normalize(destination - center);
        let g = world.orders.groups.live.get_mut(&key).expect("live");
        g.zone = Some((destination, g.radius));
        g.facing = match request.aux {
            Some(a) if a != Vec2::ZERO => a,
            _ => direction,
        };
        facing_used = direction;
    } else {
        facing_used = world.orders.groups.live[&key].facing;
    }
    if !(wants && same_command && had_command) {
        with_group(world, key, |g, w| g.place(w, center, facing_used));
    }
    hand_out(world, key, &request);
    if world.orders.groups.live.contains_key(&key) {
        schedule(world, key, ONE_SECOND);
    }
    true
}

/// 0x4DF100: 0x4DE7F0 for each member still in the group.
fn hand_out(world: &mut World, key: u32, request: &Request) {
    let units: Vec<Unit> = world.orders.groups.live.get(&key).map_or(Vec::new(), |g| g.members.iter().map(|m| m.unit).collect());
    for u in units {
        if world.orders.groups.live.get(&key).is_some_and(|g| g.entry(u).is_some()) {
            give(world, key, u, request);
        }
    }
}

/// 0x4DE7F0: the command if the member takes it (the flag first: its Move reads
/// it), else a Move for a command that wants a place.
fn give(world: &mut World, key: u32, unit: Unit, request: &Request) {
    let Some(g) = world.orders.groups.live.get(&key) else { return };
    let kingdom = g.kingdom;
    if !alive(world, unit) {
        return;
    }
    let (kind, command) = if accepts(world, kingdom, unit, request.kind) {
        (request.kind, request.to_one(unit, Vec2::ZERO))
    } else if request.wants_position() || request.wants_target() {
        // 0x4CA760 (the position) or 0x4DDE60 (the target): a Move of type 5.
        let command = match (request.position, request.target.and_then(|t| crate::ids::target_actor(world, t))) {
            (Some(to), _) => Command::Move { kingdom, units: vec![unit], to, attack_move: false },
            (None, Some(target)) => Command::Follow { kingdom, units: vec![unit], target },
            (None, None) => return,
        };
        if !accepts(world, kingdom, unit, 5) {
            return;
        }
        (5, command)
    } else {
        return;
    };
    set_flag(world, key, unit, kind, request);
    deliver(world, command, request.aux);
}
fn set_flag(world: &mut World, key: u32, unit: Unit, kind: u32, request: &Request) {
    let flag = {
        let g = &world.orders.groups.live[&key];
        let mut asked = request.clone();
        asked.kind = kind;
        g.formation_flag(world, unit, &asked)
    };
    if let Some(e) = world.orders.groups.live.get_mut(&key).and_then(|g| g.members.iter_mut().find(|m| m.unit == unit)) {
        e.in_formation = flag;
    }
}

/// 0x4D9B60 with the Mind handler of a kill or a Move at a hostile thing
/// (0x527FB0, 0x527E30): the bombard state of an indirect layout, the kill state,
/// or a Move at a thing that is no enemy.
fn deliver(world: &mut World, command: Command, aux: Option<Vec2>) {
    let command = member_handler(world, command);
    world.orders.groups.delivering = true;
    world.orders.next_aux = aux;
    world.apply(command);
    world.orders.groups.delivering = false;
}
fn member_handler(world: &World, command: Command) -> Command {
    // 0x527FB0 without a target: the bombard of a location (0x527C20).
    if let Command::Move { kingdom, units, to, attack_move: true } = &command
        && matches!(units[0], Unit::Org(o) if guess::bombard_layout(world, o))
    {
        return Command::Bombard { kingdom: *kingdom, units: units.clone(), target: AbilityTarget::Point(*to) };
    }
    if let Command::Attack { kingdom, units, target } = &command {
        let (kingdom, target) = (*kingdom, *target);
        let unit = units[0];
        let hostile = super::hostile_to(world, kingdom, target);
        if !hostile {
            return Command::Follow { kingdom, units: units.clone(), target };
        }
        if matches!(unit, Unit::Org(o) if guess::bombard_layout(world, o)) {
            return Command::Bombard { kingdom, units: units.clone(), target: AbilityTarget::Actor(target) };
        }
    }
    command
}

/// 0x4DEE40: the group rests at `at`; with `reform` and nothing queued the members
/// take their places there (0x4DEA20).
fn finish(world: &mut World, key: u32, at: Vec2, reform: bool) {
    let queued = {
        let g = world.orders.groups.live.get_mut(&key).expect("live");
        g.state = IDLE;
        g.position = at;
        !g.queue.is_empty()
    };
    if !queued && reform {
        take_places(world, key, at);
    }
    let g = world.orders.groups.live.get_mut(&key).expect("live");
    for m in &mut g.members {
        m.in_formation = false;
    }
    g.zone = None;
    schedule(world, key, AT_ONCE);
}

/// 0x4DEA20: a Move of each member to its place at `at`.
fn take_places(world: &mut World, key: u32, at: Vec2) {
    let (kingdom, moves): (KingdomId, Vec<(Unit, Vec2)>) = {
        let g = &world.orders.groups.live[&key];
        (g.kingdom, g.members.iter().filter(|m| alive(world, m.unit)).map(|m| (m.unit, at + g.offset_at(world, m, at))).collect())
    };
    for (u, to) in moves {
        if accepts(world, kingdom, u, 5) {
            deliver(world, Command::Move { kingdom, units: vec![u], to, attack_move: false }, None);
        }
    }
}

/// 0x4DEB80: a near member is fighting something other than what it was told to
/// kill, or a near structure is under siege.
fn attacked(world: &World, g: &Group) -> bool {
    for m in &g.members {
        if !m.near || !alive(world, m.unit) {
            continue;
        }
        match m.unit {
            Unit::Org(o) => {
                let Some(c) = world.combat.org(o) else { continue };
                if c.auto_order.is_some() || c.engaging() {
                    // A kill target on the engage list is what it was told.
                    let told = match c.explicit {
                        Some(TargetRef::Org(t)) => c.engage.iter().any(|e| e.target == TargetRef::Org(t)),
                        Some(TargetRef::Actor(t)) => c.engage.iter().any(|e| e.target == TargetRef::Actor(t)),
                        None => false,
                    };
                    return !told;
                }
            }
            Unit::Actor(a) => {
                if crate::supply::under_siege(world, a) {
                    return true;
                }
            }
        }
    }
    false
}

/// 0x4E0880, the deferred delivery of the Updatable.
pub fn update(world: &mut World, key: u32) {
    let Some(g) = world.orders.groups.live.get_mut(&key) else { return };
    if g.members.is_empty() {
        destroy(world, key);
        return;
    }
    let radius = {
        let g = &world.orders.groups.live[&key];
        g.largest_radius(world)
    };
    {
        let g = world.orders.groups.live.get_mut(&key).expect("live");
        g.radius = radius;
        if let Some(z) = &mut g.zone {
            z.1 = radius;
        }
    }
    formation_changed(world, key);
    let Some(state) = world.orders.groups.live.get(&key).map(|g| g.state) else { return };
    match state {
        IDLE => update_idle(world, key),
        COMMANDED => update_commanded(world, key),
        ATTACKED => update_attacked(world, key),
        _ => {}
    }
}

/// 0x4DFD80: the members' formation changed.
fn formation_changed(world: &mut World, key: u32) {
    let (changed, center, facing, state, command) = {
        let g = &world.orders.groups.live[&key];
        let new = g.common_formation(world);
        let changed = new.as_ref().map(|f| &f.0) != g.formation.as_ref().map(|f| &f.0);
        (changed.then_some(new), g.center(world), g.facing, g.state, g.command.clone())
    };
    let Some(new) = changed else { return };
    let some = new.is_some();
    world.orders.groups.live.get_mut(&key).expect("live").formation = new;
    if !some {
        return;
    }
    with_group(world, key, |g, w| g.place(w, center, facing));
    if state == IDLE {
        take_places(world, key, center);
    } else if state == COMMANDED {
        match command.filter(|c| c.wants_position() || c.wants_target()) {
            Some(c) => {
                run(world, key, c);
            }
            None => take_places(world, key, center),
        }
    }
}

/// 0x4DFFF0.
fn update_idle(world: &mut World, key: u32) {
    let (fight, center, facing) = {
        let g = &world.orders.groups.live[&key];
        (attacked(world, g), g.center(world), g.facing)
    };
    if fight {
        with_group(world, key, |g, w| {
            g.place(w, center, facing);
            g.state = ATTACKED;
            g.set_near(w, center, true);
        });
        schedule(world, key, AT_ONCE);
        return;
    }
    let g = world.orders.groups.live.get_mut(&key).expect("live");
    if let Some(next) = g.queue.pop_front() {
        run(world, key, next);
        return;
    }
    schedule(world, key, ONE_SECOND);
}

/// 0x4E0330.
fn update_commanded(world: &mut World, key: u32) {
    let (fight, center) = {
        let g = &world.orders.groups.live[&key];
        (attacked(world, g), g.center(world))
    };
    if fight {
        with_group(world, key, |g, w| {
            g.state = ATTACKED;
            g.set_near(w, center, true);
        });
        schedule(world, key, AT_ONCE);
        return;
    }
    let g = &world.orders.groups.live[&key];
    let leader_gone = g.leader.is_none_or(|l| !alive(world, l));
    let Some(command) = g.command.clone() else {
        finish(world, key, center, true);
        return;
    };
    if leader_gone {
        run(world, key, command);
        return;
    }
    let busy = g.members.iter().any(|m| alive(world, m.unit) && !reforming(world, m.unit) && !resting(world, m.unit));
    if !busy {
        // Every member rests: the command is over. 0x4DDCD0, a stop rests where it
        // stands.
        let at = match command.position {
            Some(p) => clamp_552640(world, p),
            None => command.target.and_then(|t| position(world, t)).map_or(center, |p| clamp_552640(world, p)),
        };
        let g = world.orders.groups.live.get_mut(&key).expect("live");
        g.command = None;
        finish(world, key, at, command.kind != 6);
        return;
    }
    // 0x4E0330's target: the zone goes with it, and an enemy target (vt+140) has
    // its guard zone (slot 5) and GroupSupplyZoneBuffer kept off.
    let mut buffer = 0.0f32;
    if let Some(t) = command.target {
        let target_position = position(world, t);
        let g = world.orders.groups.live.get(&key).expect("live");
        let kingdom = g.kingdom;
        let radius = g.radius;
        if let Some(rep) = crate::ids::target_actor(world, t)
            && super::hostile_to(world, kingdom, rep)
        {
            if let Some(denizen) = world.data.def(world.actors[rep as usize].def).denizen.as_ref() {
                buffer = denizen.guard_range + world.data.rule_ci("GroupSupplyZoneBuffer").unwrap_or(0.0);
            }
        }
        if let Some(p) = target_position {
            let at = clamp_552640(world, p);
            world.orders.groups.live.get_mut(&key).expect("live").zone = Some((at, radius));
        }
    }
    even_speeds(world, key, &command, buffer);
    schedule(world, key, ONE_SECOND);
}

/// The loop of 0x4E0330 from 0x4E0610: each moving member's time to its place; the
/// slowest sets the others' speed (Mind vt+88 = 0x5205E0). A member whose flag
/// changed plans again (Mind vt+48 = 0x524870).
fn even_speeds(world: &mut World, key: u32, command: &Request, buffer: f32) {
    let units: Vec<Unit> = world.orders.groups.live[&key].members.iter().map(|m| m.unit).collect();
    let mut times: Vec<(Unit, f32)> = Vec::new();
    let mut longest = 0.0f32;
    for u in units {
        let Unit::Org(o) = u else { continue };
        let (flag, same, counted, offset, facing, aux) = {
            let g = &world.orders.groups.live[&key];
            let Some(e) = g.entry(u) else { continue };
            let flag = g.formation_flag(world, u, command);
            (flag, e.in_formation == flag, g.together || e.near, e.offset, g.facing, command.aux)
        };
        if !same {
            if let Some(e) = world.orders.groups.live.get_mut(&key).and_then(|g| g.members.iter_mut().find(|m| m.unit == u)) {
                e.in_formation = flag;
            }
            crate::movement::org_replan(world, o);
            continue;
        }
        if !counted || !crate::movement::org_move_active(world, o) {
            continue;
        }
        let mut left = crate::movement::org_path_left(world, o);
        if buffer > 0.0 {
            let forward = match aux {
                Some(a) if a != Vec2::ZERO => offset.y * facing.y - offset.x * facing.x,
                _ => offset.y,
            };
            left -= forward + buffer;
            if left < 0.0 {
                left = 0.0;
            }
        }
        let time = left / top_speed(world, u);
        times.push((u, time));
        if longest < time {
            longest = time;
        }
    }
    if longest > 0.0 {
        for (u, time) in times {
            if let Unit::Org(o) = u {
                let speed = top_speed(world, u) * (time / longest);
                crate::movement::org_set_speed(world, o, speed);
            }
        }
    }
}

/// 0x4E00A0.
fn update_attacked(world: &mut World, key: u32) {
    let units: Vec<Unit> = world.orders.groups.live[&key].members.iter().filter(|m| m.near).map(|m| m.unit).collect();
    let mut helping = true;
    for u in units {
        if !alive(world, u) || under_attack(world, u) {
            continue;
        }
        let kingdom = world.orders.groups.live[&key].kingdom;
        let mut buddy = None;
        if helping {
            // 0x4DED00: the last member that is fighting.
            buddy = world.orders.groups.live[&key].members.iter().rev().find(|m| m.unit != u && alive(world, m.unit) && under_attack(world, m.unit)).map(|m| m.unit);
            if buddy.is_none() {
                helping = false;
            }
        }
        let request = match buddy {
            Some(b) if helping && accepts(world, kingdom, u, 2) => {
                let command = Command::Order { kingdom, units: vec![u], order: super::ActorOrder::Guard { target: Some(b), at: Vec2::ZERO, aux: None } };
                Request::of(&command)
            }
            _ => Request::of(&Command::Stop { kingdom, units: vec![u] }),
        };
        if let Some(r) = request {
            give(world, key, u, &r);
        }
        if !world.orders.groups.live.contains_key(&key) {
            return;
        }
    }
    if !helping {
        let g = &world.orders.groups.live[&key];
        match g.command.clone() {
            Some(c) => {
                run(world, key, c);
            }
            None => {
                let at = g.position;
                finish(world, key, at, true);
            }
        }
        return;
    }
    schedule(world, key, ONE_SECOND);
}

// ---- what the rest of the game asks ---------------------------------------

/// 0x4ED360: a command with `units` on its way to the systems. A command to more
/// than one thing goes to a new group with the behaviour on, or to each thing with
/// its offset; the result is None then. Otherwise it goes on, and a thing given a
/// command of its own leaves its group (0x4DFF30, not for set_formation).
pub fn selection(world: &mut World, command: Command) -> Option<Command> {
    if world.orders.groups.delivering {
        return Some(command);
    }
    let Some(mut request) = Request::of(&command) else { return Some(command) };
    if request.aux.is_none() {
        request.aux = world.orders.next_aux;
    }
    let selected: Vec<Unit> = request.units.iter().copied().filter(|&u| alive(world, u)).collect();
    if selected.len() < 2 {
        if !world.orders.groups.live.is_empty() {
            for &u in &request.units {
                if accepts(world, request.kingdom, u, request.kind) {
                    leave(world, u);
                }
            }
        }
        return Some(command);
    }
    let settings = world.orders.groups.settings.get(&request.kingdom).copied().unwrap_or_default();
    if settings.on {
        if let Some(key) = form(world, request.kingdom, &selected) {
            run(world, key, request);
        }
        return None;
    }
    // The lower path: the centre of the selection, each thing's position moved
    // by where it stands (0x4DD860: within the range as it is, else on its edge).
    let center = robust_center(selected.iter().filter_map(|&u| position(world, u)).collect(), range_min(world));
    let range = range_min(world);
    let offsets = request.position.is_some() && request.applies_offset();
    for u in selected {
        let offset = match position(world, u).filter(|_| offsets) {
            Some(p) => {
                let d = center - p;
                let near = d.x * d.x + d.y * d.y <= range * range;
                let l = p - center;
                if near { l } else { normalize(l) * range }
            }
            None => Vec2::ZERO,
        };
        if accepts(world, request.kingdom, u, request.kind) {
            leave(world, u);
        }
        world.orders.next_aux = request.aux;
        world.apply(request.to_one(u, offset));
    }
    None
}

/// 0x51D840: a member's offset at `point` while its group holds it in formation
/// (0x4DDEE0, then 0x4DE6C0), else zero.
pub fn member_offset(world: &World, unit: Unit, point: Vec2) -> Vec2 {
    for g in world.orders.groups.live.values() {
        if let Some(e) = g.entry(unit) {
            if g.state == COMMANDED && e.in_formation {
                return g.offset_at(world, e, point);
            }
            break;
        }
    }
    Vec2::ZERO
}
/// 0x51D7F0: actor +164 valid and 0x4DDEE0.
pub fn in_formation(world: &World, unit: Unit) -> bool {
    world.orders.groups.live.values().find_map(|g| g.entry(unit).map(|e| g.state == COMMANDED && e.in_formation)).unwrap_or(false)
}
/// 0x44DEA0 on actor +164: the thing belongs to a group.
pub fn in_group(world: &World, unit: Unit) -> bool {
    world.orders.groups.live.values().any(|g| g.entry(unit).is_some())
}

/// 0x4D3900 / 0x4DFF30: a member that died or entered a state with
/// `remove_from_group` (routing, exhausted) leaves its group.
pub(super) fn tick(world: &mut World) {
    if world.orders.groups.live.is_empty() {
        return;
    }
    let mut gone = Vec::new();
    for g in world.orders.groups.live.values() {
        for m in &g.members {
            let out = match m.unit {
                Unit::Org(o) => {
                    let org = world.orgs.get(o as usize);
                    let state = world.combat.org(o);
                    !org.is_some_and(|x| x.alive) || state.is_some_and(|c| c.routing.is_some() || c.exhausted_state)
                }
                Unit::Actor(a) => !world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed),
            };
            if out {
                gone.push(m.unit);
            }
        }
    }
    for m in gone {
        leave(world, m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_center_throws_out_the_farthest() {
        let c = robust_center(vec![Vec2::new(0.0, 0.0), Vec2::new(2.0, 0.0), Vec2::new(100.0, 0.0)], 18.0);
        assert_eq!(c, Vec2::new(1.0, 0.0));
    }
    #[test]
    fn the_center_of_a_tight_group_is_the_mean() {
        let c = robust_center(vec![Vec2::new(0.0, 0.0), Vec2::new(4.0, 0.0)], 18.0);
        assert_eq!(c, Vec2::new(2.0, 0.0));
    }
}
