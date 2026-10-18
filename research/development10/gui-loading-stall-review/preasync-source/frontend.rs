//! Application shell: the original front end (main menu, single player,
//! options dialog, credits, loading screen; built from Data/UI/*.tgi by
//! aa-ui and painted by `tgi_frontend.rs`) and the asset viewer.
//!
//! Skirmish uses the actual local Host/worker/activation controller. A reached
//!native service failure remains a stopped attempt with its real owners.

use crate::audio::Audio;
use crate::gpu::SceneRenderer;
use crate::menu_ui;
use crate::ui_tex::UiTextures;
use aa_assets::AssetStore;
use anyhow::Result;
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

const GOLD: egui::Color32 = egui::Color32::from_rgb(222, 196, 120);
const TEXT: egui::Color32 = egui::Color32::from_rgb(232, 226, 205);

/// Per-user settings (%APPDATA%\AxisAlliesRust\settings.json). The original
/// keeps its options in User/UVars.tgi; this port never writes the original
/// profile and stores the changed user variables here instead.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// LAN nickname (options dialog, network page).
    pub player_name: String,
    /// Mirrors of the audio user variables (main, music, 2D, speech).
    pub master: f32,
    pub music: f32,
    pub effects: f32,
    pub voice: f32,
    pub ui_scale: f32,
    pub fullscreen: bool,
    /// User variables changed through the options dialog (name -> value).
    pub uvars: std::collections::BTreeMap<String, String>,
}
impl Default for Prefs {
    fn default() -> Self {
        Self {
            player_name: String::new(),
            master: 0.0,
            music: 0.0,
            effects: 0.0,
            voice: 0.0,
            ui_scale: 1.0,
            fullscreen: false,
            uvars: Default::default(),
        }
    }
}
impl Prefs {
    fn path() -> PathBuf {
        user_dir().join("settings.json")
    }
    pub fn load() -> Self {
        std::fs::read(Self::path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    pub fn save(&self) {
        let p = Self::path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(b) = serde_json::to_vec_pretty(self) {
            let _ = std::fs::write(p, b);
        }
    }
}

/// Settings and saves live here, never in the original game's Profile.
pub fn user_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("AxisAlliesRust")
}

/// Command-line UI testing: a scripted input file and a final screenshot.
#[derive(Clone, Default)]
pub struct AutoStart {
    pub shot: Option<PathBuf>,
    pub script: Option<PathBuf>,
}

enum Screen {
    MainMenu,
    /// Original LoadingScreen shown for a few frames before the viewer
    /// builds its map (the build blocks this thread).
    Loading(u32),
    Viewer,
    SkirmishSetup,
    SkirmishLoading(u32),
    SkirmishSession,
}

pub struct Shell {
    ///Actual application-time-seeded B39690+84 visual stream. The UI owns
    ///its separate+D4 stream; both persist across VWorld/mission replacement.
    _application_random:aa_ui::application_random::State,
    store: Arc<AssetStore>,
    render_state: egui_wgpu::RenderState,
    /// Created at start-up (GPU check); the asset viewer builds its own.
    _renderer: SceneRenderer,
    screen: Screen,
    ui: UiTextures,
    video: crate::menu_video::MenuVideo,
    /// Original front-end screens built from Data/UI/*.tgi (aa-ui).
    tgi: crate::tgi_frontend::TgiFrontend,
    /// MiniMenuBackground cinematic (AAGlobeBig) shown through the logo.
    globe: crate::menu_video::MenuVideo,
    audio: Audio,
    prefs: Prefs,
    viewer: Option<crate::App>,
    battle:crate::battle_controller::Controller,
    battle_ui_queue:crate::battle_ui_bridge::Queue,
    battle_ui_client:crate::battle_ui_bridge::Client,
    battle_load_settings:crate::loading::LoadSettings,
    battle_scene:crate::battle_scene_startup::UiScene,
    window_focus:crate::windows_focus::Focus,
    skirmish_map:Option<String>,
    skirmish_name:String,
    skirmish_player:String,
    skirmish_seed:u32,
    skirmish_observers:bool,
    memory_policy: aa_resources::MemoryPolicy,
    last_frame: Instant,
    error: Option<String>,
    pub auto: AutoStart,
    script: Option<crate::ui_script::Script>,
    pending_shot: Option<PathBuf>,
    last_input: Instant,
}

pub fn style(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = egui::Color32::from_rgb(34, 32, 27);
    v.window_fill = egui::Color32::from_rgb(40, 37, 30);
    v.window_stroke = egui::Stroke::new(1.0_f32, GOLD.gamma_multiply(0.6));
    v.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(70, 64, 48);
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(70, 64, 48);
    v.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(105, 94, 62);
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(105, 94, 62);
    v.widgets.active.weak_bg_fill = egui::Color32::from_rgb(140, 120, 70);
    v.selection.bg_fill = egui::Color32::from_rgb(120, 100, 50);
    v.override_text_color = Some(TEXT);
    ctx.set_visuals(v);
}


impl Shell {
    pub fn new(
        render_state: egui_wgpu::RenderState,
        ctx: &egui::Context,
        store: Arc<AssetStore>,
        memory_policy: aa_resources::MemoryPolicy,
    ) -> Result<Self> {
        style(ctx);
        let mut application_random=aa_ui::application_random::State::from_application_time32().map_err(anyhow::Error::msg)?;
        let root = store.root().to_owned();
        menu_ui::install_font(ctx, &root);
        let prefs = Prefs::load();
        ctx.set_zoom_factor(prefs.ui_scale);
        if prefs.fullscreen {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        }
        let renderer = SceneRenderer::new(render_state.clone())?;
        let battle_load_settings=crate::loading::LoadSettings::new(&store,
            render_state.device.limits().max_texture_dimension_2d,memory_policy)?;
        let mut tgi = crate::tgi_frontend::TgiFrontend::new(&root);
        application_random.seed_ui_5c8650(&mut tgi.ui).map_err(anyhow::Error::msg)?;
        tgi.set_saved_vars(prefs.uvars.clone());
        tgi.nickname = prefs.player_name.clone();
        // live channel volumes from the user variables (Data/UVars.tgi
        // defaults plus the saved changes)
        let mut prefs = prefs;
        prefs.master = tgi.var_f32("AudioMainVolume");
        prefs.music = tgi.var_f32("AudioMusicVolume");
        prefs.effects = tgi.var_f32("Audio2DVolume");
        prefs.voice = tgi.var_f32("AudioSpeechVolume");
        let mut audio = Audio::new(&root);
        audio.set_volumes(prefs.master, prefs.music, prefs.effects, prefs.voice);
        audio.play_menu_music();
        // Intro movie frames sit in assets-cache beside the material sidecar.
        let video_root = crate::model_fixes::ModelFixes::discover(&store)
            .ok()
            .and_then(|f| f.metadata_path().parent().map(|p| p.join("video")));
        let video_dir = video_root.as_ref().map(|p| p.join("MainMenu"));
        // research/phase8-ui/bake_globe.py
        let globe_dir = video_root.as_ref().map(|p| p.join("MainMenuGlobe"));
        let mut window_focus=crate::windows_focus::Focus::new();
        window_focus.observe();
        let repaint=ctx.clone();
        let(battle_ui_queue,battle_ui_client)=crate::battle_ui_bridge::Queue::with_wakeup(move||repaint.request_repaint());
        let battle=crate::battle_controller::Controller::new(&root,window_focus.reader(),battle_ui_client.clone());
        let skirmish_player=prefs.player_name.clone();
        Ok(Self {
            _application_random:application_random,
            ui: UiTextures::new(&root),
            video: crate::menu_video::MenuVideo::new(video_dir.as_deref()),
            tgi,
            globe: crate::menu_video::MenuVideo::new(globe_dir.as_deref()),
            store,
            render_state,
            _renderer: renderer,
            screen: Screen::MainMenu,
            audio,
            prefs,
            viewer: None,
            battle,battle_ui_queue,battle_ui_client,battle_load_settings,
            battle_scene:crate::battle_scene_startup::UiScene::empty(),window_focus,skirmish_map:None,
            skirmish_name:"Local skirmish".into(),skirmish_player,
            skirmish_seed:0,skirmish_observers:true,
            memory_policy,
            last_frame: Instant::now(),
            error: None,
            auto: AutoStart::default(),
            script: None,
            pending_shot: None,
            last_input: Instant::now(),
        })
    }

    /// The original backdrop: intro movies, or the default splash art.
    fn backdrop(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());
        if self.video.paint(ctx, &painter, screen) {
            return;
        }
        painter.rect_filled(screen, 0.0, egui::Color32::from_rgb(12, 12, 10));
        let l = self.ui.get(ctx, "/Nations/Default/SplashScreenLeft.tga");
        let r = self.ui.get(ctx, "/Nations/Default/SplashScreenRight.tga");
        if let (Some(l), Some(r)) = (l, r) {
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            let mid = screen.center().x;
            painter.image(l.id(), egui::Rect::from_min_max(screen.min, egui::pos2(mid, screen.bottom())), uv, egui::Color32::WHITE);
            painter.image(r.id(), egui::Rect::from_min_max(egui::pos2(mid, screen.top()), screen.max), uv, egui::Color32::WHITE);
        }
    }

    /// The original main menu and single player menu, built from
    /// Data/UI/Menus/*.tgi by aa-ui over the intro movies.
    fn main_menu(&mut self, ctx: &egui::Context) -> Option<Screen> {
        if !self.video.available() {
            self.backdrop(ctx);
        }
        let mut next = None;
        let globe = if self.globe.available() { Some(&mut self.globe) } else { None };
        for action in self.tgi.frame(ctx, &mut self.ui, &mut self.video, globe, true) {
            use crate::tgi_frontend::Action;
            match action {
                Action::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Action::Editor => {
                    self.tgi.loading("Campaign/tutorial01", "app_status_loading_resources");
                    next = Some(Screen::Loading(0));
                }
                Action::Skirmish => {
                    next=Some(if matches!(self.battle.status(),crate::battle_controller::Status::Activating|crate::battle_controller::Status::AwaitingNativeView) {Screen::SkirmishSession} else {Screen::SkirmishSetup});
                }
                Action::SaveOptions(vars) => {
                    self.prefs.uvars = vars;
                    self.prefs.save();
                }
                // live channels (0x7453A0): 4 main, 2 music, 0/1 2D/3D, 3 speech
                Action::Volume { channel, value } => {
                    match channel {
                        4 => self.prefs.master = value,
                        2 => self.prefs.music = value,
                        0 | 1 => self.prefs.effects = value,
                        3 => self.prefs.voice = value,
                        _ => {}
                    }
                    self.audio.set_volumes(self.prefs.master, self.prefs.music, self.prefs.effects, self.prefs.voice);
                }
                Action::Sound(ids) => {
                    if self.audio.has_id(&ids) {
                        self.audio.play(&ids, 1.0);
                    }
                }
                Action::Nickname(n) => self.prefs.player_name = n,
                Action::UserReady=>if let Err(error)=self.battle.user_ready_command(){self.error=Some(error)},
            }
        }
        next
    }

    fn skirmish_setup(&mut self,ctx:&egui::Context)->Option<Screen> {
        self.backdrop(ctx);
        let mut next=None;let mut start=false;
        egui::Window::new("Skirmish startup preview").anchor(egui::Align2::CENTER_CENTER,egui::Vec2::ZERO).resizable(false).collapsible(false).default_width(560.0).show(ctx,|ui| {
            ui.colored_label(egui::Color32::from_rgb(255,185,110), "Development 10: playable battles are unfinished.");
            ui.label("Choose an original map to test the current startup path.");
            let current=self.skirmish_map.as_ref().and_then(|id|self.store.maps().iter().find(|m|&m.id==id));
            egui::ComboBox::from_label("Map").selected_text(current.map(|m|m.name.as_str()).unwrap_or("Select a map")).width(380.0).show_ui(ui,|ui| {
                for map in self.store.maps().iter().filter(|m|!m.id.starts_with("Campaign/")) {
                    ui.selectable_value(&mut self.skirmish_map,Some(map.id.clone()),format!("{} ({})",map.name,map.id));
                }
            });
            ui.horizontal(|ui|{ui.label("Player name");ui.text_edit_singleline(&mut self.skirmish_player);});
            ui.horizontal(|ui|{ui.label("Game name");ui.text_edit_singleline(&mut self.skirmish_name);});
            ui.horizontal(|ui|{ui.label("Random seed");ui.add(egui::DragValue::new(&mut self.skirmish_seed));});
            ui.checkbox(&mut self.skirmish_observers,"Allow observers");
            ui.label("Uses the selected map's player slots. Startup stops when it reaches an unfinished step; this preview cannot complete a battle.");
            ui.separator();
            ui.horizontal(|ui| {
                let valid=self.skirmish_map.is_some()&&!self.skirmish_player.trim().is_empty()&&!self.skirmish_name.trim().is_empty();
                start=ui.add_enabled(valid,egui::Button::new("Test skirmish startup")).clicked();
                if ui.button("Main menu").clicked(){next=Some(Screen::MainMenu);}
            });
        });
        if start {
            let result=(|| {
                self.window_focus.require_observed()?;
                let map=self.store.maps().iter().find(|m|Some(&m.id)==self.skirmish_map.as_ref()).ok_or("selected original map is unavailable")?;
                let request=crate::battle_controller::Request{map_id:map.id.clone(),source_path:map.source.clone(),player_name:self.skirmish_player.clone(),game_name:self.skirmish_name.clone(),seed:self.skirmish_seed,observers:self.skirmish_observers};
                self.battle.queue(request)?;
                self.tgi.loading(&map.id,"app_status_loading_resources");
                Ok::<_,String>(())
            })();
            match result {Ok(())=>next=Some(Screen::SkirmishLoading(0)),Err(e)=>self.error=Some(e)}
        }
        next
    }

    fn skirmish_session(&mut self,ctx:&egui::Context)->Option<Screen> {
        self.backdrop(ctx);let mut next=None;let mut leave=false;
        //Paint the same TGI forest selected by the real AppState notification
        //path. Presentation never writes Loading15/17 or unfreezes Sim29.
        for action in self.tgi.frame(ctx,&mut self.ui,&mut self.video,None,true){match action{
            crate::tgi_frontend::Action::UserReady=>if let Err(error)=self.battle.user_ready_command(){self.error=Some(error)},
            crate::tgi_frontend::Action::Sound(ids)=>{self.audio.play(&ids,1.0);},
            other=>self.error=Some(format!("native battle UI action {other:?} requires its application callback")),
        }}
        if matches!(self.battle.status(),crate::battle_controller::Status::Activating){ctx.request_repaint();}
        egui::Window::new("Local skirmish").anchor(egui::Align2::CENTER_CENTER,egui::Vec2::ZERO).resizable(true).collapsible(false).default_width(650.0).show(ctx,|ui| {
            ui.colored_label(egui::Color32::from_rgb(255,185,110), "Development preview — playable battle unavailable");
            if let Some(request)=self.battle.request(){ui.heading(&request.game_name);ui.label(&request.map_id);}
            if matches!(self.battle.status(),crate::battle_controller::Status::Failed){ui.colored_label(egui::Color32::from_rgb(255,185,110),"Skirmish could not start");}
            ui.label(self.battle.message());
            ui.separator();
            ui.horizontal(|ui| {
                if matches!(self.battle.status(),crate::battle_controller::Status::Failed)&&ui.button("Change setup").clicked(){next=Some(Screen::SkirmishSetup);}
                if ui.button("Main menu").clicked(){next=Some(Screen::MainMenu);}
                if matches!(self.battle.status(),crate::battle_controller::Status::Failed)&&ui.button("Discard stopped attempt").clicked(){leave=true;next=Some(Screen::SkirmishSetup);}
            });
        });
        if leave && let Err(error)=self.battle.discard_stopped_attempt() {self.error=Some(error);}
        next
    }

    fn viewer(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) -> Option<Screen> {
        if self.viewer.is_none() {
            match crate::App::new(
                self.render_state.clone(),
                ctx.clone(),
                self.store.clone(),
                crate::PreviewRequest::Map("Campaign/tutorial01".into()),
                self.memory_policy,
            ) {
                Ok(v) => self.viewer = Some(v),
                Err(e) => {
                    self.error = Some(format!("{e:#}"));
                    return Some(Screen::MainMenu);
                }
            }
        }
        let mut leave = false;
        if let Some(v) = &mut self.viewer {
            <crate::App as eframe::App>::update(v, ctx, frame);
            egui::Area::new(egui::Id::new("viewer_back"))
                .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 34.0))
                .show(ctx, |ui| {
                    if ui.button("\u{25c0} Main menu").clicked() {
                        leave = true;
                    }
                });
        }
        if leave {
            self.viewer = None;
            style(ctx);
            return Some(Screen::MainMenu);
        }
        None
    }

    /// Scripted UI input and screenshots (--ui-script / --shot).
    fn automation(&mut self, ctx: &egui::Context) {
        if self.script.is_none()
            && let Some(path) = self.auto.script.take()
        {
            match crate::ui_script::Script::load(&path) {
                Ok(s) => self.script = Some(s),
                Err(e) => eprintln!("ui-script {}: {e:#}", path.display()),
            }
        }
        if let Some(script) = &mut self.script {
            if let Some(p) = script.shot_request.take() {
                self.pending_shot = Some(p);
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            if script.quit {
                if let Some(p) = self.auto.shot.take() {
                    self.pending_shot = Some(p);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                } else if self.pending_shot.is_none() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            ctx.request_repaint();
        }
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = shot
            && let Some(path) = self.pending_shot.take()
        {
            let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
            match image::RgbaImage::from_raw(image.size[0] as u32, image.size[1] as u32, rgba).map(|img| img.save(&path)) {
                Some(Ok(())) => println!("saved {}", path.display()),
                _ => eprintln!("screenshot failed"),
            }
            if self.script.as_ref().is_some_and(|s| s.quit) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

impl Drop for Shell{
    fn drop(&mut self){
        //Wake any simulation thread waiting on a real UI/GPU reply before the
        //Controller's retained SimulationThread is dropped/joined.
        self.battle_ui_queue.close();
    }
}
impl eframe::App for Shell {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.window_focus.observe();
        let now = Instant::now();
        let dt = now.duration_since(self.last_input).as_secs_f32();
        self.last_input = now;
        if let Some(script) = &mut self.script {
            script.feed(raw, dt);
        }
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.window_focus.observe_frame(frame);
        let(ui,menus)=self.tgi.native_parts();
        if let Err(error)=self.battle_ui_queue.pump(&mut crate::battle_ui_bridge::Context{
            ui,menus,audio:&mut self.audio,renderer:&mut self._renderer,store:&self.store,
            load_settings:&self.battle_load_settings,battle_scene:&mut self.battle_scene}){self.error=Some(error);}
        self.battle.poll();
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.audio.update(dt);
        self.automation(ctx);
        if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
            self.prefs.fullscreen = !self.prefs.fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.prefs.fullscreen));
        }
        let next = match self.screen {
            Screen::MainMenu => self.main_menu(ctx),
            Screen::Loading(n) => {
                for _ in self.tgi.frame(ctx, &mut self.ui, &mut self.video, None, false) {}
                ctx.request_repaint();
                // paint the loading screen at least twice before blocking
                Some(if n >= 2 { Screen::Viewer } else { Screen::Loading(n + 1) })
            }
            Screen::Viewer => self.viewer(ctx, frame),
            Screen::SkirmishSetup=>self.skirmish_setup(ctx),
            Screen::SkirmishLoading(n)=>{
                for _ in self.tgi.frame(ctx,&mut self.ui,&mut self.video,None,false) {}
                ctx.request_repaint();
                if n>=2 {self.battle.start_queued();Some(Screen::SkirmishSession)} else {Some(Screen::SkirmishLoading(n+1))}
            }
            Screen::SkirmishSession=>self.skirmish_session(ctx),
        };
        if let Some(s) = next {
            if matches!(s, Screen::MainMenu) && !matches!(self.screen, Screen::MainMenu) {
                self.tgi.show(crate::tgi_frontend::Page::MainMenu);
            }
            self.screen = s;
        }
        // errors are shown in the original shared message box (0x66DF90)
        if let Some(e) = self.error.take() {
            self.tgi.message(&e);
            self.screen = Screen::MainMenu;
        }
    }
}
