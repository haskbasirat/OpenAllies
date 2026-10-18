use aa_mapworld::{Chunk, ChunkDirectory, SourceIdentity,archive::Encoding};
use aa_rules::tgi::Limits;
use aa_runtime::{
    film_text::{self, MapTextOrder},
    localization::Localization,
};
use serde_json::Value;

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
fn chunk(bytes: Vec<u8>) -> Chunk {
    let source = SourceIdentity::of(&bytes);
    Chunk {
        map_source: source.clone(),
        directory: ChunkDirectory {
            name: "MAPSTRINGS".into(),
            directory_ordinal: 0,
            packed_offset: 12,
            packed_bytes: bytes.len() as u64,
            source_timestamp: 0,
            decoded_bytes: bytes.len() as u64,
        },
        payload_source: source,
        encoding: Encoding::Raw,
        bytes,
    }
}
fn table(chunk: &Chunk) -> Localization {
    let mut text = Localization::default();
    text.add_map_strings(chunk, Limits::default()).unwrap();
    text
}
#[test]
fn original_v14_all_pc_native_field_order_and_text_representation_values() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-text.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 27);
    for row in rows {
        let name: Vec<u16> = serde_json::from_value(row["input"]["name"].clone()).unwrap();
        let entries: Vec<(String, Vec<u16>)> =
            serde_json::from_value(row["input"]["entries"].clone()).unwrap();
        let refs: Vec<_> = entries
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_slice()))
            .collect();
        let encoded = film_text::encode(&name, &refs).unwrap();
        assert_eq!(encoded, unhex(row["output"]["bytes_hex"].as_str().unwrap()));
        let input = chunk(encoded);
        let mut text = table(&input);
        let order = MapTextOrder::bind(&input, &text).unwrap();
        assert_eq!(
            serde_json::json!(order.entry_names().collect::<Vec<_>>()),
            row["output"]["entry_order"]
        );
        let native_names: Vec<String> =
            serde_json::from_value(row["output"]["entry_order"].clone()).unwrap();
        let ordered: Vec<_> = native_names
            .iter()
            .map(|key| {
                let (_, v) = entries.iter().find(|(k, _)| k == key).unwrap();
                (key.as_str(), v.as_slice())
            })
            .collect();
        assert_eq!(
            order.serialize_current(&text).unwrap(),
            film_text::encode(&name, &ordered).unwrap()
        );
        // Same current entry is read again; no frozen source text value copy.
        text.declarations.get_mut("map_name").unwrap()[0].plain_utf16 = Ok(vec![78, 69, 87]);
        let bytes = order.serialize_current(&text).unwrap();
        let changed = table(&chunk(bytes));
        assert_eq!(
            changed
                .display(&[109, 97, 112, 95, 110, 97, 109, 101])
                .unwrap(),
            vec![78, 69, 87]
        );
        let foreign = chunk(
            b"[Text language=Default]\n{\nids=map\nname=\"Other\"\nmap_name=\"foreign\"\n}\n"
                .to_vec(),
        );
        assert!(order.serialize_current(&table(&foreign)).is_err());
    }
    let actual = fixture["actual_sources"].as_array().unwrap();
    assert_eq!(actual.len(), 9);
    for row in actual {
        let input = chunk(unhex(row["source_hex"].as_str().unwrap()));
        assert_eq!(input.payload_source.sha256, row["sha256"].as_str().unwrap());
        let text = table(&input);
        let order = MapTextOrder::bind(&input, &text).unwrap();
        assert_eq!(
            serde_json::json!(order.entry_names().collect::<Vec<_>>()),
            row["output"]["entry_order"]
        );
        let restored = table(&chunk(order.serialize_current(&text).unwrap()));
        for pair in row["output"]["entries"].as_array().unwrap() {
            let key: Vec<u16> = pair[0].as_str().unwrap().encode_utf16().collect();
            let expected: Vec<u16> = serde_json::from_value(pair[1].clone()).unwrap();
            assert_eq!(restored.display(&key).unwrap(), expected);
        }
    }
    for text in [
        vec![0],
        vec![34],
        vec![38],
        vec![92],
        vec![0x100],
        vec![127],
    ] {
        assert!(film_text::encode(&[], &[("entry", &text)]).is_err());
    }
}
