//! Replays research/phase8-ai/native-cai-unpack.json (original 526E60
//! KKC_CAIStateUnpack::Think with 51E800, 4D9E20/4CA2D0 and 52A810 mode 0
//! under Unicorn) against aa_mind::cai_thinks::unpack::think.
use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId, World,
    cai_thinks::{
        RegimentWorld,
        stack_action::{ActionCommand, ActionWorld},
        unit_order::UnitOrderWorld,
        unpack::{UnpackThinkWorld, think},
    },
};
use serde_json::{Value, json};

const STATES: [(&str, u32); 4] = [
    ("unpack", 0x280),
    ("sleep", 0x34),
    ("move", 0x68),
    ("other", 0x700),
];
const UNIT: u32 = 1;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-unpack.json"
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
    issue: Vec<bool>,
    attempts: u32,
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
        unreachable!("mode 0 never asks")
    }
    fn enter(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        self.event("enter", json!({"state": state_name(state)}), arena);
        Ok(())
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
impl ActionWorld for Host {
    fn action_definition(&mut self, ordinal: u32) -> Result<u32> {
        Ok(u(&self.c["definitions"][ordinal.to_string()]))
    }
    fn issue_action(
        &mut self,
        unit: u32,
        action: &ActionCommand,
        arena: &mut Arena,
    ) -> Result<bool> {
        assert_eq!(unit, UNIT);
        let r = self.issue.remove(0);
        self.event(
            "issue_action",
            json!({"unit": "unit", "vtable": 0xa3_cc38, "definition": action.definition,
                   "flags": action.flags, "word10": action.word10, "words": action.words18_28,
                   "byte30": action.byte30, "result": r}),
            arena,
        );
        Ok(r)
    }
    fn release_action(&mut self, action: &ActionCommand, arena: &mut Arena) -> Result<()> {
        self.event(
            "release_action",
            json!({"definition": action.definition, "flags": action.flags}),
            arena,
        );
        Ok(())
    }
}
impl UnitOrderWorld for Host {
    fn component_unit(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok((!self.c["unit"].is_null()).then_some(UNIT))
    }
    fn unit_4d1080(&mut self, unit: u32, arena: &mut Arena) -> Result<bool> {
        assert_eq!(unit, UNIT);
        let r = self.c["unit"]["check_4d1080"].as_bool().unwrap();
        self.event("unit_4d1080", json!({"unit": "unit", "result": r}), arena);
        Ok(r)
    }
    fn organization_name(&mut self, _mind: MindId, arena: &mut Arena) -> Result<String> {
        self.event("organization_name", json!({}), arena);
        Ok("organization name".into())
    }
    fn display_string(&mut self, key: &str, arena: &mut Arena) -> Result<String> {
        self.event("display_string", json!({"key": key}), arena);
        Ok(format!("display:{key}"))
    }
    fn organization_owner(&mut self, _mind: MindId, arena: &mut Arena) -> Result<u32> {
        let r = u(&self.c["owner"]);
        self.event("organization_owner", json!({"result": r}), arena);
        Ok(r)
    }
    fn post_message(
        &mut self,
        _mind: MindId,
        kind: u32,
        owner: u32,
        format: &str,
        argument: &str,
        arena: &mut Arena,
    ) -> Result<()> {
        self.event(
            "post_message",
            json!({"kind": kind, "owner": owner, "word5": 0, "format": format,
                   "argument": argument}),
            arena,
        );
        Ok(())
    }
}
impl UnpackThinkWorld for Host {
    fn unit_mind(&mut self, _unit: u32) -> Result<Option<u32>> {
        Ok(self.c["unit"]["mind"].as_bool().unwrap().then_some(2))
    }
    fn unit_mind_virtual_7c(&mut self, unit_mind: u32, arena: &mut Arena) -> Result<bool> {
        assert_eq!(unit_mind, 2);
        let r = self.c["unit"]["mind_7c"].as_bool().unwrap();
        self.event("unit_mind_virtual_7c", json!({"result": r}), arena);
        Ok(r)
    }
    fn unit_4d1050(&mut self, unit: u32, arena: &mut Arena) -> Result<bool> {
        assert_eq!(unit, UNIT);
        let r = self.c["unit"]["check_4d1050"].as_bool().unwrap();
        self.event("unit_4d1050", json!({"unit": "unit", "result": r}), arena);
        Ok(r)
    }
    fn unpack_points(&mut self, _state: StateId) -> Result<[u32; 4]> {
        let p: Vec<u32> = self.c["points"].as_array().unwrap().iter().map(u).collect();
        Ok([p[0], p[1], p[2], p[3]])
    }
    fn unpack_attempts(&mut self, _state: StateId) -> Result<u32> {
        Ok(self.attempts)
    }
    fn set_unpack_attempts(&mut self, _state: StateId, value: u32) -> Result<()> {
        self.attempts = value;
        Ok(())
    }
    fn mind_virtual_1b0(&mut self, _mind: MindId, flag: u32, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_1b0", json!({"args": [flag]}), arena);
        Ok(())
    }
}

#[test]
fn native_unpack_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 240);
    for (_, cov) in f["coverage"].as_object().unwrap() {
        assert_eq!(cov["visited"], cov["instructions"]);
    }
    let mut faults = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
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
            issue: c["issue"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| b.as_bool().unwrap())
                .collect(),
            attempts: u(&c["attempts"]),
        };
        let status_in = u(&c["states"]["unpack"]);
        let result = think(id("unpack"), &mut arena, &mut host);
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
        if !o["fault"].is_null() {
            assert!(matches!(result, Err(Error::World(_))), "{name}");
            faults += 1;
            continue;
        }
        let wrong = result.unwrap();
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
        assert_eq!(json!(host.attempts), o["attempts"], "{name}");
    }
    assert_eq!(faults, 7);
}
