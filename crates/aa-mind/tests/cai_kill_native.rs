//! Replays research/phase8-ai/native-cai-kill.json (original 520730
//! KKC_CAIStateKill::Think with 51DA20, 51D130, 529C00 and 52A810 mode 1
//! under Unicorn) against aa_mind::cai_thinks::kill::think.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    cai_thinks::{
        RegimentWorld,
        kill::{KillThinkWorld, think},
        siege_target::SiegeTargetWorld,
    },
};
use serde_json::{Value, json};
use std::collections::VecDeque;

const STATES: [(&str, u32); 5] = [
    ("kill", 0xc0),
    ("siege", 0xf8),
    ("move", 0x68),
    ("sleep", 0x34),
    ("other", 0x700),
];

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-kill.json"
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

/// Object handles are index+1 in key order.
struct Host {
    c: Value,
    keys: Vec<String>,
    events: Vec<Value>,
    accepts: VecDeque<bool>,
    siege_words: [u32; 2],
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
        unreachable!("mode 1 never asks")
    }
    fn enter(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        self.event("enter", json!({"state": state_name(state)}), arena);
        Ok(())
    }
    fn leave(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        self.event("leave", json!({"state": state_name(state)}), arena);
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
impl RegimentWorld for Host {
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
    fn embedded_state(&mut self, _mind: MindId, offset: u32) -> Result<StateId> {
        Ok(StateId(
            STATES.iter().position(|(_, o)| *o == offset).unwrap() as u32 + 1,
        ))
    }
    fn embedded_think(&mut self, _mind: MindId, offset: u32, arena: &mut Arena) -> Result<()> {
        let name = STATES.iter().find(|(_, o)| *o == offset).unwrap().0;
        self.event("embedded_think", json!({"state": name}), arena);
        Ok(())
    }
    fn mind_virtual_140(&mut self, _mind: MindId, gtid: u32, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_140", json!({"args": [gtid]}), arena);
        Ok(())
    }
    fn mind_virtual_b8(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
}
impl SiegeTargetWorld for Host {
    fn object_word90(&mut self, object: u32) -> Result<u32> {
        Ok(u(&self.object(object)["word90"]))
    }
    fn object_4d11d0(&mut self, object: u32, arena: &mut Arena) -> Result<Option<u32>> {
        let site = self.object(object)["site"].clone();
        let k = self.key(object);
        let result = if site.is_null() {
            json!(0)
        } else {
            site.clone()
        };
        self.event(
            "object_4d11d0",
            json!({"object": k, "result": result}),
            arena,
        );
        Ok(site.as_str().map(|s| self.handle(s)))
    }
    fn organization_virtual_16c(
        &mut self,
        _mind: MindId,
        site: u32,
        arena: &mut Arena,
    ) -> Result<bool> {
        let r = self.c["org_16c"].as_bool().unwrap();
        let k = self.key(site);
        self.event(
            "organization_virtual_16c",
            json!({"site": k, "result": r}),
            arena,
        );
        Ok(r)
    }
    fn component_57a500(
        &mut self,
        _mind: MindId,
        object: u32,
        a: bool,
        b: bool,
        arena: &mut Arena,
    ) -> Result<bool> {
        let r = self.accepts.pop_front().unwrap();
        let k = self.key(object);
        self.event(
            "component_57a500",
            json!({"object": k, "flags": [u32::from(a), u32::from(b)], "result": r}),
            arena,
        );
        Ok(r)
    }
    fn set_siege_words(&mut self, _state: StateId, target: u32, site: u32) -> Result<()> {
        self.siege_words = [target, site];
        Ok(())
    }
}
impl KillThinkWorld for Host {
    fn kill_target(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["target"]))
    }
}
#[test]
fn native_kill_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 400);
    let cov = &f["coverage"]["think_520730"];
    assert_eq!(cov["visited"], cov["instructions"]);
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        assert!(o["fault"].is_null(), "{name}");
        let mut arena = Arena::default();
        let mut mind = Mind::base(1);
        mind.stack = c["stack"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().map(id))
            .collect();
        let w: Vec<u32> = c["words"].as_array().unwrap().iter().map(u).collect();
        mind.words_24_30 = [w[0], w[1], w[2], w[3]];
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
            keys: c["objects"].as_object().unwrap().keys().cloned().collect(),
            events: vec![],
            accepts: c["accepts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| b.as_bool().unwrap())
                .collect(),
            siege_words: [u(&c["siege_words"][0]), u(&c["siege_words"][1])],
        };
        let status_in = u(&c["states"]["kill"]);
        let wrong = think(id("kill"), &mut arena, &mut host).unwrap();
        let native: Vec<Value> = o["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["event"] != "diagnostic_log")
            .cloned()
            .collect();
        let logged = o["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["event"] == "diagnostic_log");
        assert_eq!(host.events, native, "{name}");
        assert_eq!(wrong, logged, "{name}");
        assert_eq!(wrong, status_in != 2, "{name}");
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
        assert_eq!(json!(host.siege_words), o["siege_words"], "{name}");
    }
}
