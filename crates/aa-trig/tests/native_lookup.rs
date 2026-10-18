use aa_numeric::Precision;
use aa_trig::{Error, lookup_cos, lookup_sin, rotate};
use serde::Deserialize;

#[derive(Deserialize)]
struct Lookup {
    precision: Precision,
    function: String,
    angle_bits: u32,
    result_bits: u32,
    index: i32,
    in_table: bool,
}
#[derive(Deserialize)]
struct Rotation {
    precision: Precision,
    from_bits: [u32; 2],
    angle_bits: u32,
    clockwise: bool,
    result_bits: [u32; 2],
    indices: [i32; 2],
    in_table: bool,
}
#[derive(Deserialize)]
struct Fixture {
    lookups: Vec<Lookup>,
    rotations: Vec<Rotation>,
}
fn fixture() -> Fixture {
    serde_json::from_str(include_str!(
        "../../../research/phase4-trig/native-trig-vectors.json"
    ))
    .unwrap()
}

#[test]
fn lookup_matches_original_or_explicitly_rejects_out_of_table_access() {
    let rows = fixture().lookups;
    assert_eq!(rows.len(), 1452);
    for row in rows {
        let result = match row.function.as_str() {
            "sin" => lookup_sin(row.angle_bits, row.precision),
            "cos" => lookup_cos(row.angle_bits, row.precision),
            _ => panic!("unknown oracle function"),
        };
        if row.in_table {
            assert_eq!(
                result.unwrap(),
                row.result_bits,
                "{} {:08x} {:?}, index {}",
                row.function,
                row.angle_bits,
                row.precision,
                row.index
            );
        } else {
            assert!(matches!(result, Err(Error::InvalidLookupIndex(i)) if i == row.index));
        }
    }
}

#[test]
fn native_rotation_keeps_quantization_and_original_store_order() {
    let rows = fixture().rotations;
    assert_eq!(rows.len(), 480);
    for row in rows {
        let result = rotate(row.from_bits, row.angle_bits, row.clockwise, row.precision);
        if row.in_table {
            assert_eq!(
                result.unwrap(),
                row.result_bits,
                "{:x?} angle {:08x} clockwise {} {:?}",
                row.from_bits,
                row.angle_bits,
                row.clockwise,
                row.precision
            );
        } else {
            assert!(
                matches!(result, Err(Error::InvalidLookupIndex(i)) if row.indices.contains(&i))
            );
        }
    }
}

#[test]
fn unsupported_nonfinite_or_integer_overflow_inputs_are_checked() {
    for value in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN, f32::MAX] {
        assert!(lookup_sin(value.to_bits(), Precision::Extended64).is_err());
        assert!(lookup_cos(value.to_bits(), Precision::Single24).is_err());
    }
}
