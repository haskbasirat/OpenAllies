//!Native48A440/48A540 registered enum names consumed by47DC50.
fn lookup(value: &str, names: &[&str]) -> Result<u32, String> {
    names
        .iter()
        .position(|s| s.eq_ignore_ascii_case(value))
        .map(|i| i as u32)
        .ok_or_else(|| format!("native ActorState enum value is invalid: {value}"))
}
pub(crate) fn formation(value: &str) -> Result<u32, String> {
    lookup(value, &["DESIRED", "BROKEN", "COMBAT", "RETREAT"])
}
pub(crate) fn behavior(value: &str) -> Result<u32, String> {
    lookup(value, &["PASSIVE", "AGGRESSIVE", "DESIRED"])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_native_enum_construction_and_lookup() {
        let doc:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-stored-organization-init-draft/native-state-enums.json")).unwrap();
        let mut rows = 0;
        for machine in doc["machines"].as_array().unwrap() {
            for row in machine["queries"].as_array().unwrap() {
                let value = row["name"].as_str().unwrap();
                let actual = if row["kind"] == "formation" {
                    formation(value)
                } else {
                    behavior(value)
                };
                assert_eq!(
                    actual.ok(),
                    row["value"].as_u64().map(|i| i as u32),
                    "{row}"
                );
                rows += 1;
            }
        }
        assert_eq!(rows, 108);
    }
}
