//! Replays research/phase8-actor-creation/native-group-records.json
//! (original 4DDC50 under Unicorn) against aa_actors::group_records.
use aa_actors::group_records::{GroupRecord, group_record};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-actor-creation/native-group-records.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}

#[test]
fn native_group_records_match_original() {
    let f = fixture();
    assert_eq!(f["coverage"]["4DDC50"], json!([43, 43]));
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 300);
    for row in cases {
        let c = &row["input"];
        let name = &c["name"];
        let all: Vec<GroupRecord> = c["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| GroupRecord {
                gtid: u(&r[0]),
                word4: u(&r[1]),
                word8: u(&r[2]),
                byte_c: u(&r[3]) as u8,
                byte_d: u(&r[4]) as u8,
            })
            .collect();
        let count = c["count"].as_i64().unwrap();
        let live = if count > 0 { &all[..count as usize] } else { &all[..0] };
        let r = group_record(live, u(&c["gtid"]));
        let mut bytes = vec![0x5a_u8; 16];
        bytes[0..4].copy_from_slice(&r.gtid.to_le_bytes());
        bytes[4..8].copy_from_slice(&r.word4.to_le_bytes());
        bytes[8..12].copy_from_slice(&r.word8.to_le_bytes());
        bytes[12] = r.byte_c;
        bytes[13] = r.byte_d;
        assert_eq!(json!(bytes), row["output"], "{name}");
    }
}
