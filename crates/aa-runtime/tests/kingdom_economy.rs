use aa_actors::ActorId;
use aa_mapworld::special_state::{NativeVectorState, SpecialShell};
use aa_runtime::kingdom_economy::{Definitions, reset_flow_vectors};

#[test]
fn flow_reset_matches_original_index_order_growth_and_preserved_suffix() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-runtime/native-kingdom-flow-init.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let mut state = SpecialShell::kingdom(ActorId::from_raw(1), 7).unwrap();
        state.vectors.get_mut(&0x1c4).unwrap().values[0] = 123.0f32.to_bits();
        for (index, offset) in [0x1a0, 0x1ac, 0x1b8].iter().enumerate() {
            let before = &case["before"][index];
            state.vectors.insert(
                *offset,
                NativeVectorState {
                    values: before["values"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_u64().unwrap() as u32)
                        .collect(),
                    capacity_hint: before["capacity"].as_u64().unwrap() as u32,
                },
            );
        }
        reset_flow_vectors(&mut state, case["resource_count"].as_i64().unwrap() as i32).unwrap();
        for (index, offset) in [0x1a0, 0x1ac, 0x1b8].iter().enumerate() {
            let value = &state.vectors[offset];
            assert_eq!(
                serde_json::json!({"values":value.values,"capacity":value.capacity_hint}),
                case["after"][index]
            );
        }
        assert_eq!(state.vectors[&0x1c4].values[0], 123.0f32.to_bits());
    }
}
#[test]
fn reset_failure_preserves_prior_index_write_and_later_vectors() {
    let mut state = SpecialShell::kingdom(ActorId::from_raw(1), 7).unwrap();
    state.vectors.get_mut(&0x1a0).unwrap().values = vec![7, 8];
    state.vectors.remove(&0x1ac);
    state.vectors.get_mut(&0x1b8).unwrap().values = vec![9, 10];
    assert!(reset_flow_vectors(&mut state, 2).is_err());
    assert_eq!(state.vectors[&0x1a0].values, vec![0, 8]);
    assert_eq!(state.vectors[&0x1b8].values, vec![9, 10]);
}
#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 source assets"]
fn source_tutorial_kingdoms_transfer_resources_without_second_mutable_copy() {
    use aa_numeric::Precision;
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use aa_runtime::{localization::Localization, restoration::Attempt, settings::Settings};
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut source = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let resources = ResourceCatalog::load_v14(&mut source).unwrap();
    let settings = Settings::load_v14(&mut source, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut source, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut source).unwrap();
    let defs = Definitions::bind(&resources, &actors, Precision::Double53).unwrap();
    let mut mismatched = resources.clone();
    mismatched
        .sources
        .iter_mut()
        .find(|s| s.path.eq_ignore_ascii_case("Game/resources.tgi"))
        .unwrap()
        .sha256 = "00".repeat(32);
    assert!(
        Definitions::bind(&mismatched, &actors, Precision::Double53).is_err(),
        "equal IDs cannot combine resource definitions from different source snapshots"
    );
    assert_eq!(
        defs.catalog()
            .resources
            .iter()
            .map(|r| r.id.clone())
            .collect::<Vec<_>>(),
        actors
            .native_resource_registry(Precision::Double53)
            .unwrap()
            .ids
    );
    for name in ["tutorial01", "tutorial02"] {
        let map = aa_mapworld::MapArchive::from_bytes(
            std::fs::read(game.join(format!(
                "reconstruction/assets/original/Data/Maps/Campaign/{name}.RMP"
            )))
            .unwrap(),
        )
        .unwrap();
        let mut attempt = Attempt::load(
            &map,
            &actors,
            settings.clone(),
            text.clone(),
            Precision::Double53,
            u64::MAX,
        )
        .unwrap();
        let ids = attempt
            .registry
            .iter()
            .filter(|v| matches!(v.value, aa_mapworld::restore::RestoredRecord::Kingdom(_)))
            .map(|v| v.id)
            .collect::<Vec<_>>();
        assert_eq!(ids.len(), 3);
        for id in ids {
            let aa_mapworld::restore::RestoredRecord::Kingdom(kingdom) =
                attempt.registry.value_mut(id).unwrap()
            else {
                unreachable!()
            };
            assert_eq!(
                kingdom.completion,
                aa_mapworld::restore::LoadCompletion::AwaitingPostLoad
            );
            let state = kingdom.runtime.as_mut().unwrap().derived.as_mut().unwrap();
            let stock = state.vectors[&0x1c4].values.clone();
            assert!(
                defs.take_completed_arrays(state).is_err(),
                "load ctor flow arrays are not live economy state"
            );
            reset_flow_vectors(state, defs.resource_count() as i32).unwrap();
            let mut owned = defs.take_completed_arrays(state).unwrap();
            assert_eq!(
                owned
                    .economy
                    .state()
                    .stockpiled
                    .iter()
                    .map(|v| v.0)
                    .collect::<Vec<_>>(),
                stock
            );
            for offset in [0x1a0, 0x1ac, 0x1b8, 0x1c4] {
                assert!(!state.vectors.contains_key(&offset));
            }
            assert!(
                defs.take_completed_arrays(state).is_err(),
                "ownership cannot transfer twice"
            );
            let mut production = vec![aa_economy::F32Bits::ZERO; defs.resource_count()];
            let money = defs
                .catalog()
                .resources
                .iter()
                .position(|r| r.id == "money")
                .unwrap();
            production[money] = aa_economy::F32Bits(10.0f32.to_bits());
            owned.economy.adjust_providers(&production, true).unwrap();
            assert_eq!(owned.economy.state().raw_provided[money], production[money]);
            assert_eq!(
                owned
                    .economy
                    .state()
                    .stockpiled
                    .iter()
                    .map(|v| v.0)
                    .collect::<Vec<_>>(),
                stock
            );
        }
    }
}
