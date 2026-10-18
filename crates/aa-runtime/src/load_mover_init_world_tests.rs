use super::*;
use aa_actors::{
    ActorId,
    ground_states::MoverDefinition,
    loading::{LoadKind, construct_shell_with_components},
};
use aa_body::F32;
use serde_json::{Value, json};

#[test]
fn native_mover_lazy_version_queue_and_state_prefixes_match288_rows() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-load-component-init/native-mover-init.json");
    let f: Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
    assert_eq!(f["rows"].as_array().unwrap().len(), 288);
    for r in f["rows"].as_array().unwrap() {
        let c = &r["input"];
        let n = &r["native"];
        let owner = ActorId::from_raw(0x10001);
        let mut shell =
            construct_shell_with_components::<(), RestoredComponent>(LoadKind::Actor, ());
        shell.thing.flags = c["flags"].as_u64().unwrap() as u32;
        let actor = shell.actor.as_mut().unwrap();
        actor.components.state.low = c["state_low"].as_u64().unwrap() as u32;
        let mut v = MoverState::new(
            owner,
            MoverDefinition {
                speed: F32::ONE,
                acceleration: F32::ONE,
                turn_radius: F32::ZERO,
                turn_type: 0,
            },
            Precision::Double53,
        )
        .unwrap();
        v.bytes.insert(0x56, c["falling"].as_u64().unwrap() as u8);
        v.header.scheduled_head = c["scheduled"].as_bool().unwrap().then_some(1);
        let before = format!("{:?}", v);
        actor
            .components
            .insert(ComponentSlot::Mover, RestoredComponent::Mover(Box::new(v)));
        let mut called = false;
        let result = prefix(&mut shell, || {
            called = true;
            Ok(c["version"].as_u64().unwrap() as u32)
        });
        let outcome = match result {
            Ok(true) => "complete",
            Ok(false) => panic!("Mover never returns false"),
            Err(e) if e.to_string().contains("50AC00") => "required50AC00",
            Err(e) if e.to_string().contains("4D4A90") => "required4D4A90",
            Err(e) => panic!("unexpected native boundary:{e}"),
        };
        assert_eq!(outcome, n["outcome"].as_str().unwrap());
        assert_eq!(called, c["falling"].as_u64().unwrap() != 0);
        assert_eq!(json!(shell.thing.flags), n["flags"]);
        assert_eq!(
            json!(shell.actor.as_ref().unwrap().components.state.low),
            n["state_low"]
        );
        assert_eq!(format!("{:?}", mover(&shell).unwrap()), before);
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT and verified capture03"]
fn actual_kursk_nonfalling_mover_never_reads_world_version() {
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
    let map_bytes =
        std::fs::read(game.join("reconstruction/assets/original/Data/Maps/(2v2)Kursk.RMP"))
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
        settings,
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

    let id = attempt
        .registry
        .iter()
        .find_map(|e| {
            let RestoredRecord::Actor(v) = &e.value else {
                return None;
            };
            let RestoredComponent::Mover(m) = v
                .shell
                .actor
                .as_ref()?
                .components
                .get(ComponentSlot::Mover)?
            else {
                return None;
            };
            (m.bytes.get(&0x56) == Some(&0)).then_some(e.id)
        })
        .expect("actual map contains no restored nonfalling Mover");
    let mut held = std::mem::replace(
        attempt.registry.value_mut(id).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Actor(v) = &mut held else {
        unreachable!()
    };
    let catalog = Catalog::new(&actors);
    *catalog.definition_bank.borrow_mut() = attempt.definitions;
    let source_pc = catalog
        .definition_bank
        .borrow()
        .combat_source()
        .unwrap()
        .source_precision();
    let mut record = Record {
        owner: id,
        shell: &mut v.shell,
        definition: &v.definition,
        registry: &mut attempt.registry,
    };
    let before = format!("{:?}", record.shell);
    let pointer = mover(record.shell).unwrap() as *const _ as usize;
    let mut rows = Vec::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        assert!(
            init_existing(&catalog, &mut record, pc, || panic!(
                "native+56zero must not read worldVersion"
            ))
            .unwrap()
        );
        after_init(&catalog, &mut record).unwrap();
        assert_eq!(format!("{:?}", record.shell), before);
        assert_eq!(mover(record.shell).unwrap() as *const _ as usize, pointer);
        rows.push(json!({"precision":pc,"init_result":true,"world_version_read":false,"same_allocation":true}));
    }
    assert!(
        init_existing(
            &Catalog::new(&actors),
            &mut record,
            Precision::Double53,
            || panic!("sourceguard before version")
        )
        .is_err()
    );
    let result = json!({"scope":"Actual stopped Kursk nonfalling Mover constant branch only; no wholeActorInit/battle or invented version", "map_sha256":sha256(&map_bytes),"owner":id.raw(),"definition":record.definition.proof,"source_precision":source_pc,"mover":mover(record.shell).unwrap(),"rows":rows});
    if let Some(p) = std::env::var_os("AA_MOVER_INIT_CAPTURE") {
        serde_json::to_writer_pretty(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(p)
                .unwrap(),
            &result,
        )
        .unwrap();
    } else {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../research/skirmish-20261016-load-component-init/actual-nonfalling-mover.json",
        );
        let expected: Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
        assert_eq!(result, expected);
    }
    *attempt.registry.value_mut(id).unwrap() = held;
}
