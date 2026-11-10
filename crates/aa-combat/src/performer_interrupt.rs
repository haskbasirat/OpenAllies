//! Performer (component +74) leave virtual.
//!
//! - `0x57DCB0` (virtual +40): another component takes the actor over. The
//!   Performer's events are cancelled (`0x50B250` on its +8 Updatable); an
//!   ability still in progress (`+10` stage other than 4) tells the owner's
//!   Mind (`+70` virtual +13C); a pending turret update (`+5C`) stays on
//!   (`0x50AC00(1)`).
//!
//! Proven by research/phase8-combat/native_performer_interrupt.py
//! (tests/native_performer_interrupt.rs).
use crate::{Result, constructors::OwnedPerformer};

/// Services of the Performer leave.
pub trait InterruptWorld {
    /// `0x50B250` on the Performer `+8` Updatable.
    fn cancel_events(&mut self) -> Result<()>;
    /// The owner's Mind (`+70`) virtual `+13C`.
    fn mind_interrupted(&mut self, owner: u32) -> Result<()>;
    /// `0x50AC00(1)` on the Performer `+8` Updatable.
    fn activate_updates(&mut self) -> Result<()>;
}

/// Native `0x57DCB0`.
pub fn left_current(performer: &OwnedPerformer, world: &mut impl InterruptWorld) -> Result<()> {
    world.cancel_events()?;
    if performer.state.stage != 4 {
        world.mind_interrupted(performer.state.owner)?;
    }
    if performer.turret_update_pending {
        world.activate_updates()?;
    }
    Ok(())
}
