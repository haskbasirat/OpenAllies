//! Code-defined widget hierarchies of the original screens.
//!
//! The original builds every widget in code and only takes layout/art from
//! the TGI node of the same name (TGI child widgets that code does not
//! create are ignored, `0x5F2070` loop at 0x5F2492). The builders below
//! reproduce the native constructors (addresses per widget) in their
//! AddChild order, which is also the draw order.
//!
//! Not ported yet: the internal children of list boxes, drop-downs,
//! scroll bars, sliders, viewports, edit boxes, message logs and the
//! minimap (their constructors are known, see `Widget::composite`, but
//! their bounds, visibility and items are set by class code that is not
//! ported), the runtime-created staging slots, and the HUD panels' own
//! code-created children (object display, action buttons, side panel,
//! floaters, idle buttons). Only these widgets' own `[View]` is drawn.

use crate::ui::Ui;
use crate::widget::{Widget, WidgetClass as C};

fn w(class: C, name: &str, reg: bool) -> Widget {
    Widget::new(class, name, reg)
}

/// Native composite constructors (see [`Widget::composite`]).
pub fn composite(class: C, name: &str, reg: bool) -> Widget {
    Widget::composite(class, name, reg)
}

fn button(name: &str, reg: bool, command: &str) -> Widget {
    Widget::push_button(name, reg, command)
}

fn toggle(name: &str, reg: bool, command: &str) -> Widget {
    Widget::toggle_button(name, reg, command)
}

/// Load and connect a top-level widget.
pub fn show(ui: &mut Ui, mut x: Widget) {
    ui.load_widget(&mut x, None);
    ui.connect(x);
}

/// `0x652BE0` MenuBackground (key -FLT_MAX, 0x652C20); LoadData 0x652C60
/// creates the registered `IntroCinematic` child when `use_bink` is set.
pub fn menu_background(ui: &mut Ui) -> Widget {
    let mut bg = w(C::Panel, "/UI/Menus/menu_background.tgi:MenuBackground", false);
    bg.key = -f32::MAX;
    let use_bink = ui
        .store
        .node("/UI/Menus/menu_background.tgi:MenuBackground")
        .ok()
        .and_then(|n| n.bool_or("use_bink", false).ok())
        .unwrap_or(false);
    if use_bink {
        bg.add_child(w(C::Cinematic, "IntroCinematic", true));
    }
    bg
}

/// `0x64F730` "/MainMenu" panel with its eight push buttons (commands
/// compiled by 0x50CBE0; CheckForUpdates becomes ExternalPatchCheckConfirm
/// when the PatchExternalPath file exists, 0x64FCDC).
pub fn main_menu_panel() -> Widget {
    let mut p = w(C::Panel, "/MainMenu", false);
    for (n, c) in [
        ("MainMenuButtonSinglePlayer", "RequestSinglePlayer"),
        ("MainMenuButtonMultiplayer", "RequestMultiplayer"),
        ("MainMenuButtonEditor", "RequestEditor"),
        ("MainMenuButtonFilm", "RequestFilm"),
        ("MainMenuButtonOptions", "OpenOptionsMenu"),
        ("MainMenuButtonCredits", "RequestCredits"),
        ("MainMenuButtonQuit", "QuitConfirm"),
        ("MainMenuButtonCheckForUpdates", "CheckForUpdates"),
    ] {
        p.add_child(button(n, true, c));
    }
    p
}

/// The widgets the per-frame UI update (`0x5C92D0`, block 0x5C95B0..0x5C96D4)
/// connects for an interface with `show_menu_background` /
/// `show_logo_and_build_stamp`: MenuBackground, MiniMenuBackground (when
/// AVar `UIUseMiniMenuBackground`), MenuBackgroundLogo, MainMenuLabelVersion
/// (text = Version + " AA", `0x64FF00`), MenuLegalText.
pub fn menu_backdrop(ui: &mut Ui) {
    let bg = menu_background(ui);
    show(ui, bg);
    if ui.avars.flag("UIUseMiniMenuBackground").unwrap_or(false) {
        show(ui, w(C::Cinematic, "/UI/Menus/menu_background.tgi:MiniMenuBackground", false));
    }
    show(ui, w(C::Decal, "/UI/Menus/menu_background.tgi:MenuBackgroundLogo", false));
    let mut version = w(C::Label, "/UI/Menus/Main.tgi:MainMenuLabelVersion", true);
    ui.load_widget(&mut version, None);
    let v = format!("{}{}", ui.avars.get("Version").map(crate::node::unquote).unwrap_or_default(), " AA");
    if let Some(crate::ve::ViewElement { kind: crate::ve::VeKind::Label(l), .. }) = &mut version.sub_ve {
        l.text = v;
    }
    ui.connect(version);
    show(ui, w(C::Label, "/UI/Menus/Main.tgi:MenuLegalText", false));
}

/// Main menu: backdrop, then MainMenuInterface (`0x650A80`, slot 0) brought
/// to the front by `0x650400`.
pub fn main_menu(ui: &mut Ui) {
    ui.clear();
    menu_backdrop(ui);
    let mut iface = w(C::Interface, "UI/Menus/main.tgi:MainMenuInterface", false);
    iface.add_child(main_menu_panel());
    show(ui, iface);
    ui.layout();
}

/// Single player menu (`0x65B9D0`, Init `0x65B340`), shown by `0x65B6F0`
/// after MainMenuInterface is disconnected (`0x64F250`).
pub fn single_player_menu(ui: &mut Ui) {
    ui.clear();
    menu_backdrop(ui);
    let mut sp = w(C::TopLevel, "/UI/Menus/SinglePlayer_menu.tgi:SinglePlayerMenu", true);
    for (n, c) in [
        ("SinglePlayerMenuButtonCampaign", "RequestCampaign"),
        ("SinglePlayerMenuButtonCustom", "RequestCustom"),
        ("SinglePlayerMenuButtonMetagame", "ConfirmPlayDynamicCampaign"),
        ("SinglePlayerMenuButtonMain", "RequestMainMenu"),
    ] {
        sp.add_child(button(n, true, c));
    }
    show(ui, sp);
    ui.layout();
}

/// Loading screen interface (`0x6E87C0`, slot 2, Init `0x6E88E0`).
/// `ready` shows the ReadyButton (hidden at 0x6E8BDF until loading ends).
pub fn loading_screen(ui: &mut Ui, ready: bool) {
    ui.clear();
    ui.depth_mode = crate::ui::DepthMode::TwoLists;
    let mut ls = w(C::Interface, "UI/Loading/loading_screen.tgi:LoadingScreen", false);
    ls.add_child(w(C::Label, "/MapNameLabel", false));
    ls.add_child(w(C::Label, "/MapDescriptionLabel", false));
    let mut objectives = w(C::Panel, "/ObjectivesList", false);
    objectives.add_child(w(C::Viewport, "SlotViewport", false));
    ls.add_child(objectives);
    ls.add_child(w(C::Decal, "/LoadingTextureLeft", false));
    ls.add_child(w(C::Decal, "/LoadingTextureRight", false));
    ls.add_child(w(C::Label, "/StatusLabel", false));
    ls.add_child(button("/ReadyButton", false, "UserReady"));
    ui.load_widget(&mut ls, None);
    // LoadData 0x6E8580: the left/right textures named by the interface node
    if let Ok(node) = ui.store.node("UI/Loading/loading_screen.tgi:LoadingScreen") {
        for (child, key) in [("LoadingTextureLeft", "LoadingTextureLeft"), ("LoadingTextureRight", "LoadingTextureRight")] {
            let tex = node.string_or_empty(key);
            if let Some(c) = ls.child_mut(child) {
                if let Some(crate::ve::ViewElement { kind: crate::ve::VeKind::Decal(d), .. }) = &mut c.sub_ve {
                    d.texture = Some(tex);
                }
            }
        }
    }
    if let Some(r) = ls.child_mut("ReadyButton") {
        r.set_visible(ready);
    }
    ui.connect(ls);
    ui.layout();
}

/// What the loading screen refresh (`0x6E83A0`) reads from the session.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LoadingInfo {
    /// Session state 5 (`[session+0xF8]`, or `[0xB36550+0x58]`) hides the
    /// map name and description labels.
    pub hide_map_labels: bool,
    /// World name (`world+0x24`) and description (`world+0x2C`).
    pub map_name: String,
    pub map_description: String,
    /// `[session+0xC]`: 0 shows the objectives list, 4 selects the
    /// dynamic-campaign art.
    pub kind: i32,
    /// The current campaign mission's own art (`entry+0x14/+0x18`).
    pub mission_textures: Option<(String, String)>,
    /// A world with a local player exists (`0xB3653C` and `0x5C0E10`):
    /// only then is the art chosen from the mission / nation.
    pub local_player: bool,
    /// The local player's nation (`0x50DC00`), whose definition names the
    /// nation art (`loading_texture_left/right`, read by `0x48C050`).
    pub nation: Option<String>,
    /// Status text (`0x6E8300`: `0x76E4C0` session status).
    pub status: String,
}

/// The nation art (`0x48C050`): `loading_texture_left/right` of
/// Data/Nations/<nation>.tgi, relative to the Nations folder (`0x4763C0`
/// prefixes the definition's folder unless the value starts with '/').
pub fn nation_loading_textures(ui: &mut Ui, nation: &str) -> Option<(String, String)> {
    let node = ui.store.node(&format!("Nations/{}.tgi:Nation", nation.to_ascii_lowercase())).ok()?;
    let fix = |v: String| if let Some(s) = v.strip_prefix('/') { s.to_string() } else { format!("Nations/{v}") };
    let l = node.string_or_empty("loading_texture_left");
    let r = node.string_or_empty("loading_texture_right");
    if l.is_empty() || r.is_empty() {
        return None;
    }
    Some((format!("/{}", fix(l)), format!("/{}", fix(r))))
}

/// Loading screen refresh (`0x6E83A0`) after `loading_screen`: label
/// visibility and texts, objectives list visibility, the art choice
/// (mission art, else dynamic campaign art for kind 4, else the local
/// player's nation art, else the interface defaults), ReadyButton hidden,
/// status text. The dynamic-campaign art is only loaded when
/// `0x40D0F0(0xB)` holds (LoadData 0x6E8580): menu table slot 11, the
/// dynamic-campaign menu, exists, i.e. `MetaGame/UI/choose_dynamic_campaign.tgi`
/// is present (`0x943680`). For kind 0 the objectives list is populated
/// (`0x673D40`) and then cleared by the same function (SlotViewport
/// ClearItems at its end), so it is always shown empty.
pub fn loading_refresh(ui: &mut Ui, info: &LoadingInfo) {
    let base = "LoadingScreen";
    for (child, text) in [("MapNameLabel", &info.map_name), ("MapDescriptionLabel", &info.map_description)] {
        let path = format!("{base}/{child}");
        ui.with_widget(&path, |w| w.set_visible(!info.hide_map_labels));
        if !info.hide_map_labels {
            ui.set_label(&path, text);
        }
    }
    ui.with_widget(&format!("{base}/ObjectivesList"), |w| w.set_visible(info.kind == 0));
    // LoadData 0x6E8580 defaults (+0xEC/+0xF0) and dynamic art (+0xF4/+0xF8)
    let node = ui.store.node("UI/Loading/loading_screen.tgi:LoadingScreen").ok();
    let tex = |k: &str| node.as_ref().map(|n| n.string_or_empty(k)).filter(|s| !s.is_empty());
    let defaults = (tex("LoadingTextureLeft"), tex("LoadingTextureRight"));
    let dynamic = if ui.store.root.exists("MetaGame/UI/choose_dynamic_campaign.tgi") {
        (tex("DynamicCampaignLoadingTextureLeft"), tex("DynamicCampaignLoadingTextureRight"))
    } else {
        (None, None)
    };
    let nation = info.nation.as_deref().and_then(|n| nation_loading_textures(ui, n));
    let (left, right) = if !info.local_player {
        defaults
    } else if let Some((l, r)) = &info.mission_textures {
        (Some(l.clone()), Some(r.clone()))
    } else if info.kind == 4 {
        dynamic
    } else if let Some((l, r)) = nation {
        (Some(l), Some(r))
    } else {
        defaults
    };
    for (child, t) in [("LoadingTextureLeft", left), ("LoadingTextureRight", right)] {
        ui.with_widget(&format!("{base}/{child}"), |w| {
            if let Some(crate::ve::ViewElement { kind: crate::ve::VeKind::Decal(d), .. }) = &mut w.sub_ve {
                d.texture = t;
            }
            w.set_visible(true);
        });
    }
    ui.with_widget(&format!("{base}/ReadyButton"), |w| w.set_visible(false));
    ui.set_label(&format!("{base}/StatusLabel"), &info.status);
    ui.layout();
}

/// Loading screen update (`0x6E8C00`) once the session finished loading:
/// status `app_status_finished` and the ReadyButton shown.
pub fn loading_finished(ui: &mut Ui) {
    let status = ui.strings.resolve("#app_status_finished");
    ui.set_label("LoadingScreen/StatusLabel", &status);
    ui.with_widget("LoadingScreen/ReadyButton", |w| w.set_visible(true));
    ui.layout();
}

/// Custom-game staging (`0x661BC0`, slot 3, Init `0x65D350`).
pub fn staging(ui: &mut Ui) {
    ui.clear();
    let mut st = w(C::Interface, "UI/Menus/staging.tgi:StagingMenu", false);
    // 1. WorldParamsPanel (0x69E240)
    let mut wp = w(C::Panel, "WorldParamsPanel", false);
    wp.add_child(w(C::Static, "ActionPanelSeparation", false));
    wp.add_child(w(C::Viewport, "Slots", false));
    wp.add_child(w(C::Panel, "TeamActionPanel", false));
    wp.add_child(w(C::Panel, "KingdomActionPanel", false));
    st.add_child(wp);
    // 2. StagingChatPanel (0x6673A0)
    let mut chat = w(C::Panel, "StagingChatPanel", false);
    let mut mlp = w(C::Panel, "MessageLogPanel", false);
    mlp.add_child(w(C::Static, "MessageLogSeparation", false));
    mlp.add_child(w(C::MessageLog, "MessageLog", false));
    mlp.add_child(w(C::Editbox, "MessageEditbox", false));
    mlp.add_child(button("SendButton", false, "ChatMessage"));
    chat.add_child(mlp);
    let mut plp = w(C::Panel, "PlayersListBoxPanel", false);
    plp.add_child(w(C::Static, "PlayersListboxSeparation", false));
    plp.add_child(w(C::Listbox, "PlayersListbox", false));
    let mut filter = w(C::Panel, "FilterPanel", false);
    filter.add_child(toggle("AllToggle", false, "ChatToggleAll"));
    filter.add_child(toggle("TeamToggle", false, "ChatToggleTeam"));
    filter.add_child(toggle("WhisperToggle", false, "ChatToggleWhisper"));
    filter.add_child(button("BuddyButton", false, "ChatBuddyList"));
    plp.add_child(filter);
    chat.add_child(plp);
    st.add_child(chat);
    // 3. InfoPanel (0x692A70)
    let mut info = w(C::Panel, "InfoPanel", false);
    info.add_child(w(C::Label, "GameNameLabel", false));
    info.add_child(w(C::Label, "NumPlayersLabel", false));
    info.add_child(w(C::Decal, "PasswordIndicatorDecal", false));
    info.add_child(w(C::DropDown, "TeamFormatDropdown", false));
    info.add_child(w(C::Label, "TeamFormatLabel", false));
    info.add_child(button("GameSettingsButton", false, "OpenSettings"));
    st.add_child(info);
    // 4. ActionPanel (0x6897F0)
    let mut act = w(C::Panel, "ActionPanel", false);
    for (n, c) in [
        ("OptionsButton", "StagingOpenOptions"),
        ("GameOptionsButton", "OpenSettings"),
        ("ScoreButton", "StagingOpenScoring"),
        ("DetailsButton", "StagingOpenDetails"),
    ] {
        act.add_child(button(n, false, c));
    }
    act.add_child(toggle("ReadyToggle", false, "StagingReady %d"));
    for (n, c) in [
        ("LaunchButton", "StagingLaunch"),
        ("AbortLaunchButton", "StagingAbortLaunch"),
        ("ExitButton", "StagingConfirmExit"),
    ] {
        act.add_child(button(n, false, c));
    }
    st.add_child(act);
    // 5. MapPreviewPanel
    let mut mp = w(C::Panel, "MapPreviewPanel", false);
    mp.add_child(w(C::Viewport, "MapPreviewViewport", false));
    mp.add_child(w(C::Static, "MapPreviewPanelSeparation", false));
    st.add_child(mp);
    // 6. launch-type buttons
    for (n, c) in [
        ("MapButton", "SelectLaunchType LOAD_MAP"),
        ("SaveButton", "SelectLaunchType LOAD_SAVED_GAME"),
        ("FilmButton", "SelectLaunchType LOAD_FILM"),
        ("RMCButton", "SelectLaunchType RANDOM_MAP"),
    ] {
        st.add_child(button(n, false, c));
    }
    show(ui, st);
    ui.layout();
}

/// In-game HUD: Game interface (`0x61A650`, Init `0x6182B0`) and, on
/// connect (`0x60BAE0`), ControlPanel (`0x615FB0`) and ControlPanelTop.
pub fn game_hud(ui: &mut Ui) {
    game_hud_scene(ui, crate::hud::PlaceholderScene::Ingame);
}

/// The game interface (`hud.rs`) filled from a PLACEHOLDER world that
/// reproduces one of the original screenshots.
pub fn game_hud_scene(ui: &mut Ui, scene: crate::hud::PlaceholderScene) {
    let world = crate::hud::PlaceholderWorld { scene };
    let mut hud = crate::hud::Hud::show(ui, &world);
    if scene == crate::hud::PlaceholderScene::RecruitMenu {
        // 08-left1: one message in the game message log (PLACEHOLDER
        // text in the native owner-prefix format)
        let owner = crate::hud::OwnerInfo {
            name: "Rundstedt AI's Anti-Aircraft Battery destroyed".into(),
            rank_glyph: None,
            kingdom_color: [0.5, 0.5, 0.6],
            relation_color: None,
        };
        let uv = ui.uvars.clone();
        ui.messages.clear();
        ui.messages.send(&uv, "BUILDING_DESTROYED", &crate::hud::owner_text(ui, &owner), None);
    }
    // the movers settle the economy bar (slide_time .25 s)
    for t in [0u64, 1000, 2000] {
        ui.update(t);
        hud.update(ui, &world);
    }
    ui.layout();
}

/// In-game menu dialog (`0x5C0C40` -> `0x61D3B0`, Init `0x61D5D0`) over the
/// HUD. ButtonFinished's client is a label (`0x5CF4F0`, text set by code).
pub fn game_menu(ui: &mut Ui) {
    game_hud_scene(ui, crate::hud::PlaceholderScene::RecruitMenu);
    let mut d = w(C::TopLevel, "UI/Game/game_menu.tgi:GameMenuDialog", true);
    d.add_child(w(C::Label, "GamePausedLabel", false));
    d.add_child(button("ButtonOptions", false, "OptionsDialog"));
    d.add_child(button("ButtonLoad", false, "GameLoadDialog"));
    d.add_child(button("ButtonSave", false, "GameSaveDialog"));
    let mut finished = w(C::PushButton, "ButtonFinished", false);
    finished.command = Some("RequestFinishedConfirm".into());
    finished.add_child(w(C::Label, "Client", false));
    d.add_child(finished);
    d.add_child(button("ButtonQuit", false, "QuitConfirm"));
    d.add_child(button("ButtonConnectionStatus", false, "DisplayConnectionStatus"));
    d.add_child(button("ButtonReturnToGame", false, "ReturnToGame"));
    show(ui, d);
    ui.layout();
}

/// The quit confirmation (app command `QuitConfirm`, handler `0x413230`):
/// localized `quit_confirm|s` with `game_name`, buttons `button_ok` (runs
/// `Quit`) and `button_cancel` (no command), shown in the message box that
/// `0x5C0EE0` selects for the current interface.
pub fn quit_confirm(ui: &mut Ui, in_game: bool) {
    let game = ui.strings.resolve("#game_name");
    let text = ui.strings.format("quit_confirm|s", &[&game]);
    let ok = ui.strings.resolve("#button_ok");
    let cancel = ui.strings.resolve("#button_cancel");
    let path = if in_game { crate::message_box::GAME } else { crate::message_box::SHARED };
    ui.show_message_box(path, crate::message_box::MessageBoxRequest { text, buttons: vec![(ok, "Quit".into()), (cancel, String::new())] });
}

/// In-game menu with the quit confirmation on top (13-quit.png).
pub fn game_quit(ui: &mut Ui) {
    game_menu(ui);
    quit_confirm(ui, true);
}

/// Main menu with the quit confirmation on top.
pub fn main_menu_quit(ui: &mut Ui) {
    main_menu(ui);
    quit_confirm(ui, false);
}
