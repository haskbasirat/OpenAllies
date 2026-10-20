//! Input routing, activation, per-frame update and the deferred callback
//! queue of the UI, with the control class behaviour (`controls.rs`).
//!
//! * Pick (`0x5E2B50`) walks children tail to head and returns the topmost
//!   widget that takes input: buttons, slider grips and tracks, owned
//!   list items, drop-downs and edit boxes. A drop-down's SelectedItem has
//!   no owner and does not consume clicks, so they reach the drop-down.
//!   Open drop-down list boxes (the UI overlay) are picked first.
//! * Activate (`0x5EE1D0` -> vtable+0x48; base `0x5EF1C0`) runs when a
//!   top-level widget is shown: list box `0x5D4F30` (before the base),
//!   slider `0x5DD940` (background + grip after the base), drop-down
//!   `0x5D4430` (close, list box out of the tree, base, Place,
//!   UpdateSelected), viewport scroll bar update.
//! * Per frame (`vtable+0x0C`): list box `0x5D4A30` (bind the visible
//!   rows, refresh, scroll range), edit box `0x5D0030` (caret blink and
//!   placement `0x5CF9B0`).
//! * Value and command callbacks are queued (`0x5C3EB0`) and run by
//!   [`Ui::run_queue`] after each input event.
//!
//! Event codes used by the natives (`event+8`): 0x100 left button, 0x101
//! right button, 0x108/0x109 double clicks, 0x110 wheel. Double clicks are
//! not delivered by this port yet.

use crate::controls::{self as k, Arg, Geo, ListCallback, ListEntry, SliderCallback};
use crate::layout::ui_scale;
use crate::state::widget_flag as wf;
use crate::ui::{MouseButton, Ui, UiEvent, child_segments, detached, path_mut, path_ref};
use crate::ve::VeKind;
use crate::widget::{Widget, WidgetClass as C};
use std::sync::Arc;

/// A queued callback (`0x5C3EB0`).
#[derive(Clone, Debug, PartialEq)]
pub enum Deferred {
    /// Slider `+0x9C(value)`.
    SliderValue { path: String, value: f32 },
    /// List box command for entry `index` (`0x5D50A0`).
    ListCommand { path: String, callback: ListCallback, index: usize, param: String },
}

/// Keys the edit box and drop-downs handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiKey {
    Enter,
    Escape,
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    Insert,
}

/// MSVC `_wtol`: spaces, sign, digits.
fn wtol(s: &str) -> i32 {
    let t = s.trim_start();
    let (neg, rest) = match t.chars().next() {
        Some('-') => (true, &t[1..]),
        Some('+') => (false, &t[1..]),
        _ => (false, t),
    };
    let mut v: i32 = 0;
    for c in rest.chars() {
        match c.to_digit(10) {
            Some(d) => v = v.wrapping_mul(10).wrapping_add(d as i32),
            None => break,
        }
    }
    if neg { v.wrapping_neg() } else { v }
}

fn rel(path: &str) -> &str {
    path.split_once('/').map(|(_, r)| r).unwrap_or("")
}

fn parent_of(path: &str) -> Option<&str> {
    path.rsplit_once('/').map(|(p, _)| p)
}

/// Classes that take pointer input.
fn interactive(w: &Widget) -> bool {
    match w.class {
        C::PushButton | C::ToggleButton | C::Grip | C::Slider | C::SliderAdvanced | C::DropDown | C::Editbox | C::SelectablePanel | C::MiniMap => true,
        C::ListItem => w.list_item().is_some_and(|i| i.owned),
        _ => false,
    }
}

fn set_ve_active(w: &mut Widget, on: bool) {
    let f = crate::layout::ve_flag::ACTIVE;
    let set = |flags: &mut u32| if on { *flags |= f } else { *flags &= !f };
    set(&mut w.root.flags);
    if let Some(v) = &mut w.view {
        set(&mut v.layout.flags);
    }
    if let Some(v) = &mut w.sub_ve {
        set(&mut v.layout.flags);
    }
}

impl Ui {
    pub fn geo(&self) -> Geo {
        let (sx, sy) = ui_scale(self.screen, self.reference, self.precision);
        Geo { sx, sy }
    }

    /// Widget by a pick/draw path (`Root/A/B#1`).
    pub fn widget(&self, path: &str) -> Option<&Widget> {
        path_ref(&self.root, rel(path))
    }

    pub fn widget_mut(&mut self, path: &str) -> Option<&mut Widget> {
        path_mut(&mut self.root, rel(path))
    }

    pub(crate) fn set_flag(&mut self, path: &str, bit: u32, on: bool) {
        if let Some(w) = self.widget_mut(path) {
            if on {
                w.flags |= bit;
            } else {
                w.flags &= !bit;
            }
        }
    }

    fn sound(&mut self, name: &str) {
        if !name.is_empty() && !name.eq_ignore_ascii_case("null") {
            self.events.push(UiEvent::Sound { name: name.to_string() });
        }
    }

    // ------------------------------------------------------------ activation

    /// Activate the top-level widget at `path` (`Root/Name`) and its
    /// subtree; call after `connect`.
    pub fn activate(&mut self, path: &str) {
        self.layout();
        self.activate_rec(path);
        self.layout();
        self.update_controls();
        self.layout();
    }

    /// Deactivate (`0x5EEED0`): close drop-downs, drop focus and capture.
    pub fn deactivate(&mut self, path: &str) {
        let prefix = format!("{path}/");
        let open: Vec<String> = self.overlay.iter().filter(|p| p.starts_with(&prefix) || *p == path).cloned().collect();
        for dd in open {
            self.dropdown_close(&dd);
        }
        self.capture.retain(|c| !c.starts_with(&prefix));
        // edit box Deactivate 0x5CF5B0: unregistered (0x5C24A0)
        self.editboxes.retain(|e| !(e.starts_with(&prefix) || e == path));
        fn clear(w: &mut Widget) {
            w.flags &= !wf::ACTIVE;
            set_ve_active(w, false);
            for c in &mut w.children {
                clear(c);
            }
        }
        if let Some(w) = self.widget_mut(path) {
            clear(w);
        }
    }

    fn activate_rec(&mut self, path: &str) {
        let Some(w) = self.widget(path) else { return };
        let class = w.class;
        let kids: Vec<(String, bool)> = w
            .children
            .iter()
            .zip(child_segments(w))
            .map(|(c, s)| (format!("{path}/{s}"), class == C::DropDown && c.class == C::Listbox))
            .collect();
        match class {
            C::Listbox => self.listbox_activate(path),
            C::DropDown => self.dropdown_close(path),
            // edit box Activate 0x5D0390: registered for Cycle (0x5C5300)
            C::Editbox => {
                if !self.editboxes.iter().any(|p| p == path) {
                    self.editboxes.push(path.to_string());
                }
            }
            C::NumericEditbox => self.numeric_set_step(path, 1),
            _ => {}
        }
        if let Some(w) = self.widget_mut(path) {
            w.flags |= wf::ACTIVE;
            set_ve_active(w, true);
        }
        for (kid, skip) in kids {
            if !skip {
                self.activate_rec(&kid);
            }
        }
        match class {
            C::Slider | C::SliderAdvanced => {
                let g = self.geo();
                if let Some(w) = self.widget_mut(path) {
                    k::layout_background(w, g);
                    k::update_grip(w, g);
                }
                self.slider_labels(path);
            }
            C::DropDown => {
                self.dropdown_place(path);
                self.dropdown_update_selected(path);
            }
            C::Viewport => self.viewport_update(path, false),
            _ => {}
        }
    }

    // ------------------------------------------------------------ slider

    /// Advanced slider labels (`0x5DEC80`, run after every UpdateGrip).
    pub(crate) fn slider_labels(&mut self, path: &str) {
        let Some(s) = self.widget(path).and_then(|w| w.slider()).cloned() else { return };
        let Some((_, _, fmt)) = s.advanced else { return };
        let strings = self.strings.clone();
        let loc = |key: &str| strings.resolve(&format!("#{key}"));
        for (name, v) in [("MinBoundLabel", s.min), ("MaxBoundLabel", s.max), ("CurrentValueLabel", s.value)] {
            let p = format!("{path}/{name}");
            let visible = self.widget(&p).is_some_and(|w| w.flags & 1 != 0);
            if visible {
                let text = k::format_value(fmt, v, &s, &loc);
                self.set_label(rel(&p), &text);
            }
        }
    }

    /// `SetValue(v, snap)`.
    pub fn slider_set_value(&mut self, path: &str, v: f32, snap: bool) -> bool {
        let g = self.geo();
        let changed = self.widget_mut(path).is_some_and(|w| k::set_value(w, v, snap, g));
        if changed {
            self.slider_labels(path);
        }
        changed
    }

    /// `SetMin` / `SetMax`.
    pub fn slider_set_range(&mut self, path: &str, min: f32, max: f32) {
        let g = self.geo();
        if let Some(w) = self.widget_mut(path) {
            k::set_min(w, min, g);
            k::set_max(w, max, g);
        }
        self.slider_labels(path);
    }

    /// `SetValueNotify` (`0x5DDCA0`).
    pub fn slider_set_value_notify(&mut self, path: &str, v: f32) {
        if !self.slider_set_value(path, v, true) {
            return;
        }
        let Some(w) = self.widget(path) else { return };
        let Some(s) = w.slider() else { return };
        if w.flags & wf::ACTIVE != 0 && s.callback != SliderCallback::None {
            let value = s.value;
            self.queue.push(Deferred::SliderValue { path: path.to_string(), value });
        }
    }

    pub fn slider_value(&self, path: &str) -> Option<f32> {
        self.widget(path).and_then(|w| w.slider()).map(|s| s.value)
    }

    fn axis_px(&self, path: &str, x: f32, y: f32) -> f32 {
        let vertical = self.widget(path).is_some_and(|w| w.flags & wf::VERTICAL != 0);
        if vertical { y } else { x }
    }

    /// Track click (`0x5DDFB0`).
    fn slider_track_click(&mut self, path: &str, x: f32, y: f32) {
        let g = self.geo();
        let Some(w) = self.widget(path) else { return };
        let Some(s) = w.slider().cloned() else { return };
        let vertical = w.flags & wf::VERTICAL != 0;
        let m = self.axis_px(path, x, y) / if vertical { g.sy } else { g.sx };
        let Some((gl, gr)) = k::grip_ref(w, g) else { return };
        let in_scrollbar = parent_of(path).and_then(|p| self.widget(p)).is_some_and(|p| p.class == C::Scrollbar);
        let v = if in_scrollbar {
            if m < gl {
                s.value - s.page
            } else if m > gr {
                s.value + s.page
            } else {
                s.value
            }
        } else if m <= gl || m >= gr {
            k::pos_to_value(w, &s, m, g)
        } else {
            return;
        };
        self.slider_set_value_notify(path, v);
        self.sound(&s.sound_clicked);
    }

    /// Grip press (`0x5DDC00`): grab offset `0x5DD5D0`, drag start
    /// `0x5E0AC0` (PRESSED).
    fn grip_press(&mut self, grip: &str, x: f32, y: f32) {
        let Some(slider) = parent_of(grip).map(str::to_string) else { return };
        let g = self.geo();
        let Some(w) = self.widget(&slider) else { return };
        let Some(s) = w.slider().cloned() else { return };
        let Some((gl, _)) = k::grip_ref(w, g) else { return };
        let (start, _) = k::usage_abs(w, &s, g);
        if let Some(sm) = self.widget_mut(&slider).and_then(|w| w.slider_mut()) {
            sm.grab = gl - start;
        }
        self.set_flag(grip, wf::PRESSED, true);
        self.drag = Some((grip.to_string(), x, y));
        self.sound(&s.sound_clicked);
    }

    /// Drag step (`0x5E0A10` -> grip vtable+0x68 `0x5DDEB0`).
    fn grip_drag(&mut self, x: f32, y: f32) {
        let Some((grip, x0, y0)) = self.drag.clone() else { return };
        let Some(slider) = parent_of(&grip).map(str::to_string) else { return };
        let g = self.geo();
        let Some(w) = self.widget(&slider) else { return };
        let Some(s) = w.slider().cloned() else { return };
        let vertical = w.flags & wf::VERTICAL != 0;
        let (delta, scale) = if vertical { (y - y0, g.sy) } else { (x - x0, g.sx) };
        let (start, end) = k::usage_abs(w, &s, g);
        let len = end - start;
        if len - s.grip_size == 0.0 {
            return;
        }
        let t = (delta / scale + s.grab) / (len - s.grip_size);
        let v = if s.min == s.max { s.min } else { k::snap_clamp(&s, s.min + (s.max - s.min) * t, true) };
        self.slider_set_value_notify(&slider, v);
    }

    /// Less/More buttons of a scroll bar (`0x5DDD20`).
    fn scrollbar_step(&mut self, scrollbar: &str, more: bool) {
        let slider = format!("{scrollbar}/Slider");
        let Some(s) = self.widget(&slider).and_then(|w| w.slider()).cloned() else { return };
        if let Some(owner) = parent_of(scrollbar) {
            if self.widget(owner).is_some_and(|w| w.class == C::Viewport) {
                let owner = owner.to_string();
                self.viewport_step(&owner, more);
                return;
            }
        }
        let delta = if s.step != 0.0 { s.step } else { s.page };
        let v = if more { s.value + delta } else { s.value - delta };
        self.slider_set_value_notify(&slider, v);
    }

    // ------------------------------------------------------------ list box

    fn listbox_geom(&self, path: &str) -> Option<(f32, f32, i32, bool)> {
        let lb = self.widget(path)?.listbox()?;
        Some((lb.item_height, lb.vertical_item_offset, lb.num_visible, lb.resize_to_fit))
    }

    /// Activate (`0x5D4F30`).
    fn listbox_activate(&mut self, path: &str) {
        let g = self.geo();
        let Some((ih, off, nv, resize)) = self.listbox_geom(path) else { return };
        let Some(w) = self.widget(path) else { return };
        let h = (w.root.outer.rect.b - w.root.outer.rect.t) / g.sy;
        let mut n = if ih > 0.0 { ((h + k::EPS - (off + off)) / ih) as i32 } else { 1 };
        if n < 1 {
            n = 1;
        }
        if resize && n != nv {
            if let Some(w) = self.widget_mut(path) {
                k::apply_list_height(w, n);
            }
        }
        if let Some(lb) = self.widget_mut(path).and_then(|w| w.listbox_mut()) {
            lb.num_visible = n;
        }
        if let Some(s) = self.widget_mut(&format!("{path}/Scrollbar/Slider")).and_then(|w| w.slider_mut()) {
            s.page = n as f32;
        }
        let slots = self.widget(path).and_then(|w| w.listbox()).map(|l| l.slots).unwrap_or(0);
        if slots == 0 {
            self.listbox_create_slot(path);
            if let Some(item) = self.widget_mut(&format!("{path}/ItemLook")) {
                item.set_visible(false);
            }
        }
        self.listbox_mark_dirty(path);
    }

    fn listbox_mark_dirty(&mut self, path: &str) {
        if let Some(lb) = self.widget_mut(path).and_then(|w| w.listbox_mut()) {
            for e in &mut lb.items {
                e.flags |= 2;
            }
            lb.first_dirty = 0;
        }
    }

    /// Create the next item widget (`0x5D48E0`).
    fn listbox_create_slot(&mut self, path: &str) {
        let Some(w) = self.widget(path) else { return };
        let Some(lb) = w.listbox() else { return };
        let Some(node) = lb.node.clone() else { return };
        let (slot, ih, off) = (lb.slots, lb.item_height, lb.vertical_item_offset);
        let threshold = w.render_threshold;
        let mut item = Widget::new(C::ListItem, "ItemLook", false);
        item.control = Some(Box::new(k::Control::ListItem(k::ListItem { owned: true, ..Default::default() })));
        {
            let strings = &self.strings;
            let resolve = |v: &str| strings.resolve(v);
            let root = Arc::new(node.clone());
            item.load(&mut self.store, Some((&node, &root)), Some(threshold), &resolve);
        }
        k::place_item(&mut item, slot, ih, off);
        if let Some(w) = self.widget_mut(path) {
            w.add_child(item);
            if let Some(lb) = w.listbox_mut() {
                lb.slots += 1;
            }
        }
    }

    fn item_path(path: &str, slot: usize) -> String {
        if slot == 0 { format!("{path}/ItemLook") } else { format!("{path}/ItemLook#{slot}") }
    }

    /// Text that fits `width_px` with the localized ellipsis (`0x735D50`,
    /// native-proven in `rtext.rs`; list items pass the label VE width and
    /// height 0).
    fn truncate_text(&mut self, font: &str, text: &str, width_px: f32) -> (String, bool) {
        let (t, s) = self.ellipsis_text(font, text, width_px);
        (s, t)
    }

    /// Bind entry `idx` (or nothing with `None`) to the item widget at
    /// `item` (`0x5D3E10`).
    fn bind_item(&mut self, item: &str, entry: Option<(usize, ListEntry)>) {
        let label = self.widget(item).and_then(|w| match &w.sub_ve {
            Some(crate::ve::ViewElement { kind: VeKind::Label(l), layout, .. }) => {
                Some((l.font.clone(), layout.outer.rect.r - layout.outer.rect.l))
            }
            _ => None,
        });
        let (text, truncated) = match (&entry, &label) {
            (Some((_, e)), Some((font, wpx))) => {
                let (font, wpx) = (font.clone(), *wpx);
                self.truncate_text(&font, &e.text, wpx)
            }
            (Some((_, e)), None) => (e.text.clone(), false),
            (None, _) => (String::new(), false),
        };
        if let Some(w) = self.widget_mut(item) {
            let selected = entry.as_ref().is_some_and(|(_, e)| e.selected());
            if selected {
                w.flags |= wf::SELECTED;
            } else {
                w.flags &= !wf::SELECTED;
            }
            // 0x5EDFE0: the entry colour replaces the label's (when the item
            // has its label view element, widget+0x6C)
            if let Some((_, e)) = &entry {
                if let Some(crate::ve::ViewElement { kind: VeKind::Label(l), .. }) = &mut w.sub_ve {
                    l.color = e.color;
                }
            }
            if let Some(it) = w.list_item_mut() {
                match &entry {
                    Some((i, e)) => {
                        it.index = *i as i32;
                        it.param = if e.param.is_empty() { e.text.clone() } else { e.param.clone() };
                        it.text = e.text.clone();
                    }
                    None => {
                        it.index = -1;
                        it.param.clear();
                        it.text.clear();
                    }
                }
                it.truncated = truncated;
            }
        }
        self.set_label(rel(item), &text);
    }

    /// Per-frame update (`0x5D4A30`) with Refresh `0x5D42C0` and the
    /// scroll range `0x5D31B0`.
    fn listbox_update(&mut self, path: &str) {
        let Some(lb) = self.widget(path).and_then(|w| w.listbox()).cloned() else { return };
        let count = lb.items.len();
        let mut created = false;
        let mut slots = lb.slots;
        for slot in 0..lb.num_visible.max(0) as usize {
            let idx = lb.top.max(0) as usize + slot;
            if slot < slots {
                let ip = Self::item_path(path, slot);
                if idx < count {
                    let dirty = lb.items[idx].flags & 2 != 0 || (lb.first_dirty >= 0);
                    if dirty {
                        self.bind_item(&ip, Some((idx, lb.items[idx].clone())));
                    }
                    if let Some(w) = self.widget_mut(&ip) {
                        w.set_visible(true);
                    }
                } else if let Some(w) = self.widget_mut(&ip) {
                    w.set_visible(false);
                }
            } else if idx < count {
                self.listbox_create_slot(path);
                slots += 1;
                created = true;
            }
        }
        if created {
            // new rows need their label rects before binding
            self.layout();
            for slot in 0..lb.num_visible.max(0) as usize {
                let idx = lb.top.max(0) as usize + slot;
                if slot < slots && idx < count {
                    let ip = Self::item_path(path, slot);
                    self.bind_item(&ip, Some((idx, lb.items[idx].clone())));
                }
            }
        }
        for slot in lb.num_visible.max(0) as usize..slots {
            if let Some(w) = self.widget_mut(&Self::item_path(path, slot)) {
                w.set_visible(false);
            }
        }
        if let Some(l) = self.widget_mut(path).and_then(|w| w.listbox_mut()) {
            for e in &mut l.items {
                e.flags &= !2;
            }
            l.first_dirty = -1;
        }
        // scroll range
        let slider = format!("{path}/Scrollbar/Slider");
        if count as i32 <= lb.num_visible {
            if let Some(sb) = self.widget_mut(&format!("{path}/Scrollbar")) {
                sb.set_visible(false);
            }
            self.slider_set_value_notify(&slider, 0.0);
        } else {
            if let Some(sb) = self.widget_mut(&format!("{path}/Scrollbar")) {
                sb.set_visible(true);
            }
            let g = self.geo();
            if let Some(w) = self.widget_mut(&slider) {
                k::set_max(w, (count as i32 - lb.num_visible) as f32, g);
            }
        }
    }

    /// Add an entry (text, param); returns its index.
    pub fn listbox_add(&mut self, path: &str, text: &str, param: &str) -> usize {
        self.widget_mut(path).map(|w| crate::control_load::add_entry(w, ListEntry::new(text, param))).unwrap_or(0)
    }

    pub fn listbox_clear(&mut self, path: &str) {
        if let Some(w) = self.widget_mut(path) {
            crate::control_load::clear_entries(w);
        }
    }

    pub fn listbox_entries(&self, path: &str) -> Vec<ListEntry> {
        self.widget(path).and_then(|w| w.listbox()).map(|l| l.items.clone()).unwrap_or_default()
    }

    /// First selected entry index.
    pub fn listbox_selected(&self, path: &str) -> Option<usize> {
        self.widget(path).and_then(|w| w.listbox()).and_then(|l| l.selection.first().copied())
    }

    /// `Select(idx, toggle)` (`0x5D4030`). Returns whether the selection
    /// changed.
    pub fn listbox_select(&mut self, path: &str, idx: usize, toggle: bool) -> bool {
        let Some(lb) = self.widget(path).and_then(|w| w.listbox()).cloned() else { return false };
        if idx >= lb.items.len() {
            return false;
        }
        let mut lb2 = lb.clone();
        if !lb.multiselect {
            if lb.selection.first() == Some(&idx) && lb.ignore_redundant_select {
                return false;
            }
            for (i, e) in lb2.items.iter_mut().enumerate() {
                if e.flags & 4 != 0 || i == idx {
                    e.flags |= 2;
                }
                e.flags &= !4;
            }
            lb2.selection.clear();
        } else if toggle && lb.selection.contains(&idx) {
            lb2.selection.retain(|s| *s != idx);
            lb2.items[idx].flags = (lb2.items[idx].flags & !4) | 2;
            if let Some(w) = self.widget_mut(path) {
                w.control = Some(Box::new(k::Control::Listbox(lb2)));
            }
            return true;
        }
        lb2.selection.push(idx);
        lb2.items[idx].flags |= 6;
        let (top, nv) = (lb2.top, lb2.num_visible);
        if let Some(w) = self.widget_mut(path) {
            w.control = Some(Box::new(k::Control::Listbox(lb2)));
        }
        let i = idx as i32;
        if i < top || i >= top + nv {
            self.slider_set_value_notify(&format!("{path}/Scrollbar/Slider"), (i - (nv - 1) / 2) as f32);
        }
        true
    }

    /// Select with the list box's select command (`0x5D5230` -> `0x5D50A0`).
    pub fn listbox_select_notify(&mut self, path: &str, idx: usize) {
        self.listbox_select(path, idx, false);
        self.listbox_fire(path, idx, 0);
    }

    /// Fire command slot `which` (0 select, 1 double, 2 right, 3 right
    /// double) for entry `idx` (`0x5D50A0`; fired even when the select was
    /// redundant).
    fn listbox_fire(&mut self, path: &str, idx: usize, which: usize) {
        let Some(lb) = self.widget(path).and_then(|w| w.listbox()) else { return };
        let cb = [&lb.on_select, &lb.on_double, &lb.on_right, &lb.on_right_double][which].clone();
        let Some(e) = lb.items.get(idx) else { return };
        let param = if e.param.is_empty() { e.text.clone() } else { e.param.clone() };
        if cb != ListCallback::None {
            self.queue.push(Deferred::ListCommand { path: path.to_string(), callback: cb, index: idx, param });
        }
    }

    /// Scroll bar functor of a list box (`0x5D2CC0`).
    fn listbox_scrolled(&mut self, path: &str, value: f32) {
        let top = (value + 0.5) as i32;
        let changed = self.widget_mut(path).and_then(|w| w.listbox_mut()).is_some_and(|lb| {
            let c = lb.top != top;
            lb.top = top;
            c
        });
        if changed {
            self.listbox_mark_dirty(path);
        }
    }

    // ------------------------------------------------------------ drop-down

    /// Place the list box below (or above, `dropup`) the drop-down in
    /// absolute reference units (`0x5D3450`; only when active).
    fn dropdown_place(&mut self, path: &str) {
        let g = self.geo();
        let Some(w) = self.widget(path) else { return };
        if w.flags & wf::ACTIVE == 0 {
            return;
        }
        let Some(dd) = w.dropdown().cloned() else { return };
        let r = w.root.outer.rect;
        let lbp = format!("{path}/Listbox");
        let Some(lb) = self.widget(&lbp).and_then(|l| l.listbox()).cloned() else { return };
        let n = (lb.items.len() as i32).min(dd.max_visible).max(1);
        if lb.resize_to_fit && n != lb.num_visible {
            if let Some(l) = self.widget_mut(&lbp) {
                if let Some(x) = l.listbox_mut() {
                    x.num_visible = n;
                }
                k::apply_list_height(l, n);
            }
        }
        let nv = self.widget(&lbp).and_then(|l| l.listbox()).map(|l| l.num_visible).unwrap_or(n);
        let h = nv as f32 * lb.item_height + (lb.vertical_item_offset + lb.vertical_item_offset);
        let t = if dd.dropup { r.t / g.sy - h } else { r.b / g.sy };
        if let Some(l) = self.widget_mut(&lbp) {
            k::set_bounds(&mut l.root.outer, [r.l / g.sx, t, r.r / g.sx, h], 0x200);
        }
    }

    /// `UpdateSelected` (`0x5D4390`).
    fn dropdown_update_selected(&mut self, path: &str) {
        let lbp = format!("{path}/Listbox");
        let entry = self
            .widget(&lbp)
            .and_then(|l| l.listbox())
            .and_then(|l| l.selection.first().map(|&i| (i, l.items[i].clone())));
        self.layout();
        self.bind_item(&format!("{path}/SelectedItem"), entry);
    }

    fn dropdown_open(&mut self, path: &str) {
        self.dropdown_place(path);
        let dd = self.widget(path).and_then(|w| w.dropdown()).cloned().unwrap_or_default();
        self.sound(&dd.sound_open);
        if let Some(w) = self.widget_mut(path) {
            w.flags |= wf::DROPDOWN_OPEN | wf::OPEN;
        }
        if !self.overlay.iter().any(|p| p == path) {
            self.overlay.push(path.to_string());
        }
        self.capture.push(path.to_string());
        self.layout();
        let lbp = format!("{path}/Listbox");
        self.activate_rec(&lbp);
        self.layout();
        self.listbox_update(&lbp);
        self.layout();
    }

    /// Close (`0x5D2DD0`).
    pub fn dropdown_close(&mut self, path: &str) {
        let open = self.widget(path).is_some_and(|w| w.flags & wf::DROPDOWN_OPEN != 0);
        if open {
            let dd = self.widget(path).and_then(|w| w.dropdown()).cloned().unwrap_or_default();
            self.sound(&dd.sound_close);
        }
        if let Some(w) = self.widget_mut(path) {
            w.flags &= !(wf::DROPDOWN_OPEN | wf::OPEN);
        }
        self.overlay.retain(|p| p != path);
        if self.capture.last().map(String::as_str) == Some(path) {
            self.capture.pop();
        }
    }

    fn dropdown_toggle(&mut self, path: &str) {
        let open = self.widget(path).is_some_and(|w| w.flags & wf::DROPDOWN_OPEN != 0);
        if open {
            self.dropdown_close(path);
        } else {
            self.dropdown_open(path);
        }
        self.layout();
    }

    /// List box select functor of a drop-down (`0x5D5540` -> `0x5D51C0`).
    fn dropdown_selected(&mut self, path: &str, index: usize) {
        self.dropdown_close(path);
        self.dropdown_update_selected(path);
        let active = self.widget(path).is_some_and(|w| w.flags & wf::ACTIVE != 0);
        let cmd = self.widget(path).and_then(|w| w.dropdown()).and_then(|d| d.command.clone());
        let param = self.widget(&format!("{path}/SelectedItem")).and_then(|w| w.list_item()).map(|i| i.param.clone()).unwrap_or_default();
        let functor = self.widget(path).and_then(|w| w.dropdown()).is_some_and(|d| d.functor);
        if active {
            if functor {
                self.events.push(UiEvent::Select { widget: path.to_string(), index, param });
            } else if let Some(c) = cmd {
                let command = k::sprintf1(&c, &Arg::Str(param));
                self.events.push(UiEvent::Command { widget: path.to_string(), command });
            }
        }
        self.layout();
    }

    /// Set the drop-down's command (`+0x7C`, `0x5D6100` factories).
    pub fn dropdown_set_command(&mut self, path: &str, command: &str) {
        if let Some(k::Control::DropDown(d)) = self.widget_mut(path).and_then(|w| w.control.as_deref_mut()) {
            d.command = Some(command.to_string());
        }
    }

    /// Select entry `idx` of a drop-down without firing its command
    /// (`0x5D4030` + UpdateSelected).
    pub fn dropdown_select(&mut self, path: &str, idx: usize) {
        self.listbox_select(&format!("{path}/Listbox"), idx, false);
        self.dropdown_update_selected(path);
    }

    /// Select and notify (`0x5D5230` -> `0x5D50A0` -> `0x5D0B10`).
    pub fn dropdown_select_notify(&mut self, path: &str, idx: usize) {
        let lbp = format!("{path}/Listbox");
        self.listbox_select(&lbp, idx, false);
        self.dropdown_selected(path, idx);
    }

    // ------------------------------------------------------------ viewport

    fn viewport_panel_height(&self, path: &str) -> f32 {
        let g = self.geo();
        self.widget(&format!("{path}/ItemPanel")).map(|w| (w.root.outer.rect.b - w.root.outer.rect.t) / g.sy).unwrap_or(0.0)
    }

    /// Item height (`0x5ED150`): B converted to a size.
    fn viewport_item_height(w: &mut Widget, sy: f32) -> f32 {
        let o = &mut w.root.outer;
        if o.flags & 0x288 != 0x200 {
            o.raw[3] = (o.rect.b - o.rect.t) / sy;
            o.flags = (o.flags & !0x88) | 0x200;
        }
        o.raw[3]
    }

    /// Lay the ItemPanel children out top-down (`0x5ED510` with Place
    /// `0x5ED1C0`).
    fn viewport_layout(&mut self, path: &str) {
        let g = self.geo();
        let scroll_visible = self.widget(&format!("{path}/Scrollbar")).is_some_and(|w| w.flags & 1 != 0);
        let scroll = self.widget(path).and_then(|w| w.viewport()).map(|v| v.scroll).unwrap_or(0.0);
        if let Some(panel) = self.widget_mut(&format!("{path}/ItemPanel")) {
            let mut y = 0.0f32;
            for c in &mut panel.children {
                let h = Self::viewport_item_height(c, g.sy);
                let yy = if scroll_visible { y - scroll } else { y };
                let o = &mut c.root.outer;
                if (o.raw[1] - yy).abs() >= k::EPS || o.flags & 0x22 != 0 {
                    o.raw[1] = yy;
                    o.flags &= !0x22;
                }
                y += h;
            }
        }
    }

    /// Scroll bar update (`0x5ED560`).
    fn viewport_update(&mut self, path: &str, force_bottom: bool) {
        let Some(vp) = self.widget(path).and_then(|w| w.viewport()).cloned() else { return };
        let sbp = format!("{path}/Scrollbar");
        if self.widget(&sbp).is_none() {
            self.viewport_layout(path);
            return;
        }
        let over = vp.total - self.viewport_panel_height(path);
        let slider = format!("{sbp}/Slider");
        if over <= 0.0 {
            if let Some(w) = self.widget_mut(&sbp) {
                w.set_visible(false);
            }
            if let Some(v) = self.widget_mut(path).and_then(|w| w.viewport_mut()) {
                v.scroll = 0.0;
            }
        } else {
            let was_visible = self.widget(&sbp).is_some_and(|w| w.flags & 1 != 0);
            if let Some(w) = self.widget_mut(&sbp) {
                w.set_visible(true);
            }
            if !was_visible {
                if let Some(v) = self.widget_mut(path).and_then(|w| w.viewport_mut()) {
                    v.at_bottom = v.bottom_anchored;
                }
            }
            let s = self.widget(&slider).and_then(|w| w.slider()).cloned().unwrap_or_default();
            let at_bottom = self.widget(path).and_then(|w| w.viewport()).is_some_and(|v| v.at_bottom);
            let pin = force_bottom || at_bottom || s.value >= s.max - k::EPS || s.value >= over - k::EPS;
            let g = self.geo();
            let changed = self.widget_mut(&slider).is_some_and(|w| k::set_max(w, over, g));
            if pin && !changed {
                self.slider_set_value(&slider, over, true);
            }
            let v = self.slider_value(&slider).unwrap_or(0.0);
            if let Some(vp) = self.widget_mut(path).and_then(|w| w.viewport_mut()) {
                vp.scroll = v;
            }
        }
        self.viewport_layout(path);
    }

    /// AddItem (`0x5ED730`).
    pub fn viewport_add(&mut self, path: &str, item: Widget) {
        let panel_path = format!("{path}/ItemPanel");
        let Some(item_path) = self.attach(&panel_path, item) else { return };
        // AddChild lays the item out against the panel (0x5E34B0) before
        // its height is read
        self.layout();
        let g = self.geo();
        let h = self.widget_mut(&item_path).map(|w| Self::viewport_item_height(w, g.sy)).unwrap_or(0.0);
        if let Some(v) = self.widget_mut(path).and_then(|w| w.viewport_mut()) {
            v.total += h;
        }
        self.viewport_update(path, false);
    }

    /// AddChild (`0x5F0080`, append) to the widget at `parent`; a child
    /// added to an active parent is activated (`0x5EFA10` -> vtable+8 with
    /// the parent's active bit). Returns the child's path.
    pub fn attach(&mut self, parent: &str, child: Widget) -> Option<String> {
        let tag_name = child.name.trim_start_matches('/').to_string();
        let p = self.widget_mut(parent)?;
        let active = p.flags & wf::ACTIVE != 0;
        p.add_child(child);
        // the new child is the last of its name among equal keys: find its
        // segment by position
        let p = self.widget(parent)?;
        let segs = child_segments(p);
        let idx = p.children.iter().rposition(|c| c.name.trim_start_matches('/').eq_ignore_ascii_case(&tag_name))?;
        let path = format!("{parent}/{}", segs[idx]);
        if active {
            self.layout();
            self.activate_rec(&path);
        }
        Some(path)
    }

    /// AddChild at the head of its key group (vtable+0xD4, `0x5F0080` with
    /// `front`) to the widget at `parent`, activated like [`Ui::attach`].
    pub fn attach_front(&mut self, parent: &str, child: Widget) -> Option<String> {
        let tag_name = child.name.trim_start_matches('/').to_string();
        let p = self.widget_mut(parent)?;
        let active = p.flags & wf::ACTIVE != 0;
        p.add_child_front(child);
        // the new child is the first of its name among equal keys
        let p = self.widget(parent)?;
        let segs = child_segments(p);
        let key = p.children.iter().find(|c| c.name.trim_start_matches('/').eq_ignore_ascii_case(&tag_name))?.key;
        let idx = p.children.iter().position(|c| c.key >= key && c.name.trim_start_matches('/').eq_ignore_ascii_case(&tag_name))?;
        let path = format!("{parent}/{}", segs[idx]);
        if active {
            self.layout();
            self.activate_rec(&path);
        }
        Some(path)
    }

    /// Viewport refresh (`0x5ED6B0`): total = sum of the item heights,
    /// the slider page = the item panel height, scroll bar update, layout.
    pub fn viewport_refresh(&mut self, path: &str) {
        let g = self.geo();
        let total = self
            .widget_mut(&format!("{path}/ItemPanel"))
            .map(|p| p.children.iter_mut().map(|c| Self::viewport_item_height(c, g.sy)).sum::<f32>())
            .unwrap_or(0.0);
        if let Some(v) = self.widget_mut(path).and_then(|w| w.viewport_mut()) {
            v.total = total;
        }
        let page = self.viewport_panel_height(path);
        if let Some(s) = self.widget_mut(&format!("{path}/Scrollbar/Slider")).and_then(|w| w.slider_mut()) {
            s.page = page;
        }
        self.viewport_update(path, false);
    }

    /// Snap (`0x5ED2C0`): mode 0 nearest, 1 end of item, 2 start of item.
    fn viewport_snap(&self, path: &str, v: f32, mode: u8) -> f32 {
        let g = self.geo();
        let max = self.slider_value_max(&format!("{path}/Scrollbar/Slider"));
        let v = v.min(max);
        let Some(panel) = self.widget(&format!("{path}/ItemPanel")) else { return v };
        let heights: Vec<f32> = panel
            .children
            .iter()
            .map(|c| {
                let o = c.root.outer;
                if o.flags & 0x288 != 0x200 { (o.rect.b - o.rect.t) / g.sy } else { o.raw[3] }
            })
            .collect();
        let mut rem = v;
        let mut last = None;
        for h in &heights {
            if rem <= 0.0 {
                break;
            }
            rem -= h;
            last = Some(*h);
        }
        let Some(h) = last else { return v };
        if rem > 0.0 || self.viewport_panel_height(path) < h {
            return v;
        }
        match mode {
            2 => v - rem - h,
            1 => v - rem,
            _ => {
                if -rem < 0.5 * h || v == max {
                    v - rem
                } else {
                    v - rem - h
                }
            }
        }
    }

    fn slider_value_max(&self, path: &str) -> f32 {
        self.widget(path).and_then(|w| w.slider()).map(|s| s.max).unwrap_or(0.0)
    }

    /// Step (`0x5ED450`).
    fn viewport_step(&mut self, path: &str, more: bool) {
        let slider = format!("{path}/Scrollbar/Slider");
        let v = self.slider_value(&slider).unwrap_or(0.0);
        let target = if more { self.viewport_snap(path, v + 1.0, 1) } else { self.viewport_snap(path, v - 1.0, 2) };
        self.slider_set_value_notify(&slider, target);
    }

    /// Scroll bar functor (`0x5ED980`).
    fn viewport_scrolled(&mut self, path: &str, value: f32) {
        let snapped = self.viewport_snap(path, value, 0);
        let max = self.slider_value_max(&format!("{path}/Scrollbar/Slider"));
        if let Some(v) = self.widget_mut(path).and_then(|w| w.viewport_mut()) {
            v.scroll = snapped;
            v.at_bottom = snapped >= max - k::EPS;
        }
        self.viewport_layout(path);
    }

    // ------------------------------------------------------------ edit box

    fn editbox_focus(&mut self, path: &str) {
        if let Some(w) = self.widget_mut(path) {
            w.flags |= wf::EDIT_FOCUS;
            if let Some(e) = w.editbox_mut() {
                let n = e.text.chars().count();
                e.selection = if n > 0 { Some((0, n)) } else { None };
                e.caret = n;
                e.blink_ms = 0;
            }
        }
        if self.capture.last().map(String::as_str) != Some(path) {
            self.capture.push(path.to_string());
        }
    }

    /// Focus loss (`0x5D0DE0`): commit when `+0xAD` is set and nothing was
    /// committed yet.
    fn editbox_unfocus(&mut self, path: &str) {
        let commit = self.widget(path).and_then(|w| w.editbox()).is_some_and(|e| e.commit_on_unfocus && !e.committed);
        if commit {
            self.editbox_commit(path);
        }
        if let Some(w) = self.widget_mut(path) {
            w.flags &= !wf::EDIT_FOCUS;
            if let Some(e) = w.editbox_mut() {
                e.selection = None;
            }
        }
        self.capture.retain(|c| c != path);
    }

    /// Commit (`0x5D0D60`): `swprintf(+0x84, text)`, committed = 1.
    fn editbox_commit(&mut self, path: &str) {
        let Some(e) = self.widget(path).and_then(|w| w.editbox()).cloned() else { return };
        if let Some(c) = &e.commit_command {
            let command = k::sprintf1(c, &Arg::Str(e.text.clone()));
            self.events.push(UiEvent::Command { widget: path.to_string(), command });
        }
        if let Some(x) = self.widget_mut(path).and_then(|w| w.editbox_mut()) {
            x.committed = true;
        }
    }

    /// Change command (`0x5D0DA0`): `+0x88` with the text, committed = 0.
    fn editbox_changed(&mut self, path: &str) {
        let Some(e) = self.widget(path).and_then(|w| w.editbox()).cloned() else { return };
        if let Some(c) = &e.change_command {
            let command = k::sprintf1(c, &Arg::Str(e.text.clone()));
            self.events.push(UiEvent::Command { widget: path.to_string(), command });
        }
        if let Some(x) = self.widget_mut(path).and_then(|w| w.editbox_mut()) {
            x.committed = false;
            x.blink_ms = 0;
        }
        self.set_label(&format!("{}/Label", rel(path)), &e.text);
    }

    /// Insert a character (`0x5D11F0`).
    fn editbox_insert(&mut self, path: &str, ch: char) -> bool {
        let overwrite = self.widget(path).is_some_and(|w| w.flags & wf::OVERWRITE != 0);
        let Some(e) = self.widget_mut(path).and_then(|w| w.editbox_mut()) else { return false };
        if e.integer_only && !ch.is_ascii_digit() {
            return false;
        }
        let mut chars: Vec<char> = e.text.chars().collect();
        let mut replaced = false;
        if let Some((a, b)) = e.selection.take() {
            if a < b && b <= chars.len() {
                chars.drain(a..b);
                e.caret = a;
                replaced = true;
            }
        }
        let caret = e.caret.min(chars.len());
        if overwrite && caret < chars.len() {
            chars[caret] = ch;
        } else {
            if e.max_characters != 0 && chars.len() as i32 >= e.max_characters && !replaced {
                e.text = chars.into_iter().collect();
                return false;
            }
            chars.insert(caret, ch);
        }
        e.caret = caret + 1;
        e.text = chars.into_iter().collect();
        if e.integer_only && e.integer_max >= 0 && e.text.parse::<i64>().unwrap_or(0) > e.integer_max as i64 {
            e.text = format!("{}", e.integer_max);
            e.caret = e.text.chars().count();
        }
        true
    }

    /// Text typed while an edit box has the edit focus.
    pub fn text_input(&mut self, text: &str) {
        let Some(path) = self.capture.last().cloned() else { return };
        if self.widget(&path).is_none_or(|w| w.class != C::Editbox) {
            return;
        }
        let mut changed = false;
        for ch in text.chars().filter(|c| !c.is_control()) {
            changed |= self.editbox_insert(&path, ch);
        }
        if changed {
            self.editbox_changed(&path);
        }
        self.run_queue();
    }

    /// A key press. Returns whether the UI consumed it.
    pub fn key_down(&mut self, key: UiKey) -> bool {
        let Some(path) = self.capture.last().cloned() else { return false };
        let class = self.widget(&path).map(|w| w.class);
        match class {
            Some(C::DropDown) => {
                if key == UiKey::Escape {
                    self.dropdown_close(&path);
                    self.layout();
                    return true;
                }
                false
            }
            Some(C::Editbox) => {
                let consumed = match key {
                    UiKey::Enter => {
                        let has = self.widget(&path).and_then(|w| w.editbox()).is_some_and(|e| e.commit_command.is_some());
                        if has {
                            self.editbox_commit(&path);
                        }
                        has
                    }
                    UiKey::Escape => {
                        self.editbox_unfocus(&path);
                        true
                    }
                    UiKey::Insert => {
                        if let Some(w) = self.widget_mut(&path) {
                            w.flags ^= wf::OVERWRITE;
                        }
                        true
                    }
                    _ => {
                        let mut changed = false;
                        if let Some(e) = self.widget_mut(&path).and_then(|w| w.editbox_mut()) {
                            let mut chars: Vec<char> = e.text.chars().collect();
                            let sel = e.selection.take().filter(|(a, b)| a < b && *b <= chars.len());
                            match key {
                                UiKey::Backspace | UiKey::Delete if sel.is_some() => {
                                    let (a, b) = sel.unwrap();
                                    chars.drain(a..b);
                                    e.caret = a;
                                    changed = true;
                                }
                                UiKey::Backspace if e.caret > 0 => {
                                    e.caret -= 1;
                                    chars.remove(e.caret);
                                    changed = true;
                                }
                                UiKey::Delete if e.caret < chars.len() => {
                                    chars.remove(e.caret);
                                    changed = true;
                                }
                                UiKey::Left => e.caret = e.caret.saturating_sub(1),
                                UiKey::Right => e.caret = (e.caret + 1).min(chars.len()),
                                UiKey::Home => e.caret = 0,
                                UiKey::End => e.caret = chars.len(),
                                _ => {}
                            }
                            e.text = chars.into_iter().collect();
                            e.blink_ms = 0;
                        }
                        if changed {
                            self.editbox_changed(&path);
                        }
                        true
                    }
                };
                self.run_queue();
                consumed
            }
            _ => false,
        }
    }

    /// Caret blink and placement (`0x5D0030`, `0x5CF9B0`).
    fn editbox_update(&mut self, path: &str) {
        let g = self.geo();
        let Some(w) = self.widget(path) else { return };
        let focused = w.flags & wf::EDIT_FOCUS != 0;
        // label text (0x5D0140): the selection highlight 0x5D0550 inserts
        // "%c%s%c" (font system +0x1C '<' / +0x1E '>') "h_off" after the
        // selection end, then "h_on" before its start
        if let Some(e) = w.editbox() {
            let mut shown = e.text.clone();
            if let Some((a, b)) = e.selection.filter(|(a, b)| focused && a < b) {
                let mut chars: Vec<char> = e.text.chars().collect();
                let b = b.min(chars.len());
                chars.splice(b..b, "<h_off>".chars());
                chars.splice(a..a, "<h_on>".chars());
                shown = chars.into_iter().collect();
            }
            let current = w.child("Label").and_then(|l| match &l.sub_ve {
                Some(crate::ve::ViewElement { kind: VeKind::Label(d), .. }) => Some(d.text.clone()),
                _ => None,
            });
            if current.is_some_and(|c| c != shown) {
                self.set_label(&format!("{}/Label", rel(path)), &shown);
            }
        }
        let Some(w) = self.widget(path) else { return };
        let cursor = format!("{path}/Cursor");
        if !focused {
            if let Some(c) = self.widget_mut(&cursor) {
                c.set_visible(false);
            }
            return;
        }
        let Some(e) = w.editbox().cloned() else { return };
        let overwrite = w.flags & wf::OVERWRITE != 0;
        let label = w.child("Label").and_then(|l| match &l.sub_ve {
            Some(crate::ve::ViewElement { kind: VeKind::Label(d), layout, .. }) => Some((d.clone(), layout.outer.rect, layout.outer.clip, l.root.outer.raw)),
            _ => None,
        });
        // blink: toggle every 500 ms
        let now = self.now_ms;
        let blink_start = if e.blink_ms == 0 { now } else { e.blink_ms };
        let on = ((now.saturating_sub(blink_start)) / 500) % 2 == 0;
        if let Some(x) = self.widget_mut(path).and_then(|w| w.editbox_mut()) {
            x.blink_ms = blink_start;
        }
        let glyph_name = if overwrite { "ch_overwrite" } else { "ch_insert" };
        let cursor_char = self.strings.glyphs.get(glyph_name).copied().unwrap_or('|');
        let Some((d, rect, clip, raw)) = label else { return };
        let spec = |scroll: f32| crate::ui::LabelSpec {
            font: &d.font,
            rect,
            clip,
            justify: d.justify,
            multiline: d.multiline,
            color: d.color,
            background: d.background_color,
            scroll,
        };
        // text scrolling 0x5CF770 over the rebuilt glyph records
        // (GlyphRect 0x72FF70: pen rects, the scroll included)
        let mut scroll = e.scroll;
        let drawn = self.label_rebuild(&d.text, &spec(scroll));
        let grect = |i: i64| drawn.glyphs.get(usize::try_from(i).ok()?).map(|g| g.rect);
        let vis_w = rect.r - rect.l;
        if let Some(r) = grect(e.caret as i64) {
            if r[2] > vis_w {
                let mut acc = 0.0f32;
                let mut i = e.caret as i64;
                while i >= 0 {
                    let g = grect(i).unwrap_or_default();
                    acc = g[2] - g[0] + acc;
                    if acc > vis_w {
                        break;
                    }
                    i -= 1;
                }
                // ScrollX(-GlyphRect(first).x0) 0x72F410: +0x74 += dx
                scroll += grect(i + 1).map(|g| g[0]).unwrap_or(0.0);
            } else if r[0] < 0.0 {
                scroll += r[0];
            }
        }
        if let Some(x) = self.widget_mut(path).and_then(|w| w.editbox_mut()) {
            x.scroll = scroll;
        }
        if let Some(l) = self.widget_mut(&format!("{path}/Label")) {
            if let Some(crate::ve::ViewElement { kind: VeKind::Label(d), .. }) = &mut l.sub_ve {
                d.scroll_px = scroll;
            }
        }
        // caret placement 0x5CF9B0: GlyphRect(caret) / screen scale + 1,
        // plus the label's raw position; size = the cursor glyph cell and
        // the label font's line height
        let drawn = if scroll != e.scroll { self.label_rebuild(&d.text, &spec(scroll)) } else { drawn };
        let Some(r) = drawn.glyphs.get(e.caret).map(|g| g.rect) else { return };
        let fonts = self.ui_fonts();
        let fid = fonts.id(&d.font);
        use crate::rtext::GlyphTable;
        let fid = if fid == 0 { self.text_env().default_font } else { fid };
        let cw = fonts.glyph(fid, cursor_char as u32).map(|c| c.w as f32).unwrap_or(0.0);
        let lh = fonts.glyph(fid, 'A' as u32).map(|c| c.h as f32).unwrap_or(0.0);
        if let Some(c) = self.widget_mut(&cursor) {
            k::set_bounds(&mut c.root.outer, [r[0] / g.sx + 1.0 + raw[0], r[1] / g.sy + raw[1], cw / g.sx, lh / g.sy], 0x300);
            c.set_visible(on);
        }
        self.set_label(&format!("{}/Cursor", rel(path)), &cursor_char.to_string());
    }

    /// SetStep (`0x5D7910`): the arrow labels show "+%i" / "-%i".
    pub fn numeric_set_step(&mut self, path: &str, step: i32) {
        if let Some(w) = self.widget_mut(path) {
            w.control = Some(Box::new(k::Control::Numeric(step)));
        }
        let r = rel(path).to_string();
        self.set_label(&format!("{r}/UpButton/Client"), &format!("+{step}"));
        self.set_label(&format!("{r}/DownButton/Client"), &format!("-{step}"));
    }

    /// Up `0x5D79F0` / Down `0x5D7AC0`: `_wtol(text) +/- step`, Up clamped
    /// to integer_max when >= 0, Down clamped at 0; SetText without the
    /// change or commit commands.
    fn numeric_step(&mut self, path: &str, up: bool) {
        let step = match self.widget(path).and_then(|w| w.control.as_deref()) {
            Some(k::Control::Numeric(s)) => *s,
            _ => 1,
        };
        let eb = format!("{path}/Editbox");
        let Some(e) = self.widget(&eb).and_then(|w| w.editbox()).cloned() else { return };
        let v = wtol(&e.text);
        let v = if up {
            let v = v.wrapping_add(step);
            if e.integer_max >= 0 && v > e.integer_max { e.integer_max } else { v }
        } else {
            let v = v.wrapping_sub(step);
            if v < 0 { 0 } else { v }
        };
        self.editbox_set_text(&eb, &format!("{v}"));
    }

    /// The edit box commands (`0x5C13E0` SelectAll, `0x5C4190` Copy,
    /// `0x5C1400` Cut, `0x5C26F0` Cycle, `0x5C2730` CyclePrevious) on the
    /// focused edit box. Paste is [`Ui::paste`]. `EditboxUndo` is not a
    /// command of the original (no handler, no undo buffer).
    pub fn editbox_command(&mut self, cmd: &str) -> bool {
        let Some(path) = self.capture.last().cloned() else { return false };
        if self.widget(&path).is_none_or(|w| w.class != C::Editbox) {
            return false;
        }
        match cmd {
            "EditboxSelectAll" => {
                if let Some(e) = self.widget_mut(&path).and_then(|w| w.editbox_mut()) {
                    let n = e.text.chars().count();
                    e.selection = if n > 0 { Some((0, n)) } else { None };
                }
            }
            "EditboxCopy" | "EditboxCut" => {
                let Some(e) = self.widget(&path).and_then(|w| w.editbox()).cloned() else { return false };
                if let Some((a, b)) = e.selection.filter(|(a, b)| a < b) {
                    // 0x5D0840: the displayed glyphs, CR skipped, 0x0E -> ' '
                    let text: String = e
                        .text
                        .chars()
                        .skip(a)
                        .take(b - a)
                        .filter(|c| *c != '\0' && *c != '\r')
                        .map(|c| if c == '\u{e}' { ' ' } else { c })
                        .collect();
                    self.events.push(UiEvent::Clipboard { text });
                    if cmd == "EditboxCut" {
                        // 0x5D0E40: delete, caret = start, change command
                        if let Some(x) = self.widget_mut(&path).and_then(|w| w.editbox_mut()) {
                            let mut chars: Vec<char> = x.text.chars().collect();
                            chars.drain(a..b);
                            x.text = chars.into_iter().collect();
                            x.caret = a;
                            x.selection = None;
                        }
                        self.editbox_changed(&path);
                    }
                }
            }
            "EditboxCycle" | "EditboxCyclePrevious" => {
                if let Some(next) = self.editbox_find_next(&path, cmd == "EditboxCyclePrevious") {
                    self.editbox_unfocus(&path);
                    self.editbox_focus(&next);
                }
            }
            _ => return false,
        }
        self.run_queue();
        true
    }

    /// FindNext (`0x5C2500`): the geometric tab order over the registered,
    /// shown and enabled edit boxes (root VE outer L/T in pixels).
    fn editbox_find_next(&self, cur: &str, prev: bool) -> Option<String> {
        let pos = |p: &str| self.widget(p).map(|w| (w.root.outer.rect.l, w.root.outer.rect.t));
        let (cl, ct) = pos(cur)?;
        let mut top: Option<(f32, String)> = None;
        let mut bottom: Option<(f32, String)> = None;
        let mut best: Option<(f32, f32, String)> = None;
        let mut best_up: Option<(f32, String)> = None;
        for p in &self.editboxes {
            if p == cur {
                continue;
            }
            let Some(w) = self.widget(p) else { continue };
            if w.flags & wf::SHOWN == 0 || w.flags & wf::ENABLED == 0 {
                continue;
            }
            let (l, t) = (w.root.outer.rect.l, w.root.outer.rect.t);
            if top.as_ref().is_none_or(|(bt, _)| t < *bt) {
                top = Some((t, p.clone()));
            }
            if bottom.as_ref().is_none_or(|(bb, _)| t > *bb) {
                bottom = Some((t, p.clone()));
            }
            let (dy, dx) = (t - ct, l - cl);
            if dy >= 0.0 {
                let take = match &best {
                    None => dy <= 100000.0 && dx <= 100000.0,
                    Some((by, bx, _)) => dy < *by || (dy == *by && dx <= *bx),
                };
                if take {
                    best = Some((dy, dx, p.clone()));
                }
            }
            let up = ct - t;
            if up >= 0.0 && best_up.as_ref().is_none_or(|(b, _)| up < *b) && up < 100000.0 {
                best_up = Some((up, p.clone()));
            }
        }
        if prev {
            best_up.map(|x| x.1).or(bottom.map(|x| x.1))
        } else {
            best.map(|x| x.2).or(top.map(|x| x.1))
        }
    }

    /// Paste (`0x5D0F30` -> InsertString `0x5D1470`): clipboard text at the
    /// caret, cut to max_characters and (single line) at the first '\n',
    /// inserted char by char until one is rejected; change command.
    pub fn paste(&mut self, text: &str) {
        let Some(path) = self.capture.last().cloned() else { return };
        let Some(e) = self.widget(&path).and_then(|w| w.editbox()).cloned() else { return };
        let mut s: Vec<char> = text.chars().collect();
        if e.max_characters != 0 && s.len() > e.max_characters as usize {
            s.truncate(e.max_characters as usize);
        }
        if let Some(i) = s.iter().position(|c| *c == '\n') {
            s.truncate(i);
        }
        if s.is_empty() {
            return;
        }
        for ch in s {
            if ch.is_control() {
                break;
            }
            if !self.editbox_insert(&path, ch) {
                break;
            }
        }
        self.editbox_changed(&path);
        self.run_queue();
    }

    /// Edit box text.
    pub fn editbox_text(&self, path: &str) -> Option<String> {
        self.widget(path).and_then(|w| w.editbox()).map(|e| e.text.clone())
    }

    pub fn editbox_set_text(&mut self, path: &str, text: &str) {
        if let Some(e) = self.widget_mut(path).and_then(|w| w.editbox_mut()) {
            e.text = text.to_string();
            e.caret = text.chars().count();
            e.selection = None;
        }
        self.set_label(&format!("{}/Label", rel(path)), text);
    }

    pub fn editbox_set_max(&mut self, path: &str, max: i32) {
        if let Some(e) = self.widget_mut(path).and_then(|w| w.editbox_mut()) {
            e.max_characters = max;
        }
    }

    // ------------------------------------------------------------ frame

    /// Per-frame update (`vtable+0x0C` of the controls), then the queue.
    pub fn update(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
        self.update_controls();
        self.run_queue();
        self.layout();
    }

    /// Start a mover (`0x5C3260`, replacing none: the caller removes an
    /// existing one first as `0x602480` does).
    pub fn move_widget(&mut self, path: &str, from: (f32, f32), to: (f32, f32), seconds: f32) {
        let duration_ms = (seconds * 1000.0) as i64 as u32;
        self.movers.push(crate::ui::Mover { path: path.to_string(), from, to, start_ms: self.now_ms, duration_ms });
        self.set_flag(path, 0x20000, true);
    }

    /// The mover of a widget (`0x5C3310`).
    pub fn mover(&self, path: &str) -> Option<&crate::ui::Mover> {
        self.movers.iter().find(|m| m.path == path)
    }

    /// Remove a widget's mover (`0x5C3BA0`).
    pub fn remove_mover(&mut self, path: &str) {
        self.movers.retain(|m| m.path != path);
    }

    /// SetPosition `0x5EEF10`: raw L/T in reference units, the current
    /// size kept as relative R/B.
    pub fn set_position(&mut self, path: &str, x: f32, y: f32) {
        let g = self.geo();
        if let Some(w) = self.widget_mut(path) {
            let r = w.root.outer.rect;
            let (wd, ht) = ((r.r - r.l) / g.sx, (r.b - r.t) / g.sy);
            k::set_bounds(&mut w.root.outer, [x, y, wd, ht], 0x300);
        }
    }

    /// The raw position (reference units) of a widget (`0x5EE220`).
    pub fn raw_position(&self, path: &str) -> Option<(f32, f32)> {
        self.widget(path).map(|w| (w.root.outer.raw[0], w.root.outer.raw[1]))
    }

    /// Movers step (`0x5C3CF0` / `0x5C28B0` / `0x5C2800`): linear
    /// interpolation, removed once complete.
    fn update_movers(&mut self) {
        let now = self.now_ms;
        let movers = self.movers.clone();
        let mut keep = Vec::new();
        for m in movers {
            let t = m.progress(now).min(1.0);
            let x = (1.0 - t) * m.from.0 + t * m.to.0;
            let y = (1.0 - t) * m.from.1 + t * m.to.1;
            self.set_position(&m.path, x, y);
            if m.progress(now) < 1.0 {
                keep.push(m);
            } else {
                self.set_flag(&m.path, 0x20000, false);
            }
        }
        self.movers = keep;
        self.layout();
    }

    fn update_controls(&mut self) {
        if !self.movers.is_empty() {
            self.update_movers();
        }
        let mut lists = Vec::new();
        let mut edits = Vec::new();
        let mut logs = Vec::new();
        fn walk(w: &Widget, p: String, lists: &mut Vec<String>, edits: &mut Vec<String>, logs: &mut Vec<String>) {
            if w.flags & wf::ACTIVE != 0 {
                match w.class {
                    C::Listbox => lists.push(p.clone()),
                    C::Editbox => edits.push(p.clone()),
                    C::MessageLog => logs.push(p.clone()),
                    _ => {}
                }
            }
            for (c, s) in w.children.iter().zip(child_segments(w)) {
                if detached(w, c) {
                    continue;
                }
                walk(c, format!("{p}/{s}"), lists, edits, logs);
            }
        }
        let root = self.root.name.trim_start_matches('/').to_string();
        walk(&self.root, root, &mut lists, &mut edits, &mut logs);
        for p in logs {
            self.message_log_update(&p);
        }
        for dd in self.overlay.clone() {
            lists.push(format!("{dd}/Listbox"));
        }
        for p in lists {
            self.listbox_update(&p);
        }
        for p in edits {
            self.editbox_update(&p);
        }
    }

    /// Run the deferred callbacks (`0x5C3EB0` queue).
    pub fn run_queue(&mut self) {
        let mut guard = 0;
        while !self.queue.is_empty() && guard < 64 {
            guard += 1;
            for d in std::mem::take(&mut self.queue) {
                match d {
                    Deferred::SliderValue { path, value } => {
                        let cb = self.widget(&path).and_then(|w| w.slider()).map(|s| s.callback.clone()).unwrap_or_default();
                        match cb {
                            SliderCallback::None => {}
                            SliderCallback::Command(c) => {
                                let command = k::sprintf1(&c, &Arg::Float(value as f64));
                                self.events.push(UiEvent::Command { widget: path, command });
                            }
                            SliderCallback::Notify => self.events.push(UiEvent::Value { widget: path, value }),
                            SliderCallback::Owner => {
                                // Slider -> Scrollbar -> owner
                                let owner = parent_of(&path).and_then(parent_of).map(str::to_string);
                                if let Some(o) = owner {
                                    match self.widget(&o).map(|w| w.class) {
                                        Some(C::Listbox) => self.listbox_scrolled(&o, value),
                                        Some(C::Viewport) => self.viewport_scrolled(&o, value),
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    Deferred::ListCommand { path, callback, index, param } => match callback {
                        ListCallback::None => {}
                        ListCallback::Command(c) => {
                            let command = k::sprintf1(&c, &Arg::Str(param));
                            self.events.push(UiEvent::Command { widget: path, command });
                        }
                        ListCallback::DropDown => {
                            if let Some(dd) = parent_of(&path).map(str::to_string) {
                                self.dropdown_selected(&dd, index);
                            }
                        }
                    },
                }
            }
        }
        self.update_controls_light();
    }

    /// Rebind list rows after callbacks changed them.
    fn update_controls_light(&mut self) {
        let dirty: Vec<String> = {
            let mut v = Vec::new();
            fn walk(w: &Widget, p: String, out: &mut Vec<String>) {
                if w.class == C::Listbox && w.listbox().is_some_and(|l| l.first_dirty >= 0) && w.flags & wf::ACTIVE != 0 {
                    out.push(p.clone());
                }
                for (c, s) in w.children.iter().zip(child_segments(w)) {
                    walk(c, format!("{p}/{s}"), out);
                }
            }
            let root = self.root.name.trim_start_matches('/').to_string();
            walk(&self.root, root, &mut v);
            v
        };
        for p in dirty {
            self.listbox_update(&p);
        }
    }

    // ------------------------------------------------------------ pointer

    /// Topmost widget taking input under the point.
    pub fn pick(&self, x: f32, y: f32) -> Option<String> {
        fn rec(w: &Widget, p: String, x: f32, y: f32) -> Option<String> {
            if w.flags & wf::SHOWN == 0 || !w.loaded {
                return None;
            }
            if w.class.internals_ported() {
                let segs = child_segments(w);
                for (c, seg) in w.children.iter().zip(segs).rev() {
                    if detached(w, c) {
                        continue;
                    }
                    if let Some(hit) = rec(c, format!("{p}/{seg}"), x, y) {
                        return Some(hit);
                    }
                }
            }
            if interactive(w) && w.root.outer.rect.contains(x, y) {
                // the minimap picks only inside its diamond (0x5E9400)
                if let Some(crate::controls::Control::MiniMap(m)) = w.control.as_deref() {
                    let rect = w.sub_ve.as_ref().map(|v| v.layout.outer.rect).unwrap_or(w.root.outer.rect);
                    if !crate::minimap::inside(&crate::minimap::quad(m, rect), (x, y)) {
                        return None;
                    }
                }
                return Some(p);
            }
            None
        }
        for dd in self.overlay.iter().rev() {
            if let Some(lb) = self.widget(dd).and_then(|d| d.child("Listbox")) {
                if let Some(hit) = rec(lb, format!("{dd}/Listbox"), x, y) {
                    return Some(hit);
                }
            }
        }
        let root = self.root.name.trim_start_matches('/').to_string();
        match &self.modal {
            Some(m) => {
                let w = self.root.children.iter().find(|c| c.name.trim_start_matches('/') == m.trim_start_matches('/'))?;
                rec(w, format!("{root}/{}", w.name.trim_start_matches('/')), x, y)
            }
            None => rec(&self.root, root, x, y),
        }
    }

    /// Pointer moved: focus follows the hovered widget; a running grip
    /// drag follows the pointer.
    pub fn mouse_move(&mut self, x: f32, y: f32) {
        if let Some(mm) = self.minimap_drag.clone() {
            // 0x5D7830: jump only while the point maps inside the world
            if let Some((u, v)) = self.minimap_uv(&mm, x, y) {
                if (0.0..1.0).contains(&u) && (0.0..1.0).contains(&v) {
                    self.events.push(UiEvent::Minimap { widget: mm, u, v });
                }
            }
            return;
        }
        if self.drag.is_some() {
            self.grip_drag(x, y);
            self.run_queue();
            self.layout();
            return;
        }
        let hit = self.pick(x, y);
        if hit != self.hovered {
            if let Some(old) = self.hovered.take() {
                self.set_flag(&old, wf::FOCUSED, false);
            }
            if let Some(new) = &hit {
                self.set_flag(new, wf::FOCUSED, true);
            }
            self.hovered = hit;
        }
    }

    pub fn mouse_down(&mut self, x: f32, y: f32, button: MouseButton) {
        self.mouse_move(x, y);
        let hit = self.hovered.clone();
        // an open drop-down captures the pointer (0x5D2D90); a press outside
        // it and its list closes it (INFERRED)
        if let Some(cap) = self.capture.last().cloned() {
            let inside = hit.as_deref().is_some_and(|h| h == cap || h.starts_with(&format!("{cap}/")));
            match self.widget(&cap).map(|w| w.class) {
                Some(C::DropDown) if !inside => {
                    self.dropdown_close(&cap);
                    self.layout();
                    return;
                }
                Some(C::Editbox) if !inside => self.editbox_unfocus(&cap),
                _ => {}
            }
        }
        let Some(h) = hit else { return };
        let Some(w) = self.widget(&h) else { return };
        if w.flags & wf::ENABLED == 0 {
            return;
        }
        let class = w.class;
        match (class, button) {
            (C::PushButton | C::ToggleButton, MouseButton::Left) => {
                let snd = w.set_sound.clone();
                self.set_flag(&h, wf::PRESSED, true);
                self.pressed = Some(h);
                self.sound(&snd);
            }
            (C::Grip, MouseButton::Left) => self.grip_press(&h, x, y),
            (C::Slider | C::SliderAdvanced, MouseButton::Left) => self.slider_track_click(&h, x, y),
            (C::DropDown, MouseButton::Left) => self.dropdown_toggle(&h),
            (C::Editbox, MouseButton::Left) => {
                if self.capture.last().map(String::as_str) != Some(h.as_str()) {
                    self.editbox_focus(&h);
                }
            }
            (C::ListItem, _) => self.pressed = Some(h),
            (C::MiniMap, MouseButton::Left) => {
                // 0x5D77A0: jump (no bounds check) and start the drag tracker
                if let Some((u, v)) = self.minimap_uv(&h, x, y) {
                    self.events.push(UiEvent::Minimap { widget: h.clone(), u, v });
                }
                self.minimap_drag = Some(h);
            }
            (C::SelectablePanel, MouseButton::Left) => {
                let snd = w.set_sound.clone();
                self.select_panel(&h);
                self.sound(&snd);
            }
            _ => {}
        }
        self.run_queue();
        self.layout();
    }

    /// Select a selectable panel in its group (`0x5DCA90`): the others in
    /// the same parent and group lose SELECTED.
    pub fn select_panel(&mut self, path: &str) {
        let Some(w) = self.widget(path) else { return };
        let (group, tag) = (w.radio_group, w.tag);
        if let Some(parent) = parent_of(path).map(str::to_string) {
            if let Some(pw) = self.widget_mut(&parent) {
                for c in &mut pw.children {
                    if c.class == C::SelectablePanel && c.radio_group == group {
                        c.flags &= !wf::SELECTED;
                    }
                }
            }
        }
        self.set_flag(path, wf::SELECTED, true);
        self.events.push(UiEvent::Selected { widget: path.to_string(), tag });
    }

    /// Screen point to world fractions of a minimap (`0x5D74B0`).
    pub fn minimap_uv(&self, path: &str, x: f32, y: f32) -> Option<(f32, f32)> {
        let w = self.widget(path)?;
        let crate::controls::Control::MiniMap(m) = w.control.as_deref()? else { return None };
        let rect = w.sub_ve.as_ref().map(|v| v.layout.outer.rect).unwrap_or(w.root.outer.rect);
        Some(crate::minimap::inverse(&crate::minimap::quad(m, rect), (x.trunc(), y.trunc())))
    }

    /// Path of the widget carrying `tag` (`Root/...` with `#k` segments).
    pub fn find_tag(&self, tag: u64) -> Option<String> {
        fn rec(w: &Widget, p: String, tag: u64) -> Option<String> {
            if w.tag == tag {
                return Some(p);
            }
            for (c, s) in w.children.iter().zip(child_segments(w)) {
                if let Some(hit) = rec(c, format!("{p}/{s}"), tag) {
                    return Some(hit);
                }
            }
            None
        }
        let root = self.root.name.trim_start_matches('/').to_string();
        rec(&self.root, root, tag)
    }

    /// Viewport Clear (vtable+0xCC `0x5ED880`): the items leave the item
    /// panel (returned to the caller's pools); total height 0.
    pub fn viewport_clear(&mut self, path: &str) -> Vec<Widget> {
        let items = self.widget_mut(&format!("{path}/ItemPanel")).map(|p| std::mem::take(&mut p.children)).unwrap_or_default();
        if let Some(v) = self.widget_mut(path).and_then(|w| w.viewport_mut()) {
            v.total = 0.0;
            v.scroll = 0.0;
        }
        self.viewport_update(path, false);
        items
    }

    pub fn mouse_up(&mut self, x: f32, y: f32, button: MouseButton) {
        if button == MouseButton::Left && self.minimap_drag.take().is_some() {
            return;
        }
        if let Some((grip, _, _)) = self.drag.take() {
            self.set_flag(&grip, wf::PRESSED, false);
            self.layout();
            return;
        }
        self.mouse_move(x, y);
        let Some(p) = self.pressed.take() else { return };
        let same = self.hovered.as_deref() == Some(p.as_str());
        let Some(w) = self.widget(&p) else { return };
        let class = w.class;
        if class == C::ListItem {
            // item vtable+0x34 (0x5D55A0): 0x100/0x101 + release
            let idx = w.list_item().map(|i| i.index).unwrap_or(-1);
            if same && idx >= 0 {
                if let Some(lb) = parent_of(&p).map(str::to_string) {
                    self.listbox_select(&lb, idx as usize, false);
                    let which = if button == MouseButton::Right { 2 } else { 0 };
                    self.listbox_fire(&lb, idx as usize, which);
                }
            }
            self.run_queue();
            self.layout();
            return;
        }
        if button != MouseButton::Left {
            return;
        }
        self.set_flag(&p, wf::PRESSED, false);
        if !same {
            self.layout();
            return;
        }
        let (command, group, unset) = match self.widget(&p) {
            Some(w) => (w.command.clone(), w.radio_group, w.unset_sound.clone()),
            None => return,
        };
        if class == C::ToggleButton {
            let sel = self.widget(&p).is_some_and(|w| w.flags & wf::SELECTED != 0);
            if group != 0 {
                // radio group (0x5F0420): the clicked toggle is selected
                if let Some(parent) = parent_of(&p).map(str::to_string) {
                    if let Some(pw) = self.widget_mut(&parent) {
                        for c in &mut pw.children {
                            if c.radio_group == group {
                                c.flags &= !wf::SELECTED;
                            }
                        }
                    }
                }
                self.set_flag(&p, wf::SELECTED, true);
            } else if !self.widget(&p).is_some_and(|w| w.no_auto_toggle) {
                self.set_flag(&p, wf::SELECTED, !sel);
                if sel {
                    self.sound(&unset);
                }
            }
        }
        // numeric edit box arrows (0x5D79F0 / 0x5D7AC0)
        if let Some(ne) = parent_of(&p).map(str::to_string) {
            if self.widget(&ne).is_some_and(|w| w.class == C::NumericEditbox) {
                let name = p.rsplit('/').next().unwrap_or("").to_string();
                if name.eq_ignore_ascii_case("UpButton") || name.eq_ignore_ascii_case("DownButton") {
                    self.numeric_step(&ne, name.eq_ignore_ascii_case("UpButton"));
                    self.layout();
                    return;
                }
            }
        }
        // scroll bar buttons (0x5DDDD0 / 0x5DDDE0)
        if let Some(sb) = parent_of(&p).map(str::to_string) {
            if self.widget(&sb).is_some_and(|w| w.class == C::Scrollbar) {
                let name = p.rsplit('/').next().unwrap_or("");
                if name.eq_ignore_ascii_case("LessButton") || name.eq_ignore_ascii_case("MoreButton") {
                    self.scrollbar_step(&sb, name.eq_ignore_ascii_case("MoreButton"));
                    self.run_queue();
                    self.layout();
                    return;
                }
            }
        }
        if let Some(c) = command {
            // message box buttons (0x66D7C0): the request's command
            if let Some(i) = c.strip_prefix("MessageBoxButtonAction ").and_then(|i| i.parse::<usize>().ok()) {
                if let Some(req) = self.message_box.clone() {
                    self.close_message_box();
                    if let Some((_, cmd)) = req.buttons.get(i) {
                        self.events.push(UiEvent::Command { widget: p, command: cmd.clone() });
                    }
                    self.layout();
                    return;
                }
            }
            self.events.push(UiEvent::Command { widget: p, command: c });
        }
        self.run_queue();
        self.layout();
    }

    /// A double click (event 0x108 left / 0x109 right) in place of the
    /// second press. Sliders, grips and drop-downs treat 0x108 as a press
    /// (0x5DDFB0, 0x5DDC00, 0x5D3AD0); an owned list item selects and fires
    /// its select command, then the double-click command (0x5D55A0: +0x9C
    /// then +0xA0, right: +0xA8); the release that follows is a normal
    /// release. Other widgets take it as a press (INFERRED).
    pub fn mouse_double(&mut self, x: f32, y: f32, button: MouseButton) {
        self.mouse_move(x, y);
        let Some(h) = self.hovered.clone() else { return };
        let Some(w) = self.widget(&h) else { return };
        if w.class == C::ListItem {
            let idx = w.list_item().map(|i| i.index).unwrap_or(-1);
            if idx >= 0 {
                if let Some(lb) = parent_of(&h).map(str::to_string) {
                    self.listbox_select(&lb, idx as usize, false);
                    self.listbox_fire(&lb, idx as usize, 0);
                    self.listbox_fire(&lb, idx as usize, if button == MouseButton::Right { 3 } else { 1 });
                }
            }
            self.pressed = Some(h);
            self.run_queue();
            self.layout();
            return;
        }
        self.mouse_down(x, y, button);
    }

    /// Mouse wheel (0x110): viewports step while their scroll bar is
    /// visible (`0x5ED940`).
    pub fn mouse_wheel(&mut self, x: f32, y: f32, delta: f32) {
        // deepest shown widget under the point
        fn any(w: &Widget, p: String, x: f32, y: f32) -> Option<String> {
            if w.flags & wf::SHOWN == 0 || !w.loaded {
                return None;
            }
            for (c, seg) in w.children.iter().zip(child_segments(w)).rev() {
                if detached(w, c) {
                    continue;
                }
                if let Some(hit) = any(c, format!("{p}/{seg}"), x, y) {
                    return Some(hit);
                }
            }
            w.root.outer.rect.contains(x, y).then_some(p)
        }
        let root = self.root.name.trim_start_matches('/').to_string();
        let mut at = any(&self.root, root, x, y);
        while let Some(p) = at {
            if self.widget(&p).is_some_and(|w| w.class == C::Viewport) {
                let visible = self.widget(&format!("{p}/Scrollbar")).is_some_and(|w| w.flags & 1 != 0);
                if visible {
                    self.viewport_step(&p, delta < 0.0);
                    self.run_queue();
                    self.layout();
                }
                return;
            }
            at = parent_of(&p).map(str::to_string);
        }
    }
}
