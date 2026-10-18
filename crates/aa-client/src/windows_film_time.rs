//! Actual native6FB1F0 operating-system leaf used by current film headers.
//! Local FILETIME is a serialized timestamp; it never drives simulation time.
#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct FileTime {
    low: u32,
    high: u32,
}
#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetSystemTimeAsFileTime(time: *mut FileTime);
    fn FileTimeToLocalFileTime(utc: *const FileTime, local: *mut FileTime) -> i32;
}
pub fn local_file_time() -> Result<[u32; 2], String> {
    #[cfg(windows)]
    {
        let mut utc = FileTime::default();
        let mut local = FileTime::default();
        //Both arguments are valid aligned FILETIME allocations for the full
        //duration of each synchronous Win32 call, in native call order.
        unsafe {
            GetSystemTimeAsFileTime(&mut utc);
            if FileTimeToLocalFileTime(&utc, &mut local) == 0 {
                return Err(format!(
                    "FileTimeToLocalFileTime: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }
        Ok([local.low, local.high])
    }
    #[cfg(not(windows))]
    {
        Err("native local FILETIME requires Windows".into())
    }
}
