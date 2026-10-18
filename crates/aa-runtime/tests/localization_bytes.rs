//! Original executable evidence for no-BOM DisplayString byte decoding.
use aa_rules::{
    sources::{Origin, ResolvedFile, sha256},
    tgi::Limits,
};
use aa_runtime::localization::Localization;

fn hex_bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn source(path: &str, bytes: Vec<u8>) -> ResolvedFile {
    ResolvedFile {
        logical_path: path.into(),
        origin: Origin::V14Loose,
        sha256: sha256(&bytes),
        bytes,
    }
}

#[test]
fn original_x86_byte_strings_and_developer_map_names() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase8-localization/native-display-bytes.json"
    ))
    .unwrap();
    let mut lookups = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let bytes = hex_bytes(case["bytes_hex"].as_str().unwrap());
        assert_eq!(sha256(&bytes), case["sha256"].as_str().unwrap());
        let table = Localization::from_files(
            [source(case["name"].as_str().unwrap(), bytes)],
            Limits::default(),
        )
        .unwrap();
        for lookup in case["lookups"].as_array().unwrap() {
            let name = lookup["key"].as_str().unwrap();
            let key = name.encode_utf16().collect::<Vec<_>>();
            let expected: Vec<u16> =
                serde_json::from_value(lookup["output_utf16"].clone()).unwrap();
            assert_eq!(
                table.display(&key).unwrap(),
                expected,
                "{}:{name}",
                case["name"]
            );
            assert_eq!(table.lookup(&key).unwrap(), Some(expected));
            lookups += 1;
        }
    }
    assert_eq!(lookups, 13);
}

#[test]
fn escape_tables_and_bom_encoded_high_bytes_remain_explicitly_unsupported() {
    for literal in [b"A&symbol.B".as_slice(), b"A\\nB", b"A\x01B"] {
        let mut bytes = b"[Text language=Default]\n{\nkey=\"".to_vec();
        bytes.extend_from_slice(literal);
        bytes.extend_from_slice(b"\"\n}\n");
        let table = Localization::from_files([source("unsupported.tgi", bytes)], Limits::default())
            .unwrap();
        assert!(
            table
                .display(&"key".encode_utf16().collect::<Vec<_>>())
                .is_err()
        );
    }
    let bytes = b"\xef\xbb\xbf[Text language=Default]\n{\nkey=\"caf\xc3\xa9\"\n}\n".to_vec();
    let table = Localization::from_files([source("utf8.tgi", bytes)], Limits::default()).unwrap();
    assert!(
        table
            .display(&"key".encode_utf16().collect::<Vec<_>>())
            .unwrap_err()
            .to_string()
            .contains("UTF-8 BOM")
    );
}

#[test]
fn duplicate_declarations_preserve_source_order_and_still_refuse_lookup() {
    let file = |path, value| {
        source(
            path,
            format!("[Text language=Default]\n{{\nkey=\"{value}\"\n}}\n").into_bytes(),
        )
    };
    let table = Localization::from_files(
        [file("first.tgi", "one"), file("second.tgi", "two")],
        Limits::default(),
    )
    .unwrap();
    let rows = &table.declarations["key"];
    assert_eq!(rows[0].source.path, "first.tgi");
    assert_eq!(rows[1].source.path, "second.tgi");
    assert!(
        table
            .display(&"key".encode_utf16().collect::<Vec<_>>())
            .unwrap_err()
            .to_string()
            .contains("duplicate")
    );
}

#[test]
#[ignore = "requires AA_GAME_ROOT original map"]
fn original_developer_map_chunk_uses_the_same_decoder() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let bytes = std::fs::read(
        root.join("reconstruction/assets/original/Data/Maps/DeveloperMaps/GER_Art_Map.RMP"),
    )
    .unwrap();
    assert_eq!(
        sha256(&bytes),
        "fac71cca645dfcfeaa5a8787879cb39177c6946588e1d2e7cf7a06fba229949d"
    );
    let map = aa_mapworld::MapArchive::from_bytes(bytes).unwrap();
    let mut table = Localization::default();
    table
        .add_map_strings(
            &map.decode_chunk("MAPSTRINGS", u64::MAX).unwrap(),
            Limits::default(),
        )
        .unwrap();
    for (key, expected) in [
        ("name_192", "Fallschrimjäger"),
        ("name_195", "Fallschrimjäger-Führer"),
    ] {
        assert_eq!(
            table
                .display(&key.encode_utf16().collect::<Vec<_>>())
                .unwrap(),
            expected.encode_utf16().collect::<Vec<_>>()
        );
    }
}
