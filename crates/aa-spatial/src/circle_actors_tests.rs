use super::*;
use crate::{collision, index::{ActorKey, Member}, index_world::CollisionPolicy};
use serde_json::{Value, json};

struct Policy;
fn unreachable_child<T>() -> collision::Result<T> {
    panic!("zero-mask native circle descriptor reached an unrelated child")
}
impl CollisionPolicy for Policy {
    fn definition_flags(&self, _: u32) -> collision::Result<u32> { unreachable_child() }
    fn definition_flag_1a8(&self, _: u32) -> collision::Result<bool> {
        //The fixture's allocated Data+1A8 byte remains zero.
        Ok(false)
    }
    fn custom_filter(&mut self, _: &mut SpatialIndex, _: ActorKey, _: &mut Descriptor) -> collision::Result<bool> { unreachable_child() }
    fn source_excludes(&mut self, _: &mut SpatialIndex, _: ActorKey, _: ActorKey, _: &mut Descriptor) -> collision::Result<bool> { unreachable_child() }
    fn allegiance(&mut self, _: &mut SpatialIndex, _: ActorKey, _: &mut Descriptor) -> collision::Result<Option<u32>> { unreachable_child() }
    fn hostility_dispatch(&self, _: u32) -> collision::Result<u32> { unreachable_child() }
    fn hostile(&mut self, _: &mut SpatialIndex, _: u32, _: Option<u32>, _: Option<u32>, _: &mut Descriptor) -> collision::Result<bool> { unreachable_child() }
    fn moving_exclusion(&mut self, _: &mut SpatialIndex, _: ActorKey, _: &mut Descriptor) -> collision::Result<bool> { unreachable_child() }
    fn outer_actor(&mut self, _: &mut SpatialIndex, _: ActorKey, _: &mut Descriptor) -> collision::Result<Option<u32>> { unreachable_child() }
}
impl QueryPolicy for Policy {
    fn definition_has_object_data(&self, _: u32) -> collision::Result<bool> { unreachable_child() }
}

#[test]
fn actual_original_v14_mode1_circle_actor_order_and_cleanup() {
    let doc: Value = serde_json::from_str(include_str!("../../../research/live-battle-ticks/native-circle-actors.json")).unwrap();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 90);
    for c in rows {
        let input = &c["input"];
        let mut index = SpatialIndex::new(
            serde_json::from_value(input["grid"].clone()).unwrap(),
            serde_json::from_value(input["flags"].clone()).unwrap(),
            serde_json::from_value(c["precision"].clone()).unwrap(),
        )
        .unwrap();
        for member in input["members"].as_array().unwrap() {
            let m: Member = serde_json::from_value(member.clone()).unwrap();
            let key = m.key;
            index.register(m).unwrap();
            index.insert(key).unwrap();
        }
        for key in input["premarked"].as_array().unwrap() {
            let key = key.as_u64().unwrap() as u32;
            index.set_query_link(key, Some(key)).unwrap();
        }
        let geometry = QueryGeometry::from_native_extents(
            &index,
            serde_json::from_value(input["extent_bits"].clone()).unwrap(),
            index.precision(),
        )
        .unwrap();
        let mut cursor = Cursor::begin(
            &mut index,
            &geometry,
            serde_json::from_value(input["position"].clone()).unwrap(),
            input["radius"].as_u64().unwrap() as u32,
            serde_json::from_value(input["source_facing"].clone()).unwrap(),
        )
        .unwrap();
        let mut found = Vec::new();
        while let Some(reference) = cursor.next(&mut index, &mut Policy).unwrap() {
            found.push(reference);
        }
        cursor.finish(&mut index).unwrap();
        assert_eq!(json!(found), c["native"]["references"], "{}", c["scenario"]);
        for m in index.members() {
            assert_eq!(json!(index.query_link(m.key).unwrap()), c["native"]["links"][m.key.to_string()], "{}", c["scenario"]);
        }
    }
}
