//! Finite x87 leaves used by original52FD90 waypoint turn offsets.
//! The tangent input is bounded by the original signed-angle result. There is
//! no range reduction, host libm approximation, or extreme-vector fallback.
use crate::{Error, signed_angle};
use aa_numeric::{Binary, Precision};

#[cfg(target_arch = "x86_64")]
#[repr(C, align(16))]
struct FxState([u8; 512]);

fn finite(values: impl IntoIterator<Item = u32>) -> Result<(), Error> {
    if values.into_iter().any(|x| x & 0x7f80_0000 == 0x7f80_0000) {
        Err(Error::NonFiniteInput)
    } else {
        Ok(())
    }
}
#[cfg(target_arch = "x86_64")]
fn control(p: Precision) -> u16 {
    match p {
        Precision::Single24 => 0x007f,
        Precision::Double53 => 0x027f,
        Precision::Extended64 => 0x037f,
    }
}

///51D510: strict Manhattan distance < sourceA32B58. The second return preserves
/// the newly raised precision flag for the following native CRT angle call.
pub fn near_waypoint(
    from: [u32; 2],
    to: [u32; 2],
    precision: Precision,
) -> Result<(bool, bool), Error> {
    finite(from.into_iter().chain(to))?;
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = precision;
        Err(Error::UnsupportedPlatform)
    }
    #[cfg(target_arch = "x86_64")]
    {
        let mut saved = FxState([0; 512]);
        let cw = control(precision);
        let threshold = 0x3951_b717u32;
        let mut status: u16;
        let mut result: u32;
        // SAFETY: bounded live inputs, aligned FXSAVE buffer, no calls or exit
        // path between save/restore. All x87/SSE state is restored on return.
        unsafe {
            core::arch::asm!(
                "fxsave64 [{saved}]", "fninit", "fldcw word ptr [{cw}]",
                "fld dword ptr [{to}]", "fsub dword ptr [{from}]", "fabs",
                "fld dword ptr [{to}+4]", "fsub dword ptr [{from}+4]", "fabs",
                "fadd st(0), st(1)", "fcomp dword ptr [{threshold}]",
                "fnstsw ax", "fstp st(0)", "mov {status:x}, ax",
                "xor {result:e}, {result:e}", "test ah, 5", "setnp {result:l}",
                "fxrstor64 [{saved}]",
                saved=in(reg) saved.0.as_mut_ptr(), cw=in(reg) &cw,
                from=in(reg) from.as_ptr(), to=in(reg) to.as_ptr(),
                threshold=in(reg) &threshold, status=out(reg) status,
                result=out(reg) result, out("eax") _, options(nostack),
            );
        }
        Ok((result != 0, status & 0x20 != 0))
    }
}

///445400 ->43AA00 ->FPTAN from52FD90, including the stored angle, unstored
/// half-angle, and final radius multiplication. Incoming PE is explicit.
pub fn waypoint_turn_offset(
    radius_bits: u32,
    from: [u32; 2],
    to: [u32; 2],
    precision: Precision,
    precision_exception_pending: bool,
) -> Result<u32, Error> {
    finite([radius_bits])?;
    let angle = signed_angle(from, to, precision, precision_exception_pending)?;
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = angle;
        Err(Error::UnsupportedPlatform)
    }
    #[cfg(target_arch = "x86_64")]
    {
        let mut saved = FxState([0; 512]);
        let cw = control(precision);
        let half = 0x3f00_0000u32;
        let mut result = 0u32;
        // SAFETY: same isolated bounded x87 state contract as near_waypoint.
        // Signed angle is finite and within source PI; FPTAN always reduces
        // successfully here. The comparison/FCHS preserves source negative0.
        unsafe {
            core::arch::asm!(
                "fxsave64 [{saved}]", "fninit", "fldcw word ptr [{cw}]",
                "fld dword ptr [{angle}]", "ftst", "fnstsw ax", "test ah, 1",
                "jz 20f", "fchs", "20:",
                "fmul dword ptr [{half}]", "fptan", "fstp st(0)",
                "fmul dword ptr [{radius}]", "fstp dword ptr [{result}]",
                "fxrstor64 [{saved}]",
                saved=in(reg) saved.0.as_mut_ptr(),cw=in(reg) &cw,angle=in(reg) &angle,
                half=in(reg) &half,radius=in(reg) &radius_bits,result=in(reg) &mut result,
                out("eax") _,options(nostack),
            );
        }
        if result & 0x7f80_0000 == 0x7f80_0000 {
            Err(Error::NonFiniteResult)
        } else {
            Ok(result)
        }
    }
}

/// Exact finite FSIN/FCOS results, including their64-bit significands. Used by
///532B50 before native asymmetric binary32 stores. Values outside the native
///small-angle domain are rejected instead of relying on approximate reduction.
pub fn small_sin_cos(angle: u32, precision: Precision) -> Result<(Binary, Binary), Error> {
    finite([angle])?;
    if angle & 0x7fff_ffff > 0x4049_0fdb {
        return Err(Error::NonFiniteResult);
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = precision;
        Err(Error::UnsupportedPlatform)
    }
    #[cfg(target_arch = "x86_64")]
    {
        let mut saved = FxState([0; 512]);
        let cw = control(precision);
        let mut sine = [0u8; 10];
        let mut cosine = [0u8; 10];
        // SAFETY: isolated x87 instructions on bounded small finite input.
        unsafe {
            core::arch::asm!(
            "fxsave64 [{saved}]","fninit","fldcw word ptr [{cw}]",
            "fld dword ptr [{angle}]","fsin","fstp tbyte ptr [{sine}]",
            "fld dword ptr [{angle}]","fcos","fstp tbyte ptr [{cosine}]",
            "fxrstor64 [{saved}]",saved=in(reg)saved.0.as_mut_ptr(),cw=in(reg)&cw,
            angle=in(reg)&angle,sine=in(reg)sine.as_mut_ptr(),cosine=in(reg)cosine.as_mut_ptr(),options(nostack));
        }
        Ok((binary80(sine)?, binary80(cosine)?))
    }
}
#[cfg(target_arch = "x86_64")]
fn binary80(raw: [u8; 10]) -> Result<Binary, Error> {
    let sign_exp = u16::from_le_bytes([raw[8], raw[9]]);
    let sig = u64::from_le_bytes(raw[..8].try_into().unwrap());
    if sign_exp & 0x7fff == 0x7fff {
        return Err(Error::NonFiniteResult);
    }
    if sig == 0 {
        return Ok(Binary::from_f32_bits(u32::from(sign_exp >> 15) << 31)?);
    }
    let exponent = i32::from(sign_exp & 0x7fff) - 16383 - 63;
    if !(-1022..=1023).contains(&exponent) {
        return Err(Error::NonFiniteResult);
    }
    let p = Precision::Extended64;
    let value = Binary::from_u32((sig >> 32) as u32)
        .mul(&Binary::from_f64_bits(0x41f0000000000000)?, p)?
        .add(&Binary::from_u32(sig as u32), p)?
        .mul(&Binary::from_f64_bits(((exponent + 1023) as u64) << 52)?, p)?;
    Ok(if sign_exp & 0x8000 != 0 {
        value.negated()
    } else {
        value
    })
}

///532CAA..532D16 turn-angle selection and finite96DBB0 acos. The CRT receives
/// an extended argument and stores it through binary64 for classification.
pub fn recovery_angle(
    radius: u32,
    squared: u32,
    precision: Precision,
    pending: bool,
) -> Result<u32, Error> {
    finite([radius, squared])?;
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (precision, pending);
        Err(Error::UnsupportedPlatform)
    }
    #[cfg(target_arch = "x86_64")]
    {
        let mut saved = FxState([0; 512]);
        let cw = control(precision);
        let mut result = 0x3f490fdbu32;
        let mut temp = 0u32;
        let mut wide = 0u64;
        let source_angle = 0x3fe921fb60000000u64;
        let one = 0x3f800000u32;
        let three = 0x40400000u32;
        // SAFETY: fixed buffers, no external calls, every path restores state.
        // The supported finite interior and +/-1 endpoints follow source CRT.
        unsafe {
            core::arch::asm!(
            "fxsave64 [{saved}]","fninit","fldcw word ptr [{cw}]",
            "test {pending:e},{pending:e}","jz 20f","fld1","fdiv dword ptr [{three}]","fstp st(0)","20:",
            "fld qword ptr [{source_angle}]","fcos","fsubr dword ptr [{one}]",
            "fmul dword ptr [{radius}]","fadd st(0),st(0)","fmul st(0),st(0)",
            "fld dword ptr [{squared}]","fcompp","fnstsw ax","test ah,5","jp 29f",
            "mov eax,dword ptr [{radius}]","and eax,0x7fffffff","jz 29f",
            "fld dword ptr [{squared}]","fsqrt","fstp dword ptr [{temp}]",
            "fld dword ptr [{radius}]","fadd st(0),st(0)","fdivr dword ptr [{temp}]","fsubr dword ptr [{one}]",
            "fst qword ptr [{wide}]",
            "mov rax,qword ptr [{wide}]","btr rax,63","mov {scratch},0x3ff0000000000000",
            "cmp rax,{scratch}","ja 28f","je 25f",
            "fld1","fadd st(0),st(1)","fld1","fsub st(0),st(2)","fmulp st(1),st(0)","fsqrt","fxch st(1)","fpatan","jmp 26f",
            "25:","fstp st(0)","test byte ptr [{wide}+7],0x80","jz 30f","fldpi","jmp 26f","30:","fldz",
            "26:","cmp word ptr [{cw}],0x27f","je 27f","fnstsw ax","test ax,0x20","jz 27f",
            "fstp qword ptr [{wide}]","fld qword ptr [{wide}]","27:","fstp dword ptr [{result}]","jmp 29f",
            "28:","fstp st(0)","mov dword ptr [{result}],0x7fc00000","29:","fxrstor64 [{saved}]",
            saved=in(reg)saved.0.as_mut_ptr(),cw=in(reg)&cw,pending=in(reg)u32::from(pending),three=in(reg)&three,
            source_angle=in(reg)&source_angle,one=in(reg)&one,radius=in(reg)&radius,squared=in(reg)&squared,
            temp=in(reg)&mut temp,wide=in(reg)&mut wide,result=in(reg)&mut result,scratch=out(reg)_,out("rax")_,options(nostack));
        }
        if result & 0x7f800000 == 0x7f800000 {
            Err(Error::NonFiniteResult)
        } else {
            Ok(result)
        }
    }
}

#[cfg(all(test, target_arch = "x86_64"))]
mod tests {
    use super::*;
    #[test]
    fn turning_leaves_restore_caller_x87_environment() {
        let mut original = FxState([0; 512]);
        let mut before = FxState([0; 512]);
        let mut after = FxState([0; 512]);
        let control = 0x0b7fu16;
        let mxcsr = 0x9fc0u32;
        let three = 0x40400000u32;
        // SAFETY: fixed aligned buffers. Caller state is restored before any
        // assertions; no unsupported instruction or unchecked memory access.
        unsafe {
            core::arch::asm!("fxsave64 [{original}]","fninit","fldcw word ptr [{control}]",
            "ldmxcsr dword ptr [{mxcsr}]","fld1","fldpi","fld1","fdiv dword ptr [{three}]","fstp st(0)","fxsave64 [{before}]",
            original=in(reg)original.0.as_mut_ptr(),control=in(reg)&control,mxcsr=in(reg)&mxcsr,three=in(reg)&three,before=in(reg)before.0.as_mut_ptr(),options(nostack));
        }
        let results = [
            near_waypoint([0, 0], [1, 0], Precision::Single24).is_ok(),
            waypoint_turn_offset(
                0x40000000,
                [0x3f800000, 0],
                [0, 0x3f800000],
                Precision::Extended64,
                true,
            )
            .is_ok(),
            small_sin_cos(0x3f490fdb, Precision::Double53).is_ok(),
            recovery_angle(0x41a00000, 0x3f9ae148, Precision::Single24, true).is_ok(),
        ];
        unsafe {
            core::arch::asm!("fxsave64 [{after}]","fxrstor64 [{original}]",after=in(reg)after.0.as_mut_ptr(),original=in(reg)original.0.as_mut_ptr(),options(nostack));
        }
        assert!(results.into_iter().all(|x| x));
        assert_eq!(&before.0[..5], &after.0[..5]);
        assert_eq!(&before.0[24..28], &after.0[24..28]);
        for slot in 0..8 {
            let start = 32 + slot * 16;
            assert_eq!(&before.0[start..start + 10], &after.0[start..start + 10]);
        }
    }
    #[test]
    fn turning_rejects_unsupported_inputs_explicitly() {
        assert!(near_waypoint([0x7fc00000, 0], [0, 0], Precision::Extended64).is_err());
        assert!(small_sin_cos(0x7f7fffff, Precision::Extended64).is_err());
        assert!(recovery_angle(0x7f800000, 0, Precision::Extended64, false).is_err());
    }
}
