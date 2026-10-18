//!5827B0(definition,source-default-layout,1,NULL) on the SAME Recruit/Kingdom.
//!Source projections use the moved Bank PC. Recruitment table fields stay in
//!their actual Kingdom owner; no inventory table or availability copy is used.
use crate::{
    actor_world::{ActorAccess, ObjectKey, RuntimeComponent, Source, SpecialSource},
    catalog::{ActorBindings, Catalog},
    definitions::Bank,
    recruit_eligibility_core::{self as core, RequirementResult},
};
use aa_actors::{
    ActorId,
    ability_flags::{Error, Result},
    components::ComponentSlot as Slot,
};
use aa_numeric::Precision;
use aa_rules::native::{layout_data::OrganizationComponentData, recruit_data};
use std::sync::Arc;
fn error(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}
fn required<T>(message: impl std::fmt::Display) -> Result<T> {
    Err(error(message))
}

pub(crate) fn eligibility(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    bank: &mut Bank,
    recruiter: ActorId,
    definition: &str,
    layout: &Option<OrganizationComponentData>,
    source_pc: Precision,
) -> Result<u32> {
    catalog.require_source_owner().map_err(error)?;
    let source = bank.combat_source().map_err(error)?;
    source.completed_means().map_err(error)?;
    if source.source_precision() != source_pc
        || source.source_identities() != catalog.actors.sources.as_slice()
    {
        return required("5827B0 same moved Bank source universe/precision differs");
    }
    let mut world = Bound {
        catalog,
        actors,
        bank,
        recruiter,
        source_pc,
    };
    world.recruiter_key()?;
    //The caller's native582C40 already obtained this exact source default.
    //Reject a foreign/custom metadata projection before treating it as +2F0.
    let actual = world.default_layout_for(definition)?;
    if actual.as_ref() != layout.as_ref() {
        return required("5827B0 argument is not the actual source-default Layout");
    }
    core::eligibility(
        &mut world,
        Some(&definition.to_owned()),
        layout.as_ref(),
        true,
    )
}

struct Bound<'a, 'store, 'source> {
    catalog: &'a Catalog<'source>,
    actors: &'a mut ActorAccess<'store>,
    bank: &'a mut Bank,
    recruiter: ActorId,
    source_pc: Precision,
}
impl Bound<'_, '_, '_> {
    fn data(&self, id: &str) -> Result<Arc<ActorBindings>> {
        self.catalog
            .require_untranslated_actor_identity(id)
            .map_err(error)?;
        let data = self
            .catalog
            .actor_bindings(id, self.source_pc)
            .map_err(error)?;
        if data.precision != self.source_pc || data.definition.native_class != 3 {
            return required("581F70 actual target ActorData source differs");
        }
        Ok(data)
    }
    fn default_layout_for(&self, id: &str) -> Result<Option<OrganizationComponentData>> {
        self.data(id)?;
        self.catalog
            .layout_data(self.source_pc)
            .map_err(error)?
            .organization_component(self.catalog.actors, id, self.source_pc)
            .map_err(error)
    }
    fn recruiter_key(&self) -> Result<ObjectKey> {
        let key = self
            .actors
            .key(self.recruiter)
            .ok_or_else(|| error("581F70 actual Recruit GTID absent"))?;
        let object = self.actors.object(key).map_err(error)?;
        crate::live_object_init::Source::bind_in_bank(self.catalog, self.bank, &object.definition)
            .map_err(error)?;
        if !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof)
            || object.kind != aa_actors::loading::LoadKind::Actor
            || object.definition.native_class != 3
        {
            return required("581F70 actual Recruit Actor source/layout differs");
        }
        match object.components().map_err(error)?.get(Slot::Recruit) {
            Some(RuntimeComponent::Recruit(value)) if value.header.owner == self.recruiter => {
                Ok(key)
            }
            _ => required("581F70 actual Recruit allocation/owner differs"),
        }
    }
    fn kingdom_key(&self, id: ActorId) -> Result<ObjectKey> {
        let key = self
            .actors
            .key(id)
            .ok_or_else(|| error("4E65E0 current Kingdom generation absent"))?;
        let object = self.actors.object(key).map_err(error)?;
        crate::live_object_init::Source::bind_in_bank(self.catalog, self.bank, &object.definition)
            .map_err(error)?;
        if object.definition.native_class != 4
            || !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof)
            || !crate::load_spatial::ordinary_layout(
                4,
                object.kind,
                object.object.is_some(),
                object.actor.is_some(),
            )
        {
            return required("4E65E0 actual Kingdom source/layout differs");
        }
        Ok(key)
    }
    fn query_item(&mut self, kingdom: ActorId, definition: &str, wanted: i32) -> Result<bool> {
        let key = self.kingdom_key(kingdom)?;
        let bank = &*self.bank;
        let Source::Special(value) = &mut self.actors.entity_mut(key).map_err(error)?.source else {
            return required("4E3FF0 actual Kingdom derived owner absent");
        };
        if !matches!(value.source, SpecialSource::Kingdom(_)) {
            return required("4E3FF0 actual source is not Kingdom");
        }
        let table = &mut value
            .derived
            .as_mut()
            .ok_or_else(|| error("4E3FF0 actual Kingdom fields absent"))?
            .recruit_items;
        //The table stores these SAME Bank identities. A definition that has
        //never had an item registered cannot have a hash match; do not intern
        //a new portable token or inspect unrelated item payloads.
        let Some(token) = bank.token("ActorDefinition", definition) else {
            return Ok(false);
        };
        table.query_item(token, wanted, true).map_err(error)
    }

    fn layout_entry_numbers(&self, layout: &OrganizationComponentData) -> Result<Vec<i32>> {
        let data = self.catalog.layout_data(self.source_pc).map_err(error)?;
        let actual = data
            .layouts
            .iter()
            .find(|l| l.ordinal == layout.layout)
            .ok_or_else(|| error("4E4050 actual LayoutData ordinal absent"))?;
        if actual.ids != layout.layout_ids || actual.entries.len() != layout.parameters.len() {
            return required("4E4050 actual source Layout count/identity differs");
        }
        Ok(actual.entries.iter().map(|entry| entry.number).collect())
    }
}
impl core::World for Bound<'_, '_, '_> {
    type Definition = String;
    type Layout = OrganizationComponentData;
    fn organization(&mut self, id: &String) -> Result<bool> {
        Ok(self
            .data(id)?
            .definition
            .component_mask
            .ok_or_else(|| error("581F70 completed component mask absent"))?
            & (1 << 7)
            != 0)
    }
    fn default_layout(&mut self, id: &String) -> Result<Option<Self::Layout>> {
        self.default_layout_for(id)
    }
    fn owner(&mut self) -> Result<Option<ActorId>> {
        let key = self.recruiter_key()?;
        let actor = self
            .actors
            .object(key)
            .map_err(error)?
            .actor
            .as_ref()
            .ok_or_else(|| error("581F70 actual Actor absent"))?;
        let owner = *actor
            .words
            .get(&0xe8)
            .ok_or_else(|| error("581F70 actual ActorE8 uninitialized"))?;
        Ok((owner != 0).then_some(ActorId::from_raw(owner)))
    }
    fn owner_allows(
        &mut self,
        owner: ActorId,
        id: &String,
        layout: Option<&Self::Layout>,
    ) -> Result<bool> {
        let binding = self.data(id)?;
        let class = match &binding.source_economy {
            Some(source) => source.recruitable_word,
            None => {
                self.catalog
                    .actors
                    .native_economy_inputs(id, self.source_pc)
                    .map_err(error)?
                    .recruitable_word
            }
        };
        let mask = binding
            .definition
            .component_mask
            .ok_or_else(|| error("45E5A0 completed component mask absent"))?;
        let recruitable = aa_actors::economy_values::recruitable_fields(class, mask, || {
            binding
                .source_ground
                .organization
                .as_ref()
                .map(|value| value.layout.primary_index)
                .ok_or_else(|| {
                    aa_actors::economy_values::Error::World("45E5A0 actual Layout+18 absent".into())
                })
        })
        .map_err(error)?;
        if !recruitable {
            return Ok(false);
        }
        let fallback;
        let layout = if let Some(layout) = layout {
            Some(layout)
        } else if mask & (1 << 7) != 0 {
            fallback = self.default_layout_for(id)?;
            Some(
                fallback
                    .as_ref()
                    .ok_or_else(|| error("4E65E0 reached null default Layout"))?,
            )
        } else {
            None
        };
        if let Some(layout) = layout {
            let numbers = self.layout_entry_numbers(layout)?;
            for (member, wanted) in layout.parameters.iter().zip(numbers) {
                if let Some(member) = member {
                    self.data(member)?;
                    if !self.query_item(owner, member, wanted)? {
                        return Ok(false);
                    }
                }
            }
            Ok(true)
        } else {
            self.query_item(owner, id, 1)
        }
    }
    fn own_recruit_list_present(&mut self) -> Result<bool> {
        let key = self.recruiter_key()?;
        let id = &self.actors.object(key).map_err(error)?.definition.proof.id;
        Ok(
            !recruit_data::actor_recruit_targets(self.catalog.actors, id)
                .map_err(error)?
                .ids
                .is_empty(),
        )
    }
    fn own_recruit_list_contains(&mut self, target: &String) -> Result<bool> {
        let key = self.recruiter_key()?;
        let id = &self.actors.object(key).map_err(error)?.definition.proof.id;
        Ok(recruit_data::actor_recruit_targets(self.catalog.actors, id)
            .map_err(error)?
            .ids
            .iter()
            .any(|id| id == target))
    }
    fn layout_count(&mut self, layout: &Self::Layout) -> Result<i32> {
        self.layout_entry_numbers(layout)?;
        i32::try_from(layout.parameters.len()).map_err(error)
    }
    fn layout_member(&mut self, layout: &Self::Layout, index: i32) -> Result<Option<String>> {
        layout
            .parameters
            .get(index as usize)
            .cloned()
            .ok_or_else(|| error("581F70 actual Layout parameter ordinal absent"))
    }
    fn requirements(&mut self, id: &String) -> Result<RequirementResult> {
        required(format!(
            "581F70 actual4C5250 requirements source/current owner for {id}"
        ))
    }
    fn strict_requirements(&mut self, id: &String) -> Result<bool> {
        required(format!(
            "581F70 actual ActorData+1B3 current source for {id}"
        ))
    }
}
