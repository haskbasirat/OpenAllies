//! Original SAI goal-engine passes (Source/Sai/SAIGoalEngine.cpp).
//!
//! The controller (aa_navigation::sai_state::Controller `7A5750`) owns 23
//! buckets, one per goal type, each with three goal lists: inactive (+0),
//! candidate (+8) and selected (+10). Per player think (`7B4AD0` ->
//! `7A9840`) the engine maintains the lists (`7A7700`) and then selects goals
//! in priority order while reserving resources (`7A93B0`).
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// `7A30B0`: qsort comparator of the goal selection `7A93B0`; orders goals by
/// base priority (+38) descending. Equal (including +0/-0) gives 0; any
/// unordered pair gives 1 in both directions, exactly as the x87 sequence.
pub fn base_priority_order(a_bits: u32, b_bits: u32) -> i32 {
    let (a, b) = (f32::from_bits(a_bits), f32::from_bits(b_bits));
    if a == b {
        0
    } else if a > b {
        -1
    } else {
        1
    }
}

/// `96CE20` with `7A30B0` over goal base priorities: sorts `goals` (any
/// handle type) by `base(goal)`, reproducing the native comparison order.
pub fn sort_by_base_priority<T>(
    goals: &mut [T],
    mut base: impl FnMut(&T) -> Result<u32>,
) -> Result<()> {
    super::crt_qsort::qsort(goals, |a, b| Ok(base_priority_order(base(a)?, base(b)?)))
}

/// Number of goal-type buckets in the controller (+8 array, 0x5C bytes).
pub const BUCKETS: usize = 23;
/// Bucket list index; the goal status word (+8) uses the same numbering.
pub const INACTIVE: usize = 0;
pub const CANDIDATE: usize = 1;
pub const SELECTED: usize = 2;

pub type GoalId = u32;

/// Native goal fields used by the engine passes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalState {
    /// +8 status (0 inactive, 1 candidate, 2 selected).
    pub status: u32,
    /// +1C/+20 per-resource costs (binary32).
    pub costs: Vec<u32>,
    /// +34 current (actor/travel adjusted) priority.
    pub current: u32,
    /// +38 base priority.
    pub base: u32,
    /// +30 average actor travel time (binary32).
    #[serde(default)]
    pub travel: u32,
}

/// One controller bucket. A node may hold a null goal pointer, which most
/// native passes would dereference (reported as an error here).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bucket {
    pub lists: [VecDeque<Option<GoalId>>; 3],
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Controller {
    pub buckets: Vec<Bucket>,
    pub goals: BTreeMap<GoalId, GoalState>,
}
impl Controller {
    pub fn goal(&self, id: GoalId) -> Result<&GoalState> {
        self.goals
            .get(&id)
            .ok_or_else(|| Error::World(format!("missing SAI goal {id}")))
    }
    pub fn goal_mut(&mut self, id: GoalId) -> Result<&mut GoalState> {
        self.goals
            .get_mut(&id)
            .ok_or_else(|| Error::World(format!("missing SAI goal {id}")))
    }
    fn bucket(&mut self, index: usize) -> Result<&mut Bucket> {
        self.buckets
            .get_mut(index)
            .ok_or_else(|| Error::World(format!("SAI goal bucket {index} outside the goal types")))
    }
}

/// Goal virtuals and engine services, in native call order.
pub trait EngineWorld {
    /// Goal virtual +4 (goal type, i.e. bucket index).
    fn goal_type(&mut self, goal: GoalId, ctl: &mut Controller) -> Result<u32>;
    /// Goal virtual +14 (expired).
    fn expired(&mut self, goal: GoalId, ctl: &mut Controller) -> Result<bool>;
    /// Goal virtual +0 with delete flag 1 (destroys the goal object).
    fn destroy(&mut self, goal: GoalId, ctl: &mut Controller) -> Result<()>;
    /// Goal virtual +60 with the player's economy resource array (`7A67B0`
    /// for most classes): recompute the goal's priorities.
    fn reevaluate(&mut self, goal: GoalId, ctl: &mut Controller) -> Result<()>;
    /// Goal virtual +80 (status change: release/deselect).
    fn deselect(&mut self, goal: GoalId, ctl: &mut Controller) -> Result<()>;
    /// `7A9150(&running, &order, index)`.
    fn evaluate(
        &mut self,
        goal: GoalId,
        running: &mut [u32],
        order: &[GoalId],
        index: usize,
        ctl: &mut Controller,
    ) -> Result<()>;
    /// `7A7E30(&running, &order, index)`.
    fn assign(
        &mut self,
        goal: GoalId,
        running: &mut [u32],
        order: &[GoalId],
        index: usize,
        ctl: &mut Controller,
    ) -> Result<()>;
    /// `79DF70(0)`: fiber time-slice check; the world may advance here.
    fn yield_point(&mut self, ctl: &mut Controller) -> Result<()>;
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}
fn require(goal: Option<GoalId>) -> Result<GoalId> {
    goal.ok_or_else(|| Error::World("null SAI goal node (native null dereference)".into()))
}
fn status_list(status: u32) -> Result<usize> {
    usize::try_from(status)
        .ok()
        .filter(|s| *s < 3)
        .ok_or_else(|| Error::World(format!("SAI goal status {status} has no bucket list")))
}

/// Complete `7A7700` (list maintenance before selection).
pub fn reevaluate_goals(ctl: &mut Controller, world: &mut impl EngineWorld) -> Result<()> {
    // Expired goals are destroyed; null nodes are removed (7A32F0/7A2FB0).
    for b in 0..BUCKETS {
        for l in 0..3 {
            let mut k = 0;
            while k < ctl.bucket(b)?.lists[l].len() {
                let goal = require(ctl.bucket(b)?.lists[l][k])?;
                if world.expired(goal, ctl)? {
                    world.destroy(goal, ctl)?;
                    ctl.bucket(b)?.lists[l][k] = None;
                }
                k += 1;
            }
            ctl.bucket(b)?.lists[l].retain(Option::is_some);
        }
    }
    world.yield_point(ctl)?;
    for b in 0..BUCKETS {
        for l in 0..3 {
            let mut k = 0;
            while k < ctl.bucket(b)?.lists[l].len() {
                let goal = require(ctl.bucket(b)?.lists[l][k])?;
                world.reevaluate(goal, ctl)?;
                world.yield_point(ctl)?;
                k += 1;
            }
        }
    }
    // Inactive goals with a strictly positive base priority become
    // candidates (collected head-first, appended after demotion).
    let mut promoted: Vec<VecDeque<Option<GoalId>>> = vec![VecDeque::new(); BUCKETS];
    for (b, promoted) in promoted.iter_mut().enumerate() {
        let mut k = 0;
        while k < ctl.bucket(b)?.lists[INACTIVE].len() {
            let goal = require(ctl.bucket(b)?.lists[INACTIVE][k])?;
            if host(ctl.goal(goal)?.base) > 0.0 {
                promoted.push_front(Some(goal));
                if ctl.goal(goal)?.status != 1 {
                    world.deselect(goal, ctl)?;
                    ctl.goal_mut(goal)?.status = 1;
                }
                ctl.bucket(b)?.lists[INACTIVE][k] = None;
            }
            k += 1;
        }
        ctl.bucket(b)?.lists[INACTIVE].retain(Option::is_some);
        world.yield_point(ctl)?;
    }
    // Candidate/selected goals whose base priority is <= 0 (ordered) are
    // pushed at the head of the inactive list.
    for b in 0..BUCKETS {
        for l in [CANDIDATE, SELECTED] {
            let mut k = 0;
            while k < ctl.bucket(b)?.lists[l].len() {
                let goal = require(ctl.bucket(b)?.lists[l][k])?;
                if host(ctl.goal(goal)?.base) <= 0.0 {
                    ctl.bucket(b)?.lists[INACTIVE].push_front(Some(goal));
                    if ctl.goal(goal)?.status != 0 {
                        world.deselect(goal, ctl)?;
                        ctl.goal_mut(goal)?.status = 0;
                    }
                    ctl.bucket(b)?.lists[l][k] = None;
                }
                k += 1;
            }
        }
        ctl.bucket(b)?.lists[CANDIDATE].retain(Option::is_some);
        ctl.bucket(b)?.lists[SELECTED].retain(Option::is_some);
        world.yield_point(ctl)?;
    }
    for (b, promoted) in promoted.into_iter().enumerate() {
        ctl.bucket(b)?.lists[CANDIDATE].extend(promoted);
    }
    Ok(())
}

fn subtract_costs(running: &mut [u32], costs: &[u32], p: aa_numeric::Precision) -> Result<()> {
    use crate::x87::{n, num, store};
    for i in (0..costs.len()).rev() {
        let slot = running.get_mut(i).ok_or_else(|| {
            Error::World(format!("goal cost index {i} exceeds the resource array"))
        })?;
        *slot = store(num(n(*slot)?.sub(&n(costs[i])?, p))?)?;
    }
    Ok(())
}

/// Complete `7A93B0` (priority-ordered goal selection with resource
/// reservation). `economy` is the player's resource array (economy
/// +1C4/+1C8), copied fresh before each of the two passes.
pub fn select_goals(
    ctl: &mut Controller,
    economy: &[u32],
    world: &mut impl EngineWorld,
    p: aa_numeric::Precision,
) -> Result<()> {
    let total: usize = ctl
        .buckets
        .iter()
        .map(|b| b.lists[CANDIDATE].len() + b.lists[SELECTED].len())
        .sum();
    if total == 0 {
        return Ok(());
    }
    let mut collected: Vec<Option<GoalId>> = Vec::new();
    for b in 0..BUCKETS {
        for l in [CANDIDATE, SELECTED] {
            while let Some(goal) = ctl.bucket(b)?.lists[l].pop_front() {
                collected.push(goal);
            }
        }
    }
    let mut order: Vec<GoalId> = collected.into_iter().map(require).collect::<Result<_>>()?;
    {
        let goals = &ctl.goals;
        sort_by_base_priority(&mut order, |g| {
            goals
                .get(g)
                .map(|s| s.base)
                .ok_or_else(|| Error::World(format!("missing SAI goal {g}")))
        })?;
    }
    world.yield_point(ctl)?;
    let mut running = economy.to_vec();
    let count = order.len();
    let mut i = 0_isize;
    while (i as usize) < total {
        let index = i as usize;
        let goal = order[index];
        world.evaluate(goal, &mut running, &order, index, ctl)?;
        if ctl.goal(goal)?.status == 2 {
            let costs = ctl.goal(order[index])?.costs.clone();
            subtract_costs(&mut running, &costs, p)?;
            // Move the goal up while its current priority is strictly higher.
            let mut k = index;
            while k > 0 {
                let cur = ctl.goal(order[k])?.current;
                let prev = ctl.goal(order[k - 1])?.current;
                if host(cur).partial_cmp(&host(prev)) != Some(std::cmp::Ordering::Greater) {
                    break;
                }
                order.swap(k, k - 1);
                k -= 1;
            }
            // Then down while strictly lower; a downward move revisits `i`.
            if k < count - 1 {
                let mut moved = false;
                let mut revisit = true;
                while k < count - 1 {
                    let cur = ctl.goal(order[k])?.current;
                    let next = ctl.goal(order[k + 1])?.current;
                    if host(cur).partial_cmp(&host(next)) != Some(std::cmp::Ordering::Less) {
                        revisit = moved;
                        break;
                    }
                    order.swap(k, k + 1);
                    k += 1;
                    moved = true;
                }
                if revisit {
                    i -= 1;
                }
            }
        }
        world.yield_point(ctl)?;
        i += 1;
    }
    for &goal in &order {
        let status = ctl.goal(goal)?.status;
        if status != 2 {
            let kind = world.goal_type(goal, ctl)? as usize;
            let list = status_list(status)?;
            ctl.bucket(kind)?.lists[list].push_front(Some(goal));
        }
    }
    let mut selected = Vec::with_capacity(order.len());
    for goal in order {
        if ctl.goal(goal)?.status == 2 {
            selected.push(goal);
        }
    }
    world.yield_point(ctl)?;
    let mut running = economy.to_vec();
    for (index, &goal) in selected.iter().enumerate() {
        if ctl.goal(goal)?.status == 2 {
            world.assign(goal, &mut running, &selected, index, ctl)?;
            if ctl.goal(goal)?.status == 2 {
                let costs = ctl.goal(goal)?.costs.clone();
                subtract_costs(&mut running, &costs, p)?;
            }
        }
        world.yield_point(ctl)?;
    }
    for &goal in &selected {
        let status = ctl.goal(goal)?.status;
        let kind = world.goal_type(goal, ctl)? as usize;
        let list = status_list(status)?;
        ctl.bucket(kind)?.lists[list].push_front(Some(goal));
    }
    world.yield_point(ctl)?;
    Ok(())
}
