//! Replays research/phase8-ai/native-unreachable-memory.json (original
//! 532840 with 530E00 and 530ED0 under Unicorn) against
//! aa_mind::unreachable_memory.
use aa_mind::unreachable_memory::{UnreachableRecord, remember, remembered};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-unreachable-memory.json"
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
fn native_unreachable_memory_matches_original() {
    let f = fixture();
    assert_eq!(f["coverage"]["530E00"], json!([28, 28]));
    assert_eq!(f["coverage"]["532840"], json!([51, 51]));
    // 530ED0: only its allocation-failure instruction is not reached.
    assert_eq!(f["coverage"]["530ED0"], json!([21, 22]));
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 200);
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let gtid = |k: &Value| k.as_str().map_or(0, |k| u(&c["objects"][k]["gtid"]));
        let mut list: Vec<UnreachableRecord> = vec![];
        for step in c["remember"].as_array().unwrap() {
            let other = step["other"]
                .as_str()
                .map(|k| (u(&c["objects"][k]["gtid"]), pair(&c["objects"][k]["pos"])));
            remember(
                &mut list,
                gtid(&step["object"]),
                u(&step["time"]),
                other,
                pair(&c["null_point"]),
            );
        }
        let queries: Vec<u8> = c["queries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| u8::from(remembered(&list, gtid(k))))
            .collect();
        let records: Vec<Value> = list
            .iter()
            .map(|r| json!([r.gtid, r.time, r.other, r.point[0], r.point[1]]))
            .collect();
        assert_eq!(json!(records), o["records"], "{name}");
        assert_eq!(json!(queries), o["queries"], "{name}");
        assert_eq!(o["tail_is_first_pushed"], json!(true), "{name}");
    }
}
