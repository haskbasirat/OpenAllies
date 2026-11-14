//! Replays research/phase8-movement/native-turn-circle.json (original
//! 52E6A0 under Unicorn, at PC24/PC53/PC64) against
//! aa_navigation::turn_circle.
use aa_navigation::turn_circle::within_turning_circles;
use aa_numeric::Precision;
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-movement/native-turn-circle.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}

#[test]
fn native_turn_circle_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 1500);
    assert_eq!(f["coverage"]["52E6A0"], json!([76, 76]));
    let mut inside = 0;
    for row in cases {
        let c = &row["input"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let r = within_turning_circles(
            u(&c["radius"]),
            pair(&c["pos"]),
            pair(&c["facing"]),
            pair(&c["point"]),
            p,
        )
        .unwrap();
        assert_eq!(json!(u8::from(r)), row["output"]["result"], "{name}");
        inside += usize::from(r);
    }
    assert_eq!(inside, 1075);
}
