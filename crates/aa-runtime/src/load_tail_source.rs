//! Narrow immutable ActorData+1B4 binding for the existing4F1C20 camera walk.
//! `464DB3..464DE0` requires GetInteger(starting_location_priority), stores it
//! and rejects a negative value. Missing/incorrectly typed fields are errors,
//! even though4625A0 initialized the allocation's earlier word to zero.
use crate::catalog::Catalog;
use aa_mapworld::restore::RuntimeDefinition;
use aa_numeric::Precision;
use aa_rules::native::TypedValue;

/// Native World constructor-owned fields later mutated by4F1E10/4FD46x.
/// Retain/move this exact owner with World; do not recreate it at PostLoad.
/// No Clone/Default: construction is a lifecycle operation.
pub struct TailState {
    pub started_274: bool,
    pub camera: crate::load_tail_world::CameraState,
    pub mission_camera_messages: Vec<crate::mission_camera_world::Log>,
    pub mission_sun_warnings:Vec<String>,
    pub mission_city_clamps:Vec<(i32,i32,i32)>,
}
impl TailState {
    /// Direct stores4F7FA7/AD and4F803C after4F7D35's EBX=0.
    /// This constructs only these fields, not all of native GWorld.
    pub fn from_world_constructor() -> Self {
        Self {
            started_274: false,
            camera: crate::load_tail_world::CameraState {
                look_at: Some([0, 0]),
                messages: vec![],
            },
            mission_camera_messages:vec![],
            mission_sun_warnings:vec![],
            mission_city_clamps:vec![],
        }
    }
}

#[cfg(test)]
#[path = "load_tail_source_tests.rs"]
mod tests;

fn checked_priority(input: Option<&TypedValue>, id: &str) -> Result<i32, String> {
    let value = input
        .ok_or("native GetInteger(starting_location_priority) has no field")?
        .integer()
        .map_err(|e| e.to_string())?;
    if value < 0 {
        return Err(format!(
            "starting_location_priority for actor '{id}' needs to be positive"
        ));
    }
    Ok(value)
}

/// Canonical source-load input, not a second mutable ActorData allocation.
/// The source PC comes from the same completed Bank owner, never restore PC.
#[derive(Debug)]
pub struct StartingLocationPriority {
    token: u32,
    source_precision: Precision,
    input: TypedValue,
    value: i32,
}
impl StartingLocationPriority {
    pub fn token(&self) -> u32 {
        self.token
    }
    pub fn source_precision(&self) -> Precision {
        self.source_precision
    }
    pub fn input(&self) -> &TypedValue {
        &self.input
    }
    pub fn value(&self) -> i32 {
        self.value
    }
}

/// Controlled completed source path: one canonical identity, template chain,
/// category and completed source Bank. No new token is allocated here.
pub fn starting_location_priority(
    catalog: &Catalog<'_>,
    definition: &RuntimeDefinition,
) -> Result<StartingLocationPriority, String> {
    let bank = catalog
        .definition_bank
        .try_borrow()
        .map_err(|e| e.to_string())?;
    starting_location_priority_in_bank(catalog, definition, &bank)
}

/// Same binding after the actual Bank has moved from Catalog into Staged.
/// Pass that owned Bank, never a rebuilt token arena or copied source graph.
pub fn starting_location_priority_in_bank(
    catalog: &Catalog<'_>,
    definition: &RuntimeDefinition,
    bank: &crate::definitions::Bank,
) -> Result<StartingLocationPriority, String> {
    let id = &definition.proof.id;
    catalog.require_untranslated_actor_identity(id)?;
    let canonical = catalog
        .actors
        .find(id)
        .ok_or("camera definition is absent")?;
    let owner = bank.combat_source()?;
    let token = bank
        .token("ActorDefinition", id)
        .ok_or("camera definition has no existing canonical token")?;
    // This requires the complete verified reference owner, not its diagnostic
    // phase/constructor marker; it does not certify unrelated source passes.
    owner.completed_values(token)?;
    let pc = owner.source_precision();
    if owner.source_identities() != catalog.actors.sources.as_slice()
        || owner.definition_source(token)? != &canonical.source
        || !owner.definition_id(token)?.eq_ignore_ascii_case(id)
        || canonical.source != definition.proof.source
        || canonical.template_chain != definition.proof.template_chain
        || definition.native_class != definition.proof.category.runtime_class
        || catalog
            .actors
            .native_category(id)
            .map_err(|e| e.to_string())?
            != definition.proof.category
    {
        return Err("camera starting priority canonical source ownership differs".into());
    }
    let node = catalog
        .actors
        .resolved_actor(id)
        .map_err(|e| e.to_string())?;
    let input = node
        .field("starting_location_priority", pc)
        .map_err(|e| e.to_string())?
        .ok_or("native GetInteger(starting_location_priority) has no field")?;
    let value = checked_priority(Some(&input), id)?;
    Ok(StartingLocationPriority {
        token,
        source_precision: pc,
        input,
        value,
    })
}
