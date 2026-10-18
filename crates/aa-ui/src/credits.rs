//! The credits screen (`CreditsInterface`, ctor `0x646150`, vtable
//! 0xA62ED8, menu table slot 10).
//!
//! * LoadData `0x645F70`: `scroll_speed`, `fade_length`; the credit nodes
//!   of every CreditsData object (`0x645BD0`, data manager
//!   `[0xB36530]+0x788`, Data/Credits/*.tgi); the root GroupVE's first and
//!   second DecalVE (A, B) and NIFVE (A, B) (`0x645960`).
//! * Children (`0x646560`): DecalWidgets `CreditsTextGradientTop` and
//!   `CreditsTextGradientBottom` (AddChild flag 0); label widgets are made
//!   on demand per `label_ids` (`0x645D10`, LabelWidget `0x5F2980`, hidden,
//!   AddChild flag 1 = in front of equal keys) and pooled (`0x646020`
//!   acquire, `0x646670` release).
//! * OnConnected `0x646B50`: Reset `0x646790`, spawn once `0x646A90`,
//!   restart the clock. Update `0x646C10`: scroll by
//!   `(now - last) * scroll_speed` (real time, seconds as f32), two fade
//!   steps (`0x645890` runs twice), place every active label at
//!   `T = H - y, B = h` (size), spawn while the last label has scrolled
//!   past its leading, release labels above the top, post
//!   `CreditsEndCredits` once nothing is left. `CreditsEndCredits`
//!   (`0x645800`) re-activates MainMenuInterface; Esc posts it too
//!   (hotkeys_ui.txt).
//! * Node types (`0x471180`): TEXT (`label_ids`, localized `text`,
//!   `leading`), DECAL (`texture`), NIF (`nif`, `camera`). A leading `@`
//!   in a localized value is stripped and the value is not looked up
//!   (`0x5BA3C0`).
//! * Background art: crossfade `0x645730` (B takes A's texture at full
//!   alpha, A the new one at alpha 0; fade over `fade_length` scrolled
//!   units). NIF nodes swap NIF VE models; AA's credits have none, and the
//!   NIF view elements are not rendered by this port.

use crate::ui::Ui;
use crate::ve::{VeKind, ViewElement};
use crate::widget::{Widget, WidgetClass as C};

pub const INTERFACE: &str = "UI/Menus/credits.tgi:Credits";
pub const PATH: &str = "Root/Credits";
pub const END_COMMAND: &str = "CreditsEndCredits";

#[derive(Clone, Debug, PartialEq)]
pub enum CreditNode {
    Text { label_ids: String, text: String, leading: f32 },
    Decal { texture: String },
    Nif { nif: String, camera: String },
}

#[derive(Clone, Debug)]
struct Entry {
    label_ids: String,
    tag: u64,
    y: f32,
    h: f32,
    leading: f32,
}

pub struct Credits {
    pub nodes: Vec<CreditNode>,
    next: usize,
    pub scroll_speed: f32,
    pub fade_length: f32,
    active: Vec<Entry>,
    /// Free label widgets per label_ids (tags).
    pool: Vec<(String, u64)>,
    next_tag: u64,
    /// `+0x12C` decal fade timer.
    fade_t: f32,
    /// `+0x118` time of the last tick in seconds.
    last_s: f32,
    /// Clock start (ms).
    start_ms: u64,
    /// `CreditsEndCredits` has been posted.
    pub ended: bool,
    pub diagnostics: Vec<String>,
}

/// Localized value (`0x5BA3C0` strips a leading `@`; `0x5B5620` looks up
/// `#key`).
fn localized(ui: &Ui, raw: &str) -> String {
    match raw.strip_prefix('@') {
        Some(lit) => lit.to_string(),
        None => ui.strings.resolve(raw),
    }
}

/// The credit nodes of every CreditsData object in Data/Credits
/// (`0x645BD0`; file order of `Credits/*.tgi`).
pub fn load_nodes(ui: &mut Ui, diags: &mut Vec<String>) -> Vec<CreditNode> {
    let _ = ui.store.load_file("Templates/template_credits.tgi");
    let mut out = Vec::new();
    for file in crate::resources::list_tgi(&ui.store.root, "Credits") {
        let Ok(root) = ui.store.load_file(&file) else {
            diags.push(format!("{file}: not loaded"));
            continue;
        };
        for data in &root.children {
            for n in data.children_named("CreditNode") {
                let ty = n.string_or_empty("type");
                match ty.to_ascii_uppercase().as_str() {
                    "TEXT" => out.push(CreditNode::Text {
                        label_ids: n.string_or_empty("label_ids"),
                        text: localized(ui, &n.string_or_empty("text")),
                        leading: n.float("leading").unwrap_or(0.0),
                    }),
                    "DECAL" => out.push(CreditNode::Decal { texture: n.string_or_empty("texture") }),
                    "NIF" => out.push(CreditNode::Nif { nif: n.string_or_empty("nif"), camera: n.string_or_empty("camera") }),
                    _ => diags.push(format!("Unknown credit node type '{ty}' in {file}")),
                }
            }
        }
    }
    out
}

/// The interface's decal VEs A and B (root GroupVE children, depth-first,
/// `0x645960`).
fn decals(w: &mut Widget) -> Vec<&mut ViewElement> {
    fn walk<'a>(ve: &'a mut ViewElement, out: &mut Vec<&'a mut ViewElement>) {
        if matches!(ve.kind, VeKind::Decal(_)) {
            out.push(ve);
            return;
        }
        for c in &mut ve.children {
            walk(c, out);
        }
    }
    let mut out = Vec::new();
    if let Some(v) = &mut w.view {
        walk(v, &mut out);
    }
    out.truncate(2);
    out
}

fn set_decal(ve: &mut ViewElement, texture: Option<Option<String>>, alpha: Option<f32>, visible: Option<bool>) {
    if let VeKind::Decal(d) = &mut ve.kind {
        if let Some(t) = texture {
            d.texture = t;
        }
        if let Some(a) = alpha {
            d.color[3] = a;
        }
    }
    if let Some(v) = visible {
        let f = crate::layout::ve_flag::VISIBLE;
        if v {
            ve.layout.flags |= f;
        } else {
            ve.layout.flags &= !f;
        }
    }
}

impl Credits {
    /// Build, connect and start the credits (`RequestCredits` `0x64F290`:
    /// `menuTable[10]->Activate(1)`).
    pub fn show(ui: &mut Ui, now_ms: u64) -> Self {
        ui.clear();
        let mut diagnostics = Vec::new();
        let mut iface = Widget::new(C::Interface, INTERFACE, false);
        iface.add_child(Widget::new(C::Decal, "CreditsTextGradientTop", false));
        iface.add_child(Widget::new(C::Decal, "CreditsTextGradientBottom", false));
        ui.load_widget(&mut iface, None);
        let node = ui.store.node(INTERFACE).ok();
        let scroll_speed = node.as_ref().and_then(|n| n.float("scroll_speed").ok()).unwrap_or(0.0);
        let fade_length = node.as_ref().and_then(|n| n.float("fade_length").ok()).unwrap_or(0.0);
        if decals(&mut iface).len() < 2 {
            diagnostics.push("CreditsInterface must contain a NIF and decal VE.".to_string());
        }
        let nodes = load_nodes(ui, &mut diagnostics);
        ui.connect(iface);
        let mut c = Credits {
            nodes,
            next: 0,
            scroll_speed,
            fade_length,
            active: Vec::new(),
            pool: Vec::new(),
            next_tag: 1,
            fade_t: 0.0,
            last_s: 0.0,
            start_ms: now_ms,
            ended: false,
            diagnostics,
        };
        // OnConnected 0x646B50
        c.reset(ui);
        c.spawn(ui);
        c.start_ms = now_ms;
        c.last_s = 0.0;
        c.place(ui);
        c
    }

    fn iface<'a>(ui: &'a mut Ui) -> Option<&'a mut Widget> {
        ui.widget_mut(PATH)
    }

    fn label_mut<'a>(ui: &'a mut Ui, tag: u64) -> Option<&'a mut Widget> {
        Self::iface(ui)?.children.iter_mut().find(|c| c.tag == tag)
    }

    /// Reset (`0x646790`).
    fn reset(&mut self, ui: &mut Ui) {
        self.next = 0;
        if let Some(w) = Self::iface(ui) {
            let mut d = decals(w);
            if d.len() == 2 {
                set_decal(d[1], Some(None), None, Some(false));
                set_decal(d[0], Some(None), None, None);
            }
        }
        while !self.active.is_empty() {
            self.release(ui, 0);
        }
    }

    /// Release the active entry `i` (`0x646670`).
    fn release(&mut self, ui: &mut Ui, i: usize) {
        let e = self.active.remove(i);
        if let Some(w) = Self::label_mut(ui, e.tag) {
            w.set_visible(false);
        }
        self.pool.push((e.label_ids, e.tag));
    }

    /// Acquire a label for `label_ids` (`0x646020` / create `0x645D10`).
    fn acquire(&mut self, ui: &mut Ui, label_ids: &str) -> Option<u64> {
        if let Some(i) = self.pool.iter().position(|(l, _)| l.eq_ignore_ascii_case(label_ids)) {
            return Some(self.pool.remove(i).1);
        }
        let node = ui.store.node(INTERFACE).ok()?;
        if node.child(label_ids).is_none() {
            self.diagnostics.push(format!("Credit label '{label_ids}' is not a child of the Credits interface"));
            return None;
        }
        let mut w = Widget::new(C::Label, label_ids, false);
        w.set_visible(false);
        {
            let strings = &ui.strings;
            let resolve = |v: &str| strings.resolve(v);
            let root = std::sync::Arc::new(node.clone());
            w.load(&mut ui.store, Some((&node, &root)), Some(10), &resolve);
        }
        let tag = self.next_tag;
        self.next_tag += 1;
        w.tag = tag;
        Self::iface(ui)?.add_child_front(w);
        Some(tag)
    }

    /// Spawn (`0x646A90`): run nodes up to and including the next TEXT.
    fn spawn(&mut self, ui: &mut Ui) {
        while self.next < self.nodes.len() {
            let n = self.nodes[self.next].clone();
            self.next += 1;
            match n {
                CreditNode::Text { label_ids, text, leading } => {
                    if let Some(tag) = self.acquire(ui, &label_ids) {
                        // entry setup 0x645820: text, measured height
                        ui.layout();
                        let path = format!("Credits/{}", self.path_seg(ui, tag));
                        ui.set_label(&path, &text);
                        let g = ui.geo();
                        let wrap = Self::label_mut(ui, tag).map(|w| (w.root.outer.rect.r - w.root.outer.rect.l) / g.sx).unwrap_or(0.0);
                        let (_, h, _) = ui.measure_label(&path, &text, wrap);
                        let y = if self.active.is_empty() {
                            0.0
                        } else {
                            let p = self.active.last().unwrap();
                            p.y - p.h - p.leading
                        };
                        self.active.push(Entry { label_ids, tag, y, h, leading });
                    }
                    break;
                }
                CreditNode::Decal { texture } => self.crossfade(ui, &texture),
                CreditNode::Nif { .. } => {}
            }
        }
    }

    /// Path segment of the label with `tag` (`Name#k`).
    fn path_seg(&self, ui: &mut Ui, tag: u64) -> String {
        let Some(w) = Self::iface(ui) else { return String::new() };
        let segs = crate::ui::child_segments(w);
        w.children.iter().zip(segs).find(|(c, _)| c.tag == tag).map(|(_, s)| s).unwrap_or_default()
    }

    /// Crossfade (`0x645730`).
    fn crossfade(&mut self, ui: &mut Ui, texture: &str) {
        let fade_length = self.fade_length;
        let mut fade_t = fade_length;
        if let Some(w) = Self::iface(ui) {
            let mut d = decals(w);
            if d.len() == 2 {
                let a_tex = match &d[0].kind {
                    VeKind::Decal(x) => x.texture.clone(),
                    _ => None,
                };
                if a_tex.is_some() {
                    set_decal(d[1], Some(a_tex), Some(1.0), Some(true));
                    set_decal(d[0], None, Some(0.0), None);
                    fade_t = 0.0;
                }
                set_decal(d[0], Some(Some(texture.to_string())), None, None);
            }
        }
        self.fade_t = fade_t;
    }

    /// Fade step (`0x645890`).
    fn fade_step(&mut self, ui: &mut Ui, d: f32) {
        let fade_length = self.fade_length;
        let Some(w) = Self::iface(ui) else { return };
        let mut v = decals(w);
        if v.len() < 2 || v[1].layout.flags & crate::layout::ve_flag::VISIBLE == 0 {
            return;
        }
        self.fade_t += d;
        if self.fade_t <= fade_length {
            let t = self.fade_t / fade_length;
            set_decal(v[1], None, Some(1.0 - t), None);
            set_decal(v[0], None, Some(t), None);
        } else {
            set_decal(v[1], None, None, Some(false));
            set_decal(v[0], None, Some(1.0), None);
        }
    }

    /// Place every active label (`T = H - y`, `B = h` as a size).
    fn place(&mut self, ui: &mut Ui) {
        let g = ui.geo();
        let h_ref = Self::iface(ui).map(|w| (1.0 / g.sy) * w.root.outer.rect.b - (1.0 / g.sy) * w.root.outer.rect.t).unwrap_or(0.0);
        for e in &self.active {
            if let Some(w) = Self::label_mut(ui, e.tag) {
                let o = &mut w.root.outer;
                o.raw[1] = h_ref - e.y;
                o.raw[3] = e.h;
                o.flags = (o.flags & !0x22 & !0x88) | 0x200;
                if w.flags & 1 == 0 {
                    w.set_visible(true);
                }
            }
        }
        ui.layout();
    }

    /// Per-frame update (`0x646C10`); returns true once
    /// `CreditsEndCredits` should run.
    pub fn update(&mut self, ui: &mut Ui, now_ms: u64) -> bool {
        // 0x59ABA0 whole milliseconds as u32, * 0.001f
        let now = (now_ms.saturating_sub(self.start_ms) as u32) as f32 * 0.001;
        let d = (now - self.last_s) * self.scroll_speed;
        self.fade_step(ui, d);
        self.fade_step(ui, d);
        let g = ui.geo();
        let h_ref = Self::iface(ui).map(|w| (1.0 / g.sy) * w.root.outer.rect.b - (1.0 / g.sy) * w.root.outer.rect.t).unwrap_or(0.0);
        for e in &mut self.active {
            e.y += d;
        }
        self.place(ui);
        if !self.active.is_empty() {
            while self.next < self.nodes.len() {
                let l = self.active.last().unwrap();
                if l.y - l.h - l.leading < 0.0 {
                    break;
                }
                self.spawn(ui);
                self.place(ui);
            }
        }
        while let Some(f) = self.active.first() {
            if f.y - f.h - f.leading > h_ref {
                self.release(ui, 0);
            } else {
                break;
            }
        }
        self.last_s = now;
        if self.active.is_empty() {
            self.ended = true;
        }
        ui.layout();
        self.ended
    }

    /// Labels on screen (label_ids, text top in reference units, height).
    pub fn active_labels(&self) -> Vec<(String, f32, f32)> {
        self.active.iter().map(|e| (e.label_ids.clone(), e.y, e.h)).collect()
    }
}
