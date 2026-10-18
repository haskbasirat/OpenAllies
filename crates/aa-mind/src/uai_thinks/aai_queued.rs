//! Original `KKC_AAIStateQueued::Think` (`519F70`, aircraft state
//! `aai_queued` at Mind+188, vtable A3246C slot +C): an aircraft waiting
//! for its base (actor+C0) to release it.
//!
//! Order (note that `529E60` runs only on the waiting branch):
//! 1. A base whose +4C byte is set releases the aircraft: actor virtual
//!    +104 (ST0, stored binary32) goes to the flight component (actor+BC)
//!    +8C, the definition +414 word to flight +18 and +80, flight +10 byte is
//!    cleared, and the transport state (Mind+198) is activated in mode 1 at
//!    the base +30/+34 point (`518550`). Done.
//! 2. Otherwise, when the home GTID (Mind+28) does not resolve, `4D7960`
//!    runs on the actor; then `529E60` ([`crate::think_base`]) and 1.0 s is
//!    scheduled.
use super::{AircraftWorld, ONE_BITS};
use crate::{Arena, MindId, Result, StateId, think_base, x87::store};

/// Services of `519F70` in native order.
pub trait QueuedThinkWorld: AircraftWorld {
    /// Actor +C0 (base), `None` when null, and its +4C byte (released).
    fn base(&mut self, mind: MindId) -> Result<Option<u32>>;
    fn base_released(&mut self, base: u32) -> Result<bool>;
    /// Base +30/+34 (binary32 point).
    fn base_point(&mut self, base: u32) -> Result<[u32; 2]>;
    /// `518550(mode, point)` on the transport state (Mind+198, not ported).
    fn transport_activate(
        &mut self,
        mind: MindId,
        mode: u32,
        point: [u32; 2],
        arena: &mut Arena,
    ) -> Result<()>;
}

/// Complete `519F70`. Returns the base Think's wrong-activity flag (false
/// on the release branch, where `529E60` does not run).
pub fn think(state: StateId, arena: &mut Arena, world: &mut impl QueuedThinkWorld) -> Result<bool> {
    let mind = arena.state(state)?.parent;
    if let Some(base) = world.base(mind)?
        && world.base_released(base)?
    {
        let value = store(world.actor_virtual_104(mind)?)?;
        world.set_flight_word(mind, 0x8c, value)?;
        let word = world.definition_word(mind, 0x414)?;
        world.set_flight_word(mind, 0x18, word)?;
        world.set_flight_word(mind, 0x80, word)?;
        world.set_flight_byte(mind, 0x10, 0)?;
        let point = world.base_point(base)?;
        world.transport_activate(mind, 1, point, arena)?;
        return Ok(false);
    }
    let home = arena.mind(mind)?.words_24_30[1];
    if world.resolve(home)?.is_none() {
        world.actor_4d7960(mind, arena)?;
    }
    let wrong = think_base(state, arena)?;
    world.schedule(mind, ONE_BITS, 0, arena)?;
    Ok(wrong)
}
