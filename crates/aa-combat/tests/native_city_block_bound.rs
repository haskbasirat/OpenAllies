//! 51D560 against original x86 executions (research/target-rules).
use aa_combat::Result;
use aa_combat::target_rules::{GtidTable, city_block_bound};
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/target-rules/native-city-block-bound.json"
    ))
    .unwrap()
}

struct Table<'a> {
    input: &'a Value,
    index: u16,
}
impl GtidTable for Table<'_> {
    fn generation(&mut self, index: u16) -> Result<u16> {
        assert_eq!(index, self.index);
        Ok(self.input["table_generation"].as_u64().unwrap() as u16)
    }
    fn slot_live(&mut self, index: u16) -> Result<bool> {
        assert_eq!(index, self.index);
        Ok(self.input["live"].as_bool().unwrap())
    }
}

#[test]
fn live_stale_and_null_slots_match_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1080);
    for row in rows {
        let input = &row["input"];
        let handle = input["handle"].as_u64().unwrap() as u32;
        let mut table = Table { input, index: handle as u16 };
        let bound = city_block_bound(&mut table, handle).unwrap();
        assert_eq!(u64::from(bound), row["al"].as_u64().unwrap(), "{row}");
    }
}
