//! Standalone test-only x86_64 capture. Input records: u32 CW,u8 wide,3pad,u64 x,y.
//! Output records: ten-byte extended remainder then four-byte binary32 store.
use std::{fs, io};
#[repr(C, align(16))]
struct State([u8; 512]);
#[repr(C)]
struct Work {
    x: u64,
    y: u64,
    result: [u8; 10],
    control: u16,
    narrow: u32,
}
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "input and new output required",
        ));
    }
    let inputs = fs::read(&args[1])?;
    assert_eq!(inputs.len() % 24, 0);
    let mut output = Vec::new();
    for record in inputs.chunks_exact(24) {
        let control = u32::from_le_bytes(record[..4].try_into().unwrap()) as u16;
        assert!([0x7f, 0x27f, 0x37f].contains(&control));
        let wide = u32::from(record[4]);
        assert!(wide <= 1);
        let x = u64::from_le_bytes(record[8..16].try_into().unwrap());
        let y = u64::from_le_bytes(record[16..].try_into().unwrap());
        let (finite_mask, absolute_mask) = if wide == 1 {
            (0x7ff0_0000_0000_0000, 0x7fff_ffff_ffff_ffff)
        } else {
            (0x7f80_0000, 0x7fff_ffff)
        };
        assert!(
            x & finite_mask != finite_mask
                && y & finite_mask != finite_mask
                && y & absolute_mask != 0
        );
        let mut saved = State([0; 512]);
        let mut work = Work {
            x,
            y,
            result: [0; 10],
            control,
            narrow: 0,
        };
        // SAFETY: x86_64, fixed valid local buffers; all paths restore the caller's
        // FP environment and register stack. Finite nonzero divisor terminates
        // the real FPREM loop. EAX/flags are declared clobbered, no call/SP changes.
        unsafe {
            core::arch::asm!(
            "fxsave64 [{saved}]", "fninit", "fldcw word ptr [{w}+{control}]",
            "test {wide:e},{wide:e}", "jnz 20f",
            "fld dword ptr [{w}+{y}]", "fld dword ptr [{w}+{x}]", "jmp 21f",
            "20:", "fld qword ptr [{w}+{y}]", "fld qword ptr [{w}+{x}]",
            "21:", "fprem", "fnstsw ax", "test ah,4", "jnz 21b",
            "fld st(0)", "fstp tbyte ptr [{w}+{result}]", "fstp dword ptr [{w}+{narrow}]",
            "fstp st(0)", "fxrstor64 [{saved}]",
            saved=in(reg) saved.0.as_mut_ptr(),w=in(reg) &mut work,wide=in(reg) wide,
            x=const std::mem::offset_of!(Work,x),y=const std::mem::offset_of!(Work,y),
            result=const std::mem::offset_of!(Work,result),control=const std::mem::offset_of!(Work,control),
            narrow=const std::mem::offset_of!(Work,narrow),out("eax") _,options(nostack));
        }
        output.extend_from_slice(&work.result);
        output.extend_from_slice(&work.narrow.to_le_bytes());
    }
    use std::io::Write;
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args[2])?
        .write_all(&output)
}
