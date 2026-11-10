//! `0x55F160` (thiscall aircraft, args group, leading; AirCraft.cpp): the
//! aircraft move state's group order (`aircraft_move_state` follow).
//!
//! 1. With a usable +0xAC group (virtual +0x14 false), the leader of +0xAC
//!    (`0x5685B0`, reread) has its aircraft's +0xB0 follower group released
//!    and cleared; otherwise (null or unusable +0xAC) the aircraft leaves its
//!    formation (`0x55EDC0`).
//! 2. It joins `group` (`0x55EB30`).
//! 3. With a non-null `leading` group whose leader (`0x5685B0`) resolves, that
//!    leader's aircraft +0xB0 takes this aircraft's +0xA8 group (`0x7A1030`:
//!    retain the new one, release the old one, store).
//! 4. +0xAC takes `leading` the same way.
//!
//! The leader's aircraft is its actor +0xBC component (this unit's own when
//! the leader is this aircraft's actor; absent is the native null
//! dereference). A group whose deleting destructor ran is not used again here;
//! the native would read freed memory, which is an error.
use crate::aircraft::{
    ActorKey, AircraftError, AircraftResult, AircraftState, AircraftUnit, GroupKey, GroupWorld,
    group_leader, join_formation, leave_formation,
};

fn aircraft_of<'a, W: GroupWorld + ?Sized>(
    unit: &'a mut AircraftUnit,
    world: &'a mut W,
    actor: ActorKey,
) -> AircraftResult<&'a mut AircraftState> {
    if actor == unit.actor.key { Ok(&mut unit.aircraft) } else { world.member_aircraft(actor) }
}

struct Released(Vec<GroupKey>);
impl Released {
    fn live(&self, group: GroupKey) -> AircraftResult<()> {
        if self.0.contains(&group) {
            return Err(AircraftError::World(format!("group {} is used after its deleting destructor", group.0)));
        }
        Ok(())
    }
    fn release<W: GroupWorld + ?Sized>(&mut self, world: &mut W, group: GroupKey) -> AircraftResult<()> {
        self.live(group)?;
        if world.release_group(group)? {
            self.0.push(group);
        }
        Ok(())
    }
    fn retain<W: GroupWorld + ?Sized>(&self, world: &mut W, group: GroupKey) -> AircraftResult<()> {
        self.live(group)?;
        world.retain_group(group)
    }
    fn check(&self, group: Option<GroupKey>) -> AircraftResult<()> {
        group.map_or(Ok(()), |g| self.live(g))
    }
}

/// `0x7A1030(&slot, &value)`: retain `value`, release the old slot, store.
fn assign<W: GroupWorld + ?Sized>(
    released: &mut Released,
    world: &mut W,
    old: Option<GroupKey>,
    value: Option<GroupKey>,
) -> AircraftResult<bool> {
    if old == value {
        return Ok(false);
    }
    if let Some(v) = value {
        released.retain(world, v)?;
    }
    if let Some(o) = old {
        released.release(world, o)?;
    }
    Ok(true)
}

/// `0x55F160(group, leading)` on this aircraft.
pub fn follow_group<W: GroupWorld + ?Sized>(
    unit: &mut AircraftUnit,
    world: &mut W,
    group: Option<GroupKey>,
    leading: Option<GroupKey>,
) -> AircraftResult<()> {
    let mut released = Released(Vec::new());
    let usable = match unit.aircraft.group_b {
        Some(b) => !world.group_unusable(b)?,
        None => false,
    };
    if usable {
        let b = unit
            .aircraft
            .group_b
            .ok_or(AircraftError::NullDereference("0x55F177 reads the null +0xAC group"))?;
        if let Some(leader) = group_leader(world, b)? {
            let followers = aircraft_of(unit, world, leader)?.group_c;
            if let Some(c) = followers {
                released.release(world, c)?;
                aircraft_of(unit, world, leader)?.group_c = None;
            }
        }
    } else {
        leave_formation(unit, world)?;
    }
    released.check(group)?;
    released.check(unit.aircraft.group_a)?;
    released.check(unit.aircraft.group_b)?;
    join_formation(unit, world, group)?;
    if let Some(l) = leading {
        released.live(l)?;
        if let Some(leader) = group_leader(world, l)? {
            let value = unit.aircraft.group_a;
            let old = aircraft_of(unit, world, leader)?.group_c;
            if assign(&mut released, world, old, value)? {
                aircraft_of(unit, world, leader)?.group_c = unit.aircraft.group_a;
            }
        }
    }
    let old = unit.aircraft.group_b;
    if assign(&mut released, world, old, leading)? {
        unit.aircraft.group_b = leading;
    }
    Ok(())
}
