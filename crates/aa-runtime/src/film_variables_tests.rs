use super::*;
use crate::localization::Text;
use aa_rules::{actors::ActorCatalog, sources::SourceLimits};
use serde_json::Value;

#[test]
fn original_v14_separate_text_current_bool_setter_preserves_source_default() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-separate-variable.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 48);
    for row in rows {
        let mut state = State {
            path_films: Vec::new(),
            last_film_name: Vec::new(),
            rotation: 5,
            last_index: 0,
            map_save_compressed: 1,
            editor_map_strings_separate: row["input"]["before"].as_u64().unwrap() as u8,
            source_files: Vec::new(),
        };
        state.assign_separate_map_strings_byte(row["input"]["requested"].as_u64().unwrap() as u8);
        assert_eq!(
            u64::from(state.separate_map_strings_byte()),
            row["output"]["current"]
        );
        //The reached current owner does not overwrite native registered
        //default byte1 or any source declaration when backend policy writes0.
        assert_eq!(row["output"]["default"], 0);
        assert_eq!(row["output"]["type"], 1);
        assert_eq!(row["output"]["flags"], 0x10);
    }
}

#[test]
fn original_v14_all_pc_full_rotation_names_and_same_current_index() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-rotation.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 27);
    for row in cases {
        let input = &row["input"];
        let mut text = Localization::default();
        text.declarations.insert(
            "last_film_filename".into(),
            vec![Text::new(
                Provenance {
                    path: "explicit/native-fixture".into(),
                    sha256: "".into(),
                    span: aa_rules::tgi::Span { start: 0, end: 0 },
                },
                Ok(input["localized"]
                    .as_str()
                    .unwrap()
                    .encode_utf16()
                    .collect()),
            )],
        );
        let mut state = State {
            path_films: Vec::new(),
            last_film_name: input["fallback"].as_str().unwrap().encode_utf16().collect(),
            rotation: input["queue"].as_i64().unwrap() as i32,
            last_index: input["last"].as_i64().unwrap() as i32,
            map_save_compressed: 1,
            editor_map_strings_separate: 1,
            source_files: Vec::new(),
        };
        let mut name: Vec<u16> = input["old_name"].as_str().unwrap().encode_utf16().collect();
        state
            .filename(&mut name, input["rotate"].as_u64().unwrap() as u8, &text)
            .unwrap();
        assert_eq!(serde_json::json!(name), row["output"]["name_utf16"]);
        assert_eq!(
            u32::from_ne_bytes(state.last_index.to_ne_bytes()) as u64,
            row["output"]["last_index"].as_u64().unwrap()
        );
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT original archive and v14 overlay"]
fn actual_current_source_vars_no_profile_and_mutation_persistence() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(crate::settings::Settings::load_v14(&mut sources, Limits::default()).unwrap());
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    assert!(State::from_fresh_sources(&catalog, &mut sources, NativeOverrides::Unbound).is_err());
    let mut state =
        State::from_fresh_sources(&catalog, &mut sources, NativeOverrides::ControlledNone).unwrap();
    assert_eq!(state.rotation_queue_size(), 5);
    assert_eq!(state.last_index(), 0);
    assert_eq!(state.compressed_byte(), 1);
    assert_eq!(state.separate_map_strings_byte(), 1);
    state.assign_separate_map_strings_byte(0);
    assert_eq!(state.separate_map_strings_byte(), 0);
    state.assign_separate_map_strings_byte(0x80);
    assert_eq!(state.separate_map_strings_byte(), 0x80);
    assert_eq!(String::from_utf16(state.path_films()).unwrap(), "Films/");
    let mut name = Vec::new();
    for i in 0..8 {
        let expected = format!("LastFilm_{}", i % 5);
        assert_eq!(
            String::from_utf16(state.filename(&mut name, 1, &text).unwrap()).unwrap(),
            expected
        );
    }
    state.assign_compressed_byte(0x80);
    assert_eq!(state.compressed_byte(), 0x80);
    state.assign_last_index(-1);
    state.assign_rotation_queue_size(2);
    assert_eq!(
        String::from_utf16(state.filename(&mut name, 1, &text).unwrap()).unwrap(),
        "LastFilm_0"
    );
    assert_eq!(state.last_index(), 1);
    state.assign_rotation_queue_size(0);
    let before = name.clone();
    assert!(state.filename(&mut name, 1, &text).is_err());
    assert_eq!(name, before);
    assert_eq!(state.last_index(), 1);
    //Public Settings changes cannot bless a different private source assignment.
    catalog
        .settings
        .as_mut()
        .unwrap()
        .app_variables
        .get_mut("filmrotationqueuesize")
        .unwrap()[0]
        .raw = "0".into();
    assert!(
        State::from_fresh_sources(&catalog, &mut sources, NativeOverrides::ControlledNone).is_err()
    );
}
