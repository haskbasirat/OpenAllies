//! Shared570020/5704C0 over the same held Actor and stored Kingdom flows.
//! The existing borrowed Economy/value/flow kernels own every mutation; no
//! Organization, inventory, ActorStore or flow-array replacement is fabricated.
use crate::{
    catalog::Catalog,
    live_object_init::{Record, Source},
    saved_organization_actions::{SavedActionContext, SavedActions},
};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_mapworld::{
    WorldInventory,
    restore::{RestoredComponent, RestoredRecord},
};
use aa_numeric::Precision;

#[cfg(test)]
#[path = "load_economy_refresh_world_tests.rs"]
mod tests;

/// The actual inventory is borrowed from the same live load context. Economy
/// refresh does not inspect its unrelated map fields; it is never synthesized
/// merely to satisfy the existing saved-action ownership context.
pub fn refresh(
    catalog: &Catalog<'_>,
    inventory: &WorldInventory,
    record: &mut Record<'_>,
    precision: Precision,
) -> Result<(), String> {
    let source_precision =
        crate::load_actor_init_world::validate(catalog, record).map_err(|e| e.to_string())?;
    let current = record.owner;
    let actor = record
        .shell
        .actor
        .as_ref()
        .ok_or("570020 actual held Actor is absent")?;
    let Some(RestoredComponent::Economy(economy)) = actor.components.get(ComponentSlot::Economy)
    else {
        return if actor.components.get(ComponentSlot::Economy).is_none() {
            Ok(())
        } else {
            Err("570020 actual Economy slot payload differs".into())
        };
    };
    if economy.header.owner != current {
        return Err("570020 actual Economy header owner differs".into());
    }
    let owner = ActorId::from_raw(
        *actor
            .words
            .get(&0xe8)
            .ok_or("570020 actual Actor+E8 is absent")?,
    );
    if owner.is_null() {
        return Ok(());
    }
    if owner == current {
        return Err("570020 requires held Kingdom derived flow owner".into());
    }
    // Reconstructing a second Kingdom from a diagnostic snapshot is forbidden.
    // This translates the actual nonnull owner pointer to its current GTID and
    // authenticates that stored allocation; vector fields remain lazy.
    let Some(RestoredRecord::Kingdom(kingdom)) = record.registry.get(owner).map(|e| &e.value)
    else {
        return Err("570020 actual stored Kingdom generation is absent".into());
    };
    if kingdom.source.serialized_id != owner.raw() {
        return Err("570020 stored Kingdom identity differs".into());
    }
    let runtime = kingdom
        .runtime
        .as_ref()
        .ok_or("570020 stored Kingdom runtime is absent")?;
    Source::bind(catalog, &runtime.definition).map_err(|e| e.to_string())?;
    if runtime.definition.native_class != 4
        || runtime.base.kind != aa_actors::loading::LoadKind::Actor
        || runtime.base.actor.is_none()
        || runtime.base.object.is_none()
        || !crate::load_spatial::same_proof(
            &runtime.base.thing.definition,
            &runtime.definition.proof,
        )
        || runtime.definition.component_mask
            != Some(
                catalog.canonical_component_mask(&runtime.definition.proof.id, source_precision)?,
            )
        || runtime.derived.as_ref().and_then(|d| d.recruitment_owner) != Some(owner)
    {
        return Err("570020 stored Kingdom source/constructor/owner differs".into());
    }
    let object = record
        .shell
        .object
        .as_ref()
        .ok_or("570020 actual Object storage is absent")?;
    let actor = record
        .shell
        .actor
        .as_mut()
        .ok_or("570020 actual Actor storage is absent")?;
    crate::saved_economy_refresh::refresh_member(
        &mut SavedActions {
            catalog,
            context: SavedActionContext {
                inventory,
                owner: current,
                owner_actor: actor,
                owner_definition: record.definition,
                owner_thing_flags: record.shell.thing.flags,
                owner_thing_word_c: record.shell.thing.word_c,
                owner_object_byte_4c: object.byte_4c,
                organization: None,
                registry: record.registry,
                precision,
                source_precision,
            },
            active_mind: None,
        },
        current,
    )
    .map_err(|e| e.to_string())
}
