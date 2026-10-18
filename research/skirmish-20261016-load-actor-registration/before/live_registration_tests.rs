use super::*;
use crate::{
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use std::{path::PathBuf, sync::Arc};

struct RequiredEffects {
    calls: usize,
}
impl Effects for RequiredEffects {
    fn register_actor(
        &mut self,
        _: ActorId,
        _: &mut RestoredRecord,
        _: &IdentityRegistry<RestoredRecord>,
        _: &Catalog<'_>,
        _: Precision,
    ) -> Result<()> {
        self.calls += 1;
        Err("4D62F0 actual state/grid service is unbound".into())
    }
    fn assert_duplicate_available_city(&mut self) -> Result<bool> {
        Err("855D30 assertion service is unbound".into())
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT recovered data"]
fn actual_saved_records_mutate_owned_fields_and_require_real_actor_service() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut src = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut src, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut src, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut src).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut src).unwrap());
    let token = ScopedManagedLoad::from_sources(&mut src, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(root.join("reconstruction/assets/original/Data/Maps/(2v2)Kursk.RMP"))
            .unwrap(),
    )
    .unwrap();
    let mut attempt = Attempt::load_with_bindings(
        &map,
        &actors,
        settings,
        text,
        Precision::Double53,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::InspectPartial,
        LoadBindings {
            native_load_mode: Some(1),
            inactive_load: Some(token),
            resources: Some(resources),
        },
    )
    .unwrap();
    let catalog = Catalog::new(&actors);
    let ids: Vec<_> = attempt
        .registry
        .iter()
        .filter_map(|e| definition(&e.value).ok().map(|d| (e.id, d.native_class)))
        .filter(|(_, class)| matches!(class, 3 | 4 | 5 | 6 | 7))
        .collect();
    assert!(ids.iter().any(|(_, c)| *c == 3));
    assert!(ids.iter().any(|(_, c)| *c == 5));
    let mut seen = std::collections::BTreeSet::new();
    let mut published = Vec::new();
    for (id, class) in ids {
        if !seen.insert(class) {
            continue;
        }
        let mut record = std::mem::replace(
            attempt.registry.value_mut(id).unwrap(),
            RestoredRecord::Loading,
        );
        let address = shell(&record).unwrap() as *const _;
        let original_flags = shell(&record).unwrap().thing.flags;
        let original_name = shell(&record).unwrap().thing.name_utf16.clone();
        let slot = if let RestoredRecord::Team(team) = &record {
            Some(
                *team
                    .runtime
                    .as_ref()
                    .unwrap()
                    .derived
                    .as_ref()
                    .unwrap()
                    .words
                    .get(&0x38)
                    .unwrap() as usize,
            )
        } else {
            None
        };
        let mut state = State::from_world_constructor(vec![None; slot.map_or(0, |s| s + 1)], false);
        let mut effects = RequiredEffects { calls: 0 };
        let result = register_restored(
            &mut state,
            id,
            &mut record,
            &attempt.registry,
            &catalog,
            &mut effects,
            Precision::Double53,
        );
        assert_eq!(shell(&record).unwrap() as *const _, address);
        assert_eq!(
            shell(&record).unwrap().thing.flags,
            original_flags & 0x7fff_ffff
        );
        if !original_name.is_empty() {
            assert_eq!(state.names().lookup(&original_name).unwrap(), Some(id));
        }
        match class {
            3 | 4 => {
                assert!(result.unwrap_err().contains("4D62F0"));
                assert_eq!(effects.calls, 1);
                assert!(state.available_cities().is_empty());
            }
            5 => {
                result.unwrap();
                assert_eq!(state.teams()[slot.unwrap()], Some(id));
                assert_eq!(effects.calls, 0);
            }
            6 => {
                result.unwrap();
                assert_eq!(state.areas(), [id]);
            }
            7 => {
                result.unwrap();
                assert_eq!(state.cameras(), [id]);
            }
            _ => unreachable!(),
        }
        // Source access is lazy and checks the actual Catalog owner only when
        // ObjectData+34 is reached. Team needs no ObjectData source at all.
        let other = actors.clone();
        let mut changed_catalog = Catalog::new(&actors);
        changed_catalog.actors = &other;
        let mut badstate =
            State::from_world_constructor(vec![None; slot.map_or(0, |s| s + 1)], true);
        let mut effects = RequiredEffects { calls: 0 };
        let result = register_restored(
            &mut badstate,
            id,
            &mut record,
            &attempt.registry,
            &changed_catalog,
            &mut effects,
            Precision::Double53,
        );
        if class == 5 {
            result.unwrap();
        } else if matches!(class, 3 | 4) {
            assert!(result.unwrap_err().contains("4D62F0"));
        } else {
            assert!(result.unwrap_err().contains("source owner changed"));
        }
        assert_eq!(
            shell(&record).unwrap().thing.flags,
            original_flags & 0x7fff_ffff
        );
        if matches!(class, 3 | 4) {
            // Independently exercise the reached-field adapters on this actual
            // saved allocation, without pretending the missing4D62F0 succeeded.
            let old_required = shell(&record).unwrap().actor.as_ref().unwrap().words[&0x158];
            for pc in [
                Precision::Single24,
                Precision::Double53,
                Precision::Extended64,
            ] {
                let mut effects = RequiredEffects { calls: 0 };
                let mut world = Restored {
                    id,
                    record: &mut record,
                    registry: &attempt.registry,
                    catalog: &catalog,
                    effects: &mut effects,
                    pc,
                };
                let space = world.settlement_space().unwrap();
                assert_eq!(space, settlement_space(&catalog, world.record, pc).unwrap());
                let city = world.city_present().unwrap();
                assert_eq!(
                    city,
                    shell(world.record)
                        .unwrap()
                        .actor
                        .as_ref()
                        .unwrap()
                        .components
                        .present(ComponentSlot::City)
                );
                for reference in [id, ActorId::from_raw(id.raw() ^ 0x10000), ActorId::NULL] {
                    world.record.loaded_mut().map(|r| {
                        r.shell
                            .actor
                            .as_mut()
                            .unwrap()
                            .words
                            .insert(0x158, reference.raw())
                    });
                    if let RestoredRecord::Kingdom(kingdom) = world.record {
                        kingdom
                            .runtime
                            .as_mut()
                            .unwrap()
                            .base
                            .actor
                            .as_mut()
                            .unwrap()
                            .words
                            .insert(0x158, reference.raw());
                    }
                    assert_eq!(world.required_city_live().unwrap(), reference == id);
                    if reference == id {
                        assert_eq!(world.required_city_settlement_space().unwrap(), space);
                    }
                }
                if let Some(loaded) = world.record.loaded_mut() {
                    loaded
                        .shell
                        .actor
                        .as_mut()
                        .unwrap()
                        .words
                        .insert(0x158, old_required);
                }
                if let RestoredRecord::Kingdom(kingdom) = world.record {
                    kingdom
                        .runtime
                        .as_mut()
                        .unwrap()
                        .base
                        .actor
                        .as_mut()
                        .unwrap()
                        .words
                        .insert(0x158, old_required);
                }
            }
            let mut effects = RequiredEffects { calls: 0 };
            let mut world = Restored {
                id,
                record: &mut record,
                registry: &attempt.registry,
                catalog: &changed_catalog,
                effects: &mut effects,
                pc: Precision::Double53,
            };
            assert!(
                world
                    .settlement_space()
                    .unwrap_err()
                    .contains("source owner changed")
            );
        }
        // The held owner never becomes a copied view of the Loading placeholder.
        assert!(matches!(
            attempt.registry.get(id).unwrap().value,
            RestoredRecord::Loading
        ));
        let completion = record.completion();
        *attempt.registry.value_mut(id).unwrap() = record;
        published.push((id, class, state, original_name, completion));
    }
    eprintln!("actual saved native classes checked: {seen:?}");
    let store = crate::actor_world::ActorStore::from_restored(attempt.registry).unwrap();
    for (id, class, mut state, name, completion) in published {
        let key = store.key(id).unwrap();
        let view = state.stored(&store);
        assert_eq!(view.key(id).unwrap(), key);
        if !name.is_empty() {
            assert_eq!(view.named(&name).unwrap(), Some(key));
        }
        match class {
            5 => assert!(
                state
                    .teams()
                    .iter()
                    .enumerate()
                    .any(|(slot, team)| *team == Some(id) && view.team(slot).unwrap() == Some(key))
            ),
            6 => assert_eq!(view.areas().collect::<Result<Vec<_>>>().unwrap(), [key]),
            7 => assert_eq!(view.cameras().collect::<Result<Vec<_>>>().unwrap(), [key]),
            _ => (),
        }
        // Staging moves ownership without promoting partially loaded records.
        let staged_completion = match &store.entity(key).unwrap().source {
            crate::actor_world::Source::Loaded(v) => v.completion,
            crate::actor_world::Source::Special(v) => v.completion,
            _ => panic!("expected the moved saved source"),
        };
        assert_eq!(staged_completion, completion);
        let stale = ActorId::from_raw(id.raw() ^ 0x10000);
        assert!(view.key(stale).is_err());
        let stale_name: Vec<_> = "__stale_registration_probe".encode_utf16().collect();
        state.names_mut().alias(&stale_name, stale).unwrap();
        assert_eq!(state.stored(&store).named(&stale_name).unwrap(), None);
    }
}
