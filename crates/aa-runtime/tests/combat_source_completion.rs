//! Actual-source dependency census. No completed value is supplied by this test.
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use aa_runtime::{
    catalog::Catalog,
    combat_source_completion::{self as completion, Event, Globals, Phase},
    combat_source_receipt::{self as receipt, TargetFiles, VerifiedStartupReceipt},
    combat_value_source,
    combat_value_source_context::{self as context, CsvScope, SourceFpContext},
    settings::Settings,
};
use serde_json::{Value, json};

#[test]
#[ignore = "requires actual original/v14 source bytes via AA_GAME_ROOT"]
fn census_current_full_manager_combat_dependencies() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let output = std::env::var_os("AA_CV_SOURCE_COMPLETION_CENSUS");
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
    catalog.settings =
        Some(Settings::load_v14(&mut sources, aa_rules::tgi::Limits::default()).unwrap());
    let csv = context::bind_csv_from_sources(
        &mut sources,
        CsvScope::FreshNoProfile {
            label: "explicit diagnostic full-source census; no native application profile imported",
        },
    )
    .unwrap();
    let order = combat_value_source::actor_manager_order(&catalog).unwrap();
    let mut rows = Vec::new();
    let mut numeric = Vec::new();
    for (pc, cw) in [
        (Precision::Single24, 0x7f),
        (Precision::Double53, 0x27f),
        (Precision::Extended64, 0x37f),
    ] {
        let fp = SourceFpContext::supplied_diagnostic(
            cw,
            0,
            0x1f80,
            true,
            "full-source census laboratory context only",
        )
        .unwrap();
        numeric.push(
            json!({"precision":pc,"values":context::bind_numeric(&catalog,&fp).unwrap().values()}),
        );
        for id in &order.ids {
            let mask = catalog.canonical_component_mask(id, pc);
            let full = combat_value_source::definition_inputs(&catalog, id, pc);
            let binding = catalog.actor_bindings(id, pc).unwrap();
            rows.push(json!({"id":id,"precision":pc,"mask":mask,"input":full.as_ref().map(|v|json!({"source":v.source,"flags":v.flags_174,"body":v.body.map(|(d,h)|[d.0,h.0]),"speed":v.speed_228.map(|v|v.0),"bias":v.performer_bias_210.map(|v|v.0),"properties":v.properties_1d4.iter().map(|(id,v)|json!([id,v.0])).collect::<Vec<_>>(),"children":v.children_2f0,"weighted":v.weighted_35c,"abilities":v.abilities.iter().enumerate().map(|(i,a)| {let loaded=&binding.performer.as_ref().unwrap().abilities()[i];json!({"definition_flags":a.definition.flags_54,"range":a.definition.range_24.0,"area":a.definition.area_38.0,"projectile":a.definition.projectile_3c,"power":a.definition.power_64.0,"reload":a.value_04.0,"recharge":a.recharge_18.0,"animation":a.first_entry_10.map(|v|v.0),"shared_reference":loaded.authored.shared_definition_id,"definition_sources":loaded.authored.definition.sources,"local_sources":loaded.authored.sources})}).collect::<Vec<_>>()})).map_err(Clone::clone)}));
        }
    }
    let result = json!({"scope":"Actual-source diagnostic binding census only; all three explicit laboratory PCs and FreshNoProfile; no observed native startup FP/profile or completed CV","source_diagnostics":order.source_diagnostics,"source_identities":actors.sources,"manager_order":order.ids,"numeric":numeric,"csv":csv,"rows":rows});
    if let Some(output) = output {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .unwrap();
        serde_json::to_writer_pretty(file, &result).unwrap();
    } else {
        let expected: Value=serde_json::from_str(include_str!("../../../research/skirmish-20261016-combat-source-completion/full-source-inputs-v3.json")).unwrap();
        assert_eq!(result, expected);
    }
}

#[test]
#[ignore = "requires actual v14 reference build and sealed Windows capture files via AA_GAME_ROOT"]
fn verified_reference_computes_then_publishes_the_captured_whole_source_cache() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let native = game.join("reconstruction/native-game/runtime-expanded-v14");
    let capture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/development10/native-source-capture-03");
    let exe = std::fs::read(native.join("AA.exe")).unwrap();
    let renderer = std::fs::read(native.join("D3D9.dll")).unwrap();
    let config = std::fs::read(native.join("dgVoodoo.conf")).unwrap();
    let capture = std::fs::read(capture_dir.join("capture.json")).unwrap();
    let manifest = std::fs::read(capture_dir.join("runtime-files.json")).unwrap();
    let files = || TargetFiles {
        native_exe: &exe,
        renderer_dll: &renderer,
        renderer_config: &config,
        capture_receipt: &capture,
        file_manifest_receipt: &manifest,
    };
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        &native,
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(Settings::load_v14(&mut sources, aa_rules::tgi::Limits::default()).unwrap());
    let mut altered = capture.clone();
    altered[0] ^= 1;
    assert!(
        VerifiedStartupReceipt::verify(
            &catalog,
            &mut sources,
            TargetFiles {
                capture_receipt: &altered,
                ..files()
            }
        )
        .err()
        .unwrap()
        .contains("sealed native capture")
    );
    assert!(
        VerifiedStartupReceipt::verify(
            &catalog,
            &mut sources,
            TargetFiles {
                renderer_dll: b"wrong renderer",
                ..files()
            }
        )
        .err()
        .unwrap()
        .contains("D3D9.dll")
    );
    let external = VerifiedStartupReceipt::verify(&catalog, &mut sources, files()).unwrap();
    let verified =
        VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut sources).unwrap();
    assert_eq!(
        external.context().reference_startup(),
        verified.context().reference_startup()
    );
    assert_eq!(
        external.csv().reference_startup(),
        verified.csv().reference_startup()
    );
    assert!(verified.context().is_observed_native_startup());
    assert!(verified.csv().is_observed_native_startup());
    // A valid A receipt cannot certify Catalog B, even if the altered source
    // field is outside CV and the final numeric fingerprint would be identical.
    let mut other_actors = actors.clone();
    other_actors.sources[0].sha256 = "unrelated changed source identity".into();
    let mut other = Catalog::new(&other_actors);
    other.settings = catalog.settings.clone();
    assert!(
        completion::install(
            &other,
            &mut sources,
            verified.context(),
            verified.csv(),
            verified.globals()
        )
        .unwrap_err()
        .contains("destination logical source identities")
    );
    assert!(other.definition_bank.borrow().combat_source().is_err());
    assert!(
        other
            .definition_bank
            .borrow()
            .token("ActorDefinition", "minefield")
            .is_none()
    );
    let mut wrong_settings = Catalog::new(&actors);
    wrong_settings.settings = catalog.settings.clone();
    wrong_settings
        .settings
        .as_mut()
        .unwrap()
        .declarations
        .get_mut("combatvaluemeanvaluesdiscardpercent")
        .unwrap()
        .last_mut()
        .unwrap()
        .raw = "0.21".into();
    assert!(
        completion::install(
            &wrong_settings,
            &mut sources,
            verified.context(),
            verified.csv(),
            verified.globals()
        )
        .unwrap_err()
        .contains("Settings value/span")
    );
    assert!(
        wrong_settings
            .definition_bank
            .borrow()
            .combat_source()
            .is_err()
    );
    let native_capture: Value = serde_json::from_slice(&capture).unwrap();
    let entry = native_capture["captures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["label"] == "source_entry")
        .unwrap();
    assert_eq!(
        verified.context().crt_environment().x87_control,
        entry["x87_control"].as_u64().unwrap() as u16
    );
    assert_eq!(
        verified.context().x87_status(),
        entry["x87_status"].as_u64().unwrap() as u16
    );
    assert_eq!(
        verified.context().crt_environment().mxcsr,
        entry["mxcsr"].as_u64().unwrap() as u32
    );
    assert!(
        native_capture["fp_state_never_written_by_debugger"]
            .as_bool()
            .unwrap()
    );
    assert!(
        native_capture["folder_redirection_observed"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(
        receipt::source_fingerprint(&actors.sources),
        receipt::SOURCES_SHA256
    );
    completion::install(
        &catalog,
        &mut sources,
        verified.context(),
        verified.csv(),
        verified.globals(),
    )
    .unwrap();
    assert!(
        catalog
            .definition_bank
            .borrow()
            .combat_source()
            .unwrap()
            .completed_maximum_74()
            .is_err()
    );
    completion::compute(&catalog).unwrap();
    let bank = catalog.definition_bank.borrow();
    let state = bank.combat_source().unwrap();
    assert_eq!(state.phase(), Phase::Complete);
    assert_eq!(
        state.result_fingerprint().unwrap(),
        receipt::COMBAT_RESULT_SHA256
    );
    let captured = native_capture["captures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["label"] == "source_return")
        .unwrap();
    let rows = captured["actors"].as_array().unwrap();
    assert_eq!(state.manager_order().len(), rows.len());
    for (&token, row) in state.manager_order().iter().zip(rows) {
        assert_eq!(
            state.definition_id(token).unwrap(),
            row["ids"].as_str().unwrap()
        );
        let v = state.completed_values(token).unwrap();
        assert_eq!(
            json!([v.value_1e4.0, v.children_30c.0, v.weighted_36c.0]),
            json!([
                row["fields"]["1e4"],
                row["fields"]["30c"],
                row["fields"]["36c"]
            ])
        );
    }
    let m = state.completed_means().unwrap();
    assert_eq!(
        json!([
            m.maximum_74.0,
            m.attack.0,
            m.attack_rate.0,
            m.defense.0,
            m.movement.0,
            m.health.0,
            m.lifetime.0,
            m.offense.0
        ]),
        json!([
            captured["manager_means"]["74"],
            captured["manager_means"]["78"],
            captured["manager_means"]["7c"],
            captured["manager_means"]["80"],
            captured["manager_means"]["84"],
            captured["manager_means"]["88"],
            captured["manager_means"]["8c"],
            captured["manager_means"]["90"]
        ])
    );
    assert_eq!(state.completed_maximum_74().unwrap().0, 0);
}

#[test]
#[ignore = "requires actual original/v14 source bytes via AA_GAME_ROOT"]
fn full_shipped_shared_combat_source_matches_native_all_values_and_order() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let evidence = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-combat-source-completion");
    let native: Value = serde_json::from_slice(
        &std::fs::read(evidence.join("native-source-completion-v3.json")).unwrap(),
    )
    .unwrap();
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let settings = Settings::load_v14(&mut sources, aa_rules::tgi::Limits::default()).unwrap();
    let csv = context::bind_csv_from_sources(
        &mut sources,
        CsvScope::FreshNoProfile {
            label: "explicit all-source native replay diagnostic; no application profile claim",
        },
    )
    .unwrap();
    let mut results = Vec::new();
    for (pc, cw) in [
        (Precision::Single24, 0x7f),
        (Precision::Double53, 0x27f),
        (Precision::Extended64, 0x37f),
    ] {
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(settings.clone());
        let fp = SourceFpContext::supplied_diagnostic(
            cw,
            0,
            0x1f80,
            true,
            "all-source completion diagnostic only",
        )
        .unwrap();
        completion::install(
            &catalog,
            &mut sources,
            &fp,
            &csv,
            Globals {
                assertion_guard: 0,
                ranged_lookup_guard: 0,
                csv_lookup_guard: 0,
                debug_stream_is_closed: true,
            },
        )
        .unwrap();
        {
            let bank = catalog.definition_bank.borrow();
            let state = bank.combat_source().unwrap();
            assert_eq!(state.phase(), Phase::Bound);
            assert_eq!(state.manager_order().len(), 1028);
            assert!(state.diagnostic_means().is_err());
            for &token in state.manager_order() {
                assert_eq!(bank.entry(token).unwrap().key.domain, "actordefinition");
                assert_eq!(
                    state.diagnostic_values(token).unwrap().value_1e4.0,
                    0xbf800000
                );
                assert!(state.completed_values(token).is_err());
            }
        }
        completion::compute(&catalog).unwrap();
        assert!(
            completion::compute(&catalog)
                .unwrap_err()
                .contains("already started")
        );
        assert!(
            completion::install(
                &catalog,
                &mut sources,
                &fp,
                &csv,
                Globals {
                    assertion_guard: 0,
                    ranged_lookup_guard: 0,
                    csv_lookup_guard: 0,
                    debug_stream_is_closed: true
                }
            )
            .unwrap_err()
            .contains("already installed")
        );
        let mut bank = catalog.definition_bank.borrow_mut();
        // Existing runtime token allocation resolves to the SAME source state.
        let infantry = bank.intern("ActorDefinition", "usa_infantry").unwrap();
        let state = bank.combat_source().unwrap();
        assert_eq!(state.phase(), Phase::Complete);
        assert!(!state.is_observed_native_startup());
        assert!(state.completed_values(infantry).is_err());
        let means = state.diagnostic_means().unwrap();
        let means = json!([
            means.maximum_74.0,
            means.attack.0,
            means.attack_rate.0,
            means.defense.0,
            means.movement.0,
            means.health.0,
            means.lifetime.0,
            means.offense.0
        ]);
        let values=state.manager_order().iter().map(|&token|{let v=state.diagnostic_values(token).unwrap();json!({"id":state.definition_id(token).unwrap(),"value_1e4":v.value_1e4.0,"children_30c":v.children_30c.0,"weighted_36c":v.weighted_36c.0})}).collect::<Vec<_>>();
        let events = state
            .events()
            .iter()
            .map(|event| match event {
                Event::Definition(token) => {
                    json!(["definition", state.definition_id(*token).unwrap()])
                }
                Event::Store {
                    definition,
                    field,
                    value,
                } => json!([
                    "store",
                    state.definition_id(*definition).unwrap(),
                    match field {
                        aa_combat::combat_value::DefinitionField::Value1E4 => 0x1e4,
                        aa_combat::combat_value::DefinitionField::Children30C => 0x30c,
                        aa_combat::combat_value::DefinitionField::Weighted36C => 0x36c,
                    },
                    value.0
                ]),
                Event::Lookup { name, kind } => json!(["lookup", name, kind]),
                Event::MeanLog {
                    line,
                    value_binary64,
                } => json!(["mean_log", line, format!("{value_binary64:016x}")]),
            })
            .collect::<Vec<_>>();
        for row in native["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["precision"] == json!(pc))
        {
            assert_eq!(means, row["means"], "{} {:?} means", row["image"], pc);
            let expected_values = row["values"].as_array().unwrap();
            assert_eq!(values.len(), expected_values.len());
            for (index, (actual, expected)) in values.iter().zip(expected_values).enumerate() {
                assert_eq!(
                    actual, expected,
                    "{} {:?} value row {index}",
                    row["image"], pc
                );
            }
            let expected_events = row["events"].as_array().unwrap();
            assert_eq!(events.len(), expected_events.len());
            for (index, (actual, expected)) in events.iter().zip(expected_events).enumerate() {
                assert_eq!(actual, expected, "{} {:?} event {index}", row["image"], pc);
            }
        }
        results.push(json!({"precision":pc,"means":means,"values":values,"events":events,"context_observed_native_startup":false}));
    }
    if let Some(path) = std::env::var_os("AA_CV_SOURCE_COMPLETION_RESULT") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file,&json!({"scope":"actual shared Bank CV dependency slice under explicit diagnostic contexts; not full4BD800/native startup","results":results})).unwrap();
    }
}
