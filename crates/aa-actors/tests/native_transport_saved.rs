//! Replays research/phase8-saved-components/native-transport.json: the
//! original Transport constructor 58B9A0 and saved Load 58C810 under Unicorn.
use aa_actors::{
    ActorId,
    organization_state::{Arg, Definitions, LayoutEntry, LayoutInfo, LoadError, Message, Result},
    transport_state::TransportState,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/phase8-saved-components/native-transport.json"
);

struct Defs {
    actors: Vec<String>,
    layouts: BTreeMap<String, usize>,
    calls: Vec<Value>,
}
impl Definitions<String> for Defs {
    fn layout_data(&mut self, ids: &[u16]) -> Result<Option<String>> {
        let name = String::from_utf16(ids).unwrap();
        let found = self.layouts.contains_key(&name);
        self.calls.push(json!({"seam": "463FA0", "ids": name, "found": found}));
        Ok(found.then_some(name))
    }
    fn actor_data(&mut self, ids: &[u16]) -> Result<Option<String>> {
        let name = String::from_utf16(ids).unwrap();
        let found = self.actors.contains(&name);
        self.calls.push(json!({"seam": "4BDBA0", "ids": name, "found": found}));
        Ok(found.then_some(name))
    }
    fn layout_info(&self, layout: &String) -> Result<LayoutInfo> {
        let count = self.layouts[layout];
        Ok(LayoutInfo {
            name: layout.clone(),
            primary_slot: -1,
            entries: (0..count)
                .map(|_| LayoutEntry {
                    property: 0,
                    property_name: String::new(),
                    required: false,
                    locked: false,
                    morale_line: false,
                })
                .collect(),
            slot_groups: Vec::new(),
            formations: Vec::new(),
            formation_44: 0,
            formation_48: 0,
            property_50: 0,
        })
    }
    fn name(&self, definition: &String) -> Result<String> {
        Ok(definition.clone())
    }
}

fn message(outcome: &Value) -> Message {
    let format: &'static str = Box::leak(outcome["format"].as_str().unwrap().to_owned().into_boxed_str());
    let args = outcome["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| match a {
            Value::String(s) => Arg::Text(s.clone()),
            v => Arg::Int(v.as_i64().unwrap() as i32),
        })
        .collect();
    Message { format, args }
}
pub fn expected_error(outcome: &Value) -> LoadError {
    match outcome["kind"].as_str().unwrap() {
        "Truncated" => LoadError::Truncated {
            at: outcome["at"].as_u64().unwrap() as usize,
            wanted: outcome["wanted"].as_u64().unwrap() as usize,
        },
        "ThrowException" => LoadError::Exception(message(outcome)),
        "Fatal" => LoadError::Fatal(message(outcome)),
        "Warning" => LoadError::Warning(message(outcome)),
        "NullDereference" => LoadError::NullDereference(
            u32::from_str_radix(outcome["pc"].as_str().unwrap().trim_start_matches("0x"), 16).unwrap(),
        ),
        k => panic!("unexpected outcome {k}"),
    }
}
fn label(prefix: &str, value: &Option<String>) -> Value {
    value.as_ref().map_or(Value::Null, |v| json!(format!("{prefix}:{v}")))
}
fn vector(native: &Value, values: &[aa_body::F32]) {
    let count = native["count"].as_i64().unwrap();
    assert_eq!(count, values.len() as i64);
    for (n, r) in native["values"].as_array().unwrap().iter().zip(values) {
        let n = n.as_u64().unwrap() as u32;
        // 0xA5A5A5A5: never-written heap (allocator fill), no defined value.
        if n != 0xa5a5_a5a5 {
            assert_eq!(n, r.0);
        }
    }
}
fn compare(native: &Value, state: &TransportState<String>) {
    vector(&native["cost"], &state.cost.values);
    vector(&native["upkeep"], &state.upkeep.values);
    assert_eq!(native["cargo_definition"], label("actor", &state.cargo_definition));
    match &state.cargo_layout {
        None => assert!(native["cargo_layout"].is_null()),
        Some(l) => {
            let n = &native["cargo_layout"];
            assert_eq!(n["definition"], json!(format!("layout:{}", l.definition)));
            let params: Vec<Value> = l.parameters.iter().map(|p| label("actor", p)).collect();
            assert_eq!(n["parameters"], json!(params));
        }
    }
    let w = |k: &str| native[k].as_u64().unwrap() as u32;
    assert_eq!(native["word_30"], json!(state.word_30));
    assert_eq!(w("byte_38"), u32::from(state.byte_38));
    assert_eq!(w("byte_39"), u32::from(state.byte_39));
    assert_eq!(w("word_3c"), state.word_3c);
    assert_eq!(w("word_40"), state.word_40);
    assert_eq!(w("word_44"), state.word_44);
    assert_eq!(w("word_48"), state.word_48);
    assert_eq!(w("byte_4c"), u32::from(state.byte_4c));
    assert_eq!(native["words_50"], json!(state.words_50));
    assert_eq!(native["text_58"].as_str().unwrap(), String::from_utf16(&state.text_58).unwrap());
    assert_eq!(w("word_5c"), state.word_5c);
}

#[test]
fn transport_constructor_and_load_match_original() {
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    assert_eq!(doc["passed"], true);
    let defs = &doc["definitions"];
    let actors: Vec<String> = serde_json::from_value(defs["actor_data"].clone()).unwrap();
    let layouts: BTreeMap<String, usize> = serde_json::from_value(defs["layout_data_parameter_counts"].clone()).unwrap();
    let (mut loaded, mut refused, mut errors) = (0, 0, 0);
    for (i, case) in doc["cases"].as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let native = &case["native"];
        let payload = hex(input["payload"].as_str().unwrap());
        let count = input["resource_count"].as_u64().unwrap() as usize;
        let mut state = TransportState::<String>::new(ActorId::from_raw(0x10001), count);
        compare(&native["after_ctor"], &state);
        let ctor = &native["after_ctor"];
        assert_eq!(ctor["cost"]["capacity"].as_u64().unwrap() as usize, state.cost.initial_capacity);
        assert_eq!(ctor["upkeep"]["capacity"].as_u64().unwrap() as usize, state.upkeep.initial_capacity);
        let mut world = Defs { actors: actors.clone(), layouts: layouts.clone(), calls: Vec::new() };
        let result = state.load(&payload, &mut world);
        let negative = native["state"].is_object()
            && (native["state"]["cost"]["count"].as_i64().unwrap() < 0
                || native["state"]["upkeep"]["count"].as_i64().unwrap() < 0);
        if negative {
            // Native stores a negative ResourceVector size and continues.
            assert!(matches!(result, Err(LoadError::Unsupported(_))), "case {i}: {result:?}");
            refused += 1;
            continue;
        }
        match (&native["outcome"], result) {
            (Value::Null, Ok(consumed)) => {
                assert_eq!(consumed as u64, native["consumed"].as_u64().unwrap(), "case {i}");
                loaded += 1;
            }
            (outcome, Err(e)) if !outcome.is_null() => {
                assert_eq!(e, expected_error(outcome), "case {i}");
                errors += 1;
            }
            (outcome, r) => panic!("case {i}: native {outcome} rust {r:?}"),
        }
        assert_eq!(world.calls, native["calls"].as_array().unwrap().clone(), "case {i} seams");
        if native["state"].is_object() {
            compare(&native["state"], &state);
        }
    }
    assert_eq!(loaded + refused + errors, doc["cases"].as_array().unwrap().len());
    assert!(loaded > 100 && refused == 1 && errors > 100, "{loaded} {refused} {errors}");
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

/// research/phase8-saved-components/native-transport-requirement.json: the
/// original 4CFB00 ValidateComponents with the real Transport vtable A40100
/// (+20 = 7C7840 returns 3) fails exactly when the Mover slot is empty.
#[test]
fn transport_requires_the_mover_component() {
    use aa_actors::{components::ComponentSlot, transport_state::REQUIRED_COMPONENT};
    let path = FIXTURE.replace("native-transport.json", "native-transport-requirement.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(doc["transport_vtable_20"], "0x7c7840");
    assert_eq!(REQUIRED_COMPONENT, ComponentSlot::Mover);
    let mover = REQUIRED_COMPONENT as u8 as u64;
    let transport = ComponentSlot::Transport as u8 as u64;
    for case in doc["cases"].as_array().unwrap() {
        let slots: Vec<u64> = case["slots"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
        let calls = case["calls"].as_array().unwrap();
        // 4CFB00 stops at the first failing slot (ascending order).
        let transport_failure = calls.iter().any(|c| c["args"][1] == transport);
        if transport_failure {
            assert_eq!(calls[0]["args"][2], mover);
            assert!(!slots.contains(&mover));
            assert_eq!(case["result"], 0);
        }
        if slots.contains(&transport) && slots.contains(&mover) {
            assert!(!transport_failure, "{slots:?}");
        }
        if slots.contains(&transport) && !slots.contains(&mover) {
            let first = calls[0]["args"][1].as_u64().unwrap();
            assert!(first <= transport, "{slots:?}");
        }
    }
}

/// research/phase8-saved-components/native-transport-payload.json: the
/// original 58BA90 (58B2A0) payload load on a constructed Transport.
#[test]
fn transport_payload_load_matches_original() {
    use aa_actors::{ground_states::LayoutDefinition, transport_state::PayloadWorld};
    struct World<'a> {
        c: &'a Value,
        calls: Vec<Value>,
    }
    impl PayloadWorld<String> for World<'_> {
        fn cargo_upkeep(&mut self, cargo: &String, layout: Option<&LayoutDefinition<String>>) -> Result<Vec<aa_body::F32>> {
            self.calls.push(json!({"seam": "56F580", "cargo": cargo, "layout": layout.map(|_| "layout"), "context": 0}));
            Ok(self.c["upkeep"].as_array().unwrap().iter().map(|v| aa_body::F32(v.as_u64().unwrap() as u32)).collect())
        }
        fn slot_groups(&self, _layout: &String) -> Result<Vec<u32>> {
            Ok(self.c["layout"][1].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect())
        }
        fn refresh_owner(&mut self) -> Result<()> {
            self.calls.push(json!({"seam": "4CF0F0", "this": "owner"}));
            Ok(())
        }
    }
    let path = FIXTURE.replace("native-transport.json", "native-transport-payload.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    for (i, case) in doc["cases"].as_array().unwrap().iter().enumerate() {
        let c = &case["input"];
        let native = &case["native"];
        let mut state = TransportState::<String>::new(ActorId::from_raw(0x10001), c["resource_count"].as_u64().unwrap() as usize);
        if !c["old_layout"].is_null() {
            state.cargo_layout = Some(LayoutDefinition {
                definition: "old".into(),
                parameters: vec![Some("inf".into())],
                slots: 1,
            });
        }
        let layout = (!c["layout"].is_null()).then(|| LayoutDefinition {
            definition: c["layout"][0].as_str().unwrap().to_owned(),
            parameters: c["layout"][2].as_array().unwrap().iter().map(|p| p.as_str().map(str::to_owned)).collect(),
            slots: c["layout"][1].as_array().unwrap().len(),
        });
        let pair = |v: &Value| [v[0].as_u64().unwrap() as u32, v[1].as_u64().unwrap() as u32];
        let name: Vec<u16> = c["name"].as_str().unwrap().encode_utf16().collect();
        let mut w = World { c, calls: Vec::new() };
        let r = state
            .load_payload("cargo_regiment".into(), &name, layout, pair(&c["start"]), pair(&c["drop"]), c["resource_count"].as_u64().unwrap() as usize, &mut w)
            .unwrap();
        assert_eq!(u64::from(r), native["result"].as_u64().unwrap(), "case {i}");
        assert_eq!(w.calls, native["calls"].as_array().unwrap().clone(), "case {i}");
        let s = &native["state"];
        let bits = |v: &[aa_body::F32]| json!(v.iter().map(|x| x.0).collect::<Vec<_>>());
        assert_eq!(bits(&state.cost.values), s["cost"], "case {i}");
        assert_eq!(bits(&state.upkeep.values), s["upkeep"], "case {i}");
        assert_eq!(json!(state.cargo_definition), s["cargo_definition"], "case {i}");
        match (&state.cargo_layout, &s["cargo_layout"]) {
            (None, Value::Null) => {}
            (Some(l), n) => {
                assert_eq!(n["definition"], json!(format!("layoutdata:{}", l.definition)));
                assert_eq!(n["parameters"], json!(l.parameters));
            }
            (l, n) => panic!("case {i}: {l:?} {n}"),
        }
        assert_eq!(json!(state.word_30), s["word_30"]);
        assert_eq!(json!(state.byte_38), s["byte_38"]);
        assert_eq!(json!(state.byte_39), s["byte_39"]);
        assert_eq!(json!(state.word_3c), s["word_3c"], "case {i}");
        assert_eq!(json!([state.word_40, state.word_44, state.word_48]), json!([s["word_40"], s["word_44"], s["word_48"]]));
        assert_eq!(json!(state.byte_4c), s["byte_4c"]);
        assert_eq!(json!(state.words_50), s["words_50"]);
        assert_eq!(String::from_utf16(&state.text_58).unwrap(), s["text_58"].as_str().unwrap());
        assert_eq!(json!(state.word_5c), s["word_5c"]);
    }
}
