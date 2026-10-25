//! Replays research/phase8-movement/native-aircraft-flight.json: original x86
//! execution (Unicorn) of the AIRCRAFT flight entry points `0x560DB0`
//! (fly_to), `0x560ED0` (replace_path), `0x560F10` (fly_formation) and the
//! formation follow planner `0x560A70`, with everything below them unchanged
//! (teardown `0x55EDC0`, map clamp `0x552640`, slot point `0x517DF0`, planner
//! `0x5601E0`, leg builder `0x55F230`, member reset `0x55EBE0`), plus
//! `0x517DF0` and `0x55E7E0` alone. The return value, the group virtual calls
//! and terrain queries in order, the reference counts and every word of this
//! and the other aircraft (paths and segments) must match.
use aa_movement::aircraft::*;
use aa_movement::falling::MapBounds;
use aa_movement::mover::MoverLimits;
use aa_movement::{ActorMotion, MoverState};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};

fn document() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-movement/native-aircraft-flight.json");
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
fn pose(v: &Value) -> Pose {
    let w = words(v);
    let g = |i: usize| f32::from_bits(w[i]);
    Pose { position: [g(0), g(1), g(2)], direction: [g(3), g(4), g(5)], speed: g(6) }
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
    heights: VecDeque<Value>,
    map: MapBounds,
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
        let speeds =
            case["definition_speeds"].as_object().unwrap().iter().map(|(k, v)| (k.parse().unwrap(), u(v))).collect();
        let heights =
            case["responses"]["GroundHeight"].as_array().map_or(VecDeque::new(), |h| h.iter().cloned().collect());
        let m = words(&case["map"]);
        let map = MapBounds { cell_size: f32::from_bits(m[0]), extent: [f32::from_bits(m[1]), f32::from_bits(m[2])] };
        World { others, groups, handles, speeds, heights, map, events: vec![] }
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
        unreachable!("not read by the flight entry points")
    }
    fn actor_flags(&mut self, actor: ActorKey) -> AircraftResult<u32> {
        Ok(self.others[&actor.0].flags)
    }
    fn group_members(&mut self, group: GroupKey) -> AircraftResult<Vec<u32>> {
        Ok(self.group(group).members.clone())
    }
    fn group_layout(&mut self, _: GroupKey) -> AircraftResult<GroupLayout> {
        unreachable!("not read by the flight entry points")
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
impl TerrainWorld for World {
    fn ground_height(&mut self, xy: [f32; 2], flags: u32) -> AircraftResult<Binary> {
        self.record("GroundHeight", &[xy[0].to_bits(), xy[1].to_bits(), flags]);
        let r = self.heights.pop_front().expect("unexpected GroundHeight");
        let text = r["f64"].as_str().unwrap();
        Ok(Binary::from_f64_bits(u64::from_str_radix(text.trim_start_matches("0x"), 16).unwrap()).unwrap())
    }
}
impl FlightWorld for World {
    fn map_bounds(&mut self) -> AircraftResult<MapBounds> {
        Ok(self.map)
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
fn flight_entry_points_match_original_x86() {
    let doc = document();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 400, "fixture too small");
    let mut counts = BTreeMap::<String, usize>::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let op = case["operation"].as_str().unwrap();
        *counts.entry(op.to_string()).or_default() += 1;
        let pc: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let a = &case["args"];
        let mut world = World::new(case);
        let mut unit = unit(case);
        let direction = (!a["direction"].is_null() && a.get("direction").is_some()).then(|| f3(&a["direction"]));
        let out = &case["output"];
        let mut point = None;
        let result: AircraftResult<Option<bool>> = match op {
            "fly_to" => fly_to(&mut unit, f3(&a["point"]), direction, pc, &mut world).map(Some),
            "replace_path" => replace_path(&mut unit, f3(&a["point"]), direction, pc, &mut world).map(Some),
            "fly_formation" => {
                fly_formation(&mut unit, f3(&a["offset"]), f3(&a["goal"]), direction, pc, &mut world).map(|()| None)
            }
            "follow_path" => {
                let leader = world.others[&u(&a["leader"])].aircraft.as_ref().unwrap().path.clone();
                follow_path(&mut unit.aircraft.path, &pose(&a["pose"]), &leader, f3(&a["offset"]), pc, &mut world)
                    .map(Some)
            }
            "slot_point" => {
                let facing = words(&a["facing"]).into_iter().map(f32::from_bits).collect::<Vec<_>>();
                point = Some(
                    slot_point(f3(&a["position"]), [facing[0], facing[1]], f3(&a["offset"]), pc)
                        .unwrap_or_else(|e| panic!("{name}: {e}")),
                );
                Ok(None)
            }
            "set_path_speed" => {
                set_path_speed(&mut unit.aircraft.path, f(&a["speed"]));
                Ok(None)
            }
            other => panic!("{other}"),
        };
        let native: Vec<Value> = out["events"].as_array().unwrap().iter().filter(|e| e["name"] != "New").cloned().collect();
        assert_eq!(world.events, native, "{name}: group calls and terrain queries");
        if let Some(error) = out.get("error").and_then(Value::as_str) {
            let got = result.expect_err(name);
            assert!(error.starts_with("NullDereference") && matches!(got, AircraftError::NullDereference(_)), "{name}: {got}");
            continue;
        }
        let returned = result.unwrap_or_else(|e| panic!("{name}: {e}"));
        if let Some(r) = returned {
            assert_eq!(u32::from(r), u(&out["return_al"]), "{name}: return");
        }
        if let Some(p) = point {
            assert_eq!(bits3(p), words(&out["point"]), "{name}: slot point");
        }
        assert!(world.heights.is_empty(), "{name}: unused terrain responses");
        assert_eq!(aircraft_json(&unit.aircraft), out["aircraft"], "{name}: aircraft");
        for (k, expected) in out["others"].as_object().unwrap() {
            let got = world.others[&k.parse::<u32>().unwrap()].aircraft.as_ref().unwrap();
            assert_eq!(aircraft_json(got), *expected, "{name}: aircraft of actor {k}");
        }
        let refs: BTreeMap<String, u32> =
            world.groups.iter().filter(|(_, g)| !g.deleted).map(|(k, g)| (k.to_string(), g.refs)).collect();
        assert_eq!(json!(refs), out["refs"], "{name}: reference counts");
    }
    for op in ["fly_to", "replace_path", "fly_formation", "follow_path", "slot_point", "set_path_speed"] {
        assert!(counts.get(op).copied().unwrap_or(0) >= 40, "{op}: too few cases");
    }
}
