//! Replays research/phase8-ai/native-uai-bombard.json (original 5353E0 KKC_UAIStateBombard::Think with 4DB0F0, 432E40, 4CF1B0 and
//! the 4364C0 member iterator under Unicorn, at PC24/PC53/PC64) against
//! aa_mind::uai_thinks::uai_bombard.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    uai_kill::{KillDecision, PerformerState},
    uai_thinks::{
        ThinkWorld,
        uai_bombard::{BombardThinkWorld, think},
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
const ZONE: u32 = 500;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-uai-bombard.json"
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
    fn rec(&self, r: &KillDecision) -> Value {
        json!([
            self.name(r.ability),
            r.kind,
            r.status,
            r.score,
            self.name(r.target),
            r.distance2,
            r.priority
        ])
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
        Ok([u(&self.c["null_point"][0]), u(&self.c["null_point"][1])])
    }
    fn mind_virtual_140(&mut self, _mind: MindId, gtid: u32, _arena: &mut Arena) -> Result<()> {
        self.events
            .push(json!({"event": "mind_virtual_140", "args": [gtid]}));
        Ok(())
    }
    fn current_performer(&mut self, _mind: MindId) -> Result<Option<u32>> {
        unreachable!()
    }
    fn locomotion(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok(self.c["locomotion"].as_bool().unwrap().then_some(901))
    }
    fn locomotion_follow(&mut self, _l: u32, _object: u32, _delay_bits: u32) -> Result<()> {
        unreachable!()
    }
}
impl BombardThinkWorld for Host {
    fn bombard_target(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["target"]))
    }
    fn area_length(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["area_length"]))
    }
    fn bombard_point(&mut self, _state: StateId) -> Result<[u32; 2]> {
        Ok([u(&self.c["point"][0]), u(&self.c["point"][1])])
    }
    fn performer(&mut self, _mind: MindId) -> Result<Option<PerformerState>> {
        let p = &self.c["performer"];
        Ok((!p.is_null()).then(|| PerformerState {
            current: p["current"].as_bool().unwrap(),
            stage: u(&p["stage"]),
        }))
    }
    fn actor_owner(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<u32> {
        let r = u(&self.c["owner"]);
        self.events
            .push(json!({"event": "actor_owner", "result": r}));
        Ok(r)
    }
    fn target_virtual_5c(&mut self, target: u32, owner: u32, flag: u32) -> Result<bool> {
        let r = self.object(target)["v5c"].as_bool().unwrap();
        let t = self.name(target);
        self.events.push(
            json!({"event": "target_virtual_5c", "target": t, "owner": owner,
            "flag": flag, "result": r}),
        );
        Ok(r)
    }
    fn object_word6c(&mut self, object: u32) -> Result<u32> {
        Ok(u(&self.object(object)["word6c"]))
    }
    fn organization(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok(self.c["organization"].as_bool().unwrap().then_some(60))
    }
    fn actor_name(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<String> {
        self.events.push(json!({"event": "actor_name"}));
        Ok("actor name".into())
    }
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]> {
        let p = &self.object(object)["pos"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn report(&mut self, kind: u32, owner: u32, point: [u32; 2], name: &str) -> Result<()> {
        self.events
            .push(json!({"event": "report", "kind": kind, "owner": owner,
            "point": point, "name": name}));
        Ok(())
    }
    fn zone_at(&mut self, point: [u32; 2]) -> Result<u32> {
        self.events
            .push(json!({"event": "zone_at", "point": point}));
        Ok(ZONE)
    }
    fn zone_check(&mut self, zone: u32, owner: u32) -> Result<bool> {
        assert_eq!(zone, ZONE);
        let r = self.c["zone_check"].as_bool().unwrap();
        self.events
            .push(json!({"event": "zone_check", "owner": owner, "result": r}));
        Ok(r)
    }
    fn mind_virtual_144(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "mind_virtual_144"}));
        Ok(())
    }
    fn mind_virtual_124(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "mind_virtual_124"}));
        Ok(())
    }
    fn relation(&mut self, _mind: MindId, target: u32, a: u32, b: u32) -> Result<i32> {
        let r = self.c["relation"].as_i64().unwrap() as i32;
        let t = self.name(target);
        self.events
            .push(json!({"event": "relation", "target": t, "a": a, "b": b, "result": r}));
        Ok(r)
    }
    fn point_relation(&mut self, _mind: MindId, point: [u32; 2], flag: u32) -> Result<i32> {
        let r = self.c["point_relation"].as_i64().unwrap() as i32;
        self.events
            .push(json!({"event": "point_relation", "point": point, "flag": flag, "result": r}));
        Ok(r)
    }
    fn performer_approach(
        &mut self,
        _mind: MindId,
        mode: u32,
        object: Option<u32>,
        point: [u32; 2],
    ) -> Result<()> {
        let o = self.name(object.unwrap_or(0));
        self.events.push(
            json!({"event": "performer_approach", "mode": mode, "object": o,
            "point": point}),
        );
        Ok(())
    }
    fn move_activate(&mut self, _mind: MindId, r: Activation, _arena: &mut Arena) -> Result<bool> {
        self.events.push(json!({"event": "move_activate", "args": [
            r.activation_mode, r.destination_bits[0], r.destination_bits[1], r.movement_mode,
            r.requested_speed_bits, r.auxiliary_bits[0], r.auxiliary_bits[1]]}));
        Ok(true)
    }
    fn object_component(&mut self, object: u32) -> Result<Option<u32>> {
        Ok((!self.object(object)["members"].is_null()).then_some(1000 + object))
    }
    fn component_members(&mut self, component: u32) -> Result<Vec<Option<u32>>> {
        let members = self.object(component - 1000)["members"].clone();
        Ok(members
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m.as_str().map(|k| self.handle(k)))
            .collect())
    }
    fn actor_position(&mut self, _mind: MindId) -> Result<[u32; 2]> {
        Ok([u(&self.c["pos"][0]), u(&self.c["pos"][1])])
    }
    fn choose(
        &mut self,
        _mind: MindId,
        target: Option<u32>,
        point: Option<[u32; 2]>,
        record: &mut KillDecision,
    ) -> Result<u32> {
        let t = self.name(target.unwrap_or(0));
        let before = self.rec(record);
        self.events
            .push(json!({"event": "choose", "target": t, "point": point,
            "by_point": 1, "before": before}));
        let w = self.c["record"].clone();
        let word = |v: &Value| match v.as_str() {
            Some(k) => self.handle(k),
            None => u(v),
        };
        *record = KillDecision {
            ability: word(&w[0]),
            kind: word(&w[1]),
            status: word(&w[2]),
            score: word(&w[3]),
            target: word(&w[4]),
            distance2: word(&w[5]),
            priority: word(&w[6]) as i32,
        };
        Ok(record.status)
    }
    fn apply(
        &mut self,
        _mind: MindId,
        record: &mut KillDecision,
        point: Option<[u32; 2]>,
        _arena: &mut Arena,
    ) -> Result<()> {
        let r = self.rec(record);
        self.events
            .push(json!({"event": "apply", "record": r, "point": point}));
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
fn native_uai_bombard_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 909);
    assert_eq!(f["coverage"]["5353E0"], json!([402, 402]));
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
        };
        let result = think(state_id("bombard"), &mut arena, &mut host, p);
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
    }
    assert_eq!(faults, 62);
}
