//! Replays research/phase8-ai/native-goal-priority.json (original 7A5200 and
//! 7A3860 executed under Unicorn) against aa_mind::sai::goal_priority.
use aa_mind::{
    Result,
    sai::goal_priority::{
        Context, EgoGoalSettings, Goal, GoalPriority, GoalWorld, ResourceSlot, affordable, finalize,
    },
};
use aa_numeric::Precision;
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-goal-priority.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn goal_json(g: &Goal) -> Value {
    json!({"status": g.status, "fuzzy": g.fuzzy, "inertia_applied": g.inertia_applied,
           "priority": g.priority, "costs": g.costs})
}
fn goal_from(v: &Value) -> Goal {
    Goal {
        status: u(&v["status"]),
        fuzzy: u(&v["fuzzy"]),
        inertia_applied: u(&v["inertia_applied"]),
        priority: u(&v["priority"]),
        costs: v["costs"].as_array().unwrap().iter().map(u).collect(),
    }
}

struct Host {
    c: Value,
    rng: u32,
    events: Vec<Value>,
    mutations: Vec<Value>,
}
impl Host {
    fn event(&mut self, name: &str, goal: &mut Goal, result: Value) {
        self.events
            .push(json!({"event": name, "goal": goal_json(goal), "result": result}));
        if self.mutations.first().is_some_and(|m| m["event"] == name) {
            let m = self.mutations.remove(0);
            let mut g = goal_json(goal);
            for (k, v) in m["goal"].as_object().unwrap() {
                g[k] = v.clone();
            }
            *goal = goal_from(&g);
        }
    }
}
impl GoalWorld for Host {
    fn goal_type(&mut self, goal: &mut Goal) -> Result<u32> {
        let t = u(&self.c["goal_type"]);
        self.event("goal_type", goal, json!(t));
        Ok(t)
    }
    fn base_priority(&mut self, goal: &mut Goal) -> Result<()> {
        goal.priority = u(&self.c["base_priority"]);
        self.event("base_priority", goal, Value::Null);
        Ok(())
    }
    fn settlement_reserve_exempt(&mut self, goal: &mut Goal) -> Result<bool> {
        let r = self.c["reserve_exempt"].as_bool().unwrap();
        self.event("reserve_exempt", goal, json!(r));
        Ok(r)
    }
    fn sai_count(&mut self, goal: &mut Goal) -> Result<i32> {
        let n = self.c["sai_count"].as_i64().unwrap() as i32;
        self.event("sai_count", goal, json!(n));
        Ok(n)
    }
    fn sai_random(&mut self) -> Result<u32> {
        // 85D840 step on the SAI RNG word (same as aa_simulation NativeRng).
        let wide = u64::from(self.rng) * 0x41c6_4e6d + 0x3039;
        self.rng = wide as u32;
        Ok((wide >> 16) as u32)
    }
}

fn context(c: &Value) -> Context {
    let e = &c["ego"];
    Context {
        world_time: u(&c["world_time"]),
        settlements: c["settlements"].as_i64().unwrap() as i32,
        ego: EgoGoalSettings {
            default_fuzziness: u(&e["default_fuzziness"]),
            default_inertia: u(&e["default_inertia"]),
            priorities: e["priorities"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| GoalPriority {
                    bonus: u(&r["bonus"]),
                    multiplier: u(&r["multiplier"]),
                    minimum: u(&r["minimum"]),
                    fuzziness: u(&r["fuzziness"]),
                    inertia: u(&r["inertia"]),
                })
                .collect(),
            saving_time_limit: u(&e["saving_time_limit"]),
            settlement_reserved_amount: u(&e["reserved_amount"]),
            settlement_reserved_time: u(&e["reserved_time"]),
            settlement_reserved_max_settlements: e["reserved_max_settlements"].as_i64().unwrap()
                as i32,
        },
        resources: c["resources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| ResourceSlot {
                definition_word18: u(&r["word18"]),
                economy_1a0: u(&r["word1a0"]),
                economy_1b8: u(&r["word1b8"]),
                available: u(&r["available"]),
            })
            .collect(),
    }
}

#[test]
fn native_goal_priority_and_affordability_match_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 1356);
    let (mut finalized, mut direct) = (0, 0);
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut host = Host {
            c: c.clone(),
            rng: u(&c["rng_state"]),
            events: vec![],
            mutations: c
                .get("mutations")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
        };
        let ctx = context(c);
        let mut goal = goal_from(&c["goal"]);
        let result = if c["entry"] == "finalize" {
            finalize(&mut goal, u(&c["draw"]) != 0, &ctx, &mut host, p).unwrap();
            finalized += 1;
            Value::Null
        } else {
            direct += 1;
            json!(u32::from(
                affordable(&mut goal, u(&c["relaxed"]) != 0, &ctx, &mut host, p).unwrap()
            ))
        };
        let name = &c["name"];
        assert!(host.mutations.is_empty(), "{name}");
        assert_eq!(goal_json(&goal), o["goal"], "{name}");
        assert_eq!(json!(host.rng), o["rng_state"], "{name}");
        assert_eq!(result, o["affordable"], "{name}");
        let native: Vec<Value> = o["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["event"] != "debug_setting_query")
            .cloned()
            .collect();
        assert_eq!(host.events, native, "{name}");
    }
    assert_eq!((finalized, direct), (696, 660));
}

#[test]
fn native_sai_enum_tables_match_original_constructors() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-sai-tables.json"
    );
    let f: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let check = |key: &str, table: &[&str]| {
        let entries = f["tables"][key]["entries"].as_array().unwrap();
        assert_eq!(entries.len(), table.len(), "{key}");
        for (i, e) in entries.iter().enumerate() {
            assert_eq!(e["value"], json!(i), "{key}");
            assert_eq!(e["name"], json!(table[i]), "{key}");
            assert_eq!(e["flag"], json!(0), "{key}");
        }
    };
    use aa_mind::sai::tables::*;
    check("goal_types", &GOAL_TYPES);
    check("difficulty_levels", &DIFFICULTY_LEVELS);
    check("goal_actor_proximity", &GOAL_ACTOR_PROXIMITY);
    check("component_types", &COMPONENT_TYPES);
    assert_eq!(goal_type("RECRUIT"), Some(12));
}
