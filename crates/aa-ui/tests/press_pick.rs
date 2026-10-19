//! KKC_UI 0x5C3BC0's press pick (press_pick.rs) on the in-game HUD tree
//! built from the original UI data: the world is reached only where no
//! drawn HUD element covers the pixel, and VWorld is the Game interface's
//! head child (0x60BAE0 -> 0x43B490).

use aa_ui::fs::DataRoot;
use aa_ui::hud::{GAME, Hud, HudWorld, PlaceholderScene, PlaceholderWorld};
use aa_ui::layout::Screen;
use aa_ui::press_pick::{PressTarget, VWORLD_PATH};
use aa_ui::ui::Ui;

fn hud(width: i32, height: i32) -> (Ui, Hud) {
    let mut ui = Ui::new(DataRoot::workspace_default(), Screen { width, height });
    ui.set_skin(Some("USA"), DataRoot::workspace_skins());
    let world = PlaceholderWorld { scene: PlaceholderScene::Ingame };
    let mut hud = Hud::show(&mut ui, &world);
    for t in [0u64, 1000] {
        ui.update(t);
        hud.update(&mut ui, &world as &dyn HudWorld);
    }
    let _ = ui.draw_list();
    (ui, hud)
}

fn rect(ui: &Ui, path: &str) -> [f32; 4] {
    let r = ui.widget(path).unwrap_or_else(|| panic!("{path}")).root.outer.rect;
    [r.l, r.t, r.r, r.b]
}

#[test]
fn the_world_is_picked_only_outside_drawn_hud_elements() {
    let (ui, _hud) = hud(1024, 768);
    let game = ui.widget(GAME).unwrap();
    // 0x43B490: Game vt+0xD4, the head of its key-0 group
    let first_key0 = game.children.iter().position(|c| c.key >= 0.0).unwrap();
    assert!(game.children[first_key0].name.eq_ignore_ascii_case("VWorld"));
    assert_eq!(game.children.iter().filter(|c| c.name.eq_ignore_ascii_case("VWorld")).count(), 1);
    let v = rect(&ui, VWORLD_PATH);
    assert_eq!(v, rect(&ui, GAME));
    // the middle of the screen is world
    assert_eq!(ui.press_target_5c3bc0(512, 300), PressTarget::VWorld);
    // the control panel strip at the bottom is HUD
    let cp = rect(&ui, &format!("{GAME}/ControlPanel"));
    let (x, y) = (((cp[0] + cp[2]) / 2.0) as i16, (cp[3] - 4.0) as i16);
    match ui.press_target_5c3bc0(x, y) {
        PressTarget::Widget { path, .. } => assert!(path.starts_with(&format!("{GAME}/ControlPanel")), "{path}"),
        other => panic!("control panel pixel picked {other:?}"),
    }
    // the minimap answers vt+0x90 true
    let mm = rect(&ui, &format!("{GAME}/ControlPanel/GameMiniMap"));
    let centre = (((mm[0] + mm[2]) / 2.0) as i16, ((mm[1] + mm[3]) / 2.0) as i16);
    let picked = ui.press_target_5c3bc0(centre.0, centre.1);
    assert!(matches!(picked, PressTarget::MiniMap { .. }), "{picked:?}");
    // the economy bar is HUD
    let bar = rect(&ui, &format!("{GAME}/EconomyBar"));
    let bar_pixel = (((bar[0] + bar[2]) / 2.0) as i16, ((bar[1] + bar[3]) / 2.0) as i16);
    assert!(matches!(ui.press_target_5c3bc0(bar_pixel.0, bar_pixel.1), PressTarget::Widget { .. }));
    // outside the screen nothing is hit
    assert_eq!(ui.press_target_5c3bc0(-5, -5), PressTarget::None);
}

#[test]
fn a_hidden_widget_does_not_shadow_the_world() {
    let (mut ui, _hud) = hud(1024, 768);
    let pause = format!("{GAME}/DisplayPause");
    let r = rect(&ui, &pause);
    let (x, y) = (((r[0] + r[2]) / 2.0) as i16, ((r[1] + r[3]) / 2.0) as i16);
    // DisplayPause is hidden while the simulation runs (612010)
    ui.with_widget(&pause, |w| w.set_visible(false));
    ui.layout();
    assert_eq!(ui.press_target_5c3bc0(x, y), PressTarget::VWorld);
}
