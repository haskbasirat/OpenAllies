//! Original Mind-state priority admission (the metadata +4 word of each
//! state type):
//!
//! * `529F80(priority)` on a Mind: `priority` is at least the current
//!   (stack head) state's priority, or at least 0 without a head state
//!   (signed compare) ([`admits`]).
//! * `529FC0` on a state (the usual state virtual +4, "can activate"): the
//!   same test with the state's own priority ([`can_activate`]).
use crate::{Arena, MindId, Result, StateId};

/// `529F80(priority)` on `mind`; `priority_of` reads a state's metadata +4.
pub fn admits(
    mind: MindId,
    priority: i32,
    arena: &Arena,
    mut priority_of: impl FnMut(StateId) -> Result<i32>,
) -> Result<bool> {
    let head = match arena.mind(mind)?.stack.front().copied().flatten() {
        Some(state) => priority_of(state)?,
        None => 0,
    };
    Ok(priority >= head)
}

/// `529FC0` on `state`.
pub fn can_activate(
    state: StateId,
    arena: &Arena,
    mut priority_of: impl FnMut(StateId) -> Result<i32>,
) -> Result<bool> {
    let own = priority_of(state)?;
    admits(arena.state(state)?.parent, own, arena, priority_of)
}
