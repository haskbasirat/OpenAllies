//! Actual current Windows window observations feed the retained native App+34
//!word. Egui's keyboard Focused event alone is not WM_ACTIVATE: the original
//!WM_SETFOCUS only logs. This backend samples foreground process, active HWND
//!and IsIconic, then translates their current values to the two native writers.
//!It does not claim to reproduce Windows message scheduling or all native
//!OnActivate timer/input effects. No missing observation becomes focused=true.
use aa_runtime::application_window_state::{ApplicationWindowState, Reader};

pub struct Focus {
    state: ApplicationWindowState,
    window: Option<usize>,
    last: Option<(bool, bool, bool)>,
    error: Option<String>,
}
impl Focus {
    pub fn new() -> Self {
        Self {
            state: ApplicationWindowState::from_native_constructor(),
            window: None,
            last: None,
            error: None,
        }
    }
    pub fn reader(&self) -> Reader {
        self.state.reader()
    }
    ///Use the exact eframe main-window handle, never whichever popup happens
    ///to be active when the first input arrives. wgpu reexports the same
    ///raw-window-handle version already used by eframe; no new dependency.
    pub fn observe_frame(&mut self, frame: &eframe::Frame) {
        #[cfg(windows)]
        {
            use wgpu::rwh::{HasWindowHandle, RawWindowHandle};
            let bound = (|| {
                let handle = frame
                    .window_handle()
                    .map_err(|e| format!("actual eframe window handle is unavailable: {e}"))?;
                let RawWindowHandle::Win32(handle) = handle.as_raw() else {
                    return Err("eframe window is not an actual Win32 HWND".into());
                };
                let current = handle.hwnd.get() as usize;
                if self.window.is_some_and(|previous| previous != current) {
                    return Err(
                        "eframe main-window allocation changed while its focus owner was retained"
                            .into(),
                    );
                }
                self.window = Some(current);
                Ok::<_, String>(())
            })();
            if let Err(error) = bound {
                let _ = self.state.observation_result(Err(error.clone()));
                self.error = Some(error);
                return;
            }
        }
        #[cfg(not(windows))]
        let _ = frame;
        self.observe();
    }
    pub fn require_observed(&self) -> Result<(), String> {
        if let Some(error) = &self.error {
            Err(error.clone())
        } else if self.last.is_some() {
            Ok(())
        } else {
            Err(self.error.clone().unwrap_or_else(|| {
                "actual application window activation has not been observed".into()
            }))
        }
    }
    ///Call on the real UI thread on each input/update boundary, including while
    ///no match is running. The simulation reads the same atomic owner lazily.
    pub fn observe(&mut self) {
        match self.sample() {
            Ok((app, active, minimized)) => {
                if self.last.map(|v| v.0) != Some(app) {
                    self.state.activate_app(app);
                }
                if self.last.map(|v| (v.1, v.2)) != Some((active, minimized)) {
                    self.state
                        .window_activate(u32::from(active) | (u32::from(minimized) << 16));
                }
                self.last = Some((app, active, minimized));
                self.error = self.state.observation_result(Ok(())).err();
            }
            Err(error) => {
                let _ = self.state.observation_result(Err(error.clone()));
                self.error = Some(error);
            }
        }
    }
    #[cfg(windows)]
    fn sample(&mut self) -> Result<(bool, bool, bool), String> {
        use std::ffi::c_void;
        #[link(name = "user32")]
        unsafe extern "system" {
            fn GetActiveWindow() -> *mut c_void;
            fn GetForegroundWindow() -> *mut c_void;
            fn GetWindowThreadProcessId(window: *mut c_void, process: *mut u32) -> u32;
            fn IsIconic(window: *mut c_void) -> i32;
            fn IsWindow(window: *mut c_void) -> i32;
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetCurrentThreadId() -> u32;
            fn GetCurrentProcessId() -> u32;
        }
        //SAFETY: Win32 handles are observed only; no pointers are dereferenced,
        //window procedure changed, or OS window lifetime extended by this type.
        unsafe {
            let thread = GetCurrentThreadId();
            let process = GetCurrentProcessId();
            let active = GetActiveWindow();
            let window = self
                .window
                .ok_or("actual eframe main-window HWND has not been bound")?
                as *mut c_void;
            if IsWindow(window) == 0 {
                return Err("retained application HWND has been destroyed".into());
            }
            let mut window_process = 0;
            if GetWindowThreadProcessId(window, &mut window_process) != thread
                || window_process != process
            {
                return Err("retained HWND no longer belongs to this application UI thread".into());
            }
            let foreground = GetForegroundWindow();
            let mut foreground_process = 0;
            if !foreground.is_null()
                && GetWindowThreadProcessId(foreground, &mut foreground_process) == 0
            {
                return Err("foreground HWND process lookup failed".into());
            }
            Ok((
                !foreground.is_null() && foreground_process == process,
                active == window && foreground == window,
                IsIconic(window) != 0,
            ))
        }
    }
    #[cfg(not(windows))]
    fn sample(&mut self) -> Result<(bool, bool, bool), String> {
        Err("native window activation requires Windows".into())
    }
}
