//! Bind saved Regiment528C20 to the actual current Organization layout and
//! immutable source registries. This helper does not retain a cloned mutable
//! Organization, construct a replacement, or publish a world on failure.
use crate::catalog::Catalog;
use aa_actors::{ActorId, ground_states::OrganizationState};
use aa_mapworld::restore::RestoreDefinitions;
use aa_mind::{
    Arena,
    construct::Installed,
    regiment_load::{self, AppliedRegiment, Regiment},
    saved_load::CommandBindings,
};
use aa_numeric::Precision;

/// Facts borrowed from the actor currently undergoing saved Mind Load.
/// `world_time_bits` is GWorld+E0, the restore clock's begin word.
pub struct LoadContext<'a> {
    pub owner: ActorId,
    pub world_time_bits: u32,
    pub organization: Option<&'a OrganizationState<String>>,
}

/// Consume and apply v2+ saved data against real owners. The caller retains the
/// returned `owned` value with that Mind and moves it once into canonical actor
/// storage. It must not replace it with an empty container at staging.
pub fn apply(
    catalog: &Catalog<'_>,
    saved: Regiment,
    installed: &mut Installed,
    arena: &mut Arena,
    commands: &CommandBindings,
    context: LoadContext<'_>,
    pc: Precision,
) -> Result<AppliedRegiment, String> {
    if arena
        .mind(installed.mind_id)
        .map_err(|e| e.to_string())?
        .actor
        != context.owner.raw()
    {
        return Err("saved Regiment context belongs to a different actor".into());
    }
    if let Some(org) = context.organization {
        if org.header.owner != context.owner {
            return Err("saved Regiment Organization belongs to a different actor".into());
        }
    }
    // 4C2D90 only dereferences LayoutData when the serialized index >=0.
    let formations = if saved
        .desired_formation_index
        .is_some_and(|index| index >= 0)
    {
        let org = context
            .organization
            .ok_or("saved Regiment has no current Organization formation owner")?;
        Some(
            catalog
                .organization_layout_info(&org.layout.definition, pc)?
                .formations,
        )
    } else {
        None
    };
    let construct_definition = match &saved.construct_definition {
        Some(Some(units)) => {
            let end = units.iter().position(|c| *c == 0).unwrap_or(units.len());
            let ids = String::from_utf16(&units[..end])
                .map_err(|_| "saved Regiment ActorData IDS contains invalid UTF16")?;
            if ids.is_empty() {
                return Err("Unknown ActorData IDS in saved Regiment".into());
            }
            let definition = catalog
                .actors
                .find(&ids)
                .ok_or_else(|| format!("Unknown ActorData IDS {ids:?} in saved Regiment"))?;
            // Preserve the source lookup IDS so category validation does not
            // apply a second alias translation to the canonical name.
            if catalog
                .actors
                .native_category(&ids)
                .map_err(|e| e.to_string())?
                .native_kind
                != 2
            {
                return Err(format!(
                    "4BDBA0 non-ActorData reference {ids:?} in saved Regiment is unsupported"
                ));
            }
            Some(
                catalog
                    .definition_bank
                    .try_borrow_mut()
                    .map_err(|_| "nested saved Regiment ActorData registration")?
                    .intern("ActorDefinition", &definition.id)?,
            )
        }
        Some(None) => None,
        None => return Err("saved Regiment construct reference payload is missing".into()),
    };
    regiment_load::apply_with_context(
        saved,
        installed,
        arena,
        commands,
        regiment_load::Context {
            world_time_bits: context.world_time_bits,
            formations: formations.as_deref(),
            construct_definition,
        },
    )
    .map_err(|e| e.to_string())
}
