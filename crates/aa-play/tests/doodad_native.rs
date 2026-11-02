//! The DynamicDoodad deferred updates against the original's code under Unicorn:
//! research/phase8-doodad/native-doodad.json (240 runs of 0x5902C0, 480 of
//! 0x58FA30, PC53).
use aa_play::data::{BreakableDef, DisturbableDef};
use aa_play::doodad::{BreakableWords, breakable_step, disturbable_step};
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../research/phase8-doodad/native-doodad.json")).unwrap()
}

fn bits(v: &Value, key: &str) -> u32 {
    v[key].as_u64().unwrap_or_else(|| panic!("{key}")) as u32
}
fn float(v: &Value, key: &str) -> f32 {
    f32::from_bits(bits(v, key))
}
fn ops(v: &Value) -> Vec<&str> {
    v["log"].as_array().unwrap().iter().map(|e| e[0].as_str().unwrap()).collect()
}
fn rearm(v: &Value) -> Option<(u32, u32)> {
    v["log"].as_array().unwrap().iter().find(|e| e[0] == "rearm").map(|e| (e[1].as_u64().unwrap() as u32, e[2].as_u64().unwrap() as u32))
}

#[test]
fn disturbable_step_is_the_original_0x5902c0() {
    let doc = fixture();
    let cases = doc["disturbable"].as_array().unwrap();
    assert_eq!(cases.len(), 240);
    for (i, case) in cases.iter().enumerate() {
        let (c, want) = (&case["input"], &case["output"]);
        let d = DisturbableDef { update_time: float(c, "update_time"), affect_radius: 1.0, recharge_time_min: float(c, "min"), recharge_time_max: float(c, "max"), actions: 1 };
        let scan = c["scan"].as_bool().unwrap();
        let mut drawn = 0;
        let out = disturbable_step(scan, float(c, "next"), float(c, "now"), &d, || {
            drawn += 1;
            bits(c, "draw")
        });
        let log = ops(want);
        assert_eq!(out.acted, log.contains(&"act"), "case {i}: action list");
        assert_eq!(drawn, u32::from(log.contains(&"draw")), "case {i}: draw taken");
        assert_eq!(out.draws, drawn, "case {i}");
        assert_eq!(out.next.to_bits(), bits(want, "next"), "case {i}: next time");
        assert_eq!(Some((out.rearm.to_bits(), 0)), rearm(want), "case {i}: re-arm");
    }
}

#[test]
fn breakable_step_is_the_original_0x58fa30() {
    let doc = fixture();
    let cases = doc["breakable"].as_array().unwrap();
    assert_eq!(cases.len(), 480);
    for (i, case) in cases.iter().enumerate() {
        let (c, want) = (&case["input"], &case["output"]);
        let d = BreakableDef {
            update_time: float(c, "update_time"),
            topple_duration: float(c, "topple"),
            bounce_duration: float(c, "bounce"),
            regrow: bits(c, "regrow") != 0,
            regrow_sink_duration: float(c, "sink"),
            regrow_dead_duration: float(c, "dead"),
            regrow_grow_duration: float(c, "grow"),
        };
        let words = BreakableWords { state: bits(c, "state"), time: float(c, "time"), grow_start: float(c, "grow_start"), scale: float(c, "scale") };
        let out = breakable_step(words, float(c, "now"), &d);
        let log = ops(want);
        assert_eq!(out.cancelled, log.contains(&"cancel"), "case {i}: cancel");
        assert_eq!(out.toppled, log.contains(&"topple"), "case {i}: topple");
        assert_eq!(out.words.state, bits(want, "state"), "case {i}: state");
        assert_eq!(out.words.time.to_bits(), bits(want, "time"), "case {i}: time");
        assert_eq!(out.words.grow_start.to_bits(), bits(want, "grow_start"), "case {i}: grow start");
        assert_eq!(out.words.scale.to_bits(), bits(want, "scale"), "case {i}: scale");
        assert_eq!(out.rearm.map(|r| (r.to_bits(), 0)), rearm(want), "case {i}: re-arm");
    }
}
