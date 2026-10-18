//! Finite x87 compatibility for original445400 ->96D780 ->97792B.
//! Inputs/results are bits; no host asin/sin/cos approximation is used.
//! The selected PC and incoming sticky precision flag are explicit inputs.
//! Caller x87/SSE state is saved/restored, including existing exception flags.

use crate::Error;
use aa_numeric::Precision;

#[cfg(target_arch = "x86_64")]
#[repr(C, align(16))]
struct FxState([u8; 512]);

#[cfg(target_arch = "x86_64")]
#[repr(C)]
struct Work {
    from: [u32; 2],
    to: [u32; 2],
    cross: u32,
    dot_negative: u32,
    result: u32,
    pending: u32,
    rounded_asin: u64,
    control: u16,
    status: u16,
    zero: u32,
    one: u32,
    minus_one: u32,
    three: u32,
    pi: u32,
    minus_pi: u32,
    half_pi80: [u8; 10],
    /// Nonzero: store ST0 with FSTP TBYTE into `result80` instead of the
    /// caller-side binary32 FSTP DWORD into `result`.
    wide: u32,
    result80: [u8; 10],
    invalid: u32,
}

/// 0x445400 followed by the caller's binary32 store of ST0.
pub fn signed_angle(
    from: [u32; 2],
    to: [u32; 2],
    precision: Precision,
    precision_exception_pending: bool,
) -> Result<u32, Error> {
    run(from, to, precision, precision_exception_pending, false).map(|(stored, _)| stored)
}

/// 0x445400 exactly as it returns: the unstored x87 ST0 value as an 80-bit
/// register image (sign bit 79, biased exponent 64..78, explicit significand
/// 0..63), e.g. for callers that keep computing in the register (RoutWorld
/// angle in 52A930). Same PC/sticky-PE inputs as [`signed_angle`].
pub fn signed_angle_extended(
    from: [u32; 2],
    to: [u32; 2],
    precision: Precision,
    precision_exception_pending: bool,
) -> Result<u128, Error> {
    run(from, to, precision, precision_exception_pending, true).map(|(_, wide)| wide)
}

fn run(
    from: [u32; 2],
    to: [u32; 2],
    precision: Precision,
    precision_exception_pending: bool,
    wide: bool,
) -> Result<(u32, u128), Error> {
    if from
        .into_iter()
        .chain(to)
        .any(|v| v & 0x7f80_0000 == 0x7f80_0000)
    {
        return Err(Error::NonFiniteInput);
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (precision, precision_exception_pending, wide);
        Err(Error::UnsupportedPlatform)
    }
    #[cfg(target_arch = "x86_64")]
    {
        let mut saved = FxState([0; 512]);
        let mut work = Work {
            from,
            to,
            cross: 0,
            dot_negative: 0,
            result: 0,
            pending: u32::from(precision_exception_pending),
            rounded_asin: 0,
            control: match precision {
                Precision::Single24 => 0x007f,
                Precision::Double53 => 0x027f,
                Precision::Extended64 => 0x037f,
            },
            status: 0,
            zero: 0,
            one: 0x3f80_0000,
            minus_one: 0xbf80_0000,
            three: 0x4040_0000,
            pi: 0x4049_0fdb,
            minus_pi: 0xc049_0fdb,
            // Exact original80-bit constant at0xB359BA, not host PI/2.
            half_pi80: [0x35, 0xc2, 0x68, 0x21, 0xa2, 0xda, 0x0f, 0xc9, 0xff, 0x3f],
            wide: u32::from(wide),
            result80: [0; 10],
            invalid: 0,
        };
        // SAFETY: x86_64 guarantees FXSAVE/FXRSTOR support. `saved` is512 bytes,
        //16-byte aligned; all other memory operands refer to live Work fields.
        // No call, stack-pointer change, unbounded memory access or branch can
        // bypass FXRSTOR. FNINIT/masked RN control isolate caller exceptions.
        // FXSAVE64 itself does not wait on a pending unmasked caller exception.
        // General flags/AX are declared clobbered; x87/SSE state is restored.
        unsafe {
            core::arch::asm!(
                "fxsave64 [{saved}]", "fninit", "fldcw word ptr [{w}+{control}]",
                "cmp dword ptr [{w}+{pending}], 0", "je 20f",
                "fld1", "fdiv dword ptr [{w}+{three}]", "fstp st(0)",
                "20:",
                // Original cross, with its mandatory binary32 store.
                "fld dword ptr [{w}+{from}]", "fmul dword ptr [{w}+{to}+4]",
                "fld dword ptr [{w}+{from}+4]", "fmul dword ptr [{w}+{to}]",
                "fsubp st(1), st(0)", "fstp dword ptr [{w}+{cross}]",
                // Original dot sign is tested before it is stored to binary32.
                "fld dword ptr [{w}+{from}+4]", "fmul dword ptr [{w}+{to}+4]",
                "fld dword ptr [{w}+{from}]", "fmul dword ptr [{w}+{to}]",
                "faddp st(1), st(0)", "fcomp dword ptr [{w}+{zero}]",
                "fnstsw ax", "test ah, 5", "jp 21f",
                "mov dword ptr [{w}+{negative}], 1", "21:",
                // Original [-1,1] clamp (does not normalize either vector).
                "fld dword ptr [{w}+{cross}]", "fcomp dword ptr [{w}+{minus_one}]",
                "fnstsw ax", "test ah, 5", "jp 22f",
                "mov dword ptr [{w}+{cross}], 0xbf800000", "jmp 23f",
                "22:", "fld dword ptr [{w}+{cross}]", "fcomp dword ptr [{w}+{one}]",
                "fnstsw ax", "test ah, 0x41", "jne 23f",
                "mov dword ptr [{w}+{cross}], 0x3f800000", "23:",
                "mov eax, dword ptr [{w}+{cross}]", "and eax, 0x7fffffff",
                "cmp eax, 0x7f800000", "jb 24f",
                // Finite inputs can overflow/cancel into NaN. That unsupported
                // intermediate returns a typed error, after restoring state.
                "mov dword ptr [{w}+{result}], 0x7fc00000",
                "mov dword ptr [{w}+{invalid}], 1", "jmp 29f",
                "24:", "fld dword ptr [{w}+{cross}]",
                "fst qword ptr [{w}+{rounded}]",
                "cmp eax, 0x3f800000", "je 25f",
                // Native CRT asin finite interior: (1+x)*(1-x),sqrt,atan.
                "fld1", "fadd st(0), st(1)", "fld1", "fsub st(0), st(2)",
                "fmulp st(1), st(0)", "fsqrt", "fpatan", "jmp 26f",
                "25:", "fstp st(0)", "fld tbyte ptr [{w}+{half_pi}]",
                "test dword ptr [{w}+{cross}], 0x80000000", "jz 26f", "fchs",
                "26:",
                // Original97792B takes a qword round-trip iff PC!=53 and PE.
                "cmp word ptr [{w}+{control}], 0x027f", "je 27f",
                "fnstsw ax", "test ax, 0x20", "jz 27f",
                "fstp qword ptr [{w}+{rounded}]", "fld qword ptr [{w}+{rounded}]",
                "27:", "cmp dword ptr [{w}+{negative}], 0", "je 28f",
                "fcom dword ptr [{w}+{zero}]", "fnstsw ax", "test ah, 1", "jnz 30f",
                "fsubr dword ptr [{w}+{pi}]", "jmp 28f",
                "30:", "fsubr dword ptr [{w}+{minus_pi}]",
                "28:", "cmp dword ptr [{w}+{wide}], 0", "je 31f",
                "fstp tbyte ptr [{w}+{result80}]", "jmp 32f",
                "31:", "fstp dword ptr [{w}+{result}]", "32:",
                "fnstsw word ptr [{w}+{status}]",
                "29:", "fxrstor64 [{saved}]",
                saved=in(reg) saved.0.as_mut_ptr(), w=in(reg) &mut work,
                from=const core::mem::offset_of!(Work,from), to=const core::mem::offset_of!(Work,to),
                cross=const core::mem::offset_of!(Work,cross), negative=const core::mem::offset_of!(Work,dot_negative),
                result=const core::mem::offset_of!(Work,result), pending=const core::mem::offset_of!(Work,pending),
                rounded=const core::mem::offset_of!(Work,rounded_asin), control=const core::mem::offset_of!(Work,control),
                status=const core::mem::offset_of!(Work,status), zero=const core::mem::offset_of!(Work,zero),
                one=const core::mem::offset_of!(Work,one), minus_one=const core::mem::offset_of!(Work,minus_one),
                three=const core::mem::offset_of!(Work,three), pi=const core::mem::offset_of!(Work,pi),
                minus_pi=const core::mem::offset_of!(Work,minus_pi), half_pi=const core::mem::offset_of!(Work,half_pi80),
                wide=const core::mem::offset_of!(Work,wide), result80=const core::mem::offset_of!(Work,result80),
                invalid=const core::mem::offset_of!(Work,invalid),
                out("eax") _, options(nostack),
            );
        }
        let mut image = [0u8; 16];
        image[..10].copy_from_slice(&work.result80);
        let wide_bits = u128::from_le_bytes(image);
        let non_finite = if wide {
            work.invalid != 0 || (wide_bits >> 64) & 0x7fff == 0x7fff
        } else {
            work.result & 0x7f80_0000 == 0x7f80_0000
        };
        if non_finite {
            Err(Error::NonFiniteResult)
        } else {
            Ok((work.result, wide_bits))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_clean_angle_fixtures_match_host_x87() {
        let report: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/phase4-trig/native-trig-vectors.json"
        ))
        .unwrap();
        let rows = report["angles"].as_array().unwrap();
        assert_eq!(rows.len(), 168);
        for row in rows {
            let from = serde_json::from_value(row["from_bits"].clone()).unwrap();
            let to = serde_json::from_value(row["to_bits"].clone()).unwrap();
            let pc = serde_json::from_value(row["precision"].clone()).unwrap();
            assert_eq!(
                signed_angle(from, to, pc, false).unwrap(),
                row["result_bits"].as_u64().unwrap() as u32,
                "{row}"
            );
        }
    }

    #[test]
    fn original_sticky_precision_angle_fixtures_match_host_x87() {
        let report: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/phase4-trig-angle/native-angle-status-vectors.json"
        ))
        .unwrap();
        let rows = report["cases"].as_array().unwrap();
        assert_eq!(rows.len(), 1608);
        for row in rows {
            let from = serde_json::from_value(row["from_bits"].clone()).unwrap();
            let to = serde_json::from_value(row["to_bits"].clone()).unwrap();
            let pc = serde_json::from_value(row["precision"].clone()).unwrap();
            let pending = row["precision_exception_pending"].as_bool().unwrap();
            assert_eq!(
                signed_angle(from, to, pc, pending).unwrap(),
                row["result_bits"].as_u64().unwrap() as u32,
                "{row}"
            );
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn host_sincos(value: u32, cosine: bool, control: u16) -> u32 {
        let mut saved = FxState([0; 512]);
        let mut out = 0u32;
        // SAFETY: private fixed-size buffers and all paths restore FP state.
        unsafe {
            core::arch::asm!(
            "fxsave64 [{saved}]", "fninit", "fldcw word ptr [{control}]",
            "fld dword ptr [{value}]", "test {cosine:e}, {cosine:e}", "jz 20f",
            "fcos", "jmp 21f", "20:", "fsin", "21:",
            "fstp dword ptr [{out}]", "fxrstor64 [{saved}]",
            saved=in(reg) saved.0.as_mut_ptr(),control=in(reg) &control,value=in(reg) &value,
            cosine=in(reg) u32::from(cosine),out=in(reg) &mut out,options(nostack));
        }
        out
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn original4096_tables_match_host_fsin_fcos() {
        let report: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/phase4-trig/native-trig-vectors.json"
        ))
        .unwrap();
        let tau = aa_numeric::Binary::from_f32_bits(0x40c90fdb).unwrap();
        let count = aa_numeric::Binary::from_u32(4096);
        for (mode, cw) in [
            (Precision::Single24, 0x7f),
            (Precision::Double53, 0x27f),
            (Precision::Extended64, 0x37f),
        ] {
            for index in 0..4096 {
                let input = aa_numeric::Binary::from_u32(index)
                    .mul(&tau, mode)
                    .unwrap()
                    .div(&count, mode)
                    .unwrap()
                    .store_f32()
                    .unwrap();
                for (name, cosine) in [("cos", true), ("sin", false)] {
                    assert_eq!(
                        host_sincos(input, cosine, cw),
                        report["tables"][name][index as usize].as_u64().unwrap() as u32,
                        "{name} index{index} mode{mode:?}"
                    );
                }
            }
        }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn caller_precision_status_stack_and_mxcsr_are_preserved() {
        let mut original = FxState([0; 512]);
        let mut before = FxState([0; 512]);
        let mut after = FxState([0; 512]);
        // A different caller rounding mode, two live x87 stack values, sticky
        // PE, and a nondefault MXCSR detect accidental environment leakage.
        let control = 0x0b7fu16;
        let mxcsr = 0x9fc0u32;
        let values = [0x4050_0000u32, 0xc060_0000, 0x4040_0000];
        // SAFETY: scoped test thread state; aligned512byte buffers. Original is
        // restored before assertions, regardless of the returned Result.
        unsafe {
            core::arch::asm!(
            "fxsave64 [{original}]", "fninit", "fldcw word ptr [{control}]",
            "ldmxcsr dword ptr [{mxcsr}]", "fld dword ptr [{values}]",
            "fld dword ptr [{values}+4]", "fld1", "fdiv dword ptr [{values}+8]",
            "fstp st(0)", "fxsave64 [{before}]",
            original=in(reg) original.0.as_mut_ptr(),control=in(reg) &control,mxcsr=in(reg) &mxcsr,
            values=in(reg) values.as_ptr(),before=in(reg) before.0.as_mut_ptr(),options(nostack));
        }
        let result = signed_angle(
            [0x3f80_0000, 0],
            [0, 0x3f80_0000],
            Precision::Extended64,
            false,
        );
        unsafe {
            core::arch::asm!("fxsave64 [{after}]", "fxrstor64 [{original}]",
            after=in(reg) after.0.as_mut_ptr(), original=in(reg) original.0.as_mut_ptr(),options(nostack));
        }
        assert!(result.is_ok());
        assert_eq!(
            &before.0[..5],
            &after.0[..5],
            "CW/status/abridged tags changed"
        );
        assert_eq!(&before.0[24..28], &after.0[24..28], "MXCSR changed");
        for slot in 0..8 {
            let start = 32 + slot * 16;
            assert_eq!(
                &before.0[start..start + 10],
                &after.0[start..start + 10],
                "x87register{slot} changed"
            );
        }
    }

    #[test]
    fn nonfinite_inputs_are_explicit_errors() {
        for bits in [0x7f80_0000, 0xff80_0000, 0x7fc0_0001, 0x7f80_0001] {
            assert!(matches!(
                signed_angle([bits, 0], [0, 0], Precision::Extended64, false),
                Err(Error::NonFiniteInput)
            ));
            assert!(matches!(
                signed_angle([0, 0], [bits, 0], Precision::Extended64, true),
                Err(Error::NonFiniteInput)
            ));
        }
    }
}
