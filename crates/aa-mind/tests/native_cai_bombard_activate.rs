//! Replays research/phase8-ai/native-cai-bombard-activate.json (original
//! 527B00 KKC_CAIStateBombard::Activate with 52A000, 4D1080 and 52A810 under
//! Unicorn, at PC24/PC53/PC64) against aa_mind::cai_thinks::bombard_activate.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    cai_thinks::bombard_activate::{BombardActivateWorld, activate_bombard},
};
use serde_json::{Value, json};

const STATES: [(&str, u32); 21] = [
    ("sleep", 0x34),
    ("rampage", 0x54),
    ("move", 0x68),
    ("kill", 0xc0),
    ("bombard", 0xd4),
    ("siege", 0xf8),
    ("chasing", 0x120),
    ("construct", 0x134),
    ("repair", 0x160),
    ("guard", 0x17c),
    ("regroup", 0x1a8),
    ("reform", 0x1b8),
    ("engage", 0x1cc),
    ("retreat", 0x1f0),
    ("rout", 0x20c),
    ("explore", 0x220),
    ("join", 0x24c),
    ("exhausted", 0x270),
    ("unpack", 0x280),
    ("pack", 0x2a4),
    ("other", 0x700),
];
const TARGET: u32 = 5;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-bombard-activate.json"
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
    point: [u32; 2],
}
impl World for Host {
    fn can_activate(&mut self, state: StateId, _arena: &mut Arena) -> Result<bool> {
        let n = state_name(state);
        let r = self.c["can_activate"][n].as_bool().unwrap();
        self.events
            .push(json!({"event": "can_activate", "state": n, "result": r}));
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
        self.events
            .push(json!({"event": "reset_default", "args": []}));
        Ok(())
    }
    fn state_changed(&mut self, _m: MindId, _a: &mut Arena) -> Result<()> {
        self.events
            .push(json!({"event": "state_changed", "args": []}));
        Ok(())
    }
}
impl BombardActivateWorld for Host {
    fn component_57a5e0(&mut self, _mind: MindId) -> Result<bool> {
        let r = self.c["component"].as_bool().unwrap();
        self.events
            .push(json!({"event": "component_57a5e0", "result": r}));
        Ok(r)
    }
    fn bombard_target_gtid(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["bombard_target"]))
    }
    fn object_gtid(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, TARGET);
        Ok(u(&self.c["target"]["gtid"]))
    }
    fn mind_word_c(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["mind_word_c"]))
    }
    fn organization_4d1080(&mut self, _mind: MindId) -> Result<bool> {
        Ok(u(&self.c["org_word104"]) & 4 != 0)
    }
    fn organization_relation(&mut self, _mind: MindId, target: u32, a: u32, b: u32) -> Result<i32> {
        assert_eq!(target, TARGET);
        let r = self.c["relation"].as_i64().unwrap() as i32;
        self.events.push(
            json!({"event": "relation", "target": "target", "a": a, "b": b, "result": r}),
        );
        Ok(r)
    }
    fn bombard_set_target(&mut self, state: StateId, target: Option<u32>) -> Result<()> {
        assert_eq!(state_name(state), "bombard");
        let t = target.map_or(json!(0), |_| json!("target"));
        self.events.push(json!({"event": "set_target", "target": t}));
        Ok(())
    }
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]> {
        assert_eq!(object, TARGET);
        let p = &self.c["target"]["pos"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn set_bombard_point(&mut self, _state: StateId, point: [u32; 2]) -> Result<()> {
        self.point = point;
        Ok(())
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
                discard_on_suspend: matches!(*n, "sleep" | "rampage"),
                status: c["states"][*n].as_u64().unwrap_or(0) as u32,
            },
        );
    }
    arena
}

#[test]
fn native_cai_bombard_activate_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 600);
    assert_eq!(f["coverage"]["527B00"], json!([94, 94]));
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let mut arena = arena_for(c);
        let mut host = Host {
            c: c.clone(),
            events: vec![],
            point: [0x1111_1111, 0x2222_2222],
        };
        let target = (!c["target"].is_null()).then_some(TARGET);
        let r = activate_bombard(state_id("bombard"), u(&c["mode"]), target, &mut arena, &mut host)
            .unwrap();
        assert_eq!(json!(host.events), o["events"], "{name}");
        assert_eq!(json!(u8::from(r)), o["result"], "{name}");
        assert_eq!(json!(host.point), o["point"], "{name}");
        assert_eq!(o["bombard_target"], c["bombard_target"], "{name}");
        let states: serde_json::Map<String, Value> = c["tracked"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| {
                let k = k.as_str().unwrap();
                (k.to_string(), json!(arena.states[&state_id(k)].status))
            })
            .collect();
        assert_eq!(json!(states), o["states"], "{name}");
        let stack: Vec<Value> = arena.minds[&MindId(1)]
            .stack
            .iter()
            .map(|s| s.map_or(Value::Null, |s| json!(state_name(s))))
            .collect();
        assert_eq!(json!(stack), o["stack"], "{name}");
    }
}
