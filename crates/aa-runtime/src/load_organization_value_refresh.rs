//! Native5788B0 on the actual held Actor and Organization component.
//! Same owned member list/Body/Performer values, completed source cache and
//! existing proved arithmetic kernel; no detached actor/guessed combat values.
use crate::{catalog::Catalog,live_object_init::Record};
use aa_actors::{actor_creation::{Error,Result},components::ComponentSlot};
use aa_mapworld::restore::{OrganizationCombatContext,RestoredComponent};
use aa_numeric::Precision;

/// The exact same component Box is returned on Result error, retaining native
/// current/max reset and accumulation prefixes. No publication is performed.
pub fn refresh(catalog:&Catalog<'_>,record:&mut Record<'_>,live_pc:Precision)->Result<()> {
    let source_pc=crate::load_actor_init_world::validate(catalog,record)?;
    if record.definition.component_mask.is_none_or(|m|m&(1<<7)==0) {
        return Err(Error::Unsupported("5788B0 Organization outside actual source factory"));
    }
    match record.shell.actor.as_ref().and_then(|a|a.components.get(ComponentSlot::Organization)) {
        Some(RestoredComponent::Organization(org)) if org.header.owner==record.owner=>(),
        _=>return Err(Error::Unsupported("5788B0 actual held Organization payload/owner")),
    }
    let component=record.shell.actor.as_mut().unwrap().components.remove(ComponentSlot::Organization).unwrap();
    let RestoredComponent::Organization(mut organization)=component else{unreachable!()};
    let outcome=crate::saved_organization_combat::recompute_at_source(catalog,OrganizationCombatContext {
        owner:record.owner,owner_actor:record.shell.actor.as_ref().unwrap(),organization:&mut organization,
        registry:&*record.registry,precision:live_pc,
    },source_pc).map_err(|e|Error::World(e.to_string()));
    assert!(record.shell.actor.as_mut().unwrap().components.insert(ComponentSlot::Organization,RestoredComponent::Organization(organization)).is_none());
    outcome
}
