//! Kingdom stockpile row helpers and the native resource transfers built on
//! them: steal-resources ability effect `0x4CA680`, team "give resources"
//! command (`0x509840` command type 3), "faction ability" (Commander/general power)
//! affordability `0x4E24B0`/`0x4CDD60` and the Commander-XP wrapper `0x4E0FD0`.
//!
//! Target/kingdom resolution, defeated-state lookup, messages and glyph text
//! are caller responsibilities; the functions here start where the native
//! code reads resource arrays and stop before presentation, returning the
//! values the native events carry.
use crate::binary::Binary;
use crate::tick::{CommanderExperience, Statistics, add_commander_experience};
use crate::{Economy, EconomyError, F32Bits, ResourceKind, X87Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Binary32 0.5 at `0x9FBDB8`, added before the CRT floor in `0x432650`.
pub const DISPLAY_ROUNDING_BITS: F32Bits = F32Bits(0x3f00_0000);

fn b(value: F32Bits) -> Binary {
    Binary::from_bits(value)
}

fn finite(values: &[F32Bits]) -> Result<(), EconomyError> {
    if values.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(EconomyError::NonFiniteInput)
    }
}

/// The other kingdom of a two-kingdom transfer. `Same` reproduces the native
/// sequential effect when both native pointers name one kingdom.
pub enum Counterparty<'a> {
    Same,
    Other(&'a mut Economy),
}

impl Economy {
    /// `0x4E2080` -> `0x4E1630`: binary32(amount + stock), then values <= 0
    /// become +0 and values above EconomyStockpileResourceMax become the max.
    pub fn add_stockpile_row(&mut self, index: usize, amount: F32Bits) -> Result<(), EconomyError> {
        let stock = *self
            .state
            .stockpiled
            .get(index)
            .ok_or(EconomyError::ResourceIndex)?;
        finite(&[amount, stock])?;
        let value = b(amount).add(&b(stock), self.precision).store_f32()?;
        let mut stored = if b(value).cmp(&Binary::from_i32(0)) == Ordering::Greater {
            value
        } else {
            F32Bits::ZERO
        };
        if b(self.limits.stockpiled_maximum).cmp(&b(stored)) == Ordering::Less {
            stored = self.limits.stockpiled_maximum;
        }
        self.state.stockpiled[index] = stored;
        Ok(())
    }

    /// `0x4E16D0`: binary32(stock - amount) without clamping.
    pub fn subtract_stockpile_row(
        &mut self,
        index: usize,
        amount: F32Bits,
    ) -> Result<(), EconomyError> {
        let stock = *self
            .state
            .stockpiled
            .get(index)
            .ok_or(EconomyError::ResourceIndex)?;
        finite(&[amount, stock])?;
        self.state.stockpiled[index] = b(stock).sub(&b(amount), self.precision).store_f32()?;
        Ok(())
    }

    /// `0x473520`: first STOCKPILED definition in registry order (the
    /// "primary" resource, cached natively at registry+0x844).
    pub fn primary_resource(&self) -> Option<usize> {
        self.catalog
            .resources
            .iter()
            .position(|r| r.kind == ResourceKind::Stockpiled)
    }
}

fn add_to(
    first: &mut Economy,
    other: &mut Counterparty<'_>,
    index: usize,
    amount: F32Bits,
) -> Result<(), EconomyError> {
    match other {
        Counterparty::Same => first.add_stockpile_row(index, amount),
        Counterparty::Other(economy) => economy.add_stockpile_row(index, amount),
    }
}

/// `0x4CA680` after the target actor and its kingdom were resolved (a missing
/// target or kingdom logs and returns false natively, without any change).
/// Every positive amount moves `min(victim stock, amount)` (stock chosen only
/// when strictly less) row by row: victim `0x4E16D0`, then thief `0x4E2080`.
/// Returns the stolen value per row (zero bits for skipped rows).
pub fn steal_resources(
    amounts: &[F32Bits],
    victim: &mut Economy,
    mut thief: Counterparty<'_>,
) -> Result<Vec<F32Bits>, EconomyError> {
    let count = victim.catalog.resources.len();
    if amounts.len() != count {
        return Err(EconomyError::Shape);
    }
    if let Counterparty::Other(other) = &thief
        && other.catalog.resources.len() != count
    {
        return Err(EconomyError::Shape);
    }
    finite(amounts)?;
    let mut stolen = vec![F32Bits::ZERO; count];
    let zero = Binary::from_i32(0);
    for index in 0..count {
        let amount = amounts[index];
        if b(amount).cmp(&zero) != Ordering::Greater {
            continue;
        }
        let stock = victim.state.stockpiled[index];
        finite(&[stock])?;
        let value = if b(stock).cmp(&b(amount)) == Ordering::Less {
            stock
        } else {
            amount
        };
        victim.subtract_stockpile_row(index, value)?;
        add_to(victim, &mut thief, index, value)?;
        stolen[index] = value;
    }
    Ok(stolen)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GiveOutcome {
    /// Primary resource row that moved.
    pub resource: usize,
    /// Command amount after the native clamp to the giver's stock (+0x1C).
    pub amount: F32Bits,
    /// Integer carried by events 0x1D (giver) and 0x1E (receiver).
    pub display: i32,
}

/// `0x509840` command type 3. A defeated receiver (kingdom+0x19C == 2) logs
/// and changes nothing (`None`). Otherwise the amount is clamped to the
/// giver's primary stock when the stock is strictly smaller, the giver gets
/// `0x4E2080(-amount)` and the receiver `0x4E2080(amount)`.
pub fn give_primary_resource(
    giver: &mut Economy,
    mut receiver: Counterparty<'_>,
    amount: F32Bits,
    receiver_defeated: bool,
) -> Result<Option<GiveOutcome>, EconomyError> {
    if receiver_defeated {
        return Ok(None);
    }
    let resource = giver
        .primary_resource()
        .ok_or(EconomyError::ResourceIndex)?;
    if let Counterparty::Other(other) = &receiver
        && other.catalog.resources.len() != giver.catalog.resources.len()
    {
        return Err(EconomyError::Shape);
    }
    finite(&[amount])?;
    let stock = giver.state.stockpiled[resource];
    finite(&[stock])?;
    let amount = if b(stock).cmp(&b(amount)) == Ordering::Less {
        stock
    } else {
        amount
    };
    giver.add_stockpile_row(resource, F32Bits(amount.0 ^ 0x8000_0000))?;
    add_to(giver, &mut receiver, resource, amount)?;
    Ok(Some(GiveOutcome {
        resource,
        amount,
        display: display_amount(amount, giver.precision)?,
    }))
}

/// `0x432650` + `0x96CC4C`: floor(binary64(PC(amount + 0.5))) as an integer.
pub fn display_amount(amount: F32Bits, precision: X87Precision) -> Result<i32, EconomyError> {
    finite(&[amount])?;
    b(amount)
        .add(&b(DISPLAY_ROUNDING_BITS), precision)
        .rounded(X87Precision::Double53)
        .floor_i32()
}

/// Ability definition fields read by `0x4E24B0`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityCost {
    /// Definition+0x74 integer Commander experience cost (`FILD`).
    pub experience: i32,
    /// Dense resource cost produced by `0x4BEB50`.
    pub resources: Vec<F32Bits>,
}

/// `0x4E24B0`: no definition means affordable; with a Commander the
/// experience cost must be <= the balance (`0x4CDD60`); then `0x4E20A0`.
pub fn can_afford_commander_ability(
    economy: &Economy,
    commander: Option<&CommanderExperience>,
    ability: Option<&AbilityCost>,
) -> Result<bool, EconomyError> {
    let Some(ability) = ability else {
        return Ok(true);
    };
    if let Some(commander) = commander {
        finite(&[commander.balance])?;
        if Binary::from_i32(ability.experience).cmp(&b(commander.balance)) == Ordering::Greater {
            return Ok(false);
        }
    }
    economy.can_afford(&ability.resources)
}

/// `0x4E0FD0`: kingdom+0x230 present -> `0x4CDFD0(amount, multiplier)` with
/// `*(cache+0x17C)`.
pub fn add_kingdom_commander_experience(
    commander: Option<&mut CommanderExperience>,
    owner_statistics: Option<&mut Statistics>,
    amount: F32Bits,
    multiplier: F32Bits,
    precision: X87Precision,
) -> Result<(), EconomyError> {
    match commander {
        Some(commander) => add_commander_experience(
            commander,
            owner_statistics,
            amount,
            Some(multiplier),
            precision,
        ),
        None => Ok(()),
    }
}

/// `fld factor; fmul value; fstp` as used by `0x4CD770` (ResourceData scale).
pub fn scale(
    factor: F32Bits,
    value: F32Bits,
    precision: X87Precision,
) -> Result<F32Bits, EconomyError> {
    finite(&[factor, value])?;
    b(factor).mul(&b(value), precision).store_f32()
}

/// Leaves of the team "tribute actor" command (`0x509840` type 4).
pub trait TributeWorld {
    /// Generation-checked GTID lookup (`0x432E40`).
    fn resolve(&self, gtid: u32) -> Option<u32>;
    /// `actor.0x4CED40()`: the actor itself when it has a SettlementComponent,
    /// otherwise its organization's settlement (virtual +0x100).
    fn settlement_of(&mut self, actor: u32) -> Result<Option<u32>, EconomyError>;
    /// `[settlement+0x98]` member list count (+0x1C) and entries (+0x18).
    fn settlement_member_count(&mut self, settlement: u32) -> Result<usize, EconomyError>;
    fn settlement_member(&mut self, settlement: u32, index: usize) -> Result<u32, EconomyError>;
    /// Filter `0x4D9E20(0x14)` + `0x4D9B60(member)`: members that stay.
    fn keeps_member(&mut self, member: u32) -> Result<bool, EconomyError>;
    /// `world.0x4F6760(member, 0)`.
    fn destroy_actor(&mut self, member: u32) -> Result<(), EconomyError>;
    /// Actor virtual +0x160 (change kingdom).
    fn change_owner(&mut self, actor: u32, receiver: u32) -> Result<(), EconomyError>;
    /// "cannot tribute actors to a kingdom that has been defeated" (`0x404F30`).
    fn log_defeated_receiver(&mut self) -> Result<(), EconomyError>;
    /// `0x42D490(0x1F, giver, ...)` then `0x42D490(0x20, receiver, ...)`.
    fn tribute_event(&mut self, kind: u32, kingdom: u32) -> Result<(), EconomyError>;
}

/// `0x509840` type 4 -> `0x509440`, then events 0x1F/0x20. The events are
/// sent even when the receiver was defeated and nothing moved (native). The
/// tributed GTID must resolve (the native code dereferences it).
pub fn tribute_actor(
    gtid: u32,
    giver: u32,
    receiver: u32,
    receiver_defeated: bool,
    world: &mut impl TributeWorld,
) -> Result<(), EconomyError> {
    if receiver_defeated {
        world.log_defeated_receiver()?;
    } else {
        let actor = world
            .resolve(gtid)
            .ok_or_else(|| EconomyError::World("tributed actor GTID is dead".into()))?;
        if let Some(settlement) = world.settlement_of(actor)? {
            let count = world.settlement_member_count(settlement)?;
            for index in (0..count).rev() {
                let member = world.settlement_member(settlement, index)?;
                if !world.keeps_member(member)? {
                    world.destroy_actor(member)?;
                }
            }
        }
        let actor = world
            .resolve(gtid)
            .ok_or_else(|| EconomyError::World("tributed actor GTID died".into()))?;
        world.change_owner(actor, receiver)?;
    }
    if world.resolve(gtid).is_none() {
        return Err(EconomyError::World(
            "tribute event actor GTID is dead".into(),
        ));
    }
    world.tribute_event(0x1f, giver)?;
    world.tribute_event(0x20, receiver)
}

/// `fld a; fadd b; fstp` (binary32 store of a PC-rounded sum).
pub fn add_bits(a: u32, b_: u32, precision: X87Precision) -> Result<u32, EconomyError> {
    finite(&[F32Bits(a), F32Bits(b_)])?;
    Ok(b(F32Bits(a)).add(&b(F32Bits(b_)), precision).store_f32()?.0)
}
