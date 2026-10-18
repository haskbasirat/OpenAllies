use aa_mind::{Arena, Error, Mind, MindId, Result, State, StateId, element_move::*};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::VecDeque};

struct Fixture {
    c: Value,
    trace: RefCell<Vec<Value>>,
    fail: Option<&'static str>,
}
impl Fixture {
    fn log(&mut self, name: &str, mut row: Value, p: &mut Payload, a: &mut Arena) -> Result<()> {
        row["op"] = json!(name);
        self.trace.borrow_mut().push(row);
        if self.fail == Some(name) {
            return Err(Error::World(format!("fixture required {name} failed")));
        }
        let m = &self.c["mutations"][name];
        if let Some(v) = m["owner"].as_u64() {
            a.mind_mut(MindId(1))?.actor = v as u32;
        }
        if let Some(v) = m.get("destination") {
            p.plan.movement.destination_bits = serde_json::from_value(v.clone()).unwrap();
        }
        if let Some(v) = m["mode"].as_u64() {
            p.plan.movement.mode = v as u32;
        }
        let m = m.clone();
        for field in ["time", "detached_seconds"] {
            if let Some(v) = m.get(field) {
                self.c[field] = v.clone();
            }
        }
        Ok(())
    }
    fn cb(&self, name: &str) -> bool {
        self.c["callbacks"][name].as_bool().unwrap()
    }
    fn number(&self, name: &str) -> u32 {
        self.c["callbacks"][name].as_u64().unwrap() as u32
    }
}
impl World for Fixture {
    fn actor(&self, key: u32) -> Result<Actor> {
        serde_json::from_value(self.c["actors"][key.to_string()].clone())
            .map_err(|e| Error::World(e.to_string()))
    }
    fn element(&self, key: u32) -> Result<Element> {
        Ok(serde_json::from_value(self.c["elements"][key.to_string()].clone()).unwrap())
    }
    fn mover(&self, key: u32) -> Result<Mover> {
        Ok(serde_json::from_value(self.c["movers"][key.to_string()].clone()).unwrap())
    }
    fn resolve(&self, id: u32) -> Result<Option<u32>> {
        Ok(
            if id >> 16 == 1 && self.c["actors"].get((id & 0xffff).to_string()).is_some() {
                Some(id & 0xffff)
            } else {
                None
            },
        )
    }
    fn world_time_bits(&self) -> Result<u32> {
        Ok(self.c["time"].as_u64().unwrap() as u32)
    }
    fn world_extents_bits(&self) -> Result<[u32; 2]> {
        Ok(serde_json::from_value(self.c["extents"].clone()).unwrap())
    }
    fn ensure_detached_setting(&mut self) -> Result<()> {
        Ok(())
    }
    fn detached_seconds_bits(&self) -> Result<u32> {
        Ok(self.c["detached_seconds"].as_u64().unwrap() as u32)
    }
    fn diagnostic_already_emitted(&self) -> bool {
        self.c["diagnostic"].as_bool().unwrap()
    }
    fn emit_status_diagnostic(&mut self, _: StateId, p: &mut Payload, a: &mut Arena) -> Result<()> {
        self.log("diagnostic", json!({}), p, a)
    }
    fn set_diagnostic_emitted(&mut self) {
        self.c["diagnostic"] = json!(true);
    }
    fn organization_ready(
        &mut self,
        organization: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool> {
        self.log(
            "organization_ready",
            json!({"organization":organization}),
            p,
            a,
        )?;
        Ok(self.cb("organization_ready"))
    }
    fn destination_has_kingdom_object(
        &mut self,
        point: [u32; 2],
        kingdom: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool> {
        self.log(
            "destination_has_kingdom_object",
            json!({"point":point,"kingdom":kingdom}),
            p,
            a,
        )?;
        Ok(self.cb("destination_has_kingdom_object"))
    }
    fn component88_blocks(
        &mut self,
        component: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool> {
        self.log("component88_blocks", json!({"component":component}), p, a)?;
        Ok(self.cb("component88_blocks"))
    }
    fn actor_can_move(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<bool> {
        self.log("actor_can_move", json!({"actor":actor}), p, a)?;
        Ok(self.cb("actor_can_move"))
    }
    fn mind_event(
        &mut self,
        mind: MindId,
        event: MindEvent,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool> {
        self.log("mind_event", json!({"mind":mind.0,"event":event}), p, a)?;
        Ok(self.cb("detached_result"))
    }
    fn update_target(&mut self, _: StateId, p: &mut Payload, a: &mut Arena) -> Result<Option<u32>> {
        self.log("update_target", json!({}), p, a)?;
        Ok(self.c["callbacks"]["update_target"]
            .as_u64()
            .map(|n| n as u32))
    }
    fn radius(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<Binary> {
        self.log("radius", json!({"actor":actor}), p, a)?;
        if let Some(bits) = self.c["callbacks"]["radius64"].as_u64() {
            Binary::from_f64_bits(bits).map_err(|e| Error::World(e.to_string()))
        } else {
            Binary::from_f32_bits(self.number("radius")).map_err(|e| Error::World(e.to_string()))
        }
    }
    fn property(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<u32> {
        self.log("property", json!({"actor":actor}), p, a)?;
        Ok(self.number("property"))
    }
    fn accepts_target_property(
        &mut self,
        actor: u32,
        property: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool> {
        self.log(
            "accepts_target_property",
            json!({"actor":actor,"property":property}),
            p,
            a,
        )?;
        Ok(self.cb("accepts_target_property"))
    }
    fn soft_shapes_in_range(
        &mut self,
        source: u32,
        target: u32,
        range_bits: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool> {
        self.log(
            "soft_shapes_in_range",
            json!({"source":source,"target":target,"range_bits":range_bits}),
            p,
            a,
        )?;
        Ok(self.cb("soft_shapes_in_range"))
    }
    fn mind_target(
        &mut self,
        mind: MindId,
        slot: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<Option<u32>> {
        self.log("mind_target", json!({"mind":mind.0,"slot":slot}), p, a)?;
        Ok(self.c["callbacks"][if slot == 0x4c {
            "mind_target4c"
        } else {
            "mind_target50"
        }]
        .as_u64()
        .map(|n| n as u32))
    }
    fn mind_has_location(&mut self, mind: MindId, p: &mut Payload, a: &mut Arena) -> Result<bool> {
        self.log("mind_has_location", json!({"mind":mind.0}), p, a)?;
        Ok(self.cb("mind_has_location"))
    }
    fn mind_location(&mut self, mind: MindId, p: &mut Payload, a: &mut Arena) -> Result<[u32; 2]> {
        self.log("mind_location", json!({"mind":mind.0}), p, a)?;
        Ok(serde_json::from_value(self.c["callbacks"]["mind_location"].clone()).unwrap())
    }
    fn target_point(
        &mut self,
        target: u32,
        property: u32,
        point: &mut [u32; 2],
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<bool> {
        self.log(
            "target_point",
            json!({"target":target,"property":property}),
            p,
            a,
        )?;
        *point = serde_json::from_value(self.c["callbacks"]["target_point"].clone()).unwrap();
        Ok(self.cb("target_point_ok"))
    }
    fn component74_target(
        &mut self,
        component: u32,
        mode: u32,
        target: Option<u32>,
        point: [u32; 2],
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()> {
        self.log(
            "component74_target",
            json!({"component":component,"mode":mode,"target":target,"point":point}),
            p,
            a,
        )
    }
    fn clear_component74(&mut self, component: u32, p: &mut Payload, a: &mut Arena) -> Result<()> {
        self.log("clear_component74", json!({"component":component}), p, a)
    }
    fn region_at(&self, _: [u32; 2]) -> Result<u32> {
        Ok(self.number("region"))
    }
    fn region_neighbors(&self, _: u32) -> Result<Vec<u32>> {
        Ok(serde_json::from_value(self.c["callbacks"]["neighbors"].clone()).unwrap())
    }
    fn result_plan(
        &mut self,
        _: StateId,
        trigger: Option<u32>,
        argument: bool,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()> {
        self.log(
            "result_plan",
            json!({"trigger":trigger,"argument":argument}),
            p,
            a,
        )
    }
    fn continue_path(&mut self, _: StateId, p: &mut Payload, a: &mut Arena) -> Result<()> {
        self.log("continue_path", json!({}), p, a)
    }
    fn stop_mover(&mut self, mover: u32, p: &mut Payload, a: &mut Arena) -> Result<()> {
        self.log("stop_mover", json!({"mover":mover}), p, a)
    }
    fn mover_active(&self, mover: u32) -> Result<bool> {
        self.trace
            .borrow_mut()
            .push(json!({"op":"mover_active","mover":mover}));
        Ok(self.c["movers"][mover.to_string()]["active"]
            .as_bool()
            .unwrap())
    }
    fn stop_character(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<()> {
        self.log("stop_character", json!({"actor":actor}), p, a)
    }
    fn schedule(
        &mut self,
        mind: MindId,
        delay_bits: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()> {
        self.log(
            "schedule",
            json!({"mind":mind.0,"delay_bits":delay_bits}),
            p,
            a,
        )
    }
    fn write_requested_speed(&mut self, mover: u32, bits: u32) -> Result<()> {
        self.c["movers"][mover.to_string()]["requested"] = json!(bits);
        Ok(())
    }
    fn write_speed_factor(&mut self, mover: u32, bits: u32) -> Result<()> {
        self.c["movers"][mover.to_string()]["factor"] = json!(bits);
        Ok(())
    }
}
fn setup(c: &Value) -> (Payload, Arena, Fixture) {
    let payload = serde_json::from_value(c["payload"].clone()).unwrap();
    let mut a = Arena::default();
    let mut mind = Mind::base(c["owner"].as_u64().unwrap() as u32);
    if c["active"].as_bool().unwrap() {
        mind.stack = VecDeque::from([Some(StateId(1))]);
    }
    a.minds.insert(MindId(1), mind);
    a.states.insert(
        StateId(1),
        State {
            parent: MindId(1),
            discard_on_suspend: false,
            status: c["status"].as_u64().unwrap() as u32,
        },
    );
    (
        payload,
        a,
        Fixture {
            c: c.clone(),
            trace: RefCell::new(vec![]),
            fail: None,
        },
    )
}
fn execute(
    operation: &str,
    p: &mut Payload,
    a: &mut Arena,
    precision: Precision,
    w: &mut Fixture,
) -> Result<()> {
    match operation {
        "think" => think(StateId(1), p, a, precision, w),
        "enter" => enter(StateId(1), p, a, precision, w),
        "leave" => leave(StateId(1), p, a, w),
        _ => panic!(),
    }
}
#[test]
fn original_element_move_think_entry_exit_and_live_rereads() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-element-move/native-element-move.json"
    ))
    .unwrap();
    assert_eq!(data["cases"].as_array().unwrap().len(), 384);
    for row in data["cases"].as_array().unwrap() {
        let c = &row["input"];
        let (mut p, mut a, mut w) = setup(c);
        let precision: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        execute(
            c["operation"].as_str().unwrap(),
            &mut p,
            &mut a,
            precision,
            &mut w,
        )
        .unwrap();
        let movers: serde_json::Map<String, Value> = w.c["movers"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    json!({"requested":v["requested"],"factor":v["factor"]}),
                )
            })
            .collect();
        let result = json!({"payload":p,"status":a.state(StateId(1)).unwrap().status,"owner":a.mind(MindId(1)).unwrap().actor,"time":w.c["time"],"detached_seconds":w.c["detached_seconds"],"diagnostic":w.c["diagnostic"],"movers":movers,"trace":*w.trace.borrow()});
        assert_eq!(result, row["output"], "{}", c["name"]);
    }
}
#[test]
fn required_move_service_failure_keeps_prior_writes_and_skips_later_work() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-element-move/native-element-move.json"
    ))
    .unwrap();
    let c = &data["cases"][0]["input"];
    let (mut p, mut a, mut w) = setup(c);
    w.fail = Some("result_plan");
    assert!(think(StateId(1), &mut p, &mut a, Precision::Double53, &mut w).is_err());
    assert_eq!(a.state(StateId(1)).unwrap().status, 2);
    assert_eq!(p.detached_time_bits, c["time"].as_u64().unwrap() as u32);
    assert_eq!(w.trace.borrow().last().unwrap()["op"], "result_plan");
    let (mut p, mut a, mut w) = setup(c);
    w.c["actors"]["1"]["component74"] = json!(74);
    w.fail = Some("clear_component74");
    assert!(leave(StateId(1), &mut p, &mut a, &mut w).is_err());
    assert_eq!(a.state(StateId(1)).unwrap().status, 0);
    assert_eq!(w.trace.borrow().len(), 1);
}

#[test]
fn original_arrival_thresholds_and_world_edge_float_stores() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-element-move/native-element-move.json"
    ))
    .unwrap();
    assert_eq!(data["leaf_cases"].as_array().unwrap().len(), 288);
    for (i, c) in data["leaf_cases"].as_array().unwrap().iter().enumerate() {
        let precision: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut point = serde_json::from_value(c["point"].clone()).unwrap();
        let actual = if c["operation"] == "near" {
            json!(
                near_point(
                    serde_json::from_value(c["position"].clone()).unwrap(),
                    point,
                    c["threshold"].as_u64().unwrap() as u32,
                    precision
                )
                .unwrap()
            )
        } else {
            clamp_destination(
                &mut point,
                serde_json::from_value(c["extents"].clone()).unwrap(),
                precision,
            )
            .unwrap();
            json!(point)
        };
        assert_eq!(actual, c["result"], "leaf{i}");
    }
}
