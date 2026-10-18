//! Replays research/phase8-ai/native-aai-attack.json (original 51B8C0
//! KKC_AAIStateAttack::Think with 529E60, 529C30, 516720, 401060, 401290,
//! 445250, 45E790 and 46B000 under Unicorn, at PC24/PC53/PC64) against
//! aa_mind::uai_thinks::aai_attack.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    uai_thinks::{
        AircraftWorld, ThinkWorld,
        aai_attack::{AirAttackThinkWorld, think},
    },
    x87::binary_from_x87,
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::collections::BTreeMap;

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

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn state_name(s: StateId) -> &'static str {
    STATES[s.0 as usize - 1].0
}
fn state_id(name: &str) -> StateId {
    StateId(STATES.iter().position(|(n, _)| *n == name).unwrap() as u32 + 1)
}
fn x87(v: &Value) -> Result<Binary> {
    binary_from_x87(
        v["significand"].as_u64().unwrap(),
        v["sign_exponent"].as_u64().unwrap() as u16,
    )
}

struct Host {
    c: Value,
    events: Vec<Value>,
    keys: Vec<String>,
    attack: BTreeMap<u32, u32>,
    bytes: BTreeMap<u32, u8>,
    flight: BTreeMap<u32, u32>,
    flight_b10: u8,
}
impl Host {
    fn object(&self, h: u32) -> &Value {
        &self.c["objects"][&self.keys[h as usize - 1]]
    }
    fn key(&self, h: u32) -> String {
        self.keys[h as usize - 1].clone()
    }
    fn handle(&self, key: &str) -> u32 {
        self.keys.iter().position(|k| k == key).unwrap() as u32 + 1
    }
    fn ev(&mut self, v: Value) {
        self.events.push(v);
    }
}

impl World for Host {
    fn can_activate(&mut self, state: StateId, _arena: &mut Arena) -> Result<bool> {
        self.ev(json!({"event": "can_activate", "state": state_name(state), "result": true}));
        Ok(true)
    }
    fn enter(&mut self, state: StateId, _arena: &mut Arena) -> Result<()> {
        self.ev(json!({"event": "enter", "state": state_name(state)}));
        Ok(())
    }
    fn leave(&mut self, state: StateId, _arena: &mut Arena) -> Result<()> {
        self.ev(json!({"event": "leave", "state": state_name(state)}));
        Ok(())
    }
    fn schedule(&mut self, _m: MindId, delay_bits: u32, data: u32, _a: &mut Arena) -> Result<()> {
        self.ev(json!({"event": "schedule", "delay": delay_bits, "data": data}));
        Ok(())
    }
    fn reset_default(&mut self, _m: MindId, _a: &mut Arena) -> Result<()> {
        self.ev(json!({"event": "reset_default"}));
        Ok(())
    }
    fn state_changed(&mut self, _m: MindId, _a: &mut Arena) -> Result<()> {
        self.ev(json!({"event": "state_changed"}));
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
        self.ev(json!({"event": "embedded_think", "state": name}));
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
        self.ev(json!({"event": "join_activate", "mode": mode}));
        Ok(())
    }
    fn object_removed(&mut self, _object: u32) -> Result<bool> {
        unreachable!()
    }
    fn object_accepts(&mut self, _object: u32, _mind: MindId) -> Result<bool> {
        unreachable!()
    }
    fn actor_4d7960(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn actor_virtual_104(&mut self, _mind: MindId) -> Result<Binary> {
        self.ev(json!({"event": "actor_virtual_104"}));
        x87(&self.c["actor"]["actor104"])
    }
    fn definition_word(&mut self, _mind: MindId, _offset: u32) -> Result<u32> {
        unreachable!()
    }
    fn flight_word(&mut self, _mind: MindId, offset: u32) -> Result<u32> {
        Ok(self.flight.get(&offset).copied().unwrap_or(0))
    }
    fn set_flight_word(&mut self, _mind: MindId, offset: u32, bits: u32) -> Result<()> {
        self.flight.insert(offset, bits);
        Ok(())
    }
    fn flight_byte(&mut self, _mind: MindId, _offset: u32) -> Result<u8> {
        Ok(self.flight_b10)
    }
    fn set_flight_byte(&mut self, _mind: MindId, offset: u32, value: u8) -> Result<()> {
        assert_eq!(offset, 0x10);
        self.flight_b10 = value;
        Ok(())
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
        let o = object.unwrap();
        let r = self.object(o)["relation"].as_i64().unwrap() as i32;
        let k = self.key(o);
        self.ev(json!({"event": "relation", "object": k, "result": r}));
        Ok(r)
    }
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]> {
        let p = &self.object(object)["pos"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn actor_position(&mut self, _mind: MindId) -> Result<[u32; 2]> {
        unreachable!()
    }
    fn actor_position3(&mut self, _mind: MindId) -> Result<[u32; 3]> {
        let p = &self.c["actor"]["pos"];
        Ok([u(&p[0]), u(&p[1]), u(&p[2])])
    }
    fn object_word(&mut self, object: u32, offset: u32) -> Result<u32> {
        let o = self.object(object);
        Ok(match offset {
            0x7c => u(&o["w7c"]),
            0xbc => u(&o["wbc"]),
            0x6c => u(&o["w6c"]),
            0x2c..=0x34 => u(&o["dir"][((offset - 0x2c) / 4) as usize]),
            _ => unreachable!(),
        })
    }
    fn actor_word(&mut self, _mind: MindId, offset: u32) -> Result<u32> {
        Ok(u(&self.c["actor"]["dir"][((offset - 0x2c) / 4) as usize]))
    }
}
impl AirAttackThinkWorld for Host {
    fn attack_word(&mut self, _state: StateId, offset: u32) -> Result<u32> {
        Ok(self.attack[&offset])
    }
    fn set_attack_word(&mut self, _state: StateId, offset: u32, bits: u32) -> Result<()> {
        self.attack.insert(offset, bits);
        Ok(())
    }
    fn attack_byte(&mut self, _state: StateId, offset: u32) -> Result<u8> {
        Ok(self.bytes[&offset])
    }
    fn set_attack_byte(&mut self, _state: StateId, offset: u32, value: u8) -> Result<()> {
        self.bytes.insert(offset, value);
        Ok(())
    }
    fn mind_byte(&mut self, _mind: MindId, offset: u32) -> Result<u8> {
        assert_eq!(offset, 0x1d0);
        Ok(u(&self.c["mind_b1d0"]) as u8)
    }
    fn flight_airborne(&mut self, _mind: MindId) -> Result<bool> {
        let r = self.c["airborne"].as_bool().unwrap();
        self.ev(json!({"event": "flight_airborne", "result": r}));
        Ok(r)
    }
    fn flight_reset_28(&mut self, _mind: MindId) -> Result<()> {
        self.ev(json!({"event": "flight_reset_28"}));
        Ok(())
    }
    fn group_5685b0(&mut self, group: u32) -> Result<Option<u32>> {
        assert_eq!(group, u(&self.c["words"][3]));
        let r = self.c["group_object"].clone();
        self.ev(json!({"event": "group_5685b0", "result": r}));
        Ok(r.as_str().map(|k| self.handle(k)))
    }
    fn object_mind_4c(&mut self, object: u32) -> Result<Option<u32>> {
        let k = self.key(object);
        let r = self.object(object)["mind4c"].clone();
        self.ev(json!({"event": "object_mind_4c", "object": k, "result": r}));
        Ok(r.as_str().map(|k| self.handle(k)))
    }
    fn attack_518c40(&mut self, _state: StateId, _arena: &mut Arena) -> Result<()> {
        self.helper("attack_518c40");
        Ok(())
    }
    fn attack_51a920(&mut self, _state: StateId, _arena: &mut Arena) -> Result<()> {
        self.helper("attack_51a920");
        Ok(())
    }
    fn attack_51b0c0(&mut self, _state: StateId, _arena: &mut Arena) -> Result<()> {
        self.helper("attack_51b0c0");
        Ok(())
    }
    fn attack_51b4c0(&mut self, _state: StateId, _arena: &mut Arena) -> Result<()> {
        self.helper("attack_51b4c0");
        Ok(())
    }
    fn attack_51b290(&mut self, _state: StateId, _arena: &mut Arena) -> Result<(bool, u32)> {
        let ok = self.c["b290"][0].as_bool().unwrap();
        self.ev(json!({"event": "attack_51b290", "result": ok}));
        Ok((ok, u(&self.c["b290"][1])))
    }
    fn ground_height(&mut self, xy: [u32; 2], flags: u32) -> Result<Binary> {
        assert_eq!(flags, 0x123);
        self.ev(json!({"event": "ground_height", "xy": xy}));
        x87(&self.c["ground"])
    }
    fn actor_virtual_18(&mut self, _mind: MindId) -> Result<Binary> {
        self.ev(json!({"event": "actor_virtual_18"}));
        x87(&self.c["actor"]["actor18"])
    }
    fn object_virtual_18(&mut self, object: u32) -> Result<Binary> {
        let k = self.key(object);
        self.ev(json!({"event": "object_virtual_18", "object": k}));
        x87(&self.object(object)["v18"])
    }
    fn object_virtual_104(&mut self, object: u32) -> Result<Binary> {
        let k = self.key(object);
        self.ev(json!({"event": "object_virtual_104", "object": k}));
        x87(&self.object(object)["v104"])
    }
    fn move_to_point(&mut self, _mind: MindId, point: [u32; 3], _arena: &mut Arena) -> Result<()> {
        self.ev(json!({"event": "move_to_point", "point": point}));
        Ok(())
    }
}
impl Host {
    fn helper(&mut self, name: &str) {
        self.ev(json!({"event": name}));
        if let Some(after) = self.c["after"][name].as_object().cloned() {
            for (off, v) in after {
                let off = u32::from_str_radix(off.trim_start_matches("0x"), 16).unwrap();
                self.attack.insert(off, u(&v));
            }
        }
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
                discard_on_suspend: false,
                status: u(&c["status"][*n]),
            },
        );
    }
    arena
}

#[test]
fn native_aai_attack_think_matches_original() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-aai-attack.json"
    );
    let f: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 1200);
    let mut faults = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = c["name"].as_str().unwrap();
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let keys: Vec<String> = c["objects"].as_object().unwrap().keys().cloned().collect();
        let mut attack = BTreeMap::new();
        for off in [0x10u32, 0x18, 0x1c, 0x24, 0x28, 0x2c, 0x30, 0x38] {
            attack.insert(off, u(&c["attack"][format!("w{off:x}")]));
        }
        let mut bytes = BTreeMap::new();
        bytes.insert(0x14u32, u(&c["attack"]["b14"]) as u8);
        bytes.insert(0x20u32, u(&c["attack"]["b20"]) as u8);
        let mut host = Host {
            c: c.clone(),
            events: vec![],
            keys,
            attack,
            bytes,
            flight: BTreeMap::new(),
            flight_b10: 0,
        };
        let mut arena = arena_for(c);
        let result = think(state_id("attack"), &mut arena, &mut host, p);
        assert_eq!(json!(host.events), o["events"], "{name}: events");
        if !o["fault"].is_null() {
            assert!(result.is_err(), "{name}: native faulted");
            faults += 1;
            continue;
        }
        result.unwrap_or_else(|e| panic!("{name}: {e}"));
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
        assert_eq!(json!(status), o["status"], "{name}: status");
        let mut got = serde_json::Map::new();
        for off in [0x10u32, 0x18, 0x24, 0x28, 0x2c, 0x30] {
            got.insert(format!("w{off:x}"), json!(host.attack[&off]));
        }
        got.insert("b14".into(), json!(host.bytes[&0x14]));
        got.insert("b20".into(), json!(host.bytes[&0x20]));
        assert_eq!(Value::Object(got), o["attack"], "{name}: attack state");
        let flight = json!({
            "b10": host.flight_b10,
            "w18": host.flight.get(&0x18).copied().unwrap_or(0),
            "w80": host.flight.get(&0x80).copied().unwrap_or(0),
        });
        assert_eq!(flight, o["flight"], "{name}: flight");
    }
    assert!(faults > 0);
    assert_eq!(f["coverage"]["51B8C0"], json!([589, 592]));
}
