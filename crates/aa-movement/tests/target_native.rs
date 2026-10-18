use aa_movement::{MoverState, target::set_target};
use aa_numeric::Precision;
use serde_json::Value;
fn mover(speed: u32) -> MoverState {
    MoverState {
        destination: [0.; 3],
        last_step_speed: 0.,
        speed: 0.,
        requested_speed: f32::from_bits(speed),
        acceleration: 0.,
        turn_rate: 0.,
        desired_direction: [0.; 3],
        target_id: 0xdeadbeef,
        target_range: Some(-777.),
        ignore_terrain_speed_limit: false,
        speed_factor: 1.,
        instant_turn: false,
        turning: false,
        falling: false,
    }
}
#[test]
fn native_target_range_compares_extended_step_before_binary32_store() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-ability-motion/native-motion-fixtures.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in f["cases"].as_array().unwrap() {
        let c = &row["input"];
        if c["operation"] != "target" {
            continue;
        }
        let mut m = mover(c["actors"]["1"]["requested_speed"].as_u64().unwrap() as u32);
        let t = c["decision"]["target"]
            .as_u64()
            .map(|i| c["actors"][i.to_string()]["id"].as_u64().unwrap() as u32);
        set_target(
            &mut m,
            t,
            c["target_range"].as_u64().unwrap() as u32,
            c["delta"].as_u64().unwrap() as u32,
            serde_json::from_value(c["precision"].clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            serde_json::json!([m.target_id, m.target_range.unwrap().to_bits()]),
            row["output"]["movers"]["1"],
            "{}",
            c["name"]
        );
        count += 1;
    }
    assert_eq!(count, 42);
}
#[test]
fn failed_numeric_boundary_keeps_target_id_and_previous_range() {
    let mut m = mover(f32::INFINITY.to_bits());
    assert!(
        set_target(
            &mut m,
            Some(0x12340005),
            1f32.to_bits(),
            1f32.to_bits(),
            Precision::Double53
        )
        .is_err()
    );
    assert_eq!(m.target_id, 0x12340005);
    assert_eq!(m.target_range, Some(-777.));
}
