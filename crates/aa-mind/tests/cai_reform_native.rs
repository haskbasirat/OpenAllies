//! Replays research/phase8-ai/native-cai-reform.json (original 51DE70
//! KKC_CAIStateReform::Think under Unicorn at PC24/PC53/PC64) against
//! aa_mind::cai_thinks::reform::think.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    cai_thinks::{
        RegimentWorld,
        reform::{ReformThinkWorld, think},
    },
};
use aa_numeric::Precision;
use serde_json::{Value, json};

const STATES: [(&str, u32); 4] = [
    ("reform", 0x1b8),
    ("sleep", 0x34),
    ("move", 0x68),
    ("other", 0x700),
];

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-reform.json"
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
    point: [u32; 2],
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
    fn leave(&mut self, _state: StateId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn schedule(&mut self, _mind: MindId, delay: u32, data: u32, arena: &mut Arena) -> Result<()> {
        self.event("schedule", json!({"delay": delay, "data": data}), arena);
        Ok(())
    }
    fn reset_default(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn state_changed(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
}
impl RegimentWorld for Host {
    fn resolve(&mut self, _gtid: u32) -> Result<Option<u32>> {
        unreachable!()
    }
    fn object_gtid(&mut self, _object: u32) -> Result<u32> {
        unreachable!()
    }
    fn embedded_state(&mut self, _mind: MindId, _offset: u32) -> Result<StateId> {
        unreachable!()
    }
    fn embedded_think(&mut self, _mind: MindId, _offset: u32, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn mind_virtual_140(&mut self, _mind: MindId, _gtid: u32, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn mind_virtual_b8(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
}
impl ReformThinkWorld for Host {
    fn game_time(&mut self) -> Result<u32> {
        Ok(u(&self.c["time"]))
    }
    fn reform_start(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["start"]))
    }
    fn reform_timeout(&mut self) -> Result<u32> {
        Ok(u(&self.c["timeout"]))
    }
    fn mind_virtual_178(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_178", json!({"args": []}), arena);
        Ok(())
    }
    fn sleep_point(&mut self, _mind: MindId) -> Result<[u32; 2]> {
        Ok(self.point)
    }
    fn set_sleep_point(&mut self, _mind: MindId, point: [u32; 2]) -> Result<()> {
        self.point = point;
        Ok(())
    }
    fn null_point(&mut self) -> Result<[u32; 2]> {
        Ok([u(&self.c["null_point"][0]), u(&self.c["null_point"][1])])
    }
    fn organization_move(
        &mut self,
        _mind: MindId,
        point: [u32; 2],
        arena: &mut Arena,
    ) -> Result<()> {
        self.event("organization_move", json!({"point": point}), arena);
        Ok(())
    }
    fn component_word40(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["word40"]))
    }
    fn check_4cede0(&mut self, object: u32, arena: &mut Arena) -> Result<bool> {
        let r = self.c["check_4cede0"].as_bool().unwrap();
        self.event(
            "check_4cede0",
            json!({"object": object, "result": r}),
            arena,
        );
        Ok(r)
    }
    fn mind_virtual_11c(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_11c", json!({"args": []}), arena);
        Ok(())
    }
}

#[test]
fn native_reform_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 360);
    let cov = &f["coverage"]["think_51de70"];
    assert_eq!(cov["visited"], cov["instructions"]);
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        assert!(o["fault"].is_null(), "{name}");
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut arena = Arena::default();
        let mut mind = Mind::base(1);
        mind.stack = c["stack"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().map(id))
            .collect();
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
            point: [u(&c["point"][0]), u(&c["point"][1])],
        };
        let status_in = u(&c["states"]["reform"]);
        let wrong = think(id("reform"), &mut arena, &mut host, p).unwrap();
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
        assert_eq!(json!(host.point), o["point"], "{name}");
    }
}
