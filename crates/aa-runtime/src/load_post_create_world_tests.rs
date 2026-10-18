use super::*;
use crate::{
    combat_source_completion as completion,
    combat_source_receipt::VerifiedStartupReceipt,
    localization::Localization,
    restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind},
    settings::Settings,
};
use aa_mapworld::restore::RestoreMode;
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use std::sync::Arc;
struct Missing;
impl Effects for Missing {}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn actual_first_saved_kingdom_postcreate_and_empty_created_fx_use_the_held_allocation() {
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
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
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
            let proof = VerifiedStartupReceipt::verify_embedded_reference(catalog, &mut sources)?;
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
    let mut catalog = Catalog::new(&actors);
    catalog.definition_bank = std::cell::RefCell::new(std::mem::take(&mut attempt.definitions));
    let id = ActorId::from_raw(781);
    let mut held = std::mem::replace(
        attempt.registry.value_mut(id).unwrap(),
        RestoredRecord::Loading,
    );
    let RestoredRecord::Kingdom(entity) = &mut held else {
        panic!("actual first Kingdom")
    };
    let runtime = entity.runtime.as_mut().unwrap();
    assert_eq!(runtime.definition.proof.id, "kingdom");
    let address = &runtime.base as *const _ as usize;
    let before = format!("{:?}", runtime.base);
    let mut record = Record {
        owner: id,
        shell: &mut runtime.base,
        definition: &runtime.definition,
        registry: &mut attempt.registry,
    };
    let present = aa_actors::components::ALL_SLOTS
        .iter()
        .filter(|s| {
            record
                .shell
                .actor
                .as_ref()
                .unwrap()
                .components
                .get(**s)
                .is_some()
        })
        .map(|s| s.index())
        .collect::<Vec<_>>();
    assert_eq!(present, vec![6]);
    assert_eq!(
        *record
            .shell
            .actor
            .as_ref()
            .unwrap()
            .words
            .get(&0xe8)
            .unwrap(),
        0
    );
    let data = catalog.layout_data(Precision::Double53).unwrap();
    assert!(
        layout_data::native_actor_properties(&actors, &data, "kingdom", Precision::Double53)
            .unwrap()
            .is_empty()
    );
    let fx = crate::definition_fx::Source::bind(&catalog, record.definition).unwrap();
    assert_eq!(fx.canonical_id(), "kingdom");
    assert_eq!(fx.source_precision(), Precision::Double53);
    for list in crate::definition_fx::ListKind::ALL {
        assert!(fx.list(list).is_certified_empty());
    }
    let mut outcomes = Vec::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        post_create_record(&catalog, &mut record, &mut Missing, pc).unwrap();
        publish_created_fx_record(&catalog, &mut record, &mut Missing, pc).unwrap();
        assert_eq!(record.shell as *const _ as usize, address);
        assert_eq!(format!("{:?}", record.shell), before);
        outcomes.push(serde_json::json!({"precision":format!("{pc:?}"),"id":id.raw(),"definition":"kingdom","components":present,"stored_owner":0,"initial_property_count":0,"created_fx_count":0,"same_allocation":true,"post_create_completed":true,"definition_fx_null_head_completed":true}));
    }
    record.registry.remove(id).unwrap();
    assert!(
        post_create_record(&catalog, &mut record, &mut Missing, Precision::Double53)
            .unwrap_err()
            .contains("Loading")
    );
    assert_eq!(format!("{:?}", record.shell), before);
    if let Some(path) = std::env::var_os("AA_POST_CREATE_CAPTURE") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file,&serde_json::json!({"scope":"Actual held saved Kingdom PostCreate and empty-source FX guard only; concrete Init was deliberately omitted by this prefix diagnostic and no world/battle readiness is implied","cases":outcomes})).unwrap();
    }
}

#[test]
fn native_saved_flag_one_caller_order_and_partial_effects_match_both_images_all_precision() {
    use aa_actors::world_publication::{CreateWorld, Error, Result as R};
    use serde_json::{Value, json};
    struct World {
        input: Value,
        trace: Vec<Value>,
        flags: u32,
    }
    impl CreateWorld for World {
        type Record = ();
        fn has_component(&self, _: ActorId, s: ComponentSlot) -> R<bool> {
            Ok(self.input["present"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_u64() == Some(s.index() as u64)))
        }
        fn component_post_create(
            &mut self,
            _: ActorId,
            s: ComponentSlot,
            _: &(),
            flag: u32,
        ) -> R<()> {
            assert_eq!(flag, 1);
            if s == ComponentSlot::Properties {
                for p in self.input["properties"].as_array().unwrap() {
                    self.trace.push(json!(["property", p, 0, 0]));
                    if self.input["stop_property"].as_u64() == Some(self.trace.len() as u64) {
                        return Err(Error::Unsupported("required stop"));
                    }
                    self.flags ^= 1;
                }
            }
            Ok(())
        }
        fn stored_owner(&self, _: ActorId) -> R<Option<ActorId>> {
            Ok(self.input["owner"]
                .as_bool()
                .unwrap_or(false)
                .then(|| ActorId::from_raw(0x1234)))
        }
        fn owner_post_create_4e6180(&mut self, _: ActorId, _: ActorId) -> R<()> {
            self.trace.push(json!(["owner", 0x1234]));
            if self.input["stop_owner"].as_bool() == Some(true) {
                return Err(Error::Unsupported("required stop"));
            }
            self.flags |= 1 << 28;
            Ok(())
        }
        fn map_mode_a8(&self) -> R<u32> {
            panic!("flag1 read map mode")
        }
        fn object_ground_post_create(&mut self, _: ActorId, _: &()) -> R<()> {
            panic!("flag1 ground")
        }
        fn definition_mask(&self, _: ActorId) -> R<u32> {
            panic!("flag1 city mask")
        }
        fn city_predicate_45d6a0(&mut self, _: ActorId) -> R<bool> {
            panic!("flag1 city predicate")
        }
        fn city_argument_34(&self, _: &()) -> R<u32> {
            panic!("flag1 null params")
        }
        fn city_post_create_5644f0(&mut self, _: ActorId, _: u32) -> R<()> {
            panic!("flag1 city")
        }
    }
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-post-create/native-post-create.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in fixture["cases"].as_array().unwrap() {
        if !matches!(row["input"]["class"].as_u64(), Some(3 | 4)) {
            continue;
        }
        let mut w = World {
            input: row["input"].clone(),
            trace: vec![],
            flags: 0x80000000,
        };
        let result = world_publication::post_create(&mut w, ActorId::from_raw(9), &(), 1);
        assert_eq!(result.is_ok(), row["output"]["status"] == "ok", "{row}");
        assert_eq!(json!(w.trace), row["output"]["trace"], "{row}");
        assert_eq!(
            u64::from(w.flags),
            row["output"]["flags"].as_u64().unwrap(),
            "{row}"
        );
        count += 1;
    }
    assert_eq!(count, 120);
}
