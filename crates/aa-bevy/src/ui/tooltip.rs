//! Tooltips (KKC_UI's ToolTipGame / ToolTipShared widgets, `0x5C7950`):
//! * shown once the pointer rests on a widget with tooltip text for UVar
//!   UIToolTipsDelay seconds (0.5), while UVar UIToolTipsShow is set; a
//!   press hides it until the pointer moves to another widget;
//! * the text is the tooltip name in AVar TextFontIDSToolTipName
//!   (small_bold), then a blank line and the tooltip (`0x6388F0` /
//!   `0x638D90`), in the TGI `LeftLabel` (small, left, multiline);
//! * the widget is UI/Game/tooltip.tgi:ToolTipGame in the game (padding 18 x
//!   15) and UI/Shared/tooltip.tgi:ToolTipShared elsewhere (16 x 12);
//! * in the game the tooltip is anchored (game_interface.tgi
//!   anchored_tooltips: x 1024, y 521, width 346, `0x5EDED0`): right edge
//!   at x, bottom at y, the text wrapped to the width minus the padding;
//!   elsewhere it follows the pointer and wraps at UiTooltipsMaxWidth (0.5)
//!   of the screen width.
//!
//! The text comes from the widget's `tooltip_name` / `ToolTip` data, or
//! from the game HUD for the action and formation buttons.
use aa_ui::ui::Ui;
use aa_ui::widget::{Widget, WidgetClass as C};

const GAME_TIP: &str = "UI/Game/tooltip.tgi:ToolTipGame";
const SHARED_TIP: &str = "UI/Shared/tooltip.tgi:ToolTipShared";

#[derive(Default)]
pub struct Tooltips {
    /// The widget (path) the pointer rests on and since when (ms).
    hover: Option<(String, u64)>,
    /// A press hid the tooltip of this widget.
    suppressed: Option<String>,
    /// The connected tooltip widget (its top-level name) and its text.
    shown: Option<(&'static str, String)>,
}

fn top_name(file_node: &str) -> &'static str {
    if file_node == GAME_TIP { "ToolTipGame" } else { "ToolTipShared" }
}

/// Resolve a "#key" (a plain text stays).
fn text(ui: &Ui, s: &str) -> String {
    if s.starts_with('#') { ui.strings.resolve(s) } else { s.to_string() }
}

impl Tooltips {
    /// The tooltip (name, text) of the widget at `path` or its parents.
    fn source(ui: &Ui, path: &str, dynamic: &dyn Fn(&str) -> Option<(String, String)>) -> Option<(String, (String, String))> {
        let mut p = path.to_string();
        loop {
            if let Some(t) = dynamic(&p) {
                return Some((p, t));
            }
            if let Some(w) = ui.widget(&p) {
                if !w.tooltip_name.is_empty() || !w.tooltip.is_empty() {
                    return Some((p.clone(), (text(ui, &w.tooltip_name), text(ui, &w.tooltip))));
                }
            }
            let cut = p.rfind('/')?;
            p.truncate(cut);
            if p == "Root" || p.is_empty() {
                return None;
            }
        }
    }

    fn hide(&mut self, ui: &mut Ui) {
        if let Some((name, _)) = self.shown.take() {
            ui.disconnect(name);
            ui.layout();
        }
    }

    /// One frame: `pointer` in window pixels, `pressed` a button went down.
    pub fn frame(&mut self, ui: &mut Ui, pointer: Option<(f32, f32)>, pressed: bool, now_ms: u64, game: bool, dynamic: &dyn Fn(&str) -> Option<(String, String)>) {
        let show = ui.uvars.get_bool("UIToolTipsShow");
        let Some((x, y)) = pointer.filter(|_| show) else {
            self.hide(ui);
            self.hover = None;
            return;
        };
        // the tooltip itself never takes the pointer
        let picked = ui.pick(x, y).filter(|p| !p.contains("ToolTipGame") && !p.contains("ToolTipShared"));
        let found = picked.as_deref().and_then(|p| Self::source(ui, p, dynamic));
        let Some((path, (name, body))) = found.filter(|(_, (n, t))| !n.is_empty() || !t.is_empty()) else {
            self.hide(ui);
            self.hover = None;
            self.suppressed = None;
            return;
        };
        // the hovered widget and what it says (a button that changed its
        // command under the pointer counts as a new widget)
        let id = format!("{path}\u{1}{name}\u{1}{body}");
        if self.hover.as_ref().is_none_or(|(p, _)| *p != id) {
            self.hide(ui);
            self.hover = Some((id.clone(), now_ms));
            self.suppressed = None;
        }
        if pressed {
            self.hide(ui);
            self.suppressed = Some(id.clone());
        }
        if self.suppressed.as_deref() == Some(id.as_str()) {
            return;
        }
        let delay = Some(ui.uvars.get_f32("UIToolTipsDelay")).filter(|d| *d > 0.0).unwrap_or(0.5);
        let since = self.hover.as_ref().map_or(now_ms, |h| h.1);
        if (now_ms.saturating_sub(since) as f32) < delay * 1000.0 {
            return;
        }
        // 0x6388F0: <Font=name font>name<Font=default>, then 0x638D90:
        // a blank line and the tooltip
        let name_font = ui.avars.get("TextFontIDSToolTipName").unwrap_or("small_bold").to_string();
        let default_font = ui.avars.get("TextFontIDSDefault").unwrap_or("small").to_string();
        let mut full = String::new();
        if !name.is_empty() {
            full.push_str(&format!("<Font={name_font}>{name}<Font={default_font}>"));
        }
        if !body.is_empty() {
            if !full.is_empty() {
                full.push_str("\n\n");
            }
            full.push_str(&body);
        }
        let file = if game { GAME_TIP } else { SHARED_TIP };
        if self.shown.as_ref().is_some_and(|(n, t)| *n == top_name(file) && *t == full) {
            // follows the pointer outside the game
            if !game {
                self.place(ui, file, &full, (x, y), game);
            }
            return;
        }
        self.hide(ui);
        let mut d = Widget::new(C::TopLevel, file, true);
        d.add_child(Widget::new(C::Label, "LeftLabel", false));
        aa_ui::screens::show(ui, d);
        self.place(ui, file, &full, (x, y), game);
        self.shown = Some((top_name(file), full));
    }

    fn place(&mut self, ui: &mut Ui, file: &str, full: &str, (x, y): (f32, f32), game: bool) {
        let node = ui.store.node(file).ok();
        let pad = |k: &str, d: f32| node.as_ref().and_then(|n| n.float(k).ok()).unwrap_or(d);
        let (px, py) = (pad("padding_x", if game { 18.0 } else { 16.0 }), pad("padding_y", if game { 15.0 } else { 12.0 }));
        let g = ui.geo();
        let (sw, sh) = (ui.screen.width as f32, ui.screen.height as f32);
        let label_font = ui.store.node(&format!("{file}/LeftLabel/Label")).ok().map(|n| n.string_or_empty("font")).filter(|s| !s.is_empty()).unwrap_or_else(|| "small".into());
        // 0x735C00 measures in reference units (wrap width and result);
        // reference units -> window pixels: g.sx / g.sy
        let (l, t, w, h) = if game {
            // game_interface.tgi anchored tooltip (0x5EDED0)
            let gi = ui.store.node("UI/Game/game_interface.tgi:Game").ok();
            let av = |k: &str, d: f32| gi.as_ref().and_then(|n| n.float(k).ok()).unwrap_or(d);
            let (ax, ay, aw) = (av("anchored_tooltip_x", 1024.0), av("anchored_tooltip_y", 521.0), av("anchored_tooltip_width", 346.0));
            let (_, th, _) = ui.measure_text(&label_font, full, (aw - 2.0 * px).max(1.0));
            // R = x, L = x - width - 0.5, B = y, T = y - text - 2 padding - 0.5
            let width = aw + 0.5;
            let height = th + 2.0 * py + 0.5;
            ((ax - width) * g.sx, (ay - height) * g.sy, width * g.sx, height * g.sy)
        } else {
            let max = Some(ui.uvars.get_f32("UiTooltipsMaxWidth")).filter(|m| *m > 0.0).unwrap_or(0.5) * sw / g.sx;
            let (tw, _, _) = ui.measure_text(&label_font, full, 100_000.0);
            let wrap = tw.min(max - 2.0 * px).max(1.0);
            let (tw, th, _) = ui.measure_text(&label_font, full, wrap);
            let width = (tw + 2.0 * px) * g.sx;
            let height = (th + 2.0 * py) * g.sy;
            // below the pointer (the cursor is 32 px), kept on the screen
            let l = x.min(sw - width).max(0.0);
            let t = if y + 32.0 + height <= sh { y + 32.0 } else { (y - height).max(0.0) };
            (l, t, width, height)
        };
        let top = top_name(file);
        ui.with_widget(top, |wd| aa_ui::controls::set_bounds(&mut wd.root.outer, [l / g.sx, t / g.sy, w / g.sx, h / g.sy], 0x300));
        let label = format!("{top}/LeftLabel");
        ui.with_widget(&label, |wd| aa_ui::controls::set_bounds(&mut wd.root.outer, [px, py, w / g.sx - 2.0 * px, h / g.sy - 2.0 * py], 0x300));
        ui.set_label(&label, full);
        ui.layout();
    }
}
