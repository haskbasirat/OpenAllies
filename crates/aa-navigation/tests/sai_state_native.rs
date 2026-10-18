use aa_navigation::{
    sai_state::*,
    startup::Object,
    topology::{GridLayout, Topology},
};
use serde_json::Value;
fn fixture() -> Value {
    serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/phase7-sai/native-state.json"),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn concrete_region_kingdom_state_matches_original_allocations_and_masks() {
    let doc = fixture();
    let rows = doc["fixtures"].as_array().unwrap();
    assert_eq!(rows.len(), 21);
    for row in rows {
        let c = &row["input"];
        let v = &c["layout"];
        let bytes = c["input_hex"]
            .as_str()
            .unwrap()
            .as_bytes()
            .chunks_exact(2)
            .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
            .collect::<Vec<_>>();
        let layout = GridLayout {
            width: v["width"].as_i64().unwrap() as i32,
            height: v["height"].as_i64().unwrap() as i32,
            coordinate_shift: v["coordinate_shift"].as_u64().unwrap() as u32,
            extent_bits: std::array::from_fn(|i| {
                (v["extent_i32"][i].as_i64().unwrap() as f32).to_bits()
            }),
        };
        let (topology, _) = Topology::decode_prefix(&bytes, layout, 28).unwrap();
        let tiles: Vec<TileKnowledge> = serde_json::from_value(c["tiles"].clone()).unwrap();
        let settings: VisibilitySettings = serde_json::from_value(c["settings"].clone()).unwrap();
        let kingdoms: Vec<Kingdom> = serde_json::from_value(c["kingdoms"].clone()).unwrap();
        let mut state = State::new(topology.regions.len()).unwrap();
        let mut outputs = Vec::new();
        for kingdom in kingdoms {
            state
                .register_kingdom_regions(kingdom, &topology, &tiles, settings)
                .unwrap();
            for region in &mut state.regions {
                if let Some(record) = region
                    .kingdoms
                    .get_mut(kingdom.major_index as usize)
                    .and_then(Option::as_mut)
                {
                    for key in [17, 32, 17] {
                        record.register_seen(Object(key)).unwrap();
                    }
                }
            }
            outputs.push(serde_json::to_value(&state).unwrap());
        }
        assert_eq!(
            serde_json::to_value(outputs).unwrap(),
            row["native"],
            "{}",
            c["name"]
        );
    }
}
#[test]
fn native_actor_classification_and_seen_lookup_match() {
    let doc = fixture();
    assert_eq!(doc["classifications"].as_array().unwrap().len(), 88);
    for row in doc["classifications"].as_array().unwrap() {
        let fields: ActorClassFields = serde_json::from_value(row["input"].clone()).unwrap();
        assert_eq!(
            classification_three(fields),
            row["native"].as_bool().unwrap()
        );
    }
    assert_eq!(doc["seen_lookups"].as_array().unwrap().len(), 48);
    for row in doc["seen_lookups"].as_array().unwrap() {
        let kingdom = Kingdom {
            object: Object(1),
            native_id: 1,
            major_index: row["index"].as_i64().unwrap() as i32,
            category: row["category"].as_u64().unwrap() as u32,
        };
        let records: Vec<Option<Object>> = serde_json::from_value(row["records"].clone()).unwrap();
        let result = seen_record(
            Some(kingdom),
            actor_has_seen_records(row["count58"].as_i64().unwrap() as i32),
            &records,
        )
        .unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), row["native"]);
    }
}

#[test]
fn concrete_controller_constructor_and_kingdom_goals_match_original() {
    let doc: Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/phase7-sai/native-controller.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(doc["fixtures"].as_array().unwrap().len(), 6);
    for row in doc["fixtures"].as_array().unwrap() {
        let input = &row["input"];
        let kingdoms: Vec<Kingdom> = serde_json::from_value(input["kingdoms"].clone()).unwrap();
        let mut slots: Vec<Option<AiSlot>> = input["slots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                if s.is_null() {
                    return None;
                }
                Some(AiSlot {
                    kingdom: serde_json::from_value(s["kingdom"].clone()).unwrap(),
                    controller: if s["controller"].is_null() {
                        None
                    } else {
                        Some(
                            Controller::new(
                                serde_json::from_value(s["controller"]["object"].clone()).unwrap(),
                                serde_json::from_value(s["controller"]["player"].clone()).unwrap(),
                            )
                            .unwrap(),
                        )
                    },
                })
            })
            .collect();
        let mut outputs = Vec::new();
        for index in input["sequence"].as_array().unwrap() {
            let kingdom = index.as_u64().map(|i| kingdoms[i as usize]);
            for slot in slots.iter_mut().flatten() {
                if let Some(controller) = &mut slot.controller {
                    controller.notify_kingdom(slot.kingdom, kingdom).unwrap();
                }
            }
            outputs.push(
                slots
                    .iter()
                    .flatten()
                    .filter_map(|s| s.controller.as_ref())
                    .cloned()
                    .collect::<Vec<_>>(),
            );
        }
        assert_eq!(serde_json::to_value(outputs).unwrap(), row["native"]);
    }
}

#[test]
fn actor_seen_allocation_and_activity_match_native() {
    let doc: Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/phase7-sai/native-actor-leaves.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(doc["seen_tables"].as_array().unwrap().len(), 16);
    for row in doc["seen_tables"].as_array().unwrap() {
        let mut table: ActorSeenTable = serde_json::from_value(row["input"].clone()).unwrap();
        table
            .create_if_needed(row["major_count"].as_i64().unwrap() as i32)
            .unwrap();
        assert_eq!(serde_json::to_value(table).unwrap(), row["native"]);
    }
    assert_eq!(doc["activity"].as_array().unwrap().len(), 192);
    for row in doc["activity"].as_array().unwrap() {
        let fields: ActorActivity = serde_json::from_value(row["input"].clone()).unwrap();
        assert_eq!(
            actor_active_for_sai(fields),
            row["native"].as_bool().unwrap()
        );
    }
}
