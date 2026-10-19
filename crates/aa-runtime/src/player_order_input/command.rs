//! The right-click order of the game interface: the KKC_GActorCommand the
//! default UI state builds (0x620760), the facing drag that may follow
//! (state GameInterface+0xE4, 0x621230 / 0x688AA0 / 0x688990 / 0x6210B0),
//! and the send path KKC_TellSelectedCommandOrder (0x60C8A0, CanSend
//! 0x61E150 -> UIWorldState 0x436950, Send 0x61EEF0 -> 0x61E960 request 84).
//!
//! The command object (ctor 0x4D9C80, 0x40 bytes, all fields zero) is kept
//! as the wire it serializes to ([`CommandWire`], writer 0x61E6B0): +0x0C
//! flags, +0x10 word (target actor handle), +0x18 position, +0x20 auxiliary
//! (facing), +0x28 scalar, +0x3C trailing byte. No explicit command (+0x08,
//! flag bit 6) is set by these paths: the simulation chooses it
//! (KKC_GActorCommand::GuessCommandData 0x4DA570).
use super::x87::{self, b, st};
use aa_numeric::Precision;
use aa_orders::commands::CommandWire;

pub type Result<T> = std::result::Result<T, String>;

/// 0x4D9C80 plus the stores 0x620760 makes before choosing a target:
/// +0x3C = Shift or Ctrl (Input+0x28 bits 1|2), 0x4D9B30(Alt) sets or
/// clears flag 0x400, flag 0x10 with scalar 0.
pub fn new_right_click_command(modifiers_28: u32) -> CommandWire {
    let mut wire = CommandWire::default();
    wire.trailing = modifiers_28 & 6 != 0;
    if modifiers_28 & 1 != 0 {
        wire.flags |= 0x400;
    } else {
        wire.flags &= !0x400;
    }
    wire.flags |= 0x10;
    wire.scalar = 0;
    wire
}
/// 0x4D9BF0(actor): flag 1 and +0x10 = `actor+0x14` (NULL clears both).
pub fn set_target_4d9bf0(wire: &mut CommandWire, actor_handle_14: Option<u32>) {
    match actor_handle_14 {
        Some(handle) => {
            wire.flags |= 1;
            wire.word = handle;
        }
        None => {
            wire.flags &= !1;
            wire.word = 0;
        }
    }
}
/// The location stores of 0x620957..0x620971: flag 4 and +0x18/+0x1C.
pub fn set_location(wire: &mut CommandWire, xy: [u32; 2]) {
    wire.flags |= 4;
    wire.position = xy;
}
/// Flag 8 and +0x20/+0x24 (0x620989 minimap / 0x6210DD facing finish).
pub fn set_facing(wire: &mut CommandWire, xy: [u32; 2]) {
    wire.flags |= 8;
    wire.auxiliary = xy;
}

/// The order-side effects of the UI. `player` is always the local
/// 0x5C0E10 player in these paths.
pub trait Orders {
    /// 0x61E150 for the local player: UIWorldState vt+0x18 = 0x436950,
    /// i.e. any selected actor's vt+0x150(local kingdom, &cmd, 1), with the
    /// 0x25/0x26 static-group special case for explicit commands.
    /// The actor virtual may complete the command (0x4DA570's type-4 branch
    /// sets flag 2 and +0x14), so the wire is mutable.
    fn can_send_436950(&mut self, wire: &mut CommandWire) -> Result<bool>;
    /// 0x61EEF0's acknowledgement voice (sound option B39940+0x250): the
    /// 0x436600 voiced object, attack voice +0xDC when flag 1 is set and the
    /// target (0x530F10) vt+0x140(local kingdom) holds, else move voice +0xD4.
    fn acknowledge_61eef0(&mut self, wire: &CommandWire) -> Result<()>;
    /// 0x61E960: SetPlayer prefix, descriptor 84 and the 0x61E6B0 payload.
    fn emit_61e960(&mut self, wire: &CommandWire) -> Result<()>;
    /// 0x404F60 "order.CanSend() failed" (line 0xF8) and the line 0x135
    /// summary 0x61F030..0x61F1EC writes after emitting.
    fn diagnostic_61eef0(&mut self, wire: &CommandWire, sent: bool) -> Result<()>;
    /// VWorld 0x43DB10(x, y): the destination marker.
    fn marker_43db10(&mut self, xy: [u32; 2]) -> Result<()>;
}

/// 0x61EEF0 for the local player, preceded by the caller's 0x61E150 when the
/// caller makes one (`checked`). Returns whether the request was emitted.
pub fn send_61eef0(orders: &mut impl Orders, wire: &mut CommandWire) -> Result<bool> {
    if !orders.can_send_436950(wire)? {
        orders.diagnostic_61eef0(wire, false)?;
        return Ok(false);
    }
    orders.acknowledge_61eef0(wire)?;
    orders.emit_61e960(wire)?;
    orders.diagnostic_61eef0(wire, true)?;
    Ok(true)
}

/// The facing drag state at GameInterface+0xE4 (ctor 0x6232F0 over
/// 0x6895B0, vtable 0xA5C580).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facing {
    /// +0x38: the pending command.
    command: Option<CommandWire>,
    /// +0x34: the left (true) or right (false) button ends the drag.
    left_34: bool,
    /// +0x08: the drag passed the threshold.
    dragging_08: bool,
    /// +0x0C..+0x14: the facing vector (only X/Y are written by 0x688990).
    facing_0c: [u32; 3],
    /// +0x3C: an actor handle choosing KKC_TellActorCommandOrder; cleared by
    /// the deactivation 0x620220 and not set by the default right click.
    actor_3c: Option<u32>,
}
impl Default for Facing {
    fn default() -> Self {
        Self::from_constructor_6232f0()
    }
}
impl Facing {
    pub fn from_constructor_6232f0() -> Self {
        Self { command: None, left_34: false, dragging_08: false, facing_0c: [0; 3], actor_3c: None }
    }
    pub fn command(&self) -> Option<&CommandWire> {
        self.command.as_ref()
    }
    pub fn dragging(&self) -> bool {
        self.dragging_08
    }
    pub fn left_button(&self) -> bool {
        self.left_34
    }
    pub fn facing(&self) -> [u32; 3] {
        self.facing_0c
    }
    /// vt+0x58 = 0x621230(cmd, left) minus the push 0x5CCD60 does (the
    /// caller pushes the state): +0x38 = cmd, +0x34 = left, then the
    /// 0x5CCD60 resets (+8 = 0, facing zero).
    pub(super) fn begin_621230(&mut self, command: CommandWire, left: bool) {
        self.command = Some(command);
        self.left_34 = left;
        self.dragging_08 = false;
        self.facing_0c = [0; 3];
    }
    /// vt+4 = 0x620220: +0x3C = 0 (then 0x6885F0's presentation cleanup).
    pub(super) fn deactivate_620220(&mut self) {
        self.actor_3c = None;
    }
    /// vt+0x10 = 0x688AA0: start dragging once the cursor left the RIGHT
    /// press point (tracker +0x14/+0x18) by the Manhattan threshold.
    /// Returns whether the drag started this frame (presentation follows).
    pub(super) fn threshold_688aa0(&mut self, cursor: [i32; 2], right_press: [i32; 2], threshold: i32) -> bool {
        if self.dragging_08 {
            return false;
        }
        let d = (right_press[1].wrapping_sub(cursor[1])).wrapping_abs()
            .wrapping_add((right_press[0].wrapping_sub(cursor[0])).wrapping_abs());
        if d < threshold {
            return false;
        }
        self.dragging_08 = true;
        true
    }
    /// vt+8 = 0x688990: while dragging, the facing is the 0x44B600-normalized
    /// XY from the press world point (VWorld+0x19C left, +0x1B4 right) to the
    /// current pointer point VWorld+0x184. Returns true when it was rewritten.
    pub(super) fn update_688990(&mut self, pointer_184: [u32; 3], press: [u32; 3], p: Precision) -> Result<bool> {
        if !self.dragging_08 {
            return Ok(false);
        }
        let dx = st(&x87::sub(&b(pointer_184[0])?, &b(press[0])?, p)?)?;
        let dy = st(&x87::sub(&b(pointer_184[1])?, &b(press[1])?, p)?)?;
        // `fucompp` with 0.0 then `test ah,0x44; jp/jnp`: equal (incl. -0.0).
        let zero = |w: u32| w & 0x7fff_ffff == 0;
        if zero(dx) && zero(dy) {
            return Ok(false);
        }
        let [x, y] = normalize_44b600([dx, dy], p)?;
        self.facing_0c[0] = x;
        self.facing_0c[1] = y;
        Ok(true)
    }
    /// vt+0x3C = 0x6210B0 for the TellSelected branch: flag 8 with the
    /// facing (dragged) or zero, the command and its marker position.
    pub(super) fn finish_6210b0(&mut self) -> Result<(CommandWire, [u32; 2])> {
        let mut wire = self.command.clone().ok_or("0x6210D2 reads +0x38 of a facing state without a command")?;
        if self.actor_3c.is_some() {
            return Err("0x621161 TellActorCommandOrder branch (+0x3C) is not reached by the ported paths".into());
        }
        if self.dragging_08 {
            set_facing(&mut wire, [self.facing_0c[0], self.facing_0c[1]]);
        } else {
            set_facing(&mut wire, [0, 0]);
        }
        let marker = wire.position;
        Ok((wire, marker))
    }
}

/// 0x44B600: normalize (x, y) in place by `1/sqrt(x*x + y*y)`; lengths at or
/// below 0x9FD38C (~1e-6) store zero.
pub fn normalize_44b600(v: [u32; 2], p: Precision) -> Result<[u32; 2]> {
    let x = b(v[0])?;
    let y = b(v[1])?;
    let length = x87::sqrt(&x87::add(&x87::mul(&x, &x, p)?, &x87::mul(&y, &y, p)?, p)?, p)?;
    if x87::le(&length, &b(0x3586_37bd)?) {
        return Ok([0, 0]);
    }
    let inverse = x87::div(&b(0x3f80_0000)?, &length, p)?;
    Ok([st(&x87::mul(&inverse, &x, p)?)?, st(&x87::mul(&inverse, &y, p)?)?])
}
