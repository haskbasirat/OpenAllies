use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId,
    construct::{Installed, Kind},
    element_move::Payload,
    moving_target::{self, Actor, World},
};
use aa_numeric::{Binary, Precision};
use aa_spatial::{
    collision::{Descriptor, Shape},
    geometry::Placement,
    query::{Hit, PreparedQuery},
};
use serde_json::{Value, json};
struct Host {
    c: Value,
    events: Vec<Value>,
    fail: Option<&'static str>,
}
impl Host {
    fn log(
        &mut self,
        event: &str,
        mut details: Value,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()> {
        details["op"] = json!(event);
        self.events.push(details);
        if self.fail == Some(event) {
            return Err(Error::World(format!("fixture {event} failed")));
        }
        let m = self.c["mutations"][event].clone();
        if let Some(v) = m["owner"].as_u64() {
            a.mind_mut(MindId(1))?.actor = v as u32;
        }
        if let Some(v) = m.get("destination") {
            p.plan.movement.destination_bits = serde_json::from_value(v.clone()).unwrap();
        }
        if let Some(v) = m["mode"].as_u64() {
            p.plan.movement.mode = v as u32;
        }
        if let Some(v) = m["target_id"].as_u64() {
            p.plan.movement.target_id = v as u32;
        }
        if let Some(v) = m.get("target_flags") {
            self.c["actors"]["3"]["flags"] = v.clone();
        }
        if let Some(v) = m.get("previous") {
            p.plan.previous_destination_bits = serde_json::from_value(v.clone()).unwrap();
        }
        for key in ["time", "detached_seconds"] {
            if let Some(v) = m.get(key) {
                self.c[key] = v.clone();
            }
        }
        Ok(())
    }
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
impl World for Host {
    fn resolve(&self, id: u32) -> Result<Option<u32>> {
        Ok(
            (id >> 16 == 1 && self.c["actors"].get((id & 0xffff).to_string()).is_some())
                .then_some(id & 0xffff),
        )
    }
    fn actor(&self, id: u32) -> Result<Actor> {
        let a = &self.c["actors"][id.to_string()];
        Ok(Actor {
            position: serde_json::from_value(a["position"].clone()).unwrap(),
            facing: if id == 3 && !self.c["target_facing"].is_null() {
                serde_json::from_value(self.c["target_facing"].clone()).unwrap()
            } else {
                [0, 0xbf800000]
            },
            flags: u(&a["flags"]),
            performer: a["component74"].as_u64().map(|n| n as u32),
        })
    }
    fn soft_shape(&mut self, id: u32, p: &mut Payload, a: &mut Arena) -> Result<Shape> {
        self.log("soft_shape", json!({"actor":id}), p, a)?;
        let w: Vec<u32> = serde_json::from_value(self.c["shape"].clone()).unwrap();
        Ok(Shape {
            kind: w[0],
            radius_bits: w[1],
            bounds: [w[2], w[3], w[4], w[5]],
        })
    }
    fn radius(&mut self, id: u32, p: &mut Payload, a: &mut Arena) -> Result<Binary> {
        self.log("radius", json!({"actor":id}), p, a)?;
        Ok(Binary::from_f64_bits(self.c["callbacks"]["radius64"].as_u64().unwrap()).unwrap())
    }
    fn actor_query(&mut self, id: u32, p: &mut Payload, a: &mut Arena) -> Result<PreparedQuery> {
        self.log("actor_query", json!({"actor":id,"strict":false}), p, a)?;
        Ok(PreparedQuery {
            placement: Placement {
                shape: Shape {
                    kind: 0,
                    radius_bits: 0,
                    bounds: [0; 4],
                },
                facing: [0; 2],
                position: [0; 2],
            },
            descriptor: Descriptor::default(),
            required_actor_flags: 0,
        })
    }
    fn replacement_query_key(&mut self, id: u32, p: &mut Payload, a: &mut Arena) -> Result<u32> {
        self.log("replacement_query_key", json!({"actor":id}), p, a)?;
        Ok(id)
    }
    fn set_query_position(
        &mut self,
        q: &mut PreparedQuery,
        point: [u32; 2],
        _: &Arena,
    ) -> Result<()> {
        self.events
            .push(json!({"op":"set_query_position","point":point}));
        q.placement.position = point;
        Ok(())
    }
    fn first_hit(&mut self, q: &PreparedQuery, p: &mut Payload, a: &mut Arena) -> Result<Hit> {
        self.log("first_hit",json!({"point":q.placement.position,"ignore_moving":q.descriptor.ignore_moving,"ignored":q.descriptor.ignore_actor}),p,a)?;
        Ok(Hit {
            kind: u(&self.c["hit"]),
            reference: 0,
            cell: [0; 2],
        })
    }
    fn maximum_range(&mut self, id: u32, p: &mut Payload, a: &mut Arena) -> Result<Binary> {
        self.log("maximum_range", json!({"performer":id}), p, a)?;
        Ok(Binary::from_f64_bits(self.c["maximum64"].as_u64().unwrap()).unwrap())
    }
    fn speed(&mut self, id: u32, p: &mut Payload, a: &mut Arena) -> Result<Binary> {
        self.log("speed", json!({"actor":id}), p, a)?;
        Ok(Binary::from_f64_bits(self.c["speed64"].as_u64().unwrap()).unwrap())
    }
}
fn setup(c: &Value) -> (Payload, Arena, Installed, Host) {
    let mut a = Arena::default();
    let mut mind = Mind::base(u(&c["owner"]));
    let active = match c["active_offset"].as_u64() {
        Some(0xf8) => StateId(2),
        Some(0x164) => StateId(3),
        _ => StateId(1),
    };
    mind.stack.push_back(Some(active));
    a.minds.insert(MindId(1), mind);
    for id in [1, 2, 3] {
        a.states.insert(
            StateId(id),
            State {
                parent: MindId(1),
                discard_on_suspend: false,
                status: if id == 1 { u(&c["status"]) } else { 0 },
            },
        );
    }
    let installed = Installed {
        kind: Kind::Element,
        mind_id: MindId(1),
        state_bindings: [(0x7c, StateId(1)), (0xf8, StateId(2)), (0x164, StateId(3))]
            .into_iter()
            .collect(),
        update_head: None,
        words: [(0x184, u(&c["formation_mode"]))].into_iter().collect(),
        bytes: Default::default(),
        references: Default::default(),
    };
    (
        serde_json::from_value(c["payload"].clone()).unwrap(),
        a,
        installed,
        Host {
            c: c.clone(),
            events: vec![],
            fail: None,
        },
    )
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-moving-target/native-target-fixtures.json"
    ))
    .unwrap()
}
#[test]
fn native_moving_target_projection_query_and_replan_match() {
    let f = fixture();
    assert_eq!(f["cases"].as_array().unwrap().len(), 330);
    for row in f["cases"].as_array().unwrap() {
        let c = &row["input"];
        let (mut p, mut a, i, mut w) = setup(c);
        let pc: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let result = moving_target::update(StateId(1), &i, &mut p, &mut a, pc, &mut w)
            .unwrap_or_else(|e| panic!("{}: {e}", c["name"]));
        let movers = w.c["movers"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    json!({"requested":v["requested"],"factor":v["factor"]}),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        let actual = json!({"payload":p,"status":a.state(StateId(1)).unwrap().status,"owner":a.mind(MindId(1)).unwrap().actor,"time":w.c["time"],"detached_seconds":w.c["detached_seconds"],"diagnostic":w.c["diagnostic"],"movers":movers,"trace":w.events,"result":result});
        assert_eq!(actual, row["output"], "{}", c["name"]);
    }
}
#[test]
fn actual_element_mode_two_stops_at_unimplemented_formation_branch() {
    let f = fixture();
    let rows = f["unsupported_prefix_cases"].as_array().unwrap();
    assert_eq!(rows.len(), 4);
    for row in rows {
        let c = &row["input"];
        let (mut p, mut a, i, mut w) = setup(c);
        let pc = serde_json::from_value(c["precision"].clone()).unwrap();
        let error = moving_target::update(StateId(1), &i, &mut p, &mut a, pc, &mut w).unwrap_err();
        assert!(error.to_string().contains("mode2 formation placement"));
        assert_eq!(json!(p), row["output"]["payload"]);
        assert_eq!(json!(w.events), row["output"]["trace"]);
        assert_eq!(
            json!(a.mind(MindId(1)).unwrap().actor),
            row["output"]["owner"]
        );
    }
}
#[test]
fn missing_real_query_stops_before_replan_and_invalid_class_is_rejected() {
    let f = fixture();
    let c = &f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["output"]["trace"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["op"] == "first_hit")
        })
        .unwrap()["input"];
    let (mut p, mut a, mut i, mut w) = setup(c);
    let before = p.plan.movement.destination_bits;
    let nodes = p.plan.movement.path.nodes.len();
    w.fail = Some("first_hit");
    assert!(
        moving_target::update(StateId(1), &i, &mut p, &mut a, Precision::Single24, &mut w).is_err()
    );
    assert_eq!(p.plan.movement.destination_bits, before);
    assert_eq!(p.plan.movement.path.nodes.len(), nodes);
    assert_eq!(w.events.last().unwrap()["op"], "first_hit");
    i.kind = Kind::Regiment;
    w.events.clear();
    assert!(
        moving_target::update(StateId(1), &i, &mut p, &mut a, Precision::Single24, &mut w).is_err()
    );
    assert!(w.events.is_empty());
}
