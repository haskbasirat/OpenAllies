use aa_heightfield::{Error, Heightfield};
use aa_numeric::{Binary, Precision};
use serde::Deserialize;
use serde_json::Value;
use std::cmp::Ordering;

#[derive(Deserialize)]
struct Input {
    name: String,
    method: String,
    precision: Precision,
    position_bits: [u32; 2],
    grid: Heightfield,
}

// Decode the original FSTP-tbyte return without rounding it through host floats.
// The fixture domain stays within binary64 exponent range, but keeps all 64
// significand bits using two exactly represented u32 limbs.
fn extended_value(row: &Value) -> Binary {
    let sig = row["extended_significand"].as_u64().unwrap();
    let sign_exp = row["extended_sign_exponent"].as_u64().unwrap() as u16;
    let p = Precision::Extended64;
    let magnitude = Binary::from_u32((sig >> 32) as u32)
        .mul(&Binary::from_f64_bits(0x41f0000000000000).unwrap(), p)
        .unwrap()
        .add(&Binary::from_u32(sig as u32), p)
        .unwrap();
    let mut value = if sig == 0 {
        magnitude
    } else {
        let exponent = i32::from(sign_exp & 0x7fff) - 16383 - 63;
        assert!((-1022..=1023).contains(&exponent));
        let scale = Binary::from_f64_bits(((exponent + 1023) as u64) << 52).unwrap();
        magnitude.mul(&scale, p).unwrap()
    };
    if sign_exp & 0x8000 != 0 {
        value = value.negated();
    }
    value
}

#[test]
fn original_nearest_and_interpolated_samples_preserve_extended_returns() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase5-heightfield/native-heightfield-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    for row in doc["fixtures"].as_array().unwrap() {
        let c: Input = serde_json::from_value(row["input"].clone()).unwrap();
        let actual = match c.method.as_str() {
            "Nearest" => c.grid.sample_nearest(c.position_bits),
            "Interpolated" => c.grid.sample_interpolated(c.position_bits, c.precision),
            _ => panic!("unknown method"),
        }
        .unwrap_or_else(|e| panic!("{}: {e}", c.name));
        let expected = extended_value(&row["output"]);
        assert_eq!(
            actual.compare(&expected),
            Ordering::Equal,
            "{} extended",
            c.name
        );
        assert_eq!(
            actual.store_f32().unwrap(),
            row["output"]["height_bits"].as_u64().unwrap() as u32,
            "{} binary32 store",
            c.name
        );
    }
}

#[test]
fn invalid_native_storage_and_arithmetic_domains_are_explicit_errors() {
    let mut grid = Heightfield {
        width: 1,
        height: 1,
        coordinate_shift: 0,
        cell_size_bits: 1f32.to_bits(),
        index_bias: 0,
        height_bits: vec![2f32.to_bits()],
    };
    assert_eq!(
        grid.sample_nearest([0, 0]).unwrap().store_f32().unwrap(),
        2f32.to_bits()
    );
    assert!(grid.sample_nearest([0x7fc00000, 0]).is_err());
    assert!(grid.sample_nearest([0x4f000000, 0]).is_err());
    grid.cell_size_bits = 0;
    assert!(
        grid.sample_interpolated([0, 0], Precision::Double53)
            .is_err()
    );
    // Nearest sampling never reads the cell-size field.
    assert!(grid.sample_nearest([0, 0]).is_ok());
    grid.height_bits.clear();
    assert!(matches!(
        grid.sample_nearest([0, 0]),
        Err(Error::InvalidGrid)
    ));
}
