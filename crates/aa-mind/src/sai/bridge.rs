//! Conversions between the controller's native goal header
//! (`aa_navigation::sai_state::GoalBase`, constructor `7A66E0`) and the goal
//! views used by the ported engine (`engine::GoalState`, `assign::ActorList`,
//! `goal_priority::Goal`).
//!
//! GoalBase keeps the +0C/+10/+14 actor list as raw native words. Only the
//! unlinked state (null head and tail, as `7A66E0` constructs it) carries no
//! hidden node memory, so only that state converts; a linked list is an
//! error (nodes are never invented). Fields a view does not carry are left
//! untouched on write-back.
use super::assign::ActorList;
use super::engine::GoalState;
use super::goal_priority::Goal;
use crate::{Error, Result};
use aa_navigation::sai_state::GoalBase;

/// `GoalState` view: status +08, costs +1C/+20, travel +30, current +34,
/// base +38.
pub fn goal_state(base: &GoalBase) -> GoalState {
    GoalState {
        status: base.status,
        costs: base.costs.clone(),
        current: base.current_priority,
        base: base.priority,
        travel: base.travel,
    }
}

/// The +0C/+10/+14 actor list, available only while it is unlinked.
pub fn goal_actors(base: &GoalBase) -> Result<ActorList> {
    if !base.actor_list_unlinked() {
        return Err(Error::World(
            "goal actor list holds native node pointers; it cannot be converted".into(),
        ));
    }
    Ok(ActorList::from_actors([], base.actor_count()))
}

/// Writes a `GoalState` (and optionally its actor list) back, preserving
/// controller, fuzzy +18, cost capacity +24, inertia +28 and byte +2C. A
/// non-empty list cannot be expressed as native node pointers.
pub fn write_goal_state(
    state: &GoalState,
    actors: Option<&ActorList>,
    base: &mut GoalBase,
) -> Result<()> {
    if let Some(list) = actors {
        if !list.actors().is_empty() || list.tail_actor().is_some() {
            return Err(Error::World(
                "a linked actor list cannot be written into the native goal header".into(),
            ));
        }
        base.assignment_words = [0, 0, list.count as u32];
    }
    base.status = state.status;
    base.costs = state.costs.clone();
    base.current_priority = state.current;
    base.priority = state.base;
    base.travel = state.travel;
    Ok(())
}

/// `goal_priority::Goal` view used by `7A5200`/`7A3860`: status +08, fuzzy
/// +18, costs, applied inertia +28, priority +38.
pub fn priority_goal(base: &GoalBase) -> Goal {
    Goal {
        status: base.status,
        fuzzy: base.fuzzy,
        costs: base.costs.clone(),
        inertia_applied: base.inertia,
        priority: base.priority,
    }
}

/// Writes a `goal_priority::Goal` back (other GoalBase fields untouched).
pub fn write_priority_goal(goal: &Goal, base: &mut GoalBase) {
    base.status = goal.status;
    base.fuzzy = goal.fuzzy;
    base.costs = goal.costs.clone();
    base.inertia = goal.inertia_applied;
    base.priority = goal.priority;
}
