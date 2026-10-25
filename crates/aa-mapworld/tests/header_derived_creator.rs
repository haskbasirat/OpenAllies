//! Derived MAP_HEADER creators (film headers): `KKC_WorldCreatorLoad` and
//! `KKC_WorldCreatorRandom` through `MapHeader::decode_any_creator`, while
//! `MapHeader::decode` keeps refusing them.
use aa_mapworld::{
    Chunk, ChunkDirectory, SourceIdentity,
    archive::Encoding,
    header::{DerivedCreator, MapHeader},
};

fn chunk(bytes: Vec<u8>) -> Chunk {
    Chunk {
        map_source: SourceIdentity::of(b"test film"),
        payload_source: SourceIdentity::of(&bytes),
        directory: ChunkDirectory {
            name: "MAP_HEADER".into(),
            directory_ordinal: 1,
            packed_offset: 16,
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
    rules_mode(out, 0);
}
fn rules_mode(out: &mut Vec<u8>, mode: u32) {
    word(out, 4);
    word(out, mode);
    string(out, "NORMAL");
    word(out, 0);
}
fn parameters(out: &mut Vec<u8>) {
    word(out, 13);
    word(out, 0);
    word(out, 0);
}
/// MapHeader v7 up to (not including) the creator class tag.
fn prefix() -> Vec<u8> {
    prefix_mode(0)
}
fn prefix_mode(mode: u32) -> Vec<u8> {
    let mut b = Vec::new();
    word(&mut b, 7);
    rules_mode(&mut b, mode);
    parameters(&mut b);
    word(&mut b, 7);
    word(&mut b, 0);
    string(&mut b, "map_name");
    string(&mut b, "map_description");
    word(&mut b, 384.0_f32.to_bits());
    word(&mut b, 384.0_f32.to_bits());
    word(&mut b, 2004.0_f32.to_bits());
    word(&mut b, 123);
    string(&mut b, "1.0");
    string(&mut b, "A&A");
    string(&mut b, "test");
    b.push(0);
    string(&mut b, "");
    string(&mut b, "");
    word(&mut b, 0);
    word(&mut b, 1);
    word(&mut b, 8);
    b.push(0);
    b
}
fn base_payload(b: &mut Vec<u8>, seed: u32) {
    word(b, 3);
    rules(b);
    parameters(b);
    word(b, 384.0_f32.to_bits());
    word(b, 384.0_f32.to_bits());
    string(b, "chaparral");
    string(b, "");
    word(b, seed);
}
fn random_header() -> Vec<u8> {
    let mut b = prefix();
    string(&mut b, "KKC_WorldCreatorRandom");
    base_payload(&mut b, 972_257_557);
    word(&mut b, 1);
    string(&mut b, "rmc_southerneurope");
    word(&mut b, 2);
    string(&mut b, "chaparral_grass");
    word(&mut b, 10.0_f32.to_bits());
    string(&mut b, "chaparral_ocean");
    word(&mut b, 2.5_f32.to_bits());
    word(&mut b, 120.0_f32.to_bits());
    word(&mut b, 1);
    string(&mut b, "village_southerneurope");
    word(&mut b, 4.5_f32.to_bits());
    word(&mut b, 0);
    b
}
fn load_header() -> Vec<u8> {
    let mut b = prefix();
    string(&mut b, "KKC_WorldCreatorLoad");
    word(&mut b, 0);
    base_payload(&mut b, 7);
    string(&mut b, "maps_firefight");
    b
}

#[test]
fn random_creator_settings_decode_and_plain_decode_still_refuses() {
    let bytes = random_header();
    assert!(MapHeader::decode(&chunk(bytes.clone())).is_err());
    let (header, derived) = MapHeader::decode_any_creator(&chunk(bytes)).unwrap();
    assert_eq!(header.creator.rng_seed, 972_257_557);
    assert_eq!(
        String::from_utf16(&header.creator.class_name).unwrap(),
        "KKC_WorldCreatorRandom"
    );
    let DerivedCreator::Random(settings) = derived else {
        panic!("random creator expected")
    };
    assert_eq!(String::from_utf16(&settings.rmc).unwrap(), "rmc_southerneurope");
    assert_eq!(settings.terrain_weights.len(), 2);
    assert_eq!(settings.terrain_weights[1].1, 2.5_f32.to_bits());
    assert_eq!(settings.total_terrain_weight_bits, 120.0_f32.to_bits());
    assert_eq!(settings.object_counts.len(), 1);
    assert_eq!(settings.num_heroes, 0);
}

#[test]
fn load_creator_keeps_source_ids() {
    let bytes = load_header();
    assert!(MapHeader::decode(&chunk(bytes.clone())).is_err());
    let (header, derived) = MapHeader::decode_any_creator(&chunk(bytes)).unwrap();
    assert_eq!(header.creator.rng_seed, 7);
    assert_eq!(
        derived,
        DerivedCreator::Load {
            derived_version: 0,
            source_resource_ids: "maps_firefight".encode_utf16().collect()
        }
    );
}

#[test]
fn film_world_mode_is_accepted_only_for_film_headers() {
    for mode in [3u32, 1, 2] {
        let mut b = prefix_mode(mode);
        string(&mut b, "KKC_WorldCreator");
        base_payload(&mut b, 5);
        assert!(MapHeader::decode(&chunk(b.clone())).is_err(), "map decode mode {mode}");
        let film = MapHeader::decode_any_creator(&chunk(b));
        assert_eq!(film.is_ok(), mode == 3, "film decode mode {mode}");
        if let Ok((header, derived)) = film {
            assert_eq!(header.rules.mode, 3);
            assert_eq!(derived, DerivedCreator::Base);
        }
    }
}

#[test]
fn truncated_or_extended_derived_creators_fail_explicitly() {
    for bytes in [random_header(), load_header()] {
        for length in 0..bytes.len() {
            assert!(
                MapHeader::decode_any_creator(&chunk(bytes[..length].to_vec())).is_err(),
                "truncated length {length}"
            );
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(MapHeader::decode_any_creator(&chunk(trailing)).is_err());
    }
    let mut unknown = prefix();
    string(&mut unknown, "KKC_WorldCreatorPreset");
    base_payload(&mut unknown, 1);
    assert!(MapHeader::decode_any_creator(&chunk(unknown)).is_err());
}
