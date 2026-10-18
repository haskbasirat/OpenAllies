//! Replays research/phase8-ai/native-uai-engage.json (original 536E10
//! KKC_UAIStateEngage::Think with 5358A0, 529C20 and 52E830 under Unicorn,
//! at PC24/PC53/PC64) against aa_mind::uai_thinks::uai_engage.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    uai_thinks::{
        ThinkWorld,
        uai_engage::{EngageThinkWorld, think},
    },
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
const ORGANIZATION: u32 = 60;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-uai-engage.json"
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

/// Objects are handles index+1 in key order.
struct Host {
    c: Value,
    keys: Vec<String>,
    events: Vec<Value>,
    reselect: bool,
    rng: u32,
}
impl Host {
    fn name(&self, h: Option<u32>) -> Value {
        h.map_or(json!(0), |h| json!(self.keys[h as usize - 1]))
    }
    fn handle(&self, key: &str) -> u32 {
        self.keys.iter().position(|k| k == key).unwrap() as u32 + 1
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
        Ok(u(
            &self.c["objects"][&self.keys[object as usize - 1]]["gtid"],
        ))
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
impl EngageThinkWorld for Host {
    fn engage_target(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["target"]))
    }
    fn reselect(&mut self, _state: StateId) -> Result<bool> {
        Ok(self.reselect)
    }
    fn set_reselect(&mut self, _state: StateId, value: bool) -> Result<()> {
        self.reselect = value;
        Ok(())
    }
    fn organization(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok(self.c["membership"]["organization"]
            .as_bool()
            .unwrap_or(false)
            .then_some(ORGANIZATION))
    }
    fn regiment_engage_keeps(&mut self, organization: u32, target: u32) -> Result<bool> {
        assert_eq!(organization, ORGANIZATION);
        let r = self.c["keeps"].as_bool().unwrap();
        let t = self.name(Some(target));
        self.events
            .push(json!({"event": "regiment_engage_keeps", "target": t, "result": r}));
        Ok(r)
    }
    fn engage_select(
        &mut self,
        _state: StateId,
        target: Option<u32>,
        _arena: &mut Arena,
    ) -> Result<Option<u32>> {
        let r = self.c["select"].clone();
        let t = self.name(target);
        self.events
            .push(json!({"event": "engage_select", "target": t, "result": r}));
        Ok(r.as_str().map(|k| self.handle(k)))
    }
    fn kill_target(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["kill_target"]))
    }
    fn kill_select(
        &mut self,
        _mind: MindId,
        mode: u32,
        object: u32,
        _arena: &mut Arena,
    ) -> Result<()> {
        let o = self.name(Some(object));
        self.events
            .push(json!({"event": "kill_select", "mode": mode, "object": o}));
        Ok(())
    }
    fn return_to_slot(&mut self, _state: StateId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "return_to_slot"}));
        Ok(())
    }
    fn world_random(&mut self) -> Result<u32> {
        let wide = u64::from(self.rng) * 0x41c6_4e6d + 0x3039;
        self.rng = wide as u32;
        Ok((wide >> 16) as u32)
    }
    fn engage_reaction_range(&mut self) -> Result<u32> {
        Ok(u(&self.c["range"]))
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
fn native_uai_engage_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 600);
    assert_eq!(f["coverage"]["536E10"], json!([99, 99]));
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        assert!(o["fault"].is_null(), "{name}");
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut arena = arena_for(c);
        let mut host = Host {
            c: c.clone(),
            keys: c["objects"].as_object().unwrap().keys().cloned().collect(),
            events: vec![],
            reselect: c["reselect"].as_bool().unwrap(),
            rng: u(&c["rng"]),
        };
        think(state_id("engage"), &mut arena, &mut host, p).unwrap();
        assert_eq!(json!(host.events), o["events"], "{name}");
        let (status, stack) = snapshot(&arena);
        assert_eq!(status, o["status"], "{name}");
        assert_eq!(stack, o["stack"], "{name}");
        assert_eq!(json!(u8::from(host.reselect)), o["reselect"], "{name}");
        assert_eq!(json!(host.rng), o["rng"], "{name}");
    }
}
