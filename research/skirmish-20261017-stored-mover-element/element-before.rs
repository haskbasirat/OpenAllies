//! Actual Element5704D0/4B2830 on the same held saved Actor allocation.
//! Source PC belongs to the completed Bank; resupply multiplication uses the
//! current simulation PC and the actual Properties+1C modifier, when present.
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::{
    actor_creation::{Error, Result},
    components::ComponentSlot,
};
use aa_body::{F32, properties::ModifierState};
use aa_mapworld::restore::RestoredComponent;
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeRng;

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn checked(catalog: &Catalog<'_>, record: &Record<'_>) -> Result<Precision> {
    let pc = crate::load_actor_init_world::validate(catalog, record)?;
    if record
        .definition
        .component_mask
        .is_none_or(|m| m & (1 << ComponentSlot::Element.index()) == 0)
    {
        return Err(Error::Unsupported(
            "5704D0 Element outside canonical source factory",
        ));
    }
    match record
        .shell
        .actor
        .as_ref()
        .and_then(|a| a.components.get(ComponentSlot::Element))
    {
        Some(RestoredComponent::Element(v)) if v.header.owner == record.owner => Ok(pc),
        _ => Err(Error::Unsupported(
            "5704D0 held Element owner/class differs",
        )),
    }
}
///45D5B0→5014E0 returns an unrounded x87 product, then5704EA stores m32.
fn rate(base: u32, modifier: Option<&ModifierState>, pc: Precision) -> Result<u32> {
    let value = match modifier {
        Some(m) => m.multiply(0x3c, F32(base), pc).map_err(error)?,
        None => Binary::from_f32_bits(base).map_err(error)?,
    };
    value.store_f32().map_err(error)
}
pub fn init_existing(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    rng: &mut NativeRng,
    pc: Precision,
    world_version: impl FnOnce() -> Result<u32>,
) -> Result<bool> {
    let source_pc = checked(catalog, record)?;
    let source = aa_rules::native::actor_morale::actor_morale_source(
        catalog.actors,
        &record.definition.proof.id,
        source_pc,
    )
    .map_err(error)?;
    if source.actor_source != record.definition.proof.source {
        return Err(Error::Unsupported(
            "5704D0 Element source provenance differs",
        ));
    }
    let base = source
        .resupply_rate
        .ok_or(Error::Unsupported(
            "5704D0 actual ActorData+2CC source absent",
        ))?
        .bits;
    let actor = record.shell.actor.as_ref().unwrap();
    let modifier = match actor.components.get(ComponentSlot::Properties) {
        None => None,
        Some(RestoredComponent::Properties(p)) if p.owner.0 == record.owner.raw() => {
            Some(&p.modifier)
        }
        _ => {
            return Err(Error::Unsupported(
                "4CF110 current Properties owner/class differs",
            ));
        }
    };
    let value = rate(base, modifier, pc)?;
    let Some(RestoredComponent::Element(element)) = record
        .shell
        .actor
        .as_mut()
        .unwrap()
        .components
        .get_mut(ComponentSlot::Element)
    else {
        unreachable!()
    };
    element.word_20 = value;
    //The store precedes the version/head/queue suffix, including failures.
    crate::load_detection_init_world::schedule_old_existing(
        catalog,
        record,
        ComponentSlot::Element,
        rng,
        pc,
        world_version,
    )
}
///Actual Element virtual14=4B2830 RET, after successful InitExisting.
pub fn after_init(catalog: &Catalog<'_>, record: &mut Record<'_>) -> Result<()> {
    checked(catalog, record)?;
    Ok(())
}

#[cfg(test)]
#[path = "load_element_init_world_tests.rs"]
mod tests;
