//! Replays research/phase8-triggers/native-capture.json: the ORIGINAL
//! TriggerManager::QueueCaptured 54BEA0 (54B820 payload) under Unicorn. The
//! Rust binding for aa-combat's capture notification is
//! `predicates::queue_captured`; it must record the same virtual calls and
//! leave the same manager+50 queue (newest first).
mod fixture;
use aa_triggers::predicates::{Event, queue_captured};
use fixture::{FixtureWorld, manager};
use serde_json::Value;

#[test]
fn queue_captured_matches_native_54bea0() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-triggers/native-capture.json");
    let doc: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    for case in cases {
        let (c, out) = (&case["input"], &case["output"]);
        let name = c["name"].as_str().unwrap();
        let mut world = FixtureWorld::build(c);
        let mut m = manager(c, &world);
        let before = m.events().len();
        let mut result = Ok(());
        for (i, push) in c["pushes"].as_array().unwrap().iter().enumerate() {
            let object = |v: &Value| v.as_str().map(|n| world.index[n].0);
            let (captured, source) = (object(&push[0]), object(&push[1]));
            result = queue_captured(&mut m, 1000 + i as u64, captured, source, &mut world);
            if result.is_err() {
                break;
            }
        }
        assert_eq!(result.is_err(), out.get("fault").is_some(), "{name}: {result:?}");
        assert_eq!(serde_json::Value::Array(world.calls.clone()), out["calls"], "{name}");
        let native = out["events"].as_array().unwrap();
        assert_eq!(m.events().len(), native.len(), "{name}");
        for (got, want) in m.events().iter().zip(native) {
            let got = got.as_ref().unwrap();
            if want.get("fixture").is_some() {
                assert!(got.identity <= before as u64, "{name}");
                continue;
            }
            let w: Vec<u32> = want["words"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
            assert_eq!(
                got.data,
                Event::Captured { actor_id: w[0], capturer_id: w[1], kingdom_id: w[2] },
                "{name}"
            );
        }
    }
    println!("{} native 54BEA0 cases", cases.len());
}
