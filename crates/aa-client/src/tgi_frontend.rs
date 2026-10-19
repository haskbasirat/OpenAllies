//! Original front-end screens rendered from the TGI port (`aa-ui`).
//!
//! `aa-ui` builds the native widget trees (main menu, single player, options
//! dialog, credits, loading screen ...) from Data/UI/*.tgi, lays them out
//! with the original arithmetic at the window's pixel size and emits a
//! renderer-neutral draw list. This module paints that list with egui and
//! feeds pointer and keyboard input back (focus on hover, press, release
//! fires the native command string).
//!
//! Draw-list conventions (aa-ui `draw.rs`, `sprite.rs`): coordinates are
//! window pixels with pixel centres at i + 0.5; textures are sampled with
//! POINT filtering and clamp addressing (original samplers 0x742200);
//! non-blended quads discard alpha-0 texels and draw the rest opaque; the
//! list is already in the original's paint order.
//!
//! Keys: Escape/Enter go to the focused edit box or open drop-down first,
//! then to the UI hotkeys (Data/Localization/Hotkeys/hotkeys_ui.txt).

use crate::menu_video::MenuVideo;
use crate::ui_tex::{UiTextures, Variant};
use aa_ui::draw::{Blend, DrawKind, DrawList};
use aa_ui::fs::DataRoot;
use aa_ui::layout::Screen;
use aa_ui::screens;
use aa_ui::ui::{MouseButton, Ui, UiEvent};
use aa_ui::ui_controls::UiKey;
use eframe::egui;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

/// Front-end pages built from TGI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    MainMenu,
    SinglePlayer,
    /// LoadingScreen interface (0x6E87C0) while a map loads.
    Loading,
    /// CreditsInterface (0x646150, `RequestCredits` 0x64F290).
    Credits,
}

/// What the shell should do after a frame.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// `Quit` (the Ok button of the quit confirmation).
    Quit,
    /// `RequestEditor` (the shell shows the asset viewer for now).
    Editor,
    ///Original RequestCustom: configure a real local skirmish attempt.
    Skirmish,
    /// Options dialog OK: persist the user variables that differ from the
    /// defaults.
    SaveOptions(BTreeMap<String, String>),
    /// Live channel volume from the options dialog (0 = 2D, 1 = 3D,
    /// 2 = music, 3 = speech, 4 = main).
    Volume { channel: u8, value: f32 },
    /// A UI sound by audio IDS.
    Sound(String),
    /// LAN nickname set in the options dialog.
    Nickname(String),
    ///Actual source-bound loading ReadyButton command6E80F0.
    UserReady,
}

pub struct TgiFrontend {
    pub ui: Ui,
    native_menus:crate::battle_menus::State,
    page: Page,
    built_for: Option<(i32, i32, Page)>,
    /// Message box on screen (path, request); re-shown after a rebuild.
    dialog: Option<(String, aa_ui::message_box::MessageBoxRequest)>,
    fonts_installed: bool,
    root: std::path::PathBuf,
    /// Loading screen texts: map name, status string key.
    loading: (String, String),
    /// Commands whose screens would start a match or are not ported yet:
    /// their buttons are disabled (native `SetEnabled(false)`, 0x5EFB30,
    /// which also selects the DISABLED art and the grey label override).
    pub disabled: Vec<&'static str>,
    /// The options dialog (created once, `0x5C5E97`).
    options: Option<aa_ui::options::Options>,
    /// User variables saved by an earlier OK.
    saved_vars: BTreeMap<String, String>,
    /// LAN nickname shown on the network page.
    pub nickname: String,
    credits: Option<aa_ui::credits::Credits>,
    credits_end_posted: bool,
    clock: Instant,
    /// Last left press (ms, x, y) for the double-click test.
    last_press: Option<(u64, f32, f32)>,
}

/// Windows default double-click time and box (GetDoubleClickTime 500 ms,
/// SM_CXDOUBLECLK 4 px): the original leaves the test to Windows.
const DOUBLE_CLICK_MS: u64 = 500;

impl TgiFrontend {
    pub fn native_parts(&mut self)->(&mut Ui,&mut crate::battle_menus::State){(&mut self.ui,&mut self.native_menus)}
    /// `root` is the reconstruction folder (assets/original/Data, the v14
    /// loose overrides and Skins live under it).
    pub fn new(root: &Path) -> Self {
        let data = DataRoot::with_overlay(
            root.join("assets/original/Data"),
            root.join("native-game/runtime-expanded-v14/Data"),
        );
        let ui = Ui::new(data, Screen { width: 1024, height: 768 });
        Self {
            ui,
            native_menus:crate::battle_menus::State::new(),
            page: Page::MainMenu,
            built_for: None,
            dialog: None,
            fonts_installed: false,
            root: root.to_path_buf(),
            loading: (String::new(), "app_status_loading_data".into()),
            disabled: vec![
                "RequestMultiplayer",
                "RequestFilm",
                "CheckForUpdates",
                "RequestCampaign",
                "ConfirmPlayDynamicCampaign",
            ],
            options: None,
            saved_vars: BTreeMap::new(),
            nickname: String::new(),
            credits: None,
            credits_end_posted: false,
            clock: Instant::now(),
            last_press: None,
        }
    }

    /// User variables saved by an earlier OK (the shell's settings file).
    pub fn set_saved_vars(&mut self, vars: BTreeMap<String, String>) {
        self.saved_vars = vars;
        self.ui.uvars.apply_overrides(&self.saved_vars);
    }

    /// Current value of a user variable (defaults + saved values).
    pub fn var_f32(&mut self, name: &str) -> f32 {
        self.ensure_options();
        self.ui.uvars.get_f32(name)
    }

    fn ensure_options(&mut self) {
        if self.options.is_none() {
            let o = aa_ui::options::Options::new(&mut self.ui);
            self.options = Some(o);
        }
    }

    fn now_ms(&self) -> u64 {
        self.clock.elapsed().as_millis() as u64
    }

    /// Show the loading screen for `map` with a status string key
    /// (`app_status_*`, e.g. app_status_loading_resources).
    pub fn loading(&mut self, map: &str, status_key: &str) {
        self.loading = (map.to_string(), status_key.to_string());
        self.show(Page::Loading);
    }

    /// The quit confirmation (`QuitConfirm`, 0x413230) in the shared
    /// message box.
    pub fn quit_confirm(&mut self) {
        screens::quit_confirm(&mut self.ui, false);
        self.remember_dialog();
    }

    /// A one-button message (`button_ok`) in the shared message box.
    pub fn message(&mut self, text: &str) {
        let ok = self.ui.strings.resolve("#button_ok");
        self.ui.show_message_box(
            aa_ui::message_box::SHARED,
            aa_ui::message_box::MessageBoxRequest { text: text.to_string(), buttons: vec![(ok, String::new())] },
        );
        self.remember_dialog();
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

    /// Install the faces the original UI uses: A&A (GDI+ private font) and
    /// the GDI system faces named by the font TGIs (Arial).
    fn install_fonts(&mut self, ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        let defaults = fonts.families.get(&egui::FontFamily::Proportional).cloned().unwrap_or_default();
        let mut add = |name: &str, path: std::path::PathBuf| -> bool {
            match std::fs::read(&path) {
                Ok(bytes) => {
                    fonts.font_data.insert(name.into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
                    true
                }
                Err(_) => false,
            }
        };
        let aa = add("aa", self.root.join("assets/original/Data/Fonts/TrueType/A&A.ttf"));
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        let fonts_dir = std::path::Path::new(&windir).join("Fonts");
        let arial = add("arial", fonts_dir.join("arial.ttf"));
        let arialbd = add("arialbd", fonts_dir.join("arialbd.ttf"));
        let family = |first: &[(&str, bool)]| {
            let mut v: Vec<String> = first.iter().filter(|(_, ok)| *ok).map(|(n, _)| n.to_string()).collect();
            v.extend(defaults.iter().cloned());
            v
        };
        fonts.families.insert(egui::FontFamily::Proportional, family(&[("aa", aa)]));
        fonts.families.insert(egui::FontFamily::Name("A&A".into()), family(&[("aa", aa)]));
        fonts.families.insert(egui::FontFamily::Name("Arial".into()), family(&[("arial", arial), ("aa", aa)]));
        fonts.families.insert(egui::FontFamily::Name("Arial Bold".into()), family(&[("arialbd", arialbd), ("arial", arial), ("aa", aa)]));
        ctx.set_fonts(fonts);
        self.fonts_installed = true;
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
        }
        self.page = page;
        self.built_for = None;
    }

    /// The options dialog (`OpenOptionsMenu` 0x650700 -> 0x6792A0("")).
    pub fn open_options(&mut self, size: (i32, i32)) {
        self.ensure_options();
        let nick = self.nickname.clone();
        let default_connection = self.ui.avars.get("DefaultNetworkConnectionType").unwrap_or("").to_string();
        if let Some(o) = &mut self.options {
            // the renderer's only display mode is the window it draws in
            o.video_modes = vec![(size.0, size.1, 32)];
            o.current_mode = size;
            o.nickname = nick;
            if o.connection.is_empty() {
                o.connection = default_connection;
            }
            o.show(&mut self.ui, "");
        }
    }

    fn build(&mut self, size: (i32, i32)) {
        self.ui.set_screen(Screen { width: size.0, height: size.1 });
        if self.native_menus.is_bound(){
            self.ui.layout();self.built_for=Some((size.0,size.1,self.page));return;
        }
        if self.page == Page::Credits && self.credits.is_some() {
            // the credits keep running: only the layout follows the window
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
            Page::Loading => {
                screens::loading_screen(&mut self.ui, false);
                let status = self.ui.strings.resolve(&format!("#{}", self.loading.1));
                let map = self.loading.0.clone();
                // loading screen refresh 0x6E83A0: no world or local player
                // exists while the asset viewer loads, so the interface's
                // default art is used
                let info = screens::LoadingInfo { map_name: map, status, kind: 1, ..Default::default() };
                screens::loading_refresh(&mut self.ui, &info);
            }
            Page::Credits => {
                let now = self.now_ms();
                self.credits = Some(aa_ui::credits::Credits::show(&mut self.ui, now));
                self.credits_end_posted = false;
            }
        }
        let disabled = self.disabled.clone();
        fn apply(w: &mut aa_ui::widget::Widget, disabled: &[&str]) {
            if w.command.as_deref().is_some_and(|c| disabled.contains(&c)) {
                w.set_enabled(false);
            }
            for c in &mut w.children {
                apply(c, disabled);
            }
        }
        apply(&mut self.ui.root, &disabled);
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

    /// Keyboard: the focused edit box / open drop-down, then the hotkeys.
    fn keys(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());
        for e in events {
            match e {
                egui::Event::Text(t) => self.ui.text_input(&t),
                // clipboard (hotkeys_ui.txt ctrl-c / ctrl-x / ctrl-v)
                egui::Event::Copy => {
                    self.ui.editbox_command("EditboxCopy");
                }
                egui::Event::Cut => {
                    self.ui.editbox_command("EditboxCut");
                }
                egui::Event::Paste(t) => self.ui.paste(&t),
                egui::Event::Key { key: egui::Key::A, pressed: true, modifiers, .. } if modifiers.command => {
                    self.ui.hotkey("ctrl-a");
                }
                egui::Event::Key { key: egui::Key::Tab, pressed: true, modifiers, .. } => {
                    self.ui.hotkey(if modifiers.shift { "shift-tab" } else { "tab" });
                }
                egui::Event::Key { key, pressed: true, .. } => {
                    let k = match key {
                        egui::Key::Enter => Some((UiKey::Enter, "enter")),
                        egui::Key::Escape => Some((UiKey::Escape, "esc")),
                        egui::Key::Backspace => Some((UiKey::Backspace, "")),
                        egui::Key::Delete => Some((UiKey::Delete, "")),
                        egui::Key::ArrowLeft => Some((UiKey::Left, "")),
                        egui::Key::ArrowRight => Some((UiKey::Right, "")),
                        egui::Key::Home => Some((UiKey::Home, "")),
                        egui::Key::End => Some((UiKey::End, "")),
                        egui::Key::Insert => Some((UiKey::Insert, "")),
                        _ => None,
                    };
                    if let Some((k, name)) = k {
                        if !self.ui.key_down(k) && !name.is_empty() {
                            self.ui.hotkey(name);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Run one frame: input, layout, paint. Returns shell actions.
    pub fn frame(
        &mut self,
        ctx: &egui::Context,
        tex: &mut UiTextures,
        video: &mut MenuVideo,
        globe: Option<&mut MenuVideo>,
        input: bool,
    ) -> Vec<Action> {
        self.frame_with_clear(ctx, tex, video, globe, input, true)
    }

    ///The actual VWorld renderer already painted the background. Preserve the
    ///same source-bound forest, input/update/callback and draw-list order; only
    ///the legacy Page::Loading black clear is omitted. Native Loading still
    ///paints until its real AppState transition selects the Game interface.
    pub fn frame_over_battle(
        &mut self,
        ctx: &egui::Context,
        tex: &mut UiTextures,
        video: &mut MenuVideo,
        globe: Option<&mut MenuVideo>,
        input: bool,
    ) -> Vec<Action> {
        self.frame_with_clear(ctx, tex, video, globe, input, false)
    }

    fn frame_with_clear(
        &mut self,
        ctx: &egui::Context,
        tex: &mut UiTextures,
        video: &mut MenuVideo,
        globe: Option<&mut MenuVideo>,
        input: bool,
        clear_background: bool,
    ) -> Vec<Action> {
        if !self.fonts_installed {
            self.install_fonts(ctx);
        }
        let ppp = ctx.pixels_per_point();
        let screen = ctx.content_rect();
        let size = ((screen.width() * ppp).round() as i32, (screen.height() * ppp).round() as i32);
        if self.built_for != Some((size.0, size.1, self.page)) {
            self.build(size);
        }
        // pointer input in window pixels
        let (pos, pressed, released, rpressed, rreleased, wheel) = ctx.input(|i| {
            (
                i.pointer.latest_pos(),
                i.pointer.primary_pressed(),
                i.pointer.primary_released(),
                i.pointer.secondary_pressed(),
                i.pointer.secondary_released(),
                i.raw_scroll_delta.y,
            )
        });
        if let Some(p) = pos.filter(|_| input) {
            let (x, y) = ((p.x - screen.min.x) * ppp, (p.y - screen.min.y) * ppp);
            self.ui.mouse_move(x, y);
            if pressed {
                // Windows turns a second press within the double-click time
                // and box into WM_LBUTTONDBLCLK (CS_DBLCLKS, 0x6FB2DD) ->
                // event 0x108; AA.exe has no threshold of its own
                let now = self.now_ms();
                let double = self.last_press.is_some_and(|(t, px, py)| {
                    now.saturating_sub(t) <= DOUBLE_CLICK_MS && (x - px).abs() <= 2.0 && (y - py).abs() <= 2.0
                });
                if double {
                    self.ui.mouse_double(x, y, MouseButton::Left);
                    self.last_press = None;
                } else {
                    self.ui.mouse_down(x, y, MouseButton::Left);
                    self.last_press = Some((now, x, y));
                }
            }
            if released {
                self.ui.mouse_up(x, y, MouseButton::Left);
            }
            if rpressed {
                self.ui.mouse_down(x, y, MouseButton::Right);
            }
            if rreleased {
                self.ui.mouse_up(x, y, MouseButton::Right);
            }
            if wheel != 0.0 {
                self.ui.mouse_wheel(x, y, wheel);
            }
        }
        if input {
            self.keys(ctx);
        }
        let now = self.now_ms();
        self.ui.update(now);
        if let Some(c) = &mut self.credits {
            // Update 0x646C10 posts CreditsEndCredits once nothing is left
            if c.update(&mut self.ui, now) && !self.credits_end_posted {
                self.credits_end_posted = true;
                self.ui.events.push(UiEvent::Command {
                    widget: aa_ui::credits::PATH.into(),
                    command: aa_ui::credits::END_COMMAND.into(),
                });
            }
        }
        let mut actions = Vec::new();
        if self.dialog.is_some() && self.ui.message_box.is_none() {
            // a message box button was pressed (the box closed itself)
            self.dialog = None;
        }
        for e in self.ui.take_events() {
            match e {
                UiEvent::Command { command, .. } => self.command(&command, size, &mut actions),
                UiEvent::Value { widget, value } => {
                    let effects = self.options.as_mut().and_then(|o| o.value(&widget, value)).unwrap_or_default();
                    self.effects(effects, &mut actions);
                }
                UiEvent::Sound { name } => actions.push(Action::Sound(name)),
                UiEvent::Clipboard { text } => ctx.copy_text(text),
                // owned drop-down functors, slot selection and minimap jumps
                // act on a session / world this shell does not run yet
                UiEvent::Select { .. } | UiEvent::Selected { .. } | UiEvent::Minimap { .. } => {}
            }
        }
        if self.built_for != Some((size.0, size.1, self.page)) {
            self.build(size);
        }
        self.ui.layout();
        let list = self.ui.draw_list();
        let painter = ctx.layer_painter(egui::LayerId::background());
        if clear_background && matches!(self.page, Page::Credits | Page::Loading) {
            // the frame clears to black (uncovered areas of 03-launch.png);
            // the menus are covered by the menu background movie instead
            painter.rect_filled(screen, 0.0, egui::Color32::BLACK);
        }
        paint(ctx, &painter, screen.min, ppp, &list, tex, video, globe);
        if pos.is_some() || self.page == Page::Credits {
            ctx.request_repaint();
        }
        actions
    }

    fn command(&mut self, command: &str, size: (i32, i32), actions: &mut Vec<Action>) {
        // the options dialog's own commands first
        if let Some(o) = &mut self.options {
            if let Some(effects) = o.command(&mut self.ui, command) {
                if self.ui.message_box.is_some() {
                    self.remember_dialog();
                }
                self.effects(effects, actions);
                return;
            }
        }
        match command {
            "" => {}
            "Quit" => actions.push(Action::Quit),
            // 0x64F250: hide the main menu interface, show SinglePlayerMenu
            "RequestSinglePlayer" => self.show(Page::SinglePlayer),
            // 0x65B1F0: hide SinglePlayerMenu, re-show the main menu
            "RequestMainMenu" => self.show(Page::MainMenu),
            "OpenOptionsMenu" => self.open_options(size),
            "QuitConfirm" => self.quit_confirm(),
            "RequestEditor" => actions.push(Action::Editor),
            "RequestCustom" => actions.push(Action::Skirmish),
            // 0x64F290: menuTable[10]->Activate(1)
            "RequestCredits" => self.show(Page::Credits),
            "UserReady" if self.native_menus.is_bound()=>actions.push(Action::UserReady),
            // 0x645800: menuTable[0] (MainMenuInterface)->Activate(1)
            "CreditsEndCredits" => self.show(Page::MainMenu),
            other => log::info!("UI command not handled yet: {other}"),
        }
    }

    fn effects(&mut self, effects: Vec<aa_ui::options::Effect>, actions: &mut Vec<Action>) {
        use aa_ui::options::Effect;
        for e in effects {
            match e {
                Effect::Save => {
                    if self.options.is_some() {
                        self.saved_vars = self.ui.uvars.overrides();
                        actions.push(Action::SaveOptions(self.saved_vars.clone()));
                    }
                }
                Effect::Volume { channel, value } => actions.push(Action::Volume { channel, value }),
                Effect::PlaySound(s) => actions.push(Action::Sound(s)),
                Effect::Nickname(n) => {
                    self.nickname = n.clone();
                    actions.push(Action::Nickname(n));
                }
                Effect::Closed => {}
                // no camera, game speed or hotkey configuration outside a
                // running world yet
                Effect::CameraZoom(_) | Effect::GameSpeed(_) | Effect::HotkeyConfiguration(_) => {}
            }
        }
    }
}

fn color32(c: [f32; 4]) -> egui::Color32 {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    egui::Color32::from_rgba_unmultiplied(b(c[0]), b(c[1]), b(c[2]), b(c[3]))
}

/// The egui font standing in for a UI font face (A&A.ttf through GDI+,
/// Arial through GDI).
fn egui_font(ctx: &egui::Context, face: &aa_ui::text::FontFace, ppp: f32) -> egui::FontId {
    let family = if face.gdiplus {
        egui::FontFamily::Name("A&A".into())
    } else if face.bold {
        egui::FontFamily::Name(format!("{} Bold", face.face).into())
    } else {
        egui::FontFamily::Name(face.face.clone().into())
    };
    let known = ctx.fonts(|f| f.definitions().families.contains_key(&family));
    egui::FontId::new(face.em_px / ppp, if known { family } else { egui::FontFamily::Proportional })
}

/// Paint a draw list (window pixels) at `origin` (points).
#[allow(clippy::too_many_arguments)]
pub fn paint(
    ctx: &egui::Context,
    painter: &egui::Painter,
    origin: egui::Pos2,
    ppp: f32,
    list: &DrawList,
    tex: &mut UiTextures,
    video: &mut MenuVideo,
    mut globe: Option<&mut MenuVideo>,
) {
    let to_pt = |x: f32, y: f32| origin + egui::vec2(x / ppp, y / ppp);
    let skin = list.skin.as_deref();
    for cmd in &list.commands {
        match &cmd.kind {
            DrawKind::Quad(q) => {
                let variant = if q.blend == Blend::Alpha { Variant::Nearest } else { Variant::NearestAlphaTest };
                let texture = q.texture.as_deref().and_then(|t| tex.get_skinned(ctx, t, skin, variant));
                let mut mesh = match &texture {
                    Some(t) => egui::Mesh::with_texture(t.id()),
                    None => egui::Mesh::default(),
                };
                let r = egui::Rect::from_min_max(to_pt(q.rect.l, q.rect.t), to_pt(q.rect.r, q.rect.b));
                // draw-list rects use pixel centres at i + 0.5; egui samples
                // at pixel centres too, so the rect is used as is
                let uv = egui::Rect::from_min_max(egui::pos2(q.uv[0], q.uv[1]), egui::pos2(q.uv[2], q.uv[3]));
                let c = color32(q.color);
                if texture.is_some() {
                    mesh.add_rect_with_uv(r, uv, c);
                } else if q.texture.is_none() {
                    mesh.add_colored_rect(r, c);
                } else {
                    continue;
                }
                match q.clip {
                    Some(cl) => {
                        painter
                            .with_clip_rect(egui::Rect::from_min_max(to_pt(cl.l, cl.t), to_pt(cl.r, cl.b)))
                            .add(egui::Shape::mesh(mesh));
                    }
                    None => {
                        painter.add(egui::Shape::mesh(mesh));
                    }
                }
            }
            DrawKind::Text(t) => {
                // glyph quads of the native rebuild (aa_ui::rdraw): the
                // cell's top-left is rect - cell; each quad shows only its
                // clipped part of the cell
                for g in t.glyphs.iter().filter(|g| g.highlight) {
                    // <h_on> background list (solid fill: INFERRED)
                    let r = egui::Rect::from_min_max(to_pt(g.rect.l, g.rect.t), to_pt(g.rect.r, g.rect.b));
                    painter.rect_filled(r, 0.0, color32(g.color));
                }
                for g in t.glyphs.iter().filter(|g| !g.highlight) {
                    let r = egui::Rect::from_min_max(to_pt(g.rect.l, g.rect.t), to_pt(g.rect.r, g.rect.b));
                    if let Some(img) = &g.image {
                        let Some(texture) = tex.get_skinned(ctx, img, skin, Variant::Nearest) else { continue };
                        let [iw, ih] = texture.size().map(|v| v.max(1) as f32);
                        let uv = egui::Rect::from_min_max(
                            egui::pos2(g.cell[0] / iw, g.cell[1] / ih),
                            egui::pos2((g.cell[0] + g.rect.r - g.rect.l) / iw, (g.cell[1] + g.rect.b - g.rect.t) / ih),
                        );
                        let mut mesh = egui::Mesh::with_texture(texture.id());
                        mesh.add_rect_with_uv(r, uv, color32(g.color));
                        painter.add(egui::Shape::mesh(mesh));
                        continue;
                    }
                    let Some(face) = t.faces.get(g.face).and_then(|f| f.face.as_ref()) else { continue };
                    let Some(ch) = char::from_u32(g.c).filter(|c| !c.is_whitespace()) else { continue };
                    let font = egui_font(ctx, face, ppp);
                    let clip = painter.with_clip_rect(r.intersect(painter.clip_rect()));
                    let (cx, cy) = (g.rect.l - g.cell[0], g.rect.t - g.cell[1]);
                    let s = ch.to_string();
                    if face.shadow {
                        // the atlas cell bakes a one-pixel drop shadow at
                        // (+1,+1) with shadow_opacity, modulated by the
                        // vertex colour's alpha
                        let a = (face.shadow_opacity * g.color[3] * 255.0) as u8;
                        clip.text(to_pt(cx + 1.0, cy + 1.0), egui::Align2::LEFT_TOP, &s, font.clone(), egui::Color32::from_black_alpha(a));
                    }
                    clip.text(to_pt(cx, cy), egui::Align2::LEFT_TOP, &s, font, color32(g.color));
                }
            }
            DrawKind::Video(v) => {
                let r = egui::Rect::from_min_max(to_pt(v.rect.l, v.rect.t), to_pt(v.rect.r, v.rect.b));
                let globe_movie = v.resources.iter().any(|s| s.to_ascii_lowercase().contains("aaglobe"));
                if globe_movie {
                    if let Some(g) = globe.as_deref_mut() {
                        g.paint(ctx, painter, r);
                    }
                } else {
                    // without baked frames the shell's still backdrop stays
                    video.paint(ctx, painter, r);
                }
            }
            DrawKind::External { .. } => {}
        }
    }
}

#[cfg(test)]
mod frame_growth_tests {
    use super::*;
    use eframe::egui;
    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap().join("reconstruction")
    }
    struct Frame {
        indices: usize,
        vertices: usize,
        commands: usize,
        ms: f64,
    }
    fn run_frames(front: &mut TgiFrontend, over_battle: bool, frames: usize) -> Vec<Frame> {
        let ctx = egui::Context::default();
        let mut tex = UiTextures::new(&root());
        let mut video = MenuVideo::new(None);
        let mut out = Vec::new();
        for i in 0..frames {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 900.0))),
                time: Some(i as f64 / 60.0),
                ..Default::default()
            };
            let start = std::time::Instant::now();
            let output = ctx.run(input, |ctx| {
                if over_battle {
                    front.frame_over_battle(ctx, &mut tex, &mut video, None, true);
                } else {
                    front.frame(ctx, &mut tex, &mut video, None, true);
                }
            });
            let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            let (mut indices, mut vertices) = (0, 0);
            for p in &primitives {
                if let egui::epaint::Primitive::Mesh(m) = &p.primitive {
                    indices += m.indices.len();
                    vertices += m.vertices.len();
                }
            }
            let commands = front.ui.draw_list().commands.len();
            out.push(Frame { indices, vertices, commands, ms });
        }
        out
    }
    fn report(label: &str, frames: &[Frame]) {
        let first = &frames[1];
        let last = frames.last().unwrap();
        let mean = frames[1..].iter().map(|f| f.ms).sum::<f64>() / (frames.len() - 1) as f64;
        eprintln!("{label}: frame1 indices={} vertices={} commands={}; last indices={} vertices={} commands={}; mean {mean:.2} ms",
            first.indices, first.vertices, first.commands, last.indices, last.vertices, last.commands);
        for f in &frames[1..] {
            assert_eq!((f.indices, f.vertices, f.commands), (first.indices, first.vertices, first.commands), "{label} per-frame geometry grew");
        }
    }
    ///The two Loading pages a long load shows: the pre-bind TGI page and the
    ///native-menus LoadingScreen forest over the battle frame.
    #[test]
    #[ignore = "requires installed original/v14 UI data"]
    fn long_loading_frames_draw_a_constant_tree() {
        let mut front = TgiFrontend::new(&root());
        front.loading("(2v2)Berlin", "app_status_loading_resources");
        let frames = run_frames(&mut front, false, 400);
        report("pre-bind Loading page", &frames);
        let mut front = TgiFrontend::new(&root());
        let (ui, menus) = front.native_parts();
        let selectors = menus.bind(ui, &["money".to_string(), "ammo".to_string()]).unwrap();
        menus.show(ui, &selectors.loading).unwrap();
        let frames = run_frames(&mut front, true, 400);
        report("native LoadingScreen over battle", &frames);
        {
            let ui = &mut front.ui;
            let time = |label: &str, f: &mut dyn FnMut()| {
                let start = std::time::Instant::now();
                for _ in 0..400 {
                    f();
                }
                eprintln!("  {label}: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0 / 400.0);
            };
            time("ui.update", &mut || ui.update(1000));
            time("ui.layout", &mut || ui.layout());
            time("ui.draw_list", &mut || {
                ui.draw_list();
            });
            //The cached draw list equals one built with every cache cleared.
            let cached = serde_json::to_value(ui.draw_list()).unwrap();
            let screen = ui.screen;
            ui.set_screen(aa_ui::layout::Screen { width: screen.width + 1, height: screen.height });
            ui.set_screen(screen);
            ui.layout();
            let fresh = serde_json::to_value(ui.draw_list()).unwrap();
            assert_eq!(cached, fresh, "cached draw list differs from a fresh rebuild");
        }
        let (ui, menus) = front.native_parts();
        menus.show(ui, &selectors.game).unwrap();
        let frames = run_frames(&mut front, true, 200);
        report("native Game interface over battle", &frames);
    }
}
