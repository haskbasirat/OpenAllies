//! Native4D1350 over an actual held saved Actor, including ordinary Elements.
//! No detached Organization is fabricated to reuse the existing contributors.
use crate::{
    catalog::Catalog,
    live_object_init::Record,
    saved_organization_actions::{SavedActionContext, SavedActions},
};
use aa_mapworld::WorldInventory;
use aa_numeric::Precision;

/// Rebuild the current actor's real F0/F4 words. The complete owned Mind must
/// be in its actual component slot; the existing Mind contributor temporarily
/// loans and restores that same allocation. Earlier stores survive errors.
pub fn rebuild(
    catalog: &Catalog<'_>,
    inventory: &WorldInventory,
    record: &mut Record<'_>,
    precision: Precision,
) -> Result<(), String> {
    let source_precision =
        crate::load_actor_init_world::validate(catalog, record).map_err(|e| e.to_string())?;
    let object = record
        .shell
        .object
        .as_ref()
        .ok_or("4D1350 actual held Object storage is absent")?;
    let actor = record
        .shell
        .actor
        .as_mut()
        .ok_or("4D1350 actual held Actor storage is absent")?;
    // Existing native getter4D1E80 reads this same constructor/Load word.
    if !actor.words.contains_key(&0xe8) {
        return Err("4D1350 actual initialized Actor+E8 is absent".into());
    }
    let owner = record.owner;
    let mut actions = SavedActions {
        catalog,
        context: SavedActionContext {
            inventory,
            owner,
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
    };
    aa_actors::ability_flags::rebuild(&mut actions, owner).map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "saved_actor_actions_tests.rs"]
mod tests;
