//! Saved owner `4CF0F0` dispatch through the existing actual Economy refresh.
//! The absent component is the native no-op; present payloads retain the exact
//! `570020` coordinator, borrowed allocation and required real Kingdom flows.
use crate::{catalog::Catalog, saved_organization_actions::SavedActions};
use aa_mapworld::restore::OrganizationAttachContext;

pub fn refresh(
    catalog: &Catalog<'_>,
    context: OrganizationAttachContext<'_>,
) -> Result<(), String> {
    let owner = context.owner;
    crate::saved_economy_refresh::refresh_member(
        &mut SavedActions {
            catalog,
            context: context.into(),
            active_mind: None,
        },
        owner,
    )
    .map_err(|error| error.to_string())
}
