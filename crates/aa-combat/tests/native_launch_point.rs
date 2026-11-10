//! 0x4CFB70 (with 0x46E150 and 0x409230) against
//! research/phase8-combat/native-launch-point-fixtures.json, every case at
//! PC24, PC53 and PC64, all three output words bit for bit.
use aa_body::F32;
use aa_combat::launch_point::{
    DEFAULT_SHAPE_POINT, Shape, ShapeActor, ShapeDefinition, actor_shape_point,
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

#[test]
fn actor_shape_point_matches_native_at_every_precision() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-launch-point-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    assert_eq!(
        [word(&doc["default_vector_words"][0]), word(&doc["default_vector_words"][1])],
        [DEFAULT_SHAPE_POINT[0].0, DEFAULT_SHAPE_POINT[1].0]
    );
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 900);
    for (index, case) in cases.iter().enumerate() {
        let actor = ShapeActor {
            position: [0, 1, 2].map(|k| F32(word(&case["position"][k]))),
            facing: [0, 1].map(|k| F32(word(&case["facing"][k]))),
        };
        let definition = ShapeDefinition {
            base: shape(&case["base"]),
            alternate: shape(&case["alternate"]),
            use_alternate: word(&case["use_alternate"]) & 0xff != 0,
            height: F32(word(&case["height"])),
        };
        for (name, precision) in [
            ("Single24", Precision::Single24),
            ("Double53", Precision::Double53),
            ("Extended64", Precision::Extended64),
        ] {
            let got = actor_shape_point(&actor, &definition, precision).unwrap();
            let expected = [0, 1, 2].map(|k| word(&case["result"][name][k]));
            assert_eq!(got.map(|w| w.0), expected, "case {index} at {name}");
        }
    }
}
