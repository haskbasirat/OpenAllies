//! signed_angle_extended against the unstored 0x445400 ST0 results captured
//! under Unicorn: research/phase8-tick/native-angle-extended.json (own cases)
//! and the 522 calls captured inside 52A930 by research/phase8-ai.
//!
//! Emulator limits (verified in research/phase8-tick/README.txt): Unicorn
//! 2.1.4 computes FPATAN through binary64 and never raises the x87 precision
//! flag from arithmetic. The Rust variant executes the real instructions, so
//! every row must agree within one binary64 ulp and store the identical
//! binary32; bit-identical 80-bit rows are counted, not required.
use aa_numeric::Precision;
use aa_trig::{signed_angle, signed_angle_extended};
use serde_json::Value;

fn pc(v: &Value) -> Precision {
    match v {
        Value::Number(n) => match n.as_u64().unwrap() {
            24 => Precision::Single24,
            53 => Precision::Double53,
            64 => Precision::Extended64,
            _ => panic!(),
        },
        other => serde_json::from_value(other.clone()).unwrap(),
    }
}
fn pair(v: &Value) -> [u32; 2] {
    [v[0].as_u64().unwrap() as u32, v[1].as_u64().unwrap() as u32]
}
fn round53(bits: u128) -> u128 {
    aa_numeric::Binary::from_x87_extended_bits(bits)
        .unwrap()
        .add(&aa_numeric::Binary::from_i32(0), Precision::Double53)
        .unwrap()
        .to_x87_extended_bits()
        .unwrap()
}
/// |a - b| <= one binary64 ulp of a (both already rounded to binary64).
fn within_one_ulp53(a: u128, b: u128) -> bool {
    use aa_numeric::Binary;
    if a == b {
        return true;
    }
    let (x, y) = (
        Binary::from_x87_extended_bits(a).unwrap(),
        Binary::from_x87_extended_bits(b).unwrap(),
    );
    let d = x.sub(&y, Precision::Extended64).unwrap();
    let d = if d.is_negative_nonzero() {
        d.negated()
    } else {
        d
    };
    let field = ((a >> 64) & 0x7fff).max((b >> 64) & 0x7fff);
    assert!(
        field > 52,
        "nonzero angle magnitudes are far above binary64 denormals"
    );
    let ulp = Binary::from_x87_extended_bits(((field - 52) << 64) | (1u128 << 63)).unwrap();
    !d.compare(&ulp).is_gt()
}
fn store32(bits: u128) -> u32 {
    aa_numeric::Binary::from_x87_extended_bits(bits)
        .unwrap()
        .store_f32()
        .unwrap()
}
/// Unicorn's FPATAN is only binary64-accurate, so the 80-bit images are
/// compared to within one binary64 ulp, and the caller-visible binary32
/// store must be identical.
fn check(wide: u128, native: u128, _p: Precision, _pe: bool, what: &str) -> bool {
    assert!(
        within_one_ulp53(round53(wide), round53(native)),
        "binary64 {what}"
    );
    assert_eq!(store32(wide), store32(native), "binary32 {what}");
    wide == native
}
fn f80(hex: &str) -> u128 {
    let mut bytes = [0u8; 16];
    for i in 0..10 {
        bytes[i] = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap();
    }
    u128::from_le_bytes(bytes)
}

#[test]
#[cfg(target_arch = "x86_64")]
fn unstored_angle_matches_native_unicorn_cases() {
    let report: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-tick/native-angle-extended.json"
    ))
    .unwrap();
    let rows = report["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 2400);
    let mut exact = 0;
    for row in rows {
        let (a, b, p) = (
            pair(&row["from_bits"]),
            pair(&row["to_bits"]),
            pc(&row["precision"]),
        );
        let pe = row["precision_exception_pending"].as_bool().unwrap();
        let wide = signed_angle_extended(a, b, p, pe).unwrap();
        if check(
            wide,
            f80(row["st0_f80"].as_str().unwrap()),
            p,
            pe,
            &row.to_string(),
        ) {
            exact += 1;
        }
        // The stored variant is the binary32 rounding of the same register.
        let stored = aa_numeric::Binary::from_x87_extended_bits(wide)
            .unwrap()
            .store_f32()
            .unwrap();
        assert_eq!(signed_angle(a, b, p, pe).unwrap(), stored, "{row}");
    }
    println!("own cases: {exact} of 2400 bit-identical in all 80 bits");
}

#[test]
#[cfg(target_arch = "x86_64")]
fn unstored_angle_matches_calls_captured_in_rout_score() {
    let report: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-ai/native-rout-score.json"
    ))
    .unwrap();
    let (mut calls, mut exact) = (0, 0);
    for case in report["cases"].as_array().unwrap() {
        let p = pc(&case["input"]["precision"]);
        for e in case["output"]["events"].as_array().unwrap() {
            if e["event"] != "angle" {
                continue;
            }
            let expected = (u128::from(e["extended_sign_exponent"].as_u64().unwrap()) << 64)
                | u128::from(e["extended_significand"].as_u64().unwrap());
            let got = signed_angle_extended(
                pair(&e["from"]),
                pair(&e["to"]),
                p,
                e["pe"].as_bool().unwrap(),
            )
            .unwrap();
            if check(got, expected, p, e["pe"].as_bool().unwrap(), &e.to_string()) {
                exact += 1;
            }
            calls += 1;
        }
    }
    println!("captured calls: {exact} of {calls} bit-identical in all 80 bits");
    assert_eq!(calls, 522);
}
