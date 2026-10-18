//! Real Windows inputs of native Session::Reset76EC70, kept outside the
//! forbid(unsafe_code) network/simulation crates. No helper process or RNG
//! substitutes for the two original OS calls.
//!
//! Signatures/link libraries: Microsoft Learn, sysinfoapi/GetTickCount and
//! combaseapi/CoCreateGuid (checked2026-10-16). GetTickCount stays DWORD/u32;
//! the 49.7-day wrap is the original input, not upgraded to a different clock.
use aa_netcode::local_bootstrap::{Guid, ResetOs};

pub struct WindowsResetOs<'a> {
    /// The actual application/simulation owner supplies this gate. The native
    /// disabled/suspended Synchronizer guard avoids calling it when irrelevant.
    sync_gate: &'a mut dyn FnMut() -> aa_netcode::Result<bool>,
}
impl<'a> WindowsResetOs<'a> {
    pub fn new(sync_gate: &'a mut dyn FnMut() -> aa_netcode::Result<bool>) -> Self {
        Self { sync_gate }
    }
}

#[cfg(windows)]
mod platform {
    #[repr(C)]
    pub(super) struct NativeGuid {
        pub data1: u32,
        pub data2: u16,
        pub data3: u16,
        pub data4: [u8; 8],
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub(super) fn GetTickCount() -> u32;
        pub(super) fn GetCurrentThreadId() -> u32;
        pub(super) fn QueryPerformanceCounter(value: *mut i64) -> i32;
        pub(super) fn QueryPerformanceFrequency(value: *mut i64) -> i32;
    }
    #[link(name = "ole32")]
    unsafe extern "system" {
        pub(super) fn CoCreateGuid(value: *mut NativeGuid) -> i32;
    }
}

/// Real85FAA9/4296B0 OS identity producer. Capture the application identity
/// on its actual frontend thread; query this again at every native event.
pub struct WindowsThreadIds;
impl aa_runtime::execution_context::OsThreadIds for WindowsThreadIds {
    fn current_thread_id(&mut self) -> Result<u32, String> {
        #[cfg(windows)]
        {
            // SAFETY: no arguments or retained pointers; DWORD identity only.
            Ok(unsafe { platform::GetCurrentThreadId() })
        }
        #[cfg(not(windows))]
        {
            Err("native OS thread identities require Windows".into())
        }
    }
}

/// NetNode+50 timer59AB60/59ABA0. This owns its actual QPC anchor and borrows
/// no cached frame-time scalar. Reset's GetTickCount is a distinct source.
pub struct PerformanceTimer {
    anchor: i64,
    frequency: i64,
}
impl PerformanceTimer {
    pub fn from_native_constructor() -> aa_netcode::Result<Self> {
        #[cfg(windows)]
        {
            // Native59AA70 initializes the shared frequency once. Keeping the
            // same OS frequency owner avoids independently rounded time bases.
            static FREQUENCY: std::sync::OnceLock<Result<i64, String>> = std::sync::OnceLock::new();
            let frequency = FREQUENCY
                .get_or_init(|| {
                    let mut value = std::mem::MaybeUninit::<i64>::uninit();
                    // SAFETY: LARGE_INTEGER is one signed64-bit output; the API
                    // retains no pointer. Query failure is never a fake frequency.
                    if unsafe { platform::QueryPerformanceFrequency(value.as_mut_ptr()) } == 0 {
                        return Err("QueryPerformanceFrequency failed".into());
                    }
                    // SAFETY: nonzero return guarantees output initialization.
                    let mut value = unsafe { value.assume_init() };
                    // Original59AA70 tests/stores the LOW DWORD, preserving high.
                    if value as u32 == 0 {
                        value = (value & !0xffff_ffffi64) | 1;
                    }
                    Ok(value)
                })
                .as_ref()
                .map_err(|s| aa_netcode::Error::Protocol(s.clone()))?;
            Ok(Self {
                anchor: read_counter()?,
                frequency: *frequency,
            })
        }
        #[cfg(not(windows))]
        {
            Err(aa_netcode::Error::Unsupported(
                "native QPC timer requires Windows",
            ))
        }
    }
    pub fn milliseconds(&self) -> aa_netcode::Result<u32> {
        #[cfg(windows)]
        {
            native_timer_milliseconds(read_counter()?, self.anchor, self.frequency)
        }
        #[cfg(not(windows))]
        {
            Err(aa_netcode::Error::Unsupported(
                "native QPC timer requires Windows",
            ))
        }
    }
}
impl aa_netcode::local_host::MillisecondTimer for PerformanceTimer {
    fn milliseconds(&mut self) -> Result<u32, String> {
        PerformanceTimer::milliseconds(self).map_err(|e| e.to_string())
    }
}

#[cfg(windows)]
fn read_counter() -> aa_netcode::Result<i64> {
    let mut value = std::mem::MaybeUninit::<i64>::uninit();
    // SAFETY: exact LARGE_INTEGER output ABI and a valid writable local.
    if unsafe { platform::QueryPerformanceCounter(value.as_mut_ptr()) } == 0 {
        return Err(aa_netcode::Error::Protocol(
            "QueryPerformanceCounter failed".into(),
        ));
    }
    // SAFETY: successful QueryPerformanceCounter initialized the output.
    Ok(unsafe { value.assume_init() })
}

/// Exact integer body59ABA0: SUB/SBB delta, __allmul wrapping64 then
/// __alldiv signed truncation, low32 return. Invalid native divisors refuse.
fn native_timer_milliseconds(counter: i64, anchor: i64, frequency: i64) -> aa_netcode::Result<u32> {
    counter
        .wrapping_sub(anchor)
        .wrapping_mul(1000)
        .checked_div(frequency)
        .map(|v| v as u32)
        .ok_or_else(|| aa_netcode::Error::Protocol("native QPC timer division is undefined".into()))
}

impl ResetOs for WindowsResetOs<'_> {
    fn get_tick_count(&mut self) -> aa_netcode::Result<u32> {
        #[cfg(windows)]
        {
            // SAFETY: exact no-argument Win32 DWORD ABI; no pointer or resource
            // ownership crosses the boundary.
            Ok(unsafe { platform::GetTickCount() })
        }
        #[cfg(not(windows))]
        {
            Err(aa_netcode::Error::Unsupported(
                "native Reset GetTickCount requires Windows",
            ))
        }
    }
    fn co_create_guid(&mut self) -> aa_netcode::Result<Guid> {
        #[cfg(windows)]
        {
            let mut value = std::mem::MaybeUninit::<platform::NativeGuid>::uninit();
            // SAFETY: repr(C) exactly matches GUID (4+2+2+8 bytes, align4).
            // CoCreateGuid receives one valid writable GUID and retains no
            // pointer. Read it only after the documented S_OK result.
            let result = unsafe { platform::CoCreateGuid(value.as_mut_ptr()) };
            if result != 0 {
                return Err(aa_netcode::Error::Protocol(format!(
                    "CoCreateGuid failed HRESULT {:#010X}",
                    result as u32
                )));
            }
            // SAFETY: S_OK guarantees all GUID fields were written above.
            let value = unsafe { value.assume_init() };
            Ok(Guid {
                data1: value.data1,
                data2: value.data2,
                data3: value.data3,
                data4: value.data4,
            })
        }
        #[cfg(not(windows))]
        {
            Err(aa_netcode::Error::Unsupported(
                "native Reset CoCreateGuid requires Windows",
            ))
        }
    }
    fn sync_log_gate(&mut self) -> aa_netcode::Result<bool> {
        (self.sync_gate)()
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn actual_windows_reset_inputs_use_the_native_widths_and_checked_guid_result() {
        assert_eq!(std::mem::size_of::<platform::NativeGuid>(), 16);
        assert_eq!(std::mem::align_of::<platform::NativeGuid>(), 4);
        let mut unavailable = || {
            Err(aa_netcode::Error::Unsupported(
                "test logger gate is intentionally unbound",
            ))
        };
        let mut os = WindowsResetOs::new(&mut unavailable);
        let first = os.get_tick_count().unwrap();
        let value = os.co_create_guid().unwrap();
        let last = os.get_tick_count().unwrap();
        assert!(last.wrapping_sub(first) < 60_000);
        assert!(!value.native_text().is_empty());
        assert!(os.sync_log_gate().is_err());
    }
    #[test]
    fn actual_windows_performance_timer_retains_its_qpc_anchor() {
        let timer = PerformanceTimer::from_native_constructor().unwrap();
        let before = timer.milliseconds().unwrap();
        let after = timer.milliseconds().unwrap();
        assert!(after.wrapping_sub(before) < 60_000);
        assert!(timer.frequency > 0);
    }
    #[test]
    fn native_performance_counter_integer_kernel_replay() {
        let f: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-local-bootstrap/native-timer.json"
        ))
        .unwrap();
        assert_eq!(f["passed"], true);
        for row in f["cases"].as_array().unwrap() {
            let got = native_timer_milliseconds(
                row["counter"].as_i64().unwrap(),
                row["anchor"].as_i64().unwrap(),
                row["frequency"].as_i64().unwrap(),
            )
            .unwrap();
            assert_eq!(got as u64, row["milliseconds"].as_u64().unwrap());
        }
        assert!(native_timer_milliseconds(1, 0, 0).is_err());
    }
}
