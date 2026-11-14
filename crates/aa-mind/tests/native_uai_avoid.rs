//! Replays research/phase8-ai/native-uai-avoid.json (original 5358A0,
//! 5351C0 and 533D90 with 529C00, 529C20, 529C30 and 52A810 under Unicorn,
//! at PC24/PC53/PC64) against aa_mind::uai_thinks::uai_avoid.
use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId, World,
    uai_thinks::{
        ThinkWorld,
        uai_avoid::{KillSelectWorld, hand_to_kill, kill_select, set_kill_target},
    },
};
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
const ACTOR: u32 = 100;
const ACTOR_GTID: u32 = 0x0040_0040;
const LOCOMOTION: u32 = 200;
const PERFORMER: u32 = 300;
/// The other point actor virtual +108 may return (binary32 7.5, -2.25).
const OTHER: [u32; 2] = [0x40f0_0000, 0xc010_0000];

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-uai-avoid.json"
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

/// Objects are handles index+1 in key order; the actor is [`ACTOR`].
struct Host {
    c: Value,
    keys: Vec<String>,
    events: Vec<Value>,
    kill_target: u32,
    scratch: [u32; 2],
}
impl Host {
    fn name(&self, h: u32) -> Value {
        if h == ACTOR {
            json!("actor")
        } else {
            json!(self.keys[h as usize - 1])
        }
    }
    fn handle(&self, key: &str) -> u32 {
        if key == "actor" {
            ACTOR
        } else {
            self.keys.iter().position(|k| k == key).unwrap() as u32 + 1
        }
    }
}
impl World for Host {
    fn can_activate(&mut self, state: StateId, _arena: &mut Arena) -> Result<bool> {
        let r = self.c["can_activate"].as_bool().unwrap();
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
    fn embedded_think(&mut self, _mind: MindId, _offset: u32, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        if gtid == ACTOR_GTID && self.c["actor_registered"].as_bool().unwrap() {
            return Ok(Some(ACTOR));
        }
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
        if object == ACTOR {
            return Ok(ACTOR_GTID);
        }
        Ok(u(
            &self.c["objects"][&self.keys[object as usize - 1]]["gtid"],
        ))
    }
    fn null_point(&mut self) -> Result<[u32; 2]> {
        let p = &self.c["null_point"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn mind_virtual_140(&mut self, _mind: MindId, _gtid: u32, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn current_performer(&mut self, _mind: MindId) -> Result<Option<u32>> {
        unreachable!()
    }
    fn locomotion(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok((!self.c["locomotion"].is_null()).then_some(LOCOMOTION))
    }
    fn locomotion_follow(&mut self, _l: u32, _object: u32, _delay_bits: u32) -> Result<()> {
        unreachable!()
    }
}
impl KillSelectWorld for Host {
    fn kill_state_target(&mut self, _mind: MindId) -> Result<u32> {
        Ok(self.kill_target)
    }
    fn set_kill_state_target(&mut self, _mind: MindId, gtid: u32) -> Result<()> {
        self.kill_target = gtid;
        Ok(())
    }
    fn locomotion_byte56(&mut self, locomotion: u32) -> Result<bool> {
        assert_eq!(locomotion, LOCOMOTION);
        Ok(u(&self.c["locomotion"]) != 0)
    }
    fn actor_relation(&mut self, _mind: MindId, object: u32, a: u32, b: u32) -> Result<i32> {
        let r = self.c["relation"].as_i64().unwrap() as i32;
        let o = self.name(object);
        self.events.push(
            json!({"event": "relation", "object": o, "a": a, "b": b, "result": r}),
        );
        Ok(r)
    }
    fn target_released(&mut self, target: u32, _mind: MindId) -> Result<()> {
        let o = self.name(target);
        self.events.push(json!({"event": "released", "object": o}));
        Ok(())
    }
    fn actor_performer(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok(self.c["performer"].as_bool().unwrap().then_some(PERFORMER))
    }
    fn performer_halt(&mut self, performer: u32) -> Result<()> {
        assert_eq!(performer, PERFORMER);
        self.events.push(json!({"event": "halt"}));
        Ok(())
    }
    fn kill_move_to_object(
        &mut self,
        _mind: MindId,
        mode: u32,
        object: u32,
        a3: u32,
        a4: u32,
        point: [u32; 2],
        _arena: &mut Arena,
    ) -> Result<()> {
        let o = self.name(object);
        self.events.push(json!({"event": "move_to_object", "mode": mode, "object": o,
            "a3": a3, "a4": a4, "point": point}));
        Ok(())
    }
    fn object_is_actor(&mut self, _mind: MindId, object: u32) -> Result<bool> {
        Ok(object == ACTOR)
    }
    fn target_acquired(&mut self, object: u32, _mind: MindId) -> Result<()> {
        let o = self.name(object);
        self.events.push(json!({"event": "acquired", "object": o}));
        Ok(())
    }
    fn kill_in_reach(&mut self, _mind: MindId, object: u32) -> Result<bool> {
        self.scratch = [0, 0];
        self.events
            .push(json!({"event": "position_ref", "before": self.scratch}));
        let s = &self.c["scratch"];
        if !s["write"].is_null() {
            self.scratch = [u(&s["write"][0]), u(&s["write"][1])];
        }
        let point = if s["other"].as_bool().unwrap() {
            OTHER
        } else {
            self.scratch
        };
        let r = self.c["in_reach"].as_bool().unwrap();
        let o = self.name(object);
        self.events.push(
            json!({"event": "in_reach", "object": o, "point": point, "result": r}),
        );
        Ok(r)
    }
    fn kill_approach(&mut self, _mind: MindId, object: u32) -> Result<()> {
        if !self.c["performer"].as_bool().unwrap() {
            return Err(Error::World("57DD90 on a null performer".into()));
        }
        let o = self.name(object);
        self.events.push(
            json!({"event": "approach", "mode": 1, "object": o, "point": self.scratch}),
        );
        Ok(())
    }
}

fn arena_for(c: &Value) -> Arena {
    let mut arena = Arena::default();
    let mut mind = Mind::base(ACTOR);
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
fn native_uai_avoid_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 900);
    assert_eq!(f["coverage"]["5358A0"], json!([36, 36]));
    assert_eq!(f["coverage"]["5351C0"], json!([54, 54]));
    // The one instruction left is a dead re-resolve failure in 533D90.
    assert_eq!(f["coverage"]["533D90"], json!([113, 114]));
    let mut faults = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let mut arena = arena_for(c);
        let mut host = Host {
            c: c.clone(),
            keys: c["objects"].as_object().unwrap().keys().cloned().collect(),
            events: vec![],
            kill_target: u(&c["kill_target"]),
            scratch: [0, 0],
        };
        let object = c["object"].as_str().map(|k| host.handle(k));
        let mind = MindId(1);
        let result = match c["entry"].as_str().unwrap() {
            "5358A0" => hand_to_kill(mind, object, &mut arena, &mut host).map(Some),
            "5351C0" => kill_select(mind, u(&c["mode"]), object, &mut arena, &mut host).map(Some),
            _ => set_kill_target(mind, object, &mut arena, &mut host).map(|()| None),
        };
        assert_eq!(json!(host.events), o["events"], "{name}");
        if !o["fault"].is_null() {
            assert!(matches!(result, Err(Error::World(_))), "{name}");
            faults += 1;
            continue;
        }
        if let Some(r) = result.unwrap() {
            assert_eq!(json!(u8::from(r)), o["result"], "{name}");
        }
        let (status, stack) = snapshot(&arena);
        assert_eq!(status, o["status"], "{name}");
        assert_eq!(stack, o["stack"], "{name}");
        assert_eq!(json!(host.kill_target), o["kill_target"], "{name}");
    }
    assert_eq!(faults, 15);
}
