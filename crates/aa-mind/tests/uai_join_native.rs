//! Replays research/phase8-ai/native-uai-join.json (original 531AD0
//! KKC_UAIStateJoin::Think under Unicorn) against aa_mind::uai_thinks::uai_join.
use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId, World,
    uai_thinks::{
        ThinkWorld,
        uai_join::{JoinActivateWorld, JoinThinkWorld, think},
    },
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
const PERFORMER: u32 = 900;
const LOCOMOTION: u32 = 901;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-uai-join.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn state_name(s: StateId) -> &'static str {
    STATES[s.0 as usize - 1].0
}
fn state_id(name: &str) -> StateId {
    StateId(STATES.iter().position(|(n, _)| *n == name).unwrap() as u32 + 1)
}

/// Objects are handles index+1 in key order.
struct Host {
    c: Value,
    keys: Vec<String>,
    events: Vec<Value>,
    target: u32,
}
impl Host {
    fn key(&self, h: u32) -> &str {
        &self.keys[h as usize - 1]
    }
    fn pointer(&self, field: &str) -> Option<u32> {
        match self.c[field].as_str() {
            None => None,
            Some("performer") => Some(PERFORMER),
            Some(_) => Some(LOCOMOTION),
        }
    }
}
impl World for Host {
    fn can_activate(&mut self, state: StateId, _arena: &mut Arena) -> Result<bool> {
        let r = self.c["can_activate"].as_bool().unwrap_or(true);
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
impl ThinkWorld for Host {
    fn embedded_state(&mut self, _mind: MindId, offset: u32) -> Result<StateId> {
        Ok(StateId(
            STATES.iter().position(|(_, o)| *o == offset).unwrap() as u32 + 1,
        ))
    }
    fn embedded_think(&mut self, _mind: MindId, offset: u32, _arena: &mut Arena) -> Result<()> {
        let name = STATES.iter().find(|(_, o)| *o == offset).unwrap().0;
        self.events
            .push(json!({"event": "embedded_think", "state": name}));
        if name == "move"
            && let Some(t) = self.c["retarget"].as_u64()
        {
            self.target = t as u32;
        }
        Ok(())
    }
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        Ok(self
            .keys
            .iter()
            .position(|k| {
                let o = &self.c["objects"][k];
                u(&o["gtid"]) == gtid && o["registered"].as_bool().unwrap()
            })
            .map(|i| i as u32 + 1))
    }
    fn object_gtid(&mut self, _object: u32) -> Result<u32> {
        unreachable!()
    }
    fn null_point(&mut self) -> Result<[u32; 2]> {
        unreachable!()
    }
    fn mind_virtual_140(&mut self, _mind: MindId, gtid: u32, _arena: &mut Arena) -> Result<()> {
        self.events
            .push(json!({"event": "mind_virtual_140", "args": [gtid]}));
        Ok(())
    }
    fn current_performer(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok(self.pointer("current_performer"))
    }
    fn locomotion(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok(self.pointer("locomotion"))
    }
    fn locomotion_follow(&mut self, locomotion: u32, object: u32, delay_bits: u32) -> Result<()> {
        assert_eq!(locomotion, LOCOMOTION);
        let k = self.key(object).to_string();
        self.events
            .push(json!({"event": "locomotion_follow", "object": k, "delay": delay_bits}));
        Ok(())
    }
}
impl JoinActivateWorld for Host {
    fn set_join_target(&mut self, _state: StateId, _gtid: u32) -> Result<()> {
        unreachable!()
    }
}
impl JoinThinkWorld for Host {
    fn join_target(&mut self, _state: StateId) -> Result<u32> {
        Ok(self.target)
    }
}

fn arena_for(c: &Value) -> Arena {
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
    arena
}
fn snapshot(arena: &Arena) -> (Value, Value) {
    let status: serde_json::Map<String, Value> = STATES
        .iter()
        .enumerate()
        .map(|(i, (n, _))| {
            (
                n.to_string(),
                json!(arena.states[&StateId(i as u32 + 1)].status),
            )
        })
        .collect();
    let stack: Vec<Value> = arena.minds[&MindId(1)]
        .stack
        .iter()
        .map(|s| s.map_or(json!(0), |s| json!(state_name(s))))
        .collect();
    (json!(status), json!(stack))
}

#[test]
fn native_uai_join_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 240);
    assert_eq!(f["coverage"]["531AD0"], json!([54, 54]));
    let mut faults = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let mut arena = arena_for(c);
        let mut host = Host {
            c: c.clone(),
            keys: c["objects"].as_object().unwrap().keys().cloned().collect(),
            events: vec![],
            target: u(&c["target"]),
        };
        let result = think(state_id("join"), &mut arena, &mut host);
        assert_eq!(json!(host.events), o["events"], "{name}");
        if !o["fault"].is_null() {
            assert!(matches!(result, Err(Error::World(_))), "{name}");
            faults += 1;
            continue;
        }
        result.unwrap();
        let (status, stack) = snapshot(&arena);
        assert_eq!(status, o["status"], "{name}");
        assert_eq!(stack, o["stack"], "{name}");
        assert_eq!(json!(host.target), o["target"], "{name}");
    }
    assert_eq!(faults, 4);
}
