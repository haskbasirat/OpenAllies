use aa_mapworld::recruitment::{ItemState, Recruitment};
use serde_json::{Value, json};
fn hex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn actual_native_recruit_item_and_list_loads_match() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-recruitment/native-recruitment.json"
    ))
    .unwrap();
    let cases = data["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 68);
    for case in cases {
        let bytes = hex(case["hex"].as_str().unwrap());
        let (list, consumed) = Recruitment::decode_prefix(&bytes).unwrap();
        assert_eq!(consumed as u64, case["consumed"].as_u64().unwrap());
        let items:Vec<_>=list.items.iter().map(|item| {
            let (kind,words)=match &item.state {
                ItemState::Single(s)=>(0,vec![s.word_0c,s.word_10,s.word_14,s.word_18,s.word_1c]),
                ItemState::Group{word_0c,..}=>(1,vec![*word_0c]),
            };
            json!({"kind":kind,"property_id":item.base.property_id,"enabled":item.base.enabled,"words":words})
        }).collect();
        assert_eq!(
            serde_json::to_value(items).unwrap(),
            case["items"],
            "{}",
            case["name"]
        );
        if let Some(word) = list.word_24 {
            assert_eq!(word as u64, case["word_24"].as_u64().unwrap());
        }
    }
}
#[test]
fn recruit_truncation_and_unknown_types_are_rejected_without_boundary_scans() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-recruitment/native-recruitment.json"
    ))
    .unwrap();
    let case = data["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["items"].as_array().unwrap().len() == 3)
        .unwrap();
    let bytes = hex(case["hex"].as_str().unwrap());
    for length in 0..bytes.len() {
        assert!(
            Recruitment::decode_prefix(&bytes[..length]).is_err(),
            "{length}"
        );
    }
    let mut wrong = bytes;
    wrong[8..12].copy_from_slice(&2_u32.to_le_bytes());
    assert!(Recruitment::decode_prefix(&wrong).is_err());
}
