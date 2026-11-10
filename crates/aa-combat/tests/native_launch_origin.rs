//! 0x4BE780 (every branch, with 0x4BE740 and 0x4CFB70) against
//! research/phase8-combat/native-launch-origin-fixtures.json at PC24, PC53
//! and PC64, all three output words bit for bit.
use aa_body::F32;
use aa_combat::launch_point::{
    LaunchLocal, LaunchSource, Shape, ShapeActor, ShapeDefinition, launch_origin,
};
use aa_numeric::Precision;
use serde_json::Value;

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

fn shape(v: &Value) -> Shape {
    Shape {
        kind: word(&v[0]),
        radius: F32(word(&v[1])),
        min: [F32(word(&v[2])), F32(word(&v[3]))],
        max: [F32(word(&v[4])), F32(word(&v[5]))],
    }
}

fn three(v: &Value) -> [F32; 3] {
    [0, 1, 2].map(|k| F32(word(&v[k])))
}

#[test]
fn launch_origin_matches_native_at_every_precision() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-launch-origin-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 600);
    for (index, case) in cases.iter().enumerate() {
        let local = LaunchLocal { has_entry: word(&case["projectile"]) != 0, offset: three(&case["offset"]) };
        let actor = ShapeActor {
            position: three(&case["position"]),
            facing: [F32(word(&case["facing"][0])), F32(word(&case["facing"][1]))],
        };
        let definition = ShapeDefinition {
            base: shape(&case["base"]),
            alternate: shape(&case["alternate"]),
            use_alternate: word(&case["use_alternate"]) & 0xff != 0,
            height: F32(word(&case["height"])),
        };
        let source = match case["source"].as_u64() {
            None => LaunchSource::None,
            Some(0xa2_99d8) => LaunchSource::Kingdom,
            Some(0xa2_73f0 | 0xa2_9490) => LaunchSource::Actor { actor: &actor, definition: &definition, ordinary: true },
            Some(0xa2_8a00) => LaunchSource::Actor { actor: &actor, definition: &definition, ordinary: false },
            Some(other) => panic!("vtable {other:#x}"),
        };
        for (name, precision) in [
            ("Single24", Precision::Single24),
            ("Double53", Precision::Double53),
            ("Extended64", Precision::Extended64),
        ] {
            let got = launch_origin(&local, source, three(&case["fallback"]), precision).unwrap();
            let expected = [0, 1, 2].map(|k| word(&case["result"][name][k]));
            assert_eq!(got.map(|w| w.0), expected, "case {index} at {name}");
        }
    }
}
