//! Original `52E610(speed)` on the element move state (Mind+7C): store the
//! requested speed and either stop or drive the locomotion.
//!
//! Order of the original:
//! 1. state+24 = `speed`.
//! 2. `4CE6C0(actor)` picks the holder of the group GTID: the actor itself,
//!    unless its +7C is null and its membership (+80) has an organization
//!    (+10), which is then used.
//! 3. A holder +164 GTID that resolves, with `speed` below the SVar at
//!    `[B39AB0]+190`: stop (`52D660`).
//! 4. Otherwise `speed` below `[9FE0C0]` (binary32 0.1): stop (`52D660`).
//! 5. Otherwise `576D20(speed, 1.0)` on the actor +6C locomotion.
//!
//! The compares are of binary32 loads (ordered; an unordered speed drives).
use crate::Result;

/// `[9FE0C0]` (binary32 0.1): below it the unit stops.
pub const STOP_SPEED_BITS: u32 = 0x3dcc_cccd;
/// binary32 1.0: the factor passed to `576D20`.
pub const SPEED_FACTOR_BITS: u32 = 0x3f80_0000;

/// Holder of the group GTID chosen by `4CE6C0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupHolder {
    Actor,
    Organization(u32),
}

/// Services of `52E610`, in native order.
pub trait ElementSpeedWorld {
    /// Move state +24 write (requested speed, binary32).
    fn set_requested_speed(&mut self, bits: u32) -> Result<()>;
    /// Actor +7C is not null.
    fn actor_has_7c(&mut self) -> Result<bool>;
    /// Actor +80 (membership) -> +10 (organization), `None` when either is
    /// null.
    fn organization(&mut self) -> Result<Option<u32>>;
    /// Holder +164 (group GTID).
    fn group_gtid(&mut self, holder: GroupHolder) -> Result<u32>;
    /// GTID registry lookup (`None` for a stale or null entry).
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>>;
    /// SVar at `[B39AB0]+190` (binary32).
    fn group_stop_speed(&mut self) -> Result<u32>;
    /// `52D660` on the move state (crate::element_move::stop).
    fn stop(&mut self) -> Result<()>;
    /// `576D20(speed, factor)` on the actor +6C locomotion (not null-checked).
    fn locomotion_set_speed(&mut self, speed_bits: u32, factor_bits: u32) -> Result<()>;
}

/// `4CE6C0(actor)`.
pub fn group_holder(world: &mut impl ElementSpeedWorld) -> Result<GroupHolder> {
    if world.actor_has_7c()? {
        return Ok(GroupHolder::Actor);
    }
    Ok(match world.organization()? {
        Some(o) => GroupHolder::Organization(o),
        None => GroupHolder::Actor,
    })
}

fn below(a: u32, b: u32) -> bool {
    f32::from_bits(a) < f32::from_bits(b)
}

/// Complete `52E610(speed)`.
pub fn set_speed(speed_bits: u32, world: &mut impl ElementSpeedWorld) -> Result<()> {
    world.set_requested_speed(speed_bits)?;
    let holder = group_holder(world)?;
    let gtid = world.group_gtid(holder)?;
    if world.resolve(gtid)?.is_some() && below(speed_bits, world.group_stop_speed()?) {
        return world.stop();
    }
    if below(speed_bits, STOP_SPEED_BITS) {
        return world.stop();
    }
    world.locomotion_set_speed(speed_bits, SPEED_FACTOR_BITS)
}

/// Services of `52D620`.
pub trait SpeedScaleWorld {
    /// Move state +24 (requested speed, binary32) and its write.
    fn requested_speed(&mut self) -> Result<u32>;
    fn set_requested_speed(&mut self, bits: u32) -> Result<()>;
    /// Actor +6C locomotion +50 (binary32).
    fn locomotion_word50(&mut self) -> Result<u32>;
    /// `576D20(speed, factor)` on the actor +6C locomotion.
    fn locomotion_set_speed(&mut self, speed_bits: u32, factor_bits: u32) -> Result<()>;
}

/// `52D620(divisor, factor)` on the element move state (x87 at `p`):
/// state+24 = `(state+24 / divisor) * factor` stored to binary32, then
/// `576D20(locomotion+50 * state+24, locomotion+50)`; returns true.
pub fn scale_speed(
    divisor: u32,
    factor: u32,
    world: &mut impl SpeedScaleWorld,
    p: aa_numeric::Precision,
) -> Result<bool> {
    use crate::x87::{n, num, store};
    let speed = world.requested_speed()?;
    let scaled = store(num(num(n(speed)?.div(&n(divisor)?, p))?.mul(&n(factor)?, p))?)?;
    world.set_requested_speed(scaled)?;
    let word = world.locomotion_word50()?;
    let speed = store(num(n(word)?.mul(&n(world.requested_speed()?)?, p))?)?;
    world.locomotion_set_speed(speed, word)?;
    Ok(true)
}
