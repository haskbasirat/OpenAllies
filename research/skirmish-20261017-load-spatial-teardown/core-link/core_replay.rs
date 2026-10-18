use aa_numeric::Precision;
use aa_spatial::index::{Member,SpatialIndex};
use serde_json::{Value,json};
#[test]
fn original_v14_native_removal_and_unindexed_descriptor_skip() {
    let actual: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-spatial/actual-first-kingdom.json"
    ))
    .unwrap();
    let native: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-load-spatial-teardown/native-teardown.json"
    ))
    .unwrap();
    let mut compared = 0;
    for row in native["rows"].as_array().unwrap() {
        let input = actual["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["precision"] == row["precision"])
            .unwrap();
        let pc: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let mut member: Member = serde_json::from_value(input["member"].clone()).unwrap();
        member.actor_category = row["native_class"] != 2;
        let key = member.key;
        let reference = member.reference;
        let mut index = SpatialIndex::new(
            serde_json::from_value(input["grid"].clone()).unwrap(),
            serde_json::from_value(input["initial_flags"].clone()).unwrap(),
            pc,
        )
        .unwrap();
        index.register_load_reference(key, reference).unwrap();
        index.set_event_capture(true);
        if row["member_present"] == true {
            let mut lease = index.lease_load_fields(vec![member.clone()]).unwrap();
            lease.index_mut().refresh_membership(key, true).unwrap();
            member.actor.flags = lease.index().member(key).unwrap().actor.flags;
        }
        index.take_events();
        let flags = {
            let mut lease = index.lease_load_fields(vec![member]).unwrap();
            if lease.index().member(key).unwrap().actor.flags & (1 << 26) != 0 {
                lease.index_mut().remove(key).unwrap();
            }
            let flags = lease.index().member(key).unwrap().actor.flags;
            lease.index_mut().unregister(key).unwrap();
            flags
        };
        assert_eq!(
            json!({"tiles":snapshot(&index),"member_flags":{key.to_string():flags},"trace":index.take_events()}),
            row["expected"]["after"]
        );
        assert!(index.resolve_reference(reference).is_none());
        assert!(index.member(key).is_err());
        assert_eq!(row["expected"]["global_identity_still_present"], true);
        compared += 1;
    }
    assert_eq!(compared, 18);
}

fn snapshot(index: &SpatialIndex) -> Value {
    let mut tiles = Vec::new();
    let mut flag_bytes = Vec::new();
    for y in 0..index.grid().dimensions[1] {
        for x in 0..index.grid().dimensions[0] {
            let tile = index.tile([x, y]).unwrap();
            let mut cursor = tile.first_node;
            let mut last = None;
            while let Some(node) = cursor {
                last = cursor;
                cursor = index.node(node).unwrap().next;
            }
            assert_eq!(last, tile.last_node);
            flag_bytes.extend_from_slice(&tile.flags20.to_le_bytes());
            let actors = index.actors_at([x, y]).unwrap();
            if !actors.is_empty() || tile.primary_reference != 0 {
                tiles.push(json!({"cell":[x,y],"actors":actors,
                    "flags20":tile.flags20,"primary_reference":tile.primary_reference}));
            }
        }
    }
    json!({"occupied":tiles,"tile_count":flag_bytes.len()/4,
        "flags20_le_sha256":aa_rules::sources::sha256(&flag_bytes)})
}
