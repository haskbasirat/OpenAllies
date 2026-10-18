//! Replays research/phase8-ai/native-state-thinks.json (original 51E110,
//! 52DF30, 51DE10, 51E260, 523930 with native 529E60/52A000/52A340).
use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId, World,
    cai_states::{StateThinkWorld, chasing, deploy, exhausted, regroup, retreat},
};
use serde_json::{Value, json};

const NAMES: [&str; 5] = ["this", "sleep", "move68", "move7c", "other"];

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-state-thinks.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn id(name: &str) -> StateId {
    StateId(NAMES.iter().position(|n| *n == name).unwrap() as u32 + 1)
}
fn name(id: Option<StateId>) -> Value {
    id.map_or(Value::Null, |s| json!(NAMES[s.0 as usize - 1]))
}

struct Host {
    c: Value,
    events: Vec<Value>,
}
impl Host {
    fn event(&mut self, event: &str, extra: Value, arena: &Arena) {
        let mut e = json!({"event": event});
        for (k, v) in extra.as_object().unwrap() {
            e[k] = v.clone();
        }
        let states: serde_json::Map<String, Value> = NAMES
            .iter()
            .map(|n| (n.to_string(), json!(arena.states[&id(n)].status)))
            .collect();
        e["states"] = json!(states);
        e["stack"] = json!(
            arena.minds[&MindId(1)]
                .stack
                .iter()
                .map(|s| name(*s))
                .collect::<Vec<_>>()
        );
        self.events.push(e);
    }
}
impl World for Host {
    fn can_activate(&mut self, _state: StateId, _arena: &mut Arena) -> Result<bool> {
        unreachable!()
    }
    fn enter(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        self.event("enter", json!({"state": name(Some(state))}), arena);
        Ok(())
    }
    fn leave(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        self.event("leave", json!({"state": name(Some(state))}), arena);
        Ok(())
    }
    fn schedule(&mut self, _mind: MindId, delay: u32, data: u32, arena: &mut Arena) -> Result<()> {
        self.event("schedule", json!({"delay": delay, "data": data}), arena);
        Ok(())
    }
    fn reset_default(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("default", json!({"args": []}), arena);
        Ok(())
    }
    fn state_changed(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("changed", json!({"args": []}), arena);
        Ok(())
    }
}
impl StateThinkWorld for Host {
    fn embedded_think(&mut self, _mind: MindId, offset: u32, arena: &mut Arena) -> Result<()> {
        self.event("embedded_think", json!({"offset": offset}), arena);
        Ok(())
    }
    fn regroup_anchor_flag(&mut self, _mind: MindId, _arena: &Arena) -> Result<Option<bool>> {
        Ok(self.c["anchor"].as_u64().map(|b| b & 1 != 0))
    }
    fn mind_virtual_150(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_150", json!({"args": []}), arena);
        Ok(())
    }
    fn organization_status(&mut self, _mind: MindId, arena: &Arena) -> Result<u32> {
        let result = self.c["status"].as_u64().unwrap() as u32;
        self.event("organization_status", json!({"result": result}), arena);
        Ok(result)
    }
    fn chase_target(&mut self, _state: StateId, _arena: &Arena) -> Result<u32> {
        Ok(self.c["target"].as_u64().unwrap() as u32)
    }
    fn chase_continue(&mut self, _state: StateId, target: u32, arena: &mut Arena) -> Result<bool> {
        let result = self.c["chase"].as_bool().unwrap();
        self.event(
            "chase_continue",
            json!({"target": target, "result": result}),
            arena,
        );
        Ok(result)
    }
    fn mind_virtual_140(&mut self, _mind: MindId, target: u32, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_140", json!({"args": [target]}), arena);
        Ok(())
    }
}

#[test]
fn native_state_think_dispatchers_match_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 67);
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let label = &c["name"];
        let mut arena = Arena::default();
        let mut mind = Mind::base(1);
        mind.stack = c["stack"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().map(id))
            .collect();
        arena.minds.insert(MindId(1), mind);
        for n in NAMES {
            arena.states.insert(
                id(n),
                State {
                    parent: MindId(1),
                    discard_on_suspend: false,
                    status: c["states"][n].as_u64().unwrap() as u32,
                },
            );
        }
        let mut host = Host {
            c: c.clone(),
            events: vec![],
        };
        let this = id("this");
        let result = match c["kind"].as_str().unwrap() {
            "retreat" => retreat(this, &mut arena, &mut host),
            "deploy" => deploy(this, &mut arena, &mut host),
            "regroup" => regroup(this, &mut arena, &mut host),
            "exhausted" => exhausted(this, &mut arena, &mut host),
            "chasing" => chasing(this, &mut arena, &mut host),
            other => panic!("{other}"),
        };
        let native: Vec<Value> = o["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["event"] != "diagnostic_log")
            .cloned()
            .collect();
        let logged = o["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["event"] == "diagnostic_log");
        assert_eq!(host.events, native, "{label}");
        if o["fault"].is_null() {
            assert_eq!(result.unwrap(), logged, "{label}");
            let states: serde_json::Map<String, Value> = NAMES
                .iter()
                .map(|n| (n.to_string(), json!(arena.states[&id(n)].status)))
                .collect();
            assert_eq!(json!(states), o["states"], "{label}");
        } else {
            assert!(matches!(result, Err(Error::MissingActiveState)), "{label}");
        }
    }
}
