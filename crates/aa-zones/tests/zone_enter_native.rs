//! Replays research/phase8-zones/native-zone-enter.json: an actor entering a
//! tile (0x54EF10 -> zone-set slot +0x54, 0x515620 / 0x514FC0) with guard,
//! control, type-3/8 effects and the settlement-marker discovery, at
//! PC24/PC53/PC64: events, faults, every tile and the actors' +8 flags.
mod common;
use aa_numeric::Precision;
use aa_zones::tile::{MapGeometry, TileZones, ZoneMap};
use aa_zones::zone::{self, Zone};
use aa_zones::Error;
use common::*;
use serde_json::Value;

#[test]
fn zone_enter_matches_native() {
    let f = fixture("native-zone-enter.json");
    let (mut n, mut faults) = (0, 0);
    for case in f["cases"].as_array().unwrap() {
        let p = match case["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let mut w = World::from_case(case, 5);
        let addrs: Vec<u32> = w.actors.iter().map(|a| a.addr).collect();
        let tile = 8.0f32.to_bits();
        let mut map = ZoneMap::new(MapGeometry { shift: 3, tile_size_bits: tile, extent_bits: [0, 0], width: 5, height: 4 }).unwrap();
        for (i, z) in case["zones"].as_array().unwrap().iter().enumerate() {
            let a = u(&z[0]) as usize;
            let ty = u(&z[1]);
            let s = &case["zone_states"][i];
            let mut zone = Zone::new(0x7000_0000 + i as u32, addrs[a], 1 << ty);
            zone.radius_bits = u(&s[0]);
            w.enter_zones.insert((a, ty), zone);
        }
        for (t, s) in case["initial_tiles"].as_array().unwrap().iter().enumerate() {
            let set = build(&mut map, &s[0], &addrs);
            map.tiles[t] = TileZones { set, fog: u(&s[1]), supply: u(&s[2]) as u16 };
            assert_eq!(tile_state(&map, t, &w), *s);
        }
        let mut expected: Vec<Value> = (0..map.tiles.len()).map(|t| tile_state(&map, t, &w)).collect();
        for op in case["ops"].as_array().unwrap() {
            let t = u(&op["tile"]) as usize;
            let a = addrs[u(&op["actor"]) as usize];
            w.events.clear();
            let ctx = format!("seed {} {} {op}", case["seed"], case["precision"]);
            let r = zone::actor_entered_tile(&mut map, &mut w, t, a, p);
            match (&r, op["fault"].is_null()) {
                (Ok(()), true) => {}
                (Err(Error::NativeNull(_)), false) => faults += 1,
                other => panic!("{ctx}: {other:?}"),
            }
            assert_eq!(Value::Array(w.events.clone()), op["events"], "{ctx}");
            for c in op["changed"].as_array().unwrap() {
                expected[u(&c[0]) as usize] = c[1].clone();
            }
            for (x, e) in expected.iter().enumerate() {
                assert_eq!(tile_state(&map, x, &w), *e, "{ctx} tile {x}");
            }
            for (i, fl) in op["flags"].as_array().unwrap().iter().enumerate() {
                assert_eq!(w.actors[i].flags, u(fl), "{ctx} actor {i} flags");
            }
            n += 1;
        }
    }
    assert_eq!(n, u(&f["op_count"]) as usize);
    assert_eq!(faults, u(&f["faults"]) as usize);
}
