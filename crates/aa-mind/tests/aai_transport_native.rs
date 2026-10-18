//! Replays research/phase8-ai/native-aai-transport.json (original 5185D0 KKC_AAIStateTransport::Think under Unicorn) against
//! aa_mind::uai_thinks::aai_transport.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    uai_thinks::{
        AircraftWorld, ThinkWorld,
        aai_transport::{TransportThinkWorld, think},
    },
};
use aa_numeric::Binary;
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
const BASE: u32 = 40;
fn f64v(v: &Value) -> Binary {
    Binary::from_f64_bits(v.as_f64().unwrap().to_bits()).unwrap()
}

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-aai-transport.json"
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
    pending: bool,
    move34: u32,
    flight: std::collections::BTreeMap<u32, u32>,
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
    fn resolve(&mut self, _gtid: u32) -> Result<Option<u32>> {
        unreachable!()
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
    fn join_activate(&mut self, _mind: MindId, mode: u32, _arena: &mut Arena) -> Result<()> {
        self.events
            .push(json!({"event": "join_activate", "mode": mode}));
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
        let v = self.c["v104"].clone();
        self.events
            .push(json!({"event": "actor_virtual_104", "result": v}));
        Ok(f64v(&v))
    }
    fn definition_word(&mut self, _mind: MindId, _offset: u32) -> Result<u32> {
        unreachable!()
    }
    fn flight_word(&mut self, _mind: MindId, _offset: u32) -> Result<u32> {
        unreachable!()
    }
    fn set_flight_word(&mut self, _mind: MindId, offset: u32, bits: u32) -> Result<()> {
        self.flight.insert(offset, bits);
        Ok(())
    }
    fn flight_byte(&mut self, _mind: MindId, _offset: u32) -> Result<u8> {
        unreachable!()
    }
    fn set_flight_byte(&mut self, _mind: MindId, _offset: u32, _value: u8) -> Result<()> {
        unreachable!()
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
    fn vectors_differ(&mut self, _a: [u32; 3], _b: [u32; 3]) -> Result<bool> {
        unreachable!()
    }
    fn object_position3(&mut self, _object: u32) -> Result<[u32; 3]> {
        unreachable!()
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
        unreachable!()
    }
    fn object_word(&mut self, _object: u32, _offset: u32) -> Result<u32> {
        unreachable!()
    }
    fn actor_word(&mut self, _mind: MindId, _offset: u32) -> Result<u32> {
        unreachable!()
    }
}
impl TransportThinkWorld for Host {
    fn pending(&mut self, _state: StateId) -> Result<bool> {
        Ok(self.pending)
    }
    fn set_pending(&mut self, _state: StateId, value: bool) -> Result<()> {
        self.pending = value;
        Ok(())
    }
    fn transport_step(&mut self, _state: StateId, _arena: &mut Arena) -> Result<bool> {
        let r = self.c["step"].as_bool().unwrap();
        self.events
            .push(json!({"event": "transport_step", "result": r}));
        Ok(r)
    }
    fn base(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok((!self.c["base"].is_null()).then_some(BASE))
    }
    fn base_released(&mut self, base: u32) -> Result<bool> {
        assert_eq!(base, BASE);
        Ok(self.c["base"]["released"].as_bool().unwrap())
    }
    fn base_landing_point(&mut self, _base: u32) -> Result<[u32; 2]> {
        let p = &self.c["base"]["point"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn actor_vector(&mut self, _mind: MindId) -> Result<[u32; 3]> {
        let v = &self.c["vector"];
        Ok([u(&v[0]), u(&v[1]), u(&v[2])])
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
fn native_aai_transport_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 120);
    assert_eq!(f["coverage"]["5185D0"], json!([99, 99]));
    let mut faults = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let mut arena = arena_for(c);
        let mut host = Host {
            c: c.clone(),
            events: vec![],
            pending: c["pending"].as_bool().unwrap(),
            move34: 0x3333_3333,
            flight: [(0x14, 0x1111_1111), (0x84, 0x2222_2222)].into(),
        };
        let result = think(state_id("transport"), &mut arena, &mut host);
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
        assert_eq!(json!(u8::from(host.pending)), o["pending"], "{name}");
        assert_eq!(json!(host.move34), o["move34"], "{name}");
        assert_eq!(
            json!([host.flight[&0x14], host.flight[&0x84]]),
            o["flight"],
            "{name}"
        );
    }
    assert_eq!(faults, 8);
}
