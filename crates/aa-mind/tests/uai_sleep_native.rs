//! Replays research/phase8-ai/native-uai-sleep.json (original 52F950 KKC_UAIStateSleep::Think with 51E800/51E850, 4D9E20/4CA2D0 and
//! 52A000 under Unicorn, at PC24/PC53/PC64) against
//! aa_mind::uai_thinks::uai_sleep.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    cai_thinks::unit_order::StackAction,
    uai_thinks::{
        ThinkWorld,
        uai_sleep::{SleepThinkWorld, think},
    },
};
use aa_navigation::Activation;
use aa_numeric::{Binary, Precision};
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
const MEMBERSHIP: u32 = 50;
const ORGANIZATION: u32 = 60;
const LEADER: u32 = 70;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-uai-sleep.json"
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
    due: u32,
}
impl Host {
    fn flag(&mut self, who: &str, name: &str) -> bool {
        let r = self.c["flags"][who][name].as_bool().unwrap();
        self.events
            .push(json!({"event": format!("object_{name}"), "object": who, "result": r}));
        r
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
impl SleepThinkWorld for Host {
    fn actor_word110(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["word110"]))
    }
    fn actor_flags100(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["flags100"]))
    }
    fn actor_pending_point(&mut self, _mind: MindId) -> Result<Option<[u32; 2]>> {
        let p = self.c["pending_point"].clone();
        self.events
            .push(json!({"event": "actor_pending_point", "result": p}));
        Ok(p.as_array().map(|p| [u(&p[0]), u(&p[1])]))
    }
    fn move_activate(&mut self, _mind: MindId, r: Activation, _arena: &mut Arena) -> Result<bool> {
        self.events.push(json!({"event": "move_activate", "args": [
            r.activation_mode, r.destination_bits[0], r.destination_bits[1], r.movement_mode,
            r.requested_speed_bits, r.auxiliary_bits[0], r.auxiliary_bits[1]]}));
        Ok(self.c["move_result"].as_bool().unwrap())
    }
    fn membership(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok((!self.c["membership"].is_null()).then_some(MEMBERSHIP))
    }
    fn membership_organization(&mut self, _m: u32) -> Result<Option<u32>> {
        Ok((!self.c["membership"]["organization"].is_null()).then_some(ORGANIZATION))
    }
    fn membership_detached(&mut self, _m: u32) -> Result<bool> {
        Ok(self.c["membership"]["detached"].as_bool().unwrap())
    }
    fn membership_index(&mut self, _m: u32) -> Result<u32> {
        Ok(u(&self.c["membership"]["index"]))
    }
    fn mind_virtual_fc(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "mind_virtual_fc"}));
        Ok(())
    }
    fn actor_word_ac(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["word_ac"]))
    }
    fn object_word_ac(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, LEADER);
        Ok(u(
            &self.c["membership"]["organization"]["leader"]["word_ac"],
        ))
    }
    fn organization_leader(&mut self, _o: u32) -> Result<Option<u32>> {
        Ok((!self.c["membership"]["organization"]["leader"].is_null()).then_some(LEADER))
    }
    fn formation_offset(&mut self, _o: u32, index: u32) -> Result<[u32; 2]> {
        let e = &self.c["membership"]["organization"]["table"][index as usize];
        Ok([u(&e[0]), u(&e[1])])
    }
    fn world_time(&mut self) -> Result<u32> {
        Ok(u(&self.c["time"]))
    }
    fn sleep_time(&mut self, _state: StateId) -> Result<u32> {
        Ok(self.due)
    }
    fn set_sleep_time(&mut self, _state: StateId, bits: u32) -> Result<()> {
        self.due = bits;
        Ok(())
    }
    fn leader_4d1080(&mut self, _leader: u32) -> Result<bool> {
        Ok(self.flag("leader", "4d1080"))
    }
    fn leader_4d10a0(&mut self, _leader: u32) -> Result<bool> {
        Ok(self.flag("leader", "4d10a0"))
    }
    fn leader_4d0980(&mut self, _leader: u32) -> Result<bool> {
        Ok(self.flag("leader", "4d0980"))
    }
    fn actor_4d10a0(&mut self, _mind: MindId) -> Result<bool> {
        Ok(self.flag("actor", "4d10a0"))
    }
    fn definition_534_54(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["reach"]))
    }
    fn leader_virtual_0(&mut self, leader: u32) -> Result<Binary> {
        assert_eq!(leader, LEADER);
        let v = self.c["leader_v0"].clone();
        self.events
            .push(json!({"event": "leader_virtual_0", "result": v}));
        Ok(Binary::from_f64_bits(v.as_f64().unwrap().to_bits()).unwrap())
    }
    fn normalize(&mut self, vector: [u32; 2]) -> Result<[u32; 2]> {
        let r = self.c["normalized"].clone();
        self.events
            .push(json!({"event": "normalize", "vector": vector, "result": r}));
        Ok([u(&r[0]), u(&r[1])])
    }
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]> {
        assert_eq!(object, LEADER);
        let p = &self.c["membership"]["organization"]["leader"]["position"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn issue_action(
        &mut self,
        _mind: MindId,
        action: &StackAction,
        _arena: &mut Arena,
    ) -> Result<bool> {
        let definition = u(&self.c["definitions"][action.ordinal.to_string()]);
        let mut flags = if definition != 0 { 0x40 } else { 0 };
        if action.words18_24.is_some() {
            flags |= 0xc;
        }
        let words = action.words18_24.unwrap_or([0; 4]);
        let r = self.c["issue"].as_bool().unwrap();
        self.events
            .push(json!({"event": "issue_action", "vtable": 0xa3_cc38,
            "definition": definition, "flags": flags, "words": words, "result": r}));
        Ok(r)
    }
    fn resume_pending(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<bool> {
        let r = self.c["resume"].as_bool().unwrap();
        self.events
            .push(json!({"event": "resume_pending", "result": r}));
        Ok(r)
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
fn native_uai_sleep_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 762);
    assert_eq!(f["coverage"]["52F950"], json!([201, 201]));
    assert_eq!(f["coverage"]["51E800"], json!([26, 26]));
    assert_eq!(f["coverage"]["51E850"], json!([8, 8]));
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
            due: u(&c["due"]),
        };
        let result = think(state_id("sleep"), &mut arena, &mut host, p);
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
    }
    assert_eq!(faults, 0);
}
