use aa_mapworld::{Inventory, MapArchive};
use std::io::Write;
fn archive(rows: &[(&str, Vec<u8>, u64)]) -> Vec<u8> {
    let mut bytes = b"TGCK".to_vec();
    bytes.extend(2u32.to_le_bytes());
    bytes.extend((rows.len() as u32).to_le_bytes());
    let mut directory = Vec::new();
    for (name, packed, raw_length) in rows {
        let mut row = [0u8; 96];
        for (i, u) in name.encode_utf16().enumerate() {
            row[i * 2..i * 2 + 2].copy_from_slice(&u.to_le_bytes());
        }
        row[64..72].copy_from_slice(&(bytes.len() as u64).to_le_bytes());
        row[72..80].copy_from_slice(&(packed.len() as u64).to_le_bytes());
        row[88..].copy_from_slice(&raw_length.to_le_bytes());
        bytes.extend(packed);
        directory.extend(row);
    }
    bytes.extend(directory);
    bytes
}
fn packed(bytes: &[u8]) -> Vec<u8> {
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(bytes).unwrap();
    z.finish().unwrap()
}
#[test]
fn mixed_raw_zlib_chunks_respect_budget_source_identity_and_actual_stream_end() {
    let plain = b"exact original chunk bytes";
    let encoded = packed(plain);
    let bytes = archive(&[
        ("MAP", vec![0; 4], 4),
        ("TEST", encoded.clone(), plain.len() as u64),
    ]);
    let map = MapArchive::from_bytes(bytes).unwrap();
    assert_eq!(map.decode_chunk("MAP", 4).unwrap().bytes, [0; 4]);
    assert_eq!(map.decode_chunk("TEST", 100).unwrap().bytes, plain);
    assert!(map.decode_chunk("TEST", plain.len() as u64 - 1).is_err());
    assert!(map.decode_chunk("MISSING", 100).is_err());
    let mut trailing = encoded;
    trailing.push(0);
    assert!(
        MapArchive::from_bytes(archive(&[("TEST", trailing, plain.len() as u64)]))
            .unwrap()
            .decode_chunk("TEST", 100)
            .is_err()
    );
}
#[test]
fn corrupt_directory_and_record_extents_cannot_escape_or_silently_drop_data() {
    assert!(
        MapArchive::from_bytes(archive(&[("A", vec![0; 4], 4), ("A", vec![0; 4], 4)])).is_err()
    );
    let mut bytes = archive(&[("A", vec![0; 4], 4)]);
    let end = bytes.len();
    bytes[end - 32..end - 24].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(MapArchive::from_bytes(bytes).is_err());
    let data = [0u32, 11u32]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    let index = [0u32, 1u32, 100u32]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    let map = MapArchive::from_bytes(archive(&[
        ("DYNAMIC_THINGS", data.clone(), data.len() as u64),
        ("DYNAMIC_THINGS_INDEX", index.clone(), index.len() as u64),
    ]))
    .unwrap();
    assert!(Inventory::decode(&map, 100).is_err());
}
