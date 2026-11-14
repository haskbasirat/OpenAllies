//! 51D410 against original x86 executions (research/cai-behavior).
use aa_mind::cai_behavior::{self, World};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/cai-behavior/native-cai-behavior.json"
    ))
    .unwrap()
}

struct Owner<'a> {
    input: &'a Value,
    calls: Vec<Value>,
}
impl World for Owner<'_> {
    type Error = ();
    fn group4_word_24(&mut self) -> Result<Option<u32>, ()> {
        // The native lookup is 4D49B0 on the state's +4 owner with group 4.
        self.calls.push(json!(["group4", true, 4]));
        Ok(self.input["word_24"].as_u64().map(|w| w as u32))
    }
    fn organization_byte_8d(&mut self) -> Result<u8, ()> {
        Ok(self.input["byte_8d"].as_u64().unwrap() as u8)
    }
}

#[test]
fn every_descriptor_word_and_organization_byte_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 324);
    for row in rows {
        let mut owner = Owner { input: &row["input"], calls: vec![] };
        let al = cai_behavior::behavior(&mut owner).unwrap();
        assert_eq!(u64::from(al), row["al"].as_u64().unwrap(), "{row}");
        assert_eq!(json!(owner.calls), row["calls"], "{row}");
    }
}
