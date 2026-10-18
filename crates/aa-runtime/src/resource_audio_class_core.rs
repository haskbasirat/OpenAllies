//!The integer branch of native59CB00/59C360. Missing is a genuine absent
//!variable lookup; a present zero/negative value is never replaced by10.
pub(crate) fn weights(current: Option<i32>) -> Result<(i32, i32), String> {
    let ram = current.unwrap_or(10);
    if ram <= 0 {
        return Err(
            "Audio ResourceCache weight lies outside the proved positive constructor domain".into(),
        );
    }
    Ok((ram, 10))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_native_audio_class_current_and_cached_weights() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/development10/resource-audio-class/native-audio-class.json"
        ))
        .unwrap();
        for row in doc["rows"].as_array().unwrap() {
            let current = row["ram_current"].as_i64().map(|v| v as i32);
            let (ram, vram) = weights(current).unwrap();
            assert_eq!(row["ram_weight"], ram);
            assert_eq!(row["vram_weight"], vram);
            assert_eq!(row["ram_total"], ram);
            assert_eq!(row["vram_total"], vram);
            assert_eq!(row["class_count"], 1);
            assert_eq!(row["cached_lookup_count"], 0);
            assert_eq!(row["cached_same_allocation"], true);
            assert_eq!(row["class_name"], "KKC_AudioResource");
        }
        assert!(weights(Some(0)).is_err());
        assert!(weights(Some(-1)).is_err());
    }
}
