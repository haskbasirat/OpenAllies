//! Replays research/phase8-saved-components/native-hq.json: the original
//! HQ constructor 574490 and saved Load 574040 (Unicorn), against
//! aa_actors::hq_state.
use aa_actors::{
    ActorId,
    hq_state::*,
    organization_state::{Arg, LoadError},
};
use serde_json::{Value, json};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/phase8-saved-components/native-hq.json"
);

struct World<'a> {
    case: &'a Value,
    registered: Option<i32>,
    calls: Vec<Value>,
}
impl HqWorld for World<'_> {
    fn updatable_table_word0(&self) -> Result<i32> {
        Ok(self.case["table_max"].as_i64().unwrap() as i32)
    }
    fn updatable_slot_occupied(&self, id: i32) -> Result<bool> {
        Ok(self.case["occupied"].as_array().unwrap().iter().any(|v| v.as_i64() == Some(i64::from(id))))
    }
    fn register_updatable(&mut self, id: i32) -> Result<()> {
        self.registered = Some(id);
        Ok(())
    }
    fn owner_kingdom(&self) -> Result<Option<u32>> {
        Ok((!self.case["kingdom"].is_null()).then_some(1))
    }
    fn nation_badge_count(&self, kingdom: u32) -> Result<Option<i32>> {
        assert_eq!(kingdom, 1);
        Ok(self.case["kingdom"]["badges"].as_i64().map(|v| v as i32))
    }
    fn nation_name(&self, _: u32) -> Result<Vec<u16>> {
        Ok(self.case["kingdom"]["nation"].as_str().unwrap().encode_utf16().collect())
    }
    fn owner_name(&mut self) -> Result<Vec<u16>> {
        self.calls.push(json!({"seam": "owner_name"}));
        Ok(self.case["owner_name"].as_str().unwrap().encode_utf16().collect())
    }
    fn log(&mut self, log: HqLog) -> Result<()> {
        let args: Vec<Value> = log
            .message
            .args
            .iter()
            .map(|a| match a {
                Arg::Text(t) => json!(t),
                Arg::Int(i) => json!(i),
            })
            .collect();
        self.calls.push(json!({"seam": "404F60", "line": log.line, "function": log.function,
            "format": log.message.format, "args": args}));
        Ok(())
    }
}

fn item(i: &Option<HqItem>) -> Value {
    match i {
        Some(i) => json!([i.kind, i.word_04, i.word_08]),
        None => Value::Null,
    }
}
fn state(s: &HqState) -> Value {
    json!({"index": s.badge_index,
        "slots": s.slots.iter().map(item).collect::<Vec<_>>(),
        "pending": s.pending.iter().map(|i| item(&Some(*i))).collect::<Vec<_>>()})
}
fn native_state(v: &Value) -> Value {
    json!({"index": v["index"], "slots": v["slots"]["items"], "pending": v["pending"]["items"]})
}

#[test]
fn hq_constructor_and_load_match_original() {
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 26);
    for case in cases {
        let input = &case["input"];
        let native = &case["native"];
        let name = input["name"].as_str().unwrap();
        let bytes: Vec<u8> = {
            let h = input["payload"].as_str().unwrap();
            (0..h.len()).step_by(2).map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap()).collect()
        };
        let mut hq = HqState::new(ActorId::from_raw(0x10001), input["slots_530"].as_i64().unwrap() as i32);
        assert_eq!(state(&hq), native_state(&native["after_ctor"]), "{name}: ctor");
        let mut world = World {
            case: input,
            registered: None,
            calls: Vec::new(),
        };
        let result = hq.load(&bytes, &mut world);
        let outcome = &native["outcome"];
        match (&result, outcome["kind"].as_str()) {
            (Ok(n), None) => assert_eq!(*n as u64, native["consumed"].as_u64().unwrap(), "{name}"),
            (Err(LoadError::Exception(m)), Some("ThrowException")) => assert_eq!(m.format, outcome["format"], "{name}"),
            (Err(LoadError::Fatal(m)), Some("Fatal")) => {
                assert_eq!(m.format, outcome["format"], "{name}");
                assert_eq!(
                    json!(m.args.iter().map(|a| match a { Arg::Int(i) => json!(i), Arg::Text(t) => json!(t) }).collect::<Vec<_>>()),
                    outcome["args"],
                    "{name}"
                );
            }
            (Err(LoadError::NullDereference(pc)), Some("NullDereference")) => {
                assert_eq!(format!("{pc:#x}"), outcome["pc"].as_str().unwrap(), "{name}")
            }
            (Err(LoadError::Truncated { at, wanted }), Some("Truncated")) => {
                assert_eq!(*at as u64, outcome["at"].as_u64().unwrap(), "{name}");
                assert_eq!(*wanted as u64, outcome["wanted"].as_u64().unwrap(), "{name}");
            }
            (r, k) => panic!("{name}: rust {r:?} native {k:?}"),
        }
        if !native["state"].is_null() {
            assert_eq!(state(&hq), native_state(&native["state"]), "{name}: state");
        }
        assert_eq!(Value::Array(world.calls), native["calls"], "{name}: calls");
        match native.get("table_slot") {
            Some(v) if v == "hq+8" => assert!(world.registered.is_some(), "{name}"),
            _ => assert!(world.registered.is_none(), "{name}"),
        }
    }
}
