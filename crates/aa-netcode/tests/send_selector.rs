//! Replays research/phase8-net/native-send-selector.json: original NetNode
//! send selector 0x763EE0.
use aa_netcode::send::{select_connection, SelectorConn};
use serde_json::Value;

#[test]
fn native_send_selector() {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-send-selector.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    let cases = v["cases"].as_array().unwrap();
    let mut selected = 0;
    for c in cases {
        let conns: Vec<SelectorConn> = c["conns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| SelectorConn {
                within_allowance: k["used"].as_i64().unwrap() <= k["allowance"].as_i64().unwrap(),
                wanted: k["admin"].as_bool().unwrap().then(|| k["wanted"].as_u64().unwrap() as u32),
                last_served: k["last_served"].as_i64().unwrap() as i32,
            })
            .collect();
        let got = select_connection(&conns);
        let want = c["selected"].as_u64().map(|x| x as usize);
        assert_eq!(got, want, "{c}");
        // 0x74BDE0 is asked exactly for the connections within their allowance that have an AdminConn.
        let asked: Vec<usize> = c["asked"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect();
        let expected: Vec<usize> = (0..conns.len()).filter(|&i| conns[i].within_allowance && conns[i].wanted.is_some()).collect();
        assert_eq!(asked, expected, "{c}");
        selected += usize::from(got.is_some());
    }
    assert_eq!(cases.len(), 400);
    assert_eq!(selected, 288);
}
