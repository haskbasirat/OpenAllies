//! Class internals of the native controls: slider (`0x5DE360`, vtable
//! 0xA500E8), advanced slider (`0x5DEB20`, vtable 0xA504F8), slider grip
//! (`0x5DE270`, vtable 0xA4FF58), scroll bar (`0x5DE560`, vtable
//! 0xA50020), list box (`0x5D4C80`, vtable 0xA4E668), list item
//! (`0x5D3D70`, vtable 0xA4E758), drop-down (`0x5D5C30`, vtable 0xA4E588),
//! edit box (`0x5D1000`, vtable 0xA4E088) and viewport (`0x5EDAA0`,
//! vtable 0xA52108).
//!
//! Conventions (native): a widget's root `VE.outer` rect is in screen
//! pixels; reference units are pixels divided by the interface scale
//! (`0x5E18E0`..`0x5E1940`: L/sx, T/sy, R/sx, B/sy). Bounds written by code
//! go through `0x5E2940(rect, unit flags)` (raw L,T,R,B and the `0x5E3150`
//! flags) and are re-laid out. All arithmetic is f32 (the UI thread runs
//! the x87 at PC24). Value/command callbacks are posted to the UI queue
//! (`0x5C3EB0`) and run after the input event (`Ui::run_queue`).
//!
//! This module holds the per-class state and the arithmetic; the input
//! routing and queue live in `ui_controls.rs`.

use crate::layout::Bounds;
use crate::node::UiNode;
use crate::widget::Widget;
use serde::{Deserialize, Serialize};

/// `0xA4FF54`: the slider value tolerance.
pub const EPS: f32 = 1.0e-4;

/// Interface scale (`ui_scale`).
#[derive(Clone, Copy, Debug)]
pub struct Geo {
    pub sx: f32,
    pub sy: f32,
}

/// Write raw bounds and unit flags (`0x5E2940`).
pub fn set_bounds(b: &mut Bounds, raw: [f32; 4], flags: u32) {
    b.raw = raw;
    b.flags = flags;
}

/// What a slider does when its value changes through input
/// (`SetValueNotify` `0x5DDCA0` posts `+0x9C(value)`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum SliderCallback {
    /// No functor.
    #[default]
    None,
    /// Console command functor (`0x5DE150`): `swprintf(cmd, (double)v)`.
    Command(String),
    /// A code functor (options dialog callbacks): reported as
    /// [`crate::ui::UiEvent::Value`].
    Notify,
    /// The owner of a scroll bar (list box `0x5D2CC0`, viewport
    /// `0x5ED980`).
    Owner,
}

/// Label formatters of the advanced slider (`0x5DEA20` default, the
/// options dialog's own ones).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SliderFormat {
    /// `0x5DEA20`: `"%d%%"` of `_ftol(v*100)` when the range is [0,1], else
    /// `"%.0f"`.
    #[default]
    Default,
    /// `0x675E10`: `"%d"` of `floor(v + 0.5)`.
    Rounded,
    /// `0x679470`: min `options_fast`, max `options_slow`, else "".
    FastSlow,
    /// `0x6795B0`: min `options_slow`, max `options_fast`.
    SlowFast,
    /// `0x6796F0`: min `options_in`, max `options_out`.
    InOut,
    /// `0x675EC0`: `"%d%%"` of `floor(2^v * 100 + 0.5)`.
    GameSpeed,
    /// `0x679830`: min `options_none`, max `options_high`.
    NoneHigh,
    /// `0x679970`: min `options_none`, max `options_full`.
    NoneFull,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Slider {
    /// Fields +0x74..+0xA4 (`LoadData` `0x5DE800`).
    pub usage_pcg: [f32; 2],
    pub usage_abs: [f32; 2],
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub page: f32,
    /// Grip `+0x78` (`grip_display_size`).
    pub grip_size: f32,
    pub sound_clicked: String,
    pub callback: SliderCallback,
    /// Advanced slider (`show_minmax`, `show_current_val`, formatter).
    pub advanced: Option<(bool, bool, SliderFormat)>,
    /// Grip `+0x74`: grab offset of the running drag (`0x5DD5D0`).
    pub grab: f32,
}

impl Default for Slider {
    fn default() -> Self {
        Slider {
            usage_pcg: [0.0, 1.0],
            usage_abs: [0.0, 0.0],
            value: 0.0,
            min: 0.0,
            max: 1.0,
            // 0x5DE360 sets step 0 (continuous)
            step: 0.0,
            page: 0.25,
            grip_size: 10.0,
            sound_clicked: String::new(),
            callback: SliderCallback::None,
            advanced: None,
            grab: 0.0,
        }
    }
}

/// One list box entry (`ItemEntry`: param, localized text; flags bit0 =
/// valid, bit1 = dirty, bit2 = selected).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListEntry {
    pub param: String,
    pub text: String,
    pub flags: u32,
    /// Entry +0x1C: the item label colour the bind (`0x5D3E10` ->
    /// `0x5EDFE0` -> `0x5E13A0`) writes over the ItemLook label's TGI
    /// colour; the entry ctor `0x5D3CC0` sets white (`0xB322C0`).
    #[serde(default = "white")]
    pub color: [f32; 4],
}

fn white() -> [f32; 4] {
    [1.0; 4]
}

impl ListEntry {
    pub fn new(text: &str, param: &str) -> Self {
        ListEntry { param: param.to_string(), text: text.to_string(), flags: 1, color: white() }
    }
    pub fn selected(&self) -> bool {
        self.flags & 4 != 0
    }
}

/// List box commands (`0x5D5740`): select (+0x9C), double click (+0xA0),
/// right click (+0xA4), right double click (+0xA8).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum ListCallback {
    #[default]
    None,
    /// Console command `swprintf(cmd, param_or_text)`.
    Command(String),
    /// The drop-down owning this list box (`0x5D5540`).
    DropDown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Listbox {
    pub items: Vec<ListEntry>,
    /// Item widgets created so far (children named `ItemLook`).
    pub slots: usize,
    pub selection: Vec<usize>,
    pub item_height: f32,
    pub top: i32,
    /// `+0xB0`.
    pub num_visible: i32,
    /// `+0xB4` (`num_visible_items`).
    pub template_visible: i32,
    pub vertical_item_offset: f32,
    pub multiselect: bool,
    pub ignore_redundant_select: bool,
    pub resize_to_fit: bool,
    /// `+0xC4`, -1 = none.
    pub first_dirty: i32,
    pub on_select: ListCallback,
    pub on_double: ListCallback,
    pub on_right: ListCallback,
    pub on_right_double: ListCallback,
    /// The list box node clone (`+0x98`) the item looks load from.
    pub node: Option<UiNode>,
}

impl Default for Listbox {
    fn default() -> Self {
        Listbox {
            items: Vec::new(),
            slots: 0,
            selection: Vec::new(),
            item_height: 0.0,
            top: 0,
            num_visible: 0,
            template_visible: 0,
            vertical_item_offset: 0.0,
            multiselect: false,
            ignore_redundant_select: true,
            resize_to_fit: true,
            first_dirty: -1,
            on_select: ListCallback::None,
            on_double: ListCallback::None,
            on_right: ListCallback::None,
            on_right_double: ListCallback::None,
            node: None,
        }
    }
}

/// List item (label subclass): owner set for list rows, unset for a
/// drop-down's SelectedItem.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    pub owned: bool,
    pub index: i32,
    pub param: String,
    pub text: String,
    /// Text was truncated with the ellipsis (`+0x7C`).
    pub truncated: bool,
}

impl Default for ListItem {
    fn default() -> Self {
        ListItem { owned: false, index: -1, param: String::new(), text: String::new(), truncated: false }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct DropDown {
    pub dropup: bool,
    pub sound_open: String,
    pub sound_close: String,
    /// `+0x78` = the list box's `num_visible_items`.
    pub max_visible: i32,
    /// `+0x7C`: `swprintf(cmd, SelectedItem.param)`.
    pub command: Option<String>,
    /// Owned drop-down (`0x5DC4B0`): the selection goes to the owner's
    /// functor (reported as [`crate::ui::UiEvent::Select`]) instead of a
    /// console command.
    #[serde(default)]
    pub functor: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Editbox {
    pub text: String,
    pub max_characters: i32,
    pub integer_only: bool,
    pub integer_max: i32,
    /// `+0x84` commit and `+0x88` change commands.
    pub commit_command: Option<String>,
    pub change_command: Option<String>,
    pub caret: usize,
    /// Selected range (start, end) in characters.
    pub selection: Option<(usize, usize)>,
    pub committed: bool,
    /// `+0xAD`.
    pub commit_on_unfocus: bool,
    /// Caret blink timer start (ms).
    pub blink_ms: u64,
    /// Horizontal text scroll in pixels (`text+0x74`).
    pub scroll: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Viewport {
    /// `+0x74` total item height (reference units).
    pub total: f32,
    pub at_bottom: bool,
    pub bottom_anchored: bool,
    pub scroll: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Control {
    /// Numeric edit box step (`+0x80`).
    Numeric(i32),
    /// Message log (a viewport, `message_log.rs`).
    MessageLog(crate::message_log::MessageLogState),
    /// A message log row.
    MessageRow(crate::message_log::MessageRow),
    /// ModelWidget: the model id shown (renderer side).
    Model(String),
    /// The minimap (`minimap.rs`).
    MiniMap(crate::minimap::MiniMap),
    Slider(Slider),
    Listbox(Listbox),
    ListItem(ListItem),
    DropDown(DropDown),
    Editbox(Editbox),
    Viewport(Viewport),
}

impl Widget {
    pub fn slider(&self) -> Option<&Slider> {
        match self.control.as_deref() {
            Some(Control::Slider(s)) => Some(s),
            _ => None,
        }
    }
    pub fn slider_mut(&mut self) -> Option<&mut Slider> {
        match self.control.as_deref_mut() {
            Some(Control::Slider(s)) => Some(s),
            _ => None,
        }
    }
    pub fn listbox(&self) -> Option<&Listbox> {
        match self.control.as_deref() {
            Some(Control::Listbox(s)) => Some(s),
            _ => None,
        }
    }
    pub fn listbox_mut(&mut self) -> Option<&mut Listbox> {
        match self.control.as_deref_mut() {
            Some(Control::Listbox(s)) => Some(s),
            _ => None,
        }
    }
    pub fn list_item(&self) -> Option<&ListItem> {
        match self.control.as_deref() {
            Some(Control::ListItem(s)) => Some(s),
            _ => None,
        }
    }
    pub fn list_item_mut(&mut self) -> Option<&mut ListItem> {
        match self.control.as_deref_mut() {
            Some(Control::ListItem(s)) => Some(s),
            _ => None,
        }
    }
    pub fn dropdown(&self) -> Option<&DropDown> {
        match self.control.as_deref() {
            Some(Control::DropDown(s)) => Some(s),
            _ => None,
        }
    }
    pub fn editbox(&self) -> Option<&Editbox> {
        match self.control.as_deref() {
            Some(Control::Editbox(s)) => Some(s),
            _ => None,
        }
    }
    pub fn editbox_mut(&mut self) -> Option<&mut Editbox> {
        match self.control.as_deref_mut() {
            Some(Control::Editbox(s)) => Some(s),
            _ => None,
        }
    }
    pub fn viewport(&self) -> Option<&Viewport> {
        match self.control.as_deref() {
            Some(Control::Viewport(s)) => Some(s),
            Some(Control::MessageLog(s)) => Some(&s.vp),
            _ => None,
        }
    }
    pub fn viewport_mut(&mut self) -> Option<&mut Viewport> {
        match self.control.as_deref_mut() {
            Some(Control::Viewport(s)) => Some(s),
            Some(Control::MessageLog(s)) => Some(&mut s.vp),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------- slider

fn vertical(w: &Widget) -> bool {
    w.flags & crate::state::widget_flag::VERTICAL != 0
}

/// Slider extent in reference units along its axis (`0x5E18E0` family).
fn axis_ref(w: &Widget, g: Geo) -> (f32, f32) {
    let r = w.root.outer.rect;
    if vertical(w) { (r.t / g.sy, r.b / g.sy) } else { (r.l / g.sx, r.r / g.sx) }
}

/// Usage area in absolute reference units (`0x5DD2E0`, `0x5DD340(0)`,
/// `0x5DD420(0)`). `Wpx` is the slider's unscaled pixel width even when
/// the slider is vertical (native quirk).
pub fn usage_abs(w: &Widget, s: &Slider, g: Geo) -> (f32, f32) {
    let (lr, rr) = axis_ref(w, g);
    let lerp = |p: f32| (1.0 - p) * lr + rr * p;
    let r = w.root.outer.rect;
    let wpx = (r.r - r.l).abs();
    let ax = s.usage_abs[0];
    let ay = s.usage_abs[1];
    let start = lerp(s.usage_pcg[0]) + if ax > 0.0 { ax } else if ax < 0.0 { wpx + ax } else { 0.0 };
    let end = lerp(s.usage_pcg[1]) + if ay > 0.0 { -(wpx - ay) } else if ay < 0.0 { ay } else { 0.0 };
    (start, end)
}

/// Usage area relative to the slider (`0x5DD340(1)`, `0x5DD420(1)`):
/// absolute minus `(1/s) * outer.L` (or T).
pub fn usage_rel(w: &Widget, s: &Slider, g: Geo) -> (f32, f32) {
    let (start, end) = usage_abs(w, s, g);
    let r = w.root.outer.rect;
    let o = if vertical(w) { (1.0 / g.sy) * r.t } else { (1.0 / g.sx) * r.l };
    (start - o, end - o)
}

/// Snap and clamp (`0x5DD760`).
pub fn snap_clamp(s: &Slider, v: f32, snap: bool) -> f32 {
    let mut v = v;
    if snap && s.step != 0.0 {
        // PC24: the quotient and the +0.5 round to single precision
        v = ((v / s.step) + 0.5).floor() * s.step;
    }
    let (lo, hi) = if s.min > s.max { (s.max, s.min) } else { (s.min, s.max) };
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

/// `UpdateGrip` (`0x5DD960`): place the grip along the usage area. The
/// position is 0 unless the slider is laid out and active.
pub fn update_grip(w: &mut Widget, g: Geo) {
    let Some(s) = w.slider().cloned() else { return };
    let active = w.root.flags & crate::layout::ve_flag::LAID_OUT != 0
        && w.flags & crate::state::widget_flag::ACTIVE != 0;
    let size = s.grip_size;
    let pos = if active {
        let (sr, er) = usage_rel(w, &s, g);
        if s.min != s.max {
            let t = (s.value - s.min) / (s.max - s.min);
            (1.0 - t) * sr + t * (er - size)
        } else {
            (sr + (er - size)) * 0.5
        }
    } else {
        0.0
    };
    let vert = vertical(w);
    if let Some(grip) = w.child_mut("Grip") {
        if vert {
            set_bounds(&mut grip.root.outer, [0.0, pos, 0.0, size], 0x204);
        } else {
            set_bounds(&mut grip.root.outer, [pos, 0.0, size, 0.0], 0x108);
        }
    }
}

/// Background placement (`0x5DD670`): the usage area, relative to the
/// slider; the cross extent is the slider's pixel size (native quirk).
pub fn layout_background(w: &mut Widget, g: Geo) {
    let Some(s) = w.slider().cloned() else { return };
    let (sr, er) = usage_rel(w, &s, g);
    let r = w.root.outer.rect;
    let (wpx, hpx) = ((r.r - r.l).abs(), (r.b - r.t).abs());
    let vert = vertical(w);
    if let Some(bg) = &mut w.sub_ve {
        if vert {
            set_bounds(&mut bg.layout.outer, [0.0, sr, wpx, er], 0);
        } else {
            set_bounds(&mut bg.layout.outer, [sr, 0.0, er, hpx], 0);
        }
    }
}

/// `SetValue` (`0x5DDAC0`): snap/clamp; unchanged within `EPS` -> false;
/// else store and run UpdateGrip (vtable+0xE0; the advanced slider also
/// refreshes its labels, done by the caller).
pub fn set_value(w: &mut Widget, v: f32, snap: bool, g: Geo) -> bool {
    let Some(s) = w.slider() else { return false };
    let x = snap_clamp(s, v, snap);
    if (x - s.value).abs() < EPS {
        return false;
    }
    if let Some(s) = w.slider_mut() {
        s.value = x;
    }
    update_grip(w, g);
    true
}

/// `SetMin` (`0x5DDB20`). Returns whether the value changed.
pub fn set_min(w: &mut Widget, v: f32, g: Geo) -> bool {
    let Some(s) = w.slider_mut() else { return false };
    if (v - s.min).abs() < EPS {
        return false;
    }
    s.min = v;
    let below = s.value < v;
    if below && set_value(w, v, true, g) {
        return true;
    }
    update_grip(w, g);
    false
}

/// `SetMax` (`0x5DDB90`).
pub fn set_max(w: &mut Widget, v: f32, g: Geo) -> bool {
    let Some(s) = w.slider_mut() else { return false };
    if (v - s.max).abs() < EPS {
        return false;
    }
    s.max = v;
    let above = s.value > v;
    if above && set_value(w, v, true, g) {
        return true;
    }
    update_grip(w, g);
    false
}

/// `PosToValue` (`0x5DD870`) for a reference-unit position `m`.
pub fn pos_to_value(w: &Widget, s: &Slider, m: f32, g: Geo) -> f32 {
    let (start, end) = usage_abs(w, s, g);
    if s.min == s.max {
        return s.min;
    }
    if m < start {
        s.min
    } else if m > end {
        s.max
    } else {
        snap_clamp(s, s.min + ((m - start) / (end - start)) * (s.max - s.min), true)
    }
}

/// Grip extent along the axis in reference units.
pub fn grip_ref(w: &Widget, g: Geo) -> Option<(f32, f32)> {
    let grip = w.child("Grip")?;
    let r = grip.root.outer.rect;
    Some(if vertical(w) { (r.t / g.sy, r.b / g.sy) } else { (r.l / g.sx, r.r / g.sx) })
}

/// Default advanced-slider formatter (`0x5DEA20`) and the options
/// dialog's ones; `loc` resolves a string key.
pub fn format_value(f: SliderFormat, v: f32, s: &Slider, loc: &dyn Fn(&str) -> String) -> String {
    let ends = |lo: &str, hi: &str| {
        if v == s.min {
            loc(lo)
        } else if v == s.max {
            loc(hi)
        } else {
            String::new()
        }
    };
    match f {
        SliderFormat::Default => {
            if s.min == 0.0 && s.max == 1.0 {
                format!("{}%", (v * 100.0) as i32)
            } else {
                format!("{:.0}", v as f64)
            }
        }
        SliderFormat::Rounded => format!("{}", ((v as f64) + 0.5).floor() as i32),
        SliderFormat::FastSlow => ends("options_fast", "options_slow"),
        SliderFormat::SlowFast => ends("options_slow", "options_fast"),
        SliderFormat::InOut => ends("options_in", "options_out"),
        SliderFormat::GameSpeed => format!("{}%", ((2f64.powf(v as f64)) * 100.0 + 0.5).floor() as i32),
        SliderFormat::NoneHigh => ends("options_none", "options_high"),
        SliderFormat::NoneFull => ends("options_none", "options_full"),
    }
}

// ---------------------------------------------------------------- list box

/// Height formula of a resized list box (`0x5D58E0`, `0x5D4F30`,
/// `0x5D3450`): B = n * item_height + 2 * offset as a size.
pub fn apply_list_height(w: &mut Widget, n: i32) {
    let Some(lb) = w.listbox() else { return };
    let b = n as f32 * lb.item_height + (lb.vertical_item_offset + lb.vertical_item_offset);
    let o = &mut w.root.outer;
    o.flags = (o.flags & !0x88) | 0x200;
    o.raw[3] = b;
}

/// Item widget bounds (`0x5D48E0`): the ItemLook's L/R kept, T = slot *
/// item_height + offset, B = item_height as a size.
pub fn place_item(item: &mut Widget, slot: usize, item_height: f32, offset: f32) {
    let o = &mut item.root.outer;
    o.raw[1] = slot as f32 * item_height + offset;
    o.raw[3] = item_height;
    o.flags = (o.flags & !0xAA) | 0x200;
}

/// List box comparator (`0x5D32A0`): invalid entries first, then
/// case-insensitive text order.
pub fn compare_entries(a: &ListEntry, b: &ListEntry) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    if a.flags & 1 == 0 {
        return Ordering::Less;
    }
    if b.flags & 1 == 0 {
        return Ordering::Greater;
    }
    let la: Vec<u16> = a.text.to_lowercase().encode_utf16().collect();
    let lb: Vec<u16> = b.text.to_lowercase().encode_utf16().collect();
    la.cmp(&lb)
}

// ---------------------------------------------------------------- C printf

/// `swprintf` with one argument (`%s`, `%d`/`%i`, `%f`/`%.Nf`/`%g`, `%%`).
pub fn sprintf1(fmt: &str, arg: &Arg) -> String {
    let mut out = String::new();
    let mut it = fmt.chars().peekable();
    let mut used = false;
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let mut spec = String::new();
        while let Some(&n) = it.peek() {
            spec.push(n);
            it.next();
            if n.is_ascii_alphabetic() || n == '%' {
                break;
            }
        }
        let conv = spec.chars().last().unwrap_or('%');
        if conv == '%' {
            out.push('%');
            continue;
        }
        if used {
            // the original would read garbage from the stack: emit nothing
            continue;
        }
        used = true;
        let prec = spec.split_once('.').and_then(|(_, p)| p.trim_end_matches(|c: char| c.is_ascii_alphabetic()).parse::<usize>().ok());
        match (conv, arg) {
            ('s' | 'S', Arg::Str(s)) => out.push_str(s),
            ('d' | 'i', Arg::Int(i)) => out.push_str(&i.to_string()),
            ('d' | 'i', Arg::Float(f)) => out.push_str(&(*f as i64).to_string()),
            ('f', Arg::Float(f)) => out.push_str(&format!("{:.*}", prec.unwrap_or(6), f)),
            ('f', Arg::Int(i)) => out.push_str(&format!("{:.*}", prec.unwrap_or(6), *i as f64)),
            ('g', Arg::Float(f)) => out.push_str(&format!("{f}")),
            (_, Arg::Str(s)) => out.push_str(s),
            (_, Arg::Int(i)) => out.push_str(&i.to_string()),
            (_, Arg::Float(f)) => out.push_str(&format!("{f}")),
        }
    }
    out
}

/// One `swprintf` argument.
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Str(String),
    Int(i64),
    Float(f64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printf_subset() {
        assert_eq!(sprintf1("OptionsHotkeySelect %s", &Arg::Str("classic".into())), "OptionsHotkeySelect classic");
        assert_eq!(sprintf1("OptionsChangeDropDown", &Arg::Float(0.5)), "OptionsChangeDropDown");
        assert_eq!(sprintf1("Vol %f", &Arg::Float(0.5)), "Vol 0.500000");
        assert_eq!(sprintf1("%d%%", &Arg::Int(80)), "80%");
    }

    #[test]
    fn snap_and_clamp() {
        let s = Slider { min: 30.0, max: 180.0, step: 30.0, ..Default::default() };
        assert_eq!(snap_clamp(&s, 44.0, true), 30.0);
        assert_eq!(snap_clamp(&s, 46.0, true), 60.0);
        assert_eq!(snap_clamp(&s, 400.0, true), 180.0);
        let r = Slider { min: 1.0, max: 0.0, ..Default::default() };
        assert_eq!(snap_clamp(&r, 2.0, false), 1.0);
    }

    #[test]
    fn default_formatter() {
        let s = Slider { min: 0.0, max: 1.0, ..Default::default() };
        let loc = |k: &str| k.to_string();
        assert_eq!(format_value(SliderFormat::Default, 0.8, &s, &loc), "80%");
        let s2 = Slider { min: 30.0, max: 180.0, ..Default::default() };
        assert_eq!(format_value(SliderFormat::Default, 90.0, &s2, &loc), "90");
        assert_eq!(format_value(SliderFormat::FastSlow, 30.0, &s2, &loc), "options_fast");
        assert_eq!(format_value(SliderFormat::GameSpeed, 1.0, &s2, &loc), "200%");
    }
}
