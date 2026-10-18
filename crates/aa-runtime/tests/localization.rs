use aa_rules::{
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use aa_runtime::localization::Localization;
fn file(text: &str) -> ResolvedFile {
    let bytes = text.as_bytes().to_vec();
    ResolvedFile {
        logical_path: "Localization/fixture.tgi".into(),
        origin: Origin::V14Loose,
        sha256: sha256(&bytes),
        bytes,
    }
}
#[test]
fn original_text_lookup_casing_and_collision_results() {
    let f: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-runtime/native-localization.json"
    ))
    .unwrap();
    let mut text = String::from("[Text language = default]\n{\nids = example\nname = \"Table\"\n");
    for pair in f["entries"].as_array().unwrap() {
        text.push_str(&format!(
            "{} = \"{}\"\n",
            pair[0].as_str().unwrap(),
            pair[1].as_str().unwrap()
        ));
    }
    text.push_str("}\n");
    let table = Localization::from_files([file(&text)], Limits::default()).unwrap();
    for case in f["cases"].as_array().unwrap() {
        let key: Vec<u16> = serde_json::from_value(case["key_utf16"].clone()).unwrap();
        let expected: Vec<u16> = serde_json::from_value(case["output_utf16"].clone()).unwrap();
        if key == "missing".encode_utf16().collect::<Vec<_>>() {
            assert!(table.display(&key).is_err());
        } else {
            assert_eq!(table.display(&key).unwrap(), expected);
        }
    }
    assert_eq!(table.display(&[]).unwrap(), Vec::<u16>::new());
    let ambiguous =
        Localization::from_files([file(&text), file(&text)], Limits::default()).unwrap();
    assert!(
        ambiguous
            .display(&"side1".encode_utf16().collect::<Vec<_>>())
            .is_err()
    );
}
#[test]
fn unproved_escapes_and_changed_source_are_not_rendered_as_valid_names() {
    let text = "[Text language = default]\n{\nkey = \"Axis \\& Allies\"\n}\n";
    let table = Localization::from_files([file(text)], Limits::default()).unwrap();
    assert!(
        table
            .display(&"key".encode_utf16().collect::<Vec<_>>())
            .is_err()
    );
    let mut changed = file(text);
    changed.bytes.push(b' ');
    assert!(Localization::from_files([changed], Limits::default()).is_err());
}
#[test]
#[ignore = "requires AA_GAME_ROOT original archive and maps"]
fn original_tutorial_localization_coverage() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let table = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let mut report = Vec::new();
    for name in ["tutorial01", "tutorial02"] {
        let bytes = std::fs::read(root.join(format!(
            "reconstruction/assets/original/Data/Maps/Campaign/{name}.RMP"
        )))
        .unwrap();
        let map = aa_mapworld::MapArchive::from_bytes(bytes).unwrap();
        let inv = aa_mapworld::WorldInventory::decode(&map, u64::MAX).unwrap();
        let mut table = table.clone();
        table
            .add_map_strings(
                &map.decode_chunk("MAPSTRINGS", u64::MAX).unwrap(),
                Limits::default(),
            )
            .unwrap();
        let mut keys = Vec::new();
        for row in inv
            .teams
            .unwrap()
            .slots
            .into_iter()
            .filter_map(|s| s.record)
        {
            keys.push(row.display_key_utf16);
        }
        for row in inv
            .kingdoms
            .unwrap()
            .slots
            .into_iter()
            .filter_map(|s| s.record)
        {
            keys.push(row.payload.display_key_utf16);
        }
        for key in keys {
            let value = table.display(&key);
            report.push(serde_json::json!({"map":name,"key":String::from_utf16_lossy(&key),"result":value.as_ref().map(|v|String::from_utf16_lossy(v)).map_err(|e|e.to_string())}));
        }
    }
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    assert!(
        report.iter().all(|r| r["result"].get("Ok").is_some()),
        "tutorial localization remains unsupported"
    );
}
