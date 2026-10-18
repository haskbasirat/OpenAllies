//! Kingdom HQ scans 4E1430/4E1E40 used by Organization579580.
//! Actual HQ eligibility remains an explicit service, not an inferred flag.
use crate::{ActorId, ability_flags::Result};

pub trait World {
    type Definition;
    /// Owned Kingdom+2DC vector length (+2E0 in the original).
    fn hq_count(&self, owner: ActorId) -> Result<usize>;
    /// Re-read the current vector entry on each native iteration.
    fn hq_at(&self, owner: ActorId, index: usize) -> Result<ActorId>;
    fn actor_definition(&self, actor: ActorId) -> Result<Self::Definition>;
    /// HQ+D8 component573E50(definition).
    fn hq_can_recruit(&mut self, hq: ActorId, definition: &Self::Definition) -> Result<bool>;
    /// HQ+D8 component574BF0(definition, false), including574810 when reached.
    fn hq_accepts(&mut self, hq: ActorId, definition: &Self::Definition) -> Result<bool>;
    /// Organization+D0 followed by the generation-checked global ID lookup.
    fn organization_reference(&self, actor: ActorId) -> Result<Option<ActorId>>;
}

/// Original4E1430. The count is captured once, entries are read lazily, and
/// the first successful HQ ends the scan without inspecting later entries.
pub fn can_recruit<W: World>(
    world: &mut W,
    owner: ActorId,
    definition: &W::Definition,
) -> Result<bool> {
    let count = world.hq_count(owner)?;
    for index in 0..count {
        let hq = world.hq_at(owner, index)?;
        if world.hq_can_recruit(hq, definition)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Original4E1E40.574BF0 runs for every HQ, including the currently referenced
/// HQ. Only successful entries resolve Organization+D0; invalid generations
/// resolve to null. Preserve duplicates and native wrapping i32 addition.
pub fn recruit_count<W: World>(world: &mut W, owner: ActorId, actor: ActorId) -> Result<i32> {
    let count = world.hq_count(owner)?;
    let mut result = 0i32;
    for index in 0..count {
        let hq = world.hq_at(owner, index)?;
        let definition = world.actor_definition(actor)?;
        if world.hq_accepts(hq, &definition)? && world.organization_reference(actor)? != Some(hq) {
            result = result.wrapping_add(1);
        }
    }
    Ok(result)
}
