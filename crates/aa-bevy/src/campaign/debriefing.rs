//! The debriefing after a scripted mission: KKC_DebriefingInterface, slot 4 of
//! the menus (`UI/Menus/debriefing.tgi:DebriefingInterface`, constructor
//! `0x600FE0`, CreateChildren `0x6014B0`, LoadData `0x601D00`, Activate `0x600D40`).
//!
//! CreateChildren, in AddChild order: `/MissionNameLabel` (+D8),
//! `/MissionStatusPanel` holding `MissionStatusViewport` with the multiline
//! `MissionStatusLabel` (+DC), the `/PrimaryObjectivesLabel` and panel, the
//! `/SecondaryObjectivesLabel` and panel (`0x673CD0`, each with a `SlotViewport`
//! and the `Slot` of `0x673980`: `NameLabel`, `StatusIcon`), the five buttons
//! (`DebriefingScoring`, `DebriefingReplay`, `DebriefingExit`, `DebriefingSaveFilm`,
//! `DebriefingPlayNext`) and the four `..MissionStatusDecal`s of the background.
//!
//! Activate: the campaign's name, then for the local kingdom's result (+19C == 1)
//! the mission's `debriefing_success` with the `*_won` decals, else its
//! `debriefing_failure` with the `*_lost` ones; PlayNext only for a completed
//! mission with another one after it; `0x673DC0` lists the world's objectives of
//! the local kingdom in the two panels (flag +10 primary), an entry's icon is
//! `icon_completed` for state 1 and `icon_failed` for any other (+7C / +80 of
//! `0x6737C0`); SaveFilm while the game's film exists (0x772BB0).
//!
//! The world is gone when this page shows here (the original keeps it behind the
//! menus until the next one starts), so [`MissionResult`] holds what Activate reads.
//!
//! The victory and defeat music (`0x747630` of the interface's `victory_music` /
//! `defeat_music`) are `null` in the shipped file, which does nothing: the game's music
//! goes on (`audio::menu_sound`).
use aa_ui::ui::Ui;
use aa_ui::widget::{Widget, WidgetClass as C};
use std::sync::Arc;

pub const FILE: &str = "UI/Menus/debriefing.tgi";
const IFACE: &str = "DebriefingInterface";
const TAG_SLOT: u64 = 9700;

/// One objective line of the two panels (`0x673DC0`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectiveLine {
    pub text: String,
    /// Definition +10.
    pub primary: bool,
    /// Entry +4: 1 complete.
    pub state: u8,
}

/// What Activate reads from the mission that ended.
#[derive(Clone, Debug, Default)]
pub struct MissionResult {
    /// Kingdom+19C of the local kingdom is 1.
    pub won: bool,
    pub objectives: Vec<ObjectiveLine>,
    /// 0x772BB0: the game's film exists.
    pub film: bool,
}

/// What Activate shows.
#[derive(Clone, Debug, Default)]
pub struct Shown {
    /// The campaign's name key (+D8).
    pub campaign_name: String,
    /// The mission's debriefing text key (+DC).
    pub text: String,
    /// `state[current] == 2 && current < count - 1`.
    pub play_next: bool,
    pub result: MissionResult,
}

fn label(name: &str) -> Widget {
    Widget::new(C::Label, name, false)
}

/// `0x6014B0`.
fn tree() -> Widget {
    let mut iface = Widget::new(C::Interface, &format!("{FILE}:{IFACE}"), false);
    iface.add_child(label("/MissionNameLabel"));
    let mut status = Widget::new(C::Panel, "/MissionStatusPanel", false);
    status.add_child(Widget::composite(C::Viewport, "MissionStatusViewport", false));
    iface.add_child(status);
    for (name, panel) in [("/PrimaryObjectivesLabel", "/PrimaryObjectivesPanel"), ("/SecondaryObjectivesLabel", "/SecondaryObjectivesPanel")] {
        iface.add_child(label(name));
        let mut p = Widget::new(C::Panel, panel, false);
        p.add_child(Widget::composite(C::Viewport, "SlotViewport", false));
        iface.add_child(p);
    }
    for (name, command) in [
        ("/ScoringButton", "DebriefingScoring"),
        ("/ReplayButton", "DebriefingReplay"),
        ("/ExitButton", "DebriefingExit"),
        ("/SaveFilmButton", "DebriefingSaveFilm"),
        ("/PlayNextButton", "DebriefingPlayNext"),
    ] {
        iface.add_child(Widget::push_button(name, false, command));
    }
    for name in ["/TopLeftMissionStatusDecal", "/TopRightMissionStatusDecal", "/BottomLeftMissionStatusDecal", "/BottomRightMissionStatusDecal"] {
        // virtual +D4: under the others
        iface.add_child_front(Widget::new(C::Decal, name, false));
    }
    iface
}

fn set_decal(ui: &mut Ui, path: &str, texture: Option<String>) {
    ui.with_widget(path, |w| {
        for ve in w.sub_ve.iter_mut().chain(w.view.iter_mut()) {
            if let aa_ui::ve::VeKind::Decal(d) = &mut ve.kind {
                d.texture = texture.clone();
            }
        }
    });
}

fn load_under(ui: &mut Ui, item: &mut Widget, parent: &str) {
    if let Ok(n) = ui.store.node(parent) {
        let strings = ui.strings.clone();
        let resolve = |v: &str| strings.resolve(v);
        let root = Arc::new(n.clone());
        item.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
    }
}

/// Build the page (`0x6014B0`, then `0x600D40`).
pub fn show(ui: &mut Ui, shown: &Shown) {
    ui.clear();
    let mut iface = tree();
    ui.load_widget(&mut iface, None);
    ui.connect(iface);
    let root = "Root/DebriefingInterface";
    let rel = "DebriefingInterface";
    let name = ui.strings.resolve(&shown.campaign_name);
    ui.set_label(&format!("{rel}/MissionNameLabel"), &name);
    // the mission's text in the status viewport
    let viewport = format!("{root}/MissionStatusPanel/MissionStatusViewport");
    let mut item = label("MissionStatusLabel");
    load_under(ui, &mut item, &format!("{FILE}:MissionStatusPanel/MissionStatusViewport/ItemPanel"));
    item.tag = TAG_SLOT - 1;
    ui.viewport_add(&viewport, item);
    if let Some(p) = ui.find_tag(TAG_SLOT - 1) {
        let text = ui.strings.resolve(&shown.text);
        ui.set_label(p.strip_prefix("Root/").unwrap_or(&p), &text);
    }
    ui.viewport_refresh(&viewport);
    // the four decals of the result
    let suffix = if shown.result.won { "won" } else { "lost" };
    let node = ui.store.node(&format!("{FILE}:{IFACE}")).ok();
    for (widget, key) in [
        ("TopLeftMissionStatusDecal", "decal_top_left"),
        ("TopRightMissionStatusDecal", "decal_top_right"),
        ("BottomLeftMissionStatusDecal", "decal_bottom_left"),
        ("BottomRightMissionStatusDecal", "decal_bottom_right"),
    ] {
        let texture = node.as_ref().map(|n| n.string_or_empty(&format!("{key}_{suffix}"))).filter(|s| !s.is_empty());
        set_decal(ui, &format!("{rel}/{widget}"), texture);
    }
    ui.with_widget(&format!("{rel}/PlayNextButton"), |w| w.set_visible(shown.play_next));
    ui.with_widget(&format!("{rel}/SaveFilmButton"), |w| w.set_visible(shown.result.film));
    // 0x673DC0(0) and (1): the panels' slots
    let mut tag = TAG_SLOT;
    for (panel, primary) in [("PrimaryObjectivesPanel", true), ("SecondaryObjectivesPanel", false)] {
        let panel_node = format!("{FILE}:{panel}");
        let viewport = format!("{root}/{panel}/SlotViewport");
        let icons = ui.store.node(&panel_node).ok();
        let completed = icons.as_ref().map(|n| n.string_or_empty("icon_completed")).filter(|s| !s.is_empty());
        let failed = icons.as_ref().map(|n| n.string_or_empty("icon_failed")).filter(|s| !s.is_empty());
        for line in shown.result.objectives.iter().filter(|o| o.primary == primary) {
            let mut slot = Widget::new(C::Panel, "Slot", false);
            slot.add_child(label("NameLabel"));
            slot.add_child(Widget::new(C::Decal, "StatusIcon", false));
            load_under(ui, &mut slot, &panel_node);
            slot.tag = tag;
            ui.viewport_add(&viewport, slot);
            if let Some(p) = ui.find_tag(tag) {
                let p = p.strip_prefix("Root/").unwrap_or(&p).to_string();
                ui.set_label(&format!("{p}/NameLabel"), &line.text);
                set_decal(ui, &format!("{p}/StatusIcon"), if line.state == 1 { completed.clone() } else { failed.clone() });
            }
            tag += 1;
        }
        ui.viewport_refresh(&viewport);
    }
    ui.layout();
}

/// What a debriefing command asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Out {
    /// `0x600C70`: the score screen (slot 5).
    Scoring,
    /// `0x600C90` -> `0x644130`: the mission again.
    Replay,
    /// `0x600CB0`: the chooser (slot 1).
    Exit,
    /// `0x600D00` -> `0x6413B0`: the next mission.
    PlayNext,
    /// `DebriefingSaveFilm`: the film dialog of the score screen (`0x68DCE0(5, 0, 0)`).
    SaveFilm,
}

/// The commands of the constructor `0x600FE0`.
pub fn command(command: &str) -> Option<Out> {
    Some(match command.split_whitespace().next()? {
        "DebriefingScoring" => Out::Scoring,
        "DebriefingReplay" => Out::Replay,
        "DebriefingExit" => Out::Exit,
        "DebriefingPlayNext" => Out::PlayNext,
        "DebriefingSaveFilm" => Out::SaveFilm,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_commands_are_the_constructors() {
        use super::{command, Out};
        assert_eq!(command("DebriefingPlayNext"), Some(Out::PlayNext));
        assert_eq!(command("DebriefingExit"), Some(Out::Exit));
        assert_eq!(command("CampaignExit"), None);
    }
}
