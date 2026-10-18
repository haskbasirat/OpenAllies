//! Replays research/phase8-movement/native-formation.json (original x86
//! 5797E0/578250/444A90/45D400, 57BEF0, 57BA80 and 579A00 under PC24/53/64)
//! and checks the embedded direction table against its native producer record.
use aa_movement::formation::{self, FormationShape, FormationWorld, Member, Organization};
use aa_movement::Result;
use aa_numeric::Precision;
use serde_json::{Value, json};

fn load(name: &str) -> Value {
    let path = format!("{}/../../research/phase8-movement/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json")
}
fn u(v: &Value) -> u32 {
    v.as_u64().expect("u32") as u32
}
fn pair(v: &Value) -> [f32; 2] {
    [f32::from_bits(u(&v[0])), f32::from_bits(u(&v[1]))]
}
fn member(v: &Value) -> Option<Member> {
    if v.is_null() {
        return None;
    }
    Some(Member {
        id: u(&v["id"]),
        position: pair(&v["position"]),
        slot: u(&v["slot"]) as i32,
        attached: u(&v["attached"]) != 0,
    })
}
fn organization(o: &Value) -> Organization {
    Organization {
        owner_id: u(&o["owner_id"]),
        facing: pair(&o["facing"]),
        owner_has_mind: o["owner_has_mind"].as_bool().unwrap(),
        center: pair(&o["center"]),
        primary_slot: o["primary_slot"].as_i64().unwrap() as i32,
        group_sizes: o["groups"].as_array().unwrap().iter().map(|g| g.as_i64().unwrap() as i32).collect(),
        slot_count: u(&o["slot_count"]) as i32,
        members: o["members"].as_array().unwrap().iter().map(member).collect(),
        leader: member(&o["leader"]),
        formation: (!o["formation"].is_null()).then(|| FormationShape {
            oriented: u(&o["formation"]["oriented"]) != 0,
            offsets: o["formation"]["offsets"].as_array().unwrap().iter().map(pair).collect(),
        }),
        slot_positions: o["slot_positions"].as_array().unwrap().iter().map(pair).collect(),
    }
}

struct World {
    draws: Vec<u32>,
    events: Vec<Value>,
}
impl FormationWorld for World {
    fn next_random_u32(&mut self) -> Result<u32> {
        let value = self.draws.remove(0);
        self.events.push(json!({"name": "Random", "bits": [value]}));
        Ok(value)
    }
    fn mind_leader_changed(&mut self) -> Result<()> {
        self.events.push(json!({"name": "MindLeaderChanged", "bits": []}));
        Ok(())
    }
    fn detached_member(&mut self, organization_id: u32, member_id: u32, offset: [f32; 2]) -> Result<()> {
        self.events.push(json!({"name": "DetachedDiagnostic",
            "bits": [organization_id, member_id, offset[0].to_bits(), offset[1].to_bits()]}));
        Ok(())
    }
}

#[test]
fn direction_table_matches_native_pc53_static_initializer() {
    let doc = load("native-direction-table.json");
    assert_eq!(doc["object"]["count"], 2048);
    assert_eq!(doc["identical_across_precision"]["Double53"], true);
    let raw = std::fs::read(format!(
        "{}/../../research/phase8-movement/direction-table.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    for index in 0..2048u32 {
        let entry = formation::direction(index).unwrap();
        let at = index as usize * 8;
        assert_eq!(entry[0].to_bits(), u32::from_le_bytes(raw[at..at + 4].try_into().unwrap()));
        assert_eq!(entry[1].to_bits(), u32::from_le_bytes(raw[at + 4..at + 8].try_into().unwrap()));
    }
    assert_eq!(formation::direction(0).unwrap(), [1.0, 0.0]);
    assert!(formation::direction(2048).is_none());
}

#[test]
fn native_formation_operations_match_original_x86() {
    let doc = load("native-formation.json");
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), doc["case_count"].as_u64().unwrap() as usize);
    let mut swapped = 0;
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let pc: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let mut org = organization(&case["organization"]);
        let before = org.slot_positions.clone();
        let mut world = World { draws: case["draws"].as_array().unwrap().iter().map(u).collect(), events: vec![] };
        let result = match case["operation"].as_str().unwrap() {
            "compute" => formation::compute_slot_position(&mut org, case["slot"].as_i64().unwrap() as i32, pc,
                &mut world).map(|_| ()),
            "compute_all" => formation::compute_all_slot_positions(&mut org, pc, &mut world).map(|_| ()),
            "reassign" => formation::reassign_slots(&mut org, pc),
            "set_leader" => formation::set_leader(&mut org, member(&case["new_leader"]), pc, &mut world),
            other => panic!("{other}"),
        };
        if case["output"].get("null_dereference").is_some() {
            // The original faults on a null actor read; Rust stops with an error.
            assert!(result.is_err(), "{name} must reject the native null read");
            assert_eq!(Value::Array(world.events), case["output"]["events"], "{name} events before fault");
            continue;
        }
        result.unwrap();
        let out = &case["output"];
        let context = format!("{name} {pc:?}");
        let expected: Vec<[u32; 2]> =
            out["slot_positions"].as_array().unwrap().iter().map(|p| [u(&p[0]), u(&p[1])]).collect();
        let actual: Vec<[u32; 2]> = org.slot_positions.iter().map(|p| p.map(f32::to_bits)).collect();
        assert_eq!(actual, expected, "{context} positions");
        assert_eq!(org.leader.as_ref().map(|l| l.id as u64), out["leader"].as_u64(), "{context} leader");
        assert_eq!(Value::Array(world.events), out["events"], "{context} events");
        assert!(world.draws.is_empty(), "{context} draws");
        if case["operation"] == "reassign" && org.slot_positions != before {
            swapped += 1;
        }
    }
    assert!(swapped > 100, "reassignment coverage {swapped}");
}

#[test]
fn slot_world_position_adds_center() {
    let doc = load("native-formation.json");
    let org = organization(&doc["cases"][0]["organization"]);
    let p = formation::slot_world_position(&org, 2, Precision::Double53).unwrap();
    assert_eq!(p[0], org.center[0] + org.slot_positions[2][0]);
    assert_eq!(p[1], org.center[1] + org.slot_positions[2][1]);
}
