use aa_assets::{TerrainLayerInput, TerrainLayerPlanner, TerrainTransitionMask};

#[test]
fn original_x86_neighborhood_layer_and_mask_vectors() {
    let report: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase5-terrain/native-layer-fixtures.json"
    ))
    .unwrap();
    let planner = TerrainLayerPlanner::bundled().unwrap();
    let cases = report["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 1027);
    for (case_index, case) in cases.iter().enumerate() {
        let grid = std::array::from_fn(|i| {
            let id = case["neighbors"][i].as_u64().unwrap() as u32;
            TerrainLayerInput {
                palette_index: id,
                priority: case["priorities"][id as usize].as_i64().unwrap() as i32,
            }
        });
        let actual = planner.plan(grid).unwrap();
        let expected = case["layers"].as_array().unwrap();
        assert_eq!(actual.len(), expected.len(), "case {case_index}");
        for (layer, native) in actual.iter().zip(expected) {
            assert_eq!(layer.palette_index, native["id"].as_u64().unwrap() as u32);
            assert_eq!(layer.priority, native["priority"].as_i64().unwrap() as i32);
            assert_eq!(layer.neighbor_mask, native["mask"].as_u64().unwrap() as u8);
            assert_eq!(
                layer.transition.as_ref().map(|m| u64::from(m.base_mask)),
                native["base_mask"].as_u64(),
                "case {case_index}"
            );
            if let Some(mask) = &layer.transition {
                assert_eq!(
                    u64::from(mask.transform),
                    native["transform"].as_u64().unwrap()
                );
                let expected_matrix = &report["table"][usize::from(layer.neighbor_mask)]["matrix"];
                for (component, expected) in
                    mask.matrix.iter().zip(expected_matrix.as_array().unwrap())
                {
                    assert_eq!(f64::from(*component), expected.as_f64().unwrap());
                }
            }
        }
    }
}

#[test]
fn ambiguous_priorities_are_explicit_and_mask_filenames_match_native_resources() {
    let planner = TerrainLayerPlanner::bundled().unwrap();
    let center = TerrainLayerInput {
        palette_index: 10,
        priority: 20,
    };
    let mut grid = [center; 9];
    let layers = planner.plan(grid).unwrap();
    assert_eq!(layers.len(), 1);
    assert!(layers[0].transition.is_none());
    grid[0].priority = 2;
    assert!(
        planner.plan(grid).is_err(),
        "same ID cannot have conflicting priorities"
    );
    grid[0] = TerrainLayerInput {
        palette_index: 11,
        priority: 20,
    };
    assert!(
        planner.plan(grid).is_err(),
        "native equal-priority order is unresolved"
    );
    grid[0].priority = 2;
    let layers = planner.plan(grid).unwrap();
    assert_eq!(layers.len(), 2);
    let mask = layers[1].transition.as_ref().unwrap();
    // Actual native shape for a single lower-priority corner neighbor.
    assert_eq!(mask.filename_suffix(), "12345678.tga");
}

#[test]
fn all_native_base_mask_names_resolve_to_original_authored_assets() {
    let report: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase5-terrain/native-layer-fixtures.json"
    ))
    .unwrap();
    let sources = report["authored_resources"].as_array().unwrap();
    assert_eq!(sources.len(), 33);
    let table = report["table"].as_array().unwrap();
    let mut bases = std::collections::BTreeSet::new();
    for entry in table.iter().filter(|v| !v.is_null()) {
        let mask: TerrainTransitionMask = serde_json::from_value(entry.clone()).unwrap();
        bases.insert(mask.base_mask);
        let source = sources
            .iter()
            .find(|s| s["base_mask"].as_u64().unwrap() == u64::from(mask.base_mask))
            .unwrap();
        assert_eq!(
            mask.filename_suffix(),
            source["filename_suffix"].as_str().unwrap()
        );
        assert!(source["width"].as_u64().unwrap() > 0);
        assert!(source["height"].as_u64().unwrap() > 0);
    }
    assert_eq!(bases.len(), 33);
}
