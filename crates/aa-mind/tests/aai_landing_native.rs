//! Replays research/phase8-ai/native-aai-landing.json (original 519750 KKC_AAIStateLanding::Think with 529EE0, 516720, 517EE0 and
//! 517DF0 under Unicorn, at PC24/PC53/PC64) against
//! aa_mind::uai_thinks::aai_landing.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    uai_thinks::{
        AircraftWorld, ThinkWorld,
        aai_landing::{LandingThinkWorld, think},
    },
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

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-aai-landing.json"
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
    flight: BTreeMap<u32, u32>,
    flight_bytes: BTreeMap<u32, u8>,
    move34: u32,
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
    fn object_gtid(&mut self, _object: u32) -> Result<u32> {
        unreachable!()
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
    fn actor_virtual_104(&mut self, _mind: MindId) -> Result<Binary> {
        let v = self.c["v104"].clone();
        self.events
            .push(json!({"event": "actor_virtual_104", "result": v}));
        Ok(Binary::from_f64_bits(v.as_f64().unwrap().to_bits()).unwrap())
    }
    fn definition_word(&mut self, _mind: MindId, offset: u32) -> Result<u32> {
        Ok(u(&self.c["definition"][format!("{offset:x}")]))
    }
    fn flight_word(&mut self, _mind: MindId, offset: u32) -> Result<u32> {
        Ok(self.flight[&offset])
    }
    fn set_flight_word(&mut self, _mind: MindId, offset: u32, bits: u32) -> Result<()> {
        self.flight.insert(offset, bits);
        Ok(())
    }
    fn flight_byte(&mut self, _mind: MindId, offset: u32) -> Result<u8> {
        Ok(self.flight_bytes[&offset])
    }
    fn set_flight_byte(&mut self, _mind: MindId, offset: u32, value: u8) -> Result<()> {
        self.flight_bytes.insert(offset, value);
        Ok(())
    }
    fn set_move_word34(&mut self, _mind: MindId, bits: u32) -> Result<()> {
        self.move34 = bits;
        Ok(())
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
    fn object_definition_word(&mut self, object: u32, offset: u32) -> Result<u32> {
        Ok(u(&self.object(object)["definition"][format!("{offset:x}")]))
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
        unreachable!()
    }
    fn object_word(&mut self, _object: u32, _offset: u32) -> Result<u32> {
        unreachable!()
    }
    fn actor_word(&mut self, _mind: MindId, _offset: u32) -> Result<u32> {
        unreachable!()
    }
}
impl LandingThinkWorld for Host {
    fn landing_base(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["base"]))
    }
    fn object_heading(&mut self, object: u32) -> Result<[u32; 2]> {
        let h = &self.object(object)["heading"];
        Ok([u(&h[0]), u(&h[1])])
    }
    fn object_definition_byte(&mut self, object: u32, offset: u32) -> Result<u8> {
        assert_eq!(offset, 0x3a8);
        Ok(u(&self.object(object)["path2"]) as u8)
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
fn native_aai_landing_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 1020);
    assert_eq!(f["coverage"]["519750"], json!([590, 592]));
    assert_eq!(f["coverage"]["517EE0"], json!([22, 22]));
    assert_eq!(f["coverage"]["517DF0"], json!([64, 64]));
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
            flight: [0x14_u32, 0x18, 0x34, 0x80, 0x84, 0x8c]
                .into_iter()
                .map(|o| {
                    let v = c["flight"][format!("{o:x}")]
                        .as_u64()
                        .map_or(0x1111_0000 + o, |v| v as u32);
                    (o, v)
                })
                .collect(),
            flight_bytes: [(0x10, 0x5a), (0x90, u(&c["flight90"]) as u8)].into(),
            move34: 0x3333_3333,
        };
        let result = think(state_id("landing"), &mut arena, &mut host, p);
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
        let flight: serde_json::Map<String, Value> = host
            .flight
            .iter()
            .map(|(k, v)| (format!("{k:x}"), json!(v)))
            .collect();
        assert_eq!(json!(flight), o["flight"], "{name}");
        assert_eq!(json!(host.move34), o["move34"], "{name}");
    }
    assert_eq!(faults, 0);
}
