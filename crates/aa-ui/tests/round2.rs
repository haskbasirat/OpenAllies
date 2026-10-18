//! Round-2 screens: control internals, the options dialog, credits and
//! hotkeys, driven through the public UI input API at 1440x900.

use aa_ui::credits::{CreditNode, Credits};
use aa_ui::fs::DataRoot;
use aa_ui::layout::Screen;
use aa_ui::options::{self, Effect, Options};
use aa_ui::screens;
use aa_ui::state::widget_flag as wf;
use aa_ui::ui::{MouseButton, Ui, UiEvent};

fn ui() -> Ui {
    Ui::new(DataRoot::workspace_default(), Screen { width: 1440, height: 900 })
}

fn centre(ui: &Ui, path: &str) -> (f32, f32) {
    let r = ui.widget(path).unwrap_or_else(|| panic!("{path}")).root.outer.rect;
    ((r.l + r.r) / 2.0, (r.t + r.b) / 2.0)
}

fn click(ui: &mut Ui, path: &str) {
    let (x, y) = centre(ui, path);
    ui.mouse_down(x, y, MouseButton::Left);
    ui.mouse_up(x, y, MouseButton::Left);
}

fn commands(ui: &mut Ui) -> Vec<String> {
    ui.take_events()
        .into_iter()
        .filter_map(|e| match e {
            UiEvent::Command { command, .. } => Some(command),
            _ => None,
        })
        .collect()
}

fn open_options(ui: &mut Ui) -> Options {
    screens::main_menu(ui);
    let mut o = Options::new(ui);
    o.video_modes = vec![(1440, 900, 32), (1024, 768, 32), (800, 600, 16)];
    o.current_mode = (1440, 900);
    o.nickname = "Commander".into();
    o.connection = "Broadband".into();
    o.show(ui, "");
    o
}

fn p(rel: &str) -> String {
    format!("{}/{rel}", options::PATH)
}

#[test]
fn options_dialog_builds_the_native_tree_and_reads_the_user_variables() {
    let mut ui = ui();
    let o = open_options(&mut ui);
    let dlg = ui.widget(options::PATH).unwrap();
    // Init 0x6788E0 AddChild order
    let names: Vec<&str> = dlg.children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "ShowGameplayButton",
            "GamePlayPage",
            "ShowVideoButton",
            "VideoPage",
            "ShowAudioButton",
            "AudioPage",
            "ShowNetworkButton",
            "NetworkPage",
            "ShowMetaGameButton",
            "MetaGamePage",
            "OptionsButtonOK",
            "OptionsButtonCancel",
            "OptionsButtonDefaults"
        ]
    );
    // GamePlay is selected, the other pages hidden
    assert!(ui.widget(&p("ShowGameplayButton")).unwrap().flags & wf::SELECTED != 0);
    assert!(ui.widget(&p("GamePlayPage")).unwrap().flags & wf::SHOWN != 0);
    assert!(ui.widget(&p("VideoPage")).unwrap().flags & wf::SHOWN == 0);
    // ToggleEventFilterSiege is not created (AVar false)
    assert!(ui.widget(&p("GamePlayPage/MessageOptionsPanel/ToggleEventFilterSiege")).is_none());
    // audio sliders read Data/UVars.tgi (80% / 80% / 95% / 70%)
    for (s, v) in [("AudioMainVolumeSlider", 0.8), ("AudioFXVolumeSlider", 0.8), ("AudioSpeechVolumeSlider", 0.95), ("AudioMusicVolumeSlider", 0.7)] {
        let got = ui.slider_value(&p(&format!("AudioPage/VolumeOptionsPanel/{s}"))).unwrap();
        assert!((got - v).abs() < 1e-6, "{s} {got}");
    }
    // the slider label formatter 0x5DEA20 (range [0,1] -> "%d%%")
    let lbl = ui.widget(&p("AudioPage/VolumeOptionsPanel/AudioMainVolumeSlider/CurrentValueLabel")).unwrap();
    match &lbl.sub_ve.as_ref().unwrap().kind {
        aa_ui::ve::VeKind::Label(l) => assert_eq!(l.text, "80%"),
        _ => panic!(),
    }
    // the stock UVars match the "high" preset
    let preset = ui.listbox_selected(&p("VideoPage/ResolutionOptionsPanel/DropDownGraphicsPreset/Listbox")).unwrap();
    let entries = ui.listbox_entries(&p("VideoPage/ResolutionOptionsPanel/DropDownGraphicsPreset/Listbox"));
    assert_eq!(entries[preset].param, "high");
    // 16-bit modes are not listed (0x675110)
    assert_eq!(ui.listbox_entries(&p("VideoPage/ResolutionOptionsPanel/DropDownResolution/Listbox")).len(), 2);
    // game speed is disabled outside a game
    assert!(ui.widget(&p("GamePlayPage/GameSpeedOptionsPanel/GameSpeedSlider")).unwrap().flags & wf::ENABLED == 0);
    assert!(o.is_open());
}

#[test]
fn options_borrow_the_existing_ui_variables_before_and_after_construction(){
    let mut ui=ui();
    ui.uvars.set_bool("CameraRotate",true);
    ui.uvars.set_bool("UIMessageFilterGeneric",false);
    let mut options=open_options(&mut ui);
    let toggle=p("GamePlayPage/UIOptionsPanel/ToggleCameraRotate");
    assert!(ui.widget(&toggle).unwrap().flags&wf::SELECTED!=0);
    assert_eq!(ui.uvars.message_filter_generic_current().unwrap(),0);
    ui.widget_mut(&toggle).unwrap().flags&=!wf::SELECTED;
    options.command(&mut ui,"OptionsOK").unwrap();
    assert!(!ui.uvars.get_bool("CameraRotate"));
    assert_eq!(ui.uvars.message_filter_generic_current().unwrap(),0);
    //Current writes after dialog creation are read again by that SAME dialog.
    ui.uvars.set_bool("CameraRotate",true);options.show(&mut ui,"");
    assert!(ui.widget(&toggle).unwrap().flags&wf::SELECTED!=0);
}

#[test]
fn tabs_drop_downs_and_ok_write_the_user_variables() {
    let mut ui = ui();
    let mut o = open_options(&mut ui);
    // tab button -> OptionsPanelSelectTab Video
    click(&mut ui, &p("ShowVideoButton"));
    let cmds = commands(&mut ui);
    assert_eq!(cmds, ["OptionsPanelSelectTab Video"]);
    o.command(&mut ui, &cmds[0]).unwrap();
    assert!(ui.widget(&p("VideoPage")).unwrap().flags & wf::SHOWN != 0);
    assert!(ui.widget(&p("GamePlayPage")).unwrap().flags & wf::SHOWN == 0);
    // open the preset drop-down and pick "medium"
    let dd = p("VideoPage/ResolutionOptionsPanel/DropDownGraphicsPreset");
    click(&mut ui, &dd);
    ui.update(0);
    assert!(ui.widget(&dd).unwrap().flags & wf::OPEN != 0);
    let entries = ui.listbox_entries(&format!("{dd}/Listbox"));
    let medium = entries.iter().position(|e| e.param == "medium").unwrap();
    let item = if medium == 0 { format!("{dd}/Listbox/ItemLook") } else { format!("{dd}/Listbox/ItemLook#{medium}") };
    click(&mut ui, &item);
    let cmds = commands(&mut ui);
    assert_eq!(cmds, ["OptionsGraphicsPreset medium"]);
    assert!(ui.widget(&dd).unwrap().flags & wf::OPEN == 0);
    o.command(&mut ui, &cmds[0]).unwrap();
    // medium: RenderLODShadows 1, ParticleGeneratePCG 0.6
    assert_eq!(ui.slider_value(&p("VideoPage/GraphicsOptionsPanel/RenderLODShadowsSlider")), Some(1.0));
    assert!((ui.slider_value(&p("VideoPage/GraphicsOptionsPanel/ParticlesSlider")).unwrap() - 0.6).abs() < 1e-6);
    // OK writes the variables and saves
    let effects = o.command(&mut ui, "OptionsOK").unwrap();
    assert!(effects.contains(&Effect::Closed));
    assert!(effects.contains(&Effect::Save));
    assert_eq!(ui.uvars.get_i32("RenderLODShadows"), 1);
    assert!(ui.uvars.get_bool("UnitFogSimplified"));
    let saved = ui.uvars.overrides();
    assert_eq!(saved.get("RenderLODShadows").map(String::as_str), Some("1"));
    assert!(!saved.contains_key("AudioMainVolume"));
    assert!(ui.widget(options::PATH).is_none());
    // re-open: the last tab persists, ReadSettings shows the saved values
    o.show(&mut ui, "");
    assert!(ui.widget(&p("VideoPage")).unwrap().flags & wf::SHOWN != 0);
    assert_eq!(ui.slider_value(&p("VideoPage/GraphicsOptionsPanel/RenderLODShadowsSlider")), Some(1.0));
}

#[test]
fn slider_track_click_and_cancel_restore_the_live_volume() {
    let mut ui = ui();
    let mut o = open_options(&mut ui);
    o.command(&mut ui, "OptionsPanelSelectTab Audio").unwrap();
    let s = p("AudioPage/VolumeOptionsPanel/AudioMusicVolumeSlider");
    let r = ui.widget(&s).unwrap().root.outer.rect;
    // a click left of the grip jumps to that position (0x5DDFB0)
    let y = (r.t + r.b) / 2.0;
    ui.mouse_down(r.l + 90.0, y, MouseButton::Left);
    ui.mouse_up(r.l + 90.0, y, MouseButton::Left);
    let v = ui.slider_value(&s).unwrap();
    assert!(v < 0.7, "{v}");
    let values: Vec<f32> = ui
        .take_events()
        .into_iter()
        .filter_map(|e| match e {
            UiEvent::Value { widget, value } if widget == s => Some(value),
            _ => None,
        })
        .collect();
    assert_eq!(values, vec![v]);
    let fx = o.value(&s, v).unwrap();
    assert_eq!(fx, vec![Effect::Volume { channel: 2, value: v }]);
    // Cancel: live channels back to the values read at show time
    let effects = o.command(&mut ui, "OptionsCancel").unwrap();
    assert!(effects.contains(&Effect::Volume { channel: 2, value: 0.7 }));
    assert!(effects.contains(&Effect::Closed));
    assert!((ui.uvars.get_f32("AudioMusicVolume") - 0.7).abs() < 1e-6);
}

#[test]
fn defaults_ask_first_and_hotkeys_reach_the_dialog() {
    let mut ui = ui();
    let mut o = open_options(&mut ui);
    o.command(&mut ui, "OptionsDefaultsConfirm").unwrap();
    let req = ui.message_box.clone().unwrap();
    assert_eq!(req.text, ui.strings.resolve("#ui_game_confirm_reset_options"));
    assert_eq!(req.buttons.len(), 2);
    // Esc on the message box: MessageBoxCancel (last button, no command)
    assert!(ui.hotkey("esc"));
    assert!(ui.message_box.is_none());
    assert_eq!(commands(&mut ui), [""]);
    // Esc on the dialog: OptionsDialog OptionsCancel
    assert!(ui.hotkey("esc"));
    assert_eq!(commands(&mut ui), ["OptionsCancel"]);
    o.command(&mut ui, "OptionsCancel").unwrap();
    // Esc on the main menu presses MainMenuButtonQuit (QuitConfirm)
    assert!(ui.hotkey("esc"));
    assert_eq!(commands(&mut ui), ["QuitConfirm"]);
}

#[test]
fn credits_stream_the_data_nodes_and_end() {
    let mut ui = ui();
    let mut c = Credits::show(&mut ui, 0);
    assert!(c.diagnostics.is_empty(), "{:?}", c.diagnostics);
    assert_eq!(c.nodes.len(), 372);
    assert_eq!(c.nodes[0], CreditNode::Decal { texture: "/Credits/BlackCard.png".into() });
    assert!(matches!(&c.nodes[2], CreditNode::Text { label_ids, text, leading }
        if label_ids == "CreditsTextLabelHeader" && text.trim() == "-- TimeGate Studios, Inc. --" && *leading == 40.0));
    assert_eq!(c.scroll_speed, 50.0);
    assert_eq!(c.fade_length, 25.0);
    // first label starts at the bottom edge and scrolls 50 units/s
    assert_eq!(c.active_labels()[0].1, 0.0);
    c.update(&mut ui, 4000);
    assert_eq!(c.active_labels()[0].1, 200.0);
    // the labels are widgets under the interface, drawn below the gradients
    let iface = ui.widget(aa_ui::credits::PATH).unwrap();
    let last = iface.children.last().unwrap();
    assert_eq!(last.name, "CreditsTextGradientBottom");
    // run to the end: CreditsEndCredits once nothing is left
    let mut t = 4000;
    while !c.update(&mut ui, t) {
        t += 1000;
        assert!(t < 3_600_000, "credits never ended");
    }
    assert!(c.ended);
    assert!(t > 60_000);
}

#[test]
fn loading_screen_uses_the_nation_art_for_a_local_player() {
    let mut ui = ui();
    screens::loading_screen(&mut ui, false);
    let info = screens::LoadingInfo { local_player: true, nation: Some("GER".into()), kind: 1, ..Default::default() };
    screens::loading_refresh(&mut ui, &info);
    let tex = |ui: &Ui, n: &str| match &ui.widget(&format!("Root/LoadingScreen/{n}")).unwrap().sub_ve.as_ref().unwrap().kind {
        aa_ui::ve::VeKind::Decal(d) => d.texture.clone(),
        _ => None,
    };
    assert_eq!(tex(&ui, "LoadingTextureLeft").as_deref(), Some("/Nations/GER/GERSplashScreenLeft.png"));
    // no local player: the interface defaults
    screens::loading_refresh(&mut ui, &screens::LoadingInfo { kind: 1, ..Default::default() });
    assert_eq!(tex(&ui, "LoadingTextureLeft").as_deref(), Some("UI/Loading/Textures/SplashScreenLeft.tga"));
    // the objectives list shows only for kind 0
    assert!(ui.widget("Root/LoadingScreen/ObjectivesList").unwrap().flags & 1 == 0);
}

#[test]
fn edit_box_takes_focus_and_text() {
    let mut ui = ui();
    let mut o = open_options(&mut ui);
    o.command(&mut ui, "OptionsPanelSelectTab Network").unwrap();
    let e = p("NetworkPage/LANOptionsPanel/NetworkLanNicknameEditbox");
    assert_eq!(ui.editbox_text(&e).as_deref(), Some("Commander"));
    // the first click selects everything and takes the edit focus
    click(&mut ui, &e);
    assert!(ui.widget(&e).unwrap().flags & wf::EDIT_FOCUS != 0);
    ui.text_input("Haskb");
    assert_eq!(ui.editbox_text(&e).as_deref(), Some("Haskb"));
    assert!(ui.key_down(aa_ui::ui_controls::UiKey::Backspace));
    assert_eq!(ui.editbox_text(&e).as_deref(), Some("Hask"));
    // max_characters = AVar LANNicknameMaxLength (code default 18)
    ui.text_input("abcdefghijklmnopqrstuvwxyz");
    assert_eq!(ui.editbox_text(&e).unwrap().chars().count(), 18);
    // the caret cursor is shown while focused
    ui.update(0);
    assert!(ui.widget(&format!("{e}/Cursor")).unwrap().flags & 1 != 0);
    // Enter has no commit command here: not consumed (falls to hotkeys)
    assert!(!ui.key_down(aa_ui::ui_controls::UiKey::Enter));
    let effects = o.command(&mut ui, "OptionsOK").unwrap();
    assert!(effects.iter().any(|f| matches!(f, Effect::Nickname(n) if n.starts_with("Haskabc"))));
}
