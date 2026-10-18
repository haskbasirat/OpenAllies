//! Original SAI goal priority finalization `7A5200` and goal affordability
//! `7A3860` (Source/Sai/SAIGoalEngine.cpp).
//!
//! `7A5200` is called by the goal engine for every candidate goal
//! (`7A5E10`, `7A67B0`). It zeroes the applied inertia, rejects unaffordable
//! goals, asks the goal class for its base priority (virtual `+78`), then applies
//! the ego's per-goal-type `GoalPriority` record (multiplier, bonus, minimum)
//! and finally the fuzzy factor (candidate/active goals) or the fuzzy factor
//! plus inertia (goal selected last think). Every intermediate `fstp` to the
//! goal's binary32 fields is reproduced; unstored x87 sums use the explicit
//! precision-control mode.
//!
//! The SAIDebug CSV dump (`40EB50` query, `856F00` writes) is diagnostic file
//! output and is not modelled; it does not change any goal field.
use crate::{
    Error, Result,
    x87::{n, num, unit_random},
};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Number of inline `GoalPriority` records at ego+4C (goal types 0..=22 from
/// the `7BDB70` enum). The record array ends exactly at ego+274.
pub const GOAL_TYPE_COUNT: usize = 23;

/// Native constant at `0xA1B268` (binary32 1/60) used to scale the
/// `saving_time_limit` before it multiplies a per-minute rate difference.
pub const SAVING_TIME_SCALE_BITS: u32 = 0x3c88_8889;
/// TGI `[list GoalPriority]` record at ego+4C+type*18 (loader `4AF500`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalPriority {
    /// +0 `bonus`.
    pub bonus: u32,
    /// +4 `multiplier`.
    pub multiplier: u32,
    /// +8 `minimum`.
    pub minimum: u32,
    /// +C `fuzziness`; negative (or unordered) selects the ego default.
    pub fuzziness: u32,
    /// +10 `inertia`; negative (or unordered) selects the ego default.
    pub inertia: u32,
}

/// Ego goal-engine settings read by `7A5200`/`7A3860` (loader `4B0560`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EgoGoalSettings {
    /// +3C `GoalEngine/GeneralSettings/fuzziness`.
    pub default_fuzziness: u32,
    /// +40 `GoalEngine/GeneralSettings/goal_inertia`.
    pub default_inertia: u32,
    /// +4C.. per goal type.
    pub priorities: Vec<GoalPriority>,
    /// +274 `ResourceManagement/saving_time_limit`.
    pub saving_time_limit: u32,
    /// +278 `settlement_reserved_resources_amt`.
    pub settlement_reserved_amount: u32,
    /// +27C `settlement_reserved_resources_time`.
    pub settlement_reserved_time: u32,
    /// +280 `settlement_reserved_resources_max_settlements` (signed compare).
    pub settlement_reserved_max_settlements: i32,
}

/// Mutable goal fields touched by these functions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    /// +8 status, compared signed: 0/1 candidate, 2 selected last think.
    pub status: u32,
    /// +18 fuzzy amount (binary32 bits).
    pub fuzzy: u32,
    /// +1C/+20 per-resource costs (binary32 bits), iterated from the end.
    pub costs: Vec<u32>,
    /// +28 applied inertia (binary32 bits).
    pub inertia_applied: u32,
    /// +38 priority (binary32 bits).
    pub priority: u32,
}

/// One resource slot used by `7A3860`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceSlot {
    /// Resource definition+18 (global list at `[B36530]+34`); nonzero slots are
    /// skipped entirely.
    pub definition_word18: u32,
    /// Player economy +1A0 array element (binary32 bits).
    pub economy_1a0: u32,
    /// Player economy +1B8 array element (binary32 bits).
    pub economy_1b8: u32,
    /// Caller's available-resource array element (`*param_2`).
    pub available: u32,
}

/// Snapshot inputs that the native code reads but never writes here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    /// `[B36534]+E0` game time (binary32 bits).
    pub world_time: u32,
    /// Player+60 -> +20 -> +8 count compared with the reserve maximum.
    pub settlements: i32,
    pub ego: EgoGoalSettings,
    /// Indexed by resource; must cover every goal cost index.
    pub resources: Vec<ResourceSlot>,
}

/// Required live services. Callbacks may mutate the goal; native code rereads
/// every goal field after each call.
pub trait GoalWorld {
    /// Goal virtual `+4`.
    fn goal_type(&mut self, goal: &mut Goal) -> Result<u32>;
    /// Goal virtual `+78`: the class-specific base priority (writes +38).
    fn base_priority(&mut self, goal: &mut Goal) -> Result<()>;
    /// Goal virtual `+48`, consulted only when the settlement reserve applies.
    fn settlement_reserve_exempt(&mut self, goal: &mut Goal) -> Result<bool>;
    /// `79E4B0` on the SAI singleton (signed count).
    fn sai_count(&mut self, goal: &mut Goal) -> Result<i32>;
    /// One `85D840` draw from the SAI RNG (`[B36528]+2C`). That RNG is seeded
    /// from the host performance counter in the SAI constructor `7A0AA0`.
    fn sai_random(&mut self) -> Result<u32>;
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}
/// x87 `fcomp`, `test ah,1`: C0 set means less or unordered.
fn less_or_unordered(a: u32, b: u32) -> bool {
    !matches!(
        host(a).partial_cmp(&host(b)),
        Some(Ordering::Greater | Ordering::Equal)
    )
}
/// `test ah,41`/`jnp`-style strict greater-than (false for unordered).
fn greater(a: u32, b: u32) -> bool {
    host(a) > host(b)
}

/// Complete `7A3860`. `relaxed` is the native second argument (byte).
pub fn affordable(
    goal: &mut Goal,
    relaxed: bool,
    context: &Context,
    world: &mut impl GoalWorld,
    p: Precision,
) -> Result<bool> {
    let ego = &context.ego;
    let rate =
        num(num(n(ego.saving_time_limit)?.mul(&n(SAVING_TIME_SCALE_BITS)?, p))?.store_f32())?;
    let mut reserve = 0_u32;
    // fcomp/test ah,41/jp: the reserve is considered only when time <= limit.
    if host(context.world_time) <= host(ego.settlement_reserved_time)
        && context.settlements < ego.settlement_reserved_max_settlements
        && world.sai_count(goal)? > 0
        && !world.settlement_reserve_exempt(goal)?
    {
        reserve = ego.settlement_reserved_amount;
    }
    // Count and costs are read after the callbacks.
    for i in (0..goal.costs.len()).rev() {
        let slot = context
            .resources
            .get(i)
            .ok_or_else(|| Error::World(format!("goal cost index {i} has no resource slot")))?;
        if slot.definition_word18 != 0 {
            continue;
        }
        let extra = if relaxed {
            Binary::from_i32(0)
        } else {
            let difference = num(n(slot.economy_1a0)?.sub(&n(slot.economy_1b8)?, p))?;
            if difference.compare(&Binary::from_i32(0)) == Ordering::Less {
                Binary::from_i32(0)
            } else {
                num(difference.mul(&n(rate)?, p))?
            }
        };
        let value = num(extra.add(&n(slot.available)?, p))?;
        let value = num(value.sub(&n(reserve)?, p))?;
        if n(goal.costs[i])?.compare(&value) == Ordering::Greater {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Complete `7A5200` with diagnostic output disabled. `draw` is the native
/// byte argument that requests a new fuzzy amount for status 0/1 goals.
pub fn finalize(
    goal: &mut Goal,
    draw: bool,
    context: &Context,
    world: &mut impl GoalWorld,
    p: Precision,
) -> Result<()> {
    goal.inertia_applied = 0;
    if !affordable(goal, false, context, world, p)? {
        goal.priority = 0;
        return Ok(());
    }
    world.base_priority(goal)?;
    // jnp to exit for less or equal; unordered continues (and then fails as
    // unsupported nonfinite arithmetic below).
    if host(goal.priority) <= 0.0 {
        return Ok(());
    }
    let kind = world.goal_type(goal)? as usize;
    let ego = &context.ego;
    let record = *ego
        .priorities
        .get(kind)
        .filter(|_| kind < GOAL_TYPE_COUNT)
        .ok_or_else(|| {
            Error::World(format!(
                "goal type {kind} is outside the native GoalPriority table"
            ))
        })?;
    // Raw dword copies, preserving the original bit patterns.
    let fuzziness = if less_or_unordered(record.fuzziness, 0) {
        ego.default_fuzziness
    } else {
        record.fuzziness
    };
    let inertia = if less_or_unordered(record.inertia, 0) {
        ego.default_inertia
    } else {
        record.inertia
    };
    goal.priority = num(num(n(goal.priority)?.mul(&n(record.multiplier)?, p))?.store_f32())?;
    goal.priority = num(num(n(goal.priority)?.add(&n(record.bonus)?, p))?.store_f32())?;
    // test ah,5/jp: zero only for an ordered less-than.
    if host(goal.priority) < host(record.minimum) {
        goal.priority = 0;
    }
    if !greater(goal.priority, 0) {
        return Ok(());
    }
    let status = goal.status as i32;
    if status < 0 {
        return Ok(());
    }
    if status <= 1 {
        if draw {
            let random = unit_random(world.sai_random()?, p)?;
            goal.fuzzy = num(num(random.mul(&n(fuzziness)?, p))?.store_f32())?;
        }
        goal.priority = num(num(n(goal.priority)?.add(&n(goal.fuzzy)?, p))?.store_f32())?;
    } else if status == 2 {
        goal.inertia_applied = inertia;
        let sum = num(n(goal.priority)?.add(&n(goal.fuzzy)?, p))?;
        goal.priority = num(num(sum.add(&n(inertia)?, p))?.store_f32())?;
    }
    Ok(())
}
