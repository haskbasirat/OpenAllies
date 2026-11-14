//! 0x516B30 against original x86 executions (research/aircraft-rotate),
//! including the startup-built trig tables and every x87 precision.
use aa_movement::aircraft_rotate::turn_vector;
use aa_numeric::Precision;
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/aircraft-rotate/native-aircraft-rotate.json"
    ))
    .unwrap()
}

fn precision(row: &Value) -> Precision {
    match row["precision"].as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("{other}"),
    }
}

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

#[test]
fn turned_vector_bits_match_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1800);
    for row in rows {
        let input = &row["input"];
        let v = [word(&input["vector"][0]), word(&input["vector"][1])];
        let out = turn_vector(v, word(&input["angle"]), precision(row)).unwrap();
        assert_eq!(out, [word(&row["out"][0]), word(&row["out"][1])], "{row}");
    }
}
