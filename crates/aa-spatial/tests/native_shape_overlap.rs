use aa_numeric::Precision;
use aa_spatial::{geometry::Placement, shape_overlap};
use serde::Deserialize;
#[derive(Deserialize)]
struct Case {
    precision: Precision,
    a: Placement,
    b: Placement,
    result: bool,
}
#[derive(Deserialize)]
struct Footprint {
    precision: Precision,
    placement: Placement,
    result: [[u32; 2]; 3],
}
#[derive(Deserialize)]
struct Sat {
    precision: Precision,
    a: [[u32; 2]; 3],
    b: [[u32; 2]; 3],
    result: bool,
}
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
    footprints: Vec<Footprint>,
    sat: Vec<Sat>,
}
fn fixture() -> Fixture {
    serde_json::from_str(include_str!(
        "../../../research/phase7-shape-overlap/native-overlap.json"
    ))
    .unwrap()
}
#[test]
fn original_mode_zero_footprints_preserve_asymmetric_float_stores() {
    for (index, c) in fixture().footprints.into_iter().enumerate() {
        assert_eq!(
            shape_overlap::footprint(&c.placement, c.precision).unwrap(),
            c.result,
            "footprint{index}"
        );
    }
}
#[test]
fn original_rotated_projection_comparisons_match() {
    for (index, c) in fixture().sat.into_iter().enumerate() {
        assert_eq!(
            shape_overlap::footprints_overlap(c.a, c.b, c.precision).unwrap(),
            c.result,
            "SAT{index}"
        );
    }
}
#[test]
fn original_all_shape_pair_dispatch_matches() {
    for (index, c) in fixture().cases.into_iter().enumerate() {
        assert_eq!(
            shape_overlap::overlaps(&c.a, &c.b, c.precision).unwrap(),
            c.result,
            "dispatch{index}:{}:{}",
            c.a.shape.kind,
            c.b.shape.kind
        );
    }
}
#[test]
fn unsupported_numeric_inputs_do_not_invent_overlap_results() {
    let c = fixture().cases.into_iter().next().unwrap();
    for which in 0..4 {
        let mut a = c.a.clone();
        match which {
            0 => a.shape.kind = 4,
            1 => a.position[0] = f32::NAN.to_bits(),
            2 => {
                a.facing[1] = f32::INFINITY.to_bits();
                // Case 0 is point/point: 46EF30 only FUCOMPP-compares these
                // facings and 46F006 keeps shapes 0/1 out of the rotated
                // branch. Unmodified native 46EF30 (original and v14, all
                // three PCs, research/skirmish-20261017-load-nonfinite/
                // native_overlap_unused.py run()) returns the finite result.
                assert_eq!(
                    shape_overlap::overlaps(&a, &c.b, c.precision).unwrap(),
                    c.result
                );
                // A box consumes its facing in the local-frame transform:
                // nonfinite arithmetic stays outside the ported domain.
                a.shape.kind = 2;
            }
            _ => a.shape.bounds[2] = f32::NAN.to_bits(),
        }
        assert!(shape_overlap::overlaps(&a, &c.b, c.precision).is_err());
    }
    let mut a = c.a;
    a.shape.kind = 3;
    a.shape.bounds[2] = f32::MAX.to_bits();
    assert!(shape_overlap::footprint(&a, c.precision).is_err());
}
