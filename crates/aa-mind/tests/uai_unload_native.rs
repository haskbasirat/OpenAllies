//! Replays research/phase8-ai/native-uai-unload.json (original 52F440
//! KKC_UAIStateUnload::Think with 52A340, 4D09A0, 52EAE0 and 52A810 under
//! Unicorn, at PC24/PC53/PC64) against aa_mind::uai_thinks::uai_unload.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    uai_thinks::{ThinkWorld, uai_join::JoinActivateWorld, uai_unload::*},
};
use aa_navigation::Activation;
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
const TRANSPORT: u32 = 70;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-uai-unload.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn i(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}
fn state_name(s: StateId) -> &'static str {
    STATES[s.0 as usize - 1].0
}
fn state_id(name: &str) -> StateId {
    StateId(STATES.iter().position(|(n, _)| *n == name).unwrap() as u32 + 1)
}

struct Host {
    c: Value,
    keys: Vec<String>,
    events: Vec<Value>,
    due: u32,
    join_target: u32,
    loaded: i32,
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
    fn object_gtid(&mut self, object: u32) -> Result<u32> {
        Ok(u(
            &self.c["objects"][&self.keys[object as usize - 1]]["gtid"],
        ))
    }
    fn null_point(&mut self) -> Result<[u32; 2]> {
        Ok([u(&self.c["null_point"][0]), u(&self.c["null_point"][1])])
    }
    fn mind_virtual_140(&mut self, _mind: MindId, _gtid: u32, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn current_performer(&mut self, _mind: MindId) -> Result<Option<u32>> {
        unreachable!()
    }
    fn locomotion(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok(self.c["locomotion"].as_bool().unwrap().then_some(80))
    }
    fn locomotion_follow(&mut self, _l: u32, _object: u32, _delay_bits: u32) -> Result<()> {
        unreachable!()
    }
}
impl JoinActivateWorld for Host {
    fn set_join_target(&mut self, _state: StateId, gtid: u32) -> Result<()> {
        self.join_target = gtid;
        Ok(())
    }
}
impl UnloadThinkWorld for Host {
    fn world_time(&mut self) -> Result<u32> {
        Ok(u(&self.c["time"]))
    }
    fn unload_time(&mut self, _state: StateId) -> Result<u32> {
        Ok(self.due)
    }
    fn set_unload_time(&mut self, _state: StateId, bits: u32) -> Result<()> {
        self.due = bits;
        Ok(())
    }
    fn transport(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok((!self.c["transport"].is_null()).then_some(TRANSPORT))
    }
    fn transport_update(&mut self, transport: u32) -> Result<()> {
        assert_eq!(transport, TRANSPORT);
        self.events.push(json!({"event": "transport_update"}));
        if let Some(a) = self.c["transport"]["after"].as_i64() {
            self.loaded = a as i32;
        }
        Ok(())
    }
    fn transport_counts(&mut self, _transport: u32) -> Result<[i32; 2]> {
        Ok([self.loaded, i(&self.c["transport"]["capacity"])])
    }
    fn unload_interval(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["interval"]))
    }
    fn actor_flags(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["actor_flags"]))
    }
    fn transport_byte4c(&mut self, _transport: u32) -> Result<bool> {
        Ok(self.c["transport"]["byte4c"].as_bool().unwrap())
    }
    fn transport_point(&mut self, _transport: u32) -> Result<[u32; 2]> {
        let p = &self.c["transport"]["point"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn move_activate(&mut self, _mind: MindId, r: Activation, _arena: &mut Arena) -> Result<bool> {
        self.events.push(json!({"event": "move_activate", "args": [
            r.activation_mode, r.destination_bits[0], r.destination_bits[1], r.movement_mode,
            r.requested_speed_bits, r.auxiliary_bits[0], r.auxiliary_bits[1]]}));
        Ok(self.c["move_result"].as_bool().unwrap())
    }
}

fn arena_for(c: &Value) -> Arena {
    let mut arena = Arena::default();
    let mut mind = Mind::base(1);
    for key in c["stack"].as_array().unwrap() {
        mind.stack.push_back(key.as_str().map(state_id));
    }
    let w: Vec<u32> = c["words"].as_array().unwrap().iter().map(u).collect();
    mind.words_24_30 = [w[0], w[1], w[2], w[3]];
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
fn native_uai_unload_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 330);
    assert_eq!(f["coverage"]["52F440"], json!([88, 88]));
    assert_eq!(f["coverage"]["52EAE0"], json!([35, 35]));
    let mut faults = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut arena = arena_for(c);
        let mut host = Host {
            c: c.clone(),
            keys: c["objects"].as_object().unwrap().keys().cloned().collect(),
            events: vec![],
            due: u(&c["due"]),
            join_target: 0,
            loaded: c["transport"]["loaded"].as_i64().unwrap_or(0) as i32,
        };
        let result = think(state_id("unload"), &mut arena, &mut host, p);
        assert_eq!(json!(host.events), o["events"], "{name}");
        if !o["fault"].is_null() {
            assert!(result.is_err(), "{name}");
            faults += 1;
            continue;
        }
        result.unwrap();
        let (status, stack) = snapshot(&arena);
        assert_eq!(status, o["status"], "{name}");
        assert_eq!(stack, o["stack"], "{name}");
        assert_eq!(json!(host.due), o["due"], "{name}");
        assert_eq!(json!(host.join_target), o["join_target"], "{name}");
    }
    assert_eq!(faults, 29);
}
