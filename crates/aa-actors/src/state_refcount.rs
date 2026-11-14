//! Original reference-counted actor state 38 (0x26, "unloading"): `4D7D30`
//! takes a hold and `4D7D70` releases one; the count is actor +174 and the
//! state follows it through `4D64F0(0x26, enabled)` (aa_actors::state::set).
//! Both take one stack argument that they do not read.
use crate::state::Result;

/// Actor state index the count drives.
pub const UNLOADING_STATE: u8 = 38;

/// Services of `4D7D30` / `4D7D70`. The count is reread after the state
/// callback, as the original does.
pub trait StateHoldWorld {
    /// Actor +174 (signed count) and its write.
    fn hold_count(&mut self) -> Result<i32>;
    fn set_hold_count(&mut self, value: i32) -> Result<()>;
    /// `4D64F0(index, enabled)` on the actor.
    fn set_state(&mut self, index: u8, enabled: bool) -> Result<()>;
}

/// `4D7D30`: a negative count is reset to 0; a count of 0 enables state 38;
/// then the count (reread) is incremented.
pub fn hold(world: &mut impl StateHoldWorld) -> Result<()> {
    let mut count = world.hold_count()?;
    if count < 0 {
        world.set_hold_count(0)?;
        count = world.hold_count()?;
    }
    if count == 0 {
        world.set_state(UNLOADING_STATE, true)?;
    }
    let count = world.hold_count()?;
    world.set_hold_count(count.wrapping_add(1))
}

/// `4D7D70`: the count is decremented (a negative result is reset to 0);
/// a count of 0 disables state 38.
pub fn release(world: &mut impl StateHoldWorld) -> Result<()> {
    let count = world.hold_count()?.wrapping_sub(1);
    world.set_hold_count(count)?;
    if count < 0 {
        world.set_hold_count(0)?;
    }
    if world.hold_count()? == 0 {
        world.set_state(UNLOADING_STATE, false)?;
    }
    Ok(())
}
