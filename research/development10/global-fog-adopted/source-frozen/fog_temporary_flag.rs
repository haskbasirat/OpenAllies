//!Storage regression for54F400's bit30 writes; native control flow has its
//!independent1728-row original/v14 fixture. No actor/source lifecycle is minted.
use aa_spatial::index::{Member, SpatialIndex};
use serde_json::{Value, json};

#[test]
fn fog_bit30_keeps_same_current_member_geometry_and_nodes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-spatial/native-index.json"
    )).unwrap();
    for case in fixture["cases"].as_array().unwrap().iter().take(3) {
        for extra in [0, 1 << 30, 0x8000_0810, 0xffff_ffff & !(1 << 26)] {
            let mut index = SpatialIndex::new(
                serde_json::from_value(case["grid"].clone()).unwrap(),
                serde_json::from_value(case["flags"].clone()).unwrap(),
                serde_json::from_value(case["precision"].clone()).unwrap(),
            ).unwrap();
            let mut member: Member = serde_json::from_value(case["members"][0].clone()).unwrap();
            member.actor.flags = extra;
            let key = member.key;
            let reference = member.reference;
            index.register(member).unwrap();
            index.insert(key).unwrap();
            let initial_flags = index.member(key).unwrap().actor.flags;
            let address = std::ptr::from_ref(index.member(key).unwrap());
            let before = serde_json::to_value(index.member(key).unwrap()).unwrap();
            let tiles = |index: &SpatialIndex| {
                let [width, height] = index.grid().dimensions;
                (0..height).flat_map(|y| (0..width).map(move |x| [x,y]))
                    .map(|xy| serde_json::to_value(index.tile(xy).unwrap()).unwrap()).collect::<Vec<_>>()
            };
            let before_tiles = tiles(&index);
            index.set_event_capture(true);
            let _ = index.take_events();
            for enabled in [false, false, true, true, false] {
                index.set_fog_temporary_flag(key, enabled).unwrap();
                let mut expected = before.clone();
                expected["actor"]["flags"] = json!(if enabled {initial_flags | (1 << 30)} else {initial_flags & !(1 << 30)});
                assert_eq!(serde_json::to_value(index.member(key).unwrap()).unwrap(), expected);
                assert_eq!(std::ptr::from_ref(index.member(key).unwrap()), address);
                assert_eq!(index.resolve_reference(reference), Some(key));
                assert_eq!(tiles(&index), before_tiles);
                assert!(index.take_events().is_empty());
            }
            let before_error = serde_json::to_value(index.member(key).unwrap()).unwrap();
            assert!(index.set_fog_temporary_flag(u32::MAX, true).is_err());
            assert_eq!(serde_json::to_value(index.member(key).unwrap()).unwrap(), before_error);
        }
    }
}
