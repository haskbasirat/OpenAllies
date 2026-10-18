//! Replays research/phase8-ai/native-uai-unpack.json (original 5372B0 KKC_UAIStateUnpack::Think with 52EBF0 under Unicorn, at
//! PC24/PC53/PC64) against
//! aa_mind::uai_thinks::uai_unpack.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    uai_thinks::{
        ThinkWorld,
        uai_unpack::{UnpackThinkWorld, think},
    },
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
const PACK: u32 = 30;
const ORGANIZATION: u32 = 60;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-uai-unpack.json"
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
    position: [u32; 2],
    destination: [u32; 2],
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
        Ok([u(&self.c["null_point"][0]), u(&self.c["null_point"][1])])
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
impl UnpackThinkWorld for Host {
    fn organization(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok((!self.c["organization"].is_null()).then_some(ORGANIZATION))
    }
    fn actor_is_leader(&mut self, _mind: MindId, organization: u32) -> Result<bool> {
        assert_eq!(organization, ORGANIZATION);
        Ok(self.c["organization"]["leader"] == "actor")
    }
    fn pack_component(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok(self.c["pack"].as_bool().unwrap().then_some(PACK))
    }
    fn unpack_at(&mut self, component: u32, point: [u32; 2], _arena: &mut Arena) -> Result<bool> {
        assert_eq!(component, PACK);
        let r = self.c["unpack_result"].as_bool().unwrap();
        self.events
            .push(json!({"event": "unpack_at", "point": point, "result": r}));
        Ok(r)
    }
    fn unpack_position(&mut self, _state: StateId) -> Result<[u32; 2]> {
        Ok(self.position)
    }
    fn set_unpack_position(&mut self, _state: StateId, point: [u32; 2]) -> Result<()> {
        self.position = point;
        Ok(())
    }
    fn unpack_facing(&mut self, _state: StateId) -> Result<[u32; 2]> {
        Ok([u(&self.c["facing"][0]), u(&self.c["facing"][1])])
    }
    fn place(
        &mut self,
        _state: StateId,
        point: [u32; 2],
        facing: [u32; 2],
        arena: &mut Arena,
    ) -> Result<Option<[u32; 2]>> {
        let r = self.c["place"].clone();
        self.events
            .push(json!({"event": "place", "point": point, "facing": facing, "result": r}));
        if let Some(s) = self.c["place_move_status"].as_u64() {
            arena.states.get_mut(&state_id("move")).unwrap().status = s as u32;
        }
        Ok(r.as_array().map(|p| [u(&p[0]), u(&p[1])]))
    }
    fn points_equal(&mut self, a: [u32; 2], b: [u32; 2]) -> Result<bool> {
        let f = |x: u32| f32::from_bits(x);
        Ok(f(a[0]) == f(b[0]) && f(a[1]) == f(b[1]))
    }
    fn unpack_offset(&mut self, _mind: MindId) -> Result<[u32; 2]> {
        Ok([u(&self.c["offset"][0]), u(&self.c["offset"][1])])
    }
    fn set_move_destination(&mut self, _mind: MindId, point: [u32; 2]) -> Result<()> {
        self.destination = point;
        Ok(())
    }
    fn move_activate(&mut self, _mind: MindId, r: Activation, _arena: &mut Arena) -> Result<bool> {
        self.events.push(json!({"event": "move_activate", "args": [
            r.activation_mode, r.destination_bits[0], r.destination_bits[1], r.movement_mode,
            r.requested_speed_bits, r.auxiliary_bits[0], r.auxiliary_bits[1]]}));
        Ok(true)
    }
    fn mind_virtual_1b0(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "mind_virtual_1b0"}));
        Ok(())
    }
    fn actor_name(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<String> {
        self.events.push(json!({"event": "actor_name"}));
        Ok("actor name".into())
    }
    fn display_string(&mut self, key: &str, _arena: &mut Arena) -> Result<String> {
        self.events
            .push(json!({"event": "display_string", "key": key}));
        Ok(format!("display:{key}"))
    }
    fn actor_owner(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<u32> {
        let r = u(&self.c["owner"]);
        self.events
            .push(json!({"event": "actor_owner", "result": r}));
        Ok(r)
    }
    fn post_message(
        &mut self,
        _mind: MindId,
        kind: u32,
        owner: u32,
        format: &str,
        argument: &str,
        _arena: &mut Arena,
    ) -> Result<()> {
        self.events.push(
            json!({"event": "post_message", "kind": kind, "owner": owner,
            "format": format, "argument": argument}),
        );
        Ok(())
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
fn native_uai_unpack_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 540);
    assert_eq!(f["coverage"]["5372B0"], json!([211, 211]));
    assert_eq!(f["coverage"]["52EBF0"], json!([15, 15]));
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
            position: [u(&c["position"][0]), u(&c["position"][1])],
            destination: [0x1111_1111, 0x2222_2222],
        };
        let result = think(state_id("unpack"), &mut arena, &mut host, p);
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
        assert_eq!(json!(host.position), o["position"], "{name}");
        assert_eq!(json!(host.destination), o["move_destination"], "{name}");
    }
    assert_eq!(faults, 17);
}
