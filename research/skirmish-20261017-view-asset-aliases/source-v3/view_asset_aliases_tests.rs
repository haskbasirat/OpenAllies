use super::*;
use serde_json::Value;
fn fixture() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261017-view-asset-aliases/native-aliases-v3.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
#[test]
fn unsupported_alias_source_expressions_remain_explicit() {
    for raw in [
        "a b",
        "\"unclosed",
        "\"a\" trailing",
        "path\\escape",
        "a&b",
        "\u{e9}",
        "a\0b",
    ] {
        assert!(literal(raw).is_err(), "{raw:?}");
    }
    assert_eq!(
        literal("\"VFX/Test\"").unwrap(),
        "VFX/Test".encode_utf16().collect::<Vec<_>>()
    );
}
#[test]
#[ignore = "requires real source-owned Catalog/Bank and Application V14Sources"]
fn actual_current_aliases_native_source_and_moved_bank_owner() {
    use crate::combat_source_receipt::VerifiedStartupReceipt;
    use aa_rules::{actors::ActorCatalog, sources::SourceLimits};
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
    let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
    crate::combat_source_completion::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    let native = fixture();
    assert_eq!(native["rows"].as_array().unwrap().len(), 3);
    let mut state =
        State::from_fresh_sources_retained(&catalog, &mut files, NativeOverrides::ControlledNone)
            .unwrap();
    for row in native["rows"].as_array().unwrap() {
        let n = &row["native"];
        assert_eq!(n["constructor_count"], 125);
        assert_eq!(n["constructor_suffix"]["index"], 50);
        assert_eq!(n["constructor_suffix"]["kind"], 4);
        assert_eq!(n["constructor_suffix"]["flags"], 0);
        assert_eq!(n["constructor_suffix"]["value"], "-h");
        assert!(n["constructor_names"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| !keys()
                .filter(|k| *k != "RendererHBPPSuffix")
                .any(|k| Some(k) == v.as_str())));
        let bank = catalog.definition_bank.borrow();
        for expected in n["copies"].as_array().unwrap() {
            let key = expected["name"].as_str().unwrap();
            let text = state.alias_410cb0(&catalog, &bank, key).unwrap();
            assert_eq!(
                text.units(),
                expected["value"]
                    .as_str()
                    .unwrap()
                    .encode_utf16()
                    .collect::<Vec<_>>()
            );
            assert_eq!(state.applied_source_assignments(key).len(), 1);
        }
        let expected_files = n["stages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                (
                    s["file"].as_str().unwrap().to_string(),
                    s["sha256"].as_str().unwrap().to_string(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(state.source_files(), expected_files);
        for alias in n["stages"].as_array().unwrap().last().unwrap()["aliases"]
            .as_array()
            .unwrap()
        {
            assert_eq!(alias["kind"], 4);
            assert_eq!(alias["flags"], 0);
        }
    }
    assert!(
        State::from_fresh_sources_retained(&catalog, &mut files, NativeOverrides::Unbound)
            .unwrap_err()
            .state
            .is_none()
    );
    //A later malformed public projection retains every prior real source store.
    let last = native["rows"][0]["native"]["stages"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["aliases"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()["name"]
        .as_str()
        .unwrap()
        .to_ascii_lowercase();
    let original = catalog
        .settings
        .as_mut()
        .unwrap()
        .app_variables
        .get_mut(&last)
        .unwrap()[0]
        .raw
        .clone();
    catalog
        .settings
        .as_mut()
        .unwrap()
        .app_variables
        .get_mut(&last)
        .unwrap()[0]
        .raw = "forged".into();
    assert!(state
        .alias_410cb0(&catalog, &catalog.definition_bank.borrow(), "TextureCircle")
        .is_err());
    let failure =
        State::from_fresh_sources_retained(&catalog, &mut files, NativeOverrides::ControlledNone)
            .unwrap_err();
    assert!(failure.error.contains("public projection differs"));
    let partial = failure.state.unwrap();
    assert!(keys().any(|k| !partial.applied_source_assignments(k).is_empty()));
    assert!(partial
        .validate_in_bank(&catalog, &catalog.definition_bank.borrow())
        .unwrap_err()
        .contains("incomplete"));
    catalog
        .settings
        .as_mut()
        .unwrap()
        .app_variables
        .get_mut(&last)
        .unwrap()[0]
        .raw = original;
    //Move the actual Bank once; no empty Catalog fallback or reinstallation.
    let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    state.validate_in_bank(&catalog, &bank).unwrap();
    assert!(state
        .validate_in_bank(&catalog, &catalog.definition_bank.borrow())
        .is_err());
    let old = state
        .alias_410cb0(&catalog, &bank, "TextureCircle")
        .unwrap();
    let ptr = old.units().as_ptr();
    let replacement = vec![u16::from(b'Q'), 0, 0xd800];
    state
        .assign(&catalog, &bank, "TextureCircle", &replacement)
        .unwrap();
    assert_eq!(old.units().as_ptr(), ptr);
    assert_eq!(
        old.units(),
        native["rows"][0]["native"]["lifetime"]["old_value"]
            .as_str()
            .unwrap()
            .encode_utf16()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        state
            .alias_410cb0(&catalog, &bank, "TextureCircle")
            .unwrap()
            .units(),
        replacement
    );
    assert_eq!(
        native["rows"][0]["native"]["lifetime"]["new_value"],
        serde_json::json!(replacement)
    );
    assert!(state
        .alias_410cb0(&catalog, &bank, "UnprovedOtherAVar")
        .is_err());
    println!("23 current source aliases matched3 native PC rows; same moved Bank, counted temporary and retained failure verified");
}
