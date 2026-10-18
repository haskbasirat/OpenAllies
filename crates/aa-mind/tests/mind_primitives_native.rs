//! Replays research/phase8-ai/native-mind-primitives.json (original 52A340,
//! 529E60, 52A000 and 52A1F0 under Unicorn).
use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId, World, is_current, pop, think_base,
    within_arrival,
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::VecDeque;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-mind-primitives.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn key(id: Option<StateId>) -> Value {
    id.map_or(Value::Null, |s| json!(s.0.to_string()))
}
fn state_id(v: &Value) -> Option<StateId> {
    v.as_str().map(|s| StateId(s.parse().unwrap()))
}
fn snapshot(arena: &Arena) -> Value {
    let states: serde_json::Map<String, Value> = arena
        .states
        .iter()
        .map(|(id, s)| (id.0.to_string(), json!({"status": s.status})))
        .collect();
    let stack: Vec<Value> = arena.minds[&MindId(1)]
        .stack
        .iter()
        .map(|s| key(*s))
        .collect();
    json!({"states": states, "stack": stack})
}
fn load(arena: &mut Arena, states: &Value, stack: &Value) {
    for (k, v) in states.as_object().unwrap() {
        arena
            .states
            .get_mut(&StateId(k.parse().unwrap()))
            .unwrap()
            .status = v["status"].as_u64().unwrap() as u32;
    }
    arena.minds.get_mut(&MindId(1)).unwrap().stack =
        stack.as_array().unwrap().iter().map(state_id).collect();
}
fn arena(c: &Value) -> Arena {
    let mut arena = Arena::default();
    let mut mind = Mind::base(1);
    mind.stack = VecDeque::new();
    arena.minds.insert(MindId(1), mind);
    for k in c["states"].as_object().unwrap().keys() {
        arena.states.insert(
            StateId(k.parse().unwrap()),
            State {
                parent: MindId(1),
                discard_on_suspend: false,
                status: 0,
            },
        );
    }
    load(&mut arena, &c["states"], &c["stack"]);
    arena
}

struct Host {
    events: Vec<Value>,
    mutations: Vec<Value>,
}
impl Host {
    fn event(&mut self, mut e: Value, arena: &mut Arena) {
        e["arena"] = snapshot(arena);
        let name = e["event"].clone();
        self.events.push(e);
        if self.mutations.first().is_some_and(|m| m["event"] == name) {
            let m = self.mutations.remove(0);
            let mut states = snapshot(arena)["states"].clone();
            if let Some(changes) = m["states"].as_object() {
                for (k, v) in changes {
                    states[k] = v.clone();
                }
            }
            let stack = if m["stack"].is_null() {
                snapshot(arena)["stack"].clone()
            } else {
                m["stack"].clone()
            };
            load(arena, &states, &stack);
        }
    }
}
impl World for Host {
    fn can_activate(&mut self, _state: StateId, _arena: &mut Arena) -> Result<bool> {
        unreachable!("52A340 never queries activation")
    }
    fn enter(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        self.event(json!({"event": "enter", "state": key(Some(state))}), arena);
        Ok(())
    }
    fn leave(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        self.event(json!({"event": "leave", "state": key(Some(state))}), arena);
        Ok(())
    }
    fn schedule(&mut self, _mind: MindId, delay: u32, data: u32, arena: &mut Arena) -> Result<()> {
        self.event(
            json!({"event": "schedule", "delay": delay, "data": data}),
            arena,
        );
        Ok(())
    }
    fn reset_default(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event(json!({"event": "default"}), arena);
        Ok(())
    }
    fn state_changed(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event(json!({"event": "changed"}), arena);
        Ok(())
    }
}

#[test]
fn native_mind_primitives_match_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 108);
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let mut arena = arena(c);
        match c["entry"].as_str().unwrap() {
            "pop" => {
                let mut host = Host {
                    events: vec![],
                    mutations: c
                        .get("mutations")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default(),
                };
                let result = pop(MindId(1), &mut arena, &mut host);
                let mut native: Vec<Value> = o["events"].as_array().unwrap().clone();
                for e in &mut native {
                    e["arena"].as_object_mut().unwrap().remove("tail_state");
                }
                assert_eq!(host.events, native, "{name}");
                if o["fault"].is_null() {
                    result.unwrap();
                    let mut expected = o["arena"].clone();
                    expected.as_object_mut().unwrap().remove("tail_state");
                    assert_eq!(snapshot(&arena), expected, "{name}");
                } else {
                    assert!(matches!(result, Err(Error::MissingActiveState)), "{name}");
                }
            }
            "think_base" => {
                let state = state_id(&c["state"]).unwrap();
                let was_inactive = think_base(state, &mut arena).unwrap();
                let logged = o["events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|e| e["event"] == "diagnostic_log");
                assert_eq!(logged, was_inactive && c["latch"] == 0, "{name}");
                assert_eq!(o["result"] == 1, c["latch"] == 1 || was_inactive, "{name}");
                assert_eq!(snapshot(&arena)["states"], o["arena"]["states"], "{name}");
            }
            "is_current" => {
                let state = state_id(&c["state"]).unwrap();
                assert_eq!(
                    json!(u8::from(is_current(state, &arena).unwrap())),
                    o["result"],
                    "{name}"
                );
            }
            "within" => {
                let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
                let pos = [
                    c["actor_position"][0].as_u64().unwrap() as u32,
                    c["actor_position"][1].as_u64().unwrap() as u32,
                ];
                let point = [
                    c["point"][0].as_u64().unwrap() as u32,
                    c["point"][1].as_u64().unwrap() as u32,
                ];
                assert_eq!(
                    json!(u8::from(within_arrival(pos, point, p).unwrap())),
                    o["result"],
                    "{name}"
                );
            }
            other => panic!("{other}"),
        }
    }
}
