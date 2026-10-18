use super::*;
use serde_json::Value;

#[test]
fn native_registration_cached_or_shape_iterator_and_application_gate() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-zone-entry/native-grid-registration.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 300);
    let grid = Grid {
        dimensions: [8, 6],
        coordinate_shift: 2,
        cell_size_bits: 4f32.to_bits(),
    };
    let point = Shape {
        kind: 0,
        radius_bits: 0,
        bounds: [0; 4],
    };
    let mut saw_empty_cached = false;
    let mut saw_multi = false;
    for row in rows {
        let c = &row["input"];
        let p: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let words: [u32; 6] = serde_json::from_value(c["shape"].clone()).unwrap();
        let placement = Placement {
            shape: Shape {
                kind: words[0],
                radius_bits: words[1],
                bounds: words[2..].try_into().unwrap(),
            },
            facing: serde_json::from_value(c["facing"].clone()).unwrap(),
            position: serde_json::from_value(c["position"].clone()).unwrap(),
        };
        let flags = c["flags"].as_u64().unwrap() as u32;
        let cached = serde_json::from_value(c["cached"].clone()).unwrap();
        let mut tiles = vec![];
        visit_indices(
            &grid,
            [32f32.to_bits(), 24f32.to_bits()],
            &point,
            &placement,
            flags,
            cached,
            p,
            |t| {
                tiles.push(t);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(serde_json::json!(tiles), row["output"]["tile_calls"]);
        let enabled = c["application_e4"] != 3 || c["world48"] != 0;
        let leaves = if enabled { tiles.clone() } else { vec![] };
        assert_eq!(serde_json::json!(leaves), row["output"]["leaf_calls"]);
        saw_empty_cached |= tiles.is_empty();
        saw_multi |= tiles.len() > 1;
        assert_eq!(row["output"]["flags_after"], flags);
        // Callback failures stop before later geometry visits or dispatches.
        if tiles.len() > 1 {
            let mut prefix = vec![];
            let result = visit_indices(
                &grid,
                [32f32.to_bits(), 24f32.to_bits()],
                &point,
                &placement,
                flags,
                cached,
                p,
                |t| {
                    prefix.push(t);
                    Err(Error::Unsupported("explicit test-only callback refusal"))
                },
            );
            assert!(result.is_err());
            assert_eq!(prefix, tiles[..1]);
        }
    }
    assert!(saw_empty_cached && saw_multi);
}
