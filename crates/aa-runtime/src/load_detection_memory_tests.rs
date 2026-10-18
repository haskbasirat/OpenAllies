use super::*;
use aa_actors::detection_memory::State;
use serde_json::{Value, json};
fn word(v: &Value, name: &str) -> u32 {
    v[name].as_u64().unwrap() as u32
}
struct Probe<'a> {
    input: &'a Value,
    state: State,
    trace: Vec<Value>,
    stop: Option<&'static str>,
}
impl World for Probe<'_> {
    fn keeps(&mut self) -> Result<bool> {
        let value = self.state.count() > 0;
        self.trace.push(json!(["keeps", value]));
        Ok(value)
    }
    fn eligible(&mut self) -> Result<bool> {
        let value = self.input["eligible"].as_bool().unwrap();
        self.trace.push(json!(["eligible", value]));
        Ok(value)
    }
    fn kingdom_count(&mut self) -> Result<i32> {
        Ok(self.input["kingdoms"].as_array().unwrap().len() as i32)
    }
    fn kingdom_slot(&mut self, index: i32) -> Result<Option<ActorId>> {
        let k = &self.input["kingdoms"][index as usize];
        Ok((!k.is_null()).then(|| ActorId::from_raw(word(k, "id"))))
    }
    fn nonmajor(&mut self, kingdom: ActorId) -> Result<u32> {
        Ok(word(self.k(kingdom), "nonmajor"))
    }
    fn kingdom_index(&mut self, kingdom: ActorId) -> Result<u32> {
        Ok(word(self.k(kingdom), "index"))
    }
    fn slot(&mut self, index: u32) -> Result<Option<RecordId>> {
        self.state.slot(index).map_err(error)
    }
    fn shared_visible(&mut self, k: ActorId) -> Result<bool> {
        Ok(self.nonmajor(k)? != 0
            || word(self.k(k), "shared") & word(self.input, "word") & 0xffff0000 != 0)
    }
    fn shared_explored(&mut self, k: ActorId) -> Result<bool> {
        Ok(self.nonmajor(k)? != 0
            || word(self.k(k), "shared") & word(self.input, "word") & 0xffff != 0)
    }
    fn flags(&mut self) -> Result<u32> {
        Ok(word(self.input, "flags"))
    }
    fn create(&mut self, k: ActorId) -> Result<()> {
        self.trace.push(json!(["create", k.raw()]));
        if self.input["fail"] == "create" {
            self.stop = Some("create");
            return Err(Error::Unsupported("explicit create virtual seam"));
        }
        if self.state.count() == 0 {
            let count = self.kingdom_count()?;
            self.state.allocate_slots(count).unwrap();
        }
        let index = self.kingdom_index(k)?;
        let record = self.state.allocate_record().unwrap();
        self.state.set_slot(index, Some(record)).unwrap();
        Ok(())
    }
    fn delete(&mut self, k: ActorId) -> Result<()> {
        self.trace.push(json!(["delete", k.raw()]));
        if self.input["fail"] == "delete" {
            self.stop = Some("delete");
            return Err(Error::Unsupported("explicit delete virtual seam"));
        }
        let index = self.kingdom_index(k)?;
        let record = self.state.slot(index).unwrap().unwrap();
        self.state.free_record(record).unwrap();
        self.state.set_slot(index, None).unwrap();
        Ok(())
    }
    fn count(&mut self) -> Result<i32> {
        Ok(self.state.count())
    }
    fn clear(&mut self) -> Result<()> {
        self.trace.push(json!(["clear"]));
        self.state.release_slots().unwrap();
        Ok(())
    }
}
impl Probe<'_> {
    fn k(&self, id: ActorId) -> &Value {
        self.input["kingdoms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|k| !k.is_null() && word(k, "id") == id.raw())
            .unwrap()
    }
}
fn fixture() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-load-detection-memory/native-memory.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn native_original_v14_refresh_all_pc_matches_current_owned_slot_mutations_and_calls() {
    let f = fixture();
    let mut checked = 0;
    for row in f["cases"].as_array().unwrap() {
        if row["operation"] != "refresh" {
            continue;
        }
        let input = &row["input"];
        let mut state = State::from_object_constructor();
        if let Some(slots) = input["slots"].as_array() {
            state.allocate_slots(slots.len() as i32).unwrap();
            for (index, value) in slots.iter().enumerate() {
                if value.as_bool().unwrap() {
                    let record = state.allocate_record().unwrap();
                    state.set_slot(index as u32, Some(record)).unwrap();
                }
            }
        }
        let mut world = Probe {
            input,
            state,
            trace: vec![],
            stop: None,
        };
        let result = refresh_inner(&mut world);
        assert_eq!(result.is_err(), world.stop.is_some());
        let slots = if world.state.allocation_present() {
            Some(
                (0..world.state.count())
                    .map(|i| world.state.slot(i as u32).unwrap().is_some())
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };
        let actual = json!({"trace":world.trace,"stop":world.stop,"count":world.state.count(),"slots":slots});
        assert_eq!(
            actual, row["output"],
            "{} / {}",
            input["name"], row["precision"]
        );
        checked += 1;
    }
    assert_eq!(checked, 225);
}

#[test]
fn native_slot_and_record_constructor_keep_uninitialized_fields_and_partial_publication() {
    let f = fixture();
    let mut checked = 0;
    for row in f["cases"].as_array().unwrap() {
        match row["operation"].as_str().unwrap() {
            "slots" => {
                let mut state = State::from_object_constructor();
                let n = row["count"].as_i64().unwrap() as i32;
                state.allocate_slots(n).unwrap();
                assert_eq!(json!(state.count()), row["output"]["count"]);
                assert_eq!(
                    json!(state.allocation_present()),
                    row["output"]["allocated"]
                );
                assert_eq!(
                    json!(
                        (0..n)
                            .map(|i| u32::from(state.slot(i as u32).unwrap().is_some()))
                            .collect::<Vec<_>>()
                    ),
                    row["output"]["slots"]
                );
            }
            "record" | "create_prefix" => {
                let mut state = State::from_object_constructor();
                if row["operation"] == "create_prefix" {
                    state.allocate_slots(3).unwrap();
                }
                let id = state.allocate_record().unwrap();
                let actual = state.record_mut(id).unwrap();
                let expected = if row["operation"] == "record" {
                    &row["output"]["words"]
                } else {
                    actual.timestamp_bits = 0x41280000;
                    &row["output"]["record_words"]
                };
                assert_eq!(json!(actual.timestamp_bits), expected[0]);
                assert!(actual.name_utf16.is_empty());
                assert!(row["output"]["empty_name"].as_bool().unwrap());
                assert_eq!(
                    json!(actual.position_bits),
                    json!([expected[2], expected[3]])
                );
                assert_eq!(expected[4], 0);
                assert!(actual.definition.is_none());
                assert_eq!(expected[5], actual.owner.raw());
                assert_eq!(expected[6], actual.aspect);
                assert_eq!(
                    json!([
                        actual.stat_words[0],
                        actual.stat_words[1],
                        actual.stat_words[4]
                    ]),
                    json!([expected[7], expected[8], expected[11]])
                );
                assert_eq!(actual.stat_words[2..4], [None, None]);
                if row["operation"] == "record" {
                    assert_eq!(expected[9], 0xcdcdcdcdu32);
                    assert_eq!(expected[10], 0xcdcdcdcdu32);
                }
                assert_eq!(expected[12].as_u64().unwrap() as u8, actual.byte_30);
                assert_eq!(expected[13], actual.flags);
                if row["operation"] == "create_prefix" {
                    assert_eq!(row["output"]["stop"], "name");
                    assert_eq!(state.count(), 3);
                    assert_eq!(state.allocated_records(), 1);
                    assert!((0..3).all(|i| state.slot(i).unwrap().is_none()));
                }
            }
            "delete_record" | "release_slots" => {
                let mut state = State::from_object_constructor();
                state.allocate_slots(1).unwrap();
                let freed = if row["operation"] == "delete_record" {
                    let id = state.allocate_record().unwrap();
                    state.set_slot(0, Some(id)).unwrap();
                    state.free_record(id).unwrap();
                    state.set_slot(0, None).unwrap();
                    assert!(state.record(id).is_err());
                    vec!["record"]
                } else {
                    state.release_slots().unwrap();
                    vec!["slots"]
                };
                assert_eq!(
                    json!({"count":state.count(),"allocated":state.allocation_present(),
                    "slot":state.allocation_present() && state.slot(0).unwrap().is_some(),"freed":freed}),
                    row["output"]
                );
            }
            _ => continue,
        }
        checked += 1;
    }
    assert_eq!(checked, 24);
}

#[test]
#[ignore = "requires AA_GAME_ROOT actual source/session/immediate-load owners"]
fn actual_first_object_entry_uses_the_real_world_major_vector_and_current_memory_owner() {
    use crate::{
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
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
    let inactive = ScopedManagedLoad::from_sources(&mut sources, 0)
        .unwrap()
        .begin(TaskKind::Normal)
        .unwrap();
    //Actual retained Session startup E4/F8 is the same bounded root fixture;
    //the rest of its Default is not a full native Session constructor claim.
    let session = aa_netcode::lobby::Session::default();
    //aa-netcode depends on the non-test aa-runtime crate. This local borrow
    //adapts the same Session to the lib-test trait instance, without a copy.
    struct SessionRef<'a>(&'a aa_netcode::lobby::Session);
    impl crate::load_world_globals::SessionPhase for SessionRef<'_> {
        fn current_phase(&self) -> std::result::Result<u32, String> {
            Ok(self.0.field_f8)
        }
        fn application_state(&self) -> std::result::Result<u32, String> {
            Ok(self.0.state)
        }
    }
    let session_ref = SessionRef(&session);
    CAPTURE_ENABLED.set(true);
    ACTUAL_CAPTURE.with(|capture| capture.borrow_mut().take());
    let attempt = Attempt::load_immediate(
        &map,
        &actors,
        settings,
        text,
        aa_numeric::Precision::Double53,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::StopOnError,
        LoadBindings {
            native_load_mode: Some(1),
            inactive_load: Some(inactive),
            resources: Some(resources),
        },
        &mut sources,
        &session_ref,
        false,
    )
    .unwrap();
    CAPTURE_ENABLED.set(false);
    let captured = ACTUAL_CAPTURE
        .with(|capture| capture.borrow_mut().take())
        .unwrap_or_else(|| {
            panic!(
                "actual Object28 was not reached: {:?}",
                attempt.report.events
            )
        });
    assert!(captured.get("capture_error").is_none(), "{captured}");
    assert_eq!(captured["result"], json!({"Ok":null}));
    let input = &captured["input"];
    assert_eq!(input["owner"], 28);
    assert_eq!(input["source_precision"], "Double53");
    assert_eq!(
        input["kingdoms"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|k| !k.is_null())
            .count(),
        2
    );
    assert_eq!(captured["after"]["flags"], input["flags"]);
    assert_eq!(captured["after"]["word"], input["word"]);
    assert_eq!(captured["after"]["allocated_records"], 0);
    assert!(
        attempt.report.lifecycle.iter().any(|e| e.saved_id == 28
            && e.calls
                .iter()
                .any(|c| c.service == "init_null" && c.completed)),
        "Object28 derived Init must complete after the actual visibility call"
    );
    if let Some(path) = std::env::var_os("AA_CAPTURE_MEMORY") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file, &captured).unwrap();
    } else {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../research/skirmish-20261016-load-detection-memory/actual-first-object-entry.json",
        );
        let expected: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(captured, expected);
    }
}
