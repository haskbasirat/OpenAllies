//! Original `KKC_AAIStateJoin::Think` (`518D60`, aircraft state `aai_join`
//! at Mind+12C, vtable A32600 slot +C) with its inline helper `529EE0`
//! (an object the aircraft can join: live, not removed, and its virtual
//! +174(actor) holds).
//!
//! State fields: +10 join object GTID, +14..+1C the object's last known
//! position, +20..+28 the approach point, +2C byte (arrived).
//!
//! Order:
//! 1. `529E60` ([`crate::think_base`]). With a live join object: if arrived
//!    and its position (+20..+28) differs from state+14 (`428430`, the
//!    vector `!=`, unordered counts as different) it moved; otherwise it
//!    moved when the distance between its
//!    position and state+14 (components stored binary32, `sqrt((dx*dx +
//!    dz*dz) + dy*dy)` unstored) exceeds the +4 value of the struct its
//!    virtual +10 returns.
//! 2. A joinable object (`529EE0`) that did not move skips to step 4. An
//!    unjoinable one is replaced by the Mind home (Mind+28) object, then by
//!    `4E1D90(actor)` on the actor owner (actor virtual +108); if neither is
//!    joinable `4D7960` runs on the actor (done); else Mind virtual +40(the
//!    object).
//! 3. Re-approach: the object (null is a native null read) position goes to
//!    state+14, state+2C is cleared; a definition (+4) whose +370 byte is
//!    clear runs `4D7960` on the actor (done, before the position store).
//!    `518260(object, &state+20, &dir)` on the landing state (not ported;
//!    `dir` starts zero) yields the approach point and direction, and the
//!    aircraft move state flies there (`516740`, movement-owned).
//! 4. Unless arrived: arrived becomes set when twice `45D6F0(0)` on the actor
//!    definition (unstored) exceeds the stored distance `sqrt((dz*dz +
//!    dy*dy) + dx*dx)` from the actor (+20..+28) to state+20. Then the move
//!    state Thinks and 0.6 s is scheduled.
//!
//! All x87 arithmetic is at the control-word precision.
use super::{AircraftWorld, POINT_SIX_BITS, aircraft, joinable, null};
use crate::{
    Arena, MindId, Result, StateId, think_base,
    x87::{n, num, store},
};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// Services of `518D60` in native order.
pub trait AirJoinThinkWorld: AircraftWorld {
    /// Join state+10 (object GTID) with its write.
    fn join_target(&mut self, state: StateId) -> Result<u32>;
    fn set_join_target(&mut self, state: StateId, gtid: u32) -> Result<()>;
    /// Join state+2C byte (arrived) with its write.
    fn arrived(&mut self, state: StateId) -> Result<bool>;
    fn set_arrived(&mut self, state: StateId, value: bool) -> Result<()>;
    /// Join state+14..+1C (last position) with its write, and +20..+28
    /// (approach point) with its write.
    fn last_position(&mut self, state: StateId) -> Result<[u32; 3]>;
    fn set_last_position(&mut self, state: StateId, point: [u32; 3]) -> Result<()>;
    fn approach_point(&mut self, state: StateId) -> Result<[u32; 3]>;
    fn set_approach_point(&mut self, state: StateId, point: [u32; 3]) -> Result<()>;
    /// Object virtual +10: the +4 binary32 of the struct it returns.
    fn object_virtual_10(&mut self, object: u32) -> Result<u32>;
    /// Actor virtual +108 (owner) and `4E1D90(actor)` on it.
    fn actor_owner(&mut self, mind: MindId) -> Result<u32>;
    fn owner_base(&mut self, owner: u32, mind: MindId) -> Result<Option<u32>>;
    /// Mind virtual +40(object).
    fn mind_virtual_40(
        &mut self,
        mind: MindId,
        object: Option<u32>,
        arena: &mut Arena,
    ) -> Result<()>;
    /// Object definition (+4) +370 byte.
    fn definition_370(&mut self, object: u32) -> Result<bool>;
    /// `518260(object, &point, &direction)` on the landing state (not
    /// ported): the point and direction buffers after the call.
    fn landing_approach(
        &mut self,
        mind: MindId,
        object: u32,
        point: [u32; 3],
        direction: [u32; 3],
        arena: &mut Arena,
    ) -> Result<([u32; 3], [u32; 3])>;
    /// `45D6F0(0)` on the actor definition, unstored ST0.
    fn definition_45d6f0(&mut self, mind: MindId) -> Result<Binary>;
}

fn delta(a: [u32; 3], b: [u32; 3], p: Precision) -> Result<[u32; 3]> {
    Ok([
        store(num(n(a[0])?.sub(&n(b[0])?, p))?)?,
        store(num(n(a[1])?.sub(&n(b[1])?, p))?)?,
        store(num(n(a[2])?.sub(&n(b[2])?, p))?)?,
    ])
}
fn square(bits: u32, p: Precision) -> Result<Binary> {
    num(n(bits)?.mul(&n(bits)?, p))
}

/// Complete `518D60` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl AirJoinThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let mut moved = false;
    let gtid = world.join_target(state)?;
    if let Some(object) = world.resolve(gtid)? {
        let at = world.object_position3(object)?;
        let last = world.last_position(state)?;
        if world.arrived(state)? && world.vectors_differ(at, last)? {
            moved = true;
        } else {
            let [dx, dy, dz] = delta(at, last, p)?;
            let limit = world.object_virtual_10(object)?;
            let sum = num(num(square(dx, p)?.add(&square(dz, p)?, p))?.add(&square(dy, p)?, p))?;
            let distance = num(sum.sqrt(p))?;
            if distance.compare(&n(limit)?) == Ordering::Greater {
                moved = true;
            }
        }
    }
    let gtid = world.join_target(state)?;
    let current = world.resolve(gtid)?;
    let reapproach = if joinable(mind, current, world)? {
        moved
    } else {
        let home = arena.mind(mind)?.words_24_30[1];
        let gtid = match world.resolve(home)? {
            Some(h) => world.object_gtid(h)?,
            None => 0,
        };
        world.set_join_target(state, gtid)?;
        let candidate = world.resolve(gtid)?;
        if !joinable(mind, candidate, world)? {
            let owner = world.actor_owner(mind)?;
            let gtid = match world.owner_base(owner, mind)? {
                Some(b) => world.object_gtid(b)?,
                None => 0,
            };
            world.set_join_target(state, gtid)?;
            let candidate = world.resolve(gtid)?;
            if !joinable(mind, candidate, world)? {
                world.actor_4d7960(mind, arena)?;
                return Ok(wrong);
            }
        }
        let gtid = world.join_target(state)?;
        let object = world.resolve(gtid)?;
        world.mind_virtual_40(mind, object, arena)?;
        true
    };
    if reapproach {
        let gtid = world.join_target(state)?;
        let object = world
            .resolve(gtid)?
            .ok_or_else(|| null("join object +20 read"))?;
        let position = world.object_position3(object)?;
        world.set_arrived(state, false)?;
        if !world.definition_370(object)? {
            world.actor_4d7960(mind, arena)?;
            return Ok(wrong);
        }
        world.set_last_position(state, position)?;
        let point = world.approach_point(state)?;
        let (point, direction) = world.landing_approach(mind, object, point, [0; 3], arena)?;
        world.set_approach_point(state, point)?;
        world.move_fly_to(mind, point, direction, arena)?;
    }
    if !world.arrived(state)? {
        let target = world.approach_point(state)?;
        let actor = world.actor_position3(mind)?;
        let [dx, dy, dz] = delta(target, actor, p)?;
        let sum = num(num(square(dz, p)?.add(&square(dy, p)?, p))?.add(&square(dx, p)?, p))?;
        let distance = store(num(sum.sqrt(p))?)?;
        let reach = world.definition_45d6f0(mind)?;
        let twice = num(reach.add(&reach, p))?;
        if twice.compare(&n(distance)?) == Ordering::Greater {
            world.set_arrived(state, true)?;
        }
    }
    world.embedded_think(mind, aircraft::MOVE, arena)?;
    world.schedule(mind, POINT_SIX_BITS, 0, arena)?;
    Ok(wrong)
}
