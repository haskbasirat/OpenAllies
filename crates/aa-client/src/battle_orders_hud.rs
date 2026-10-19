//! The camera/presentation side of `battle_orders_input::HudLeaves` (the
//! KKC_UI +0xC0 camera slot, `*[B39990+0x60]` and the box/facing states'
//! camera children). HUD widget routing, the minimap, sounds, markers and
//! command definitions stay with their owners (`UnboundHud` until bound).
//!
//! Native paths (AA.exe):
//! - +0xC0: 5E0570/5E0590 install the UI+40 scroller (A4DA84) or UI+48 rotate
//!   (A4D968) state; their vt+0x10 (5CD410 / 5CCF00, both return 0) and vt+8
//!   (5CD320 / 4B2830) are executed by the camera frame
//!   (`battle_camera::Process::input_frame`, before the orders frame), so
//!   here they only report what the slot holds and what vt+0x10 returns.
//! - `*[B39990+0x60]`: AVar 24 "CursorDragThreshold", registered by 401BD0
//!   (402894..4028C2: int, default 10, flags 0, so every AVars*.tgi
//!   declaration assigns it in load order).
//! - Box: 5CD850 (enter), 5CCC10 -> 5CCA80 (cursor moved), 5CCA30 (exit) on
//!   `*B40048`; see `aa_runtime::native_camera::screen`.
//! - Facing: 689490 -> 689380 ghosts, 5CCD60 (arrow +18 vt+2C(0), +214 = 1);
//!   688AE2..688B25 drag start; 688A43 / 693D10 per frame; 6885F0 -> 693EB0
//!   ghosts, 5CCDA0 (arrow vt+2C(0), +214 = 0). The ghosts and the arrow are
//!   VWorld scene objects ([`FacingScene`], the view owner's).
//!
//! The camera children are recorded in native order during the orders frame
//! and applied to the SAME camera right after it (`apply`): during the frame
//! the machine reads the camera through a shared loan, and none of the
//! children writes a field it reads (+214 and Camera+168 only).
use crate::battle_orders_input::{HudLeaves, Result, UnboundHud, View};
use aa_orders::commands::CommandWire;
use aa_runtime::player_order_input::{
    event::Event,
    states::{CameraSlot, CommandDefinition},
    ui_world::StaticRef,
};

/// One camera child of the box/facing states, in the order reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Child {
    /// 5CD850 (after the state push and press copy).
    BoxBegin,
    /// 5CCC10 -> 5CCA80.
    BoxFrame,
    /// 5CCA30 (before its VWorld+13C vt+44 tail, which the machine runs).
    BoxEnd,
    /// 5CCD70: `[B40048]+214 = 1`.
    FacingLock,
    /// 5CCDB3: `[B40048]+214 = 0`.
    FacingUnlock,
}

/// The facing state's VWorld scene objects (view owner).
pub trait FacingScene {
    /// 689380: one ghost per selected view.
    fn ghosts_689380(&mut self, selected: &[View]) -> Result<()>;
    /// The embedded arrow at state+18: vt+2C(0) (5CCD7F, 5CCDA6).
    fn arrow_2c_hide(&mut self) -> Result<()>;
    /// 688AE2..688B25: arrow vt+2C(43CCA0(VWorld+1CC, pointer +19C/+1B4)),
    /// then the state's vt+44 and vt+48.
    fn drag_start_688ae2(&mut self) -> Result<()>;
    /// 688A43 (+1C vt+6C with the facing point) and 693D10 (ghosts).
    fn frame_688a43(&mut self, facing: [u32; 3], rotated: bool) -> Result<()>;
    /// 693EB0: ghost teardown.
    fn ghosts_end_693eb0(&mut self) -> Result<()>;
}
/// No view owner bound for the facing ghosts/arrow: explicit errors.
pub struct UnboundFacing;
impl FacingScene for UnboundFacing {
    fn ghosts_689380(&mut self, _: &[View]) -> Result<()> {
        Err("689380 facing ghosts (VWorld scene objects): no live accessor yet (owner: view directory)".into())
    }
    fn arrow_2c_hide(&mut self) -> Result<()> {
        Err("facing arrow state+18 vt+2C: no live accessor yet (owner: view directory)".into())
    }
    fn drag_start_688ae2(&mut self) -> Result<()> {
        Err("688AE2 facing arrow show (VWorld+1CC, 43CCA0): no live accessor yet (owner: view directory)".into())
    }
    fn frame_688a43(&mut self, _: [u32; 3], _: bool) -> Result<()> {
        Err("688A43/693D10 facing arrow and ghost placement: no live accessor yet (owner: view directory)".into())
    }
    fn ghosts_end_693eb0(&mut self) -> Result<()> {
        Err("693EB0 facing ghost teardown: no live accessor yet (owner: view directory)".into())
    }
}

/// `*[B39990+0x60]` from the actual AVars*.tgi declarations (Settings rows in
/// load order). Only plain decimal literals are accepted: another spelling
/// needs the native int variable parser, which is not ported.
pub fn cursor_drag_threshold_b39990(settings: &aa_runtime::settings::Settings) -> Result<i32> {
    let mut value = 10; // 402896 push 0Ah
    for row in settings.app_variables.get("cursordragthreshold").map(Vec::as_slice).unwrap_or(&[]) {
        if !row.field_type.as_deref().is_some_and(|t| t.eq_ignore_ascii_case("int")) {
            return Err("CursorDragThreshold declaration is not the registered int type".into());
        }
        let raw = row.raw.trim();
        let digits = raw.strip_prefix('-').unwrap_or(raw);
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("CursorDragThreshold value {raw:?} needs the native int variable parser"));
        }
        value = raw.parse::<i32>().map_err(|_| format!("CursorDragThreshold value {raw:?} is outside int"))?;
    }
    Ok(value)
}

/// The HudLeaves for one orders frame.
pub struct Leaves<'f> {
    slot_c0: CameraSlot,
    vworld_ad: bool,
    drag_threshold: Result<i32>,
    facing: &'f mut dyn FacingScene,
    children: Vec<Child>,
    hud: UnboundHud,
}
impl<'f> Leaves<'f> {
    /// `slot_c0`: the camera controls' +0xC0 (None = Empty), as left by the
    /// camera frame that ran first.
    pub fn new(
        slot_c0: Option<aa_runtime::native_camera::input::Slot>,
        vworld_ad: bool,
        drag_threshold: Result<i32>,
        facing: &'f mut dyn FacingScene,
    ) -> Self {
        let slot_c0 = if slot_c0.is_some() { CameraSlot::Generic } else { CameraSlot::Empty };
        Self { slot_c0, vworld_ad, drag_threshold, facing, children: Vec::new(), hud: UnboundHud }
    }
    pub fn children(&self) -> &[Child] {
        &self.children
    }
}
macro_rules! hud_owner {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)*) => {
        $(fn $name(&mut self, $($arg: $ty),*) -> $ret { self.hud.$name($($arg),*) })*
    };
}
impl HudLeaves for Leaves<'_> {
    hud_owner! {
        widget_route_5e0690(e: &Event) -> Result<bool>;
        pressed_widget_chain_5efff0(e: &Event) -> Result<bool>;
        widget_pick_64(x: i32, y: i32) -> Result<Option<View>>;
        brush_installed_bc() -> Result<bool>;
        map_world_5d74b0(cursor: [i32; 2]) -> Result<[u32; 2]>;
        map_auxiliary_b279c0() -> Result<[u32; 2]>;
        cursor_feedback_620640() -> Result<()>;
        selection_sound_436850(v: View) -> Result<()>;
        command_masks_437eb0(selected: &[View], group: StaticRef, masks: &mut [u32; 4]) -> Result<()>;
        brush_changed_42d4d0() -> Result<()>;
        vworld_list_changed_4325c0() -> Result<()>;
        marker_43db10(xy: [u32; 2]) -> Result<()>;
        acknowledge_61eef0(wire: &CommandWire) -> Result<()>;
        can_send_436950(wire: &mut CommandWire) -> Result<bool>;
        command_definition_298(index: u32) -> Result<Option<CommandDefinition>>;
    }
    fn camera_slot_c0(&mut self) -> Result<CameraSlot> {
        Ok(self.slot_c0)
    }
    /// 5CD410 / 5CCF00 return 0; the camera frame already ran them.
    fn camera_frame_10(&mut self) -> Result<bool> {
        Ok(false)
    }
    /// 5CD320 / 4B2830 ran in the camera frame.
    fn camera_frame_8(&mut self) -> Result<()> {
        Ok(())
    }
    fn vworld_ad(&mut self) -> Result<bool> {
        Ok(self.vworld_ad)
    }
    fn drag_threshold_b39990(&mut self) -> Result<i32> {
        self.drag_threshold.clone()
    }
    fn box_rectangle_begin(&mut self, _press: &Event) -> Result<()> {
        self.children.push(Child::BoxBegin);
        Ok(())
    }
    fn box_rectangle_frame(&mut self) -> Result<()> {
        self.children.push(Child::BoxFrame);
        Ok(())
    }
    fn box_rectangle_end(&mut self) -> Result<()> {
        self.children.push(Child::BoxEnd);
        Ok(())
    }
    /// 689490: 689380, then 5CCD60's arrow hide and +214 store.
    fn facing_begin_689380(&mut self, selected: &[View]) -> Result<()> {
        self.facing.ghosts_689380(selected)?;
        self.facing.arrow_2c_hide()?;
        self.children.push(Child::FacingLock);
        Ok(())
    }
    fn facing_drag_start(&mut self) -> Result<()> {
        self.facing.drag_start_688ae2()
    }
    fn facing_frame(&mut self, facing: [u32; 3], rotated: bool) -> Result<()> {
        self.facing.frame_688a43(facing, rotated)
    }
    /// 6885F0: 693EB0, then 5CCDA0's arrow hide and +214 store.
    fn facing_end_6885f0(&mut self) -> Result<()> {
        self.facing.ghosts_end_693eb0()?;
        self.facing.arrow_2c_hide()?;
        self.children.push(Child::FacingUnlock);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_rules::resources::Provenance;
    use aa_runtime::settings::{Declaration, Settings};
    fn row(t: &str, raw: &str) -> Declaration {
        Declaration {
            field_type: Some(t.into()),
            raw: raw.into(),
            source: Provenance { path: "AVars.tgi".into(), sha256: String::new(), span: aa_rules::tgi::Span { start: 0, end: 0 } },
        }
    }
    #[test]
    fn drag_threshold_is_the_native_default_or_the_last_declaration() {
        let mut s = Settings::default();
        assert_eq!(cursor_drag_threshold_b39990(&s).unwrap(), 10);
        s.app_variables.insert("cursordragthreshold".into(), vec![row("int", " 10"), row("Int", "4")]);
        assert_eq!(cursor_drag_threshold_b39990(&s).unwrap(), 4);
        s.app_variables.insert("cursordragthreshold".into(), vec![row("int", "0x10")]);
        assert!(cursor_drag_threshold_b39990(&s).is_err());
        s.app_variables.insert("cursordragthreshold".into(), vec![row("float", "4")]);
        assert!(cursor_drag_threshold_b39990(&s).is_err());
    }
    #[test]
    fn camera_slot_and_children_follow_the_native_states() {
        let mut facing = UnboundFacing;
        let mut l = Leaves::new(Some(aa_runtime::native_camera::input::Slot::Rotate), true, Ok(10), &mut facing);
        assert_eq!(l.camera_slot_c0().unwrap(), CameraSlot::Generic);
        assert!(!l.camera_frame_10().unwrap());
        let e = Event { x: 1, y: 2, flags: 0x81, code: 0x100, delta: 0, world: [0; 3], origin: [0; 3], direction: [0; 3] };
        l.box_rectangle_begin(&e).unwrap();
        l.box_rectangle_frame().unwrap();
        l.box_rectangle_end().unwrap();
        //689380 is reached before 5CCD60: no camera lock without the ghosts.
        assert!(l.facing_begin_689380(&[]).is_err());
        assert_eq!(l.children(), [Child::BoxBegin, Child::BoxFrame, Child::BoxEnd]);
        assert!(l.brush_installed_bc().is_err());
        let mut facing = UnboundFacing;
        let mut empty = Leaves::new(None, false, Ok(10), &mut facing);
        assert_eq!(empty.camera_slot_c0().unwrap(), CameraSlot::Empty);
    }
}
