//! Native mouse event production: `WndProc` 0x6FD4A0 and
//! `KKC_Input::AddMouseEvent` 0x59A6E0 (Input.cpp), with the event ray of
//! 0x5C2770 and the terrain point of 0x71D840.
//!
//! The 0x30-byte event (constructor 0x5997F0) is the only input record the
//! UI states, the UIWorldState and the order code read. Its world fields are
//! computed once, at the time the message is queued, from the camera current
//! at that moment ([`Event::world`] is the terrain hit, [`Event::origin`] /
//! [`Event::direction`] the camera ray).
use crate::view_pointer::{self, CameraWords};
use aa_numeric::Precision;

pub type Result<T> = std::result::Result<T, String>;

/// Event codes (`event+8`), range-checked 0x100..=0x110 by 0x59A6E0.
pub mod code {
    pub const LEFT: u16 = 0x100;
    pub const RIGHT: u16 = 0x101;
    pub const MIDDLE: u16 = 0x102;
    /// Double clicks: the button code + 8 (WM_xBUTTONDBLCLK, 0x6FD4A0).
    pub const LEFT_DOUBLE: u16 = 0x108;
    pub const RIGHT_DOUBLE: u16 = 0x109;
    pub const MIDDLE_DOUBLE: u16 = 0x10a;
    pub const WHEEL: u16 = 0x110;
}
/// Event flags (`event+4`).
pub mod flag {
    /// Button pressed (WndProc passes 1 for downs, double clicks and wheel).
    pub const PRESSED: u32 = 1;
    /// Button released (0x6FD3D0 passes 2).
    pub const RELEASED: u32 = 2;
    /// Input+0x28 bit0, set by VK_MENU (0x59A5C0).
    pub const ALT: u32 = 0x10;
    /// Input+0x28 bit1, set by VK_SHIFT.
    pub const SHIFT: u32 = 0x20;
    /// Input+0x28 bit2, set by VK_CONTROL.
    pub const CTRL: u32 = 0x40;
    /// Always ORed in by 0x59A6E0.
    pub const MOUSE: u32 = 0x80;
}

/// The native 0x30-byte input event. Float fields are raw binary32 words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    /// +0, +2: client pixel (signed 16-bit, as WndProc's LOWORD/HIWORD).
    pub x: i16,
    pub y: i16,
    /// +4
    pub flags: u32,
    /// +8
    pub code: u16,
    /// +A: wheel delta (GET_WHEEL_DELTA_WPARAM) for 0x110, otherwise 0.
    pub delta: i16,
    /// +C: terrain point 0x71D840 wrote, or the B43CF8 vector it started from.
    pub world: [u32; 3],
    /// +18: camera eye (camera+0x90).
    pub origin: [u32; 3],
    /// +24: camera ray direction (0x6FE590).
    pub direction: [u32; 3],
}
impl Event {
    pub fn pressed(&self) -> bool {
        self.flags & flag::PRESSED != 0
    }
    pub fn released(&self) -> bool {
        self.flags & flag::RELEASED != 0
    }
    /// The `(code == 0x100 && flags&1) || code == 0x108` test repeated by
    /// 0x5C3BC0, 0x5CD190, 0x5CD220 and 0x43C550.
    pub fn left_press(&self) -> bool {
        (self.code == code::LEFT && self.pressed()) || self.code == code::LEFT_DOUBLE
    }
    /// `code == 0x101 && flags&1`.
    pub fn right_press(&self) -> bool {
        self.code == code::RIGHT && self.pressed()
    }
    /// `code == 0x100 && flags&2`: the left release that ends click/box states.
    pub fn left_release(&self) -> bool {
        self.code == code::LEFT && self.released()
    }
    /// Shift (0x20) or Ctrl (0x40): the "add/toggle" selection modifier of
    /// 0x5CC960 and 0x5CCB70.
    pub fn add_modifier(&self) -> bool {
        self.flags & (flag::SHIFT | flag::CTRL) != 0
    }
}

/// What 0x59A6E0 and 0x5C2770 read from the application. Every method is a
/// required live read; none has a successful default.
pub trait Sources {
    type Camera: CameraWords;
    /// DAT_00B396C0; events are dropped while it equals 3.
    fn input_block_b396c0(&mut self) -> Result<u32>;
    /// 0x404EA0 "Mouse code %d out of range %d-%d." (Input.cpp line 0x107).
    /// The event is still queued afterwards.
    fn code_out_of_range(&mut self, code: u16) -> Result<()>;
    /// `*B40048`: the renderer's current camera.
    fn camera(&mut self) -> Result<&Self::Camera>;
    /// DAT_00B396D0 / DAT_00B396D4: the signed viewport width and height.
    fn viewport_b396d0(&mut self) -> Result<(i32, i32)>;
    /// The process vector at B43CF8..B43D00 copied before the terrain query.
    fn seed_b43cf8(&mut self) -> Result<[u32; 3]>;
    /// `B36518 != 0 && *(B36518+0xBC) != 0`: a TerrainMap exists and is ready.
    fn terrain_ready_b36518(&mut self) -> Result<bool>;
    /// 0x71D840(eye, direction, &point, NULL, 0) on B36518. It must leave
    /// `point` untouched where the native routine does not write it; AL is
    /// ignored by 0x59A6E0.
    fn terrain_71d840(&mut self, eye: [u32; 3], direction: [u32; 3], point: &mut [u32; 3]) -> Result<()>;
}

/// The KKC_Input fields 0x59A5C0/0x59A6E0 maintain (B39B18).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    /// +0x28: bit0 Alt, bit1 Shift, bit2 Ctrl.
    modifiers_28: u32,
    /// +0x44 + code*4: the last flags of each key/button code.
    states_44: Vec<u32>,
}
impl Input {
    /// The constructor-zero state of every field the two routines touch.
    pub fn new() -> Self {
        Self { modifiers_28: 0, states_44: vec![0; 0x111] }
    }
    pub fn modifiers_28(&self) -> u32 {
        self.modifiers_28
    }
    pub fn state_44(&self, code: u16) -> u32 {
        self.states_44.get(code as usize).copied().unwrap_or(0)
    }
    /// The modifier part of `AddKeyboardEvent` 0x59A5C0 (lines 0x59A5F8..
    /// 0x59A64F): VK_SHIFT/VK_CONTROL/VK_MENU set or clear Input+0x28 bit
    /// 1/2/0 from the released flag (bit1). The key event itself is routed by
    /// the hotkey path, which is outside this module.
    pub fn modifier_key_59a5c0(&mut self, vk: u16, flags: u32) {
        let bit = match vk {
            0x10 => 2,
            0x11 => 4,
            0x12 => 1,
            _ => return,
        };
        if flags & flag::RELEASED != 0 {
            self.modifiers_28 &= !bit;
        } else {
            self.modifiers_28 |= bit;
        }
    }
    /// 0x59A6E0. Returns `None` when the native routine drops the event.
    pub fn add_mouse_event_59a6e0(
        &mut self,
        sources: &mut impl Sources,
        code: u16,
        flags: u32,
        x: i16,
        y: i16,
        delta: i16,
        precision: Precision,
    ) -> Result<Option<Event>> {
        if sources.input_block_b396c0()? == 3 {
            return Ok(None);
        }
        let mut flags = flags | flag::MOUSE;
        if !(0x100..=0x110).contains(&code) {
            sources.code_out_of_range(code)?;
        }
        if self.modifiers_28 & 1 != 0 {
            flags |= flag::ALT;
        }
        if self.modifiers_28 & 2 != 0 {
            flags |= flag::SHIFT;
        }
        if self.modifiers_28 & 4 != 0 {
            flags |= flag::CTRL;
        }
        if flags & flag::RELEASED != 0 && (0x100..=0x107).contains(&code) {
            // 0x59A77E: the matching double-click slot code+8 is cleared.
            self.store(code.wrapping_add(8), 0)?;
        }
        self.store(code, flags)?;
        let (origin, direction) = ray_5c2770(sources, x, y, precision)?;
        let mut world = sources.seed_b43cf8()?;
        if sources.terrain_ready_b36518()? {
            sources.terrain_71d840(origin, direction, &mut world)?;
        }
        Ok(Some(Event { x, y, flags, code, delta, world, origin, direction }))
    }
    fn store(&mut self, code: u16, flags: u32) -> Result<()> {
        // The native table is indexed without a bound; codes outside the
        // 0x111 entries 0x59A6E0 can reach would write other Input fields.
        let slot = self
            .states_44
            .get_mut(code as usize)
            .ok_or_else(|| format!("Input+0x44 slot for code {code:#x} is outside the ported table"))?;
        *slot = flags;
        Ok(())
    }
}
impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

/// 0x5C2770: eye = camera+0x90..0x98; the pixel goes through binary32 and
/// `_ftol` (exact for 16-bit pixels), then Y/height and X/width are each
/// stored binary32 (Y first) and passed to 0x6FE590.
pub fn ray_5c2770(
    sources: &mut impl Sources,
    x: i16,
    y: i16,
    precision: Precision,
) -> Result<([u32; 3], [u32; 3])> {
    let (width, height) = sources.viewport_b396d0()?;
    let normalized = view_pointer::normalized_pixel([i32::from(x), i32::from(y)], width, height, precision)?;
    let camera = sources.camera()?;
    let origin = [camera.word(0x90)?, camera.word(0x94)?, camera.word(0x98)?];
    let direction = view_pointer::ray_6fe590(camera, normalized, precision)?;
    Ok((origin, direction))
}

/// The client rectangle clamp 0x40D280 applied by 0x6FD3D0 to release points
/// (B39690+0x40 width, +0x44 height).
pub fn clamp_release_40d280(point: [i32; 2], client: [i32; 2]) -> [i32; 2] {
    let mut p = point;
    for axis in 0..2 {
        if p[axis] < 0 {
            p[axis] = 0;
        }
        if p[axis] >= client[axis] {
            p[axis] = client[axis] - 1;
        }
    }
    p
}

#[cfg(test)]
#[path = "event_tests.rs"]
mod tests;
