use aa_actors::{creation::*, registry::*};
use aa_numeric::Precision;
use aa_simulation::NativeRng;
use serde_json::{Value, json};

fn proof() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase5-actors/creation-native-oracle.json"
    ))
    .unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn snapshot<T>(r: &IdentityRegistry<T>) -> Value {
    json!({"active":r.len(),"next_slot":r.next_slot(),"highest_slot":r.highest_slot(),
    "generations":(0..SLOT_COUNT).filter_map(|i|{let g=r.generation_at(i as u16);(g!=0).then_some([i as u32,u32::from(g)])}).collect::<Vec<_>>(),
    "entries":r.iter().map(|e|json!({"slot":e.id.slot(),"id":e.id.raw(),"key":1,"is_actor":e.is_actor})).collect::<Vec<_>>(),
    "occupied":r.occupied_words().iter().enumerate().filter_map(|(i,&v)|(v!=0).then_some([i as u32,v])).collect::<Vec<_>>()})
}

#[test]
fn original_whole_constructor_and_identity_cases_match() {
    let p = proof();
    assert_eq!(p["cases"].as_array().unwrap().len(), 282);
    for row in p["cases"].as_array().unwrap() {
        let c = &row["input"];
        let mut record = CreationRecord {
            definition: ThingDefinition {
                key: 7u32,
                flags: u(&c["definition_flags"]),
            },
            position_bits: serde_json::from_value(c["position_bits"].clone()).unwrap(),
            direction_bits: serde_json::from_value(c["direction_bits"].clone()).unwrap(),
            name_utf16: serde_json::from_value(c["name_utf16"].clone()).unwrap(),
            saved_id: ActorId::from_raw(u(&c["saved_id"])),
        };
        let pc: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let table = DirectionTable::original(
            serde_json::from_value(c["startup_precision"].clone()).unwrap(),
        );
        let mut rng = NativeRng::new(u(&c["rng_state"]));
        let thing = construct(&mut record, u(&c["world_begin_bits"]), &mut rng, table, pc).unwrap();
        let mut actual = serde_json::to_value(&thing).unwrap();
        actual.as_object_mut().unwrap().remove("definition");
        actual["id"] = json!(0);
        assert_eq!(actual, row["constructor"], "{c}");
        assert_eq!(
            json!(record.direction_bits),
            row["record_direction_after"],
            "{c}"
        );
        assert_eq!(rng.state, u(&row["rng_after"]), "{c}");
        assert_eq!(
            row["events"],
            json!([{"kind":"facing_notification","position_bits":thing.position_bits,"facing_bits":thing.facing_bits}]),
            "{c}"
        );
        if c["full_registry"] == true {
            // The native fixture isolates count32768 without creating32768
            // native objects. Rust uses a valid populated registry here.
            let mut r = IdentityRegistry::new();
            for _ in 0..NATIVE_ADD_LIMIT {
                r.allocate(false, ()).unwrap();
            }
            assert_eq!(
                initialize_identity(&mut r, ActorId::NULL, false, ()),
                Err(RegistryError::Exhausted)
            );
            assert_eq!(row["initialized"], false);
            assert_eq!(row["registered_id"], 0);
            continue;
        }
        let mut r = IdentityRegistry::new();
        let id = initialize_identity(&mut r, record.saved_id, false, thing).unwrap();
        assert_eq!(row["initialized"], true);
        assert_eq!(id.raw(), u(&row["registered_id"]));
        assert_eq!(snapshot(&r), row["registry"], "{c}");
    }
}

#[test]
fn original_startup_tables_preserve_every_vector_and_precision_identity() {
    let p = proof();
    for table in p["tables"].as_array().unwrap() {
        let pc = serde_json::from_value(table["precision"].clone()).unwrap();
        let t = DirectionTable::original(pc);
        for (i, row) in table["values"].as_array().unwrap().iter().enumerate() {
            assert_eq!(json!(t.at(i).unwrap()), *row);
        }
        assert!(t.at(2048).is_none());
    }
}

#[test]
fn unsupported_direction_does_not_guess_and_axis_precedence_avoids_rng_draw() {
    let mut record = CreationRecord {
        definition: ThingDefinition { key: (), flags: 0 },
        position_bits: [0x7fc0_1234, 0, 0x8000_0000],
        direction_bits: [0x7fc0_1234, 0],
        name_utf16: vec![],
        saved_id: ActorId::NULL,
    };
    let mut rng = NativeRng::new(17);
    let t = DirectionTable::original(Precision::Double53);
    assert!(construct(&mut record, 0, &mut rng, t, Precision::Double53).is_err());
    assert_eq!(rng.state, 17);
    record.definition.flags = (1 << 17) | (1 << 15);
    let s = construct(&mut record, 0, &mut rng, t, Precision::Double53).unwrap();
    assert_eq!(s.facing_bits, [0, 0xbf80_0000, 0]);
    assert_eq!(rng.state, 17);
    assert_eq!(s.position_bits, record.position_bits);
}
