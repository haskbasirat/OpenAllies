//! UAIStateMove continuation 0x531520 (ResultPlan 0x5336B0's "continue the
//! current route" branch): rate-limited low-path replanning to the next
//! waypoint, then ActionGo or the owner failure virtual.
use super::{
    Environment, Error, LowPathSolver, LowPathWorld, ONE, Outcome, Request, Result, ZERO, f, node,
    pop_front_kind0, push_back, solve,
};
use crate::MoveState;
use crate::result_plan::{CollisionContext, Object};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationInput {
    /// ResultPlan's unmodified second argument; only its low byte is tested
    /// (nonzero sets descriptor flag 0x20000000).
    pub argument: u32,
    /// GWorld+0xE0 simulation time (binary32).
    pub world_time_bits: u32,
    /// `LowPathingWaitMinimum` SVar (type 2), looked up lazily by 0x531520.
    pub wait_minimum_bits: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContinuationOutcome {
    /// Too soon since the last low-path solve: stop and retry in 1.0.
    Waited,
    /// Solve returned 0: kind-0 waypoints dropped, owner virtual +0x15C(1).
    Failed,
    /// Solve returned this nonzero status; ActionGo(0) was issued.
    Solved(u32),
}

/// Result of 0x531520 with the descriptor it built (a native stack local that
/// is discarded on return; exposed for callers and audits).
#[derive(Clone, Debug)]
pub struct Continuation {
    pub outcome: ContinuationOutcome,
    pub context: CollisionContext,
    /// The Solve result when the rate limit allowed one.
    pub solve: Option<Outcome>,
}

/// Owner and actor services of 0x531520. The low-path solve itself uses the
/// [`LowPathWorld`] services. `state` is the UAIStateMove.
pub trait ContinuationWorld: LowPathWorld {
    /// 0x554350(actor, 0) for the state's actor (0x529C30).
    fn collision_context(&mut self, state: &mut MoveState) -> Result<CollisionContext>;
    /// 0x4CE6E0 on the generation-valid target.
    fn replacement_actor(&mut self, object: Object, state: &mut MoveState) -> Result<Object>;
    /// Owner (UAI, state +4) word +0x54; 2 adds 0x400 to descriptor +0x38/+0x3C.
    fn owner_word_54(&mut self, state: &mut MoveState) -> Result<u32>;
    /// Actor +0x20/+0x24, read when the solve is entered.
    fn actor_position(&mut self, state: &mut MoveState) -> Result<[u32; 2]>;
    /// Entire 0x52D660.
    fn stop_action(&mut self, state: &mut MoveState) -> Result<()>;
    /// 0x4CFC80 on the actor.
    fn actor_stop_animation(&mut self, state: &mut MoveState) -> Result<()>;
    /// Owner timer (owner +8) virtual +4 with the native second argument 0.
    fn schedule(&mut self, delay_bits: u32, state: &mut MoveState) -> Result<()>;
    /// Owner virtual +0x15C with the native argument 1.
    fn path_failed(&mut self, state: &mut MoveState) -> Result<()>;
    /// 0x5300A0 with the native speed argument (0). Implementations call
    /// [`crate::try_action_go`] with the authoritative actor.
    fn action_go(&mut self, speed_bits: u32, state: &mut MoveState) -> Result<()>;
}

/// Complete 0x531520. `last_solve_time_bits` is UAIStateMove +0x78. Effects
/// made before an error remain; callers must not retry automatically.
pub fn continue_current<W: ContinuationWorld>(
    input: ContinuationInput,
    state: &mut MoveState,
    last_solve_time_bits: &mut u32,
    solver: &mut LowPathSolver,
    env: &mut Environment<'_>,
    world: &mut W,
) -> Result<Continuation> {
    let p = env.options.precision;
    let goal = state
        .path
        .nodes
        .first()
        .map_or(state.destination_bits, |node| {
            [node.words[0], node.words[1]]
        });
    let mut context = world.collision_context(state)?;
    context.words[8] = match world.resolve_actor(state.target_id) {
        Some(target) => world.replacement_actor(target, state)?.0,
        None => 0,
    };
    if input.argument & 0xff != 0 {
        context.words[14] |= 0x2000_0000;
    }
    if world.owner_word_54(state)? == 2 {
        context.words[14] |= 0x400;
        context.words[15] |= 0x400;
    }
    let last = f(*last_solve_time_bits)?;
    let due = last.compare(&f(ZERO)?) != Ordering::Greater
        || f(input.world_time_bits)?
            .sub(&last, p)?
            .compare(&f(input.wait_minimum_bits)?)
            != Ordering::Less;
    if !due {
        world.stop_action(state)?;
        world.actor_stop_animation(state)?;
        world.schedule(ONE, state)?;
        return Ok(Continuation {
            outcome: ContinuationOutcome::Waited,
            context,
            solve: None,
        });
    }
    let was_empty = state.path.nodes.is_empty();
    *last_solve_time_bits = input.world_time_bits;
    let start = world.actor_position(state)?;
    let request = Request {
        start_bits: start,
        goal_bits: goal,
    };
    let outcome = solve(solver, env, request, &mut state.path, &mut context, world)?;
    let status = outcome.status;
    if status == 0 {
        pop_front_kind0(&mut state.path);
        world.path_failed(state)?;
        return Ok(Continuation {
            outcome: ContinuationOutcome::Failed,
            context,
            solve: Some(outcome),
        });
    }
    if was_empty {
        let last = *state.path.nodes.last().ok_or(Error::EmptySolvedPath)?;
        let position = [last.words[0], last.words[1]];
        push_back(
            &mut state.path,
            node(position, [last.words[2], last.words[3]], 1),
        )?;
    }
    state.waypoint_offset_bits = 0;
    world.action_go(0, state)?;
    Ok(Continuation {
        outcome: ContinuationOutcome::Solved(status),
        context,
        solve: Some(outcome),
    })
}
