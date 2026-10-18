//! Structure589A60 over actual saved-restore component storage. The source
//! contributor reads no ActorData fields. Parent4D1350 uses the existing
//! coordinator; missing contributors/publication/Settlement payloads stay errors.
use crate::saved_organization_actions::SavedActions;
use aa_actors::{
    ActorId, ability_flags as flags,
    components::ComponentSlot,
    structure_actions::{self, StructureReference},
};
use aa_mapworld::restore::RestoredComponent;

pub(crate) fn contribute(
    actions: &mut SavedActions<'_, '_, '_>,
    actor: ActorId,
) -> flags::Result<()> {
    structure_actions::contribute(actions, actor)
}
fn structure<'a>(
    actions: &'a SavedActions<'_, '_, '_>,
    actor: ActorId,
) -> flags::Result<&'a aa_actors::structure_state::StructureState> {
    match actions.component(actor, ComponentSlot::Structure)? {
        Some(RestoredComponent::Structure(value)) => Ok(value),
        _ => Err(flags::Error::Missing(
            "saved Structure589A60 component payload",
        )),
    }
}
impl structure_actions::World for SavedActions<'_, '_, '_> {
    fn structure_owner(&self, actor: ActorId) -> flags::Result<ActorId> {
        Ok(structure(self, actor)?.header.owner)
    }
    fn structure_reference(&self, actor: ActorId) -> flags::Result<Option<StructureReference>> {
        match self.component(actor, ComponentSlot::Structure)? {
            None => Ok(None),
            Some(RestoredComponent::Structure(value)) => Ok(Some(StructureReference {
                owner: value.header.owner,
                parent: (value.object_handle_10 != 0)
                    .then_some(ActorId::from_raw(value.object_handle_10)),
            })),
            _ => Err(flags::Error::Missing(
                "saved Structure slot payload mismatch",
            )),
        }
    }
    fn settlement_reference(&self, actor: ActorId) -> flags::Result<Option<Option<ActorId>>> {
        match self.component(actor, ComponentSlot::Settlement)? {
            None => Ok(None),
            Some(_) => Err(flags::Error::Missing(
                "saved Settlement+14 payload for Structure588990",
            )),
        }
    }
    fn action_bits(&self, actor: ActorId) -> flags::Result<u64> {
        Ok(u64::from(self.word(actor, 0xf0)?) | (u64::from(self.word(actor, 0xf4)?) << 32))
    }
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> flags::Result<()> {
        flags::World::set_action_bits(self, actor, bits)
    }
    fn queue_nonempty(&self, actor: ActorId) -> flags::Result<bool> {
        Ok(!structure(self, actor)?.queue_14.is_empty())
    }
    fn rebuild_actions(&mut self, actor: ActorId) -> flags::Result<()> {
        flags::rebuild(self, actor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        catalog::Catalog, localization::Localization, restoration::Attempt, settings::Settings,
    };
    use aa_actors::structure_actions::World;
    use aa_mapworld::restore::{OrganizationAttachContext, RestoreMode, RestoredRecord};
    use aa_numeric::Precision;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };

    #[test]
    #[ignore = "requires AA_GAME_ROOT original/v14 assets"]
    fn actual_skirmish_structure_queue_and_parent_are_owned_not_defaults() {
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
        let map =
            aa_mapworld::MapArchive::from_bytes(
                std::fs::read(root.join(
                    "reconstruction/assets/original/Data/Maps/DeveloperMaps/(3v3)Firefight.RMP",
                ))
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
            RestoreMode::StopOnError,
            Some(1),
        )
        .unwrap();
        assert!(attempt.report.stopped_before.is_none());
        let mut candidate = None;
        for entry in attempt.registry.iter() {
            if entry
                .value
                .loaded()
                .and_then(|e| e.shell.actor.as_ref())
                .is_some_and(|a| a.components.present(ComponentSlot::Structure))
            {
                candidate = Some(entry.id);
                break;
            }
        }
        let actor = candidate.expect("real Firefight Structure actor");
        let taken = std::mem::replace(
            attempt.registry.value_mut(actor).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Actor(mut entity) = taken else {
            panic!("Structure actor")
        };
        let shell = entity.shell.actor.as_mut().unwrap();
        // SavedActions' attach-only context requires an Organization owner.
        // This explicit unused context fixture does not replace or manufacture
        // the Structure payload under test; no Organization query is reached.
        let mut unused_organization = aa_actors::ground_states::OrganizationState::<String>::new(
            actor,
            aa_actors::ground_states::LayoutDefinition {
                definition: "unused test context".into(),
                parameters: vec![],
                slots: 0,
            },
            aa_body::F32::ZERO,
            aa_body::F32::ZERO,
            [aa_body::F32::ZERO; 2],
        )
        .unwrap();
        let catalog = Catalog::new(&actors);
        let mut actions = SavedActions {
            catalog: &catalog,
            context: OrganizationAttachContext {
                inventory: &attempt.inventory,
                owner: actor,
                owner_actor: shell,
                owner_definition: &entity.definition,
                owner_thing_flags: entity.shell.thing.flags,
                owner_thing_word_c: entity.shell.thing.word_c,
                owner_object_byte_4c: entity.shell.object.as_ref().unwrap().byte_4c,
                organization: &mut unused_organization,
                registry: &mut attempt.registry,
                precision: Precision::Double53,
            }
            .into(),
            active_mind: None,
        };
        let reference = actions.structure_reference(actor).unwrap().unwrap();
        assert_eq!(reference.owner, actor);
        assert_eq!(reference.parent, None);
        assert!(!actions.queue_nonempty(actor).unwrap());
        let allocation = structure(&actions, actor).unwrap() as *const _;
        let initial = (1u64 << 63) | (1 << 18) | (1 << 13) | 123;
        actions
            .shell_mut(actor)
            .unwrap()
            .words
            .insert(0xf0, initial as u32);
        actions
            .shell_mut(actor)
            .unwrap()
            .words
            .insert(0xf4, (initial >> 32) as u32);
        contribute(&mut actions, actor).unwrap();
        assert_eq!(actions.action_bits(actor).unwrap(), initial & !(1 << 18));
        assert_eq!(structure(&actions, actor).unwrap() as *const _, allocation);
        // A real nonnull parent reference with no Settlement must error;
        // it must not quietly take the unrelated-structure success path.
        let Some(RestoredComponent::Structure(value)) = actions
            .shell_mut(actor)
            .unwrap()
            .components
            .get_mut(ComponentSlot::Structure)
        else {
            panic!()
        };
        value.object_handle_10 = actor.raw();
        let before = actions.action_bits(actor).unwrap();
        let error = contribute(&mut actions, actor).unwrap_err();
        assert!(error.to_string().contains("Settlement"), "{error}");
        assert_eq!(actions.action_bits(actor).unwrap(), before);
    }
}
