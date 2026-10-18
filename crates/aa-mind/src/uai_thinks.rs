//! Remaining element (UAI) and aircraft (AAI) Mind-state Thinks, one
//! submodule per Think in `src/uai_thinks/` (owned by the UAI/AAI helper).
//!
//! Every Think is ported from the original instructions and replayed against
//! a Unicorn oracle fixture (`research/phase8-ai/native-uai-*.json`,
//! `native-aai-*.json`). Embedded states are addressed by their native offset
//! inside the Mind ([`ThinkWorld`]); unported callees are explicit world
//! services of each Think's trait.
use crate::{Arena, MindId, Result, StateId, World};

pub mod aai_attack;
pub mod aai_escort;
pub mod aai_guard;
pub mod aai_join;
pub mod aai_landing;
pub mod aai_math;
pub mod aai_queued;
pub mod aai_recon;
pub mod aai_takeoff;
pub mod aai_transport;
pub mod uai_bombard;
pub mod uai_engage;
pub mod uai_join;
pub mod uai_reattach;
pub mod uai_repair;
pub mod uai_sleep;
pub mod uai_unload;
pub mod uai_unpack;

/// Element Mind embedded state offsets (inventory.json `mind_states`).
pub mod element {
    pub const UNLOAD: u32 = 0x34;
    pub const DEPLOY: u32 = 0x48;
    pub const SLEEP: u32 = 0x68;
    pub const MOVE: u32 = 0x7c;
    pub const KILL: u32 = 0xf8;
    pub const BOMBARD: u32 = 0x10c;
    pub const ENGAGE: u32 = 0x12c;
    pub const REPAIR: u32 = 0x148;
    pub const JOIN: u32 = 0x164;
    pub const REATTACH: u32 = 0x178;
    pub const UNPACK: u32 = 0x188;
}

/// Aircraft Mind embedded state offsets (inventory.json `mind_states`).
pub mod aircraft {
    pub const MOVE: u32 = 0x34;
    pub const GUARD: u32 = 0x9c;
    pub const RECON: u32 = 0xb4;
    pub const ESCORT: u32 = 0xd0;
    pub const ATTACK: u32 = 0xe8;
    pub const JOIN: u32 = 0x12c;
    pub const TAKEOFF: u32 = 0x15c;
    pub const LANDING: u32 = 0x174;
    pub const QUEUED: u32 = 0x188;
    pub const TRANSPORT: u32 = 0x198;
}

/// binary32 1e-4 (`0x38D1B717`, also `[A34770]`): "immediately".
pub const IMMEDIATE_BITS: u32 = 0x38d1_b717;
/// binary32 0.3 (`0x3E99999A`).
pub const POINT_THREE_BITS: u32 = 0x3e99_999a;
/// binary32 0.6 (`0x3F19999A`).
pub const POINT_SIX_BITS: u32 = 0x3f19_999a;
/// binary32 1.0.
pub const ONE_BITS: u32 = 0x3f80_0000;

/// Services shared by these Thinks. `World` drives the Mind primitives
/// (enter/leave/schedule/...); embedded states are addressed by native offset.
pub trait ThinkWorld: World {
    /// Identity of the embedded state at Mind+`offset`.
    fn embedded_state(&mut self, mind: MindId, offset: u32) -> Result<StateId>;
    /// Virtual +C (Think) of the embedded state at Mind+`offset`.
    fn embedded_think(&mut self, mind: MindId, offset: u32, arena: &mut Arena) -> Result<()>;
    /// GTID registry `[B364C0]` lookup: the entry for a matching 16-bit
    /// generation, `None` for a mismatch or a null entry.
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>>;
    /// Object +14 (its GTID).
    fn object_gtid(&mut self, object: u32) -> Result<u32>;
    /// `[B279C0]/[B279C4]` (the null point, binary32 words).
    fn null_point(&mut self) -> Result<[u32; 2]>;
    /// Mind virtual +140(GTID by value): the state's object is lost.
    fn mind_virtual_140(&mut self, mind: MindId, gtid: u32, arena: &mut Arena) -> Result<()>;
    /// Actor +E4 (current performer), `None` when null.
    fn current_performer(&mut self, mind: MindId) -> Result<Option<u32>>;
    /// Actor +6C (locomotion), `None` when null.
    fn locomotion(&mut self, mind: MindId) -> Result<Option<u32>>;
    /// `576AD0(object, delay)` on a locomotion.
    fn locomotion_follow(&mut self, locomotion: u32, object: u32, delay_bits: u32) -> Result<()>;
}

/// Aircraft Mind services shared by the AAI Thinks.
pub trait AircraftWorld: ThinkWorld {
    /// `5176A0(mode)` on the aircraft join state (Mind+12C, not ported):
    /// resets the flight component (+BC) and join fields, then activates the
    /// join state in `mode` (`52A810`); its result is unused by the callers.
    fn join_activate(&mut self, mind: MindId, mode: u32, arena: &mut Arena) -> Result<()>;
    /// Object +8 bit 0 (removed).
    fn object_removed(&mut self, object: u32) -> Result<bool>;
    /// Object virtual +174(actor).
    fn object_accepts(&mut self, object: u32, mind: MindId) -> Result<bool>;
    /// `4D7960` on the actor.
    fn actor_4d7960(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// Actor virtual +104, unstored ST0.
    fn actor_virtual_104(&mut self, mind: MindId) -> Result<aa_numeric::Binary>;
    /// Actor definition (+4) word at `offset`.
    fn definition_word(&mut self, mind: MindId, offset: u32) -> Result<u32>;
    /// Flight component (actor+BC) word and byte at `offset`, and writes.
    fn flight_word(&mut self, mind: MindId, offset: u32) -> Result<u32>;
    fn set_flight_word(&mut self, mind: MindId, offset: u32, bits: u32) -> Result<()>;
    fn flight_byte(&mut self, mind: MindId, offset: u32) -> Result<u8>;
    fn set_flight_byte(&mut self, mind: MindId, offset: u32, value: u8) -> Result<()>;
    /// Aircraft move state +34 (Mind+68) write.
    fn set_move_word34(&mut self, mind: MindId, bits: u32) -> Result<()>;
    /// `516740(position, direction)` on the aircraft move state.
    fn move_fly_to(
        &mut self,
        mind: MindId,
        position: [u32; 3],
        direction: [u32; 3],
        arena: &mut Arena,
    ) -> Result<()>;
    /// `428430` (vector `!=`): any binary32 component compares unequal or
    /// unordered.
    fn vectors_differ(&mut self, a: [u32; 3], b: [u32; 3]) -> Result<bool>;
    /// Object +20..+28 (binary32 position).
    fn object_position3(&mut self, object: u32) -> Result<[u32; 3]>;
    /// Object definition (+4) word at `offset`.
    fn object_definition_word(&mut self, object: u32, offset: u32) -> Result<u32>;
    /// `4D4FF0(object, a, b)` on the actor.
    fn relation(&mut self, mind: MindId, object: Option<u32>, a: u32, b: u32) -> Result<i32>;
    /// Object +20/+24 and the Mind actor +20/+24 (binary32).
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]>;
    fn actor_position(&mut self, mind: MindId) -> Result<[u32; 2]>;
    /// The Mind actor +20..+28 (binary32).
    fn actor_position3(&mut self, mind: MindId) -> Result<[u32; 3]>;
    /// Object word and actor word at `offset` (other fields).
    fn object_word(&mut self, object: u32, offset: u32) -> Result<u32>;
    fn actor_word(&mut self, mind: MindId, offset: u32) -> Result<u32>;
}

/// `516720(value)` on the aircraft move state: flight +18 and +80.
pub fn set_flight_cruise(mind: MindId, bits: u32, world: &mut impl AircraftWorld) -> Result<()> {
    world.set_flight_word(mind, 0x18, bits)?;
    world.set_flight_word(mind, 0x80, bits)
}

/// `5166F0(value)` on the aircraft move state: move +34, flight +14 and
/// +84.
pub fn set_flight_speed(mind: MindId, bits: u32, world: &mut impl AircraftWorld) -> Result<()> {
    world.set_move_word34(mind, bits)?;
    world.set_flight_word(mind, 0x14, bits)?;
    world.set_flight_word(mind, 0x84, bits)
}

/// `529EE0(object)` on an aircraft Mind: a non-null object that is not
/// removed and whose virtual +174(actor) holds.
pub fn joinable(mind: MindId, object: Option<u32>, world: &mut impl AircraftWorld) -> Result<bool> {
    match object {
        Some(o) if !world.object_removed(o)? => world.object_accepts(o, mind),
        _ => Ok(false),
    }
}

/// Status (+C) of the embedded state at Mind+`offset`.
pub fn status_at(
    world: &mut impl ThinkWorld,
    mind: MindId,
    offset: u32,
    arena: &Arena,
) -> Result<u32> {
    let s = world.embedded_state(mind, offset)?;
    Ok(arena.state(s)?.status)
}

/// `529C20`: leave the embedded state at Mind+`offset` when its status is
/// nonzero (state virtual +1C).
pub fn leave_if_active(
    world: &mut impl ThinkWorld,
    mind: MindId,
    offset: u32,
    arena: &mut Arena,
) -> Result<()> {
    let s = world.embedded_state(mind, offset)?;
    if arena.state(s)?.status != 0 {
        world.leave(s, arena)?;
    }
    Ok(())
}

/// A native call through a null `this` (or a null field read) is returned as
/// an error after the earlier effects stay committed.
pub(crate) fn null(what: &str) -> crate::Error {
    crate::Error::World(format!("native null dereference: {what}"))
}
