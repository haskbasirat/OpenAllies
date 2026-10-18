//! The message system (`KKC_MessageSystem`, `0xB399B0`,
//! `.\Source\Main\MessageSystem.cpp`) and the message log widget
//! (`MessageLog` ctor `0x6707F0`, vtable 0xA6AD10, a viewport; rows
//! `MessageWidget` ctor `0x670B20`).
//!
//! * Send `0x40B5C0`: a type whose filter UVar is false drops the message;
//!   otherwise id = ++last, the message goes to the head and the oldest is
//!   trimmed at UVar `MessageLogMaximumLines`.
//! * Log Update `0x671230`: generation reset, faded rows leave the viewport,
//!   the row pool follows the max-lines UVar, new messages are shown
//!   (`0x670A10`, least recently used row, viewport AddItem at the bottom).
//! * SetMessage `0x670450`: background alpha = `transparency`, text colour
//!   = the type colour, icon = the type icon, height = max(icon height,
//!   wrapped text height) (`0x735C00`), button command
//!   `MessageLogAction <id>`, fade timer restarted.
//! * Row fade `0x66F9A0`: opaque for `UIMessageLogFadeTime` ms, then
//!   `a = 1 - d * 0.001` for one second, then hidden.
//!
//! The camera jump of `MessageLogAction` and `CycleMessageLog` belong to the
//! world (the commands are reported to the caller). The HUD log's auto
//! height (`0x5C2220`) needs the control panel's top: callers pass it as
//! [`MessageLogState::auto_height_limit`].

use crate::ui::Ui;
use crate::ve::VeKind;
use crate::widget::{Widget, WidgetClass as C};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// `Data/Game/message_types.tgi` (loader `0x48D0A0`).
#[derive(Clone, Debug, PartialEq)]
pub struct MessageType {
    pub ids: String,
    pub icon: Option<String>,
    /// UVar name of the filter, `None` for NULL.
    pub filter: Option<String>,
    pub color: [f32; 4],
    pub sound: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: u32,
    pub kind: String,
    pub text: String,
    pub location: Option<(f32, f32)>,
    ///Exact40B7C0 payload on the source-bound live path. Diagnostic UI
    ///messages and deserialized display records have no native payload.
    #[serde(skip)]
    pub(crate) native:Option<crate::native_messages::Header>,
}
impl Message {pub fn native_header(&self)->Option<&crate::native_messages::Header>{self.native.as_ref()}}

#[derive(Debug)]
pub struct MessageSystem {
    pub types: HashMap<String, MessageType>,
    /// Newest first.
    pub messages: VecDeque<Message>,
    pub generation: u32,
    pub last: u32,
    ///Native+30 is distinct from physical list length after a caught child
    ///failure. It is never reconstructed from an externally mutable list.
    pub(crate) count_30:u32,
    pub(crate) native:Option<crate::native_messages::Binding>,
    pub(crate) native_constructor:bool,
}
impl Default for MessageSystem{
    fn default()->Self{Self{types:HashMap::new(),messages:VecDeque::new(),generation:0,last:0,count_30:0,native:None,native_constructor:true}}
}
impl Clone for MessageSystem {
    fn clone(&self)->Self{Self{types:self.types.clone(),messages:self.messages.clone(),generation:self.generation,last:self.last,
        count_30:self.count_30,native:None,native_constructor:false}}
}

impl MessageSystem {
    pub fn load(store: &mut crate::tgi::TgiStore) -> Self {
        let mut types = HashMap::new();
        if let Ok(root) = store.load_file("Game/message_types.tgi") {
            for n in root.children.iter().filter(|c| c.name.eq_ignore_ascii_case("MessageType")) {
                let ids = n.string_or_empty("IDS");
                let icon = n.string("icon").ok().filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null"));
                let filter = n.string("filter").ok().filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null"));
                let c = n.point3("color").unwrap_or([1.0, 1.0, 1.0]);
                types.insert(
                    ids.to_ascii_uppercase(),
                    MessageType {
                        ids,
                        // the icon path is relative to Data/Game (0x4763C0)
                        icon: icon.map(|i| if i.starts_with('/') { i } else { format!("/Game/{i}") }),
                        filter,
                        color: [c[0], c[1], c[2], 1.0],
                        sound: n.string_or_empty("message_sound"),
                    },
                );
            }
        }
        MessageSystem { types, ..Default::default() }
    }

    /// Send (`0x40B5C0`). Returns the id, `None` when the type's filter
    /// UVar drops it.
    pub fn send(&mut self, uvars: &crate::uvars::UVars, kind: &str, text: &str, location: Option<(f32, f32)>) -> Option<u32> {
        //Legacy UI/widget laboratory path omits actual timer/View/log children.
        //It cannot retain a live native-source capability after mutation.
        self.native=None;
        self.native_constructor=false;
        if let Some(t) = self.types.get(&kind.to_ascii_uppercase()) {
            if let Some(f) = &t.filter {
                if uvars.def(f).is_some() && !uvars.get_bool(f) {
                    return None;
                }
            }
        }
        self.last += 1;
        self.messages.push_front(Message { id: self.last, kind: kind.to_ascii_uppercase(), text: text.to_string(), location,native:None });
        let max = uvars.get_i32("MessageLogMaximumLines").max(0) as usize;
        while self.messages.len() > max {
            self.messages.pop_back();
        }
        Some(self.last)
    }

    /// Clear (`0x40B450`): a new generation.
    pub fn clear(&mut self) {
        self.native=None;
        self.native_constructor=false;
        self.messages.clear();
        self.generation += 1;
    }

    /// `0x40B1B0(id)`: the message just newer than `id` (id 0 or unknown:
    /// the oldest).
    pub fn next_after(&self, id: u32) -> Option<&Message> {
        match self.messages.iter().position(|m| m.id == id) {
            Some(0) => None,
            Some(i) => self.messages.get(i - 1),
            None => self.messages.back(),
        }
    }
}

/// Log widget state (`+0x94..+0xD4`) with its viewport.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct MessageLogState {
    pub vp: crate::controls::Viewport,
    /// UVar holding the row count (slot 0x47 UIMessageLogMaximumLines for
    /// the HUD, 0x43 MessageLogMaximumLines for side/staging logs).
    pub max_lines_var: String,
    pub fades: bool,
    pub auto_height: bool,
    pub expands_down: bool,
    /// `+0xB0`: the UVar filters apply.
    pub use_filters: bool,
    pub last_id: u32,
    pub generation: u32,
    /// Rows, most recently used first (tags).
    pub lru: Vec<u64>,
    /// Rows not in the viewport.
    pub spare: Vec<Widget>,
    /// Cloned `ItemPanel` node (`+0xA4`).
    pub node: Option<crate::node::UiNode>,
    /// Filter overrides by filter name: 0 show, 1 hide.
    pub overrides: HashMap<String, i32>,
    /// The HUD limit for the auto height (`0x5C2220`), reference units.
    pub auto_height_limit: Option<f32>,
    pub next_tag: u64,
}

/// Row state (`MessageWidget` +0x84..+0x98).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct MessageRow {
    pub transparency: f32,
    pub fades: bool,
    pub start_ms: u64,
    pub message: u32,
}

/// `0x5FD6D0(name, reg, maxLinesSlot, fades, autoHeight, cmd)`.
pub fn message_log(name: &str, max_lines_var: &str, fades: bool, auto_height: bool) -> Widget {
    let mut w = Widget::new(C::MessageLog, name, false);
    w.add_child(Widget::new(C::Panel, "ItemPanel", false));
    w.control = Some(Box::new(crate::controls::Control::MessageLog(MessageLogState {
        max_lines_var: max_lines_var.to_string(),
        fades,
        auto_height,
        use_filters: true,
        next_tag: 1,
        ..Default::default()
    })));
    w
}

fn set_alpha(w: &mut Widget, a: f32) {
    if let Some(ve) = &mut w.sub_ve {
        match &mut ve.kind {
            VeKind::Label(l) => l.color[3] = a,
            VeKind::Decal(d) => d.color[3] = a,
            _ => {}
        }
    }
}

impl Ui {
    fn log_state(&self, path: &str) -> Option<&MessageLogState> {
        match self.widget(path)?.control.as_deref()? {
            crate::controls::Control::MessageLog(s) => Some(s),
            _ => None,
        }
    }

    fn log_state_mut(&mut self, path: &str) -> Option<&mut MessageLogState> {
        match self.widget_mut(path)?.control.as_deref_mut()? {
            crate::controls::Control::MessageLog(s) => Some(s),
            _ => None,
        }
    }

    /// CreateWidget `0x671050`: a hidden row loaded from the cloned node.
    fn log_new_row(&mut self, path: &str) -> Option<Widget> {
        let st = self.log_state(path)?.clone();
        let node = st.node.clone()?;
        let mut row = Widget::new(C::Panel, "MessageWidget", false);
        row.add_child(Widget::new(C::Decal, "MessageWidgetBackground", false));
        row.add_child(Widget::new(C::Label, "MessageWidgetText", false));
        let mut button = Widget::new(C::PushButton, "MessageWidgetButton", false);
        button.command = Some("MessageLogAction".into());
        button.client_name = "MessageWidgetButtonDecal".into();
        button.add_child(Widget::new(C::Decal, "MessageWidgetButtonDecal", false));
        row.add_child(button);
        {
            let strings = &self.strings;
            let resolve = |v: &str| strings.resolve(v);
            let root = std::sync::Arc::new(node.clone());
            row.load(&mut self.store, Some((&node, &root)), Some(10), &resolve);
        }
        // LoadData 0x66F8E0: transparency clamped to [0, 1]
        let t = node.child("MessageWidget").and_then(|n| n.float("transparency").ok()).unwrap_or(0.0);
        let t = if t < 0.0 { 0.0 } else if t > 1.0 { 1.0 } else { t };
        row.control = Some(Box::new(crate::controls::Control::MessageRow(MessageRow { transparency: t, fades: st.fades, ..Default::default() })));
        row.set_visible(false);
        let s = self.log_state_mut(path)?;
        row.tag = s.next_tag;
        s.next_tag += 1;
        Some(row)
    }

    /// Remove a row from the viewport (`0x5ED7A0`) into the spare list.
    fn log_take_row(&mut self, path: &str, tag: u64) -> Option<Widget> {
        let panel = format!("{path}/ItemPanel");
        let idx = self.widget(&panel)?.children.iter().position(|c| c.tag == tag);
        if let Some(i) = idx {
            let w = self.widget_mut(&panel)?.children.remove(i);
            self.viewport_refresh(path);
            return Some(w);
        }
        let s = self.log_state_mut(path)?;
        let i = s.spare.iter().position(|w| w.tag == tag)?;
        Some(s.spare.remove(i))
    }

    /// Per-frame update (`0x671230`).
    pub fn message_log_update(&mut self, path: &str) {
        let Some(st) = self.log_state(path).cloned() else { return };
        let sys_gen = self.messages.generation;
        // 1. generation
        if st.generation != sys_gen {
            for tag in st.lru.clone() {
                let _ = self.log_take_row(path, tag);
            }
            if let Some(s) = self.log_state_mut(path) {
                s.generation = sys_gen;
                s.lru.clear();
                s.spare.clear();
                s.last_id = 0;
            }
        }
        // 2. faded rows leave the viewport
        let panel = format!("{path}/ItemPanel");
        let hidden: Vec<u64> = self
            .widget(&panel)
            .map(|p| p.children.iter().filter(|c| c.flags & 1 == 0).map(|c| c.tag).collect())
            .unwrap_or_default();
        for tag in hidden {
            if let Some(w) = self.log_take_row(path, tag) {
                if let Some(s) = self.log_state_mut(path) {
                    s.spare.push(w);
                }
            }
        }
        // 3. the row pool follows the max-lines UVar
        let n = self.uvars.get_i32(&st.max_lines_var).max(0) as usize;
        loop {
            let count = self.log_state(path).map(|s| s.lru.len()).unwrap_or(0);
            if count >= n {
                break;
            }
            let Some(row) = self.log_new_row(path) else { break };
            if let Some(s) = self.log_state_mut(path) {
                s.lru.push(row.tag);
                s.spare.push(row);
            }
        }
        loop {
            let tail = self.log_state(path).and_then(|s| if s.lru.len() > n { s.lru.last().copied() } else { None });
            let Some(tag) = tail else { break };
            let _ = self.log_take_row(path, tag);
            if let Some(s) = self.log_state_mut(path) {
                s.lru.pop();
            }
        }
        // 4. new messages
        let mut last = self.log_state(path).map(|s| s.last_id).unwrap_or(0);
        while let Some(m) = self.messages.next_after(last).cloned() {
            let show = {
                let st = self.log_state(path).cloned().unwrap_or_default();
                let filter = self.messages.types.get(&m.kind).and_then(|t| t.filter.clone());
                match filter.as_ref().and_then(|f| st.overrides.get(f)) {
                    Some(0) => true,
                    Some(1) => false,
                    _ => match &filter {
                        None => true,
                        Some(f) => st.use_filters && (self.uvars.def(f).is_none() || self.uvars.get_bool(f)),
                    },
                }
            };
            if show {
                self.log_show(path, &m);
            }
            last = m.id;
            if let Some(s) = self.log_state_mut(path) {
                s.last_id = last;
            }
        }
        // 5. auto height (0x671230): B = the HUD limit as a size
        if st.auto_height {
            if let Some(limit) = st.auto_height_limit {
                let g = self.geo();
                let v1 = self.widget(path).map(|w| w.root.outer.rect.b / g.sy).unwrap_or(0.0);
                if v1 != limit {
                    if let Some(w) = self.widget_mut(path) {
                        crate::geometry::set_height(w, limit);
                    }
                }
            }
        }
        // row fades (MessageWidget Update 0x66F9A0)
        let fade_ms = self.uvars.get_i32("UIMessageLogFadeTime") as i64;
        let now = self.now_ms;
        if let Some(p) = self.widget_mut(&panel) {
            for row in &mut p.children {
                let Some(crate::controls::Control::MessageRow(r)) = row.control.as_deref().cloned() else { continue };
                if !r.fades {
                    continue;
                }
                let e = now.saturating_sub(r.start_ms) as i64;
                if e <= fade_ms {
                    continue;
                }
                let d = e - fade_ms;
                if d > 1000 {
                    row.set_visible(false);
                    continue;
                }
                let a = 1.0f32 - d as f32 * 0.001f32;
                if let Some(bg) = row.child_mut("MessageWidgetBackground") {
                    set_alpha(bg, a * r.transparency);
                }
                if let Some(t) = row.child_mut("MessageWidgetText") {
                    set_alpha(t, a);
                }
                if let Some(b) = row.child_mut("MessageWidgetButton").and_then(|b| b.child_mut("MessageWidgetButtonDecal")) {
                    set_alpha(b, a);
                }
            }
        }
    }

    /// Show (`0x670A10`) with SetMessage (`0x670450`).
    fn log_show(&mut self, path: &str, m: &crate::message_log::Message) {
        let Some(tag) = self.log_state(path).and_then(|s| s.lru.last().copied()) else { return };
        let Some(mut row) = self.log_take_row(path, tag) else { return };
        if let Some(s) = self.log_state_mut(path) {
            s.lru.pop();
            s.lru.insert(0, tag);
        }
        let ty = self.messages.types.get(&m.kind).cloned();
        let g = self.geo();
        let panel_w = self.widget(&format!("{path}/ItemPanel")).map(|p| (p.root.outer.rect.r - p.root.outer.rect.l) / g.sx).unwrap_or(0.0);
        let transparency = match row.control.as_deref() {
            Some(crate::controls::Control::MessageRow(r)) => r.transparency,
            _ => 0.0,
        };
        if let Some(bg) = row.child_mut("MessageWidgetBackground") {
            set_alpha(bg, transparency);
        }
        let font = row
            .child("MessageWidgetText")
            .and_then(|t| match &t.sub_ve {
                Some(crate::ve::ViewElement { kind: VeKind::Label(l), .. }) => Some(l.font.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let label_l = row.child("MessageWidgetText").map(|t| t.root.outer.raw[0]).unwrap_or(0.0);
        if let Some(t) = row.child_mut("MessageWidgetText") {
            if let Some(crate::ve::ViewElement { kind: VeKind::Label(l), .. }) = &mut t.sub_ve {
                l.text = m.text.clone();
                l.color = ty.as_ref().map(|t| t.color).unwrap_or([1.0, 1.0, 1.0, 1.0]);
            }
        }
        // text wraps at the label width (the row width minus the label's L)
        let (_, text_h, _) = self.measure_text(&font, &m.text, panel_w - label_l);
        let mut icon_h = 0.0;
        if let Some(b) = row.child_mut("MessageWidgetButton").and_then(|b| b.child_mut("MessageWidgetButtonDecal")) {
            if let Some(crate::ve::ViewElement { kind: VeKind::Decal(d), .. }) = &mut b.sub_ve {
                d.texture = ty.as_ref().and_then(|t| t.icon.clone());
                d.color[3] = 1.0;
            }
            if b.root.outer.flags & 0x200 != 0 {
                icon_h = b.root.outer.raw[3];
            }
        }
        let h = if icon_h > text_h { icon_h } else { text_h };
        crate::geometry::set_size(&mut row, panel_w, h);
        if let Some(b) = row.child_mut("MessageWidgetButton") {
            b.command = Some(format!("MessageLogAction {}", m.id));
        }
        if let Some(crate::controls::Control::MessageRow(r)) = row.control.as_deref_mut() {
            r.start_ms = self.now_ms;
            r.message = m.id;
        }
        row.set_visible(true);
        self.viewport_add(path, row);
    }
}
