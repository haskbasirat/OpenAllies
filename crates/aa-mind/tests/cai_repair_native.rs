//! Replays research/phase8-ai/native-cai-repair.json (original 5239A0
//! KKC_CAIStateRepair::Think with 51E870, 4CA760, 529C20 and the member
//! iterators under Unicorn at PC24/PC53/PC64) against
//! aa_mind::cai_thinks::repair::think.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    cai_thinks::{
        RegimentWorld,
        repair::{ITERATION_END, RepairThinkWorld, think},
        stack_action::{ActionCommand, ActionWorld},
    },
};
use aa_numeric::Precision;
use serde_json::{Value, json};

const STATES: [(&str, u32); 4] = [
    ("repair", 0x160),
    ("move", 0x68),
    ("sleep", 0x34),
    ("other", 0x700),
];
/// Objects are index + 1 in key order; organization members 1000 + index;
/// the target's component 5000 and the organization component 6000.
const MEMBER_BASE: u32 = 1000;
const TARGET_COMPONENT: u32 = 5000;
const OWN_COMPONENT: u32 = 6000;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-repair.json"
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
    keys: Vec<String>,
    events: Vec<Value>,
    issue: Vec<bool>,
    byte10: u8,
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
    fn name(&self, h: u32) -> String {
        if h >= MEMBER_BASE {
            format!("m{}", h - MEMBER_BASE)
        } else {
            self.keys[h as usize - 1].clone()
        }
    }
    fn handle(&self, key: &str) -> u32 {
        self.keys.iter().position(|k| k == key).unwrap() as u32 + 1
    }
    fn object(&self, h: u32) -> &Value {
        &self.c["objects"][&self.keys[h as usize - 1]]
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
        Ok(())
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
    fn mind_virtual_b8(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_b8", json!({"args": []}), arena);
        Ok(())
    }
}
impl ActionWorld for Host {
    fn action_definition(&mut self, ordinal: u32) -> Result<u32> {
        Ok(u(&self.c["definitions"][ordinal.to_string()]))
    }
    fn issue_action(
        &mut self,
        member: u32,
        action: &ActionCommand,
        arena: &mut Arena,
    ) -> Result<bool> {
        let r = self.issue.remove(0);
        let k = self.name(member);
        self.event(
            "issue_action",
            json!({"unit": k, "vtable": 0xa3_cc38, "definition": action.definition,
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
impl RepairThinkWorld for Host {
    fn repair_target(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["target_gtid"]))
    }
    fn repair_byte10(&mut self, _state: StateId) -> Result<u8> {
        Ok(self.byte10)
    }
    fn set_repair_byte10(&mut self, _state: StateId, value: u8) -> Result<()> {
        self.byte10 = value;
        Ok(())
    }
    fn repair_byte18(&mut self, _state: StateId) -> Result<u8> {
        Ok(u(&self.c["byte18"]) as u8)
    }
    fn object_component(&mut self, object: u32) -> Result<Option<u32>> {
        assert_eq!(self.name(object), "target");
        Ok((!self.c["target_members"].is_null()).then_some(TARGET_COMPONENT))
    }
    fn organization_component(&mut self, _mind: MindId) -> Result<u32> {
        Ok(OWN_COMPONENT)
    }
    fn component_next_index(&mut self, component: u32, index: u32, flagged: bool) -> Result<u32> {
        let list = if component == TARGET_COMPONENT {
            self.c["target_members"].as_array().unwrap().clone()
        } else {
            self.c["members"].as_array().unwrap().clone()
        };
        let mut i = index.wrapping_add(1) as i32;
        while i < list.len() as i32 {
            let m = &list[i as usize];
            if !m.is_null() && (!flagged || u(&m["skip"]) != 0) {
                return Ok(i as u32);
            }
            i += 1;
        }
        Ok(ITERATION_END)
    }
    fn component_member(&mut self, component: u32, index: u32) -> Result<u32> {
        if component == TARGET_COMPONENT {
            let k = self.c["target_members"][index as usize].clone();
            return Ok(k.as_str().map_or(0, |k| self.handle(k)));
        }
        Ok(if self.c["members"][index as usize].is_null() {
            0
        } else {
            MEMBER_BASE + index
        })
    }
    fn member_4d1490(&mut self, member: u32, arena: &mut Arena) -> Result<bool> {
        let r = self.object(member)["check_4d1490"].as_bool().unwrap();
        let k = self.name(member);
        self.event("member_4d1490", json!({"object": k, "result": r}), arena);
        Ok(r)
    }
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]> {
        let p = &self.object(object)["position"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn organization_position(&mut self, _mind: MindId) -> Result<[u32; 2]> {
        let p = &self.c["org_position"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn object_word60(&mut self, object: u32) -> Result<u32> {
        Ok(u(&self.object(object)["word60"]))
    }
    fn object_flags8(&mut self, object: u32) -> Result<u32> {
        Ok(u(&self.object(object)["flags8"]))
    }
    fn object_word100(&mut self, object: u32) -> Result<u32> {
        Ok(u(&self.object(object)["word100"]))
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
    fn object_4d0940(&mut self, object: u32, arena: &mut Arena) -> Result<bool> {
        let r = self.object(object)["check_4d0940"].as_bool().unwrap();
        let k = self.name(object);
        self.event("object_4d0940", json!({"object": k, "result": r}), arena);
        Ok(r)
    }
    fn organization_owner(&mut self, _mind: MindId, arena: &mut Arena) -> Result<u32> {
        self.event(
            "organization_owner",
            json!({"result": 0x0eee_0000_u32}),
            arena,
        );
        Ok(0x0eee_0000)
    }
    fn owner_44dd70(&mut self, owner: u32, arena: &mut Arena) -> Result<bool> {
        assert_eq!(owner, 0x0eee_0000);
        let r = self.c["owner_44dd70"].as_bool().unwrap();
        self.event("owner_44dd70", json!({"result": r}), arena);
        Ok(r)
    }
    fn target_in_range(&mut self, _mind: MindId, object: u32, arena: &mut Arena) -> Result<bool> {
        let r = self.c["in_range"].as_bool().unwrap();
        let k = self.name(object);
        self.event("target_in_range", json!({"object": k, "result": r}), arena);
        Ok(r)
    }
    fn null_point(&mut self) -> Result<[u32; 2]> {
        Ok([u(&self.c["null_point"][0]), u(&self.c["null_point"][1])])
    }
    fn move_order(&mut self, _mind: MindId, args: [u32; 5], arena: &mut Arena) -> Result<()> {
        let k = self.name(args[1]);
        self.event(
            "move_order",
            json!({"args": [args[0], k, args[2], args[3], args[4]]}),
            arena,
        );
        Ok(())
    }
    fn check_5617c0(&mut self, word60: u32, arena: &mut Arena) -> Result<bool> {
        let key = self
            .keys
            .iter()
            .find(|k| u(&self.c["objects"][*k]["word60"]) == word60)
            .unwrap()
            .clone();
        let r = self.c["objects"][&key]["check_5617c0"].as_bool().unwrap();
        self.event("check_5617c0", json!({"object": key, "result": r}), arena);
        Ok(r)
    }
    fn mind_virtual_168(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_168", json!({"args": []}), arena);
        Ok(())
    }
    fn member_virtual_38(
        &mut self,
        member: u32,
        definition: u32,
        arena: &mut Arena,
    ) -> Result<bool> {
        let index = (member - MEMBER_BASE) as usize;
        let r = self.c["members"][index]["accepts"].as_bool().unwrap();
        let k = self.name(member);
        self.event(
            "member_virtual_38",
            json!({"member": k, "definition": definition, "result": r}),
            arena,
        );
        Ok(r)
    }
    fn sleep_point(&mut self, _mind: MindId) -> Result<[u32; 2]> {
        Ok([u(&self.c["sleep_point"][0]), u(&self.c["sleep_point"][1])])
    }
    fn formation_origin(&mut self, component: u32) -> Result<[u32; 2]> {
        assert_eq!(component, OWN_COMPONENT);
        Ok([u(&self.c["origin"][0]), u(&self.c["origin"][1])])
    }
    fn formation_slot(&mut self, component: u32, index: u32) -> Result<[u32; 2]> {
        assert_eq!(component, OWN_COMPONENT);
        let s = &self.c["slots"][index as usize];
        Ok([u(&s[0]), u(&s[1])])
    }
}

#[test]
fn native_repair_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 700);
    assert_eq!(
        f["coverage"]["repair_action_51e870"]["unvisited"],
        json!([])
    );
    assert_eq!(
        f["coverage"]["think_5239a0"]["unvisited"],
        json!(["0x523a16"])
    );
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
            keys: c["objects"].as_object().unwrap().keys().cloned().collect(),
            events: vec![],
            issue: c["issue"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| b.as_bool().unwrap())
                .collect(),
            byte10: u(&c["byte10"]) as u8,
        };
        let status_in = u(&c["states"]["repair"]);
        let wrong = think(id("repair"), &mut arena, &mut host, p).unwrap();
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
