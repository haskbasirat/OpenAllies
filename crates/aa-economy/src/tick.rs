//! Complete original GKingdom economy Tick (`0x4E9C40`, economy updatable at
//! kingdom+0x180, vtable `0xA29B50` slot 5) and the leaves it executes:
//! recruit-single regeneration `0x4E12D0` (+ `0x45D5B0`, `0x4E0EC0`), Commander
//! (general) experience income `0x4CDFD0` (+ score add `0x58E870`), the statistics
//! snapshot timer `0x4E37D0` and production statistics `0x58ED00`.
//!
//! Native order, binary32 stores and x87 precision are preserved. Seams that are
//! not economy code stay explicit [`TickWorld`] callbacks: event dispatch
//! `0x42D4B0` (kinds 3 and 4), victory checks `0x4E9960`/`0x4E9A40`, the GTID
//! generation table lookup and snapshot creation `0x58F530`+`0x45F240`.
//!
//! The flow/overdraw/ratio stage is computed on a copy and committed only when it
//! is fully representable (finite, nonzero ratio denominator). Later stages run
//! callbacks; an error after that point leaves earlier native effects applied.
use crate::binary::Binary;
pub use crate::statistics::{Statistics, add_score};
use crate::{Economy, EconomyError, F32Bits, FlowArithmetic, ResourceKind, X87Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Binary32 `0.05` at `0xA2A278`: minimum ratio change that is announced again.
pub const SHORTAGE_ANNOUNCE_DELTA_BITS: F32Bits = F32Bits(0x3d4c_cccd);
/// Binary32 `100.0` at `0x9F2408`: ratio-to-percent factor before `_ftol`.
pub const PERCENT_BITS: F32Bits = F32Bits(0x42c8_0000);
/// Binary32 `60.0` at `0xA028CC`: statistics snapshot period in seconds.
pub const SNAPSHOT_PERIOD_BITS: F32Bits = F32Bits(0x4270_0000);
/// Binary32 `1.0` at `0xB28194`: world time from which victory checks run.
pub const VICTORY_CHECK_TIME_BITS: F32Bits = F32Bits(0x3f80_0000);
const ONE: F32Bits = F32Bits(0x3f80_0000);

/// Statistics score indices touched by the economy code (`0x58E1A0` names).
pub const SCORE_TECHS_RESEARCHED: usize = 2;
pub const SCORE_TECH_BUILDINGS_POSSESSION_TIME: usize = 32;
pub const SCORE_EXPERIENCE_GAINED: usize = 44;
pub const SCORE_EXPERIENCE_SPENT: usize = 45;
pub const SCORE_FACTION_ABILITIES_USED: usize = 46;
pub const SCORE_TECH_BUILDINGS_OWNED: usize = 52;
/// Native statistics object: production totals inline at +0 for at most this many
/// resources (`0x48BA89` rejects index >= 10), ordinary scores at +0x28. The Rust
/// type keeps both growable; the cap is recorded, not enforced.
pub const NATIVE_STATISTICS_RESOURCE_CAP: usize = 10;

/// GKingdom fields read/written by the tick besides the four resource arrays.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickState {
    /// kingdom+0x190 (economy+0x10): nonzero suppresses the overdraw flag.
    pub suppress_overdrawn: bool,
    /// kingdom+0x1F0: last computed upkeep-shortage ratio.
    pub shortage_ratio: F32Bits,
    /// kingdom+0x1F4: ratio last announced with event 3.
    pub announced_ratio: F32Bits,
    /// kingdom+0x334: statistics snapshot accumulator in seconds.
    pub snapshot_timer: F32Bits,
}

/// Recruit item single (vtable `0xA29694`, 0x24 bytes) in the kingdom list at
/// kingdom+0x214/+0x218.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecruitSingle {
    /// +4 ActorData identity (hash key of the kingdom recruit item table).
    #[serde(default)]
    pub definition: u32,
    /// +8: may be recruited (`0x4E0EC0` sets it, `0x4E0EA0` consumes it).
    pub available: bool,
    /// +0xC / +0x10: generation-checked actor and organization GTIDs.
    pub actor: u32,
    pub organization: u32,
    /// +0x1C: regeneration progress, 1.0 when fully charged.
    pub progress: F32Bits,
    /// ActorData+0x2CC read through `0x45D5B0(0)`.
    pub regeneration_rate: F32Bits,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecruitSingles {
    /// kingdom+0x220: index currently charging, or -1.
    pub charging: i32,
    pub items: Vec<RecruitSingle>,
}

/// Commander experience at kingdom+0x230 (+0xC). This is the same native
/// field as `aa_combat::commander::State::experience`, spent by faction/general
/// abilities (`0x4CDD90`) and checked by `0x4E24B0`. The owner binding must keep
/// one authoritative value; this struct is the arithmetic view of that field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderExperience {
    /// +0xC.
    pub balance: F32Bits,
    /// Commander definition (FactionData)+0x3C through +4; a negative maximum
    /// disables the upper clamp.
    pub maximum: F32Bits,
}

/// Statistics object reached through the Commander's owner kingdom (+8)+0x32C.
pub enum OwnerStatistics<'a> {
    /// Owner is this kingdom: the same object as [`TickParts::statistics`].
    Kingdom,
    Other(&'a mut Statistics),
    Absent,
}

pub struct CommanderParts<'a> {
    pub experience: &'a mut CommanderExperience,
    pub owner_statistics: OwnerStatistics<'a>,
}

/// All native kingdom state the tick mutates. Absent native pointers are `None`.
pub struct TickParts<'a> {
    pub state: &'a mut TickState,
    pub singles: &'a mut RecruitSingles,
    pub commander: Option<CommanderParts<'a>>,
    pub statistics: Option<&'a mut Statistics>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TickInput {
    pub seconds: F32Bits,
    /// World+0xE0 time compared against [`VICTORY_CHECK_TIME_BITS`].
    pub world_time: F32Bits,
    /// `*(cache+0x180)`: Commander experience per second of tick time.
    pub commander_experience_rate: F32Bits,
}

/// Required non-economy leaves; no permissive defaults are provided.
pub trait TickWorld {
    /// Native generation-table lookup used by `0x4E12D0` (`0xB364C0`).
    fn gtid_live(&mut self, gtid: u32) -> Result<bool, EconomyError>;
    /// `0x42D4B0(kind 3, kingdom, percent)`: kingdom_upkeep_shortage.
    fn upkeep_shortage(&mut self, percent: i32) -> Result<(), EconomyError>;
    /// `0x42D4B0(kind 4, kingdom)`: kingdom_upkeep_repaired.
    fn upkeep_repaired(&mut self) -> Result<(), EconomyError>;
    /// `0x4E9960` then `0x4E9A40` are called in this order.
    fn team_victory_check(&mut self) -> Result<(), EconomyError>;
    fn percentage_victory_check(&mut self) -> Result<(), EconomyError>;
    /// `0x58F530` snapshot construction and `0x45F240` push into kingdom+0x338.
    fn statistics_snapshot(&mut self) -> Result<(), EconomyError>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickReport {
    pub flow: FlowArithmetic,
}

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

/// `0x4CDFD0`: Commander experience income with optional `*(cache+0x17C)`
/// multiplier (the tick passes `None`; `0x4E0FD0` passes the multiplier).
pub fn add_commander_experience(
    commander: &mut CommanderExperience,
    owner_statistics: Option<&mut Statistics>,
    amount: F32Bits,
    multiplier: Option<F32Bits>,
    precision: X87Precision,
) -> Result<(), EconomyError> {
    finite(&[amount, commander.balance, commander.maximum])?;
    let amount = match multiplier {
        Some(m) => {
            finite(&[m])?;
            b(amount).mul(&b(m), precision).store_f32()?
        }
        None => amount,
    };
    let zero = Binary::from_i32(0);
    let mut balance = b(amount)
        .add(&b(commander.balance), precision)
        .store_f32()?;
    if b(balance).cmp(&zero) == Ordering::Less {
        balance = F32Bits::ZERO;
    }
    let maximum = b(commander.maximum);
    if maximum.cmp(&zero) != Ordering::Less && maximum.cmp(&b(balance)) == Ordering::Less {
        balance = commander.maximum;
    }
    commander.balance = balance;
    if let Some(statistics) = owner_statistics
        && b(amount).cmp(&zero) == Ordering::Greater
    {
        add_score(statistics, SCORE_EXPERIENCE_GAINED, amount, precision)?;
    }
    Ok(())
}

/// `0x4E12D0`: at most one item charges at a time; the index-0 completion
/// quirk (charging stays 0 instead of -1) is native.
pub fn regenerate_recruit_singles(
    singles: &mut RecruitSingles,
    seconds: F32Bits,
    precision: X87Precision,
    world: &mut impl TickWorld,
) -> Result<(), EconomyError> {
    finite(&[seconds])?;
    let one = b(ONE);
    let mut index = 0usize;
    while index < singles.items.len() {
        let item = &singles.items[index];
        let eligible = item.available
            || (!world.gtid_live(item.actor)? && !world.gtid_live(item.organization)?);
        if !eligible {
            index += 1;
            continue;
        }
        let item = &mut singles.items[index];
        finite(&[item.progress, item.regeneration_rate])?;
        let native_index = i32::try_from(index).map_err(|_| EconomyError::ResourceIndex)?;
        if b(item.progress).cmp(&one) != Ordering::Less {
            if !item.available {
                item.available = true; // 0x4E0EC0
                singles.charging = native_index;
                if index != 0 {
                    singles.charging = -1;
                }
            }
        } else if singles.charging == -1 || singles.charging == native_index {
            singles.charging = native_index;
            item.progress = b(item.regeneration_rate)
                .mul(&b(seconds), precision)
                .add(&b(item.progress), precision)
                .store_f32()?;
            if b(item.progress).cmp(&one) != Ordering::Less {
                item.progress = ONE;
                item.available = true;
                singles.charging = -1;
            }
        }
        index += 1;
    }
    Ok(())
}

/// `0x58ED00`: production totals += gross; possession-time score integral.
pub fn accumulate_production_statistics(
    statistics: &mut Statistics,
    seconds: F32Bits,
    gross: &[F32Bits],
    precision: X87Precision,
) -> Result<(), EconomyError> {
    if statistics.production.len() != gross.len() {
        return Err(EconomyError::Shape);
    }
    finite(gross)?;
    finite(&statistics.production)?;
    for (total, value) in statistics.production.iter_mut().zip(gross).rev() {
        *total = b(*value).add(&b(*total), precision).store_f32()?;
    }
    let owned = *statistics
        .scores
        .get(SCORE_TECH_BUILDINGS_OWNED)
        .ok_or(EconomyError::ResourceIndex)?;
    let time = statistics
        .scores
        .get_mut(SCORE_TECH_BUILDINGS_POSSESSION_TIME)
        .ok_or(EconomyError::ResourceIndex)?;
    finite(&[seconds, owned, *time])?;
    *time = b(seconds)
        .mul(&b(owned), precision)
        .add(&b(*time), precision)
        .store_f32()?;
    Ok(())
}

/// `0x4E37D0(seconds, force=false)`: snapshot every 60 accumulated seconds.
pub fn advance_snapshot_timer(
    state: &mut TickState,
    seconds: F32Bits,
    precision: X87Precision,
    world: &mut impl TickWorld,
) -> Result<(), EconomyError> {
    finite(&[seconds, state.snapshot_timer])?;
    let accumulated = b(seconds)
        .add(&b(state.snapshot_timer), precision)
        .store_f32()?;
    state.snapshot_timer = accumulated;
    if b(accumulated).cmp(&b(SNAPSHOT_PERIOD_BITS)) == Ordering::Less {
        return Ok(());
    }
    state.snapshot_timer = b(accumulated)
        .sub(&b(SNAPSHOT_PERIOD_BITS), precision)
        .store_f32()?;
    world.statistics_snapshot()
}

impl Economy {
    /// Complete `0x4E9C40`. The flow prefix equals [`Economy::flow_arithmetic`];
    /// the shortage ratio is `1 - provided/consumed` over STOCKPILED rows plus,
    /// for each UPKEEP row, `min(provided, consumed) * cost` added to both sums
    /// (x87 accumulators, never stored until the final quotient).
    pub fn tick(
        &mut self,
        input: TickInput,
        parts: TickParts<'_>,
        world: &mut impl TickWorld,
    ) -> Result<TickReport, EconomyError> {
        let TickParts {
            state,
            singles,
            commander,
            mut statistics,
        } = parts;
        finite(&[
            input.seconds,
            input.world_time,
            input.commander_experience_rate,
            state.shortage_ratio,
            state.announced_ratio,
            state.snapshot_timer,
        ])?;
        let precision = self.precision;
        let snapshot = self.state.clone();
        let flow = self.flow_arithmetic(input.seconds, state.suppress_overdrawn)?;
        enum RatioOutcome {
            Shortage {
                ratio: F32Bits,
                percent: Option<i32>,
            },
            Repaired {
                notify: bool,
            },
        }
        let outcome = match self.shortage_outcome(state, flow.stock_went_negative) {
            Ok(outcome) => outcome,
            Err(error) => {
                self.state = snapshot;
                return Err(error);
            }
        };
        let outcome = match outcome {
            (Some(ratio), percent) => RatioOutcome::Shortage { ratio, percent },
            (None, _) => RatioOutcome::Repaired {
                notify: !b(state.shortage_ratio).is_zero(),
            },
        };
        match outcome {
            RatioOutcome::Shortage { ratio, percent } => {
                state.shortage_ratio = ratio;
                if let Some(percent) = percent {
                    state.announced_ratio = ratio;
                    world.upkeep_shortage(percent)?;
                }
            }
            RatioOutcome::Repaired { notify } => {
                if notify {
                    world.upkeep_repaired()?;
                }
                state.shortage_ratio = F32Bits::ZERO;
                state.announced_ratio = F32Bits::ZERO;
            }
        }
        regenerate_recruit_singles(singles, input.seconds, precision, world)?;
        if let Some(CommanderParts {
            experience,
            owner_statistics,
        }) = commander
        {
            let amount = b(input.seconds)
                .mul(&b(input.commander_experience_rate), precision)
                .store_f32()?;
            let owner = match owner_statistics {
                OwnerStatistics::Kingdom => statistics.as_deref_mut(),
                OwnerStatistics::Other(other) => Some(other),
                OwnerStatistics::Absent => None,
            };
            add_commander_experience(experience, owner, amount, None, precision)?;
        }
        if b(input.world_time).cmp(&b(VICTORY_CHECK_TIME_BITS)) != Ordering::Less {
            world.team_victory_check()?;
            world.percentage_victory_check()?;
        }
        if let Some(statistics) = statistics {
            advance_snapshot_timer(state, input.seconds, precision, world)?;
            accumulate_production_statistics(
                statistics,
                input.seconds,
                &flow.gross_production,
                precision,
            )?;
        }
        Ok(TickReport { flow })
    }

    /// Ratio stage at `0x4E9DA6..0x4E9EF7`. Returns `(Some(ratio), percent)`
    /// when the overdraw flag is set; `percent` is `Some` when event 3 fires.
    fn shortage_outcome(
        &self,
        state: &TickState,
        overdrawn: bool,
    ) -> Result<(Option<F32Bits>, Option<i32>), EconomyError> {
        if !overdrawn {
            return Ok((None, None));
        }
        let precision = self.precision;
        let resources = &self.catalog.resources;
        let s = &self.state;
        let mut provided = Binary::from_i32(0);
        let mut consumed = Binary::from_i32(0);
        for row in (0..resources.len()).rev() {
            match resources[row].kind {
                ResourceKind::Stockpiled => {
                    provided = provided.add(&b(s.effective_provided[row]), precision);
                    consumed = consumed.add(&b(s.consumed[row]), precision);
                }
                ResourceKind::Upkeep => {
                    for (column, target) in resources.iter().enumerate() {
                        if target.kind != ResourceKind::Stockpiled {
                            continue;
                        }
                        let have = b(s.effective_provided[row]);
                        let need = b(s.consumed[row]);
                        let lesser = if have.cmp(&need) == Ordering::Greater {
                            need
                        } else {
                            have
                        };
                        let charge =
                            lesser.mul(&b(resources[row].purchase_cost[column]), precision);
                        provided = provided.add(&charge, precision);
                        consumed = consumed.add(&charge, precision);
                    }
                }
                ResourceKind::LimitedConsumed => {}
            }
        }
        let ratio = b(ONE)
            .sub(&provided.div(&consumed, precision)?, precision)
            .store_f32()?;
        let delta = if b(state.shortage_ratio).is_zero() {
            b(ONE)
        } else {
            let difference = b(ratio).sub(&b(state.announced_ratio), precision);
            if difference.is_negative_nonzero() {
                difference.neg()
            } else {
                difference
            }
        };
        let zero = Binary::from_i32(0);
        let percent = if delta.cmp(&b(SHORTAGE_ANNOUNCE_DELTA_BITS)) == Ordering::Greater
            && b(ratio).cmp(&zero) == Ordering::Greater
        {
            Some(b(ratio).mul(&b(PERCENT_BITS), precision).truncate_i32()?)
        } else {
            None
        };
        Ok((Some(ratio), percent))
    }
}
