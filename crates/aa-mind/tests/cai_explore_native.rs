//! Replays research/phase8-ai/native-cai-explore.json (original 525D70
//! KKC_CAIStateExplore::Think with 51F200, 4D1900, 529C20 and 52A810 mode 0
//! under Unicorn at PC24/PC53/PC64) against
//! aa_mind::cai_thinks::explore::think.
use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId, World,
    cai_thinks::{
        RegimentWorld,
        explore::{Candidate, ExploreThinkWorld, think},
    },
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::VecDeque;

const STATES: [(&str, u32); 4] = [
    ("explore", 0x220),
    ("move", 0x68),
    ("sleep", 0x34),
    ("other", 0x700),
];
const GROUP: u32 = 1;
const LEADER: u32 = 2;
const LEADER_MIND: u32 = 3;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-explore.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn words(v: &Value) -> Vec<u32> {
    v.as_array().unwrap().iter().map(u).collect()
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
    target: [u32; 5],
    contains: VecDeque<bool>,
    accept: VecDeque<bool>,
    candidates_a: VecDeque<Value>,
    candidates_b: VecDeque<Value>,
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
    fn candidate(
        &mut self,
        which: &str,
        best_count: u32,
        candidate: &mut Candidate,
        arena: &Arena,
    ) {
        let spec = if which == "a" {
            self.candidates_a.pop_front().unwrap()
        } else {
            self.candidates_b.pop_front().unwrap()
        };
        let initial = candidate.words();
        if !spec.is_null() {
            let w = words(&spec);
            *candidate = Candidate::from_words([w[0], w[1], w[2], w[3], w[4]]);
        }
        self.event(
            &format!("candidate_{which}"),
            json!({"best_count": best_count, "initial": initial, "result": spec}),
            arena,
        );
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
    fn leave(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        self.event("leave", json!({"state": state_name(state)}), arena);
        if self.c["leave_clears"].as_bool().unwrap() {
            arena.state_mut(state)?.status = 0;
        }
        Ok(())
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
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        if gtid == 0x00b2_0b20 && self.c["group_registered"].as_bool().unwrap() {
            return Ok(Some(GROUP));
        }
        if gtid == 0x00b3_0b30 && self.c["leader_registered"].as_bool().unwrap() {
            return Ok(Some(LEADER));
        }
        Ok(None)
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
impl ExploreThinkWorld for Host {
    fn organization_word164(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["group_gtid"]))
    }
    fn group_4dce60(&mut self, group: u32, _mind: MindId, arena: &mut Arena) -> Result<bool> {
        assert_eq!(group, GROUP);
        let r = self.c["leads"].as_bool().unwrap();
        self.event("group_4dce60", json!({"result": r}), arena);
        Ok(r)
    }
    fn group_4dcf00(&mut self, group: u32, _state: StateId, arena: &mut Arena) -> Result<()> {
        assert_eq!(group, GROUP);
        let target = self.target;
        self.event("group_4dcf00", json!({"target": target}), arena);
        if self.c["share_sets_move"].as_bool().unwrap() {
            arena.state_mut(id("move"))?.status = 2;
        }
        Ok(())
    }
    fn group_word194(&mut self, group: u32) -> Result<u32> {
        assert_eq!(group, GROUP);
        Ok(u(&self.c["leader_gtid"]))
    }
    fn object_mind(&mut self, object: u32) -> Result<Option<u32>> {
        assert_eq!(object, LEADER);
        Ok((!self.c["leader_mind"].is_null()).then_some(LEADER_MIND))
    }
    fn mind_explore_target(&mut self, leader_mind: u32) -> Result<[u32; 5]> {
        assert_eq!(leader_mind, LEADER_MIND);
        let w = words(&self.c["leader_mind"]);
        Ok([w[0], w[1], w[2], w[3], w[4]])
    }
    fn explore_target(&mut self, _state: StateId) -> Result<[u32; 5]> {
        Ok(self.target)
    }
    fn set_explore_target(&mut self, _state: StateId, target: [u32; 5]) -> Result<()> {
        self.target = target;
        Ok(())
    }
    fn explore_point(&mut self, _state: StateId) -> Result<[u32; 2]> {
        Ok(self.point)
    }
    fn region_contains(&mut self, point: [u32; 2], arena: &mut Arena) -> Result<bool> {
        let r = self.contains.pop_front().unwrap();
        self.event(
            "region_contains",
            json!({"point": point, "result": r}),
            arena,
        );
        Ok(r)
    }
    fn region_clamp(&mut self, _state: StateId, arena: &mut Arena) -> Result<()> {
        let p = words(&self.c["clamped"]);
        self.point = [p[0], p[1]];
        self.event("region_clamp", json!({"result": p}), arena);
        Ok(())
    }
    fn region_size(&mut self) -> Result<[u32; 2]> {
        let s = words(&self.c["size"]);
        Ok([s[0], s[1]])
    }
    fn organization_position(&mut self, _mind: MindId) -> Result<[u32; 2]> {
        let p = words(&self.c["org_position"]);
        Ok([p[0], p[1]])
    }
    fn explore_radius(&mut self) -> Result<u32> {
        Ok(u(&self.c["radius"]))
    }
    fn candidate_counts(&mut self) -> Result<[u32; 2]> {
        Ok([u(&self.c["count_a"]), u(&self.c["count_b"])])
    }
    fn candidate_524170(
        &mut self,
        _state: StateId,
        best_count: u32,
        candidate: &mut Candidate,
        arena: &mut Arena,
    ) -> Result<()> {
        self.candidate("a", best_count, candidate, arena);
        Ok(())
    }
    fn candidate_5242c0(
        &mut self,
        _state: StateId,
        best_count: u32,
        candidate: &mut Candidate,
        arena: &mut Arena,
    ) -> Result<()> {
        self.candidate("b", best_count, candidate, arena);
        Ok(())
    }
    fn world_4f46e0(
        &mut self,
        _mind: MindId,
        candidate: &Candidate,
        radius: u32,
        arena: &mut Arena,
    ) -> Result<bool> {
        let r = self.accept.pop_front().unwrap();
        self.event(
            "world_4f46e0",
            json!({"candidate": candidate.words(), "radius": radius, "result": r}),
            arena,
        );
        Ok(r)
    }
    fn null_point(&mut self) -> Result<[u32; 2]> {
        let p = words(&self.c["null_point"]);
        Ok([p[0], p[1]])
    }
    fn move_order(&mut self, _mind: MindId, args: [u32; 6], arena: &mut Arena) -> Result<()> {
        self.event("move_order", json!({"args": args}), arena);
        Ok(())
    }
    fn move_retarget(&mut self, _mind: MindId, point: [u32; 2], arena: &mut Arena) -> Result<()> {
        self.event("move_retarget", json!({"point": point}), arena);
        Ok(())
    }
}

#[test]
fn native_explore_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 900);
    for (_, cov) in f["coverage"].as_object().unwrap() {
        assert_eq!(cov["visited"], cov["instructions"]);
    }
    let mut faults = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
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
        let point = words(&c["point"]);
        let target = words(&c["target"]);
        let mut host = Host {
            c: c.clone(),
            events: vec![],
            point: [point[0], point[1]],
            target: [target[0], target[1], target[2], target[3], target[4]],
            contains: c["contains"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| b.as_bool().unwrap())
                .collect(),
            accept: c["accept"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| b.as_bool().unwrap())
                .collect(),
            candidates_a: c["candidates_a"]
                .as_array()
                .unwrap()
                .iter()
                .cloned()
                .collect(),
            candidates_b: c["candidates_b"]
                .as_array()
                .unwrap()
                .iter()
                .cloned()
                .collect(),
        };
        let status_in = u(&c["states"]["explore"]);
        let result = think(id("explore"), &mut arena, &mut host, p);
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
        assert_eq!(json!(host.point), o["point"], "{name}");
        assert_eq!(json!(host.target), o["target"], "{name}");
    }
    assert_eq!(faults, 4);
}
