//! Original Kingdom4E1430/4E1E40 using the actual saved Kingdom HQ vector.
//! Empty constructed vectors return natively. Present HQs require their real
//! component eligibility services; those calls remain explicit boundaries.
use crate::saved_organization_actions::{SavedActions, action_error};
use aa_actors::{ActorId, ability_flags as flags, organization_recruitment as recruitment};
use aa_mapworld::restore::RestoredRecord;

fn hqs<'a>(actions: &'a SavedActions<'_, '_, '_>, owner: ActorId) -> flags::Result<&'a [u32]> {
    let Some(RestoredRecord::Kingdom(record)) = actions
        .context
        .registry
        .get(owner)
        .map(|entry| &entry.value)
    else {
        return Err(flags::Error::Missing(
            "saved owner Kingdom for4E1430/4E1E40",
        ));
    };
    record
        .runtime
        .as_ref()
        .and_then(|runtime| runtime.derived.as_ref())
        .and_then(|shell| shell.vectors.get(&0x2dc))
        .map(|vector| vector.values.as_slice())
        .ok_or(flags::Error::Missing("owned Kingdom+2DC HQ vector"))
}

pub(crate) fn can_recruit(
    actions: &mut SavedActions<'_, '_, '_>,
    owner: ActorId,
    actor: ActorId,
) -> flags::Result<bool> {
    let definition = actions.source_id(actor)?.to_owned();
    recruitment::can_recruit(actions, owner, &definition)
}

pub(crate) fn recruit_count(
    actions: &mut SavedActions<'_, '_, '_>,
    owner: ActorId,
    actor: ActorId,
) -> flags::Result<i32> {
    recruitment::recruit_count(actions, owner, actor)
}

impl recruitment::World for SavedActions<'_, '_, '_> {
    type Definition = String;
    fn hq_count(&self, owner: ActorId) -> flags::Result<usize> {
        Ok(hqs(self, owner)?.len())
    }
    fn hq_at(&self, owner: ActorId, index: usize) -> flags::Result<ActorId> {
        hqs(self, owner)?
            .get(index)
            .copied()
            .map(ActorId::from_raw)
            .ok_or_else(|| action_error("Kingdom HQ index outside current owned vector"))
    }
    fn actor_definition(&self, actor: ActorId) -> flags::Result<String> {
        self.source_id(actor).map(str::to_owned)
    }
    fn hq_can_recruit(&mut self, _: ActorId, _: &String) -> flags::Result<bool> {
        Err(flags::Error::Missing(
            "saved HQ+D8 573E50 recruitment eligibility",
        ))
    }
    fn hq_accepts(&mut self, _: ActorId, _: &String) -> flags::Result<bool> {
        Err(flags::Error::Missing(
            "saved HQ+D8 574BF0/574810 acceptance",
        ))
    }
    fn organization_reference(&self, actor: ActorId) -> flags::Result<Option<ActorId>> {
        let id = ActorId::from_raw(self.org_word(actor, 0xd0)?);
        Ok((id == self.context.owner || self.context.registry.get(id).is_some()).then_some(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        catalog::Catalog, localization::Localization, restoration::Attempt, settings::Settings,
    };
    use aa_actors::{ability_flags::World, components::ComponentSlot};
    use aa_mapworld::restore::{OrganizationAttachContext, RestoredComponent};
    use aa_numeric::Precision;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };

    #[test]
    #[ignore = "requires AA_GAME_ROOT original/v14 assets"]
    fn actual_saved_kingdom_hq_vector_returns_for_empty_and_requires_present_hq_service() {
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
        // InspectPartial is a diagnostic owner source, never a battle/publication proof.
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
                let shell = entry.value.loaded()?.shell.actor.as_ref()?;
                (shell.components.present(ComponentSlot::Organization)
                    && shell.words.get(&0xe8).is_some_and(|id| *id != 0))
                .then_some(entry.id)
            })
            .expect("saved Organization with owner Kingdom");
        let taken = std::mem::replace(
            attempt.registry.value_mut(owner).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Actor(mut entity) = taken else {
            panic!("Actor");
        };
        let actor = entity.shell.actor.as_mut().unwrap();
        let Some(RestoredComponent::Organization(mut organization)) =
            actor.components.remove(ComponentSlot::Organization)
        else {
            panic!("Organization");
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
        let kingdom = actions.owner(owner).unwrap().unwrap();
        assert!(hqs(&actions, kingdom).unwrap().is_empty());
        assert!(!can_recruit(&mut actions, kingdom, owner).unwrap());
        // Native4E1E40 with an empty vector never dereferences its actor argument.
        assert_eq!(
            recruit_count(&mut actions, kingdom, ActorId::from_raw(0xdeadbeef)).unwrap(),
            0
        );
        let Some(RestoredRecord::Kingdom(record)) = actions.context.registry.value_mut(kingdom)
        else {
            panic!("Kingdom");
        };
        record
            .runtime
            .as_mut()
            .unwrap()
            .derived
            .as_mut()
            .unwrap()
            .vectors
            .get_mut(&0x2dc)
            .unwrap()
            .values
            .push(owner.raw());
        assert!(
            can_recruit(&mut actions, kingdom, owner)
                .unwrap_err()
                .to_string()
                .contains("573E50")
        );
        assert!(
            recruit_count(&mut actions, kingdom, owner)
                .unwrap_err()
                .to_string()
                .contains("574BF0")
        );
        assert_eq!(hqs(&actions, kingdom).unwrap(), &[owner.raw()]);
    }
}
