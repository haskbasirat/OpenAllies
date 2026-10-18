use super::*;
use serde_json::Value;

#[test]
fn original_overlap_raw_comparisons_preserve_non_box_nan_and_infinite_facings() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-load-nonfinite/native-overlap-unused.json"
    ))
    .unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 144);
    for row in rows {
        let a: Placement = serde_json::from_value(row["input"]["a"].clone()).unwrap();
        let b: Placement = serde_json::from_value(row["input"]["b"].clone()).unwrap();
        let p: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        assert_eq!(
            overlaps(&a, &b, p).unwrap(),
            row["native"]["result"].as_bool().unwrap(),
            "{} {}",
            row["input"]["name"],
            row["precision"]
        );
        assert!(!row["native"]["facing_reads"].as_array().unwrap().is_empty());
    }
}
#[test]
fn raw_facing_equality_is_unordered_for_nan_and_boxes_still_require_finite_arithmetic() {
    assert!(!raw_equal(0xffc00000, 0xffc00000));
    assert!(!raw_equal(0x7fc01234, 0x3f800000));
    assert!(raw_equal(0, 0x80000000));
    assert!(raw_equal(0x7f800000, 0x7f800000));
    assert!(!raw_equal(0x7f800000, 0xff800000));
    let mut box_shape = Placement {
        shape: Shape {
            kind: 2,
            radius_bits: 0x3fb504f3,
            bounds: [0xbf800000, 0xbf800000, 0x3f800000, 0x3f800000],
        },
        position: [0, 0],
        facing: [0xffc00000, 0xffc00000],
    };
    let circle = Placement {
        shape: Shape {
            kind: 1,
            radius_bits: 0x3f800000,
            bounds: [0; 4],
        },
        position: [0, 0],
        facing: [0xffc00000; 2],
    };
    assert!(overlaps(&box_shape, &circle, Precision::Double53).is_err());
    box_shape.facing = [0, 0xbf800000];
    assert!(overlaps(&box_shape, &circle, Precision::Double53).is_ok());
    box_shape.position[0] = 0x7fc00000;
    assert!(overlaps(&box_shape, &circle, Precision::Double53).is_err());
}
