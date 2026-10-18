//! Pure component leaves after the caller binds its actual source/Actor.
//! Specialized Mind/Performer owners remain with their arena adapters.
use crate::actor_world::RuntimeComponent;
use aa_actors::{ActorId, component_virtuals::{self, Leaves}, components::ComponentSlot,
    init_create::factory_kind};

pub(crate) fn checked(component: &RuntimeComponent, owner: ActorId,
    slot: ComponentSlot, source_mask: u32) -> Result<Leaves, String> {
    if component.slot() != slot || source_mask & (1 << slot.index()) == 0 {
        return Err("fresh constant leaf actual payload/source factory slot differs".into());
    }
    let actual = match component {
        RuntimeComponent::Body(v) => v.header.owner,
        RuntimeComponent::Character(v) => v.header.owner,
        RuntimeComponent::Mover(v) => v.header.owner,
        RuntimeComponent::Properties(v) => ActorId::from_raw(v.owner.0),
        RuntimeComponent::Organization(v) => v.header.owner,
        RuntimeComponent::Element(v) => v.header.owner,
        RuntimeComponent::Economy(v) => v.header.owner,
        RuntimeComponent::Construction(v) => v.header.owner,
        RuntimeComponent::Denizen(v) => v.header.owner,
        RuntimeComponent::Structure(v) => v.header.owner,
        RuntimeComponent::Supply(v) => v.header.owner,
        RuntimeComponent::Recruit(v) => v.header.owner,
        RuntimeComponent::Defender(v) => v.header.owner,
        RuntimeComponent::Research(v) => v.header.owner,
        RuntimeComponent::Lair(v) => v.header.owner,
        RuntimeComponent::City(v) => v.header.owner,
        RuntimeComponent::MobileDeployment(v) => v.header.owner,
        RuntimeComponent::Experience(v) => ActorId::from_raw(v.owner.0),
        RuntimeComponent::Detection(v) => v.header.owner,
        RuntimeComponent::Transport(v) => v.owner,
        RuntimeComponent::Builder(v) => v.header.owner,
        RuntimeComponent::Hq(v) => v.header.owner,
        RuntimeComponent::Mind(_) | RuntimeComponent::Performer(_) =>
            return Err("fresh specialized constant leaf requires its actual arena adapter".into()),
    };
    if actual != owner {
        return Err("fresh constant leaf actual component owner differs".into());
    }
    let kind = factory_kind(slot, source_mask)
        .ok_or("fresh constant leaf actual factory class absent")?;
    Ok(component_virtuals::leaves(kind))
}

#[cfg(test)]
#[path = "map_component_constant_leaf_tests.rs"]
mod tests;
