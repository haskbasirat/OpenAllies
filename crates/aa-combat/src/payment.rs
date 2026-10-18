//! Native ability payment4E2590 / faction debit4CDD90 / stockpile4E1690.
//! Balances are borrowed real aa-economy state: the kingdom Economy and the
//! single Commander experience store (`commander::State::experience`, an
//! `aa_economy::tick::CommanderExperience`). Affordability remains admission's
//! responsibility (4E24B0 is `aa_economy::transfers::can_afford_commander_ability`).
//! This debit deliberately permits negative balances.
use crate::{Error, Result, ability::Ability};
use aa_body::F32;
use aa_economy::{Economy, F32Bits, X87Precision, tick::CommanderExperience};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;
/// Kingdom+0x230 Commander, as used by 4CDD90: its +0xC balance (borrowed from
/// the authoritative store) and the +8 owner's statistics handle (+0x32C).
pub struct Faction<'a> {
    pub experience: &'a mut CommanderExperience,
    pub statistics: Option<u32>,
}
pub struct State<'a> {
    pub economy: &'a mut Economy,
    pub faction: Option<Faction<'a>>,
}
pub trait World {
    fn faction_xp_cost(&self, local_definition: u32) -> Result<i32>;
    /// Dense original resource registry order, copied after faction stat callbacks.
    fn resource_costs(&self, local_definition: u32) -> Result<Vec<F32>>;
    /// Original58E870. Called with45=spent amount,46=count, only for positive cost.
    /// Native captured statistics handle is reused for the second callback.
    /// Bind to `aa_economy::tick::add_score` on the kingdom's single
    /// `aa_economy::tick::Statistics` (kingdom+0x32C); no second type exists.
    fn add_statistic(
        &mut self,
        state: &mut State<'_>,
        statistics: u32,
        index: u32,
        amount: F32,
    ) -> Result<()>;
}
fn n(f: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(f.0)?)
}
pub fn pay(state: &mut State<'_>, ability: &Ability, world: &mut impl World) -> Result<()> {
    let Some(local) = ability.local_definition else {
        return Ok(());
    };
    let p = match state.economy.precision() {
        X87Precision::Single24 => Precision::Single24,
        X87Precision::Double53 => Precision::Double53,
        X87Precision::Extended64 => Precision::Extended64,
    };
    if let Some(faction) = state.faction.as_mut() {
        let cost = F32(Binary::from_i32(world.faction_xp_cost(local)?).store_f32()?);
        faction.experience.balance = F32Bits(
            n(F32(faction.experience.balance.0))?
                .sub(&n(cost)?, p)?
                .store_f32()?,
        );
        let statistics = faction.statistics;
        if n(cost)?.compare(&Binary::from_i32(0)) == Ordering::Greater
            && let Some(statistics) = statistics
        {
            world.add_statistic(state, statistics, 45, cost)?;
            world.add_statistic(state, statistics, 46, F32::ONE)?;
        }
    }
    let costs = world
        .resource_costs(local)?
        .into_iter()
        .map(|f| F32Bits(f.0))
        .collect::<Vec<_>>();
    state
        .economy
        .debit_stockpiles(&costs)
        .map_err(|e| Error::World(format!("ability stockpile debit: {e}")))?;
    Ok(())
}
