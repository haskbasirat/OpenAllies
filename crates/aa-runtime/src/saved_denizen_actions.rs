//! Denizen569C70 over the actual saved-restoration component allocations.
//! Settlement presence is sufficient for4CED40; Wall data remains explicit
//! unsupported because no concrete restored Wall payload exists yet.
use crate::saved_organization_actions::{SavedActions, action_error};
use aa_actors::{ActorId, ability_flags as flags, components::ComponentSlot, denizen_actions};
use aa_mapworld::restore::RestoredComponent;

pub(crate) fn contribute(
    actions: &mut SavedActions<'_, '_, '_>,
    actor: ActorId,
) -> flags::Result<()> {
    denizen_actions::contribute(actions, actor)
}
fn denizen<'a>(
    actions: &'a SavedActions<'_, '_, '_>,
    actor: ActorId,
) -> flags::Result<&'a aa_actors::denizen_state::DenizenState> {
    match actions.component(actor, ComponentSlot::Denizen)? {
        Some(RestoredComponent::Denizen(value)) => Ok(value),
        _ => Err(flags::Error::Missing(
            "saved Denizen569C70 component payload",
        )),
    }
}
impl denizen_actions::World for SavedActions<'_, '_, '_> {
    fn denizen_owner(&self, component: ActorId) -> flags::Result<ActorId> {
        Ok(denizen(self, component)?.header.owner)
    }
    fn fields(&self, actor: ActorId) -> flags::Result<flags::ActorFields> {
        flags::World::actor_fields(self, actor)
    }
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> flags::Result<()> {
        flags::World::set_action_bits(self, actor, bits)
    }
    fn denizen_byte(&self, component: ActorId, offset: u32) -> flags::Result<u8> {
        let value = denizen(self, component)?;
        match offset {
            0x20 => Ok(value.byte_20),
            0x21 => Ok(value.byte_21),
            0x38 | 0x3c => value
                .words
                .get(&offset)
                .map(|word| *word as u8)
                .ok_or_else(|| action_error(format!("Denizen+{offset:X} is uninitialized"))),
            _ => Err(flags::Error::Missing("Denizen569C70 byte offset")),
        }
    }
    fn settlement_present(&self, actor: ActorId) -> flags::Result<bool> {
        Ok(self
            .shell(actor)?
            .components
            .present(ComponentSlot::Settlement))
    }
    fn structure_parent(&self, actor: ActorId) -> flags::Result<Option<Option<ActorId>>> {
        match self.component(actor, ComponentSlot::Structure)? {
            None => Ok(None),
            Some(RestoredComponent::Structure(value)) => Ok(Some(
                (value.object_handle_10 != 0).then_some(ActorId::from_raw(value.object_handle_10)),
            )),
            Some(_) => Err(flags::Error::Missing(
                "saved Structure+10 payload for4CED60",
            )),
        }
    }
    fn wall_reference(&self, actor: ActorId) -> flags::Result<Option<Option<ActorId>>> {
        match self.component(actor, ComponentSlot::Wall)? {
            None => Ok(None),
            Some(_) => Err(flags::Error::Missing("saved Wall+10 payload for58CB30")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        catalog::Catalog, localization::Localization, restoration::Attempt, settings::Settings,
    };
    use aa_actors::denizen_actions::World;
    use aa_mapworld::restore::{OrganizationAttachContext, RestoreMode, RestoredRecord};
    use aa_numeric::Precision;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use serde_json::{Value, json};

    #[test]
    #[ignore = "requires AA_GAME_ROOT original/v14 assets"]
    fn actual_saved_denizen_allocations_and_fields_match_native_without_substitutes() {
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
                root.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial01.RMP"),
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
            RestoreMode::InspectPartial,
            Some(1),
        )
        .unwrap();
        let candidates: Vec<_> = attempt
            .registry
            .iter()
            .filter_map(|e| {
                e.value
                    .loaded()?
                    .shell
                    .actor
                    .as_ref()?
                    .components
                    .present(ComponentSlot::Denizen)
                    .then_some(e.id)
            })
            .collect();
        assert_eq!(
            candidates.len(),
            2,
            "actual tutorial01 Denizen source count"
        );
        let evidence = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../research/skirmish-20261016-denizen-actions");
        let capture = std::env::var_os("AA_DENIZEN_CAPTURE").is_some();
        let expected: Option<Value> = (!capture).then(|| {
            serde_json::from_slice(
                &std::fs::read(evidence.join("native-saved-denizen-actions.json")).unwrap(),
            )
            .unwrap()
        });
        let mut inputs = vec![];
        for actor in candidates {
            let taken = std::mem::replace(
                attempt.registry.value_mut(actor).unwrap(),
                RestoredRecord::Loading,
            );
            let RestoredRecord::Actor(mut entity) = taken else {
                panic!("Denizen Actor")
            };
            let shell = entity.shell.actor.as_mut().unwrap();
            // The attach-only adapter context requires an Organization borrow.
            // This unused explicit context fixture is never queried here and
            // does not replace the actual saved Denizen under test.
            let mut unused = aa_actors::ground_states::OrganizationState::<String>::new(
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
                    organization: &mut unused,
                    registry: &mut attempt.registry,
                    precision: Precision::Double53,
                }
                .into(),
                active_mind: None,
            };
            let fields = actions.fields(actor).unwrap();
            let original = denizen(&actions, actor).unwrap();
            assert_eq!(
                original.header.owner, actor,
                "actual Denizen owner identity"
            );
            let allocation = original as *const _;
            let structure = actions.structure_parent(actor).unwrap();
            assert!(
                !actions
                    .shell(actor)
                    .unwrap()
                    .components
                    .present(ComponentSlot::Wall)
            );
            let case = json!({"name":format!("tutorial01_{:x}",actor.raw()),"source":actions.source_id(actor).unwrap(),"state":fields.state_bits,"actions":fields.action_bits,"settlement":actions.settlement_present(actor).unwrap(),"structure":structure.is_some(),"parent":structure.flatten().is_some(),"wall":false,"wall_target":false,"byte20":original.byte_20,"byte21":original.byte_21,"flags38":original.words[&0x38],"flags3c":original.words[&0x3c]});
            inputs.push(case.clone());
            contribute(&mut actions, actor).unwrap();
            assert_eq!(
                denizen(&actions, actor).unwrap() as *const _,
                allocation,
                "same Denizen allocation"
            );
            if let Some(expected) = &expected {
                let rows: Vec<_> = expected["cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|row| row["input"]["name"] == case["name"])
                    .collect();
                assert_eq!(rows.len(), 6);
                for row in rows {
                    assert_eq!(
                        row["input"], case,
                        "actual saved fields changed since oracle capture"
                    );
                    assert_eq!(
                        actions.fields(actor).unwrap().action_bits,
                        row["output"]["actions"].as_u64().unwrap()
                    );
                }
            }
        }
        if capture {
            std::fs::write(
                evidence.join("actual-saved-inputs.json"),
                serde_json::to_vec_pretty(&inputs).unwrap(),
            )
            .unwrap();
        }
    }
}
