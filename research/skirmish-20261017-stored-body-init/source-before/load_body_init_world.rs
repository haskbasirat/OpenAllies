//! Concrete saved Body InitExisting562290 prefix on the actual held shell.
//! Healthy and construction-state-skip paths complete. Reached nonconstant
//! health services explicitly stop with every preceding native store retained.
use crate::{
    catalog::Catalog,
    live_object_init::{Record, Source},
};
use aa_actors::{
    actor_creation::{Error, Result},
    components::ComponentSlot,
    ground_states::BodyState,
    loading::LoadShell,
};
use aa_mapworld::restore::{RestoredComponent, RestoredRecord};
use aa_numeric::Precision;

fn required(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}
fn low<D>(shell: &LoadShell<D, RestoredComponent>) -> Result<u32> {
    Ok(shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("Body Init Actor"))?
        .components
        .state
        .low)
}
fn body<D>(shell: &LoadShell<D, RestoredComponent>) -> Result<&BodyState> {
    match shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("Body Init Actor"))?
        .components
        .get(ComponentSlot::Body)
    {
        Some(RestoredComponent::Body(v)) => Ok(v),
        Some(_) => Err(required("Body Init slot0 actual payload differs")),
        None => Err(Error::NullDereference("Body Init slot0")),
    }
}

fn checked(catalog: &Catalog<'_>, record: &Record<'_>) -> Result<()> {
    if !record
        .registry
        .get(record.owner)
        .is_some_and(|v| matches!(v.value, RestoredRecord::Loading))
    {
        return Err(required("Body Init requires registered held Load shell"));
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
        return Err(required("Body Init actual Actor constructor/proof differs"));
    }
    let source = Source::bind(catalog, record.definition)?;
    let mask = catalog
        .canonical_component_mask(&record.definition.proof.id, source.source_precision())
        .map_err(required)?;
    if record.definition.component_mask != Some(mask) || mask & 1 == 0 {
        return Err(required("Body Init actual source factory mask differs"));
    }
    if body(record.shell)?.header.owner != record.owner {
        return Err(required("Body Init actual allocation owner differs"));
    }
    Ok(())
}

fn prefix<D>(shell: &mut LoadShell<D, RestoredComponent>) -> Result<bool> {
    //562294..2A6 reads state before health/max; construction skips everything.
    if low(shell)? & (1 << 21) != 0 {
        return Ok(true);
    }
    let differs = {
        let v = body(shell)?;
        f32::from_bits(v.body.health.0) != f32::from_bits(v.body.maximum.0)
    };
    let construction = shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("Body Init Actor"))?
        .components
        .get(ComponentSlot::Construction);
    if let Some(value) = construction {
        let RestoredComponent::Construction(value) = value else {
            return Err(required("Body Init Construction+9C payload differs"));
        };
        //565070 rereads the Construction allocation's current owner state. The
        // admitted held allocation is owned by this same registered Actor.
        if value.header.owner != body(shell)?.header.owner {
            return Err(required("Body Init Construction owner differs"));
        }
        if low(shell)? & (1 << 29) != 0
            && let Some(target) = value.definition_28
        {
            if target == 0 {
                return Err(required(
                    "Body Init nonnull Construction target has null portable definition token",
                ));
            }
            //4CF110 is the pure current Properties+1C context getter. Its
            // following561D40 requires actual modifier/linked-body effects;
            // no Body field has changed before that required service entry.
            return Err(required(format!(
                "561D40 Body Init requires current modifier/linked-body maximum service for Construction target{target}"
            )));
        }
    }
    if differs {
        let actor = shell
            .actor
            .as_mut()
            .ok_or(Error::NullDereference("Body Init Actor"))?;
        let Some(RestoredComponent::Body(v)) = actor.components.get_mut(ComponentSlot::Body) else {
            return Err(required("Body Init current allocation disappeared"));
        };
        //5622E9..301 rereads actual current fields, stores maximum into health,
        // THEN enters561E50(old_health,token0,notify1,source0). Keep that store
        // on the explicit required-service error, exactly as native prefix.
        let requested = v.body.health.0;
        v.body.health = v.body.maximum;
        return Err(required(format!(
            "561E50 Body Init health service is required(requested_bits={requested},token=0,notify=1,source=0); preceding health=max store retained"
        )));
    }
    Ok(true)
}

pub fn init_existing(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    _pc: Precision,
) -> Result<bool> {
    checked(catalog, record)?;
    prefix(record.shell)
}
///Actual Body vtable+14=4B2830 (ret); after successful component initialization.
pub fn after_init(catalog: &Catalog<'_>, record: &mut Record<'_>) -> Result<()> {
    checked(catalog, record)?;
    Ok(())
}

#[cfg(test)]
#[path = "load_body_init_world_tests.rs"]
mod tests;
