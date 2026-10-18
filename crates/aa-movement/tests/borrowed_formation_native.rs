//! Both pinned executables, all explicit PCs; lazy reads/writes and mutations.
use aa_movement::borrowed_formation::{self, World};
use aa_movement::{Error, Result};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::BTreeMap;

struct Storage {
    case: Value,
    actors: BTreeMap<u32, Value>,
    leader: Option<u32>,
    positions: Vec<Vec<[u32; 2]>>,
    current: usize,
    mind: bool,
    trace: Vec<Value>,
    events: Vec<Value>,
    counts: BTreeMap<String, u64>,
    draws: Vec<u32>,
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn xy(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}
fn id(v: &Value) -> Option<u32> {
    (!v.is_null()).then(|| u(&v["id"]))
}
impl Storage {
    fn new(c: &Value) -> Self {
        let o = &c["organization"];
        let mut actors = BTreeMap::new();
        for a in o["members"]
            .as_array()
            .unwrap()
            .iter()
            .chain([&o["leader"], &c["new_leader"]])
        {
            if let Some(k) = id(a) {
                actors.entry(k).or_insert_with(|| a.clone());
            }
        }
        let first: Vec<_> = o["slot_positions"]
            .as_array()
            .unwrap()
            .iter()
            .map(xy)
            .collect();
        let second = c
            .get("second_positions")
            .map(|a| a.as_array().unwrap().iter().map(xy).collect())
            .unwrap_or_else(|| first.clone());
        Self {
            case: c.clone(),
            actors,
            leader: id(&o["leader"]),
            positions: vec![first, second],
            current: 0,
            mind: o["owner_has_mind"].as_bool().unwrap(),
            trace: vec![],
            events: vec![],
            counts: BTreeMap::new(),
            draws: c["draws"].as_array().unwrap().iter().map(u).collect(),
        }
    }
    fn event(&mut self, name: &str, args: Vec<Value>) -> Result<()> {
        let mut entry = vec![json!(name)];
        entry.extend(args);
        self.trace.push(json!(entry));
        let count = self.counts.entry(name.into()).or_default();
        *count += 1;
        let count = *count;
        if let Some(changes) = self.case.get("mutations").and_then(Value::as_array) {
            for a in changes.clone() {
                if a["event"] != name || a["occurrence"] != count {
                    continue;
                }
                let value = &a["value"];
                match a["kind"].as_str().unwrap() {
                    "leader" => self.leader = value.as_u64().map(|v| v as u32),
                    "positions" => self.current = u(value) as usize,
                    "slot" => self.actors.get_mut(&u(&a["actor"])).unwrap()["slot"] = value.clone(),
                    "offset" => {
                        self.positions[u(&a["array"]) as usize][u(&a["slot"]) as usize]
                            [u(&a["axis"]) as usize] = u(value)
                    }
                    "mind" => self.mind = false,
                    "member" => {
                        self.case["organization"]["members"][u(&a["slot"]) as usize] = value
                            .as_u64()
                            .map(|v| self.actors[&(v as u32)].clone())
                            .unwrap_or(Value::Null)
                    }
                    k => panic!("unexpected mutation {k}"),
                }
            }
        }
        if self.case["stop"]["event"] == name && self.case["stop"]["occurrence"] == count {
            return Err(Error::World(name.into()));
        }
        Ok(())
    }
    fn actor(&self, a: u32) -> Result<&Value> {
        self.actors
            .get(&a)
            .ok_or_else(|| Error::World("native_null".into()))
    }
    fn element(&self, a: u32) -> Result<&Value> {
        if self.case["missing_elements"]
            .as_array()
            .is_some_and(|v| v.iter().any(|v| u(v) == a))
        {
            return Err(Error::World("native_null".into()));
        }
        self.actor(a)
    }
    fn output(&self, outcome: &str) -> Value {
        json!({"outcome":outcome,"leader":self.leader,"positions":self.positions,"current_positions":self.current,"events":self.events,"trace":self.trace,"remaining_draws":self.draws})
    }
}
impl World for Storage {
    type Actor = u32;
    type Positions = usize;
    type Formation = usize;
    type Mind = ();
    fn leader(&mut self) -> Result<Option<u32>> {
        self.event("leader", vec![])?;
        Ok(self.leader)
    }
    fn store_leader(&mut self, a: Option<u32>) -> Result<()> {
        self.event("store_leader", vec![json!(a)])?;
        self.leader = a;
        Ok(())
    }
    fn element_slot(&mut self, a: u32) -> Result<i32> {
        self.event("element_slot", vec![json!(a)])?;
        Ok(self.element(a)?["slot"].as_i64().unwrap() as i32)
    }
    fn member(&mut self, s: i32) -> Result<Option<u32>> {
        self.event("member", vec![json!(s)])?;
        Ok(id(&self.case["organization"]["members"][s as usize]))
    }
    fn primary_slot(&mut self) -> Result<i32> {
        self.event("primary_slot", vec![])?;
        Ok(self.case["organization"]["primary_slot"].as_i64().unwrap() as i32)
    }
    fn formation(&mut self) -> Result<Option<usize>> {
        self.event("formation", vec![])?;
        Ok((!self.case["organization"]["formation"].is_null()).then_some(0))
    }
    fn formation_oriented(&mut self, f: usize) -> Result<bool> {
        self.event("formation_oriented", vec![json!(f)])?;
        Ok(u(&self.case["organization"]["formation"]["oriented"]) != 0)
    }
    fn owner_facing(&mut self) -> Result<[f32; 2]> {
        self.event("owner_facing", vec![])?;
        Ok(xy(&self.case["organization"]["facing"]).map(f32::from_bits))
    }
    fn formation_offset(&mut self, s: i32) -> Result<[f32; 2]> {
        self.event("formation_offset", vec![json!(s)])?;
        Ok(xy(&self.case["organization"]["formation"]["offsets"][s as usize]).map(f32::from_bits))
    }
    fn positions(&mut self) -> Result<usize> {
        self.event("positions", vec![])?;
        Ok(self.current)
    }
    fn offset(&mut self, a: usize, s: i32, axis: usize) -> Result<f32> {
        self.event("offset", vec![json!(a), json!(s), json!(axis)])?;
        Ok(f32::from_bits(self.positions[a][s as usize][axis]))
    }
    fn store_offset(&mut self, a: usize, s: i32, axis: usize, value: f32) -> Result<()> {
        self.event("store_offset", vec![json!(a), json!(s), json!(axis)])?;
        self.positions[a][s as usize][axis] = value.to_bits();
        Ok(())
    }
    fn element_attached(&mut self, a: u32) -> Result<bool> {
        self.event("element_attached", vec![json!(a)])?;
        Ok(u(&self.element(a)?["attached"]) != 0)
    }
    fn actor_position(&mut self, a: u32, axis: usize) -> Result<f32> {
        self.event("actor_position", vec![json!(a), json!(axis)])?;
        Ok(f32::from_bits(u(&self.actor(a)?["position"][axis])))
    }
    fn owner_id(&mut self) -> Result<u32> {
        self.event("owner_id", vec![])?;
        Ok(u(&self.case["organization"]["owner_id"]))
    }
    fn actor_id(&mut self, a: u32) -> Result<u32> {
        self.event("actor_id", vec![json!(a)])?;
        Ok(u(&self.actor(a)?["id"]))
    }
    fn owner_mind(&mut self) -> Result<Option<()>> {
        self.event("owner_mind", vec![])?;
        Ok(self.mind.then_some(()))
    }
    fn mind_leader_changed(&mut self, _: ()) -> Result<()> {
        self.event("mind_leader_changed", vec![])?;
        self.events
            .push(json!({"name":"MindLeaderChanged","bits":[]}));
        Ok(())
    }
    fn next_random_u32(&mut self) -> Result<u32> {
        self.event("random", vec![])?;
        let d = self.draws.remove(0);
        self.events.push(json!({"name":"Random","bits":[d]}));
        Ok(d)
    }
    fn detached_member(&mut self, o: u32, a: u32, xy: [f32; 2]) -> Result<()> {
        self.event("diagnostic", vec![])?;
        self.events.push(
            json!({"name":"DetachedDiagnostic","bits":[o,a,xy[0].to_bits(),xy[1].to_bits()]}),
        );
        Ok(())
    }
}

#[test]
fn borrowed_native_leader_and_slot_replay() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-borrowed-leader/native-borrowed-leader.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 678);
    for case in cases {
        let mut storage = Storage::new(case);
        let pc: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let result = match case["operation"].as_str().unwrap() {
            "set_leader" => {
                borrowed_formation::set_leader(&mut storage, id(&case["new_leader"]), pc)
            }
            "compute" => borrowed_formation::compute_slot_position(
                &mut storage,
                case["slot"].as_i64().unwrap() as i32,
                pc,
            )
            .map(|_| ()),
            k => panic!("{k}"),
        };
        let outcome = match result {
            Ok(()) => "ok".into(),
            Err(Error::World(s)) if s.starts_with("attached member offset") => "native_null".into(),
            Err(Error::World(s)) => s,
            Err(e) => panic!("unexpected {e}"),
        };
        let mut expected = case["output"].clone();
        if outcome == "native_null" {
            // Typed handles replace the original attempted null actor load.
            if expected["trace"]
                .as_array()
                .unwrap()
                .last()
                .is_some_and(|e| e[0] == "actor_position" && e[1].is_null())
            {
                expected["trace"].as_array_mut().unwrap().pop();
            }
        }
        assert_eq!(
            storage.output(&outcome),
            expected,
            "{} {} {}",
            case["name"],
            case["image"],
            case["precision"]
        );
    }
}
