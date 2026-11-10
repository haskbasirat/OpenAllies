//! Replays research/phase8-ai/native-cai-home.json (original 52C280 Mind
//! home setter with the 52A3C0 stack clear under Unicorn) against
//! aa_mind::cai_home::set_home.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    cai_home::{HomeWorld, set_home},
};
use serde_json::{Value, json};

const STATES: [(&str, u32); 4] = [
    ("repair", 0x160),
    ("move", 0x68),
    ("sleep", 0x34),
    ("other", 0x700),
];
const ACTOR: u32 = 900;
const HOME: u32 = 1;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-home.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn id(name: &str) -> StateId {
    StateId(STATES.iter().position(|(n, _)| *n == name).unwrap() as u32 + 1)
}
fn state_name(s: StateId) -> &'static str {
    STATES[s.0 as usize - 1].0
}

struct Host {
    c: Value,
    events: Vec<Value>,
    flags8: u32,
}
impl Host {
    fn event(&mut self, event: &str, extra: Value, arena: &Arena) {
        let mut e = json!({"event": event});
        for (k, v) in extra.as_object().unwrap() {
            e[k] = v.clone();
        }
        let states: serde_json::Map<String, Value> = STATES
            .iter()
            .map(|(n, _)| (n.to_string(), json!(arena.states[&id(n)].status)))
            .collect();
        e["states"] = json!(states);
        e["stack"] = json!(
            arena.minds[&MindId(1)]
                .stack
                .iter()
                .map(|s| s.map_or(Value::Null, |s| json!(state_name(s))))
                .collect::<Vec<_>>()
        );
        self.events.push(e);
    }
}
impl World for Host {
    fn can_activate(&mut self, _state: StateId, _arena: &mut Arena) -> Result<bool> {
        unreachable!()
    }
    fn enter(&mut self, _state: StateId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn leave(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        self.event("leave", json!({"state": state_name(state)}), arena);
        if self.c["leave_clears"] == true {
            arena.state_mut(state)?.status = 0;
        }
        Ok(())
    }
    fn schedule(&mut self, _mind: MindId, delay: u32, data: u32, arena: &mut Arena) -> Result<()> {
        self.event("schedule", json!({"delay": delay, "data": data}), arena);
        Ok(())
    }
    fn reset_default(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("reset_default", json!({"args": []}), arena);
        Ok(())
    }
    fn state_changed(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("state_changed", json!({"args": []}), arena);
        Ok(())
    }
}
impl HomeWorld for Host {
    fn home_flags8(&mut self, home: u32) -> Result<u32> {
        assert_eq!(home, HOME);
        Ok(u(&self.c["home"]["flags8"]))
    }
    fn home_accepts(&mut self, home: u32, actor: u32, arena: &mut Arena) -> Result<bool> {
        assert_eq!((home, actor), (HOME, ACTOR));
        let r = self.c["home"]["accepts"].as_bool().unwrap();
        self.event(
            "home_accepts",
            json!({"actor": "organization", "result": r}),
            arena,
        );
        Ok(r)
    }
    fn home_gtid(&mut self, home: u32) -> Result<u32> {
        assert_eq!(home, HOME);
        Ok(u(&self.c["home"]["gtid"]))
    }
    fn actor_flags8(&mut self, actor: u32) -> Result<u32> {
        assert_eq!(actor, ACTOR);
        Ok(self.flags8)
    }
    fn set_actor_flags8(&mut self, actor: u32, value: u32) -> Result<()> {
        assert_eq!(actor, ACTOR);
        self.flags8 = value;
        Ok(())
    }
    fn release_pending(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("release_pending", json!({}), arena);
        Ok(())
    }
    fn actor_word_bc(&mut self, actor: u32) -> Result<u32> {
        assert_eq!(actor, ACTOR);
        Ok(u(&self.c["actor_bc"]))
    }
    fn sai_active(&mut self) -> Result<bool> {
        Ok(self.c["sai"].as_u64().is_some_and(|b| b != 0))
    }
    fn sai_notify(&mut self, actor: u32, arena: &mut Arena) -> Result<()> {
        assert_eq!(actor, ACTOR);
        self.event("sai_notify", json!({"actor": "organization"}), arena);
        Ok(())
    }
}

#[test]
fn native_set_home_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 500);
    for key in ["set_home_52c280", "clear_52a3c0"] {
        assert_eq!(f["coverage"][key]["unvisited"], json!([]), "{key}");
    }
    let mut notified = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let mut arena = Arena::default();
        let mut mind = Mind::base(ACTOR);
        mind.stack = c["stack"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().map(id))
            .collect();
        for (i, w) in c["words"].as_array().unwrap().iter().enumerate() {
            mind.words_24_30[i] = u(w);
        }
        arena.minds.insert(MindId(1), mind);
        for (n, _) in STATES {
            arena.states.insert(
                id(n),
                State {
                    parent: MindId(1),
                    discard_on_suspend: n == "sleep",
                    status: u(&c["states"][n]),
                },
            );
        }
        let mut host = Host {
            c: c.clone(),
            events: vec![],
            flags8: u(&c["actor_flags8"]),
        };
        let home = (!c["home"].is_null()).then_some(HOME);
        set_home(MindId(1), home, &mut arena, &mut host).unwrap();
        notified += host
            .events
            .iter()
            .any(|e| e["event"] == "sai_notify") as usize;
        assert_eq!(host.events, o["events"].as_array().unwrap().clone(), "{name}");
        let states: serde_json::Map<String, Value> = STATES
            .iter()
            .map(|(n, _)| (n.to_string(), json!(arena.states[&id(n)].status)))
            .collect();
        assert_eq!(json!(states), o["states"], "{name}");
        let stack: Vec<Value> = arena.minds[&MindId(1)]
            .stack
            .iter()
            .map(|s| s.map_or(Value::Null, |s| json!(state_name(s))))
            .collect();
        assert_eq!(json!(stack), o["stack"], "{name}");
        assert_eq!(
            json!(arena.minds[&MindId(1)].words_24_30[1]),
            o["home_word"],
            "{name}"
        );
        assert_eq!(json!(host.flags8), o["actor_flags8"], "{name}");
    }
    assert_eq!(notified, 86);
}
