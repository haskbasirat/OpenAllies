//! Component InitExisting/+14 on the actual held saved Actor shell.
//!
//! The first reached Kingdom Properties allocation has genuine constant native
//! leaves (6A71D0 true and4B2830 return). Nonconstant component services stay
//! explicit required boundaries until their actual typed owners are bound.
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::{
    actor_creation::{Error, Result},
    component_virtuals,
    components::ComponentSlot,
    init_create::FactoryKind,
};
use aa_body::credit::Object;
use aa_mapworld::restore::{RestoredComponent, RestoredRecord};
use aa_numeric::Precision;

fn required(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}

fn properties(catalog: &Catalog<'_>, record: &Record<'_>, slot: ComponentSlot) -> Result<()> {
    catalog.require_source_owner().map_err(required)?;
    if !record
        .registry
        .get(record.owner)
        .is_some_and(|v| matches!(v.value, RestoredRecord::Loading))
    {
        return Err(required(
            "component Init requires the actual registered held Load shell",
        ));
    }
    if !matches!(record.definition.native_class, 3 | 4)
        || record.definition.proof.category.runtime_class != record.definition.native_class
        || !crate::load_spatial::ordinary_layout(
            record.definition.native_class,
            record.shell.kind,
            record.shell.object.is_some(),
            record.shell.actor.is_some(),
        )
        || !crate::load_spatial::same_proof(
            &record.shell.thing.definition,
            &record.definition.proof,
        )
    {
        return Err(required(
            "component Init actual Actor constructor storage/source differs",
        ));
    }
    let source_pc = {
        let bank = catalog.definition_bank.try_borrow().map_err(required)?;
        let source = bank.combat_source().map_err(required)?;
        if source.source_identities() != catalog.actors.sources.as_slice() {
            return Err(required(
                "component Init source FP owner differs from Catalog",
            ));
        }
        source.source_precision()
    };
    let id = &record.definition.proof.id;
    catalog
        .require_untranslated_actor_identity(id)
        .map_err(required)?;
    let source = catalog
        .actors
        .find(id)
        .ok_or_else(|| required("component Init canonical ActorData absent"))?;
    let mask = catalog
        .canonical_component_mask(id, source_pc)
        .map_err(required)?;
    if source.source != record.definition.proof.source
        || source.template_chain != record.definition.proof.template_chain
        || catalog.actors.native_category(id).map_err(required)? != record.definition.proof.category
        || catalog
            .actors
            .native_final_flags(id, source_pc)
            .map_err(required)?
            != record.definition.flags
        || record.definition.component_mask != Some(mask)
    {
        return Err(required(
            "component Init canonical definition/proof/mask differs",
        ));
    }
    let actor = record
        .shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("component Init Actor shell"))?;
    let component = actor
        .components
        .get(slot)
        .ok_or(Error::NullDereference("component Init absent slot"))?;
    if component.slot() != slot {
        return Err(required("component Init slot payload differs"));
    }
    if slot != ComponentSlot::Properties {
        if slot == ComponentSlot::Mind {
            return Err(required(
                "Mind InitExisting requires its actual constructed51D1B0/517C60/529EC0 class and owned Mind world; not yet bound",
            ));
        }
        let leaf = component_virtuals::leaves(FactoryKind::Standard(slot));
        return Err(required(format!(
            "component {slot:?} InitExisting {:06X}/AfterInit {:06X} actual state/services are not yet bound",
            leaf.init_existing, leaf.vf14
        )));
    }
    let RestoredComponent::Properties(state) = component else {
        return Err(required("Properties Init typed allocation differs"));
    };
    if mask & (1 << slot.index()) == 0 || state.owner != Object(record.owner.raw()) {
        return Err(required("Properties Init source factory/owner differs"));
    }
    Ok(())
}

/// Native Properties virtual+8=6A71D0: mov al,1; ret. The supplied live PC
/// causes no arithmetic on this proved constant leaf.
pub fn init_existing(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    slot: ComponentSlot,
    _pc: Precision,
) -> Result<bool> {
    properties(catalog, record, slot)?;
    let leaf = component_virtuals::leaves(FactoryKind::Standard(slot));
    if leaf.init_existing != 0x6a71d0 {
        return Err(required(
            "Properties InitExisting native leaf mapping differs",
        ));
    }
    Ok(true)
}

/// Native Properties virtual+14=4B2830: ret. This is the after-all-components
/// Init hook, distinct from its nonempty PostCreate581880 initial-property path.
pub fn after_init(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    slot: ComponentSlot,
) -> Result<()> {
    properties(catalog, record, slot)?;
    let leaf = component_virtuals::leaves(FactoryKind::Standard(slot));
    if leaf.vf14 != 0x4b2830 {
        return Err(required("Properties AfterInit native leaf mapping differs"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "load_component_init_world_tests.rs"]
mod tests;
