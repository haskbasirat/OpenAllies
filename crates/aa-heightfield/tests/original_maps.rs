use aa_heightfield::binding::{GridLayout, LoadedHeightfield};
use aa_mapworld::{MapArchive, header::MapHeader};
use aa_numeric::{Binary, Precision};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::path::PathBuf;

#[test]
#[ignore = "requires the original source-pinned maps and authored SVar file; run with AA_GAME_ROOT and --include-ignored"]
fn original_maps_bind_native_extents_and_lossless_height_chunks() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"));
    let source = root.join("reconstruction/assets/original/Data");
    let evidence: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-spatial/native-real-map-heights.json"
    ))
    .unwrap();
    let rules = std::fs::read(source.join("Game/SVars.tgi")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&rules)),
        evidence["resolution_source"]["sha256"]
    );
    // An explicit authored-input fixture, not a runtime default or inferred
    // value from a serialized witness that the native loader itself ignores.
    assert!(
        rules
            .windows(b"int HeightmapResolutionBits = 1".len())
            .any(|s| s == b"int HeightmapResolutionBits = 1")
    );
    let index: Value = serde_json::from_slice(
        &std::fs::read(root.join("reconstruction/game/data/maps/index.json")).unwrap(),
    )
    .unwrap();
    let maps = index.as_array().unwrap();
    assert_eq!(maps.len(), 80);
    let mut tails = 0;
    for map in maps {
        let path = source.join(map["source"].as_str().unwrap());
        let archive = MapArchive::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let pin = &evidence["maps"][map["source"].as_str().unwrap()];
        assert_eq!(archive.source().sha256, pin["map_sha256"]);
        let header = MapHeader::decode(
            &archive
                .decode_chunk("MAP_HEADER", 64 * 1024 * 1024)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(header.creator.world_extents_bits).unwrap(),
            pin["extent_bits"]
        );
        for rule in header.rules.rules.iter().chain(&header.creator.rules.rules) {
            assert!(
                !String::from_utf16_lossy(&rule.key)
                    .eq_ignore_ascii_case("HeightmapResolutionBits"),
                "source fixture needs explicit override resolution: {}",
                path.display()
            );
        }
        let layout = GridLayout::from_world_extents(header.creator.world_extents_bits, 1).unwrap();
        let mut fields = Vec::new();
        for name in ["HEIGHT_MAP", "MOUNTAIN_HEIGHT_MAP"] {
            let chunk = archive.decode_chunk(name, 64 * 1024 * 1024).unwrap();
            let loaded = LoadedHeightfield::decode(layout, &chunk.bytes).unwrap();
            assert_eq!(loaded.chunk_sha256, chunk.payload_source.sha256);
            assert_eq!(loaded.chunk_sha256, pin["chunks"][name]);
            assert_eq!(
                loaded.state.field.height_bits.len(),
                layout.sample_count().unwrap()
            );
            tails += usize::from(!loaded.trailing_bytes.is_empty());
            for [x, y] in [
                [0, 0],
                [layout.width / 2, layout.height / 2],
                [layout.width - 1, layout.height - 1],
            ] {
                let position = [x, y].map(|v| {
                    Binary::from_i32(v)
                        .mul(
                            &Binary::from_f32_bits(layout.cell_size_bits).unwrap(),
                            Precision::Double53,
                        )
                        .unwrap()
                        .store_f32()
                        .unwrap()
                });
                let expected = loaded.state.field.height_bits[(y * layout.width + x) as usize];
                assert_eq!(
                    loaded
                        .state
                        .field
                        .sample_nearest(position)
                        .unwrap()
                        .store_f32()
                        .unwrap(),
                    expected,
                    "{} {name} [{x},{y}]",
                    path.display()
                );
            }
            fields.push(loaded);
        }
        assert_eq!(fields[0].state.layout, fields[1].state.layout);
        for sample in evidence["samples"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["source"] == map["source"])
        {
            let field = &fields[usize::from(sample["layer"] == "MOUNTAIN_HEIGHT_MAP")]
                .state
                .field;
            let xy = serde_json::from_value(sample["position_bits"].clone()).unwrap();
            let precision = serde_json::from_value(sample["precision"].clone()).unwrap();
            let actual = if sample["method"] == "Nearest" {
                field.sample_nearest(xy)
            } else {
                field.sample_interpolated(xy, precision)
            }
            .unwrap();
            assert_eq!(
                actual.compare(&extended_value(&sample["output"])),
                Ordering::Equal,
                "{} native sample {sample}",
                path.display()
            );
            assert_eq!(
                u64::from(actual.store_f32().unwrap()),
                sample["output"]["height_bits"].as_u64().unwrap()
            );
        }
    }
    eprintln!(
        "Bound160 native height layers across80 maps; preserved{tails} nonempty chunk tails."
    );
}

fn extended_value(row: &Value) -> Binary {
    let significand = row["extended_significand"].as_u64().unwrap();
    let sign_exponent = row["extended_sign_exponent"].as_u64().unwrap() as u16;
    let pc = Precision::Extended64;
    let mut value = Binary::from_u32((significand >> 32) as u32)
        .mul(&Binary::from_f64_bits(0x41f0_0000_0000_0000).unwrap(), pc)
        .unwrap()
        .add(&Binary::from_u32(significand as u32), pc)
        .unwrap();
    if significand != 0 {
        let exponent = i32::from(sign_exponent & 0x7fff) - 16383 - 63;
        assert!((-1022..=1023).contains(&exponent));
        value = value
            .mul(
                &Binary::from_f64_bits(((exponent + 1023) as u64) << 52).unwrap(),
                pc,
            )
            .unwrap();
    }
    if sign_exponent & 0x8000 != 0 {
        value = value.negated();
    }
    value
}
