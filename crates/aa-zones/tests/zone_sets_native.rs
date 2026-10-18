//! Replays research/phase8-zones/native-zone-sets.json (the original zone-set
//! classes driven through 0x54FA10 / 0x54FA80 under Unicorn) against
//! aa_zones::ZonePool: per operation the touched tile's list in native
//! iteration order and the native fault points; per case every tile and every
//! interned object with its reference count.
use aa_zones::{Entry, Error, LiveSet, SetRef, ZonePool};
use serde_json::Value;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-zones/native-zone-sets.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

fn list(v: &Value) -> Vec<Entry> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|e| Entry { actor: u(&e[0]), flags: u(&e[1]) as u16 })
        .collect()
}

#[test]
fn zone_sets_match_native() {
    let f = fixture();
    let mut ops_checked = 0;
    let mut faults = 0;
    for case in f["cases"].as_array().unwrap() {
        let zones: Vec<(u32, u16)> = case["zones"]
            .as_array()
            .unwrap()
            .iter()
            .map(|z| (u(&z["actor"]), u(&z["flags"]) as u16))
            .collect();
        let ntiles = case["final_tiles"].as_array().unwrap().len();
        let mut tiles: Vec<Option<SetRef>> = vec![None; ntiles];
        let mut pool = ZonePool::new();
        for op in case["ops"].as_array().unwrap() {
            let t = u(&op["tile"]) as usize;
            let (actor, zflags) = zones[u(&op["zone"]) as usize];
            let before = tiles[t];
            let r = if op["op"] == "add" {
                pool.tile_add(&mut tiles[t], actor, u(&op["flags"]) as u16)
            } else {
                pool.tile_remove(&mut tiles[t], actor, zflags)
            };
            match (&r, op["fault"].is_null()) {
                (Ok(()), true) => {}
                (Err(Error::NativeNullLeaf), false) | (Err(Error::NativeNullSet), false) => {
                    faults += 1;
                    assert_eq!(tiles[t], before, "a native fault leaves the tile unchanged");
                }
                other => panic!("seed {} op {op}: {other:?}", case["seed"]),
            }
            assert_eq!(pool.entries(tiles[t]).unwrap(), list(&op["list"]), "seed {} op {op}", case["seed"]);
            ops_checked += 1;
        }
        for (t, l) in case["final_tiles"].as_array().unwrap().iter().enumerate() {
            assert_eq!(pool.entries(tiles[t]).unwrap(), list(l));
        }
        let mut native: Vec<LiveSet> = case["live_sets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                let a = s.as_array().unwrap();
                let (refs, entries) = a.split_last().unwrap();
                assert_eq!(refs[0], "refs");
                LiveSet {
                    entries: list(&Value::Array(entries.to_vec())),
                    refs: u(&refs[1]),
                }
            })
            .collect();
        native.sort();
        assert_eq!(pool.live().unwrap(), native, "seed {}", case["seed"]);
    }
    assert_eq!(ops_checked, u(&f["op_count"]) as usize);
    assert_eq!(faults, u(&f["faults"]) as usize);
}
