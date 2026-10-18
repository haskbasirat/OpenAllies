use super::*;
use aa_actors::{
    IdentityRegistry,
    loading::{LoadKind, construct_shell_with_components},
};
use aa_mapworld::{
    bootstrap::DefinitionProof,
    restore::{GroundDefinitions, RuntimeDefinition},
};
use serde_json::{Value, json};

#[test]
fn native_required_source_and_bit29_gates_match_both_original_images_all_pc() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-live-map-startup/native-actor-init-gates.json"
    ))
    .unwrap();
    assert_eq!(fixture["paired_case_count"], 162);
    assert_eq!(fixture["native_execution_count"], 324);
    for row in fixture["cases"].as_array().unwrap() {
        let returned = row["output"]["outcome"] == "returned";
        match row["entry"].as_str().unwrap() {
            "004d67f0" | "004d4130" => assert_eq!(
                returned,
                refresh_bit29_returns(row["flags"].as_u64().unwrap() as u32),
                "{row}"
            ),
            "004d1fd0" => assert_eq!(returned, row["source_required_reference"] == 0),
            _ => unreachable!(),
        }
        assert_eq!(row["output"]["actor_bytes_unchanged"], true);
    }
    for row in fixture["owner_virtual108"].as_array().unwrap() {
        assert_eq!(row["leaf"], "004d1e80");
        assert_eq!(row["leaf_bytes"], "8b81e8000000c3");
    }
}

fn definition() -> RuntimeDefinition {
    RuntimeDefinition {
        proof: DefinitionProof {
            id: "synthetic-held-actor".into(),
            source: aa_rules::resources::Provenance {
                path: "synthetic".into(),
                sha256: "synthetic".into(),
                span: aa_rules::tgi::Span { start: 0, end: 1 },
            },
            template_chain: vec![],
            category: *aa_rules::native::CATEGORIES
                .iter()
                .find(|v| v.runtime_class == 3)
                .unwrap(),
            base_flag_inputs: vec![],
        },
        native_class: 3,
        flags: 0,
        object: None,
        component_mask: Some(0),
        ground: GroundDefinitions::default(),
    }
}

#[test]
fn action_refresh_clears_actual_words_and_retains_writes_before_publication_error() {
    let definition = definition();
    let mut shell = construct_shell_with_components::<_, aa_mapworld::restore::RestoredComponent>(
        LoadKind::Actor,
        definition.proof.clone(),
    );
    let id = ActorId::from_raw(17);
    let mut registry = IdentityRegistry::new();
    registry
        .register_saved(id, false, RestoredRecord::Loading)
        .unwrap();
    let actor = shell.actor.as_mut().unwrap();
    actor.words.insert(0xf0, 0x12345678);
    actor.words.insert(0xf4, 0x98765432);
    actor.words.insert(0xe8, 0);
    shell.thing.word_c = 0x12345678; // requires renderer+54 only after mask stores.
    let address = shell.actor.as_ref().unwrap() as *const _ as usize;
    let mut record = Record {
        owner: id,
        shell: &mut shell,
        definition: &definition,
        registry: &mut registry,
    };
    assert!(
        rebuild_actions(&mut record)
            .unwrap_err()
            .to_string()
            .contains("renderer")
    );
    assert_eq!(actor_word(&record, 0xf0).unwrap(), 0);
    assert_eq!(actor_word(&record, 0xf4).unwrap(), 0);
    assert_eq!(
        record.shell.actor.as_ref().unwrap() as *const _ as usize,
        address
    );
    // The same real fields now have no changed notification, so no renderer read.
    rebuild_actions(&mut record).unwrap();
    record.shell.actor.as_mut().unwrap().words.remove(&0xe8);
    assert!(
        rebuild_actions(&mut record)
            .unwrap_err()
            .to_string()
            .contains("+E8")
    );
}

#[test]
fn diagnostic_catalog_cannot_enter_actor_null_initialization() {
    struct MustNotRun;
    impl Services for MustNotRun {
        fn object_init_null(&mut self, _: &mut Record<'_>, _: Precision) -> Result<bool> {
            panic!("unverified source entered Object Init")
        }
    }
    let definition = definition();
    let mut shell = construct_shell_with_components::<_, aa_mapworld::restore::RestoredComponent>(
        LoadKind::Actor,
        definition.proof.clone(),
    );
    let mut registry = IdentityRegistry::new();
    let owner = ActorId::from_raw(17);
    registry
        .register_saved(owner, false, RestoredRecord::Loading)
        .unwrap();
    let actors =
        aa_rules::actors::ActorCatalog::from_files(vec![], aa_rules::tgi::Limits::default());
    let catalog = Catalog::new(&actors);
    let mut record = Record {
        owner,
        shell: &mut shell,
        definition: &definition,
        registry: &mut registry,
    };
    let before = format!("{:?}", record.shell);
    assert!(initialize(&catalog, &mut record, Precision::Double53, &mut MustNotRun).is_err());
    assert_eq!(format!("{:?}", record.shell), before);
    let mut wrong_definition = definition.clone();
    wrong_definition.native_class = 4;
    record.definition = &wrong_definition;
    assert!(
        initialize(&catalog, &mut record, Precision::Double53, &mut MustNotRun)
            .unwrap_err()
            .to_string()
            .contains("layout/proof")
    );
}

#[test]
#[ignore = "requires AA_GAME_ROOT and verified capture03"]
fn actual_first_kingdom_actor_init_tail_uses_real_empty_fx_all_live_pc() {
    use crate::{
        combat_source_completion as completion,
        combat_source_receipt::{TargetFiles, VerifiedStartupReceipt},
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use aa_mapworld::restore::RestoreMode;
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources, sha256},
        tgi::Limits,
    };
    use std::sync::Arc;
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let native = game.join("reconstruction/native-game/runtime-expanded-v14");
    let capture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/development10/native-source-capture-03");
    let exe = std::fs::read(native.join("AA.exe")).unwrap();
    let dll = std::fs::read(native.join("D3D9.dll")).unwrap();
    let config = std::fs::read(native.join("dgVoodoo.conf")).unwrap();
    let receipt = std::fs::read(capture.join("capture.json")).unwrap();
    let manifest = std::fs::read(capture.join("runtime-files.json")).unwrap();
    let map_bytes = std::fs::read(
        game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
    )
    .unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(map_bytes.clone()).unwrap();
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        &native,
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    let mut attempt = Attempt::load_with_source_setup(
        &map,
        &actors,
        settings.clone(),
        text,
        Precision::Double53,
        u64::MAX,
        RestoreMode::InspectPartial,
        LoadBindings {
            native_load_mode: Some(1),
            inactive_load: Some(inactive),
            resources: Some(resources),
        },
        |catalog| {
            let proof = VerifiedStartupReceipt::verify(
                catalog,
                &mut sources,
                TargetFiles {
                    native_exe: &exe,
                    renderer_dll: &dll,
                    renderer_config: &config,
                    capture_receipt: &receipt,
                    file_manifest_receipt: &manifest,
                },
            )?;
            completion::install(
                catalog,
                &mut sources,
                proof.context(),
                proof.csv(),
                proof.globals(),
            )?;
            completion::compute(catalog)
        },
    )
    .unwrap();
    let owner = attempt
        .registry
        .iter()
        .find(|e| matches!(e.value, RestoredRecord::Kingdom(_)))
        .unwrap()
        .id;
    let mut held = std::mem::replace(
        attempt.registry.value_mut(owner).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Kingdom(kingdom) = &mut held else {
        unreachable!()
    };
    let runtime = kingdom.runtime.as_mut().unwrap();
    // This test deliberately substitutes only the Object Init service. It tests
    // the real Actor shell tail, never claims terrain/Object/full Kingdom Init.
    struct TailProbe {
        object_calls: usize,
        fx_calls: usize,
    }
    impl Services for TailProbe {
        fn object_init_null(&mut self, _: &mut Record<'_>, _: Precision) -> Result<bool> {
            self.object_calls += 1;
            Ok(true)
        }
        fn existing_fx(&mut self, _: &mut Record<'_>) -> Result<()> {
            self.fx_calls += 1;
            Err(required("42A590 genuine next FX owner boundary"))
        }
    }
    // Rebind the SAME managed inactive observer capability at the same mode.
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(settings);
    catalog
        .bind_inactive_load(attempt.inactive_load.as_ref().unwrap())
        .unwrap();
    *catalog.definition_bank.borrow_mut() = attempt.definitions;
    let mut record = Record {
        owner,
        shell: &mut runtime.base,
        definition: &runtime.definition,
        registry: &mut attempt.registry,
    };
    let shell_address = record.shell as *const _ as usize;
    let source_pc = validate(&catalog, &record).unwrap();
    let state = record.shell.actor.as_ref().unwrap().components.state;
    let reached_fx = state.low & (1 << 21) == 0;
    assert!(
        reached_fx,
        "do not accidentally stop testing the actual FX branch"
    );
    let mut probe = TailProbe {
        object_calls: 0,
        fx_calls: 0,
    };
    let mut rows = Vec::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        assert!(initialize(&catalog, &mut record, pc, &mut probe).unwrap());
        assert_eq!(record.shell as *const _ as usize, shell_address);
        assert_eq!(actor_word(&record, 0xf0).unwrap(), 0);
        assert_eq!(actor_word(&record, 0xf4).unwrap(), 0);
        assert!(matches!(
            record.registry.get(owner).unwrap().value,
            RestoredRecord::Loading
        ));
        rows.push(json!({"live_precision":pc,"result":true,"same_held_allocation":true,
            "actual_properties_retained":record.shell.actor.as_ref().unwrap().components.present(ComponentSlot::Properties),
            "existing_fx_reached":true,"positive_fx_service_calls":probe.fx_calls}));
    }
    assert_eq!(probe.object_calls, 3);
    assert_eq!(probe.fx_calls, 0);
    let output = json!({"scope":"actual source-completed first Kingdom Actor Init tail with explicit Object Init seam; no whole Object/Kingdom/world success claim",
        "owner":owner.raw(),"map_sha256":sha256(&map_bytes),"source_precision":source_pc,
        "state_low":state.low,"state_high":state.high,"same_inactive_load_owner":true,
        "thing_flags":record.shell.thing.flags,"existing_fx_reached":reached_fx,"rows":rows});
    if let Some(path) = std::env::var_os("AA_ACTOR_INIT_CAPTURE") {
        let path = std::path::PathBuf::from(path);
        assert!(path.is_absolute());
        serde_json::to_writer_pretty(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap(),
            &output,
        )
        .unwrap();
    } else {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../research/skirmish-20261016-live-map-startup/actual-kingdom-actor-tail-v2.json",
        );
        let expected: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(output, expected);
    }
    println!("{output}");
    *attempt.registry.value_mut(owner).unwrap() = held;
}
