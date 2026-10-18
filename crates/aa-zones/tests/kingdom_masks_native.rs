//! Replays research/phase8-zones/native-kingdom-masks.json: the kingdom
//! shared-vision mask +0x228 and supply mask +0x22C (0x4E0F90 -> 0x54EBA0,
//! 0x559600) with the ordered kingdom +0x138 calls.
mod common;
use aa_zones::tile::kingdom_masks;
use common::*;
use serde_json::{Value, json};

#[test]
fn kingdom_masks_match_native() {
    let f = fixture("native-kingdom-masks.json");
    let mut n = 0;
    for case in f["cases"].as_array().unwrap() {
        let mut c = case.clone();
        c["actors"] = json!([]);
        for k in c["kingdoms"].as_array_mut().unwrap() {
            if !k.is_null() {
                k["shared"] = json!(0);
                k["supply"] = json!(0);
            }
        }
        let mut w = World::from_case(&c, 1);
        for call in case["calls"].as_array().unwrap() {
            let k = w.kingdoms[u(&call["kingdom"]) as usize].unwrap();
            w.events.clear();
            let (shared, supply) = kingdom_masks(&mut w, &k).unwrap();
            assert!(call["fault"].is_null());
            assert_eq!(Value::Array(w.events.clone()), call["events"], "seed {} {call}", case["seed"]);
            assert_eq!((shared, u32::from(supply)), (u(&call["shared"]), u(&call["supply"])), "seed {}", case["seed"]);
            n += 1;
        }
    }
    assert_eq!(n, u(&f["op_count"]) as usize);
}
