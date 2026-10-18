use aa_runtime::film_paths;
use serde_json::Value;
fn words(value: &Value) -> Vec<u16> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_u64().unwrap() as u16)
        .collect()
}
#[test]
fn original_v14_all_pc_counted_paths_and_no_normalization() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-paths.json"
    ))
    .unwrap();
    let cases = data["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 120);
    for row in cases {
        let input = &row["input"];
        let mut left = words(&input["left"]);
        let right = words(&input["right"]);
        let result = if input["operation"] == "extension" {
            film_paths::append_extension(&mut left, &right);
            left
        } else {
            film_paths::join(&left, &right)
        };
        assert_eq!(result, words(&row["output"]), "{input}");
    }
}
