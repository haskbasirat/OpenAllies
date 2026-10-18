//! Layout checks against the original game's screenshots
//! (`qa-output/original-ui/*.png`, 2560x1440).
//!
//! `research/phase8-ui/screenshot_match.py` rasterises every textured quad of
//! the draw lists written by `examples/dump_screens.rs` and searches the
//! integer pixel offset (within +-6 px) that best matches the screenshot.
//! These tests (1) rebuild every screen and require that each quad the
//! script found at offset (0,0) with a low error still has exactly the
//! recorded rect, and (2) check label text placement against the white ink
//! measured in the screenshots.

use aa_ui::draw::DrawKind;
use aa_ui::fs::DataRoot;
use aa_ui::layout::{Rect, Screen};
use aa_ui::screens;
use aa_ui::ui::{MouseButton, Ui, UiEvent};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

fn evidence() -> Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-ui/screenshot-match.json");
    serde_json::from_str(&std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))).unwrap()
}

fn build(ui: &mut Ui, name: &str) {
    let skin = if name.starts_with("game_") { Some("USA") } else { None };
    ui.set_skin(skin, DataRoot::workspace_skins());
    match name {
        "main_menu" => screens::main_menu(ui),
        "single_player" => screens::single_player_menu(ui),
        "staging" => { aa_ui::staging::Staging::show(ui, aa_ui::staging::placeholder_state()); }
        "loading" => launch_loading(ui),
        "game_hud" => screens::game_hud(ui),
        "game_regiment" => screens::game_hud_scene(ui, aa_ui::hud::PlaceholderScene::Regiment),
        "game_corps" => screens::game_hud_scene(ui, aa_ui::hud::PlaceholderScene::Corps),
        "game_recruit" => screens::game_hud_scene(ui, aa_ui::hud::PlaceholderScene::Recruit),
        "game_menu" => screens::game_menu(ui),
        "game_quit" => screens::game_quit(ui),
        other => panic!("{other}"),
    }
}

/// 03-launch.png: a custom game (no objectives list), USA local player,
/// finished loading (same as examples/dump_screens.rs).
fn launch_loading(ui: &mut Ui) {
    screens::loading_screen(ui, false);
    let info = screens::LoadingInfo {
        map_name: "V10 Graphics Switch QA".into(),
        map_description: ui.strings.resolve("#loading_screen_Label_text_2"),
        kind: 1,
        local_player: true,
        nation: Some("USA".into()),
        ..Default::default()
    };
    screens::loading_refresh(ui, &info);
    screens::loading_finished(ui);
}

fn rect4(v: &Value) -> [f32; 4] {
    let a = v.as_array().unwrap();
    [0, 1, 2, 3].map(|i| a[i].as_f64().unwrap() as f32)
}

#[test]
fn quads_land_where_the_original_drew_them() {
    let ev = evidence();
    let mut ui = Ui::new(DataRoot::workspace_default(), Screen { width: 2560, height: 1440 });
    let minimum: HashMap<&str, usize> =
        [("main_menu", 13), ("single_player", 9), ("staging", 260), ("loading", 42), ("game_hud", 29), ("game_regiment", 60), ("game_corps", 44), ("game_recruit", 45), ("game_menu", 65), ("game_quit", 103)]
            .into();
    for screen in ev["screens"].as_array().unwrap() {
        let name = screen["name"].as_str().unwrap();
        build(&mut ui, name);
        let dl = ui.draw_list();
        // occurrences of (widget, ve, texture) in draw order
        let mut ours: HashMap<(String, String, String), Vec<Rect>> = HashMap::new();
        for c in &dl.commands {
            if let DrawKind::Quad(q) = &c.kind {
                if let Some(t) = &q.texture {
                    ours.entry((c.widget.clone(), c.ve.clone(), t.clone())).or_default().push(q.rect);
                }
            }
        }
        let mut seen: HashMap<(String, String, String), usize> = HashMap::new();
        let mut verified = 0;
        for q in screen["quads"].as_array().unwrap() {
            let key = (
                q["widget"].as_str().unwrap().to_string(),
                q["ve"].as_str().unwrap().to_string(),
                q["texture"].as_str().unwrap().to_string(),
            );
            let k = seen.entry(key.clone()).or_default();
            let idx = *k;
            *k += 1;
            if q["status"] != "matched" || q["best_offset"] != serde_json::json!([0, 0]) {
                continue;
            }
            if q["best_error"].as_f64().unwrap() > 30.0 {
                continue;
            }
            let recorded = rect4(&q["rect"]);
            let got = ours.get(&key).and_then(|v| v.get(idx)).unwrap_or_else(|| panic!("{name}: missing {key:?}"));
            for (a, b) in got.to_array().iter().zip(recorded) {
                assert!((a - b).abs() < 1e-3, "{name} {key:?}: {got:?} != {recorded:?}");
            }
            verified += 1;
        }
        eprintln!("{name}: {verified} screenshot-verified quads");
        assert!(verified >= minimum[name], "{name}: only {verified} screenshot-verified quads");
    }
}

#[test]
fn label_text_lands_on_the_original_ink() {
    let ev = evidence();
    let mut ui = Ui::new(DataRoot::workspace_default(), Screen { width: 2560, height: 1440 });
    let mut checked = 0;
    for screen in ev["screens"].as_array().unwrap() {
        let name = screen["name"].as_str().unwrap();
        if !matches!(name, "main_menu" | "single_player" | "loading") {
            continue;
        }
        build(&mut ui, name);
        let dl = ui.draw_list();
        for t in screen["texts"].as_array().unwrap() {
            let Some(ink) = t.get("ink") else { continue };
            let text = t["text"].as_str().unwrap();
            // labels whose text comes from runtime data or special glyphs
            if text.starts_with('\u{a9}') || text == "Map Name" || text.starts_with("Status") {
                continue;
            }
            let ink = rect4(ink);
            let cmd = dl
                .commands
                .iter()
                .find_map(|c| match &c.kind {
                    DrawKind::Text(x) if c.widget == t["widget"].as_str().unwrap() && x.text == text => Some(x),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{name}: no text {text}"));
            // first row of glyph quads (rdraw.rs): the cell top-left of the
            // first glyph is the pen
            let first = cmd.glyphs.iter().find(|g| !g.highlight).unwrap();
            let (px, py) = (first.rect.l - first.cell[0], first.rect.t - first.cell[1]);
            let row: Vec<_> =
                cmd.glyphs.iter().filter(|g| !g.highlight && g.rect.t - g.cell[1] == py && g.c != ' ' as u32).collect();
            // the ink of the first glyph begins within 2 px of the pen, and
            // below the cell top (no vertical centring)
            let dx = ink[0] - px;
            let dy = ink[1] - py;
            assert!((0.0..=2.0).contains(&dx), "{name} '{text}': ink x {} vs pen {}", ink[0], px);
            assert!((0.0..=8.0).contains(&dy), "{name} '{text}': ink y {} vs row top {}", ink[1], py);
            // the row's right end (the last glyph's advance) within 4 px of the ink
            let right = row.last().unwrap().rect.r;
            assert!((right - ink[2]).abs() <= 4.0, "{name} '{text}': row end {right} vs ink end {}", ink[2]);
            checked += 1;
        }
    }
    assert!(checked >= 15, "only {checked} labels checked");
}

#[test]
fn reference_resolution_uses_authored_coordinates() {
    let mut ui = Ui::new(DataRoot::workspace_default(), Screen { width: 1024, height: 768 });
    screens::main_menu(&mut ui);
    let panel = ui.root.find("MainMenuInterface/MainMenu").unwrap();
    assert_eq!(panel.root.outer.rect, Rect::new(783.0, 8.0, 1020.0, 766.0));
    let sp = panel.child("MainMenuButtonSinglePlayer").unwrap();
    assert_eq!(sp.root.outer.rect, Rect::new(809.0, 212.0, 991.0, 258.0));
    // client label: L=13 T=12 R=13r B=8b inside the button
    let client = sp.child("Client").unwrap();
    assert_eq!(client.root.outer.rect, Rect::new(822.0, 224.0, 978.0, 250.0));
}

#[test]
fn resolution_folders_follow_the_screen_width() {
    let mut ui = Ui::new(DataRoot::workspace_default(), Screen { width: 1280, height: 960 });
    screens::main_menu(&mut ui);
    let dl = ui.draw_list();
    let top = dl
        .commands
        .iter()
        .find_map(|c| match &c.kind {
            DrawKind::Quad(q) if q.texture.as_deref().is_some_and(|t| t.ends_with("MainMenuTop.TGA")) => q.texture.clone(),
            _ => None,
        })
        .unwrap();
    assert_eq!(top, "/UI/1280/Default/Menus/MainMenu/MainMenuTop.TGA");
}

#[test]
fn main_menu_buttons_fire_their_native_commands() {
    let mut ui = Ui::new(DataRoot::workspace_default(), Screen { width: 2560, height: 1440 });
    screens::main_menu(&mut ui);
    let path = "Root/MainMenuInterface/MainMenu/MainMenuButtonSinglePlayer";
    let r = ui.widget(path).unwrap().root.outer.rect;
    let (x, y) = ((r.l + r.r) / 2.0, (r.t + r.b) / 2.0);
    ui.mouse_move(x, y);
    assert!(ui.widget(path).unwrap().flags & aa_ui::state::widget_flag::FOCUSED != 0);
    ui.layout();
    // focused + unpressed shows menubuttonfocused
    let dl = ui.draw_list();
    assert!(dl.commands.iter().any(|c| c.widget == path
        && matches!(&c.kind, DrawKind::Quad(q) if q.texture.as_deref().is_some_and(|t| t.ends_with("menubuttonfocused.TGA")))));
    ui.mouse_down(x, y, MouseButton::Left);
    ui.layout();
    // pressed: the client takes pressed_client bounds (17,14,13r,6b)
    let client = ui.widget(&format!("{path}/Client")).unwrap().root.outer.rect;
    assert_eq!(client.l, r.l + 17.0 * 2.5);
    ui.mouse_up(x, y, MouseButton::Left);
    let events = ui.take_events();
    // the press plays the button's set_sound (ButtonWidget template)
    assert!(events.iter().any(|e| matches!(e, UiEvent::Sound { .. })));
    let commands: Vec<UiEvent> = events.into_iter().filter(|e| matches!(e, UiEvent::Command { .. })).collect();
    assert_eq!(commands, vec![UiEvent::Command { widget: path.into(), command: "RequestSinglePlayer".into() }]);
}
