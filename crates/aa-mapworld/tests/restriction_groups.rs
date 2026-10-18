use aa_mapworld::{
    Chunk, ChunkDirectory, SourceIdentity,
    archive::Encoding,
    restriction_groups::{LocalizedStorage, RestrictionGroups},
};
fn chunk(bytes: Vec<u8>) -> Chunk {
    Chunk {
        map_source: SourceIdentity::of(b"map"),
        payload_source: SourceIdentity::of(&bytes),
        directory: ChunkDirectory {
            name: "RESTRICTION_GROUPS".into(),
            directory_ordinal: 0,
            packed_offset: 0,
            packed_bytes: bytes.len() as u64,
            source_timestamp: 0,
            decoded_bytes: bytes.len() as u64,
        },
        encoding: Encoding::Raw,
        bytes,
    }
}
fn string(out: &mut Vec<u8>, text: &str) {
    let u: Vec<_> = text.encode_utf16().collect();
    out.extend_from_slice(&(u.len() as u16).to_le_bytes());
    for u in u {
        out.extend_from_slice(&u.to_le_bytes());
    }
}
fn word(out: &mut Vec<u8>, n: u32) {
    out.extend_from_slice(&n.to_le_bytes());
}
#[test]
fn checked_versioned_wire_preserves_signed_counts_and_localization_mode() {
    let mut b = Vec::new();
    for n in [0, 17, 1, 4] {
        word(&mut b, n)
    }
    string(&mut b, "profile");
    word(&mut b, 25);
    string(&mut b, "#display");
    string(&mut b, "caption");
    for _ in 0..4 {
        word(&mut b, 0xffffffff)
    }
    let c = chunk(b.clone());
    let r = RestrictionGroups::decode(&c, LocalizedStorage::KeyAndValue).unwrap();
    assert_eq!(r.word_0(), 17);
    assert_eq!(r.profiles()[0].word_0, 25);
    assert_eq!(
        r.profiles()[0].display_value.as_ref().unwrap(),
        &"caption".encode_utf16().collect::<Vec<_>>()
    );
    assert!(
        r.profiles()[0]
            .restricted_actors
            .as_ref()
            .unwrap()
            .is_empty()
    );
    assert!(RestrictionGroups::decode(&c, LocalizedStorage::KeyOnly).is_err());
    for n in 0..b.len() {
        assert!(
            RestrictionGroups::decode(&chunk(b[..n].to_vec()), LocalizedStorage::KeyAndValue)
                .is_err(),
            "truncated byte{n}"
        );
    }
    let mut forged = c.clone();
    forged.bytes[4] ^= 1;
    assert!(RestrictionGroups::decode(&forged, LocalizedStorage::KeyAndValue).is_err());
    let mut legacy = Vec::new();
    for n in [0, 0, 1, 0] {
        word(&mut legacy, n)
    }
    string(&mut legacy, "old");
    word(&mut legacy, 0);
    string(&mut legacy, "label");
    string(&mut legacy, "discarded");
    word(&mut legacy, 0);
    word(&mut legacy, 0);
    let r = RestrictionGroups::decode(&chunk(legacy), LocalizedStorage::KeyOnly).unwrap();
    assert!(r.profiles()[0].restricted_actors.is_none());
    assert!(r.profiles()[0].restricted_properties.is_none());
    let negative = chunk(
        [0u32, 0, 0xffffffff]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect(),
    );
    assert!(
        RestrictionGroups::decode(&negative, LocalizedStorage::KeyOnly)
            .unwrap()
            .profiles()
            .is_empty()
    );
    // Wrong chunk identity and trailing bytes are never treated as completion.
    let mut wrong = negative.clone();
    wrong.directory.name = "TEAMS".into();
    assert!(RestrictionGroups::decode(&wrong, LocalizedStorage::KeyOnly).is_err());
}
