//! Round-2 screens at a given size (default 1440x900): the options dialog
//! on each tab over the main menu, a drop-down opened, and the credits
//! after a few seconds. Writes research/phase8-ui/drawlists/r2_*.json.
use aa_ui::fs::DataRoot;
use aa_ui::layout::Screen;
use aa_ui::options::{self, Options};
use aa_ui::screens;
use aa_ui::ui::{MouseButton, Ui};
use std::path::PathBuf;

fn write(ui: &mut Ui, out: &PathBuf, name: &str) {
    let dl = ui.draw_list();
    std::fs::write(out.join(format!("r2_{name}.json")), serde_json::to_string_pretty(&dl).unwrap()).unwrap();
    println!("{name}: {} commands", dl.commands.len());
}

fn click(ui: &mut Ui, path: &str) {
    let r = ui.widget(path).unwrap_or_else(|| panic!("{path}")).root.outer.rect;
    let (x, y) = ((r.l + r.r) / 2.0, (r.t + r.b) / 2.0);
    ui.mouse_down(x, y, MouseButton::Left);
    ui.mouse_up(x, y, MouseButton::Left);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (w, h) = if args.len() >= 3 { (args[1].parse().unwrap(), args[2].parse().unwrap()) } else { (1440, 900) };
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-ui/drawlists");
    std::fs::create_dir_all(&out).unwrap();
    let mut ui = Ui::new(DataRoot::workspace_default(), Screen { width: w, height: h });
    screens::main_menu(&mut ui);
    let mut opt = Options::new(&mut ui);
    opt.video_modes = vec![(w, h, 32)];
    opt.current_mode = (w, h);
    opt.nickname = "Commander".into();
    opt.connection = "Broadband".into();
    opt.show(&mut ui, "");
    for d in ui.root.all_diagnostics().iter().take(30) {
        println!("  diag {d}");
    }
    for d in &opt.diagnostics {
        println!("  options {d}");
    }
    for tab in ["Gameplay", "Video", "Audio", "Network", "MetaGame"] {
        let effects = opt.command(&mut ui, &format!("OptionsPanelSelectTab {tab}"));
        assert!(effects.is_some());
        ui.update(0);
        write(&mut ui, &out, &format!("options_{}", tab.to_lowercase()));
    }
    opt.command(&mut ui, "OptionsPanelSelectTab Video");
    click(&mut ui, &format!("{}/VideoPage/ResolutionOptionsPanel/DropDownGraphicsPreset", options::PATH));
    ui.update(0);
    write(&mut ui, &out, "options_video_dropdown");
    for e in ui.take_events() {
        println!("  event {e:?}");
    }
    let mut credits = aa_ui::credits::Credits::show(&mut ui, 0);
    println!("credits: {} nodes, diagnostics {:?}", credits.nodes.len(), credits.diagnostics);
    for t in [0u64, 4000, 20000] {
        credits.update(&mut ui, t);
        write(&mut ui, &out, &format!("credits_{}s", t / 1000));
        println!("  labels {:?}", credits.active_labels().iter().take(4).collect::<Vec<_>>());
    }
}
