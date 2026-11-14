//! 4E16F0 against original x86 executions (research/kingdom-bases).
use aa_economy::kingdom_structure_list::{Removal, remove_structure};
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/kingdom-bases/native-kingdom-structure-list.json"
    ))
    .unwrap()
}

fn words(v: &Value) -> Vec<u32> {
    v.as_array().unwrap().iter().map(|w| w.as_u64().unwrap() as u32).collect()
}

#[test]
fn found_absent_and_empty_removals_match_both_images_all_precisions() {
    let doc = fixture();
    let guard = doc["guard"].as_u64().unwrap() as u32;
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 126);
    for row in rows {
        let mut list = words(&row["input"]["words"]);
        let value = row["input"]["value"].as_u64().unwrap() as u32;
        let removal = remove_structure(&mut list, value);
        let count = row["count"].as_i64().unwrap();
        let below = row["below"].as_u64().unwrap() as u32;
        match removal {
            Removal::EmptyUnderflow => assert_eq!(count, -1, "{row}"),
            Removal::DroppedFirst { below: word } => assert_eq!(below, word, "{row}"),
            Removal::Removed(_) => assert_eq!(below, guard, "{row}"),
        }
        if count >= 0 {
            assert_eq!(list.len() as i64, count, "{row}");
            assert_eq!(list[..], words(&row["words"])[..list.len()], "{row}");
        }
        assert_eq!(row["above"].as_u64().unwrap() as u32, guard, "{row}");
    }
}
