//! Recruit582C40 over the same authoritative saved-restore shell. Source list
//! absence is established by the real deferred-reference producer projection;
//! an unbound5827B0 eligibility dependency remains an error when reached.
use crate::saved_organization_actions::{SavedActions, action_error};
use aa_actors::{ActorId, ability_flags as flags, components::ComponentSlot, recruit_actions};
use aa_mapworld::restore::RestoredComponent;
use aa_rules::native::{layout_data::OrganizationComponentData, recruit_data};

pub(crate) fn contribute(
    actions: &mut SavedActions<'_, '_, '_>,
    actor: ActorId,
) -> flags::Result<()> {
    recruit_actions::contribute(actions, actor)
}
fn source_id(actions: &SavedActions<'_, '_, '_>, actor: ActorId) -> flags::Result<String> {
    if actor == actions.context.owner {
        return Ok(actions.context.owner_definition.proof.id.clone());
    }
    actions
        .context
        .registry
        .get(actor)
        .and_then(|e| e.value.loaded())
        .map(|e| e.definition.proof.id.clone())
        .ok_or_else(|| {
            action_error(format!(
                "saved Recruit actor {:#x} has no source definition",
                actor.raw()
            ))
        })
}
fn recruit<'a>(
    actions: &'a SavedActions<'_, '_, '_>,
    actor: ActorId,
) -> flags::Result<&'a aa_actors::recruit_state::RecruitState> {
    match actions.component(actor, ComponentSlot::Recruit)? {
        Some(RestoredComponent::Recruit(value)) => Ok(value),
        _ => Err(flags::Error::Missing("saved Recruit payload for582C40")),
    }
}
fn targets(actions: &SavedActions<'_, '_, '_>, source: &str) -> flags::Result<Vec<String>> {
    recruit_data::actor_recruit_targets(actions.catalog.actors, source)
        .map(|r| r.ids)
        .map_err(action_error)
}

impl recruit_actions::World for SavedActions<'_, '_, '_> {
    type Cursor = (String, usize);
    type Definition = String;
    type Layout = Option<OrganizationComponentData>;
    fn recruit_owner(&self, actor: ActorId) -> flags::Result<ActorId> {
        Ok(recruit(self, actor)?.header.owner)
    }
    fn fields(&self, actor: ActorId) -> flags::Result<flags::ActorFields> {
        flags::World::actor_fields(self, actor)
    }
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> flags::Result<()> {
        flags::World::set_action_bits(self, actor, bits)
    }
    fn state_type(&self, index: u8) -> flags::Result<u32> {
        let descriptor = aa_actors::state::DESCRIPTORS
            .get(index as usize)
            .ok_or(flags::Error::Missing("saved Recruit ActorState index"))?;
        recruit_data::actor_state_type(
            self.catalog.actors,
            descriptor.name,
            self.source_precision()?,
        )
        .map_err(action_error)
    }
    fn settlement_reference(&self, actor: ActorId) -> flags::Result<Option<Option<ActorId>>> {
        match self.component(actor, ComponentSlot::Settlement)? {
            None => Ok(None),
            Some(_) => Err(flags::Error::Missing(
                "saved Settlement+14 payload for Recruit582C40",
            )),
        }
    }
    fn recruit_enabled(&self, actor: ActorId, location: bool) -> flags::Result<u8> {
        let value = recruit(self, actor)?;
        Ok(if location {
            value.byte_10
        } else {
            value.byte_11
        })
    }
    fn recruit_list_head(&self, actor: ActorId) -> flags::Result<Option<Self::Cursor>> {
        let source = source_id(self, actor)?;
        Ok((!targets(self, &source)?.is_empty()).then_some((source, 0)))
    }
    fn recruit_entry(
        &self,
        cursor: &Self::Cursor,
    ) -> flags::Result<recruit_actions::Entry<String, Self::Layout>> {
        let definition = targets(self, &cursor.0)?
            .get(cursor.1)
            .cloned()
            .ok_or(flags::Error::Missing("saved Recruit list cursor"))?;
        let location_class = recruit_data::actor_transport_class(self.catalog.actors, &definition)
            .map_err(action_error)?;
        let layout = self
            .catalog
            .layout_data(self.source_precision()?)
            .map_err(action_error)?;
        let default_layout = layout
            .organization_component(self.catalog.actors, &definition, self.source_precision()?)
            .map_err(action_error)?;
        Ok(recruit_actions::Entry {
            definition,
            location_class,
            default_layout,
        })
    }
    fn next_recruit_entry(&self, cursor: &Self::Cursor) -> flags::Result<Option<Self::Cursor>> {
        let next = cursor
            .1
            .checked_add(1)
            .ok_or(flags::Error::Missing("saved Recruit cursor overflow"))?;
        Ok((next < targets(self, &cursor.0)?.len()).then_some((cursor.0.clone(), next)))
    }
    fn eligibility(&mut self, _: ActorId, _: &String, _: &Self::Layout) -> flags::Result<u32> {
        Err(flags::Error::Missing(
            "saved Recruit5827B0 requirements/eligibility service",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        catalog::Catalog, localization::Localization, restoration::Attempt, settings::Settings,
    };
    use aa_actors::recruit_actions::World;
    use aa_mapworld::restore::{OrganizationAttachContext, RestoredRecord};
    use aa_numeric::Precision;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };

    #[test]
    #[ignore = "requires AA_GAME_ROOT original/v14 assets"]
    fn saved_zero_enable_bytes_keep_real_nonempty_lists_and_enabled_path_stops_explicitly() {
        let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let mut sources = V14Sources::open(
            root.join("Data.rwd"),
            root.join("reconstruction/native-game/runtime-expanded-v14"),
            SourceLimits {
                max_member_bytes: 64 * 1024 * 1024,
            },
        )
        .unwrap();
        let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
        let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
        let actors = ActorCatalog::load_v14(&mut sources).unwrap();
        let map = aa_mapworld::MapArchive::from_bytes(
            std::fs::read(
                root.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
            )
            .unwrap(),
        )
        .unwrap();
        let mut attempt =
            Attempt::load(&map, &actors, settings, text, Precision::Double53, u64::MAX).unwrap();
        let mut owner = None;
        let mut candidate = None;
        for entry in attempt.registry.iter() {
            let Some(entity) = entry.value.loaded() else {
                continue;
            };
            let Some(shell) = &entity.shell.actor else {
                continue;
            };
            if owner.is_none() && shell.components.present(ComponentSlot::Organization) {
                owner = Some(entry.id);
            }
            if candidate.is_none()
                && shell.components.present(ComponentSlot::Recruit)
                && !recruit_data::actor_recruit_targets(&actors, &entity.definition.proof.id)
                    .unwrap()
                    .ids
                    .is_empty()
            {
                candidate = Some(entry.id);
            }
        }
        let owner = owner.expect("real restored Organization");
        let candidate = candidate.expect("real restored Recruit with nonempty source list");
        let taken = std::mem::replace(
            attempt.registry.value_mut(owner).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Actor(mut entity) = taken else {
            panic!("Organization actor");
        };
        let actor = entity.shell.actor.as_mut().unwrap();
        let Some(RestoredComponent::Organization(mut organization)) =
            actor.components.remove(ComponentSlot::Organization)
        else {
            panic!("Organization payload");
        };
        let catalog = Catalog::new(&actors);
        let mut actions = SavedActions {
            catalog: &catalog,
            context: OrganizationAttachContext {
                inventory: &attempt.inventory,
                owner,
                owner_actor: actor,
                owner_definition: &entity.definition,
                owner_thing_flags: entity.shell.thing.flags,
                owner_thing_word_c: entity.shell.thing.word_c,
                owner_object_byte_4c: entity.shell.object.as_ref().unwrap().byte_4c,
                organization: &mut organization,
                registry: &mut attempt.registry,
                precision: Precision::Double53,
            }
            .into(),
            active_mind: None,
        };
        assert!(actions.recruit_list_head(candidate).unwrap().is_some());
        assert_eq!(actions.recruit_enabled(candidate, false).unwrap(), 0);
        assert_eq!(actions.recruit_enabled(candidate, true).unwrap(), 0);
        let initial = (1u64 << 50) | (1 << 14) | (1 << 15) | (1 << 16) | (1 << 17) | 8;
        {
            let shell = actions.shell_mut(candidate).unwrap();
            // Use explicit constructor-fixture normal state for the two
            // action-stage probes, retaining actual source/component payloads.
            shell.components.state.low = 1 << 27;
            shell.components.state.high = 0;
            shell.words.insert(0xf0, initial as u32);
            shell.words.insert(0xf4, (initial >> 32) as u32);
        }
        contribute(&mut actions, candidate).unwrap();
        assert_eq!(
            actions.fields(candidate).unwrap().action_bits,
            (1u64 << 50) | 8
        );
        {
            let shell = actions.shell_mut(candidate).unwrap();
            shell.words.insert(0xf0, initial as u32);
            let Some(RestoredComponent::Recruit(recruit)) =
                shell.components.get_mut(ComponentSlot::Recruit)
            else {
                panic!("Recruit retained");
            };
            recruit.byte_11 = 1;
        }
        let error = contribute(&mut actions, candidate).unwrap_err();
        assert!(error.to_string().contains("5827B0"), "{error}");
        assert_eq!(
            actions.fields(candidate).unwrap().action_bits,
            initial & !(1 << 14)
        );
    }
}
