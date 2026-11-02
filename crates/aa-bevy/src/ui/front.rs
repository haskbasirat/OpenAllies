//! The original front end and in-game interface driven by aa-ui: the
//! screen state machine (main menu, single player, credits, custom game
//! staging, loading screen, game interface), pointer / keyboard input fed to
//! the UI and the console commands the buttons fire.
//!
//! Follows the egui client's `tgi_frontend.rs` (the same native commands:
//! `RequestSinglePlayer` 0x64F250, `RequestMainMenu` 0x65B1F0,
//! `RequestCredits` 0x64F290, `CreditsEndCredits` 0x645800, `QuitConfirm`
//! 0x413230, `OpenOptionsMenu` 0x650700) with every button enabled as in the
//! original. Commands whose screens are not built yet are logged.
//!
//! The end of a battle (Game interface 0x61A650 commands): the local
//! player's mission result box (0x4E9720 -> 0x50D370 -> 0x50C600 case 2 ->
//! 0x440190; an observer's mission-over box 0x4F4DA0 -> 0x50D330 -> case 3
//! -> 0x4408A0), `RequestFinishedConfirm` 0x615D40, `RequestFinished`
//! 0x6125A0 (0x774C80 case 2 -> 0x60A140: the end order with 6, 0x651BD0:
//! the score screen, slot 5), `RequestKeepPlaying` 0x60CAE0 and
//! `RequestObserve` 0x60CB40; the score screen is ui/scoring.rs.
use super::custom::Custom;
use super::hud::GameHud;
use super::scoring;
use crate::session::{AppState, Battle, LoadProgress};
use aa_ui::fs::DataRoot;
use aa_ui::layout::Screen;
use aa_ui::screens;
use aa_ui::ui::{MouseButton as UiButton, Ui, UiEvent};
use aa_ui::ui_controls::UiKey;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Front-end pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    /// The intro movies before the main menu (0x5C4910 / 0x5C4AF0,
    /// ui/video.rs `intro`).
    Intro,
    MainMenu,
    SinglePlayer,
    /// CreditsInterface (0x646150).
    Credits,
    /// StagingMenu (0x661BC0) for a custom game.
    Staging,
    /// LoadingScreen interface (0x6E87C0).
    Loading,
    /// The Game interface (0x61A650) over the battle.
    Game,
    /// The Scoring interface (0x6594D0, menu slot 5) after a battle.
    Scoring,
    /// The World War II campaign pages (campaign/: ChooseDynamicCampaign
    /// 0x942AE0, the MetaGame interface 0x935130).
    Campaign,
}

/// What a launch inserts for the loader, kept for the score screen's
/// Replay (0x6590B0 -> 0x641660 / 0x652960: the session's launch settings
/// again).
#[derive(Clone)]
struct Launch {
    setup: aa_play::SkirmishSetup,
    random: Option<aa_play::mapgen::RandomMapParams>,
    profiles: Vec<(usize, String)>,
    generals: super::abilities::PendingGenerals,
    fog: Option<aa_play::fog::FogMode>,
    kingdom_names: std::collections::HashMap<String, String>,
    starting_experience: f32,
}

/// Windows default double-click time (GetDoubleClickTime 500 ms); AA.exe
/// leaves the test to Windows (CS_DBLCLKS, 0x6FB2DD).
const DOUBLE_CLICK_MS: u64 = 500;

#[derive(Resource)]
pub struct Front {
    pub ui: Ui,
    pub page: Page,
    built_for: Option<(i32, i32, Page)>,
    /// Message box on screen (path, request); re-shown after a rebuild.
    dialog: Option<(String, aa_ui::message_box::MessageBoxRequest)>,
    options: Option<aa_ui::options::Options>,
    credits: Option<aa_ui::credits::Credits>,
    credits_end_posted: bool,
    clock: Instant,
    last_press: Option<(u64, f32, f32)>,
    pub root: PathBuf,
    pub custom: Custom,
    pub hud: Option<GameHud>,
    /// Loading screen: the battle finished loading (Continue shown).
    loading_finished: bool,
    /// What the left press of this frame landed on (for the battle input).
    pub press_on_ui: bool,
    /// This frame's pointer input (window pixels, after script injection),
    /// shared with the battle input and the camera.
    pub pointer: PointerInput,
    /// The left button is down (pressed and not yet released).
    pub left_held: bool,
    /// Game actions from dialogs (options) for the battle side.
    pending: Vec<super::hud::GameAction>,
    /// The pointer is over a HUD widget (not the world).
    pub pointer_over_ui: bool,
    settings_path: Option<PathBuf>,
    pub quit: bool,
    /// The tooltip on screen and the hover timer.
    tooltips: super::tooltip::Tooltips,
    /// The loading screen's status key shown.
    loading_status: String,
    /// The launched session's StartingExperience world rule.
    starting_experience: f32,
    /// The running battle's speed (2^v), for the options dialog's slider.
    battle_speed: Option<f32>,
    /// The launched session's independent kingdoms' display names.
    kingdom_names: std::collections::HashMap<String, String>,
    /// Data/Scoring/Scoring.tgi, loaded for the first score screen.
    scoring_defs: Option<scoring::Defs>,
    /// The score screen on show.
    scoring: Option<scoring::Scoring>,
    /// The last finished world for the score screen (StagingOpenScoring
    /// 0x65BD60 opens it again while the world exists).
    score_data: Option<scoring::ScoreData>,
    /// The local kingdom's mission result was announced (GKingdom+0x19C is
    /// set once, 0x4E9720), and the observer's mission-over box (0x4408A0).
    result_shown: bool,
    over_shown: bool,
    /// The last launch (Replay).
    last_launch: Option<Launch>,
    /// The options dialog's nickname (profile+0x2C, default_lan_name).
    lan_name: Option<String>,
    /// UI sounds of this frame (`UiEvent::Sound`: a button's `set_sound` /
    /// `unset_sound`, the options dialog's volume test sounds), drained by
    /// ui/mod.rs `game_event_fx`.
    pub sounds: Vec<String>,
    /// The World War II campaign (campaign owner).
    pub campaign: crate::campaign::CampaignUi,
}

/// Commands the front end does not run yet (their screens are not built).
const NOT_BUILT: &[&str] = &[
    "RequestMultiplayer",
    "RequestEditor",
    "RequestFilm",
    "CheckForUpdates",
    "RequestCampaign",
];

impl Front {
    pub fn new(root: &Path) -> Self {
        let data = DataRoot::with_overlay(root.join("assets/original/Data"), root.join("native-game/runtime-expanded-v14/Data"));
        let mut ui = Ui::new(data, Screen { width: 1024, height: 768 });
        let settings_path = std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("AxisAlliesRust").join("settings.json"));
        // user variables saved by an earlier options OK (same file as aa-client)
        if let Some(vars) = settings_path.as_ref().and_then(|p| load_uvars(p)) {
            ui.uvars.apply_overrides(&vars);
        }
        let mut custom = Custom::new(root);
        // the profile's LAN name saved by an earlier options OK
        let lan_name = settings_path.as_ref().and_then(|p| load_lan_name(p));
        if let Some(n) = &lan_name {
            custom.set_player_name(n);
        }
        // the staging data parses in the background from the start
        super::custom::warm(ui.store.root.clone());
        super::banners::warm(ui.store.root.clone());
        super::minimap::warm(ui.store.root.clone());
        super::names::warm(ui.store.root.clone());
        Self {
            ui,
            page: if super::video::intro_enabled() { Page::Intro } else { Page::MainMenu },
            built_for: None,
            dialog: None,
            options: None,
            credits: None,
            credits_end_posted: false,
            clock: Instant::now(),
            last_press: None,
            root: root.to_path_buf(),
            custom,
            hud: None,
            loading_finished: false,
            press_on_ui: false,
            pointer: PointerInput::default(),
            left_held: false,
            pending: Vec::new(),
            pointer_over_ui: false,
            settings_path,
            quit: false,
            tooltips: Default::default(),
            loading_status: String::new(),
            starting_experience: 0.0,
            battle_speed: None,
            kingdom_names: Default::default(),
            scoring_defs: None,
            scoring: None,
            score_data: None,
            result_shown: false,
            over_shown: false,
            last_launch: None,
            lan_name,
            sounds: Vec::new(),
            campaign: crate::campaign::CampaignUi::new(root),
        }
    }

    pub fn now_ms(&self) -> u64 {
        self.clock.elapsed().as_millis() as u64
    }

    pub fn show(&mut self, page: Page) {
        if page != Page::Credits {
            self.credits = None;
        }
        if page != self.page {
            if let Some(o) = &mut self.options {
                if o.is_open() {
                    o.hide(&mut self.ui);
                }
            }
            if page != Page::Game {
                self.hud = None;
            }
            if page != Page::Scoring {
                self.scoring = None;
            }
        }
        self.page = page;
        self.built_for = None;
    }

    fn ensure_options(&mut self) {
        if self.options.is_none() {
            self.options = Some(aa_ui::options::Options::new(&mut self.ui));
        }
    }

    /// The options dialog (`OpenOptionsMenu` 0x650700 -> 0x6792A0(""); the
    /// staging's `StagingOpenOptions` 0x65DE10 -> 0x6792A0("Network")).
    fn open_options(&mut self, size: (i32, i32), tab: &str) {
        self.ensure_options();
        let default_connection = self.ui.avars.get("DefaultNetworkConnectionType").unwrap_or("").to_string();
        // the network page's nickname is the profile's LAN name (profile
        // +0x2C, the defaults 0x41E160: Format(default_lan_name))
        let nickname = self.lan_name.clone().unwrap_or_else(|| self.ui.strings.resolve("#default_lan_name"));
        if let Some(o) = &mut self.options {
            // the window is the renderer's only display mode
            o.video_modes = vec![(size.0, size.1, 32)];
            o.current_mode = size;
            if o.connection.is_empty() {
                o.connection = default_connection;
            }
            o.nickname = nickname;
            // 0x76B2F0: a game in progress (single player: this player hosts)
            // lets the slider change the speed, GameSpeedMin / GameSpeedMax
            // (SVars -2 / +2) around the current exponent
            o.game_speed = self.battle_speed.filter(|_| self.page == Page::Game).map(|s| (-2.0, 2.0, s.max(1e-3).log2()));
            o.show(&mut self.ui, tab);
        }
    }

    fn remember_dialog(&mut self) {
        if let Some(req) = self.ui.message_box.clone() {
            let path = match self.ui.modal.as_deref() {
                Some("GameMessageBox") => aa_ui::message_box::GAME,
                _ => aa_ui::message_box::SHARED,
            };
            self.dialog = Some((path.to_string(), req));
        }
    }

    /// A one-button message (`button_ok`) in the message box of the screen.
    pub fn message(&mut self, text: &str) {
        let ok = self.ui.strings.resolve("#button_ok");
        let path = if self.page == Page::Game { aa_ui::message_box::GAME } else { aa_ui::message_box::SHARED };
        self.ui.show_message_box(path, aa_ui::message_box::MessageBoxRequest { text: text.to_string(), buttons: vec![(ok, String::new())] });
        self.remember_dialog();
    }

    /// A message box with explicit buttons (label, command).
    pub fn ask(&mut self, text: &str, buttons: Vec<(String, String)>) {
        let path = if self.page == Page::Game { aa_ui::message_box::GAME } else { aa_ui::message_box::SHARED };
        self.ui.show_message_box(path, aa_ui::message_box::MessageBoxRequest { text: text.to_string(), buttons });
        self.remember_dialog();
    }

    /// The mission boxes of the game interface. The local kingdom's result
    /// (GKingdom+0x19C, set once by 0x4E9720: 1 won with its team, 2 lost;
    /// a winner settles every open kingdom outside its team as lost, 0x4F4DA0)
    /// queues 0x50D370 -> 0x440190 (host, single player): "Mission Won"
    /// with End Mission / Keep Playing, "Mission Lost" with End Mission /
    /// Observe (0x76BA40: no other human left). Without a local kingdom
    /// (an observer) the game's end (0x4F4DA0: GWorld+0x48 = 2 ->
    /// 0x50D330 -> 0x4408A0) asks "Mission Over" with End Mission / Keep
    /// Playing.
    fn mission_boxes(&mut self, b: &Battle) {
        let w = &b.world;
        let button = |ui: &Ui, cmd: &str, key: &str| (ui.strings.resolve(key), cmd.to_string());
        if b.local_kingdom().is_none() {
            if !self.over_shown && w.winner.is_some() {
                self.over_shown = true;
                let text = self.ui.strings.resolve("#ui_mission_message_over");
                let buttons = vec![
                    button(&self.ui, "RequestFinished", "#ui_mission_label_end_game"),
                    button(&self.ui, "RequestKeepPlaying", "#ui_mission_label_keep_playing"),
                ];
                self.ask(&text, buttons);
            }
            return;
        }
        // the game's end with a local kingdom: 0x4408A0 shows nothing (a
        // later Observe does not bring its box back)
        if w.winner.is_some() {
            self.over_shown = true;
        }
        if self.result_shown {
            return;
        }
        let Some(k) = w.kingdoms.get(b.local as usize) else { return };
        let Some(lost) = mission_result_lost(k.defeated, k.team, w.winner) else { return };
        self.result_shown = true;
        let (text, second) = if lost {
            ("#ui_mission_message_lost", button(&self.ui, "RequestObserve", "#ui_mission_label_observe"))
        } else {
            ("#ui_mission_message_won", button(&self.ui, "RequestKeepPlaying", "#ui_mission_label_keep_playing"))
        };
        let text = self.ui.strings.resolve(text);
        let buttons = vec![button(&self.ui, "RequestFinished", "#ui_mission_label_end_game"), second];
        self.ask(&text, buttons);
    }

    fn build(&mut self, size: (i32, i32), battle: Option<&Battle>, progress: Option<&LoadProgress>) {
        let t0 = std::time::Instant::now();
        self.build_page(size, battle, progress);
        let ms = t0.elapsed().as_secs_f32() * 1000.0;
        if ms > 20.0 {
            info!("UI page {:?} built in {ms:.0} ms", self.page);
        }
    }

    fn build_page(&mut self, size: (i32, i32), battle: Option<&Battle>, progress: Option<&LoadProgress>) {
        // a rebuilt screen has no tooltip connected
        self.tooltips = Default::default();
        self.ui.set_screen(Screen { width: size.0, height: size.1 });
        if self.page == Page::Credits && self.credits.is_some() {
            self.ui.layout();
            self.built_for = Some((size.0, size.1, self.page));
            return;
        }
        let options_open = self.options.as_ref().is_some_and(|o| o.is_open());
        if options_open {
            if let Some(o) = &mut self.options {
                o.suspend(&mut self.ui);
            }
        }
        match self.page {
            // the FullscreenVideoWidget alone (ui/video.rs draws the movie)
            Page::Intro => self.ui.clear(),
            Page::MainMenu => screens::main_menu(&mut self.ui),
            Page::SinglePlayer => screens::single_player_menu(&mut self.ui),
            Page::Credits => {
                let now = self.now_ms();
                self.credits = Some(aa_ui::credits::Credits::show(&mut self.ui, now));
                self.credits_end_posted = false;
            }
            Page::Staging => self.custom.show(&mut self.ui),
            Page::Loading => {
                screens::loading_screen(&mut self.ui, false);
                self.refresh_loading(progress);
            }
            Page::Game => {
                self.ui.set_skin(battle.and_then(|b| super::hud::local_nation(b)).as_deref(), self.root.join("assets/original/Skins"));
                let hud = GameHud::show(&mut self.ui, battle);
                self.hud = Some(hud);
            }
            Page::Scoring => {
                if self.scoring_defs.is_none() {
                    self.scoring_defs = Some(scoring::Defs::load(&self.ui.store.root));
                }
                let defs = self.scoring_defs.as_ref().unwrap();
                match &mut self.scoring {
                    Some(s) => s.rebuild(&mut self.ui, defs),
                    None => {
                        let data = self.score_data.clone().unwrap_or_default();
                        self.scoring = Some(scoring::Scoring::show(&mut self.ui, defs, data));
                    }
                }
            }
            Page::Campaign => self.campaign.show(&mut self.ui),
        }
        if !matches!(self.page, Page::Game) {
            self.ui.set_skin(None, self.root.join("assets/original/Skins"));
        }
        self.ui.layout();
        if options_open {
            if let Some(o) = &mut self.options {
                o.resume(&mut self.ui);
            }
        }
        if let Some((path, req)) = self.dialog.clone() {
            self.ui.show_message_box(&path, req);
        }
        self.built_for = Some((size.0, size.1, self.page));
    }

    /// Loading screen refresh (0x6E83A0) from the pending setup / loader.
    fn refresh_loading(&mut self, progress: Option<&LoadProgress>) {
        // the loader's stage (session::LOAD_STATUS app_status_* key), the
        // load error text otherwise
        let status_text = match progress.and_then(|p| p.error.clone()) {
            Some(e) => e,
            None => crate::session::load_status().to_string(),
        };
        self.loading_status = status_text.clone();
        let status = if self.loading_finished {
            self.ui.strings.resolve("#app_status_finished")
        } else if status_text.starts_with('#') || status_text.starts_with("app_status") {
            self.ui.strings.resolve(&format!("#{}", status_text.trim_start_matches('#')))
        } else if status_text.is_empty() {
            self.ui.strings.resolve("#app_status_loading_data")
        } else {
            status_text
        };
        let (map_name, map_description, nation) = self.custom.loading_info();
        let map_description = if map_description.starts_with('#') { self.ui.strings.resolve(&map_description) } else { map_description };
        let info = screens::LoadingInfo {
            hide_map_labels: false,
            map_name,
            map_description,
            // a custom game is session kind 1 (RequestCustom 0x64EF00 ->
            // 0x7840B0(1): the fallback of 0x651BD0 maps kind 1 to the
            // staging): the objectives list (kind 0, a campaign) is hidden
            kind: 1,
            mission_textures: None,
            local_player: nation.is_some(),
            nation,
            status,
        };
        screens::loading_refresh(&mut self.ui, &info);
        // the (empty) objectives list: the viewport scroll bar update
        // 0x5ED560 hides the bar of a viewport whose items fit
        self.ui.viewport_refresh("Root/LoadingScreen/ObjectivesList/SlotViewport");
        self.ui.with_widget("LoadingScreen/ObjectivesList/SlotViewport/Scrollbar", |w| w.set_visible(false));
        self.ui.layout();
        if self.loading_finished {
            screens::loading_finished(&mut self.ui);
        }
    }

    fn command(&mut self, command: &str, widget: &str, size: (i32, i32), out: &mut FrameOut) {
        if let Some(o) = &mut self.options {
            if let Some(effects) = o.command(&mut self.ui, command) {
                if self.ui.message_box.is_some() {
                    self.remember_dialog();
                }
                self.effects(effects);
                return;
            }
        }
        if self.page == Page::Staging {
            if let Some(action) = self.custom.command(&mut self.ui, command, widget) {
                if self.ui.message_box.is_some() {
                    self.remember_dialog();
                }
                match action {
                    super::custom::Action::None => {}
                    super::custom::Action::Exit => self.show(Page::SinglePlayer),
                    super::custom::Action::Launch => out.launch = true,
                }
                return;
            }
        }
        if self.page == Page::Game {
            if let Some(hud) = &mut self.hud {
                if let Some(acts) = hud.command(&mut self.ui, command) {
                    out.game.extend(acts);
                    if self.ui.message_box.is_some() {
                        self.remember_dialog();
                    }
                    return;
                }
            }
        }
        if self.page == Page::Scoring {
            if let (Some(s), Some(defs)) = (self.scoring.as_mut(), self.scoring_defs.as_ref()) {
                if let Some(outcome) = s.command(&mut self.ui, defs, command) {
                    match outcome {
                        scoring::Outcome::None => {}
                        // 0x659290: a custom game goes back to the staging
                        // (slot 3)
                        scoring::Outcome::Exit => {
                            self.custom.scoring_available(self.score_data.is_some());
                            self.show(Page::Staging);
                        }
                        scoring::Outcome::Replay => out.replay = true,
                        // slot 4 (campaign only; the button is hidden)
                        scoring::Outcome::Debriefing => info!("UI command ScoringDebriefing: the debriefing is the campaign's"),
                    }
                    return;
                }
            }
        }
        // the World War II campaign (campaign/)
        if self.page == Page::Campaign || crate::campaign::CampaignUi::handles(command) {
            if let Some(action) = self.campaign.command(&mut self.ui, command, widget) {
                self.campaign_action(action, size);
                return;
            }
        }
        match command {
            "" => {}
            "Quit" => self.quit = true,
            "RequestSinglePlayer" => self.show(Page::SinglePlayer),
            "RequestMainMenu" => self.show(Page::MainMenu),
            // main menu Options (0x650700) / game menu Options
            "OpenOptionsMenu" | "OptionsDialog" => self.open_options(size, ""),
            // the staging's options button (0x65DE10): the Network page
            "StagingOpenOptions" => self.open_options(size, "Network"),
            // the staging's score button (0x65BD60): the last world's score
            // screen (slot 5) while it exists
            "StagingOpenScoring" if self.page == Page::Staging => {
                if self.score_data.is_some() {
                    self.show(Page::Scoring);
                }
            }
            // the game menu's leave button and the confirmation the
            // mission boxes skip (0x615D40): single player asks
            // `score_screen_confirm` (a campaign `campaign_end_confirm`,
            // the editor test `editor_confirm`) with OK -> RequestFinished
            "RequestFinishedConfirm" if self.page == Page::Game => {
                let text = self.ui.strings.resolve("#score_screen_confirm");
                let ok = self.ui.strings.resolve("#button_ok");
                let cancel = self.ui.strings.resolve("#button_cancel");
                self.ask(&text, vec![(ok, "RequestFinished".into()), (cancel, String::new())]);
            }
            // 0x6125A0 -> 0x774C80 (host, single player: 2) -> 0x60A140:
            // the end order with 6 -> 0x651BD0 case 6: the score screen
            "RequestFinished" if self.page == Page::Game => out.leave = true,
            // 0x60CAE0: the host of a finished game keeps playing (decree
            // 0x60F2B0 -> 0x772DD0: GENERIC_EVENT
            // ui_mission_message_host_elected_keep_playing)
            "RequestKeepPlaying" if self.page == Page::Game => out.keep_playing = true,
            // 0x60CB40: observe (0x61EE60), then keep playing when over
            "RequestObserve" if self.page == Page::Game => {
                out.observe = true;
                out.keep_playing = true;
            }
            "QuitConfirm" => {
                screens::quit_confirm(&mut self.ui, self.page == Page::Game);
                self.remember_dialog();
            }
            "RequestCredits" => self.show(Page::Credits),
            "CreditsEndCredits" => self.show(Page::MainMenu),
            "RequestCustom" => {
                self.custom.enter(&mut self.ui);
                self.show(Page::Staging);
            }
            "UserReady" if self.page == Page::Loading && self.loading_finished => out.ready = true,
            // 0x65B780: the dependent-campaign confirmation, then the
            // dynamic campaign chooser (campaign/)
            "ConfirmPlayDynamicCampaign" => {
                if self.campaign.confirm_play(&mut self.ui) {
                    self.campaign.enter(&mut self.ui);
                    self.show(Page::Campaign);
                } else {
                    self.remember_dialog();
                }
            }
            other if NOT_BUILT.contains(&other) => info!("UI command {other}: its screen is not built yet"),
            other => info!("UI command not handled yet: {other}"),
        }
    }

    /// What a campaign command asked for (campaign owner).
    fn campaign_action(&mut self, action: crate::campaign::Action, size: (i32, i32)) {
        use crate::campaign::Action as A;
        if self.ui.message_box.is_some() {
            self.remember_dialog();
        }
        match action {
            A::None => {}
            A::Exit => self.show(Page::SinglePlayer),
            A::MainMenu => self.show(Page::MainMenu),
            A::Rebuild => self.show(Page::Campaign),
            A::QuitConfirm => {
                screens::quit_confirm(&mut self.ui, true);
                self.remember_dialog();
            }
            A::Options => self.open_options(size, ""),
        }
    }

    fn effects(&mut self, effects: Vec<aa_ui::options::Effect>) {
        use aa_ui::options::Effect;
        for e in effects {
            match e {
                Effect::Save => self.save_settings(),
                Effect::CameraZoom(z) => self.pending.push(super::hud::GameAction::CameraZoom(z)),
                Effect::GameSpeed(s) => self.pending.push(super::hud::GameAction::SetSpeed(2f32.powf(s))),
                // the options dialog's volume test sounds, as UI sounds
                Effect::PlaySound(ids) => self.sounds.push(ids),
                // the network page's nickname becomes the profile's LAN
                // name: the staging's local player (0x41E160 profile+0x2C)
                Effect::Nickname(n) => {
                    let n = n.trim().to_string();
                    if !n.is_empty() {
                        self.custom.set_player_name(&n);
                        self.lan_name = Some(n);
                        self.save_settings();
                    }
                }
                // the UVars change and are saved; the sound manager still
                // takes its volumes from the data files (audio/mod.rs
                // read_vars), not from these UVars
                Effect::Volume { .. } | Effect::Closed | Effect::HotkeyConfiguration(_) => {}
            }
        }
    }

    fn save_settings(&self) {
        let Some(path) = &self.settings_path else { return };
        let mut v: serde_json::Value = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let vars = self.ui.uvars.overrides();
        if let Some(obj) = v.as_object_mut() {
            obj.insert("uvars".into(), serde_json::to_value(vars).unwrap_or_default());
            if let Some(n) = &self.lan_name {
                obj.insert("lan_name".into(), serde_json::Value::String(n.clone()));
            }
        }
        let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
        if let Ok(text) = serde_json::to_string_pretty(&v) {
            let _ = std::fs::write(path, text);
        }
    }
}

/// An edit box has the edit focus (widget flag 0x10000, `0x5D1000`).
fn editbox_focused(w: &aa_ui::widget::Widget) -> bool {
    (w.class == aa_ui::widget::WidgetClass::Editbox && w.flags & aa_ui::state::widget_flag::EDIT_FOCUS != 0)
        || w.children.iter().any(editbox_focused)
}

fn load_uvars(path: &Path) -> Option<BTreeMap<String, String>> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    serde_json::from_value(v.get("uvars")?.clone()).ok()
}

/// The profile's LAN name (profile+0x2C) saved with the options.
fn load_lan_name(path: &Path) -> Option<String> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    v.get("lan_name")?.as_str().filter(|s| !s.is_empty()).map(str::to_string)
}

/// The local kingdom's mission result as far as aa-play knows it:
/// `Some(true)` lost, `Some(false)` won, `None` still playing. A kingdom
/// defeated on its own lost whoever wins later (GKingdom+0x19C is set once,
/// 0x4E9720); a winner's team won and every other open kingdom lost
/// (0x4F4DA0 settles them with 4E9720(2)).
fn mission_result_lost(defeated: bool, team: u32, winner: Option<u32>) -> Option<bool> {
    if defeated {
        return Some(true);
    }
    winner.map(|t| t != team)
}

/// The finished world for the score screen: every kingdom of the world
/// (GWorld+0x154) with its `0x4E9560` name (`0x4E93C0`: the kingdom glyph
/// in the kingdom colour unless independent) and colour. aa-play keeps no
/// statistics snapshots (GKingdom+0x338; economy.rs `statistics_snapshot`
/// is empty), so the rows read 0 and the graph has no lines until it does.
fn score_data(ui: &Ui, b: &Battle, names: &std::collections::HashMap<String, String>) -> scoring::ScoreData {
    let gem = ui.strings.glyphs.get(&ui.avars.get("KingdomGlyphInfo").unwrap_or("ch_gem").to_ascii_lowercase()).copied().unwrap_or('#');
    let g = |v: f32| aa_ui::staging::fmt_g(v as f64);
    // The sheet (0x65A460) and the graph skip null kingdom slots. The
    // original lists only the player kingdoms: the QA captures of a scenario
    // (Market Garden 2v2) and of a random map (Southern Europe 2v2), both
    // with independent-owned cities, show the four players and no
    // independent row, so the independent kingdoms are empty slots here.
    let kingdoms = b
        .world
        .kingdoms
        .iter()
        .map(|k| {
            if k.independent {
                return None;
            }
            let color = [k.color[0] as f32 / 255.0, k.color[1] as f32 / 255.0, k.color[2] as f32 / 255.0];
            let display = names.get(&k.name.to_ascii_lowercase()).cloned().unwrap_or_else(|| ui.strings.resolve(&k.name));
            let name = format!("<color={},{},{}>{gem} <rc>{display}", g(color[0]), g(color[1]), g(color[2]));
            Some(scoring::KingdomScores { name, color, snapshots: Vec::new(), favorites: Default::default() })
        })
        .collect();
    scoring::ScoreData { kingdoms, kind: 1, networked: false, film: false }
}

/// Hand a launch to the loader (session::PendingSetup and its companions)
/// and show the loading screen.
fn start(f: &mut Front, commands: &mut Commands, next: &mut NextState<AppState>, launch: Launch) {
    commands.insert_resource(crate::session::PendingAiProfiles(launch.profiles));
    f.kingdom_names = launch.kingdom_names;
    f.starting_experience = launch.starting_experience;
    commands.insert_resource(launch.generals);
    match launch.fog {
        Some(mode) => commands.insert_resource(super::fog::PendingFog(mode)),
        None => commands.remove_resource::<super::fog::PendingFog>(),
    }
    commands.insert_resource(crate::session::PendingSetup(launch.setup));
    if let Some(params) = launch.random {
        commands.insert_resource(crate::session::PendingRandomMap(params));
    } else {
        commands.remove_resource::<crate::session::PendingRandomMap>();
    }
    commands.insert_resource(LoadProgress::default());
    next.set(AppState::Loading);
}

/// What a frame of the front end asks the app to do.
#[derive(Default)]
pub struct FrameOut {
    pub launch: bool,
    pub ready: bool,
    pub game: Vec<super::hud::GameAction>,
    /// RequestFinished: leave the battle for the score screen.
    pub leave: bool,
    /// RequestKeepPlaying / RequestObserve.
    pub keep_playing: bool,
    pub observe: bool,
    /// The score screen's Replay.
    pub replay: bool,
}

/// One frame's pointer and keyboard input, in window pixels.
#[derive(Default, Clone)]
pub struct PointerInput {
    pub pos: Option<Vec2>,
    pub left_pressed: bool,
    pub left_released: bool,
    pub right_pressed: bool,
    pub right_released: bool,
    pub wheel: f32,
}

fn ui_key(code: KeyCode) -> Option<(UiKey, &'static str)> {
    Some(match code {
        KeyCode::Enter | KeyCode::NumpadEnter => (UiKey::Enter, "enter"),
        KeyCode::Escape => (UiKey::Escape, "esc"),
        KeyCode::Backspace => (UiKey::Backspace, ""),
        KeyCode::Delete => (UiKey::Delete, ""),
        KeyCode::ArrowLeft => (UiKey::Left, ""),
        KeyCode::ArrowRight => (UiKey::Right, ""),
        KeyCode::Home => (UiKey::Home, ""),
        KeyCode::End => (UiKey::End, ""),
        KeyCode::Insert => (UiKey::Insert, ""),
        _ => return None,
    })
}

/// The per-frame front end: input, commands, page builds, the draw list.
#[allow(clippy::too_many_arguments)]
/// Logs a UI frame that took long (a main-thread stall the player feels).
struct SlowFrame(std::time::Instant, Page);

impl Drop for SlowFrame {
    fn drop(&mut self) {
        let ms = self.0.elapsed().as_secs_f32() * 1000.0;
        if ms > 30.0 {
            info!("slow UI frame: {ms:.0} ms on {:?}", self.1);
        }
    }
}

pub fn frame(
    mut front: ResMut<Front>,
    mut draw: ResMut<super::render::UiDraw>,
    windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    mut keyboard: MessageReader<KeyboardInput>,
    script: Option<ResMut<super::script::Script>>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
    mut battle: Option<ResMut<Battle>>,
    progress: Option<Res<LoadProgress>>,
    selecting: Option<Res<crate::input::Selecting>>,
    mut commands: Commands,
    mut quit: ResMut<super::Quit>,
) {
    let Ok(window) = windows.single() else { return };
    let size = (window.physical_width().max(1) as i32, window.physical_height().max(1) as i32);
    let _slow = SlowFrame(std::time::Instant::now(), front.page);
    let f = front.as_mut();
    f.battle_speed = battle.as_ref().map(|b| b.speed);
    // the app state decides the page
    let want = match state.get() {
        AppState::Menu => match f.page {
            Page::Loading | Page::Game => {
                // back from a load that failed (LoadProgress.error) or a game
                if let Some(err) = progress.as_ref().and_then(|p| p.error.clone()) {
                    f.custom.load_failed(&mut f.ui, &err);
                    f.message(&err);
                    commands.insert_resource(LoadProgress::default());
                }
                Some(Page::Staging)
            }
            _ => None,
        },
        AppState::Loading => (f.page != Page::Loading).then_some(Page::Loading),
        AppState::Battle => match f.page {
            Page::Loading if !f.loading_finished => {
                // the loader inserted the battle: original "Finished Loading"
                // with the Continue button (0x6E8C00); the battle waits
                f.loading_finished = true;
                if let Some(b) = battle.as_mut() {
                    b.paused = true;
                }
                screens::loading_finished(&mut f.ui);
                None
            }
            // the score screen while the battle state is left
            Page::Loading | Page::Game | Page::Scoring => None,
            _ => Some(Page::Game),
        },
    };
    if let Some(p) = want {
        if p == Page::Loading {
            f.loading_finished = false;
            // a new world: its mission results are still to come
            f.result_shown = false;
            f.over_shown = false;
        }
        f.show(p);
    }
    let battle_ref = battle.as_deref();
    if f.built_for != Some((size.0, size.1, f.page)) {
        f.build(size, battle_ref, progress.as_deref());
    } else if f.page == Page::Loading
        && !f.loading_finished
        && (progress.as_ref().is_some_and(|p| p.is_changed()) || f.loading_status != crate::session::load_status())
    {
        f.refresh_loading(progress.as_deref());
    }
    // pointer input (window physical pixels), script injection first
    let mut input = PointerInput {
        pos: window.physical_cursor_position(),
        left_pressed: buttons.just_pressed(MouseButton::Left),
        left_released: buttons.just_released(MouseButton::Left),
        right_pressed: buttons.just_pressed(MouseButton::Right),
        right_released: buttons.just_released(MouseButton::Right),
        wheel: wheel.read().map(|e| e.y).sum(),
    };
    let mut typed: Vec<(KeyCode, Option<String>)> = keyboard
        .read()
        .filter(|e| e.state == ButtonState::Pressed)
        .map(|e| {
            let text = match &e.logical_key {
                Key::Character(s) => Some(s.to_string()),
                Key::Space => Some(" ".into()),
                _ => e.text.as_ref().map(|t| t.to_string()),
            };
            (e.key_code, text)
        })
        .collect();
    if let Some(mut s) = script {
        s.inject(&mut f.ui, &mut input, &mut typed);
    }
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    f.press_on_ui = false;
    if input.left_pressed {
        f.left_held = true;
    }
    if input.left_released {
        f.left_held = false;
    }
    f.pointer = input.clone();
    f.pointer_over_ui = false;
    if let Some(p) = input.pos {
        let (x, y) = (p.x, p.y);
        let over_world = f.page == Page::Game
            && matches!(f.ui.press_target_5c3bc0(x as i16, y as i16), aa_ui::press_pick::PressTarget::VWorld);
        f.pointer_over_ui = !over_world;
        f.ui.mouse_move(x, y);
        if input.left_pressed {
            f.press_on_ui = !over_world;
            let now = f.now_ms();
            let double = f
                .last_press
                .is_some_and(|(t, px, py)| now.saturating_sub(t) <= DOUBLE_CLICK_MS && (x - px).abs() <= 2.0 && (y - py).abs() <= 2.0);
            if double {
                f.ui.mouse_double(x, y, UiButton::Left);
                f.last_press = None;
            } else {
                f.ui.mouse_down(x, y, UiButton::Left);
                f.last_press = Some((now, x, y));
            }
        }
        if input.left_released {
            f.ui.mouse_up(x, y, UiButton::Left);
        }
        if input.right_pressed {
            f.ui.mouse_down(x, y, UiButton::Right);
        }
        if input.right_released {
            f.ui.mouse_up(x, y, UiButton::Right);
        }
        if input.wheel != 0.0 && !over_world {
            f.ui.mouse_wheel(x, y, input.wheel);
        }
    }
    // keyboard: the focused edit box / open drop-down, then the UI hotkeys
    let mut game_keys = Vec::new();
    for (code, text) in typed {
        if code == KeyCode::Tab {
            f.ui.hotkey(if shift { "shift-tab" } else { "tab" });
            continue;
        }
        if ctrl && code == KeyCode::KeyA {
            f.ui.hotkey("ctrl-a");
            continue;
        }
        if ctrl && matches!(code, KeyCode::KeyC | KeyCode::KeyX) {
            f.ui.editbox_command(if code == KeyCode::KeyC { "EditboxCopy" } else { "EditboxCut" });
            continue;
        }
        if let Some((k, name)) = ui_key(code) {
            if !f.ui.key_down(k) && !name.is_empty() && !f.ui.hotkey(name) {
                game_keys.push(code);
            }
            continue;
        }
        let editing = editbox_focused(&f.ui.root);
        if let Some(t) = text.filter(|t| !ctrl && t.chars().all(|c| !c.is_control())) {
            if editing {
                f.ui.text_input(&t);
                continue;
            }
        }
        game_keys.push(code);
    }
    let now = f.now_ms();
    f.ui.update(now);
    if let Some(c) = &mut f.credits {
        // Update 0x646C10 posts CreditsEndCredits once nothing is left
        if c.update(&mut f.ui, now) && !f.credits_end_posted {
            f.credits_end_posted = true;
            f.ui.events.push(UiEvent::Command { widget: aa_ui::credits::PATH.into(), command: aa_ui::credits::END_COMMAND.into() });
        }
    }
    if f.dialog.is_some() && f.ui.message_box.is_none() {
        f.dialog = None;
    }
    let mut out = FrameOut::default();
    for e in f.ui.take_events() {
        match e {
            UiEvent::Command { command, widget } => f.command(&command, &widget, size, &mut out),
            UiEvent::Value { widget, value } => {
                let effects = f.options.as_mut().and_then(|o| o.value(&widget, value)).unwrap_or_default();
                f.effects(effects);
            }
            UiEvent::Select { widget, index, param } => {
                if f.page == Page::Staging {
                    f.custom.select(&mut f.ui, &widget, index, &param);
                }
            }
            UiEvent::Selected { .. } => {
                if f.page == Page::Staging {
                    f.custom.event(&mut f.ui, &e);
                }
            }
            UiEvent::Minimap { u, v, .. } => out.game.push(super::hud::GameAction::MinimapJump(u, v)),
            // button set_sound / unset_sound and the other UI sounds
            UiEvent::Sound { name } => {
                if !name.is_empty() && !name.eq_ignore_ascii_case("null") {
                    f.sounds.push(name);
                }
            }
            UiEvent::Clipboard { .. } => {}
        }
    }
    if f.page == Page::Staging {
        f.custom.update(&mut f.ui);
    }
    // the campaign pages: world ticks, labels and the MetaGame hotkeys (campaign/)
    if f.page == Page::Campaign {
        let now = f.now_ms();
        if f.campaign.update(&mut f.ui, now) {
            f.built_for = None;
        }
        for k in std::mem::take(&mut game_keys) {
            if let Some(a) = f.campaign.key(&mut f.ui, k) {
                f.campaign_action(a, size);
            }
        }
    }
    // the HUD reads the battle every frame
    if f.page == Page::Game {
        let hovered = input.pos.and_then(|p| f.ui.pick(p.x, p.y));
        let starting_experience = f.starting_experience;
        if let (Some(hud), Some(b)) = (f.hud.as_mut(), battle.as_deref()) {
            hud.starting_experience = starting_experience;
            if hud.names.kingdoms.is_empty() && !f.kingdom_names.is_empty() {
                hud.names.kingdoms = f.kingdom_names.clone();
            }
            hud.set_hover(hovered.as_deref());
            hud.update(&mut f.ui, b);
        }
        if let Some(b) = battle.as_deref() {
            f.mission_boxes(b);
        }
    }
    // RequestObserve (0x61EE60): ClearSelection, no local kingdom
    if out.observe {
        if let Some(b) = battle.as_mut() {
            b.selection.clear();
            b.observer = true;
        }
    }
    // RequestKeepPlaying for a finished game (0x60CAE0: GWorld+0x48 == 2):
    // the keep-playing decree's GENERIC_EVENT (0x772DD0, type 0xF)
    if out.keep_playing && battle.as_deref().is_some_and(|b| b.world.winner.is_some()) {
        let text = f.ui.strings.resolve("#ui_mission_message_host_elected_keep_playing");
        if let Some(hud) = f.hud.as_mut() {
            hud.events.send_message(&mut f.ui, "GENERIC_EVENT", &text, None);
        }
    }
    // RequestFinished: the world's scores for the score screen (slot 5); the
    // world itself ends with the battle state (scene::leave_battle)
    if out.leave {
        if let Some(b) = battle.as_deref() {
            f.score_data = Some(score_data(&f.ui, b, &f.kingdom_names));
        }
        commands.remove_resource::<Battle>();
        next.set(AppState::Menu);
        f.show(Page::Scoring);
    }
    if out.launch {
        if let Some((mut setup, random)) = f.custom.launch_setup() {
            f.custom.localize_setup(&f.ui, &mut setup);
            info!("launch: {setup:?}, generals {:?}", f.custom.launched_profiles);
            let starting = f.custom.rule_value("StartingExperience").and_then(|v| v.trim().parse::<f32>().ok());
            let launch = Launch {
                setup,
                random,
                profiles: f.custom.launched_profiles.clone(),
                // the generals and the StartingExperience rule (ui/abilities.rs)
                generals: super::abilities::PendingGenerals { factions: f.custom.launched_factions.clone(), starting_experience: starting },
                // the FogOfWar world rule of the session (EnumValue IDS)
                fog: f.custom.rule_value("FogOfWar").and_then(|v| aa_play::fog::FogMode::from_ids(&v)),
                kingdom_names: f.custom.kingdom_names(),
                starting_experience: starting.unwrap_or(0.0),
            };
            f.last_launch = Some(launch.clone());
            start(f, &mut commands, &mut next, launch);
        }
    }
    // the score screen's Replay: the same launch again (INFERRED: the
    // session's launch settings 0x76B4C0+0x4C are the staging's, resolved
    // at the launch, so the same setup and map seed)
    if out.replay {
        if let Some(launch) = f.last_launch.clone() {
            start(f, &mut commands, &mut next, launch);
        }
    }
    if out.ready {
        if let Some(b) = battle.as_mut() {
            b.paused = false;
        }
        f.loading_finished = false;
        f.show(Page::Game);
    }
    out.game.append(&mut f.pending);
    // the in-game action hotkeys go to the HUD's buttons first
    if f.page == Page::Game {
        if let Some(hud) = f.hud.as_mut() {
            game_keys.retain(|k| match hud.hotkey(&mut f.ui, *k) {
                Some(acts) => {
                    out.game.extend(acts);
                    false
                }
                None => true,
            });
        }
    }
    // only a running battle reads them (a menu's would wait for the next one)
    if battle.is_some() && !out.leave && (!out.game.is_empty() || !game_keys.is_empty()) {
        commands.insert_resource(super::hud::GameInput { actions: std::mem::take(&mut out.game), keys: game_keys });
    }
    if f.built_for != Some((size.0, size.1, f.page)) {
        f.build(size, battle.as_deref(), progress.as_deref());
    }
    // tooltips (after the HUD set this frame's buttons)
    {
        let pointer = input.pos.map(|p| (p.x, p.y));
        let pressed = input.left_pressed || input.right_pressed;
        let game = f.page == Page::Game;
        let now = f.now_ms();
        let hud = f.hud.as_ref();
        let dynamic = |p: &str| hud.and_then(|h| h.tooltip(p));
        f.tooltips.frame(&mut f.ui, pointer, pressed, now, game, &dynamic);
    }
    f.ui.layout();
    let mut list = f.ui.draw_list();
    // the drag box (0x5CD850): a flat screen rectangle of vertex colour
    // (0, 0, 0.5) at 30 % alpha, drawn with the world (under the HUD)
    if f.page == Page::Game {
        if let Some((a, b)) = selecting.as_ref().and_then(|s| s.drag) {
            let (lo, hi) = (a.min(b), a.max(b));
            list.commands.insert(
                0,
                aa_ui::draw::DrawCmd {
                    widget: String::new(),
                    ve: String::new(),
                    kind: aa_ui::draw::DrawKind::Quad(aa_ui::draw::Quad {
                        texture: None,
                        rect: aa_ui::layout::Rect { l: lo.x, t: lo.y, r: hi.x, b: hi.y },
                        uv: [0.0, 0.0, 1.0, 1.0],
                        color: [0.0, 0.0, 0.5, 0.3],
                        blend: aa_ui::draw::Blend::Alpha,
                        address: Default::default(),
                        clip: None,
                        effect: 0,
                        tint: [0.0; 3],
                    }),
                },
            );
        }
    }
    // the score screen's graph lines (GraphVE, ui/scoring.rs)
    if f.page == Page::Scoring {
        if let (Some(s), Some(defs)) = (f.scoring.as_ref(), f.scoring_defs.as_ref()) {
            s.draw(&mut f.ui, defs, &mut list);
        }
    }
    if matches!(f.page, Page::Loading | Page::Scoring) {
        // the frame clears to black (uncovered areas of 03-launch.png); the
        // score screen's alpha-blended panel shows no world either
        list.commands.insert(
            0,
            aa_ui::draw::DrawCmd {
                widget: String::new(),
                ve: String::new(),
                kind: aa_ui::draw::DrawKind::Quad(aa_ui::draw::Quad {
                    texture: None,
                    rect: aa_ui::layout::Rect { l: 0.0, t: 0.0, r: size.0 as f32, b: size.1 as f32 },
                    uv: [0.0, 0.0, 1.0, 1.0],
                    color: [0.0, 0.0, 0.0, 1.0],
                    blend: aa_ui::draw::Blend::Opaque,
                    address: Default::default(),
                    clip: None,
                    effect: 0,
                    tint: [0.0; 3],
                }),
            },
        );
    }
    draw.list = list;
    if f.quit {
        quit.requested = true;
    }
}

#[cfg(test)]
mod tests {
    use super::mission_result_lost;

    #[test]
    fn mission_result_follows_the_winner_and_the_defeat() {
        // still playing
        assert_eq!(mission_result_lost(false, 0, None), None);
        // defeated on its own while the others play on
        assert_eq!(mission_result_lost(true, 0, None), Some(true));
        // its team won
        assert_eq!(mission_result_lost(false, 0, Some(0)), Some(false));
        // another team won by the city ratio (0x4E9A40): nobody was
        // defeated, 0x4F4DA0 settles the open kingdom as lost
        assert_eq!(mission_result_lost(false, 0, Some(1)), Some(true));
        // defeated earlier stays lost when its team wins later (+0x19C once)
        assert_eq!(mission_result_lost(true, 0, Some(0)), Some(true));
        // everybody fell in the same tick
        assert_eq!(mission_result_lost(true, 0, Some(u32::MAX)), Some(true));
    }
}
