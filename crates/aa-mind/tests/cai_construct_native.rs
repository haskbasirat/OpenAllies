//! Replays research/phase8-ai/native-cai-construct.json (original 51DB80
//! KKC_CAIStateConstruct::Think under Unicorn) against
//! aa_mind::cai_thinks::construct::think.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    cai_thinks::{
        RegimentWorld,
        construct::{ConstructThinkWorld, think},
    },
};
use serde_json::{Value, json};

const STATES: [(&str, u32); 4] = [
    ("construct", 0x134),
    ("move", 0x68),
    ("sleep", 0x34),
    ("other", 0x700),
];

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-construct.json"
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
    byte10: u32,
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
    fn schedule(&mut self, _m: MindId, _d: u32, _x: u32, _arena: &mut Arena) -> Result<()> {
        unreachable!()
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
    fn mind_virtual_140(&mut self, _mind: MindId, _gtid: u32, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn mind_virtual_b8(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
}
impl ConstructThinkWorld for Host {
    fn construct_point(&mut self, _state: StateId) -> Result<[u32; 2]> {
        Ok([u(&self.c["points"][0]), u(&self.c["points"][1])])
    }
    fn organization_count_14c(&mut self, _mind: MindId) -> Result<i32> {
        Ok(u(&self.c["count"]) as i32)
    }
    fn organization_entry_148(&mut self, _mind: MindId, index: u32) -> Result<u32> {
        Ok(u(&self.c["entries"][index as usize]))
    }
    fn map_4f2420(&mut self, entry: u32, point: [u32; 2], arena: &mut Arena) -> Result<bool> {
        let r = self.c["map_result"].as_bool().unwrap();
        self.event(
            "map_4f2420",
            json!({"entry": entry, "point": point, "result": r}),
            arena,
        );
        Ok(r)
    }
    fn set_construct_byte10(&mut self, _state: StateId, value: bool) -> Result<()> {
        self.byte10 = u32::from(value);
        Ok(())
    }
    fn construct_word14(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["word14"]))
    }
    fn construct_word28(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["word28"]))
    }
    fn component_57acc0(
        &mut self,
        _mind: MindId,
        word14: u32,
        _state: StateId,
        word28: u32,
        arena: &mut Arena,
    ) -> Result<u32> {
        let r = u(&self.c["construct_result"]);
        let points = self.c["points"].clone();
        self.event(
            "component_57acc0",
            json!({"word14": word14, "points": points, "word28": word28, "result": r}),
            arena,
        );
        Ok(r)
    }
    fn mind_virtual_170(&mut self, _mind: MindId, value: u32, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_170", json!({"args": [value]}), arena);
        Ok(())
    }
    fn mind_virtual_174(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_174", json!({"args": []}), arena);
        Ok(())
    }
}

#[test]
fn native_construct_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 200);
    let cov = &f["coverage"]["think_51db80"];
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
            byte10: u(&c["byte10"]),
        };
        let status_in = u(&c["states"]["construct"]);
        let wrong = think(id("construct"), &mut arena, &mut host).unwrap();
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
        assert_eq!(json!(host.byte10), o["byte10"], "{name}");
    }
}
