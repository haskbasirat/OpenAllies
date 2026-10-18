//! `KKC_MessageBox` (`.\Source\K2ui\Shared\MessageBox.cpp`).
//!
//! * Init `0x66D920`: children MessageBoxLabel (label), MessageBoxEditbox
//!   and MessageBoxNumericEditbox (hidden), and three hidden
//!   MessageBoxButton push buttons whose client is the label
//!   MessageBoxButtonLabel (`0x5F33C0`).
//! * LoadData `0x66D660`: `minimum_size`, `maximum_size`, `widget_spacing`,
//!   `border_size`, and the label's `justify` (+0xF0).
//! * `_Display` `0x66DF90`: size the box to the maximum, set and measure the
//!   text (wrapped at the maximum width), justify CENTER for one row else
//!   the authored justify, place the label at (border, border), shrink the
//!   label if the text is taller than the room left by the buttons, place up
//!   to three buttons (`0x66D7C0`: one centred under the text, otherwise
//!   spread at `(text_w - button_w) / (n - 1)` but at least `button_w +
//!   spacing` apart), clamp the box to the minimum/maximum size, centre a
//!   single button, set the label width to the box width minus the borders
//!   and centre the box in the interface reference space.
//!
//! The text measure (label vtable+0xCC) is GetTextExtent through the
//! reference-unit wrapper `0x735C00` (`rtext.rs`, native-proven) over the
//! UI font tables (`fonts.rs`). Buttons fire `MessageBoxButtonAction <i>` (0x66D7C0), which
//! [`crate::ui::Ui`] maps to the request's command for that button.

use crate::geometry as g;
use crate::ui::Ui;
use crate::ve::{Justify, VeKind};
use crate::widget::{Widget, WidgetClass as C};

/// One `_Display` request.
#[derive(Clone, Debug, PartialEq)]
pub struct MessageBoxRequest {
    pub text: String,
    /// (label, command) per button, at most three are shown.
    pub buttons: Vec<(String, String)>,
}

/// The shared message box data path (`0x5C0EE0` picks it outside the game,
/// metagame and editor).
pub const SHARED: &str = "UI/Shared/message_box.tgi:SharedMessageBox";
/// The in-game message box.
pub const GAME: &str = "UI/Game/message_box.tgi:GameMessageBox";

/// `0x66D920`.
pub fn build(path: &str) -> Widget {
    let mut mb = Widget::new(C::TopLevel, path, false);
    mb.add_child(Widget::new(C::Label, "MessageBoxLabel", false));
    let mut edit = Widget::composite(C::Editbox, "MessageBoxEditbox", false);
    edit.set_visible(false);
    mb.add_child(edit);
    let mut num = Widget::composite(C::NumericEditbox, "MessageBoxNumericEditbox", false);
    num.set_visible(false);
    mb.add_child(num);
    for i in 0..3 {
        let mut b = Widget::new(C::PushButton, "MessageBoxButton", false);
        b.client_name = "MessageBoxButtonLabel".into();
        b.add_child(Widget::new(C::Label, "MessageBoxButtonLabel", false));
        b.command = Some(format!("MessageBoxButtonAction {i}"));
        b.set_visible(false);
        mb.add_child(b);
    }
    mb
}

fn set_label_text(w: &mut Widget, text: &str) {
    if let Some(crate::ve::ViewElement { kind: VeKind::Label(l), .. }) = &mut w.sub_ve {
        l.text = text.to_string();
    }
}

impl Ui {
    /// Show a message box (`_Display` 0x66DF90) on top of Root, modal.
    pub fn show_message_box(&mut self, path: &str, req: MessageBoxRequest) {
        let mut mb = build(path);
        self.load_widget(&mut mb, None);
        let node = self.store.node(path).ok();
        let p2 = |k: &str, d: [f32; 2]| node.as_ref().and_then(|n| n.point2(k).ok()).unwrap_or(d);
        let f = |k: &str| node.as_ref().and_then(|n| n.float(k).ok()).unwrap_or(0.0);
        let min = p2("minimum_size", [0.0, 0.0]);
        let max = p2("maximum_size", [1024.0, 768.0]);
        let spacing = f("widget_spacing");
        let border = f("border_size");
        let label_justify = mb
            .child("MessageBoxLabel")
            .and_then(|l| match &l.sub_ve {
                Some(crate::ve::ViewElement { kind: VeKind::Label(d), .. }) => Some(d.justify),
                _ => None,
            })
            .unwrap_or(Justify::Center);
        let name = mb.name.clone();
        self.disconnect(&name);
        self.connect(mb);
        self.modal = Some(name.clone());
        self.message_box = Some(req.clone());
        let (screen, reference, p) = (self.screen, self.reference, self.precision);
        let two_b = border + border;
        let path_mb = name.clone();

        // SetSize(max)
        self.with_widget(&path_mb, |w| g::set_size(w, max[0], max[1]));
        self.layout();
        // label text and measure
        let label_path = format!("{path_mb}/MessageBoxLabel");
        self.with_widget(&label_path, |w| set_label_text(w, &req.text));
        let (tw, th, rows) = self.measure_label(&label_path, &req.text, max[0]);
        self.with_widget(&label_path, |w| {
            if let Some(crate::ve::ViewElement { kind: VeKind::Label(l), .. }) = &mut w.sub_ve {
                l.justify = if rows <= 1 { Justify::Center } else { label_justify };
            }
        });
        self.with_widget(&label_path, |w| g::set_position(w, border, border, screen, reference, p));
        self.with_widget(&label_path, |w| w.set_visible(true));
        self.layout();
        let n_raw = req.buttons.len();
        let n = n_raw.min(3);
        let button_path = |i: usize| if i == 0 { format!("{path_mb}/MessageBoxButton") } else { format!("{path_mb}/MessageBoxButton#{i}") };
        let mut avail = max[1] - two_b;
        if n_raw > 0 {
            let bh = self.widget_by(&button_path(0)).map(|w| g::height(w, screen, reference, p)).unwrap_or(0.0);
            avail -= bh + spacing;
        }
        let mut text_w = if tw + two_b < max[0] { tw } else { max[0] - two_b };
        let mut y = if avail < th {
            self.with_widget(&label_path, |w| g::set_height(w, avail));
            border + avail
        } else {
            th + border
        };
        if n != 0 {
            let (bw, bh) = self
                .widget_by(&button_path(0))
                .map(|w| (g::width(w, screen, reference, p), g::height(w, screen, reference, p)))
                .unwrap_or((0.0, 0.0));
            y += spacing;
            let mut acc = bw;
            let place = |ui: &mut Ui, i: usize, x: f32| {
                let (label, _) = &req.buttons[i];
                ui.with_widget(&button_path(i), |w| {
                    g::set_position(w, x, y, screen, reference, p);
                    w.set_visible(true);
                    if let Some(c) = w.child_mut("MessageBoxButtonLabel") {
                        set_label_text(c, label);
                    }
                });
                ui.layout();
            };
            if n < 2 {
                place(self, 0, (text_w * 0.5 - bw * 0.5) + border);
            } else {
                place(self, 0, border);
                let mut step = (text_w - bw) / (n - 1) as f32;
                if step < bw + spacing {
                    step = bw + spacing;
                    text_w = (n - 1) as f32 * step + bw;
                }
                for i in 1..n {
                    place(self, i, i as f32 * step + border);
                    acc += step;
                }
            }
            if text_w < acc {
                text_w = acc;
            }
            y += bh;
        }
        let mut w = text_w + two_b;
        let mut h = y + border;
        if w < min[0] {
            w = min[0];
        } else if w > max[0] {
            w = max[0];
        }
        if h < min[1] {
            h = min[1];
        } else if h > max[1] {
            h = max[1];
        }
        self.with_widget(&path_mb, |x| g::set_size(x, w, h));
        self.layout();
        if n_raw == 1 {
            let bw = self.widget_by(&button_path(0)).map(|x| g::width(x, screen, reference, p)).unwrap_or(0.0);
            self.with_widget(&button_path(0), |x| g::set_left(x, w * 0.5 - bw * 0.5, screen, reference, p));
        }
        self.with_widget(&label_path, |x| g::set_width(x, w - two_b));
        self.layout();
        let (iw, ih) = (reference.width, reference.height);
        self.with_widget(&path_mb, |x| g::set_position(x, iw * 0.5 - w * 0.5, ih * 0.5 - h * 0.5, screen, reference, p));
        self.layout();
    }

    /// Close the message box (Disconnect).
    pub fn close_message_box(&mut self) {
        if let Some(name) = self.modal.take() {
            self.disconnect(&name);
        }
        self.message_box = None;
    }

    /// Text size of a label in reference units: (width, height, rows),
    /// wrapped at `wrap` reference units (label vtable+0xCC -> `0x735C00`;
    /// rows as the native measure counts them).
    pub fn measure_label(&mut self, path: &str, text: &str, wrap: f32) -> (f32, f32, usize) {
        let font = match self.widget_by(path).and_then(|w| w.sub_ve.as_ref()) {
            Some(crate::ve::ViewElement { kind: VeKind::Label(l), .. }) => l.font.clone(),
            _ => return (0.0, 0.0, 0),
        };
        let (w, h, rows) = self.measure_text(&font, text, wrap);
        (w, h, rows.max(0) as usize)
    }
}
