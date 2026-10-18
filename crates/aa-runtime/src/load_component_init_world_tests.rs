use super::*;
use serde_json::{Value, json};

#[test]
#[ignore = "requires AA_GAME_ROOT and verified capture03"]
fn actual_first_kingdom_properties_constant_leaves_keep_the_same_allocation() {
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
        .find(|v| matches!(v.value, RestoredRecord::Kingdom(_)))
        .unwrap()
        .id;
    let mut held = std::mem::replace(
        attempt.registry.value_mut(id).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Kingdom(kingdom) = &mut held else {
        unreachable!()
    };
    let runtime = kingdom.runtime.as_mut().unwrap();
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
        shell: &mut runtime.base,
        definition: &runtime.definition,
        registry: &mut attempt.registry,
    };
    let component = record
        .shell
        .actor
        .as_ref()
        .unwrap()
        .components
        .get(ComponentSlot::Properties)
        .unwrap();
    let RestoredComponent::Properties(state) = component else {
        unreachable!()
    };
    let before = state.as_ref().clone();
    let pointer = state.as_ref() as *const _ as usize;
    let shell_before = format!("{:?}", record.shell);
    let mut rows = Vec::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        assert!(init_existing(&catalog, &mut record, ComponentSlot::Properties, pc).unwrap());
        after_init(&catalog, &mut record, ComponentSlot::Properties).unwrap();
        let RestoredComponent::Properties(state) = record
            .shell
            .actor
            .as_ref()
            .unwrap()
            .components
            .get(ComponentSlot::Properties)
            .unwrap()
        else {
            unreachable!()
        };
        assert_eq!(state.as_ref(), &before);
        assert_eq!(state.as_ref() as *const _ as usize, pointer);
        assert_eq!(format!("{:?}", record.shell), shell_before);
        rows.push(json!({"precision":pc,"result":true,"properties":state,"same_allocation":true}));
    }
    // Explicit authority/storage failures do not enter the native leaf.
    assert!(
        init_existing(
            &Catalog::new(&actors),
            &mut record,
            ComponentSlot::Properties,
            Precision::Double53
        )
        .is_err()
    );
    assert!(
        init_existing(
            &catalog,
            &mut record,
            ComponentSlot::Body,
            Precision::Double53
        )
        .is_err()
    );
    let saved_owner = {
        let RestoredComponent::Properties(v) = record
            .shell
            .actor
            .as_mut()
            .unwrap()
            .components
            .get_mut(ComponentSlot::Properties)
            .unwrap()
        else {
            unreachable!()
        };
        let old = v.owner;
        v.owner = Object(id.raw() ^ 0x10000);
        old
    };
    assert!(
        init_existing(
            &catalog,
            &mut record,
            ComponentSlot::Properties,
            Precision::Double53
        )
        .is_err()
    );
    if let RestoredComponent::Properties(v) = record
        .shell
        .actor
        .as_mut()
        .unwrap()
        .components
        .get_mut(ComponentSlot::Properties)
        .unwrap()
    {
        v.owner = saved_owner;
    }
    assert_eq!(format!("{:?}", record.shell), shell_before);
    let result = json!({"scope":"actual stopped tutorial02 first Kingdom Properties InitExisting/+14, not full Init/PostCreate or world readiness",
        "owner":id.raw(),"source_precision":source_pc,"map_sha256":sha256(&map_bytes),
        "definition":record.definition.proof,"component_mask":record.definition.component_mask,"rows":rows});
    if let Some(path) = std::env::var_os("AA_COMPONENT_INIT_CAPTURE") {
        serde_json::to_writer_pretty(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap(),
            &result,
        )
        .unwrap();
    } else {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../research/skirmish-20261016-load-component-init/actual-properties.json");
        let expected: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(result, expected);
    }
    *attempt.registry.value_mut(id).unwrap() = held;
}

#[test]
fn constant_component_leaf_fixture_has_no_world_or_state_effects() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-load-component-init/native-properties-init.json");
    let fixture: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let kind = FactoryKind::Standard(ComponentSlot::Properties);
    let leaves = component_virtuals::leaves(kind);
    assert_eq!(leaves.init_existing, 0x6a71d0);
    assert_eq!(leaves.vf14, 0x4b2830);
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 3);
    for row in fixture["rows"].as_array().unwrap() {
        assert_eq!(row["init_result"], true);
        assert_eq!(row["writes"], json!([]));
        assert_eq!(row["seams"], json!([]));
    }
}
