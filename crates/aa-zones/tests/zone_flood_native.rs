//! Replays research/phase8-zones/native-zone-flood.json: zone moves/releases,
//! reveal-mask changes and effects walks through the original ZoneSolver
//! (0x5402E0 Dijkstra) and flood solver (0x4F4050 FIFO) paths at PC24, PC53
//! and PC64: ordered events, faults, zone fields, every tile, tile +0x24
//! scratch, the GMap stamp and the busy byte (native queue limit 0x1F9C).
mod common;
use aa_numeric::Precision;
use aa_zones::flood::{NATIVE_QUEUE_LIMIT, Scratch, SolverState};
use aa_zones::tile::{MapGeometry, ZoneMap};
use aa_zones::zone::{self, Zone};
use aa_zones::{Error, solver};
use common::*;
use serde_json::{Value, json};

fn precision(s: &str) -> Precision {
    match s {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        _ => Precision::Extended64,
    }
}

fn zone_state(z: &Zone) -> Vec<u32> {
    vec![z.radius_bits, z.x as u32, z.y as u32, u32::from(z.flags), u32::from(z.state)]
}

fn scratch_json(s: &Scratch) -> Value {
    match s {
        Scratch::Bits(b) => json!(b),
        Scratch::Link(t) => json!(["link", t]),
    }
}

fn scratch_from(v: &Value) -> Scratch {
    if v.is_array() { Scratch::Link(u(&v[1]) as usize) } else { Scratch::Bits(u(v)) }
}

#[test]
fn zone_flood_matches_native() {
    let f = fixture("native-zone-flood.json");
    let (mut n, mut faults) = (0, 0);
    for case in f["cases"].as_array().unwrap() {
        let p = precision(case["precision"].as_str().unwrap());
        let mp = &case["map"];
        let (w_, h_) = (u(&mp["width"]) as i32, u(&mp["height"]) as i32);
        let geometry = MapGeometry {
            shift: u(&mp["shift"]),
            tile_size_bits: u(&mp["tile"]),
            extent_bits: [u(&mp["extent"][0]), u(&mp["extent"][1])],
            width: w_,
            height: h_,
        };
        let mut map = ZoneMap::new(geometry).unwrap();
        map.solver = SolverState::new(map.tiles.len(), Some(NATIVE_QUEUE_LIMIT));
        map.solver.stamp = u(&case["initial_stamp"]);
        for (t, s) in case["initial_scratch"].as_array().unwrap().iter().enumerate() {
            map.solver.scratch[t] = scratch_from(s);
        }
        let mut w = World::from_case(case, w_);
        let addrs: Vec<u32> = w.actors.iter().map(|a| a.addr).collect();
        let mut zones: Vec<Zone> = vec![];
        for (i, z) in case["zones"].as_array().unwrap().iter().enumerate() {
            let zone = Zone::new(u(&case["zone_keys"][i]), addrs[u(&z[0]) as usize], 1 << u(&z[1]));
            let s: Vec<u32> = case["zone_states"][i].as_array().unwrap().iter().map(u).collect();
            assert_eq!(zone_state(&zone), s);
            zones.push(zone);
        }
        let mut expected: Vec<Value> = (0..map.tiles.len()).map(|t| tile_state(&map, t, &w)).collect();
        for op in case["ops"].as_array().unwrap() {
            let zi = u(&op["zone"]) as usize;
            w.events.clear();
            let ctx = format!("seed {} {} op {}", case["seed"], case["precision"], op["op"]);
            let r = match op["op"].as_str().unwrap() {
                "move_to" => {
                    let pos = [u(&op["pos"][0]), u(&op["pos"][1])];
                    zone::move_to_position(&mut map, &mut w, &mut zones[zi], pos, u(&op["radius"]), p)
                }
                "move" => {
                    let (x, y) = (u(&op["x"]) as i32, u(&op["y"]) as i32);
                    zone::move_to_tile(&mut map, &mut w, &mut zones[zi], x, y, u(&op["radius"]), p)
                }
                "release" => zone::release(&mut map, &mut w, &mut zones[zi], p),
                "mask" => solver::kingdom_mask_changed(&mut map, &mut w, &mut zones[zi], u(&op["set"]), u(&op["mask_changed"]), p),
                "effects" => {
                    let z = zones[zi];
                    solver::effects_pass(&mut map, &mut w, &z)
                }
                other => panic!("{other}"),
            };
            match (&r, op["fault"].as_str()) {
                (Ok(()), None) => {}
                (Err(Error::NativeNull(_) | Error::NativeNullSet | Error::NativeAbsentActor), Some(_)) => faults += 1,
                other => panic!("{ctx}: {other:?}"),
            }
            assert_eq!(Value::Array(w.events.clone()), op["events"], "{ctx}");
            for (i, z) in zones.iter().enumerate() {
                let s: Vec<u32> = op["zones"][i].as_array().unwrap().iter().map(u).collect();
                assert_eq!(zone_state(z), s, "{ctx} zone {i}");
            }
            for c in op["changed"].as_array().unwrap() {
                expected[u(&c[0]) as usize] = c[1].clone();
            }
            for (t, e) in expected.iter().enumerate() {
                assert_eq!(tile_state(&map, t, &w), *e, "{ctx} tile {t}");
            }
            if op["fault"].is_null() {
                let sc: Vec<Value> = map.solver.scratch.iter().map(scratch_json).collect();
                assert_eq!(Value::Array(sc), op["scratch"], "{ctx} scratch");
                assert_eq!(map.solver.stamp, u(&op["stamp"]), "{ctx} stamp");
                assert_eq!(u32::from(map.solver.busy), u(&op["busy"]), "{ctx} busy");
            }
            n += 1;
        }
    }
    assert_eq!(n, u(&f["op_count"]) as usize);
    assert_eq!(faults, u(&f["faults"]) as usize);
}
