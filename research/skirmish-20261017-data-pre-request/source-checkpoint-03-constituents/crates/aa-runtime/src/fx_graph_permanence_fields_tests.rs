use super::*;
use serde_json::{Value, json};
use std::cell::RefCell;
const IMMEDIATE: [&[usize]; 8] = [&[2], &[4], &[], &[], &[6], &[], &[], &[]];
const CHAIN: [&[usize]; 8] = [&[3], &[5], &[], &[], &[], &[], &[7], &[]];
struct Lab<'a> {
    c: &'a Value,
    trace: RefCell<Vec<Value>>,
}
impl World for Lab<'_> {
    type Node = usize;
    fn is_vfx(&self, i: usize) -> Result<bool> {
        self.trace.borrow_mut().push(json!(["kind", i]));
        Ok(matches!(i, 2 | 3 | 5 | 7))
    }
    fn active(&self, i: usize) -> Result<bool> {
        self.trace.borrow_mut().push(json!(["active", i]));
        if self.c["stop"].as_u64() == Some(i as u64) {
            return Err(format!("required_{i}"));
        }
        Ok(self.c["target"].as_i64() == Some(i as i64) && self.c["duration"] == false)
    }
    fn immediate(&self, i: usize) -> Result<&[usize]> {
        Ok(IMMEDIATE[i])
    }
    fn chain(&self, i: usize) -> Result<&[usize]> {
        Ok(CHAIN[i])
    }
}
#[test]
fn native_recursive_permanence_short_circuit_and_required_fields() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-data-pre-request/native-animation-permanence.json"
    ))
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["input"];
        let w = Lab {
            c,
            trace: RefCell::new(vec![]),
        };
        let result = scan(&w, if c["empty"] == true { &[] } else { &[0, 1] });
        let (status, result) = match result {
            Ok(v) => ("ok".into(), Some(v)),
            Err(e) => (e, None),
        };
        assert_eq!(
            json!({"status":status,"result":result,"trace":w.trace.into_inner()}),
            row["output"],
            "{c}"
        );
    }
}
