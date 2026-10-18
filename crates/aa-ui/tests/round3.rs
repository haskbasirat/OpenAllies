//! Round-3 widget behaviour: message log, numeric edit box, edit box
//! commands, staging rows, text measure in the UI.

use aa_ui::fs::DataRoot;
use aa_ui::layout::Screen;
use aa_ui::message_log::message_log;
use aa_ui::state::widget_flag as wf;
use aa_ui::ui::{MouseButton, Ui, UiEvent};
use aa_ui::widget::{Widget, WidgetClass as C};

fn ui() -> Ui {
    Ui::new(DataRoot::workspace_default(), Screen { width: 1440, height: 900 })
}

fn label_text(w: &Widget) -> String {
    match &w.sub_ve.as_ref().unwrap().kind {
        aa_ui::ve::VeKind::Label(l) => l.text.clone(),
        _ => panic!(),
    }
}

#[test]
fn message_log_shows_fades_and_recycles_rows() {
    let mut ui = ui();
    let mut log = message_log("UI/Game/message_log.tgi:GameMessageLog", "UIMessageLogMaximumLines", true, false);
    ui.load_widget(&mut log, None);
    ui.connect(log);
    let path = "Root/GameMessageLog";
    ui.update(0);
    let st = |ui: &Ui| ui.widget(&format!("{path}/ItemPanel")).unwrap().children.len();
    assert_eq!(st(&ui), 0);
    // the HUD log keeps UIMessageLogMaximumLines (5) rows
    for i in 0..7 {
        let id = ui.messages.send(&ui.uvars.clone(), "GENERIC_EVENT", &format!("Message {i}"), None);
        assert!(id.is_some());
    }
    ui.update(100);
    let panel = ui.widget(&format!("{path}/ItemPanel")).unwrap();
    assert_eq!(panel.children.len(), 5);
    let texts: Vec<String> = panel.children.iter().map(|r| label_text(r.child("MessageWidgetText").unwrap())).collect();
    assert_eq!(texts, ["Message 2", "Message 3", "Message 4", "Message 5", "Message 6"]);
    let cmd = panel.children[4].child("MessageWidgetButton").unwrap().command.clone();
    assert_eq!(cmd.as_deref(), Some("MessageLogAction 7"));
    // UIMessageLogFadeTime 10000 ms: opaque, then a = 1 - d*0.001, then gone
    ui.update(100 + 10_000);
    assert_eq!(ui.widget(&format!("{path}/ItemPanel")).unwrap().children.len(), 5);
    ui.update(100 + 10_500);
    let a = match &ui.widget(&format!("{path}/ItemPanel")).unwrap().children[0].child("MessageWidgetText").unwrap().sub_ve.as_ref().unwrap().kind {
        aa_ui::ve::VeKind::Label(l) => l.color[3],
        _ => panic!(),
    };
    assert_eq!(a, 1.0f32 - 500.0f32 * 0.001f32);
    ui.update(100 + 11_100);
    ui.update(100 + 11_200);
    assert_eq!(ui.widget(&format!("{path}/ItemPanel")).unwrap().children.len(), 0);
}

#[test]
fn filtered_message_types_are_dropped() {
    let mut ui = ui();
    let mut vars = ui.uvars.clone();
    vars.set_bool("UIMessageFilterGeneric", false);
    assert_eq!(ui.messages.send(&vars, "GENERIC_EVENT", "hidden", None), None);
    assert!(ui.messages.send(&vars, "HERO_POSITIVE", "shown", None).is_some());
}

#[test]
fn numeric_edit_box_steps_and_clamps() {
    let mut ui = ui();
    aa_ui::screens::main_menu(&mut ui);
    ui.show_message_box(
        aa_ui::message_box::SHARED,
        aa_ui::message_box::MessageBoxRequest { text: "Amount".into(), buttons: vec![("Ok".into(), String::new())] },
    );
    let ne = "Root/SharedMessageBox/MessageBoxNumericEditbox";
    ui.with_widget("SharedMessageBox/MessageBoxNumericEditbox", |w| w.set_visible(true));
    ui.with_widget("SharedMessageBox/MessageBoxNumericEditbox/Editbox", |w| {
        if let Some(e) = w.editbox_mut() {
            e.integer_max = 3;
        }
    });
    ui.layout();
    ui.editbox_set_text(&format!("{ne}/Editbox"), "2");
    let up = format!("{ne}/UpButton");
    let r = ui.widget(&up).unwrap().root.outer.rect;
    let (x, y) = ((r.l + r.r) / 2.0, (r.t + r.b) / 2.0);
    for _ in 0..3 {
        ui.mouse_down(x, y, MouseButton::Left);
        ui.mouse_up(x, y, MouseButton::Left);
    }
    assert_eq!(ui.editbox_text(&format!("{ne}/Editbox")).as_deref(), Some("3"));
    let up_label = label_text(ui.widget(&format!("{up}/Client")).unwrap());
    assert_eq!(up_label, "+1");
}

#[test]
fn edit_box_clipboard_commands() {
    let mut ui = ui();
    let mut root = Widget::new(C::TopLevel, "UI/Shared/options_dialog.tgi:OptionsDialog", false);
    let mut page = Widget::new(C::Panel, "NetworkPage", false);
    let mut lan = Widget::new(C::Panel, "LANOptionsPanel", false);
    lan.add_child(Widget::composite(C::Editbox, "NetworkLanNicknameEditbox", false));
    page.add_child(lan);
    root.add_child(page);
    ui.load_widget(&mut root, None);
    ui.connect(root);
    let e = "Root/OptionsDialog/NetworkPage/LANOptionsPanel/NetworkLanNicknameEditbox";
    ui.editbox_set_text(e, "Commander");
    let r = ui.widget(e).unwrap().root.outer.rect;
    ui.mouse_down((r.l + r.r) / 2.0, (r.t + r.b) / 2.0, MouseButton::Left);
    assert!(ui.widget(e).unwrap().flags & wf::EDIT_FOCUS != 0);
    assert!(ui.hotkey("ctrl-a"));
    assert!(ui.editbox_command("EditboxCut"));
    let ev = ui.take_events();
    assert!(ev.iter().any(|x| matches!(x, UiEvent::Clipboard { text } if text == "Commander")));
    assert_eq!(ui.editbox_text(e).as_deref(), Some(""));
    ui.paste("Haskb\nsecond line");
    assert_eq!(ui.editbox_text(e).as_deref(), Some("Haskb"));
}

#[test]
fn ui_text_measure_uses_the_font_tables() {
    let mut ui = Ui::new(DataRoot::workspace_default(), Screen { width: 2560, height: 1440 });
    // small = A&A (GDI+) at floor(12*1440/768+0.5) = 23 px
    let (w, h, rows) = ui.measure_text("small", "Quit Axis & Allies?", 0.0);
    assert!(w > 0.0 && h > 0.0);
    assert_eq!(rows, 1);
    // wrapping counts two rows per wrap (native)
    let (_, h2, rows2) = ui.measure_text("small", "Quit Axis & Allies?", w * 0.6);
    assert_eq!(rows2, 3);
    assert!(h2 > h);
    // the ellipsis keeps a prefix plus "..."
    let (t, s) = ui.ellipsis_text("small", "Supercalifragilistic", 40.0);
    assert!(t && s.ends_with("...") && s.len() < 23);
}

#[test]
fn edit_box_scrolls_to_the_caret_and_highlights_the_selection() {
    let mut ui = ui();
    let mut root = Widget::new(C::TopLevel, "UI/Shared/options_dialog.tgi:OptionsDialog", false);
    let mut page = Widget::new(C::Panel, "NetworkPage", false);
    let mut lan = Widget::new(C::Panel, "LANOptionsPanel", false);
    lan.add_child(Widget::composite(C::Editbox, "NetworkLanNicknameEditbox", false));
    page.add_child(lan);
    root.add_child(page);
    ui.load_widget(&mut root, None);
    ui.connect(root);
    let e = "Root/OptionsDialog/NetworkPage/LANOptionsPanel/NetworkLanNicknameEditbox";
    ui.editbox_set_max(e, 0);
    ui.editbox_set_text(e, "");
    ui.layout();
    let r = ui.widget(e).unwrap().root.outer.rect;
    ui.mouse_down((r.l + r.r) / 2.0, (r.t + r.b) / 2.0, MouseButton::Left);
    ui.mouse_up((r.l + r.r) / 2.0, (r.t + r.b) / 2.0, MouseButton::Left);
    ui.text_input("Commander of the Allied Expeditionary Force in Europe");
    ui.update(1000);
    ui.update(1001);
    let label = ui.widget(&format!("{e}/Label")).unwrap();
    let lrect = label.sub_ve.as_ref().unwrap().layout.outer.rect;
    let scroll = match &label.sub_ve.as_ref().unwrap().kind {
        aa_ui::ve::VeKind::Label(l) => l.scroll_px,
        _ => panic!(),
    };
    assert!(scroll > 0.0, "scroll {scroll}");
    // 0x5CF770: the caret glyph (the end entry) ends inside the visible width
    let cursor = ui.widget(&format!("{e}/Cursor")).unwrap();
    assert!(cursor.root.outer.rect.l <= lrect.r + 1.0, "{:?} {:?}", cursor.root.outer.rect, lrect);
    // drawn glyph quads stay inside the label rect, the last one at its right end
    let dl = ui.draw_list();
    let text = dl
        .commands
        .iter()
        .find_map(|c| match &c.kind {
            aa_ui::draw::DrawKind::Text(t) if c.widget.ends_with("NetworkLanNicknameEditbox/Label") => Some(t.clone()),
            _ => None,
        })
        .unwrap();
    assert!(text.glyphs.iter().all(|g| g.rect.l >= lrect.l.floor() && g.rect.r <= lrect.r.ceil()));
    // ctrl-a: the label carries the native highlight markup and the drawn
    // quads gain the background (highlight) list
    assert!(ui.hotkey("ctrl-a"));
    ui.update(1100);
    assert_eq!(label_text(ui.widget(&format!("{e}/Label")).unwrap()), "<h_on>Commander of the Allied Expeditionary Force in Europe<h_off>");
    let dl = ui.draw_list();
    let n = dl
        .commands
        .iter()
        .filter_map(|c| match &c.kind {
            aa_ui::draw::DrawKind::Text(t) if c.widget.ends_with("NetworkLanNicknameEditbox/Label") => Some(t.glyphs.iter().filter(|g| g.highlight).count()),
            _ => None,
        })
        .sum::<usize>();
    assert!(n > 10, "{n}");
}
