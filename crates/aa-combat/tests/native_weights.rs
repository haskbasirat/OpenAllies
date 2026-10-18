use aa_body::F32;
use aa_combat::{
    admission::Filter,
    weights::{self, Entry},
};
use aa_numeric::Precision;
use serde_json::Value;
#[test]
fn native_other_ability_weight_matches() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-performer/native-weight-fixtures.json"
    ))
    .unwrap();
    for row in f["cases"].as_array().unwrap() {
        let i = &row["input"];
        let entries: Vec<Entry> = serde_json::from_value(i["entries"].clone()).unwrap();
        let excluded: Option<Filter> = serde_json::from_value(i["excluded"].clone()).unwrap();
        let actor: [F32; 3] = serde_json::from_value(i["actor_bonuses"].clone()).unwrap();
        let p: Precision = serde_json::from_value(i["precision"].clone()).unwrap();
        let actual = weights::maximum_other(&entries, excluded, actor, p)
            .unwrap()
            .store_f32()
            .unwrap();
        assert_eq!(
            u64::from(actual),
            row["output"].as_u64().unwrap(),
            "{}",
            i["name"]
        );
    }
}
