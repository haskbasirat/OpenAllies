//! Replays research/phase8-saved-components/native-denizen-inventory.json:
//! the original Denizen constructor 56B720 with nonempty Inventories and the
//! saved Load 56C320 under Unicorn, including the 168 shipped carrier and
//! airfield payloads. The consumer takes the producer slot list (here: the
//! native constructor's own list; for real cases also checked against the
//! aa-rules producer output stored with the fixture).
use aa_actors::{
    ActorId,
    denizen_state::{DenizenState, InventorySlot, InventorySlotDefinition},
    organization_state::LoadError,
};
use aa_body::F32;
use aa_numeric::Precision;
use serde_json::{Value, json};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/phase8-saved-components/native-denizen-inventory.json"
);

fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        _ => panic!(),
    }
}
fn inventory_json(slots: &[InventorySlot]) -> Value {
    json!(slots
        .iter()
        .map(|s| json!({"word_00": s.word_00, "target": s.target, "value_08": s.value_08.0,
            "worker": u8::from(s.worker), "militia_or_air_defensive": u8::from(s.militia_or_air_defensive)}))
        .collect::<Vec<_>>())
}
fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

#[test]
fn denizen_inventory_constructor_and_load_match_original() {
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    assert_eq!(doc["passed"], true);
    let (mut real, mut loaded, mut truncated) = (0, 0, 0);
    for case in doc["cases"].as_array().unwrap() {
        let input = &case["input"];
        let native = &case["native"];
        let name = input["name"].as_str().unwrap();
        let ctor = &native["after_ctor"];
        // Producer slot list: the native constructor's (head first).
        let slots: Vec<InventorySlotDefinition> = ctor["inventory"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| InventorySlotDefinition {
                target: s["target"].as_str().unwrap().to_owned(),
                worker: s["worker"] != 0,
                militia_or_air_defensive: s["militia_or_air_defensive"] != 0,
            })
            .collect();
        if let Some(producer) = input.get("producer_slots") {
            // aa-rules native_denizen_inventory == original 56B720 output.
            let producer: Vec<InventorySlotDefinition> = producer
                .as_array()
                .unwrap()
                .iter()
                .map(|s| InventorySlotDefinition {
                    target: s["target"].as_str().unwrap().to_owned(),
                    worker: s["worker"].as_bool().unwrap(),
                    militia_or_air_defensive: s["militia_or_air_defensive"].as_bool().unwrap(),
                })
                .collect();
            assert_eq!(producer, slots, "{name}");
            real += 1;
        }
        let mut state = DenizenState::with_inventory(ActorId::from_raw(0x10001), &slots);
        assert_eq!(inventory_json(&state.inventory), ctor["inventory"], "{name}");
        for (key, value) in ctor["words"].as_object().unwrap() {
            let offset = u32::from_str_radix(key.trim_start_matches("0x"), 16).unwrap();
            assert_eq!(json!(state.words[&offset]), *value, "{name} {key}");
        }
        assert_eq!(json!([state.byte_20, state.byte_21]), ctor["bytes"]);
        assert!(state.handles_2c.is_empty() && ctor["handles"].as_array().unwrap().is_empty());

        // The Inventory section of 56C320 (version >= 3).
        let payload = hex(input["payload"].as_str().unwrap());
        let targets = input["targets"].as_object().unwrap();
        let health = |t: &str| -> Result<u32, LoadError> { Ok(targets[t][1].as_u64().unwrap() as u32) };
        let pc = precision(input["precision"].as_str().unwrap());
        let uv = payload.get(..4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        let start = 4 + if uv >= Some(1) { 4 } else { 0 };
        let version = payload.get(start..start + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        let at = start + 4 + usize::from(version >= Some(1)) + usize::from(version >= Some(2));
        let outcome = &native["outcome"];
        let end = native["occupant_end"].as_u64().map(|v| v as usize);
        let expected = &native["state"]["inventory"];
        if version >= Some(3) && payload.len() >= at {
            let result = state.load_occupants(&payload[at..], health, pc);
            match (end, result) {
                (Some(end), Ok(n)) => {
                    assert_eq!(at + n, end, "{name}");
                    loaded += 1;
                }
                (None, Err(LoadError::Truncated { at: t, wanted })) => {
                    assert_eq!(outcome["kind"], "Truncated");
                    assert_eq!(at + t, outcome["at"].as_u64().unwrap() as usize, "{name}");
                    assert_eq!(wanted, outcome["wanted"].as_u64().unwrap() as usize, "{name}");
                    truncated += 1;
                }
                (e, r) => panic!("{name}: native end {e:?} rust {r:?}"),
            }
            if end.is_some() {
                // Decoded-list path: same slots from the complete pair list.
                let count = i32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
                let pairs: Vec<[u32; 2]> = (0..count.max(0) as usize)
                    .map(|i| {
                        let w = |k: usize| u32::from_le_bytes(payload[at + 4 + 8 * i + k..at + 8 + 8 * i + k].try_into().unwrap());
                        [w(0), w(4)]
                    })
                    .collect();
                let mut other = DenizenState::with_inventory(ActorId::from_raw(0x10001), &slots);
                other.apply_occupants(&pairs, health, pc).unwrap();
                assert_eq!(other.inventory, state.inventory, "{name}");
            }
        }
        // Whole-Load outcome: only truncation inside or before the section.
        if outcome.is_null() || outcome["kind"] == "Truncated" {
            assert_eq!(inventory_json(&state.inventory), *expected, "{name}");
        } else {
            panic!("{name}: unexpected native outcome {outcome}");
        }
    }
    assert_eq!(real, 168);
    assert!(loaded > 200 && truncated > 5, "{loaded} {truncated}");
}

#[test]
fn empty_inventory_builds_no_slot() {
    // The legacy `Definition { inventory_is_empty }` input was removed after
    // the restoration catalog switched to the 56B720 slot list.
    let s = DenizenState::with_inventory(ActorId::from_raw(1), &[]);
    assert!(s.inventory.is_empty());
    let _ = F32::ZERO;
}
