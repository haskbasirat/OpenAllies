//! Build each round-1 screen at 2560x1440 and write its draw list and widget
//! rects to research/phase8-ui/drawlists/ (input of screenshot_match.py).
use aa_ui::fs::DataRoot;
use aa_ui::layout::Screen;
use aa_ui::screens;
use aa_ui::ui::Ui;
use std::path::PathBuf;

/// 03-launch.png: a custom game (no objectives list), USA local player,
/// finished loading. The map name/description are the QA map's own texts
/// as shown in the screenshot.
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (w, h) = if args.len() >= 3 { (args[1].parse().unwrap(), args[2].parse().unwrap()) } else { (2560, 1440) };
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-ui/drawlists");
    std::fs::create_dir_all(&out_dir).unwrap();
    let mut ui = Ui::new(DataRoot::workspace_default(), Screen { width: w, height: h });
    println!("templates {} strings {} fonts {}", ui.store.template_count(), ui.strings.len(), ui.fonts.len());
    let builds: Vec<(&str, &str, fn(&mut Ui))> = vec![
        ("main_menu", "00-start.png", screens::main_menu),
        ("single_player", "01-singleplayer.png", screens::single_player_menu),
        ("staging", "02-custom.png", |ui| { aa_ui::staging::Staging::show(ui, aa_ui::staging::placeholder_state()); }),
        ("loading", "03-launch.png", launch_loading),
        ("game_hud", "04-ingame.png", screens::game_hud),
        ("game_regiment", "05-regiment.png", |ui| screens::game_hud_scene(ui, aa_ui::hud::PlaceholderScene::Regiment)),
        ("game_corps", "06-corps.png", |ui| screens::game_hud_scene(ui, aa_ui::hud::PlaceholderScene::Corps)),
        ("game_recruit", "07-recruit.png", |ui| screens::game_hud_scene(ui, aa_ui::hud::PlaceholderScene::Recruit)),
        ("game_menu", "13-quit.png", screens::game_menu),
        ("game_quit", "13-quit.png", screens::game_quit),
    ];
    let mut index = Vec::new();
    for (name, shot, build) in builds {
        // the in-game screenshots show the USA skin
        let skin = if name.starts_with("game_") { Some("USA") } else { None };
        ui.set_skin(skin, DataRoot::workspace_skins());
        build(&mut ui);
        let dl = ui.draw_list();
        let mut rects = Vec::new();
        ui.root.walk("", &mut |p, wd| {
            rects.push(serde_json::json!({"widget": p, "class": format!("{:?}", wd.class), "rect": wd.root.outer.rect, "shown": wd.flags & 2 != 0, "loaded": wd.loaded}));
        });
        let diags = ui.root.all_diagnostics();
        println!("{name}: {} commands, {} widgets, {} diagnostics", dl.commands.len(), rects.len(), diags.len());
        for d in diags.iter().take(25) {
            println!("   {d}");
        }
        let file = format!("{name}_{w}x{h}.json");
        std::fs::write(out_dir.join(&file), serde_json::to_string_pretty(&dl).unwrap()).unwrap();
        std::fs::write(
            out_dir.join(format!("{name}_{w}x{h}_widgets.json")),
            serde_json::to_string_pretty(&serde_json::json!({"widgets": rects, "diagnostics": diags})).unwrap(),
        )
        .unwrap();
        index.push(serde_json::json!({"name": name, "drawlist": file, "screenshot": shot}));
    }
    if (w, h) == (2560, 1440) {
        std::fs::write(out_dir.join("index.json"), serde_json::to_string_pretty(&index).unwrap()).unwrap();
    }
}
