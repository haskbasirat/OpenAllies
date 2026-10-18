//! Genuine constant component InitExisting leaves on held saved allocations.
//!
//! These classes use native6A71D0 (true). Only their actual typed restored
//! payloads are admitted. City AfterInit563FE0 remains a required zone service.
use crate::{
    catalog::Catalog,
    live_object_init::{Record, Source},
};
use aa_actors::{
    actor_creation::{Error, Result},
    component_virtuals,
    components::ComponentSlot,
    init_create::FactoryKind,
};
use aa_mapworld::restore::{RestoredComponent, RestoredRecord};
use aa_numeric::Precision;

fn required(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}

/// Slots with both an actual typed restored owner and proved constant +8.
/// Projectile1 and Portal28 also have a native constant +8, but have no typed
/// saved allocation in this owner; they are deliberately not admitted here.
pub fn supports(slot: ComponentSlot) -> bool {
    matches!(
        slot,
        ComponentSlot::Character
            | ComponentSlot::Structure
            | ComponentSlot::Research
            | ComponentSlot::Transport
            | ComponentSlot::City
            | ComponentSlot::Hq
    )
}

fn checked(catalog: &Catalog<'_>, record: &Record<'_>, slot: ComponentSlot) -> Result<()> {
    if !supports(slot) {
        return Err(required(format!(
            "constant Init slot {slot:?} actual typed allocation is not bound"
        )));
    }
    if !record
        .registry
        .get(record.owner)
        .is_some_and(|v| matches!(v.value, RestoredRecord::Loading))
    {
        return Err(required(
            "constant Init requires the actual registered held Load shell",
        ));
    }
    if !matches!(record.definition.native_class, 3 | 4)
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
            "constant Init actual Actor constructor/proof differs",
        ));
    }
    let source = Source::bind(catalog, record.definition)?;
    let mask = catalog
        .canonical_component_mask(&record.definition.proof.id, source.source_precision())
        .map_err(required)?;
    if record.definition.component_mask != Some(mask) || mask & (1 << slot.index()) == 0 {
        return Err(required("constant Init actual source factory mask differs"));
    }
    let component = record
        .shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("constant Init Actor"))?
        .components
        .get(slot)
        .ok_or(Error::NullDereference("constant Init actual slot"))?;
    let owner = match (slot, component) {
        (ComponentSlot::Character, RestoredComponent::Character(v)) => v.header.owner,
        (ComponentSlot::Structure, RestoredComponent::Structure(v)) => v.header.owner,
        (ComponentSlot::Research, RestoredComponent::Research(v)) => v.header.owner,
        (ComponentSlot::Transport, RestoredComponent::Transport(v)) => v.owner,
        (ComponentSlot::City, RestoredComponent::City(v)) => v.header.owner,
        (ComponentSlot::Hq, RestoredComponent::Hq(v)) => v.header.owner,
        _ => return Err(required("constant Init slot actual typed payload differs")),
    };
    if owner != record.owner {
        return Err(required("constant Init actual allocation owner differs"));
    }
    let leaf = component_virtuals::leaves(FactoryKind::Standard(slot));
    if leaf.init_existing != 0x6a71d0 {
        return Err(required("constant Init native vtable mapping differs"));
    }
    Ok(())
}

/// Native6A71D0: mov al,1; ret; no allocation or world field is read/written.
pub fn init_existing(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    slot: ComponentSlot,
    _pc: Precision,
) -> Result<bool> {
    checked(catalog, record, slot)?;
    Ok(true)
}

/// Exact native4B2830 return for the admitted classes except City's563FE0.
pub fn after_init(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    slot: ComponentSlot,
) -> Result<()> {
    checked(catalog, record, slot)?;
    let leaf = component_virtuals::leaves(FactoryKind::Standard(slot));
    if leaf.vf14 != 0x4b2830 {
        return Err(required(format!(
            "component {slot:?} AfterInit {:06X} requires actual source5B4/owned zone8 services",
            leaf.vf14
        )));
    }
    Ok(())
}

#[cfg(test)]
#[path = "load_constant_component_init_tests.rs"]
mod tests;
