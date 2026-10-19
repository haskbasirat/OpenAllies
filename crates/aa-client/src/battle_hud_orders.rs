//! The HUD side of `battle_orders_input::HudLeaves`: KKC_UI's press pick and
//! widget routing, the +0xBC brush slot, the ActorCommand definitions and the
//! HUD/presentation leaves of the game UIWorldState (AA.exe).
//!
//! UI thread
//! * [`world_under_cursor`]: the `observe` verdict. `0x5C3BC0` on the HUD
//!   tree (aa_ui::press_pick, oracle research/development10/hud-orders):
//!   the press reaches the world only when the pick lands on VWorld, the
//!   Game interface's head child, and `0x5EFA80` admits it.
//! * [`HudFrame::observe`]: the UI+0x60 target of every press message of one
//!   `Observed` frame, in order (`PressTracker` keeps UI+0x60 across frames,
//!   as `0x5C3BC0` compares with it), plus the data proofs the routing needs.
//!
//! Simulation thread
//! * [`HudOrderLeaves`]: the HudLeaves the camera owner's
//!   `battle_orders_hud::Leaves` forwards its HUD methods to. Each leaf that
//!   receives an event first replays the frame's press targets up to that
//!   event's message, so UI+0x60 is the native one at that point.
//!
//! Leaves whose native body needs machine state the trait does not pass
//! (the selected list, `+0x40` static group, the hovered target) or a world
//! owner that is not ported return explicit errors naming what is missing.
//! Voices and the destination marker have no simulation effect; they are
//! queued in native order as [`Presentation`] requests after the native
//! early-outs, for their audio/VFX owners (none bound yet: each request is
//! reported as unpresented).
use crate::battle_orders_input::{HudLeaves, Message, Observed, Result, View};
use aa_orders::commands::CommandWire;
use aa_runtime::player_order_input::{
    event::{code, flag, Event},
    states::{CameraSlot, CommandDefinition},
    ui_world::StaticRef,
};
use aa_ui::press_pick::PressTarget;
use aa_ui::ui::Ui;
use aa_ui::widget::WidgetClass;

/// `0x5C3BC0` updates UI+0x60 for a left press (0x100 with flag bit0), a
/// left double click (0x108) and a right press (0x101 with bit0) only.
pub fn updates_ui_60(code_: u16, flags: u32) -> bool {
    (code_ == code::LEFT && flags & flag::PRESSED != 0)
        || code_ == code::LEFT_DOUBLE
        || (code_ == code::RIGHT && flags & flag::PRESSED != 0)
}

/// UI thread: whether a press at `cursor` (native client pixels, the aa-ui
/// screen of the HUD drawn over the battle image) reaches VWorld.
pub fn world_under_cursor(ui: &Ui, cursor: Option<[i32; 2]>) -> bool {
    let Some([x, y]) = cursor else { return false };
    let (Ok(x), Ok(y)) = (i16::try_from(x), i16::try_from(y)) else { return false };
    ui.press_target_5c3bc0(x, y) == PressTarget::VWorld
}

/// One press message and the UI+0x60 it leaves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PressPick {
    pub code: u16,
    pub flags: u32,
    pub x: i16,
    pub y: i16,
    pub target: PressTarget,
}

/// UI-thread state kept across frames.
#[derive(Debug, Default)]
pub struct PressTracker {
    /// UI+0x60 as the path of the widget (None = NULL).
    ui_60: Option<String>,
    /// The hotkey-binding proof, computed once from the loaded data.
    mouse_bindings: Option<std::result::Result<(), String>>,
}

/// The hotkey files the hotkey configuration can load
/// (Data/Localization/Hotkeys, hotkey_configuration_data.tgi): every `map`
/// key naming a mouse button other than Button3, or another mouse key than
/// MouseWheel, would be a widget hotkey `0x5EFC70` could match for a press
/// or release. Mouse keys are the `ButtonN` / `MouseWheel` / `Wheel` /
/// `MouseZ` names of the key table (localization `key_Button1`..).
const HOTKEY_FILES: [&str; 11] = [
    "Localization/Hotkeys/hotkeys_core.txt",
    "Localization/Hotkeys/hotkeys_custom.txt",
    "Localization/Hotkeys/hotkeys_favorites.txt",
    "Localization/Hotkeys/hotkeys_glyphs.txt",
    "Localization/Hotkeys/hotkeys_groups.txt",
    "Localization/Hotkeys/hotkeys_ui.txt",
    "Localization/Hotkeys/start.txt",
    "Localization/Hotkeys/Actions/hotkeys_standard_aa.txt",
    "Localization/Hotkeys/Actions/hotkeys_visual_aa.txt",
    "Localization/Hotkeys/Actions/hotkeys_visual_dvorak_aa.txt",
    "Localization/Hotkeys/Actions/hotkeys_visual_left_handed_aa.txt",
];

fn mouse_binding_proof(ui: &Ui) -> std::result::Result<(), String> {
    for file in HOTKEY_FILES {
        let Ok(bytes) = ui.store.root.read(file) else { continue };
        let text = aa_ui::data::decode(&bytes);
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let mut words = line.split_whitespace();
            if !words.next().is_some_and(|w| w.eq_ignore_ascii_case("map")) {
                continue;
            }
            let Some(key) = words.next() else { continue };
            let k = key.trim_matches('"').to_ascii_lowercase();
            let last = k.rsplit('-').next().unwrap_or(&k).trim_start_matches(['+', '-']);
            let mouse = (last.starts_with("button") && last != "button3")
                || (last.contains("wheel") && last != "mousewheel")
                || last == "mousez";
            if mouse {
                return Err(format!("{file}: hotkey `{key}` binds a mouse key 0x5EFC70 may match (native dispatcher not ported)"));
            }
        }
    }
    Ok(())
}

/// One Observed frame's HUD facts (Send), built on the UI thread.
#[derive(Clone, Debug)]
pub struct HudFrame {
    /// UI+0x60 before the frame's first press.
    pub start_60: PressTarget,
    /// Each UI+0x60-updating press message in order.
    pub presses: Vec<PressPick>,
    /// The focus head `[UI+0x8C]` is the Game interface.
    pub focus_is_game: bool,
    /// No hotkey binds a mouse button 0x5EFC70 could match (see
    /// [`HOTKEY_FILES`]).
    pub mouse_bindings: std::result::Result<(), String>,
    /// GameInterface+0x12C EconomicImpact exists (AVar UIUseEconomicImpact).
    pub economic_impact: bool,
    /// `[B39940]+0x250` UnitAcknowledgement.
    pub unit_acknowledgement: std::result::Result<bool, String>,
}

impl HudFrame {
    /// The frame's press targets (`0x5C3BC0`, UI+0x60 carried in `tracker`).
    pub fn observe(ui: &Ui, tracker: &mut PressTracker, observed: &Observed) -> Self {
        let pick = ui.press_pick();
        let mut current = tracker.ui_60.as_deref().and_then(|p| pick.index_of(p));
        let start_60 = pick.target(current);
        let mut presses = Vec::new();
        for m in &observed.messages {
            if let Message::Button { code, flags, x, y, .. } = *m {
                if updates_ui_60(code, flags) {
                    current = pick.press(current, x, y);
                    presses.push(PressPick { code, flags, x, y, target: pick.target(current) });
                }
            }
        }
        tracker.ui_60 = current.map(|i| pick.paths[i].clone());
        let focus_is_game = pick.focus_8c.is_some_and(|f| pick.paths[f].eq_ignore_ascii_case(aa_ui::hud::GAME));
        let mouse_bindings = tracker.mouse_bindings.get_or_insert_with(|| mouse_binding_proof(ui)).clone();
        HudFrame {
            start_60,
            presses,
            focus_is_game,
            mouse_bindings,
            economic_impact: ui.widget(&format!("{}/EconomicImpact", aa_ui::hud::GAME)).is_some(),
            unit_acknowledgement: ui.uvars.unit_acknowledgement_current(),
        }
    }
}

/// A presentation request (no simulation effect), queued after the native
/// early-outs for its audio/VFX owner.
#[derive(Clone, Debug, PartialEq)]
pub enum Presentation {
    /// `0x436850`: the selection voice of `view` (0x45B7A0 actor +0x7C ->
    /// +0x40 -> +0xA4 voice owner +0x0C, its +0x14 and 0x58CB30, then
    /// 0x42A590(+0xCC, slot, 0)).
    SelectionVoice(View),
    /// `0x61EEF0`'s acknowledgement voice for the sent command (0x436600
    /// voiced object; +0xDC attack voice when flag 1 and the 0x530F10
    /// target's vt+0x140(local kingdom) hold, else the +0xD4 move voice).
    AcknowledgeVoice(Box<CommandWire>),
    /// VWorld `0x43DB10(x, y)`: the destination marker (VWorld+0x1C0; 0x43D050
    /// ground height + offset, 0x43CE30).
    Marker([u32; 2]),
}

/// Persistent per-battle HUD order state (simulation thread).
#[derive(Debug, Default)]
pub struct HudOrderState {
    /// UI+0x60 as replayed on this thread.
    ui_60: Option<PressTarget>,
    /// GameInterface+0xD8 default state +4 (`0x5E0690`'s pressed widget).
    state_4: Option<PressTarget>,
    /// `[[B36530]+0x298][index]` as loaded (39 predefined ActorCommands).
    definitions: Option<Vec<CommandDefinition>>,
    /// UIWorldState+0x30 brush +8 (`0x42D4D0`): the action buttons' refresh
    /// request.
    pub brush_dirty_8: bool,
    /// `0x4325C0(3)` on VWorld+0x140: its list 3 refresh byte (the owner of
    /// VWorld+0x140 is not ported; its presence is not proved).
    pub vworld_list_3_dirty: bool,
    /// Requests for the voice/marker owners, in native order.
    pub presentation: Vec<Presentation>,
}

impl HudOrderState {
    pub fn take_presentation(&mut self) -> Vec<Presentation> {
        std::mem::take(&mut self.presentation)
    }
}

/// The HUD leaves for one orders frame (simulation thread).
pub struct HudOrderLeaves<'a, 'c> {
    state: &'a mut HudOrderState,
    frame: &'a HudFrame,
    messages: &'a [Message],
    /// Next unreplayed message, and the next unreplayed press.
    cursor: usize,
    press: usize,
    /// B396C0 == 3: 0x59A6E0 drops every mouse message (UI+0x60 unchanged).
    dropped: bool,
    /// Session+E4 (0x436850 speaks only when it is not 3).
    session_e4: std::result::Result<u32, String>,
    catalog: Option<&'a aa_runtime::catalog::Catalog<'c>>,
}

impl<'a, 'c> HudOrderLeaves<'a, 'c> {
    pub fn new(
        state: &'a mut HudOrderState,
        frame: &'a HudFrame,
        observed: &'a Observed,
        application_kind_b396c0: u32,
        session_e4: std::result::Result<u32, String>,
        catalog: Option<&'a aa_runtime::catalog::Catalog<'c>>,
    ) -> Self {
        if state.ui_60.is_none() {
            state.ui_60 = Some(frame.start_60.clone());
        }
        Self {
            state,
            frame,
            messages: &observed.messages,
            cursor: 0,
            press: 0,
            dropped: application_kind_b396c0 == 3,
            session_e4,
            catalog,
        }
    }
    fn ui_60(&self) -> PressTarget {
        self.state.ui_60.clone().unwrap_or(PressTarget::None)
    }
    fn apply(&mut self, index: usize) {
        if let Message::Button { code, flags, .. } = self.messages[index] {
            if !self.dropped && updates_ui_60(code, flags) {
                if let Some(p) = self.frame.presses.get(self.press) {
                    self.state.ui_60 = Some(p.target.clone());
                }
                self.press += 1;
            }
        }
    }
    /// Replay the press targets up to the message `e` came from.
    fn sync_to(&mut self, e: &Event) -> Result<()> {
        let at = (self.cursor..self.messages.len()).find(|&i| match self.messages[i] {
            Message::Button { code, flags, x, y, .. } => code == e.code && x == e.x && y == e.y && flags & 3 == e.flags & 3,
            Message::Modifier { .. } => false,
        });
        let Some(at) = at else {
            return Err(format!("UI+0x60 replay: no observed message for event {:#x} at {},{}", e.code, e.x, e.y));
        };
        for i in self.cursor..=at {
            self.apply(i);
        }
        self.cursor = at + 1;
        Ok(())
    }
    /// The press at pixel (x, y) being pumped (0x5C3BC0's 0x436650 hover).
    fn sync_to_press_at(&mut self, x: i32, y: i32) {
        let at = (self.cursor..self.messages.len()).find(|&i| match self.messages[i] {
            Message::Button { code, flags, x: mx, y: my, .. } => updates_ui_60(code, flags) && i32::from(mx) == x && i32::from(my) == y,
            Message::Modifier { .. } => false,
        });
        if let Some(at) = at {
            for i in self.cursor..=at {
                self.apply(i);
            }
            self.cursor = at + 1;
        }
    }
    /// `0x5EFFF0` from `target` (the widget chain up to Root).
    fn chain(&self, target: &PressTarget, e: &Event) -> Result<bool> {
        match target {
            // VWorld vt+0x34 0x43B400 -> 0x969B10 (false); Game (A5B0B0) and
            // Root (A52C60) vt+0x34 0x5EFC70: no mouse hotkey -> false
            PressTarget::VWorld => self.hotkeys(e).map(|()| false),
            PressTarget::None => Err("0x5EFFF0 on a NULL widget (native NULL dereference)".into()),
            PressTarget::MiniMap { path } => Err(format!("0x5EFFF0 minimap chain {path}: vt+0x34 0x5D77A0 runs in the UI thread's minimap")),
            PressTarget::Widget { path, .. } => Err(format!("0x5EFFF0 HUD widget chain {path}: its verdict belongs to the UI thread")),
        }
    }
    fn hotkeys(&self, _e: &Event) -> Result<()> {
        self.frame.mouse_bindings.clone()
    }
    fn unit_acknowledgement(&self) -> Result<bool> {
        self.frame.unit_acknowledgement.clone()
    }
    fn definitions(&mut self) -> Result<&[CommandDefinition]> {
        if self.state.definitions.is_none() {
            let catalog = self.catalog.ok_or("[[B36530]+0x298] ActorCommand definitions: no Catalog bound")?;
            let mut out = Vec::new();
            for (index, d) in aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS.iter().enumerate() {
                // 4B4E60 stores the registration index at +0C
                if d.mind_type as usize != index {
                    return Err(format!("ActorCommand {} registration index {index} differs from its type", d.id));
                }
                let source = aa_runtime::actor_command_ui::Source::bind(catalog, d.id)?;
                out.push(CommandDefinition { byte_40: source.controls().can_hot_command_location(), flags_2c: d.behavior_flags });
            }
            self.state.definitions = Some(out);
        }
        Ok(self.state.definitions.as_deref().expect("set above"))
    }
}

/// The camera owner's leaves (battle_orders_hud.rs).
fn camera_owner<T>(what: &str) -> Result<T> {
    Err(format!("{what}: camera owner leaf (battle_orders_hud::Leaves), not the HUD"))
}

impl HudLeaves for HudOrderLeaves<'_, '_> {
    /// `0x5E0690` on the default state (GameInterface+0xD8).
    fn widget_route_5e0690(&mut self, e: &Event) -> Result<bool> {
        self.sync_to(e)?;
        let ui_60 = self.ui_60();
        let none = |t: &PressTarget| *t == PressTarget::None;
        if e.code == code::LEFT && e.flags & flag::RELEASED != 0 {
            // a left release whose press +4 is NULL or another widget is
            // swallowed
            match &self.state.state_4 {
                Some(s4) if *s4 == ui_60 && !none(s4) => {}
                _ => {
                    self.state.state_4 = None;
                    return Ok(true);
                }
            }
        }
        if (e.code == code::LEFT && e.flags & flag::PRESSED != 0) || e.code == code::LEFT_DOUBLE {
            self.state.state_4 = (!none(&ui_60)).then(|| ui_60.clone());
        }
        if !none(&ui_60) {
            // UI+0x60 chain, then Root's own chain (5EFC70: no mouse hotkey)
            if self.chain(&ui_60, e)? {
                return Ok(true);
            }
            self.hotkeys(e)?;
            return Ok(false);
        }
        // UI+0x60 NULL: the focus head's chain, then Root's
        if !self.frame.focus_is_game {
            return Err("0x5E0690 focus chain [UI+0x8C]: the focus head is a dialog, routed by the UI thread".into());
        }
        self.hotkeys(e)?;
        Ok(false)
    }
    /// `0x5EFFF0` on UI+0x60 inside the modal base `0x5E0910`.
    fn pressed_widget_chain_5efff0(&mut self, e: &Event) -> Result<bool> {
        self.sync_to(e)?;
        let ui_60 = self.ui_60();
        self.chain(&ui_60, e)
    }
    /// UI+0x60 vt+0x64 for a non-world widget: 0x5EDE60 (NULL) for every
    /// class except the decal widgets (A52770 0x5EF4F0: the widget+0x70
    /// handle through B364C0).
    fn widget_pick_64(&mut self, x: i32, y: i32) -> Result<Option<View>> {
        self.sync_to_press_at(x, y);
        match self.ui_60() {
            PressTarget::MiniMap { .. } => Ok(None),
            PressTarget::Widget { class: WidgetClass::Decal, path } => {
                Err(format!("0x5EF4F0 decal widget {path}: its +0x70 object handle is not bound"))
            }
            PressTarget::Widget { .. } => Ok(None),
            PressTarget::VWorld => Err("vt+0x64 of VWorld is 0x43DC70, the machine's own pick".into()),
            PressTarget::None => Err("vt+0x64 on a NULL UI+0x60 (native NULL dereference)".into()),
        }
    }
    /// UI+0xBC is written non-NULL only by 0x5C1F90 <- 0x5E0510 (brush
    /// install), reached from the action/brush handlers 0x6200A0, 0x6200C0,
    /// 0x6201E0, 0x620B20, 0x621C23, 0x689150 and the editor 0x6B8xxx; none
    /// is ported and the HUD action buttons are unbound, so the slot holds
    /// the 0x5CAE30 constructor's NULL.
    fn brush_installed_bc(&mut self) -> Result<bool> {
        Ok(false)
    }
    fn camera_slot_c0(&mut self) -> Result<CameraSlot> {
        camera_owner("UI+0xC0")
    }
    fn camera_frame_10(&mut self) -> Result<bool> {
        camera_owner("UI+0xC0 vt+0x10")
    }
    fn camera_frame_8(&mut self) -> Result<()> {
        camera_owner("UI+0xC0 vt+8")
    }
    fn map_world_5d74b0(&mut self, _cursor: [i32; 2]) -> Result<[u32; 2]> {
        Err("0x5D74B0 minimap world XY: minimap presses are routed by the UI thread's minimap (observe marks them HUD)".into())
    }
    fn map_auxiliary_b279c0(&mut self) -> Result<[u32; 2]> {
        Err("B279C0/B279C4 minimap order words: minimap presses are routed by the UI thread".into())
    }
    /// The default state's vt+8 tail (0x620648..0x62074D). Without
    /// GameInterface+0x12C (EconomicImpact) it does nothing; with UI+0x60
    /// other than VWorld only the hide branch runs, which needs
    /// EconomicImpact+0xD4 to be this state, which only this function's show
    /// branch (0x603B50 -> 0x603A00 -> 0x602640) stores and which is never
    /// reached here.
    fn cursor_feedback_620640(&mut self) -> Result<()> {
        if !self.frame.economic_impact || self.ui_60() != PressTarget::VWorld {
            return Ok(());
        }
        Err("0x620640 EconomicImpact target preview: needs the UIWorldState selection head, 0x436510 and the 0x4363B0 hovered target, which HudLeaves does not pass".into())
    }
    fn selection_sound_436850(&mut self, v: View) -> Result<()> {
        if self.session_e4.clone()? == 3 || !self.unit_acknowledgement()? {
            return Ok(());
        }
        self.state.presentation.push(Presentation::SelectionVoice(v));
        Ok(())
    }
    /// `0x437EB0`: clears UIWorldState+0x20..+0x2C; an empty selection stops
    /// there.
    fn command_masks_437eb0(&mut self, selected: &[View], _group: StaticRef, masks: &mut [u32; 4]) -> Result<()> {
        *masks = [0; 4];
        if selected.is_empty() {
            return Ok(());
        }
        Err("0x437EB0 command masks: the selected actors' vt+0x154(local kingdom, index, 1) and +0x7C/+0x100 reads have no live accessor".into())
    }
    fn brush_changed_42d4d0(&mut self) -> Result<()> {
        self.state.brush_dirty_8 = true;
        Ok(())
    }
    fn vworld_list_changed_4325c0(&mut self) -> Result<()> {
        self.state.vworld_list_3_dirty = true;
        Ok(())
    }
    fn vworld_ad(&mut self) -> Result<bool> {
        camera_owner("VWorld+0xAD")
    }
    fn drag_threshold_b39990(&mut self) -> Result<i32> {
        camera_owner("[B39990+0x60]")
    }
    fn box_rectangle_begin(&mut self, _press: &Event) -> Result<()> {
        camera_owner("0x5CD850")
    }
    fn box_rectangle_frame(&mut self) -> Result<()> {
        camera_owner("0x5CCA80")
    }
    fn box_rectangle_end(&mut self) -> Result<()> {
        camera_owner("0x5CCA30")
    }
    fn facing_begin_689380(&mut self, _selected: &[View]) -> Result<()> {
        camera_owner("0x689380")
    }
    fn facing_drag_start(&mut self) -> Result<()> {
        camera_owner("0x688AE2")
    }
    fn facing_frame(&mut self, _facing: [u32; 3], _rotated: bool) -> Result<()> {
        camera_owner("0x688A43")
    }
    fn facing_end_6885f0(&mut self) -> Result<()> {
        camera_owner("0x6885F0")
    }
    fn marker_43db10(&mut self, xy: [u32; 2]) -> Result<()> {
        self.state.presentation.push(Presentation::Marker(xy));
        Ok(())
    }
    fn acknowledge_61eef0(&mut self, wire: &CommandWire) -> Result<()> {
        if !self.unit_acknowledgement()? {
            return Ok(());
        }
        self.state.presentation.push(Presentation::AcknowledgeVoice(Box::new(wire.clone())));
        Ok(())
    }
    fn can_send_436950(&mut self, _wire: &mut CommandWire) -> Result<bool> {
        Err("0x436950 CanSend: needs UIWorldState+0x40 (static group) and the selected list for each actor's vt+0x150 \
             (player_order_world::authorize_selected_command_in_bank), which HudLeaves does not pass"
            .into())
    }
    fn command_definition_298(&mut self, index: u32) -> Result<Option<CommandDefinition>> {
        Ok(self.definitions()?.get(index as usize).copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(presses: Vec<PressPick>) -> HudFrame {
        HudFrame {
            start_60: PressTarget::None,
            presses,
            focus_is_game: true,
            mouse_bindings: Ok(()),
            economic_impact: true,
            unit_acknowledgement: Ok(true),
        }
    }
    fn button(code: u16, flags: u32, x: i16, y: i16) -> Message {
        Message::Button { code, flags, x, y, widget: aa_runtime::player_order_input::states::Widget::World }
    }
    fn event(code: u16, flags: u32, x: i16, y: i16) -> Event {
        Event { x, y, flags: flags | flag::MOUSE, code, delta: 0, world: [0; 3], origin: [0; 3], direction: [0; 3] }
    }
    fn hud(path: &str) -> PressTarget {
        PressTarget::Widget { path: path.into(), class: WidgetClass::PushButton }
    }

    #[test]
    fn routing_follows_the_replayed_ui_60() {
        // left press on a HUD button, right press on the world, left release
        let observed = Observed {
            messages: vec![button(0x100, 1, 5, 700), button(0x101, 1, 300, 200), button(0x100, 2, 300, 200)],
            ..Default::default()
        };
        let f = frame(vec![
            PressPick { code: 0x100, flags: 1, x: 5, y: 700, target: hud("Root/Game/ControlPanel/ButtonPause") },
            PressPick { code: 0x101, flags: 1, x: 300, y: 200, target: PressTarget::VWorld },
        ]);
        let mut state = HudOrderState::default();
        // the HUD press set +4 natively; here the leaf saw it first through
        // the right press's replay, so +4 is still NULL and the release is
        // swallowed exactly as 0x5E0690 does for a NULL +4
        let mut l = HudOrderLeaves::new(&mut state, &f, &observed, 0, Ok(2), None);
        assert_eq!(l.widget_route_5e0690(&event(0x101, 1, 300, 200)), Ok(false));
        assert_eq!(l.ui_60(), PressTarget::VWorld);
        assert_eq!(l.widget_route_5e0690(&event(0x100, 2, 300, 200)), Ok(true));
        // a widget pick for a HUD press
        let observed = Observed { messages: vec![button(0x100, 1, 5, 700)], ..Default::default() };
        let f = frame(vec![PressPick { code: 0x100, flags: 1, x: 5, y: 700, target: hud("Root/Game/X") }]);
        let mut state = HudOrderState::default();
        let mut l = HudOrderLeaves::new(&mut state, &f, &observed, 0, Ok(2), None);
        assert_eq!(l.widget_pick_64(5, 700), Ok(None));
        assert!(l.pressed_widget_chain_5efff0(&event(0x100, 1, 5, 700)).is_err());
    }

    #[test]
    fn voices_and_markers_follow_the_native_early_outs() {
        let observed = Observed::default();
        let mut f = frame(vec![]);
        let mut state = HudOrderState::default();
        let wire = CommandWire::default();
        f.unit_acknowledgement = Ok(false);
        HudOrderLeaves::new(&mut state, &f, &observed, 0, Ok(2), None).acknowledge_61eef0(&wire).unwrap();
        assert!(state.presentation.is_empty());
        f.unit_acknowledgement = Ok(true);
        let mut l = HudOrderLeaves::new(&mut state, &f, &observed, 0, Ok(2), None);
        l.acknowledge_61eef0(&wire).unwrap();
        l.marker_43db10([1, 2]).unwrap();
        let mut masks = [7; 4];
        l.command_masks_437eb0(&[], StaticRef::None, &mut masks).unwrap();
        assert_eq!(masks, [0; 4]);
        assert!(!l.brush_installed_bc().unwrap());
        // no world press yet: the EconomicImpact tail is the hide branch
        l.cursor_feedback_620640().unwrap();
        assert!(l.command_definition_298(0).is_err());
        assert_eq!(state.take_presentation(), vec![Presentation::AcknowledgeVoice(Box::new(wire)), Presentation::Marker([1, 2])]);
    }
}
