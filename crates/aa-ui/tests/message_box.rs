//! Message box (`_Display` 0x66DF90) layout and button routing.

use aa_ui::fs::DataRoot;
use aa_ui::layout::Screen;
use aa_ui::screens;
use aa_ui::ui::{MouseButton, Ui, UiEvent};

fn click(ui: &mut Ui, path: &str) {
    let r = ui.widget(path).unwrap_or_else(|| panic!("{path}")).root.outer.rect;
    let (x, y) = ((r.l + r.r) / 2.0, (r.t + r.b) / 2.0);
    ui.mouse_down(x, y, MouseButton::Left);
    ui.mouse_up(x, y, MouseButton::Left);
}

#[test]
fn quit_confirmation_layout_and_buttons() {
    let mut ui = Ui::new(DataRoot::workspace_default(), Screen { width: 1024, height: 768 });
    screens::main_menu_quit(&mut ui);
    let mb = ui.widget("Root/SharedMessageBox").unwrap();
    let r = mb.root.outer.rect;
    // two 182-wide buttons 236 apart (182 + spacing 54) plus borders 2*53
    assert_eq!(r.r - r.l, 524.0);
    // centred in the 1024 x 768 reference space
    assert_eq!(r.l, 512.0 - 262.0);
    assert!(((r.t + r.b) / 2.0 - 384.0).abs() < 0.5);
    let ok = ui.widget("Root/SharedMessageBox/MessageBoxButton").unwrap().root.outer.rect;
    let cancel = ui.widget("Root/SharedMessageBox/MessageBoxButton#1").unwrap().root.outer.rect;
    assert_eq!(ok.l, r.l + 53.0);
    assert_eq!(cancel.l - ok.l, 236.0);
    assert!(ui.widget("Root/SharedMessageBox/MessageBoxButton#2").is_some_and(|w| w.flags & 1 == 0));
    // the main menu is not pickable while the box is up
    let sp = ui.widget("Root/MainMenuInterface/MainMenu/MainMenuButtonSinglePlayer").unwrap().root.outer.rect;
    ui.mouse_move((sp.l + sp.r) / 2.0, (sp.t + sp.b) / 2.0);
    assert!(ui.pick((sp.l + sp.r) / 2.0, (sp.t + sp.b) / 2.0).is_none());
    // Cancel closes without a command; Ok runs Quit
    click(&mut ui, "Root/SharedMessageBox/MessageBoxButton#1");
    assert!(ui.widget("Root/SharedMessageBox").is_none());
    assert!(ui.take_events().iter().all(|e| !matches!(e, UiEvent::Command { command, .. } if command == "Quit")));
    screens::quit_confirm(&mut ui, false);
    click(&mut ui, "Root/SharedMessageBox/MessageBoxButton");
    assert!(ui.take_events().iter().any(|e| matches!(e, UiEvent::Command { command, .. } if command == "Quit")));
}
