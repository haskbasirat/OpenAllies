//! Replays research/phase8-zones/native-zone-tiles.json: zone tile callbacks
//! (0x515AD0 / 0x515BA0 through 0x54FA10 / 0x54FA80), fog/supply routines
//! (0x54F4F0, 0x54F5E0, 0x54F6A0, 0x54ED20, 0x54F930, 0x54F3C0, 0x54F3E0) and
//! zone component effects (0x54F7F0, 0x514DA0, 0x514E00, 0x514E20,
//! 0x514E40), comparing ordered events, faults and every tile after each op.
mod common;
use aa_zones::solver::{tile_add, tile_remove};
use aa_zones::tile::{self, TileZones, ZoneMap};
use aa_zones::zone::Zone;
use aa_zones::Error;
use common::*;
use serde_json::Value;

#[test]
fn zone_tiles_match_native() {
    let f = fixture("native-zone-tiles.json");
    let (mut n, mut faults) = (0, 0);
    for case in f["cases"].as_array().unwrap() {
        let mut w = World::from_case(case, 4);
        let addrs: Vec<u32> = w.actors.iter().map(|a| a.addr).collect();
        let mut map = ZoneMap::new(geometry(4, 3, 3, 8.0f32.to_bits())).unwrap();
        for (t, s) in case["initial_tiles"].as_array().unwrap().iter().enumerate() {
            let set = build(&mut map, &s[0], &addrs);
            map.tiles[t] = TileZones { set, fog: u(&s[1]), supply: u(&s[2]) as u16 };
            assert_eq!(tile_state(&map, t, &w), *s);
        }
        let mut zones: Vec<Zone> = vec![];
        for (i, z) in case["zones"].as_array().unwrap().iter().enumerate() {
            let a = addrs[u(&z[0]) as usize];
            let zone = Zone::new(u(&case["zone_keys"][i]), a, 1 << u(&z[1]));
            let s = &case["zone_states"][i];
            assert_eq!(
                (zone.radius_bits, zone.x as u32, zone.y as u32, u32::from(zone.flags), u32::from(zone.state)),
                (u(&s[0]), u(&s[1]), u(&s[2]), u(&s[3]), u(&s[4]))
            );
            zones.push(zone);
        }
        for op in case["ops"].as_array().unwrap() {
            let t = u(&op["tile"]) as usize;
            w.events.clear();
            let kingdom = |w: &World| opt(&op["kingdom"]).and_then(|i| w.kingdoms[i]);
            let r: aa_zones::Result<Option<u32>> = match op["op"].as_str().unwrap() {
                "add" => tile_add(&mut map, &mut w, &zones[u(&op["zone"]) as usize], t).map(|_| None),
                "remove" => tile_remove(&mut map, &mut w, &zones[u(&op["zone"]) as usize], t).map(|_| None),
                "restore" => tile::restore_fog(&mut map, &mut w, t).map(|_| None),
                "reveal" => {
                    let k = kingdom(&w).map_or(0, |k| k.key);
                    tile::reveal(&mut map, &mut w, t, k, false, true).map(|_| None)
                }
                "hide" => {
                    let k = kingdom(&w).map_or(0, |k| k.key);
                    tile::hide(&mut map, &mut w, t, k).map(|_| None)
                }
                "visible" => {
                    let k = kingdom(&w);
                    let fog = map.tiles[t].fog;
                    tile::visible_or_no_fog(&mut w, fog, k.as_ref()).map(|v| Some(u32::from(v)))
                }
                "shared_visible" => {
                    let k = kingdom(&w);
                    let fog = map.tiles[t].fog;
                    tile::shared_visible_or_no_fog(&mut w, fog, k.as_ref()).map(|v| Some(u32::from(v)))
                }
                other => panic!("{other}"),
            };
            match (&r, op["fault"].is_null()) {
                (Ok(v), true) => {
                    if let Some(v) = v {
                        assert_eq!(*v, u(&op["result"]), "seed {} {op}", case["seed"]);
                    }
                }
                (Err(Error::NativeNull(_)), false) => faults += 1,
                other => panic!("seed {} op {op}: {other:?}", case["seed"]),
            }
            assert_eq!(Value::Array(w.events.clone()), op["events"], "seed {} op {}", case["seed"], op["op"]);
            for (i, s) in op["tiles"].as_array().unwrap().iter().enumerate() {
                assert_eq!(tile_state(&map, i, &w), *s, "seed {} tile {i} after {}", case["seed"], op["op"]);
            }
            n += 1;
        }
    }
    assert_eq!(n, u(&f["op_count"]) as usize);
    assert_eq!(faults, u(&f["faults"]) as usize);
}
