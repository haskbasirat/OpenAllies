//! Replays research/phase8-movement/native-tile-neighbours.json (original
//! 4F14A0 under Unicorn, every direction) against
//! aa_spatial::tile_neighbours.
use aa_spatial::tile_neighbours::{NEIGHBOUR_DELTAS, neighbour_address};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-movement/native-tile-neighbours.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn i(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}

#[test]
fn native_tile_neighbours_match_original() {
    let f = fixture();
    assert_eq!(f["coverage"]["4F14A0"], json!([18, 18]));
    assert_eq!(f["table"], json!(NEIGHBOUR_DELTAS));
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 400);
    for row in cases {
        let c = &row["input"];
        let name = &c["name"];
        let base = c["base"].as_u64().unwrap() as u32;
        let got: Vec<u32> = (0..9)
            .map(|d| neighbour_address(i(&c["x"]), i(&c["y"]), i(&c["width"]), base, d).unwrap())
            .collect();
        assert_eq!(json!(got), row["output"], "{name}");
    }
    assert!(neighbour_address(0, 0, 1, 0, 9).is_none());
}
