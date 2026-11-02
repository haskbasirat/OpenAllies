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
pub mod battle;
pub mod choose;
pub mod cinematic;
pub mod data;
pub mod debriefing;
pub mod help;
pub mod interface;
pub mod map;
pub mod missions;
pub mod saves;

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
            .add_systems(Update, (map::sync, map::camera, map::pick, map::minimap).chain().after(crate::ui::front::frame))
            .add_systems(Update, cinematic::play.after(crate::ui::front::frame))
            // after the film of the battle has read the launch orders
            .add_systems(Update, battle::start.after(crate::film_record::start).before(crate::session::step_battle).run_if(in_state(crate::session::AppState::Battle)));
    }
}

/// The campaign pages of the front end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    /// ChooseDynamicCampaign (942AE0).
    Choose,
    /// The MetaGame interface over the strategic map (935130).
    Map,
    /// ChooseCampaign (642280): the scripted campaigns (RequestCampaign).
    Missions,
    /// DebriefingInterface (6014B0): after a scripted mission.
    Debriefing,
    /// The FullscreenVideoWidget alone: a cinematic mission (641D10).
    Cinematic,
}

/// The end of a cinematic mission's movie (`0x641400`), posted by `cinematic::play`.
pub const CINEMATIC_ENDED: &str = "CampaignCinematicEnded";

/// What a campaign command asks the front end to do.
#[derive(Clone, Debug)]
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
    /// A battle for the loader (the front end hands it over, `front::start`).
    Launch(Box<BattleLaunch>),
    /// DebriefingScoring (600C70): the score screen of the mission (slot 5).
    Scoring,
    /// DebriefingSaveFilm: the film dialog of the score screen.
    SaveFilm,
}

/// A battle the campaign pages launch: a mission (PlayMap 643CA0) or a metagame
/// battle (LaunchRTSGame 9336E0).
#[derive(Clone, Debug)]
pub struct BattleLaunch {
    pub setup: aa_play::SkirmishSetup,
    pub random: Option<aa_play::mapgen::RandomMapParams>,
    /// The DifficultyLevel world rule of the session (`ui/objectives.rs`).
    pub difficulty: Option<String>,
    /// The session's AI profile per player (index into `setup.players`).
    pub profiles: Vec<(usize, String)>,
    /// The kingdoms' display names (session records).
    pub names: Vec<(String, String)>,
    /// The general per player (index into `setup.players`), the StartingExperience
    /// rule and the FogOfWar rule's value.
    pub factions: Vec<(usize, Option<String>)>,
    pub starting_experience: Option<f32>,
    pub fog: Option<String>,
    /// Who plays it.
    pub origin: BattleOrigin,
}

/// Where the front end goes when the battle ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleOrigin {
    /// Back to ChooseCampaign.
    Mission,
    /// Back to the strategic map, with the battle's result (93A990).
    MetaGame,
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
    /// The scripted campaigns (RequestCampaign).
    pub missions: missions::Chooser,
    /// What the debriefing page shows.
    pub debriefed: debriefing::Shown,
    /// The movie of the FullscreenVideoWidget (`cinematic::play`).
    pub movie: Option<String>,
    /// The battle on the way (its end returns here).
    pub battle: Option<BattleOrigin>,
    /// What the metagame needs when its battle returns (93A990).
    pub meta_battle: Option<battle::MetaBattle>,
    /// The metagame battle's world side for the loader, taken by the front end.
    pub pending_battle: Option<battle::PendingBattle>,
    /// A saved-campaign dialog on the screen (68DB60 / 68DCE0 mode 3).
    saves: Option<saves::Dialog>,
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
            missions: missions::Chooser::default(),
            debriefed: debriefing::Shown::default(),
            movie: None,
            battle: None,
            meta_battle: None,
            pending_battle: None,
            saves: None,
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
    /// the campaign chooser should be shown now.
    pub fn confirm_play(&mut self, ui: &mut Ui) -> bool {
        if !ui.uvars.get_bool("UICampaignConfirmDependentCampaign") {
            return true;
        }
        let dependent = ui.avars.get("MetaGameDependentCampaignIDS").map(aa_ui::node::unquote).unwrap_or_default();
        if dependent.is_empty() || self.missions.unfinished(ui, &dependent) == Some(false) {
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
            Screen::Missions => self.missions.show(ui),
            Screen::Debriefing => debriefing::show(ui, &self.debriefed),
            Screen::Cinematic => ui.clear(),
        }
    }

    /// The nation skin of the map screen: the local kingdom's `nation_ids`.
    fn map_skin(&self) -> Option<String> {
        if self.screen != Screen::Map {
            return None;
        }
        let g = self.game.as_ref()?;
        let c = g.campaign.data.campaigns.get(g.campaign.state.campaign)?;
        let k = g.campaign.state.kingdoms.get(g.campaign.local?)?;
        let ids = &c.kingdoms.get(k.data)?.nation_ids;
        Some(ids.trim_start_matches("nation_").to_ascii_uppercase())
    }

    /// A click on the strategic map's minimap: the camera looks there.
    pub fn minimap_jump(&mut self, u: f32, v: f32) {
        let Some(g) = &self.game else { return };
        let c = &g.campaign.data.campaigns[g.campaign.state.campaign];
        let Some(m) = c.map.and_then(|m| g.campaign.data.maps.get(m)) else { return };
        self.iface.look_at = Some([u * f32::from_bits(m.width_bits), (1.0 - v) * f32::from_bits(m.height_bits)]);
    }

    /// RequestCampaign: the scripted campaigns' chooser.
    pub fn enter_missions(&mut self, ui: &mut Ui) {
        self.screen = Screen::Missions;
        self.missions.enter(ui);
    }

    /// LaunchRTSGame 9336E0 -> 932210: the pending battle's launch.
    fn rts_launch(&mut self, ui: &mut Ui) -> Option<Action> {
        let game_root = self.root.parent().map(Path::to_path_buf).unwrap_or_else(|| self.root.clone());
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(1);
        let game = self.game.as_ref()?;
        match battle::launch(game, &game_root, seed) {
            Ok((launch, pending, meta)) => {
                info!("campaign battle: {:?} on {}", launch.setup.players, launch.setup.map_id);
                self.battle = Some(BattleOrigin::MetaGame);
                self.meta_battle = Some(meta);
                self.pending_battle = Some(pending);
                Some(Action::Launch(Box::new(launch)))
            }
            Err(e) => {
                error!("the campaign battle cannot start: {e}");
                let ok = ui.strings.resolve("#button_ok");
                ui.show_message_box(aa_ui::message_box::GAME, aa_ui::message_box::MessageBoxRequest { text: e, buttons: vec![(ok, String::new())] });
                Some(Action::None)
            }
        }
    }

    /// The RTS battle came back: 93A990 with the winner (MGState+0x48: the
    /// winning kingdom's RMC name; an empty one for a battle left unfinished).
    fn rts_returned(&mut self, won: bool, winner_team: Option<u32>) {
        let (Some(meta), Some(g)) = (self.meta_battle.take(), self.game.as_mut()) else { return };
        let winner = match (meta.local_attacker, won, winner_team) {
            (Some(true), true, _) | (Some(false), false, Some(_)) => meta.attacker.clone(),
            (Some(false), true, _) | (Some(true), false, Some(_)) => meta.defender.clone(),
            (None, _, Some(0)) => meta.attacker.clone(),
            (None, _, Some(_)) => meta.defender.clone(),
            _ => String::new(),
        };
        info!("the campaign battle returns: winner '{winner}'");
        let resources = crate::preload::game_data(self.root.parent().unwrap_or(&self.root)).map(|d| d.resources.iter().map(|r| r.id.clone()).collect()).unwrap_or_default();
        let env = battle::env(g, resources);
        let moving = g.moving.clone();
        let busy = move |a: ArmyId| moving.contains(&a);
        if let Err(e) = g.campaign.rts_finished(&winner, &env, &busy) {
            error!("the campaign battle's result: {e}");
        }
    }

    /// A mission's battle (PlayMap 643CA0).
    fn mission_launch(&mut self, ui: &mut Ui, map: &str, difficulty: &str) -> Option<Action> {
        let game_root = self.root.parent().map(Path::to_path_buf).unwrap_or_else(|| self.root.clone());
        let built = crate::preload::game_data(&game_root).and_then(|data| missions::mission_setup(&game_root, map, &data, "Player", ui));
        match built {
            Ok(session) => {
                self.battle = Some(BattleOrigin::Mission);
                Some(Action::Launch(Box::new(BattleLaunch {
                    setup: session.setup,
                    random: None,
                    difficulty: Some(difficulty.to_string()),
                    profiles: Vec::new(),
                    names: Vec::new(),
                    factions: session.factions,
                    starting_experience: session.starting_experience,
                    fog: session.fog,
                    origin: BattleOrigin::Mission,
                })))
            }
            Err(e) => {
                // campaign_bad_map (643CF1)
                error!("mission {map}: {e:#}");
                let text = ui.strings.resolve("#campaign_bad_map");
                let ok = ui.strings.resolve("#button_ok");
                ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(ok, String::new())] });
                Some(Action::None)
            }
        }
    }

    /// Whether the battle that is ending is a scripted mission.
    pub fn in_mission(&self) -> bool {
        self.battle == Some(BattleOrigin::Mission)
    }

    /// A scripted mission ended (`0x60A140` -> `0x651BD0`): the debriefing (slot 4),
    /// the chooser (slot 1) after the final mission won.
    pub fn mission_ended(&mut self, ui: &Ui, result: debriefing::MissionResult) {
        self.battle = None;
        match self.missions.mission_ended(ui, result) {
            Some(shown) => {
                self.debriefed = shown;
                self.screen = Screen::Debriefing;
            }
            None => self.screen = Screen::Missions,
        }
    }

    /// The battle ended (the score screen's Exit): back to the campaign page it
    /// came from, with what it decided. Returns whether a campaign page follows.
    pub fn battle_ended(&mut self, won: bool, winner_team: Option<u32>) -> bool {
        match self.battle.take() {
            Some(BattleOrigin::Mission) => {
                self.screen = Screen::Debriefing;
                true
            }
            Some(BattleOrigin::MetaGame) => {
                self.screen = Screen::Map;
                self.rts_returned(won, winner_team);
                true
            }
            None => false,
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
        if let Some(d) = self.saves.as_mut()
            && let Some(outcome) = d.command(ui, command)
        {
            return Some(self.saves_outcome(ui, outcome));
        }
        match self.screen {
            Screen::Choose => {
                let data = data::ready().and_then(Result::ok);
                let out = self.choose.command(ui, command, data.as_deref())?;
                match out {
                    choose::Out::None => Some(Action::None),
                    choose::Out::Exit => Some(Action::Exit),
                    // 932A00 -> 68DB60(3, flag 1)
                    choose::Out::Load => {
                        self.saves = saves::Dialog::open(ui, false, "");
                        Some(Action::None)
                    }
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
            Screen::Missions => {
                let out = self.missions.command(ui, command)?;
                match out {
                    missions::Out::None => Some(Action::None),
                    missions::Out::Exit => Some(Action::Exit),
                    missions::Out::Asked => Some(Action::None),
                    missions::Out::Play { map, difficulty } => self.mission_launch(ui, &map, &difficulty),
                    missions::Out::Cinematic(name) => {
                        self.movie = Some(name);
                        self.screen = Screen::Cinematic;
                        Some(Action::Rebuild)
                    }
                }
            }
            // 0x641400 on the movie's end
            Screen::Cinematic => {
                if command != CINEMATIC_ENDED {
                    return None;
                }
                self.screen = Screen::Missions;
                match self.missions.cinematic_ended(ui) {
                    missions::Out::Play { map, difficulty } => self.mission_launch(ui, &map, &difficulty),
                    missions::Out::Cinematic(name) => {
                        self.movie = Some(name);
                        self.screen = Screen::Cinematic;
                        Some(Action::Rebuild)
                    }
                    _ => Some(Action::Rebuild),
                }
            }
            Screen::Debriefing => {
                let out = match debriefing::command(command)? {
                    debriefing::Out::Scoring => return Some(Action::Scoring),
                    debriefing::Out::SaveFilm => return Some(Action::SaveFilm),
                    debriefing::Out::Exit => {
                        self.screen = Screen::Missions;
                        return Some(Action::Rebuild);
                    }
                    debriefing::Out::Replay => self.missions.replay(ui),
                    debriefing::Out::PlayNext => self.missions.play_next(ui),
                };
                self.screen = Screen::Missions;
                match out {
                    missions::Out::Play { map, difficulty } => self.mission_launch(ui, &map, &difficulty),
                    missions::Out::Cinematic(name) => {
                        self.movie = Some(name);
                        self.screen = Screen::Cinematic;
                        Some(Action::Rebuild)
                    }
                    _ => Some(Action::Rebuild),
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
                    interface::Out::LaunchBattle => self.rts_launch(ui),
                    // 9375C0: MetaGameLoadDialog -> 68DB60(3, ..)
                    interface::Out::LoadDialog => {
                        self.saves = saves::Dialog::open(ui, false, "");
                        Some(Action::None)
                    }
                    // 9375C0: MetaGameSaveDialog -> 68DCE0(3, ..)
                    interface::Out::SaveDialog => {
                        let name = self.game.as_ref().map(|g| {
                            let round = interface::round_text(ui, g);
                            let c = &g.campaign.data.campaigns[g.campaign.state.campaign];
                            format!("{} {round}", ui.strings.resolve(&c.name))
                        });
                        self.saves = saves::Dialog::open(ui, true, &name.unwrap_or_default());
                        Some(Action::None)
                    }
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
        self.install(ui, data, campaign);
        Ok(())
    }

    /// What a saved-campaign dialog's command came to.
    fn saves_outcome(&mut self, ui: &mut Ui, outcome: saves::Outcome) -> Action {
        let ok = ui.strings.resolve("#button_ok");
        match outcome {
            saves::Outcome::Open => Action::None,
            saves::Outcome::Closed => {
                self.saves = None;
                Action::None
            }
            saves::Outcome::Save(target) => {
                self.saves = None;
                if let Some(g) = &self.game
                    && let Err(e) = saves::save(g, &target)
                {
                    error!("the campaign's save {}: {e}", target.display());
                    ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text: e, buttons: vec![(ok, String::new())] });
                }
                Action::None
            }
            saves::Outcome::Load(file) => {
                self.saves = None;
                let help_popups = help::Help::all_ids(ui);
                let loaded = std::fs::read(&file)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| data::ready().ok_or_else(|| "the campaign data is not ready".to_string()).and_then(|d| d).map(|d| (bytes, d)))
                    .and_then(|(bytes, d)| saves::decode(&bytes, &d, help_popups).map(|l| (l, d)));
                match loaded {
                    Ok(((state, local), d)) => {
                        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|t| t.as_millis() as u32).unwrap_or(0);
                        let options = Options {
                            computer_quick_battle: ui.uvars.get_bool("MetaGameUIComputerQuickBattle"),
                            auto_quick_battle: false,
                            budget: Some(std::time::Duration::from_secs_f32(ui.uvars.get_f32("SAITimePerThink").max(0.0))),
                            fast_mgai: ui.uvars.get_bool("MetaGameUIFastMGAI"),
                        };
                        let campaign = Campaign::from_state(Arc::new(d.meta.clone()), state, local, seed, options);
                        self.install(ui, d, campaign);
                        Action::Rebuild
                    }
                    Err(e) => {
                        error!("the saved campaign {}: {e}", file.display());
                        ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text: e, buttons: vec![(ok, String::new())] });
                        Action::None
                    }
                }
            }
        }
    }

    /// A campaign (new or loaded) under the MetaGame interface.
    fn install(&mut self, ui: &mut Ui, data: Arc<data::CampaignData>, campaign: Campaign) {
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
                interface::Dialog::Purchase | interface::Dialog::History => "Close",
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
        // the strategic map is drawn in the local kingdom's nation skin
        // (`Skins/<NATION>/MetaGame/UI`), the choosers in none
        let skin = self.map_skin();
        if ui.store.root.skin() != skin.as_deref() {
            ui.set_skin(skin.as_deref(), self.root.join("assets/original/Skins"));
        }
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
            Screen::Missions | Screen::Debriefing | Screen::Cinematic => {}
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
