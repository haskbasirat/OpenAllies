//! The flight of an air base's aircraft (a denizen of an airfield or a
//! carrier): the AircraftComponent MoveTick 0x5612F0 and its legs 0x560DB0
//! on the exact ports (`aa_movement::aircraft`, `aircraft_legs`), given the
//! world of one aircraft that flies alone. Owner: air.
//!
//! - Created (CreateDenizen 0x56CC10 -> 0x4F9C10): its Mover registers on
//!   queue 0, every tick (0x576B20, one draw), as a transport's does
//!   ([`registered`]).
//! - MoveTick (0x577E50 -> 0x5612F0, [`tick`]): Mind virtual +0x1B0
//!   (0x51C1D0 -> 0x52A000 on the queued state, Mind+0x188) suspends it
//!   while aai_queued is the current state, so an aircraft waiting in the
//!   hangar does not move. Otherwise the flight steps along its path, and at
//!   the path end Mind +0x144 (0x517DD0, [`super::path_finished`]).
//! - The legs (0x516740 / 0x560ED0 -> 0x560DB0, [`fly_to`]) to the move
//!   state's goal.
//!
//! Between ticks the Mover's speed (+0x20) and the actor's facing
//! (+0x2C..+0x34) are kept as [`Mover`]; the speed is also the actor's
//! current speed (virtual +0x18, `Actor::motion.speed`).
//!
//! **Renditions:** as for a transport, before its first leg an aircraft
//! with no path and no speed stands (the holding circle 0x560050 of a speed
//! of 0 is not run). Not ported: formation flight (the group legs 0x560A70 /
//! 0x561080) and the attack release (0x57E030 on the Performer, the bomb):
//! a MoveTick with a pending ability stops with a note.
use crate::scheduler::{EVERY_TICK, Owner};
use crate::trace::sites;
use crate::world::{ActorId, World};
use aa_movement::aircraft::{
    self, AbilityKey, ActorKey, AircraftActor, AircraftDefinition, AircraftError, AircraftResult, AircraftState, AircraftUnit, AircraftWorld, AttackDescriptor, FlightWorld, FormationWorld, GroupKey,
    GroupLayout, GroupWorld, HandleSlot, Pose, TerrainWorld,
};
use aa_movement::falling::{self, MapBounds};
use aa_movement::mover::MoverLimits;
use aa_movement::{ActorMotion, MoverState};
use aa_numeric::{Binary, Precision};
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

const PC: Precision = Precision::Double53;
/// The aircraft Mind's aai_queued state (Mind+0x188).
const QUEUED: u32 = 0x188;

/// A base aircraft's Mover between its MoveTicks.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Mover {
    /// Mover +0x20.
    pub speed: f32,
    /// Actor +0x2C..+0x34.
    pub facing: [f32; 3],
}

/// Whether `a` is an air base's aircraft (CreateDenizen flag +8 bit 2) and
/// not a transport.
pub fn flies(world: &World, a: ActorId) -> bool {
    world.books.actor(a).is_some_and(|b| b.transport.is_none() && b.aircraft.as_ref().is_some_and(|x| x.denizen))
}

/// 0x576B20 at the aircraft's creation: the Mover registers on queue 0
/// (requested 0.1, factor 1), and the Mover state starts at rest facing the
/// actor's heading.
pub(crate) fn registered(world: &mut World, a: ActorId) {
    let facing = world.actors[a as usize].facing;
    world.books.actor_mut(a).flight_mover = Some(Mover { speed: 0.0, facing: [facing.cos(), facing.sin(), 0.0] });
    world.schedule_periodic_at(sites::INIT_EXISTING_AIRCRAFT, EVERY_TICK, Owner::Mover(a), 0);
}

/// 0x560DB0 of a base aircraft: the legs to `point` from the end of its
/// path (`direction` zero: none). Any other aircraft goes to the movement
/// owner's entry point.
pub(crate) fn fly_to(world: &mut World, a: ActorId, point: Vec3, direction: Vec3) {
    if !flies(world, a) {
        crate::movement::fly_to(world, a, point, direction);
        return;
    }
    let dir = (direction != Vec3::ZERO).then_some([direction.x, direction.y, direction.z]);
    with_unit(world, a, |unit, host| aircraft::fly_to(unit, [point.x, point.y, point.z], dir, PC, host));
}

/// The Mover Tick of a base aircraft: 0x577E50 -> MoveTick 0x5612F0, at the
/// path end Mind +0x144 (0x517DD0).
pub fn tick(world: &mut World, a: ActorId) {
    if !world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed) {
        world.unschedule(Owner::Mover(a));
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

fn world_error(what: impl std::fmt::Display) -> AircraftError {
    AircraftError::Movement(aa_movement::Error::World(what.to_string()))
}

fn map_bounds(world: &World) -> MapBounds {
    let c = world.map.cell_size;
    MapBounds { cell_size: c, extent: [c * world.map.width as f32, c * world.map.height as f32] }
}

fn mover_of(speed: f32) -> MoverState {
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

/// The world of a base aircraft's flight: alone, no formation, no release.
struct Host<'a> {
    world: &'a mut World,
    a: ActorId,
    exhausted: bool,
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
    /// Mind virtual +0x1B0 = 0x51C1D0: 0x52A000 on aai_queued (Mind+0x188),
    /// whether it is the current state.
    fn mind_movement_blocked(&mut self, _: &mut AircraftUnit) -> AircraftResult<bool> {
        Ok(super::current_state(self.world, self.a) == Some(QUEUED))
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
    /// Actor virtual +0x104: the definition's top speed.
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
    /// 0x576BB0: inside the map or clamped into it, Mover +0x20, the pose.
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
        x.motion.speed = speed;
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
        mover: mover_of(mover.speed),
        mover_limits: MoverLimits { max_speed: d.speed_max, turn_radius: d.turn_radius },
        aircraft,
    }
}

/// Run `f` over the unit of base aircraft `a`; its state goes back after.
/// An error becomes a Mind note.
fn with_unit<R>(world: &mut World, a: ActorId, f: impl FnOnce(&mut AircraftUnit, &mut Host<'_>) -> AircraftResult<R>) -> Option<(R, bool)> {
    let mover = world.books.actor(a)?.flight_mover.clone()?;
    let aircraft = world.books.actors.get_mut(a as usize)?.aircraft.as_mut().map(|x| std::mem::replace(&mut x.flight, AircraftState::new(0.0, 0.0)))?;
    let mut unit = unit_of(world, a, aircraft, &mover);
    let mut host = Host { world, a, exhausted: false };
    let result = f(&mut unit, &mut host);
    let exhausted = host.exhausted;
    if let Some(b) = world.books.actors.get_mut(a as usize) {
        if let Some(x) = b.aircraft.as_mut() {
            x.flight = unit.aircraft;
            x.scale = unit.actor.scale;
        }
        b.flight_mover = Some(Mover { speed: unit.mover.speed, facing: unit.motion.facing });
    }
    match result {
        Ok(r) => Some((r, exhausted)),
        Err(e) => {
            world.books.minds.note(format!("aircraft {a} flight: {e}"));
            None
        }
    }
}
