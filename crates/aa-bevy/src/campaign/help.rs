//! The help popups of the World War II campaign: MetaGameHelpPopup
//! (`MetaGame/UI/metagame_help_popup.tgi`, children `0x94A5C0`, command
//! `CloseMetaGamePopupHelp` `0x94A2B0`) over `MetaGame/mg_help_popup.tgi`
//! (`KKC_MGHelpPopupData`, the type enum `0x95FA90`).
//!
//! `0x94A170(type, actor data)`: a popup of the type that lists the actor data goes
//! first; only when none does, the popups of the type without a list join the queue.
//! `0x94A060`: with nothing on show, the next queued popup the local kingdom has not
//! seen (`0x95A0D0`, Kingdom+2C) shows, while UVar MetaGameUIShowHelpPopup is set
//! (`[B39940]+330`), and the order MGHelpPopupShownOrder (`0x94A380`) adds it to the
//! kingdom's list. Closing stores the toggle in the UVar and shows the next.
//!
//! The calls (`0x933C50`, `0x9331A0`, `0x94F2F0`): welcome when the interface starts;
//! your_turn / other_turn at a turn change; select_army with the army's definition
//! and select_territory on a new selection; the result of a battle: attack success
//! or failure on the local player's turn, else defend success or failure for a local
//! defender; start_battle_available when a battle waits on the local turn;
//! start_battle_selected or watch_battle_selected when the Start Battle dialog opens.
use super::Game;
use aa_ui::ui::Ui;
use aa_ui::widget::{Widget, WidgetClass as C};
use std::collections::VecDeque;

const FILE: &str = "MetaGame/UI/metagame_help_popup.tgi:MetaGameHelpPopup";
const ROOT: &str = "MetaGameHelpPopup";
const SHOW: &str = "MetaGameUIShowHelpPopup";
const TAG_MESSAGE: u64 = 9960;

/// The type enum (`0x95FA90`), as `type` in the data.
pub const WELCOME: &str = "welcome";
pub const YOUR_TURN: &str = "your_turn";
pub const OTHER_TURN: &str = "other_turn";
pub const SELECT_ARMY: &str = "select_army";
pub const SELECT_TERRITORY: &str = "select_territory";
pub const START_BATTLE_AVAILABLE: &str = "start_battle_available";
pub const START_BATTLE_SELECTED: &str = "start_battle_selected";
pub const WATCH_BATTLE_SELECTED: &str = "watch_battle_selected";
pub const ATTACK_SUCCESS: &str = "territory_attack_success";
pub const ATTACK_FAILURE: &str = "territory_attack_failure";
pub const DEFEND_SUCCESS: &str = "territory_defend_success";
pub const DEFEND_FAILURE: &str = "territory_defend_failure";

/// One `[MGHelpPopup]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Popup {
    pub ids: String,
    pub kind: String,
    pub name: String,
    pub message: String,
    pub actors: Vec<String>,
}

#[derive(Default)]
pub struct Help {
    popups: Vec<Popup>,
    loaded: bool,
    /// +C0 / +C4.
    queue: VecDeque<usize>,
    /// +BC.
    current: Option<usize>,
    /// What the calls last saw.
    seen_kingdom: Option<usize>,
    seen_selection: Option<(Option<u32>, Option<usize>)>,
    seen_pending: bool,
    started: bool,
}

/// `0x94A170`'s choice: the popups of the kind for the actor data, in data order.
pub fn chosen(popups: &[Popup], kind: &str, actor: Option<&str>) -> Vec<usize> {
    if let Some(actor) = actor {
        let specific: Vec<usize> = (0..popups.len())
            .filter(|&i| popups[i].kind.eq_ignore_ascii_case(kind) && popups[i].actors.iter().any(|a| a.eq_ignore_ascii_case(actor)))
            .collect();
        if !specific.is_empty() {
            return specific;
        }
    }
    (0..popups.len()).filter(|&i| popups[i].kind.eq_ignore_ascii_case(kind) && popups[i].actors.is_empty()).collect()
}

impl Help {
    /// The IDS of `MetaGame/mg_help_popup.tgi` (the registry `[B4A074]` a saved
    /// kingdom's seen popups are read through, `95AA00`).
    pub fn all_ids(ui: &Ui) -> Vec<String> {
        crate::ui::datafile::load(&ui.store.root, "MetaGame/mg_help_popup.tgi").iter().filter(|c| c.name.eq_ignore_ascii_case("MGHelpPopup")).map(|n| n.get("ids").unwrap_or_default().trim_matches('"').to_string()).collect()
    }

    fn load(&mut self, ui: &Ui) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        for n in crate::ui::datafile::load(&ui.store.root, "MetaGame/mg_help_popup.tgi").iter().filter(|c| c.name.eq_ignore_ascii_case("MGHelpPopup")) {
            let get = |k: &str| n.get(k).unwrap_or_default().trim_matches('"').to_string();
            let actors = n.children_named("ActorData").into_iter().filter_map(|a| a.get("ids").map(str::to_string)).collect();
            self.popups.push(Popup { ids: get("ids"), kind: get("type"), name: get("name"), message: get("message"), actors });
        }
    }

    /// A popup is on the screen.
    pub fn shown(&self) -> bool {
        self.current.is_some()
    }

    /// `0x94A170`.
    pub fn ask(&mut self, ui: &mut Ui, game: &mut Game, kind: &str, actor: Option<&str>) {
        self.load(ui);
        self.queue.extend(chosen(&self.popups, kind, actor));
        self.next(ui, game);
    }

    /// `0x94A060`.
    fn next(&mut self, ui: &mut Ui, game: &mut Game) {
        while self.current.is_none() {
            let Some(i) = self.queue.pop_front() else { break };
            if !ui.uvars.get_bool(SHOW) {
                continue;
            }
            let Some(l) = game.campaign.local else { continue };
            let ids = self.popups[i].ids.clone();
            let kingdom = &mut game.campaign.state.kingdoms[l];
            if kingdom.help_popups.iter().any(|p| p.eq_ignore_ascii_case(&ids)) {
                continue;
            }
            // 0x94A380: the kingdom's list
            kingdom.help_popups.push(ids);
            self.current = Some(i);
        }
        let connected = ui.widget(&format!("Root/{ROOT}")).is_some();
        match self.current {
            Some(i) => {
                if !connected {
                    self.connect(ui);
                }
                self.fill(ui, i);
            }
            None => {
                if connected {
                    ui.disconnect(ROOT);
                    ui.layout();
                }
            }
        }
    }

    /// `0x94A5C0`.
    fn connect(&self, ui: &mut Ui) {
        let mut w = Widget::new(C::TopLevel, FILE, true);
        w.add_child(Widget::push_button("MetaGameButtonHelpPopupClose", false, "CloseMetaGamePopupHelp"));
        w.add_child(Widget::toggle_button("MetaGameButtonHelpPopupToggle", false, "HelpToggle"));
        w.add_child(Widget::new(C::Label, "MetaGameLabelHelpPopupTitle", false));
        w.add_child(Widget::composite(C::Viewport, "MetaGameViewportHelpPopupMessage", false));
        ui.load_widget(&mut w, None);
        ui.connect(w);
        ui.with_widget(&format!("{ROOT}/MetaGameButtonHelpPopupToggle"), |t| t.flags |= aa_ui::state::widget_flag::SELECTED);
        ui.layout();
    }

    /// `0x949D20`: the title and the message.
    fn fill(&self, ui: &mut Ui, i: usize) {
        let p = &self.popups[i];
        let title = ui.strings.resolve(&p.name);
        ui.set_label(&format!("{ROOT}/MetaGameLabelHelpPopupTitle"), &title);
        let viewport = format!("Root/{ROOT}/MetaGameViewportHelpPopupMessage");
        ui.viewport_clear(&viewport);
        let mut label = Widget::new(C::Label, "MetaGameLabelHelpPopupMessage", false);
        if let Ok(n) = ui.store.node(FILE) {
            let strings = ui.strings.clone();
            let resolve = |v: &str| strings.resolve(v);
            let root = std::sync::Arc::new(n.clone());
            label.load(&mut ui.store, Some((&n, &root)), Some(10), &resolve);
        }
        label.tag = TAG_MESSAGE;
        ui.viewport_add(&viewport, label);
        if let Some(path) = ui.find_tag(TAG_MESSAGE) {
            let text = ui.strings.resolve(&p.message);
            ui.set_label(path.strip_prefix("Root/").unwrap_or(&path), &text);
        }
        ui.viewport_refresh(&viewport);
        ui.layout();
    }

    /// The popup's commands: whether it was one.
    pub fn command(&mut self, ui: &mut Ui, game: &mut Game, command: &str) -> bool {
        match command {
            // 0x94A2B0: the toggle goes to the UVar
            "CloseMetaGamePopupHelp" => {
                let selected = ui
                    .widget(&format!("Root/{ROOT}/MetaGameButtonHelpPopupToggle"))
                    .is_some_and(|t| t.flags & aa_ui::state::widget_flag::SELECTED != 0);
                ui.uvars.set_bool(SHOW, selected);
                self.current = None;
                self.next(ui, game);
                true
            }
            "HelpToggle" => true,
            _ => false,
        }
    }

    /// The page was rebuilt: what is on show comes back.
    pub fn rebuilt(&mut self, ui: &mut Ui) {
        if let Some(i) = self.current {
            self.connect(ui);
            self.fill(ui, i);
        }
    }

    /// `0x9331A0`'s battle block: the result of a battle (`attacker_won`), for the
    /// local player's turn or a local defender.
    pub fn battle_result(&mut self, ui: &mut Ui, game: &mut Game, defender: Option<usize>, attacker_won: bool) {
        if game.campaign.local_turn() {
            self.ask(ui, game, if attacker_won { ATTACK_SUCCESS } else { ATTACK_FAILURE }, None);
        } else if defender.is_some() && defender == game.campaign.local {
            self.ask(ui, game, if attacker_won { DEFEND_FAILURE } else { DEFEND_SUCCESS }, None);
        }
    }

    /// `0x933C50` and `0x9331A0`, every interface update.
    pub fn update(&mut self, ui: &mut Ui, game: &mut Game) {
        let local_turn = game.campaign.local_turn();
        if !self.started {
            self.started = true;
            self.ask(ui, game, WELCOME, None);
        }
        let kingdom = game.campaign.state.current;
        if kingdom != self.seen_kingdom {
            self.seen_kingdom = kingdom;
            if kingdom.is_some() {
                self.ask(ui, game, if local_turn { YOUR_TURN } else { OTHER_TURN }, None);
            }
        }
        let selection = (game.selected_army.map(|a| a.0), game.selected_territory);
        if self.seen_selection != Some(selection) {
            self.seen_selection = Some(selection);
            if let Some(a) = game.selected_army {
                let ids = game.campaign.data.armies[game.campaign.state.army(a).definition].ids.clone();
                self.ask(ui, game, SELECT_ARMY, Some(&ids));
            } else if selection.1.is_some() {
                self.ask(ui, game, SELECT_TERRITORY, None);
            }
        }
        // 93AB30 on the local player's turn
        let pending = local_turn && game.campaign.pending_battle().ok().flatten().is_some();
        if pending && !self.seen_pending {
            self.ask(ui, game, START_BATTLE_AVAILABLE, None);
        }
        self.seen_pending = pending;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn popup(ids: &str, kind: &str, actors: &[&str]) -> Popup {
        Popup { ids: ids.into(), kind: kind.into(), name: String::new(), message: String::new(), actors: actors.iter().map(|a| a.to_string()).collect() }
    }

    #[test]
    fn an_army_with_its_own_popup_takes_that_one() {
        let p = [popup("generic", SELECT_ARMY, &[]), popup("defensive", SELECT_ARMY, &["mg_usa_china"]), popup("t", SELECT_TERRITORY, &[])];
        assert_eq!(chosen(&p, SELECT_ARMY, Some("MG_USA_CHINA")), [1]);
        assert_eq!(chosen(&p, SELECT_ARMY, Some("mg_other")), [0]);
        assert_eq!(chosen(&p, SELECT_TERRITORY, None), [2]);
        assert!(chosen(&p, WELCOME, None).is_empty());
    }
}
