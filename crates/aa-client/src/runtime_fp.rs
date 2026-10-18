//! Read-only current-thread controls for the supported native CRT pow path.
//! This is sampled at runtime; no source-phase capture supplies these values.
//! Software native arithmetic still receives its separate explicit live PC.
use aa_numeric::crt_pow::{CrtEnvironment, PowPath, select_path};

pub fn sample() -> Result<CrtEnvironment, String> {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        let mut x87_control = 0u16;
        let mut mxcsr = 0u32;
        //Each instruction writes only the local output cell. No control word,
        //rounding mode, exception mask, status word or MXCSR register is changed.
        unsafe {
            core::arch::asm!("fnstcw [{}]",in(reg)&mut x87_control,options(nostack,preserves_flags));
            core::arch::asm!("stmxcsr [{}]",in(reg)&mut mxcsr,options(nostack,preserves_flags));
        }
        let environment = CrtEnvironment {
            sse2_available: std::is_x86_feature_detected!("sse2"),
            mxcsr,
            x87_control,
        };
        if select_path(environment) != PowPath::Sse2 {
            return Err(
                "runtime CRT96D890 selects unported x87 96D8ED path for actual thread controls"
                    .into(),
            );
        }
        if x87_control & 0x0C00 != 0 {
            return Err(format!(
                "runtime CRT x87 rounding control {x87_control:#06x} is unsupported"
            ));
        }
        if mxcsr & !0x3F != 0x1F80 {
            return Err(format!(
                "runtime CRT MXCSR {mxcsr:#010x} rounding/FTZ/DAZ control is unsupported"
            ));
        }
        Ok(environment)
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        Err(
            "runtime native CRT environment sampler requires actual x86/x86_64 thread controls"
                .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn current_thread_sampler_reads_supported_controls_without_changing_them() {
        let before = super::sample().unwrap();
        let after = super::sample().unwrap();
        assert_eq!(before, after);
        assert!(before.sse2_available);
        assert_eq!(before.mxcsr & !0x3F, 0x1F80);
        assert_eq!(before.x87_control & 0x0C00, 0);
        println!(
            "actual current-thread SSE2={} CW={:#06x} MXCSR={:#010x}",
            before.sse2_available, before.x87_control, before.mxcsr
        );
    }
}
