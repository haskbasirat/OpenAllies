//! Replays research/phase8-net/native-sync.json: original TGC_Synchronizer
//! constructor, loggers, reset and ValidateChecksum sequences.
use aa_orders::net::sync::Synchronizer;
use serde_json::Value;

#[test]
fn native_synchronizer_sequences() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-sync.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut ops_checked = 0;
    for seq in v["sequences"].as_array().unwrap() {
        let mut s = Synchronizer::new(0);
        let mut gate = true;
        for op in seq.as_array().unwrap() {
            let i = |k: &str| op[k].as_i64().unwrap();
            match op["op"].as_str().unwrap() {
                "new" => s = Synchronizer::new(i("max") as i32),
                "reset" => s.reset(i("index") as i32, i("hash") as i32),
                "int" => s.log_int(gate, "x", i("value") as u32),
                "float" => s.log_float_bits(gate, "x", i("bits") as u32),
                "bool" => s.log_bool(gate, "x", i("value") == 1),
                "text" => {
                    let units: Vec<u16> = serde_json::from_value(op["units"].clone()).unwrap();
                    s.log_text(gate, "x", &units)
                }
                "validate" => {
                    let r = s.validate(i("index") as i32, i("hash") as i32);
                    assert_eq!(r, op["result"].as_bool().unwrap(), "{op}");
                }
                "suspend" => s.suspend = i("value") as i32,
                "thread_gate" => gate = op["open"].as_bool().unwrap(),
                other => panic!("{other}"),
            }
            let st = &op["state"];
            let native_entries: Vec<i64> = st["entries"].as_array().unwrap().iter().map(|e| e["hash"].as_i64().unwrap()).collect();
            let rust_entries: Vec<i64> = (s.begin()..s.end()).map(|k| i64::from(s.entry(k).unwrap().hash)).collect();
            assert_eq!(
                (s.enabled as i64, s.suspend as i64, s.hash as i64, s.begin() as i64, s.end() as i64, &rust_entries),
                (
                    st["enabled"].as_i64().unwrap(),
                    st["suspend"].as_i64().unwrap(),
                    st["hash"].as_i64().unwrap(),
                    st["begin"].as_i64().unwrap(),
                    st["end"].as_i64().unwrap(),
                    &native_entries
                ),
                "{op}"
            );
            ops_checked += 1;
        }
    }
    assert!(ops_checked > 1000);
}
