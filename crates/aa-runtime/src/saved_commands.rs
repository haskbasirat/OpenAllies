//! Saved 4DACE0 command references resolved through the original data managers.
//! This binds data and ownership only; it does not dispatch saved commands.
use crate::{catalog::Catalog, mind_world::PlainCommand};
use aa_mind::{command::Definition, saved_command::Command};
use aa_numeric::Precision;

/// CString presence is determined by serialized length, while lookup stops at
/// its first NUL. A nonempty string beginning with NUL is an unknown IDS, not
/// an absent reference. Unsupported UTF16 is never replaced lossily.
fn name(units: &[u16]) -> Result<String, String> {
    let end = units.iter().position(|u| *u == 0).unwrap_or(units.len());
    String::from_utf16(&units[..end]).map_err(|_| "saved command IDS contains invalid UTF16".into())
}

fn intern(catalog: &Catalog<'_>, domain: &str, canonical: &str) -> Result<u32, String> {
    catalog
        .definition_bank
        .try_borrow_mut()
        .map_err(|_| "nested command definition registration")?
        .intern(domain, canonical)
}

/// Bind plain fields without interpreting flags as a network-codec presence
/// mask. Native saved Load reads these fields even when their flags are clear.
/// Unknown references remain explicit failures; no default command is invented.
pub fn bind(catalog: &Catalog<'_>, saved: &Command, pc: Precision) -> Result<PlainCommand, String> {
    if saved.flags & (1 << 5) != 0 {
        return Err("saved command ParameterSet ownership is not implemented".into());
    }
    let (definition_id, definition) = match &saved.definition {
        None => (0, None),
        Some(units) => {
            let id = name(units)?;
            if id.is_empty() {
                return Err("Unknown IDS for ActorCommandData in saved command".into());
            }
            // Native manager translates once, then searches its compiled
            // registry. That registry also contains commands with no TGI root.
            let translated = catalog
                .actors
                .native_data_registry("ActorCommand")
                .and_then(|r| {
                    r.translations()
                        .into_iter()
                        .find(|(old, _)| old.eq_ignore_ascii_case(&id))
                })
                .map(|(_, new)| new)
                .unwrap_or(id);
            let d = aa_rules::native::actor_command::predefined_actor_command(&translated)
                .map_err(|e| e.to_string())?;
            (
                intern(catalog, "ActorCommand", d.id)?,
                Some(Definition {
                    component_slot: d.component_slot,
                    mind_type: d.mind_type,
                    behavior_flags: d.behavior_flags,
                }),
            )
        }
    };
    let object_definition = match &saved.object_definition {
        None => None,
        Some(units) => {
            let id = name(units)?;
            let source = catalog
                .actors
                .find(&id)
                .ok_or_else(|| format!("Unknown ActorData IDS {id:?} in saved command"))?;
            // Pass the original lookup IDS to every source getter. Passing the
            // resolved name instead would apply a second translation.
            if catalog
                .actors
                .native_category(&id)
                .map_err(|e| e.to_string())?
                .native_kind
                != 2
            {
                return Err(format!("saved command IDS {id:?} is not ActorData"));
            }
            Some(intern(catalog, "ActorDefinition", &source.id)?)
        }
    };
    let parameter_definition = match &saved.parameter_definition {
        None => None,
        Some(units) => {
            let id = name(units)?;
            let data = catalog.layout_data(pc)?;
            let formation = data
                .find_formation(&id)
                .ok_or_else(|| format!("Unknown FormationData IDS {id:?} in saved command"))?;
            Some(intern(catalog, "Formation", &formation.ids)?)
        }
    };
    let extra_definition = match &saved.extra_definition {
        None => None,
        Some(units) => {
            let id = name(units)?;
            let data = catalog.layout_data(pc)?;
            let Some(aa_rules::native::layout_data::PropertyRef::Registered(index)) =
                data.find_property(&id, None)
            else {
                return Err(format!(
                    "Unknown SharedPropertyData IDS {id:?} in saved command"
                ));
            };
            let property = data
                .properties
                .get(index as usize)
                .ok_or("saved command Property registry identity is invalid")?;
            Some(intern(catalog, "Property", &property.ids)?)
        }
    };
    Ok(PlainCommand {
        definition_id,
        definition,
        flags: saved.flags,
        word10: saved.word10,
        object_definition,
        position_bits: saved.position_bits,
        auxiliary_bits: saved.auxiliary_bits,
        scalar_bits: saved.scalar_bits,
        parameter_definition,
        extra_definition,
        boolean: saved.boolean,
        trailing: saved.trailing,
    })
}
