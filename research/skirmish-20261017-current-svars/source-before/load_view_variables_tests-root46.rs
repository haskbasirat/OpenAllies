use super::*;
use serde_json::{Value, json};

#[test]
fn original_v14_all_pc_reset_publication_and_live_assignment_words() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-view-variables/native-view-variables.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 141);
    for row in rows {
        let c = &row["input"];
        let expected = &row["output"];
        let word = |key: &str| c[key].as_u64().unwrap() as u32;
        let mut current =
            Current::source_reset(word("source_default") as i32, word("fast_source") as u8);
        assert_eq!(
            json!({"fog":current.fog_of_war as u32,"cheat":current.cheat_fog,"fast":current.fast_detection_zones}),
            expected["reset"],
            "{row}"
        );
        current.assign_fog(word("rule_value") as i32);
        assert_eq!(
            current.fog_of_war as u32,
            expected["published_fog"].as_u64().unwrap() as u32,
            "{row}"
        );
        current.assign_fog(word("later_fog") as i32);
        current.assign_cheat(word("later_cheat") as u8);
        current.assign_fast(word("later_fast") as u8);
        assert_eq!(
            current.fog_of_war as u32,
            expected["current_fog"].as_u64().unwrap() as u32,
            "{row}"
        );
        assert_eq!(
            current.cheat_fog,
            expected["current_cheat"].as_u64().unwrap() as u8,
            "{row}"
        );
        assert_eq!(
            current.fast_detection_zones,
            expected["current_fast"].as_u64().unwrap() as u8,
            "{row}"
        );
        assert_eq!(
            current.fog_of_war != 0,
            expected["fog_enabled"].as_bool().unwrap(),
            "{row}"
        );
        assert_eq!(
            current.cheat_fog != 0,
            expected["cheat_enabled"].as_bool().unwrap(),
            "{row}"
        );
        assert_eq!(
            current.fast_detection_zones != 0,
            expected["fast_enabled"].as_bool().unwrap(),
            "{row}"
        );
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT actual v14 sources and sealed reference source inputs"]
fn actual_source_reset_standalone_rules_and_same_mutable_owner() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(
        crate::settings::Settings::load_v14(&mut sources, aa_rules::tgi::Limits::default())
            .unwrap(),
    );
    assert!(
        State::reset_from_fresh_source(&catalog, NativeOverrides::Unbound)
            .err()
            .unwrap()
            .contains("override")
    );
    let proof = crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
        &catalog,
        &mut sources,
    )
    .unwrap();
    crate::combat_source_completion::install(
        &catalog,
        &mut sources,
        proof.context(),
        proof.csv(),
        proof.globals(),
    )
    .unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    catalog.native_load_mode = Some(1);
    let mut state =
        State::reset_from_fresh_source(&catalog, NativeOverrides::ControlledNone).unwrap();
    assert_eq!(
        state.fog_integer(),
        1,
        "WorldRule default is applied BEFORE snapshot/reset, not sourceTGI2"
    );
    assert!(!state.cheat_fog_enabled());
    assert!(state.fast_detection_zones());
    assert_eq!(state.stockpile_default_bits(), 500f32.to_bits());
    assert_eq!(state.stockpile_maximum_bits(), 99999f32.to_bits());
    assert_eq!(state.city_victory_ratio(), 0);
    assert_eq!(state.starting_experience_bits(),200f32.to_bits());
    assert!(state.supply_provide_under_siege(),"actual AA untyped assignment updates the source Bool to true");
    assert_eq!(state.supply_recharge_rate_bits(),1f32.to_bits());
    state.assign_supply_provide_under_siege(0x80);
    state.assign_supply_recharge_rate_bits(0x7f800001);
    assert!(state.require_published().is_err());
    assert_eq!(state.constructor_game_speed(&catalog).unwrap(),
        catalog.settings.as_ref().unwrap().unique_fixed("GameSpeed",Precision::Double53).unwrap().0);
    for name in [
        "Campaign/tutorial02.RMP",
        "DeveloperMaps/(3v3)Firefight.RMP",
        "(2v2)Berlin.RMP",
    ] {
        let map = aa_mapworld::MapArchive::from_bytes(
            std::fs::read(
                game.join("reconstruction/assets/original/Data/Maps")
                    .join(name),
            )
            .unwrap(),
        )
        .unwrap();
        let mut inventory = WorldInventory::decode(&map, u64::MAX).unwrap();
        let expected =
            saved_action_settings::published_fog_integer(&catalog, &inventory, Precision::Double53)
                .unwrap();
        let previous_max = state.stockpile_maximum_bits();
        state.publish_world_rules(&catalog, &inventory).unwrap();
        let mut loaded=crate::load_world_rules::Owner::load(&catalog,&inventory).unwrap();
        let expected_experience=loaded.record("StartingExperience").unwrap().value();
        assert_eq!(state.starting_experience_bits(),expected_experience);
        //512200 visits the SAME retained Rules. This fixed/flags0 setter keeps
        //the raw payload; no source re-read or UI editbox clamp is substituted.
        loaded.assign_word("StartingExperience",2,0x7f800001).unwrap();
        state.publish_loaded_rules(&catalog,&loaded).unwrap();
        assert_eq!(state.starting_experience_bits(),0x7f800001);
        assert_eq!(
            state.stockpile_maximum_bits(),
            previous_max,
            "no registered Max WorldRule must leave the current Var untouched"
        );
        state.require_published().unwrap();
        assert!(state.supply_provide_under_siege(),"unregistered current Supply Bool is not reset by WorldRules");
        assert_eq!(state.supply_recharge_rate_bits(),0x7f800001,
            "unregistered current Supply Fixed cell preserves the raw payload through publication");
        assert!(state.constructor_game_speed(&catalog).unwrap_err().contains("after Rule publication"));
        assert_eq!(state.fog_integer(), expected);
        state.assign_fog_integer(-9);
        assert!(!state.fog_enabled());
        state.assign_fog_integer(9);
        assert_eq!(state.fog_integer(), 2);
        state.assign_cheat_fog(128);
        assert!(state.cheat_fog_enabled());
        assert_eq!(*state.cheat_fog_word(), 128);
        state.assign_fast_detection_zones(0);
        assert!(!state.fast_detection_zones());
        state.assign_stockpile_bits(0x7f800001);
        assert_eq!(
            state.stockpile_default_bits(),
            0x7f800001,
            "flags0 keeps the signalling-NaN payload as a raw DWORD"
        );
        state.assign_city_victory_ratio(i32::MIN);
        assert_eq!(
            state.city_victory_ratio(),
            i32::MIN,
            "native flags0 has no UI slider clamp"
        );
        state.assign_stockpile_maximum_bits(1234f32.to_bits());
        // A corrupt actual source must fail before changing any current words.
        let before = (
            state.fog_integer(),
            state.cheat_fog_byte(),
            state.fast_detection_zones_byte(),
            state.stockpile_default_bits(),
            state.city_victory_ratio(),
            state.stockpile_maximum_bits(),
        );
        let rules = inventory
            .opaque_initialization
            .iter_mut()
            .find(|s| s.name.eq_ignore_ascii_case("RULES"))
            .unwrap();
        rules.bytes[0] ^= 1;
        assert!(state.publish_world_rules(&catalog, &inventory).is_err());
        assert_eq!(
            (
                state.fog_integer(),
                state.cheat_fog_byte(),
                state.fast_detection_zones_byte(),
                state.stockpile_default_bits(),
                state.city_victory_ratio(),
                state.stockpile_maximum_bits()
            ),
            before
        );
    }
    catalog
        .settings
        .as_mut()
        .unwrap()
        .declarations
        .get_mut("fogofwar")
        .unwrap()[0]
        .raw = "0".into();
    assert!(
        State::reset_from_fresh_source(&catalog, NativeOverrides::ControlledNone)
            .err()
            .unwrap()
            .contains("private registered source")
    );
    catalog
        .settings
        .as_mut()
        .unwrap()
        .declarations
        .get_mut("fogofwar")
        .unwrap()[0]
        .raw = "2".into();
    let actual_bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    assert!(
        state.validate_catalog(&catalog).is_err(),
        "the old Catalog no longer owns the moved Bank"
    );
    state.validate_in_bank(&catalog, &actual_bank).unwrap();
}
