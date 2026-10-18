use super::*;
use serde_json::{json, Value};

struct Memory {
    words: Vec<u32>,
    reads: Vec<u32>,
    writes: Vec<[u32; 2]>,
    fail: Option<usize>,
}
impl Fields for Memory {
    fn word(&mut self, offset: u32) -> Result<u32> {
        self.reads.push(offset);
        if self.fail == Some(self.reads.len()) {
            return Err("required field".into());
        }
        Ok(self.words[offset as usize / 4])
    }
    fn put(&mut self, offset: u32, word: u32) {
        self.writes.push([offset, word]);
        self.words[offset as usize / 4] = word;
    }
}

#[test]
fn native_mode2_matrix_plane_wrapping_counter_and_every_required_read_prefix() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/development10/view-environment-projection/native-projection.json"
    )).unwrap();
    let mut count = 0;
    for row in doc["rows"].as_array().unwrap() {
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => panic!(),
        };
        let mut fields = Memory {
            words: serde_json::from_value(row["input"]["words"].clone()).unwrap(),
            reads: vec![], writes: vec![],
            fail: row["input"]["fail"].as_u64().map(|n| n as usize),
        };
        let result = update_mode2_86ba10(&mut fields, pc);
        assert_eq!(if result.is_ok() { "ok" } else { "required field" },
            row["output"]["outcome"].as_str().unwrap(), "row{count}: {result:?}");
        assert_eq!(json!(fields.reads), row["output"]["reads"], "reads row{count}");
        assert_eq!(json!(fields.writes), row["output"]["writes"], "writes row{count}");
        assert_eq!(json!(fields.words), row["output"]["words"], "words row{count}");
        count += 1;
    }
    assert_eq!(count, 153);
}

#[test]
fn other_projection_mode_refuses_before_any_numeric_field_or_write() {
    let mut fields = Memory { words: vec![0; 100], reads: vec![], writes: vec![], fail: None };
    fields.words[0x164 / 4] = 3;
    assert!(update_mode2_86ba10(&mut fields, Precision::Double53).is_err());
    assert_eq!(fields.reads, [0x164]);
    assert!(fields.writes.is_empty());
}
