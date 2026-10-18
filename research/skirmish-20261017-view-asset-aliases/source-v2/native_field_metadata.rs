//!Metadata-only successor wrapper; source identity remains the caller's task.
use aa_rules::{
    actors::native_fresh_field_order,
    tgi::{Document, Item, Limits},
};
#[test]
fn metadata_order_matches_real_native_fresh_field_fixture() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-svar-field-order/native-field-order.json"
    ))
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let raw = row["source_hex"].as_str().unwrap();
        let bytes: Vec<u8> = (0..raw.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).unwrap())
            .collect();
        let doc = Document::parse_native(bytes, Limits::default()).unwrap();
        let node = doc
            .items
            .iter()
            .find_map(|v| if let Item::Node(n) = v { Some(n) } else { None })
            .unwrap();
        let names = node
            .items
            .iter()
            .filter_map(|v| {
                if let Item::Property(p) = v {
                    Some(doc.property_name(p).unwrap())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let actual = native_fresh_field_order(&names)
            .unwrap()
            .into_iter()
            .map(|i| names[i])
            .collect::<Vec<_>>();
        let expected = row["native"]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
    assert!(native_fresh_field_order(&["Same", "same"]).is_err());
}
