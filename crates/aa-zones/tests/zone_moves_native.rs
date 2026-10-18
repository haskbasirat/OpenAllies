//! Replays research/phase8-zones/native-zone-moves.json: KKC_Zone
//! construction, move (0x515F40 / 0x515970), release (0x514CE0), reveal-mask
//! change (0x5406D0) through the fast ZoneSolver circle builder, at PC24, PC53
//! and PC64: ordered events, native faults, zone fields and every tile.
mod common;
use aa_numeric::Precision;
use aa_zones::tile::ZoneMap;
use aa_zones::zone::{self, Zone};
use aa_zones::{Error, solver};
use common::*;
use serde_json::Value;

fn precision(s: &str) -> Precision {
    match s {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("{other}"),
    }
}

fn zone_state(z: &Zone) -> Vec<u32> {
    vec![z.radius_bits, z.x as u32, z.y as u32, u32::from(z.flags), u32::from(z.state)]
}

#[test]
fn zone_moves_match_native() {
    let f = fixture("native-zone-moves.json");
    let (mut n, mut faults) = (0, 0);
    for case in f["cases"].as_array().unwrap() {
        let p = precision(case["precision"].as_str().unwrap());
        let mp = &case["map"];
        let (w_, h_) = (u(&mp["width"]) as i32, u(&mp["height"]) as i32);
        let geometry = aa_zones::tile::MapGeometry {
            shift: u(&mp["shift"]),
            tile_size_bits: u(&mp["tile"]),
            extent_bits: [u(&mp["extent"][0]), u(&mp["extent"][1])],
            width: w_,
            height: h_,
        };
        let mut map = ZoneMap::new(geometry).unwrap();
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
                "set_reveal" => {
                    w.actors[u(&op["actor"]) as usize].reveal = Some(u(&op["mask"]));
                    Ok(())
                }
                other => panic!("{other}"),
            };
            match (&r, op["fault"].as_str()) {
                (Ok(()), None) => {}
                (Err(Error::NativeNull(_) | Error::NativeNullSet), Some(_)) => faults += 1,
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
            n += 1;
        }
    }
    assert_eq!(n, u(&f["op_count"]) as usize);
    assert_eq!(faults, u(&f["faults"]) as usize);
}
