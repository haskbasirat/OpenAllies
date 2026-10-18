use aa_numeric::{Binary, Error, Precision};
use serde::Deserialize;
use std::cmp::Ordering;

#[derive(Deserialize)]
struct Row {
    precision: Precision,
    kind: String,
    inputs: [u64; 2],
    result80: String,
    result32: u32,
}
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Row>,
}
fn fixture() -> Fixture {
    serde_json::from_str(include_str!(
        "../../../research/phase5-numeric/remainder-hardware-vectors.json"
    ))
    .unwrap()
}
fn raw80(text: &str) -> [u8; 10] {
    std::array::from_fn(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).unwrap())
}
fn expected(raw: [u8; 10]) -> Binary {
    let sig = u64::from_le_bytes(raw[..8].try_into().unwrap());
    let field = u16::from_le_bytes(raw[8..].try_into().unwrap());
    if sig == 0 {
        return Binary::from_f32_bits(u32::from(field & 0x8000) << 16).unwrap();
    }
    let pc = Precision::Extended64;
    let mut value = Binary::from_u32((sig >> 32) as u32)
        .mul(&Binary::from_f64_bits(0x41f0_0000_0000_0000).unwrap(), pc)
        .unwrap()
        .add(&Binary::from_u32(sig as u32), pc)
        .unwrap();
    let mut shift = i32::from(field & 0x7fff) - 16383 - 63;
    while shift != 0 {
        let step = shift.clamp(-1022, 1023);
        value = value
            .mul(
                &Binary::from_f64_bits(((step + 1023) as u64) << 52).unwrap(),
                pc,
            )
            .unwrap();
        shift -= step;
    }
    if field & 0x8000 != 0 {
        value.negated()
    } else {
        value
    }
}
fn input(row: &Row, n: usize) -> Binary {
    if row.kind == "f32" {
        Binary::from_f32_bits(row.inputs[n] as u32).unwrap()
    } else {
        Binary::from_f64_bits(row.inputs[n]).unwrap()
    }
}

#[test]
fn exact_remainder_matches_full_x87_loop_before_binary32_store() {
    let rows = fixture().cases;
    assert_eq!(rows.len(), 3696);
    for row in rows {
        let value = input(&row, 0).remainder_trunc(&input(&row, 1)).unwrap();
        assert_eq!(
            value.compare(&expected(raw80(&row.result80))),
            Ordering::Equal,
            "{} {:?} {:?}",
            row.kind,
            row.inputs,
            row.precision
        );
        if row.result32 & 0x7fff_ffff == 0x7f80_0000 {
            assert_eq!(value.store_f32(), Err(Error::Binary32Overflow));
        } else {
            assert_eq!(value.store_f32().unwrap(), row.result32);
        }
    }
}

#[test]
fn zero_divisor_is_rejected_without_losing_dividend_zero_sign() {
    for bits in [0, 0x8000_0000] {
        let zero = Binary::from_f32_bits(bits).unwrap();
        assert_eq!(
            zero.remainder_trunc(&Binary::from_u32(3))
                .unwrap()
                .store_f32()
                .unwrap(),
            bits
        );
        assert!(matches!(
            Binary::from_u32(1).remainder_trunc(&zero),
            Err(Error::DivisionByZero)
        ));
    }
}

#[cfg(target_arch = "x86_64")]
#[test]
fn remainder_oracle_matches_actual_host_fpu_at_each_precision() {
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
            y: row.inputs[1],
            result: [0; 10],
            control: match row.precision {
                Precision::Single24 => 0x7f,
                Precision::Double53 => 0x27f,
                Precision::Extended64 => 0x37f,
            },
        };
        let wide = u32::from(row.kind == "f64");
        // SAFETY: fixed local fields and16-byte aligned512-byte saved state.
        // Finite operands/nonzero divisor make the native FPREM loop terminate.
        // Every path restores x87/SSE state; there are no calls or SP changes.
        unsafe {
            core::arch::asm!(
            "fxsave64 [{saved}]", "fninit", "fldcw word ptr [{w}+{control}]",
            "test {wide:e},{wide:e}", "jnz 20f",
            "fld dword ptr [{w}+{y}]", "fld dword ptr [{w}+{x}]", "jmp 21f",
            "20:", "fld qword ptr [{w}+{y}]", "fld qword ptr [{w}+{x}]",
            "21:", "fprem", "fnstsw ax", "test ah,4", "jnz 21b",
            "fstp tbyte ptr [{w}+{result}]", "fstp st(0)", "fxrstor64 [{saved}]",
            saved=in(reg) saved.0.as_mut_ptr(),w=in(reg) &mut work,wide=in(reg) wide,
            x=const core::mem::offset_of!(Work,x),y=const core::mem::offset_of!(Work,y),
            result=const core::mem::offset_of!(Work,result),control=const core::mem::offset_of!(Work,control),
            out("eax") _,options(nostack));
        }
        assert_eq!(
            work.result,
            raw80(&row.result80),
            "{} {:?} {:?}",
            row.kind,
            row.inputs,
            row.precision
        );
    }
}
