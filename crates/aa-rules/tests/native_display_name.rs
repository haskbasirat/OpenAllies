//! Replays research/phase8-saved-components/native-display-name.json: the
//! original parser 5BD080 on each snippet, then ThingData 47D960 /
//! GetLocalized 5B5620 under Unicorn (string table 4B9F40 seamed).
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    native::display_name::native_stored_name,
    sources::{Origin, ResolvedFile, sha256},
    tgi::Limits,
};
use serde_json::{Value, json};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/phase8-saved-components/native-display-name.json"
);

#[test]
fn display_name_matches_original_get_localized() {
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    assert_eq!(doc["passed"], true);
    let table = doc["string_table"].as_object().unwrap();
    for case in doc["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let hex = case["source_hex"].as_str().unwrap();
        let bytes: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
        let catalog = ActorCatalog::from_files(
            vec![ResolvedFile {
                logical_path: "oracle.tgi".into(),
                origin: Origin::V14Loose,
                sha256: sha256(&bytes),
                bytes,
            }],
            Limits::default(),
        );
        let node = catalog.resolved_definition("Thing", "x", Precision::Double53).unwrap();
        let mut lookups = Vec::new();
        let result = native_stored_name(&node, &mut |key: &[u16]| {
            let key = String::from_utf16(key).unwrap();
            let text = table.get(&key).map(|v| v.as_str().unwrap().encode_utf16().collect());
            lookups.push(json!({"key": key, "found": text.is_some()}));
            Ok(text)
        });
        let native = &case["native"];
        match (&native["outcome"], result) {
            (Value::Null, Ok(units)) => {
                assert_eq!(json!(units), native["stored_utf16"], "{name}");
            }
            (o, Err(e)) if !o.is_null() => {
                let text = e.to_string();
                match o["kind"].as_str().unwrap() {
                    "TGC_DataFileNodeException" => assert!(text.contains("Node doesn't contain field."), "{name}: {text}"),
                    "ParseException" => assert!(text.contains("missing from node using strict template"), "{name}: {text}"),
                    k => panic!("{name}: {k}"),
                }
            }
            (o, r) => panic!("{name}: native {o} rust {r:?}"),
        }
        assert_eq!(json!(lookups), native["lookups"], "{name}");
    }
}

/// Every shipped v14 actor definition yields a stored name (no native
/// exception); counts of `#key` names vs literal names are printed.
#[test]
#[ignore = "requires original Data.rwd and the v14 runtime via AA_GAME_ROOT"]
fn shipped_actors_have_native_display_names() {
    use aa_rules::sources::{SourceLimits, V14Sources};
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: 64 * 1024 * 1024 },
    )
    .unwrap();
    let (catalog, _) = aa_rules::actors::native_load::load(&mut sources).unwrap();
    let (mut keyed, mut literal, mut failed) = (0, 0, Vec::new());
    for actor in &catalog.actors {
        let mut was_key = false;
        match catalog.native_display_name(&actor.id, &mut |_key: &[u16]| {
            was_key = true;
            Ok(None)
        }) {
            Ok(_) if was_key => keyed += 1,
            Ok(_) => literal += 1,
            Err(e) => failed.push((actor.id.clone(), e.to_string())),
        }
    }
    println!("keyed {keyed} literal {literal} failed {}", failed.len());
    for f in failed.iter().take(10) {
        println!("{f:?}");
    }
    assert!(keyed > 0);
}
