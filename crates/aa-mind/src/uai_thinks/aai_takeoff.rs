//! Original `KKC_AAIStateTakeoff::Think` (`519150`, aircraft state
//! `aai_takeoff` at Mind+15C, vtable A32624 slot +C) with the inline helpers
//! `529EE0` ([`super::joinable`]), `516720`/`5166F0`
//! ([`super::set_flight_cruise`] / [`super::set_flight_speed`]) and the frame
//! math `517EE0`/`517DF0` ([`super::aai_math`]).
//!
//! State fields: +10 byte (launched), +14 base GTID. The base object frame is
//! its position +20..+28 and heading +2C/+30; its definition (+4) holds the
//! takeoff path: +374..+37C (first point), +380/+384 (first heading),
//! +388 (first run), +38C..+394 (second point), +398/+39C (second heading),
//! +3A0 (second run).
//!
//! Order:
//! 1. `529E60` ([`crate::think_base`]). Launched: finish the state
//!    (`52A340`, [`crate::pop`]), clear +10 and flight +10, schedule 0 s.
//! 2. A flight component with a clear +90 byte and a nonzero +34 word:
//!    schedule 0.6 s and the move state Thinks.
//! 3. An unjoinable base (`529EE0`): `4D7960` on the actor.
//! 4. Flight +18/+80 = definition +40C, flight +8C = definition +410. The
//!    first heading is rotated into the base frame (`517EE0`); with a
//!    positive first run the first point is transformed (`517DF0`), the
//!    cruise becomes point z + definition +40C, the run is added along the
//!    heading, the speed becomes definition +410 and the aircraft flies
//!    there (`516740`). When the second point differs (`428430`) the same is
//!    done with the second point and heading (no run, no speed). A positive
//!    second run is added along the heading, the speed becomes actor virtual
//!    +104 and the aircraft flies there; otherwise flight +8C becomes actor
//!    virtual +104 and the run is 6.0 (`[A32694]`). Then four times the run
//!    (`[9FD59C]` 4.0) is added along the heading, the cruise becomes
//!    definition +414 (flight +18 then +80) and the aircraft flies there; 1e-4 s
//!    is scheduled and the move state Thinks.
use super::{
    AircraftWorld, IMMEDIATE_BITS, POINT_SIX_BITS, aai_math, aircraft, joinable, null,
    set_flight_cruise, set_flight_speed,
};
use crate::{
    Arena, Result, StateId, pop, think_base,
    x87::{n, num, store},
};
use aa_numeric::{Binary, Precision};

/// `[A32694]` binary32 6.0: the run when the second run is not positive.
pub const DEFAULT_RUN_BITS: u32 = 0x40c0_0000;
/// `[9FD59C]` binary32 4.0: the final run multiplier.
pub const FINAL_RUN_SCALE_BITS: u32 = 0x4080_0000;

/// Services of `519150` in native order.
pub trait TakeoffThinkWorld: AircraftWorld {
    /// Takeoff state+10 byte (launched) with its write, and +14 (base GTID).
    fn launched(&mut self, state: StateId) -> Result<bool>;
    fn set_launched(&mut self, state: StateId, value: bool) -> Result<()>;
    fn takeoff_base(&mut self, state: StateId) -> Result<u32>;
    /// Object +2C/+30 (binary32 heading).
    fn object_heading(&mut self, object: u32) -> Result<[u32; 2]>;
}

/// Base definition words at `offsets`.
fn defw<const N: usize>(
    world: &mut impl TakeoffThinkWorld,
    base: u32,
    offsets: [u32; N],
) -> Result<[u32; N]> {
    let mut out = [0; N];
    for (slot, offset) in out.iter_mut().zip(offsets) {
        *slot = world.object_definition_word(base, offset)?;
    }
    Ok(out)
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}
/// `along + run * heading` for x and y, each stored binary32.
fn advance(position: [u32; 2], run: &Binary, heading: [u32; 2], p: Precision) -> Result<[u32; 2]> {
    let x = store(num(
        num(run.mul(&n(heading[0])?, p))?.add(&n(position[0])?, p)
    )?)?;
    let y = store(num(
        num(run.mul(&n(heading[1])?, p))?.add(&n(position[1])?, p)
    )?)?;
    Ok([x, y])
}

/// Complete `519150` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl TakeoffThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    if world.launched(state)? {
        pop(mind, arena, world)?;
        world.set_launched(state, false)?;
        world.set_flight_byte(mind, 0x10, 0)?;
        world.schedule(mind, 0, 0, arena)?;
        return Ok(wrong);
    }
    if world.flight_byte(mind, 0x90)? == 0 && world.flight_word(mind, 0x34)? != 0 {
        world.schedule(mind, POINT_SIX_BITS, 0, arena)?;
        world.embedded_think(mind, aircraft::MOVE, arena)?;
        return Ok(wrong);
    }
    let gtid = world.takeoff_base(state)?;
    let base = world.resolve(gtid)?;
    if !joinable(mind, base, world)? {
        world.actor_4d7960(mind, arena)?;
        return Ok(wrong);
    }
    let speed = world.definition_word(mind, 0x410)?;
    let cruise = world.definition_word(mind, 0x40c)?;
    let gtid = world.takeoff_base(state)?;
    let base = world
        .resolve(gtid)?
        .ok_or_else(|| null("takeoff base +2C read"))?;
    let point = world.object_position3(base)?;
    let heading = world.object_heading(base)?;
    set_flight_cruise(mind, cruise, world)?;
    world.set_flight_word(mind, 0x8c, speed)?;
    let a1 = defw(world, base, [0x380, 0x384])?;
    let mut along = aai_math::rotate(heading[0], heading[1], a1[0], a1[1], p)?;
    let mut facing = [along[0], along[1], 0];
    let mut position = [point[0], point[1]];
    let mut z = point[2];
    let [run1] = defw(world, base, [0x388])?;
    if host(run1) > 0.0 {
        let q1 = defw(world, base, [0x374, 0x378, 0x37c])?;
        let out = aai_math::transform(point, heading, q1, p)?;
        position = [out[0], out[1]];
        z = out[2];
        let [q1z] = defw(world, base, [0x37c])?;
        set_flight_cruise(mind, store(num(n(q1z)?.add(&n(cruise)?, p))?)?, world)?;
        position = advance(position, &n(run1)?, along, p)?;
        set_flight_speed(mind, speed, world)?;
        world.move_fly_to(mind, [position[0], position[1], z], facing, arena)?;
    }
    let q2 = defw(world, base, [0x38c, 0x390, 0x394])?;
    let q1 = defw(world, base, [0x374, 0x378, 0x37c])?;
    if world.vectors_differ(q1, q2)? {
        let a2 = defw(world, base, [0x398, 0x39c])?;
        along = aai_math::rotate(heading[0], heading[1], a2[0], a2[1], p)?;
        let q2 = defw(world, base, [0x38c, 0x390, 0x394])?;
        let out = aai_math::transform(point, heading, q2, p)?;
        let [q2z] = defw(world, base, [0x394])?;
        set_flight_cruise(mind, store(num(n(q2z)?.add(&n(cruise)?, p))?)?, world)?;
        position = [out[0], out[1]];
        z = out[2];
        facing = [along[0], along[1], facing[2]];
        world.move_fly_to(mind, [position[0], position[1], z], facing, arena)?;
    }
    let run2 = world.object_definition_word(base, 0x3a0)?;
    let run = if host(run2) > 0.0 {
        position = advance(position, &n(run2)?, along, p)?;
        let v = store(world.actor_virtual_104(mind)?)?;
        set_flight_speed(mind, v, world)?;
        world.move_fly_to(mind, [position[0], position[1], z], facing, arena)?;
        n(run2)?
    } else {
        let v = store(world.actor_virtual_104(mind)?)?;
        world.set_flight_word(mind, 0x8c, v)?;
        n(DEFAULT_RUN_BITS)?
    };
    let scale = n(FINAL_RUN_SCALE_BITS)?;
    let x = num(num(n(along[0])?.mul(&run, p))?.mul(&scale, p))?;
    position[0] = store(num(x.add(&n(position[0])?, p))?)?;
    let y = num(num(run.mul(&n(along[1])?, p))?.mul(&scale, p))?;
    position[1] = store(num(y.add(&n(position[1])?, p))?)?;
    let final_cruise = world.definition_word(mind, 0x414)?;
    world.set_flight_word(mind, 0x18, final_cruise)?;
    world.set_flight_word(mind, 0x80, final_cruise)?;
    world.move_fly_to(mind, [position[0], position[1], z], facing, arena)?;
    world.schedule(mind, IMMEDIATE_BITS, 0, arena)?;
    world.embedded_think(mind, aircraft::MOVE, arena)?;
    Ok(wrong)
}
