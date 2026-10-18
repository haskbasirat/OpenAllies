use aa_actors::ActorId;
use aa_economy::{Catalog, EconomyError, F32Bits, State, X87Precision, borrowed_flows};
use aa_mapworld::special_state::SpecialShell;
use aa_runtime::saved_kingdom_flows::BorrowedKingdom;
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-borrowed-flows/native-borrowed-flows.json"
    ))
    .unwrap()
}
fn install(state: &State) -> SpecialShell {
    let mut shell = SpecialShell::kingdom(ActorId::from_raw(1), 0).unwrap();
    for (offset, values) in [
        (0x1a0, &state.effective_provided),
        (0x1ac, &state.raw_provided),
        (0x1b8, &state.consumed),
        (0x1c4, &state.stockpiled),
    ] {
        shell.vectors.get_mut(&offset).unwrap().values = values.iter().map(|v| v.0).collect();
    }
    shell
}
fn snapshot(shell: &SpecialShell) -> State {
    let values = |offset| {
        shell.vectors[&offset]
            .values
            .iter()
            .copied()
            .map(F32Bits)
            .collect()
    };
    State {
        effective_provided: values(0x1a0),
        raw_provided: values(0x1ac),
        consumed: values(0x1b8),
        stockpiled: values(0x1c4),
    }
}

#[test]
fn native_fixture_results_use_the_actual_saved_vectors_without_reallocation() {
    let fixture = fixture();
    let mut checked = 0;
    for row in fixture["cases"].as_array().unwrap() {
        let input = &row["input"];
        let catalog: Catalog = serde_json::from_value(input["catalog"].clone()).unwrap();
        let initial: State = serde_json::from_value(input["initial"].clone()).unwrap();
        // The concrete owner exposes its real lengths. Fixture-only overridden
        // lengths and injected memory/setter faults belong to the pure oracle.
        let failure = &input["failure"];
        if input["resource_count"]
            .as_i64()
            .is_some_and(|v| v != catalog.resources.len() as i64)
            || input["effective_count"]
                .as_i64()
                .is_some_and(|v| v != initial.effective_provided.len() as i64)
            || failure["write"].is_number()
            || failure["coefficient"] == true
            || failure["consumed_missing"] == true
        {
            continue;
        }
        let mut shell = install(&initial);
        let allocation = |shell: &SpecialShell| {
            [0x1a0, 0x1ac, 0x1b8, 0x1c4].map(|offset| {
                let vector = &shell.vectors[&offset];
                (
                    vector.values.as_ptr(),
                    vector.values.len(),
                    vector.capacity_hint,
                )
            })
        };
        let before = allocation(&shell);
        let mut maximum = || {
            if failure["maximum"] == true {
                Err(EconomyError::World("fixture maximum unavailable".into()))
            } else {
                Ok(input["limits"]["provided_maximum"].as_i64().unwrap() as i32)
            }
        };
        let mut world = BorrowedKingdom {
            kingdom: &mut shell,
            resources: &catalog,
            consumption_count: input["consumption_count"]
                .as_i64()
                .unwrap_or(catalog.resources.len() as i64) as i32,
            provider_maximum: &mut maximum,
        };
        let pc: X87Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let action = &input["action"];
        let mut values: Vec<F32Bits> = action
            .get("values")
            .map(|v| serde_json::from_value(v.clone()).unwrap())
            .unwrap_or_default();
        if let Some(length) = failure["input_length"].as_u64() {
            values.truncate(length as usize);
        }
        let result = match action["kind"].as_str().unwrap() {
            "Provider" => borrowed_flows::adjust_provider(
                &mut world,
                action["index"].as_u64().unwrap() as usize,
                F32Bits(action["delta"].as_u64().unwrap() as u32),
                pc,
            ),
            "Providers" => borrowed_flows::adjust_providers(
                &mut world,
                &values,
                action["add"].as_bool().unwrap(),
                pc,
            ),
            "Consumption" => borrowed_flows::adjust_consumption(
                &mut world,
                &values,
                action["add"].as_bool().unwrap(),
                pc,
            ),
            _ => panic!("unknown fixture action"),
        };
        assert_eq!(
            result.is_ok(),
            row["output"]["outcome"] == "ok",
            "{row} {result:?}"
        );
        assert_eq!(json!(snapshot(&shell)), row["output"]["state"], "{row}");
        assert_eq!(allocation(&shell), before);
        checked += 1;
    }
    assert_eq!(checked, 876);
    println!("replayed {checked} original/v14 storage cases");
}

#[test]
fn constructor_arrays_fail_when_reached_without_becoming_initialized() {
    let fixture = fixture();
    let catalog: Catalog =
        serde_json::from_value(fixture["cases"][0]["input"]["catalog"].clone()).unwrap();
    let mut shell =
        SpecialShell::kingdom(ActorId::from_raw(1), catalog.resources.len() as u32).unwrap();
    let before = snapshot(&shell);
    let mut maximum = || panic!("missing storage is reached before maximum");
    let mut world = BorrowedKingdom {
        kingdom: &mut shell,
        resources: &catalog,
        consumption_count: 1,
        provider_maximum: &mut maximum,
    };
    let error = borrowed_flows::adjust_consumption(
        &mut world,
        &[F32Bits::ZERO],
        true,
        X87Precision::Double53,
    )
    .unwrap_err();
    assert!(error.to_string().contains("1b8[0]"));
    assert_eq!(snapshot(&shell), before);
    assert!(
        [0x1a0, 0x1ac, 0x1b8]
            .iter()
            .all(|offset| shell.vectors[offset].values.is_empty())
    );
}

#[test]
fn missing_effective_slot_retains_raw_store_and_never_grows_storage() {
    let resources = Catalog {
        resources: vec![aa_economy::ResourceDefinition {
            id: "upkeep".into(),
            kind: aa_economy::ResourceKind::Upkeep,
            limited_provider: false,
            purchase_cost: vec![F32Bits::ZERO],
        }],
    };
    let mut shell = install(&State {
        raw_provided: vec![F32Bits::from_f32(3.0)],
        effective_provided: vec![],
        consumed: vec![],
        stockpiled: vec![F32Bits::from_f32(17.0)],
    });
    let raw_ptr = shell.vectors[&0x1ac].values.as_ptr();
    let mut maximum = || panic!("unlimited providers never read this setting");
    let mut world = BorrowedKingdom {
        kingdom: &mut shell,
        resources: &resources,
        consumption_count: 1,
        provider_maximum: &mut maximum,
    };
    let error = borrowed_flows::adjust_provider(
        &mut world,
        0,
        F32Bits::from_f32(2.0),
        X87Precision::Double53,
    )
    .unwrap_err();
    assert!(error.to_string().contains("1a0[0]"));
    assert_eq!(shell.vectors[&0x1ac].values, [5.0f32.to_bits()]);
    assert_eq!(shell.vectors[&0x1ac].values.as_ptr(), raw_ptr);
    assert!(shell.vectors[&0x1a0].values.is_empty());
    assert!(shell.vectors[&0x1b8].values.is_empty());
    assert_eq!(shell.vectors[&0x1c4].values, [17.0f32.to_bits()]);
}

#[test]
fn source_integer_overwrite_matches_native_settings_and_rejects_ambiguous_sources() {
    use aa_rules::{
        actors::native_load::{NativeLoadReport, load_files},
        sources::{Origin, ResolvedFile, sha256},
        tgi::Limits,
    };
    use aa_runtime::{catalog::Catalog, saved_kingdom_flows::provider_maximum, settings::Settings};
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-saved-flows/native-source-bindings.json"
    ))
    .unwrap();
    let file = |path: &str, text: String| {
        let bytes = text.into_bytes();
        ResolvedFile {
            logical_path: path.into(),
            origin: Origin::V14Loose,
            sha256: sha256(&bytes),
            bytes,
        }
    };
    for row in fixture["settings_cases"].as_array().unwrap() {
        let sequence = row["sequence"].as_array().unwrap();
        if sequence.len() != 2 {
            continue;
        }
        let files = [
            file(
                "Game/SVars.tgi",
                format!(
                    "[SVars]\n{{\nint EconomyLimitedResourceMax = {}\n}}\n",
                    sequence[0]
                ),
            ),
            file(
                "Game/SVars_AA.tgi",
                format!(
                    "[SVars]\n{{\nint EconomyLimitedResourceMax = {}\n}}\n",
                    sequence[1]
                ),
            ),
        ];
        let actors = load_files(
            files
                .iter()
                .map(|source| {
                    file(
                        &source.logical_path,
                        String::from_utf8(source.bytes.clone()).unwrap(),
                    )
                })
                .collect(),
            Limits::default(),
            &mut NativeLoadReport::default(),
        );
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(Settings::from_files(files, Limits::default()).unwrap());
        let pc = aa_numeric::Precision::Double53;
        let expected = row["states"][1]["value"].as_u64().unwrap() as u32 as i32;
        assert_eq!(provider_maximum(&catalog, pc).unwrap(), expected);
        let rows = catalog
            .settings
            .as_mut()
            .unwrap()
            .declarations
            .get_mut("economylimitedresourcemax")
            .unwrap();
        rows.reverse(); // The provenance order, not Vec order, controls overwrite.
        assert_eq!(provider_maximum(&catalog, pc).unwrap(), expected);
        catalog
            .settings
            .as_mut()
            .unwrap()
            .declarations
            .get_mut("economylimitedresourcemax")
            .unwrap()[0]
            .source
            .sha256 = "different".into();
        assert!(
            provider_maximum(&catalog, pc)
                .unwrap_err()
                .to_string()
                .contains("identities differ")
        );
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets; bounded two-map restore diagnostic"]
fn original_saved_kingdom_flags_and_flow_lifecycle_boundary() {
    use aa_mapworld::restore::{RestoreMode, RestoredRecord};
    use aa_numeric::Precision;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use aa_runtime::{localization::Localization, restoration::Attempt, settings::Settings};
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let resource_source = aa_rules::resources::ResourceCatalog::load_v14(&mut sources).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let definitions =
            aa_runtime::kingdom_economy::Definitions::bind(&resource_source, &actors, pc).unwrap();
        assert_eq!(
            aa_runtime::saved_kingdom_flows::consumption_count(&definitions).unwrap(),
            7
        );
        let mut catalog = aa_runtime::catalog::Catalog::new(&actors);
        catalog.settings = Some(settings.clone());
        assert_eq!(
            aa_runtime::saved_kingdom_flows::provider_maximum(&catalog, pc).unwrap(),
            100
        );
    }
    let mut reports = Vec::new();
    for path in [
        "Campaign/tutorial02.RMP",
        "DeveloperMaps/(3v3)Firefight.RMP",
    ] {
        let bytes = std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps")
                .join(path),
        )
        .unwrap();
        let hash = aa_rules::sources::sha256(&bytes);
        let map = aa_mapworld::MapArchive::from_bytes(bytes).unwrap();
        let attempt = Attempt::load_with_native_mode(
            &map,
            &actors,
            settings.clone(),
            text.clone(),
            Precision::Double53,
            u64::MAX,
            RestoreMode::InspectPartial,
            Some(1),
        )
        .unwrap();
        let mut kingdoms = Vec::new();
        for entry in attempt.registry.iter() {
            let RestoredRecord::Kingdom(entity) = &entry.value else {
                continue;
            };
            let runtime = entity.runtime.as_ref().unwrap();
            let shell = runtime.derived.as_ref().unwrap();
            let vectors = [0x1a0, 0x1ac, 0x1b8, 0x1c4].map(|offset| {
                let vector = &shell.vectors[&offset];
                json!({"offset":format!("{offset:x}"),"length":vector.values.len(),"capacity_hint":vector.capacity_hint,"values":vector.values})
            });
            kingdoms.push(json!({"id":entry.id.raw(),"definition":runtime.definition.proof.id,"saved_flags":entity.source.thing.serialized_flags,"runtime_flags":runtime.base.thing.flags,"bit7":runtime.base.thing.flags & 0x80 != 0,"completion":format!("{:?}",entity.completion),"failure":entity.failure,"vectors":vectors}));
        }
        assert!(!kingdoms.is_empty());
        reports.push(json!({"map":path,"sha256":hash,"kingdoms":kingdoms,"publication_ready":attempt.report.runtime_publication_ready,"first_error":attempt.report.events.iter().find(|event| event.error.is_some())}));
    }
    let report = json!({"scope":"Unmodified two-map InspectPartial restore diagnostic; no PostLoad/publication/battle claim","maps":reports});
    if let Some(path) = std::env::var_os("AA_SAVED_KINGDOM_FLOWS_OUT") {
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file, &report).unwrap();
    } else {
        println!("{report}");
    }
}
