use aa_mapworld::{
    Chunk, ChunkDirectory, SourceIdentity,
    archive::Encoding,
    terrain::{ResolvedVariation, TerrainChunk, TerrainGridLayout, recompute_passability},
};
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase6-spatial/native-terrain.json"
    ))
    .unwrap()
}
fn unhex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn chunk(bytes: Vec<u8>) -> Chunk {
    Chunk {
        map_source: SourceIdentity::of(b"native terrain test"),
        payload_source: SourceIdentity::of(&bytes),
        directory: ChunkDirectory {
            name: "TERRAIN".into(),
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
fn palette(row: &Value) -> Vec<Option<ResolvedVariation>> {
    row["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            if d["unresolved"].as_bool() == Some(true) {
                return None;
            }
            Some(ResolvedVariation {
                id_utf16: d["id"].as_str().unwrap().encode_utf16().collect(),
                native_type: d["native_type"].as_u64().unwrap() as u32,
                passability_bits: d["passability_bits"].as_u64().unwrap() as u8,
                source: SourceIdentity::of(b"explicit native fixture definition"),
            })
        })
        .collect()
}

#[test]
fn missing_variations_carry_native_previous_definition_and_explicit_biome_default() {
    let f = fixture();
    assert_eq!(f["fallback"].as_array().unwrap().len(), 8);
    for row in f["fallback"].as_array().unwrap() {
        let input = chunk(unhex(row["input_hex"].as_str().unwrap()));
        let decoded = TerrainChunk::decode(&input).unwrap();
        let layout = TerrainGridLayout::from_world_extents([6f32.to_bits(); 2], 1).unwrap();
        let p = palette(row);
        assert!(decoded.bind(layout, &p).is_err());
        let d = &row["initial"];
        let initial = ResolvedVariation {
            id_utf16: d["id"].as_str().unwrap().encode_utf16().collect(),
            native_type: d["native_type"].as_u64().unwrap() as u32,
            passability_bits: d["passability_bits"].as_u64().unwrap() as u8,
            source: SourceIdentity::of(b"explicit biome default"),
        };
        let bound = decoded
            .bind_with_native_fallback(layout, &p, initial)
            .unwrap();
        assert_eq!(bound.missing_variation_ordinals, vec![0, 1, 3, 4, 6, 8]);
        assert_eq!(
            bound.missing_variation_ordinals.len() as u64,
            row["native"]["missing_count"].as_u64().unwrap()
        );
        for (actual, native) in bound
            .cells
            .iter()
            .zip(row["native"]["cells"].as_array().unwrap())
        {
            assert_eq!(actual.flags20, native["flags20"].as_u64().unwrap() as u32);
            assert_eq!(
                serde_json::to_value(actual.resolved_palette_index).unwrap(),
                native["variation_index"]
            );
            assert_eq!(
                u64::from(actual.byte_2c),
                native["byte_2c"].as_u64().unwrap()
            );
        }
    }
}

#[test]
fn native_grid_layout_and_all_passability_byte_combinations_match() {
    let f = fixture();
    assert_eq!(f["layouts"].as_array().unwrap().len(), 20);
    for row in f["layouts"].as_array().unwrap() {
        let result = TerrainGridLayout::from_world_extents(
            serde_json::from_value(row["extent_bits"].clone()).unwrap(),
            row["resolution"].as_u64().unwrap() as u32,
        );
        if row["native"]["width"].as_u64().unwrap() == 0
            || row["native"]["height"].as_u64().unwrap() == 0
        {
            assert!(
                result.is_err(),
                "zero dimension would make native border initialization unsafe"
            );
        } else {
            assert_eq!(
                serde_json::to_value(result.unwrap()).unwrap(),
                row["native"]
            );
        }
    }
    assert_eq!(f["flags"].as_array().unwrap().len(), 1048);
    for row in f["flags"].as_array().unwrap() {
        assert_eq!(
            recompute_passability(
                row["flags20"].as_u64().unwrap() as u32,
                row["passability_bits"].as_u64().unwrap() as u8
            ),
            row["native"].as_u64().unwrap() as u32
        );
    }
}

#[test]
fn original_v6_v7_tile_load_matches_exact_native_fields_and_stream_extent() {
    let f = fixture();
    assert_eq!(f["loads"].as_array().unwrap().len(), 32);
    for row in f["loads"].as_array().unwrap() {
        let input = chunk(unhex(row["input_hex"].as_str().unwrap()));
        let decoded = TerrainChunk::decode(&input).unwrap();
        let width = row["width"].as_u64().unwrap() as i32;
        let height = row["height"].as_u64().unwrap() as i32;
        let layout = TerrainGridLayout::from_world_extents(
            [
                (width as f32 * 2.).to_bits(),
                (height as f32 * 2.).to_bits(),
            ],
            1,
        )
        .unwrap();
        let bound = decoded.bind(layout, &palette(row)).unwrap();
        assert_eq!(
            decoded.suffix_offset,
            row["native"]["consumed"].as_u64().unwrap() as usize
        );
        assert_eq!(decoded.pending_auxiliary_bytes, b"opaque suffix");
        assert_eq!(
            decoded.serialized_tile_bytes,
            &input.bytes[decoded.tile_data_offset..decoded.suffix_offset]
        );
        for (index, (actual, native)) in bound
            .cells
            .iter()
            .zip(row["native"]["cells"].as_array().unwrap())
            .enumerate()
        {
            assert_eq!(actual.flags20, native["flags20"].as_u64().unwrap() as u32);
            assert_eq!(
                u64::from(actual.variation_index),
                native["variation_index"].as_u64().unwrap()
            );
            assert_eq!(
                u64::from(actual.byte_2c),
                native["byte_2c"].as_u64().unwrap()
            );
            assert_eq!(
                native["runtime_region_word"], 0,
                "mode false never installs the saved explored mask"
            );
            assert_eq!(
                actual.saved_explored_mask,
                decoded.tiles[index].explored_mask()
            );
            assert_eq!(
                bound.cell(index as i32 % width, index as i32 / width),
                Some(actual)
            );
        }
        assert!(bound.cell(-1, 0).is_none());
        assert!(bound.cell(width, height).is_none());
        for length in 0..decoded.suffix_offset {
            assert!(
                TerrainChunk::decode(&chunk(input.bytes[..length].to_vec())).is_err(),
                "truncation{length}"
            );
        }
    }
}

#[test]
fn bad_resolution_missing_definitions_and_unproven_legacy_input_fail() {
    let row = &fixture()["loads"][0];
    let input = unhex(row["input_hex"].as_str().unwrap());
    let decoded = TerrainChunk::decode(&chunk(input.clone())).unwrap();
    let layout =
        TerrainGridLayout::from_world_extents([2f32.to_bits(), 2f32.to_bits()], 1).unwrap();
    assert!(decoded.bind(layout, &[]).is_err());
    let mut p = palette(row);
    p[0].as_mut().unwrap().id_utf16 = vec![0];
    assert!(decoded.bind(layout, &p).is_err());
    let mut p = palette(row);
    p[0].as_mut().unwrap().passability_bits = 4;
    assert!(decoded.bind(layout, &p).is_err());
    let mut p = palette(row);
    p[0].as_mut().unwrap().native_type = 3;
    assert!(decoded.bind(layout, &p).is_err());
    let layout =
        TerrainGridLayout::from_world_extents([2f32.to_bits(), 2f32.to_bits()], 0).unwrap();
    assert!(decoded.bind(layout, &palette(row)).is_err());
    for version in [0u32, 3, 5, 8] {
        let mut bad = input.clone();
        bad[..4].copy_from_slice(&version.to_le_bytes());
        assert!(TerrainChunk::decode(&chunk(bad)).is_err());
    }
    for bad in [f32::NAN, f32::INFINITY, -1., 0., 2147483648.] {
        assert!(TerrainGridLayout::from_world_extents([bad.to_bits(), 2f32.to_bits()], 1).is_err());
    }
    assert!(TerrainGridLayout::from_world_extents([2f32.to_bits(); 2], 31).is_err());
}
