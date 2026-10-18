//! Research583FC0 over actual saved component/definition owners. Reuses the
//! completed source ResearchItem list already retained for5849E0 restoration.
use crate::saved_organization_actions::{SavedActions, action_error};
use aa_actors::{ActorId, ability_flags as flags, components::ComponentSlot, research_actions};
use aa_mapworld::restore::RestoredComponent;
use aa_rules::native::recruit_data;

pub(crate) fn contribute(
    actions: &mut SavedActions<'_, '_, '_>,
    actor: ActorId,
) -> flags::Result<()> {
    research_actions::contribute(actions, actor)
}

impl research_actions::World for SavedActions<'_, '_, '_> {
    fn research_owner(&self, actor: ActorId) -> flags::Result<ActorId> {
        match self.component(actor, ComponentSlot::Research)? {
            Some(RestoredComponent::Research(value)) => Ok(value.header.owner),
            _ => Err(flags::Error::Missing("saved Research payload for583FC0")),
        }
    }
    fn state_bits(&self, actor: ActorId) -> flags::Result<u64> {
        let state = self.shell(actor)?.components.state;
        Ok(u64::from(state.low) | (u64::from(state.high) << 32))
    }
    fn state_type(&self, index: u8) -> flags::Result<u32> {
        let descriptor = aa_actors::state::DESCRIPTORS
            .get(index as usize)
            .ok_or(flags::Error::Missing("saved Research ActorState index"))?;
        recruit_data::actor_state_type(
            self.catalog.actors,
            descriptor.name,
            self.source_precision()?,
        )
        .map_err(action_error)
    }
    fn research_items_present(&self, actor: ActorId) -> flags::Result<bool> {
        let definition = if actor == self.context.owner {
            self.context.owner_definition
        } else {
            &self
                .context
                .registry
                .get(actor)
                .and_then(|entry| entry.value.loaded())
                .ok_or(flags::Error::Missing("saved Research ActorData owner"))?
                .definition
        };
        definition
            .ground
            .research_items
            .as_ref()
            .map(|items| !items.is_empty())
            .ok_or(flags::Error::Missing(
                "completed ActorData+558 ResearchItem list",
            ))
    }
    fn action_bits(&self, actor: ActorId) -> flags::Result<u64> {
        Ok(u64::from(self.word(actor, 0xf0)?) | (u64::from(self.word(actor, 0xf4)?) << 32))
    }
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> flags::Result<()> {
        flags::World::set_action_bits(self, actor, bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        catalog::Catalog, localization::Localization, restoration::Attempt, settings::Settings,
    };
    use aa_mapworld::restore::{OrganizationAttachContext, RestoredRecord};
    use aa_numeric::Precision;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };

    #[test]
    #[ignore = "requires AA_GAME_ROOT original/v14 assets"]
    fn actual_saved_research_uses_completed_definition_list_and_busy_short_circuit() {
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
        let mut attempt = Attempt::load_with_native_mode(
            &map,
            &actors,
            settings,
            text,
            Precision::Double53,
            u64::MAX,
            aa_mapworld::restore::RestoreMode::InspectPartial,
            Some(1),
        )
        .unwrap();
        let owner = attempt
            .registry
            .iter()
            .find_map(|entry| {
                let entity = entry.value.loaded()?;
                entity
                    .shell
                    .actor
                    .as_ref()?
                    .components
                    .present(ComponentSlot::Organization)
                    .then_some(entry.id)
            })
            .expect("real restored Organization");
        let candidate = attempt
            .registry
            .iter()
            .find_map(|entry| {
                if entry.id == owner {
                    return None;
                }
                let entity = entry.value.loaded()?;
                (entity
                    .shell
                    .actor
                    .as_ref()?
                    .components
                    .present(ComponentSlot::Research)
                    && !entity.definition.ground.research_items.as_ref()?.is_empty())
                .then_some(entry.id)
            })
            .expect("real saved Research and completed nonempty source list");
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
        use research_actions::World;
        assert!(actions.research_items_present(candidate).unwrap());
        let initial = (1u64 << 50) | (1 << 35) | 0x98765432;
        let original_in_progress = match actions
            .component(candidate, ComponentSlot::Research)
            .unwrap()
        {
            Some(RestoredComponent::Research(value)) => value.items.clone(),
            _ => panic!("Research payload"),
        };
        // Explicit probe states on actual payload/definition ownership. No
        // gameplay rule or original save is changed by these test inputs.
        for index in 25..32 {
            let ty = research_actions::World::state_type(&actions, index).unwrap();
            let shell = actions.shell_mut(candidate).unwrap();
            shell.components.state.low = 1 << index;
            shell.components.state.high = 0;
            shell.words.insert(0xf0, initial as u32);
            shell.words.insert(0xf4, (initial >> 32) as u32);
            contribute(&mut actions, candidate).unwrap();
            let expected = if ty == 1 {
                initial & !(1 << 35)
            } else {
                initial
            };
            assert_eq!(actions.action_bits(candidate).unwrap(), expected);
        }
        match actions
            .component(candidate, ComponentSlot::Research)
            .unwrap()
        {
            Some(RestoredComponent::Research(value)) => {
                assert_eq!(value.items, original_in_progress)
            }
            _ => panic!("Research retained"),
        }
        let busy = (25..32)
            .find(|i| research_actions::World::state_type(&actions, *i).unwrap() == 1)
            .unwrap();
        let normal = (25..32)
            .find(|i| research_actions::World::state_type(&actions, *i).unwrap() != 1)
            .unwrap();
        // Inject only an unavailable completed source list to prove that BUSY
        // never consults it, while nonbusy fails before writing any action.
        actions
            .context
            .registry
            .value_mut(candidate)
            .unwrap()
            .loaded_mut()
            .unwrap()
            .definition
            .ground
            .research_items = None;
        actions.shell_mut(candidate).unwrap().components.state.low = 1 << busy;
        contribute(&mut actions, candidate).unwrap();
        let before = actions.action_bits(candidate).unwrap();
        actions.shell_mut(candidate).unwrap().components.state.low = 1 << normal;
        let error = contribute(&mut actions, candidate).unwrap_err();
        assert!(error.to_string().contains("+558"), "{error}");
        assert_eq!(actions.action_bits(candidate).unwrap(), before);
        // Explicit empty completed list is distinct from unavailable data.
        actions
            .context
            .registry
            .value_mut(candidate)
            .unwrap()
            .loaded_mut()
            .unwrap()
            .definition
            .ground
            .research_items = Some(Vec::new());
        contribute(&mut actions, candidate).unwrap();
        assert_eq!(
            actions.action_bits(candidate).unwrap(),
            initial & !(1 << 35)
        );
    }
}
