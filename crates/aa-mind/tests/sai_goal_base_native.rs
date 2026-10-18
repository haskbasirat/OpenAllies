//! Replays research/phase8-ai/native-goal-base.json (original SAI base goal
//! virtuals 7A6930, 7A6A90, 7A67B0, 7A3B00, 7A4FB0, 7A5020, 7A5080, 7A5600
//! and SAIData 4A7130 under Unicorn) against aa_mind::sai::goal_base.
use aa_mind::{
    Result,
    sai::{
        assign::{ActorId, ActorList, Resources, Roster, SaiActor, State},
        engine::{Controller, GoalId, GoalState},
        goal_base::{self, GoalBaseWorld, TravelWeighting},
    },
    x87::binary_from_x87,
};
use aa_numeric::{Binary, Precision, crt_pow::CrtEnvironment};
use serde_json::{Value, json};
use std::cmp::Ordering;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-goal-base.json"
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
const LOCATION: u32 = 0x4c00_0000;

struct Host {
    c: Value,
    cw: u16,
    events: Vec<Value>,
}
impl Host {
    fn key(&self, table: &str, id: u32) -> &Value {
        &self.c[table][id.to_string()]
    }
    fn hash(&self, kind: u32, ids: &[u32]) -> u32 {
        let mut xs = vec![u(&self.c["seed"]), kind];
        xs.extend_from_slice(ids);
        mix(&xs)
    }
    fn hv(&self, kind: u32, ids: &[u32]) -> u64 {
        let k = i64::from(self.hash(kind, ids) % 61);
        let bias = self.c["bias"].as_i64().unwrap();
        ((k - bias) as f64 / self.c["div"].as_f64().unwrap()).to_bits()
    }
    fn chance(&self, kind: u32, ids: &[u32]) -> bool {
        u64::from(self.hash(kind, ids) % 100) < self.c["pct"][kind.to_string()].as_u64().unwrap()
    }
    fn listed(&self, table: &str, id: u32) -> bool {
        self.c[table].as_array().unwrap().iter().any(|g| u(g) == id)
    }
}
fn st0(bits: u64) -> Result<Binary> {
    Ok(Binary::from_f64_bits(bits).unwrap())
}
impl GoalBaseWorld for Host {
    fn goal_type(&mut self, _s: &mut State<'_>, goal: GoalId) -> Result<u32> {
        let r = u(self.key("type", goal));
        self.events
            .push(json!({"event": "goal_type", "goal": goal, "result": r}));
        Ok(r)
    }
    fn category(&mut self, _s: &mut State<'_>, goal: GoalId) -> Result<u32> {
        let r = u(self.key("category", goal));
        self.events
            .push(json!({"event": "category", "goal": goal, "result": r}));
        Ok(r)
    }
    fn location(&mut self, _s: &mut State<'_>, goal: GoalId) -> Result<u32> {
        self.events.push(json!({"event": "location", "goal": goal}));
        Ok(LOCATION + 0x10 * goal)
    }
    fn value(
        &mut self,
        _s: &mut State<'_>,
        goal: GoalId,
        actor: Option<ActorId>,
        check: bool,
    ) -> Result<Binary> {
        let r = self.hv(1, &[goal, actor.unwrap_or(0), u32::from(check)]);
        self.events
            .push(json!({"event": "value", "goal": goal, "actor": actor,
            "check": u8::from(check), "result": r}));
        st0(r)
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
        st0(r)
    }
    fn count(&mut self, s: &mut State<'_>, goal: GoalId) -> Result<i32> {
        let r = s.roster.goal_list(goal)?.count;
        self.events
            .push(json!({"event": "count", "goal": goal, "result": r}));
        Ok(r)
    }
    fn required_actor(&mut self, _s: &mut State<'_>, goal: GoalId) -> Result<Option<ActorId>> {
        let r = opt(self.key("required", goal));
        self.events
            .push(json!({"event": "required_actor", "goal": goal, "result": r}));
        Ok(r)
    }
    fn remove_actor(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        actor: Option<ActorId>,
        resources: Resources<'_>,
        refresh: bool,
        keep: bool,
        extra: u32,
    ) -> Result<()> {
        self.events.push(
            json!({"event": "remove_actor", "goal": goal, "actor": actor,
            "res": res_name(&resources), "refresh": u8::from(refresh), "keep": u8::from(keep),
            "extra": extra}),
        );
        let list = s.roster.goal_list_mut(goal)?;
        if list.remove_first(actor) {
            list.count -= 1;
        }
        if let Some(a) = actor {
            let record = s.roster.actors.get_mut(&a).unwrap();
            if record.owner == Some(goal) {
                record.owner = None;
            }
        }
        Ok(())
    }
    fn priority_hook(
        &mut self,
        _s: &mut State<'_>,
        goal: GoalId,
        add: Option<ActorId>,
        remove: Option<ActorId>,
    ) -> Result<Binary> {
        let r = self.hv(3, &[goal, add.unwrap_or(0), remove.unwrap_or(0)]);
        self.events
            .push(json!({"event": "priority_hook", "goal": goal, "add": add,
            "remove": remove, "result": r}));
        st0(r)
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
    fn actor_flag4(&mut self, _s: &mut State<'_>, actor: ActorId) -> Result<bool> {
        let r = self.chance(5, &[actor]);
        self.events
            .push(json!({"event": "actor_flag4", "actor": actor, "result": r}));
        Ok(r)
    }
    fn actor_can_serve(
        &mut self,
        _s: &mut State<'_>,
        actor: ActorId,
        goal: GoalId,
        flag: bool,
    ) -> Result<bool> {
        let r = self.chance(7, &[actor, goal, u32::from(flag)]);
        self.events.push(
            json!({"event": "actor_can_serve", "actor": actor, "goal": goal,
            "flag": u8::from(flag), "result": r}),
        );
        Ok(r)
    }
    fn unit_check(&mut self, _s: &mut State<'_>, actor: ActorId) -> Result<bool> {
        let r = self.chance(6, &[actor]);
        self.events
            .push(json!({"event": "unit_check", "actor": actor, "result": r}));
        Ok(r)
    }
    fn travel_time(&mut self, _s: &mut State<'_>, actor: ActorId, location: u32) -> Result<Binary> {
        let goal = (location - LOCATION) / 0x10;
        let k = self.hash(4, &[actor, goal]) % 61;
        let r = (f64::from(k) / self.c["tdiv"].as_f64().unwrap()).to_bits();
        self.events
            .push(json!({"event": "travel_time", "actor": actor, "goal": goal, "result": r}));
        st0(r)
    }
    fn modifiers(&mut self, _s: &mut State<'_>, goal_type: u32) -> Result<Option<Vec<u32>>> {
        let m = &self.c["modifiers"][goal_type.to_string()];
        Ok(m.as_array().map(|v| v.iter().map(u).collect()))
    }
    fn apply_modifier(
        &mut self,
        _s: &mut State<'_>,
        modifier: u32,
        actor: ActorId,
        sum: &mut u32,
        product: &mut u32,
        matched: &mut bool,
    ) -> Result<()> {
        let flag_in = u8::from(*matched);
        let r = self.chance(8, &[modifier, actor]);
        if r {
            let addend = ((self.hash(9, &[modifier]) % 21) as f32 - 10.0) / 4.0;
            let factor = (self.hash(10, &[modifier]) % 9 + 6) as f32 / 8.0;
            *sum = (f32::from_bits(*sum) + addend).to_bits();
            if !*matched {
                *matched = true;
                *product = (f32::from_bits(*product) * factor).to_bits();
            }
        }
        self.events.push(
            json!({"event": "apply_modifier", "modifier": modifier, "actor": actor,
            "matched_in": flag_in, "result": r, "sum": *sum, "product": *product}),
        );
        Ok(())
    }
    fn travel_weighting(&mut self, _s: &mut State<'_>) -> Result<TravelWeighting> {
        Ok(TravelWeighting {
            normaliser: u(&self.c["norm"]),
            exponent: u(&self.c["exponent"]),
            crt: CrtEnvironment {
                sse2_available: true,
                mxcsr: 0x1f80,
                x87_control: self.cw,
            },
        })
    }
    fn affordable(
        &mut self,
        _s: &mut State<'_>,
        goal: GoalId,
        resources: Resources<'_>,
        relaxed: bool,
    ) -> Result<bool> {
        let r = self.listed("affordable", goal);
        self.events.push(
            json!({"event": "affordable", "goal": goal, "res": res_name(&resources),
            "relaxed": u8::from(relaxed), "result": r}),
        );
        Ok(r)
    }
    fn finalize(
        &mut self,
        s: &mut State<'_>,
        goal: GoalId,
        resources: Resources<'_>,
        draw: bool,
    ) -> Result<()> {
        self.events.push(
            json!({"event": "finalize", "goal": goal, "res": res_name(&resources),
            "draw": u8::from(draw)}),
        );
        s.ctl.goal_mut(goal)?.base = u(self.key("finalized", goal));
        Ok(())
    }
}

#[test]
fn native_goal_base_virtuals_match_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 640);
    let (mut faults, mut results) = (0, 0);
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let cw = match p {
            Precision::Single24 => 0x007f,
            Precision::Double53 => 0x027f,
            Precision::Extended64 => 0x037f,
        };
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
                    travel: u(&g["travel"]),
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
                    travel_cache: u(&a["travel_cache"]),
                    route: a["route"].as_bool().unwrap().then_some(1),
                },
            );
        }
        roster.pools = c["pools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (!p.is_null()).then(|| list_from(p)))
            .collect();
        let reserved: Vec<Option<u32>> =
            c["reserved"].as_array().unwrap().iter().map(opt).collect();
        let n = reserved.len() as i32;
        roster.reserved = ActorList::from_actors(reserved, n);
        let running: Vec<u32> = c["economy"].as_array().unwrap().iter().map(u).collect();
        let mut host = Host {
            c: c.clone(),
            cw,
            events: vec![],
        };
        let goal = u(&c["goal"]);
        let a = &c["args"];
        let res = |v: &Value| match v.as_str().unwrap() {
            "null" => Resources::Null,
            "running" => Resources::Running(&running),
            "economy" => Resources::Economy,
            other => panic!("{other}"),
        };
        let flag = |v: &Value| v.as_u64().unwrap() != 0;
        let result: Result<Option<Binary>> = {
            let mut s = State {
                ctl: &mut ctl,
                roster: &mut roster,
            };
            let h = &mut host;
            match c["entry"].as_str().unwrap() {
                "add" => goal_base::add_actor(
                    h,
                    &mut s,
                    goal,
                    opt(&a[0]),
                    res(&a[1]),
                    flag(&a[2]),
                    flag(&a[3]),
                    p,
                )
                .map(|_| None),
                "remove" => goal_base::remove_actor(
                    h,
                    &mut s,
                    goal,
                    opt(&a[0]),
                    res(&a[1]),
                    flag(&a[2]),
                    flag(&a[3]),
                    p,
                )
                .map(|_| None),
                "reevaluate" => goal_base::reevaluate(h, &mut s, goal, res(&a[0]), p).map(|_| None),
                "priority" => {
                    goal_base::priority_with_actors(h, &mut s, goal, opt(&a[0]), opt(&a[1]), p)
                        .map(Some)
                }
                "swap" => {
                    goal_base::swap_value(h, &mut s, goal, opt(&a[0]), opt(&a[1]), flag(&a[2]), p)
                        .map(Some)
                }
                "value" => goal_base::value(h, &mut s, goal, opt(&a[0]), flag(&a[1]), p).map(Some),
                "held" => goal_base::held_value(h, &mut s, goal, opt(&a[0]), p).map(Some),
                "deselect" => goal_base::deselect(h, &mut s, goal).map(|_| None),
                "weight" => goal_base::travel_weight(h, &mut s, u(&a[0]), p).map(Some),
                other => panic!("{other}"),
            }
        };
        assert_eq!(json!(host.events), o["events"], "{name}");
        if !o["fault"].is_null() {
            assert!(result.is_err(), "{name}");
            faults += 1;
            continue;
        }
        let result = result.unwrap_or_else(|e| panic!("{name}: {e}"));
        if let Some(v) = result {
            let r = &o["result"];
            let native = binary_from_x87(
                r["extended_significand"].as_u64().unwrap(),
                r["extended_sign_exponent"].as_u64().unwrap() as u16,
            )
            .unwrap();
            assert_eq!(
                v.compare(&native),
                Ordering::Equal,
                "{name} {v:?} {native:?}"
            );
            assert_eq!(
                v.is_negative_nonzero(),
                native.is_negative_nonzero(),
                "{name}"
            );
            assert_eq!(v.store_f32().unwrap(), u(&r["f32_bits"]), "{name}");
            results += 1;
        }
        let goals: serde_json::Map<String, Value> = ctl
            .goals
            .iter()
            .map(|(g, s)| {
                let mut v = list_json(&roster.goal_actors[g]);
                v["status"] = json!(s.status);
                v["current"] = json!(s.current);
                v["base"] = json!(s.base);
                v["travel"] = json!(s.travel);
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
        let actors: serde_json::Map<String, Value> = roster
            .actors
            .iter()
            .map(|(a, r)| {
                (
                    a.to_string(),
                    json!({"owner": r.owner, "travel_cache": r.travel_cache, "route": r.route.is_some()}),
                )
            })
            .collect();
        assert_eq!(json!(actors), o["actors"], "{name}");
    }
    assert_eq!(faults, 36);
    assert_eq!(results, 337);
}
