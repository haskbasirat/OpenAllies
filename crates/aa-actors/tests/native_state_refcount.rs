//! Replays research/phase8-actor-creation/native-state-refcount.json
//! (original 4D7D30 / 4D7D70 under Unicorn) against
//! aa_actors::state_refcount.
use aa_actors::{
    state::Result,
    state_refcount::{StateHoldWorld, hold, release},
};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-actor-creation/native-state-refcount.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

struct Host {
    count: i32,
    rewrites: Vec<Value>,
    events: Vec<Value>,
}
impl StateHoldWorld for Host {
    fn hold_count(&mut self) -> Result<i32> {
        Ok(self.count)
    }
    fn set_hold_count(&mut self, value: i32) -> Result<()> {
        self.count = value;
        Ok(())
    }
    fn set_state(&mut self, index: u8, enabled: bool) -> Result<()> {
        self.events.push(
            json!({"event": "set_state", "state": index, "enabled": u8::from(enabled)}),
        );
        if !self.rewrites.is_empty() {
            let v = self.rewrites.remove(0);
            if let Some(v) = v.as_i64() {
                self.count = v as i32;
            }
        }
        Ok(())
    }
}

#[test]
fn native_state_refcount_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 300);
    assert_eq!(f["coverage"]["4D7D30"], json!([16, 16]));
    assert_eq!(f["coverage"]["4D7D70"], json!([10, 10]));
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let mut host = Host {
            count: c["count"].as_i64().unwrap() as i32,
            rewrites: c["rewrites"].as_array().unwrap().clone(),
            events: vec![],
        };
        let mut counts = vec![];
        for op in c["ops"].as_array().unwrap() {
            if op == "hold" {
                hold(&mut host).unwrap();
            } else {
                release(&mut host).unwrap();
            }
            counts.push(host.count);
        }
        assert_eq!(json!(host.events), o["events"], "{name}");
        assert_eq!(json!(counts), o["counts"], "{name}");
    }
}
