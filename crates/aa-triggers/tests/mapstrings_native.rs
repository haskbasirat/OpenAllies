//! Replays research/phase8-triggers/native-mapstrings.json: the ORIGINAL
//! MAPSTRINGS loader (47EA40 path, KKC_TextData 4BA710, lookups 4B9D40 and
//! 4B9F40) run under Unicorn on every original map and on synthetic probes.
mod sha;
use aa_triggers::{
    decode::{Localization, SavedManager, Value},
    mapstrings::{LookupError, MapStrings, ParseError},
};
use serde_json::Value as J;
use std::path::Path;

/// Synthetic probes the original accepts or rejects but which lie outside the
/// proven subset; the Rust port must refuse them explicitly.
const UNSUPPORTED: &[&str] = &[
    "high_in_key",
    "no_text_node",
    "semicolon_key",
    "name_unquoted",
    "ampersand",
    "backslash",
    "escape_n",
    "long_value",
    "line_2048",
    "line_2048_lf",
    "nul_byte",
    "brace_missing",
    "other_node_after",
    "noattr_then_english",
    "semicolon_between_quotes",
];

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn throw_site(message: &str) -> u32 {
    let hex = message
        .strip_prefix("C++ exception from 0x")
        .and_then(|s| s.split(':').next())
        .unwrap_or_else(|| panic!("unexpected native error {message}"));
    u32::from_str_radix(hex, 16).unwrap()
}

/// Entries and lookups must equal the native TextData list and 4B9F40 results.
fn compare(name: &str, strings: &MapStrings, case: &J) {
    let native = case["entries"].as_array().unwrap();
    assert_eq!(strings.entries.len(), native.len(), "{name}");
    for pair in native {
        let (key, text) = (pair[0].as_str().unwrap(), pair[1].as_str().unwrap());
        let entry = strings
            .entries
            .iter()
            .find(|e| e.key == key)
            .unwrap_or_else(|| panic!("{name}: missing {key}"));
        // The C-locale byte reader widens every byte (0x80..0xFF included).
        assert_eq!(entry.text, utf16(text), "{name} {key}");
    }
    for (key, text) in case["lookups"].as_object().unwrap() {
        let got = strings.resolve(&utf16(key));
        match (text.as_str(), got) {
            (None, Err(LookupError::Missing { .. })) => {}
            (Some(text), Ok(units)) => assert_eq!(units, utf16(text), "{name} lookup {key}"),
            (text, got) => panic!("{name} lookup {key}: native {text:?}, rust {got:?}"),
        }
    }
}

#[test]
fn map_strings_match_the_native_loader() {
    let doc: J = serde_json::from_str(include_str!(
        "../../../research/phase8-triggers/native-mapstrings.json"
    ))
    .unwrap();
    let chunks = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-triggers/chunks");
    let index: J = serde_json::from_slice(&std::fs::read(chunks.join("index.json")).unwrap()).unwrap();
    let (mut maps, mut entries, mut lookups, mut synthetic, mut rejected) = (0, 0, 0, 0, 0);
    let mut unsupported = Vec::new();
    for case in doc["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        if let Some(probe) = name.strip_prefix("synthetic/") {
            synthetic += 1;
            let bytes: Vec<u8> = (0..case["bytes_hex"].as_str().unwrap().len() / 2)
                .map(|i| u8::from_str_radix(&case["bytes_hex"].as_str().unwrap()[2 * i..2 * i + 2], 16).unwrap())
                .collect();
            assert_eq!(sha::hex(&bytes), case["sha256"].as_str().unwrap());
            let parsed = MapStrings::parse(&bytes, "English");
            match (case.get("error").and_then(J::as_str), parsed) {
                (_, Err(ParseError::Unsupported { .. })) => unsupported.push(probe.to_string()),
                (Some(error), Err(ParseError::Native { kind, .. })) => {
                    assert_eq!(kind.throw_site(), throw_site(error), "{probe}");
                    rejected += 1;
                }
                (Some(error), Ok(_)) => panic!("{probe}: native throws {error}, rust accepted"),
                (None, Err(e)) => panic!("{probe}: native accepts, rust {e}"),
                (None, Ok(strings)) => compare(name, &strings, case),
            }
            continue;
        }
        let entry = index
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["map"] == name)
            .unwrap_or_else(|| panic!("{name} not extracted"));
        assert_eq!(entry["map_sha256"], case["map_sha256"]);
        let bytes = std::fs::read(chunks.join(entry["mapstrings_file"].as_str().unwrap())).unwrap();
        assert_eq!(sha::hex(&bytes), entry["mapstrings_sha256"].as_str().unwrap());
        assert_eq!(sha::hex(&bytes), case["sha256"].as_str().unwrap());
        let strings = MapStrings::parse(&bytes, "English").unwrap_or_else(|e| panic!("{name}: {e}"));
        compare(name, &strings, case);
        maps += 1;
        entries += strings.entries.len();
        lookups += case["lookups"].as_object().unwrap().len();
    }
    unsupported.sort();
    let mut expected: Vec<String> = UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    expected.sort();
    assert_eq!(unsupported, expected);
    println!(
        "{maps} maps, {entries} entries, {lookups} lookups; {synthetic} probes \
         ({rejected} native rejections reproduced, {} refused as unproven)",
        unsupported.len()
    );
    assert_eq!(maps, 80);
    assert!(synthetic >= 85);
}

/// Every localized trigger parameter of every original map binds through MAPSTRINGS.
#[test]
fn trigger_localized_parameters_bind() {
    let chunks = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-triggers/chunks");
    let index: J = serde_json::from_slice(&std::fs::read(chunks.join("index.json")).unwrap()).unwrap();
    let mut total = 0;
    let mut maps = 0;
    for entry in index.as_array().unwrap() {
        let bytes = std::fs::read(chunks.join(entry["file"].as_str().unwrap())).unwrap();
        let mut saved = SavedManager::decode(&bytes, Localization::External, 64 << 20)
            .unwrap_or_else(|e| panic!("{}: {e}", entry["map"]));
        let text = std::fs::read(chunks.join(entry["mapstrings_file"].as_str().unwrap())).unwrap();
        let strings = MapStrings::parse(&text, "English").unwrap();
        total += strings.bind(&mut saved).unwrap_or_else(|e| panic!("{}: {e}", entry["map"]));
        for t in &saved.triggers {
            for e in t.events.iter().chain(&t.conditions).chain(&t.actions) {
                for p in &e.parameters {
                    if let Value::Localized { inline, .. } = &p.value {
                        assert!(inline.is_some());
                    }
                }
            }
        }
        maps += 1;
    }
    println!("{maps} maps, {total} localized parameters bound");
    assert_eq!(total, 55);
}
