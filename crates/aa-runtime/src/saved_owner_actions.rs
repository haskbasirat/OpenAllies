//! Owner4D1350 over the actual held Organization and saved Actor components.
//! The existing native-backed coordinator and SavedActions perform the full
//! reached dispatch; missing contributors/publication remain required errors
//! after all preceding authoritative mask stores.
use crate::{catalog::Catalog, saved_organization_actions::SavedActions};
use aa_mapworld::restore::OrganizationAttachContext;

pub fn rebuild(
    catalog: &Catalog<'_>,
    context: OrganizationAttachContext<'_>,
) -> Result<(), String> {
    let owner = context.owner;
    aa_actors::ability_flags::rebuild(
        &mut SavedActions {
            catalog,
            context: context.into(),
            active_mind: None,
        },
        owner,
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "saved_owner_actions_tests.rs"]
mod tests;
