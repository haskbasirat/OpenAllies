//! The flight of an air base's aircraft (a denizen of an airfield or a
//! carrier): the AircraftComponent MoveTick 0x5612F0 and its legs 0x560DB0
//! on the exact ports (`aa_movement::aircraft`, `aircraft_legs`), given the
//! world of one aircraft that flies alone. Owner: air.
//!
//! - Created (CreateDenizen 0x56CC10 -> 0x4F9C10): InitExisting 0x4D7230
//!   calls the AircraftComponent's virtual +8 (0x55E600), which asks the
//!   slot-3 Mover to move (0x576B20): it registers on queue 0, every tick,
//!   with one draw ([`registered`]).
//! - MoveTick (0x577E50 -> 0x5612F0, [`tick`]) opens with Mind virtual
//!   +0x1B0 (0x51C1D0 -> 0x52A000 on the queued state, Mind+0x188): while
//!   aai_queued is the current state the tick does nothing, so an aircraft
//!   waiting in the hangar does not move. Otherwise the flight steps along
//!   its path, and at the path end Mind +0x144 (0x517DD0,
//!   [`super::path_finished`]).
//! - The legs (0x516740 / 0x560ED0 -> 0x560DB0, [`fly_to`]) to the move
//!   state's goal.
//! - Each step's pose goes through 0x576BB0 ([`AircraftWorld::commit_pose`]
//!   below): the clamp into the map with the Mover's +0x88 flag, the map
//!   exit 0x4D04B0 / entry 0x4D7EA0 in part (bit 30 only), Mover +0x20,
//!   the position commit 0x4EE5C0 and the facing (actor virtual +0x54,
//!   stored without 0x4EE9F0's normalising).
//! - The top speed (Mover +0x28, actor virtual +0x104) carries the
//!   kingdom's SPEED_MAX modifiers ([`top_speed`]).
//!
//! Between ticks the Mover's speed (+0x20), requested speed (+0x24) and
//! +0x88 flag and the actor's facing (+0x2C..+0x34) are kept as [`Mover`];
//! the speed is also the actor's current speed (virtual +0x18,
//! `Actor::motion.speed`).
//!
//! **Renditions:** as for a transport, an aircraft that is not queued, has
//! no path and no speed stands (the holding circle 0x560050 of a speed of 0
//! is not run). Not ported: formation flight (the group legs 0x560A70 /
//! 0x561080) and the attack release (0x57E030 on the Performer, the bomb):
//! a MoveTick with a pending ability stops with a note. 0x576B20's
//! activation of the Mover (0x4CFC10) also sets actor+0xE4 (the current
//! Mover), calls the old Mover's +0x40, and the new Mover's +0x3C
//! (0x576790) runs 0x50B250 and fires the ActorData +0x24C trigger list
//! (0x42A590); none of that is run here.
use crate::scheduler::{EVERY_TICK, Owner};
use crate::trace::sites;
use crate::world::{ActorId, World};
use aa_movement::aircraft::{
    self, AbilityKey, ActorKey, AircraftActor, AircraftDefinition, AircraftError, AircraftResult, AircraftState, AircraftUnit, AircraftWorld, AttackDescriptor, FlightWorld, FormationWorld, GroupKey,
    GroupLayout, GroupWorld, HandleSlot, Pose, TerrainWorld,
};
use aa_movement::falling::{self, FallingState, FallingWorld, MapBounds};
use aa_movement::mover::MoverLimits;
use aa_movement::{ActorMotion, MoverState};
use aa_numeric::{Binary, Precision};
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

const PC: Precision = Precision::Double53;
/// The aircraft Mind's aai_queued state (Mind+0x188).
const QUEUED: u32 = 0x188;
/// `[9FE0C0]`: 0x55E600's least requested speed.
const LEAST_REQUESTED: f32 = 0.1;

/// A base aircraft's Mover between its MoveTicks.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Mover {
    /// Mover +0x20.
    pub speed: f32,
    /// Actor +0x2C..+0x34.
    pub facing: [f32; 3],
    /// Mover +0x24, the requested speed 0x576B20 wrote.
    #[serde(default)]
    pub requested: f32,
    /// Mover +0x88: the last pose commit was clamped into the map.
    #[serde(default)]
    pub outside_map: bool,
    /// The last flight error noted for this aircraft (the same error is not
    /// noted again on every tick).
    #[serde(default)]
    pub failed: Option<String>,
}

/// Whether `a` is an air base's aircraft (CreateDenizen flag +8 bit 2) and
/// not a transport.
pub fn flies(world: &World, a: ActorId) -> bool {
    world.books.actor(a).is_some_and(|b| b.transport.is_none() && b.aircraft.as_ref().is_some_and(|x| x.denizen))
}

/// 0x55E600 -> 0x576B20 at the aircraft's creation. The Mover is new
/// (constructor 0x576920: +0x20, +0x24 and +0x88 zero), so 0x55E600 asks for
/// its +0x24 when that is not below 0.1 (an unordered value is kept), else
/// for 0.1, at factor 1. 0x576B20 writes +0x24 / +0x50 and registers the
/// Mover on queue 0 (0x50AC00(0), one draw) unless it is falling (+0x56) or
/// its virtual +0x38 refuses: 0x4DB0E0 -> 0x4CFC10, which for an actor with
/// flag +8 bit 0 (removed) asks the Mover's +0x44 (0x6A71F0: false).
pub(crate) fn registered(world: &mut World, a: ActorId) {
    let facing = world.actors[a as usize].facing;
    let mut mover = Mover { speed: 0.0, facing: [facing.cos(), facing.sin(), 0.0], requested: 0.0, outside_map: false, failed: None };
    let requested = if mover.requested < LEAST_REQUESTED { LEAST_REQUESTED } else { mover.requested };
    let falling = world.books.actor(a).is_some_and(|b| b.fall.is_some());
    let removed = world.actors[a as usize].removed;
    if !falling && !removed {
        mover.requested = requested;
        world.schedule_periodic_at(sites::INIT_EXISTING_AIRCRAFT, EVERY_TICK, Owner::Mover(a), 0);
    }
    world.books.actor_mut(a).flight_mover = Some(mover);
}

/// 0x560DB0 of a base aircraft: the legs to `point` from the end of its
/// path, with the direction pointer (`None`: null). 0x516740 always passes
/// one (the move state's +0x24); the move-state Think 0x51A720 passes it
/// only when +0x30 is set. Any other aircraft goes to the movement owner's
/// entry point.
pub fn fly_to(world: &mut World, a: ActorId, point: Vec3, direction: Option<Vec3>) {
    if !flies(world, a) {
        crate::movement::fly_to(world, a, point, direction.unwrap_or(Vec3::ZERO));
        return;
    }
    let dir = direction.map(|d| [d.x, d.y, d.z]);
    with_unit(world, a, |unit, host| aircraft::fly_to(unit, [point.x, point.y, point.z], dir, PC, host));
}

/// The Mover Tick of a base aircraft: 0x577E50 -> MoveTick 0x5612F0, at the
/// path end Mind +0x144 (0x517DD0).
pub fn tick(world: &mut World, a: ActorId) {
    if !world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed) {
        world.unschedule(Owner::Mover(a));
        return;
    }
    // 0x5612F0's first test, Mind virtual +0x1B0: queued, nothing moves
    // (`aircraft::move_tick` asks it again, with the same answer).
    if queued(world, a) {
        return;
    }
    let unplanned = world.books.actor(a).is_some_and(|b| {
        b.flight_mover.as_ref().is_some_and(|m| m.speed == 0.0) && b.aircraft.as_ref().is_some_and(|x| x.flight.path.segments.is_empty())
    });
    if unplanned {
        return;
    }
    let delta = f32::from_bits(world.clock.step.0);
    let Some((_, exhausted)) = with_unit(world, a, |unit, host| aircraft::move_tick(unit, delta, PC, host)) else { return };
    if exhausted {
        super::path_finished(world, a);
    }
}

/// Mind virtual +0x1B0 = 0x51C1D0: 0x52A000 on aai_queued (Mind+0x188),
/// whether it is the current state.
fn queued(world: &World, a: ActorId) -> bool {
    super::current_state(world, a) == Some(QUEUED)
}

fn world_error(what: impl std::fmt::Display) -> AircraftError {
    AircraftError::Movement(aa_movement::Error::World(what.to_string()))
}

fn map_bounds(world: &World) -> MapBounds {
    let c = world.map.cell_size;
    MapBounds { cell_size: c, extent: [c * world.map.width as f32, c * world.map.height as f32] }
}

fn mover_of(mover: &Mover) -> MoverState {
    MoverState {
        destination: [0.0; 3],
        last_step_speed: 0.0,
        speed: mover.speed,
        requested_speed: mover.requested,
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

/// The world of a base aircraft's flight: alone, no formation, no release.
struct Host<'a> {
    world: &'a mut World,
    a: ActorId,
    exhausted: bool,
    /// Mover +0x88.
    outside_map: bool,
}

fn alone<T>() -> AircraftResult<T> {
    Err(world_error("formation flight of a base aircraft is not ported"))
}

impl FormationWorld for Host<'_> {
    fn handle_slot(&mut self, _: u16) -> AircraftResult<HandleSlot> {
        alone()
    }
    fn actor_handle(&mut self, _: ActorKey) -> AircraftResult<u32> {
        alone()
    }
    fn actor_flags(&mut self, _: ActorKey) -> AircraftResult<u32> {
        alone()
    }
    fn group_members(&mut self, _: GroupKey) -> AircraftResult<Vec<u32>> {
        alone()
    }
    fn group_layout(&mut self, _: GroupKey) -> AircraftResult<GroupLayout> {
        alone()
    }
    fn member_aircraft(&mut self, _: ActorKey) -> AircraftResult<&mut AircraftState> {
        alone()
    }
}
impl GroupWorld for Host<'_> {
    fn group_unusable(&mut self, _: GroupKey) -> AircraftResult<bool> {
        Ok(true)
    }
    fn retain_group(&mut self, _: GroupKey) -> AircraftResult<()> {
        alone()
    }
    fn release_group(&mut self, _: GroupKey) -> AircraftResult<bool> {
        alone()
    }
    fn definition_speed(&mut self, _: ActorKey) -> AircraftResult<f32> {
        Ok(self.world.data.def(self.world.actors[self.a as usize].def).speed_max)
    }
}
impl TerrainWorld for Host<'_> {
    fn ground_height(&mut self, xy: [f32; 2], _: u32) -> AircraftResult<Binary> {
        let h = self.world.map.height_at(Vec2::new(xy[0], xy[1]));
        Binary::from_f32_bits(h.to_bits()).map_err(world_error)
    }
}
impl FlightWorld for Host<'_> {
    fn map_bounds(&mut self) -> AircraftResult<MapBounds> {
        Ok(map_bounds(self.world))
    }
}
impl AircraftWorld for Host<'_> {
    fn mind_movement_blocked(&mut self, _: &mut AircraftUnit) -> AircraftResult<bool> {
        Ok(queued(self.world, self.a))
    }
    fn ability_definition_word(&mut self, _: AbilityKey) -> AircraftResult<u32> {
        Err(world_error("the release of a base aircraft (0x57E030) is not ported"))
    }
    fn attack_descriptor(&mut self, _: u32, _: &mut AircraftUnit) -> AircraftResult<()> {
        Err(world_error("the release of a base aircraft (0x4C8300) is not ported"))
    }
    fn ability_check(&mut self, _: AbilityKey, _: &AttackDescriptor, _: &mut AircraftUnit) -> AircraftResult<u32> {
        Err(world_error("the release of a base aircraft (0x4C8F20) is not ported"))
    }
    fn fire_ability(&mut self, _: Option<AbilityKey>, _: &AttackDescriptor, _: &mut AircraftUnit) -> AircraftResult<()> {
        Err(world_error("the release of a base aircraft (0x57E030) is not ported"))
    }
    fn actor_speed(&mut self, unit: &mut AircraftUnit) -> AircraftResult<Binary> {
        Binary::from_f32_bits(unit.mover.speed.to_bits()).map_err(world_error)
    }
    /// Actor virtual +0x104 (0x4CEEA0): the Mover's +0x28, the top speed with
    /// its modifiers ([`top_speed`]).
    fn formation_speed(&mut self, _: &mut AircraftUnit) -> AircraftResult<Binary> {
        let v = top_speed(self.world, self.a);
        Binary::from_f32_bits(v.to_bits()).map_err(world_error)
    }
    fn group_disbanded(&mut self, _: GroupKey, _: &mut AircraftUnit) -> AircraftResult<bool> {
        Ok(true)
    }
    fn leave_formation(&mut self, _: &mut AircraftUnit) -> AircraftResult<()> {
        Ok(())
    }
    fn plan_follow_path(&mut self, _: &mut AircraftUnit, _: &Pose, _: ActorKey, _: [f32; 3], _: f32) -> AircraftResult<()> {
        alone()
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
    /// 0x576BB0 on the exact port (`falling::commit_pose`): the clamp with
    /// the +0x88 map exit / entry, Mover +0x20, 0x4EE5C0 and the facing.
    fn commit_pose(&mut self, position: [f32; 3], facing: [f32; 3], speed: f32, unit: &mut AircraftUnit) -> AircraftResult<()> {
        let bounds = map_bounds(self.world);
        let mut state = FallingState { drag: 0.0, orient_to_motion: 0, direction: [0.0; 3], aimed: 0, aim_point: [0.0; 3], aim_delta: [0.0; 3], outside_map: u8::from(self.outside_map) };
        let mut commit = Commit { world: &mut *self.world, a: self.a };
        falling::commit_pose(&mut unit.mover, &mut state, &mut unit.motion, position, facing, speed, bounds, PC, &mut commit)?;
        self.outside_map = state.outside_map != 0;
        self.world.actors[self.a as usize].motion.speed = speed;
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

/// The calls of 0x576BB0 for a base aircraft. The other falling callbacks
/// are not reached from a pose commit.
struct Commit<'a> {
    world: &'a mut World,
    a: ActorId,
}

fn not_a_fall<T>(what: &str) -> aa_movement::Result<T> {
    Err(aa_movement::Error::World(format!("{what} is not called by a pose commit")))
}

impl FallingWorld for Commit<'_> {
    fn has_aircraft(&mut self, _: &mut ActorMotion) -> aa_movement::Result<bool> {
        not_a_fall("actor+0xBC")
    }
    fn aircraft_height_offset(&mut self, _: &mut ActorMotion) -> aa_movement::Result<f32> {
        not_a_fall("ActorData +0x40C")
    }
    fn ground_height(&mut self, _: [f32; 2], _: &mut MoverState, _: &mut FallingState, _: &mut ActorMotion) -> aa_movement::Result<Binary> {
        not_a_fall("0x4F0E50")
    }
    /// 0x4D04B0: actor flag +8 bit 30 (the aircraft's `hidden`). Its call of
    /// actor virtual +0x50(0) is not run: 0x4D4A10 -> 0x4EE840 sets or
    /// clears flag +8 bit 10 (the definition's `blocking`), lists the actor
    /// in the tiles again (0x4EE4C0) and passes the call to the organization
    /// (+0x168).
    fn left_map(&mut self, _: &mut ActorMotion) -> aa_movement::Result<()> {
        if let Some(x) = self.world.books.actor_mut(self.a).aircraft.as_mut() {
            x.hidden = true;
        }
        Ok(())
    }
    /// 0x4D7EA0: flag +8 bit 30 clear, as the runway release
    /// (`crate::denizen::runway_release`). Not run: its first call, actor
    /// virtual +0x50(ActorData +0x1C bit 10), which may set flag +8 bit 10
    /// (see `left_map`), and after it 0x4D67F0 and 0x4EB6E0, which act only
    /// with that bit.
    fn entered_map(&mut self, _: &mut ActorMotion) -> aa_movement::Result<()> {
        if let Some(x) = self.world.books.actor_mut(self.a).aircraft.as_mut() {
            x.hidden = false;
        }
        Ok(())
    }
    /// 0x4EE5C0: the position and the tile lists.
    fn commit_position(&mut self, position: [f32; 3], actor: &mut ActorMotion) -> aa_movement::Result<()> {
        actor.position = position;
        self.world.actors[self.a as usize].pos = Vec3::new(position[0], position[1], position[2]);
        crate::movement::thing_moved(self.world, self.a);
        Ok(())
    }
    /// Actor virtual +0x54, stored as given. Not ported: 0x4EEC50 skips the
    /// write when flag +8 bit 17 is set, else 0x4EE9F0 normalises the
    /// vector again (in XY when bit 12 is clear, 0x4453A0 when set),
    /// updates the spatial shape with bit 26 and runs 0x50C270.
    fn set_facing(&mut self, facing: [f32; 3], actor: &mut ActorMotion) -> aa_movement::Result<()> {
        actor.facing = facing;
        self.world.actors[self.a as usize].facing = facing[1].atan2(facing[0]);
        Ok(())
    }
    fn actor_state(&mut self, _: u32, _: bool, _: &mut ActorMotion) -> aa_movement::Result<()> {
        not_a_fall("0x4D64F0")
    }
    fn schedule_immediate(&mut self, _: &mut ActorMotion) -> aa_movement::Result<()> {
        not_a_fall("0x50AC00")
    }
    fn cancel_scheduled(&mut self, _: &mut ActorMotion) -> aa_movement::Result<()> {
        not_a_fall("0x50B250")
    }
    fn landed(&mut self, _: &mut ActorMotion) -> aa_movement::Result<()> {
        not_a_fall("0x4D64A0")
    }
}

/// The aircraft's top speed with its modifiers, as the Mover's +0x28 holds
/// it: the definition's speed_max times the kingdom's SPEED_MAX MULTIPLY sum
/// (0x502A70) and the actor's enchantments, as `crate::movement`'s Mover
/// +0x28. The original's trace of a film (a saved game whose kingdom has a
/// SPEED_MAX technology) shows a ger_fighter's +0x28 at 13.75, its
/// speed_max 11 times 1.25, and its takeoff leg speeding up to 13.75 at the
/// rate that top speed gives.
pub fn top_speed(world: &World, a: ActorId) -> f32 {
    let actor = &world.actors[a as usize];
    let def = world.data.def(actor.def);
    let factor = actor.kingdom.map_or(1.0, |k| crate::production::multiply_factor(world, k, &def.properties, "SPEED_MAX"));
    let enchanted = crate::special_ops::mods(world, a).map_or(0.0, |m| m.speed);
    if enchanted != 0.0 {
        let clamp = world.data.rule_ci("ModifierAttributeClampMultiply").unwrap_or(0.05);
        return def.speed_max * (factor + enchanted).max(clamp);
    }
    def.speed_max * factor
}

fn unit_of(world: &World, a: ActorId, aircraft: AircraftState, mover: &Mover) -> AircraftUnit {
    let d = world.data.def(world.actors[a as usize].def);
    let air = d.aircraft.clone().unwrap_or_default();
    let p = world.actors[a as usize].pos;
    AircraftUnit {
        actor: AircraftActor {
            key: ActorKey(a),
            handle: world.ids.actor(a).map_or(0, |g| g.raw()),
            scale: world.books.actor(a).and_then(|b| b.aircraft.as_ref()).map_or(1.0, |x| x.scale),
            definition: AircraftDefinition { taxi_scale: air.taxi_scale, taxi_height: air.taxi_height, operational_height: air.operational_height },
        },
        motion: ActorMotion { position: [p.x, p.y, p.z], facing: mover.facing, flags: 0, has_mind: true, has_character: false, character_stopped: false, terrain_speed_limit: 0.0 },
        mover: mover_of(mover),
        mover_limits: MoverLimits { max_speed: top_speed(world, a), turn_radius: d.turn_radius },
        aircraft,
    }
}

/// Run `f` over the unit of base aircraft `a`; its state goes back after.
/// An error becomes a Mind note, once while it repeats.
fn with_unit<R>(world: &mut World, a: ActorId, f: impl FnOnce(&mut AircraftUnit, &mut Host<'_>) -> AircraftResult<R>) -> Option<(R, bool)> {
    let mover = world.books.actor(a)?.flight_mover.clone()?;
    let aircraft = world.books.actors.get_mut(a as usize)?.aircraft.as_mut().map(|x| std::mem::replace(&mut x.flight, AircraftState::new(0.0, 0.0)))?;
    let mut unit = unit_of(world, a, aircraft, &mover);
    let mut host = Host { world, a, exhausted: false, outside_map: mover.outside_map };
    let result = f(&mut unit, &mut host);
    let (exhausted, outside_map) = (host.exhausted, host.outside_map);
    let mut failed = mover.failed;
    let mut note = None;
    match &result {
        Err(e) => {
            let text = format!("aircraft {a} flight: {e}");
            if failed.as_deref() != Some(text.as_str()) {
                note = Some(text.clone());
                failed = Some(text);
            }
        }
        // A good call ends the repeat: the same error later is noted again.
        Ok(_) => failed = None,
    }
    if let Some(b) = world.books.actors.get_mut(a as usize) {
        if let Some(x) = b.aircraft.as_mut() {
            x.flight = unit.aircraft;
            x.scale = unit.actor.scale;
        }
        b.flight_mover = Some(Mover { speed: unit.mover.speed, facing: unit.motion.facing, requested: unit.mover.requested_speed, outside_map, failed });
    }
    if let Some(text) = note {
        world.books.minds.note(text);
    }
    result.ok().map(|r| (r, exhausted))
}
