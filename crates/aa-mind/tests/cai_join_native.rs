//! Replays research/phase8-ai/native-cai-join.json (original 521850
//! KKC_CAIStateJoin::Think with 5217C0, 432E40 and the 4364C0 member
//! iterator under Unicorn) against aa_mind::cai_thinks::join::think.
use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId, World,
    cai_thinks::{
        RegimentWorld,
        join::{ITERATION_END, JoinThinkWorld, think},
    },
};
use serde_json::{Value, json};

const STATES: [(&str, u32); 4] = [
    ("join", 0x24c),
    ("move", 0x68),
    ("sleep", 0x34),
    ("other", 0x700),
];
/// The target object is handle 1000; member i is handle i + 1 and its Mind
/// 100 + i + 1.
const TARGET: u32 = 1000;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-join.json"
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
    fn name(&self, object: u32) -> Value {
        match object {
            0 => json!(0),
            TARGET => json!("target"),
            m => json!(format!("m{}", m - 1)),
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
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        let t = &self.c["target"];
        if gtid == u(&t["gtid"]) && t["registered"].as_bool().unwrap() {
            return Ok(Some(TARGET));
        }
        for (i, m) in self.c["members"].as_array().unwrap().iter().enumerate() {
            if !m.is_null() && u(&m["gtid"]) == gtid {
                return Ok(Some(i as u32 + 1));
            }
        }
        Ok(None)
    }
    fn object_gtid(&mut self, object: u32) -> Result<u32> {
        Ok(u(&self.c["members"][object as usize - 1]["gtid"]))
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
impl JoinThinkWorld for Host {
    fn join_target(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["target_gtid"]))
    }
    fn join_arrived(&mut self, _state: StateId) -> Result<bool> {
        Ok(u(&self.c["arrived"]) != 0)
    }
    fn join_gathering(&mut self, _state: StateId) -> Result<bool> {
        Ok(u(&self.c["gathering"]) != 0)
    }
    fn join_listed(&mut self, _state: StateId) -> Result<Vec<u32>> {
        let count = u(&self.c["listed_count"]) as i32;
        Ok(self.c["listed"]
            .as_array()
            .unwrap()
            .iter()
            .take(count.max(0) as usize)
            .map(u)
            .collect())
    }
    fn organization_count_14c(&mut self, _mind: MindId) -> Result<i32> {
        Ok(u(&self.c["count"]) as i32)
    }
    fn organization_entry_148(&mut self, _mind: MindId, index: u32) -> Result<u32> {
        Ok(u(&self.c["entries"][index as usize]))
    }
    fn target_virtual_130(
        &mut self,
        target: u32,
        entry: u32,
        flag: bool,
        arena: &mut Arena,
    ) -> Result<bool> {
        assert_eq!(target, TARGET);
        let r = self.c["target"]["v130"].as_bool().unwrap();
        self.event(
            "target_virtual_130",
            json!({"entry": entry, "flag": u32::from(flag), "result": r}),
            arena,
        );
        Ok(r)
    }
    fn mind_virtual_144(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_144", json!({"args": []}), arena);
        Ok(())
    }
    fn mind_virtual_16c(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_16c", json!({"args": []}), arena);
        Ok(())
    }
    fn null_point(&mut self) -> Result<[u32; 2]> {
        Ok([u(&self.c["null_point"][0]), u(&self.c["null_point"][1])])
    }
    fn move_order(&mut self, _mind: MindId, args: [u32; 5], arena: &mut Arena) -> Result<()> {
        let target = self.name(args[1]);
        self.event(
            "move_order",
            json!({"args": [args[0], target, args[2], args[3], args[4]]}),
            arena,
        );
        Ok(())
    }
    fn next_member_index(&mut self, _mind: MindId, index: u32) -> Result<u32> {
        let count = u(&self.c["member_count"]) as i32;
        let members = self.c["members"].as_array().unwrap();
        let mut i = index.wrapping_add(1) as i32;
        while i < count {
            if !members[i as usize].is_null() {
                return Ok(i as u32);
            }
            i += 1;
        }
        Ok(ITERATION_END)
    }
    fn component_member(&mut self, _mind: MindId, index: u32) -> Result<u32> {
        Ok(if self.c["members"][index as usize].is_null() {
            0
        } else {
            index + 1
        })
    }
    fn member_mind(&mut self, member: u32) -> Result<Option<u32>> {
        Ok(self.c["members"][member as usize - 1]["mind"]
            .as_bool()
            .unwrap()
            .then_some(100 + member))
    }
    fn member_mind_virtual_f4(
        &mut self,
        member_mind: u32,
        target: u32,
        arena: &mut Arena,
    ) -> Result<()> {
        let member = self.name(member_mind - 100);
        let target = self.name(target);
        self.event(
            "member_mind_virtual_f4",
            json!({"member": member, "target": target}),
            arena,
        );
        Ok(())
    }
}

#[test]
fn native_join_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 400);
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
        };
        let status_in = u(&c["states"]["join"]);
        let result = think(id("join"), &mut arena, &mut host);
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
    }
    assert_eq!(faults, 4);
}
