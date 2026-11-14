//! Replays research/phase8-ai/native-uai-unpack-events.json (original
//! 52EB90 unpack Enter and 52EB50 unpack/reattach Leave with 52A240 and
//! 529C20 under Unicorn, at PC24/PC53/PC64) against
//! aa_mind::uai_unpack_events.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    uai_thinks::ThinkWorld,
    uai_unpack_events::{UnpackEnterWorld, enter, leave},
};
use aa_numeric::Precision;
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
        "/../../research/phase8-ai/native-uai-unpack-events.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
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
    fn can_activate(&mut self, _state: StateId, _arena: &mut Arena) -> Result<bool> {
        unreachable!()
    }
    fn enter(&mut self, _state: StateId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn leave(&mut self, state: StateId, _arena: &mut Arena) -> Result<()> {
        self.events
            .push(json!({"event": "leave", "state": state_name(state)}));
        Ok(())
    }
    fn schedule(&mut self, _m: MindId, _d: u32, _data: u32, _a: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn reset_default(&mut self, _m: MindId, _a: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn state_changed(&mut self, _m: MindId, _a: &mut Arena) -> Result<()> {
        unreachable!()
    }
}
impl ThinkWorld for Host {
    fn embedded_state(&mut self, _mind: MindId, offset: u32) -> Result<StateId> {
        Ok(StateId(
            STATES.iter().position(|(_, o)| *o == offset).unwrap() as u32 + 1,
        ))
    }
    fn embedded_think(&mut self, _mind: MindId, _offset: u32, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn resolve(&mut self, _gtid: u32) -> Result<Option<u32>> {
        unreachable!()
    }
    fn object_gtid(&mut self, _object: u32) -> Result<u32> {
        unreachable!()
    }
    fn null_point(&mut self) -> Result<[u32; 2]> {
        Ok(pair(&self.c["null_point"]))
    }
    fn mind_virtual_140(&mut self, _mind: MindId, _gtid: u32, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn current_performer(&mut self, _mind: MindId) -> Result<Option<u32>> {
        unreachable!()
    }
    fn locomotion(&mut self, _mind: MindId) -> Result<Option<u32>> {
        unreachable!()
    }
    fn locomotion_follow(&mut self, _l: u32, _object: u32, _delay_bits: u32) -> Result<()> {
        unreachable!()
    }
}
impl UnpackEnterWorld for Host {
    fn unpack_point(&mut self, _state: StateId) -> Result<[u32; 2]> {
        Ok(pair(&self.c["point"]))
    }
    fn actor_position(&mut self, _mind: MindId) -> Result<[u32; 2]> {
        Ok(pair(&self.c["pos"]))
    }
    fn move_order(&mut self, _mind: MindId, args: [u32; 7], _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "move_order", "args": args}));
        Ok(())
    }
}

#[test]
fn native_uai_unpack_events_match_original() {
    let f = fixture();
    assert_eq!(f["coverage"]["52EB90"], json!([33, 33]));
    assert_eq!(f["coverage"]["52EB50"], json!([8, 8]));
    assert_eq!(f["coverage"]["52A240"], json!([28, 28]));
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 600);
    let mut orders = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut arena = Arena::default();
        arena.minds.insert(MindId(1), Mind::base(1));
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
        if c["entry"] == "enter" {
            enter(state_id("unpack"), &mut arena, &mut host, p).unwrap();
        } else {
            leave(state_id(c["this"].as_str().unwrap()), &mut arena, &mut host).unwrap();
        }
        assert_eq!(json!(host.events), o["events"], "{name}");
        let status: serde_json::Map<String, Value> = STATES
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (n.to_string(), json!(arena.states[&StateId(i as u32 + 1)].status)))
            .collect();
        assert_eq!(json!(status), o["status"], "{name}");
        orders += host.events.iter().filter(|e| e["event"] == "move_order").count();
    }
    assert_eq!(orders, 125);
}
