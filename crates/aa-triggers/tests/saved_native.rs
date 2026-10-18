mod sha;
use aa_triggers::decode::{Localization, SavedManager};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct Source {
    map_sha256: String,
    chunk_sha256: String,
}
#[derive(Deserialize)]
struct Case {
    map: String,
    source: Source,
    expected: SavedManager,
}
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}
#[test]
fn original_tutorial_records_match_native_writer_validated_fields() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../research/phase7-triggers/saved-rust-fixtures.json"
    ))
    .unwrap();
    // Decompressed TRIGGERS chunks extracted from the original RMP archives by
    // research/phase8-triggers/extract_chunks.py; map and chunk SHA256 pins.
    let chunks = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-triggers/chunks");
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(chunks.join("index.json")).unwrap()).unwrap();
    for case in fixture.cases {
        let entry = index
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["map"] == format!("Campaign/{}.RMP", case.map))
            .unwrap();
        assert_eq!(entry["map_sha256"], case.source.map_sha256.as_str());
        let bytes = std::fs::read(chunks.join(entry["file"].as_str().unwrap())).unwrap();
        assert_eq!(sha::hex(&bytes), case.source.chunk_sha256);
        let chunk = bytes;
        let decoded =
            SavedManager::decode(&chunk, Localization::External, chunk.len()).unwrap();
        assert_eq!(decoded, case.expected, "{}", case.map);
    }
}
#[test]
fn unsupported_versions_truncation_and_counts_fail() {
    let minimal = [5_u32, 1, 0, 0, 0]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    assert!(SavedManager::decode(&minimal, Localization::External, minimal.len()).is_ok());
    for end in 0..minimal.len() {
        assert!(
            SavedManager::decode(&minimal[..end], Localization::External, minimal.len()).is_err()
        );
    }
    let mut bad = minimal.clone();
    bad[..4].copy_from_slice(&6_u32.to_le_bytes());
    assert!(SavedManager::decode(&bad, Localization::External, bad.len()).is_err());
    bad = minimal.clone();
    bad[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(SavedManager::decode(&bad, Localization::External, bad.len()).is_err());
    assert!(SavedManager::decode(&minimal, Localization::External, minimal.len() - 1).is_err());
    // Version 4: no string list; one string and one word read and discarded.
    let mut v4 = [4_u32, 1, 0, 0]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    v4.extend([1, 0, b'x', 0, 7, 0, 0, 0]);
    let decoded = SavedManager::decode(&v4, Localization::External, v4.len()).unwrap();
    assert!(decoded.strings.is_empty());
    for end in 0..v4.len() {
        assert!(SavedManager::decode(&v4[..end], Localization::External, v4.len()).is_err());
    }
    for version in [0_u32, 1, 2, 3] {
        v4[..4].copy_from_slice(&version.to_le_bytes());
        assert!(SavedManager::decode(&v4, Localization::External, v4.len()).is_err());
    }
}
