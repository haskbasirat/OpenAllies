//!Real egui input routed into the ported native camera controls
//!(aa_runtime::native_camera::{input,hotkeys}) on the camera's own
//!SimulationThread owner. Only Send event packets cross threads.
//!
//!UI thread: egui events become KKC_Input events (59A5C0 keyboard codes,
//!59A6E0 mouse codes 0x100.., wheel 0x110 with the WM_MOUSEWHEEL delta), the
//!cursor is mapped from the window client area onto the native client
//!resolution (the camera target, 0,0,W-1,H-1 as 40E210) and the frame's QPC
//!counter is captured. Simulation thread: each queued UI frame in order runs
//!hotkeys_core.txt bindings (5EFC70) -> camera commands -> one native UI
//!frame (5C92F0 subset) whose QPC reads all return that captured counter.
//!
//!Provisional modern mappings (no TGI VWorld widget exists yet; reported for
//!the HUD owner): VWorld+24 bit19 = native battle frame presented; 5EFA80
//!hover = cursor inside the client window; context `vworld`/`game` valid
//!while the frame is presented. App+48 = the eframe viewport is not
//!fullscreen, App+49 = 0 (no coop borderless mode). Numpad keys cannot be
//!told apart by egui and are not produced.
use aa_numeric::Precision;
use aa_runtime::native_camera::{
    hotkeys::{self, Bindings, Command},
    input::{self, Controls, Rect, Trace},
};
use eframe::egui;
type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    ///KKC_Input key/mouse-button event: code and flags (1 down, 2 up, 4 repeat,
    ///0x10 alt, 0x20 shift, 0x40 ctrl).
    Key { code: u32, flags: u32 },
    ///WM_MOUSEWHEEL HIWORD(wParam) as a signed short.
    Wheel { delta: i16, flags: u32 },
}
///One UI frame of observed input (Send).
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub events: Vec<Event>,
    pub cursor: Option<[i32; 2]>,
    pub client: [u32; 2],
    pub presented: bool,
    pub windowed: bool,
    ///QueryPerformanceCounter read when this UI frame was observed. Every
    ///native QPC read of the frame's controls uses it, so frames queued
    ///behind a running camera job keep their real inter-frame times.
    pub counter: Option<i64>,
}
impl Frame {
    ///5C92F0 consumes every KKC_Input event queued since the previous frame
    ///(59A500 queue, filled by WndProc) in ONE native UI frame, then polls the
    ///cursor once (59AD30). The port's native frame is one presentation job
    ///(one rendered snapshot), so a UI frame observed while a job is pending
    ///joins the next native frame: every event in order (no press or release
    ///is dropped), and the latest cursor, client, gates and QPC frame time.
    pub fn coalesce(&mut self, next: Frame) {
        self.events.extend(next.events);
        self.cursor = next.cursor;
        self.client = next.client;
        self.presented = next.presented;
        self.windowed = next.windowed;
        self.counter = next.counter;
    }
}
fn modifier_flags(m: &egui::Modifiers) -> u32 {
    (if m.alt { hotkeys::ALT } else { 0 })
        | (if m.shift { hotkeys::SHIFT } else { 0 })
        | (if m.ctrl { hotkeys::CTRL } else { 0 })
}
///Windows virtual-key codes for the keys present in the camera bindings.
fn virtual_key(key: egui::Key) -> Option<u32> {
    use egui::Key::*;
    Some(match key {
        ArrowLeft => 0x25,
        ArrowUp => 0x26,
        ArrowRight => 0x27,
        ArrowDown => 0x28,
        PageUp => 0x21,
        PageDown => 0x22,
        End => 0x23,
        Home => 0x24,
        _ => return None,
    })
}
///Collect this UI frame. `image` is where the native target is drawn and
///`pixels` its native size; the cursor becomes native client pixels.
///`client` is the window client area (the native 59AD30 rectangle covers the
///whole client window, letterbox bars included) and `pixels` the native
///camera target size used as the client resolution.
pub fn observe(ctx: &egui::Context, client: egui::Rect, pixels: [u32; 2], presented: bool) -> Frame {
    let counter = input::Qpc::counter(&mut Qpc).ok();
    let image = client;
    ctx.input(|i| {
        let mut events = Vec::new();
        for e in &i.events {
            match e {
                egui::Event::Key { key, pressed, repeat, modifiers, .. } => {
                    if let Some(code) = virtual_key(*key) {
                        let flags = if *pressed { if *repeat { 5 } else { 1 } } else { 2 };
                        events.push(Event::Key { code, flags: flags | modifier_flags(modifiers) });
                    }
                }
                egui::Event::PointerButton { button: egui::PointerButton::Middle, pressed, modifiers, .. } => {
                    let flags = if *pressed { 1 } else { 2 };
                    events.push(Event::Key { code: 0x102, flags: flags | 0x80 | modifier_flags(modifiers) });
                }
                egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta, modifiers } => {
                    //winit LineDelta = WM_MOUSEWHEEL delta / WHEEL_DELTA(120).
                    let native = (delta.y * 120.0).round();
                    if native != 0.0 && native.abs() <= 32767.0 {
                        events.push(Event::Wheel { delta: native as i16, flags: 1 | 0x80 | modifier_flags(modifiers) });
                    }
                }
                _ => {}
            }
        }
        let cursor = i.pointer.latest_pos().filter(|_| image.width() > 0.0 && image.height() > 0.0).map(|p| {
            let x = (p.x - image.min.x) * pixels[0] as f32 / image.width();
            let y = (p.y - image.min.y) * pixels[1] as f32 / image.height();
            [x.floor() as i32, y.floor() as i32]
        });
        let windowed = !i.viewport().fullscreen.unwrap_or(false);
        Frame { events, cursor, client: pixels, presented, windowed, counter }
    })
}

///Hotkey bindings, controls and down-key table on the camera owner.
pub struct Native {
    bindings: Bindings,
    controls: Controls,
    down: std::collections::BTreeSet<u32>,
    active: bool,
    started: bool,
}
pub struct Qpc;
impl input::Qpc for Qpc {
    fn counter(&mut self) -> Result<i64> {
        #[cfg(windows)]
        {
            let mut value = 0i64;
            //SAFETY: one writable LARGE_INTEGER; no pointer is retained.
            if unsafe { platform::QueryPerformanceCounter(&mut value) } == 0 {
                return Err("QueryPerformanceCounter failed".into());
            }
            Ok(value)
        }
        #[cfg(not(windows))]
        Err("native QPC requires Windows".into())
    }
    fn frequency(&mut self) -> Result<i64> {
        #[cfg(windows)]
        {
            let mut value = 0i64;
            //SAFETY: one writable LARGE_INTEGER; no pointer is retained.
            if unsafe { platform::QueryPerformanceFrequency(&mut value) } == 0 {
                return Err("QueryPerformanceFrequency failed".into());
            }
            //59AA70 replaces a zero low DWORD by 1, preserving the high word.
            if value as u32 == 0 {
                value = (value & !0xffff_ffff) | 1;
            }
            Ok(value)
        }
        #[cfg(not(windows))]
        Err("native QPC requires Windows".into())
    }
}
///The frame's observed counter for every QPC read of that native UI frame.
pub struct FrameClock(Option<i64>);
impl input::Qpc for FrameClock {
    fn counter(&mut self) -> Result<i64> {
        match self.0 {
            Some(value) => Ok(value),
            None => input::Qpc::counter(&mut Qpc),
        }
    }
    fn frequency(&mut self) -> Result<i64> {
        input::Qpc::frequency(&mut Qpc)
    }
}
#[cfg(windows)]
mod platform {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub(super) fn QueryPerformanceCounter(value: *mut i64) -> i32;
        pub(super) fn QueryPerformanceFrequency(value: *mut i64) -> i32;
    }
}

///Current frontend facts for the native gates of one UI frame.
pub struct Gates<'a> {
    pub focus_flags: &'a aa_runtime::application_window_state::Reader,
    pub vworld_ad: bool,
    pub visual_map: bool,
    pub frame: &'a Frame,
}
impl input::Ui for Gates<'_> {
    fn application_active(&mut self) -> Result<bool> {
        Ok(self.focus_flags.flags_b396c4()? >> 1 & 1 != 0)
    }
    fn vworld_ad(&mut self) -> Result<bool> {
        Ok(self.vworld_ad)
    }
    fn vworld_flag_24_bit19(&mut self) -> Result<bool> {
        Ok(self.frame.presented)
    }
    fn visual_map_bc(&mut self) -> Result<bool> {
        Ok(self.visual_map)
    }
    fn vworld_hovered_5efa80(&mut self) -> Result<bool> {
        let [w, h] = self.frame.client;
        Ok(self.frame.cursor.is_some_and(|[x, y]| x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h))
    }
    fn app_byte_48(&mut self) -> Result<bool> {
        Ok(self.frame.windowed)
    }
    fn app_byte_49(&mut self) -> Result<bool> {
        Ok(false)
    }
    fn ui_widget_60_consumes_move(&mut self) -> Result<bool> {
        Ok(false)
    }
}

impl Native {
    pub fn new(core_hotkeys: &str, client: [u32; 2]) -> Result<Self> {
        let bindings = Bindings::parse_core(core_hotkeys)?;
        let controls = Controls::from_native_constructors(&mut Qpc, Rect::client_40e210(client[0], client[1])?)?;
        Ok(Self { bindings, controls, down: Default::default(), active: false, started: false })
    }
    pub fn controls(&self) -> &Controls {
        &self.controls
    }
    ///Diagnostic summary of the retained native control state.
    pub fn status(&self) -> String {
        let c = &self.controls;
        let s = c.scroller();
        format!("slot={:?} bits={} keys={} edge={} inside={} ui_cursor={:?} impulses={} down={:?}",
            c.active(), s.bits(), s.keys(), s.edge(), c.input_cursor().inside_24, c.ui_cursor(),
            c.impulses().len(), self.down)
    }
    ///Process this frame's events and one native UI frame, in native order.
    pub fn frame(
        &mut self,
        frame: &Frame,
        gates: &mut Gates<'_>,
        vars: &mut impl input::Variables,
        camera: &mut impl input::Camera,
        p: Precision,
    ) -> Result<Vec<Trace>> {
        let mut trace = Vec::new();
        self.controls.set_rect_59aeb0(Rect::client_40e210(frame.client[0], frame.client[1])?);
        if !self.started {
            if let Some(cursor) = frame.cursor {
                self.controls.adopt_cursor(cursor);
                self.started = true;
            }
        }
        let active = input::Ui::application_active(gates)?;
        let mut events = frame.events.clone();
        if self.active && !active {
            //40DA30 OnDeactivate: 59AF40 releases (mouse codes first, then
            //keys, ascending) and 599930 empties the impulse list.
            let mut held: Vec<u32> = self.down.iter().copied().collect();
            held.sort_by_key(|c| (*c < 0x100, *c));
            events.extend(held.into_iter().map(|code| Event::Key { code, flags: 2 }));
            self.controls.clear_impulses_599930();
        }
        self.active = active;
        let mut clock = FrameClock(frame.counter);
        for event in events {
            let (code, flags, wheel) = match event {
                Event::Key { code, flags } => (code, flags, None),
                Event::Wheel { delta, flags } => (hotkeys::WHEEL, flags, Some(delta)),
            };
            //KKC_Input +44 state table: down events (wheel included) set
            //it, up events clear it; 59AF40 releases whatever is set.
            if flags & 1 != 0 {
                self.down.insert(code);
            } else if flags & 2 != 0 {
                self.down.remove(&code);
            }
            let presented = frame.presented;
            let Some(binding) = self.bindings.lookup(code, flags, |_| Ok(presented))? else { continue };
            let mut text = binding.command.clone();
            if let Some(delta) = wheel {
                let argument = input::wheel_argument_5efccc(delta, p)?;
                let (formatted, _) = hotkeys::wheel_command_value(argument)?;
                text = text.replacen("%f", &formatted, 1);
            }
            self.execute(&text, gates, vars, camera, &mut clock, &mut trace, p)?;
        }
        let mut clock = FrameClock(frame.counter);
        trace.extend(self.controls.frame_5c92f0(frame.cursor, gates, vars, camera, &mut clock, p)?);
        Ok(trace)
    }
    fn execute(
        &mut self,
        text: &str,
        gates: &mut Gates<'_>,
        vars: &mut impl input::Variables,
        camera: &mut impl input::Camera,
        clock: &mut FrameClock,
        trace: &mut Vec<Trace>,
        p: Precision,
    ) -> Result<()> {
        match Command::parse(text)? {
            Command::BeginScroll(bit) => self.controls.begin_scroll_5cd560(bit, gates, clock, p),
            Command::EndScroll(bit) => self.controls.end_scroll_5ccd30(bit, gates, p),
            Command::RegisterImpulse(name) => self.controls.register_impulse_5c3f00(&name, clock, p),
            Command::UnregisterImpulse(name) => {
                self.controls.unregister_impulse_5c3f90(&name);
                Ok(())
            }
            Command::DeltaZoom(argument) => {
                //4257D0 _wtof then the binary32 store; 426330 bounds in 43BC80.
                let value: f64 = argument
                    .trim()
                    .parse()
                    .map_err(|_| format!("DeltaZoom argument `{argument}` is not a native float"))?;
                input::delta_zoom_43bc80((value as f32).to_bits(), camera, trace, p)
            }
            Command::ZoomMin => input::zoom_limit_43bc20(0x20c, camera, trace, p),
            Command::ZoomMax => input::zoom_limit_43bc20(0x210, camera, trace, p),
            Command::BeginCameraRotate => self.controls.begin_rotate_5c21c0(vars, camera, gates, p),
            Command::EndCameraRotate => {
                self.controls.end_rotate_5cc870();
                Ok(())
            }
            Command::DefaultKeyUpHandler => Ok(()),
        }
    }
}
