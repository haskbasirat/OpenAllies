//! Replays research/phase8-movement/native-company.json (original x86 under
//! PC24/53/64): 445280, 52CBD0, 52D230, 5525D0, 51E9B0/51D840, 520320,
//! 520440, 523080/5205E0, 51D7B0, 526940, 52E9D0/52E900.
use aa_navigation::company_move::{self, CompanyMoveState, CompanyWorld, Extents, SlotReturnWorld, SlotTarget};
use aa_navigation::{ActorState, MoveState, MoverRequest, NavigationWorld, OwnedPath, PathNode, Result};
use aa_numeric::Precision;
use serde_json::{Value, json};

#[test]
fn actual_regiment_command_reads_location_activation_result(){
    let doc:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-player-order-bank/native-company-result.json")).unwrap();
    let mut count=0;
    for row in doc["rows"].as_array().unwrap(){
        let case=&row["input"];let expected=&row["output"];
        let mut fields=state(&case["state"]);
        let mut world=World{case:case.clone(),responses:case["responses"].as_array().unwrap().clone(),events:vec![],mind_auxiliary:[0;2],move_status:0};
        let args:Vec<u32>=case["args"].as_array().unwrap().iter().map(u).collect();
        let extents=Extents{extent_bits:case.get("extent").map(pair).unwrap_or([256f32.to_bits(),192f32.to_bits()])};
        let result=company_move::activate_to_location_result(&mut fields,2,[args[0],args[1]],args[2],[args[3],args[4]],extents,
            serde_json::from_value(case["precision"].clone()).unwrap(),&mut world).unwrap();
        assert_eq!(result as u64,expected["al"].as_u64().unwrap());
        assert!(world.responses.is_empty());
        assert_eq!(fields.destination_bits,pair(&expected["state"]["destination_bits"]));
        assert_eq!(fields.path_destination_bits,pair(&expected["state"]["path_destination_bits"]));
        assert_eq!(fields.auxiliary_bits,pair(&expected["state"]["auxiliary_bits"]));
        assert_eq!(fields.speed_bits,u(&expected["state"]["speed_bits"]));count+=1;
    }
    assert_eq!(count,24);
}

fn u(v: &Value) -> u32 {
    v.as_u64().expect("u32") as u32
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}
fn path(v: &Value) -> OwnedPath {
    OwnedPath {
        nodes: v["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| PathNode { words: n.as_array().unwrap().iter().map(u).collect::<Vec<_>>().try_into().unwrap() })
            .collect(),
        kind1_count: v["kind1_count"].as_i64().unwrap() as i32,
        kind2_count: v["kind2_count"].as_i64().unwrap() as i32,
        dirty: v.get("dirty").and_then(Value::as_bool).unwrap_or(false),
        word18: v.get("word18").and_then(Value::as_u64).unwrap_or(0) as u32,
    }
}
fn path_json(p: &OwnedPath) -> Value {
    json!({"nodes": p.nodes.iter().map(|n| n.words.to_vec()).collect::<Vec<_>>(), "kind1_count": p.kind1_count,
           "kind2_count": p.kind2_count, "dirty": p.dirty, "word18": p.word18})
}
fn state(v: &Value) -> CompanyMoveState {
    CompanyMoveState {
        status: u(&v["status"]),
        target_id: u(&v["target_id"]),
        path_destination_bits: pair(&v["path_destination_bits"]),
        destination_bits: pair(&v["destination_bits"]),
        auxiliary_bits: pair(&v["auxiliary_bits"]),
        speed_bits: u(&v["speed_bits"]),
        tries: v["tries"].as_i64().unwrap() as i32,
        path: path(&v["path"]),
        formation_word: u(&v["formation_word"]),
        start_time_bits: u(&v["start_time_bits"]),
    }
}

struct World {
    case: Value,
    responses: Vec<Value>,
    events: Vec<Value>,
    mind_auxiliary: [u32; 2],
    move_status: u32,
}
impl World {
    fn record(&mut self, name: &str, bits: &[u32]) {
        self.events.push(json!({"name": name, "bits": bits}));
    }
    fn response(&mut self, name: &str) -> Value {
        let r = self.responses.remove(0);
        assert_eq!(r["name"], name);
        r
    }
}
impl CompanyWorld for World {
    fn can_activate(&mut self, mode: u32, _: &mut CompanyMoveState) -> Result<bool> {
        if mode < 2 {
            return Ok(true);
        }
        self.record("CanActivate", &[]);
        Ok(self.response("CanActivate")["value"].as_bool().unwrap())
    }
    fn activate(&mut self, mode: u32, _: &mut CompanyMoveState) -> Result<bool> {
        self.record("Activate", &[mode]);
        Ok(self.response("Activate")["value"].as_bool().unwrap())
    }
    fn group_offset(&mut self, _: &mut CompanyMoveState) -> Result<Option<[u32; 2]>> {
        if !self.case["actor"]["group_valid"].as_bool().unwrap() {
            return Ok(None);
        }
        self.record("GroupActive", &[]);
        if !self.response("GroupActive")["value"].as_bool().unwrap() {
            return Ok(None);
        }
        self.record("GroupOffset", &[]);
        Ok(Some(pair(&self.response("GroupOffset")["offset"])))
    }
    fn actor_position(&mut self, id: u32) -> Result<Option<[u32; 2]>> {
        Ok(self.case["targets"]
            .as_array()
            .and_then(|t| t.iter().find(|t| u(&t["id"]) == id && id != 0))
            .map(|t| pair(&t["position"])))
    }
    fn set_mind_auxiliary(&mut self, auxiliary: [u32; 2]) -> Result<()> {
        self.mind_auxiliary = auxiliary;
        Ok(())
    }
    fn organization_speed_cap(&mut self) -> Result<u32> {
        Ok(u(&self.case["actor"]["speed_cap"]))
    }
    fn member_slots(&mut self) -> Result<Vec<usize>> {
        Ok(self.case["actor"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.is_null())
            .map(|(i, _)| i)
            .collect())
    }
    fn member_accepts_speed(&mut self, slot: usize) -> Result<bool> {
        self.record("MemberAcceptsSpeed", &[slot as u32]);
        Ok(self.response("MemberAcceptsSpeed")["value"].as_bool().unwrap())
    }
    fn member_set_speed(&mut self, slot: usize, speed_bits: u32) -> Result<()> {
        self.record("MemberSetSpeed", &[slot as u32, speed_bits]);
        Ok(())
    }
    fn world_time(&mut self) -> Result<u32> {
        Ok(self.case.get("world_time").map(u).unwrap_or(50f32.to_bits()))
    }
    fn schedule(&mut self, delay_bits: u32) -> Result<()> {
        self.record("Schedule", &[delay_bits, 0]);
        Ok(())
    }
    fn move_failed(&mut self) -> Result<()> {
        self.record("MoveFailed", &[]);
        Ok(())
    }
}
impl NavigationWorld for World {
    fn special_state_active(&mut self, _: &mut MoveState, _: &mut ActorState) -> bool {
        unreachable!()
    }
    fn can_activate(&mut self, _: &mut MoveState, _: &mut ActorState) -> bool {
        unreachable!("mode 0 activations never consult eligibility")
    }
    fn activate(&mut self, mode: u32, _: &mut MoveState, _: &mut ActorState) -> bool {
        self.record("Activate", &[mode]);
        self.response("Activate")["value"].as_bool().unwrap()
    }
    fn node_reached(&mut self, _: &mut MoveState, _: &mut ActorState) {
        unreachable!()
    }
    fn single_node_placement(&mut self, _: [u32; 2], _: &mut MoveState, _: &mut ActorState) -> bool {
        unreachable!()
    }
    fn actor_is_moving(&mut self, _: &mut MoveState, _: &mut ActorState) -> bool {
        unreachable!()
    }
    fn formation_ready(&mut self, _: &mut MoveState, _: &mut ActorState) -> bool {
        unreachable!()
    }
    fn actor_or_leader_handle_valid(&mut self, _: &mut MoveState, _: &mut ActorState) -> bool {
        unreachable!()
    }
    fn organization_ready(&mut self, _: &mut MoveState, _: &mut ActorState) -> bool {
        unreachable!()
    }
    fn leader_is_moving(&mut self, _: &mut MoveState, _: &mut ActorState) -> bool {
        unreachable!()
    }
    fn stop_action(&mut self, _: &mut MoveState, _: &mut ActorState) {
        unreachable!()
    }
    fn begin_mover(&mut self, _: MoverRequest, _: &mut MoveState, _: &mut ActorState) -> bool {
        unreachable!()
    }
    fn next_world_random_u32(&mut self, _: &mut MoveState, _: &mut ActorState) -> u32 {
        unreachable!()
    }
    fn schedule(&mut self, _: u32, _: &mut MoveState, _: &mut ActorState) {
        unreachable!()
    }
}
impl SlotReturnWorld for World {
    fn leave_move_state(&mut self, _: &mut MoveState, _: &mut ActorState) -> Result<()> {
        if self.move_status != 0 {
            self.record("MoveStateLeave", &[]);
        }
        Ok(())
    }
    fn move_state_think(&mut self, _: &mut MoveState, _: &mut ActorState) -> Result<()> {
        self.record("MoveStateThink", &[]);
        Ok(())
    }
    fn mind_virtual_180(&mut self, _: &mut MoveState, _: &mut ActorState) -> Result<()> {
        self.record("MindVirtual180", &[]);
        Ok(())
    }
}

fn actor() -> ActorState {
    ActorState {
        flags: 0,
        position_bits: [0, 0],
        mover_instant_turn: false,
        mover_requested_speed_bits: 0,
        mover_speed_factor_bits: 0,
        mover_maximum_speed_bits: 0,
        has_mover: true,
        organization_special: false,
        has_leader: false,
    }
}

#[test]
fn native_company_move_helpers_match_original_x86() {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-movement/native-company.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), doc["case_count"].as_u64().unwrap() as usize);
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let op = case["operation"].as_str().unwrap();
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let out = &case["output"];
        let context = format!("{name} {p:?}");
        let extents = Extents {
            extent_bits: case.get("extent").map(pair).unwrap_or([256f32.to_bits(), 192f32.to_bits()]),
        };
        let mut world = World {
            case: case.clone(),
            responses: case["responses"].as_array().cloned().unwrap_or_default(),
            events: vec![],
            mind_auxiliary: [0, 0],
            move_status: 0,
        };
        let filtered: Vec<Value> = out["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["name"] != "GroupLookup")
            .cloned()
            .collect();
        match op {
            "intersection" => {
                let pts: Vec<[u32; 2]> = case["points"].as_array().unwrap().iter().map(pair).collect();
                let r = company_move::line_intersection(pts[0], pts[1], pts[2], pts[3], p).unwrap();
                assert_eq!(r.is_some() as u64, out["result"].as_u64().unwrap(), "{context}");
                if let Some(point) = r {
                    assert_eq!(point, pair(&out["point"]), "{context}");
                }
            }
            "slot_path" | "translate_path" => {
                let source = path(&case["source"]);
                let mut target = path(&case["target"]);
                if op == "slot_path" {
                    target.assign_from(&source).unwrap();
                    company_move::offset_path_for_slot(&mut target, pair(&case["slot_offset"]), pair(&case["facing"]),
                        &source, p).unwrap();
                } else {
                    company_move::translate_path(&mut target, &source, pair(&case["slot_offset"]), p).unwrap();
                }
                assert_eq!(path_json(&target), out["path"], "{context}");
            }
            "clamp" => {
                let mut point = pair(&case["point"]);
                company_move::clamp_to_extent(extents, &mut point, p).unwrap();
                assert_eq!(point, pair(&out["point"]), "{context}");
            }
            "return_to_slot" => {
                let m = &case["move"];
                world.move_status = u(&m["status"]);
                let mut ms = MoveState {
                    target_id: 0,
                    destination_bits: pair(&m["destination_bits"]),
                    auxiliary_bits: [0, 0],
                    requested_speed_bits: u(&m["requested_speed_bits"]),
                    phase: 0,
                    mode: u(&m["mode"]),
                    waypoint_offset_bits: 0,
                    path: OwnedPath { nodes: vec![], kind1_count: 0, kind2_count: 0, dirty: false, word18: 0 },
                };
                let target = (!case["organization"].is_null()).then(|| SlotTarget {
                    center_bits: pair(&case["organization"]["center"]),
                    slot_offset_bits: pair(&case["organization"]["slot_offset"]),
                });
                company_move::return_to_slot(&mut ms, &mut actor(), world.move_status, target, p, &mut world).unwrap();
                let e = &out["move"];
                assert_eq!(ms.destination_bits, pair(&e["destination_bits"]), "{context}");
                assert_eq!(ms.mode, u(&e["mode"]), "{context}");
                assert_eq!(ms.requested_speed_bits, u(&e["requested_speed_bits"]), "{context}");
                assert_eq!(ms.auxiliary_bits, pair(&e["auxiliary_bits"]), "{context}");
                assert_eq!(ms.phase, u(&e["phase"]), "{context}");
            }
            _ => {
                let mut s = state(&case["state"]);
                let al = match op {
                    "set_destination" => {
                        let args: Vec<u32> = case["args"].as_array().unwrap().iter().map(u).collect();
                        company_move::set_destination(&mut s, [args[0], args[1]], extents, p, &mut world).unwrap();
                        None
                    }
                    "activate_actor" => {
                        let args: Vec<u32> = case["args"].as_array().unwrap().iter().map(u).collect();
                        let target = (!case["target"].is_null())
                            .then(|| (u(&case["target"]["id"]), pair(&case["target"]["position"])));
                        Some(company_move::activate_to_actor(&mut s, u(&case["mode"]), target, args[0],
                            [args[1], args[2]], extents, p, &mut world).unwrap())
                    }
                    "activate_location" => {
                        let args: Vec<u32> = case["args"].as_array().unwrap().iter().map(u).collect();
                        company_move::activate_to_location(&mut s, u(&case["mode"]), [args[0], args[1]], args[2],
                            [args[3], args[4]], extents, p, &mut world).unwrap();
                        None
                    }
                    "enter" => {
                        company_move::enter(&mut s, extents, p, &mut world).unwrap();
                        None
                    }
                    "leave" => {
                        company_move::leave(&mut s);
                        None
                    }
                    "tries" => Some(company_move::increment_tries(&mut s, u(&case["min_seconds"]), p, &mut world).unwrap()),
                    other => panic!("{other}"),
                };
                if let Some(al) = al {
                    assert_eq!(al as u64, out["al"].as_u64().unwrap(), "{context} AL");
                }
                let e = &out["state"];
                assert_eq!(s.status, u(&e["status"]), "{context} status");
                assert_eq!(s.target_id, u(&e["target_id"]), "{context}");
                assert_eq!(s.path_destination_bits, pair(&e["path_destination_bits"]), "{context} goal");
                assert_eq!(s.destination_bits, pair(&e["destination_bits"]), "{context} destination");
                assert_eq!(s.auxiliary_bits, pair(&e["auxiliary_bits"]), "{context}");
                assert_eq!(s.speed_bits, u(&e["speed_bits"]), "{context} speed");
                assert_eq!(s.tries as i64, e["tries"].as_i64().unwrap(), "{context}");
                assert_eq!(path_json(&s.path), e["path"], "{context} path");
                assert_eq!(s.formation_word, u(&e["formation_word"]), "{context}");
                assert_eq!(s.start_time_bits, u(&e["start_time_bits"]), "{context}");
                if op == "enter" {
                    assert_eq!(world.mind_auxiliary, pair(&e["mind_auxiliary"]), "{context}");
                }
            }
        }
        assert_eq!(Value::Array(world.events), Value::Array(filtered), "{context} events");
        assert!(world.responses.is_empty(), "{context} responses");
        *counts.entry(op.to_string()).or_default() += 1;
    }
    eprintln!("{counts:?}");
}
