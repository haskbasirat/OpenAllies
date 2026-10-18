use super::*;
fn hex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks_exact(2)
        .map(|x| u8::from_str_radix(std::str::from_utf8(x).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn actual_reader_native_metadata_and_quality_ranges() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-texture-reader/native-reader-metadata.json"
    ))
    .unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 165);
    for row in rows {
        let input = &row["input"];
        let output = &row["output"];
        let reader = match input["extension"]
            .as_str()
            .unwrap()
            .to_ascii_lowercase()
            .as_str()
        {
            "dds" => Reader::Dds,
            "tga" => Reader::Tga,
            "png" => Reader::Png,
            _ => panic!(),
        };
        let data = hex(input["bytes"].as_str().unwrap());
        let mode = input["mode"].as_u64().unwrap() as u32;
        let result = read(&data, reader, mode);
        if !output["accepted"].as_bool().unwrap() {
            assert!(result.is_err());
            continue;
        }
        let got = result.unwrap();
        assert_eq!(serde_json::json!(got.format), output["format"]);
        assert_eq!(serde_json::json!(got.widths), output["widths"]);
        assert_eq!(serde_json::json!(got.heights), output["heights"]);
        assert_eq!(serde_json::json!(got.offsets), output["offsets"]);
        assert_eq!(serde_json::json!(got.stride), output["stride"]);
        if reader == Reader::Dds {
            let original =
                if le(&data, 108).unwrap() & 0x400008 == 0x400008 && le(&data, 28).unwrap() != 1 {
                    le(&data, 28).unwrap() as usize
                } else {
                    1
                };
            assert_eq!(got.source_levels.end, original);
            let first = got.source_levels.start;
            assert_eq!(got.widths[0], le(&data, 16).unwrap() >> first);
            assert_eq!(got.heights[0], le(&data, 12).unwrap() >> first);
        } else {
            assert_eq!(got.source_levels, 0..1);
        }
    }
}

#[test]
fn unsupported_or_invalid_metadata_cannot_become_native_size() {
    assert!(read(&[], Reader::Dds, 0).is_err());
    assert!(read(&[0; 18], Reader::Tga, 0).is_err());
    assert!(read(&[0; 33], Reader::Png, 0).is_err());
    assert!(allocate(RGBA, u32::MAX, u32::MAX, 1, 0).is_err());
    assert!(allocate(RGBA, 1, 1, 16, 0).is_err());
    assert!(allocate(RGBA, 0, 1, 1, 0).is_err());
}
