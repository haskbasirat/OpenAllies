//! The options dialog (`OptionsDialog`, `.\Source\K2ui\Shared\OptionsDialog.cpp`).
//!
//! * Created once at UI setup (`0x5C5E97`): ctor `0x67B110`
//!   (`UI/Shared/options_dialog.tgi:OptionsDialog`, registered) creates the
//!   pages GamePlay `0x6769F0`, Video `0x6772F0`, Audio `0x677630`, Network
//!   `0x677B80` and, when its TGI loads, MetaGame `0x9444C0`
//!   (`MetaGame/UI/metagame_options_page.tgi:MetaGamePage`); it registers
//!   `OptionsOK` `0x6768D0`, `OptionsCancel` `0x674430`, `OptionsDefaults`
//!   `0x674480`, `OptionsDefaultsConfirm` `0x6793B0` and
//!   `OptionsPanelSelectTab` `0x6749D0`.
//! * Init `0x6788E0`: one tab toggle per page (`0x5F3190`, client
//!   "Client", command `"OptionsPanelSelectTab " + tab name`), the pages'
//!   CreateChildren, AddTabPage `0x5DF6C0` (toggle, then page; the toggle
//!   joins the radio group), SelectTab(GamePlay) `0x5DF250`, then the OK,
//!   Cancel and Defaults push buttons.
//! * Show `0x6792A0`: re-show (close when active), ShowDialog, and select
//!   the named tab when one is given; activation `0x6743E0` runs every
//!   page's ReadSettings.
//! * OK: every page's Apply, close (unless the video page keeps the dialog
//!   open), save the UVars. Cancel: every page's Cancel, close. Defaults:
//!   a confirmation box (`ui_game_confirm_reset_options`), then every
//!   page's Defaults (controls only).
//!
//! Layout comes from data only. Engine services the port does not have are
//! inputs: the display modes (the original enumerates D3D modes,
//! `0x675110`), the camera zoom limits (`camera+0x20C/+0x210`), the game
//! speed state (`0x76B2F0`); a display mode change cannot be made by this
//! renderer, so it takes the native failure path
//! (`failed_to_change_resolution`, revert).

use crate::controls::{self as k, SliderCallback, SliderFormat};
use crate::message_box::{MessageBoxRequest, SHARED};
use crate::state::widget_flag as wf;
use crate::ui::Ui;
use crate::widget::{Widget, WidgetClass as C};

pub const DIALOG: &str = "UI/Shared/options_dialog.tgi:OptionsDialog";
pub const METAGAME_FILE: &str = "MetaGame/UI/metagame_options_page.tgi";
/// Path of the connected dialog.
pub const PATH: &str = "Root/OptionsDialog";

/// What the caller has to do after a dialog command.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// The dialog closed.
    Closed,
    /// OK: persist the same [`Ui::uvars`] (`0x4B3610` to User/UVars.tgi in the
    /// original).
    Save,
    /// Live channel volume (`0x7453A0`): 0 = 2D, 1 = 3D, 2 = music,
    /// 3 = speech, 4 = main.
    Volume { channel: u8, value: f32 },
    /// Play a test sound by audio IDS.
    PlaySound(String),
    /// `camera->0x7005F0(zoom)`.
    CameraZoom(f32),
    /// Game speed change message (`0x60D0B0`).
    GameSpeed(f32),
    /// `OptionsHotkeySelect`: load the hotkey configuration.
    HotkeyConfiguration(String),
    /// LAN nickname (`0x41E3B0`).
    Nickname(String),
}

/// (tab name, page widget segment, tab button segment).
const PAGES: [(&str, &str, &str); 5] = [
    ("Gameplay", "GamePlayPage", "ShowGameplayButton"),
    ("Video", "VideoPage", "ShowVideoButton"),
    ("Audio", "AudioPage", "ShowAudioButton"),
    ("Network", "NetworkPage", "ShowNetworkButton"),
    ("MetaGame", "MetaGamePage", "ShowMetaGameButton"),
];

/// Toggles mapped directly (or inverted) to a UVar: (path below the
/// dialog, variable, inverted).
const TOGGLES: &[(&str, &str, bool)] = &[
    ("GamePlayPage/UIOptionsPanel/ToggleCameraRotate", "CameraRotate", false),
    ("GamePlayPage/UIOptionsPanel/ToggleToolTips", "UIToolTipsShow", false),
    ("GamePlayPage/UIOptionsPanel/ToggleConfirmationDialog", "UIMessageBoxConfirmation", false),
    ("GamePlayPage/UIOptionsPanel/ToggleStructureAutoPlace", "UIStructureAutoPlacement", false),
    ("GamePlayPage/UIOptionsPanel/ToggleShowVFXObjectName", "ViewShowVFXObjectName", false),
    ("GamePlayPage/MessageOptionsPanel/ToggleEventFilterSiege", "UIEventFilterSiege", false),
    ("GamePlayPage/MessageOptionsPanel/ToggleEventFilterEngagement", "UIEventFilterEngagement", false),
    ("GamePlayPage/MessageOptionsPanel/ToggleEventFilterEngagementAudioOnly", "UIEventFilterEngagementMessages", true),
    ("GamePlayPage/MessageOptionsPanel/ToggleEventFilterProduction", "UIEventFilterProduction", false),
    ("GamePlayPage/MapOptionsPanel/ToggleDamageNumbers", "ViewShowDamage", false),
    ("GamePlayPage/MapOptionsPanel/ToggleZoneSupply", "ViewShowZoneFriendlySupply", false),
    ("GamePlayPage/MapOptionsPanel/ToggleElapsedGameTime", "ViewShowElapsedGameTime", false),
    ("VideoPage/GraphicsOptionsPanel/ToggleRenderFootsteps", "RenderDrawFootsteps", false),
    ("VideoPage/GraphicsOptionsPanel/ToggleTerrainHighQuality", "TerrainExtraQuality", false),
    ("VideoPage/GraphicsOptionsPanel/ToggleTerrainTrilinearFiltering", "TerrainTrilinearFilter", false),
    ("VideoPage/GraphicsOptionsPanel/ToggleRenderUnitFog", "UnitFogSimplified", true),
    ("VideoPage/GraphicsOptionsPanel/ToggleRenderHighQualityTextures", "RendererUseHighQualityTextures", false),
    ("VideoPage/GraphicsOptionsPanel/ToggleRenderDecals", "PerformanceDisableDecals", true),
    ("AudioPage/SoundsFilterOptionsPanel/ToggleUnitAcknowledgement", "UnitAcknowledgement", false),
    ("AudioPage/SoundsFilterOptionsPanel/ToggleChatSounds", "UIEventFilterChatSound", false),
    ("NetworkPage/InternetOptionsPanel/NetworkReportGameInProgressToggle", "NetworkReportGameInProgress", false),
    ("NetworkPage/ChatOptionsPanel/ToggleMessageFilterBuddy", "UIMessageFilterBuddy", false),
    ("NetworkPage/ChatOptionsPanel/ToggleMessageFilterChat", "UIMessageFilterChat", false),
    ("NetworkPage/ChatOptionsPanel/ToggleMessageFilterAFK", "UIMessageFilterAFK", false),
    ("NetworkPage/ChatOptionsPanel/ToggleMessageFilterPlayerJoinLeave", "UIMessageFilterPlayerJoinLeave", false),
    ("NetworkPage/InternetGameBrowserOptionsPanel/NetworkGamesDifferentVersionToggle", "NetworkFilterSameVersion", true),
    ("NetworkPage/InternetGameBrowserOptionsPanel/NetworkGamesDifferentGroupRoomToggle", "NetworkFilterSameGroupRoom", true),
    ("NetworkPage/InternetGameBrowserOptionsPanel/NetworkGamesFullGamesToggle", "NetworkFilterFullGames", true),
    ("NetworkPage/InternetGameBrowserOptionsPanel/NetworkGamesInProgressToggle", "NetworkFilterPlayingGames", true),
    ("NetworkPage/InternetGameBrowserOptionsPanel/NetworkGamesDifferentLanguageToggle", "NetworkFilterSameLanguage", true),
    ("NetworkPage/InternetGameBrowserOptionsPanel/NetworkAutoRefreshToggle", "NetworkAutoRefresh", false),
    ("MetaGamePage/MetaGameMessagePanel/ToggleMessageFilterMetaGameGeneric", "UIMessageFilterMetaGameGeneric", false),
    ("MetaGamePage/MetaGameMessagePanel/ToggleMessageFilterMetaGameMovement", "UIMessageFilterMetaGameMovement", false),
    ("MetaGamePage/MetaGameMessagePanel/ToggleMessageFilterMetaGamePurchases", "UIMessageFilterMetaGamePurchases", false),
    ("MetaGamePage/MetaGameMessagePanel/ToggleMessageFilterMetaGameBattles", "UIMessageFilterMetaGameBattles", false),
    ("MetaGamePage/MetaGameOptionsPanel/ToggleMetaGameShowVFXMovement", "MetaGameViewShowVFXMovement", false),
    ("MetaGamePage/MetaGameOptionsPanel/ToggleMetaGameShowVFXObjectName", "MetaGameViewShowVFXObjectName", false),
    ("MetaGamePage/MetaGameOptionsPanel/ToggleMetaGameShowVFXArrows", "MetaGameViewShowVFXArrows", false),
    ("MetaGamePage/MetaGameOptionsPanel/ToggleMetaGameShowUIHelpPopup", "MetaGameUIShowHelpPopup", false),
    ("MetaGamePage/MetaGameOptionsPanel/ToggleMetaGameFastMGAI", "MetaGameUIFastMGAI", false),
    ("MetaGamePage/MetaGameOptionsPanel/ToggleMetaGameComputerQuickBattle", "MetaGameUIComputerQuickBattle", false),
    ("MetaGamePage/MetaGameOptionsPanel/ToggleMetaGamePurchaseConfirm", "MetaGameUIPurchaseConfirm", false),
];

/// Sliders mapped to a UVar: (path, variable, integer).
const SLIDERS: &[(&str, &str, bool)] = &[
    ("GamePlayPage/MessageLogOptionsPanel/MessageLogMaximumLinesSlider", "UIMessageLogMaximumLines", true),
    ("GamePlayPage/MessageLogOptionsPanel/MessageLogFadeTimeSlider", "UIMessageLogFadeTime", true),
    ("VideoPage/GraphicsOptionsPanel/RenderLODShadowsSlider", "RenderLODShadows", true),
    ("VideoPage/GraphicsOptionsPanel/ParticlesSlider", "ParticleGeneratePCG", false),
    ("AudioPage/VolumeOptionsPanel/AudioMainVolumeSlider", "AudioMainVolume", false),
    ("AudioPage/VolumeOptionsPanel/AudioFXVolumeSlider", "Audio2DVolume", false),
    ("AudioPage/VolumeOptionsPanel/AudioSpeechVolumeSlider", "AudioSpeechVolume", false),
    ("AudioPage/VolumeOptionsPanel/AudioMusicVolumeSlider", "AudioMusicVolume", false),
    ("NetworkPage/ChatOptionsPanel/NetworkAFKTimeSlider", "AFKTime", true),
    ("NetworkPage/InternetGameBrowserOptionsPanel/NetworkRefreshTimeSlider", "NetworkRefreshTime", true),
];

/// The graphics preset variables (`0x67BE90` compares them all).
const PRESET_VARS: [&str; 9] = [
    "TerrainTrilinearFilter",
    "TerrainQuadSize",
    "TerrainExtraQuality",
    "RenderLODShadows",
    "ParticleGeneratePCG",
    "RenderDrawFootsteps",
    "UnitFogSimplified",
    "RendererUseHighQualityTextures",
    "PerformanceDisableDecals",
];

const GAMEPLAY: &str = "GamePlayPage";
const HOTKEY_DD: &str = "GamePlayPage/GameSpeedOptionsPanel/DropDownHotkeySelect";
const GAMESPEED: &str = "GamePlayPage/GameSpeedOptionsPanel/GameSpeedSlider";
const CAMSCROLL: &str = "GamePlayPage/CameraOptionsPanel/CameraScrollSlider";
const CAMZOOM: &str = "GamePlayPage/CameraOptionsPanel/CameraZoomSlider";
const ENGAGEMENT: &str = "GamePlayPage/MessageOptionsPanel/ToggleEventFilterEngagement";
const AUDIO_ONLY: &str = "GamePlayPage/MessageOptionsPanel/ToggleEventFilterEngagementAudioOnly";
const VFX_NAMES: &str = "GamePlayPage/UIOptionsPanel/ToggleShowVFXObjectName";
const RES_DD: &str = "VideoPage/ResolutionOptionsPanel/DropDownResolution";
const PRESET_DD: &str = "VideoPage/ResolutionOptionsPanel/DropDownGraphicsPreset";
const DOUBLE_DENSITY: &str = "VideoPage/GraphicsOptionsPanel/ToggleTerrainDoubleDensity";
const LOD: &str = "VideoPage/GraphicsOptionsPanel/RenderLODShadowsSlider";
const PARTICLES: &str = "VideoPage/GraphicsOptionsPanel/ParticlesSlider";
const VOL_MAIN: &str = "AudioPage/VolumeOptionsPanel/AudioMainVolumeSlider";
const VOL_FX: &str = "AudioPage/VolumeOptionsPanel/AudioFXVolumeSlider";
const VOL_SPEECH: &str = "AudioPage/VolumeOptionsPanel/AudioSpeechVolumeSlider";
const VOL_MUSIC: &str = "AudioPage/VolumeOptionsPanel/AudioMusicVolumeSlider";
const CONNECTION_DD: &str = "NetworkPage/InternetOptionsPanel/NetworkConnectionDropdown";
const NICKNAME: &str = "NetworkPage/LANOptionsPanel/NetworkLanNicknameEditbox";
const AFK: &str = "NetworkPage/ChatOptionsPanel/NetworkAFKTimeSlider";
const AUTO_REFRESH: &str = "NetworkPage/InternetGameBrowserOptionsPanel/NetworkAutoRefreshToggle";
const REFRESH_TIME: &str = "NetworkPage/InternetGameBrowserOptionsPanel/NetworkRefreshTimeSlider";

fn p(rel: &str) -> String {
    format!("{PATH}/{rel}")
}

// factories (all children reg=0)
fn panel(name: &str) -> Widget {
    Widget::new(C::Panel, name, false)
}
fn label(name: &str) -> Widget {
    Widget::new(C::Label, name, false)
}
/// `0x5DEE40(&name, 0, callback, 0, formatter)`.
fn slider(name: &str, cb: SliderCallback, fmt: SliderFormat) -> Widget {
    let mut s = Widget::composite(C::SliderAdvanced, name, false);
    if let Some(x) = s.slider_mut() {
        x.callback = cb;
        if let Some(a) = &mut x.advanced {
            a.2 = fmt;
        }
    }
    s
}
/// `0x5DEED0(&name, 0, &command, 0, formatter)`.
fn command_slider(name: &str, cmd: &str, fmt: SliderFormat) -> Widget {
    slider(name, SliderCallback::Command(cmd.to_string()), fmt)
}
/// `0x5D6100(&name, 0, &command)`.
fn dropdown(name: &str, cmd: &str) -> Widget {
    let mut d = Widget::composite(C::DropDown, name, false);
    if let Some(k::Control::DropDown(x)) = d.control.as_deref_mut() {
        x.command = Some(cmd.to_string());
    }
    d
}
/// `0x5F3080` (no command, no client), `0x5F3160` (command), `0x5F3140`
/// (client), `0x5F3190` (command and client).
fn toggle(name: &str, cmd: Option<&str>, client: bool) -> Widget {
    let mut t = Widget::new(C::ToggleButton, name, false);
    t.command = cmd.map(str::to_string);
    if client {
        t.add_child(Widget::new(C::Static, "Client", false));
    }
    t
}
fn with(mut w: Widget, children: Vec<Widget>) -> Widget {
    for c in children {
        w.add_child(c);
    }
    w
}

/// GamePlayPage CreateChildren (`0x679C70`).
fn gameplay_page(siege: bool) -> Widget {
    let mut message_toggles = Vec::new();
    if siege {
        message_toggles.push(toggle("ToggleEventFilterSiege", None, false));
    }
    message_toggles.push(toggle("ToggleEventFilterEngagement", Some("OptionsRefreshEngagementAudioOnly"), false));
    message_toggles.push(toggle("ToggleEventFilterEngagementAudioOnly", None, false));
    message_toggles.push(toggle("ToggleEventFilterProduction", None, false));
    with(
        panel(GAMEPLAY),
        vec![
            with(
                panel("MessageLogOptionsPanel"),
                vec![
                    slider("MessageLogMaximumLinesSlider", SliderCallback::None, SliderFormat::Rounded),
                    slider("MessageLogFadeTimeSlider", SliderCallback::None, SliderFormat::FastSlow),
                ],
            ),
            with(
                panel("CameraOptionsPanel"),
                vec![
                    slider("CameraScrollSlider", SliderCallback::None, SliderFormat::SlowFast),
                    slider("CameraZoomSlider", SliderCallback::Notify, SliderFormat::InOut),
                ],
            ),
            with(
                panel("GameSpeedOptionsPanel"),
                vec![
                    slider("GameSpeedSlider", SliderCallback::Notify, SliderFormat::GameSpeed),
                    dropdown("DropDownHotkeySelect", "OptionsHotkeySelect %s"),
                ],
            ),
            with(
                panel("UIOptionsPanel"),
                vec![
                    toggle("ToggleCameraRotate", None, false),
                    toggle("ToggleToolTips", None, false),
                    toggle("ToggleConfirmationDialog", None, false),
                    toggle("ToggleStructureAutoPlace", None, false),
                    toggle("ToggleShowVFXObjectName", None, false),
                ],
            ),
            with(panel("MessageOptionsPanel"), message_toggles),
            with(
                panel("MapOptionsPanel"),
                vec![
                    toggle("ToggleDamageNumbers", None, false),
                    toggle("ToggleZoneSupply", None, false),
                    toggle("ToggleElapsedGameTime", None, false),
                ],
            ),
            label("MessageLogTitle"),
            label("CameraTitle"),
            label("GameTitle"),
            label("UITitle"),
            label("MessagesTitle"),
            label("MapTitle"),
        ],
    )
}

/// VideoPage CreateChildren (`0x67A830`).
fn video_page() -> Widget {
    let t = |n: &str| toggle(n, Some("OptionsChangeDropDown"), false);
    with(
        panel("VideoPage"),
        vec![
            with(
                panel("ResolutionOptionsPanel"),
                vec![
                    dropdown("DropDownResolution", "OptionsVideoChange"),
                    dropdown("DropDownGraphicsPreset", "OptionsGraphicsPreset %s"),
                ],
            ),
            with(
                panel("GraphicsOptionsPanel"),
                vec![
                    t("ToggleRenderFootsteps"),
                    t("ToggleTerrainHighQuality"),
                    t("ToggleTerrainDoubleDensity"),
                    t("ToggleTerrainTrilinearFiltering"),
                    command_slider("RenderLODShadowsSlider", "OptionsChangeDropDown", SliderFormat::NoneHigh),
                    command_slider("ParticlesSlider", "OptionsChangeDropDown", SliderFormat::NoneFull),
                    t("ToggleRenderUnitFog"),
                    t("ToggleRenderHighQualityTextures"),
                    t("ToggleRenderDecals"),
                ],
            ),
            label("BasicTitle"),
            label("AdvancedTitle"),
        ],
    )
}

/// AudioPage CreateChildren (`0x677740`).
fn audio_page() -> Widget {
    let s = |n: &str| slider(n, SliderCallback::Notify, SliderFormat::Default);
    with(
        panel("AudioPage"),
        vec![
            with(
                panel("VolumeOptionsPanel"),
                vec![s("AudioMainVolumeSlider"), s("AudioFXVolumeSlider"), s("AudioSpeechVolumeSlider"), s("AudioMusicVolumeSlider")],
            ),
            with(
                panel("SoundsFilterOptionsPanel"),
                vec![toggle("ToggleUnitAcknowledgement", None, false), toggle("ToggleChatSounds", None, false)],
            ),
            label("VolumeTitle"),
            label("SoundsTitle"),
        ],
    )
}

/// NetworkPage CreateChildren (`0x677D40`).
fn network_page() -> Widget {
    with(
        panel("NetworkPage"),
        vec![
            with(
                panel("InternetOptionsPanel"),
                vec![
                    dropdown("NetworkConnectionDropdown", "OptionsNetworkSelectConnection %s"),
                    toggle("NetworkReportGameInProgressToggle", None, true),
                ],
            ),
            with(panel("LANOptionsPanel"), vec![Widget::composite(C::Editbox, "NetworkLanNicknameEditbox", false)]),
            with(
                panel("ChatOptionsPanel"),
                vec![
                    command_slider("NetworkAFKTimeSlider", "", SliderFormat::Rounded),
                    toggle("ToggleMessageFilterBuddy", None, false),
                    toggle("ToggleMessageFilterChat", None, false),
                    toggle("ToggleMessageFilterAFK", None, false),
                    toggle("ToggleMessageFilterPlayerJoinLeave", None, false),
                ],
            ),
            with(
                panel("InternetGameBrowserOptionsPanel"),
                vec![
                    toggle("NetworkGamesDifferentVersionToggle", None, true),
                    toggle("NetworkGamesDifferentGroupRoomToggle", None, true),
                    toggle("NetworkGamesFullGamesToggle", None, true),
                    toggle("NetworkGamesInProgressToggle", None, true),
                    toggle("NetworkGamesDifferentLanguageToggle", None, true),
                    toggle("NetworkAutoRefreshToggle", Some("OptionsNetworkToggleAutoRefresh"), true),
                    command_slider("NetworkRefreshTimeSlider", "", SliderFormat::Rounded),
                ],
            ),
            label("InternetTitle"),
            label("LANTitle"),
            label("ChatTitle"),
            label("GameBrowserTitle"),
        ],
    )
}

/// MetaGamePage CreateChildren (`0x943F20`).
fn metagame_page() -> Widget {
    let t = |n: &str| toggle(n, None, false);
    with(
        Widget::new(C::Panel, &format!("{METAGAME_FILE}:MetaGamePage"), false),
        vec![
            label("MetaGameOptionsTitle"),
            label("MetaGameMessageTitle"),
            with(
                panel("MetaGameMessagePanel"),
                vec![
                    t("ToggleMessageFilterMetaGameGeneric"),
                    t("ToggleMessageFilterMetaGameMovement"),
                    t("ToggleMessageFilterMetaGamePurchases"),
                    t("ToggleMessageFilterMetaGameBattles"),
                ],
            ),
            with(
                panel("MetaGameOptionsPanel"),
                vec![
                    t("ToggleMetaGameShowVFXMovement"),
                    t("ToggleMetaGameShowVFXObjectName"),
                    t("ToggleMetaGameShowVFXArrows"),
                    t("ToggleMetaGameShowUIHelpPopup"),
                    t("ToggleMetaGameFastMGAI"),
                    t("ToggleMetaGameComputerQuickBattle"),
                    t("ToggleMetaGamePurchaseConfirm"),
                ],
            ),
        ],
    )
}

pub struct Options {
    /// Display modes (width, height, bpp) the renderer offers.
    pub video_modes: Vec<(i32, i32, i32)>,
    /// The current display mode (width, height).
    pub current_mode: (i32, i32),
    /// Camera zoom limits (`camera+0x20C/+0x210`), when a camera exists.
    pub camera_zoom_range: Option<(f32, f32)>,
    /// Adjustable game speed (min, max, value) while a game allows it
    /// (`0x76B2F0`); `None` disables the slider.
    pub game_speed: Option<(f32, f32, f32)>,
    /// LAN nickname (`[0xB39D04]+0x2C`).
    pub nickname: String,
    /// Network connection type IDS (`[0xB39D04]+0xA4`).
    pub connection: String,
    widget: Option<Widget>,
    open: bool,
    has_metagame: bool,
    /// Current page (`+0xA8`).
    current: usize,
    /// VideoPage `+0xAC` (resolution changed).
    resolution_dirty: bool,
    /// AudioPage `+0xA0..+0xAC`: live volumes at ReadSettings (main, 2D,
    /// speech, music).
    live: [f32; 4],
    /// `+0xFC` close on OK.
    close_on_ok: bool,
    /// AudioPage LoadData `0x67C730`: `fx_test`, `speech_test` sound IDS.
    fx_test: String,
    speech_test: String,
    pub diagnostics: Vec<String>,
}

impl Options {
    pub fn new(_ui: &mut Ui) -> Self {
        Options {
            video_modes: Vec::new(),
            current_mode: (0, 0),
            camera_zoom_range: None,
            game_speed: None,
            nickname: String::new(),
            connection: String::new(),
            widget: None,
            open: false,
            has_metagame: false,
            current: 0,
            resolution_dirty: false,
            live: [0.0; 4],
            close_on_ok: true,
            fx_test: String::new(),
            speech_test: String::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    fn avar_flag(ui: &Ui, name: &str, code_default: bool) -> bool {
        ui.avars.flag(name).unwrap_or(code_default)
    }

    fn avar_f32(ui: &Ui, name: &str, code_default: f32) -> f32 {
        ui.avars.float(name).unwrap_or(code_default)
    }

    /// ctor `0x67B110` + Init `0x6788E0` + LoadData.
    fn build(&mut self, ui: &mut Ui) -> Widget {
        // UIOptionsDialogShowFilterSiegeToggle (code default false)
        let siege = Self::avar_flag(ui, "UIOptionsDialogShowFilterSiegeToggle", false);
        let mut pages = vec![gameplay_page(siege), video_page(), audio_page(), network_page()];
        // the dialog's templates (OptionsColumnPanel ...) are registered
        // before the MetaGame page file is read
        let _ = ui.store.load_file(DIALOG.split(':').next().unwrap_or(""));
        self.has_metagame = ui.store.load_file(METAGAME_FILE).is_ok();
        if self.has_metagame {
            pages.push(metagame_page());
        } else {
            self.diagnostics.push(format!("{METAGAME_FILE} did not load: no MetaGame tab"));
        }
        let mut dlg = Widget::new(C::TopLevel, DIALOG, true);
        for (i, page) in pages.into_iter().enumerate() {
            let (tab, _, button) = PAGES[i];
            let mut t = toggle(button, Some(&format!("OptionsPanelSelectTab {tab}")), true);
            t.radio_group = 1;
            dlg.add_child(t);
            dlg.add_child(page);
        }
        for (name, cmd) in
            [("OptionsButtonOK", "OptionsOK"), ("OptionsButtonCancel", "OptionsCancel"), ("OptionsButtonDefaults", "OptionsDefaultsConfirm")]
        {
            dlg.add_child(Widget::push_button(name, false, cmd));
        }
        // SelectTab(GamePlay) during Init
        self.current = 0;
        Self::apply_tab_flags(&mut dlg, 0);
        ui.load_widget(&mut dlg, None);
        dlg
    }

    fn page_count(&self) -> usize {
        if self.has_metagame { 5 } else { 4 }
    }

    fn page_child<'a>(dlg: &'a mut Widget, seg: &str) -> Option<&'a mut Widget> {
        dlg.child_mut(seg)
    }

    /// SelectTab flags (`0x5DF250`): the selected page shown and its toggle
    /// selected, every other page hidden.
    fn apply_tab_flags(dlg: &mut Widget, current: usize) {
        for (i, (_, page, button)) in PAGES.iter().enumerate() {
            if let Some(pw) = Self::page_child(dlg, page) {
                pw.set_visible(i == current);
            }
            if let Some(b) = dlg.child_mut(button) {
                if i == current {
                    b.flags |= wf::SELECTED;
                } else {
                    b.flags &= !wf::SELECTED;
                }
            }
        }
    }

    /// Show (`0x6792A0`): `tab` "" keeps the last tab.
    pub fn show(&mut self, ui: &mut Ui, tab: &str) {
        if self.open {
            self.hide(ui);
        }
        let w = match self.widget.take() {
            Some(w) => w,
            None => self.build(ui),
        };
        ui.connect(w);
        self.open = true;
        self.close_on_ok = true;
        self.load_data(ui);
        // activation 0x6743E0: every page's ReadSettings
        self.read_gameplay(ui);
        self.read_video(ui);
        self.read_audio(ui);
        self.read_network(ui);
        if self.has_metagame {
            self.read_toggles(ui, "MetaGamePage/");
        }
        if !tab.is_empty() {
            self.select_tab_name(ui, tab);
        }
        ui.update(ui.now_ms);
    }

    /// Close (`0x679340`); the dialog object is kept.
    pub fn hide(&mut self, ui: &mut Ui) {
        if let Some(w) = ui.disconnect("OptionsDialog") {
            self.widget = Some(w);
        }
        self.open = false;
    }

    /// Take the connected dialog out before the UI is rebuilt and put it
    /// back afterwards (window size changes), keeping unsaved edits.
    pub fn suspend(&mut self, ui: &mut Ui) {
        if self.open {
            if let Some(w) = ui.disconnect("OptionsDialog") {
                self.widget = Some(w);
            }
        }
    }

    pub fn resume(&mut self, ui: &mut Ui) {
        if self.open {
            if let Some(w) = self.widget.take() {
                ui.connect(w);
            }
        }
    }

    /// Page LoadData: slider ranges from the UVar descriptors (`0x674040`,
    /// `0x67B040`, `0x67C730`) and the AVars (network page).
    fn load_data(&mut self, ui: &mut Ui) {
        if let Ok(n) = ui.store.node("UI/Shared/options_dialog.tgi:OptionsDialog/AudioPage") {
            self.fx_test = n.string_or_empty("fx_test");
            self.speech_test = n.string_or_empty("speech_test");
        }
        for (path, var) in [
            (CAMSCROLL, "CameraScrollSpeedMax"),
            ("GamePlayPage/MessageLogOptionsPanel/MessageLogMaximumLinesSlider", "UIMessageLogMaximumLines"),
            ("GamePlayPage/MessageLogOptionsPanel/MessageLogFadeTimeSlider", "UIMessageLogFadeTime"),
            (LOD, "RenderLODShadows"),
            (PARTICLES, "ParticleGeneratePCG"),
            (VOL_MAIN, "AudioMainVolume"),
            (VOL_FX, "Audio2DVolume"),
            (VOL_SPEECH, "AudioSpeechVolume"),
            (VOL_MUSIC, "AudioMusicVolume"),
        ] {
            if let Some((lo, hi)) = ui.uvars.range(var) {
                ui.slider_set_range(&p(path), lo, hi);
            }
        }
        // AFKTime / refresh time: AVars min/max/delta
        for (path, base, d) in [(AFK, "AFKTime", (30.0, 180.0, 30.0)), (REFRESH_TIME, "GameSpyServerRefreshTime", (10.0, 60.0, 10.0))] {
            let lo = Self::avar_f32(ui, &format!("{base}Min"), d.0);
            let hi = Self::avar_f32(ui, &format!("{base}Max"), d.1);
            let step = Self::avar_f32(ui, &format!("{base}Delta"), d.2);
            ui.slider_set_range(&p(path), lo, hi);
            if let Some(s) = ui.widget_mut(&p(path)).and_then(|w| w.slider_mut()) {
                s.step = step;
            }
        }
    }

    fn set_toggle(ui: &mut Ui, rel: &str, on: bool) {
        ui.set_flag(&p(rel), wf::SELECTED, on);
    }

    fn toggle_on(ui: &Ui, rel: &str) -> bool {
        ui.widget(&p(rel)).is_some_and(|w| w.flags & wf::SELECTED != 0)
    }

    fn set_enabled(ui: &mut Ui, rel: &str, on: bool) {
        if let Some(w) = ui.widget_mut(&p(rel)) {
            w.set_enabled(on);
        }
    }

    fn read_toggles(&mut self, ui: &mut Ui, prefix: &str) {
        for (path, var, inv) in TOGGLES.iter().filter(|t| t.0.starts_with(prefix)) {
            let v = ui.uvars.get_bool(var);
            Self::set_toggle(ui, path, v != *inv);
        }
    }

    fn write_toggles(&mut self, ui: &mut Ui, prefix: &str) {
        for (path, var, inv) in TOGGLES.iter().filter(|t| t.0.starts_with(prefix)) {
            if ui.widget(&p(path)).is_some() {
                let on = Self::toggle_on(ui, path);
                ui.uvars.set_bool(var, on != *inv);
            }
        }
    }

    fn default_toggles(&self, ui: &mut Ui, prefix: &str) {
        for (path, var, inv) in TOGGLES.iter().filter(|t| t.0.starts_with(prefix)) {
            Self::set_toggle(ui, path, ui.uvars.default_bool(var) != *inv);
        }
    }

    fn read_sliders(&self, ui: &mut Ui, prefix: &str, defaults: bool) {
        for (path, var, int) in SLIDERS.iter().filter(|s| s.0.starts_with(prefix)) {
            let v = match (int, defaults) {
                (true, false) => ui.uvars.get_i32(var) as f32,
                (true, true) => ui.uvars.default_i32(var) as f32,
                (false, false) => ui.uvars.get_f32(var),
                (false, true) => ui.uvars.default_f32(var),
            };
            ui.slider_set_value(&p(path), v, true);
        }
    }

    fn write_sliders(&mut self, ui: &mut Ui, prefix: &str) {
        for (path, var, int) in SLIDERS.iter().filter(|s| s.0.starts_with(prefix)) {
            if let Some(v) = ui.slider_value(&p(path)) {
                if *int {
                    // _ftol truncation (0x96CC4C)
                    ui.uvars.set_i32(var, v as i32);
                } else {
                    ui.uvars.set_f32(var, v);
                }
            }
        }
    }

    // ---------------------------------------------------------- game play

    fn read_gameplay(&mut self, ui: &mut Ui) {
        let show_vfx = Self::avar_flag(ui, "OptionsDialogShowVFXNamesToggle", true);
        if let Some(w) = ui.widget_mut(&p(VFX_NAMES)) {
            w.set_visible(show_vfx);
        }
        self.read_toggles(ui, "GamePlayPage/");
        let engagement = Self::toggle_on(ui, ENGAGEMENT);
        Self::set_enabled(ui, AUDIO_ONLY, engagement);
        self.read_sliders(ui, "GamePlayPage/", false);
        ui.slider_set_value(&p(CAMSCROLL), ui.uvars.get_f32("CameraScrollSpeedMax"), true);
        match self.camera_zoom_range {
            Some((lo, hi)) => ui.slider_set_range(&p(CAMZOOM), lo, hi),
            None => self.diagnostics.push("CameraZoomSlider: no camera, the data range is kept".into()),
        }
        ui.slider_set_value(&p(CAMZOOM), ui.uvars.get_f32("CameraScaleZoom"), true);
        // game speed (0x76B2F0)
        match self.game_speed {
            Some((lo, hi, v)) => {
                Self::set_enabled(ui, GAMESPEED, true);
                ui.slider_set_range(&p(GAMESPEED), lo, hi);
                ui.slider_set_value(&p(GAMESPEED), v, true);
            }
            None => {
                Self::set_enabled(ui, GAMESPEED, false);
                // GVars GameSpeedMin/Max (code defaults -2/2)
                ui.slider_set_range(&p(GAMESPEED), -2.0, 2.0);
                ui.slider_set_value(&p(GAMESPEED), 0.0, true);
            }
        }
        // hotkey configurations (0x674F80)
        let dd = p(HOTKEY_DD);
        let lb = format!("{dd}/Listbox");
        ui.listbox_clear(&lb);
        let configs = hotkey_configs(ui);
        for (ids, name) in &configs {
            ui.listbox_add(&lb, name, ids);
        }
        let want = ui.uvars.get_str("HotkeyConfiguration");
        let entries = ui.listbox_entries(&lb);
        let idx = entries.iter().position(|e| e.param.eq_ignore_ascii_case(&want)).unwrap_or(0);
        if !entries.is_empty() {
            ui.dropdown_select(&dd, idx);
        }
    }

    fn apply_gameplay(&mut self, ui: &mut Ui) {
        self.write_toggles(ui, "GamePlayPage/");
        self.write_sliders(ui, "GamePlayPage/");
        // camera scroll: the max and the speed/acceleration scaled from
        // their defaults (0x6763E0)
        if let Some(v) = ui.slider_value(&p(CAMSCROLL)) {
            ui.uvars.set_f32("CameraScrollSpeedMax", v);
            let ratio = v / ui.uvars.default_f32("CameraScrollSpeedMax");
            ui.uvars.set_f32("CameraScrollSpeed", ui.uvars.default_f32("CameraScrollSpeed") * ratio);
            ui.uvars.set_f32("CameraScrollAcceleration", ui.uvars.default_f32("CameraScrollAcceleration") * ratio);
        }
        if let Some(v) = ui.slider_value(&p(CAMZOOM)) {
            ui.uvars.set_f32("CameraScaleZoom", v);
        }
        if let Some(e) = ui.listbox_selected(&p(&format!("{HOTKEY_DD}/Listbox"))).and_then(|i| ui.listbox_entries(&p(&format!("{HOTKEY_DD}/Listbox"))).get(i).cloned()) {
            ui.uvars.set_str("HotkeyConfiguration", &e.param);
        }
    }

    fn defaults_gameplay(&mut self, ui: &mut Ui) {
        self.default_toggles(ui, "GamePlayPage/");
        self.read_sliders(ui, "GamePlayPage/", true);
        ui.slider_set_value(&p(CAMSCROLL), ui.uvars.default_f32("CameraScrollSpeedMax"), true);
        ui.slider_set_value(&p(CAMZOOM), ui.uvars.default_f32("CameraScaleZoom"), true);
        // GVar GameSpeed (code default 0)
        ui.slider_set_value(&p(GAMESPEED), 0.0, true);
        let want = ui.uvars.default_str("HotkeyConfiguration");
        let entries = ui.listbox_entries(&p(&format!("{HOTKEY_DD}/Listbox")));
        if let Some(i) = entries.iter().position(|e| e.param.eq_ignore_ascii_case(&want)) {
            ui.dropdown_select(&p(HOTKEY_DD), i);
        }
    }

    // ---------------------------------------------------------- video

    fn read_video(&mut self, ui: &mut Ui) {
        self.resolution_dirty = false;
        self.read_toggles(ui, "VideoPage/");
        Self::set_toggle(ui, DOUBLE_DENSITY, ui.uvars.get_i32("TerrainQuadSize") == 1);
        self.read_sliders(ui, "VideoPage/", false);
        self.fill_resolutions(ui);
        // presets (0x67C8B0)
        let dd = p(PRESET_DD);
        let lb = format!("{dd}/Listbox");
        ui.listbox_clear(&lb);
        for (ids, name) in graphics_presets(ui).iter().map(|(i, n, _)| (i.clone(), n.clone())) {
            ui.listbox_add(&lb, &name, &ids);
        }
        self.match_preset(ui);
    }

    /// Resolution list (`0x6767B0`, mode callback `0x675110`).
    fn fill_resolutions(&mut self, ui: &mut Ui) {
        let dd = p(RES_DD);
        let lb = format!("{dd}/Listbox");
        ui.listbox_clear(&lb);
        let min_x = ui.avars.float("MinimumGameResolutionX").unwrap_or(640.0) as i32;
        let four_three = Self::avar_flag(ui, "Requires43Resolutions", false);
        for &(w, h, bpp) in &self.video_modes {
            if bpp != 32 || w < min_x || (four_three && w * 3 != h * 4) {
                continue;
            }
            ui.listbox_add(&lb, &format!("{w}x{h}x{bpp}"), &format!("{w} {h} {bpp}"));
        }
        let (cw, ch) = self.current_mode;
        let want = format!("{cw} {ch} 32");
        let fallback = format!("{} {} 32", ui.uvars.default_i32("ResolutionX"), ui.uvars.default_i32("ResolutionY"));
        let entries = ui.listbox_entries(&lb);
        if let Some(i) = entries.iter().position(|e| e.param == want).or_else(|| entries.iter().position(|e| e.param == fallback)) {
            ui.dropdown_select(&dd, i);
        }
    }

    /// The control values compared with a preset.
    fn control_preset_values(ui: &Ui) -> [f32; 9] {
        let t = |rel: &str| if Self::toggle_on(ui, rel) { 1.0 } else { 0.0 };
        [
            t("VideoPage/GraphicsOptionsPanel/ToggleTerrainTrilinearFiltering"),
            if Self::toggle_on(ui, DOUBLE_DENSITY) { 1.0 } else { 2.0 },
            t("VideoPage/GraphicsOptionsPanel/ToggleTerrainHighQuality"),
            ui.slider_value(&p(LOD)).unwrap_or(0.0),
            ui.slider_value(&p(PARTICLES)).unwrap_or(0.0),
            t("VideoPage/GraphicsOptionsPanel/ToggleRenderFootsteps"),
            1.0 - t("VideoPage/GraphicsOptionsPanel/ToggleRenderUnitFog"),
            t("VideoPage/GraphicsOptionsPanel/ToggleRenderHighQualityTextures"),
            1.0 - t("VideoPage/GraphicsOptionsPanel/ToggleRenderDecals"),
        ]
    }

    /// Preset match (`0x67BE90`): select low/medium/high when every control
    /// matches (dropping "custom"), else add and select "custom".
    fn match_preset(&mut self, ui: &mut Ui) {
        let dd = p(PRESET_DD);
        let lb = format!("{dd}/Listbox");
        let values = Self::control_preset_values(ui);
        let presets = graphics_presets(ui);
        let mut hit = None;
        for name in ["low", "medium", "high"] {
            if let Some((_, _, v)) = presets.iter().find(|(i, _, _)| i.eq_ignore_ascii_case(name)) {
                if v.iter().zip(values.iter()).all(|(a, b)| a == b) {
                    hit = Some(name);
                    break;
                }
            }
        }
        let entries = ui.listbox_entries(&lb);
        let custom_idx = entries.iter().position(|e| e.param == "custom");
        match hit {
            Some(name) => {
                if custom_idx.is_some() {
                    let keep: Vec<_> = entries.into_iter().filter(|e| e.param != "custom").collect();
                    ui.listbox_clear(&lb);
                    for e in keep {
                        ui.listbox_add(&lb, &e.text, &e.param);
                    }
                }
                let entries = ui.listbox_entries(&lb);
                if let Some(i) = entries.iter().position(|e| e.param.eq_ignore_ascii_case(name)) {
                    ui.dropdown_select(&dd, i);
                }
            }
            None => {
                let i = match custom_idx {
                    Some(i) => i,
                    None => {
                        let text = ui
                            .store
                            .node(&format!("{}:OptionsDialog/VideoPage", DIALOG.split(':').next().unwrap_or("")))
                            .ok()
                            .map(|n| n.string_or_empty("custom_preset_string"))
                            .unwrap_or_default();
                        let text = ui.strings.resolve(&text);
                        ui.listbox_add(&lb, &text, "custom")
                    }
                };
                ui.dropdown_select(&dd, i);
            }
        }
    }

    /// `OptionsGraphicsPreset <ids>` (`0x67BB80`).
    fn apply_preset(&mut self, ui: &mut Ui, ids: &str) {
        let presets = graphics_presets(ui);
        let Some((_, _, v)) = presets.iter().find(|(i, _, _)| i.eq_ignore_ascii_case(ids)) else { return };
        let v = *v;
        Self::set_toggle(ui, "VideoPage/GraphicsOptionsPanel/ToggleTerrainTrilinearFiltering", v[0] != 0.0);
        Self::set_toggle(ui, DOUBLE_DENSITY, v[1] == 1.0);
        Self::set_toggle(ui, "VideoPage/GraphicsOptionsPanel/ToggleTerrainHighQuality", v[2] != 0.0);
        ui.slider_set_value(&p(LOD), v[3], true);
        ui.slider_set_value(&p(PARTICLES), v[4], true);
        Self::set_toggle(ui, "VideoPage/GraphicsOptionsPanel/ToggleRenderFootsteps", v[5] != 0.0);
        Self::set_toggle(ui, "VideoPage/GraphicsOptionsPanel/ToggleRenderUnitFog", v[6] == 0.0);
        Self::set_toggle(ui, "VideoPage/GraphicsOptionsPanel/ToggleRenderHighQualityTextures", v[7] != 0.0);
        Self::set_toggle(ui, "VideoPage/GraphicsOptionsPanel/ToggleRenderDecals", v[8] == 0.0);
        self.match_preset(ui);
    }

    /// Mode change (`0x67CD00`): the renderer cannot switch display modes,
    /// so the native failure path runs (`failed_to_change_resolution`,
    /// revert to the current mode).
    fn change_mode(&mut self, ui: &mut Ui) {
        let text = ui.strings.resolve("#failed_to_change_resolution");
        let ok = ui.strings.resolve("#button_ok");
        self.resolution_dirty = false;
        self.fill_resolutions(ui);
        ui.show_message_box(SHARED, MessageBoxRequest { text, buttons: vec![(ok, String::new())] });
    }

    fn apply_video(&mut self, ui: &mut Ui) -> bool {
        if self.resolution_dirty {
            self.change_mode(ui);
            // the dialog stays open (dialog+0xFC = 0)
            return false;
        }
        self.write_toggles(ui, "VideoPage/");
        ui.uvars.set_i32("TerrainQuadSize", if Self::toggle_on(ui, DOUBLE_DENSITY) { 1 } else { 2 });
        self.write_sliders(ui, "VideoPage/");
        true
    }

    fn defaults_video(&mut self, ui: &mut Ui) {
        let dd = p(RES_DD);
        let want = format!("{} {} 32", ui.uvars.default_i32("ResolutionX"), ui.uvars.default_i32("ResolutionY"));
        let entries = ui.listbox_entries(&format!("{dd}/Listbox"));
        if let Some(i) = entries.iter().position(|e| e.param == want) {
            ui.dropdown_select(&dd, i);
            let (cw, ch) = self.current_mode;
            self.resolution_dirty = want != format!("{cw} {ch} 32");
        }
        // preset "high" with notify (fires OptionsGraphicsPreset high)
        let pdd = p(PRESET_DD);
        let pentries = ui.listbox_entries(&format!("{pdd}/Listbox"));
        if let Some(i) = pentries.iter().position(|e| e.param.eq_ignore_ascii_case("high")) {
            ui.dropdown_select_notify(&pdd, i);
        }
    }

    // ---------------------------------------------------------- audio

    fn read_audio(&mut self, ui: &mut Ui) {
        self.read_toggles(ui, "AudioPage/");
        self.read_sliders(ui, "AudioPage/", false);
        self.live = [
            ui.uvars.get_f32("AudioMainVolume"),
            ui.uvars.get_f32("Audio2DVolume"),
            ui.uvars.get_f32("AudioSpeechVolume"),
            ui.uvars.get_f32("AudioMusicVolume"),
        ];
    }

    fn apply_audio(&mut self, ui: &mut Ui) {
        self.write_toggles(ui, "AudioPage/");
        self.write_sliders(ui, "AudioPage/");
        if let Some(v) = ui.slider_value(&p(VOL_FX)) {
            ui.uvars.set_f32("Audio3DVolume", v);
        }
    }

    /// Cancel (`0x673EC0`): restore the live volumes; channel 1 takes the
    /// FX slider's current value (native quirk).
    fn cancel_audio(&self, ui: &Ui) -> Vec<Effect> {
        let fx_now = ui.slider_value(&p(VOL_FX)).unwrap_or(self.live[1]);
        vec![
            Effect::Volume { channel: 4, value: self.live[0] },
            Effect::Volume { channel: 0, value: self.live[1] },
            Effect::Volume { channel: 1, value: fx_now },
            Effect::Volume { channel: 3, value: self.live[2] },
            Effect::Volume { channel: 2, value: self.live[3] },
        ]
    }

    fn defaults_audio(&mut self, ui: &mut Ui) -> Vec<Effect> {
        self.default_toggles(ui, "AudioPage/");
        self.read_sliders(ui, "AudioPage/", true);
        vec![
            Effect::Volume { channel: 4, value: ui.uvars.default_f32("AudioMainVolume") },
            Effect::Volume { channel: 0, value: ui.uvars.default_f32("Audio2DVolume") },
            Effect::Volume { channel: 1, value: ui.uvars.default_f32("Audio2DVolume") },
            Effect::Volume { channel: 3, value: ui.uvars.default_f32("AudioSpeechVolume") },
            Effect::Volume { channel: 2, value: ui.uvars.default_f32("AudioMusicVolume") },
        ]
    }

    // ---------------------------------------------------------- network

    fn read_network(&mut self, ui: &mut Ui) {
        self.read_toggles(ui, "NetworkPage/");
        self.read_sliders(ui, "NetworkPage/", false);
        let auto = Self::toggle_on(ui, AUTO_REFRESH);
        Self::set_enabled(ui, REFRESH_TIME, auto);
        // connection types (0x6745D0)
        let dd = p(CONNECTION_DD);
        let lb = format!("{dd}/Listbox");
        ui.listbox_clear(&lb);
        for (ids, name) in connection_types(ui) {
            ui.listbox_add(&lb, &name, &ids);
        }
        let entries = ui.listbox_entries(&lb);
        if let Some(i) = entries.iter().position(|e| e.param.eq_ignore_ascii_case(&self.connection)) {
            ui.dropdown_select(&dd, i);
        }
        let max = ui.avars.float("LANNicknameMaxLength").unwrap_or(18.0) as i32;
        ui.editbox_set_max(&p(NICKNAME), max);
        ui.editbox_set_text(&p(NICKNAME), &self.nickname);
    }

    fn apply_network(&mut self, ui: &mut Ui) -> Vec<Effect> {
        self.write_toggles(ui, "NetworkPage/");
        self.write_sliders(ui, "NetworkPage/");
        let lb = p(&format!("{CONNECTION_DD}/Listbox"));
        if let Some(e) = ui.listbox_selected(&lb).and_then(|i| ui.listbox_entries(&lb).get(i).cloned()) {
            self.connection = e.param;
        }
        let mut out = Vec::new();
        if let Some(n) = ui.editbox_text(&p(NICKNAME)) {
            self.nickname = n.clone();
            out.push(Effect::Nickname(n));
        }
        out
    }

    fn defaults_network(&mut self, ui: &mut Ui) {
        self.default_toggles(ui, "NetworkPage/");
        self.read_sliders(ui, "NetworkPage/", true);
        // connection and nickname reset to their current values
        let entries = ui.listbox_entries(&p(&format!("{CONNECTION_DD}/Listbox")));
        if let Some(i) = entries.iter().position(|e| e.param.eq_ignore_ascii_case(&self.connection)) {
            ui.dropdown_select(&p(CONNECTION_DD), i);
        }
        ui.editbox_set_text(&p(NICKNAME), &self.nickname);
        let auto = Self::toggle_on(ui, AUTO_REFRESH);
        Self::set_enabled(ui, REFRESH_TIME, auto);
    }

    // ---------------------------------------------------------- tabs

    /// `0x674920`: case-sensitive tab name match.
    fn select_tab_name(&mut self, ui: &mut Ui, name: &str) {
        if let Some(i) = PAGES.iter().take(self.page_count()).position(|(t, _, _)| *t == name) {
            self.select_tab(ui, i);
        } else {
            // keep the radio group on the current page
            self.select_tab(ui, self.current);
        }
    }

    /// SelectTab (`0x5DF250`) with the VideoPage CanLeave (`0x67CFF0`).
    fn select_tab(&mut self, ui: &mut Ui, i: usize) {
        if i != self.current && self.current == 1 && self.resolution_dirty {
            self.change_mode(ui);
            // leaving is refused this time
            self.flags_for_current(ui);
            return;
        }
        self.current = i;
        self.flags_for_current(ui);
    }

    fn flags_for_current(&self, ui: &mut Ui) {
        if let Some(dlg) = ui.widget_mut(PATH) {
            Self::apply_tab_flags(dlg, self.current);
        }
        ui.layout();
    }

    // ---------------------------------------------------------- commands

    /// Handle a UI command; `None` when it is not an options command.
    pub fn command(&mut self, ui: &mut Ui, cmd: &str) -> Option<Vec<Effect>> {
        let (name, arg) = match cmd.split_once(' ') {
            Some((n, a)) => (n, a.trim()),
            None => (cmd, ""),
        };
        let mut out = Vec::new();
        match name {
            "OptionsPanelSelectTab" => self.select_tab_name(ui, arg),
            "OptionsOK" => {
                self.apply_gameplay(ui);
                let video_done = self.apply_video(ui);
                self.apply_audio(ui);
                out.extend(self.apply_network(ui));
                if self.has_metagame {
                    self.write_toggles(ui, "MetaGamePage/");
                }
                if !video_done {
                    self.close_on_ok = false;
                }
                if self.close_on_ok {
                    self.hide(ui);
                    out.push(Effect::Closed);
                }
                self.close_on_ok = true;
                for (ch, var) in [(4u8, "AudioMainVolume"), (0, "Audio2DVolume"), (1, "Audio3DVolume"), (3, "AudioSpeechVolume"), (2, "AudioMusicVolume")] {
                    out.push(Effect::Volume { channel: ch, value: ui.uvars.get_f32(var) });
                }
                out.push(Effect::Save);
            }
            "OptionsCancel" => {
                out.extend(self.cancel_audio(ui));
                self.resolution_dirty = false;
                self.hide(ui);
                out.push(Effect::Closed);
            }
            "OptionsDefaultsConfirm" => {
                let text = ui.strings.resolve("#ui_game_confirm_reset_options");
                let ok = ui.strings.resolve("#button_ok");
                let cancel = ui.strings.resolve("#button_cancel");
                ui.show_message_box(
                    SHARED,
                    MessageBoxRequest { text, buttons: vec![(ok, "OptionsDefaults".into()), (cancel, String::new())] },
                );
            }
            "OptionsDefaults" => {
                self.defaults_gameplay(ui);
                self.defaults_video(ui);
                out.extend(self.defaults_audio(ui));
                self.defaults_network(ui);
                if self.has_metagame {
                    self.default_toggles(ui, "MetaGamePage/");
                }
            }
            "OptionsRefreshEngagementAudioOnly" => {
                let on = Self::toggle_on(ui, ENGAGEMENT);
                Self::set_enabled(ui, AUDIO_ONLY, on);
            }
            "OptionsNetworkToggleAutoRefresh" => {
                let on = Self::toggle_on(ui, AUTO_REFRESH);
                Self::set_enabled(ui, REFRESH_TIME, on);
            }
            "OptionsVideoChange" => {
                let (cw, ch) = self.current_mode;
                let lb = p(&format!("{RES_DD}/Listbox"));
                let sel = ui.listbox_selected(&lb).and_then(|i| ui.listbox_entries(&lb).get(i).cloned());
                self.resolution_dirty = sel.is_some_and(|e| e.param != format!("{cw} {ch} 32"));
                self.match_preset(ui);
            }
            "OptionsGraphicsPreset" => self.apply_preset(ui, arg),
            "OptionsChangeDropDown" => self.match_preset(ui),
            "OptionsHotkeySelect" => out.push(Effect::HotkeyConfiguration(arg.to_string())),
            "OptionsToggleDamageNumbers" => {
                let on = Self::toggle_on(ui, "GamePlayPage/MapOptionsPanel/ToggleDamageNumbers");
                Self::set_toggle(ui, "GamePlayPage/MapOptionsPanel/ToggleDamageNumbers", !on);
            }
            // registered by the dialog without a handler
            "OptionsNetworkSelectConnection" => {}
            _ => return None,
        }
        ui.layout();
        Some(out)
    }

    /// A slider code functor fired (`UiEvent::Value`).
    pub fn value(&mut self, path: &str, value: f32) -> Option<Vec<Effect>> {
        let rel = path.strip_prefix(&format!("{PATH}/"))?;
        let fx = || self.fx_test.clone();
        Some(match rel {
            CAMZOOM => vec![Effect::CameraZoom(value)],
            GAMESPEED => vec![Effect::GameSpeed(value)],
            // 0x673F40: play fx_test, channel 4
            VOL_MAIN => vec![Effect::PlaySound(fx()), Effect::Volume { channel: 4, value }],
            // 0x673F80: play fx_test, channels 0 and 1
            VOL_FX => vec![Effect::PlaySound(fx()), Effect::Volume { channel: 0, value }, Effect::Volume { channel: 1, value }],
            // 0x673FD0: play speech_test, channel 3
            VOL_SPEECH => vec![Effect::PlaySound(self.speech_test.clone()), Effect::Volume { channel: 3, value }],
            // 0x674010: channel 2
            VOL_MUSIC => vec![Effect::Volume { channel: 2, value }],
            _ => return None,
        })
    }
}

/// Hotkey configurations (`[0xB36530]+0x5A0`: text `+0x10`, IDS `+8`).
fn hotkey_configs(ui: &mut Ui) -> Vec<(String, String)> {
    let file = "Localization/Hotkeys/Actions/hotkey_configuration_data.tgi";
    let Ok(root) = ui.store.load_file(file) else { return Vec::new() };
    root.children
        .iter()
        .map(|n| (n.string_or_empty("IDS"), ui.strings.resolve(&n.string_or_empty("name"))))
        .filter(|(i, _)| !i.is_empty())
        .collect()
}

/// Graphics presets (`[0xB36530]+0x37C`: text `+0x38`, IDS `+0x30`) with
/// their values in [`PRESET_VARS`] order.
fn graphics_presets(ui: &mut Ui) -> Vec<(String, String, [f32; 9])> {
    let Ok(root) = ui.store.load_file("Game/GraphicsPresets/GraphicsPresets.tgi") else { return Vec::new() };
    root.children
        .iter()
        .map(|n| {
            let v = PRESET_VARS.map(|k| {
                let raw = n.string_or_empty(k);
                match raw.trim().to_ascii_lowercase().as_str() {
                    "true" => 1.0,
                    "false" => 0.0,
                    t => crate::uvars::parse_number(t).unwrap_or(0.0) as f32,
                }
            });
            (n.string_or_empty("ids"), ui.strings.resolve(&n.string_or_empty("name")), v)
        })
        .collect()
}

/// Connection types (`[0xB36530]+0x594`).
fn connection_types(ui: &mut Ui) -> Vec<(String, String)> {
    let Ok(root) = ui.store.load_file("Game/connection_types.tgi") else { return Vec::new() };
    root.children
        .iter()
        .map(|n| (n.string_or_empty("ids"), ui.strings.resolve(&n.string_or_empty("name"))))
        .collect()
}
