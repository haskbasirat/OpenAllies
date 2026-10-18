use super::*;
use aa_actors::{
    loading::{construct_shell_with_components, LoadKind},
    ActorId,
};
use aa_body::{credit::Object, properties::PropertiesState, F32};
use serde_json::{json, Value};

fn fixture() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-load-component-init/native-body-prefix.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn native_body_healthy_and_required_service_prefixes_match216_rows() {
    let f = fixture();
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 216);
    for row in rows {
        let c = &row["input"];
        let expected = &row["native"];
        let owner = ActorId::from_raw(0x10001);
        let mut shell =
            construct_shell_with_components::<(), RestoredComponent>(LoadKind::Actor, ());
        let actor = shell.actor.as_mut().unwrap();
        actor.components.state.low = c["state_low"].as_u64().unwrap() as u32;
        let mut v = BodyState::new(
            owner,
            F32(c["maximum_bits"].as_u64().unwrap() as u32),
            F32::ONE,
        )
        .unwrap();
        v.body.health = F32(c["health_bits"].as_u64().unwrap() as u32);
        let before = v.clone();
        actor
            .components
            .insert(ComponentSlot::Body, RestoredComponent::Body(Box::new(v)));
        if !c["construction"].is_null() {
            let mut construction = aa_actors::construction_state::ConstructionState::new(
                owner,
                &aa_actors::construction_state::Definition {
                    base_cost_words: None,
                    empty_predecessor_chain: true,
                },
            )
            .unwrap();
            construction.definition_28 = c["construction"].as_bool().unwrap().then_some(1);
            actor.components.insert(
                ComponentSlot::Construction,
                RestoredComponent::Construction(Box::new(construction)),
            );
        }
        if c["properties"].as_bool().unwrap() {
            actor.components.insert(
                ComponentSlot::Properties,
                RestoredComponent::Properties(Box::new(
                    PropertiesState::new(Object(owner.raw()), 2).unwrap(),
                )),
            );
        }
        let low_before = actor.components.state.low;
        let result = prefix(&mut shell);
        let outcome = match &result {
            Ok(true) => "complete",
            Ok(false) => panic!("Body native leaf never returns false"),
            Err(e) if e.to_string().contains("561D40") => "required_561D40",
            Err(e) if e.to_string().contains("561E50") => "required_561E50",
            Err(e) => panic!("unexpected Body prefix error:{e}"),
        };
        assert_eq!(outcome, expected["outcome"].as_str().unwrap());
        let actual = body(&shell).unwrap();
        assert_eq!(json!(actual.body.health.0), expected["health_bits"]);
        assert_eq!(json!(actual.body.maximum.0), expected["maximum_bits"]);
        let mut retained = before;
        retained.body.health = actual.body.health;
        assert_eq!(
            actual, &retained,
            "only the native health prefix store may change"
        );
        assert_eq!(
            shell.actor.as_ref().unwrap().components.state.low,
            low_before
        );
        if outcome == "required_561E50" {
            assert_eq!(expected["writes"].as_array().unwrap().len(), 1);
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("preceding health=max store retained"));
        } else {
            assert_eq!(expected["writes"], json!([]));
        }
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT and verified capture03"]
fn actual_healthy_skirmish_body_keeps_same_actor_and_allocation() {
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
        sources::{sha256, SourceLimits, V14Sources},
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
    // First eligible ACTUAL healthy Body actor in native registry order, without
    // modifying health/state to manufacture a completed Init path.
    let id = attempt
        .registry
        .iter()
        .find_map(|entry| {
            let RestoredRecord::Actor(v) = &entry.value else {
                return None;
            };
            let actor = v.shell.actor.as_ref()?;
            let RestoredComponent::Body(b) = actor.components.get(ComponentSlot::Body)? else {
                return None;
            };
            let state = actor.components.state.low;
            let target = match actor.components.get(ComponentSlot::Construction) {
                None => false,
                Some(RestoredComponent::Construction(c)) => {
                    state & (1 << 29) != 0 && c.definition_28.is_some()
                }
                _ => return None,
            };
            ((state & (1 << 21) != 0 || b.body.health == b.body.maximum) && !target)
                .then_some(entry.id)
        })
        .expect("actual healthy Body actor required");
    let mut held = std::mem::replace(
        attempt.registry.value_mut(id).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Actor(entity) = &mut held else {
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
        shell: &mut entity.shell,
        definition: &entity.definition,
        registry: &mut attempt.registry,
    };
    let before = body(record.shell).unwrap().clone();
    let pointer = body(record.shell).unwrap() as *const _ as usize;
    let shell_before = format!("{:?}", record.shell);
    let mut rows = Vec::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        assert!(init_existing(&catalog, &mut record, pc).unwrap());
        after_init(&catalog, &mut record).unwrap();
        assert_eq!(body(record.shell).unwrap(), &before);
        assert_eq!(body(record.shell).unwrap() as *const _ as usize, pointer);
        assert_eq!(format!("{:?}", record.shell), shell_before);
        rows.push(json!({"precision":pc,"complete":true,"body":body(record.shell).unwrap(),"same_allocation":true}));
    }
    assert!(init_existing(&Catalog::new(&actors), &mut record, Precision::Double53).is_err());
    assert_eq!(format!("{:?}", record.shell), shell_before);
    let result = json!({"scope":"first eligible actual healthy Body actor in stopped Kursk diagnostic; not wholeActorInit or battle readiness",
        "owner":id.raw(),"map":"(2v2)Kursk","map_sha256":sha256(&map_bytes),"source_precision":source_pc,
        "definition":record.definition.proof,"state_low":low(record.shell).unwrap(),"rows":rows});
    if let Some(path) = std::env::var_os("AA_BODY_INIT_CAPTURE") {
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
            .join("../../research/skirmish-20261016-load-component-init/actual-healthy-body.json");
        let expected: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(result, expected);
    }
    *attempt.registry.value_mut(id).unwrap() = held;
}
