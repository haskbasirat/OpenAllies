//! Original `KKC_AAIStateLanding::Think` (`519750`, aircraft state
//! `aai_landing` at Mind+174, vtable A32648 slot +C) with the inline helpers
//! `529EE0` ([`super::joinable`]), `5166F0`-equivalent stores and the frame
//! math `517EE0`/`517DF0` ([`super::aai_math`]).
//!
//! State+10 is the base GTID. The base frame is its position +20..+28 and
//! heading +2C/+30; its definition (+4) holds two approach paths selected by
//! the +3A8 byte. Each leg is a point (transformed into the base frame,
//! `517DF0`), a heading (rotated, `517EE0`) and a run; the cruise (flight
//! +18/+80) becomes the leg point z plus the actor definition +40C.
//!
//! Order:
//! 1. `529E60` ([`crate::think_base`]). An unjoinable base (`529EE0`):
//!    `4D7960` on the actor (done). A flight component with a clear +90 byte
//!    and a nonzero +34 word: schedule 0.6 s and the move state Thinks.
//! 2. Path 1 (+3A8 clear): leg (+38C point, +398 heading, +3A0 run) with the
//!    negated heading: fly to point + run * heading, then the speed becomes
//!    actor definition +410 (move +34, flight +14/+84) and a nonzero run
//!    (unordered counts as nonzero) flies to the point itself. A nonzero
//!    +388 run adds the leg (+374 point, +380 heading): fly to the advanced
//!    point, then to the point.
//! 3. Path 2 (+3A8 set): leg (+3C4 point, +3D0 heading, +3D8 run) with the
//!    heading as is: fly to the point, set the speed, and a nonzero run flies
//!    to point + run * heading. A nonzero +3C0 run adds the leg (+3AC point,
//!    +3B8 heading): fly to the point, then to the advanced point.
//! 4. 1e-4 s is scheduled and the move state Thinks.
//!
//! All x87 arithmetic is at the control-word precision; positions are stored
//! binary32 component by component.
use super::{AircraftWorld, IMMEDIATE_BITS, POINT_SIX_BITS, aai_math, aircraft, joinable, null};
use crate::{
    Arena, MindId, Result, StateId, think_base,
    x87::{n, num, store},
};
use aa_numeric::Precision;

/// Services of `519750` in native order.
pub trait LandingThinkWorld: AircraftWorld {
    /// Landing state+10 (base GTID).
    fn landing_base(&mut self, state: StateId) -> Result<u32>;
    /// Object +2C/+30 (binary32 heading).
    fn object_heading(&mut self, object: u32) -> Result<[u32; 2]>;
    /// Object definition (+4) byte at `offset`.
    fn object_definition_byte(&mut self, object: u32, offset: u32) -> Result<u8>;
}

fn defw<const N: usize>(
    world: &mut impl LandingThinkWorld,
    base: u32,
    offsets: [u32; N],
) -> Result<[u32; N]> {
    let mut out = [0; N];
    for (slot, offset) in out.iter_mut().zip(offsets) {
        *slot = world.object_definition_word(base, offset)?;
    }
    Ok(out)
}
/// `fucompp` against 0.0 followed by `jnp`: equal only for a zero value.
fn nonzero(bits: u32) -> bool {
    f32::from_bits(bits) != 0.0
}
fn advance(point: [u32; 3], run: u32, heading: [u32; 2], p: Precision) -> Result<[u32; 3]> {
    let x = store(num(
        num(n(run)?.mul(&n(heading[0])?, p))?.add(&n(point[0])?, p)
    )?)?;
    let y = store(num(
        num(n(run)?.mul(&n(heading[1])?, p))?.add(&n(point[1])?, p)
    )?)?;
    Ok([x, y, point[2]])
}
/// The leg point (`517DF0`) and heading (`517EE0`) for definition offsets
/// `point` (3 words) and `heading` (2 words), and the cruise store.
#[allow(clippy::too_many_arguments)]
fn leg(
    mind: MindId,
    base: u32,
    position: [u32; 3],
    frame: [u32; 2],
    point: u32,
    heading: u32,
    cruise: u32,
    world: &mut impl LandingThinkWorld,
    p: Precision,
) -> Result<([u32; 3], [u32; 2])> {
    let a = defw(world, base, [heading, heading + 4])?;
    let along = aai_math::rotate(frame[0], frame[1], a[0], a[1], p)?;
    let q = defw(world, base, [point, point + 4, point + 8])?;
    let out = aai_math::transform(position, frame, q, p)?;
    let [qz] = defw(world, base, [point + 8])?;
    let level = store(num(n(qz)?.add(&n(cruise)?, p))?)?;
    super::set_flight_cruise(mind, level, world)?;
    Ok((out, along))
}

fn negated(bits: u32) -> u32 {
    bits ^ 0x8000_0000
}

/// Complete `519750` at x87 precision `p`. Returns the base Think's
/// wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl LandingThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let gtid = world.landing_base(state)?;
    let base = world.resolve(gtid)?;
    if !joinable(mind, base, world)? {
        world.actor_4d7960(mind, arena)?;
        return Ok(wrong);
    }
    if world.flight_byte(mind, 0x90)? == 0 && world.flight_word(mind, 0x34)? != 0 {
        world.schedule(mind, POINT_SIX_BITS, 0, arena)?;
        world.embedded_think(mind, aircraft::MOVE, arena)?;
        return Ok(wrong);
    }
    let speed = world.definition_word(mind, 0x410)?;
    let cruise = world.definition_word(mind, 0x40c)?;
    let gtid = world.landing_base(state)?;
    let base = world
        .resolve(gtid)?
        .ok_or_else(|| null("landing base +4 read"))?;
    let frame = world.object_heading(base)?;
    let position = world.object_position3(base)?;
    if world.object_definition_byte(base, 0x3a8)? == 0 {
        let (out, along) = leg(mind, base, position, frame, 0x38c, 0x398, cruise, world, p)?;
        let [run] = defw(world, base, [0x3a0])?;
        let facing = [negated(along[0]), negated(along[1]), 0];
        let target = advance(out, run, along, p)?;
        world.move_fly_to(mind, target, facing, arena)?;
        super::set_flight_speed(mind, speed, world)?;
        if nonzero(run) {
            world.move_fly_to(mind, out, facing, arena)?;
        }
        let [run] = defw(world, base, [0x388])?;
        if nonzero(run) {
            let (out, along) = leg(mind, base, position, frame, 0x374, 0x380, cruise, world, p)?;
            let facing = [negated(along[0]), negated(along[1]), 0];
            let target = advance(out, run, along, p)?;
            world.move_fly_to(mind, target, facing, arena)?;
            world.move_fly_to(mind, out, facing, arena)?;
        }
    } else {
        let (out, along) = leg(mind, base, position, frame, 0x3c4, 0x3d0, cruise, world, p)?;
        let facing = [along[0], along[1], 0];
        world.move_fly_to(mind, out, facing, arena)?;
        let [run] = defw(world, base, [0x3d8])?;
        let target = advance(out, run, along, p)?;
        super::set_flight_speed(mind, speed, world)?;
        if nonzero(run) {
            world.move_fly_to(mind, target, facing, arena)?;
        }
        let [run] = defw(world, base, [0x3c0])?;
        if nonzero(run) {
            let (out, along) = leg(mind, base, position, frame, 0x3ac, 0x3b8, cruise, world, p)?;
            let facing = [along[0], along[1], 0];
            world.move_fly_to(mind, out, facing, arena)?;
            let target = advance(out, run, along, p)?;
            world.move_fly_to(mind, target, facing, arena)?;
        }
    }
    world.schedule(mind, IMMEDIATE_BITS, 0, arena)?;
    world.embedded_think(mind, aircraft::MOVE, arena)?;
    Ok(wrong)
}
