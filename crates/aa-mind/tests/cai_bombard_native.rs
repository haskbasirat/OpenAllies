//! Replays research/phase8-ai/native-cai-bombard.json (original 5207F0
//! KKC_CAIStateBombard::Think with 51E7A0, 4CA760, 529C20 and the member
//! iterator under Unicorn at PC24/PC53/PC64) against
//! aa_mind::cai_thinks::bombard::think.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    cai_thinks::{
        RegimentWorld,
        bombard::{BombardThinkWorld, ITERATION_END, think},
        stack_action::{ActionCommand, ActionWorld},
    },
};
use aa_numeric::Precision;
use serde_json::{Value, json};

const STATES: [(&str, u32); 4] = [
    ("bombard", 0xd4),
    ("move", 0x68),
    ("sleep", 0x34),
    ("other", 0x700),
];
/// The target is object 1; organization members are 1000 + index and the
/// organization component 6000.
const TARGET: u32 = 1;
const MEMBER_BASE: u32 = 1000;
const OWN_COMPONENT: u32 = 6000;
const OWNER: u32 = 0x0eee_0000;
const REGION: u32 = 0x0bad_4000;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-bombard.json"
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
    byte20: bool,
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
            assert_eq!(h, TARGET);
            "target".into()
        }
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
        let t = &self.c["target"];
        if gtid == u(&t["gtid"]) && t["registered"].as_bool().unwrap() {
            return Ok(Some(TARGET));
        }
        Ok(None)
    }
    fn object_gtid(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, TARGET);
        Ok(u(&self.c["target"]["gtid"]))
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
impl BombardThinkWorld for Host {
    fn bombard_target(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["target_gtid"]))
    }
    fn bombard_label_length(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["label_length"]))
    }
    fn bombard_point(&mut self, _state: StateId) -> Result<[u32; 2]> {
        Ok([u(&self.c["point"][0]), u(&self.c["point"][1])])
    }
    fn bombard_byte20(&mut self, _state: StateId) -> Result<bool> {
        Ok(self.byte20)
    }
    fn set_bombard_byte20(&mut self, _state: StateId, value: bool) -> Result<()> {
        self.byte20 = value;
        Ok(())
    }
    fn organization_owner(&mut self, _mind: MindId, arena: &mut Arena) -> Result<u32> {
        self.event("organization_owner", json!({"result": OWNER}), arena);
        Ok(OWNER)
    }
    fn organization_name(&mut self, _mind: MindId, arena: &mut Arena) -> Result<String> {
        self.event("organization_name", json!({}), arena);
        Ok("organization name".into())
    }
    fn target_virtual_5c(
        &mut self,
        target: u32,
        owner: u32,
        flag: u32,
        arena: &mut Arena,
    ) -> Result<bool> {
        assert_eq!(target, TARGET);
        let r = self.c["target"]["v5c"].as_bool().unwrap();
        self.event(
            "target_virtual_5c",
            json!({"owner": owner, "flag": flag, "result": r}),
            arena,
        );
        Ok(r)
    }
    fn object_flags8(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, TARGET);
        Ok(u(&self.c["target"]["flags8"]))
    }
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]> {
        assert_eq!(object, TARGET);
        let p = &self.c["target"]["position"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn post_alert(
        &mut self,
        kind: u32,
        owner: u32,
        position: [u32; 2],
        name: &str,
        arena: &mut Arena,
    ) -> Result<()> {
        self.event(
            "post_alert",
            json!({"kind": kind, "owner": owner, "position": position, "argument": name}),
            arena,
        );
        Ok(())
    }
    fn region_at(&mut self, point: [u32; 2], arena: &mut Arena) -> Result<u32> {
        self.event("region_at", json!({"point": point}), arena);
        Ok(REGION)
    }
    fn region_54f3c0(&mut self, region: u32, owner: u32, arena: &mut Arena) -> Result<bool> {
        assert_eq!(region, REGION);
        let r = self.c["region_check"].as_bool().unwrap();
        self.event("region_54f3c0", json!({"owner": owner, "result": r}), arena);
        Ok(r)
    }
    fn relation(
        &mut self,
        _mind: MindId,
        target: u32,
        a: u32,
        b: u32,
        arena: &mut Arena,
    ) -> Result<i32> {
        let r = self.c["relation"].as_i64().unwrap() as i32;
        let k = self.name(target);
        self.event(
            "relation",
            json!({"object": k, "a": a, "b": b, "result": r}),
            arena,
        );
        Ok(r)
    }
    fn point_relation(
        &mut self,
        _mind: MindId,
        _state: StateId,
        b: u32,
        arena: &mut Arena,
    ) -> Result<i32> {
        let r = self.c["point_relation"].as_i64().unwrap() as i32;
        self.event("point_relation", json!({"b": b, "result": r}), arena);
        Ok(r)
    }
    fn mind_virtual_144(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_144", json!({"args": []}), arena);
        Ok(())
    }
    fn null_point(&mut self) -> Result<[u32; 2]> {
        Ok([u(&self.c["null_point"][0]), u(&self.c["null_point"][1])])
    }
    fn move_order_object(
        &mut self,
        _mind: MindId,
        args: [u32; 5],
        arena: &mut Arena,
    ) -> Result<()> {
        let k = self.name(args[1]);
        self.event(
            "move_order_object",
            json!({"args": [args[0], k, args[2], args[3], args[4]]}),
            arena,
        );
        Ok(())
    }
    fn move_order_point(&mut self, _mind: MindId, args: [u32; 6], arena: &mut Arena) -> Result<()> {
        self.event("move_order_point", json!({"args": args}), arena);
        Ok(())
    }
    fn organization_component(&mut self, _mind: MindId) -> Result<u32> {
        Ok(OWN_COMPONENT)
    }
    fn component_next_index(&mut self, component: u32, index: u32) -> Result<u32> {
        assert_eq!(component, OWN_COMPONENT);
        let list = self.c["members"].as_array().unwrap();
        let mut i = index.wrapping_add(1) as i32;
        while i < list.len() as i32 {
            if !list[i as usize].is_null() {
                return Ok(i as u32);
            }
            i += 1;
        }
        Ok(ITERATION_END)
    }
    fn component_member(&mut self, component: u32, index: u32) -> Result<u32> {
        assert_eq!(component, OWN_COMPONENT);
        Ok(if self.c["members"][index as usize].is_null() {
            0
        } else {
            MEMBER_BASE + index
        })
    }
    fn member_virtual_34(&mut self, member: u32, value: u32, arena: &mut Arena) -> Result<bool> {
        let index = (member - MEMBER_BASE) as usize;
        let r = self.c["members"][index]["accepts"].as_bool().unwrap();
        let k = self.name(member);
        self.event(
            "member_virtual_34",
            json!({"member": k, "value": value, "result": r}),
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
fn native_bombard_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 800);
    for (_, cov) in f["coverage"].as_object().unwrap() {
        assert_eq!(cov["visited"], cov["instructions"]);
    }
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
            issue: c["issue"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| b.as_bool().unwrap())
                .collect(),
            byte20: u(&c["byte20"]) != 0,
        };
        let status_in = u(&c["states"]["bombard"]);
        let wrong = think(id("bombard"), &mut arena, &mut host, p).unwrap();
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
        assert_eq!(json!(u32::from(host.byte20)), o["byte20"], "{name}");
    }
}
