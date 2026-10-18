use super::*;
#[test]
fn exact_native_loading_flag_store_keeps_same_member_and_tile_lists() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/development10/fresh-world-registration/native-loading-flag.json"
    ))
    .unwrap();
    assert_eq!(fixture["executions"], 432);
    for row in fixture["rows"].as_array().unwrap() {
        let flags = row["input"].as_u64().unwrap() as u32;
        let pc = serde_json::from_value(row["precision"].clone()).unwrap();
        let mut index = SpatialIndex::new(
            Grid {
                dimensions: [4, 4],
                coordinate_shift: 0,
                cell_size_bits: 1f32.to_bits(),
            },
            vec![0; 16],
            pc,
        )
        .unwrap();
        index.set_event_capture(true);
        let shape = Shape {
            kind: 0,
            radius_bits: 0,
            bounds: [0; 4],
        };
        index
            .register(Member {
                key: 17,
                reference: 0x10001,
                actor: Actor {
                    definition: 42,
                    flags: flags & !(1 << 26),
                    position: [1.25f32.to_bits(); 2],
                    facing: [1f32.to_bits(), 0],
                },
                height_bits: 3f32.to_bits(),
                facing_z_bits: 0,
                cached_tile: [1, 1],
                blocking: shape.clone(),
                soft: shape.clone(),
                target: shape,
                actor_category: true,
                wall_present: false,
            })
            .unwrap();
        if flags & (1 << 26) != 0 {
            index.insert(17).unwrap();
        }
        assert_eq!(index.member(17).unwrap().actor.flags, flags);
        let address = index.member(17).unwrap() as *const _ as usize;
        let mut expected = serde_json::to_value(index.member(17).unwrap()).unwrap();
        expected["actor"]["flags"] = row["output"].clone();
        let snapshot = |index: &SpatialIndex| {
            (0..4)
                .flat_map(|y| (0..4).map(move |x| [x, y]))
                .map(|cell| {
                    (
                        format!("{:?}", index.tile(cell).unwrap()),
                        index.actors_at(cell).unwrap(),
                    )
                })
                .collect::<Vec<_>>()
        };
        let tiles = snapshot(&index);
        index.take_events();
        index.clear_loading_flag(17).unwrap();
        assert_eq!(
            serde_json::to_value(index.member(17).unwrap()).unwrap(),
            expected
        );
        assert_eq!(index.member(17).unwrap() as *const _ as usize, address);
        assert_eq!(snapshot(&index), tiles);
        assert!(index.take_events().is_empty());
        assert_eq!(index.resolve_reference(0x10001), Some(17));
        assert!(
            index.clear_registration_flags(17, 1 << 31).is_err(),
            "unrelated existing API stays restricted"
        );
        let before = serde_json::to_value(index.member(17).unwrap()).unwrap();
        assert!(index.clear_loading_flag(999).is_err());
        assert_eq!(
            serde_json::to_value(index.member(17).unwrap()).unwrap(),
            before
        );
    }
}
