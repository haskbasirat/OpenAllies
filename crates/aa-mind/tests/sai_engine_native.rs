//! Replays research/phase8-ai/native-goal-engine.json (original 7A7700 and
//! 7A93B0 under Unicorn) against aa_mind::sai::engine.
use aa_mind::{
    Result,
    sai::engine::{
        BUCKETS, Bucket, Controller, EngineWorld, GoalId, GoalState, reevaluate_goals, select_goals,
    },
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::VecDeque;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-goal-engine.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn goal_json(g: &GoalState) -> Value {
    json!({"status": g.status, "current": g.current, "base": g.base, "costs": g.costs})
}

struct Host {
    c: Value,
    effects: Vec<(Value, bool)>,
    events: Vec<Value>,
    destroyed: Vec<GoalId>,
}
impl Host {
    fn event(&mut self, name: &str, goal: GoalId, extra: Value, ctl: &mut Controller) {
        let mut e = json!({"event": name, "goal": goal});
        for (k, v) in extra.as_object().unwrap() {
            e[k] = v.clone();
        }
        self.events.push(e);
        if let Some((effect, used)) = self
            .effects
            .iter_mut()
            .find(|(e, used)| !*used && e["event"] == name && e["goal"] == goal)
        {
            *used = true;
            let g = ctl.goals.get_mut(&goal).unwrap();
            for (k, v) in effect["set"].as_object().unwrap() {
                match k.as_str() {
                    "status" => g.status = u(v),
                    "current" => g.current = u(v),
                    "base" => g.base = u(v),
                    "costs" => g.costs = v.as_array().unwrap().iter().map(u).collect(),
                    other => panic!("{other}"),
                }
            }
        }
    }
}
impl EngineWorld for Host {
    fn goal_type(&mut self, goal: GoalId, ctl: &mut Controller) -> Result<u32> {
        let t = u(&self.c["types"][goal.to_string()]);
        self.event("type", goal, json!({"result": t}), ctl);
        Ok(t)
    }
    fn expired(&mut self, goal: GoalId, ctl: &mut Controller) -> Result<bool> {
        let r = self.c["expired"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| u(g) == goal);
        self.event("expired", goal, json!({"result": r}), ctl);
        Ok(r)
    }
    fn destroy(&mut self, goal: GoalId, ctl: &mut Controller) -> Result<()> {
        self.event("destroy", goal, json!({}), ctl);
        self.destroyed.push(goal);
        Ok(())
    }
    fn reevaluate(&mut self, goal: GoalId, ctl: &mut Controller) -> Result<()> {
        self.event("reevaluate", goal, json!({}), ctl);
        Ok(())
    }
    fn deselect(&mut self, goal: GoalId, ctl: &mut Controller) -> Result<()> {
        self.event("deselect", goal, json!({}), ctl);
        Ok(())
    }
    fn evaluate(
        &mut self,
        goal: GoalId,
        running: &mut [u32],
        order: &[GoalId],
        index: usize,
        ctl: &mut Controller,
    ) -> Result<()> {
        self.event(
            "evaluate",
            goal,
            json!({"running": running, "order": order, "index": index}),
            ctl,
        );
        Ok(())
    }
    fn assign(
        &mut self,
        goal: GoalId,
        running: &mut [u32],
        order: &[GoalId],
        index: usize,
        ctl: &mut Controller,
    ) -> Result<()> {
        self.event(
            "assign",
            goal,
            json!({"running": running, "order": order, "index": index}),
            ctl,
        );
        Ok(())
    }
    fn yield_point(&mut self, _ctl: &mut Controller) -> Result<()> {
        self.events.push(json!({"event": "yield"}));
        Ok(())
    }
}

#[test]
fn native_goal_engine_passes_match_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 255);
    let mut faults = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut ctl = Controller {
            buckets: vec![Bucket::default(); BUCKETS],
            goals: Default::default(),
        };
        for (k, g) in c["goals"].as_object().unwrap() {
            ctl.goals.insert(
                k.parse().unwrap(),
                GoalState {
                    status: u(&g["status"]),
                    costs: g["costs"].as_array().unwrap().iter().map(u).collect(),
                    current: u(&g["current"]),
                    base: u(&g["base"]),
                    travel: 0,
                },
            );
        }
        for (b, lists) in c["buckets"].as_array().unwrap().iter().enumerate() {
            for (l, items) in lists.as_array().unwrap().iter().enumerate() {
                ctl.buckets[b].lists[l] = items
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|g| g.as_u64().map(|g| g as u32))
                    .collect::<VecDeque<_>>();
            }
        }
        let economy: Vec<u32> = c["economy"].as_array().unwrap().iter().map(u).collect();
        let mut host = Host {
            c: c.clone(),
            effects: c["effects"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| (e.clone(), false))
                .collect(),
            events: vec![],
            destroyed: vec![],
        };
        let result = match c["entry"].as_str().unwrap() {
            "select" => select_goals(&mut ctl, &economy, &mut host, p),
            "reevaluate" => reevaluate_goals(&mut ctl, &mut host),
            other => panic!("{other}"),
        };
        assert_eq!(json!(host.events), o["events"], "{name}");
        if !o["fault"].is_null() {
            assert!(result.is_err(), "{name}");
            faults += 1;
            continue;
        }
        result.unwrap();
        let lists: Vec<Value> = ctl
            .buckets
            .iter()
            .map(|b| json!([b.lists[0], b.lists[1], b.lists[2]]))
            .collect();
        assert_eq!(json!(lists), o["lists"], "{name}");
        let goals: serde_json::Map<String, Value> = ctl
            .goals
            .iter()
            .filter(|(g, _)| !host.destroyed.contains(g))
            .map(|(g, s)| (g.to_string(), goal_json(s)))
            .collect();
        assert_eq!(json!(goals), o["goals"], "{name}");
        assert_eq!(json!(host.destroyed), o["destroyed"], "{name}");
    }
    assert_eq!(faults, 6);
}
