//! Replays research/phase8-restoration/native-supply.json: the original
//! Supply constructor 58AA50 followed by saved Load 58ABD0 on one object.
use aa_actors::{
    ActorId,
    supply_state::{Definition, SupplyState, register_source},
};
use aa_mapworld::saved_components::SavedComponent;
use aa_numeric::Precision;
use serde_json::Value;

const FIXTURE: &str = include_str!("../../../research/phase8-restoration/native-supply.json");

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn original_supply_constructor_and_load_replay() {
    let doc: Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 540);
    let me = ActorId::from_raw(0x0001_0042);
    for case in cases {
        let input = &case["input"];
        let native = &case["native"];
        let pc = match input["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let definition = Definition {
            supply_range_bits: input["supply_range"].as_u64().unwrap() as u32,
            is_relay: input["is_relay"].as_bool().unwrap(),
            stockpile_max_bits: input["stockpile_max"].as_u64().unwrap() as u32,
        };
        let mut state = SupplyState::new(me, &definition);
        // Owner Kingdom list before construction (head first), then 4E1F00.
        let mut list: Option<Vec<ActorId>> = input["owner"].as_array().map(|entries| {
            entries
                .iter()
                .map(|e| match e.as_str() {
                    Some("self") => me,
                    _ => ActorId::from_raw(0x7000 + e.as_u64().unwrap() as u32),
                })
                .collect()
        });
        if let Some(list) = &mut list {
            register_source(list, me);
        }
        let ctor = &native["after_ctor"];
        assert_eq!(u64::from(state.supply_range.0), ctor["0x10"].as_u64().unwrap());
        assert_eq!(state.stockpile.0, 0);
        assert_eq!(state.word_1c, 0);
        let payload = hex(input["payload"].as_str().unwrap());
        let (SavedComponent::Supply(saved), consumed) =
            SavedComponent::decode_prefix(11, &payload).unwrap()
        else {
            panic!()
        };
        assert_eq!(consumed, native["consumed"].as_u64().unwrap() as usize);
        state
            .load(
                &definition,
                saved.updatable.is_some(),
                saved.amount_multiplier_bits.zip(saved.word_1c),
                pc,
            )
            .unwrap();
        let out = &native["state"];
        assert_eq!(u64::from(state.supply_range.0), out["0x10"].as_u64().unwrap(), "{case}");
        assert_eq!(u64::from(state.byte_14), out["0x14"].as_u64().unwrap());
        assert_eq!(u64::from(state.stockpile.0), out["0x18"].as_u64().unwrap(), "{case}");
        assert_eq!(u64::from(state.word_1c), out["0x1c"].as_u64().unwrap(), "{case}");
        match (&list, native["kingdom_list"].as_object()) {
            (None, None) => {}
            (Some(list), Some(expected)) => {
                let names: Vec<Value> = list
                    .iter()
                    .map(|id| {
                        if *id == me {
                            Value::from("self")
                        } else {
                            Value::from(id.raw() - 0x7000)
                        }
                    })
                    .collect();
                assert_eq!(Value::Array(names.clone()), expected["head_first"], "{case}");
                assert_eq!(names.last().unwrap(), &expected["tail"]);
            }
            _ => panic!("owner presence mismatch {case}"),
        }
    }
}
