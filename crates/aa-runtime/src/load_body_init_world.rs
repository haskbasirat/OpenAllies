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

struct Held<'a, D>(&'a mut LoadShell<D, RestoredComponent>);
impl<D> crate::body_init_fields::Fields for Held<'_, D> {
    fn state_low(&self) -> Result<u32> {
        low(self.0)
    }
    fn health(&self) -> Result<u32> {
        Ok(body(self.0)?.body.health.0)
    }
    fn maximum(&self) -> Result<u32> {
        Ok(body(self.0)?.body.maximum.0)
    }
    fn construction_target(&self) -> Result<Option<u32>> {
        let actor = self
            .0
            .actor
            .as_ref()
            .ok_or(Error::NullDereference("Body Init Actor"))?;
        let Some(value) = actor.components.get(ComponentSlot::Construction) else {
            return Ok(None);
        };
        let RestoredComponent::Construction(value) = value else {
            return Err(required("Body Init Construction+9C payload differs"));
        };
        if value.header.owner != body(self.0)?.header.owner {
            return Err(required("Body Init Construction owner differs"));
        }
        if low(self.0)? & (1 << 29) == 0 {
            return Ok(None);
        }
        let target = value.definition_28;
        if target == Some(0) {
            return Err(required(
                "Body Init nonnull Construction target has null portable definition token",
            ));
        }
        Ok(target)
    }
    fn update_maximum(&mut self, target: u32) -> Result<()> {
        //4CF110 reads only the current actual Properties context. No native
        //Body store precedes the required561D40 child in this held adapter.
        let actor = self
            .0
            .actor
            .as_ref()
            .ok_or(Error::NullDereference("Body Init Actor"))?;
        if let Some(value) = actor.components.get(ComponentSlot::Properties) {
            let RestoredComponent::Properties(value) = value else {
                return Err(required("Body Init Properties payload differs"));
            };
            if value.owner.0 != body(self.0)?.header.owner.raw() {
                return Err(required("Body Init Properties owner differs"));
            }
        }
        Err(required(format!("561D40 Body Init requires current modifier/linked-body maximum service for Construction target{target}")))
    }
    fn store_health(&mut self, bits: u32) -> Result<()> {
        let actor = self
            .0
            .actor
            .as_mut()
            .ok_or(Error::NullDereference("Body Init Actor"))?;
        let Some(RestoredComponent::Body(value)) = actor.components.get_mut(ComponentSlot::Body)
        else {
            return Err(required("Body Init current allocation disappeared"));
        };
        value.body.health = aa_body::F32(bits);
        Ok(())
    }
    fn set_health(&mut self, requested: u32) -> Result<()> {
        Err(required(format!("561E50 Body Init health service is required(requested_bits={requested},token=0,notify=1,source=0); preceding health=max store retained")))
    }
}
fn prefix<D>(shell: &mut LoadShell<D, RestoredComponent>) -> Result<bool> {
    crate::body_init_fields::initialize(&mut Held(shell))
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
