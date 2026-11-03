//! A transport that flies in: the TransportComponent (actor +0xC0, vtable 0xA40100), the
//! AircraftComponent's flight under it and the fall of what it drops. Owner: core.
//!
//! The native flow of a recruit at a location, `0x582190` from `0x589F90`:
//! 1. The structure's own aircraft comes first when its Denizen has one of the kind
//!    (`0x568120`, `0x56DC50`); else the plane enters where the line from the structure
//!    through the point crosses the map's rectangle (`0x53A820`, clamped by `0x552640`), at
//!    its operational height, facing the point ([`recruit`]). `0x4F9C10` makes it, its
//!    Mover registers (`0x55E600` -> `0x576B20`) and its Mind stands in `aai_queued`.
//! 2. `0x58BA90` loads the Transport (cargo +0x28, layout +0x2C, point +0x30, count +0x3C,
//!    the way out +0x50 with +0x4C set); the Move command `0x4CA760` goes to the Mind,
//!    which cannot leave the queued state, so the queued Think `0x519F70` releases it next:
//!    flight +0x8C the Mover's maximum, +0x18/+0x80 the height, on_ground clear, and
//!    `0x518550` activates `aai_transport` (`crate::combat::air`).
//! 3. `0x5179E0` aims at the point less the lead `0x58B180` + (count - 1) / 2 deployments
//!    (ActorData +0x41C) at half speed. At the path end (`0x5179C0`) the state's next
//!    Think runs `0x517B60`: half speed, [`drop_one`] (`0x58BCB0`), and while members are
//!    left the next point. The Think comes every 0.6 s, so a member drops 9 or 10 ticks
//!    after the last.
//! 4. The hold empty, `0x5185D0` flies the way out; `0x516D10` destroys the plane within
//!    0.5 m of it (`0x58B130`).
//!
//! The flight is `aa_movement::aircraft` and `aircraft_legs`, the fall
//! `aa_movement::falling`: exact ports, here given their world. Only a transport's Mover is
//! on the queue: **an air base's aircraft do not fly yet** ([`crate::movement::fly_to`]
//! keeps its no-op for them, NEXT-ROUND's aircraft list).
//!
//! **Renditions:** the structure's own aircraft (`0x56DC50`) needs the runway flight, so
//! that branch flies in from the edge too. `0x554850`'s search around a landing point that
//! is taken is not ported: the point stands. The Transport's name +0x58 and the +0x5C
//! attachment have no film yet.
use crate::data::{DefClass, DefId, NO_DEF};
use crate::scheduler::{EVERY_TICK, Owner};
use crate::trace::sites;
use crate::world::{ActorId, Event, KingdomId, OrgId, World};
use aa_movement::aircraft::{self, AbilityKey, ActorKey, AircraftActor, AircraftDefinition, AircraftError, AircraftResult, AircraftState, AircraftUnit, AircraftWorld, AttackDescriptor, FlightWorld, FormationWorld, GroupKey, GroupLayout, GroupWorld, HandleSlot, Pose, TerrainWorld};
use aa_movement::falling::{self, FallContext, FallingState, FallingWorld, MapBounds};
use aa_movement::mover::MoverLimits;
use aa_movement::{ActorMotion, MoverState};
use aa_numeric::{Binary, Precision};
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

const PC: Precision = Precision::Double53;
/// `[9FBDB8]`: `0x55D990`, the part of its speed a transport drops at.
const DROP_SPEED: f32 = 0.5;
/// `[A033BC]`: `0x52A1F0`, squared metres from the way out.
const OUT_SQUARED: f32 = 0.25;

/// The TransportComponent while it holds a cargo.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transport {
    /// +0x28.
    pub cargo: DefId,
    /// +0x2C, the layout: the cargo is an organization.
    pub organization: bool,
    /// +0x30/+0x34, the point.
    pub point: [u32; 2],
    /// +0x3C.
    pub count: i32,
    /// +0x40.
    pub dropped: i32,
    /// +0x44, the organization made by the first drop.
    pub org: Option<OrgId>,
    /// +0x48, the next layout index.
    pub next: usize,
    /// +0x50/+0x54 with +0x4C set: the way out.
    pub out: [u32; 2],
    /// Actor +0x2C..+0x34 as the flight left it.
    pub facing: [f32; 3],
    /// Mover +0x20.
    pub speed: f32,
    /// `aai_transport` +0x28/+0x2C: the direction.
    pub direction: [f32; 2],
    /// `aai_transport` +0x30/+0x34: the point it flies to now.
    pub at: [f32; 2],
    /// A summon (`0x4CB110`): whether the player commands what lands, and the property it
    /// takes.
    #[serde(default)]
    pub summon: Option<(bool, Option<String>)>,
}

/// A Mover falling: Mover +0x20, +0x56 and +0x58..+0x88.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fall {
    pub speed: f32,
    pub state: FallingState,
    pub facing: [f32; 3],
}

fn world_error(what: impl std::fmt::Display) -> AircraftError {
    AircraftError::Movement(aa_movement::Error::World(what.to_string()))
}

fn map_bounds(world: &World) -> MapBounds {
    let c = world.map.cell_size;
    MapBounds { cell_size: c, extent: [c * world.map.width as f32, c * world.map.height as f32] }
}

/// `0x53A3A0`: `from -> to` against the segment `a -> b`, the parameter along `from -> to`.
fn crossing(from: Vec2, to: Vec2, a: Vec2, b: Vec2) -> Option<f32> {
    let d = (to.x - from.x) * (b.y - a.y) - (b.x - a.x) * (to.y - from.y);
    if d == 0.0 {
        return None;
    }
    let inverse = 1.0 / d;
    let t = ((from.y - a.y) * (b.x - a.x) - (from.x - a.x) * (b.y - a.y)) * inverse;
    let u = ((from.y - a.y) * (to.x - from.x) - (from.x - a.x) * (to.y - from.y)) * inverse;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some(t)
}

/// `0x53A820` over the map's rectangle, the four sides in the native order; the point of
/// `(1 - t) * from + t * to`.
fn map_crossing(world: &World, from: Vec2, to: Vec2) -> Option<Vec2> {
    let MapBounds { extent: [w, h], .. } = map_bounds(world);
    let corners = [Vec2::ZERO, Vec2::new(w, 0.0), Vec2::new(w, h), Vec2::new(0.0, h)];
    let t = (0..4).find_map(|i| crossing(from, to, corners[i], corners[(i + 1) % 4]))?;
    let r = 1.0 - t;
    Some(Vec2::new(t * to.x + r * from.x, r * from.y + t * to.y))
}

/// `0x582190`'s plane: `false` for a thing with no transport, which lands where it is
/// recruited (`crate::production::land_recruit`).
pub(crate) fn recruit(world: &mut World, kingdom: KingdomId, cargo: DefId, structure: Vec2, point: Vec2) -> bool {
    let Some(plane) = world.data.def(cargo).transport_actor.clone().and_then(|t| world.data.find(&t)) else { return false };
    if world.map.width == 0 {
        return false;
    }
    let bounds = map_bounds(world);
    let longest = if bounds.extent[1] < bounds.extent[0] { bounds.extent[0] } else { bounds.extent[1] };
    let far = (structure - point) * longest;
    let Some(enter) = map_crossing(world, far, point) else { return false };
    let mut xy = [enter.x, enter.y];
    if falling::clamp_to_interior(bounds, &mut xy, PC).is_err() {
        return false;
    }
    let enter = Vec2::new(xy[0], xy[1]);
    let d = (point - enter).normalize_or_zero();
    fly_in(world, kingdom, plane, cargo, enter, d, point, None);
    true
}

/// `0x4CB110`: a summon's plane enters at the farther map edge (`transport_entry`), facing
/// along the axis, and flies to the point as a recruit's does.
pub(crate) fn summon(world: &mut World, kingdom: KingdomId, plane: DefId, cargo: DefId, point: Vec2, controllable: bool, property: Option<String>) -> bool {
    if world.map.width == 0 {
        return false;
    }
    let bounds = map_bounds(world);
    let entry = aa_combat::summon::transport_entry([aa_body::F32(point.x.to_bits()), aa_body::F32(point.y.to_bits())], [aa_body::F32(bounds.extent[0].to_bits()), aa_body::F32(bounds.extent[1].to_bits())], PC);
    let Ok((spawn, direction)) = entry else { return false };
    let mut xy = [f32::from_bits(spawn[0].0), f32::from_bits(spawn[1].0)];
    if falling::clamp_to_interior(bounds, &mut xy, PC).is_err() {
        return false;
    }
    let d = Vec2::new(f32::from_bits(direction[0].0), f32::from_bits(direction[1].0));
    fly_in(world, kingdom, plane, cargo, Vec2::new(xy[0], xy[1]), d, point, Some((controllable, property)));
    true
}

/// `0x4F9C10` of the plane and `0x58BA90`.
#[allow(clippy::too_many_arguments)]
fn fly_in(world: &mut World, kingdom: KingdomId, plane: DefId, cargo: DefId, enter: Vec2, d: Vec2, point: Vec2, summon: Option<(bool, Option<String>)>) {
    let pd = world.data.def(plane);
    let height = pd.aircraft.as_ref().map_or(0.0, |x| x.operational_height);
    let cd = world.data.def(cargo);
    let organization = cd.class == Some(DefClass::Organization);
    let count = if organization { cd.slots.iter().filter(|s| holds(s.0)).count() as i32 } else { 1 };
    let a = world.spawn_actor(plane, Some(kingdom), Vec3::new(enter.x, enter.y, height), d.y.atan2(d.x));
    let health = crate::production::max_health(world, Some(kingdom), plane);
    world.actors[a as usize].health = health;
    let now = world.time;
    world.books.actor_mut(a).created_at = now;
    // 0x4D7287 -> 0x55E600 -> 0x576B20: requested 0.1, factor 1.
    world.schedule_periodic_at(sites::INIT_EXISTING_AIRCRAFT, EVERY_TICK, Owner::Mover(a), 0);
    world.books.actor_mut(a).transport = Some(Transport {
        cargo,
        organization,
        point: [point.x.to_bits(), point.y.to_bits()],
        count,
        dropped: 0,
        org: None,
        next: 0,
        out: [enter.x.to_bits(), enter.y.to_bits()],
        facing: [d.x, d.y, 0.0],
        speed: 0.0,
        direction: [0.0, 0.0],
        at: [0.0, 0.0],
        summon,
    });
    // The Move command `0x4CA760` -> `0x4D9B60` makes the Mind think in the next update
    // (`0x52C320`): measured, its first step is two updates after the creation.
    if world.is_scheduled(Owner::Mind(a)) {
        world.unschedule(Owner::Mind(a));
    }
    world.schedule_after(crate::scheduler::THINK, Owner::Mind(a), 0, f32::from_bits(0x38d1_b717));
}

/// A layout's slot with a thing in it.
fn holds(def: DefId) -> bool {
    def != NO_DEF && def != u32::MAX
}

/// Whether `a` is a transport in flight.
pub fn flying(world: &World, a: ActorId) -> bool {
    world.books.actor(a).is_some_and(|b| b.transport.is_some())
}

/// Whether `a` is falling (actor state 24).
pub fn is_falling(world: &World, a: ActorId) -> bool {
    world.books.actor(a).is_some_and(|b| b.fall.is_some())
}

/// An organization a transport is still dropping (`0x579110` finds its state 19 on).
pub(crate) fn dropping_into(world: &World, o: OrgId) -> bool {
    world.books.actors.iter().any(|b| b.transport.as_ref().is_some_and(|t| t.org == Some(o) && t.dropped < t.count))
}

/// `0x58B180`: the farthest a thing of the cargo carries falling from `height` at `speed`.
fn lead_distance(world: &World, t: &Transport, height: f32, speed: f32) -> f32 {
    if !t.organization {
        return fall_distance(world, t.cargo, height, speed);
    }
    let mut most = 0.0f32;
    for s in &world.data.def(t.cargo).slots {
        if holds(s.0) {
            let v = fall_distance(world, s.0, height, speed);
            if most <= v {
                most = v;
            }
        }
    }
    most
}

/// `0x577380`: `sqrt(height / gravity * -2) * speed - fall_drag`, at least 0.
fn fall_distance(world: &World, def: DefId, height: f32, speed: f32) -> f32 {
    let gravity = world.data.rule_ci("Gravity").unwrap_or(-49.05);
    let v = (f64::from(height) / f64::from(gravity) * -2.0).sqrt() * f64::from(speed) - f64::from(world.data.def(def).fall_drag);
    if v < 0.0 { 0.0 } else { v as f32 }
}

/// `0x5179E0`'s arithmetic: state +0x14/+0x18 the point; the first drop point and the
/// direction, kept on the Transport.
pub(crate) fn aim(world: &mut World, a: ActorId, point: [u32; 2]) -> Option<([u32; 2], [u32; 2])> {
    let t = world.books.actor(a)?.transport.clone()?;
    let d = world.data.def(world.actors[a as usize].def);
    let half = (f64::from(d.speed_max) * f64::from(DROP_SPEED)) as f32;
    let height = d.aircraft.as_ref().map_or(0.0, |x| x.operational_height);
    let delay = d.transport_delay;
    let fall = lead_distance(world, &t, height, half);
    let lead = (f64::from(delay) * f64::from((t.count - 1) / 2) * f64::from(half) + f64::from(fall)) as f32;
    let p = world.actors[a as usize].pos;
    let (px, py) = (f32::from_bits(point[0]), f32::from_bits(point[1]));
    let (dx, dy) = (px - p.x, py - p.y);
    let length = (dx * dx + dy * dy).sqrt();
    let (mut ux, mut uy) = (0.0f32, 0.0f32);
    if length > 1e-4 {
        let inverse = 1.0 / length;
        ux = dx * inverse;
        uy = inverse * dy;
    }
    let first = [px - ux * lead, py - uy * lead];
    if let Some(t) = world.books.actor_mut(a).transport.as_mut() {
        t.direction = [ux, uy];
        t.at = first;
    }
    Some(([first[0].to_bits(), first[1].to_bits()], [ux.to_bits(), uy.to_bits()]))
}

/// The half of `0x517B60` after `0x58BCB0`: the next point while members are left, moved
/// by a deployment at the Mover's speed (ActorData +0x41C * `0x45D6F0(0)`). The hold is
/// counted as it will be: the drop itself runs when the Mind's world returns.
pub(crate) fn next_point(world: &mut World, a: ActorId) -> Option<([u32; 2], [u32; 2])> {
    let d = world.data.def(world.actors[a as usize].def);
    let step = (f64::from(d.transport_delay) * f64::from(d.speed_max)) as f32;
    let t = world.books.actor_mut(a).transport.as_mut()?;
    if t.dropped + 1 >= t.count {
        return None;
    }
    t.at[0] += step * t.direction[0];
    t.at[1] += step * t.direction[1];
    Some(([t.at[0].to_bits(), t.at[1].to_bits()], [t.direction[0].to_bits(), t.direction[1].to_bits()]))
}

/// `0x516D10`: the way out reached (`0x52A1F0`).
pub(crate) fn at_way_out(world: &World, a: ActorId) -> bool {
    let Some(t) = world.books.actor(a).and_then(|b| b.transport.as_ref()) else { return false };
    let p = world.actors[a as usize].pos;
    let (x, y) = (f32::from_bits(t.out[0]) - p.x, f32::from_bits(t.out[1]) - p.y);
    x * x + y * y < OUT_SQUARED
}

/// `0x58B130`: `0x4F6760(actor, 1)`.
pub(crate) fn leave(world: &mut World, a: ActorId) {
    crate::scheduler::release_actor(world, a);
    if let Some(x) = world.actors.get_mut(a as usize) {
        x.alive = false;
        x.removed = true;
    }
}

/// `0x58BCB0`: one thing out. The first turn of an organization makes it at the point
/// first (`0x554BF0` within 4 m from the plane, `0x4F9C10`, `0x58B160`).
pub(crate) fn drop_one(world: &mut World, a: ActorId) {
    let Some(mut t) = world.books.actor(a).and_then(|b| b.transport.clone()) else { return };
    if t.dropped >= t.count {
        return;
    }
    let plane = world.actors[a as usize].clone();
    let Some(kingdom) = plane.kingdom else { return };
    let (fx, fy) = (t.facing[0], t.facing[1]);
    let heading = fy.atan2(fx);
    let data = world.data.clone();
    let m = if t.organization {
        let d = data.def(t.cargo);
        if t.dropped == 0 {
            let point = Vec2::new(f32::from_bits(t.point[0]), f32::from_bits(t.point[1]));
            let leader = d.slots.iter().map(|s| s.0).find(|&s| holds(s)).unwrap_or(t.cargo);
            let at = crate::production::open_position(world, leader, point, plane.pos.truncate(), Vec2::new(fx, fy), 4.0);
            let o = crate::production::new_organization(world, t.cargo, kingdom, at, heading, None);
            t.org = Some(o);
            crate::statistics::recruited(world, kingdom, t.cargo, Some(o));
            world.events.push(Event::Recruited { kingdom, org: Some(o), actor: None });
        }
        let Some(o) = t.org.filter(|&o| world.orgs.get(o as usize).is_some_and(|x| x.alive)) else {
            t.dropped = t.count;
            world.books.actor_mut(a).transport = Some(t);
            return;
        };
        let mut slot = t.next;
        while slot < d.slots.len() && !holds(d.slots[slot].0) {
            slot += 1;
        }
        t.next = slot + 1;
        if slot >= d.slots.len() {
            t.dropped = t.count;
            world.books.actor_mut(a).transport = Some(t);
            return;
        }
        let member = crate::production::slot_element(world, o, slot).unwrap_or(d.slots[slot].0);
        let z = plane.pos.z - data.def(member).height;
        let m = crate::production::spawn_single(world, member, Some(kingdom), plane.pos.truncate(), heading);
        world.actors[m as usize].pos.z = z;
        world.actors[m as usize].org = Some(o);
        world.actors[m as usize].slot = Some(slot as u16);
        world.orgs[o as usize].members.push(m);
        crate::movement::member_added(world, o, m);
        m
    } else {
        let z = plane.pos.z - data.def(t.cargo).height;
        let m = crate::production::spawn_single(world, t.cargo, Some(kingdom), plane.pos.truncate(), heading);
        world.actors[m as usize].pos.z = z;
        crate::statistics::recruited(world, kingdom, t.cargo, None);
        world.events.push(Event::Recruited { kingdom, org: None, actor: Some(m) });
        m
    };
    if let Some((controllable, property)) = &t.summon {
        if !controllable {
            world.special_ops.uncontrollable.push(m);
        }
        if let Some(p) = property {
            let b = world.books.actor_mut(m);
            if !b.properties.iter().any(|q| q.eq_ignore_ascii_case(p)) {
                b.properties.push(p.clone());
            }
        }
    }
    t.dropped += 1;
    let (speed, facing) = (t.speed, t.facing);
    world.books.actor_mut(a).transport = Some(t);
    begin_fall(world, m, facing, speed);
}

/// The tail of `0x58BCB0`: the thing takes the plane's speed (`0x576BB0`), and its fall aims
/// (`0x5770A0`) at where `0x577380` carries it, along the plane's way.
fn begin_fall(world: &mut World, m: ActorId, facing: [f32; 3], speed: f32) {
    let def = world.actors[m as usize].def;
    let p = world.actors[m as usize].pos;
    let height = p.z - world.map.height_at(p.truncate());
    let drag = world.data.def(def).fall_drag;
    let carry = fall_distance(world, def, height, speed);
    let mut unit = [facing[0], facing[1], 0.0];
    let length = (unit[0] * unit[0] + unit[1] * unit[1]).sqrt();
    if length > 0.0 {
        let inverse = 1.0 / length;
        unit = [unit[0] * inverse, unit[1] * inverse, 0.0];
    }
    let bounds = map_bounds(world);
    let mut xy = [unit[0] * carry + p.x, unit[1] * carry + p.y];
    let _ = falling::clamp_to_interior(bounds, &mut xy, PC);
    let mut mover = plain_mover(speed);
    let mut state = FallingState { drag: 0.0, orient_to_motion: 0, direction: [0.0; 3], aimed: 0, aim_point: [0.0; 3], aim_delta: [0.0; 3], outside_map: 0 };
    let mut motion = motion_of(world, m, facing);
    let mut host = FallHost { world, a: m };
    if falling::begin_aimed_fall(&mut mover, &mut state, &mut motion, drag, xy, 0, PC, &mut host).is_ok() {
        world.books.actor_mut(m).fall = Some(Fall { speed: mover.speed, state, facing });
    }
}

fn plain_mover(speed: f32) -> MoverState {
    MoverState {
        destination: [0.0; 3],
        last_step_speed: 0.0,
        speed,
        requested_speed: 0.1,
        acceleration: 0.0,
        turn_rate: 0.0,
        desired_direction: [0.0; 3],
        target_id: 0,
        target_range: None,
        ignore_terrain_speed_limit: false,
        speed_factor: 1.0,
        instant_turn: false,
        turning: false,
        falling: false,
    }
}

fn motion_of(world: &World, a: ActorId, facing: [f32; 3]) -> ActorMotion {
    let p = world.actors[a as usize].pos;
    ActorMotion { position: [p.x, p.y, p.z], facing, flags: 0, has_mind: true, has_character: false, character_stopped: false, terrain_speed_limit: 0.0 }
}

/// The world of a fall.
struct FallHost<'a> {
    world: &'a mut World,
    a: ActorId,
}
impl FallingWorld for FallHost<'_> {
    fn has_aircraft(&mut self, _: &mut ActorMotion) -> aa_movement::Result<bool> {
        Ok(false)
    }
    fn aircraft_height_offset(&mut self, _: &mut ActorMotion) -> aa_movement::Result<f32> {
        Ok(0.0)
    }
    fn ground_height(&mut self, xy: [f32; 2], _: &mut MoverState, _: &mut FallingState, _: &mut ActorMotion) -> aa_movement::Result<Binary> {
        let h = self.world.map.height_at(Vec2::new(xy[0], xy[1]));
        Binary::from_f32_bits(h.to_bits()).map_err(|e| aa_movement::Error::World(e.to_string()))
    }
    fn left_map(&mut self, _: &mut ActorMotion) -> aa_movement::Result<()> {
        Ok(())
    }
    fn entered_map(&mut self, _: &mut ActorMotion) -> aa_movement::Result<()> {
        Ok(())
    }
    fn commit_position(&mut self, position: [f32; 3], actor: &mut ActorMotion) -> aa_movement::Result<()> {
        actor.position = position;
        self.world.actors[self.a as usize].pos = Vec3::new(position[0], position[1], position[2]);
        crate::movement::thing_moved(self.world, self.a);
        Ok(())
    }
    fn set_facing(&mut self, facing: [f32; 3], actor: &mut ActorMotion) -> aa_movement::Result<()> {
        actor.facing = facing;
        Ok(())
    }
    fn actor_state(&mut self, _: u32, _: bool, _: &mut ActorMotion) -> aa_movement::Result<()> {
        Ok(())
    }
    /// `0x50AC00(0)` of `0x5770A0` (site 0x5770E9).
    fn schedule_immediate(&mut self, _: &mut ActorMotion) -> aa_movement::Result<()> {
        self.world.schedule_periodic_at(sites::CARGO_FALL, EVERY_TICK, Owner::Fall(self.a), 0);
        Ok(())
    }
    fn cancel_scheduled(&mut self, _: &mut ActorMotion) -> aa_movement::Result<()> {
        self.world.unschedule(Owner::Fall(self.a));
        Ok(())
    }
    /// `0x4D64A0` of a live thing: with a Character and no Aircraft `0x563D50`.
    fn landed(&mut self, _: &mut ActorMotion) -> aa_movement::Result<()> {
        crate::character::landed(self.world, self.a);
        Ok(())
    }
}

/// The world's tick length, GWorld +0xE8.
fn tick_length(world: &World) -> f32 {
    f32::from_bits(world.clock.step.0)
}

/// The Mover Tick of a falling thing: `0x577E50` -> `0x577A70`.
pub fn fall_tick(world: &mut World, a: ActorId) {
    let Some(Fall { speed, mut state, facing }) = world.books.actor_mut(a).fall.take() else { return };
    if !world.actors[a as usize].alive {
        world.unschedule(Owner::Fall(a));
        return;
    }
    let mut mover = plain_mover(speed);
    mover.falling = true;
    let mut motion = motion_of(world, a, facing);
    let context = FallContext { delta: tick_length(world), gravity: world.data.rule_ci("Gravity").unwrap_or(-49.05), plane_fall_gravity_tweak: None, map: map_bounds(world), precision: PC };
    let mut host = FallHost { world, a };
    if falling::fall_step(&mut mover, &mut state, &mut motion, &context, &mut host).is_err() {
        world.unschedule(Owner::Fall(a));
        return;
    }
    if mover.falling {
        world.books.actor_mut(a).fall = Some(Fall { speed: mover.speed, state, facing: motion.facing });
    }
}

/// The world of a flight: a lone aircraft, no formation.
struct FlightHost<'a> {
    world: &'a mut World,
    a: ActorId,
    exhausted: bool,
}

fn group_refused<T>() -> AircraftResult<T> {
    Err(world_error("a transport flies in no formation"))
}

impl FormationWorld for FlightHost<'_> {
    fn handle_slot(&mut self, _: u16) -> AircraftResult<HandleSlot> {
        group_refused()
    }
    fn actor_handle(&mut self, _: ActorKey) -> AircraftResult<u32> {
        group_refused()
    }
    fn actor_flags(&mut self, _: ActorKey) -> AircraftResult<u32> {
        group_refused()
    }
    fn group_members(&mut self, _: GroupKey) -> AircraftResult<Vec<u32>> {
        group_refused()
    }
    fn group_layout(&mut self, _: GroupKey) -> AircraftResult<GroupLayout> {
        group_refused()
    }
    fn member_aircraft(&mut self, _: ActorKey) -> AircraftResult<&mut AircraftState> {
        group_refused()
    }
}
impl GroupWorld for FlightHost<'_> {
    fn group_unusable(&mut self, _: GroupKey) -> AircraftResult<bool> {
        Ok(true)
    }
    fn retain_group(&mut self, _: GroupKey) -> AircraftResult<()> {
        group_refused()
    }
    fn release_group(&mut self, _: GroupKey) -> AircraftResult<bool> {
        group_refused()
    }
    fn definition_speed(&mut self, _: ActorKey) -> AircraftResult<f32> {
        Ok(self.world.data.def(self.world.actors[self.a as usize].def).speed_max)
    }
}
impl TerrainWorld for FlightHost<'_> {
    fn ground_height(&mut self, xy: [f32; 2], _: u32) -> AircraftResult<Binary> {
        let h = self.world.map.height_at(Vec2::new(xy[0], xy[1]));
        Binary::from_f32_bits(h.to_bits()).map_err(world_error)
    }
}
impl FlightWorld for FlightHost<'_> {
    fn map_bounds(&mut self) -> AircraftResult<MapBounds> {
        Ok(map_bounds(self.world))
    }
}
impl AircraftWorld for FlightHost<'_> {
    fn mind_movement_blocked(&mut self, _: &mut AircraftUnit) -> AircraftResult<bool> {
        Ok(false)
    }
    fn ability_definition_word(&mut self, _: AbilityKey) -> AircraftResult<u32> {
        Err(world_error("a transport fires nothing"))
    }
    fn attack_descriptor(&mut self, _: u32, _: &mut AircraftUnit) -> AircraftResult<()> {
        Err(world_error("a transport fires nothing"))
    }
    fn ability_check(&mut self, _: AbilityKey, _: &AttackDescriptor, _: &mut AircraftUnit) -> AircraftResult<u32> {
        Err(world_error("a transport fires nothing"))
    }
    fn fire_ability(&mut self, _: Option<AbilityKey>, _: &AttackDescriptor, _: &mut AircraftUnit) -> AircraftResult<()> {
        Err(world_error("a transport fires nothing"))
    }
    fn actor_speed(&mut self, unit: &mut AircraftUnit) -> AircraftResult<Binary> {
        Binary::from_f32_bits(unit.mover.speed.to_bits()).map_err(world_error)
    }
    fn formation_speed(&mut self, _: &mut AircraftUnit) -> AircraftResult<Binary> {
        let v = self.world.data.def(self.world.actors[self.a as usize].def).speed_max;
        Binary::from_f32_bits(v.to_bits()).map_err(world_error)
    }
    fn group_disbanded(&mut self, _: GroupKey, _: &mut AircraftUnit) -> AircraftResult<bool> {
        Ok(true)
    }
    fn leave_formation(&mut self, _: &mut AircraftUnit) -> AircraftResult<()> {
        Ok(())
    }
    fn plan_follow_path(&mut self, _: &mut AircraftUnit, _: &Pose, _: ActorKey, _: [f32; 3], _: f32) -> AircraftResult<()> {
        group_refused()
    }
    fn build_leg(&mut self, unit: &mut AircraftUnit, from: &Pose, target: [f32; 3], side: [f32; 3], flag: u32) -> AircraftResult<Pose> {
        aircraft::build_leg(&mut unit.aircraft.path, from, target, Some(side), flag as u8, 0, PC, self)
    }
    fn mind_scheduled(&mut self, _: &mut AircraftUnit) -> AircraftResult<bool> {
        Ok(self.world.is_scheduled(Owner::Mind(self.a)))
    }
    fn report_stopped_thinking(&mut self, _: u32, _: u32) -> AircraftResult<()> {
        Ok(())
    }
    fn mind_reschedule(&mut self, delay_bits: u32, argument: u32, _: &mut AircraftUnit) -> AircraftResult<()> {
        let owner = Owner::Mind(self.a);
        if self.world.is_scheduled(owner) {
            self.world.unschedule(owner);
        }
        self.world.schedule_after(crate::scheduler::THINK, owner, argument, f32::from_bits(delay_bits));
        Ok(())
    }
    /// `0x576BB0`: inside the map or clamped into it, Mover +0x20, the pose.
    fn commit_pose(&mut self, position: [f32; 3], facing: [f32; 3], speed: f32, unit: &mut AircraftUnit) -> AircraftResult<()> {
        let bounds = map_bounds(self.world);
        let mut xy = [position[0], position[1]];
        if !falling::inside_map(bounds, xy, PC)? {
            falling::clamp_to_interior(bounds, &mut xy, PC)?;
        }
        unit.mover.speed = speed;
        unit.motion.position = [xy[0], xy[1], position[2]];
        unit.motion.facing = facing;
        let x = &mut self.world.actors[self.a as usize];
        x.pos = Vec3::new(xy[0], xy[1], position[2]);
        x.facing = facing[1].atan2(facing[0]);
        Ok(())
    }
    fn ground_height(&mut self, xy: [f32; 2], flags: u32, _: &mut AircraftUnit) -> AircraftResult<Binary> {
        TerrainWorld::ground_height(self, xy, flags)
    }
    fn mind_path_exhausted(&mut self, _: &mut AircraftUnit) -> AircraftResult<()> {
        self.exhausted = true;
        Ok(())
    }
}

fn unit_of(world: &World, a: ActorId, aircraft: AircraftState, t: &Transport) -> AircraftUnit {
    let d = world.data.def(world.actors[a as usize].def);
    let air = d.aircraft.clone().unwrap_or_default();
    AircraftUnit {
        actor: AircraftActor {
            key: ActorKey(a),
            handle: world.ids.actor(a).map_or(0, |g| g.raw()),
            scale: world.books.actor(a).and_then(|b| b.aircraft.as_ref()).map_or(1.0, |x| x.scale),
            definition: AircraftDefinition { taxi_scale: air.taxi_scale, taxi_height: air.taxi_height, operational_height: air.operational_height },
        },
        motion: motion_of(world, a, t.facing),
        mover: plain_mover(t.speed),
        mover_limits: MoverLimits { max_speed: d.speed_max, turn_radius: d.turn_radius },
        aircraft,
    }
}

/// Run `f` over the unit of transport `a`; the unit's state goes back after.
fn with_unit<R>(world: &mut World, a: ActorId, f: impl FnOnce(&mut AircraftUnit, &mut FlightHost<'_>) -> AircraftResult<R>) -> Option<(R, bool)> {
    let t = world.books.actor(a)?.transport.clone()?;
    let aircraft = world.books.actors.get_mut(a as usize)?.aircraft.as_mut().map(|x| std::mem::replace(&mut x.flight, AircraftState::new(0.0, 0.0)))?;
    let mut unit = unit_of(world, a, aircraft, &t);
    let mut host = FlightHost { world, a, exhausted: false };
    let result = f(&mut unit, &mut host);
    let exhausted = host.exhausted;
    if let Some(b) = world.books.actors.get_mut(a as usize) {
        if let Some(x) = b.aircraft.as_mut() {
            x.flight = unit.aircraft;
            x.scale = unit.actor.scale;
        }
        if let Some(t) = b.transport.as_mut() {
            t.facing = unit.motion.facing;
            t.speed = unit.mover.speed;
        }
    }
    match result {
        Ok(r) => Some((r, exhausted)),
        Err(e) => {
            world.books.minds.note(format!("transport {a} flight: {e}"));
            None
        }
    }
}

/// `0x560DB0` of a transport: the legs to the point from the end of its path.
pub(crate) fn fly_to(world: &mut World, a: ActorId, point: Vec3, direction: Vec3) {
    let dir = (direction != Vec3::ZERO).then_some([direction.x, direction.y, direction.z]);
    with_unit(world, a, |unit, host| aircraft::fly_to(unit, [point.x, point.y, point.z], dir, PC, host));
}

/// The Mover Tick of a transport: `0x577E50` -> Aircraft MoveTick `0x5612F0`, at the path end
/// Mind +0x144 (`0x517DD0`).
pub fn flight_tick(world: &mut World, a: ActorId) {
    if !world.actors[a as usize].alive {
        world.unschedule(Owner::Mover(a));
        return;
    }
    // Before its first Think the plane has no path and no speed, and stands (rendition: the
    // holding circle 0x560050 of a speed of 0 is not run). Measured, Berlin2v2_1: made at
    // 3841, six steps by 3848.
    let unplanned = world.books.actor(a).is_some_and(|b| b.transport.as_ref().is_some_and(|t| t.speed == 0.0) && b.aircraft.as_ref().is_some_and(|x| x.flight.path.segments.is_empty()));
    if unplanned {
        return;
    }
    let delta = tick_length(world);
    let Some((_, exhausted)) = with_unit(world, a, |unit, host| aircraft::move_tick(unit, delta, PC, host)) else { return };
    if exhausted {
        crate::combat::air::path_finished(world, a);
    }
}
