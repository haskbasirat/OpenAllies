//! Original Research action contributor583FC0. Group13 state type1 disables
//! research without reading ActorData+558; otherwise its list-head presence
//! determines action35. The only mutation is517FA0, after these reads.
use crate::{
    ActorId,
    ability_flags::{Error, Result, set_action},
    state,
};

/// Authoritative field reads, not gameplay callbacks. The storage identity
/// addresses the same Research component throughout the native function.
pub trait World {
    fn research_owner(&self, research: ActorId) -> Result<ActorId>;
    fn state_bits(&self, actor: ActorId) -> Result<u64>;
    /// Loaded ActorStateData+28 for the selected group13 descriptor.
    fn state_type(&self, state_index: u8) -> Result<u32>;
    /// Completed ActorData+558 list head, not the in-progress component list.
    fn research_items_present(&self, actor: ActorId) -> Result<bool>;
    fn action_bits(&self, actor: ActorId) -> Result<u64>;
    /// Equivalent to the native low-word then high-word stores, with no
    /// intervening callback. All other action bits must be retained.
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> Result<()>;
}

pub fn contribute(world: &mut impl World, research: ActorId) -> Result<()> {
    let owner = world.research_owner(research)?;
    let selected = state::group(world.state_bits(owner)?, 13)
        .map_err(|error| Error::World(error.to_string()))?
        .ok_or(Error::Missing(
            "583FC0 group13 selected -1; native ActorState lookup is invalid",
        ))?;
    let enabled = world.state_type(selected.index)? != 1
        && world.research_items_present(world.research_owner(research)?)?;
    let owner = world.research_owner(research)?;
    let mut bits = world.action_bits(owner)?;
    set_action(&mut bits, 35, enabled)?;
    world.set_action_bits(owner, bits)
}
