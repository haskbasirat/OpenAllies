//! Replays research/phase8-movement/native-follow.json: original x86 52FF00
//! with 52E400/52E300/52CE70/46B000/45D880/577090/576D20 under PC24/53/64.
//! Mind +0x44 accessor events (pure `lea eax,[ecx+off]`) are not callbacks in
//! the Rust API and are filtered; every other event must match in order.
use aa_navigation::formation_follow::{self, FollowElement, FollowLeader, FollowOrganization, FollowWorld};
use aa_navigation::{ActorState, MoveState, OwnedPath, PathNode, Result};
use aa_numeric::Precision;
use serde_json::{Value, json};

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
            .map(|n| {
                let w: Vec<u32> = n.as_array().unwrap().iter().map(u).collect();
                PathNode { words: w.try_into().unwrap() }
            })
            .collect(),
        kind1_count: v["kind1_count"].as_i64().unwrap() as i32,
        kind2_count: v["kind2_count"].as_i64().unwrap() as i32,
        dirty: false,
        word18: 0,
    }
}

struct World {
    responses: Vec<Value>,
    events: Vec<Value>,
}
impl FollowWorld for World {
    fn leader_is_moving(&mut self, _: &mut MoveState, _: &mut ActorState) -> Result<bool> {
        self.events.push(json!({"name": "LeaderMoving", "bits": []}));
        let r = self.responses.remove(0);
        assert_eq!(r["name"], "LeaderMoving");
        Ok(r["value"].as_bool().unwrap())
    }
    fn stop_action(&mut self, _: &mut MoveState, _: &mut ActorState) -> Result<()> {
        self.events.push(json!({"name": "StopAction", "bits": []}));
        Ok(())
    }
    fn schedule(&mut self, delay: u32, _: &mut MoveState, _: &mut ActorState) -> Result<()> {
        self.events.push(json!({"name": "Schedule", "bits": [delay, 0]}));
        Ok(())
    }
    fn finish_state(&mut self, _: &mut MoveState, _: &mut ActorState) -> Result<()> {
        self.events.push(json!({"name": "FinishState", "bits": []}));
        Ok(())
    }
}

#[test]
fn native_formation_follow_speed_matches_original_x86() {
    let path_to = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-movement/native-follow.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path_to).unwrap()).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), doc["case_count"].as_u64().unwrap() as usize);
    let mut faults = 0;
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let p: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let s = &case["state"];
        let mut state = MoveState {
            target_id: 0,
            destination_bits: [0, 0],
            auxiliary_bits: [0, 0],
            requested_speed_bits: u(&s["requested_speed"]),
            phase: 0,
            mode: u(&s["mode"]),
            waypoint_offset_bits: 0,
            path: path(&s["path"]),
        };
        let a = &case["actor"];
        let mut actor = ActorState {
            flags: 0,
            position_bits: pair(&a["position"]),
            mover_instant_turn: false,
            mover_requested_speed_bits: u(&a["mover_requested_speed"]),
            mover_speed_factor_bits: u(&a["mover_speed_factor"]),
            mover_maximum_speed_bits: u(&a["mover_maximum_speed"]),
            has_mover: true,
            organization_special: false,
            has_leader: false,
        };
        let element = (!case["element"].is_null()).then(|| {
            let e = &case["element"];
            FollowElement {
                slot: e["slot"].as_i64().unwrap() as i32,
                organization: (!e["organization"].is_null()).then(|| {
                    let o = &e["organization"];
                    FollowOrganization {
                        position_bits: pair(&o["position"]),
                        facing_bits: pair(&o["facing"]),
                        path: path(&o["path"]),
                        slot_positions_bits: o["slot_positions"].as_array().unwrap().iter().map(pair).collect(),
                        leader: (!o["leader"].is_null()).then(|| FollowLeader {
                            position_bits: pair(&o["leader"]["position"]),
                            path: path(&o["leader"]["path"]),
                        }),
                    }
                }),
            }
        });
        let mut world = World { responses: case["responses"].as_array().unwrap().clone(), events: vec![] };
        let result = formation_follow::formation_speed(&mut state, &mut actor, element.as_ref(), u(&case["catchup_factor"]),
            p, &mut world);
        let out = &case["output"];
        let expected_events: Vec<Value> = out["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["name"] != "OrganizationPath" && e["name"] != "LeaderPath")
            .cloned()
            .collect();
        let context = format!("{name} {p:?}");
        if out.get("null_dereference").is_some() {
            assert!(result.is_err(), "{context}: native null read must be an error");
            faults += 1;
            continue;
        }
        let al = result.unwrap();
        assert_eq!(al as u64, out["return_al"].as_u64().unwrap(), "{context} AL");
        assert_eq!(world.events, expected_events, "{context} events");
        assert!(world.responses.is_empty(), "{context} responses");
        assert_eq!(state.mode, u(&out["state_mode"]), "{context} mode");
        assert_eq!(state.requested_speed_bits, u(&out["state_requested_speed"]), "{context} state speed");
        assert_eq!(actor.mover_requested_speed_bits, u(&out["mover_requested_speed"]), "{context} mover speed");
        assert_eq!(actor.mover_speed_factor_bits, u(&out["mover_speed_factor"]), "{context} factor");
    }
    assert_eq!(faults, 3);
}
