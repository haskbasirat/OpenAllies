//! Real held Performer `57E000 -> 57DCF0(NULL)` and empty `4B2830` AfterInit.
//! Primary Ability identities and enabled bytes stay in the same Catalog Arena.
//! A changed-byte prefix is followed by the actual full-Actor `4D1350` tail.
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::{
    actor_creation::{Error, Result},
    components::ComponentSlot,
};
use aa_combat::{
    constructors::OwnedPerformer,
    performer::{self, RankWorld},
};
use aa_mapworld::{WorldInventory, restore::RestoredComponent};
use aa_numeric::Precision;
#[cfg(test)]
#[path = "load_performer_init_world_tests.rs"]
mod tests;

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn combat(e: impl std::fmt::Display) -> aa_combat::Error {
    aa_combat::Error::World(e.to_string())
}

fn performer<'a>(record: &'a Record<'_>) -> Result<&'a OwnedPerformer> {
    match record
        .shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("57E000 actual Actor storage"))?
        .components
        .get(ComponentSlot::Performer)
    {
        Some(RestoredComponent::Performer(value)) if value.state.owner == record.owner.raw() => {
            Ok(value)
        }
        _ => Err(Error::Unsupported(
            "57E000 actual held Performer owner/payload differs",
        )),
    }
}
fn validate(catalog: &Catalog<'_>, record: &Record<'_>) -> Result<Precision> {
    let source_pc = crate::load_actor_init_world::validate(catalog, record)?;
    if record
        .definition
        .component_mask
        .is_none_or(|mask| mask & (1 << ComponentSlot::Performer.index()) == 0)
    {
        return Err(Error::Unsupported(
            "57E000 Performer outside actual source factory",
        ));
    }
    performer(record)?;
    Ok(source_pc)
}

///Native reads the local pointer (+8), not the shared definition (+4). Resolve
///that exact immutable Bank identity and read only its +60/+64 integer fields.
fn rank_limits(catalog: &Catalog<'_>, token: u32, source_pc: Precision) -> Result<[i32; 2]> {
    let name = {
        let bank = catalog.definition_bank.try_borrow().map_err(error)?;
        let entry = bank.entry(token).map_err(error)?;
        if entry.key.domain != "actorlocalability" {
            return Err(Error::Unsupported(
                "57DCF0 local pointer belongs to another native manager",
            ));
        }
        if bank.token("ActorLocalAbility", &entry.key.name) != Some(token) {
            return Err(Error::Unsupported(
                "57DCF0 actual local definition Bank identity differs",
            ));
        }
        entry.key.name.clone()
    };
    let (id, ordinal, defender) = if let Some((id, index)) = name.rsplit_once("/ability/") {
        (id, index, false)
    } else if let Some((id, index)) = name.rsplit_once("/defender/") {
        (id, index, true)
    } else {
        return Err(Error::Unsupported(
            "57DCF0 actual local definition source producer unavailable",
        ));
    };
    let ordinal = ordinal.parse::<usize>().map_err(error)?;
    catalog
        .require_untranslated_actor_identity(id)
        .map_err(error)?;
    let binding = catalog.actor_bindings(id, source_pc).map_err(error)?;
    let defender_source;
    let loaded = if defender {
        defender_source = catalog
            .actors
            .native_loaded_defender(
                id,
                source_pc,
                binding
                    .character
                    .as_ref()
                    .map(|value| value as &dyn aa_rules::abilities::AnimationResolver),
            )
            .map_err(error)?;
        defender_source
            .as_ref()
            .ok_or(Error::Unsupported(
                "57DCF0 actual Defender local source absent",
            ))?
            .abilities()
            .get(ordinal)
    } else {
        binding
            .performer
            .as_ref()
            .ok_or(Error::Unsupported(
                "57DCF0 actual Performer local source absent",
            ))?
            .abilities()
            .get(ordinal)
    }
    .ok_or(Error::Unsupported(
        "57DCF0 actual local source ordinal absent",
    ))?;
    let read = |field| {
        loaded
            .authored
            .fields
            .get(field)
            .ok_or(Error::Unsupported(
                "57DCF0 actual required-experience field unavailable",
            ))?
            .integer()
            .map_err(error)
    };
    Ok([
        read("required_experience_min")?,
        read("required_experience_max")?,
    ])
}
struct Bound<'a, 'source> {
    catalog: &'a Catalog<'source>,
    source_pc: Precision,
}
impl RankWorld for Bound<'_, '_> {
    fn ability_local_definition(&self, handle: u32) -> aa_combat::Result<Option<u32>> {
        Ok(self
            .catalog
            .ability_arena
            .try_borrow()
            .map_err(combat)?
            .get(handle)
            .map_err(combat)?
            .local_definition)
    }
    fn rank_limits(&self, local: u32) -> aa_combat::Result<[i32; 2]> {
        rank_limits(self.catalog, local, self.source_pc).map_err(combat)
    }
    fn ability_enabled(&self, handle: u32) -> aa_combat::Result<bool> {
        Ok(self
            .catalog
            .ability_arena
            .try_borrow()
            .map_err(combat)?
            .get(handle)
            .map_err(combat)?
            .enabled)
    }
    fn set_ability_enabled(&mut self, handle: u32, value: bool) -> aa_combat::Result<()> {
        self.catalog
            .ability_arena
            .try_borrow_mut()
            .map_err(combat)?
            .get_mut(handle)
            .map_err(combat)?
            .enabled = value;
        Ok(())
    }
    fn notify(&mut self, _owner: u32) -> aa_combat::Result<()> {
        Err(combat(
            "57DCF0 full notify requires released Performer loan and SAME held Actor; prefix-only adapter cannot simulate it",
        ))
    }
}

pub fn init_existing(
    catalog: &Catalog<'_>,
    inventory: &WorldInventory,
    record: &mut Record<'_>,
    live_pc: Precision,
) -> Result<bool> {
    let source_pc = validate(catalog, record)?;
    let changed = {
        let current = performer(record)?;
        if current.state.abilities.len() > i32::MAX as usize {
            return Err(Error::Unsupported(
                "57DCF0 signed primary Ability count representation",
            ));
        }
        //57E000's PUSH0 is an actual NULL rank object:57DCF0 then reads0,
        //not current Experience state and not a fabricated Ability context.
        performer::gate_by_rank_prefix(&current.state, None, &mut Bound { catalog, source_pc })
            .map_err(error)?
    };
    //The immutable component/Arena inspections have ended. The real component
    //remains in its slot, so actions reentry observes all completed byte stores.
    if changed {
        crate::load_actor_action_refresh::refresh(catalog, inventory, record, live_pc)?;
    }
    Ok(true)
}
pub fn after_init(catalog: &Catalog<'_>, record: &mut Record<'_>) -> Result<()> {
    validate(catalog, record)?;
    let leaf = aa_actors::component_virtuals::leaves(
        aa_actors::init_create::FactoryKind::Standard(ComponentSlot::Performer),
    );
    if leaf.vf14 != 0x4b2830 {
        return Err(Error::Unsupported(
            "Performer AfterInit actual virtual14 leaf differs",
        ));
    }
    Ok(())
}
