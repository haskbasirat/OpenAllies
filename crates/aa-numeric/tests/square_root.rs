use aa_numeric::{Binary, Error, Precision};
use serde::Deserialize;
use std::cmp::Ordering;

#[derive(Deserialize)]
struct Case {
    precision: Precision,
    kind: String,
    inputs: Vec<u64>,
    result80: String,
    result32: u32,
}
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}
fn fixture() -> Fixture {
    serde_json::from_str(include_str!(
        "../../../research/phase5-numeric/sqrt-vectors.json"
    ))
    .unwrap()
}
fn bytes80(text: &str) -> [u8; 10] {
    std::array::from_fn(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap())
}
fn expected_extended(raw: [u8; 10]) -> Binary {
    let significand = u64::from_le_bytes(raw[..8].try_into().unwrap());
    let field = u16::from_le_bytes(raw[8..].try_into().unwrap());
    if significand == 0 {
        return Binary::from_f32_bits(u32::from(field & 0x8000) << 16).unwrap();
    }
    let shift = i32::from(field & 0x7fff) - 16383 - 63;
    assert!((-1022..=1023).contains(&shift));
    let hi = Binary::from_u32((significand >> 32) as u32)
        .mul(
            &Binary::from_f64_bits(0x41f0_0000_0000_0000).unwrap(),
            Precision::Extended64,
        )
        .unwrap();
    let sig = hi
        .add(&Binary::from_u32(significand as u32), Precision::Extended64)
        .unwrap();
    let scaled = sig
        .mul(
            &Binary::from_f64_bits(((shift + 1023) as u64) << 52).unwrap(),
            Precision::Extended64,
        )
        .unwrap();
    if field & 0x8000 != 0 {
        scaled.negated()
    } else {
        scaled
    }
}
fn input(case: &Case) -> Binary {
    match case.kind.as_str() {
        "f32" => Binary::from_f32_bits(case.inputs[0] as u32).unwrap(),
        "f64" => Binary::from_f64_bits(case.inputs[0]).unwrap(),
        "sum_squares_f32" => {
            let x = Binary::from_f32_bits(case.inputs[0] as u32).unwrap();
            let y = Binary::from_f32_bits(case.inputs[1] as u32).unwrap();
            x.mul(&x, case.precision)
                .unwrap()
                .add(&y.mul(&y, case.precision).unwrap(), case.precision)
                .unwrap()
        }
        _ => panic!("unknown arithmetic fixture"),
    }
}

#[test]
fn square_root_matches_extended_x87_values_before_binary32_store() {
    let rows = fixture().cases;
    assert_eq!(rows.len(), 3267);
    for row in rows {
        let actual = input(&row).sqrt(row.precision).unwrap();
        assert_eq!(
            actual.compare(&expected_extended(bytes80(&row.result80))),
            Ordering::Equal,
            "{} {:?} {:?}",
            row.kind,
            row.inputs,
            row.precision
        );
        if row.result32 & 0x7fff_ffff == 0x7f80_0000 {
            assert_eq!(actual.store_f32(), Err(Error::Binary32Overflow));
        } else {
            assert_eq!(actual.store_f32().unwrap(), row.result32);
        }
    }
}

#[test]
fn square_root_handles_signed_zero_negative_domain_and_extended_exponents() {
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        assert_eq!(
            Binary::from_f32_bits(0x8000_0000)
                .unwrap()
                .sqrt(pc)
                .unwrap()
                .store_f32()
                .unwrap(),
            0x8000_0000
        );
        assert!(matches!(
            Binary::from_i32(-1).sqrt(pc),
            Err(Error::NegativeSquareRoot)
        ));
        let mut value = Binary::from_u32(2);
        for _ in 0..12 {
            value = value.mul(&value, pc).unwrap();
        }
        let square = value.mul(&value, pc).unwrap();
        assert_eq!(square.sqrt(pc).unwrap().compare(&value), Ordering::Equal);
        let small = Binary::from_u32(1).div(&square, pc).unwrap();
        assert_eq!(
            small
                .sqrt(pc)
                .unwrap()
                .compare(&Binary::from_u32(1).div(&value, pc).unwrap()),
            Ordering::Equal
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[test]
fn oracle_sqrt_results_match_actual_host_x87() {
    #[repr(C, align(16))]
    struct State([u8; 512]);
    #[repr(C)]
    struct Work {
        x: u64,
        y: u64,
        result: [u8; 10],
        control: u16,
    }
    for row in fixture().cases {
        let mut saved = State([0; 512]);
        let mut work = Work {
            x: row.inputs[0],
            y: *row.inputs.get(1).unwrap_or(&0),
            result: [0; 10],
            control: match row.precision {
                Precision::Single24 => 0x7f,
                Precision::Double53 => 0x27f,
                Precision::Extended64 => 0x37f,
            },
        };
        let kind = match row.kind.as_str() {
            "f32" => 0u32,
            "f64" => 1,
            _ => 2,
        };
        // SAFETY: x86_64 supplies FXSAVE/FXRSTOR. Every memory access addresses
        // a fixed local field; saved is512 bytes and16-byte aligned. All paths
        // restore the caller's complete x87/SSE state, with no calls/SP changes.
        unsafe {
            core::arch::asm!(
            "fxsave64 [{saved}]", "fninit", "fldcw word ptr [{w}+{control}]",
            "cmp {kind:e}, 1", "je 20f", "fld dword ptr [{w}+{x}]",
            "cmp {kind:e}, 2", "jne 21f", "fmul dword ptr [{w}+{x}]",
            "fld dword ptr [{w}+{y}]", "fmul dword ptr [{w}+{y}]", "faddp st(1), st(0)", "jmp 21f",
            "20:", "fld qword ptr [{w}+{x}]", "21:", "fsqrt",
            "fstp tbyte ptr [{w}+{result}]", "fxrstor64 [{saved}]",
            saved=in(reg) saved.0.as_mut_ptr(),w=in(reg) &mut work,kind=in(reg) kind,
            x=const core::mem::offset_of!(Work,x),y=const core::mem::offset_of!(Work,y),
            control=const core::mem::offset_of!(Work,control),result=const core::mem::offset_of!(Work,result),
            options(nostack));
        }
        assert_eq!(
            work.result,
            bytes80(&row.result80),
            "{} {:?} {:?}",
            row.kind,
            row.inputs,
            row.precision
        );
    }
}
