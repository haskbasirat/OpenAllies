//! KKC_UI in-game camera controls (UI.cpp, KKC_Input B39B18) ported from AA.exe.
//!
//! Native input path (all addresses in the original image, base 0x400000):
//! - WndProc 6FD4A0 -> KKC_Input 59A5C0/59A6E0 events -> hotkey 5EFC70/40A610
//!   -> command 426C20 (bindings from Data/Localization/Hotkeys/hotkeys_core.txt,
//!   see `hotkeys`). Wheel: 5EFC70 formats `(float)(short)delta/1200.0` (A42868).
//! - Begin/EndScroll{Left,Right,Up,Down} 5C12C0..5C1360 -> UI+40 scroller
//!   (vtable A4DA84): 5CD560 set key bit, 5CCD30 clear, 5CC7D0 direction,
//!   5CCC90 screen-edge bits, 5CCC20 acceleration, 5CD320 per-frame update
//!   -> camera 700500 -> 6FF130 -> 6FEDB0 (map rectangle clamp + 550210 height).
//! - Edge activation: game UI state (vtable A73CD0, built by 6B2CC0) update
//!   5CD6B0 -> 5CD420; scroller mouse-move 5CD410.
//! - `+name`/`-name` -> RegisterImpulse/UnregisterImpulse 5C3F00/5C3F90 ->
//!   table B284A4 (5C36A0) -> KKC_Input list 599DF0/5998F0, tick 599940/5995F0:
//!   yawleft/yawright 5C2930/5C2950 -> 5C2090 -> 700440 -> 6FF220,
//!   PitchUp/PitchDown 5C2970/5C2990 -> 5C20F0 -> 7004A0 -> 6FF220,
//!   ZoomIn/ZoomOut 5C29B0/5C2A10 -> 7010F0 -> 7005F0.
//! - VWorld commands (442C00): DeltaZoom 43BC80, ZoomMin 43BC20, ZoomMax 43BC50.
//! - Begin/EndCameraRotate 5C21C0/5C1370 -> UI+48 rotate state (vtable A4D968):
//!   5CCEB0/5CC870, mouse-move 5CCF00 -> 6FEBA0 + 6FF200.
//! - UI frame 5C92F0: cursor copy 5BF2B0 -> mouse-move dispatch 5C1AF0 ->
//!   impulse tick 599940 (tail: cursor poll 59AD30) ... UI-state updates (+8).
//!
//! The UI+C0 active-state slot is shared in the original with non-camera UI
//! states that are not ported here; only the two camera states are owned.
//! Every frontend fact (application active, VWorld flags, hover widget, ...)
//! is an explicit `Ui` service read at its reached native point.
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;
type Result<T> = std::result::Result<T, String>;

pub const SCROLL_UP: u32 = 1;
pub const SCROLL_DOWN: u32 = 2;
pub const SCROLL_LEFT: u32 = 4;
pub const SCROLL_RIGHT: u32 = 8;

const ONE: u32 = 0x3f80_0000; //9F32E0
const NEG_ONE: u32 = 0xbf80_0000; //5C2954/5C2994 push
const ZERO: u32 = 0; //9F32DC, B43DBC/B43DC0 (zero-initialised bss, no writer)
const MILLI: u32 = 0x3a83_126f; //9F4580
const NEG_MILLI: u32 = 0xba83_126f; //A4BB9C
const MICRO: u32 = 0x3586_37bd; //9FD38C
const DEG2RAD: u32 = 0x3c8e_fa35; //9FC064
const RAD2DEG: u32 = 0x4265_2ee1; //9FDA94
const TWO32: u32 = 0x4f80_0000; //9F240C
const TICK_MAX: u32 = 0x47c3_5000; //A42870 / 5CD399 push
const TICK_MIN: u32 = 0x461c_4000; //A4286C / 5CD3A2 push
const MICROSECONDS: u32 = 0x4974_2400; //A42A40
const WHEEL_DIVISOR: u32 = 0x4496_0000; //A42868
const DELTA_ZOOM_SIGN: u32 = 0xbf80_0000; //9FDC48

fn b(word: u32) -> Result<Binary> {
    Binary::from_f32_bits(word).map_err(|e| e.to_string())
}
fn f(value: &Binary) -> Result<u32> {
    value.store_f32().map_err(|e| e.to_string())
}
fn mul(a: &Binary, c: &Binary, p: Precision) -> Result<Binary> {
    a.mul(c, p).map_err(|e| e.to_string())
}
fn add(a: &Binary, c: &Binary, p: Precision) -> Result<Binary> {
    a.add(c, p).map_err(|e| e.to_string())
}
fn sub(a: &Binary, c: &Binary, p: Precision) -> Result<Binary> {
    a.sub(c, p).map_err(|e| e.to_string())
}
fn div(a: &Binary, c: &Binary, p: Precision) -> Result<Binary> {
    a.div(c, p).map_err(|e| e.to_string())
}
///FILD m32 of a native DWORD later corrected by FADD 2^32 when its sign bit
///is set (5C20B7/5C29D8/700518 pattern). The correction rounds at PC.
fn unsigned(value: u32, p: Precision) -> Result<Binary> {
    let signed = Binary::from_i32(value as i32);
    if (value as i32) < 0 {
        add(&signed, &b(TWO32)?, p)
    } else {
        Ok(signed)
    }
}
///_ftol2 96CC4C: truncation toward zero of the extended register. Values
///outside signed32 (whose low DWORD native would return) are refused.
fn ftol(value: &Binary) -> Result<u32> {
    value
        .truncate_i32()
        .map(|v| v as u32)
        .map_err(|_| "native _ftol2 operand outside the supported signed32 domain".into())
}
///FILD m64 is exact; an i64 always fits the 64-bit significand.
fn from_i64(value: i64) -> Result<Binary> {
    let high = Binary::from_i32((value >> 32) as i32);
    let low = Binary::from_u32(value as u32);
    add(&mul(&high, &b(TWO32)?, Precision::Extended64)?, &low, Precision::Extended64)
}

///The process QueryPerformanceCounter/Frequency pair. Frequency is the value
///59AA70 normalised (a zero low DWORD becomes 1).
pub trait Qpc {
    fn counter(&mut self) -> Result<i64>;
    fn frequency(&mut self) -> Result<i64>;
}
///A native 8-byte QPC anchor (59AB60 reset, 59AAD0 elapsed microseconds).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timer {
    anchor: i64,
}
impl Timer {
    ///59AB60: anchor = current counter.
    pub fn reset_59ab60(qpc: &mut impl Qpc) -> Result<Self> {
        Ok(Self { anchor: qpc.counter()? })
    }
    pub fn anchor(&self) -> i64 {
        self.anchor
    }
    ///A retained native anchor value (oracle replay / persisted UI state).
    pub fn from_anchor(anchor: i64) -> Self {
        Self { anchor }
    }
    ///59AAD0: FILD(counter-anchor) * 1e6f / FILD(frequency), both at PC. The
    ///extended result is what the callers then store or compare.
    pub fn elapsed_59aad0(&self, qpc: &mut impl Qpc, p: Precision) -> Result<Binary> {
        let delta = qpc.counter()?.wrapping_sub(self.anchor);
        let frequency = qpc.frequency()?;
        if frequency == 0 {
            return Err("59AAD0 native QPC frequency is zero".into());
        }
        div(&mul(&from_i64(delta)?, &b(MICROSECONDS)?, p)?, &from_i64(frequency)?, p)
    }
}

///Camera allocation reached through *B40048 (the current directory camera).
///Implementations borrow the actual normalized camera; every mutation is the
///corresponding ported camera operation.
pub trait Camera {
    fn word(&mut self, offset: u32) -> Result<u32>;
    fn byte_214(&mut self) -> Result<u8>;
    fn developer_6fe9c0(&mut self) -> Result<bool>;
    fn pan_700500(&mut self, millis: u32, direction: [u32; 2]) -> Result<()>;
    fn zoom_7005f0(&mut self, value: u32) -> Result<()>;
    fn pose_6ff220(&mut self, center: [u32; 3], yaw: u32, pitch: u32, zoom: u32) -> Result<()>;
    fn rotate_6feba0(&mut self, yaw: u32, pitch: u32) -> Result<()>;
    fn look_at_6ff200(&mut self, xy: [u32; 2]) -> Result<()>;
}
///UVar fast table B39940 (+18 ScrollSpeed, +1C Acceleration, +20 SpeedMax,
///+2C ScalePitch, +30 ScaleYaw, +34 ScaleZoom, +38 RotateSpeed, +B4 Rotate)
///and AVars B39990 (+50 CameraPitch with its +14/+18 range, +54 PitchMin).
pub trait Variables {
    fn uvar(&mut self, offset: u32) -> Result<u32>;
    fn avar(&mut self, offset: u32) -> Result<u32>;
    fn avar_range(&mut self, offset: u32) -> Result<[u32; 2]>;
    fn metagame_40cee0(&mut self) -> Result<bool>;
}
///Current frontend owners, read where the native code reads them.
pub trait Ui {
    ///App+34 bit1 (40DAC0/40DA30, window activation).
    fn application_active(&mut self) -> Result<bool>;
    ///VWorld(B36538)+AD. A missing VWorld is an error, as native dereferences it.
    fn vworld_ad(&mut self) -> Result<bool>;
    ///VWorld+24 bit 19 (widget input flag).
    fn vworld_flag_24_bit19(&mut self) -> Result<bool>;
    ///B36518 non-null and B36518+BC (5CD341..5CD356).
    fn visual_map_bc(&mut self) -> Result<bool>;
    ///5EFA80: VWorld+14 widget is UI+8C hover target or UI+74 capture.
    fn vworld_hovered_5efa80(&mut self) -> Result<bool>;
    ///App+48 (B396D8) and App+49 (B396D9) read by 5CD420.
    fn app_byte_48(&mut self) -> Result<bool>;
    fn app_byte_49(&mut self) -> Result<bool>;
    ///Game UI state +10 (5E04F0): UI+60 widget non-null and its +30 result.
    fn ui_widget_60_consumes_move(&mut self) -> Result<bool>;
}

///KKC_UI+40 scroller (constructor 5CD2E0). Direction +10/+14 has no
///constructor writer; only 5CC7D0 binds it.
#[derive(Clone, Debug)]
pub struct Scroller {
    bits_04: u32,
    edge_08: u32,
    keys_0c: u32,
    direction_10: Option<[u32; 2]>,
    timer_18: Timer,
    accumulated_20: u32,
    millis_24: u32,
}
///KKC_UI+48 rotate state (constructor 5CCE90: +4/+8/+C zero; +10 unwritten).
#[derive(Clone, Debug)]
pub struct Rotate {
    rate_04: u32,
    last_08: [i32; 2],
    first_10: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Scroller,
    Rotate,
}
///Table B284A4 (5C36A0 matches names with _wcsicmp).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImpulseFn {
    YawLeft,
    YawRight,
    PitchUp,
    PitchDown,
    ZoomIn,
    ZoomOut,
}
impl ImpulseFn {
    pub fn lookup_5c36a0(name: &str) -> Option<Self> {
        [
            ("yawleft", Self::YawLeft),
            ("yawright", Self::YawRight),
            ("PitchUp", Self::PitchUp),
            ("PitchDown", Self::PitchDown),
            ("ZoomIn", Self::ZoomIn),
            ("ZoomOut", Self::ZoomOut),
        ]
        .into_iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Impulse {
    pub function: ImpulseFn,
    pub last: u32,
}
///Native client rectangle (59AEB0 from 40E210: 0,0,width-1,height-1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
impl Rect {
    ///40E210: App+40/+44 client size minus one.
    pub fn client_40e210(width: u32, height: u32) -> Result<Self> {
        let w = i32::try_from(width).map_err(|_| "client width outside native int")?;
        let h = i32::try_from(height).map_err(|_| "client height outside native int")?;
        Ok(Self { left: 0, top: 0, right: w.wrapping_sub(1), bottom: h.wrapping_sub(1) })
    }
}
///KKC_Input cursor fields +0C/+10 (clamped client position), +14..+20 rect
///and +24 inside flag written by 59AD30.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputCursor {
    pub position_0c: [i32; 2],
    pub rect_14: Rect,
    pub inside_24: bool,
}
impl InputCursor {
    ///59AD30 after GetCursorPos/ScreenToClient: inside test then clamp.
    pub fn poll_59ad30(&mut self, client: [i32; 2]) {
        let r = self.rect_14;
        let [x, y] = client;
        self.position_0c = client;
        self.inside_24 = !(x < r.left || r.right < x || y < r.top || r.bottom < y);
        if !self.inside_24 {
            if self.position_0c[0] < r.left {
                self.position_0c[0] = r.left
            } else if r.right < self.position_0c[0] {
                self.position_0c[0] = r.right
            }
            if self.position_0c[1] < r.top {
                self.position_0c[1] = r.top;
                return;
            }
            if r.bottom < self.position_0c[1] {
                self.position_0c[1] = r.bottom
            }
        }
    }
}

///One trace record per camera operation (oracle/diagnostic comparison).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Trace {
    Pan { millis: u32, direction: [u32; 2] },
    Zoom { value: u32 },
    Pose { center: [u32; 3], yaw: u32, pitch: u32, zoom: u32 },
    Rotate { yaw: u32, pitch: u32 },
    LookAt { xy: [u32; 2] },
}

///The UI-owned camera control state: KKC_UI+40/+48/+58/+C0 and the KKC_Input
///impulse list/timer/cursor. One allocation per application UI.
#[derive(Clone, Debug)]
pub struct Controls {
    scroller: Scroller,
    rotate: Rotate,
    active_c0: Option<Slot>,
    ui_cursor_58: [i32; 2],
    input: InputCursor,
    impulses: Vec<Impulse>,
    impulse_timer_38: Timer,
}

impl Controls {
    ///KKC_Input 59A020 (timer +38: 59AA70/59AB60, cursor zero) and the UI
    ///constructors of +40 (5CD2E0) and +48 (5CCE90).
    pub fn from_native_constructors(qpc: &mut impl Qpc, rect: Rect) -> Result<Self> {
        let impulse_timer_38 = Timer::reset_59ab60(qpc)?;
        let scroller_timer = Timer::reset_59ab60(qpc)?;
        Ok(Self::from_native_anchors(impulse_timer_38, scroller_timer, rect))
    }
    ///Constructor state with explicit retained timer anchors.
    pub fn from_native_anchors(impulse_timer_38: Timer, scroller_timer: Timer, rect: Rect) -> Self {
        Self {
            scroller: Scroller {
                bits_04: 0,
                edge_08: 0,
                keys_0c: 0,
                direction_10: None,
                timer_18: scroller_timer,
                accumulated_20: 0,
                millis_24: 0,
            },
            rotate: Rotate { rate_04: 0, last_08: [0, 0], first_10: None },
            active_c0: None,
            ui_cursor_58: [0, 0],
            input: InputCursor { position_0c: [0, 0], rect_14: rect, inside_24: false },
            impulses: Vec::new(),
            impulse_timer_38,
        }
    }
    ///UI+58 cursor as copied by 5BF2B0; returns whether it changed.
    pub fn set_ui_cursor_5bf2b0(&mut self, position: [i32; 2]) -> bool {
        let moved = self.ui_cursor_58 != position;
        self.ui_cursor_58 = position;
        moved
    }
    ///The KKC_Input poll and 5BF2B0 copy that ran on every earlier native UI
    ///frame: a modern owner created mid-session first adopts the current
    ///cursor so stale constructor zeros are not taken as a screen edge.
    pub fn adopt_cursor(&mut self, client: [i32; 2]) {
        self.input.poll_59ad30(client);
        self.ui_cursor_58 = self.input.position_0c;
    }
    ///KKC_Input +24 inside flag (59AD30 result) without a new position.
    pub fn set_input_inside_24(&mut self, inside: bool) {
        self.input.inside_24 = inside;
    }
    ///5CD410 scroller mouse-move: recompute bits; returns false.
    pub fn scroller_mouse_move_5cd410(&mut self, ui: &mut impl Ui, p: Precision) -> Result<()> {
        let bits = self.edge_5ccc90(ui)?;
        self.direction_5cc7d0(bits, p)
    }
    pub fn scroller(&self) -> &Scroller {
        &self.scroller
    }
    pub fn rotate(&self) -> &Rotate {
        &self.rotate
    }
    pub fn active(&self) -> Option<Slot> {
        self.active_c0
    }
    pub fn impulses(&self) -> &[Impulse] {
        &self.impulses
    }
    pub fn input_cursor(&self) -> &InputCursor {
        &self.input
    }
    pub fn ui_cursor(&self) -> [i32; 2] {
        self.ui_cursor_58
    }
    ///59AEB0 SetRect (client resize). The clamp/inside test uses it next poll.
    pub fn set_rect_59aeb0(&mut self, rect: Rect) {
        self.input.rect_14 = rect;
    }

    ///5E0570 -> 5C1FD0: exit the previous C0 state, then publish/enter.
    fn activate_5e0570(&mut self, slot: Slot) {
        if let Some(old) = self.active_c0.take() {
            self.exit(old);
        }
        self.active_c0 = Some(slot); //enter: 4B2830 RET for both camera states
    }
    ///5E0590: only the published C0 state exits.
    fn deactivate_5e0590(&mut self, slot: Slot) {
        if self.active_c0 == Some(slot) {
            self.active_c0 = None;
            self.exit(slot);
        }
    }
    fn exit(&mut self, slot: Slot) {
        match slot {
            //5CC7C0 clears +C,+8,+4 (direction/accumulator/timer remain).
            Slot::Scroller => {
                self.scroller.keys_0c = 0;
                self.scroller.edge_08 = 0;
                self.scroller.bits_04 = 0;
            }
            Slot::Rotate => {} //4B2830
        }
    }

    ///5CCC90 (this = scroller). Writes +8 first; returns +C|+8.
    fn edge_5ccc90(&mut self, ui: &mut impl Ui) -> Result<u32> {
        self.scroller.edge_08 = 0;
        if !ui.vworld_ad()? || !ui.vworld_flag_24_bit19()? {
            return Ok(0);
        }
        let r = self.input.rect_14;
        let [x, y] = self.ui_cursor_58;
        if x.wrapping_sub(r.left) < 1 {
            self.scroller.edge_08 = SCROLL_LEFT
        } else if r.right.wrapping_sub(x) < 1 {
            self.scroller.edge_08 = SCROLL_RIGHT
        }
        if y.wrapping_sub(r.top) < 1 {
            self.scroller.edge_08 |= SCROLL_UP;
            return Ok(self.scroller.keys_0c | self.scroller.edge_08);
        }
        if r.bottom.wrapping_sub(y) < 1 {
            self.scroller.edge_08 |= SCROLL_DOWN
        }
        Ok(self.scroller.keys_0c | self.scroller.edge_08)
    }
    ///5CC7D0. Each axis adds/subtracts 1.0f with its own binary32 store.
    fn direction_5cc7d0(&mut self, bits: u32, p: Precision) -> Result<()> {
        if bits == 0 {
            self.deactivate_5e0590(Slot::Scroller);
            return Ok(());
        }
        let s = &mut self.scroller;
        if s.bits_04 == bits {
            return Ok(());
        }
        if s.bits_04 == 0 {
            s.accumulated_20 = 0
        }
        s.bits_04 = bits;
        let mut d = [ZERO, ZERO];
        if bits & SCROLL_LEFT != 0 {
            d[0] = f(&sub(&b(d[0])?, &b(ONE)?, p)?)?
        }
        if bits & SCROLL_RIGHT != 0 {
            d[0] = f(&add(&b(d[0])?, &b(ONE)?, p)?)?
        }
        if bits & SCROLL_UP != 0 {
            d[1] = f(&add(&b(d[1])?, &b(ONE)?, p)?)?
        }
        if bits & SCROLL_DOWN != 0 {
            d[1] = f(&sub(&b(d[1])?, &b(ONE)?, p)?)?
        }
        s.direction_10 = Some(d);
        Ok(())
    }
    ///5CD560 BeginScroll(bit).
    pub fn begin_scroll_5cd560(
        &mut self,
        bit: u32,
        ui: &mut impl Ui,
        qpc: &mut impl Qpc,
        p: Precision,
    ) -> Result<()> {
        if !ui.vworld_ad()? || !ui.vworld_flag_24_bit19()? {
            return Ok(());
        }
        self.scroller.keys_0c |= bit;
        if self.active_c0 != Some(Slot::Scroller) {
            self.activate_5e0570(Slot::Scroller);
            self.scroller.bits_04 = 0;
            self.scroller.timer_18 = Timer::reset_59ab60(qpc)?;
        }
        let bits = self.edge_5ccc90(ui)?;
        self.direction_5cc7d0(bits, p)
    }
    ///5CCD30 EndScroll(bit): inactive scrollers keep their key bit.
    pub fn end_scroll_5ccd30(&mut self, bit: u32, ui: &mut impl Ui, p: Precision) -> Result<()> {
        if self.active_c0 != Some(Slot::Scroller) {
            return Ok(());
        }
        self.scroller.keys_0c &= !bit;
        let bits = self.edge_5ccc90(ui)?;
        self.direction_5cc7d0(bits, p)
    }

    ///Mouse-move dispatch 5C1AF0 for the represented states (called only when
    ///5BF2B0 reported a changed cursor). The game state's +10 (5E04F0) runs
    ///first; a consuming UI+60 widget stops the chain.
    fn mouse_move_5c1af0(
        &mut self,
        ui: &mut impl Ui,
        camera: &mut impl Camera,
        trace: &mut Vec<Trace>,
        p: Precision,
    ) -> Result<()> {
        if ui.ui_widget_60_consumes_move()? {
            return Ok(());
        }
        match self.active_c0 {
            Some(Slot::Scroller) => self.scroller_mouse_move_5cd410(ui, p),
            Some(Slot::Rotate) => self.rotate_move_5ccf00(camera, trace, p),
            None => Ok(()),
        }
    }
    ///5CCF00: horizontal cursor delta times rate turns the camera about its
    ///current target, then 6FF200 restores the captured target XY.
    pub fn rotate_move_5ccf00(
        &mut self,
        camera: &mut impl Camera,
        trace: &mut Vec<Trace>,
        p: Precision,
    ) -> Result<()> {
        let r = &mut self.rotate;
        if r.first_10.ok_or("rotate state +10 has no reached native writer")? {
            r.last_08 = self.ui_cursor_58;
            r.first_10 = Some(false);
        }
        let [x, y] = self.ui_cursor_58;
        let delta = Binary::from_i32(x.wrapping_sub(r.last_08[0]));
        let value = mul(&delta, &b(r.rate_04)?, p)?;
        if !value.is_zero() {
            let target = [camera.word(0x24c)?, camera.word(0x250)?];
            let yaw = f(&sub(&b(camera.word(0x2bc)?)?, &value, p)?)?;
            let pitch = camera.word(0x2b8)?;
            trace.push(Trace::Rotate { yaw, pitch });
            camera.rotate_6feba0(yaw, pitch)?;
            trace.push(Trace::LookAt { xy: target });
            camera.look_at_6ff200(target)?;
        }
        self.rotate.last_08 = [x, y];
        Ok(())
    }
    ///5C21C0 BeginCameraRotate -> 5CCEB0.
    pub fn begin_rotate_5c21c0(
        &mut self,
        vars: &mut impl Variables,
        camera: &mut impl Camera,
        ui: &mut impl Ui,
        p: Precision,
    ) -> Result<()> {
        if vars.metagame_40cee0()? {
            return Ok(());
        }
        if vars.uvar(0xb4)? & 0xff == 0 && !camera.developer_6fe9c0()? {
            return Ok(());
        }
        let rate = f(&mul(&b(vars.uvar(0x38)?)?, &b(DEG2RAD)?, p)?)?;
        //5CCEB0
        if !ui.vworld_ad()? || !ui.vworld_flag_24_bit19()? {
            return Ok(());
        }
        if b(rate)?.is_zero() {
            return Ok(());
        }
        if self.active_c0 != Some(Slot::Rotate) {
            self.activate_5e0570(Slot::Rotate);
        }
        self.rotate.rate_04 = rate;
        self.rotate.first_10 = Some(true);
        Ok(())
    }
    ///5C1370 -> 5CC870 EndCameraRotate.
    pub fn end_rotate_5cc870(&mut self) {
        if self.active_c0 == Some(Slot::Rotate) {
            self.deactivate_5e0590(Slot::Rotate);
            self.rotate.rate_04 = 0;
        }
    }

    ///RegisterImpulse 5C3F00 -> 599DF0 (unknown names reach 599DF0(0): no-op).
    pub fn register_impulse_5c3f00(&mut self, name: &str, qpc: &mut impl Qpc, p: Precision) -> Result<()> {
        let Some(function) = ImpulseFn::lookup_5c36a0(name) else { return Ok(()) };
        if self.impulses.iter().any(|i| i.function == function) {
            return Ok(());
        }
        let last = f(&self.impulse_timer_38.elapsed_59aad0(qpc, p)?)?;
        self.impulses.push(Impulse { function, last });
        Ok(())
    }
    ///599930 from 40DA30 OnDeactivate: impulse count = 0.
    pub fn clear_impulses_599930(&mut self) {
        self.impulses.clear();
    }
    ///UnregisterImpulse 5C3F90 -> 5998F0 -> 5996E0 (ordered removal).
    pub fn unregister_impulse_5c3f90(&mut self, name: &str) {
        let Some(function) = ImpulseFn::lookup_5c36a0(name) else { return };
        if let Some(i) = self.impulses.iter().position(|v| v.function == function) {
            self.impulses.remove(i);
        }
    }

    ///One native UI frame (5C92F0 camera subset, in original order). `client`
    ///is the current cursor position in native client pixels (GetCursorPos
    ///relative to the window client) used by the 59AD30 poll.
    pub fn frame_5c92f0(
        &mut self,
        client: Option<[i32; 2]>,
        ui: &mut impl Ui,
        vars: &mut impl Variables,
        camera: &mut impl Camera,
        qpc: &mut impl Qpc,
        p: Precision,
    ) -> Result<Vec<Trace>> {
        let mut trace = Vec::new();
        if ui.application_active()? {
            //5BF2B0: copy the KKC_Input cursor polled at the previous tick.
            let moved = self.ui_cursor_58 != self.input.position_0c;
            self.ui_cursor_58 = self.input.position_0c;
            if moved {
                self.mouse_move_5c1af0(ui, camera, &mut trace, p)?;
            }
            //599940: every registered impulse, then tail 59AD30.
            let mut i = 0;
            while i < self.impulses.len() {
                let now = f(&self.impulse_timer_38.elapsed_59aad0(qpc, p)?)?;
                let dt = impulse_dt_5995f0(self.impulses[i].last, now, p)?;
                let function = self.impulses[i].function;
                run_impulse(function, dt, vars, camera, &mut trace, p)?;
                self.impulses[i].last = now;
                i += 1;
            }
            match client {
                Some(position) => self.input.poll_59ad30(position),
                None => self.input.inside_24 = false,
            }
        }
        //UI-state +8 updates: game state 5CD6B0, then the C0 state.
        self.game_state_update_5cd6b0(ui, qpc, p)?;
        if self.active_c0 == Some(Slot::Scroller) {
            self.scroller_update_5cd320(ui, vars, camera, qpc, &mut trace, p)?;
        }
        Ok(trace)
    }
    ///5CD6B0 (A73CD0+8). VWorld+13C hover pick (vfunc+38) belongs to the
    ///selection owner and is not represented here.
    pub fn game_state_update_5cd6b0(&mut self, ui: &mut impl Ui, qpc: &mut impl Qpc, p: Precision) -> Result<()> {
        if !ui.vworld_ad()? {
            return Ok(());
        }
        if self.edge_activation_5cd420(ui)? {
            self.activate_5e0570(Slot::Scroller);
            self.scroller.bits_04 = 0;
            self.scroller.timer_18 = Timer::reset_59ab60(qpc)?;
            let bits = self.edge_5ccc90(ui)?;
            self.direction_5cc7d0(bits, p)?;
        }
        Ok(())
    }
    ///5CD420.
    fn edge_activation_5cd420(&mut self, ui: &mut impl Ui) -> Result<bool> {
        if ui.app_byte_48()? && !ui.app_byte_49()? {
            return Ok(false);
        }
        if self.active_c0 == Some(Slot::Scroller) {
            return Ok(false);
        }
        if !ui.vworld_hovered_5efa80()? {
            return Ok(false);
        }
        if self.edge_5ccc90(ui)? == 0 {
            return Ok(false);
        }
        Ok(self.active_c0 != Some(Slot::Rotate))
    }
    ///5CD320 scroller update (A4DA84+8): QPC microseconds since the previous
    ///reset, clamped to [10000,100000], 700500 with the accelerated direction,
    ///then 59AB60 reset (also when the pan branch is skipped).
    pub fn scroller_update_5cd320(
        &mut self,
        ui: &mut impl Ui,
        vars: &mut impl Variables,
        camera: &mut impl Camera,
        qpc: &mut impl Qpc,
        trace: &mut Vec<Trace>,
        p: Precision,
    ) -> Result<()> {
        if !ui.vworld_flag_24_bit19()? || !ui.visual_map_bc()? {
            self.deactivate_5e0590(Slot::Scroller);
            return Ok(());
        }
        if camera.byte_214()? == 0
            && ui.application_active()?
            && self.scroller.bits_04 != 0
            && self.input.inside_24
        {
            let elapsed = f(&self.scroller.timer_18.elapsed_59aad0(qpc, p)?)?;
            let dt = clamp_401290(elapsed, TICK_MIN, TICK_MAX)?;
            self.scroller.millis_24 = ftol(&mul(&b(dt)?, &b(MILLI)?, p)?)?;
            let direction = self
                .scroller
                .direction_10
                .ok_or("scroller +10 direction has no reached 5CC7D0 writer")?;
            let speed = [vars.uvar(0x18)?, vars.uvar(0x1c)?, vars.uvar(0x20)?];
            let (accumulated, out) =
                accelerate_5ccc20(self.scroller.accumulated_20, direction, dt, speed, p)?;
            self.scroller.accumulated_20 = accumulated;
            trace.push(Trace::Pan { millis: self.scroller.millis_24, direction: out });
            camera.pan_700500(self.scroller.millis_24, out)?;
        }
        self.scroller.timer_18 = Timer::reset_59ab60(qpc)?;
        Ok(())
    }
}
impl Scroller {
    pub fn bits(&self) -> u32 {
        self.bits_04
    }
    pub fn keys(&self) -> u32 {
        self.keys_0c
    }
    pub fn edge(&self) -> u32 {
        self.edge_08
    }
    pub fn direction(&self) -> Option<[u32; 2]> {
        self.direction_10
    }
    pub fn accumulated(&self) -> u32 {
        self.accumulated_20
    }
    pub fn millis(&self) -> u32 {
        self.millis_24
    }
    pub fn timer(&self) -> Timer {
        self.timer_18
    }
}
impl Rotate {
    pub fn rate(&self) -> u32 {
        self.rate_04
    }
    pub fn last(&self) -> [i32; 2] {
        self.last_08
    }
}

///401290 clamp of a binary32 local: lower bound first (ordered compares).
pub fn clamp_401290(value: u32, low: u32, high: u32) -> Result<u32> {
    let v = b(value)?;
    if v.compare(&b(low)?) == Ordering::Less {
        return Ok(low);
    }
    if b(high)?.compare(&v) == Ordering::Less {
        return Ok(high);
    }
    Ok(value)
}
///5CCC20: returns (+20 accumulator store, output direction). The FST of the
///sum keeps the extended value for the speed product; the max compare is
///against the binary32 UVar and replaces only when strictly greater.
///`speed` = [UVar18 ScrollSpeed, UVar1C Acceleration, UVar20 SpeedMax].
pub fn accelerate_5ccc20(
    accumulated: u32,
    direction: [u32; 2],
    dt: u32,
    speed: [u32; 3],
    p: Precision,
) -> Result<(u32, [u32; 2])> {
    let sum = add(&b(dt)?, &b(accumulated)?, p)?;
    let stored = f(&sum)?;
    let mut v = mul(&mul(&sum, &b(MICRO)?, p)?, &b(speed[1])?, p)?;
    v = add(&v, &b(speed[0])?, p)?;
    if v.compare(&b(speed[2])?) == Ordering::Greater {
        v = b(speed[2])?;
    }
    let factor = div(&v, &b(speed[0])?, p)?;
    let x = f(&mul(&factor, &b(direction[0])?, p)?)?;
    let y = f(&mul(&factor, &b(direction[1])?, p)?)?;
    Ok((stored, [x, y]))
}
///5995F0: `now < last` -> 10000; extended (now-last) > 100000 -> 100000;
///binary32 (now-last) < 10000 -> 10000; otherwise the binary32 difference.
pub fn impulse_dt_5995f0(last: u32, now: u32, p: Precision) -> Result<u32> {
    let n = b(now)?;
    if n.compare(&b(last)?) == Ordering::Less {
        return Ok(TICK_MIN);
    }
    let d = sub(&n, &b(last)?, p)?;
    let stored = f(&d)?;
    if d.compare(&b(TICK_MAX)?) == Ordering::Greater {
        return Ok(TICK_MAX);
    }
    if b(stored)?.compare(&b(TICK_MIN)?) == Ordering::Less {
        return Ok(TICK_MIN);
    }
    Ok(stored)
}
fn impulse_millis(dt: u32, p: Precision) -> Result<u32> {
    ftol(&mul(&b(dt)?, &b(MILLI)?, p)?)
}
///Impulse callbacks of table B284A4 (`dt` microseconds, binary32).
pub fn run_impulse(
    function: ImpulseFn,
    dt: u32,
    vars: &mut impl Variables,
    camera: &mut impl Camera,
    trace: &mut Vec<Trace>,
    p: Precision,
) -> Result<()> {
    match function {
        ImpulseFn::YawLeft => yaw_5c2090(impulse_millis(dt, p)?, ONE, vars, camera, trace, p),
        ImpulseFn::YawRight => yaw_5c2090(impulse_millis(dt, p)?, NEG_ONE, vars, camera, trace, p),
        ImpulseFn::PitchUp => pitch_5c20f0(impulse_millis(dt, p)?, ONE, vars, camera, trace, p),
        ImpulseFn::PitchDown => pitch_5c20f0(impulse_millis(dt, p)?, NEG_ONE, vars, camera, trace, p),
        ImpulseFn::ZoomIn => zoom_rate_5c29b0(dt, NEG_MILLI, vars, camera, trace, p),
        ImpulseFn::ZoomOut => zoom_rate_5c29b0(dt, MILLI, vars, camera, trace, p),
    }
}
///5C2090: yaw += ((ScaleYaw*Speed)*ms*0.001)*(dir*DEG2RAD), then 700440.
pub fn yaw_5c2090(
    millis: u32,
    direction: u32,
    vars: &mut impl Variables,
    camera: &mut impl Camera,
    trace: &mut Vec<Trace>,
    p: Precision,
) -> Result<()> {
    if camera.byte_214()? != 0 {
        return Ok(());
    }
    let rate = mul(&b(vars.uvar(0x30)?)?, &b(vars.uvar(0x18)?)?, p)?;
    let amount = mul(&mul(&rate, &unsigned(millis, p)?, p)?, &b(MILLI)?, p)?;
    let turn = mul(&amount, &mul(&b(direction)?, &b(DEG2RAD)?, p)?, p)?;
    let yaw = f(&add(&turn, &b(camera.word(0x2bc)?)?, p)?)?;
    yaw_700440(yaw, camera, trace)
}
///5C20F0: pitch in degrees clamped to the AVar50 range, then raised to
///AVar54, all on the extended register, then 7004A0.
pub fn pitch_5c20f0(
    millis: u32,
    direction: u32,
    vars: &mut impl Variables,
    camera: &mut impl Camera,
    trace: &mut Vec<Trace>,
    p: Precision,
) -> Result<()> {
    if camera.byte_214()? != 0 {
        return Ok(());
    }
    let rate = mul(&b(vars.uvar(0x2c)?)?, &b(vars.uvar(0x18)?)?, p)?;
    let amount = mul(&mul(&rate, &unsigned(millis, p)?, p)?, &b(MILLI)?, p)?;
    let turn = mul(&amount, &mul(&b(direction)?, &b(DEG2RAD)?, p)?, p)?;
    let mut degrees = mul(&add(&turn, &b(camera.word(0x2b8)?)?, p)?, &b(RAD2DEG)?, p)?;
    let [low, high] = vars.avar_range(0x50)?;
    if degrees.compare(&b(low)?) == Ordering::Less {
        degrees = b(low)?
    } else if degrees.compare(&b(high)?) == Ordering::Greater {
        degrees = b(high)?
    }
    let minimum = vars.avar(0x54)?;
    if degrees.compare(&b(minimum)?) == Ordering::Less {
        degrees = b(minimum)?
    }
    let pitch = f(&mul(&degrees, &b(DEG2RAD)?, p)?)?;
    pitch_7004a0(pitch, camera, trace)
}
///5C29B0/5C2A10: 7010F0(f32(u32(ms)*UVar34*UVar18*(-/+0.001))).
pub fn zoom_rate_5c29b0(
    dt: u32,
    sign_milli: u32,
    vars: &mut impl Variables,
    camera: &mut impl Camera,
    trace: &mut Vec<Trace>,
    p: Precision,
) -> Result<()> {
    if camera.byte_214()? != 0 {
        return Ok(());
    }
    let millis = impulse_millis(dt, p)?;
    let mut v = mul(&unsigned(millis, p)?, &b(vars.uvar(0x34)?)?, p)?;
    v = mul(&v, &b(vars.uvar(0x18)?)?, p)?;
    let delta = f(&mul(&v, &b(sign_milli)?, p)?)?;
    zoom_delta_7010f0(delta, camera, trace, p)
}
///700440: FUCOMPP equality (unordered proceeds), byte214, then 6FF220.
pub fn yaw_700440(yaw: u32, camera: &mut impl Camera, trace: &mut Vec<Trace>) -> Result<()> {
    if b(camera.word(0x2bc)?)?.compare(&b(yaw)?) == Ordering::Equal || camera.byte_214()? != 0 {
        return Ok(());
    }
    let center = [camera.word(0x24c)?, camera.word(0x250)?, camera.word(0x254)?];
    let (pitch, zoom) = (camera.word(0x2b8)?, camera.word(0x208)?);
    trace.push(Trace::Pose { center, yaw, pitch, zoom });
    camera.pose_6ff220(center, yaw, pitch, zoom)
}
///7004A0: as 700440 for pitch.
pub fn pitch_7004a0(pitch: u32, camera: &mut impl Camera, trace: &mut Vec<Trace>) -> Result<()> {
    if b(camera.word(0x2b8)?)?.compare(&b(pitch)?) == Ordering::Equal || camera.byte_214()? != 0 {
        return Ok(());
    }
    let center = [camera.word(0x24c)?, camera.word(0x250)?, camera.word(0x254)?];
    let (yaw, zoom) = (camera.word(0x2bc)?, camera.word(0x208)?);
    trace.push(Trace::Pose { center, yaw, pitch, zoom });
    camera.pose_6ff220(center, yaw, pitch, zoom)
}
///7010F0: 7005F0(f32(delta + zoom208)).
pub fn zoom_delta_7010f0(
    delta: u32,
    camera: &mut impl Camera,
    trace: &mut Vec<Trace>,
    p: Precision,
) -> Result<()> {
    let value = f(&add(&b(delta)?, &b(camera.word(0x208)?)?, p)?)?;
    trace.push(Trace::Zoom { value });
    camera.zoom_7005f0(value)
}
///43BC20 ZoomMin (+20C) / 43BC50 ZoomMax (+210): 7005F0(f32(1/(1/limit))).
pub fn zoom_limit_43bc20(
    offset: u32,
    camera: &mut impl Camera,
    trace: &mut Vec<Trace>,
    p: Precision,
) -> Result<()> {
    if !matches!(offset, 0x20c | 0x210) {
        return Err("ZoomMin/ZoomMax read only Camera+20C/+210".into());
    }
    let one = b(ONE)?;
    let value = f(&div(&one, &div(&one, &b(camera.word(offset)?)?, p)?, p)?)?;
    trace.push(Trace::Zoom { value });
    camera.zoom_7005f0(value)
}
///5EFCCC: `(float)(int)(short)delta / 1200.0f` at PC, passed as a double to
///vswprintf("deltazoom %f"). Returns the binary64 argument bits.
pub fn wheel_argument_5efccc(delta: i16, p: Precision) -> Result<u64> {
    let q = div(&Binary::from_i32(i32::from(delta)), &b(WHEEL_DIVISOR)?, p)?;
    aa_numeric::crt_pow::store_binary64(&q).map_err(|e| format!("{e:?}"))
}
///43BC80 DeltaZoom after 426330: the `_wtof` value stored to binary32 must
///lie in [-1,1] (else the native command throws and nothing changes); then
///FMUL -1.0, FSTP and 7010F0.
pub fn delta_zoom_43bc80(
    parsed: u32,
    camera: &mut impl Camera,
    trace: &mut Vec<Trace>,
    p: Precision,
) -> Result<()> {
    let v = b(parsed)?;
    if v.compare(&b(NEG_ONE)?) == Ordering::Less || v.compare(&b(ONE)?) == Ordering::Greater {
        return Err("DeltaZoom argument outside the native [-1,1] range (426330 throws)".into());
    }
    let delta = f(&mul(&v, &b(DELTA_ZOOM_SIGN)?, p)?)?;
    zoom_delta_7010f0(delta, camera, trace, p)
}

#[cfg(test)]
#[path = "native_camera_input_tests.rs"]
mod tests;
