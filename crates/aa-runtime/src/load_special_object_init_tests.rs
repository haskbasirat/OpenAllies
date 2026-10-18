use super::*;
use serde_json::{Value, json};

struct Services<'a> {
    input: &'a Value,
    trace: Vec<Value>,
    pending: [Vec<usize>; 2],
    freed: Vec<usize>,
    serial: usize,
    stop: Option<String>,
}
impl TailServices for Services<'_> {
    fn period(&mut self, slot: u8, _: &Component) -> Result<u32> {
        let bits = self.input["components"][usize::from(slot)]["period"]
            .as_u64()
            .unwrap() as u32;
        self.trace
            .push(json!({"call":"period","slot":slot,"bits":bits}));
        if self.input["fail"] == format!("definition:{slot}") {
            self.stop = Some(format!("definition:{slot}"));
            return Err(Error::Unsupported("explicit missing definition probe"));
        }
        Ok(bits)
    }
    fn schedule(&mut self, slot: u8, _: &mut Component, bits: u32) -> Result<()> {
        if self.input["fail"] == format!("schedule:{slot}") {
            self.trace
                .push(json!({"call":"schedule_required","slot":slot,"delay":bits}));
            self.stop = Some(format!("schedule:{slot}"));
            return Err(Error::Unsupported("explicit missing queue probe"));
        }
        self.trace
            .push(json!({"call":"schedule","slot":slot,"delay":bits,"data":0}));
        self.serial += 1;
        self.pending[usize::from(slot)].insert(0, self.serial);
        Ok(())
    }
    fn clear(&mut self, slot: u8, _: &mut Component) -> Result<()> {
        self.trace.push(json!({"call":"clear","slot":slot}));
        self.freed.append(&mut self.pending[usize::from(slot)]);
        Ok(())
    }
    fn legacy_area(&mut self, _: &mut Area) -> Result<()> {
        self.trace.push(json!({"call":"legacy"}));
        self.stop = Some("legacy".into());
        Err(Error::Unsupported("explicit legacy scheduling probe"))
    }
}
fn u32_at(v: &Value, key: &str) -> u32 {
    v[key].as_u64().unwrap() as u32
}
fn component(slot: u8, state: u32) -> Component {
    Component {
        owner: 28,
        definition_owner: "explicit-test-Doodad".into(),
        slot,
        scheduled_head: None,
        registered_updatable_id: None,
        words: if slot == 0 {
            vec![state]
        } else {
            vec![state, 0, 0, 0, 0, 0x3f800000]
        },
    }
}

#[test]
fn original_v14_all_pc_special_init_tails_preserve_trace_rng_and_owned_lists() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-special-object-init/native-special-tail.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 126);
    for row in rows {
        let input = &row["input"];
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => unreachable!(),
        };
        let mut state = State {
            camera_generation: input["counters"][0].as_u64().unwrap() as u32,
            area_generation: input["counters"][1].as_u64().unwrap() as u32,
        };
        let mut rng = NativeRng {
            state: u32_at(input, "rng"),
        };
        let mut area = Area {
            scheduled_head: input["area_head"].as_bool().unwrap().then_some(1),
            ..Area::default()
        };
        let mut camera = Camera::default();
        let mut doodad = DynamicDoodad::default();
        let mut services = Services {
            input,
            trace: vec![json!({"call":"object","value":input["base_success"]})],
            pending: [vec![], vec![]],
            freed: vec![],
            serial: 0,
            stop: None,
        };
        for slot in 0..2usize {
            let c = &input["components"][slot];
            if c.is_null() {
                continue;
            }
            doodad.components[slot] = Some(component(slot as u8, u32_at(c, "state")));
            for _ in 0..c["pending"].as_u64().unwrap() {
                services.serial += 1;
                services.pending[slot].insert(0, services.serial);
            }
        }
        let result = if !input["base_success"].as_bool().unwrap() {
            Ok(false)
        } else {
            let derived = match u32_at(input, "class_id") {
                6 => Derived::Area(&mut area),
                7 => Derived::Camera(&mut camera),
                9 => Derived::Doodad(&mut doodad),
                _ => unreachable!(),
            };
            tail(
                derived,
                &mut state,
                &u32_at(input, "version"),
                &mut rng,
                pc,
                &mut services,
            )
        };
        assert_eq!(result.is_err(), services.stop.is_some(), "{input}");
        let actual = json!({"returned":result.ok(),"stop":services.stop,
            "trace":services.trace,"counters":[state.camera_generation,state.area_generation],
            "rng":rng.state,"pending":services.pending,"freed":services.freed});
        assert_eq!(actual, row["output"], "{} / {pc:?}", input["name"]);
    }
}

#[test]
fn real_pre_store_queue_and_owner_allocation_survive_tail_failures() {
    use crate::scheduler_owners::Target;
    use aa_world::queues::{Clock, Kind, Queues};
    let actors =
        aa_rules::actors::ActorCatalog::from_files(vec![], aa_rules::tgi::Limits::default());
    let mut queues = Queues::new(
        Clock {
            begin_bits: 0,
            end_bits: 0x3dcccccd,
            step_bits: 0x3dcccccd,
        },
        Precision::Double53,
    )
    .unwrap();
    queues.initialize(3, Kind::Deferred, 100).unwrap();
    let catalog = Catalog::new(&actors);
    catalog.load_queues.replace(Some(&mut queues));
    let owner = aa_actors::ActorId::from_raw(28);
    let mut services = Owned {
        catalog: &catalog,
        owner,
        canonical: "explicit-test-Doodad",
        live_pc: Precision::Double53,
    };
    let mut first = component(0, 0);
    let mut second = component(1, 2);
    services.schedule(0, &mut first, 0x3f000000).unwrap();
    let target = Target::Restoring(UpdatableTarget::DynamicDoodad { owner: 28, slot: 0 });
    let first_token = catalog.load_owners.borrow().get(target).unwrap();
    let first_head = catalog
        .load_queues
        .borrow()
        .as_ref()
        .unwrap()
        .owner_head(first_token)
        .unwrap();
    // A later source error leaves the actual first event and token untouched.
    assert!(services.period(1, &second).is_err());
    assert_eq!(
        catalog.load_owners.borrow().get(target).unwrap(),
        first_token
    );
    assert_eq!(
        catalog
            .load_queues
            .borrow()
            .as_ref()
            .unwrap()
            .owner_head(first_token),
        Some(first_head)
    );
    services.schedule(1, &mut second, 0).unwrap();
    let second_token = catalog
        .load_owners
        .borrow()
        .get(Target::Restoring(UpdatableTarget::DynamicDoodad {
            owner: 28,
            slot: 1,
        }))
        .unwrap();
    assert_ne!(first_token, second_token);
    assert!(
        catalog
            .load_queues
            .borrow()
            .as_ref()
            .unwrap()
            .has_owner(second_token)
    );
    // Constructor/load head receipt is not the scheduler's current head.
    first.scheduled_head = Some(0xdeadbeef);
    services.clear(0, &mut first).unwrap();
    assert!(
        !catalog
            .load_queues
            .borrow()
            .as_ref()
            .unwrap()
            .has_owner(first_token)
    );
    assert!(
        catalog
            .load_queues
            .borrow()
            .as_ref()
            .unwrap()
            .has_owner(second_token)
    );
    assert_eq!(first.scheduled_head, Some(0xdeadbeef));
    let mut absent = component(0, 0);
    services.live_pc = Precision::Single24;
    assert!(services.schedule(0, &mut absent, 0).is_err());
    assert!(
        !catalog
            .load_queues
            .borrow()
            .as_ref()
            .unwrap()
            .has_owner(first_token)
    );
}

#[test]
fn persistent_counter_constructor_and_wrapping_mutations() {
    let mut state = State::from_world_constructor();
    assert_eq!((state.camera_generation(), state.area_generation()), (0, 0));
    state.camera_generation = u32::MAX;
    state.area_generation = u32::MAX;
    state.camera_changed();
    state.area_changed();
    assert_eq!((state.camera_generation(), state.area_generation()), (0, 0));
}

#[test]
fn actual_derived_constructor_payloads_and_world_generations_match_native() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-special-object-init/native-special-classes.json"
    ))
    .unwrap();
    let mut checked = 0;
    for row in fixture["cases"].as_array().unwrap() {
        let out = &row["output"];
        match row["operation"].as_str().unwrap() {
            "world_counter_constructor" => {
                let state = State::from_world_constructor();
                assert_eq!(
                    json!([state.camera_generation(), state.area_generation()]),
                    *out
                );
                checked += 1;
            }
            "constructor" => {
                match u32_at(row, "class_id") {
                    6 => {
                        let mut area = Area::default();
                        assert_eq!(Derived::Area(&mut area).class(), 6);
                        assert_eq!(out["head"], 0);
                        assert!(area.scheduled_head.is_none());
                        assert_eq!(json!(area.shape_words), out["shape"]);
                        assert!(
                            out["empty_label"].as_bool().unwrap() && area.label_utf16.is_empty()
                        );
                        assert_eq!(out["entry_head"], 0);
                        assert_eq!(out["entry_tail"], 0);
                        assert!(area.entries.is_empty());
                    }
                    7 => {
                        let mut camera = Camera::default();
                        assert_eq!(Derived::Camera(&mut camera).class(), 7);
                        assert_eq!(json!(camera.word_60), out["word60"]);
                        assert!(
                            out["empty_label"].as_bool().unwrap() && camera.label_utf16.is_empty()
                        );
                    }
                    9 => {
                        let mut doodad = DynamicDoodad::default();
                        assert_eq!(Derived::Doodad(&mut doodad).class(), 9);
                        assert_eq!(out["components"], json!([0, 0]));
                        assert!(doodad.components.iter().all(Option::is_none));
                    }
                    _ => unreachable!(),
                }
                checked += 1;
            }
            "memory" => {} // replayed by the owning live Object source tests.
            _ => unreachable!(),
        }
    }
    assert_eq!(checked, 15);
}
