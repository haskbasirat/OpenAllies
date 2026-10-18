use aa_mapworld::{
    Chunk, ChunkDirectory, MapArchive, SourceIdentity, archive::Encoding, header::MapHeader,
};
use serde_json::Value;
use std::path::PathBuf;

fn chunk(bytes: Vec<u8>) -> Chunk {
    Chunk {
        map_source: SourceIdentity::of(b"test map"),
        payload_source: SourceIdentity::of(&bytes),
        directory: ChunkDirectory {
            name: "MAP_HEADER".into(),
            directory_ordinal: 0,
            packed_offset: 12,
            packed_bytes: bytes.len() as u64,
            source_timestamp: 0,
            decoded_bytes: bytes.len() as u64,
        },
        encoding: Encoding::Raw,
        bytes,
    }
}
fn word(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn string(out: &mut Vec<u8>, text: &str) {
    let units: Vec<_> = text.encode_utf16().collect();
    out.extend_from_slice(&(units.len() as u16).to_le_bytes());
    for u in units {
        out.extend_from_slice(&u.to_le_bytes());
    }
}
fn rules(out: &mut Vec<u8>) {
    word(out, 4);
    word(out, 0);
    string(out, "NORMAL");
    word(out, 0);
}
fn parameters(out: &mut Vec<u8>, version: u32) {
    word(out, version);
    word(out, 0);
    word(out, 0);
}
fn minimal(version: u32, params_version: u32) -> Vec<u8> {
    let mut b = Vec::new();
    word(&mut b, version);
    rules(&mut b);
    parameters(&mut b, params_version);
    word(&mut b, 7);
    word(&mut b, 0);
    string(&mut b, "map_name");
    string(&mut b, "map_description");
    if version >= 7 {
        word(&mut b, 448.0_f32.to_bits());
        word(&mut b, 320.0_f32.to_bits());
    }
    word(&mut b, 2004.0_f32.to_bits());
    word(&mut b, 123);
    string(&mut b, "1.0");
    string(&mut b, "A&A");
    string(&mut b, "test");
    b.push(0);
    string(&mut b, "Maps/test.rmp");
    string(&mut b, "");
    if version >= 6 {
        word(&mut b, 0);
    }
    word(&mut b, 1);
    word(&mut b, 8);
    b.push(0);
    string(&mut b, "KKC_WorldCreator");
    word(&mut b, 3);
    rules(&mut b);
    parameters(&mut b, params_version);
    word(&mut b, 448.0_f32.to_bits());
    word(&mut b, 320.0_f32.to_bits());
    string(&mut b, "temperate");
    string(&mut b, "");
    word(&mut b, 0x12345678);
    b
}
#[test]
fn versioned_headers_preserve_creator_dimensions_and_raw_ids() {
    for version in 4..=7 {
        for pv in 11..=13 {
            let value = MapHeader::decode(&chunk(minimal(version, pv))).unwrap();
            assert_eq!(value.version, version);
            assert_eq!(value.parameters.version, pv);
            assert_eq!(
                value.creator.world_extents_bits,
                [448.0_f32.to_bits(), 320.0_f32.to_bits()]
            );
            assert_eq!(value.creator.rng_seed, 0x12345678);
            assert_eq!(value.display_extents_bits.is_some(), version >= 7);
            assert_eq!(
                String::from_utf16(&value.creator.biome).unwrap(),
                "temperate"
            );
        }
    }
}
#[test]
fn incomplete_or_unproven_header_inputs_fail_explicitly() {
    let bytes = minimal(7, 13);
    for length in 0..bytes.len() {
        assert!(
            MapHeader::decode(&chunk(bytes[..length].to_vec())).is_err(),
            "truncated length {length}"
        );
    }
    for (offset, value) in [(0, 8), (4, 3), (8, 1)] {
        let mut wrong = bytes.clone();
        wrong[offset..offset + 4].copy_from_slice(&u32::to_le_bytes(value));
        assert!(MapHeader::decode(&chunk(wrong)).is_err());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(MapHeader::decode(&chunk(trailing)).is_err());
}
#[test]
#[ignore = "requires original local maps; run with AA_GAME_ROOT and --include-ignored"]
fn all_original_headers_match_executed_native_reader_extents() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let maps = root.join("reconstruction/assets/original/Data/Maps");
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-mapheader/native-header.json"
    ))
    .unwrap();
    let rows = fixture["maps"].as_array().unwrap();
    assert_eq!(rows.len(), 80);
    for row in rows {
        let original = row["path"].as_str().unwrap().replace('\\', "/");
        let relative = original.split("/Data/Maps/").nth(1).unwrap();
        let archive = MapArchive::from_bytes(std::fs::read(maps.join(relative)).unwrap()).unwrap();
        let payload = archive.decode_chunk("MAP_HEADER", u64::MAX).unwrap();
        assert_eq!(
            payload.payload_source.sha256,
            row["payload_sha256"].as_str().unwrap()
        );
        let h = MapHeader::decode(&payload).unwrap();
        assert_eq!(
            h.creator_offset as u64,
            row["prefix_bytes"].as_u64().unwrap()
        );
        assert_eq!(
            payload.bytes.len() as u64,
            row["decoded_bytes"].as_u64().unwrap()
        );
        assert_eq!(
            serde_json::to_value(h.creator.world_extents_bits).unwrap(),
            row["creator_size_bits"]
        );
        assert_eq!(h.creator.rng_seed as u64, row["rng_seed"].as_u64().unwrap());
        if let Some(size) = h.display_extents_bits {
            assert_eq!(
                serde_json::to_value(size).unwrap(),
                row["display_extents_bits"]
            );
        }
    }
}
