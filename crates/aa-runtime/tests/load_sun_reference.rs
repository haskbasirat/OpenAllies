use aa_runtime::load_sun_reference::{ConstructorInput, REFERENCE_SHA256};

#[test]
fn sealed_reference_has_actual_three_word_watch_coverage() {
    let bytes = include_bytes!(
        "../../../research/skirmish-20261016-sun-startup-capture/sun-startup-reference.json"
    );
    assert_eq!(aa_rules::sources::sha256(bytes), REFERENCE_SHA256);
    let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    assert_eq!(value["words"], serde_json::json!([0, 0, 0]));
    assert_eq!(
        value["watched_addresses"],
        serde_json::json!([0xb2814c, 0xb28150, 0xb28154])
    );
    assert_eq!(value["armed_threads"], 62);
    assert_eq!(value["post_cutoff_calibration_probes"], 3);
    assert_eq!(value["observed_sun_constructor"], false);
    assert_eq!(value["cleanup_succeeded"], true);
}

#[test]
#[ignore = "requires original Data.rwd and accepted v14 sources via AA_GAME_ROOT"]
fn actual_source_constructs_sun_and_rechecks_consumed_input_destination() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
    };
    use aa_runtime::{
        catalog::Catalog, combat_source_receipt::VerifiedStartupReceipt, settings::Settings,
    };
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
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(Settings::load_v14(&mut sources, aa_rules::tgi::Limits::default()).unwrap());
    let receipt =
        VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
    let input = ConstructorInput::from_verified_startup_reference(&catalog, &receipt).unwrap();
    let foreign_input =
        ConstructorInput::from_verified_startup_reference(&catalog, &receipt).unwrap();
    let changed_input =
        ConstructorInput::from_verified_startup_reference(&catalog, &receipt).unwrap();
    let sun = input.create_sun(&catalog).unwrap();
    assert_eq!(sun.current_colors, [[0; 3]; 3]);
    assert_eq!(sun.target_colors, [[0; 3]; 3]);
    assert_eq!(sun.direction, [0, 0, 0x3f80_0000]);
    assert_eq!(sun.start_time, 0xbf80_0000);
    let empty = ActorCatalog::from_files(vec![], aa_rules::tgi::Limits::default());
    assert!(foreign_input.create_sun(&Catalog::new(&empty)).is_err());
    assert!(
        ConstructorInput::from_verified_startup_reference(&Catalog::new(&empty), &receipt).is_err()
    );
    for declaration in catalog
        .settings
        .as_mut()
        .unwrap()
        .declarations
        .get_mut("combatdamageminimum")
        .unwrap()
    {
        declaration.raw = "2".into();
    }
    assert!(
        changed_input.create_sun(&catalog).is_err(),
        "retained input must recheck current destination"
    );
    assert!(ConstructorInput::from_verified_startup_reference(&catalog, &receipt).is_err());
}
