//! Native4CFCC0 action-mask publication. Parent rebuilding is synchronous:
//!529D70 loads Organization.owner and tail-calls4D1350; it does not queue work.
use crate::{ActorId, ability_flags::Result};

/// All getters read live state. UI and renderer publication are required services,
/// including when a callback changed an owner or renderer reference meanwhile.
pub trait World {
    /// None means no Element or its organization pointer is null.
    fn element_organization(&self, actor: ActorId) -> Result<Option<ActorId>>;
    /// The Organization component must exist when the Element parent is nonnull.
    fn organization_owner(&self, organization_actor: ActorId) -> Result<ActorId>;
    fn rebuild_actions(&mut self, actor: ActorId) -> Result<()>;
    fn owner(&mut self, actor: ActorId) -> Result<Option<ActorId>>;
    /// Raw Object+4C is tested as a signed byte by native JLE.
    fn object_byte_4c(&self, actor: ActorId) -> Result<u8>;
    fn refresh_owner_views(&mut self, owner: Option<ActorId>) -> Result<()>;
    /// None means Thing+C is null. The native bit17 test uses current ref+54.
    fn renderer_flags(&self, actor: ActorId) -> Result<Option<u32>>;
    fn refresh_selected_view(&mut self, actor: ActorId) -> Result<()>;
}

pub fn changed(actor: ActorId, world: &mut impl World) -> Result<()> {
    if let Some(parent) = world.element_organization(actor)? {
        let owner = world.organization_owner(parent)?;
        world.rebuild_actions(owner)?;
    }
    if world.owner(actor)?.is_some() && (world.object_byte_4c(actor)? as i8) > 0 {
        let owner = world.owner(actor)?;
        // A second null result is deliberately forwarded, not substituted.
        world.refresh_owner_views(owner)?;
    }
    if world
        .renderer_flags(actor)?
        .is_some_and(|flags| flags & (1 << 17) != 0)
    {
        world.refresh_selected_view(actor)?;
    }
    Ok(())
}
