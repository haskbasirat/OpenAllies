use super::*;
fn append_input(
    s: &mut Scalar,
    key: &str,
    v: &serde_json::Value,
    i: &serde_json::Value,
    pc: Precision,
    trace: &mut Vec<String>,
) -> std::result::Result<(), String> {
    trace.push(key.to_owned());
    if i.get("stop_resource").and_then(|v| v.as_str()) == Some(key) {
        return Err(format!("required_resource_{key}"));
    }
    s.append(
        v["id"].as_str().unwrap(),
        v["weight"].as_u64().unwrap() as u32,
        pc,
    )
    .map_err(|_| "native_data_error".into())
}
#[test]
fn fresh_default_index_and_append_scalar_prefixes_match_native() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-object-aspect-source/native-object-aspect-source.json"
    ))
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let i = &row["input"];
        let n = &row["native"];
        let pc: Precision = serde_json::from_value(row["pc"].clone()).unwrap();
        let mut s = Scalar::new();
        assert_eq!(
            serde_json::json!({"count":s.ids.len(),"default":s.default,"construction_count":0}),
            n["constructor"]
        );
        s.variations = i["variations"].as_bool().unwrap();
        let mut outcome = Ok(());
        let mut trace = Vec::new();
        if !s.variations {
            if i["normal"].is_null() {
                outcome = Err("native_data_error".to_string());
            } else {
                outcome = append_input(&mut s, "normal", &i["normal"], i, pc, &mut trace);
                if outcome.is_ok() {
                    s.normal_finished().unwrap();
                }
            }
        }
        if outcome.is_ok() {
            for (index, a) in i["others"].as_array().unwrap().iter().enumerate() {
                outcome = append_input(&mut s, &index.to_string(), a, i, pc, &mut trace);
                if outcome.is_err() {
                    break;
                }
            }
        }
        assert_eq!(
            outcome.err().unwrap_or_else(|| "complete".into()),
            n["outcome"].as_str().unwrap(),
            "{row}"
        );
        assert_eq!(serde_json::json!(s.ids), n["aspects"], "{row}");
        assert_eq!(serde_json::json!(trace), n["resource_trace"], "{row}");
        assert_eq!(s.default as u64, n["default"].as_u64().unwrap(), "{row}");
        assert_eq!(s.total as u64, n["total"].as_u64().unwrap(), "{row}");
    }
}
