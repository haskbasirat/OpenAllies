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
use super::custom::Custom;
use super::hud::GameHud;
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
}

/// Commands the front end does not run yet (their screens are not built).
const NOT_BUILT: &[&str] = &[
    "RequestMultiplayer",
    "RequestEditor",
    "RequestFilm",
    "CheckForUpdates",
    "RequestCampaign",
    "ConfirmPlayDynamicCampaign",
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
        let custom = Custom::new(root);
        Self {
            ui,
            page: Page::MainMenu,
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
        }
        self.page = page;
        self.built_for = None;
    }

    fn ensure_options(&mut self) {
        if self.options.is_none() {
            self.options = Some(aa_ui::options::Options::new(&mut self.ui));
        }
    }

    /// The options dialog (`OpenOptionsMenu` 0x650700 -> 0x6792A0("")).
    fn open_options(&mut self, size: (i32, i32)) {
        self.ensure_options();
        let default_connection = self.ui.avars.get("DefaultNetworkConnectionType").unwrap_or("").to_string();
        if let Some(o) = &mut self.options {
            // the window is the renderer's only display mode
            o.video_modes = vec![(size.0, size.1, 32)];
            o.current_mode = size;
            if o.connection.is_empty() {
                o.connection = default_connection;
            }
            o.show(&mut self.ui, "");
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

    fn build(&mut self, size: (i32, i32), battle: Option<&Battle>, progress: Option<&LoadProgress>) {
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
            // a custom game: the objectives list is shown (kind 0)
            kind: 0,
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
        match command {
            "" => {}
            "Quit" => self.quit = true,
            "RequestSinglePlayer" => self.show(Page::SinglePlayer),
            "RequestMainMenu" => self.show(Page::MainMenu),
            // main menu Options (0x650700) / game menu Options
            "OpenOptionsMenu" | "OptionsDialog" => self.open_options(size),
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
            other if NOT_BUILT.contains(&other) => info!("UI command {other}: its screen is not built yet"),
            other => info!("UI command not handled yet: {other}"),
        }
    }

    fn effects(&mut self, effects: Vec<aa_ui::options::Effect>) {
        use aa_ui::options::Effect;
        for e in effects {
            match e {
                Effect::Save => self.save_settings(),
                Effect::CameraZoom(z) => self.pending.push(super::hud::GameAction::CameraZoom(z)),
                Effect::GameSpeed(s) => self.pending.push(super::hud::GameAction::SetSpeed(2f32.powf(s))),
                // no sound output in this client yet
                Effect::Volume { .. } | Effect::PlaySound(_) | Effect::Nickname(_) | Effect::Closed | Effect::HotkeyConfiguration(_) => {}
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

/// What a frame of the front end asks the app to do.
#[derive(Default)]
pub struct FrameOut {
    pub launch: bool,
    pub ready: bool,
    pub game: Vec<super::hud::GameAction>,
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
    let f = front.as_mut();
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
            Page::Loading | Page::Game => None,
            _ => Some(Page::Game),
        },
    };
    if let Some(p) = want {
        if p == Page::Loading {
            f.loading_finished = false;
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
            UiEvent::Sound { .. } | UiEvent::Clipboard { .. } => {}
        }
    }
    if f.page == Page::Staging {
        f.custom.update(&mut f.ui);
    }
    // the HUD reads the battle every frame
    if f.page == Page::Game {
        let hovered = input.pos.and_then(|p| f.ui.pick(p.x, p.y));
        let starting_experience = f.starting_experience;
        if let (Some(hud), Some(b)) = (f.hud.as_mut(), battle.as_deref()) {
            hud.starting_experience = starting_experience;
            hud.set_hover(hovered.as_deref());
            hud.update(&mut f.ui, b);
        }
    }
    if out.launch {
        if let Some((mut setup, random)) = f.custom.launch_setup() {
            f.custom.localize_setup(&f.ui, &mut setup);
            info!("launch: {setup:?}, generals {:?}", f.custom.launched_profiles);
            commands.insert_resource(crate::session::PendingAiProfiles(f.custom.launched_profiles.clone()));
            f.starting_experience = f.custom.rule_value("StartingExperience").and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(0.0);
            // the FogOfWar world rule of the session (EnumValue IDS)
            match f.custom.rule_value("FogOfWar").and_then(|v| aa_play::fog::FogMode::from_ids(&v)) {
                Some(mode) => commands.insert_resource(super::fog::PendingFog(mode)),
                None => commands.remove_resource::<super::fog::PendingFog>(),
            }
            commands.insert_resource(crate::session::PendingSetup(setup));
            if let Some(params) = random {
                commands.insert_resource(crate::session::PendingRandomMap(params));
            } else {
                commands.remove_resource::<crate::session::PendingRandomMap>();
            }
            commands.insert_resource(LoadProgress::default());
            next.set(AppState::Loading);
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
    if !out.game.is_empty() || !game_keys.is_empty() {
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
                    }),
                },
            );
        }
    }
    if f.page == Page::Loading {
        // the frame clears to black (uncovered areas of 03-launch.png)
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
                }),
            },
        );
    }
    draw.list = list;
    if f.quit {
        quit.requested = true;
    }
}
