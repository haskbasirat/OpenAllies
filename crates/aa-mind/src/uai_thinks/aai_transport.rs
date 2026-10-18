//! Original `KKC_AAIStateTransport::Think` (`5185D0`, aircraft state
//! `aai_transport` at Mind+198, vtable A32490 slot +C).
//!
//! Order:
//! 1. `529E60` ([`crate::think_base`]).
//! 2. With the state+11 byte set and `517B60` (transport step, not ported)
//!    false: actor virtual +104 (ST0) is stored binary32 to the move state
//!    +34 (Mind+68) and to the flight component (actor+BC) +14 and +84. A
//!    base (actor+C0; null is a native null read) with the +4C byte orders
//!    the aircraft move state (`516740`, movement-owned) to the base +50/+54
//!    point (z 0) facing the negated actor +2C/+30/+34 vector, and clears
//!    state+11. Without it the join state is activated in mode 2 (`5176A0`),
//!    1e-4 s is scheduled and the Think ends.
//! 3. The aircraft move state (Mind+34) Thinks and 0.6 s is scheduled.
use super::{AircraftWorld, IMMEDIATE_BITS, POINT_SIX_BITS, aircraft, null, set_flight_speed};
use crate::{Arena, MindId, Result, StateId, think_base, x87::store};

/// Services of `5185D0` in native order.
pub trait TransportThinkWorld: AircraftWorld {
    /// Transport state+11 byte (pending) and its write.
    fn pending(&mut self, state: StateId) -> Result<bool>;
    fn set_pending(&mut self, state: StateId, value: bool) -> Result<()>;
    /// `517B60` on the transport state (not ported).
    fn transport_step(&mut self, state: StateId, arena: &mut Arena) -> Result<bool>;
    /// Actor +C0 (base), `None` when null; its +4C byte and +50/+54 point.
    fn base(&mut self, mind: MindId) -> Result<Option<u32>>;
    fn base_released(&mut self, base: u32) -> Result<bool>;
    fn base_landing_point(&mut self, base: u32) -> Result<[u32; 2]>;
    /// Actor +2C/+30/+34 (binary32 vector).
    fn actor_vector(&mut self, mind: MindId) -> Result<[u32; 3]>;
}

/// `fld dword; fchs; fstp dword` of a finite binary32.
fn negated(bits: u32) -> u32 {
    bits ^ 0x8000_0000
}

/// Complete `5185D0`. Returns the base Think's wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl TransportThinkWorld,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    if world.pending(state)? && !world.transport_step(state, arena)? {
        let value = store(world.actor_virtual_104(mind)?)?;
        set_flight_speed(mind, value, world)?;
        let base = world.base(mind)?.ok_or_else(|| null("base +4C read"))?;
        if !world.base_released(base)? {
            world.join_activate(mind, 2, arena)?;
            world.schedule(mind, IMMEDIATE_BITS, 0, arena)?;
            return Ok(wrong);
        }
        let [x, y, z] = world.actor_vector(mind)?;
        let direction = [negated(x), negated(y), negated(z)];
        let [px, py] = world.base_landing_point(base)?;
        world.move_fly_to(mind, [px, py, 0], direction, arena)?;
        world.set_pending(state, false)?;
    }
    world.embedded_think(mind, aircraft::MOVE, arena)?;
    world.schedule(mind, POINT_SIX_BITS, 0, arena)?;
    Ok(wrong)
}
