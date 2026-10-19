//! Original Mind state activation and stack lifecycle. Required synchronous
//! callbacks connect state entry/exit and scheduling to the live engine.
#![forbid(unsafe_code)]

pub mod ability_decision;
pub mod ability_motion;
pub mod action;
pub mod action_mask;
pub mod cai_engage;
pub mod cai_engage_think;
pub mod cai_guard;
pub mod cai_member_added;
pub mod cai_reactions;
pub mod cai_rout;
pub mod cai_sleep;
pub mod cai_states;
pub mod cai_thinks;
pub mod command;
pub mod construct;
pub mod decision_score;
pub mod disable_command;
pub mod element_command;
pub mod element_update;
pub mod element_move;
pub mod element_move_continuation;
pub mod moving_target;
pub mod recover_select;
pub mod rout_score;
pub mod rout_select;
pub mod sai;
pub mod uai_kill;
pub mod uai_thinks;
pub mod x87;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("missing Mind identity {0}")]
    MissingMind(u32),
    #[error("missing state identity {0}")]
    MissingState(u32),
    #[error("native state callback left the required active state null")]
    MissingActiveState,
    #[error("could not allocate state stack storage")]
    Allocation,
    #[error("Mind world callback failed: {0}; earlier effects remain committed")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MindId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StateId(pub u32);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mind {
    pub actor: u32,
    /// Native +10/+14 linked stack, head first. A node can contain null state.
    pub stack: VecDeque<Option<StateId>>,
    /// Native +18/+1C pending commands and +20 active command. These identities
    /// are preserved for the separate command dispatcher; no execution guessed.
    pub pending_commands: VecDeque<u32>,
    pub current_command: Option<u32>,
    pub words_24_30: [u32; 4],
}
impl Mind {
    /// 52C130/4DB020 base fields. Derived element, regiment and aircraft state
    /// constructors must separately register their embedded states.
    pub fn base(actor: u32) -> Self {
        Self {
            actor,
            stack: VecDeque::new(),
            pending_commands: VecDeque::new(),
            current_command: None,
            words_24_30: [0; 4],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub parent: MindId,
    /// Native metadata pointer+8 byte. Supplied from the registered state type;
    /// not inferred from status or a guessed state name.
    pub discard_on_suspend: bool,
    /// State+0C, set to1 before original virtual OnEnter+18.
    pub status: u32,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arena {
    pub minds: BTreeMap<MindId, Mind>,
    pub states: BTreeMap<StateId, State>,
}
impl Arena {
    pub fn mind(&self, id: MindId) -> Result<&Mind> {
        self.minds.get(&id).ok_or(Error::MissingMind(id.0))
    }
    pub fn mind_mut(&mut self, id: MindId) -> Result<&mut Mind> {
        self.minds.get_mut(&id).ok_or(Error::MissingMind(id.0))
    }
    pub fn state(&self, id: StateId) -> Result<&State> {
        self.states.get(&id).ok_or(Error::MissingState(id.0))
    }
    pub fn state_mut(&mut self, id: StateId) -> Result<&mut State> {
        self.states.get_mut(&id).ok_or(Error::MissingState(id.0))
    }
    fn top(&self, id: MindId) -> Result<Option<StateId>> {
        Ok(self.mind(id)?.stack.front().copied().flatten())
    }
}

/// Callback order and mutable rereads are part of the native contract. Methods
/// have no permissive defaults. Errors do not roll back completed state changes.
pub trait World {
    fn can_activate(&mut self, state: StateId, arena: &mut Arena) -> Result<bool>;
    fn enter(&mut self, state: StateId, arena: &mut Arena) -> Result<()>;
    fn leave(&mut self, state: StateId, arena: &mut Arena) -> Result<()>;
    /// Mind Updatable virtual+4, with the actual raw delay and data words.
    fn schedule(
        &mut self,
        mind: MindId,
        delay_bits: u32,
        data: u32,
        arena: &mut Arena,
    ) -> Result<()>;
    /// Mind virtual+1A0, reached after clearing the stack in52A3C0.
    fn reset_default(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// Mind virtual+1A4 after state entry or reset.
    fn state_changed(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
}

fn leave_top<W: World>(mind: MindId, arena: &mut Arena, world: &mut W) -> Result<()> {
    if let Some(state) = arena.top(mind)? {
        world.leave(state, arena)?;
    }
    Ok(())
}
fn enter_top<W: World>(
    mind: MindId,
    required: bool,
    arena: &mut Arena,
    world: &mut W,
) -> Result<()> {
    let Some(state) = arena.top(mind)? else {
        return if required {
            Err(Error::MissingActiveState)
        } else {
            Ok(())
        };
    };
    arena.state_mut(state)?.status = 1;
    // The original reads the head again after the status write (no callback).
    world.enter(state, arena)
}

/// Native52A290. Exit callback may replace the head; discard and insertion use
/// the resulting live stack. Scheduling precedes the new head's entry callback.
pub fn push<W: World>(
    mind: MindId,
    state: StateId,
    arena: &mut Arena,
    world: &mut W,
) -> Result<()> {
    if arena.top(mind)?.is_some() {
        leave_top(mind, arena, world)?;
        if let Some(top) = arena.top(mind)?
            && arena.state(top)?.discard_on_suspend
        {
            arena.mind_mut(mind)?.stack.pop_front();
        }
    }
    let stack = &mut arena.mind_mut(mind)?.stack;
    stack.try_reserve(1).map_err(|_| Error::Allocation)?;
    stack.push_front(Some(state));
    world.schedule(mind, 0, 0, arena)?;
    enter_top(mind, true, arena, world)?;
    world.state_changed(mind, arena)
}

/// Native52A420. Only the former head receives leave; remaining stack nodes
/// are removed without synthetic exit callbacks.
pub fn replace_all<W: World>(
    mind: MindId,
    state: StateId,
    arena: &mut Arena,
    world: &mut W,
) -> Result<()> {
    leave_top(mind, arena, world)?;
    arena.mind_mut(mind)?.stack.clear();
    push(mind, state, arena, world)
}

/// Native52A470 resumes the next state before pushing its replacement. Thus
/// that resumed state can receive enter followed by leave in this operation.
pub fn replace_top<W: World>(
    mind: MindId,
    state: StateId,
    arena: &mut Arena,
    world: &mut W,
) -> Result<()> {
    leave_top(mind, arena, world)?;
    arena.mind_mut(mind)?.stack.pop_front();
    enter_top(mind, false, arena, world)?;
    push(mind, state, arena, world)
}

/// Native52A810. Unknown modes preserve the native accepted-no-transition
/// behavior after eligibility, rather than being interpreted as a new order.
pub fn activate<W: World>(
    state: StateId,
    mode: u32,
    arena: &mut Arena,
    world: &mut W,
) -> Result<bool> {
    if mode != 0 && mode != 1 && !world.can_activate(state, arena)? {
        return Ok(false);
    }
    match mode {
        0 => {
            arena.state_mut(state)?.status = 1;
            world.enter(state, arena)?;
        }
        1 => replace_top(arena.state(state)?.parent, state, arena, world)?,
        2 => replace_all(arena.state(state)?.parent, state, arena, world)?,
        3 => push(arena.state(state)?.parent, state, arena, world)?,
        _ => {}
    }
    Ok(true)
}

/// Native 52A340 (finish the current state): leave the head state, unlink the
/// (reread) head node, schedule an immediate update, then resume the new head
/// (status 1 and enter) or restore the derived default, and report the change.
/// A missing head or a null head state is a native null dereference and is
/// returned as `MissingActiveState`; earlier callbacks stay committed.
pub fn pop<W: World>(mind: MindId, arena: &mut Arena, world: &mut W) -> Result<()> {
    let top = arena.top(mind)?.ok_or(Error::MissingActiveState)?;
    world.leave(top, arena)?;
    arena.mind_mut(mind)?.stack.pop_front();
    world.schedule(mind, 0, 0, arena)?;
    if arena.mind(mind)?.stack.is_empty() {
        world.reset_default(mind, arena)?;
    } else {
        enter_top(mind, true, arena, world)?;
    }
    world.state_changed(mind, arena)
}

/// Native 529E60 `KKC_TAIState::Think` base: a state thinking while not
/// active (status != 2) is forced to 2. Returns true in that case, where the
/// original also emits its one-time "wrong activity" diagnostic (global
/// latch B3D248, not simulation state).
pub fn think_base(state: StateId, arena: &mut Arena) -> Result<bool> {
    let state = arena.state_mut(state)?;
    if state.status == 2 {
        return Ok(false);
    }
    state.status = 2;
    Ok(true)
}

/// Native 52A000: the state is the Mind's current (stack head) state.
pub fn is_current(state: StateId, arena: &Arena) -> Result<bool> {
    let mind = arena.state(state)?.parent;
    Ok(arena.top(mind)? == Some(state))
}

/// Native 52A1F0: the Mind actor (+20/+24 position) is within 0.5 world units
/// of `point` (x stored to binary32, squared sum compared with 0.25).
pub fn within_arrival(
    position: [u32; 2],
    point: [u32; 2],
    p: aa_numeric::Precision,
) -> Result<bool> {
    use x87::{n, num, store};
    let dx = store(num(n(point[0])?.sub(&n(position[0])?, p))?)?;
    let dy = num(n(point[1])?.sub(&n(position[1])?, p))?;
    let squared = num(num(dy.mul(&dy, p))?.add(&num(n(dx)?.mul(&n(dx)?, p))?, p))?;
    Ok(squared.compare(&n(0x3e80_0000)?) == std::cmp::Ordering::Less)
}

/// Native52A3C0 clears the stack, schedules an immediate update, restores the
/// derived default state, then reports its change. Required callbacks remain real.
pub fn clear<W: World>(mind: MindId, arena: &mut Arena, world: &mut W) -> Result<()> {
    leave_top(mind, arena, world)?;
    arena.mind_mut(mind)?.stack.clear();
    world.schedule(mind, 0, 0, arena)?;
    world.reset_default(mind, arena)?;
    world.state_changed(mind, arena)
}

pub mod node_reached;
pub mod saved_load;
pub mod saved_command;
pub mod regiment_load;
pub mod cai_member_command;
pub mod cai_arrivals;
pub mod regiment_update;
