//! Replays research/phase8-movement/native-path-match.json (original 52CED0
//! under Unicorn) against aa_navigation::path_match.
use aa_navigation::{OwnedPath, PathNode, path_match::matching_kind1_node};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-movement/native-path-match.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn native_path_match_matches_original() {
    let f = fixture();
    assert_eq!(f["coverage"]["52CED0"], json!([21, 21]));
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 500);
    let mut found = 0;
    for row in cases {
        let c = &row["input"];
        let name = &c["name"];
        let nodes = c["kinds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| {
                let mut words = [0; 8];
                words[4] = k.as_u64().unwrap() as u32;
                PathNode { words }
            })
            .collect();
        let path = OwnedPath {
            nodes,
            kind1_count: c["kind1"].as_i64().unwrap() as i32,
            kind2_count: 0,
            dirty: false,
            word18: 0,
        };
        let r = matching_kind1_node(&path, c["other_kind1"].as_i64().unwrap() as i32);
        assert_eq!(json!(r), row["output"], "{name}");
        found += usize::from(r.is_some());
    }
    assert_eq!(found, 109);
}
