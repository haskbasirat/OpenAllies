use aa_actors::{ActorId, IdentityRegistry, components::Components};
use aa_body::F32;
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use aa_runtime::{
    actor_world::ActorStore,
    catalog::Catalog,
    definitions::Bank,
    saved_performer::{self, AbilityArena},
};
use std::collections::BTreeMap;
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 sources"]
fn actual_source_saved_constructor_arena_moves_without_reinitializing() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let catalog = Catalog::new(&actors);
    let pc = Precision::Double53;
    let mut bank = Bank::default();
    let mut arena = AbilityArena::new();
    let components = Components::new(Default::default());
    let mut handles = Vec::new();
    for (index, name) in [
        "usa_infantry",
        "gbr_infantry",
        "ger_infantry",
        "jpn_infantry",
        "rus_infantry",
    ]
    .iter()
    .enumerate()
    {
        let binding = catalog.actor_bindings(name, pc).unwrap();
        let mut flags = binding.definition.flags;
        let mut words = BTreeMap::from([(0xf0, 0), (0xf4, 0)]);
        let performer = saved_performer::construct(
            &catalog,
            &mut bank,
            &mut arena,
            ActorId::from_raw(index as u32 + 1),
            name,
            &components,
            &mut flags,
            &mut words,
            saved_performer::SavedShell {
                object_byte_4c: 0,
                thing_word_c: 0,
            },
            pc,
        )
        .unwrap();
        assert_eq!(
            performer.state.abilities.len(),
            binding.performer.as_ref().unwrap().abilities().len()
        );
        assert_eq!(words[&0xf0], 0);
        assert_eq!(words[&0xf4], 0);
        for handle in performer.state.abilities {
            let ability = arena.get(handle).unwrap();
            bank.entry(ability.definition).unwrap();
            bank.entry(ability.local_definition.unwrap()).unwrap();
            arena
                .apply_saved(
                    handle,
                    &aa_combat::commander::SavedAbility {
                        version: 1,
                        use_count: 2,
                        last_use: F32(3f32.to_bits()),
                        word14: 77,
                        enabled_byte: 0,
                        summoned: Some(aa_combat::commander::SavedWords {
                            serialized_count: 2,
                            values: vec![10, 11],
                        }),
                    },
                )
                .unwrap();
            handles.push(handle);
        }
    }
    let first = *handles.first().unwrap();
    let saved = arena.get(first).unwrap();
    assert_eq!(saved.use_count, 2);
    assert_eq!(saved.summoned_ids, vec![11, 10]);
    assert!(!saved.enabled);
    let next = arena.next_handle();
    let mut store = ActorStore::from_restored_with_abilities(IdentityRegistry::new(), arena).unwrap();
    for handle in handles {
        let a = &store.abilities[&handle];
        assert_eq!(a.use_count, 2);
        assert_eq!(a.word14, 77);
        assert!(!a.enabled);
        assert_eq!(a.last_use, F32(3f32.to_bits()));
    }
    let new = store
        .allocate_ability(aa_combat::ability::Ability::new(1, None, F32::ZERO))
        .unwrap();
    assert_eq!(new, next);
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 sources"]
fn constructor_rebuild_clears_prior_native_mask_and_publishes() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let catalog = Catalog::new(&actors);
    let pc = Precision::Double53;
    let mut bank = Bank::default();
    let mut arena = AbilityArena::new();
    let mut flags = 0;
    let mut words = BTreeMap::from([(0xf0, 1), (0xf4, 0)]);
    let result = saved_performer::construct(
        &catalog,
        &mut bank,
        &mut arena,
        ActorId::from_raw(1),
        "usa_infantry",
        &Components::new(Default::default()),
        &mut flags,
        &mut words,
        saved_performer::SavedShell {
            object_byte_4c: 0,
            thing_word_c: 0,
        },
        pc,
    );
    // Phase8: the saved-factory 4CFCC0 publication is bound (no Element, no
    // owner word, Object+4C zero, no renderer link), so the cleared prior
    // mask publishes without any live service and construction completes.
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(arena.entries().len(), 2);
    assert_eq!(arena.next_handle(), 3);
    assert_eq!(words[&0xf0], 0);
    assert_eq!(words[&0xf4], 0);
}



