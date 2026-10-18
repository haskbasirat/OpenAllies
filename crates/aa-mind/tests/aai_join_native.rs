//! Replays research/phase8-ai/native-aai-join.json (original 518D60 KKC_AAIStateJoin::Think with 529EE0 under Unicorn, at
//! PC24/PC53/PC64) against
//! aa_mind::uai_thinks::aai_join.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    uai_thinks::{
        AircraftWorld, ThinkWorld,
        aai_join::{AirJoinThinkWorld, think},
    },
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

const STATES: [(&str, u32); 10] = [
    ("move", 0x34),
    ("guard", 0x9c),
    ("recon", 0xb4),
    ("escort", 0xd0),
    ("attack", 0xe8),
    ("join", 0x12c),
    ("takeoff", 0x15c),
    ("landing", 0x174),
    ("queued", 0x188),
    ("transport", 0x198),
];
const OWNER: u32 = 77;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-aai-join.json"
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

struct Host {
    c: Value,
    events: Vec<Value>,
    keys: Vec<String>,
    target: u32,
    arrived: bool,
    last: [u32; 3],
    point: [u32; 3],
}
impl Host {
    fn object(&self, h: u32) -> &Value {
        &self.c["objects"][&self.keys[h as usize - 1]]
    }
    fn name(&self, h: u32) -> Value {
        if h == 0 {
            json!(0)
        } else {
            json!(self.keys[h as usize - 1])
        }
    }
    fn handle(&self, key: &str) -> u32 {
        self.keys.iter().position(|k| k == key).unwrap() as u32 + 1
    }
}
fn v3(v: &Value) -> [u32; 3] {
    [u(&v[0]), u(&v[1]), u(&v[2])]
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
        Ok(u(&self.object(object)["gtid"]))
    }
    fn null_point(&mut self) -> Result<[u32; 2]> {
        unreachable!()
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
impl AircraftWorld for Host {
    fn join_activate(&mut self, _mind: MindId, _mode: u32, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn object_removed(&mut self, object: u32) -> Result<bool> {
        Ok(u(&self.object(object)["flags"]) & 1 != 0)
    }
    fn object_accepts(&mut self, object: u32, _mind: MindId) -> Result<bool> {
        let r = self.object(object)["accepts"].as_bool().unwrap();
        let k = self.name(object);
        self.events
            .push(json!({"event": "object_accepts", "object": k, "result": r}));
        Ok(r)
    }
    fn actor_4d7960(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "actor_4d7960"}));
        Ok(())
    }
    fn actor_virtual_104(&mut self, _mind: MindId) -> Result<aa_numeric::Binary> {
        unreachable!()
    }
    fn definition_word(&mut self, _mind: MindId, _offset: u32) -> Result<u32> {
        unreachable!()
    }
    fn flight_word(&mut self, _mind: MindId, _offset: u32) -> Result<u32> {
        unreachable!()
    }
    fn set_flight_word(&mut self, _mind: MindId, _offset: u32, _bits: u32) -> Result<()> {
        unreachable!()
    }
    fn flight_byte(&mut self, _mind: MindId, _offset: u32) -> Result<u8> {
        unreachable!()
    }
    fn set_flight_byte(&mut self, _mind: MindId, _offset: u32, _value: u8) -> Result<()> {
        unreachable!()
    }
    fn set_move_word34(&mut self, _mind: MindId, _bits: u32) -> Result<()> {
        unreachable!()
    }
    fn move_fly_to(
        &mut self,
        _mind: MindId,
        position: [u32; 3],
        direction: [u32; 3],
        _arena: &mut Arena,
    ) -> Result<()> {
        self.events
            .push(json!({"event": "move_fly_to", "position": position, "direction": direction}));
        Ok(())
    }
    fn vectors_differ(&mut self, a: [u32; 3], b: [u32; 3]) -> Result<bool> {
        Ok((0..3).any(|k| f32::from_bits(a[k]) != f32::from_bits(b[k])))
    }
    fn object_position3(&mut self, object: u32) -> Result<[u32; 3]> {
        Ok(v3(&self.object(object)["pos"]))
    }
    fn object_definition_word(&mut self, _object: u32, _offset: u32) -> Result<u32> {
        unreachable!()
    }
    fn relation(&mut self, _mind: MindId, _o: Option<u32>, _a: u32, _b: u32) -> Result<i32> {
        unreachable!()
    }
    fn object_position(&mut self, _object: u32) -> Result<[u32; 2]> {
        unreachable!()
    }
    fn actor_position(&mut self, _mind: MindId) -> Result<[u32; 2]> {
        unreachable!()
    }
    fn actor_position3(&mut self, _mind: MindId) -> Result<[u32; 3]> {
        Ok(v3(&self.c["pos"]))
    }
    fn object_word(&mut self, _object: u32, _offset: u32) -> Result<u32> {
        unreachable!()
    }
    fn actor_word(&mut self, _mind: MindId, _offset: u32) -> Result<u32> {
        unreachable!()
    }
}
impl AirJoinThinkWorld for Host {
    fn join_target(&mut self, _state: StateId) -> Result<u32> {
        Ok(self.target)
    }
    fn set_join_target(&mut self, _state: StateId, gtid: u32) -> Result<()> {
        self.target = gtid;
        Ok(())
    }
    fn arrived(&mut self, _state: StateId) -> Result<bool> {
        Ok(self.arrived)
    }
    fn set_arrived(&mut self, _state: StateId, value: bool) -> Result<()> {
        self.arrived = value;
        Ok(())
    }
    fn last_position(&mut self, _state: StateId) -> Result<[u32; 3]> {
        Ok(self.last)
    }
    fn set_last_position(&mut self, _state: StateId, point: [u32; 3]) -> Result<()> {
        self.last = point;
        Ok(())
    }
    fn approach_point(&mut self, _state: StateId) -> Result<[u32; 3]> {
        Ok(self.point)
    }
    fn set_approach_point(&mut self, _state: StateId, point: [u32; 3]) -> Result<()> {
        self.point = point;
        Ok(())
    }
    fn object_virtual_10(&mut self, object: u32) -> Result<u32> {
        Ok(u(&self.object(object)["limit"]))
    }
    fn actor_owner(&mut self, _mind: MindId) -> Result<u32> {
        self.events.push(json!({"event": "actor_owner"}));
        Ok(OWNER)
    }
    fn owner_base(&mut self, owner: u32, _mind: MindId) -> Result<Option<u32>> {
        assert_eq!(owner, OWNER);
        let r = self.c["base"].clone();
        self.events
            .push(json!({"event": "owner_base", "result": r}));
        Ok(r.as_str().map(|k| self.handle(k)))
    }
    fn mind_virtual_40(
        &mut self,
        _mind: MindId,
        object: Option<u32>,
        _arena: &mut Arena,
    ) -> Result<()> {
        let k = self.name(object.unwrap_or(0));
        self.events
            .push(json!({"event": "mind_virtual_40", "args": [k]}));
        Ok(())
    }
    fn definition_370(&mut self, object: u32) -> Result<bool> {
        Ok(self.object(object)["def370"].as_bool().unwrap())
    }
    fn landing_approach(
        &mut self,
        _mind: MindId,
        object: u32,
        point: [u32; 3],
        direction: [u32; 3],
        _arena: &mut Arena,
    ) -> Result<([u32; 3], [u32; 3])> {
        let k = self.name(object);
        self.events.push(
            json!({"event": "landing_approach", "object": k, "point": point,
            "direction": direction}),
        );
        let a = &self.c["approach"];
        Ok((v3(&a[0]), v3(&a[1])))
    }
    fn definition_45d6f0(&mut self, _mind: MindId) -> Result<Binary> {
        let v = self.c["reach"].clone();
        self.events
            .push(json!({"event": "definition_45d6f0", "result": v}));
        Ok(Binary::from_f64_bits(v.as_f64().unwrap().to_bits()).unwrap())
    }
}

fn arena_for(c: &Value) -> Arena {
    let mut arena = Arena::default();
    let mut mind = Mind::base(1);
    for key in c["stack"].as_array().unwrap() {
        mind.stack.push_back(key.as_str().map(state_id));
    }
    if let Some(w) = c["words"].as_array() {
        let w: Vec<u32> = w.iter().map(u).collect();
        mind.words_24_30 = [w[0], w[1], w[2], w[3]];
    }
    arena.minds.insert(MindId(1), mind);
    for (i, (n, _)) in STATES.iter().enumerate() {
        arena.states.insert(
            StateId(i as u32 + 1),
            State {
                parent: MindId(1),
                discard_on_suspend: false,
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
fn native_aai_join_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 909);
    assert_eq!(f["coverage"]["518D60"], json!([285, 294]));
    assert_eq!(f["coverage"]["529EE0"], json!([16, 16]));
    let mut faults = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let mut arena = arena_for(c);
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut host = Host {
            c: c.clone(),
            events: vec![],
            keys: c["objects"].as_object().unwrap().keys().cloned().collect(),
            target: u(&c["target"]),
            arrived: c["arrived"].as_bool().unwrap(),
            last: v3(&c["last"]),
            point: v3(&c["point"]),
        };
        let result = think(state_id("join"), &mut arena, &mut host, p);
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
        assert_eq!(json!(host.target), o["target"], "{name}");
        assert_eq!(json!(u8::from(host.arrived)), o["arrived"], "{name}");
        assert_eq!(json!(host.last), o["last"], "{name}");
        assert_eq!(json!(host.point), o["point"], "{name}");
    }
    assert_eq!(faults, 0);
}
