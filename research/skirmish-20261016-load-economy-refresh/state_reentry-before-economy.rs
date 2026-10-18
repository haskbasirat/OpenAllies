//! Reached4D4A90 saved Actor state entry over the held allocation.
//! Empty source lists are certified from one canonical ActorData declaration;
//! positive effects retain their native position and require their real owner.
use crate::{
    catalog::Catalog,
    live_object_init::{Record, Source},
    load_actor_registration::Reentry,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    components::ComponentSlot,
    loading::LoadKind,
};
use aa_numeric::Precision;
use aa_rules::{native::TypedValue, states::ActorStateInputs};

#[cfg(test)]
#[path = "load_actor_state_reentry_tests.rs"]
mod tests;

fn error(v: impl std::fmt::Display) -> Error {
    Error::World(v.to_string())
}

/// No cached actor fields, fabricated component handles, or default world mode.
pub struct Context;
impl Reentry for Context {
    fn state_reenter(
        &mut self,
        catalog: &Catalog<'_>,
        record: &mut Record<'_>,
        index: u32,
        source: ActorStateInputs,
        pc: Precision,
    ) -> Result<()> {
        let _ = pc; // The supported null-Body/empty-Stance branch performs no FP arithmetic.
        let canonical = Source::bind(catalog, record.definition)?;
        canonical.validate_record(record)?;
        if !matches!(record.definition.native_class, 3 | 4)
            || record.shell.kind != LoadKind::Actor
            || record.shell.actor.is_none()
            || record.shell.object.is_none()
            || !record
                .registry
                .get(record.owner)
                .is_some_and(|e| matches!(e.value, aa_mapworld::restore::RestoredRecord::Loading))
        {
            return Err(Error::Unsupported(
                "4D4A90 actual held Actor/Kingdom storage",
            ));
        }
        let source_pc = canonical.source_precision();
        if record.definition.component_mask
            != Some(
                catalog
                    .canonical_component_mask(&record.definition.proof.id, source_pc)
                    .map_err(error)?,
            )
        {
            return Err(Error::Unsupported("4D4A90 canonical component-mask owner"));
        }
        let descriptor = aa_actors::state::DESCRIPTORS
            .get(index as usize)
            .ok_or(Error::Unsupported("4D4A90 registered state index"))?;
        if !catalog
            .actors
            .native_data_registry("ActorState")
            .is_some_and(|m| m.mode == Some(2) && m.manager.eq_ignore_ascii_case("ActorState"))
        {
            return Err(Error::Unsupported("4D4A90 actual mode2 state manager"));
        }
        if !source.id.eq_ignore_ascii_case(descriptor.name)
            || !catalog
                .actors
                .native_data_find("ActorState", descriptor.name)
                .is_some_and(|e| e.ids.eq_ignore_ascii_case(descriptor.name))
        {
            return Err(Error::Unsupported("4D4A90 native descriptor identity"));
        }
        let actual = catalog
            .actors
            .native_actor_state(descriptor.name, source_pc)
            .map_err(error)?;
        //47DD32 only loads1C..1F for the native fixed Mind group4. Other
        //groups retain474270's zero bytes; authored shape cannot change group.
        if (descriptor.group == Some(4)) != actual.mind.is_some() {
            return Err(Error::Unsupported(
                "4D4A90 typed Mind fields differ from native group4",
            ));
        }
        if serde_json::to_value(&source).map_err(error)?
            != serde_json::to_value(&actual).map_err(error)?
        {
            return Err(Error::Unsupported(
                "4D4A90 supplied descriptor differs from canonical source",
            ));
        }
        enter(&mut Borrowed {
            catalog,
            record,
            source: actual,
            source_pc,
            lists: None,
        })
    }
}

/// Separate narrow coordinator so all supported native rows exercise the same
/// branch ordering as the live owner; callback errors retain earlier stores.
trait World {
    fn component(&self, slot: ComponentSlot) -> Result<bool>;
    fn state_properties_empty(&self) -> Result<bool>;
    fn stance_count(&mut self) -> Result<usize>;
    fn current_view(&self) -> Result<i32>;
    fn old_property_is_null(&mut self, index: usize) -> Result<bool>;
    fn set_view(&mut self, value: i32) -> Result<()>;
    fn active_fx_only(&self) -> Result<bool>;
    fn enter_fx_empty(&mut self) -> Result<bool>;
    fn remove_from_group(&self) -> Result<bool>;
    fn group_reference(&self) -> Result<ActorId>;
    fn resolve(&self, id: ActorId) -> Result<bool>;
}
fn enter(w: &mut impl World) -> Result<()> {
    if w.component(ComponentSlot::Organization)? {
        return Err(Error::Unsupported(
            "57C400 actual Organization state transition",
        ));
    }
    if w.component(ComponentSlot::Economy)? {
        return Err(Error::Unsupported("570020 actual Economy refresh"));
    }
    if !w.state_properties_empty()? {
        return Err(Error::Unsupported(
            "4D4A90 actual state Property list handles",
        ));
    }
    let count = w.stance_count()?;
    if count > i32::MAX as usize {
        return Err(Error::Unsupported("45E620 signed stance count"));
    }
    if count != 0 && w.component(ComponentSlot::Body)? {
        return Err(Error::Unsupported(
            "4B4590 actual Body health/source stance rules",
        ));
    }
    //45E620 returns-1 for an empty source array or when every rule sees Body=NULL.
    let next = -1;
    let old = w.current_view()?;
    if old != next {
        if w.component(ComponentSlot::Character)? {
            return Err(Error::Unsupported(
                "5638B0 actual Character stance transition",
            ));
        }
        if old < 0 || old as usize >= count {
            return Err(Error::Unsupported(
                "4D4A90 native invalid previous stance diagnostic",
            ));
        }
        if !w.old_property_is_null(old as usize)? && w.component(ComponentSlot::Properties)? {
            return Err(Error::Unsupported(
                "581810 actual old stance property removal",
            ));
        }
        w.set_view(next)?;
    }
    let _active = w.active_fx_only()?; //4F0460 executes even when the FX list is null.
    if !w.enter_fx_empty()? {
        return Err(Error::Unsupported(
            "4D0A00 actual ActorData WhenEnter FXC owner",
        ));
    }
    if w.remove_from_group()? {
        let id = w.group_reference()?;
        if w.resolve(id)? {
            if !w.resolve(id)? {
                return Err(Error::Unsupported("4D4A90 second group generation changed"));
            }
            return Err(Error::Unsupported("4DFF30 actual current Group removal"));
        }
    }
    Ok(())
}

struct Lists {
    stance_properties: Vec<Option<TypedValue>>,
    enter_count: usize,
}
struct Borrowed<'c, 's, 'r> {
    catalog: &'c Catalog<'s>,
    record: &'c mut Record<'r>,
    source: ActorStateInputs,
    source_pc: Precision,
    lists: Option<Lists>,
}
impl Borrowed<'_, '_, '_> {
    fn lists(&mut self) -> Result<&Lists> {
        if self.lists.is_none() {
            let id = &self.record.definition.proof.id;
            //4624B0/79E130 construct zero count/head/tail.464BD0 appends;
            //therefore absence is only a null proof with one source declaration.
            let mut declarations = 0;
            for tag in aa_rules::actors::native_load::THING_TAGS {
                for node in self
                    .catalog
                    .actors
                    .resolved_definitions(tag)
                    .map_err(error)?
                {
                    if let Some(TypedValue::String { value, .. }) =
                        node.field("IDS", self.source_pc).map_err(error)?
                        && value.eq_ignore_ascii_case(id)
                    {
                        declarations += 1;
                    }
                }
            }
            if declarations != 1 {
                return Err(Error::Unsupported(
                    "ActorData state lists require actual shared reload ownership",
                ));
            }
            let node = self.catalog.actors.resolved_actor(id).map_err(error)?;
            let stances = node.children("Stance").map_err(error)?;
            // Preserve optional source fields without interpreting them before
            //the selected old rule's property is actually reached.
            let stance_properties = stances
                .iter()
                .map(|n| n.field("property_IDS", self.source_pc).map_err(error))
                .collect::<Result<Vec<_>>>()?;
            //464FFB/42B8E0 may copy eligible WhenLeave graphs into WhenEnter.
            //Without the FXC owner, only BOTH source lists empty certify null.
            let enter_count = node.children("WhenEnter").map_err(error)?.len()
                + node.children("WhenLeave").map_err(error)?.len();
            self.lists = Some(Lists {
                stance_properties,
                enter_count,
            });
        }
        Ok(self.lists.as_ref().unwrap())
    }
}
impl World for Borrowed<'_, '_, '_> {
    fn component(&self, s: ComponentSlot) -> Result<bool> {
        Ok(self
            .record
            .shell
            .actor
            .as_ref()
            .ok_or(Error::Unsupported("held Actor"))?
            .components
            .get(s)
            .is_some())
    }
    fn state_properties_empty(&self) -> Result<bool> {
        Ok(self.source.properties.is_empty())
    }
    fn stance_count(&mut self) -> Result<usize> {
        Ok(self.lists()?.stance_properties.len())
    }
    fn current_view(&self) -> Result<i32> {
        self.record
            .shell
            .actor
            .as_ref()
            .and_then(|a| a.words.get(&0x15c))
            .copied()
            .map(|v| v as i32)
            .ok_or(Error::Unsupported("actual Actor15C"))
    }
    fn old_property_is_null(&mut self, i: usize) -> Result<bool> {
        match self
            .lists()?
            .stance_properties
            .get(i)
            .ok_or(Error::Unsupported("old stance source index"))?
        {
            Some(TypedValue::String { value, .. }) => {
                Ok(value.is_empty() || value.eq_ignore_ascii_case("null"))
            }
            _ => Err(Error::Unsupported("old stance typed property_IDS")),
        }
    }
    fn set_view(&mut self, v: i32) -> Result<()> {
        let a = self
            .record
            .shell
            .actor
            .as_mut()
            .ok_or(Error::Unsupported("held Actor"))?;
        let word = a
            .words
            .get_mut(&0x15c)
            .ok_or(Error::Unsupported("actual Actor15C"))?;
        *word = v as u32;
        Ok(())
    }
    fn active_fx_only(&self) -> Result<bool> {
        self.catalog
            .native_load_mode
            .map(|v| v != 0)
            .ok_or(Error::Unsupported(
                "4F0460 actual retained MapResource mode",
            ))
    }
    fn enter_fx_empty(&mut self) -> Result<bool> {
        Ok(self.lists()?.enter_count == 0)
    }
    fn remove_from_group(&self) -> Result<bool> {
        match &self.source.mind {
            Some(v) => v.remove_from_group.boolean().map_err(error),
            None => Ok(false),
        }
    }
    fn group_reference(&self) -> Result<ActorId> {
        self.record
            .shell
            .actor
            .as_ref()
            .and_then(|a| a.words.get(&0x164))
            .copied()
            .map(ActorId::from_raw)
            .ok_or(Error::Unsupported("actual Actor164 group reference"))
    }
    fn resolve(&self, id: ActorId) -> Result<bool> {
        Ok(self.record.registry.get(id).is_some())
    }
}
