use super::*;
use serde_json::{Value, json};
struct Probe {
    input: Value,
    reads: Vec<&'static str>,
    result: Option<Value>,
}
impl Fields for Probe {
    type Error = &'static str;
    fn word138(&mut self) -> Result<u32, Self::Error> {
        self.reads.push("word138");
        Ok(self.input["word138"].as_u64().unwrap() as u32)
    }
    fn structure(&mut self) -> Result<bool, Self::Error> {
        self.reads.push("structure");
        Ok(self.input["structure"].as_bool().unwrap())
    }
    fn remote_range(&mut self) -> Result<u32, Self::Error> {
        self.reads.push("remote_range");
        Ok(self.input["remote"].as_u64().unwrap() as u32)
    }
    fn attached_path(&mut self) -> Result<(), Self::Error> {
        self.result = Some(json!({"kind":"required_attached"}));
        Err("required attached child")
    }
    fn radius(&mut self) -> Result<u32, Self::Error> {
        self.reads.push("radius");
        Ok(self.input["radius"].as_u64().unwrap() as u32)
    }
    fn flags(&mut self) -> Result<u32, Self::Error> {
        self.reads.push("flags");
        Ok(self.input["flags"].as_u64().unwrap() as u32)
    }
    fn positive(&mut self, r: u32) -> Result<bool, Self::Error> {
        assert!(f32::from_bits(r).is_finite());
        Ok(f32::from_bits(r) > 0.0)
    }
    fn zone_change(&mut self, enabled: bool, radius: u32) -> Result<(), Self::Error> {
        self.result = Some(json!({"kind":"zone","enabled":enabled,"radius":radius}));
        Err("required zone child")
    }
}
#[test]
fn native_56ec90_all96_front_dispatch_rows_preserve_lazy_semantic_read_order() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/development10/stored-detection-init/native-after-fields.json"
    ))
    .unwrap();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 96);
    for row in rows {
        let mut world = Probe {
            input: row["input"].clone(),
            reads: vec![],
            result: None,
        };
        let outcome = after(&mut world);
        assert_eq!(json!(world.reads), row["native"]["reads"], "{row}");
        let actual = world.result.unwrap_or_else(|| json!({"kind":"unchanged"}));
        let mut expected = row["native"]["result"].clone();
        expected.as_object_mut().unwrap().remove("entry");
        assert_eq!(actual, expected, "{row}");
        assert_eq!(outcome.is_ok(), actual["kind"] == "unchanged");
    }
}
