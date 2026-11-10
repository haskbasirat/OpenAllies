//! Denizen slot health.
//!
//! - `0x5690F0`: a denizen slot (`+0` handle, `+8` binary32 health) reads
//!   the health of the denizen's Body (actor `+60`, Body `+10`) while the
//!   handle names a live slot of the handle table and the actor has a Body;
//!   otherwise the slot's own `+8`. The actor's `+8` flags are not
//!   consulted. Every path loads one binary32.
//!
//! Proven by research/phase8-combat/native_denizen_slot.py
//! (tests/native_denizen_slot.rs).
use crate::Result;
use aa_body::F32;

/// Services of the slot health.
pub trait DenizenSlotWorld {
    /// The handle table (`[0xB364C0]`): the actor in the slot when the
    /// generation matches and the slot is filled.
    fn resolve(&self, handle: u32) -> Result<Option<u32>>;
    /// Actor `+60` Body `+10`, None without a Body.
    fn body_health(&self, actor: u32) -> Result<Option<F32>>;
}

/// Native `0x5690F0` for the slot (`handle`, `stored` health).
pub fn slot_health(handle: u32, stored: F32, world: &impl DenizenSlotWorld) -> Result<F32> {
    if let Some(actor) = world.resolve(handle)?
        && let Some(health) = world.body_health(actor)?
    {
        return Ok(health);
    }
    Ok(stored)
}
