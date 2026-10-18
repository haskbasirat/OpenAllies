//! Replays research/phase8-zones/native-detection-component.json: the
//! original detection component (0x56F140 tick, 0x56EC90, 0x56EB40,
//! 0x56EA30, 0x56EC10, 0x56EFC0, 0x56EA10) with real zones created by
//! 0x4D69A0 and moved by the fast ZoneSolver, at PC24/PC53/PC64: results,
//! faults, events, component fields, actor zone vectors and every tile.
mod common;
use aa_numeric::Precision;
use aa_zones::component::{self, DetectionComponent};
use aa_zones::tile::{MapGeometry, ZoneMap};
use aa_zones::zone::Zone;
use aa_zones::Error;
use common::*;
use serde_json::{Value, json};

fn zone_json(z: &Option<Zone>) -> Value {
    match z {
        None => Value::Null,
        Some(z) => json!([z.radius_bits, z.x as u32, z.y as u32, z.flags, z.state]),
    }
}

#[test]
fn detection_component_matches_native() {
    let f = fixture("native-detection-component.json");
    let (mut n, mut faults) = (0, 0);
    for case in f["cases"].as_array().unwrap() {
        let p = match case["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let mp = &case["map"];
        let (w_, h_) = (u(&mp["width"]) as i32, u(&mp["height"]) as i32);
        let mut map = ZoneMap::new(MapGeometry {
            shift: u(&mp["shift"]),
            tile_size_bits: u(&mp["tile"]),
            extent_bits: [u(&mp["extent"][0]), u(&mp["extent"][1])],
            width: w_,
            height: h_,
        })
        .unwrap();
        let mut w = World::from_case(case, w_);
        for (i, a) in case["actors"].as_array().unwrap().iter().enumerate() {
            let addr = w.actors[i].addr;
            w.dcomps.insert(i, DetectionComponent { actor: addr, reveal_mask: 0, zod_radius_bits: u(&a["zod"]) });
            if u(&a["slots"]) == 9 {
                w.zone_vectors.insert(i, aa_zones::component::ActorZones { slots: vec![None; 9] });
            }
        }
        let mut expected: Vec<Value> = (0..map.tiles.len()).map(|t| tile_state(&map, t, &w)).collect();
        for op in case["ops"].as_array().unwrap() {
            let ai = u(&op["actor"]) as usize;
            let a = w.actors[ai].addr;
            w.events.clear();
            let ctx = format!("seed {} {} {op}", case["seed"], case["precision"]);
            let r: aa_zones::Result<Option<bool>> = match op["op"].as_str().unwrap() {
                "tick" => component::tick(&mut map, &mut w, a, p).map(|_| None),
                "activate" => component::activate(&mut map, &mut w, a, p).map(|_| None),
                "enable" => component::enable_zone(
                    &mut map, &mut w, a, u(&op["index"]), u(&op["enable"]) != 0, u(&op["radius"]), p,
                )
                .map(|_| None),
                "update_mask" => component::update_reveal_mask(&mut map, &mut w, a, p).map(|_| None),
                "zod_enabled" => component::zod_enabled(&mut w, a).map(Some),
                "rzod_enabled" => component::rzod_enabled(&mut w, a).map(Some),
                "set_radius" => component::set_zod_radius(&mut w, a, 0).map(|_| None),
                "actor_moved" => {
                    // 0x4D6350: seamed callees recorded as in the producer, the
                    // zone loop is the port under test.
                    w.events.push(json!(["tile_changed", ai]));
                    w.events.push(json!(["4d4130", ai]));
                    if w.actors[ai].members.is_some() {
                        w.events.push(json!(["579ba0", null]));
                    }
                    let r = component::move_actor_zones(&mut map, &mut w, a, p);
                    if r.is_ok() {
                        w.events.push(json!(["4cfea0", ai]));
                    }
                    r.map(|_| None)
                }
                "mutate" => {
                    let v = &op["value"];
                    match op["what"].as_str().unwrap() {
                        "pos" => w.actors[ai].pos = [u(&v[0]), u(&v[1])],
                        "sources" => w.actors[ai].sources = u(v),
                        "flags" => w.actors[ai].flags = u(v),
                        _ => {
                            w.actors[ai].data.insert(0x1cc, u(v));
                        }
                    }
                    Ok(None)
                }
                other => panic!("{other}"),
            };
            match (&r, op["fault"].is_null()) {
                (Ok(v), true) => {
                    if let Some(v) = v {
                        assert_eq!(u32::from(*v), u(&op["result"]), "{ctx}");
                    }
                }
                (Err(Error::NativeNull(_)), false) => faults += 1,
                other => panic!("{ctx}: {other:?}"),
            }
            assert_eq!(Value::Array(w.events.clone()), op["events"], "{ctx}");
            for (i, c) in op["components"].as_array().unwrap().iter().enumerate() {
                let d = w.dcomps[&i];
                assert_eq!([d.reveal_mask, d.zod_radius_bits], [u(&c[0]), u(&c[1])], "{ctx} component {i}");
            }
            for (i, zs) in op["zones"].as_array().unwrap().iter().enumerate() {
                let mine: Vec<Value> =
                    w.zone_vectors.get(&i).map_or(vec![], |v| v.slots.iter().map(zone_json).collect());
                assert_eq!(Value::Array(mine), *zs, "{ctx} zones of actor {i}");
            }
            for c in op["changed"].as_array().unwrap() {
                expected[u(&c[0]) as usize] = c[1].clone();
            }
            for (t, e) in expected.iter().enumerate() {
                assert_eq!(tile_state(&map, t, &w), *e, "{ctx} tile {t}");
            }
            n += 1;
        }
    }
    assert_eq!(n, u(&f["op_count"]) as usize);
    assert_eq!(faults, u(&f["faults"]) as usize);
}
