use aa_mind::{
    Arena, Error, MindId, Result, StateId, ability_decision::Decision, ability_motion::*,
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

struct Host {
    c: Value,
    events: Vec<Value>,
    movers: std::collections::BTreeMap<u32, aa_movement::MoverState>,
    fail: Option<&'static str>,
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn opt(v: &Value) -> Option<u32> {
    v.as_u64().map(|n| n as u32)
}
fn binary(v: &Value) -> Binary {
    Binary::from_f64_bits(v.as_u64().unwrap()).unwrap()
}
fn state(c: &Value) -> aa_movement::MoverState {
    aa_movement::MoverState {
        destination: [0.; 3],
        last_step_speed: 0.,
        speed: 0.,
        requested_speed: f32::from_bits(u(&c["requested_speed"])),
        acceleration: 0.,
        turn_rate: 0.,
        desired_direction: [0.; 3],
        target_id: 0xdeadbeef,
        target_range: Some(-777.),
        ignore_terrain_speed_limit: false,
        speed_factor: 1.,
        instant_turn: false,
        turning: false,
        falling: false,
    }
}
impl Host {
    fn new(c: &Value) -> Self {
        Self {
            c: c.clone(),
            events: vec![],
            movers: (1..=4)
                .map(|i| (i, state(&c["actors"][i.to_string()])))
                .collect(),
            fail: None,
        }
    }
    fn actor(&self, a: u32) -> &Value {
        &self.c["actors"][a.to_string()]
    }
    fn event(
        &mut self,
        name: &str,
        args: Vec<Value>,
        decision: Option<&mut Decision>,
        arena: &mut Arena,
    ) -> Result<()> {
        let mut d: Decision = decision
            .as_deref()
            .copied()
            .unwrap_or_else(|| serde_json::from_value(self.c["decision"].clone()).unwrap());
        let snapshot = json!({"mind_actor":arena.mind(MindId(1))?.actor,"decision":d});
        let mut event = vec![json!(name)];
        event.extend(args);
        event.push(snapshot);
        self.events.push(json!(event));
        if self.fail == Some(name) {
            return Err(Error::World(format!("fixture {name} failure")));
        }
        if self.c["mutations"]
            .as_array()
            .unwrap()
            .first()
            .is_some_and(|m| m["event"] == name)
        {
            let m = self.c["mutations"].as_array_mut().unwrap().remove(0);
            let mut v = serde_json::to_value(d).unwrap();
            if let Some(changes) = m["decision"].as_object() {
                for (k, x) in changes {
                    v[k] = x.clone();
                }
            }
            d = serde_json::from_value(v).unwrap();
            for k in ["mind_actor", "bombard_mode", "delta"] {
                if !m[k].is_null() {
                    self.c[k] = m[k].clone();
                }
            }
            if !m["mind_actor"].is_null() {
                arena.mind_mut(MindId(1))?.actor = u(&m["mind_actor"]);
            }
            if let Some(actors) = m["actors"].as_object() {
                for (i, changes) in actors {
                    for (k, x) in changes.as_object().unwrap() {
                        self.c["actors"][i][k] = x.clone();
                    }
                }
            }
        }
        self.c["decision"] = json!(d);
        if let Some(decision) = decision {
            *decision = d;
        }
        Ok(())
    }
    fn plain_event(&mut self, n: &str, args: Vec<Value>) -> Result<()> {
        let mut a = Arena::default();
        a.minds
            .insert(MindId(1), aa_mind::Mind::base(u(&self.c["mind_actor"])));
        self.event(n, args, None, &mut a)
    }
    fn result(&self, d: &Decision, a: &Arena) -> Value {
        json!({"mind_actor":a.mind(MindId(1)).unwrap().actor,"decision":d,"events":self.events,
        "movers":self.movers.iter().map(|(id,m)|(id.to_string(),[m.target_id,m.target_range.unwrap().to_bits()])).collect::<std::collections::BTreeMap<_,_>>()})
    }
}
impl aa_mind::action::World for Host {
    fn actor(&self, a: u32) -> Result<aa_mind::action::Actor> {
        Ok(aa_mind::action::Actor {
            mover: opt(&self.actor(a)["mover"]),
            active_component: opt(&self.actor(a)["active"]),
        })
    }
    fn component_owner(&self, m: u32) -> Result<u32> {
        Ok(m)
    }
    fn stop_mover(&mut self, m: u32, keep: bool, clear: bool, a: &mut Arena) -> Result<()> {
        self.event(
            "stop",
            vec![json!(m), json!(u32::from(keep)), json!(u32::from(clear))],
            None,
            a,
        )
    }
    fn default_stance(&mut self, a: u32, arena: &mut Arena) -> Result<()> {
        self.event("stance", vec![json!(a)], None, arena)
    }
}
impl World for Host {
    fn recharge_remaining(&mut self, b: u32, a: u32) -> Result<Binary> {
        self.plain_event("recharge", vec![json!(b), json!(a)])?;
        Ok(binary(&self.c["recharge"]))
    }
    fn schedule_wait(
        &mut self,
        _: MindId,
        delay: u32,
        data: u32,
        d: &mut Decision,
        a: &mut Arena,
    ) -> Result<()> {
        self.event("schedule", vec![json!(delay), json!(data)], Some(d), a)
    }
    fn bombard_mode(&self, _: MindId) -> Result<u32> {
        Ok(u(&self.c["bombard_mode"]))
    }
    fn local_definition(&self, b: u32) -> Result<u32> {
        Ok(b)
    }
    fn actor_definition(&mut self, m: MindId, a: &mut Arena) -> Result<u32> {
        Ok(a.mind(m)?.actor)
    }
    fn local_weight(&mut self, l: u32, d: u32) -> Result<Binary> {
        self.plain_event("weight", vec![json!(l), json!(d)])?;
        Ok(binary(&self.c["weight"]))
    }
    fn performer(&self, a: u32) -> Result<u32> {
        Ok(u(&self.actor(a)["performer"]))
    }
    fn maximum_weight(&mut self, p: u32) -> Result<Binary> {
        self.plain_event("maximum", vec![json!(p)])?;
        Ok(binary(&self.c["maximum"]))
    }
    fn reconsider(&mut self, _: MindId, d: &mut Decision, a: &mut Arena) -> Result<()> {
        self.event("reconsider", vec![], Some(d), a)
    }
    fn think_move(&mut self, _: StateId, d: &mut Decision, a: &mut Arena) -> Result<()> {
        self.event("think", vec![], Some(d), a)
    }
    fn modifier(&mut self, a: u32) -> Result<Option<u32>> {
        Ok((self.actor(a)["modifier"] == true).then_some(a))
    }
    fn effective_range(&mut self, b: u32, m: Option<u32>) -> Result<Binary> {
        self.plain_event("range", vec![json!(b), json!(m)])?;
        Ok(binary(&self.c["range"]))
    }
    fn target_is_moving(&mut self, t: u32) -> Result<bool> {
        self.plain_event("moving", vec![json!(t)])?;
        Ok(self.c["moving"] == true)
    }
    fn facing(&self, a: u32) -> Result<[u32; 2]> {
        Ok(serde_json::from_value(self.actor(a)["facing"].clone()).unwrap())
    }
    fn world_delta(&self) -> Result<u32> {
        Ok(u(&self.c["delta"]))
    }
    fn target_speed(&mut self, t: u32, d: &mut Decision, a: &mut Arena) -> Result<Binary> {
        self.event("speed", vec![json!(t)], Some(d), a)?;
        Ok(binary(&self.c["speed"]))
    }
    fn set_target(&mut self, m: u32, t: Option<u32>, r: u32) -> Result<()> {
        self.plain_event("target", vec![json!(m), json!(t), json!(r)])?;
        let id = t.map(|t| u(&self.actor(t)["id"]));
        let delta = u(&self.c["delta"]);
        let pc = serde_json::from_value(self.c["precision"].clone()).unwrap();
        aa_movement::target::set_target(self.movers.get_mut(&m).unwrap(), id, r, delta, pc)
            .map_err(|e| Error::World(e.to_string()))
    }
    fn target_decision(
        &mut self,
        _: MindId,
        t: Option<u32>,
        d: &mut Decision,
        a: &mut Arena,
    ) -> Result<i32> {
        self.event("decision", vec![json!(t)], Some(d), a)?;
        Ok(self.c["target_decision"].as_i64().unwrap() as i32)
    }
}
fn setup(c: &Value) -> (Arena, aa_mind::construct::Installed, Decision, Host) {
    let mut a = Arena::default();
    a.minds
        .insert(MindId(1), aa_mind::Mind::base(u(&c["mind_actor"])));
    a.states.insert(
        StateId(4),
        aa_mind::State {
            parent: MindId(1),
            discard_on_suspend: false,
            status: 1,
        },
    );
    let installed = aa_mind::construct::Installed {
        kind: aa_mind::construct::Kind::Element,
        mind_id: MindId(1),
        state_bindings: [(0x7c, StateId(4))].into_iter().collect(),
        update_head: None,
        words: Default::default(),
        bytes: Default::default(),
        references: Default::default(),
    };
    (
        a,
        installed,
        serde_json::from_value(c["decision"].clone()).unwrap(),
        Host::new(c),
    )
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-ability-motion/native-motion-fixtures.json"
    ))
    .unwrap()
}
#[test]
fn native_wait_and_pursuit_preserve_callback_order_stores_and_live_reads() {
    let f = fixture();
    let mut count = 0;
    for row in f["cases"].as_array().unwrap() {
        let c = &row["input"];
        if c["operation"] == "target" {
            continue;
        }
        let (mut a, i, mut d, mut h) = setup(c);
        let pc: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        if c["operation"] == "wait" {
            wait(&i, &mut d, &mut a, &mut h).unwrap();
        } else {
            pursue(&i, &mut d, &mut a, &mut h, pc).unwrap();
        }
        assert_eq!(h.result(&d, &a), row["output"], "{}", c["name"]);
        assert!(h.c["mutations"].as_array().unwrap().is_empty());
        count += 1;
    }
    assert_eq!(count, 213);
}
#[test]
fn boundary_failure_keeps_prior_target_commit_and_prevents_reconsider() {
    let f = fixture();
    let c = &f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["input"]["operation"] == "pursue"
                && r["input"]["moving"] == true
                && r["input"]["target_decision"] == 4
        })
        .unwrap()["input"];
    let (mut a, i, mut d, mut h) = setup(c);
    h.fail = Some("decision");
    assert!(pursue(&i, &mut d, &mut a, &mut h, Precision::Single24).is_err());
    assert_eq!(h.movers[&1].target_id, 0x30003);
    assert_eq!(h.events.last().unwrap()[0], "decision");
    let (mut a, i, mut d, mut h) = setup(c);
    h.c["actors"]["1"]["mover"] = Value::Null;
    h.c["actors"]["1"]["active"] = Value::Null;
    assert!(pursue(&i, &mut d, &mut a, &mut h, Precision::Single24).is_err());
    assert_eq!(h.events.len(), 1);
}
