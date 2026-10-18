//!562290/Body+14 over the SAME ActorStore component and moved source Bank.
//!No LoadShell, component clone, pose snapshot or temporary Bank installation.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent},
    catalog::Catalog,
    definitions::Bank,
    restoration::Staged,
};
use aa_actors::{
    actor_creation::{Error, Result},
    components::ComponentSlot as Slot,
    ground_states::BodyState,
    loading::LoadKind,
    ActorId,
};
use aa_numeric::Precision;

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn body(actors: &ActorStore, owner: ActorId) -> Result<&BodyState> {
    match actors.component(owner, Slot::Body).map_err(error)? {
        Some(RuntimeComponent::Body(v)) if v.header.owner == owner => Ok(v),
        _ => Err(error("stored562290 actual Body allocation/owner differs")),
    }
}
fn checked(
    catalog: &Catalog<'_>,
    actors: &ActorStore,
    bank: &Bank,
    key: ObjectKey,
) -> Result<ActorId> {
    let owner = actors
        .id(key)
        .ok_or_else(|| error("stored562290 current GTID absent"))?;
    if actors.key(owner) != Some(key) {
        return Err(error("stored562290 actual registry generation differs"));
    }
    let object = actors.object(key).map_err(error)?;
    if object.kind != LoadKind::Actor
        || object.actor.is_none()
        || object.object.is_none()
        || object.definition.native_class != 3
        || !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof)
    {
        return Err(error(
            "stored562290 actual ordinary Actor constructor/proof differs",
        ));
    }
    let source = crate::live_object_init::Source::bind_in_bank(catalog, bank, &object.definition)?;
    let mask = catalog
        .canonical_component_mask(&object.definition.proof.id, source.source_precision())
        .map_err(error)?;
    if object.definition.component_mask != Some(mask) || mask & 1 == 0 {
        return Err(error("stored562290 actual source Body factory differs"));
    }
    //Fresh entities retain the exact cached source allocation. Restored
    //entities have no such Arc and use the actual canonical proof above.
    if let Some(binding) = &actors.entity(key).map_err(error)?.bindings {
        let expected = catalog
            .actor_bindings(&object.definition.proof.id, source.source_precision())
            .map_err(error)?;
        if !std::sync::Arc::ptr_eq(binding, &expected) {
            return Err(error(
                "stored562290 actual source Catalog allocation differs",
            ));
        }
    }
    body(actors, owner)?;
    Ok(owner)
}

struct Stored<'a> {
    actors: &'a mut ActorStore,
    key: ObjectKey,
    owner: ActorId,
}
impl crate::body_init_fields::Fields for Stored<'_> {
    fn state_low(&self) -> Result<u32> {
        Ok(self
            .actors
            .object(self.key)
            .map_err(error)?
            .components()
            .map_err(error)?
            .state
            .low)
    }
    fn health(&self) -> Result<u32> {
        Ok(body(self.actors, self.owner)?.body.health.0)
    }
    fn maximum(&self) -> Result<u32> {
        Ok(body(self.actors, self.owner)?.body.maximum.0)
    }
    fn construction_target(&self) -> Result<Option<u32>> {
        let Some(component) = self
            .actors
            .component(self.owner, Slot::Construction)
            .map_err(error)?
        else {
            return Ok(None);
        };
        let RuntimeComponent::Construction(value) = component else {
            return Err(error("stored562290 actual Construction payload differs"));
        };
        if value.header.owner != self.owner {
            return Err(error("stored562290 actual Construction owner differs"));
        }
        if self.state_low()? & (1 << 29) == 0 {
            return Ok(None);
        }
        if value.definition_28 == Some(0) {
            return Err(error(
                "stored562290 nonnull Construction definition token is null",
            ));
        }
        Ok(value.definition_28)
    }
    fn update_maximum(&mut self, target: u32) -> Result<()> {
        //4CF110 obtains the actual current Properties+1C context before the
        //561D40 child. Do not fabricate an empty Modifier or linked actor.
        if let Some(value) = self
            .actors
            .component(self.owner, Slot::Properties)
            .map_err(error)?
        {
            let RuntimeComponent::Properties(value) = value else {
                return Err(error("stored562290 actual Properties payload differs"));
            };
            if value.owner.0 != self.owner.raw() {
                return Err(error("stored562290 actual Properties owner differs"));
            }
        }
        Err(error(format!("561D40 stored Body Init requires current modifier/linked-body maximum service for Construction target{target}")))
    }
    fn store_health(&mut self, bits: u32) -> Result<()> {
        let Some(RuntimeComponent::Body(value)) = self
            .actors
            .component_mut(self.owner, Slot::Body)
            .map_err(error)?
        else {
            return Err(error("stored562290 current Body allocation disappeared"));
        };
        if value.header.owner != self.owner {
            return Err(error("stored562290 current Body owner differs"));
        }
        value.body.health = aa_body::F32(bits);
        Ok(())
    }
    fn set_health(&mut self, requested: u32) -> Result<()> {
        Err(error(format!("561E50 stored Body Init health service is required(requested_bits={requested},token=0,notify=1,source=0); preceding health=max store retained")))
    }
}

pub fn init_existing(
    catalog: &Catalog<'_>,
    staged: &mut Staged,
    key: ObjectKey,
    live_pc: Precision,
) -> Result<bool> {
    init_in_store(
        catalog,
        &mut staged.actors,
        &staged.definitions,
        key,
        live_pc,
    )
}
pub(crate) fn init_in_store(
    catalog: &Catalog<'_>,
    actors: &mut ActorStore,
    bank: &Bank,
    key: ObjectKey,
    _live_pc: Precision,
) -> Result<bool> {
    let owner = checked(catalog, actors, bank, key)?;
    crate::body_init_fields::initialize(&mut Stored { actors, key, owner })
}
///Actual Body vtable+14=4B2830 RET, with portable source/storage preflight.
pub fn after_init(
    catalog: &Catalog<'_>,
    actors: &ActorStore,
    bank: &Bank,
    key: ObjectKey,
) -> Result<()> {
    checked(catalog, actors, bank, key)?;
    Ok(())
}

#[cfg(test)]
#[path = "stored_body_init_world_tests.rs"]
mod tests;
