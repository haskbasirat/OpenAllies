use super::*;
use serde_json::Value;
fn word(row: &Value, key: &str) -> u32 {
    row["fields"][key].as_u64().unwrap() as u32
}
fn text(row: &Value, key: &str) -> Vec<u16> {
    row["strings"][key]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u16)
        .collect()
}

#[test]
fn game_settings_value_replays_actual_outer_copy_all_pcs_and_images() {
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261016-local-bootstrap/game-settings/native-game-settings.json")).unwrap();
    assert_eq!(fixture["passed"], true);
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 18);
    for row in fixture["cases"].as_array().unwrap() {
        let mut source = Fields {
            words_0c_10: [word(row, "0c"), word(row, "10")],
            word_14: Some(word(row, "14")),
            words_18_1c: [word(row, "18"), word(row, "1c")],
            name_key: text(row, "20"),
            name_text: Some(text(row, "24")),
            description_key: text(row, "28"),
            description_text: Some(text(row, "2c")),
            display_extents: Some([word(row, "30"), word(row, "34")]),
            string_38: text(row, "38"),
            string_3c: text(row, "3c"),
            word_40: Some(word(row, "40")),
            entries: Vec::new(),
        };
        let copied = source.copy_value();
        assert_eq!(copied.words_0c_10, [word(row, "0c"), word(row, "10")]);
        assert_eq!(copied.word_14, Some(word(row, "14")));
        assert_eq!(copied.words_18_1c, [word(row, "18"), word(row, "1c")]);
        assert_eq!(
            copied.display_extents,
            Some([word(row, "30"), word(row, "34")])
        );
        assert_eq!(copied.name_key, text(row, "20"));
        assert_eq!(copied.name_text, Some(text(row, "24")));
        assert_eq!(copied.description_key, text(row, "28"));
        assert_eq!(copied.description_text, Some(text(row, "2c")));
        assert_eq!(copied.string_38, text(row, "38"));
        assert_eq!(copied.string_3c, text(row, "3c"));
        assert_eq!(copied.word_40, Some(word(row, "40")));
        assert_eq!(row["uncopied_50_64"], true);
        if !source.name_key.is_empty() {
            assert_ne!(source.name_key.as_ptr(), copied.name_key.as_ptr());
        }
        source.name_key.push(0xffff);
        source.name_text.as_mut().unwrap().clear();
        source.words_0c_10[0] = 0;
        assert_eq!(copied.name_key, text(row, "20"));
        assert_eq!(copied.name_text, Some(text(row, "24")));
        assert_eq!(copied.words_0c_10[0], word(row, "0c"));
    }
}

#[test]
fn game_settings_unknown_native_words_remain_unbound_after_copy() {
    let fields = Fields {
        words_0c_10: [1, 2],
        word_14: None,
        words_18_1c: [3, 4],
        name_key: vec![65],
        name_text: None,
        description_key: vec![66],
        description_text: None,
        display_extents: None,
        string_38: vec![],
        string_3c: vec![],
        word_40: None,
        entries: vec![],
    };
    let copied = fields.copy_value();
    assert!(copied.word_14.is_none());
    assert!(copied.name_text.is_none());
    assert!(copied.description_text.is_none());
    assert!(copied.word_40.is_none());
    assert!(copied.display_extents.is_none());
}
