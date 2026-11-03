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
use aa_parity::playback::{FilmData, FilmEvent, FilmFailure, FilmPlayer, SyncMode, open_film};
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
    /// The main menu's film route (`RequestFilm` 0x64F3F0 -> 0x64EFF0: session
    /// kind 3, menu slot 9): ChooseFilm (0x6455B0, `UI/Menus/choose_film.tgi`)
    /// shows the menu background and only the film mode of the file dialog.
    ChooseFilm,
    /// `MultiplayerMenu` (0x653780) and the LAN `LobbyMenu` (0x64AB20): `crate::net`.
    Multiplayer,
    Lobby,
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
    /// The DifficultyLevel world rule of the session (ui/objectives.rs).
    difficulty: Option<String>,
    /// The Sun world rule of the session (scene.rs, 0x4FDCE0).
    sun: Option<String>,
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
    /// The World War II campaign and the scripted campaigns (campaign owner).
    pub campaign: crate::campaign::CampaignUi,
    /// A battle a campaign page launches, for this frame's `start`.
    campaign_launch: Option<Box<crate::campaign::BattleLaunch>>,
    /// The local kingdom's result when the battle was left (Kingdom+19C == 1).
    campaign_won: bool,
    /// The world's winning team then.
    campaign_winner: Option<u32>,
    /// The running battle's speed limits (exponents), for the options slider.
    battle_speed_limits: (f32, f32),
    /// The film being opened (`KKC_ResourceManagerTaskLoadFilm`, 0x5A3980).
    film_load: Option<FilmLoad>,
    /// The End of Film box is up: it has neither an accept nor a cancel
    /// button, Enter and Esc do nothing (0x66E790, [O]).
    film_box_without_keys: bool,
    /// ObserverGameMusic was asked for (0x4F96D0, first world tick).
    observer_music: bool,
    /// The out-of-sync text went to the message log.
    film_oos_told: bool,
    /// The film session started from the main menu (session kind 3): Exit on
    /// the score screen goes to slot 9 (`0x659290`).
    film_from_menu: bool,
    /// Session+0x150: the film of the running or last game (`0x76FE60`), none
    /// while viewing one. (Appended by the recording owner.)
    pub session_film: Option<aa_parity::recording::FilmOpened>,
    /// The score screen's film save dialog (`0x68DCE0(5, 0, 0)`).
    save_film: Option<crate::film_record::SaveFilmDialog>,
    /// The multiplayer session (`crate::net`).
    pub lan: crate::net::Lan,
    /// The battle that just ended was a session's: its score screen's Exit
    /// goes to the lobby.
    lan_left_game: bool,
}

/// The background open of a film.
type FilmLoad = std::sync::Arc<std::sync::Mutex<Option<Result<(aa_play::World, FilmPlayer), FilmFailure>>>>;

/// `AA_FILM_SYNC=strict`: the native stop at the first differing checkpoint
/// (0x752F60, 0x751EB0). Default until films pass: report and keep playing
/// (FILM-PLAYBACK.md section 11).
fn film_sync_mode() -> SyncMode {
    match std::env::var("AA_FILM_SYNC").as_deref() {
        Ok(v) if v.eq_ignore_ascii_case("strict") => SyncMode::Strict,
        _ => SyncMode::Report,
    }
}

/// Commands the front end does not run yet (their screens are not built).
const NOT_BUILT: &[&str] = &[
    "RequestEditor",
    "CheckForUpdates",
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
        // the profile's film (preferences +0x18, saved with the rest, 0x41D570)
        if let Some(film) = settings_path.as_ref().and_then(|p| load_last_film(p)) {
            custom.remember_film(film);
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
            lan: crate::net::Lan::new(root.to_path_buf()),
            lan_left_game: false,
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
            campaign_launch: None,
            campaign_won: false,
            campaign_winner: None,
            battle_speed_limits: (-2.0, 2.0),
            film_load: None,
            film_box_without_keys: false,
            observer_music: false,
            film_oos_told: false,
            film_from_menu: false,
            session_film: None,
            save_film: None,
        }
    }

    /// `0x652B10` / KKC_Session::Launch for the session's film: the film
    /// opens on a background thread behind the loading screen (0x76C0A0 ->
    /// 0x5A3980); the map's scene loads once it has, with the world frozen
    /// until Continue. The speed starts at 100% (a new `Battle`, [O]).
    fn launch_film(&mut self) {
        let Some(film) = self.custom.film().cloned() else { return };
        let game_root = self.root.parent().map(Path::to_path_buf).unwrap_or_else(|| self.root.clone());
        let mut data = FilmData::for_film(game_root.clone(), &film.path);
        data.force_playback = self.ui.uvars.get_bool("FilmForcePlayback");
        data.draws_scene = true;
        let sync = film_sync_mode();
        info!("film: launch {} (sync {sync:?}, overlay {:?})", film.path.display(), data.overlay);
        let slot: FilmLoad = Default::default();
        let out = slot.clone();
        let path = film.path.clone();
        let reconstruction = self.root.clone();
        crate::session::set_load_status("app_status_loading_data");
        std::thread::spawn(move || {
            let started = Instant::now();
            // A film on a profile's data (lead's decision 6) draws that data's art:
            // its conversion lies over the stock art until the battle ends.
            let art = data.definition_overlay().and_then(crate::session::art_folder);
            if let (Some(art), Some(folder)) = (&art, data.definition_overlay())
                && !art.join("assets/converted/models/manifest.json").is_file()
            {
                crate::session::convert_art(&reconstruction, folder, art);
            }
            if let Ok(store) = crate::preload::store(&reconstruction) {
                store.set_art_overlay(art.as_deref());
            }
            // what the client has loaded already
            data.preloaded.game = crate::preload::game_data(&game_root).ok();
            data.preloaded.generals = crate::preload::generals(&game_root).ok();
            data.preloaded.random_maps = crate::preload::random_maps(&game_root).ok();
            // the world load's stages (`session::LOAD_STATUS`): a random map's
            // creator runs the generator
            if std::fs::read(&path).ok().and_then(|b| aa_parity::film::read_header(b).ok()).is_some_and(|h| matches!(h.header.creator, aa_mapworld::header::DerivedCreator::Random(_))) {
                crate::session::set_load_status("app_status_generating_map");
            }
            // a panic of the load is the task's failure, not a screen that
            // waits for ever
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| open_film(&path, &data, sync)))
                .unwrap_or_else(|_| Err(FilmFailure::CreateWorld { detail: "the film's load panicked".into() }));
            info!("film: opened in {:.2} s", started.elapsed().as_secs_f32());
            if let Ok(mut s) = out.lock() {
                *s = Some(result);
            }
        });
        self.film_load = Some(slot);
        self.clear_message_log();
        self.loading_finished = false;
        self.result_shown = false;
        self.over_shown = false;
        self.observer_music = false;
        self.film_oos_told = false;
        self.show(Page::Loading);
    }

    /// KKC_Session::Launch 0x77A950 -> 0x770500, leaving state 1 (the staging):
    /// `KKC_MessageSystem::Clear` 0x40B450 empties the log and starts its ids
    /// over, so a launch never shows the last battle's messages.
    fn clear_message_log(&mut self) {
        self.ui.messages.clear();
        self.ui.messages.last = 0;
    }

    /// The failure of a film's open: the text of `FilmFailure`
    /// (`failed_film_*`, 0x5A3980 / 0x5A2B90, `load_error_missing_map|s`,
    /// `unable_to_select_film_source_file_not_found|s`) with one Ok on the
    /// page the launch came from; no world was made.
    fn film_failed(&mut self, failure: &FilmFailure) {
        warn!("film: {failure:?}");
        let text = super::dialogs::film_failure_text(&self.ui, failure);
        self.show(if self.film_from_menu { Page::ChooseFilm } else { Page::Staging });
        self.dialog = Some((aa_ui::message_box::SHARED.to_string(), aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(self.ui.strings.resolve("#button_ok"), String::new())] }));
    }

    /// `SyncMode::Report` (lead's decision 1): the native text in the message
    /// log, once, so the player is not misled; the film keeps playing.
    fn film_out_of_sync_message(&mut self) {
        if self.film_oos_told {
            return;
        }
        self.film_oos_told = true;
        let text = self.ui.strings.resolve("#film_end_of_film_oos");
        if let Some(hud) = self.hud.as_mut() {
            hud.events.send_message(&mut self.ui, "GENERIC_EVENT", &text, None);
        }
    }

    /// The film state's notification handler `0x652020`, END_OF_FILM (type
    /// 0x13) on the host: out of sync one Ok with `RequestFinished` (0x66F4F0),
    /// a natural end `film_end_of_film2` with `film_exit_to_scoring` and
    /// `film_keep_watching` (0x66D360 / 0x66E790; the second only while
    /// GWorld+0x275 is clear, which `aa-play` does not keep yet).
    ///
    /// Lead's decision 1: in `SyncMode::Report` a mismatch logs once and goes
    /// to the message log (`FirstMismatch`), and the natural end then comes
    /// with `out_of_sync` set (`aa_parity::playback`), the out-of-sync box.
    fn film_end(&mut self, out_of_sync: bool, keep_watching: bool) {
        if out_of_sync {
            let text = self.ui.strings.resolve("#film_end_of_film_oos");
            let ok = self.ui.strings.resolve("#button_ok");
            self.film_box_without_keys = false;
            self.ask(&text, vec![(ok, "RequestFinished".into())]);
        } else {
            let text = self.ui.strings.resolve("#film_end_of_film2");
            let exit = self.ui.strings.resolve("#film_exit_to_scoring");
            let keep = self.ui.strings.resolve("#film_keep_watching");
            self.film_box_without_keys = true;
            // the second button only with GWorld+0x275 clear
            let mut buttons = vec![(exit, "RequestFinished".to_string())];
            if keep_watching {
                buttons.push((keep, String::new()));
            }
            self.ask(&text, buttons);
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
            // (a film: GameSpeedFilmMin / GameSpeedFilmMax, 0x675F80, 0x6A2670)
            let (lo, hi) = self.battle_speed_limits;
            o.game_speed = self.battle_speed.filter(|_| self.page == Page::Game).map(|s| (lo, hi, s.max(1e-3).log2()));
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
        // a film: 0x440190 returns after its first branch, 0x4408A0 is skipped
        // by its first check
        if b.in_film() {
            return;
        }
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
            Page::Staging if self.lan.active() => self.lan.sync_staging(&mut self.ui, &mut self.custom, true),
            Page::Staging => self.custom.show(&mut self.ui),
            Page::Multiplayer => crate::net::multiplayer_menu(&mut self.ui),
            Page::Lobby => crate::net::lobby_menu(&mut self.ui),
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
            // show_menu_background = true, no interface of its own: the
            // staging file dialog in mode 5 (0x6455B0, 0x68DB60(5, 0))
            Page::ChooseFilm => {
                self.ui.clear();
                screens::menu_backdrop(&mut self.ui);
                self.custom.open_film_dialog(&mut self.ui, false);
            }
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
        // the multiplayer menus and the staging of a networked session (`crate::net`)
        if matches!(self.page, Page::Multiplayer | Page::Lobby) {
            if let Some(a) = self.lan.lobby_command(&mut self.ui, &mut self.custom, command) {
                if self.ui.message_box.is_some() {
                    self.remember_dialog();
                }
                self.lan_action(a, out);
                return;
            }
        }
        if self.page == Page::Staging && self.lan.active() {
            if let Some(a) = self.lan.staging_command(&mut self.ui, &mut self.custom, command, widget) {
                self.lan_action(a, out);
                return;
            }
            if let Some(action) = self.custom.command(&mut self.ui, command, widget) {
                if self.ui.message_box.is_some() {
                    self.remember_dialog();
                }
                if std::mem::take(&mut self.custom.net_dialog_choice) {
                    self.lan.after_custom_command(&mut self.custom);
                }
                match action {
                    // StagingExit: 0x651990 leaves the session
                    super::custom::Action::Exit => {
                        self.lan.leave();
                        self.custom.net_reset();
                        self.show(Page::Lobby);
                    }
                    // StagingLaunch 0x65DE80 -> 0x651950
                    super::custom::Action::Launch => self.lan.start_launch(),
                    _ => {}
                }
                return;
            }
        }
        if matches!(self.page, Page::Staging | Page::ChooseFilm) {
            if let Some(action) = self.custom.command(&mut self.ui, command, widget) {
                if self.ui.message_box.is_some() {
                    self.remember_dialog();
                }
                match action {
                    super::custom::Action::None => {}
                    super::custom::Action::Exit => self.show(Page::SinglePlayer),
                    super::custom::Action::Launch => out.launch = true,
                    // Commit 0x68EF50, mode 5: the staging shows the film;
                    // the main menu's `FilmOK` (0x645200) launches at once
                    // (0x652B10)
                    super::custom::Action::FilmChosen => {
                        self.save_settings();
                        if self.page == Page::ChooseFilm {
                            out.launch = true;
                        } else {
                            self.custom.refresh_staging(&mut self.ui);
                        }
                    }
                    // FilmCancel 0x645540, session kind 3: 0x651990 leaves
                    // the local session and shows the main menu
                    super::custom::Action::FilmDialogCancelled => {
                        if self.page == Page::ChooseFilm {
                            self.film_from_menu = false;
                            self.show(Page::MainMenu);
                        }
                    }
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
        if let Some(dialog) = self.save_film.as_mut() {
            let data = self.custom.game_data(&self.ui);
            if let Some(outcome) = dialog.command(&mut self.ui, data, command) {
                if outcome == crate::film_record::SaveOutcome::Closed {
                    // 0x690D75: the saved film is the profile's film now
                    if let Some(path) = self.save_film.take().and_then(|d| d.saved) {
                        self.custom.remember_film(path);
                        self.save_settings();
                    }
                    let film = crate::film_record::film_exists(self.session_film.as_ref());
                    if let Some(s) = self.scoring.as_mut() {
                        s.update_save_film(&mut self.ui, film);
                    }
                    // 0x600CE0: the debriefing's button goes the same way.
                    self.ui.with_widget("DebriefingInterface/SaveFilmButton", |w| w.set_visible(film));
                }
                return;
            }
        }
        if self.page == Page::Scoring {
            if let (Some(s), Some(defs)) = (self.scoring.as_mut(), self.scoring_defs.as_ref()) {
                if let Some(outcome) = s.command(&mut self.ui, defs, command) {
                    match outcome {
                        scoring::Outcome::None => {}
                        // 0x659290: a custom game goes back to the staging
                        // (slot 3), the film still selected; a film from the
                        // main menu (session kind 3) to slot 9, ChooseFilm
                        scoring::Outcome::Exit => {
                            // a campaign's battle goes back to its campaign page
                            if self.campaign.battle_ended(self.campaign_won, self.campaign_winner) {
                                self.show(Page::Campaign);
                            } else {
                                self.custom.scoring_available(self.score_data.is_some());
                                // a left session goes back to the lobby (0x651990 ran)
                                if self.lan_left_game {
                                    self.lan_left_game = false;
                                    self.custom.net_reset();
                                    self.show(Page::Lobby);
                                } else {
                                self.show(if self.film_from_menu { Page::ChooseFilm } else { Page::Staging });
                                }
                            }
                        }
                        scoring::Outcome::Replay => out.replay = true,
                        // slot 4 (a campaign's mission)
                        scoring::Outcome::Debriefing => self.show(Page::Campaign),
                        // 0x658F90 -> 0x68DCE0(5, 0, 0)
                        scoring::Outcome::SaveFilm => {
                            if let Some(film) = self.session_film.clone() {
                                let name = self.custom.loading_info().0;
                                self.save_film = Some(crate::film_record::SaveFilmDialog::open(&mut self.ui, &film, &name));
                            }
                        }
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
            // 0x650A80: the multiplayer menu (slot of MultiplayerMenu 0x653780)
            "RequestMultiplayer" => {
                self.lan.local_name = self.lan_name.clone().unwrap_or_else(|| self.ui.strings.resolve("#default_lan_name"));
                self.lan.warm(&self.ui, &mut self.custom);
                self.show(Page::Multiplayer);
            }
            "LobbyExit" => self.show(Page::Multiplayer),
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
                let text = self.ui.strings.resolve(if self.campaign.in_mission() { "#campaign_end_confirm" } else { "#score_screen_confirm" });
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
            // 0x65B9D0: the scripted campaigns' chooser (campaign/missions.rs)
            "RequestCampaign" => {
                self.campaign.enter_missions(&mut self.ui);
                self.show(Page::Campaign);
            }
            "RequestCustom" => {
                self.custom.enter(&mut self.ui);
                self.film_from_menu = false;
                self.show(Page::Staging);
            }
            // 0x64F3F0 -> 0x64EFF0 (0x7513F0(3, 1), 0x7840B0(3, 0, 0)): the
            // local session, kind 3, whose menu is ChooseFilm (slot 9)
            "RequestFilm" => {
                self.film_from_menu = true;
                self.show(Page::ChooseFilm);
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

    /// What the session asked of the front end (`crate::net`).
    fn lan_action(&mut self, action: crate::net::Action, out: &mut FrameOut) {
        use crate::net::Action as A;
        match action {
            A::None => {}
            A::Staging => {
                self.film_from_menu = false;
                self.show(Page::Staging);
            }
            A::Lobby => {
                self.custom.net_reset();
                self.show(Page::Lobby);
            }
            A::LoadWorld => out.lan_load = true,
            A::BeginPlay => {
                self.loading_finished = false;
                self.show(Page::Game);
            }
            A::SessionLost => out.leave = true,
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
            // the frame hands it to the loader
            A::Launch(l) => self.campaign_launch = Some(l),
            // 0x600C70: the score screen (slot 5)
            A::Scoring => self.show(Page::Scoring),
            // DebriefingSaveFilm: the score screen's dialog (0x68DCE0(5, 0, 0))
            A::SaveFilm => {
                if let Some(film) = self.session_film.clone() {
                    let name = self.custom.loading_info().0;
                    self.save_film = Some(crate::film_record::SaveFilmDialog::open(&mut self.ui, &film, &name));
                }
            }
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
            if let Some(film) = self.custom.remembered_film() {
                obj.insert("last_film".into(), serde_json::Value::String(film.to_string_lossy().into_owned()));
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

/// The profile's film (preferences +0x18).
fn load_last_film(path: &Path) -> Option<PathBuf> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    v.get("last_film")?.as_str().filter(|s| !s.is_empty()).map(PathBuf::from)
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
/// in the kingdom colour unless independent), colour, statistics snapshots
/// (GKingdom+0x338, `aa_play::statistics`, each read through `0x58F3C0` by
/// StatData index) and favourites (`0x58F820` -> `0x58D9D0`: a definition's name, a
/// kingdom's `0x4E9560` for the enemy, no entry reads score_graph_none).
fn score_data(ui: &Ui, b: &Battle, names: &std::collections::HashMap<String, String>) -> scoring::ScoreData {
    let gem = ui.strings.glyphs.get(&ui.avars.get("KingdomGlyphInfo").unwrap_or("ch_gem").to_ascii_lowercase()).copied().unwrap_or('#');
    let g = |v: f32| aa_ui::staging::fmt_g(v as f64);
    let table = &b.world.data.statistics;
    let total = table.derived_start + table.derived.len();
    let shown_name = |name: &str| names.get(&name.to_ascii_lowercase()).cloned().unwrap_or_else(|| ui.strings.resolve(name));
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
            let display = shown_name(&k.name);
            let name = format!("<color={},{},{}>{gem} <rc>{display}", g(color[0]), g(color[1]), g(color[2]));
            let snapshots = b
                .world
                .books
                .kingdom(k.id)
                .map(|books| books.statistics.snapshots.iter().map(|s| (0..total).map(|i| s.value(table, i)).collect()).collect())
                .unwrap_or_default();
            let mut favorites = std::collections::HashMap::new();
            for index in [aa_play::statistics::FAVORITE_UNIT, aa_play::statistics::FAVORITE_UNIT + 1, aa_play::statistics::FAVORITE_UNIT + 2, aa_play::statistics::FAVORITE_ORGANIZATION] {
                if let Some(def) = aa_play::statistics::favorite_definition(&b.world, k.id, index) {
                    favorites.insert(index, ui.strings.resolve(&b.world.data.def(def).name));
                }
            }
            if let Some(enemy) = aa_play::statistics::favorite_enemy(&b.world, k.id).and_then(|e| b.world.kingdoms.get(e as usize)) {
                favorites.insert(aa_play::statistics::FAVORITE_ENEMY, shown_name(&enemy.name));
            }
            Some(scoring::KingdomScores { name, color, snapshots, favorites })
        })
        .collect();
    scoring::ScoreData { kingdoms, kind: 1, networked: false, film: false }
}

/// What the launch takes for a film: its loader draws the film's own world
/// (`session::PendingFilm`), with no computer players (the SAI main is gated off
/// in state 4, 0x7A0080), and the generals and fog of that world
/// (`aa_parity::playback`).
fn film_launch(world: &aa_play::World, player: &FilmPlayer) -> Option<Launch> {
    let setup = player.scene().setup.clone()?;
    let names = world.kingdoms.iter().filter(|k| k.independent).map(|k| (k.name.to_ascii_lowercase(), k.name.clone())).collect();
    Some(Launch { setup, random: None, profiles: Vec::new(), generals: Default::default(), fog: None, kingdom_names: names, starting_experience: 0.0, difficulty: None, sun: None })
}

/// Hand a launch to the loader (session::PendingSetup and its companions)
/// and show the loading screen.
fn start(f: &mut Front, commands: &mut Commands, next: &mut NextState<AppState>, launch: Launch) {
    // OnGWorldReadyForState 0x7757F0 will open a film for this launch; a film
    // being viewed records none.
    if f.custom.film().is_some() {
        commands.remove_resource::<crate::film_record::PendingFilmSession>();
    } else {
        let random_generals = (0..launch.setup.players.len())
            .filter(|i| !launch.generals.factions.iter().any(|(p, faction)| p == i && faction.is_some()))
            .collect();
        commands.insert_resource(crate::film_record::PendingFilmSession {
            setup: launch.setup.clone(),
            random: launch.random.clone(),
            // A campaign's battle is no staging session's: its map's rows or the defaults.
            rules: if f.campaign.battle.is_some() { Vec::new() } else { f.custom.film_rule_rows() },
            profiles: launch.profiles.clone(),
            random_generals,
        });
    }
    commands.insert_resource(crate::session::PendingAiProfiles(launch.profiles));
    f.kingdom_names = launch.kingdom_names;
    f.starting_experience = launch.starting_experience;
    commands.insert_resource(launch.generals);
    match launch.fog {
        Some(mode) => commands.insert_resource(super::fog::PendingFog(mode)),
        None => commands.remove_resource::<super::fog::PendingFog>(),
    }
    match launch.difficulty {
        Some(d) => commands.insert_resource(super::objectives::PendingDifficulty(d)),
        None => commands.remove_resource::<super::objectives::PendingDifficulty>(),
    }
    match launch.sun {
        Some(ids) => commands.insert_resource(crate::session::PendingSunRule(ids)),
        None => commands.remove_resource::<crate::session::PendingSunRule>(),
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
    /// `LanEvent::LoadWorld`: the session's launch task builds the battle.
    pub lan_load: bool,
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

fn digit_key(text: &str) -> Option<KeyCode> {
    Some(match text {
        "0" => KeyCode::Digit0,
        "1" => KeyCode::Digit1,
        "2" => KeyCode::Digit2,
        "3" => KeyCode::Digit3,
        "4" => KeyCode::Digit4,
        "5" => KeyCode::Digit5,
        "6" => KeyCode::Digit6,
        "7" => KeyCode::Digit7,
        "8" => KeyCode::Digit8,
        "9" => KeyCode::Digit9,
        _ => return None,
    })
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
    mut sfx: Option<ResMut<crate::audio::SfxQueue>>,
) {
    let Ok(window) = windows.single() else { return };
    let size = (window.physical_width().max(1) as i32, window.physical_height().max(1) as i32);
    let _slow = SlowFrame(std::time::Instant::now(), front.page);
    let f = front.as_mut();
    f.battle_speed = battle.as_ref().map(|b| b.speed);
    if let Some(b) = battle.as_ref() {
        f.battle_speed_limits = b.speed_limits();
    }
    // the app state decides the page
    let want = match state.get() {
        AppState::Menu => match f.page {
            // the film is opening: its loading screen stays
            Page::Loading if f.film_load.is_some() => None,
            Page::Loading | Page::Game => {
                // back from a load that failed (LoadProgress.error) or a game
                if let Some(err) = progress.as_ref().and_then(|p| p.error.clone()) {
                    f.custom.load_failed(&mut f.ui, &err);
                    f.message(&err);
                    commands.insert_resource(LoadProgress::default());
                }
                Some(if f.film_from_menu { Page::ChooseFilm } else { Page::Staging })
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
                if f.lan.active() {
                    // 0x6E8C00: no Continue in a multiplayer launch; the battle
                    // joins the session and the task is reported (304)
                    if let Some(b) = battle.as_mut() {
                        let (kingdoms, _) = f.lan.kingdoms(&f.custom);
                        f.lan.attach(b, kingdoms);
                    }
                } else {
                    screens::loading_finished(&mut f.ui);
                }
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
        // a character a script types is its key when no edit box takes it
        // (the observer's `map 1 ... SelectObservingKingdom 0`)
        let code = match (code, text.as_deref()) {
            (KeyCode::Unidentified(_), Some(t)) if !editbox_focused(&f.ui.root) => digit_key(t).unwrap_or(code),
            _ => code,
        };
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
        if f.film_box_without_keys && f.ui.message_box.is_some() && matches!(code, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Escape) {
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
    if f.ui.message_box.is_none() {
        f.film_box_without_keys = false;
    }
    let mut out = FrameOut::default();
    // the session's frame, and its events (`crate::net`)
    if f.lan.active() {
        let staging = f.page == Page::Staging;
        let Front { lan, ui, custom, .. } = f;
        let actions = lan.pump(ui, staging, custom);
        for a in actions {
            f.lan_action(a, &mut out);
        }
    }
    for e in f.ui.take_events() {
        match e {
            UiEvent::Command { command, widget } => f.command(&command, &widget, size, &mut out),
            UiEvent::Value { widget, value } => {
                let effects = f.options.as_mut().and_then(|o| o.value(&widget, value)).unwrap_or_default();
                f.effects(effects);
            }
            UiEvent::Select { widget, index, param } => {
                if f.page == Page::Staging && f.lan.active() {
                    f.lan.staging_select(&mut f.ui, &mut f.custom, &widget, &param);
                } else if f.page == Page::Staging {
                    f.custom.select(&mut f.ui, &widget, index, &param);
                }
            }
            UiEvent::Selected { .. } => {
                if f.page == Page::Staging {
                    f.custom.event(&mut f.ui, &e);
                }
            }
            // the strategic map's camera goes there (0x5D77A0 -> 6FF200)
            UiEvent::Minimap { u, v, .. } if f.page == Page::Campaign => f.campaign.minimap_jump(u, v),
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
        // the film state's notifications (0x652020) and, under Report, the
        // first mismatch (lead's decision 1); 0x4F96D0 on the first world
        // tick: no kingdom plays AVars ObserverGameMusic
        if let Some(b) = battle.as_mut().filter(|b| b.in_film()) {
            let events = b.take_film_events();
            let tick = b.world.tick;
            for e in events {
                match e {
                    FilmEvent::FirstMismatch { index } => {
                        warn!("FILM OUT OF SYNC({index})");
                        f.film_out_of_sync_message();
                    }
                    // 0x75D710: the same stop as a differing checkpoint
                    FilmEvent::NoOrderingPlayer { player } => {
                        warn!("Film order with no ordering player {player}");
                        f.film_out_of_sync_message();
                    }
                    // a 328's kingdom name: `World::kingdoms[k].name` has it
                    FilmEvent::KingdomRenamed { kingdom, name } => info!("film: kingdom {kingdom} is now {name}"),
                    FilmEvent::EndOfFilm { out_of_sync } => {
                        info!("film: END_OF_FILM out_of_sync={out_of_sync} at tick {tick}: {:?}", b.film.as_ref().map(|p| p.stats()));
                        f.film_end(out_of_sync, !aa_play::triggers::cutscene(&b.world));
                    }
                }
            }
            if !f.observer_music && tick > 0 {
                f.observer_music = true;
                if let (Some(ids), Some(q)) = (f.ui.avars.get("ObserverGameMusic").map(aa_ui::node::unquote), sfx.as_mut()) {
                    q.play.push(crate::audio::Sfx { ids, place: crate::audio::Place::Global, owner: None, voice: None, looping: true });
                }
            }
        }
    }
    // RequestObserve (0x61EE60): ClearSelection, no local kingdom
    if out.observe {
        if let Some(b) = battle.as_mut() {
            b.observe();
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
            // 0x658FF0: SaveFilm while the game's film exists (0x772BB0)
            if let Some(data) = f.score_data.as_mut() {
                data.film = crate::film_record::film_exists(f.session_film.as_ref());
            }
            // Kingdom+19C of the local kingdom: what the campaign page reads
            f.campaign_won = aa_play::triggers::mission_result(&b.world, b.local) == 1;
            f.campaign_winner = b.world.winner;
            // 0x60A140 / 0x651BD0: a scripted mission's end order goes to the
            // debriefing (slot 4), whose scores are a campaign's (kind 0)
            if f.campaign.in_mission() {
                let film = f.score_data.as_ref().is_some_and(|d| d.film);
                if let Some(data) = f.score_data.as_mut() {
                    data.kind = 0;
                }
                let result = crate::campaign::debriefing::MissionResult {
                    won: f.campaign_won,
                    film,
                    objectives: aa_play::triggers::objectives(&b.world, b.local)
                        .into_iter()
                        .map(|o| crate::campaign::debriefing::ObjectiveLine {
                            text: if o.text.starts_with('#') { f.ui.strings.resolve(&o.text) } else { o.text.clone() },
                            primary: o.primary,
                            state: o.state as u8,
                        })
                        .collect(),
                };
                f.campaign.mission_ended(&f.ui, result);
            }
        }
        if f.lan.active() {
            f.lan.leave();
            f.lan_left_game = true;
        } else if f.lan.in_battle {
            f.lan.in_battle = false;
            f.lan_left_game = true;
        }
        let mission_page = f.campaign.screen == crate::campaign::Screen::Debriefing || (f.campaign.screen == crate::campaign::Screen::Missions && f.campaign.battle.is_none() && f.film_load.is_none() && f.score_data.as_ref().is_some_and(|d| d.kind == 0));
        commands.remove_resource::<Battle>();
        next.set(AppState::Menu);
        f.show(if mission_page { Page::Campaign } else { Page::Scoring });
    }
    // a campaign page's battle (campaign owner)
    if let Some(l) = f.campaign_launch.take() {
        let launch = Launch {
            setup: l.setup,
            random: l.random,
            profiles: l.profiles,
            generals: super::abilities::PendingGenerals { factions: l.factions, starting_experience: l.starting_experience },
            fog: l.fog.and_then(|v| aa_play::fog::FogMode::from_ids(&v)),
            kingdom_names: l.names.into_iter().collect(),
            starting_experience: l.starting_experience.unwrap_or(0.0),
            difficulty: l.difficulty,
            sun: None,
        };
        start(f, &mut commands, &mut next, launch);
        match f.campaign.pending_battle.take() {
            Some(p) => commands.insert_resource(p),
            None => commands.remove_resource::<crate::campaign::battle::PendingBattle>(),
        }
    }
    // KKC_Session::Launch 0x77A950 -> 0x775EE0: the launch task of a session
    if out.lan_load {
        if let Some(seed) = f.lan.launch_seed() {
            f.custom.net_seed(u64::from(seed));
        }
        f.custom.net_fill_implicit();
        out.launch = true;
    }
    if out.launch && f.custom.film().is_some() {
        f.launch_film();
    } else if out.launch {
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
                difficulty: f.custom.rule_value("DifficultyLevel"),
                sun: f.custom.rule_value("Sun"),
            };
            f.last_launch = Some(launch.clone());
            f.clear_message_log();
            start(f, &mut commands, &mut next, launch);
            // (a session's battle: `Battle::local` follows its player; scene.rs)
            if f.lan.active() {
                let (_, local) = f.lan.kingdoms(&f.custom);
                commands.insert_resource(crate::session::PendingLocalKingdom(local));
            }
        }
    }
    // the score screen's Replay: a film starts again (0x6590B0, +0x54 == 4:
    // 0x652B10); else the same launch again (INFERRED: the session's launch
    // settings 0x76B4C0+0x4C are the staging's, resolved at the launch, so
    // the same setup and map seed)
    if out.replay && f.custom.film().is_some() {
        f.launch_film();
    } else if out.replay {
        if let Some(launch) = f.last_launch.clone() {
            // the score screen is state 1 too (`KKC_Session::SetNewState(1)`)
            f.clear_message_log();
            start(f, &mut commands, &mut next, launch);
        }
    }
    // the film being opened behind the loading screen: the map's scene next,
    // or the failure on the page it came from
    let opened = f.film_load.as_ref().and_then(|slot| slot.lock().ok().and_then(|mut s| s.take()));
    if let Some(result) = opened {
        f.film_load = None;
        match result {
            Ok((world, player)) => match film_launch(&world, &player) {
                Some(launch) => {
                    commands.insert_resource(crate::session::PendingFilm(std::sync::Mutex::new(Some((world, player)))));
                    start(f, &mut commands, &mut next, launch);
                }
                None => f.film_failed(&FilmFailure::CreateWorld { detail: "the film's battle has no setup".into() }),
            },
            Err(failure) => f.film_failed(&failure),
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
        let campaign_iface = (f.page == Page::Campaign).then_some(&f.campaign.iface);
        let dynamic = |p: &str| hud.and_then(|h| h.tooltip(p)).or_else(|| campaign_iface.and_then(|i| i.tooltip(p)));
        f.tooltips.anchor = if f.page == Page::Campaign { f.campaign.tooltip_anchor(&mut f.ui) } else { None };
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
