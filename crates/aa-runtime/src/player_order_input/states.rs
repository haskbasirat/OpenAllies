//! The UI manager's world-input dispatch and the game interface's UI states.
//!
//! UI manager (DAT_00B3653C) slots: the modal stack +0xB4 (head = top), the
//! single slots +0xBC (brush/targeting states), +0xC0 (camera states such as
//! UI+0x40, vtable 0xA4D968) and +0xC4 (the game default state). The game
//! interface owns the states it installs:
//!
//! | GameInterface | vtable | role |
//! |---|---|---|
//! | +0xD8 | 0xA57530 | default (+0xC4): right-click orders 0x620760, base 0x5CD190 |
//! | +0xDC | 0xA57560 | left-click pending: 0x5CC960, drag threshold 0x5CDBE0 |
//! | +0xE0 | 0xA575A4 | drag box: 0x5CCB70, entered by 0x5CD850 |
//! | +0xE4 | 0xA5C580 | facing drag of a right-click location order |
//!
//! Event order per frame (0x5C92F0): cursor tracker 0x5BF2B0 and, when it
//! moved, the states' vt+0x10 (0x5C1AF0, first true stops); then each queued
//! event through 0x5C3BC0 (widget pick of presses, hover at the press,
//! 0x5C1B80 dispatch); then every slot's vt+8 (modal top only, +0xBC, +0xC0,
//! +0xC4). Widget (HUD) routing is the caller's: [`Machine::event`] is the
//! part of 0x5C1B80 after the root widget chain declined the event.
use super::command::{self, Facing, Orders};
use super::event::{self, code, flag, Event, Input};
use super::pick::{self, Scene};
use super::ui_world::{self, Leaves, State as Selection};
use crate::view_pointer;
use aa_orders::commands::CommandWire;

pub type Result<T> = std::result::Result<T, String>;

/// UI+0x58 cursor tracker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tracker {
    /// +4/+8: the cursor copied from Input+0xC/+0x10 by 0x5BF2B0.
    pub current: [i32; 2],
    /// +0xC/+0x10: the last left press or double click (0x5BF2F0).
    pub left_press: [i32; 2],
    /// +0x14/+0x18: the last right press (0x5BF2F0).
    pub right_press: [i32; 2],
}
impl Tracker {
    /// 0x5BF2B0: copy the Input cursor; true when it changed.
    pub fn update_5bf2b0(&mut self, cursor: [i32; 2]) -> bool {
        let moved = self.current != cursor;
        self.current = cursor;
        moved
    }
    /// 0x5BF2F0 (always returns false: the event goes on).
    pub fn press_5bf2f0(&mut self, e: &Event) {
        if e.left_press() {
            self.left_press = [i32::from(e.x), i32::from(e.y)];
        } else if e.right_press() {
            self.right_press = [i32::from(e.x), i32::from(e.y)];
        }
    }
}

/// UI+0x60: the widget under the cursor, set by 0x5C3BC0 on presses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Widget {
    None,
    /// The VWorld widget DAT_00B36538 (its vt+0x90 is 0x6A71F0, false).
    World,
    /// A widget whose vt+0x90 is true (the minimap); 0x5D74B0 maps the
    /// cursor to a world XY on it.
    Map,
    /// Any other widget (HUD), with an identity the UI thread assigns (the
    /// widget allocation): 0x5E0690 compares its +4 with UI+0x60 by pointer.
    Other(u32),
}

/// The +0xC0 slot as far as world input sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraSlot {
    Empty,
    /// vtable 0xA4D968 (UI+0x40 and siblings): vt+0x14 and vt+0x18 return
    /// false, vt+0x20 true, vt+0x24 0x5E05D0; vt+0x10 0x5CCF00 belongs to the
    /// camera code.
    Generic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Modal {
    /// GameInterface+0xDC
    Click,
    /// GameInterface+0xE0
    DragBox,
    /// GameInterface+0xE4
    Facing,
}
impl Modal {
    /// vt+0x2C: false pops the whole stack before the push (0x5C35A0).
    fn stacks_2c(self) -> bool {
        matches!(self, Modal::Facing)
    }
}

/// The ActorCommand definition fields the facing check of 0x620A00 reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandDefinition {
    /// def+0x40 (can_hot_command_location).
    pub byte_40: bool,
    /// def+0x2C.
    pub flags_2c: u32,
}

/// What 0x620640 reads from the UI manager and VWorld+0x13C before its
/// actor reads: UI+0x60 (0x62066A), 0x436510, the +8 head object and the
/// +0x10 hovered object whose vt+0x54 (0x4363B0) is the target. The
/// mapping [`ui_world::State::pick_target_6409a0`] is left to the caller so
/// it runs only where the native code reaches 0x4363B0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CursorSelection<V> {
    pub pressed_60: Widget,
    pub multiple_436510: bool,
    pub head_8: Option<V>,
    pub hovered_10: Option<V>,
}

/// Everything the states read besides the selection manager and orders.
pub trait Host: Scene + Orders {
    /// DAT_00B36518 != 0 (43C550 / the VWorld per-frame pointer update).
    fn map_present_b36518(&mut self) -> Result<bool>;
    /// 0x43C4B0(pixel, secondary) with the current camera; the secondary
    /// land point is written by 0x71D840 where it writes it.
    fn project_43c4b0(&mut self, pixel: [i32; 2], secondary: &mut [u32; 3]) -> Result<[u32; 3]>;
    /// 0x5E06E8..0x5E0759, the chains of 0x5E0690 after the machine's +4
    /// step ([`Machine::default_pressed_4`]): the UI+0x60 chain then Root's,
    /// or with UI+0x60 NULL the focus head [UI+0x8C] chain then Root's.
    /// Called only with UI+0x60 VWorld or NULL (a HUD widget's and the
    /// minimap's chain are their UI-thread handlers).
    fn widget_route_5e0690(&mut self, e: &Event) -> Result<bool>;
    /// vt+0x64(x, y) of a non-world UI+0x60 widget (base 0x5EDE60: NULL).
    fn widget_pick_64(&mut self, x: i32, y: i32) -> Result<Option<Self::View>>;
    /// The UI+0x60 widget chain 0x5EFFF0 called inside 0x5E0910.
    fn pressed_widget_chain_5efff0(&mut self, e: &Event) -> Result<bool>;
    /// `*[B39990+0x60]`: the drag threshold (pixels, Manhattan).
    fn drag_threshold_b39990(&mut self) -> Result<i32>;
    /// The +0xBC slot: a brush state installed by the action buttons.
    fn brush_installed_bc(&mut self) -> Result<bool>;
    /// The +0xC0 slot.
    fn camera_slot_c0(&mut self) -> Result<CameraSlot>;
    /// The +0xC0 state's vt+0x10 / vt+8 (camera code).
    fn camera_frame_10(&mut self) -> Result<bool>;
    fn camera_frame_8(&mut self) -> Result<()>;
    /// 0x5D74B0 on the Map widget: the world XY under Input+0xC.
    fn map_world_5d74b0(&mut self, cursor: [i32; 2]) -> Result<[u32; 2]>;
    /// B279C0/B279C4: the minimap order's auxiliary words.
    fn map_auxiliary_b279c0(&mut self) -> Result<[u32; 2]>;
    /// actor vt+0x34(index) of the selected view's 0x45B7A0 actor.
    fn actor_can_34(&mut self, view: Self::View, index: u32) -> Result<bool>;
    /// `[[B36530]+0x298][index]` (NULL = None).
    fn command_definition_298(&mut self, index: u32) -> Result<Option<CommandDefinition>>;
    /// `actor+0x14` of the 0x45B7A0 actor (the command target handle).
    fn actor_handle_14(&mut self, view: Self::View) -> Result<u32>;
    /// 0x459600 on the 0x4363B0 target view after an attack order was sent
    /// (its +0x98 flash time).
    fn flash_459600(&mut self, view: Self::View) -> Result<()>;
    /// The default state's per-frame tail 0x620648..0x62074D (EconomicImpact
    /// preview) with the UI-manager and UIWorldState fields it reads.
    fn cursor_feedback_620640(&mut self, selection: CursorSelection<Self::View>) -> Result<()>;
    /// Presentation children (no simulation effect): box rectangle 0x5CD850 /
    /// per-frame 0x5CCA80 / removal 0x5CCA30, facing
    /// begin 0x689380, drag start 0x688AE2..0x688B25, per-frame 0x688A43 and
    /// 0x693D10, end 0x6885F0.
    fn box_rectangle_begin(&mut self, press: &Event) -> Result<()>;
    fn box_rectangle_frame(&mut self) -> Result<()>;
    fn box_rectangle_end(&mut self) -> Result<()>;
    fn facing_begin_689380(&mut self, selected: &[Self::View]) -> Result<()>;
    fn facing_drag_start(&mut self) -> Result<()>;
    fn facing_frame(&mut self, facing: [u32; 3], rotated: bool) -> Result<()>;
    fn facing_end_6885f0(&mut self) -> Result<()>;
}

/// The UI-side owner of world input for one battle: KKC_Input fields, the
/// cursor tracker, UI+0x60, the modal stack and the game states, the VWorld
/// pointer fields and the game UIWorldState.
pub struct Machine<V> {
    input: Input,
    cursor_0c: [i32; 2],
    tracker: Tracker,
    pressed_60: Widget,
    modal: Vec<Modal>,
    click_event: Option<Event>,
    box_event: Option<Event>,
    facing: Facing,
    default_c4: bool,
    /// GameInterface+0xD8 +4: the widget of the last left press 0x5E0690
    /// saw (NULL = [`Widget::None`]; constructor 0).
    default_4: Widget,
    pointer: view_pointer::State,
    selection: Selection<V>,
}

/// The view_pointer services over the machine's tracker and the host.
struct Pointer<'a, H> {
    host: &'a mut H,
    current: [i32; 2],
}
impl<H: Host> view_pointer::Services for Pointer<'_, H> {
    fn map_present(&mut self) -> Result<bool> {
        self.host.map_present_b36518()
    }
    fn current_input_pixel(&mut self) -> Result<[i32; 2]> {
        Ok(self.current)
    }
    fn project_43c4b0(&mut self, point: [i32; 2], _: view_pointer::Slot, secondary: &mut [u32; 3]) -> Result<[u32; 3]> {
        self.host.project_43c4b0(point, secondary)
    }
}

impl<V: Copy + Eq + std::fmt::Debug> Machine<V> {
    /// The state after KKC_GameInterface 0x60BAE0 installed the default state
    /// (+0xD8 into +0xC4) and bound its UIWorldState to VWorld+0x13C.
    pub fn for_game_interface_60bae0() -> Self {
        Self {
            input: Input::new(),
            cursor_0c: [0; 2],
            tracker: Tracker::default(),
            pressed_60: Widget::None,
            modal: Vec::new(),
            click_event: None,
            box_event: None,
            facing: Facing::from_constructor_6232f0(),
            default_c4: true,
            default_4: Widget::None,
            pointer: view_pointer::State::from_vworld_constructor(),
            selection: Selection::from_constructor_640640(),
        }
    }
    pub fn selection(&self) -> &Selection<V> {
        &self.selection
    }
    pub fn selection_mut(&mut self) -> &mut Selection<V> {
        &mut self.selection
    }
    pub fn input(&self) -> &Input {
        &self.input
    }
    pub fn input_mut(&mut self) -> &mut Input {
        &mut self.input
    }
    pub fn tracker(&self) -> Tracker {
        self.tracker
    }
    pub fn pressed_widget(&self) -> Widget {
        self.pressed_60
    }
    pub fn pointer(&self) -> &view_pointer::State {
        &self.pointer
    }
    pub fn facing(&self) -> &Facing {
        &self.facing
    }
    /// The modal stack from the top: "click", "box", "facing".
    pub fn modal_names(&self) -> Vec<&'static str> {
        self.modal
            .iter()
            .map(|m| match m {
                Modal::Click => "click",
                Modal::DragBox => "box",
                Modal::Facing => "facing",
            })
            .collect()
    }
    /// Input+0xC/+0x10: the client cursor position the window code keeps.
    pub fn set_cursor(&mut self, cursor: [i32; 2]) {
        self.cursor_0c = cursor;
    }

    // ---- the current-state queries used by the selection manager ----

    /// 0x436D70's single-object test on the selection list.
    fn single_selected(&self) -> Option<V> {
        let s = self.selection.selected();
        match (s.first(), s.last()) {
            (Some(h), Some(t)) if h == t => Some(*h),
            _ => None,
        }
    }
    fn top(&self) -> Option<Modal> {
        self.modal.first().copied()
    }
    /// The first installed of modal top, +0xBC, +0xC0, +0xC4.
    fn current<H: Host<View = V>>(&self, host: &mut H) -> Result<Current> {
        if let Some(m) = self.top() {
            return Ok(Current::Modal(m));
        }
        if host.brush_installed_bc()? {
            return Err("UI+0xBC brush states (0x5CD220 family) are not ported".into());
        }
        if host.camera_slot_c0()? == CameraSlot::Generic {
            return Ok(Current::Camera);
        }
        Ok(if self.default_c4 { Current::Default } else { Current::None })
    }

    // ---- per frame ----

    /// The part of 0x5C92F0 before the event pump: VWorld's per-frame pointer
    /// store (0x4400F8..0x44012F), 0x5BF2B0 and, when the cursor moved, the
    /// states' vt+0x10 (0x5C1AF0).
    pub fn frame_begin<H: Host<View = V>>(&mut self, host: &mut H) -> Result<()> {
        let mut pointer = Pointer { host: &mut *host, current: self.tracker.current };
        self.pointer.update_43c550(None, &mut pointer)?;
        if self.tracker.update_5bf2b0(self.cursor_0c) {
            self.frame_10(host)?;
        }
        Ok(())
    }
    fn frame_10<H: Host<View = V>>(&mut self, host: &mut H) -> Result<bool> {
        if let Some(m) = self.top() {
            let handled = match m {
                Modal::Click => self.click_frame_5cdbe0(host)?,
                Modal::DragBox => {
                    host.box_rectangle_frame()?;
                    false
                }
                Modal::Facing => {
                    let threshold = host.drag_threshold_b39990()?;
                    if self.facing.threshold_688aa0(self.tracker.current, self.tracker.right_press, threshold) {
                        host.facing_drag_start()?;
                    }
                    false
                }
            };
            if handled {
                return Ok(true);
            }
        }
        if host.brush_installed_bc()? {
            return Err("UI+0xBC brush states are not ported".into());
        }
        if host.camera_slot_c0()? == CameraSlot::Generic && host.camera_frame_10()? {
            return Ok(true);
        }
        // +0xC4 default: vt+0x10 = 0x5E04F0 (UI+0x60 widget vt+0x30).
        Ok(false)
    }
    /// The per-frame vt+8 of every slot (0x5C943E..0x5C948A).
    pub fn frame_end<H: Host<View = V>>(&mut self, host: &mut H) -> Result<()> {
        match self.top() {
            Some(Modal::Click) | None => {}
            Some(Modal::DragBox) => {
                // vt+8 = 0x5CC770 -> 0x437DC0: tracker left press and cursor.
                let (press, cursor) = (self.tracker.left_press, self.tracker.current);
                let mut adapter = Adapter { machine_current: self.current(host)?, single: self.single_selected(), widget: self.pressed_60, planes: None, host: &mut *host };
                self.selection.box_preview_4371a0(&mut adapter, press, cursor)?;
            }
            Some(Modal::Facing) => {
                let press = if self.facing.left_button() {
                    self.pointer.position(view_pointer::Slot::Down)
                } else {
                    self.pointer.position(view_pointer::Slot::Up)
                };
                let p = Leaves::precision(host);
                let rotated = self.facing.update_688990(self.pointer.position(view_pointer::Slot::Current), press, p)?;
                host.facing_frame(self.facing.facing(), rotated)?;
            }
        }
        if host.brush_installed_bc()? {
            return Err("UI+0xBC brush states are not ported".into());
        }
        if host.camera_slot_c0()? == CameraSlot::Generic {
            host.camera_frame_8()?;
        }
        if self.default_c4 {
            self.default_frame_620640(host)?;
        }
        Ok(())
    }
    /// 0x620640: 0x5CD6B0 (hover at the tracker cursor while VWorld+0xAD;
    /// the UI+0x40 edge-scroll activation belongs to the camera code) and the
    /// cursor feedback.
    fn default_frame_620640<H: Host<View = V>>(&mut self, host: &mut H) -> Result<()> {
        if host.vworld_ad()? {
            let [x, y] = self.tracker.current;
            let mut adapter = Adapter { machine_current: self.current(host)?, single: self.single_selected(), widget: self.pressed_60, planes: None, host: &mut *host };
            self.selection.update_hover_436650(&mut adapter, x, y)?;
        }
        let s = &self.selection;
        let selection = CursorSelection {
            pressed_60: self.pressed_60,
            multiple_436510: s.multiple_436510(),
            head_8: s.selected().first().copied(),
            hovered_10: s.hovered_10(),
        };
        host.cursor_feedback_620640(selection)
    }

    // ---- events ----

    /// 0x59A6E0 for a window mouse message (WndProc 0x6FD4A0): returns the
    /// queued event, or None when it is dropped.
    pub fn mouse_message<H: Host<View = V> + event::Sources>(
        &mut self,
        host: &mut H,
        code: u16,
        flags: u32,
        x: i16,
        y: i16,
        delta: i16,
    ) -> Result<Option<Event>> {
        let p = Leaves::precision(host);
        self.input.add_mouse_event_59a6e0(host, code, flags, x, y, delta, p)
    }
    /// The 0x5C3BC0 prefix for one event: on presses, UI+0x60 = `widget`
    /// (the caller's 0x5EE360/0x5EFA80 hit test) and the UIWorldState hover
    /// at the press pixel while VWorld+0xAD.
    pub fn pump_press_5c3bc0<H: Host<View = V>>(&mut self, host: &mut H, e: &Event, widget: Widget) -> Result<()> {
        let press = (e.code == code::LEFT && e.pressed()) || e.code == code::LEFT_DOUBLE || (e.code == code::RIGHT && e.pressed());
        if !press {
            return Ok(());
        }
        self.pressed_60 = widget;
        if host.vworld_ad()? {
            let mut adapter = Adapter { machine_current: self.current(host)?, single: self.single_selected(), widget: self.pressed_60, planes: None, host: &mut *host };
            self.selection.update_hover_436650(&mut adapter, i32::from(e.x), i32::from(e.y))?;
        }
        Ok(())
    }
    /// 0x5C1B80's cursor tracker step, which runs before the widget chain.
    pub fn track_5bf2f0(&mut self, e: &Event) {
        self.tracker.press_5bf2f0(e);
    }
    /// 0x5C1B80 after the root widget chain (0x5EFFF0 on UI+0x74) declined:
    /// 0x43C550, then modal top, +0xBC, +0xC0, +0xC4. Returns whether a state
    /// took the event (the caller then skips the focus chain UI+0x8C).
    pub fn event<H: Host<View = V>>(&mut self, host: &mut H, e: &Event) -> Result<bool> {
        let pe = view_pointer::Event { pixel: [e.x, e.y], flags: e.flags, code: e.code };
        let mut pointer = Pointer { host: &mut *host, current: self.tracker.current };
        self.pointer.update_43c550(Some(&pe), &mut pointer)?;
        if let Some(m) = self.top() {
            let handled = match m {
                Modal::Click => self.click_input_5cc960(host, e)?,
                Modal::DragBox => self.box_input_5ccb70(host, e)?,
                Modal::Facing => self.facing_input_6886b0(host, e)?,
            };
            if handled {
                return Ok(true);
            }
        }
        if host.brush_installed_bc()? {
            return Err("UI+0xBC brush states (0x5CD220 family) are not ported".into());
        }
        // +0xC0 Generic vt+0x14 = 0x969B10 (false).
        if self.default_c4 && self.default_input_620760(host, e)? {
            return Ok(true);
        }
        Ok(false)
    }

    // ---- modal stack ----

    /// 0x5E0780 -> 0x5C35A0: vt+0x2C false pops every state first (0x5C2050,
    /// each with its vt+4), then the push and vt[0] (all 0x4B2830 here).
    fn push<H: Host<View = V>>(&mut self, host: &mut H, m: Modal) -> Result<()> {
        if !m.stacks_2c() {
            while !self.modal.is_empty() {
                let popped = self.modal.remove(0);
                self.deactivate(host, popped)?;
            }
        }
        self.modal.insert(0, m);
        Ok(())
    }
    /// 0x5E0890: only the top state removes itself, then its vt+4.
    fn pop_self<H: Host<View = V>>(&mut self, host: &mut H, m: Modal) -> Result<()> {
        if self.top() == Some(m) {
            self.modal.remove(0);
            self.deactivate(host, m)?;
        }
        Ok(())
    }
    /// vt+4 of each modal state.
    fn deactivate<H: Host<View = V>>(&mut self, host: &mut H, m: Modal) -> Result<()> {
        match m {
            Modal::Click => Ok(()),
            // 0x5CCA30: renderer+0x214 = 0, rectangle removed, then
            // UIWorldState vt+0x44.
            Modal::DragBox => {
                host.box_rectangle_end()?;
                let mut adapter = Adapter { machine_current: self.current(host)?, single: self.single_selected(), widget: self.pressed_60, planes: None, host: &mut *host };
                self.selection.clear_candidates_6406b0(&mut adapter)
            }
            Modal::Facing => {
                self.facing.deactivate_620220();
                host.facing_end_6885f0()
            }
        }
    }
    /// 0x5E0910: Esc pops the top, the UI+0x60 chain may take the event, and
    /// a top state whose vt+0x28 holds (all three) swallows mouse events.
    fn modal_base_5e0910<H: Host<View = V>>(&mut self, host: &mut H, m: Modal, e: &Event) -> Result<bool> {
        if self.top() == Some(m) && e.pressed() && e.code == 0x1b {
            self.pop_self(host, m)?;
            return Ok(true);
        }
        // A mouse event reaching the top state is swallowed (vt+0x28)
        // whatever the UI+0x60 chain answers, so for it the chain is only its
        // own effects: VWorld's hotkey chain here, a HUD widget's or the
        // minimap's handlers on the UI thread.
        if self.top() == Some(m) && e.flags & flag::MOUSE != 0 {
            if self.pressed_60 == Widget::World {
                host.pressed_widget_chain_5efff0(e)?;
            }
            return Ok(true);
        }
        if self.pressed_60 != Widget::None && host.pressed_widget_chain_5efff0(e)? {
            return Ok(true);
        }
        Ok(false)
    }

    // ---- default state (+0xC4 = GameInterface+0xD8) ----

    /// vt+0x14 = 0x620760.
    fn default_input_620760<H: Host<View = V>>(&mut self, host: &mut H, e: &Event) -> Result<bool> {
        let map = self.pressed_60 == Widget::Map;
        let world = self.pressed_60 == Widget::World;
        if !(world || map) || !e.right_press() || self.selection.selected().is_empty() {
            return self.default_base_5cd190(host, e);
        }
        let mut wire = command::new_right_click_command(self.input.modifiers_28());
        let mut adapter = Adapter { machine_current: self.current(host)?, single: self.single_selected(), widget: self.pressed_60, planes: None, host: &mut *host };
        let target = self.selection.hovered_target_4363b0(&mut adapter)?;
        if world {
            if let Some(target) = target {
                if Leaves::actor_present_14(host, target)? && Leaves::actor_45b7a0(host, target)? {
                    command::set_target_4d9bf0(&mut wire, Some(host.actor_handle_14(target)?));
                    // 0x61E150 then 0x61EEF0 (which checks again).
                    if self.can_send_436950(host, &mut wire)? {
                        self.send_61eef0(host, &mut wire)?;
                        host.flash_459600(target)?;
                    }
                    return Ok(true);
                }
            }
        }
        let xy = if map {
            host.map_world_5d74b0(self.cursor_0c)?
        } else {
            let p = self.pointer.position(view_pointer::Slot::Current);
            [p[0], p[1]]
        };
        command::set_location(&mut wire, xy);
        if map {
            let aux = host.map_auxiliary_b279c0()?;
            command::set_facing(&mut wire, aux);
            self.send_61eef0(host, &mut wire)?;
            host.marker_43db10(xy)?;
            return Ok(true);
        }
        for view in self.selection.selected().to_vec() {
            if !Leaves::actor_45b7a0(host, view)? {
                continue;
            }
            for index in 0..0x27 {
                if !host.actor_can_34(view, index)? {
                    continue;
                }
                if let Some(d) = host.command_definition_298(index)? {
                    if d.byte_40 && d.flags_2c >> 5 & 1 != 0 {
                        self.facing_begin_621230(host, wire, false)?;
                        return Ok(true);
                    }
                }
            }
        }
        self.send_61eef0(host, &mut wire)?;
        host.marker_43db10(xy)?;
        Ok(true)
    }
    /// 0x61E150 / 0x61EEF0's CanSend for the local player: [VWorld+0x13C]
    /// vt+0x18 = 0x436950.
    fn can_send_436950<H: Host<View = V>>(&self, host: &mut H, wire: &mut CommandWire) -> Result<bool> {
        let mut adapter = Adapter { machine_current: self.current(host)?, single: self.single_selected(), widget: self.pressed_60, planes: None, host: &mut *host };
        self.selection.can_send_436950(&mut adapter, wire)
    }
    /// 0x61EEF0 for the local player (the 5C0E10 test holds): its own
    /// 0x436950, then voice, 0x61E960 and the diagnostics.
    fn send_61eef0<H: Host<View = V>>(&self, host: &mut H, wire: &mut CommandWire) -> Result<bool> {
        let can_send = self.can_send_436950(host, wire)?;
        command::send_61eef0(host, wire, can_send)
    }
    /// 0x5CD190: a left press or double click on the world opens the click
    /// state (vt+0x28 = 0x6203A0 -> GameInterface+0xDC) with the event.
    fn default_base_5cd190<H: Host<View = V>>(&mut self, host: &mut H, e: &Event) -> Result<bool> {
        if self.pressed_60 == Widget::World && e.left_press() {
            self.push(host, Modal::Click)?;
            self.click_event = Some(*e);
            return Ok(true);
        }
        self.route_5e0690(host, e)
    }
    /// 0x5E0690 on the default state: a left release whose +4 is NULL or
    /// another widget than UI+0x60 clears +4 and is taken; a left press or
    /// double click stores UI+0x60 in +4; then the chains (UI+0x60 and Root,
    /// or the focus head [UI+0x8C] and Root). A HUD widget's and the
    /// minimap's chain is their handler on the UI thread, which takes it.
    fn route_5e0690<H: Host<View = V>>(&mut self, host: &mut H, e: &Event) -> Result<bool> {
        if e.code == code::LEFT && e.released() && (self.default_4 == Widget::None || self.default_4 != self.pressed_60) {
            self.default_4 = Widget::None;
            return Ok(true);
        }
        if (e.code == code::LEFT && e.pressed()) || e.code == code::LEFT_DOUBLE {
            self.default_4 = self.pressed_60;
        }
        match self.pressed_60 {
            Widget::Other(_) | Widget::Map => Ok(true),
            Widget::World | Widget::None => host.widget_route_5e0690(e),
        }
    }
    /// GameInterface+0xD8 +4 (0x5E0690's pressed widget).
    pub fn default_pressed_4(&self) -> Widget {
        self.default_4
    }

    // ---- click state (GameInterface+0xDC) ----

    /// vt+0x14 = 0x5CC960.
    fn click_input_5cc960<H: Host<View = V>>(&mut self, host: &mut H, e: &Event) -> Result<bool> {
        if e.left_release() {
            let press = self.click_event.ok_or("click state without its press event")?;
            let double = press.code == code::LEFT_DOUBLE;
            let mut adapter = Adapter { machine_current: self.current(host)?, single: self.single_selected(), widget: self.pressed_60, planes: None, host: &mut *host };
            // vt+0x40 of the click state is 0x6A71F0 (false).
            if e.add_modifier() {
                self.selection.click_add_640e10(&mut adapter, e.origin, e.direction, double)?;
            } else {
                self.selection.click_640cf0(&mut adapter, e.origin, e.direction, double)?;
            }
            self.pop_self(host, Modal::Click)?;
            return Ok(true);
        }
        if e.right_press() || (e.pressed() && e.code == 0x1b) {
            self.pop_self(host, Modal::Click)?;
            return Ok(true);
        }
        self.modal_base_5e0910(host, Modal::Click, e)
    }
    /// vt+0x10 = 0x5CDBE0: past the threshold from the LEFT press, enter the
    /// drag box (vt+0x3C = 0x6203B0 -> GameInterface+0xE0) via 0x5CD850.
    fn click_frame_5cdbe0<H: Host<View = V>>(&mut self, host: &mut H) -> Result<bool> {
        let t = self.tracker;
        let d = (t.left_press[1].wrapping_sub(t.current[1])).wrapping_abs()
            .wrapping_add((t.left_press[0].wrapping_sub(t.current[0])).wrapping_abs());
        if d >= host.drag_threshold_b39990()? {
            let press = self.click_event.ok_or("click state without its press event")?;
            // 0x5CD850: push (clears the stack: vt+0x2C false), copy the
            // press event to +8, build the rectangle.
            self.push(host, Modal::DragBox)?;
            self.box_event = Some(press);
            host.box_rectangle_begin(&press)?;
        }
        Ok(false)
    }

    // ---- drag box (GameInterface+0xE0) ----

    /// vt+0x14 = 0x5CCB70.
    fn box_input_5ccb70<H: Host<View = V>>(&mut self, host: &mut H, e: &Event) -> Result<bool> {
        if e.left_release() {
            let press = self.box_event.ok_or("box state without its press event")?;
            let mut adapter = Adapter { machine_current: self.current(host)?, single: self.single_selected(), widget: self.pressed_60, planes: None, host: &mut *host };
            if e.add_modifier() {
                self.selection.box_select_4377b0(&mut adapter, &press, e)?;
            } else {
                self.selection.box_replace_436460(&mut adapter, &press, e)?;
            }
            self.pop_self(host, Modal::DragBox)?;
            return Ok(true);
        }
        if e.right_press() {
            self.pop_self(host, Modal::DragBox)?;
            return Ok(true);
        }
        self.modal_base_5e0910(host, Modal::DragBox, e)
    }

    // ---- facing drag (GameInterface+0xE4) ----

    /// vt+0x58 = 0x621230 -> 0x689490 -> 0x689380 / 0x5CCD60.
    fn facing_begin_621230<H: Host<View = V>>(&mut self, host: &mut H, wire: CommandWire, left: bool) -> Result<()> {
        self.facing.begin_621230(wire, left);
        host.facing_begin_689380(self.selection.selected())?;
        self.push(host, Modal::Facing)
    }
    /// vt+0x14 = 0x6886B0.
    fn facing_input_6886b0<H: Host<View = V>>(&mut self, host: &mut H, e: &Event) -> Result<bool> {
        let ends = if self.facing.left_button() {
            e.code == code::LEFT && e.released()
        } else {
            e.code == code::RIGHT && e.released()
        };
        if !ends {
            return self.modal_base_5e0910(host, Modal::Facing, e);
        }
        // vt+0x3C = 0x6210B0, then 0x5E0890.
        let (mut wire, marker) = self.facing.finish_6210b0()?;
        // 0x6211BB 0x61E150, then 0x61EEF0 (which checks again); bl = 1.
        if self.can_send_436950(host, &mut wire)? {
            self.send_61eef0(host, &mut wire)?;
            host.marker_43db10(marker)?;
        }
        self.pop_self(host, Modal::Facing)?;
        Ok(true)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Current {
    Modal(Modal),
    Camera,
    Default,
    None,
}

/// The selection-manager host as the machine sees it: the data leaves come
/// from the host; the UI-manager hooks (0x5C1E70 / 0x5C1970 / 0x5C19E0 tail)
/// answer for the current state, and the pick hooks run [`pick`].
struct Adapter<'a, H: Host> {
    machine_current: Current,
    /// The 0x436D70 input (head == tail object) captured for 0x43DC70.
    single: Option<H::View>,
    /// UI+0x60 for 0x436650's vt+0x64 call.
    widget: Widget,
    /// The planes 0x436DD0 / 0x4371A0 built for this call's 0x4367A0.
    planes: Option<[pick::Plane; 4]>,
    host: &'a mut H,
}
macro_rules! forward {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)*) => {
        $(fn $name(&mut self, $($arg: $ty),*) -> $ret { self.host.$name($($arg),*) })*
    };
}
impl<H: Host> Leaves for Adapter<'_, H> {
    type View = H::View;
    fn precision(&self) -> aa_numeric::Precision {
        self.host.precision()
    }
    forward! {
        flags_54(v: Self::View) -> Result<u32>;
        set_flags_54(v: Self::View, f: u32) -> Result<()>;
        position_38(v: Self::View) -> Result<[u32; 3]>;
        field_10_58cb30(v: Self::View) -> Result<u64>;
        pickable_9c(v: Self::View) -> Result<bool>;
        class_78(v: Self::View) -> Result<bool>;
        class_84(v: Self::View) -> Result<bool>;
        class_7c(v: Self::View) -> Result<bool>;
        class_80(v: Self::View) -> Result<bool>;
        virtual_50(v: Self::View) -> Result<bool>;
        virtual_54(v: Self::View) -> Result<bool>;
        actor_present_14(v: Self::View) -> Result<bool>;
        actor_45b7a0(v: Self::View) -> Result<bool>;
        actor_flag8_bit0(v: Self::View) -> Result<bool>;
        actor_virtual_58(v: Self::View) -> Result<bool>;
        actor_virtual_84(v: Self::View) -> Result<bool>;
        actor_4d12c0(v: Self::View) -> Result<bool>;
        actor_virtual_20(v: Self::View) -> Result<bool>;
        actor_70(v: Self::View) -> Result<bool>;
        actor_90(v: Self::View) -> Result<bool>;
        link_16c_view(v: Self::View) -> Result<Option<Self::View>>;
        link_94_view(v: Self::View) -> Result<Option<Self::View>>;
        link_94_live(v: Self::View) -> Result<bool>;
        hidden_4549c0(v: Self::View) -> Result<bool>;
        actor_view_0c(v: Self::View) -> Result<Option<Self::View>>;
        root_view_4ce6c0(v: Self::View) -> Result<Option<Self::View>>;
        voiced_436600(v: Self::View) -> Result<Option<Option<Self::View>>>;
        actor_virtual_150(v: Self::View, w: &mut CommandWire) -> Result<bool>;
        selection_sound_436850(v: Self::View) -> Result<()>;
        brush_changed_42d4d0() -> Result<()>;
        vworld_list_changed_4325c0() -> Result<()>;
        vworld_ad() -> Result<bool>;
        floater_remove_42eb10(v: Self::View) -> Result<()>;
        floater_update_453470(v: Self::View) -> Result<()>;
    }
    fn command_masks_437eb0(&mut self, s: &[Self::View], g: ui_world::StaticRef, m: &mut [u32; 4]) -> Result<()> {
        self.host.command_masks_437eb0(s, g, m)
    }
    fn requests(&mut self) -> &mut dyn super::requests::Requests<Self::View> {
        self.host.requests()
    }
}
impl<H: Host> Scene for Adapter<'_, H> {
    type Camera = H::Camera;
    forward! {
        on_list_100() -> Result<Vec<Self::View>>;
        proximity_18(v: Self::View) -> Result<i32>;
        model_78(v: Self::View) -> Result<Option<pick::Model>>;
        selection_radius_469ac0(v: Self::View) -> Result<u32>;
        update_model_86e8d0(v: Self::View) -> Result<pick::Model>;
        viewport_b396d0() -> Result<(i32, i32)>;
    }
    fn camera(&mut self) -> Result<&Self::Camera> {
        self.host.camera()
    }
}
impl<H: Host> ui_world::Host for Adapter<'_, H> {
    fn find_object_at_43d240(&mut self, o: [u32; 3], d: [u32; 3], hits: Option<&mut Vec<Self::View>>) -> Result<Option<Self::View>> {
        pick::find_object_at_43d240(self, o, d, hits)
    }
    fn box_frustum_436dd0(&mut self, start: &Event, end: &Event) -> Result<()> {
        let p = self.host.precision();
        let camera = self.host.camera()?;
        self.planes = Some(pick::box_planes_436dd0(camera, start.world, end.world, p)?);
        Ok(())
    }
    fn box_frustum_4371a0(&mut self, rect: [i32; 4]) -> Result<()> {
        let p = self.host.precision();
        let viewport = self.host.viewport_b396d0()?;
        let camera = self.host.camera()?;
        self.planes = Some(pick::screen_box_planes_4371a0(camera, viewport, rect, p)?);
        Ok(())
    }
    /// 0x4367A0's `+0x18 == 2`, vt+0x9C, then 0x454B50 -> 0x715D40 (a NULL
    /// model +0x78 is the native NULL dereference).
    fn box_inside_4367a0(&mut self, v: Self::View) -> Result<bool> {
        if self.host.proximity_18(v)? != 2 || !self.host.pickable_9c(v)? {
            return Ok(false);
        }
        let planes = self.planes.ok_or("0x4367A0 without its planes")?;
        let model = self.host.model_78(v)?.ok_or("0x454B50 reads VObject+0x78 of a view without a model (native NULL dereference)")?;
        let p = self.host.precision();
        let host = &mut *self.host;
        pick::model_in_planes_715d40(&model, &planes, p, &mut || host.update_model_86e8d0(v))
    }
    fn vworld_list_94(&mut self) -> Result<Vec<Self::View>> {
        self.host.on_list_100()
    }
    /// UI+0x60 vt+0x64: VWorld 0x43DC70; other widgets' (base 0x5EDE60
    /// returns NULL) are the host's.
    fn pick_under_cursor_43dc70(&mut self, x: i32, y: i32) -> Result<Option<Self::View>> {
        match self.widget {
            Widget::None => Ok(None),
            Widget::World => {
                let single = self.single;
                pick::pick_at_pixel_43dc70(self, [x, y], single)
            }
            Widget::Map | Widget::Other(_) => self.host.widget_pick_64(x, y),
        }
    }
    /// 0x5C1E70: vt+0x20 of the current state (box 0x6A71F0 false; click,
    /// facing, camera and default 0x6A71D0 true).
    fn hover_enabled_5c1e70(&mut self) -> Result<bool> {
        Ok(!matches!(self.machine_current, Current::Modal(Modal::DragBox) | Current::None))
    }
    /// 0x5C1970: every installed game/camera state uses 0x5E05D0.
    fn choose_5c1970(&mut self, a: Option<Self::View>, da: u32, b: Option<Self::View>, db: u32) -> Result<Option<Self::View>> {
        if self.machine_current == Current::None {
            return less_or_b(a, da, b, db);
        }
        choose_5e05d0(self, a, da, b, db)
    }
    /// The 0x5C19E0 tail: vt+0x18 of the current state.
    fn state_pick_filter_18(&mut self, v: Self::View) -> Result<Option<bool>> {
        Ok(match self.machine_current {
            Current::None => None,
            // 0xA4D968 vt+0x18 = 0x969B10.
            Current::Camera => Some(false),
            // 0x60CA00 / 0x623320: 0x45B470.
            _ => Some(Selection::<H::View>::selectable_45b470(self, v)?),
        })
    }
}

/// The unstated-UI fallback of 0x5C1970: `fld a; fcomp b; test ah,5; jnp`.
fn less_or_b<V>(a: Option<V>, da: u32, b: Option<V>, db: u32) -> Result<Option<V>> {
    let lt = super::x87::lt(&super::x87::b(da)?, &super::x87::b(db)?);
    Ok(if lt { a } else { b })
}
/// 0x5E05D0: prefer the object whose actor has no live +0x94 link, otherwise
/// the nearer (`da < db ? a : b`; unordered picks b).
pub fn choose_5e05d0<H: Leaves>(
    host: &mut H,
    a: Option<H::View>,
    da: u32,
    b: Option<H::View>,
    db: u32,
) -> Result<Option<H::View>> {
    if let Some(av) = a {
        if host.actor_45b7a0(av)? && host.link_94_live(av)? {
            if b.is_some() {
                return Ok(b);
            }
            return less_or_b(a, da, b, db);
        }
    }
    if let Some(bv) = b {
        if host.actor_45b7a0(bv)? && host.link_94_live(bv)? && a.is_some() {
            return Ok(a);
        }
    }
    less_or_b(a, da, b, db)
}

#[cfg(test)]
#[path = "states_tests.rs"]
mod tests;
