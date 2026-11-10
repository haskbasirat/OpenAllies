//! Mover component activation.
//!
//! - `0x576790(flag)`: the Mover's events are cancelled (`0x50B250` on its
//!   +8 Updatable); when the flag byte is clear the owner's ActorData `+24C`
//!   trigger list runs for the owner (`0x42A590(owner, 0)`).
//!
//! Proven by research/phase8-combat/native_performer_interrupt.py
//! (tests/native_mover_activation.rs).
use crate::Result;

/// Services of the Mover activation.
pub trait ActivationWorld {
    /// `0x50B250` on the Mover `+8` Updatable.
    fn cancel_events(&mut self) -> Result<()>;
    /// `0x42A590(owner, 0)` on the owner's ActorData `+24C` trigger list.
    fn activation_triggers(&mut self, owner: u32) -> Result<()>;
}

/// Native `0x576790`: `flag` is the low byte of its argument.
pub fn activate(owner: u32, flag: bool, world: &mut impl ActivationWorld) -> Result<()> {
    world.cancel_events()?;
    if !flag {
        world.activation_triggers(owner)?;
    }
    Ok(())
}
