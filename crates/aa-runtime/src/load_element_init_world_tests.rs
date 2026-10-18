use super::*;
#[test]
fn native_element_rate_store_matches_all_192_whole_function_rows() {
    let doc: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/development10/native-element-init.json"
    ))
    .unwrap();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 192);
    for row in rows {
        let input = &row["input"];
        let mut modifier = ModifierState::new(0);
        modifier.scalars[(0x3c / 4 - 1) as usize] =
            F32(input["multiplier"].as_u64().unwrap() as u32);
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => unreachable!(),
        };
        assert_eq!(
            u64::from(
                rate(
                    input["base"].as_u64().unwrap() as u32,
                    input["context"].as_bool().unwrap().then_some(&modifier),
                    pc
                )
                .unwrap()
            ),
            row["native"]["rate"],
            "{row}"
        );
    }
}
