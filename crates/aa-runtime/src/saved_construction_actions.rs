//! Saved-restore bridge to the existing native565140/565090 port. This is
//! separate from the early Performer-constructor adapter: later Structure
//! and Properties payloads are now read from their authoritative owner.
use crate::saved_organization_actions::{SavedActions, action_error};
use aa_actors::{ActorId, ability_flags as flags, components::ComponentSlot, construction_actions};
use aa_mapworld::restore::RestoredComponent;

pub(crate) fn contribute(
    actions: &mut SavedActions<'_, '_, '_>,
    actor: ActorId,
) -> flags::Result<()> {
    // The proven pure port uses a stable owner identity. No callback below
    // mutates this component; reject mismatched ownership before using it.
    match actions.component(actor, ComponentSlot::Construction)? {
        Some(RestoredComponent::Construction(value)) if value.header.owner == actor => {}
        _ => {
            return Err(flags::Error::Missing(
                "saved Construction565140 owner/payload",
            ));
        }
    }
    construction_actions::contribute(actions, actor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        catalog::Catalog, localization::Localization, restoration::Attempt, settings::Settings,
    };
    use aa_actors::construction_actions::World;
    use aa_mapworld::restore::{OrganizationAttachContext, RestoreMode, RestoredRecord};
    use aa_numeric::Precision;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };

    #[test]
    #[ignore = "requires AA_GAME_ROOT original/v14 assets"]
    fn actual_skirmish_construction_source_masks_and_partial_error_prefix() {
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
            settings.clone(),
            text,
            Precision::Double53,
            u64::MAX,
            RestoreMode::StopOnError,
            Some(1),
        )
        .unwrap();
        assert!(attempt.report.stopped_before.is_none());
        let actor = attempt
            .registry
            .iter()
            .find(|entry| {
                entry
                    .value
                    .loaded()
                    .and_then(|e| e.shell.actor.as_ref())
                    .is_some_and(|a| a.components.present(ComponentSlot::Construction))
            })
            .unwrap()
            .id;
        let taken = std::mem::replace(
            attempt.registry.value_mut(actor).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Actor(mut entity) = taken else {
            panic!("Construction actor")
        };
        let id = entity.definition.proof.id.clone();
        let source = actors
            .native_construction_flags(&id, Precision::Double53)
            .unwrap()
            .unwrap();
        assert_eq!(source.upgrade_count, 0);
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(settings);
        // Only context plumbing; no Organization query is reached. The actual
        // Construction/Structure/Properties payloads stay owned and un-cloned.
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
        let shell = entity.shell.actor.as_mut().unwrap();
        let context = OrganizationAttachContext {
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
        };
        let mut actions = SavedActions {
            catalog: &catalog,
            context: context.into(),
            active_mind: None,
        };
        assert_eq!(
            actions.definition(actor).unwrap(),
            construction_actions::Definition {
                can_sell: source.can_sell,
                can_raze: source.can_raze,
                upgrade_count: 0
            }
        );
        assert!(
            !actions.allow_sell_under_siege().unwrap(),
            "SVars_AA overrides SVars true with false"
        );
        assert_eq!(actions.structure_parent(actor).unwrap(), Some(None));
        let properties = actions.properties_flags(actor).unwrap();
        assert!(properties.is_some(), "use real restored Properties");
        println!("source actor={id} flags={source:?} properties={properties:?}");
        let ptr = actions
            .component(actor, ComponentSlot::Construction)
            .unwrap()
            .unwrap() as *const _;
        let initial = u64::MAX;
        // Explicit actor state variations; source definitions and components
        // remain real. The native oracle covers these branch outcomes.
        for state in [
            1u64 << 27,
            (1 << 27) | (1 << 20),
            (1 << 27) | (1 << 21),
            1 << 29,
        ] {
            let shell = actions.shell_mut(actor).unwrap();
            shell.components.state.low = state as u32;
            shell.components.state.high = (state >> 32) as u32;
            flags::World::set_action_bits(&mut actions, actor, initial).unwrap();
            contribute(&mut actions, actor).unwrap();
            let mask = flags::World::actor_fields(&actions, actor)
                .unwrap()
                .action_bits;
            let outside = !((1u64 << 19) | (1 << 20) | (1 << 21) | (1 << 22));
            assert_eq!(mask & outside, initial & outside);
            assert_eq!(mask & (1 << 21), 0, "source-proven empty Upgrade graph");
            if state & (1 << 21) != 0 || state == 1 << 29 {
                assert_ne!(mask & (1 << 19), 0);
                assert_eq!(mask & ((1 << 20) | (1 << 22)), 0);
            } else {
                assert_eq!(mask & (1 << 19), 0);
                let unblocked = properties.unwrap() & 0x800 == 0 && state & (1 << 20) == 0;
                assert_eq!(mask & (1 << 20) != 0, source.can_sell && unblocked);
                assert_eq!(mask & (1 << 22) != 0, source.can_raze && unblocked);
            }
        }
        assert_eq!(
            actions
                .component(actor, ComponentSlot::Construction)
                .unwrap()
                .unwrap() as *const _,
            ptr
        );
        // Loss of the required Settings dependency occurs after bit21/22/19
        // writes. Preserve that prefix, including old bit20, without rollback.
        let shell = actions.shell_mut(actor).unwrap();
        shell.components.state.low = (1u32 << 27) | (1 << 20);
        shell.components.state.high = 0;
        flags::World::set_action_bits(&mut actions, actor, initial).unwrap();
        let context = actions.context;
        catalog.settings = None;
        let mut actions = SavedActions {
            catalog: &catalog,
            context: context.into(),
            active_mind: None,
        };
        let error = contribute(&mut actions, actor).unwrap_err();
        assert!(error.to_string().contains("Settings"), "{error}");
        assert_eq!(
            flags::World::actor_fields(&actions, actor)
                .unwrap()
                .action_bits,
            initial & !((1 << 21) | (1 << 22) | (1 << 19))
        );
        // State21 early return does not request the unavailable Settings.
        actions.shell_mut(actor).unwrap().components.state.low |= 1 << 21;
        contribute(&mut actions, actor).unwrap();
        assert_eq!(
            flags::World::actor_fields(&actions, actor)
                .unwrap()
                .action_bits,
            (initial & !((1 << 21) | (1 << 22) | (1 << 20))) | (1 << 19)
        );
    }
}

impl construction_actions::World for SavedActions<'_, '_, '_> {
    fn fields(&self, actor: ActorId) -> flags::Result<flags::ActorFields> {
        flags::World::actor_fields(self, actor)
    }
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> flags::Result<()> {
        flags::World::set_action_bits(self, actor, bits)
    }
    fn definition(&self, actor: ActorId) -> flags::Result<construction_actions::Definition> {
        let id = self.source_id(actor)?;
        self.catalog
            .require_untranslated_actor_identity(id)
            .map_err(action_error)?;
        let value = self
            .catalog
            .actors
            .native_construction_flags(id, self.source_precision()?)
            .map_err(action_error)?
            .ok_or(flags::Error::Missing("saved Construction ActorData inputs"))?;
        Ok(construction_actions::Definition {
            can_sell: value.can_sell,
            can_raze: value.can_raze,
            upgrade_count: value.upgrade_count,
        })
    }
    fn structure_parent(&self, actor: ActorId) -> flags::Result<Option<Option<ActorId>>> {
        match self.component(actor, ComponentSlot::Structure)? {
            None => Ok(None),
            Some(RestoredComponent::Structure(value)) if value.header.owner == actor => Ok(Some(
                (value.object_handle_10 != 0).then_some(ActorId::from_raw(value.object_handle_10)),
            )),
            // 588990 compares Settlement+14 with Structure.header.owner.
            // The existing pure port assumes that owner is the queried actor.
            _ => Err(flags::Error::Missing(
                "saved Construction Structure owner/payload",
            )),
        }
    }
    fn wall_reference(&self, actor: ActorId) -> flags::Result<Option<Option<ActorId>>> {
        match self.component(actor, ComponentSlot::Wall)? {
            None => Ok(None),
            Some(_) => Err(flags::Error::Missing(
                "saved Wall+10 payload for Construction",
            )),
        }
    }
    fn settlement_reference(&self, actor: ActorId) -> flags::Result<Option<Option<ActorId>>> {
        match self.component(actor, ComponentSlot::Settlement)? {
            None => Ok(None),
            Some(_) => Err(flags::Error::Missing(
                "saved Settlement+14 payload for Construction",
            )),
        }
    }
    fn properties_flags(&self, actor: ActorId) -> flags::Result<Option<u32>> {
        match self.component(actor, ComponentSlot::Properties)? {
            None => Ok(None),
            Some(RestoredComponent::Properties(value)) if value.owner.0 == actor.raw() => {
                Ok(Some(value.modifier.flags))
            }
            _ => Err(flags::Error::Missing(
                "saved Construction Properties owner/payload",
            )),
        }
    }
    fn allow_sell_under_siege(&mut self) -> flags::Result<bool> {
        // 474C50/512200 bind WorldRule values to SVars by definition IDS.
        // The current original/v14 registry has no rule for this variable.
        // Refuse a changed source graph rather than ignoring its saved value.
        let rules = self
            .catalog
            .actors
            .native_data_registry("WorldRule")
            .ok_or(flags::Error::Missing(
                "saved Construction WorldRule registry",
            ))?;
        if rules
            .entries
            .iter()
            .any(|rule| rule.ids.eq_ignore_ascii_case("AllowSellUnderSiege"))
        {
            return Err(flags::Error::Missing(
                "AllowSellUnderSiege WorldRule override is unbound",
            ));
        }
        self.catalog
            .settings
            .as_ref()
            .ok_or(flags::Error::Missing(
                "saved Construction effective source Settings",
            ))?
            .allow_sell_under_siege()
            .map(|(value, _)| value)
            .map_err(action_error)
    }
    fn settlement_upgrades_ready(&mut self, _: ActorId) -> flags::Result<bool> {
        Err(flags::Error::Missing(
            "585F00 saved Settlement upgrade readiness",
        ))
    }
}
