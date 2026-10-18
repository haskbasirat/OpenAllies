use super::*;
use serde_json::{Value, json};

#[test]
fn native_state_entry_null_components_lists_and_group_generations() {
    struct Fixture {
        input: Value,
        view: i32,
    }
    impl World for Fixture {
        fn component(&self, s: ComponentSlot) -> Result<bool> {
            Ok(self.input["positive_component"] == format!("{s:?}")
                || s == ComponentSlot::Body && self.input["body"].as_bool().unwrap())
        }
        fn state_properties_empty(&self) -> Result<bool> {
            Ok(true)
        }
        fn stance_count(&mut self) -> Result<usize> {
            Ok(self.input["stance_count"].as_u64().unwrap() as usize)
        }
        fn current_view(&self) -> Result<i32> {
            Ok(self.view)
        }
        fn old_property_is_null(&mut self, _: usize) -> Result<bool> {
            Ok(true)
        }
        fn set_view(&mut self, v: i32) -> Result<()> {
            self.view = v;
            Ok(())
        }
        fn active_fx_only(&self) -> Result<bool> {
            self.input["mode"]
                .as_u64()
                .map(|v| v != 0)
                .ok_or(Error::Unsupported("fixture reached unbound mode"))
        }
        fn enter_fx_empty(&mut self) -> Result<bool> {
            Ok(true)
        }
        fn remove_from_group(&self) -> Result<bool> {
            Ok(self.input["remove_group"].as_bool().unwrap())
        }
        fn group_reference(&self) -> Result<ActorId> {
            Ok(ActorId::from_raw(
                self.input["group"].as_u64().unwrap() as u32
            ))
        }
        fn resolve(&self, _: ActorId) -> Result<bool> {
            Ok(false)
        }
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-load-actor-state-reentry/native-state-entry.json");
    let doc: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    for row in doc["cases"].as_array().unwrap() {
        let mut w = Fixture {
            input: row["input"].clone(),
            view: row["input"]["old_view"].as_i64().unwrap() as i32,
        };
        enter(&mut w).unwrap();
        assert_eq!(json!(w.view), row["output"]["view"], "{row}");
    }
    // A required child before the stance store leaves it untouched. A required
    // mode read after the actual store retains that successful mutation.
    let base =
        json!({"body":false,"stance_count":1,"old_view":0,"mode":1,"remove_group":false,"group":0});
    let mut before = Fixture {
        input: base.clone(),
        view: 0,
    };
    before.input["positive_component"] = json!("Organization");
    assert!(
        enter(&mut before)
            .unwrap_err()
            .to_string()
            .contains("57C400")
    );
    assert_eq!(before.view, 0);
    let mut after = Fixture {
        input: base,
        view: 0,
    };
    after.input["mode"] = Value::Null;
    assert!(
        enter(&mut after)
            .unwrap_err()
            .to_string()
            .contains("unbound mode")
    );
    assert_eq!(after.view, -1);
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets"]
fn actual_saved_3172_state_entry_all_precisions_and_source_guards() {
    use crate::{
        combat_source_completion as completion,
        combat_source_receipt::VerifiedStartupReceipt,
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use aa_mapworld::restore::RestoredRecord;
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
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
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = std::sync::Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let map = aa_mapworld::MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut rows = Vec::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
            .unwrap()
            .begin(TaskKind::Normal)
            .unwrap();
        let mut attempt = Attempt::load_with_source_setup(
            &map,
            &actors,
            settings.clone(),
            text.clone(),
            Precision::Double53,
            u64::MAX,
            aa_mapworld::restore::RestoreMode::InspectPartial,
            LoadBindings {
                native_load_mode: Some(1),
                inactive_load: Some(inactive),
                resources: Some(std::sync::Arc::clone(&resources)),
            },
            |catalog| {
                let proof =
                    VerifiedStartupReceipt::verify_embedded_reference(catalog, &mut sources)?;
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
        catalog.native_load_mode = Some(1);
        catalog.definition_bank = std::cell::RefCell::new(std::mem::take(&mut attempt.definitions));
        let id = ActorId::from_raw(3172);
        let mut held = std::mem::replace(
            attempt.registry.value_mut(id).unwrap(),
            RestoredRecord::Loading,
        );
        let RestoredRecord::Kingdom(k) = &mut held else {
            panic!("actual3172 Kingdom")
        };
        let runtime = k.runtime.as_mut().unwrap();
        let mut record = Record {
            owner: id,
            shell: &mut runtime.base,
            definition: &runtime.definition,
            registry: &mut attempt.registry,
        };
        let bits = record.shell.actor.as_ref().unwrap().components.state;
        assert_ne!(bits.low | bits.high, 0);
        let before = record.shell.actor.as_ref().unwrap().words[&0x15c] as i32;
        let component_presence = aa_actors::components::ALL_SLOTS
            .into_iter()
            .map(|s| {
                (
                    format!("{s:?}"),
                    record
                        .shell
                        .actor
                        .as_ref()
                        .unwrap()
                        .components
                        .get(s)
                        .is_some(),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let group_reference = record.shell.actor.as_ref().unwrap().words[&0x164];
        let mut state_rows = Vec::new();
        for index in 0..42u32 {
            if (u64::from(bits.low) | (u64::from(bits.high) << 32)) & (1u64 << index) == 0 {
                continue;
            }
            let source = actors
                .native_actor_state(
                    aa_actors::state::DESCRIPTORS[index as usize].name,
                    Precision::Double53,
                )
                .unwrap();
            let mut forged = source.clone();
            forged.id.push_str("_forged");
            assert!(
                Context
                    .state_reenter(&catalog, &mut record, index, forged, pc)
                    .is_err()
            );
            let mut forged = source.clone();
            let TypedValue::Integer { value, .. } = &mut forged.priority else {
                panic!("typed priority")
            };
            *value = value.wrapping_add(1);
            assert!(
                Context
                    .state_reenter(&catalog, &mut record, index, forged, pc)
                    .unwrap_err()
                    .to_string()
                    .contains("canonical source")
            );
            let mut definition = record.definition.clone();
            definition.component_mask = definition.component_mask.map(|m| m ^ 1);
            let mut wrong = Record {
                owner: record.owner,
                shell: &mut *record.shell,
                definition: &definition,
                registry: &mut *record.registry,
            };
            assert!(
                Context
                    .state_reenter(&catalog, &mut wrong, index, source.clone(), pc)
                    .unwrap_err()
                    .to_string()
                    .contains("component-mask")
            );
            Context
                .state_reenter(&catalog, &mut record, index, source.clone(), pc)
                .unwrap();
            state_rows.push(json!({"index":index,"source":source}));
        }
        let after = record.shell.actor.as_ref().unwrap().words[&0x15c] as i32;
        let mut borrowed = Borrowed {
            catalog: &catalog,
            record: &mut record,
            source: actors
                .native_actor_state("mind_attacking", Precision::Double53)
                .unwrap(),
            source_pc: Precision::Double53,
            lists: None,
        };
        let stance_count = borrowed.stance_count().unwrap();
        let enter_empty = borrowed.enter_fx_empty().unwrap();
        rows.push(json!({"precision":pc,"id":3172,"definition":runtime.definition.proof.id,"state_bits":[bits.low,bits.high],"states":state_rows,"old_view":before,"view_after":after,"stance_count":stance_count,"enter_fx_empty":enter_empty,"component_presence":component_presence,"group_reference":group_reference,"map_resource_mode":catalog.native_load_mode,"source_pc":"Double53","scope":"actual decoded saved Kingdom; no full immediate Init/PostCreate/register replay"}));
        println!(
            "{pc:?}: actual3172 definition{} states{:x}/{:x} view{before}->{after}, stances{stance_count}",
            runtime.definition.proof.id, bits.low, bits.high
        );
    }
    if let Some(path) = std::env::var_os("AA_STATE_REENTRY_CAPTURE") {
        let out = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(out, &json!({"cases":rows})).unwrap();
    }
}
