//! Replays research/phase8-ai/native-uai-reattach-event.json (original
//! 52F160 KKC_UAI::EventReattach with 52A000, 52D6D0 and 52A810 under
//! Unicorn) against aa_mind::uai_reattach_event.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    state_helpers::MemberActivateWorld,
    uai_reattach_event::{ReattachEventWorld, event_reattach},
};
use serde_json::{Value, json};

const STATES: [(&str, u32); 11] = [
    ("unload", 0x34),
    ("deploy", 0x48),
    ("sleep", 0x68),
    ("move", 0x7c),
    ("kill", 0xf8),
    ("bombard", 0x10c),
    ("engage", 0x12c),
    ("repair", 0x148),
    ("join", 0x164),
    ("reattach", 0x178),
    ("unpack", 0x188),
];

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-uai-reattach-event.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn state_name(s: StateId) -> &'static str {
    STATES[s.0 as usize - 1].0
}
fn state_id(name: &str) -> StateId {
    StateId(STATES.iter().position(|(n, _)| *n == name).unwrap() as u32 + 1)
}

struct Host {
    c: Value,
    events: Vec<Value>,
}
impl World for Host {
    fn can_activate(&mut self, state: StateId, _arena: &mut Arena) -> Result<bool> {
        let r = self.c["can_activate"].as_bool().unwrap();
        self.events
            .push(json!({"event": "can_activate", "state": state_name(state), "result": r}));
        Ok(r)
    }
    fn enter(&mut self, state: StateId, _arena: &mut Arena) -> Result<()> {
        self.events
            .push(json!({"event": "enter", "state": state_name(state)}));
        Ok(())
    }
    fn leave(&mut self, state: StateId, _arena: &mut Arena) -> Result<()> {
        self.events
            .push(json!({"event": "leave", "state": state_name(state)}));
        Ok(())
    }
    fn schedule(&mut self, _m: MindId, delay_bits: u32, data: u32, _a: &mut Arena) -> Result<()> {
        self.events
            .push(json!({"event": "schedule", "delay": delay_bits, "data": data}));
        Ok(())
    }
    fn reset_default(&mut self, _m: MindId, _a: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "reset_default"}));
        Ok(())
    }
    fn state_changed(&mut self, _m: MindId, _a: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "state_changed"}));
        Ok(())
    }
}
impl MemberActivateWorld for Host {
    fn has_organization(&mut self, _state: StateId) -> Result<bool> {
        Ok(self.c["membership"].as_bool().unwrap_or(false))
    }
}
impl ReattachEventWorld for Host {
    fn reattach_state(&mut self, _mind: MindId) -> Result<StateId> {
        Ok(state_id("reattach"))
    }
    fn actor_flags8(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["flags"]))
    }
    fn mind_word_c(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["mind_word_c"]))
    }
}

#[test]
fn native_uai_reattach_event_matches_original() {
    let f = fixture();
    assert_eq!(f["coverage"]["52F160"], json!([49, 49]));
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 300);
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let mut arena = Arena::default();
        let mut mind = Mind::base(1);
        for key in c["stack"].as_array().unwrap() {
            mind.stack.push_back(key.as_str().map(state_id));
        }
        arena.minds.insert(MindId(1), mind);
        for (i, (n, _)) in STATES.iter().enumerate() {
            arena.states.insert(
                StateId(i as u32 + 1),
                State {
                    parent: MindId(1),
                    discard_on_suspend: matches!(*n, "deploy" | "sleep"),
                    status: u(&c["status"][*n]),
                },
            );
        }
        let mut host = Host {
            c: c.clone(),
            events: vec![],
        };
        let r = event_reattach(MindId(1), &mut arena, &mut host).unwrap();
        assert_eq!(json!(host.events), o["events"], "{name}");
        assert_eq!(json!(u8::from(r)), o["result"], "{name}");
        let status: serde_json::Map<String, Value> = STATES
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (n.to_string(), json!(arena.states[&StateId(i as u32 + 1)].status)))
            .collect();
        assert_eq!(json!(status), o["status"], "{name}");
        let stack: Vec<Value> = arena.minds[&MindId(1)]
            .stack
            .iter()
            .map(|s| s.map_or(json!(0), |s| json!(state_name(s))))
            .collect();
        assert_eq!(json!(stack), o["stack"], "{name}");
    }
}
