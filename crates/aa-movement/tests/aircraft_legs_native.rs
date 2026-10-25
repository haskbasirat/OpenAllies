//! Replays research/phase8-movement/native-aircraft-legs.json: original x86
//! execution (Unicorn) of the AIRCRAFT leg builder `0x55F230` (with its piece
//! subdivision recursion), the multi-leg planner `0x5601E0` (leg target
//! `0x55FE60`, circle test `0x55E1E0`, path clear `0x55E1B0`) and the helpers
//! `0x55DEF0`, `0x55E0F0`, `0x55D920` and `0x55E1E0` alone, under
//! PC24/53/64. The out pose or return value, every terrain query with its
//! argument bits in order, the number of segment allocations and every word
//! of the resulting path and its segments must match.
use aa_movement::aircraft::*;
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::collections::VecDeque;

fn document() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-movement/native-aircraft-legs.json");
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
fn pose_bits(p: &Pose) -> Vec<u32> {
    let mut out = bits3(p.position);
    out.extend(bits3(p.direction));
    out.push(p.speed.to_bits());
    out
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
fn f80_bits(hex: &str) -> u128 {
    (0..10).fold(0u128, |acc, i| acc | (u128::from(u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()) << (8 * i)))
}
fn x87(value: &Binary) -> u128 {
    value.to_x87_extended_bits().expect("x87 image")
}

struct Terrain {
    heights: VecDeque<Value>,
    events: Vec<Value>,
}
impl TerrainWorld for Terrain {
    fn ground_height(&mut self, xy: [f32; 2], flags: u32) -> AircraftResult<Binary> {
        self.events.push(json!({"name": "GroundHeight", "bits": [xy[0].to_bits(), xy[1].to_bits(), flags]}));
        let r = self.heights.pop_front().expect("unexpected GroundHeight");
        let text = r["f64"].as_str().unwrap();
        Ok(Binary::from_f64_bits(u64::from_str_radix(text.trim_start_matches("0x"), 16).unwrap()).unwrap())
    }
}

#[test]
fn leg_planning_matches_original_x86() {
    let doc = document();
    let rows = doc["cases"].as_array().unwrap();
    assert!(rows.len() >= 500, "fixture too small");
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    let mut degenerate = 0;
    for row in rows {
        let case = &row["case"];
        let name = case["name"].as_str().unwrap();
        let out = row.get("result").unwrap_or_else(|| panic!("{name}: native error {}", row["error"]));
        let op = case["operation"].as_str().unwrap();
        *counts.entry(op.to_string()).or_default() += 1;
        let pc: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let a = &case["args"];
        let mut state = aircraft(&case["aircraft"]);
        let before = state.path.segments.len();
        let heights = case["responses"]["GroundHeight"].as_array().map_or(VecDeque::new(), |h| h.iter().cloned().collect());
        let mut terrain = Terrain { heights, events: vec![] };
        let direction = || (!a["direction"].is_null()).then(|| f3(&a["direction"]));
        match op {
            "build_leg" => {
                let got = build_leg(
                    &mut state.path,
                    &pose(&a["from"]),
                    f3(&a["target"]),
                    direction(),
                    u(&a["flag"]) as u8,
                    u(&a["extra"]) as u8,
                    pc,
                    &mut terrain,
                )
                .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(pose_bits(&got), words(&out["out"]), "{name}: out pose");
            }
            "append_legs" => {
                let got = append_legs(
                    &mut state.path,
                    &pose(&a["pose"]),
                    f3(&a["target"]),
                    direction(),
                    u(&a["flag"]) as u8,
                    u(&a["single"]) as u8,
                    pc,
                    &mut terrain,
                );
                let got = match got {
                    // A zero-length leg (no horizontal heading) is computed
                    // natively through infinities and NaNs: outside the
                    // finite domain, it must stop with a numeric error.
                    Err(e) if a["degenerate"] == true => {
                        assert!(
                            matches!(
                                e,
                                AircraftError::Movement(
                                    aa_movement::Error::NonFinite
                                        | aa_movement::Error::Numeric(
                                            aa_numeric::Error::DivisionByZero | aa_numeric::Error::NonFiniteInput
                                        )
                                ) | AircraftError::NonFinite(_)
                            ),
                            "{name}: {e}"
                        );
                        degenerate += 1;
                        continue;
                    }
                    other => other.unwrap_or_else(|e| panic!("{name}: {e}")),
                };
                assert_eq!(u32::from(got), u(&out["return_al"]), "{name}: return");
            }
            "flat_length" => {
                let got = flat_length(&state.path.basis, &segment(&a["segment"]), f(&a["t0"]), f(&a["t1"]), pc)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(x87(&got), f80_bits(out["length_f80"].as_str().unwrap()), "{name}: length");
            }
            "length_parameter" => {
                let got = length_parameter(&state.path.basis, &segment(&a["segment"]), f(&a["t0"]), f(&a["piece"]), pc)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                let got = Binary::from_f32_bits(got.to_bits()).unwrap();
                assert_eq!(x87(&got), f80_bits(out["parameter_f80"].as_str().unwrap()), "{name}: parameter");
            }
            "altitude_band" => {
                let target = f3(&a["target"]);
                let z = altitude_band(&state.path, target[2], pose(&a["from"]).position[2], pc)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(bits3([target[0], target[1], z]), words(&out["target"]), "{name}: target");
            }
            "circle_side" => {
                let got = circle_side(&state.path, &pose(&a["pose"]), f3(&a["side"]), f3(&a["target"]), pc)
                    .unwrap_or_else(|e| panic!("{name}: {e}"));
                assert_eq!(got, u(&out["return_eax"]), "{name}: circle");
            }
            other => panic!("{other}"),
        }
        let native: Vec<&Value> = out["events"].as_array().unwrap().iter().collect();
        let queries: Vec<Value> = native.iter().filter(|e| e["name"] == "GroundHeight").map(|e| (*e).clone()).collect();
        assert_eq!(terrain.events, queries, "{name}: terrain queries");
        assert!(terrain.heights.is_empty(), "{name}: unused terrain responses");
        assert_eq!(aircraft_json(&state), out["aircraft"], "{name}: aircraft");
        if op == "build_leg" {
            let allocations = native.iter().filter(|e| e["name"] == "New").count();
            assert_eq!(state.path.segments.len() - before, allocations, "{name}: segment allocations");
        }
    }
    for op in ["build_leg", "append_legs", "flat_length", "length_parameter", "altitude_band", "circle_side"] {
        assert!(counts.get(op).copied().unwrap_or(0) >= 40, "{op}: too few cases");
    }
    // Most of the zero-heading cases still reach finite legs.
    assert!(degenerate * 3 < counts["append_legs"] / 10, "{degenerate} zero-heading errors");
}
