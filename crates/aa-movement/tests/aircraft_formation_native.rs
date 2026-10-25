//! Replays research/phase8-movement/native-aircraft-formation.json: original
//! x86 execution (Unicorn) of the AIRCRAFT formation join `0x55EB30` and
//! teardown `0x55EDC0` (with its follower recursion, the leader getter
//! `0x5685B0` and the inline handle resolution). Every aircraft word of this
//! and the other aircraft, every group reference count, the group virtual
//! calls (+0x14 usable, +0 deleting destructor) in order, and the native
//! faults (null dereference, use after the deleting destructor) must match.
use aa_movement::aircraft::*;
use aa_movement::mover::MoverLimits;
use aa_movement::{ActorMotion, MoverState};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn document() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-movement/native-aircraft-formation.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json")
}
fn u(v: &Value) -> u32 {
    v.as_u64().expect("u32") as u32
}
fn f(v: &Value) -> f32 {
    f32::from_bits(u(v))
}
fn words(v: &Value) -> Vec<u32> {
    v.as_array().expect("array").iter().map(u).collect()
}
fn f3(v: &Value) -> [f32; 3] {
    let w = words(v);
    [f32::from_bits(w[0]), f32::from_bits(w[1]), f32::from_bits(w[2])]
}
fn bits3(v: [f32; 3]) -> Vec<u32> {
    v.map(f32::to_bits).to_vec()
}
fn key(v: &Value) -> Option<u32> {
    Some(u(v)).filter(|&k| k != 0)
}
fn segment(v: &Value) -> PathSegment {
    let c = words(&v["control"]);
    let p = |i: usize| [f32::from_bits(c[3 * i]), f32::from_bits(c[3 * i + 1]), f32::from_bits(c[3 * i + 2])];
    PathSegment {
        control: [p(0), p(1), p(2), p(3)],
        length: f(&v["length"]),
        field_3c: f(&v["field_3c"]),
        end_direction: f3(&v["end_direction"]),
        end_speed: f(&v["end_speed"]),
        start_speed: f(&v["start_speed"]),
        tolerance: f(&v["tolerance"]),
    }
}
fn segment_json(s: &PathSegment) -> Value {
    let control: Vec<u32> = s.control.iter().flat_map(|p| bits3(*p)).collect();
    json!({"control": control, "length": s.length.to_bits(), "field_3c": s.field_3c.to_bits(),
        "end_direction": bits3(s.end_direction), "end_speed": s.end_speed.to_bits(),
        "start_speed": s.start_speed.to_bits(), "tolerance": s.tolerance.to_bits()})
}
fn aircraft(v: &Value) -> AircraftState {
    let g = |i: usize| key(&v["groups"][i]).map(GroupKey);
    let p = &v["path"];
    let basis = words(&p["basis"]);
    AircraftState {
        on_ground: u(&v["on_ground"]) as u8,
        cruise_speed: f(&v["cruise_speed"]),
        cruise_altitude: 0.0,
        path: AircraftPath {
            planning_speed: f(&p["planning_speed"]),
            turn_radius: f(&p["turn_radius"]),
            segments: p["segments"].as_array().unwrap().iter().map(segment).collect(),
            t: f(&p["t"]),
            basis: std::array::from_fn(|i| f32::from_bits(basis[i])),
            altitude: f(&p["altitude"]),
            leg_speed: f(&p["leg_speed"]),
            total_length: f(&p["total_length"]),
            speed: f(&p["speed"]),
        },
        circling: u(&v["circling"]) as u8,
        attack_target: u(&v["attack_target"]),
        attack_point: f3(&v["attack_point"]),
        pending_ability: key(&v["ability"]).map(AbilityKey),
        group_a: g(0),
        group_b: g(1),
        group_c: g(2),
    }
}
fn aircraft_json(a: &AircraftState) -> Value {
    let p = &a.path;
    let g = |k: Option<GroupKey>| k.map_or(0, |k| k.0);
    json!({
        "on_ground": a.on_ground, "cruise_speed": a.cruise_speed.to_bits(), "circling": a.circling,
        "attack_target": a.attack_target, "attack_point": bits3(a.attack_point),
        "ability": a.pending_ability.map_or(0, |k| k.0), "groups": [g(a.group_a), g(a.group_b), g(a.group_c)],
        "path": {"planning_speed": p.planning_speed.to_bits(), "turn_radius": p.turn_radius.to_bits(),
            "t": p.t.to_bits(), "basis": p.basis.map(f32::to_bits).to_vec(), "altitude": p.altitude.to_bits(),
            "leg_speed": p.leg_speed.to_bits(), "total_length": p.total_length.to_bits(),
            "speed": p.speed.to_bits(), "segments": p.segments.iter().map(segment_json).collect::<Vec<_>>()},
    })
}

struct Other {
    flags: u32,
    aircraft: Option<AircraftState>,
}
struct Group {
    members: Vec<u32>,
    unusable: bool,
    refs: u32,
    deleted: bool,
}
struct World {
    others: BTreeMap<u32, Other>,
    groups: BTreeMap<u32, Group>,
    handles: BTreeMap<u16, HandleSlot>,
    speeds: BTreeMap<u32, u32>,
    events: Vec<Value>,
}
impl World {
    fn new(case: &Value) -> Self {
        let others = case["others"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| {
                let aircraft = (!o["aircraft"].is_null()).then(|| aircraft(&o["aircraft"]));
                (u(&o["key"]), Other { flags: u(&o["flags"]), aircraft })
            })
            .collect();
        let groups = case["groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| {
                (u(&g["key"]), Group {
                    members: words(&g["members"]),
                    unusable: g["disbanded"].as_bool().unwrap(),
                    refs: u(&g["refs"]),
                    deleted: false,
                })
            })
            .collect();
        let handles = case["handles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| (u(&h[0]) as u16, HandleSlot { generation: u(&h[1]) as u16, actor: key(&h[2]).map(ActorKey) }))
            .collect();
        let speeds = case["definition_speeds"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.parse().unwrap(), u(v)))
            .collect();
        World { others, groups, handles, speeds, events: vec![] }
    }
    fn record(&mut self, name: &str, bits: &[u32]) {
        self.events.push(json!({"name": name, "bits": bits}));
    }
    fn group(&mut self, group: GroupKey) -> &mut Group {
        let g = self.groups.get_mut(&group.0).expect("group");
        assert!(!g.deleted, "the port touched a deleted group");
        g
    }
}
impl FormationWorld for World {
    fn handle_slot(&mut self, index: u16) -> AircraftResult<HandleSlot> {
        Ok(self.handles.get(&index).copied().unwrap_or(HandleSlot { generation: 0, actor: None }))
    }
    fn actor_handle(&mut self, _: ActorKey) -> AircraftResult<u32> {
        unreachable!("not read by the join/teardown")
    }
    fn actor_flags(&mut self, actor: ActorKey) -> AircraftResult<u32> {
        Ok(self.others[&actor.0].flags)
    }
    fn group_members(&mut self, group: GroupKey) -> AircraftResult<Vec<u32>> {
        Ok(self.group(group).members.clone())
    }
    fn group_layout(&mut self, _: GroupKey) -> AircraftResult<GroupLayout> {
        unreachable!("not read by the join/teardown")
    }
    fn member_aircraft(&mut self, actor: ActorKey) -> AircraftResult<&mut AircraftState> {
        self.others
            .get_mut(&actor.0)
            .and_then(|o| o.aircraft.as_mut())
            .ok_or(AircraftError::NullDereference("member actor+0xBC"))
    }
}
impl GroupWorld for World {
    fn group_unusable(&mut self, group: GroupKey) -> AircraftResult<bool> {
        self.record("GroupDisbanded", &[group.0]);
        Ok(self.group(group).unusable)
    }
    fn retain_group(&mut self, group: GroupKey) -> AircraftResult<()> {
        self.group(group).refs += 1;
        Ok(())
    }
    fn release_group(&mut self, group: GroupKey) -> AircraftResult<bool> {
        let g = self.group(group);
        g.refs = g.refs.wrapping_sub(1);
        if g.refs != 0 {
            return Ok(false);
        }
        g.deleted = true;
        self.record("GroupDelete", &[group.0]);
        Ok(true)
    }
    fn definition_speed(&mut self, actor: ActorKey) -> AircraftResult<f32> {
        Ok(f32::from_bits(self.speeds[&actor.0]))
    }
}

fn unit(case: &Value) -> AircraftUnit {
    let a = &case["actor"];
    let d = &a["definition"];
    AircraftUnit {
        actor: AircraftActor {
            key: ActorKey(1),
            handle: u(&a["handle"]),
            scale: f(&a["scale"]),
            definition: AircraftDefinition {
                taxi_scale: f(&d["taxi_scale"]),
                taxi_height: f(&d["taxi_height"]),
                operational_height: f(&d["operational_height"]),
            },
        },
        motion: ActorMotion {
            position: f3(&a["position"]),
            facing: f3(&a["facing"]),
            flags: u(&a["flags"]),
            has_mind: a["has_mind"].as_bool().unwrap(),
            has_character: false,
            character_stopped: false,
            terrain_speed_limit: 0.0,
        },
        mover: MoverState {
            destination: [0.0; 3],
            last_step_speed: 0.0,
            speed: 0.0,
            requested_speed: 0.0,
            acceleration: 0.0,
            turn_rate: 0.0,
            desired_direction: [0.0; 3],
            target_id: 0,
            target_range: None,
            ignore_terrain_speed_limit: false,
            speed_factor: 0.0,
            instant_turn: false,
            turning: false,
            falling: false,
        },
        mover_limits: MoverLimits { max_speed: 0.0, turn_radius: 0.0 },
        aircraft: aircraft(&case["aircraft"]),
    }
}

#[test]
fn formation_join_and_teardown_match_original_x86() {
    let doc = document();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 300, "fixture too small");
    let (mut faults, mut deletes) = (0, 0);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let mut world = World::new(case);
        let mut unit = unit(case);
        let mut value = None;
        let result = match case["operation"].as_str().unwrap() {
            "leave" => leave_formation(&mut unit, &mut world),
            "join" => join_formation(&mut unit, &mut world, key(&case["args"]["group"]).map(GroupKey)),
            "follower" => following_leader(&mut unit, &mut world).map(|v| value = Some(u32::from(v))),
            other => panic!("{other}"),
        };
        let out = &case["output"];
        if let Some(v) = value {
            assert_eq!(v, u(&out["value"]), "{name}: 0x55E660");
        }
        assert_eq!(json!(world.events), out["events"], "{name}: group virtual calls");
        deletes += world.events.iter().filter(|e| e["name"] == "GroupDelete").count();
        if let Some(error) = out.get("error").and_then(Value::as_str) {
            faults += 1;
            let got = result.expect_err(name);
            match error.split(':').next().unwrap() {
                "NullDereference" => assert!(matches!(got, AircraftError::NullDereference(_)), "{name}: {got}"),
                "UseAfterFree" => {
                    assert!(got.to_string().contains("after its deleting destructor"), "{name}: {got}")
                }
                other => panic!("{name}: {other}"),
            }
            continue;
        }
        result.unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(aircraft_json(&unit.aircraft), out["aircraft"], "{name}: aircraft");
        for (k, expected) in out["others"].as_object().unwrap() {
            let got = world.others[&k.parse::<u32>().unwrap()].aircraft.as_ref().unwrap();
            assert_eq!(aircraft_json(got), *expected, "{name}: aircraft of actor {k}");
        }
        let refs: BTreeMap<String, u32> =
            world.groups.iter().filter(|(_, g)| !g.deleted).map(|(k, g)| (k.to_string(), g.refs)).collect();
        assert_eq!(json!(refs), out["refs"], "{name}: reference counts");
    }
    assert!(faults > 0 && deletes > 0, "fixture lost its fault or destructor cases");
}

#[test]
fn constructor_matches_the_native_field_values() {
    // 0x55E8E0: on the ground, zero words, the 0xB281D8 basis, the definition
    // speed and turn radius in the path.
    let state = AircraftState::new(f32::from_bits(0x41a0_0000), f32::from_bits(0x4100_0000));
    assert_eq!(state.byte(0x10), Some(1));
    assert_eq!(state.byte(0x90), Some(0));
    for offset in [0x14, 0x18, 0x34, 0x3c, 0x80, 0x84, 0x88, 0x8c, 0x94] {
        assert_eq!(state.word(offset), Some(0), "{offset:#x}");
    }
    assert_eq!(state.word(0x2c), Some(0x41a0_0000));
    assert_eq!(state.word(0x30), Some(0x4100_0000));
    assert_eq!(state.path.basis.map(f32::to_bits), BEZIER_BASIS_BITS);
    assert!(state.group_a.is_none() && state.group_b.is_none() && state.group_c.is_none());
    // fld/fstp quiets a signaling NaN; the turn radius is a plain word copy.
    let quiet = AircraftState::new(f32::from_bits(0x7f80_0001), f32::from_bits(0x7f80_0001));
    assert_eq!(quiet.word(0x2c), Some(0x7fc0_0001));
    assert_eq!(quiet.word(0x30), Some(0x7f80_0001));
}

#[test]
fn follower_recursion_into_an_aircraft_already_leaving_is_unbounded() {
    // Natively 0x55EDC0 recurses into the same aircraft forever when it is a
    // live member of its own usable follower group.
    let doc = document();
    let mut case = doc["cases"][0].clone();
    case["aircraft"]["groups"] = json!([0, 0, 1]);
    case["groups"][0]["disbanded"] = json!(false);
    case["groups"][0]["members"] = json!([0x0003_0001]);
    case["actor"]["flags"] = json!(0);
    let mut world = World::new(&case);
    let mut unit = unit(&case);
    assert!(matches!(leave_formation(&mut unit, &mut world), Err(AircraftError::UnboundedRecursion)));
}
