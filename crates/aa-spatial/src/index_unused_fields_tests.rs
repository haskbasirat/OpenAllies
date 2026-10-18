use super::*;
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-load-nonfinite/native-unused-fields.json"
    ))
    .unwrap()
}
fn index(input: &Value) -> SpatialIndex {
    let mut index = SpatialIndex::new(
        serde_json::from_value(input["grid"].clone()).unwrap(),
        serde_json::from_value(input["initial_flags"].clone()).unwrap(),
        serde_json::from_value(input["precision"].clone()).unwrap(),
    )
    .unwrap();
    index.set_event_capture(true);
    index
}
fn compare(index: &mut SpatialIndex, input: &Value, native: &Value) {
    let mut occupied = Vec::new();
    for y in 0..index.grid.dimensions[1] {
        for x in 0..index.grid.dimensions[0] {
            let cell = [x, y];
            let tile = index.tile(cell).unwrap();
            let actors = index.actors_at(cell).unwrap();
            assert_eq!(
                tile.flags20,
                input["initial_flags"][(y * index.grid.dimensions[0] + x) as usize]
                    .as_u64()
                    .unwrap() as u32
            );
            if !actors.is_empty() || tile.primary_reference != 0 {
                occupied.push(json!({"cell":cell,"actors":actors,"flags20":tile.flags20,"primary_reference":tile.primary_reference}));
            }
        }
    }
    assert_eq!(json!(occupied), native["tiles"]["occupied"]);
    assert_eq!(
        json!({"1":index.member(1).unwrap().actor.flags}),
        native["member_flags"]
    );
    assert_eq!(json!(index.take_events()), native["trace"]);
    let actual = index.member(1).unwrap();
    assert_eq!(
        json!(actual.actor.facing),
        input["member"]["actor"]["facing"]
    );
    assert_eq!(json!(actual.height_bits), input["member"]["height_bits"]);
    assert_eq!(
        json!(actual.facing_z_bits),
        input["member"]["facing_z_bits"]
    );
}

#[test]
fn original_circle_and_point_membership_preserve_unused_nonfinite_words() {
    let fixture = fixture();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 30);
    for row in cases {
        let input = &row["input"];
        let member: Member = serde_json::from_value(input["member"].clone()).unwrap();
        //Direct registration and actual membership share the same read domain.
        let mut direct = index(input);
        direct.register(member.clone()).unwrap();
        direct.refresh_membership(1, true).unwrap();
        compare(&mut direct, input, &row["native"]["first"]);
        direct.refresh_membership(1, true).unwrap();
        compare(&mut direct, input, &row["native"]["second"]);

        //Pre-staging fields then the same owned-field adoption, with no tile
        //node recreation, preserve both identity and raw unused pose words.
        let mut leased = index(input);
        leased.register_load_reference(1, member.reference).unwrap();
        let committed = {
            let mut lease = leased.lease_load_fields(vec![member]).unwrap();
            lease.index_mut().refresh_membership(1, true).unwrap();
            compare(lease.index_mut(), input, &row["native"]["first"]);
            lease.index().member(1).unwrap().clone()
        };
        assert_eq!(leased.members().count(), 0);
        leased.attach_loaded_fields(committed).unwrap();
        leased.refresh_membership(1, true).unwrap();
        compare(&mut leased, input, &row["native"]["second"]);
    }
}

#[test]
fn reached_box_facing_and_position_remain_in_the_finite_domain() {
    let fixture = fixture();
    let input = &fixture["cases"][1]["input"];
    let poison: Member = serde_json::from_value(input["member"].clone()).unwrap();
    for route in 0..3 {
        let mut member = poison.clone();
        if route < 2 {
            member.soft.kind = 2;
            member.soft.bounds = [0xbf800000, 0xbf800000, 0x3f800000, 0x3f800000];
        } else {
            member.actor.position[0] = 0x7fc00000;
        }
        let mut live = index(input);
        if route == 0 {
            assert!(live.register(member).is_err());
        } else {
            live.register_load_reference(1, member.reference).unwrap();
            if route == 1 {
                assert!(live.lease_load_fields(vec![member]).is_err());
            } else {
                assert!(live.attach_loaded_fields(member).is_err());
            }
        }
        assert!(live.members().next().is_none());
        assert!(live.take_events().is_empty());
    }
    let mut foreign = index(input);
    assert!(foreign.lease_load_fields(vec![poison]).is_err()); //Actual generation binding remains mandatory.
}
