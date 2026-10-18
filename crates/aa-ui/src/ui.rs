//! The UI runtime: data, the Root widget, layout, draw list and input.
//!
//! * `KKC_UI` constructor `0x5CAE30` (render level +0xDC = 0), template lists
//!   `0x5C5060`, Root `"ui/shared/root.tgi:Root"` (`0x5F32F0`, `0x5C5C70`).
//! * Connect/Disconnect `0x5C0FB0`/`0x5C1920`: a shown top-level widget is a
//!   child of Root (append); "show" helpers re-append to bring it to front.
//! * Draw order: depth-first pre-order of the root view-element tree; each
//!   view element takes a layer number from a pre-order counter (`0x5E17E0`
//!   / group `0x5E1BA0`). Non-blended sprites form one render list and
//!   blended sprites a second one drawn after it (see `draw_list`).

use crate::draw::{DrawCmd, DrawKind, DrawList, Quad, Text, Video};
use crate::layout::{Precision, Rect, Reference, Screen, VeLayout, layout_ve};
use crate::resources::{AVars, StringTable, load_fonts};
use crate::sprite::{self, PieceStyle, blend_of, decal_fit, directional_pieces, nine_slice_rects, piece_quads};
use crate::state::{self, ve_visible, widget_flag};
use crate::text::{FontDef, FontFace, TtfMetrics};
use crate::tgi::TgiStore;
use crate::ve::{VeKind, ViewElement};
use crate::widget::{Widget, WidgetClass};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// The label view element fields the text rebuild reads.
#[derive(Clone, Debug)]
pub struct LabelSpec<'a> {
    pub font: &'a str,
    pub rect: Rect,
    /// Clipped amounts (left, top, right, bottom) of the outer rect.
    pub clip: [f32; 4],
    pub justify: crate::ve::Justify,
    pub multiline: bool,
    pub color: [f32; 4],
    pub background: [f32; 4],
    /// Edit box horizontal scroll (text object +0x74 offset).
    pub scroll: f32,
}

#[cfg(test)]mod view_random_tests{
    use super::*;
    #[test]
    #[ignore="loads actual UI data; no native game/window/GPU"]
    fn actual_ui_constructor_random_state_is_not_current_before_reseed(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-ui-random.json")).unwrap();
        let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),6);
        let mut ui=Ui::new(crate::fs::DataRoot::workspace_default(),Screen{width:1280,height:720});
        assert_eq!(ui.random_d4.state,rows[0]["seed"].as_u64().unwrap()as u32);
        for row in rows{assert_eq!(row["values"],rows[0]["values"]);}
        assert!(ui.next_view_random_word_d4().is_err());ui.clear();assert!(ui.next_view_random_word_d4().is_err());
        assert_eq!(ui.random_d4.state,rows[0]["seed"].as_u64().unwrap()as u32);
        //This separately verifies the integer primitive against the native
        //constructor stream; it does not substitute it for initialized UI+D4.
        let mut primitive=aa_simulation::NativeRng::default();
        for draw in rows[0]["values"].as_array().unwrap(){assert_eq!(primitive.next_u32(),draw["value"].as_u64().unwrap()as u32);assert_eq!(primitive.state,draw["state"].as_u64().unwrap()as u32);}
    }
}

/// A widget mover (`0x5C14A0`): from / to raw positions (reference
/// units), the start time and the duration `ftol(seconds * 1000)` ms.
#[derive(Clone, Debug, PartialEq)]
pub struct Mover {
    pub path: String,
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub start_ms: u64,
    pub duration_ms: u32,
}

impl Mover {
    /// `0x5C1510`: elapsed / duration (1 when the duration is 0).
    pub fn progress(&self, now_ms: u64) -> f32 {
        if self.duration_ms == 0 {
            return 1.0;
        }
        now_ms.saturating_sub(self.start_ms) as f32 / self.duration_ms as f32
    }
}

/// An event produced by input handling.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum UiEvent {
    /// A button fired its console command string.
    Command { widget: String, command: String },
    /// A slider with a code functor changed its value (`SetValueNotify`).
    Value { widget: String, value: f32 },
    /// A sound the original plays here (`set_sound`, `sound_clicked`,
    /// `sound_open` ...), by audio IDS.
    Sound { name: String },
    /// An owned drop-down's functor (`0x5DC4B0`): the entry picked.
    Select { widget: String, index: usize, param: String },
    /// A selectable panel was selected (`0x5DC540` input, group
    /// `0x5DC6E0`).
    Selected { widget: String, tag: u64 },
    /// Text for the system clipboard (`EditboxCopy` / `EditboxCut`).
    Clipboard { text: String },
    /// A minimap press, double click or drag step: the world point as
    /// fractions of the map size (`0x5D77A0` / `0x5D7830` jump the camera).
    Minimap { widget: String, u: f32, v: f32 },
}

/// How the two render lists combine (see `Ui::draw_list`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DepthMode {
    /// Depth-tested layers: tree order.
    #[default]
    Layers,
    /// No depth test: all opaque sprites, then all blended ones.
    TwoLists,
}

/// Pointer buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
}

pub struct Ui {
    ///Sole UI+D4 random stream.5CAFD0 invokes the actual85D870 constructor;
    ///it is distinct from the authoritative simulation RNG and is not reset
    ///by clearing widgets or beginning another VWorld.
    pub(crate) random_d4:aa_simulation::NativeRng,
    pub(crate) random_initialized:bool,
    pub(crate) random_identity:Arc<()>,
    pub store: TgiStore,
    pub strings: StringTable,
    pub avars: AVars,
    pub fonts: HashMap<String, FontDef>,
    pub screen: Screen,
    pub reference: Reference,
    pub precision: Precision,
    /// UI render level (`KKC_UI`+0xDC, 0 at construction).
    pub render_level: i32,
    pub root: Widget,
    ttf_cache: HashMap<String, Option<Arc<TtfMetrics>>>,
    /// Font atlas tables for the current screen (`0x708B30`), built on use.
    ui_fonts: Option<Arc<crate::fonts::UiFonts>>,
    /// `character_sets.tgi` (min, max, undisplayable, spacing standin).
    char_sets: Vec<(u32, u32, u32, u32)>,
    /// Ellipsis sizes per font id (`font+0x10/+0x14`).
    ellipsis_cache: HashMap<u32, [f32; 2]>,
    tex_size_cache: HashMap<String, Option<(u32, u32)>>,
    pub(crate) hovered: Option<String>,
    pub(crate) pressed: Option<String>,
    pub events: Vec<UiEvent>,
    /// Open drop-downs whose list box sits in the UI overlay
    /// (`[0xB3653C]+0x74`), by path.
    pub(crate) overlay: Vec<String>,
    /// Input capture stack (`0x5E0840`): open drop-downs, the focused
    /// edit box.
    pub(crate) capture: Vec<String>,
    /// Running grip drag: grip path and the press position (px).
    pub(crate) drag: Option<(String, f32, f32)>,
    /// Deferred callbacks (UI queue `0x5C3EB0`).
    pub(crate) queue: Vec<crate::ui_controls::Deferred>,
    /// Clock of the last `update` (ms).
    pub now_ms: u64,
    /// Render list combination (the loading screen uses `TwoLists`).
    pub depth_mode: DepthMode,
    /// The message system (`0xB399B0`) the message logs show.
    pub messages: crate::message_log::MessageSystem,
    /// User variables (`0xB39938`): defaults + the caller's saved values.
    pub uvars: crate::uvars::UVars,
    /// The minimap being dragged (its drag tracker, vt 0xA6F2C4).
    pub(crate) minimap_drag: Option<String>,
    /// Widget movers (UI+0xA4 list, `0x5C3260` / `0x5C28B0`).
    pub(crate) movers: Vec<Mover>,
    /// Registered edit boxes in activation order (UI+0x170).
    pub(crate) editboxes: Vec<String>,
    /// `hotkeys_ui.txt` entries (loaded on first use).
    pub(crate) hotkeys: Option<Vec<crate::hotkeys::Hotkey>>,
    /// Top-level widget that receives all input (message boxes).
    pub modal: Option<String>,
    /// The request of the message box on screen.
    pub message_box: Option<crate::message_box::MessageBoxRequest>,
}

/// Path segments of a widget's children: the name, with `#k` appended for
/// the k-th (k > 0) of several children sharing a name.
pub fn child_segments(w: &Widget) -> Vec<String> {
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    w.children
        .iter()
        .map(|c| {
            let n = c.name.trim_start_matches('/').to_string();
            let k = seen.entry(n.to_ascii_lowercase()).or_insert(0);
            let seg = if *k == 0 { n } else { format!("{n}#{k}") };
            *k += 1;
            seg
        })
        .collect()
}

/// Resolve `A/B#2/C` below `w` (`#k` = the k-th child of that name).
pub(crate) fn path_mut<'a>(w: &'a mut Widget, path: &str) -> Option<&'a mut Widget> {
    let mut at = w;
    for part in path.split('/').filter(|p| !p.is_empty()) {
        let (name, k) = match part.split_once('#') {
            Some((n, k)) => (n, k.parse::<usize>().ok()?),
            None => (part, 0),
        };
        let name = name.trim_start_matches('/');
        at = at.children.iter_mut().filter(|c| c.name.trim_start_matches('/').eq_ignore_ascii_case(name)).nth(k)?;
    }
    Some(at)
}

pub(crate) fn path_ref<'a>(w: &'a Widget, path: &str) -> Option<&'a Widget> {
    let mut at = w;
    for part in path.split('/').filter(|p| !p.is_empty()) {
        let (name, k) = match part.split_once('#') {
            Some((n, k)) => (n, k.parse::<usize>().ok()?),
            None => (part, 0),
        };
        let name = name.trim_start_matches('/');
        at = at.children.iter().filter(|c| c.name.trim_start_matches('/').eq_ignore_ascii_case(name)).nth(k)?;
    }
    Some(at)
}

fn text_resolver(strings: &StringTable) -> impl Fn(&str) -> String + '_ {
    move |v: &str| strings.resolve(v)
}

impl Ui {
    /// Load templates, strings, AVars and fonts; create an empty Root.
    pub fn new(root: crate::fs::DataRoot, screen: Screen) -> Self {
        let mut store = TgiStore::new(root);
        store.load_ui_templates();
        let strings = StringTable::load(&mut store);
        let avars = AVars::load(&mut store);
        let fonts = load_fonts(&mut store);
        let reference = Reference {
            width: avars.float("InterfacePositioningWidth").unwrap_or(1024.0),
            height: avars.float("InterfacePositioningHeight").unwrap_or(768.0),
        };
        let mut ui = Ui {
            random_d4:aa_simulation::NativeRng::default(),
            random_initialized:false,
            random_identity:Arc::new(()),
            store,
            strings,
            avars,
            fonts,
            screen,
            reference,
            precision: Precision::Pc24,
            render_level: 0,
            root: Widget::new(WidgetClass::TopLevel, "ui/shared/root.tgi:Root", false),
            ttf_cache: HashMap::new(),
            ui_fonts: None,
            char_sets: Vec::new(),
            ellipsis_cache: HashMap::new(),
            tex_size_cache: HashMap::new(),
            hovered: None,
            pressed: None,
            events: Vec::new(),
            overlay: Vec::new(),
            capture: Vec::new(),
            drag: None,
            queue: Vec::new(),
            now_ms: 0,
            depth_mode: DepthMode::Layers,
            hotkeys: None,
            minimap_drag: None,
            movers: Vec::new(),
            editboxes: Vec::new(),
            messages: Default::default(),
            uvars: Default::default(),
            modal: None,
            message_box: None,
        };
        ui.messages = crate::message_log::MessageSystem::load(&mut ui.store);
        ui.uvars = crate::uvars::UVars::load(&mut ui.store);
        let mut root = std::mem::replace(&mut ui.root, Widget::new(WidgetClass::Static, "", false));
        ui.load_widget(&mut root, None);
        ui.root = root;
        ui
    }

    ///The reached UI+D4 draw consumed by451F90 and49B900. Other native UI
    ///random consumers must borrow this same owner when they are ported.
    pub fn next_view_random_word_d4(&mut self)->Result<u32,String> {
        if !self.random_initialized{return Err("5C8650 actual application B39714 draw and85D880 UI+D4 reseed owner is required".into())}
        Ok(self.random_d4.next_u32())
    }

    /// Load a top-level widget (own `file:node`) or a child given its
    /// parent's data context.
    pub fn load_widget(&mut self, w: &mut Widget, parent_threshold: Option<i32>) {
        let strings = self.strings.clone();
        let resolve = text_resolver(&strings);
        w.load(&mut self.store, None, parent_threshold.or(Some(self.root.render_threshold)), &resolve);
    }

    /// Connect a loaded top-level widget to Root (append = on top) and
    /// activate it (`0x5C0FB0` ShowDialog -> `0x5EE1D0`).
    pub fn connect(&mut self, w: Widget) {
        let seg = w.name.trim_start_matches('/').to_string();
        self.root.children.retain(|c| c.name != w.name || c.file != w.file);
        self.root.add_child(w);
        let path = format!("{}/{seg}", self.root.name.trim_start_matches('/'));
        self.activate(&path);
    }

    /// Disconnect (`0x5C1920`): deactivate and remove from Root.
    pub fn disconnect(&mut self, name: &str) -> Option<Widget> {
        let i = self.root.children.iter().position(|c| c.name.trim_start_matches('/') == name.trim_start_matches('/'))?;
        let path = format!("{}/{}", self.root.name.trim_start_matches('/'), name.trim_start_matches('/'));
        self.deactivate(&path);
        if self.hovered.as_deref().is_some_and(|h| h == path || h.starts_with(&format!("{path}/"))) {
            self.hovered = None;
        }
        if self.pressed.as_deref().is_some_and(|h| h == path || h.starts_with(&format!("{path}/"))) {
            self.pressed = None;
        }
        Some(self.root.children.remove(i))
    }

    pub fn clear(&mut self) {
        self.root.children.clear();
        self.hovered = None;
        self.pressed = None;
        self.overlay.clear();
        self.capture.clear();
        self.drag = None;
        self.queue.clear();
        self.depth_mode = DepthMode::Layers;
        self.editboxes.clear();
        self.modal = None;
        self.message_box = None;
    }

    /// Mutate the widget at `path` below Root (`Name/Child#k/...`).
    pub fn with_widget(&mut self, path: &str, f: impl FnOnce(&mut Widget)) -> bool {
        match path_mut(&mut self.root, path) {
            Some(w) => {
                f(w);
                true
            }
            None => false,
        }
    }

    /// Set the text of a label widget (`[Label]` view element) or of a
    /// button's client label (label vtable+0xC8 SetText).
    pub fn set_label(&mut self, path: &str, text: &str) -> bool {
        self.with_widget(path, |w| {
            let target = if w.class.is_button() {
                let client = w.client_name.clone();
                w.children.iter_mut().find(|c| c.name == client)
            } else {
                Some(w)
            };
            if let Some(t) = target {
                for ve in t.sub_ve.iter_mut().chain(t.view.iter_mut()) {
                    if let crate::ve::VeKind::Label(l) = &mut ve.kind {
                        l.text = text.to_string();
                    }
                }
            }
        })
    }

    /// The widget at `path` below Root.
    pub fn widget_by(&self, path: &str) -> Option<&Widget> {
        path_ref(&self.root, path)
    }

    /// TrueType metrics of a face (data file or system font).
    pub fn ttf_metrics(&mut self, face: &FontFace) -> Option<Arc<TtfMetrics>> {
        self.ttf(&face.face, face.bold)
    }

    pub fn set_screen(&mut self, screen: Screen) {
        if screen.width != self.screen.width || screen.height != self.screen.height {
            self.ui_fonts = None;
            self.ellipsis_cache.clear();
        }
        self.screen = screen;
        self.tex_size_cache.clear();
    }

    /// The font atlas tables of the current screen size.
    pub fn ui_fonts(&mut self) -> Arc<crate::fonts::UiFonts> {
        if let Some(f) = &self.ui_fonts {
            return f.clone();
        }
        let defs = self.fonts.clone();
        let specials = self.strings.special.clone();
        let root = self.store.root.clone();
        let (screen, ih) = ((self.screen.width, self.screen.height), self.reference.height);
        let mut ttf = |face: &FontFace| self.ttf(&face.face, face.bold);
        let built = Arc::new(crate::fonts::UiFonts::build(&defs, &specials, screen, ih, &root, &mut ttf));
        self.ui_fonts = Some(built.clone());
        built
    }

    /// Font-system constants for the text measure (AVars, character sets).
    pub fn text_env(&mut self) -> crate::rtext::TextEnv {
        if self.char_sets.is_empty() {
            if let Ok(root) = self.store.load_file("Fonts/character_sets.tgi") {
                for s in root.children.iter().filter(|c| c.name.eq_ignore_ascii_case("Set")) {
                    let g = |k: &str| s.int(k).unwrap_or(0) as u32;
                    self.char_sets.push((g("range_min"), g("range_max"), g("undisplayable_character"), g("spacing_standin")));
                }
            }
        }
        let fonts = self.ui_fonts();
        let av = |k: &str, d: f32| self.avars.float(k).unwrap_or(d);
        let default_ids = self.avars.get("TextFontIDSDefault").unwrap_or("small").to_string();
        crate::rtext::TextEnv {
            precision: self.precision,
            default_font: fonts.id(&default_ids),
            undisplayable: av("TextDefaultUndisplayableCharacter", 149.0) as u32,
            fallback: av("TextFallbackDefaultUndisplayableCharacter", 35.0) as u32,
            standin: av("TextDefaultSpacingStandinCharacter", 116.0) as u32,
            sets: self.char_sets.clone(),
            // registration defaults 27.0f (0x402DF7 / 0x402E3D)
            world_scale: av("TextWorldScaleFactor", 27.0),
            baseline_scale: av("TextBaselineScaleFactor", 27.0),
            screen: (self.screen.width as f32, self.screen.height as f32),
            reference: (self.reference.width, self.reference.height),
            ellipsis: self.strings.resolve("#grammar_ellipsis"),
        }
    }

    /// `0x735C00`: size of `text` in reference units for the font IDS,
    /// wrapped at `wrap` reference units (0 = no wrap): (w, h, rows).
    pub fn measure_text(&mut self, font_ids: &str, text: &str, wrap: f32) -> (f32, f32, i32) {
        let env = self.text_env();
        let fonts = self.ui_fonts();
        let id = fonts.id(font_ids);
        let rt = crate::rtext::RText { env: &env, table: fonts.as_ref() };
        match rt.ref_extent(text, id, false, false, wrap) {
            crate::rtext::Measured::Done { size, rows, .. } => (size[0], size[1], rows.unwrap_or(0)),
            crate::rtext::Measured::Hang => (0.0, 0.0, 0),
        }
    }

    /// `0x735D50` for a font IDS: (truncated, text) fitting `w` pixels.
    pub fn ellipsis_text(&mut self, font_ids: &str, text: &str, w: f32) -> (bool, String) {
        let env = self.text_env();
        let fonts = self.ui_fonts();
        let id = fonts.id(font_ids);
        let mut cache = self.ellipsis_cache.get(&id).copied();
        let rt = crate::rtext::RText { env: &env, table: fonts.as_ref() };
        let r = rt.ellipsis(text, w, 0.0, id, &mut cache);
        if let Some(c) = cache {
            self.ellipsis_cache.insert(id, c);
        }
        r
    }

    /// The label text rebuild `0x735B90` as the view element sets it up
    /// (`0x5E2C90`): area = the clipped outer rect, origin = (-clip left -
    /// edit box scroll, -clip top), colour VE+0xD4, highlight colour
    /// VE+0xE4 (`background_color`).
    pub fn label_rebuild(&mut self, text: &str, l: &LabelSpec) -> crate::rdraw::DrawnText {
        let env = self.text_env();
        let fonts = self.ui_fonts();
        let params = crate::rdraw::DrawParams {
            font: fonts.id(l.font),
            color: l.color,
            highlight: l.background,
            origin: (-l.clip[0] - l.scroll, -l.clip[1]),
            area: [l.rect.l, l.rect.t, l.rect.r, l.rect.b],
            justify: match l.justify {
                crate::ve::Justify::Left => 0,
                crate::ve::Justify::Center => 1,
                crate::ve::Justify::Right => 2,
            },
            multiline: l.multiline,
            fixed: false,
            debug_tint: false,
        };
        crate::rdraw::RDraw { env: &env, table: fonts.as_ref() }.rebuild(text, &params)
    }

    /// Select the nation skin (`Skins/<NATION>`) used for in-game art.
    pub fn set_skin(&mut self, nation: Option<&str>, skins_root: impl Into<std::path::PathBuf>) {
        self.store.root.set_skin(nation, skins_root);
        self.tex_size_cache.clear();
    }

    // ------------------------------------------------------------ layout

    /// Lay out the whole tree (`0x5E34B0` per view element).
    pub fn layout(&mut self) {
        self.root.update_effective(true, true);
        let (screen, reference, p) = (self.screen, self.reference, self.precision);
        let pressed = self.pressed.clone();
        layout_widget(&mut self.root, None, screen, reference, p, "", pressed.as_deref());
    }

    // ------------------------------------------------------------ drawing

    fn tex_size(&mut self, raw: &str) -> (String, Option<(u32, u32)>) {
        let path = crate::fs::resolve_resolution_path(raw, self.screen.width, |p| self.store.root.exists(p));
        let canonical = self.store.root.canonical(&path).unwrap_or_else(|| {
            if path.starts_with('/') { path.clone() } else { format!("/{path}") }
        });
        if let Some(s) = self.tex_size_cache.get(&canonical) {
            return (canonical, *s);
        }
        let size = self.store.root.read(&canonical).ok().and_then(|b| crate::fs::image_size(&b));
        self.tex_size_cache.insert(canonical.clone(), size);
        (canonical, size)
    }

    fn ttf(&mut self, face: &str, bold: bool) -> Option<Arc<TtfMetrics>> {
        let key = format!("{face}|{bold}");
        if let Some(m) = self.ttf_cache.get(&key) {
            return m.clone();
        }
        let bytes = if face.to_ascii_lowercase().ends_with(".ttf") {
            self.store.root.read(face).ok()
        } else {
            // GDI face name: the system font file (Windows Fonts folder)
            let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
            let stem = face.to_ascii_lowercase().replace(' ', "");
            let candidates = if bold { vec![format!("{stem}bd.ttf"), format!("{stem}.ttf")] } else { vec![format!("{stem}.ttf")] };
            candidates.iter().find_map(|c| std::fs::read(std::path::Path::new(&windir).join("Fonts").join(c)).ok())
        };
        let m = bytes.and_then(TtfMetrics::parse).map(Arc::new);
        self.ttf_cache.insert(key, m.clone());
        m
    }

    /// Font face for an IDS at the current screen height.
    pub fn font_face(&self, ids: &str) -> Option<FontFace> {
        let def = self
            .fonts
            .get(&ids.to_ascii_lowercase())
            .or_else(|| self.fonts.get(&self.avars.get("TextFontIDSDefault").unwrap_or("small").to_ascii_lowercase()))?;
        def.face_for(self.screen.height, self.reference.height)
    }

    /// Build the draw list for the current state.
    pub fn draw_list(&mut self) -> DrawList {
        let mut out = DrawList {
            screen: [self.screen.width, self.screen.height],
            skin: self.store.root.skin().map(str::to_string),
            commands: Vec::new(),
        };
        let root = std::mem::replace(&mut self.root, Widget::new(WidgetClass::Static, "", false));
        self.draw_widget(&root, "", &mut out);
        // the UI overlay (open drop-down list boxes) draws above the tree
        for dd in self.overlay.clone() {
            let rel = dd.split_once('/').map(|(_, r)| r).unwrap_or("");
            if let Some(lb) = path_ref(&root, rel).and_then(|d| d.child("Listbox")) {
                self.draw_widget_seg(lb, &dd, "Listbox", &mut out);
            }
        }
        self.root = root;
        // Two render lists (0x743750): non-blended sprites (0x742CC0: blend
        // off, z-write on) render before blended ones (0x742630: blend on,
        // z-write off); every view element takes a layer from one pre-order
        // counter over the whole tree (0x5C2420 -> vtable+0x40, 0x5E17E0)
        // and sprites sit at z = far - layer * far * 2^-24 (0x744677,
        // 0x744BB0). In the menus a later layer always covers an earlier
        // one whatever its list (the original options dialog hides the main
        // menu and its drop-down fields cover the earlier label bars,
        // reconstruction/native-game/qa/controls-v12-original-default-video.png):
        // the depth test resolves the layers, which is tree (painter's)
        // order. The loading screen instead shows its earlier blended
        // briefing box over the later opaque loading art (03-launch.png):
        // INFERRED, it is drawn by the loader's own present path without the
        // depth test, i.e. the two lists in order ([`DepthMode::TwoLists`]).
        if self.depth_mode == DepthMode::TwoLists {
            let (opaque, blended): (Vec<DrawCmd>, Vec<DrawCmd>) = out.commands.drain(..).partition(|c| match &c.kind {
                DrawKind::Quad(q) => q.blend != crate::draw::Blend::Alpha,
                DrawKind::Text(_) => false,
                DrawKind::Video(_) | DrawKind::External { .. } => true,
            });
            out.commands = opaque;
            out.commands.extend(blended);
        }
        out
    }

    fn draw_widget(&mut self, w: &Widget, parent_path: &str, out: &mut DrawList) {
        let seg = w.name.trim_start_matches('/').to_string();
        self.draw_widget_seg(w, parent_path, &seg, out)
    }

    fn draw_widget_seg(&mut self, w: &Widget, parent_path: &str, seg: &str, out: &mut DrawList) {
        let path = if parent_path.is_empty() { seg.to_string() } else { format!("{parent_path}/{seg}") };
        if w.flags & widget_flag::SHOWN == 0 || !w.loaded {
            return;
        }
        if let Some(v) = &w.view {
            self.draw_ve(v, w, &path, "View", out);
        }
        if let Some(v) = &w.sub_ve {
            let name = v.name.clone();
            self.draw_ve(v, w, &path, &name, out);
        }
        if w.class.internals_ported() {
            for (c, seg) in w.children.iter().zip(child_segments(w)) {
                if detached(w, c) {
                    continue;
                }
                self.draw_widget_seg(c, &path, &seg, out);
            }
        }
    }

    fn draw_ve(&mut self, ve: &ViewElement, w: &Widget, wpath: &str, vepath: &str, out: &mut DrawList) {
        if ve.layout.flags & crate::layout::ve_flag::VISIBLE == 0
            || !ve_visible(ve.when, w.flags, w.render_threshold, self.render_level)
        {
            return;
        }
        let rect = ve.layout.outer.rect;
        let (screen, reference) = (self.screen, self.reference);
        let push_quads = |quads: Vec<Quad>, out: &mut DrawList| {
            for q in quads {
                out.commands.push(DrawCmd { widget: wpath.to_string(), ve: vepath.to_string(), kind: DrawKind::Quad(q) });
            }
        };
        match &ve.kind {
            VeKind::Decal(d) => {
                let tex = d.texture.as_deref().map(|t| self.tex_size(t));
                let style = PieceStyle { color: d.color, blend: blend_of(d.alpha_blending) };
                let uv = match &tex {
                    Some((_, Some(size))) => decal_fit(rect, *size, d.tiled_x, d.tiled_y, d.stretch, self.precision),
                    _ => [0.0, 1.0, 1.0, 0.0],
                };
                let mut quads = Vec::new();
                piece_quads(
                    &mut quads,
                    tex.as_ref().map(|t| t.0.as_str()),
                    tex.as_ref().and_then(|t| t.1),
                    rect,
                    uv,
                    (d.tiled_x, d.tiled_y),
                    &style,
                    screen,
                    reference,
                    self.precision,
                );
                push_quads(quads, out);
            }
            VeKind::DecalTiled(t) => {
                let texes: Vec<Option<(String, Option<(u32, u32)>)>> =
                    t.textures.iter().map(|x| x.as_deref().map(|p| self.tex_size(p))).collect();
                let sizes: [Option<(u32, u32)>; 6] = std::array::from_fn(|i| texes[i].as_ref().and_then(|x| x.1));
                let rects = nine_slice_rects(rect, sizes);
                let style = PieceStyle { color: t.color, blend: blend_of(t.alpha_blending) };
                let mut quads = Vec::new();
                for (i, (tex_i, uv, tx, ty)) in sprite::NINE_SLICE_PIECES.iter().enumerate() {
                    let tex = texes[*tex_i].as_ref();
                    piece_quads(
                        &mut quads,
                        tex.map(|t| t.0.as_str()),
                        tex.and_then(|t| t.1),
                        rects[i],
                        *uv,
                        (*tx, *ty),
                        &style,
                        screen,
                        reference,
                        self.precision,
                    );
                }
                push_quads(quads, out);
            }
            VeKind::StatusBar(sb) => {
                // 0x5E3A00 / 0x5E2FE0 / 0x5E2100: one sprite in the value's
                // colour over the value's part of the rect
                let tex = sb.texture.as_deref().map(|t| self.tex_size(t));
                let style = PieceStyle { color: sb.color(), blend: blend_of(false) };
                let mut quads = Vec::new();
                piece_quads(
                    &mut quads,
                    tex.as_ref().map(|t| t.0.as_str()),
                    tex.as_ref().and_then(|t| t.1),
                    sb.rect(rect),
                    [0.0, 1.0, 1.0, 0.0],
                    (false, false),
                    &style,
                    screen,
                    reference,
                    self.precision,
                );
                push_quads(quads, out);
            }
            VeKind::DecalTiledDirectional(d) => {
                let texes: Vec<Option<(String, Option<(u32, u32)>)>> =
                    d.textures.iter().map(|x| x.as_deref().map(|p| self.tex_size(p))).collect();
                let ends = texes[0].as_ref().and_then(|t| t.1);
                let style = PieceStyle { color: d.color, blend: blend_of(d.alpha_blending) };
                let mut quads = Vec::new();
                for (tex_i, r, uv, tx, ty) in directional_pieces(rect, ends, d.horizontal) {
                    let tex = texes[tex_i].as_ref();
                    piece_quads(&mut quads, tex.map(|t| t.0.as_str()), tex.and_then(|t| t.1), r, uv, (tx, ty), &style, screen, reference, self.precision);
                }
                push_quads(quads, out);
            }
            VeKind::Label(l) => {
                let state = state::state_bits(w.flags);
                // 0x5E2C10: the first override (list head = last declared)
                // whose when-mask matches supplies its attributes.
                let ov = l.overrides.iter().find(|o| o.when & state == o.when);
                let text = ov.and_then(|o| o.text.clone()).unwrap_or_else(|| l.text.clone());
                let font = ov.and_then(|o| o.font.clone()).unwrap_or_else(|| l.font.clone());
                let justify = ov.and_then(|o| o.justify).unwrap_or(l.justify);
                let color = ov.and_then(|o| o.color).unwrap_or(l.color);
                let background = ov.and_then(|o| o.background_color).unwrap_or(l.background_color);
                if !text.is_empty() {
                    let spec = LabelSpec {
                        font: &font,
                        rect,
                        clip: ve.layout.outer.clip,
                        justify,
                        multiline: l.multiline,
                        color,
                        background,
                        scroll: l.scroll_px,
                    };
                    let drawn = self.label_rebuild(&text, &spec);
                    let fonts = self.ui_fonts();
                    let mut faces: Vec<crate::draw::TextFace> = Vec::new();
                    let mut glyphs = Vec::new();
                    for q in &drawn.quads {
                        let f = fonts.font(q.font);
                        let ids = f.map(|f| f.ids.clone()).unwrap_or_default();
                        let face = match faces.iter().position(|x| x.ids == ids) {
                            Some(i) => i,
                            None => {
                                faces.push(crate::draw::TextFace { ids, face: f.map(|f| f.face.clone()) });
                                faces.len() - 1
                            }
                        };
                        glyphs.push(crate::draw::GlyphQuad {
                            c: q.glyph,
                            face,
                            rect: Rect::new(q.geo[0], q.geo[1], q.geo[2], q.geo[3]),
                            cell: [q.geo[4], q.geo[5]],
                            color: q.color,
                            highlight: q.highlight,
                            image: f.and_then(|f| f.images.get(&q.glyph)).map(|i| i.path.clone()),
                        });
                    }
                    // justification comes from the Justify enum (no 0x404EA0
                    // error); a null-font crash case draws nothing
                    out.commands.push(DrawCmd {
                        widget: wpath.to_string(),
                        ve: vepath.to_string(),
                        kind: DrawKind::Text(Text {
                            text,
                            face: self.font_face(&font),
                            font,
                            faces,
                            glyphs,
                            rect,
                            justify,
                            multiline: l.multiline,
                            color,
                            clip: Some(rect),
                        }),
                    });
                }
            }
            VeKind::Video => {
                out.commands.push(DrawCmd {
                    widget: wpath.to_string(),
                    ve: vepath.to_string(),
                    kind: DrawKind::Video(Video { resources: w.videos.clone(), repeating: w.repeating_video, rect }),
                });
            }
            VeKind::Nif { .. } | VeKind::Unsupported(_) => {
                out.commands.push(DrawCmd {
                    widget: wpath.to_string(),
                    ve: vepath.to_string(),
                    kind: DrawKind::External {
                        ve_type: format!("{:?}", ve.ve_type),
                        rect,
                        model: match w.control.as_deref() {
                            Some(crate::controls::Control::Model(m)) => Some(m.clone()),
                            _ => None,
                        },
                    },
                });
            }
            VeKind::Group { .. } | VeKind::Dummy => {}
        }
        for (i, c) in ve.children.iter().enumerate() {
            let p = format!("{vepath}/{}[{i}]", c.name);
            self.draw_ve(c, w, wpath, &p, out);
        }
    }

    // ------------------------------------------------------------ input

    pub fn take_events(&mut self) -> Vec<UiEvent> {
        std::mem::take(&mut self.events)
    }
}

fn layout_tree(ve: &mut ViewElement, parent: &VeLayout, screen: Screen, reference: Reference, p: Precision) {
    layout_ve(&mut ve.layout, Some(parent), screen, reference, p);
    let me = ve.layout;
    for c in &mut ve.children {
        layout_tree(c, &me, screen, reference, p);
    }
}

fn layout_widget(
    w: &mut Widget,
    parent: Option<&VeLayout>,
    screen: Screen,
    reference: Reference,
    p: Precision,
    parent_path: &str,
    pressed: Option<&str>,
) {
    let path = if parent_path.is_empty() {
        w.name.trim_start_matches('/').to_string()
    } else {
        format!("{parent_path}/{}", w.name.trim_start_matches('/'))
    };
    // a pressed button's client takes the pressed client bounds (0x5F2DF0)
    if w.class.is_button() {
        let _ = pressed;
        let is_pressed = w.flags & widget_flag::PRESSED != 0;
        let b = if is_pressed { w.pressed_client } else { w.client_bounds };
        let client = w.client_name.clone();
        if let (Some(b), Some(c)) = (b, w.children.iter_mut().find(|c| c.name == client)) {
            c.root.outer.raw = b.raw;
            c.root.outer.flags = b.flags;
        }
    }
    layout_ve(&mut w.root, parent, screen, reference, p);
    let me = w.root;
    if let Some(v) = &mut w.view {
        layout_tree(v, &me, screen, reference, p);
    }
    if let Some(v) = &mut w.sub_ve {
        layout_tree(v, &me, screen, reference, p);
    }
    let dropdown_active = w.class == WidgetClass::DropDown && w.flags & widget_flag::ACTIVE != 0;
    for c in &mut w.children {
        // an active drop-down's list box belongs to the UI overlay (0x5D4430)
        let parent = if dropdown_active && c.class == WidgetClass::Listbox { None } else { Some(&me) };
        layout_widget(c, parent, screen, reference, p, &path, pressed);
    }
}

/// An active drop-down's list box is not its child for drawing and
/// picking (Activate `0x5D4430` removes it; while open it is drawn by the
/// UI overlay).
pub(crate) fn detached(parent: &Widget, child: &Widget) -> bool {
    parent.class == WidgetClass::DropDown && parent.flags & widget_flag::ACTIVE != 0 && child.class == WidgetClass::Listbox
}

/// Rect helper for tests and callers: the laid-out root rect of a widget.
pub fn widget_rect(w: &Widget) -> Rect {
    w.root.outer.rect
}
