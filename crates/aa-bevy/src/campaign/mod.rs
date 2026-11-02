//! The World War II campaign (the original's metagame) on Bevy: the
//! campaign chooser, the strategic map with its interface, and the engine
//! side that runs `aa_metagame` (the ported KKC_MGState rules, commands,
//! battle resolution and the MGAI) through `aa_metagame::driver`.
//!
//! Flow (AA.exe): Single Player "World War II" -> ConfirmPlayDynamicCampaign
//! (65B780: the dependent-campaign confirmation) -> RequestDynamicCampaign
//! (64F060) -> ChooseDynamicCampaign (942AE0) -> DynamicCampaignNew (941F90 ->
//! 941E80: SetDynamicCampaign / SetNewDynamicCampaign orders, game type 4,
//! 944A20 builds the MGState) -> the MetaGame interface (935130) over the
//! strategic map world.
//!
//! The front end (`ui::front`) owns the UI; it hands the campaign page's
//! builds, commands and frames to [`CampaignUi`]. The 3D strategic map is
//! drawn by [`map`] from the same state.
pub mod choose;
pub mod data;
pub mod interface;
pub mod map;

use aa_metagame::driver::{Campaign, Note, Options};
use aa_metagame::state::ArmyId;
use aa_ui::ui::Ui;
use bevy::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct CampaignPlugin;

impl Plugin for CampaignPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<map::MapScene>()
            .add_systems(Update, (map::sync, map::camera, map::pick).chain().after(crate::ui::front::frame));
    }
}

/// The campaign pages of the front end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    /// ChooseDynamicCampaign (942AE0).
    Choose,
    /// The MetaGame interface over the strategic map (935130).
    Map,
}

/// What a campaign command asks the front end to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    /// Back to the Single Player menu (DynamicCampaignExit 941A40 -> 651990).
    Exit,
    /// Leave the campaign to the main menu (RequestFinished).
    MainMenu,
    /// The page was rebuilt (another campaign screen).
    Rebuild,
    /// Quit the program (MetaGameButtonQuit -> QuitConfirm).
    QuitConfirm,
    /// Open the options dialog (OptionsDialog 935930).
    Options,
}

/// A running campaign with what the interface keeps about it.
pub struct Game {
    /// Identity of this campaign run (the map rebuilds its scene for a new one).
    pub id: u64,
    pub campaign: Campaign,
    pub data: Arc<data::CampaignData>,
    /// The selected army (KKC selection of the world) and territory.
    pub selected_army: Option<ArmyId>,
    pub selected_territory: Option<usize>,
    /// The hot mode (+0xE4, 956B30): a command (move / attack, 935A70 `& 0x70`)
    /// waiting for its territory; the next left click gives it and ends the
    /// mode (956E40), a right click only ends it (5CD28E).
    pub targeting: Option<aa_metagame::commands::Command>,
    /// Armies whose actor is still moving (`+0x70` vtable `+0x64`), set by
    /// the map's mover every frame.
    pub moving: HashSet<ArmyId>,
    /// Seconds not yet spent on world ticks.
    pub accumulator: f32,
    /// GamePause.
    pub paused: bool,
    /// The game speed (SpeedUp / SpeedDown).
    pub speed: f32,
    /// Move visuals (962880) the map has not started yet.
    pub visual_moves: Vec<(ArmyId, Option<usize>, usize)>,
}

impl Game {
    pub fn local_kingdom(&self) -> Option<usize> {
        self.campaign.local
    }
}

pub struct CampaignUi {
    /// The reconstruction folder (original Data).
    pub root: PathBuf,
    pub screen: Screen,
    pub choose: choose::Chooser,
    pub game: Option<Game>,
    pub iface: interface::MetaGameUi,
    data_error_shown: bool,
    last_ms: Option<u64>,
    hotkeys: Option<Vec<(String, String)>>,
}

/// The hotkey names (hotkeys_*.txt) of the keys the MetaGame binds.
fn key_name(code: bevy::input::keyboard::KeyCode) -> Option<&'static str> {
    use bevy::input::keyboard::KeyCode as K;
    Some(match code {
        K::Escape => "esc",
        K::Enter => "enter",
        K::Tab => "tab",
        K::KeyA => "a",
        K::KeyD => "d",
        K::KeyE => "e",
        K::KeyH => "h",
        K::KeyM => "m",
        K::KeyN => "n",
        K::KeyP => "p",
        K::KeyV => "v",
        _ => return None,
    })
}

/// `map <key> MetaGame <command...>` lines (the command may be quoted).
fn metagame_hotkeys(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let mut it = line.split_whitespace();
        if !it.next().is_some_and(|w| w.eq_ignore_ascii_case("map")) {
            continue;
        }
        let (Some(key), Some(context)) = (it.next(), it.next()) else { continue };
        if !context.eq_ignore_ascii_case("MetaGame") {
            continue;
        }
        let command = it.collect::<Vec<_>>().join(" ").trim_matches('"').to_string();
        if !command.is_empty() {
            out.push((key.to_ascii_lowercase(), command));
        }
    }
    out
}

impl CampaignUi {
    pub fn new(root: &Path) -> Self {
        let game_root = root.parent().map(Path::to_path_buf).unwrap_or_else(|| root.to_path_buf());
        data::warm(game_root);
        Self {
            root: root.to_path_buf(),
            screen: Screen::Choose,
            choose: choose::Chooser::default(),
            game: None,
            iface: interface::MetaGameUi::default(),
            data_error_shown: false,
            last_ms: None,
            hotkeys: None,
        }
    }

    /// `ConfirmPlayDynamicCampaign` (65B780): with UVar
    /// `UICampaignConfirmDependentCampaign` set and the AVar
    /// `MetaGameDependentCampaignIDS` campaign's progress record (411790,
    /// created for a new profile) not finished (`+4` nonzero, 1 when new), the
    /// confirmation `campaign_dependent_campaign_confirm|ss` (dependent campaign
    /// name, `metagame_campaign_name`) with Ok = "Uvar
    /// UICampaignConfirmDependentCampaign 0 ; RequestDynamicCampaign";
    /// otherwise RequestDynamicCampaign (64F060) at once. Returns true when
    /// the campaign chooser should be shown now. The campaign progress
    /// records are not kept by this client (the tutorial campaign is not
    /// built): the confirmation is always asked, until the player turns it off.
    pub fn confirm_play(&mut self, ui: &mut Ui) -> bool {
        if !ui.uvars.get_bool("UICampaignConfirmDependentCampaign") {
            return true;
        }
        let dependent = ui.avars.get("MetaGameDependentCampaignIDS").map(aa_ui::node::unquote).unwrap_or_default();
        if dependent.is_empty() {
            return true;
        }
        // the dependent campaign's name (campaign data +0x10)
        let name = ui.strings.resolve(&format!("#{dependent}_name"));
        let metagame = ui.strings.resolve("#metagame_campaign_name");
        let text = ui.strings.format("campaign_dependent_campaign_confirm|ss", &[&name, &metagame]);
        let ok = ui.strings.resolve("#button_ok");
        let cancel = ui.strings.resolve("#button_cancel");
        ui.show_message_box(
            aa_ui::message_box::SHARED,
            aa_ui::message_box::MessageBoxRequest {
                text,
                buttons: vec![(ok, "Uvar UICampaignConfirmDependentCampaign 0 ; RequestDynamicCampaign".into()), (cancel, String::new())],
            },
        );
        false
    }

    /// Whether a console command line belongs to the campaign (handled by
    /// [`Self::command`] from any front-end page).
    pub fn handles(command: &str) -> bool {
        command.split(';').any(|c| {
            let c = c.trim();
            c.eq_ignore_ascii_case("RequestDynamicCampaign") || c.to_ascii_lowercase().starts_with("uvar uicampaignconfirmdependentcampaign")
        })
    }

    /// Enter the chooser (RequestDynamicCampaign).
    pub fn enter(&mut self, ui: &mut Ui) {
        self.screen = Screen::Choose;
        self.choose.enter(ui);
    }

    /// Build the current campaign page.
    pub fn show(&mut self, ui: &mut Ui) {
        match self.screen {
            Screen::Choose => {
                let data = data::ready().and_then(Result::ok);
                self.choose.show(ui, data.as_deref());
            }
            Screen::Map => match &mut self.game {
                Some(g) => self.iface.show(ui, g),
                None => {
                    self.screen = Screen::Choose;
                    let data = data::ready().and_then(Result::ok);
                    self.choose.show(ui, data.as_deref());
                }
            },
        }
    }

    /// A console command (button, hotkey, message box) of the campaign pages.
    pub fn command(&mut self, ui: &mut Ui, command: &str, widget: &str) -> Option<Action> {
        // command lines ("a ; b") run in order
        if command.contains(';') {
            let mut last = None;
            for part in command.split(';') {
                if let Some(a) = self.command(ui, part.trim(), widget) {
                    last = Some(a);
                }
            }
            return last;
        }
        let mut words = command.split_whitespace();
        let head = words.next().unwrap_or("");
        if head.eq_ignore_ascii_case("Uvar") {
            let (Some(name), Some(value)) = (words.next(), words.next()) else { return Some(Action::None) };
            let mut overrides = std::collections::BTreeMap::new();
            overrides.insert(name.to_string(), value.to_string());
            ui.uvars.apply_overrides(&overrides);
            return Some(Action::None);
        }
        if head.eq_ignore_ascii_case("RequestDynamicCampaign") {
            self.enter(ui);
            return Some(Action::Rebuild);
        }
        match self.screen {
            Screen::Choose => {
                let data = data::ready().and_then(Result::ok);
                let out = self.choose.command(ui, command, data.as_deref())?;
                match out {
                    choose::Out::None => Some(Action::None),
                    choose::Out::Exit => Some(Action::Exit),
                    choose::Out::New { campaign, faction } => {
                        let data = data?;
                        match self.start(ui, data, campaign, faction) {
                            Ok(()) => Some(Action::Rebuild),
                            Err(e) => {
                                error!("campaign start failed: {e}");
                                let ok = ui.strings.resolve("#button_ok");
                                ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text: e, buttons: vec![(ok, String::new())] });
                                Some(Action::None)
                            }
                        }
                    }
                }
            }
            Screen::Map => {
                let game = self.game.as_mut()?;
                let out = self.iface.command(ui, game, command)?;
                match out {
                    interface::Out::None => Some(Action::None),
                    interface::Out::Finished => {
                        if let Some(g) = &mut self.game {
                            g.campaign.stop_ai();
                        }
                        self.game = None;
                        self.screen = Screen::Choose;
                        Some(Action::MainMenu)
                    }
                    interface::Out::QuitConfirm => Some(Action::QuitConfirm),
                    interface::Out::Options => Some(Action::Options),
                }
            }
        }
    }

    /// DynamicCampaignNew (941F90 -> 941E80): the new campaign world (944A20
    /// -> 9399A0 MGState with the chosen faction, InitCreate; the campaign's
    /// `cinematic_intro` is not baked for the video player and is skipped)
    /// and the MetaGame interface. The local player plays the campaign's
    /// `default_kingdom` (9447E0 -> 76AC60).
    fn start(&mut self, ui: &mut Ui, data: Arc<data::CampaignData>, campaign: usize, faction: Option<String>) -> Result<(), String> {
        let meta = Arc::new(data.meta.clone());
        let c = meta.campaigns.get(campaign).ok_or("no such campaign")?;
        let local_data = c.default_kingdom;
        // the SAI RNG seed: the 59ABA0 timer milliseconds (7A0AA0)
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u32).unwrap_or(0);
        let options = Options {
            computer_quick_battle: ui.uvars.get_bool("MetaGameUIComputerQuickBattle"),
            auto_quick_battle: false,
            // 765410 -> 7A0080: the think argument, seconds * 1000 ms
            budget: Some(std::time::Duration::from_secs_f32(ui.uvars.get_f32("SAITimePerThink").max(0.0))),
            fast_mgai: ui.uvars.get_bool("MetaGameUIFastMGAI"),
        };
        let mut campaign = Campaign::new(meta, campaign, None, faction, seed, options).map_err(|e| e.to_string())?;
        // InitCreate keeps the campaign kingdom order: MG kingdom k = campaign kingdom k
        campaign.local = campaign.state.kingdoms.iter().position(|k| k.data == local_data);
        static RUNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        self.game = Some(Game {
            id: RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            campaign,
            data,
            selected_army: None,
            selected_territory: None,
            targeting: None,
            moving: HashSet::new(),
            accumulator: 0.0,
            paused: false,
            speed: 1.0,
            visual_moves: Vec::new(),
        });
        self.iface = interface::MetaGameUi::default();
        self.screen = Screen::Map;
        ui.messages.clear();
        Ok(())
    }

    /// A key the front end's UI did not take: the MetaGame interface hotkeys
    /// of `Localization/Hotkeys/hotkeys_custom.txt` (`map <key> MetaGame
    /// <command>`: Esc the menu, A attack, M move, E end turn, P purchase, H
    /// history), or an open dialog's (`map esc MetaGameButtonReturnToGame`,
    /// `map esc MetaGameButtonStartBattleCancel`, the Cancel button shown only
    /// on the local player's turn). Returns the command's action.
    pub fn key(&mut self, ui: &mut Ui, code: bevy::input::keyboard::KeyCode) -> Option<Action> {
        if self.screen != Screen::Map || ui.message_box.is_some() {
            return None;
        }
        if let Some(dialog) = self.iface.dialog {
            if code != bevy::input::keyboard::KeyCode::Escape {
                return None;
            }
            let local_turn = self.game.as_ref().is_some_and(|g| g.campaign.local_turn());
            let command = match dialog {
                interface::Dialog::Menu => "ReturnToGame",
                interface::Dialog::StartBattle if local_turn => "Cancel",
                interface::Dialog::StartBattle => return None,
            };
            return self.command(ui, command, "");
        }
        let name = key_name(code)?;
        if self.hotkeys.is_none() {
            let text = ui.store.root.read("Localization/Hotkeys/hotkeys_custom.txt").map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
            self.hotkeys = Some(metagame_hotkeys(&text));
        }
        let cmd = self.hotkeys.as_ref()?.iter().find(|(k, _)| k == name).map(|(_, c)| c.clone())?;
        self.command(ui, &cmd, "")
    }

    /// One frame of the campaign pages: the chooser's data arrival, the
    /// world ticks of a running campaign and the interface's labels.
    pub fn update(&mut self, ui: &mut Ui, now_ms: u64) -> bool {
        let dt = self.last_ms.map_or(0.0, |l| now_ms.saturating_sub(l) as f32 / 1000.0).min(0.25);
        self.last_ms = Some(now_ms);
        let mut rebuild = false;
        match self.screen {
            Screen::Choose => {
                if let Some(r) = data::ready() {
                    match r {
                        Ok(d) => rebuild |= self.choose.data_arrived(ui, &d),
                        Err(e) if !self.data_error_shown => {
                            self.data_error_shown = true;
                            let ok = ui.strings.resolve("#button_ok");
                            ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text: e, buttons: vec![(ok, String::new())] });
                        }
                        Err(_) => {}
                    }
                }
            }
            Screen::Map => {
                if let Some(g) = &mut self.game {
                    if let Some(error) = run_ticks(g, dt) {
                        // The campaign cannot go on (an unported path, see
                        // aa_metagame::driver): stopped, and said so.
                        let ok = ui.strings.resolve("#button_ok");
                        ui.show_message_box(aa_ui::message_box::GAME, aa_ui::message_box::MessageBoxRequest { text: error, buttons: vec![(ok, String::new())] });
                    }
                    let notes = std::mem::take(&mut g.campaign.notes);
                    self.iface.notes(ui, g, &notes);
                    self.iface.update(ui, g);
                }
            }
        }
        rebuild
    }
}

/// The pointer is over the strategic map, not over a widget of the
/// interface (the interface root itself covers the screen with no view).
pub fn map_pointer_over_world(front: &crate::ui::front::Front) -> bool {
    let Some(p) = front.pointer.pos else { return false };
    match front.ui.pick(p.x, p.y) {
        None => true,
        Some(path) => {
            let root = front.ui.root.name.trim_start_matches('/');
            path.eq_ignore_ascii_case(&format!("{root}/MetaGame"))
        }
    }
}

/// The world ticks of this frame (16 Hz, `aa_play::DT`), at most 8 per
/// frame like the battle. A tick that fails stops the world (`paused`) and
/// returns the reason once.
fn run_ticks(g: &mut Game, dt: f32) -> Option<String> {
    if g.paused || g.campaign.state.game_over {
        return None;
    }
    g.accumulator += dt * g.speed;
    let mut steps = 0;
    while g.accumulator >= aa_play::DT && steps < 8 {
        g.accumulator -= aa_play::DT;
        let moving = g.moving.clone();
        let busy = move |a: ArmyId| moving.contains(&a);
        let start = g.campaign.notes.len();
        if let Err(e) = g.campaign.tick(&busy) {
            error!("campaign tick: {e}");
            g.paused = true;
            return Some(e.to_string());
        }
        // the move visuals for the map (962880 in 940020)
        for n in &g.campaign.notes[start..] {
            match n {
                Note::Engine(aa_metagame::resolve::EngineCall::MoveVisual { army, from, to }) => {
                    g.visual_moves.push((*army, *from, *to));
                    g.moving.insert(*army);
                }
                _ => {}
            }
        }
        steps += 1;
    }
    if steps == 8 {
        g.accumulator = 0.0;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metagame_hotkeys_read_the_map_lines() {
        let text = "# MetaGame Interface hotkeys\nmap esc MetaGame OpenMetaGameMenuDialog\nmap a MetaGame MGCommand attack\nmap tab metagame \"CycleSelection 0\"\nmap esc MetaGameButtonReturnToGame\nbind x y z\n";
        assert_eq!(
            metagame_hotkeys(text),
            vec![
                ("esc".to_string(), "OpenMetaGameMenuDialog".to_string()),
                ("a".to_string(), "MGCommand attack".to_string()),
                ("tab".to_string(), "CycleSelection 0".to_string()),
            ]
        );
    }

    #[test]
    fn only_the_confirmation_commands_reach_the_campaign_from_other_pages() {
        assert!(CampaignUi::handles("Uvar UICampaignConfirmDependentCampaign 0 ; RequestDynamicCampaign"));
        assert!(CampaignUi::handles("RequestDynamicCampaign"));
        assert!(!CampaignUi::handles("RequestCredits"));
    }
}
