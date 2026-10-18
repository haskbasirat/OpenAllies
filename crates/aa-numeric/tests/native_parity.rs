use aa_numeric::{Binary, Error, Precision};
use std::cmp::Ordering;

fn value(v: &serde_json::Value) -> Binary {
    match v["kind"].as_str().unwrap() {
        "f32" => Binary::from_f32_bits(v["bits"].as_u64().unwrap() as u32).unwrap(),
        "f64" => Binary::from_f64_bits(v["bits"].as_u64().unwrap()).unwrap(),
        "i32" => Binary::from_i32(v["bits"].as_i64().unwrap() as i32),
        "u32" => Binary::from_u32(v["bits"].as_u64().unwrap() as u32),
        _ => panic!("unknown fixture kind"),
    }
}

#[test]
fn explicit_x87_precision_arithmetic_and_store_matches_native() {
    let report: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase4-body/numeric-native-oracle.json"
    ))
    .unwrap();
    assert_eq!(report["passed"], true);
    assert_eq!(report["cases"].as_array().unwrap().len(), 234);
    for row in report["cases"].as_array().unwrap() {
        let mode: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let a = value(&row["values"][0]);
        let b = value(&row["values"][1]);
        let result = match row["operation"].as_str().unwrap() {
            "add" => a.add(&b, mode),
            "sub" => a.sub(&b, mode),
            "mul" => a.mul(&b, mode),
            "div" => a.div(&b, mode),
            "reciprocal_mul" => a
                .div(&b, mode)
                .and_then(|v| v.mul(&value(&row["values"][2]), mode)),
            _ => panic!("unknown fixture operation"),
        }
        .unwrap()
        .store_f32();
        let expected = row["result"].as_u64().unwrap() as u32;
        if expected & 0x7f80_0000 == 0x7f80_0000 {
            assert_eq!(result, Err(Error::Binary32Overflow), "{row}");
        } else {
            assert_eq!(result.unwrap(), expected, "{row}");
        }
    }
}

#[test]
fn checked_domain_and_comparison_do_not_use_host_fpu() {
    assert!(matches!(
        Binary::from_f32_bits(0x7f80_0000),
        Err(Error::NonFiniteInput)
    ));
    assert!(matches!(
        Binary::from_f64_bits(0xfff8_0000_0000_0001),
        Err(Error::NonFiniteInput)
    ));
    assert!(matches!(
        Binary::from_u32(1).div(&Binary::from_u32(0), Precision::Extended64),
        Err(Error::DivisionByZero)
    ));
    assert_eq!(
        Binary::from_i32(-1).compare(&Binary::from_u32(0)),
        Ordering::Less
    );
    assert_eq!(
        Binary::from_f32_bits(0x8000_0000)
            .unwrap()
            .compare(&Binary::from_u32(0)),
        Ordering::Equal
    );
    let mut high = Binary::from_u32(2);
    for _ in 0..13 {
        high = high.mul(&high, Precision::Extended64).unwrap();
    }
    assert!(matches!(
        high.mul(&high, Precision::Extended64),
        Err(Error::ExtendedExponentRange)
    ));
    let tiny = Binary::from_u32(1)
        .div(&high, Precision::Extended64)
        .unwrap();
    assert!(matches!(
        tiny.mul(&tiny, Precision::Extended64),
        Err(Error::ExtendedExponentRange)
    ));
    assert_eq!(tiny.store_f32().unwrap(), 0);
}

#[test]
fn integer_conversion_truncates_without_binary32_rounding() {
    for (bits, expected) in [
        (0x0000_0000_0000_0000, 0),
        (0x8000_0000_0000_0000, 0),
        (0x3ffc_0000_0000_0000, 1),
        (0xbffc_0000_0000_0000, -1),
        (0x3fe0_0000_0000_0000, 0),
        (0xbfe0_0000_0000_0000, 0),
        (0x41df_ffff_ffe0_0000, i32::MAX),
        (0xc1e0_0000_0010_0000, i32::MIN),
    ] {
        assert_eq!(
            Binary::from_f64_bits(bits).unwrap().truncate_i32().unwrap(),
            expected
        );
    }
    assert_eq!(
        Binary::from_u32(0x8000_0000).truncate_i32(),
        Err(Error::IntegerRange)
    );
    assert_eq!(
        Binary::from_u32(0x8000_0001).negated().truncate_i32(),
        Err(Error::IntegerRange)
    );
}
