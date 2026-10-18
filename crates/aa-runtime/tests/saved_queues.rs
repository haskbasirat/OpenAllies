use aa_numeric::Precision;
use aa_runtime::{saved_queues, updatables::Table};
use aa_world::initialization::ClockPlan;
fn bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn all_eighty_original_map_queue_loads_preserve_exact_buckets_and_order() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-runtime/native-saved-queues.json"
    ))
    .unwrap();
    let plan = ClockPlan::from_tick_rate(16f32.to_bits(), Precision::Double53).unwrap();
    assert_eq!(fixture["maps"].as_array().unwrap().len(), 80);
    for case in fixture["maps"].as_array().unwrap() {
        let data = bytes(case["payload"].as_str().unwrap());
        let count = i32::from_le_bytes(data[..4].try_into().unwrap());
        let mut table = Table::new(count).unwrap();
        for id in 0..count {
            table.register(id, id as u64).unwrap();
        }
        let decoded = saved_queues::decode(
            &data,
            case["world_version"].as_u64().unwrap() as u32,
            &table,
            |&v| v,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&decoded.boundaries).unwrap(),
            case["boundaries"],
            "{}",
            case["map"]
        );
        let mut queues = plan
            .initialize_queues(plan.new_clock(), Precision::Double53)
            .unwrap();
        assert_eq!(decoded.install(&mut queues).unwrap(), 0);
        assert_eq!(
            serde_json::to_value(queues.snapshot()).unwrap(),
            case["snapshot"],
            "{}",
            case["map"]
        );
    }
}
#[test]
fn original_null_slot_discards_do_not_reorder_remaining_updates() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-runtime/native-saved-queues.json"
    ))
    .unwrap();
    let case = &fixture["maps"][0];
    let data = bytes(case["payload"].as_str().unwrap());
    let count = i32::from_le_bytes(data[..4].try_into().unwrap());
    let mut table = Table::new(count).unwrap();
    for id in (0..count).filter(|id| id % 2 == 0) {
        table.register(id, id as u64).unwrap();
    }
    let decoded = saved_queues::decode(
        &data,
        case["world_version"].as_u64().unwrap() as u32,
        &table,
        |&v| v,
    )
    .unwrap();
    let mut expected = case["snapshot"].clone();
    let mut removed = 0;
    for queue in expected.as_array_mut().unwrap() {
        for bucket in queue["buckets"].as_array_mut().unwrap() {
            let v = bucket.as_array_mut().unwrap();
            let before = v.len();
            v.retain(|e| e[0].as_u64().unwrap() % 2 == 0);
            removed += before - v.len();
        }
    }
    assert_eq!(decoded.discarded_updates, removed);
    assert!(removed > 0);
    let plan = ClockPlan::from_tick_rate(16f32.to_bits(), Precision::Double53).unwrap();
    let mut queues = plan
        .initialize_queues(plan.new_clock(), Precision::Double53)
        .unwrap();
    decoded.install(&mut queues).unwrap();
    assert_eq!(serde_json::to_value(queues.snapshot()).unwrap(), expected);
}
