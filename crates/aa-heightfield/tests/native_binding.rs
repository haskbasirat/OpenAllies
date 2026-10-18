use aa_heightfield::{Error, binding::*};
use serde_json::{Value, json};
fn fixtures() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase6-spatial/native-height-binding.json"
    ))
    .unwrap()
}
fn decode<T: serde::de::DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn snapshot(state: &InitializedHeightfield) -> Value {
    json!({"extent_i32":state.layout.extent_i32,"width":state.field.width,"height":state.field.height,
        "coordinate_shift":state.field.coordinate_shift,"cell_size_bits":state.field.cell_size_bits,
        "index_bias":state.field.index_bias,"minimum_bits":state.minimum_bits,"maximum_bits":state.maximum_bits,
        "default_bits":state.default_bits,"height_bits":state.field.height_bits,"flags":state.sample_flags,
        "owns_storage":true})
}
#[test]
fn original_constructor_layout_and_initial_storage_match() {
    let f = fixtures();
    assert!(f["passed"].as_bool().unwrap());
    for row in f["constructor"].as_array().unwrap() {
        let input = &row["input"];
        let layout = GridLayout::from_world_extents(
            decode(&input["extent_bits"]),
            decode(&input["resolution_bits"]),
        )
        .unwrap();
        let state = InitializedHeightfield::new(
            layout,
            decode(&input["minimum_bits"]),
            decode(&input["maximum_bits"]),
            decode(&input["default_bits"]),
        )
        .unwrap();
        assert_eq!(snapshot(&state), row["output"]);
        assert_eq!(
            json!([state.field.height_bits.len() * 4, state.sample_flags.len()]),
            row["allocations"]
        );
    }
}
#[test]
fn original_chunk_reads_preserve_raw_samples_flags_limits_and_tail() {
    for row in fixtures()["loads"].as_array().unwrap() {
        let layout = GridLayout::from_world_extents(
            decode(&row["extent_bits"]),
            decode(&row["resolution_bits"]),
        )
        .unwrap();
        let bytes: Vec<u8> = decode(&row["bytes"]);
        let loaded = LoadedHeightfield::decode(layout, &bytes).unwrap();
        assert_eq!(snapshot(&loaded.state), row["output"]["state"]);
        assert_eq!(json!(loaded.consumed_bytes), row["output"]["consumed"]);
        assert_eq!(json!(loaded.trailing_bytes), row["output"]["remaining"]);
        assert_eq!(loaded.chunk_sha256.len(), 64);
    }
}
#[test]
fn invalid_native_layouts_and_mismatched_chunk_witnesses_are_explicit_errors() {
    for shift in [0, 31, 32, u32::MAX] {
        assert!(GridLayout::from_world_extents([0, 0], shift).is_err());
    }
    for extent in [
        (-1.0f32).to_bits(),
        f32::NAN.to_bits(),
        f32::INFINITY.to_bits(),
        (2147483648.0f32).to_bits(),
    ] {
        assert!(GridLayout::from_world_extents([extent, 0], 1).is_err());
    }
    let row = &fixtures()["loads"][1];
    let layout = GridLayout::from_world_extents(decode(&row["extent_bits"]), 1).unwrap();
    let bytes: Vec<u8> = decode(&row["bytes"]);
    assert!(matches!(
        LoadedHeightfield::decode(layout, &bytes[..23]),
        Err(Error::TruncatedChunk)
    ));
    for at in [16, 20] {
        let mut bad = bytes.clone();
        bad[at..at + 4].copy_from_slice(&1234u32.to_le_bytes());
        assert!(matches!(
            LoadedHeightfield::decode(layout, &bad),
            Err(Error::LayoutMismatch)
        ));
    }
    let mut bad = bytes.clone();
    bad[..4].copy_from_slice(&2u32.to_le_bytes());
    assert!(matches!(
        LoadedHeightfield::decode(layout, &bad),
        Err(Error::UnsupportedVersion(2))
    ));
    bad[..4].copy_from_slice(&0u32.to_le_bytes());
    assert!(matches!(
        LoadedHeightfield::decode(layout, &bad),
        Err(Error::UnsupportedVersion(0))
    ));
    assert!(matches!(
        LoadedHeightfield::decode(layout, &bad[..24]),
        Err(Error::UnsupportedVersion(0))
    ));
    let mut bad = layout;
    bad.coordinate_shift = u32::MAX;
    assert!(matches!(bad.sample_count(), Err(Error::InvalidLayout)));
}
