//! Native UAI StopAction52D660. Mover stop5767C0 is implemented by aa-movement;
//! this layer preserves its dispatch and the subsequent live component checks.
use crate::{Arena, Result, StateId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub mover: Option<u32>,
    pub active_component: Option<u32>,
}
pub trait World {
    fn actor(&self, actor: u32) -> Result<Actor>;
    fn component_owner(&self, component: u32) -> Result<u32>;
    /// Bind to actual aa_movement::stop_ground_motion with both flags false;
    /// canceling scheduled Mover updates can synchronously change actor state.
    fn stop_mover(
        &mut self,
        mover: u32,
        keep_scheduled: bool,
        clear_speed: bool,
        arena: &mut Arena,
    ) -> Result<()>;
    /// Native4CFC80, including Character/default component notifications.
    fn default_stance(&mut self, actor: u32, arena: &mut Arena) -> Result<()>;
}
fn actor_id(state: StateId, arena: &Arena) -> Result<u32> {
    Ok(arena.mind(arena.state(state)?.parent)?.actor)
}
pub fn stop<W: World>(state: StateId, arena: &mut Arena, world: &mut W) -> Result<()> {
    if let Some(mover) = world.actor(actor_id(state, arena)?)?.mover {
        world.stop_mover(mover, false, false, arena)?;
    }
    if let Some(mover) = world.actor(actor_id(state, arena)?)?.mover {
        let owner = world.component_owner(mover)?;
        if world.actor(owner)?.active_component == Some(mover) {
            return Ok(());
        }
    }
    world.default_stance(actor_id(state, arena)?, arena)
}
