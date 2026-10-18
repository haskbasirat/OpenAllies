use super::*;
#[test]
fn same_map_solver_allocations_move_without_reconstruction_and_reject_another_map() {
    let geometry = MapGeometry {
        shift: 1,
        tile_size_bits: 2.0f32.to_bits(),
        extent_bits: [16.0f32.to_bits(); 2],
        width: 8,
        height: 8,
    };
    let map = ZoneMap::new(geometry).unwrap();
    let state = State::from_world_init(&map).unwrap();
    let release = std::ptr::from_ref(state.release.as_ref());
    let movement = std::ptr::from_ref(state.movement.as_ref());
    let (state, mut map) = (state, map);
    state.validate_map(&map).unwrap();
    map.tiles.reserve(128);
    state.validate_map(&map).unwrap();
    assert!(state.validate_map(&map.clone()).is_err());
    assert_eq!(std::ptr::from_ref(state.release.as_ref()), release);
    assert_eq!(std::ptr::from_ref(state.movement.as_ref()), movement);
    assert!(
        state
            .validate_map(&ZoneMap::new(geometry).unwrap())
            .is_err()
    );
    let mut altered = map;
    altered.tiles[0].fog = 1;
    assert!(State::from_world_init(&altered).is_err());
    let witness = altered.owner_witness();
    drop(altered);
    assert!(!witness.matches(&ZoneMap::new(geometry).unwrap()));
}
#[test]
fn native_presentation_gate_matches24_rows_and_skips_view_read_in_phase6() {
    let fixture: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../research/skirmish-20261016-load-zone-globals/native-zone-globals.json"
    ))
    .unwrap();
    let rows = fixture["presentation_rows"].as_array().unwrap();
    assert_eq!(rows.len(), 24);
    for row in rows {
        let mut reads = 0;
        let result = present(row["phase"].as_u64().unwrap() as u32, || {
            reads += 1;
            row["view"].as_bool().unwrap().then_some(71)
        });
        assert_eq!(reads, row["native"]["view_reads"].as_array().unwrap().len());
        if row["native"]["outcome"] == "complete" {
            result.unwrap();
        } else {
            assert!(result.unwrap_err().to_string().contains("43BA80"));
        }
    }
    let ctors = fixture["constructors"].as_array().unwrap();
    assert_eq!(ctors.len(), 9);
    for row in ctors {
        let n = &row["native"];
        assert_eq!(n["release_class"], "0xa36048");
        assert_eq!(n["move_class"], "0xa36954");
        assert_eq!(n["release_map_same"], true);
        assert_eq!(n["heap_count"], 0);
        assert_eq!(n["native_capacity"], 8092);
        assert_eq!(n["all_unused_elements_constructed"], true);
    }
}
