//! Replays research/phase8-ai/native-cai-repair-accept.json (original 51ECF0
//! repair-state accept with 529C00, 52A000, 44DD70, 4C57F0, 4D0940, 579F70,
//! 4CF1B0 and the 52A810 activation under Unicorn) against
//! aa_mind::cai_thinks::repair_accept::accept.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    cai_thinks::{
        repair::ITERATION_END,
        repair_accept::{RepairAcceptWorld, accept},
    },
};
use serde_json::{Value, json};

const STATES: [(&str, u32); 4] = [
    ("repair", 0x160),
    ("move", 0x68),
    ("sleep", 0x34),
    ("other", 0x700),
];
const TARGET: u32 = 1;
const COMPONENT: u32 = 5000;
const MEMBER_BASE: u32 = 1000;
const OWNER: u32 = 7000;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-repair-accept.json"
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
    target: u32,
    byte18: u8,
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
    fn object(&self) -> &Value {
        &self.c["object"]
    }
    fn members(&self) -> Vec<Value> {
        self.object()["members"].as_array().unwrap().clone()
    }
}
impl World for Host {
    fn can_activate(&mut self, state: StateId, arena: &mut Arena) -> Result<bool> {
        let s = state_name(state);
        let r = self.c["can_activate"][s].as_bool().unwrap();
        self.event("can_activate", json!({"state": s, "result": r}), arena);
        Ok(r)
    }
    fn enter(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        self.event("enter", json!({"state": state_name(state)}), arena);
        Ok(())
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
impl RepairAcceptWorld for Host {
    fn object_flags8(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, TARGET);
        Ok(u(&self.object()["flags8"]))
    }
    fn organization_virtual_34(
        &mut self,
        _mind: MindId,
        value: u32,
        arena: &mut Arena,
    ) -> Result<bool> {
        let r = self.c["org_34"].as_bool().unwrap();
        self.event(
            "organization_virtual_34",
            json!({"value": value, "result": r}),
            arena,
        );
        Ok(r)
    }
    fn repair_target(&mut self, _state: StateId) -> Result<u32> {
        Ok(self.target)
    }
    fn repair_byte18(&mut self, _state: StateId) -> Result<u8> {
        Ok(self.byte18)
    }
    fn set_repair_order(&mut self, _state: StateId, gtid: u32, byte18: u8) -> Result<()> {
        self.target = gtid;
        self.byte18 = byte18;
        Ok(())
    }
    fn object_gtid(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, TARGET);
        Ok(u(&self.object()["gtid"]))
    }
    fn mind_word_c(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["mind_c"]))
    }
    fn object_component(&mut self, object: u32) -> Result<Option<u32>> {
        assert_eq!(object, TARGET);
        Ok((!self.object()["members"].is_null()).then_some(COMPONENT))
    }
    fn object_word60(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, TARGET);
        Ok(u(&self.object()["word60"]))
    }
    fn object_word100(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, TARGET);
        Ok(u(&self.object()["word100"]))
    }
    fn organization_owner(&mut self, _mind: MindId, arena: &mut Arena) -> Result<u32> {
        self.event("organization_owner", json!({}), arena);
        Ok(OWNER)
    }
    fn owner_word1f0(&mut self, owner: u32) -> Result<u32> {
        assert_eq!(owner, OWNER);
        Ok(u(&self.c["owner_1f0"]))
    }
    fn component_next_index(&mut self, component: u32, index: u32, flagged: bool) -> Result<u32> {
        assert_eq!(component, COMPONENT);
        assert!(!flagged);
        let list = self.members();
        let mut i = index.wrapping_add(1) as usize;
        while i < list.len() {
            if !list[i].is_null() {
                return Ok(i as u32);
            }
            i += 1;
        }
        Ok(ITERATION_END)
    }
    fn component_member(&mut self, component: u32, index: u32) -> Result<u32> {
        assert_eq!(component, COMPONENT);
        Ok(if self.members()[index as usize].is_null() {
            0
        } else {
            MEMBER_BASE + index
        })
    }
    fn member_4d1490(&mut self, member: u32, arena: &mut Arena) -> Result<bool> {
        let index = member - MEMBER_BASE;
        let r = self.members()[index as usize]["check_4d1490"]
            .as_bool()
            .unwrap();
        self.event(
            "member_4d1490",
            json!({"object": format!("m{index}"), "result": r}),
            arena,
        );
        Ok(r)
    }
}

#[test]
fn native_repair_accept_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 800);
    for key in ["accept_51ecf0", "ready_member_579f70", "upkeep_44dd70"] {
        assert_eq!(f["coverage"][key]["unvisited"], json!([]), "{key}");
    }
    let mut accepted = 0;
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
            target: u(&c["state_target"]),
            byte18: u(&c["state_byte18"]) as u8,
        };
        let object = (!c["object"].is_null()).then_some(TARGET);
        let r = accept(
            id("repair"),
            u(&c["mode"]),
            object,
            u(&c["keep"]) as u8,
            &mut arena,
            &mut host,
        )
        .unwrap();
        accepted += r as usize;
        assert_eq!(json!(r as u8), o["result"], "{name}");
        let native: Vec<Value> = o["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["event"] != "diagnostic_log")
            .cloned()
            .collect();
        assert_eq!(host.events, native, "{name}");
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
        assert_eq!(json!(host.target), o["state_target"], "{name}");
        assert_eq!(json!(host.byte18), o["state_byte18"], "{name}");
    }
    assert_eq!(accepted, 216);
}
