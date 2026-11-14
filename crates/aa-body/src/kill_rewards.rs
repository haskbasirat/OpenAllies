//! Native kill rewards called by the kill credit `4D57C0` (see `credit`):
//!
//! * `4D3E20` (GActor.cpp OnKilledLair), for a victim with a `+C4`
//!   component: a killer with a `+E8` kingdom object that can see the lair
//!   draws the world RNG once (`45D640` over the lair definition's `+528` /
//!   `+52C` bounds) and adds the binary32 amount to every resource whose
//!   `+18` word is zero (`4E2080`), posting message 10 for each.
//! * `4D10F0`, for a victim with an OrganizationComponent: `7A2370(victim)`
//!   on the killer's `+154` object, or else on its organization's.
//!
//! Oracle: research/phase8-combat/native_kill_rewards.py.
use crate::{F32, Result};
use aa_numeric::{Binary, Precision};

/// Binary64 `2^32` (`[9FDAA0]`), added to a negative draw.
const TWO_POW_32: u64 = 0x41f0_0000_0000_0000;
/// Binary64 `[9FDA98]` scale of the unsigned draw.
const DRAW_SCALE: u64 = 0x3df0_0000_0010_0000;

/// Native `45D640(rng)` on an ActorData with `low = +528`, `high = +52C`,
/// given the `85D840` draw: `fild` of the signed draw (`+2^32` when
/// negative), times the scale, times `high - low`, plus `low`. The ST0
/// result is not rounded to binary32.
pub fn reward_draw(low: F32, high: F32, draw: u32, precision: Precision) -> Result<Binary> {
    let signed = draw as i32;
    let mut unit = Binary::from_i32(signed);
    if signed < 0 {
        unit = unit.add(&Binary::from_f64_bits(TWO_POW_32)?, precision)?;
    }
    let unit = unit.mul(&Binary::from_f64_bits(DRAW_SCALE)?, precision)?;
    let low = Binary::from_f32_bits(low.0)?;
    let span = Binary::from_f32_bits(high.0)?.sub(&low, precision)?;
    Ok(unit.mul(&span, precision)?.add(&low, precision)?)
}

pub trait LairWorld {
    /// Actor `+E8` kingdom object.
    fn kingdom_object(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Actor virtual `+108`.
    fn kingdom(&mut self, actor: u32) -> Result<u32>;
    /// Lair virtual `+70(kingdom, flag)`.
    fn can_see(&mut self, lair: u32, kingdom: u32, flag: u32) -> Result<bool>;
    /// GActor.cpp line 0xECC debug log naming the lair's `+14` id ("ignoring
    /// reward since can't see it").
    fn ignored(&mut self, lair: u32) -> Result<()>;
    /// Lair `+4` definition `+528` and `+52C`.
    fn reward_bounds(&mut self, lair: u32) -> Result<[F32; 2]>;
    /// `4F02B0` world RNG plus one `85D840` draw (authoritative sequence).
    fn draw_random(&mut self) -> Result<u32>;
    /// `[B36530]+38` resource count, read once.
    fn resource_count(&mut self) -> Result<i32>;
    /// Resource `[B36530]+34[index]` `+18` word.
    fn resource_word18(&mut self, index: i32) -> Result<u32>;
    /// `4E2080(index, amount)` on a kingdom object.
    fn add_resource(&mut self, kingdom_object: u32, index: i32, amount: F32) -> Result<()>;
    /// Lair virtual `+24`: its name string (released after the message).
    fn name(&mut self, lair: u32) -> Result<u32>;
    /// `471A30` on the resource: its `+28` record's `+18` word.
    fn resource_label(&mut self, index: i32) -> Result<u16>;
    /// `42D490([B39D8C], 10, kingdom, lair, label, (double)amount, name)`.
    fn post_message(&mut self, kingdom: u32, lair: u32, label: u16, amount: F32, name: u32) -> Result<()>;
}

/// Native `4D3E20(lair)` on the killer.
pub fn lair_reward(world: &mut impl LairWorld, killer: u32, lair: u32, precision: Precision) -> Result<()> {
    if world.kingdom_object(killer)?.is_none() {
        return Ok(());
    }
    let kingdom = world.kingdom(killer)?;
    if !world.can_see(lair, kingdom, 1)? {
        return world.ignored(lair);
    }
    let [low, high] = world.reward_bounds(lair)?;
    let draw = world.draw_random()?;
    let amount = F32(reward_draw(low, high, draw, precision)?.store_f32()?);
    let count = world.resource_count()?;
    let mut index = 0;
    while index < count {
        if world.resource_word18(index)? == 0 {
            // +E8 is reread for every grant.
            let object = world
                .kingdom_object(killer)?
                .ok_or(crate::Error::Callback("4D3E20 killer kingdom object vanished".into()))?;
            world.add_resource(object, index, amount)?;
            let name = world.name(lair)?;
            let label = world.resource_label(index)?;
            let kingdom = world.kingdom(killer)?;
            world.post_message(kingdom, lair, label, amount, name)?;
        }
        index += 1;
    }
    Ok(())
}

pub trait OrganizationRewardWorld {
    /// Actor `+154` object.
    fn object154(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Actor `+80` membership and its `+10` organization actor; None when
    /// either is null.
    fn membership_organization(&mut self, actor: u32) -> Result<Option<u32>>;
    /// `7A2370(victim)` on a `+154` object.
    fn organization_reward(&mut self, object: u32, victim: u32) -> Result<()>;
}

/// Native `4D10F0(victim)` on the killer.
pub fn organization_reward(world: &mut impl OrganizationRewardWorld, killer: u32, victim: u32) -> Result<()> {
    let object = match world.object154(killer)? {
        Some(object) => Some(object),
        None => match world.membership_organization(killer)? {
            Some(organization) => world.object154(organization)?,
            None => None,
        },
    };
    if let Some(object) = object {
        world.organization_reward(object, victim)?;
    }
    Ok(())
}
