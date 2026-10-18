//! Replays research/phase8-ai/native-goal-assign.json (original 7A9150 and
//! 7A7E30 with all assignment callees under Unicorn) against
//! aa_mind::sai::assign.
use aa_mind::{
    Result,
    sai::{
        assign::{
            ActorId, ActorList, AssignWorld, Resources, Roster, SaiActor, State, assign, evaluate,
        },
        engine::{Controller, GoalId, GoalState},
    },
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-goal-assign.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn opt(v: &Value) -> Option<u32> {
    v.as_u64().map(|x| x as u32)
}
fn mix(xs: &[u32]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for &x in xs {
        h ^= x;
        h = h.wrapping_mul(0x0100_0193);
        h ^= h >> 13;
    }
    h
}
fn res_name(r: &Resources<'_>) -> &'static str {
    match r {
        Resources::Null => "null",
        Resources::Running(_) => "running",
        Resources::Economy => "economy",
    }
}
fn list_json(l: &ActorList) -> Value {
    json!({
        "actors": l.actors(),
        "tail": l.tail_actor().map(|a| vec![a]).unwrap_or_default(),
        "count": l.count,
    })
}
fn list_from(v: &Value) -> ActorList {
    ActorList::from_actors(
        v["actors"].as_array().unwrap().iter().map(opt),
        v["count"].as_i64().unwrap() as i32,
    )
}

struct Host {
    c: Value,
    economy_len: usize,
    events: Vec<Value>,
}
impl Host {
    fn key(&self, table: &str, goal: GoalId) -> &Value {
        &self.c[table][goal.to_string()]
    }
    fn hv(&self, kind: u32, ids: &[u32]) -> u64 {
        let mut xs = vec![u(&self.c["seed"]), kind];
        xs.extend_from_slice(ids);
        let k = i64::from(mix(&xs) % 61);
        let bias = self.c["bias"].as_i64().unwrap();
        ((k - bias) as f64 / self.c["div"].as_f64().unwrap()).to_bits()
    }
    fn chance(&self, kind: u32, ids: &[u32]) -> bool {
        let mut xs = vec![u(&self.c["seed"]), kind];
        xs.extend_from_slice(ids);
        u64::from(mix(&xs) % 100) < self.c["pct"][kind.to_string()].as_u64().unwrap()
    }
    fn listed(&self, table: &str, id: u32) -> bool {
        self.c[table].as_array().unwrap().iter().any(|g| u(g) == id)
    }
    fn st0(&mut self, bits: u64) -> Result<Binary> {
        Ok(Binary::from_f64_bits(bits).unwrap())
    }
}
impl AssignWorld for Host {
    fn category(&mut self, _s: &mut State<'_>, goal: GoalId) -> Result<u32> {
        let r = u(self.key("category", goal));
        self.events
            .push(json!({"event": "category", "goal": goal, "result": r}));
        Ok(r)
    }
    fn satisfied(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<bool> {
        let need = self.key("need", goal).as_i64();
        let count = s.roster.goal_list(goal)?.count;
        let r = need.is_some_and(|n| i64::from(count) >= n);
        self.events
            .push(json!({"event": "satisfied", "goal": goal, "result": r}));
        Ok(r)
    }
    fn expired(&mut self, _s: &mut State<'_>, goal: GoalId) -> Result<bool> {
        let r = self.listed("expired", goal);
        self.events
            .push(json!({"event": "expired", "goal": goal, "result": r}));
        Ok(r)
    }
    fn swap_value(
        &mut self,
        _s: &mut State<'_>,
        goal: GoalId,
        gained: Option<ActorId>,
        lost: Option<ActorId>,
    ) -> Result<Binary> {
        let r = self.hv(3, &[goal, gained.unwrap_or(0), lost.unwrap_or(0)]);
        self.events.push(json!({"event": "swap_value", "goal": goal, "gained": gained, "lost": lost, "result": r}));
        self.st0(r)
    }
    fn value(
        &mut self,
        _s: &mut State<'_>,
        goal: GoalId,
        actor: Option<ActorId>,
    ) -> Result<Binary> {
        let r = self.hv(1, &[goal, actor.unwrap_or(0)]);
        self.events
            .push(json!({"event": "value", "goal": goal, "actor": actor, "result": r}));
        self.st0(r)
    }
    fn held_value(
        &mut self,
        _s: &mut State<'_>,
        goal: GoalId,
        actor: Option<ActorId>,
    ) -> Result<Binary> {
        let r = self.hv(2, &[goal, actor.unwrap_or(0)]);
        self.events
            .push(json!({"event": "held_value", "goal": goal, "actor": actor, "result": r}));
        self.st0(r)
    }
    fn spare(&mut self, _s: &mut State<'_>, goal: GoalId) -> Result<i32> {
        let r = self.key("spare", goal).as_i64().unwrap() as i32;
        self.events
            .push(json!({"event": "spare", "goal": goal, "result": r}));
        Ok(r)
    }
    fn required_actor(&mut self, _s: &mut State<'_>, goal: GoalId) -> Result<Option<ActorId>> {
        let r = opt(self.key("required", goal));
        self.events
            .push(json!({"event": "required_actor", "goal": goal, "result": r}));
        Ok(r)
    }
    fn critical(&mut self, _s: &mut State<'_>, goal: GoalId) -> Result<bool> {
        let r = self.listed("critical", goal);
        self.events
            .push(json!({"event": "critical", "goal": goal, "result": r}));
        Ok(r)
    }
    fn add_actor(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        actor: Option<ActorId>,
        resources: Resources<'_>,
        a: bool,
        b: bool,
    ) -> Result<()> {
        if let Resources::Running(r) = &resources {
            assert_eq!(r.len(), self.economy_len);
        }
        self.events
            .push(json!({"event": "add_actor", "goal": goal, "actor": actor,
            "res": res_name(&resources), "a": u8::from(a), "b": u8::from(b)}));
        let list = s.roster.goal_actors.get_mut(&goal).unwrap();
        list.push_front(actor);
        list.count += 1;
        if let Some(actor) = actor {
            s.roster.actors.get_mut(&actor).unwrap().owner = Some(goal);
        }
        Ok(())
    }
    fn remove_actor(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        actor: Option<ActorId>,
        resources: Resources<'_>,
        a: bool,
        b: bool,
        c: bool,
    ) -> Result<()> {
        self.events.push(
            json!({"event": "remove_actor", "goal": goal, "actor": actor,
            "res": res_name(&resources), "a": u8::from(a), "b": u8::from(b), "c": u8::from(c)}),
        );
        let list = s.roster.goal_actors.get_mut(&goal).unwrap();
        if list.remove_first(actor) {
            list.count -= 1;
        }
        if let Some(actor) = actor {
            let record = s.roster.actors.get_mut(&actor).unwrap();
            if record.owner == Some(goal) {
                record.owner = None;
            }
        }
        Ok(())
    }
    fn excluded(
        &mut self,
        _s: &mut State<'_>,
        goal: GoalId,
        actor: Option<ActorId>,
    ) -> Result<bool> {
        let r = self.chance(6, &[goal, actor.unwrap_or(0)]);
        self.events
            .push(json!({"event": "excluded", "goal": goal, "actor": actor, "result": r}));
        Ok(r)
    }
    fn deselect(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<()> {
        self.events.push(json!({"event": "deselect", "goal": goal}));
        let g = s.ctl.goal_mut(goal)?;
        if g.status != 0 {
            g.status = 1;
        }
        Ok(())
    }
    fn actor_dead(&mut self, _s: &mut State<'_>, actor: ActorId) -> Result<bool> {
        let r = self.listed("dead", actor);
        self.events
            .push(json!({"event": "actor_dead", "actor": actor, "result": r}));
        Ok(r)
    }
    fn actor_can_serve(
        &mut self,
        _s: &mut State<'_>,
        actor: ActorId,
        goal: GoalId,
    ) -> Result<bool> {
        let r = self.chance(7, &[actor, goal]);
        self.events
            .push(json!({"event": "actor_can_serve", "actor": actor, "goal": goal, "result": r}));
        Ok(r)
    }
    fn affordable(
        &mut self,
        _s: &mut State<'_>,
        goal: GoalId,
        running: &[u32],
        relaxed: bool,
    ) -> Result<bool> {
        assert_eq!(running.len(), self.economy_len);
        let r = self.listed("affordable", goal);
        self.events.push(
            json!({"event": "affordable", "goal": goal, "relaxed": u8::from(relaxed), "result": r}),
        );
        Ok(r)
    }
    fn priority_with_actors(&mut self, _s: &mut State<'_>, goal: GoalId) -> Result<Binary> {
        let r = self.key("priority", goal).as_u64().unwrap();
        self.events
            .push(json!({"event": "priority_with_actors", "goal": goal, "result": r}));
        self.st0(r)
    }
    fn yield_point(&mut self, _s: &mut State<'_>) -> Result<()> {
        self.events.push(json!({"event": "yield"}));
        Ok(())
    }
}

#[test]
fn native_goal_assignment_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 606);
    let mut faults = 0;
    let mut events = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut ctl = Controller::default();
        let mut roster = Roster::default();
        for (k, g) in c["goals"].as_object().unwrap() {
            let id: GoalId = k.parse().unwrap();
            ctl.goals.insert(
                id,
                GoalState {
                    status: u(&g["status"]),
                    costs: vec![],
                    current: u(&g["current"]),
                    base: u(&g["base"]),
                    travel: 0,
                },
            );
            roster.goal_actors.insert(id, list_from(g));
        }
        for (k, a) in c["actors"].as_object().unwrap() {
            roster.actors.insert(
                k.parse().unwrap(),
                SaiActor {
                    owner: opt(&a["owner"]),
                    categories: a["mask"].as_u64().unwrap(),
                    ..Default::default()
                },
            );
        }
        roster.pools = c["pools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (!p.is_null()).then(|| list_from(p)))
            .collect();
        let order: Vec<GoalId> = c["order"].as_array().unwrap().iter().map(u).collect();
        let index = c["index"].as_u64().unwrap() as usize;
        let mut running: Vec<u32> = c["economy"].as_array().unwrap().iter().map(u).collect();
        let mut host = Host {
            c: c.clone(),
            economy_len: running.len(),
            events: vec![],
        };
        let goal = order[index];
        let result = {
            let mut s = State {
                ctl: &mut ctl,
                roster: &mut roster,
            };
            match c["entry"].as_str().unwrap() {
                "evaluate" => evaluate(&mut host, &mut s, goal, &mut running, &order, index),
                "assign" => assign(&mut host, &mut s, goal, &mut running, &order, index, p),
                other => panic!("{other}"),
            }
        };
        assert_eq!(json!(host.events), o["events"], "{name}");
        events += host.events.len();
        if !o["fault"].is_null() {
            assert!(result.is_err(), "{name}");
            faults += 1;
            continue;
        }
        result.unwrap();
        let goals: serde_json::Map<String, Value> = ctl
            .goals
            .iter()
            .map(|(g, s)| {
                let mut v = list_json(&roster.goal_actors[g]);
                v["status"] = json!(s.status);
                v["current"] = json!(s.current);
                v["base"] = json!(s.base);
                (g.to_string(), v)
            })
            .collect();
        assert_eq!(json!(goals), o["goals"], "{name}");
        let pools: Vec<Value> = roster
            .pools
            .iter()
            .map(|p| p.as_ref().map_or(Value::Null, list_json))
            .collect();
        assert_eq!(json!(pools), o["pools"], "{name}");
        let owners: serde_json::Map<String, Value> = roster
            .actors
            .iter()
            .map(|(a, r)| (a.to_string(), json!(r.owner)))
            .collect();
        assert_eq!(json!(owners), o["owners"], "{name}");
    }
    assert_eq!(faults, 18);
    assert_eq!(events, 9599);
}
