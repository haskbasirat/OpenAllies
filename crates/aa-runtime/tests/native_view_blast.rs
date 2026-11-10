//! 0x43B910 against research/phase8-combat/native-view-blast-fixtures.json:
//! the same virtual +9C / +38 calls and 0x455510 impulses, in order, at
//! PC24, PC53 and PC64.
use aa_numeric::{Fast, Precision};
use aa_runtime::view_blast_impulse::{BlastObjects, blast_impulse};
use aa_trig::fast_normalize::sqrt_table;
use serde_json::{Value, json};

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

struct List<'a> {
    case: &'a Value,
    events: Vec<Value>,
}

impl BlastObjects for List<'_> {
    type Object = usize;
    fn first(&mut self) -> Option<usize> {
        let empty = self.case["empty_holder"].as_bool().unwrap();
        (!empty && !self.case["nodes"].as_array().unwrap().is_empty()).then_some(0)
    }
    fn next(&mut self, object: usize) -> Option<usize> {
        (object + 1 < self.case["nodes"].as_array().unwrap().len()).then_some(object + 1)
    }
    fn kind(&mut self, object: usize) -> u32 {
        word(&self.case["nodes"][object]["kind"])
    }
    fn reacts(&mut self, object: usize) -> bool {
        self.events.push(json!({"reacts": object}));
        word(&self.case["nodes"][object]["reacts"]) & 0xff != 0
    }
    fn position(&mut self, object: usize) -> [u32; 2] {
        let p = &self.case["nodes"][object]["position"];
        [word(&p[0]), word(&p[1])]
    }
    fn radius(&mut self, object: usize) -> Fast {
        self.events.push(json!({"radius": object}));
        Fast::from_f64_bits(self.case["nodes"][object]["radius"].as_u64().unwrap()).unwrap()
    }
    fn apply(&mut self, object: usize, impulse: [u32; 3], word: u32) {
        self.events.push(json!({"apply": object, "impulse": impulse, "word": word}));
    }
}

#[test]
fn blast_impulse_matches_native_at_every_precision() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-view-blast-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let table = sqrt_table().unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 300);
    let mut applied = 0;
    for (index, case) in cases.iter().enumerate() {
        let center = [word(&case["center"][0]), word(&case["center"][1])];
        for (name, precision) in [
            ("Single24", Precision::Single24),
            ("Double53", Precision::Double53),
            ("Extended64", Precision::Extended64),
        ] {
            let mut list = List { case, events: Vec::new() };
            blast_impulse(
                &mut list,
                center,
                word(&case["strength"]),
                word(&case["blast_radius"]),
                &table,
                precision,
            )
            .unwrap();
            assert_eq!(json!(list.events), case["events"][name], "case {index} at {name}");
            applied += list.events.iter().filter(|e| e.get("apply").is_some()).count();
        }
    }
    assert!(applied > 100);
}
