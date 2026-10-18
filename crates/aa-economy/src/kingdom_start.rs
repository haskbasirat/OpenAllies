//! GKingdom start-mission setup `0x4E6DB0`, run for every kingdom by the world
//! mission start `0x4FDCE0`.
//!
//! Order: optional stockpile reset (kingdom+0x23A) to `*(cache+0x110)`
//! (+0 unless strictly positive) clamped to the stock maximum, the Commander
//! definition's technologies (definition+0x30 list), credit of the kingdom
//! starting resources (+0x258), the start profile (+0x244: credit of its +0x20
//! vector, then its technology `0x48D040` unless already owned), the start
//! technology list (+0x24C, owned ones skipped), the nation faction check
//! (diagnostic only), the Commander starting experience `0x4CDF10(*(cache+
//! 0x134))`, then status +0x19C = 0 and started +0x23B = 1. Technologies are
//! granted with `0x4E3AD0(definition, 2, kingdom, 1)` through the callback (the
//! port is `aa_production::research::add_kingdom_property`).
use crate::tick::CommanderExperience;
use crate::{Economy, EconomyError, F32Bits};

/// The only operations here are x87 comparisons (FCOM/FCOMP), which are
/// exact, so binary32 host comparisons reproduce them including the
/// unordered (NaN) outcomes.
fn f(value: F32Bits) -> f32 {
    f32::from_bits(value.0)
}

/// Kingdom+0x244 start profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartProfile {
    /// +0x20 resource vector (registry length).
    pub resources: Vec<F32Bits>,
    /// `0x48D040`: technology named at +0x18 (None when empty or unknown).
    pub technology: Option<u32>,
}

/// Inputs of `0x4E6DB0` read from the kingdom and the rules cache.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartMission {
    /// Kingdom+0x23A.
    pub reset_stockpiles: bool,
    /// `*(cache+0x110)`.
    pub starting_stockpile: F32Bits,
    /// Kingdom+0x258 (registry length).
    pub starting_resources: Vec<F32Bits>,
    pub profile: Option<StartProfile>,
    /// Kingdom+0x24C; null entries are skipped natively.
    pub start_technologies: Vec<Option<u32>>,
    /// Commander definition (kingdom+0x230 -> +4), if a Commander exists.
    pub commander_definition: Option<u32>,
    /// Commander definition+0x30 technology list.
    pub commander_technologies: Vec<u32>,
    /// Nation (kingdom+0x234) faction list +0x48/+0x4C; `None` without a nation.
    pub nation_factions: Option<Vec<u32>>,
    /// `*(cache+0x134)`.
    pub commander_start_experience: F32Bits,
}

/// Kingdom fields written at the end of `0x4E6DB0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartedKingdom {
    /// +0x19C.
    pub status: u32,
    /// +0x23B.
    pub started: bool,
}

pub trait StartMissionWorld {
    /// `0x5032D0(kingdom+0x318, definition, 0, 0)`.
    fn has_property(&mut self, definition: u32) -> Result<bool, EconomyError>;
    /// `kingdom.0x4E3AD0(definition, 2, kingdom, 1)`.
    fn add_property(&mut self, definition: u32) -> Result<(), EconomyError>;
    /// `0x4E4BC0` start_mission_kingdom_invalid_faction_for_nation|sss.
    fn invalid_faction(&mut self) -> Result<(), EconomyError>;
}

/// `0x4CDF10(value)`: balance = value, then +0 when negative, then the
/// definition maximum when that maximum is >= 0 and smaller (unordered
/// comparisons keep the value, as natively).
pub fn set_commander_experience(commander: &mut CommanderExperience, value: F32Bits) {
    let mut balance = value;
    if f(balance) < 0.0 {
        balance = F32Bits::ZERO;
    }
    let maximum = f(commander.maximum);
    if maximum >= 0.0 && maximum < f(balance) {
        balance = commander.maximum;
    }
    commander.balance = balance;
}

/// `0x4730D0(definition)` on the nation: a null definition is allowed.
fn faction_allowed(factions: &[u32], definition: Option<u32>) -> bool {
    definition.is_none_or(|d| factions.contains(&d))
}

/// `0x4E6DB0`.
pub fn start_mission(
    economy: &mut Economy,
    input: &StartMission,
    commander: Option<&mut CommanderExperience>,
    world: &mut impl StartMissionWorld,
) -> Result<StartedKingdom, EconomyError> {
    if commander.is_some() != input.commander_definition.is_some() {
        return Err(EconomyError::World(
            "Commander experience and definition must be present together".into(),
        ));
    }
    if input.reset_stockpiles {
        let value = if f(input.starting_stockpile) > 0.0 {
            input.starting_stockpile
        } else {
            F32Bits::ZERO
        };
        let maximum = economy.limits.stockpiled_maximum;
        let row = if f(maximum) < f(value) {
            maximum
        } else {
            value
        };
        for stock in economy.state.stockpiled.iter_mut() {
            *stock = row;
        }
    }
    if input.commander_definition.is_some() {
        for &definition in &input.commander_technologies {
            world.add_property(definition)?;
        }
    }
    economy.credit_stockpiles(&input.starting_resources)?;
    if let Some(profile) = &input.profile {
        economy.credit_stockpiles(&profile.resources)?;
        if let Some(technology) = profile.technology
            && !world.has_property(technology)?
        {
            world.add_property(technology)?;
        }
    }
    for definition in input.start_technologies.iter().flatten() {
        if !world.has_property(*definition)? {
            world.add_property(*definition)?;
        }
    }
    if let Some(factions) = &input.nation_factions
        && !factions.is_empty()
        && !faction_allowed(factions, input.commander_definition)
    {
        world.invalid_faction()?;
    }
    if let Some(commander) = commander {
        set_commander_experience(commander, input.commander_start_experience);
    }
    Ok(StartedKingdom {
        status: 0,
        started: true,
    })
}
