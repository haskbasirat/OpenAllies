//! Structure statistics on the kingdom's ordinary statistics (kingdom
//! `+32C`).
//!
//! - `0x58EF60(actor, category, amount)`: with statistics on (world `+274`),
//!   an actor whose statistic category (actor `+13C`, `0x4CEEC0`) is
//!   already `category`, or is 0x32 when `category` is 0x33, only adds
//!   `amount` to score 0x33 when `category` is 0x33; otherwise it leaves its
//!   category (`0x58EBA0`, not from 0x38) and joins the new one (`0x58E8B0`,
//!   not for 0x38).
//! - `0x584B70(_, count)`: a component whose `+10` count is above `count`
//!   puts the difference (binary32) on score 0x33 of its owner's kingdom
//!   through `0x58EF60`, then adds 1.0 to score 0x18 (`0x58E870`).
//!
//! Proven by research/phase8-combat/native_structure_statistics.py
//! (tests/native_structure_statistics.rs).
use crate::statistics::Statistics;
use crate::{EconomyError, F32Bits, X87Precision};

/// Score 0x33 (statistics `+F4`).
pub const STRUCTURE_SCORE: u32 = 0x33;
/// The category that joins score 0x33 without leaving (`0x58EF60`).
pub const STRUCTURE_PENDING: u32 = 0x32;
/// The category that is neither left nor joined.
pub const UNTRACKED: u32 = 0x38;
/// Score 0x18 (`0x584B70`).
pub const STRUCTURE_LOST_SCORE: usize = 0x18;

/// Services of the structure statistics.
pub trait StructureStatisticsWorld {
    /// World `+274`.
    fn statistics_enabled(&self) -> Result<bool, EconomyError>;
    /// Actor `+13C` (`0x4CEEC0`).
    fn statistic_category(&self, actor: u32) -> Result<u32, EconomyError>;
    /// `0x58EBA0(actor)` on the statistics.
    fn leave_category(&mut self, actor: u32) -> Result<(), EconomyError>;
    /// `0x58E8B0(actor, category, amount)` on the statistics.
    fn join_category(&mut self, actor: u32, category: u32, amount: F32Bits)
    -> Result<(), EconomyError>;
}

/// Native `0x58EF60` on `statistics`.
pub fn set_category(
    statistics: &mut Statistics,
    actor: u32,
    category: u32,
    amount: F32Bits,
    world: &mut impl StructureStatisticsWorld,
    precision: X87Precision,
) -> Result<(), EconomyError> {
    if !world.statistics_enabled()? {
        return Ok(());
    }
    let current = world.statistic_category(actor)?;
    if current == category || (current == STRUCTURE_PENDING && category == STRUCTURE_SCORE) {
        if category == STRUCTURE_SCORE {
            statistics.add_score(STRUCTURE_SCORE as usize, amount, precision)?;
        }
        return Ok(());
    }
    if current != UNTRACKED {
        world.leave_category(actor)?;
    }
    if category != UNTRACKED {
        world.join_category(actor, category, amount)?;
    }
    Ok(())
}

/// Native `0x584B70(_, count)`: `component_count` is the component's `+10`;
/// `statistics` is the owner's kingdom (virtual `+108`) `+32C`, when both
/// exist.
pub fn structure_count_lost(
    component_count: i32,
    count: i32,
    owner: u32,
    statistics: Option<&mut Statistics>,
    world: &mut impl StructureStatisticsWorld,
    precision: X87Precision,
) -> Result<(), EconomyError> {
    if count >= component_count {
        return Ok(());
    }
    let Some(statistics) = statistics else {
        return Ok(());
    };
    // fild of the int difference, stored to binary32 (round to nearest).
    let amount = F32Bits((component_count.wrapping_sub(count) as f32).to_bits());
    set_category(statistics, owner, STRUCTURE_SCORE, amount, world, precision)?;
    statistics.add_score(STRUCTURE_LOST_SCORE, F32Bits(0x3F80_0000), precision)
}
