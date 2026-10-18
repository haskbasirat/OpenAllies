//! Original Structure589A60 action contributor with ordinary GActor virtuals
//! +158=4CE980/588990 and+15C=4CE9A0. Parent rebuilding is synchronous;
//! callbacks and earlier writes are not rolled back after a later failure.
use crate::{
    ActorId,
    ability_flags::{Error, Result, set_action},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StructureReference {
    /// Structure header+4, which588990 compares with Settlement+14.
    pub owner: ActorId,
    /// Structure+10 parent actor, not an authored definition reference.
    pub parent: Option<ActorId>,
}

/// All reads use the actual current actor/component owners. The input
/// `structure` identifies stable component storage even if its header owner
/// changes inside the parent's4D1350 callback. Setters write only action bits.
pub trait World {
    fn structure_owner(&self, structure: ActorId) -> Result<ActorId>;
    fn structure_reference(&self, actor: ActorId) -> Result<Option<StructureReference>>;
    /// OuterNone is an absent Settlement; innerNone is its null+14 reference.
    fn settlement_reference(&self, actor: ActorId) -> Result<Option<Option<ActorId>>>;
    fn action_bits(&self, actor: ActorId) -> Result<u64>;
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> Result<()>;
    /// Exact live Structure+18 nonzero test; missing payloads must error.
    fn queue_nonempty(&self, structure: ActorId) -> Result<bool>;
    fn rebuild_actions(&mut self, actor: ActorId) -> Result<()>;
}

/// Ordinary GActor+158:4CE980 ->588990. A nonnull Structure parent with
/// no Settlement would be dereferenced natively; report a required dependency
/// error rather than treating it as an unrelated structure.
pub fn settlement_center(world: &impl World, actor: ActorId) -> Result<bool> {
    let Some(reference) = world.structure_reference(actor)? else {
        return Ok(false);
    };
    let Some(parent) = reference.parent else {
        return Ok(false);
    };
    let center = world.settlement_reference(parent)?.ok_or(Error::Missing(
        "588990 parent Settlement+14 unavailable (native null dereference)",
    ))?;
    Ok(center == Some(reference.owner))
}

///4CE9A0 repeats+158 then rereads the current Structure+10.
pub fn center_parent(world: &impl World, actor: ActorId) -> Result<Option<ActorId>> {
    if !settlement_center(world, actor)? {
        return Ok(None);
    }
    Ok(world
        .structure_reference(actor)?
        .ok_or(Error::Missing(
            "4CE9A0 Structure disappeared after successful+158",
        ))?
        .parent)
}

///589A60. Clear bit13 before merging the rebuilt parent's CURRENT mask;
/// the merge may intentionally set bit13 again. Bit18 follows the stable
/// Structure component's current queue, including changes during rebuild.
pub fn contribute(world: &mut impl World, structure: ActorId) -> Result<()> {
    let actor = world.structure_owner(structure)?;
    if settlement_center(world, actor)? {
        let actor = world.structure_owner(structure)?;
        if let Some(parent) = center_parent(world, actor)? {
            world.rebuild_actions(parent)?;
            let actor = world.structure_owner(structure)?;
            let mut bits = world.action_bits(actor)?;
            set_action(&mut bits, 13, false)?;
            world.set_action_bits(actor, bits)?;
            let actor = world.structure_owner(structure)?;
            let parent_bits = world.action_bits(parent)?;
            let bits = world.action_bits(actor)? | parent_bits;
            world.set_action_bits(actor, bits)?;
        }
    }
    let actor = world.structure_owner(structure)?;
    let enabled = world.queue_nonempty(structure)?;
    let mut bits = world.action_bits(actor)?;
    set_action(&mut bits, 18, enabled)?;
    world.set_action_bits(actor, bits)
}
