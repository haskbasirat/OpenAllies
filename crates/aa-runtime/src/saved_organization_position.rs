//! Native 57C800/57C680 over the actual in-progress saved Organization.
//! No parallel position cache or substitute Mind predicate is introduced.
use crate::catalog::{Catalog, RestoredMindParts};
use aa_actors::{ActorId, components::ComponentSlot, element_position as position};
use aa_body::F32;
use aa_mapworld::restore::{OrganizationLeaderContext, RestoredComponent, RestoredRecord};

struct Borrowed<'a, 'world, 'source> {
    catalog: &'a Catalog<'source>,
    context: OrganizationLeaderContext<'world>,
}

/// Follow a notification from the actual selected member. Required live Mind,
/// relocation and scheduling services fail at their native reached boundary;
/// preceding goal stores remain in this exact Organization allocation.
pub fn follow_leader(
    catalog: &Catalog<'_>,
    context: OrganizationLeaderContext<'_>,
    member: ActorId,
) -> Result<(), String> {
    if context.organization.header.owner != context.owner {
        return Err("57C800 active Organization owner identity mismatch".into());
    }
    let owner = context.owner;
    let mut world = Borrowed { catalog, context };
    // Saved add-member callers supply a real member. Preserve native guards
    // even for a null notification, then refuse the selected-null caller
    // domain explicitly. Native can return or dereference it depending on Mind
    // and owner flags; neither outcome is fabricated by this bounded adapter.
    if member.is_null() {
        if position::World::world_mode(&world).map_err(|e| e.to_string())? == 6
            || position::World::selected_member(&world, owner).map_err(|e| e.to_string())?.is_some()
        {
            return Ok(());
        }
        return Err("57C800 null selected notification is outside the bound saved caller domain".into());
    }
    position::organization_moved(owner, member, &mut world).map_err(|e| e.to_string())
}

impl Borrowed<'_, '_, '_> {
    fn require_organization(&self, id: ActorId) -> position::Result<()> {
        if id != self.context.owner {
            return Err(position::Error::World(format!(
                "57C800 active Organization {:#010x} differs from requested {:#010x}",
                self.context.owner.raw(), id.raw()
            )));
        }
        Ok(())
    }
    fn actor_shell(&self, id: ActorId) -> position::Result<&aa_actors::loading::ActorShell<RestoredComponent>> {
        if id == self.context.owner {
            return Ok(self.context.owner_actor);
        }
        self.context.registry.get(id)
            .and_then(|entry| match &entry.value { RestoredRecord::Actor(entity) => Some(entity), _ => None })
            .and_then(|entity| entity.shell.actor.as_ref())
            .ok_or_else(|| position::Error::World(format!(
                "57C800 actor {:#010x} is not a current restored Actor", id.raw()
            )))
    }
    fn fail<T>(what: &str) -> position::Result<T> {
        Err(position::Error::World(format!("{what} during saved Organization follow is unbound")))
    }
}

impl position::World for Borrowed<'_, '_, '_> {
    type Definition = String;
    fn world_mode(&self) -> position::Result<u32> {
        self.catalog.native_load_mode.ok_or_else(|| position::Error::World(
            "57C800 actual GWorld+D8 load mode is unavailable".into()
        ))
    }
    fn actor(&self, id: ActorId) -> position::Result<position::Actor> {
        let shell = self.actor_shell(id)?;
        let (flags, point) = if id == self.context.owner {
            (self.context.owner_thing.flags, self.context.owner_thing.position_bits)
        } else {
            let entity = match &self.context.registry.get(id)
                .ok_or_else(|| position::Error::World("57C800 position Actor disappeared".into()))?.value
            {
                RestoredRecord::Actor(entity) => entity,
                _ => return Self::fail("57C800 actual position Actor"),
            };
            (entity.shell.thing.flags, entity.shell.thing.position_bits)
        };
        Ok(position::Actor {
            flags,
            position: [F32(point[0]), F32(point[1])],
            // Opaque token for THIS actor's real Mind slot, never a native
            // pointer cast or an invented predicate. Payload validation is
            // intentionally delayed until native actually dispatches Mind.
            mind: shell.components.present(ComponentSlot::Mind).then_some(id.raw()),
        })
    }
    fn organization(&self, id: ActorId) -> position::Result<&aa_actors::ground_states::OrganizationState<String>> {
        self.require_organization(id)?;
        Ok(self.context.organization)
    }
    fn organization_mut(&mut self, id: ActorId) -> position::Result<&mut aa_actors::ground_states::OrganizationState<String>> {
        self.require_organization(id)?;
        Ok(self.context.organization)
    }
    fn selected_member(&self, id: ActorId) -> position::Result<Option<ActorId>> {
        self.require_organization(id)?;
        let word = self.context.organization.words.get(&0x40).copied().ok_or_else(|| {
            position::Error::World("57C800 Organization+40 is uninitialized".into())
        })?;
        Ok((word != 0).then(|| ActorId::from_raw(word)))
    }
    fn mind_predicate(&mut self, token: u32, slot: u32) -> position::Result<bool> {
        let owner = ActorId::from_raw(token);
        let Some(RestoredComponent::Mind(component)) = self.actor_shell(owner)?.components.get(ComponentSlot::Mind) else {
            return Self::fail("578E90 required actual Mind slot");
        };
        let parts = component.state.as_any().downcast_ref::<RestoredMindParts>()
            .ok_or_else(|| position::Error::World("578E90 Mind payload is not its restored allocation".into()))?;
        let core = parts.mind.as_ref().ok_or_else(|| position::Error::World(
            "578E90 owned Mind core is temporarily unavailable".into()
        ))?;
        if core.actor != owner.raw() {
            return Self::fail("578E90 Mind owner identity");
        }
        if parts.installed.mind_id.0 == 0 {
            return Self::fail("578E90 installed Mind identity");
        }
        Self::fail(&format!("578E90 actual Mind virtual+{slot:X} for actor {:#010x}", owner.raw()))
    }
    fn relocate_owner(&mut self, _: ActorId, _: [F32; 2]) -> position::Result<()> {
        Self::fail("4EEB40 actual ground relocation")
    }
    fn interpolate(&mut self, _: ActorId) -> position::Result<bool> {
        Self::fail("57C080 actual Organization interpolation")
    }
    fn has_scheduled(&mut self, _: ActorId, _: u32, _: u32) -> position::Result<bool> {
        Self::fail("50B220 actual Organization event lookup")
    }
    fn schedule(&mut self, _: ActorId, _: u32) -> position::Result<()> {
        Self::fail("50AC00 actual Organization scheduling")
    }
}

#[cfg(test)]
#[path = "saved_organization_position_tests.rs"]
mod tests;
