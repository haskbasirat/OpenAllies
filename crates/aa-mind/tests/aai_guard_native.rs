//! Replays research/phase8-ai/native-aai-guard.json (original 51C900 KKC_AAIStateGuard::Think with 51C540, 51A550, 517F30,
//! 432E40 and 529C20 under Unicorn, at PC24/PC53/PC64) against
//! aa_mind::uai_thinks::aai_guard.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    uai_thinks::{
        AircraftWorld, ThinkWorld,
        aai_guard::{AirGuardThinkWorld, AttackTargetWorld, think},
    },
};
use aa_numeric::Precision;
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

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-aai-guard.json"
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
    current: u32,
    attack18: u32,
}
impl Host {
    fn object(&self, h: u32) -> &Value {
        &self.c["objects"][&self.keys[(h % 1000) as usize - 1]]
    }
    fn name(&self, h: u32) -> Value {
        if h == 0 {
            json!(0)
        } else {
            json!(self.keys[(h % 1000) as usize - 1])
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
    fn join_activate(&mut self, _mind: MindId, mode: u32, _arena: &mut Arena) -> Result<()> {
        self.events
            .push(json!({"event": "join_activate", "mode": mode}));
        Ok(())
    }
    fn object_removed(&mut self, object: u32) -> Result<bool> {
        Ok(u(&self.object(object)["flags"]) & 1 != 0)
    }
    fn object_accepts(&mut self, _object: u32, _mind: MindId) -> Result<bool> {
        unreachable!()
    }
    fn actor_4d7960(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
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
        _position: [u32; 3],
        _direction: [u32; 3],
        _arena: &mut Arena,
    ) -> Result<()> {
        unreachable!()
    }
    fn vectors_differ(&mut self, _a: [u32; 3], _b: [u32; 3]) -> Result<bool> {
        unreachable!()
    }
    fn object_position3(&mut self, _object: u32) -> Result<[u32; 3]> {
        unreachable!()
    }
    fn object_definition_word(&mut self, _object: u32, _offset: u32) -> Result<u32> {
        unreachable!()
    }
    fn relation(&mut self, _mind: MindId, object: Option<u32>, a: u32, b: u32) -> Result<i32> {
        assert_eq!((a, b), (1, 0));
        let r = object.map_or(0, |o| self.object(o)["relation"].as_i64().unwrap() as i32);
        let k = self.name(object.unwrap_or(0));
        self.events
            .push(json!({"event": "relation", "object": k, "result": r}));
        Ok(r)
    }
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]> {
        let p = &self.object(object)["pos"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn actor_position(&mut self, _mind: MindId) -> Result<[u32; 2]> {
        Ok([u(&self.c["pos"][0]), u(&self.c["pos"][1])])
    }
    fn actor_position3(&mut self, _mind: MindId) -> Result<[u32; 3]> {
        unreachable!()
    }
    fn object_word(&mut self, _object: u32, _offset: u32) -> Result<u32> {
        unreachable!()
    }
    fn actor_word(&mut self, _mind: MindId, _offset: u32) -> Result<u32> {
        unreachable!()
    }
}
impl AttackTargetWorld for Host {
    fn state_target(&mut self, _state: StateId) -> Result<u32> {
        Ok(self.current)
    }
    fn set_state_target(&mut self, _state: StateId, gtid: u32) -> Result<()> {
        self.current = gtid;
        Ok(())
    }
    fn attack_word30(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["attack30"]))
    }
    fn attack_word18(&mut self, _mind: MindId) -> Result<u32> {
        Ok(self.attack18)
    }
    fn move_byte48(&mut self, _mind: MindId) -> Result<u8> {
        Ok(u(&self.c["move48"]) as u8)
    }
    fn attack_activate(
        &mut self,
        _mind: MindId,
        a: u32,
        object: u32,
        c: u32,
        arena: &mut Arena,
    ) -> Result<()> {
        let k = self.name(object);
        self.events
            .push(json!({"event": "attack_activate", "a": a, "object": k, "c": c}));
        let after = self.c["attack_after"].clone();
        if !after.is_null() {
            arena.states.get_mut(&state_id("attack")).unwrap().status = u(&after["status"]);
            if after["word18"] == "object" {
                self.attack18 = u(&self.object(object)["gtid"]);
            }
        }
        Ok(())
    }
}
impl AirGuardThinkWorld for Host {
    fn guarded(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["guarded"]))
    }
    fn denizen_component(&mut self, object: u32) -> Result<Option<u32>> {
        Ok((!self.object(object)["threats"].is_null()).then_some(1000 + object))
    }
    fn component_threats(&mut self, component: u32) -> Result<Vec<u32>> {
        let k = self.name(component);
        self.events
            .push(json!({"event": "component_threats", "object": k}));
        Ok(self.object(component)["threats"]
            .as_array()
            .unwrap()
            .iter()
            .map(u)
            .collect())
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
fn native_aai_guard_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 906);
    assert_eq!(f["coverage"]["51C900"], json!([252, 253]));
    assert_eq!(f["coverage"]["51C540"], json!([52, 52]));
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
            current: u(&c["current"]),
            attack18: u(&c["attack18"]),
        };
        let result = think(state_id("guard"), &mut arena, &mut host, p);
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
        assert_eq!(json!(host.current), o["current"], "{name}");
        assert_eq!(json!(host.attack18), o["attack18"], "{name}");
    }
    assert_eq!(faults, 0);
}
