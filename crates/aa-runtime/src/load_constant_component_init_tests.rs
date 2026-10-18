use super::*;
use serde_json::{Value, json};

fn allocation(v: &RestoredComponent) -> usize {
    match v {
        RestoredComponent::Character(v) => v.as_ref() as *const _ as usize,
        RestoredComponent::Structure(v) => v.as_ref() as *const _ as usize,
        RestoredComponent::Research(v) => v.as_ref() as *const _ as usize,
        RestoredComponent::Transport(v) => v.as_ref() as *const _ as usize,
        RestoredComponent::City(v) => v.as_ref() as *const _ as usize,
        RestoredComponent::Hq(v) => v.as_ref() as *const _ as usize,
        _ => unreachable!(),
    }
}

#[test]
fn exact_native_constant_leaves_match_all18_typed_rows() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../research/skirmish-20261016-load-component-init/native-typed-constant-init.json",
    );
    let f: Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
    assert_eq!(f["rows"].as_array().unwrap().len(), 18);
    for r in f["rows"].as_array().unwrap() {
        let slot = ComponentSlot::from_index(r["slot"].as_u64().unwrap() as usize).unwrap();
        assert!(supports(slot));
        let v = component_virtuals::leaves(FactoryKind::Standard(slot));
        assert_eq!(v.init_existing, 0x6a71d0);
        assert_eq!(
            v.vf14,
            if slot == ComponentSlot::City {
                0x563fe0
            } else {
                0x4b2830
            }
        );
        assert_eq!(r["init_result"], true);
        assert_eq!(r["after_executed"], slot != ComponentSlot::City);
        assert_eq!(r["reads"], json!([]));
        assert_eq!(r["writes"], json!([]));
        assert_eq!(r["seams"], json!([]));
    }
    assert!(!supports(ComponentSlot::Projectile));
    assert!(!supports(ComponentSlot::Portal));
    assert!(!supports(ComponentSlot::Body));
}

#[test]
#[ignore = "requires AA_GAME_ROOT and verified capture03"]
fn actual_kursk_typed_constant_leaves_retain_same_allocations() {
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

    use std::collections::BTreeMap;
    let mut selected = BTreeMap::new();
    let mut census = BTreeMap::<ComponentSlot, usize>::new();
    for e in attempt.registry.iter() {
        let shell = match &e.value {
            RestoredRecord::Actor(v) => Some(&v.shell),
            RestoredRecord::Kingdom(v) => v.runtime.as_ref().map(|v| &v.base),
            _ => None,
        };
        if let Some(a) = shell.and_then(|v| v.actor.as_ref()) {
            for slot in aa_actors::components::ALL_SLOTS
                .into_iter()
                .filter(|s| supports(*s))
            {
                if a.components.get(slot).is_some() {
                    *census.entry(slot).or_default() += 1;
                    selected.entry(slot).or_insert(e.id);
                }
            }
        }
    }
    assert!(
        !selected.is_empty(),
        "actual map has no admitted typed allocation"
    );
    let catalog = Catalog::new(&actors);
    *catalog.definition_bank.borrow_mut() = attempt.definitions;
    let source_pc = catalog
        .definition_bank
        .borrow()
        .combat_source()
        .unwrap()
        .source_precision();
    let mut rows = Vec::new();
    for (slot, id) in selected {
        let mut held = std::mem::replace(
            attempt.registry.value_mut(id).unwrap(),
            RestoredRecord::Loading,
        );
        let (shell, definition) = match &mut held {
            RestoredRecord::Actor(v) => (&mut v.shell, &v.definition),
            RestoredRecord::Kingdom(v) => {
                let v = v.runtime.as_mut().unwrap();
                (&mut v.base, &v.definition)
            }
            _ => unreachable!(),
        };
        let mut record = Record {
            owner: id,
            shell,
            definition,
            registry: &mut attempt.registry,
        };
        let before = format!("{:?}", record.shell);
        let component = record
            .shell
            .actor
            .as_ref()
            .unwrap()
            .components
            .get(slot)
            .unwrap();
        let pointer = allocation(component);
        let mut pc_rows = Vec::new();
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            assert!(init_existing(&catalog, &mut record, slot, pc).unwrap());
            let after = after_init(&catalog, &mut record, slot);
            if slot == ComponentSlot::City {
                assert!(after.unwrap_err().to_string().contains("563FE0"));
            } else {
                after.unwrap();
            }
            assert_eq!(format!("{:?}", record.shell), before);
            assert_eq!(
                allocation(
                    record
                        .shell
                        .actor
                        .as_ref()
                        .unwrap()
                        .components
                        .get(slot)
                        .unwrap()
                ),
                pointer
            );
            pc_rows.push(json!({"precision":pc,"init_result":true,"after_result":if slot==ComponentSlot::City {"required563FE0"}else{"complete"},"same_allocation":true}));
        }
        assert!(
            init_existing(
                &Catalog::new(&actors),
                &mut record,
                slot,
                Precision::Double53
            )
            .is_err()
        );
        let mut changed = record.definition.clone();
        changed.component_mask = Some(0);
        {
            let mut wrong = Record {
                owner: id,
                shell: &mut *record.shell,
                definition: &changed,
                registry: &mut *record.registry,
            };
            assert!(init_existing(&catalog, &mut wrong, slot, Precision::Double53).is_err());
        }
        assert_eq!(format!("{:?}", record.shell), before);
        rows.push(json!({"slot":slot,"owner":id.raw(),"definition":record.definition.proof,"component_mask":record.definition.component_mask,"actual_allocation_count":census[&slot],"rows":pc_rows}));
        *attempt.registry.value_mut(id).unwrap() = held;
    }
    let result = json!({"scope":"Actual stopped Kursk typed constant component leaves only; source restore is diagnostic, no wholeActorInit/battle. Missing typed classes and absent map components are not invented.","map_sha256":sha256(&map_bytes),"source_precision":source_pc,"rows":rows});
    if let Some(p) = std::env::var_os("AA_TYPED_CONSTANT_CAPTURE") {
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
            "../../research/skirmish-20261016-load-component-init/actual-typed-constant-init.json",
        );
        let expected: Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
        assert_eq!(result, expected);
    }
}
