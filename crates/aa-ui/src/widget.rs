//! Widgets (`TGC_Widget` and subclasses, `.\Source\Ui\System\Widget.cpp`).
//!
//! * Constructor `0x5F1D00(name, reg)`: flags 0x80007 (visible, effective
//!   visible, enabled, effective enabled), render threshold 10, a root group
//!   view element (flags |= 2: sorted by the widget key); a `file:node` name
//!   is split on `:`.
//! * AddChild `0x5F0080(child, front)`: children sorted by the widget key
//!   (vtable+0xA8, 0.0 for every widget except MenuBackground's -FLT_MAX at
//!   0x652C20); equal keys keep insertion order (append, or head with
//!   `front`). The child's root view element joins the parent's root group
//!   (`0x5EFA10` -> `0x5E37A0`); a widget's own view elements use the key
//!   -FLT_MAX and therefore sit below its child widgets.
//! * `TGC_Widget::LoadData` `0x5F2070`: data node = own file root + node
//!   path, else `parent_node / name` (`0x5B4410`, leading `/` = file root);
//!   `tooltip_name`, `ToolTip`, `intercept_pick` (flag 0x200000), optional
//!   `RenderThreshold` (flag 0x40) else the parent's threshold, `View`
//!   (`0x5E4B40` into the root group; the root takes the view's outer and
//!   client bounds (`0x5E29B0`, `0x5EEB80`) and the view is reset to
//!   `0,0,0r,0b`), `visible` while still visible, then every code-created
//!   child with this node. TGI child widgets that code does not create are
//!   ignored.
//! * Panel `0x5F2C10` (`set_sound`, `unset_sound`), top-level panel
//!   `0x5F2D60` (anchored tooltip fields), interface `0x5C0B50`
//!   (`interface_music`, `show_menu_background`,
//!   `show_logo_and_build_stamp`), button `0x5F2DF0` (sounds, client
//!   bounds saved by `0x5EF8A0`, `pressed_client_*` via `0x5E3150`), label
//!   `0x5F29B0` (`[Label]` view element of type LABEL via `0x5E4C20`), decal
//!   `0x5F28B0` (`[Decal]`), cinematic (`[VideoResource]` list).

use crate::layout::{Bounds, VeLayout, parse_bounds};
use crate::node::UiNode;
use crate::state::widget_flag;
use crate::ve::ViewElement;
use crate::tgi::{TgiStore, find_path};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Native widget classes (constructors/vtables in research/phase8-ui).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WidgetClass {
    /// `0x5F1D00`, vtable 0xA52350.
    Static,
    /// `0x5F2730`, vtable 0xA52418.
    Panel,
    /// `0x5F32F0`, vtable 0xA52C60 (Root, dialogs, top-level menus).
    TopLevel,
    /// `0x5C0740`, vtable 0xA4B638.
    Interface,
    /// `0x5F2830`, vtable 0xA52698.
    PushButton,
    /// vtable 0xA525C8.
    ToggleButton,
    /// `0x5F2980`, vtable 0xA52838.
    Label,
    /// `0x5F2880`, vtable 0xA52770.
    Decal,
    /// `0x5CDF50`, vtable 0xA4DBD0.
    Cinematic,
    /// `0x5EDAA0`, vtable 0xA52108.
    Viewport,
    /// `0x5D4C80`.
    Listbox,
    /// `0x5D5C30`.
    DropDown,
    /// `0x5D1000`.
    Editbox,
    /// `0x5D7B80`.
    NumericEditbox,
    /// `0x5DE560`.
    Scrollbar,
    /// `0x5DE360`.
    Slider,
    /// `0x5DEB20` (slider with Min/Max/CurrentValue labels).
    SliderAdvanced,
    /// `0x5DE270` slider grip.
    Grip,
    /// `0x5D3D70` list item (label subclass).
    ListItem,
    /// `0x5DC540` selectable panel (staging slots); selection groups
    /// `0x5DC6E0` use [`Widget::radio_group`].
    SelectablePanel,
    /// `0x5D71A0`.
    MiniMap,
    /// `0x5FD6D0` message log.
    MessageLog,
    /// StatusBarWidget `0x5DEF90`, vtable 0xA50648 (LoadData `0x5DF0F0`):
    /// a `[StatusBar]` view element (type 7) and, with
    /// `has_target_percentage`, a `TargetPercentage` decal widget.
    StatusBar,
    /// ModelWidget `0x6236B0` (`[Model]` view element, drawn by the
    /// renderer into its rect).
    Model,
}

impl WidgetClass {
    pub fn is_button(self) -> bool {
        matches!(self, Self::PushButton | Self::ToggleButton)
    }

    /// Whether the class code that positions, shows and fills this class's
    /// own children is ported. When it is not, the widget's `[View]` is
    /// drawn but its children are neither drawn nor picked.
    pub fn internals_ported(self) -> bool {
        true
    }
}

/// Interface fields (`0x5C0B50`).
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct InterfaceProps {
    pub music: String,
    pub show_menu_background: bool,
    pub show_logo_and_build_stamp: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Widget {
    /// Name as constructed (`/MainMenu`, `MainMenuButtonSinglePlayer`,
    /// `Client`); for a `file:node` name this is the node path.
    pub name: String,
    /// Own data file (`UI/Menus/main.tgi`) from a `file:node` name.
    pub file: Option<String>,
    pub class: WidgetClass,
    /// Registered with `KKC_UI::RegisterWidget` (`0x5C9700`).
    pub registered: bool,
    /// Native flags (widget+0x24).
    pub flags: u32,
    pub render_threshold: i32,
    /// Sort key (vtable+0xA8).
    pub key: f32,
    /// Root group view element bounds (the view's authored bounds).
    pub root: VeLayout,
    pub view: Option<ViewElement>,
    /// `[Label]` / `[Decal]` view element of label and decal widgets.
    pub sub_ve: Option<ViewElement>,
    pub children: Vec<Widget>,
    /// Console command string fired by a button (`0x50CBE0`).
    pub command: Option<String>,
    /// Name of a button's client child (SetClient `0x5F0100`): "Client" for
    /// `0x5F3440` buttons, "MessageBoxButtonLabel" for message boxes.
    pub client_name: String,
    pub tooltip_name: String,
    pub tooltip: String,
    /// Button client bounds when unpressed (`0x5EF8A0`) and pressed.
    pub client_bounds: Option<Bounds>,
    pub pressed_client: Option<Bounds>,
    pub interface: Option<InterfaceProps>,
    pub videos: Vec<String>,
    pub repeating_video: bool,
    pub set_sound: String,
    pub unset_sound: String,
    pub loaded: bool,
    pub diagnostics: Vec<String>,
    /// Class state of controls (`controls.rs`).
    #[serde(default)]
    pub control: Option<Box<crate::controls::Control>>,
    /// Radio group of a toggle button (tab buttons, `0x5F0420`); 0 = none.
    #[serde(default)]
    pub radio_group: u32,
    /// Identity for code that keeps several same-named children (credit
    /// labels); 0 = none. Not native data.
    #[serde(default)]
    pub tag: u64,
    /// Toggle button +0xE8 cleared (ctor default 1): a click does not flip
    /// SELECTED (`0x5F11A0`), the command still fires.
    #[serde(default)]
    pub no_auto_toggle: bool,
}

pub const INITIAL_FLAGS: u32 = 0x80007;

impl Widget {
    /// `0x5F1D00` (`reg` = register with the UI by the last path part).
    pub fn new(class: WidgetClass, name: &str, registered: bool) -> Self {
        let (file, name) = match name.split_once(':') {
            Some((f, n)) => (Some(f.to_string()), n.to_string()),
            None => (None, name.to_string()),
        };
        let mut root = VeLayout::default();
        root.flags = 0x1004 | 0x2;
        Widget {
            name,
            file,
            class,
            registered,
            flags: INITIAL_FLAGS,
            render_threshold: 10,
            key: 0.0,
            root,
            view: None,
            sub_ve: None,
            children: Vec::new(),
            command: None,
            client_name: "Client".into(),
            tooltip_name: String::new(),
            tooltip: String::new(),
            client_bounds: None,
            pressed_client: None,
            interface: None,
            videos: Vec::new(),
            repeating_video: false,
            set_sound: String::new(),
            unset_sound: String::new(),
            loaded: false,
            diagnostics: Vec::new(),
            control: None,
            radio_group: 0,
            tag: 0,
            no_auto_toggle: false,
        }
    }

    /// Push button with its unregistered static `Client` child
    /// (`0x5F3440(&name, reg, &command, &clientName)`, client `0x5F2BB0`,
    /// attached by SetClient `0x5F0100`).
    pub fn push_button(name: &str, registered: bool, command: &str) -> Self {
        let mut b = Self::new(WidgetClass::PushButton, name, registered);
        b.command = Some(command.to_string());
        b.add_child(Self::new(WidgetClass::Static, "Client", false));
        b
    }

    /// Toggle button with a client (`0x5F3080`..`0x5F3190`).
    pub fn toggle_button(name: &str, registered: bool, command: &str) -> Self {
        let mut b = Self::new(WidgetClass::ToggleButton, name, registered);
        b.command = Some(command.to_string());
        b.add_child(Self::new(WidgetClass::Static, "Client", false));
        b
    }

    /// Native composite constructors that create fixed-name children
    /// (names from the constructors' string references).
    pub fn composite(class: WidgetClass, name: &str, reg: bool) -> Self {
        Self::composite_with(class, name, reg, false)
    }

    /// Composite constructors; `vertical` is the slider/scroll bar flag
    /// (0x1000).
    pub fn composite_with(class: WidgetClass, name: &str, reg: bool, vertical: bool) -> Self {
        use crate::controls as k;
        use WidgetClass as C;
        let mut x = Self::new(class, name, reg);
        match class {
            // 0x5DE560: Slider, LessButton (+Client), MoreButton (+Client);
            // sort key 0.2 (0x5DD150)
            C::Scrollbar => {
                x.key = 0.2;
                let mut slider = Self::composite_with(C::Slider, "Slider", false, vertical);
                if let Some(s) = slider.slider_mut() {
                    s.callback = k::SliderCallback::Owner;
                }
                x.add_child(slider);
                let mut less = Self::new(C::PushButton, "LessButton", false);
                less.add_child(Self::new(C::Static, "Client", false));
                x.add_child(less);
                let mut more = Self::new(C::PushButton, "MoreButton", false);
                more.add_child(Self::new(C::Static, "Client", false));
                x.add_child(more);
            }
            // 0x5DE360: Grip (0x5DE270) with its DisableDecal (0x5F2880)
            C::Slider | C::SliderAdvanced => {
                if vertical {
                    x.flags |= crate::state::widget_flag::VERTICAL;
                }
                let mut grip = Self::new(C::Grip, "Grip", false);
                grip.add_child(Self::new(C::Decal, "DisableDecal", false));
                x.add_child(grip);
                let mut s = k::Slider::default();
                if class == C::SliderAdvanced {
                    // 0x5DEB20: the three labels
                    x.add_child(Self::new(C::Label, "MinBoundLabel", false));
                    x.add_child(Self::new(C::Label, "MaxBoundLabel", false));
                    x.add_child(Self::new(C::Label, "CurrentValueLabel", false));
                    s.advanced = Some((true, true, k::SliderFormat::Default));
                }
                x.control = Some(Box::new(k::Control::Slider(s)));
            }
            // 0x5D4C80: Scrollbar (vertical, 0x5DE770)
            C::Listbox => {
                x.add_child(Self::composite_with(C::Scrollbar, "Scrollbar", false, true));
                x.control = Some(Box::new(k::Control::Listbox(k::Listbox::default())));
            }
            // 0x5D5C30: Listbox (+0x6C, select functor 0x5D5540) and the
            // SelectedItem list item (+0x74, no owner, index -1)
            C::DropDown => {
                let mut lb = Self::composite(C::Listbox, "Listbox", false);
                if let Some(l) = lb.listbox_mut() {
                    l.on_select = k::ListCallback::DropDown;
                }
                x.add_child(lb);
                let mut sel = Self::new(C::ListItem, "SelectedItem", false);
                sel.control = Some(Box::new(k::Control::ListItem(k::ListItem::default())));
                x.add_child(sel);
                x.control = Some(Box::new(k::Control::DropDown(k::DropDown::default())));
            }
            // 0x5D1000: Label (+0x70), Cursor (+0x8C)
            C::Editbox => {
                x.add_child(Self::new(C::Label, "Label", false));
                x.add_child(Self::new(C::Label, "Cursor", false));
                x.control = Some(Box::new(k::Control::Editbox(k::Editbox { integer_max: -1, ..Default::default() })));
            }
            // 0x5D7B80: UpButton (+0x70), DownButton (+0x74), Editbox (+0x6C,
            // integer_only); step +0x80 = 1
            C::NumericEditbox => {
                for (n, step) in [("UpButton", "+1"), ("DownButton", "-1")] {
                    let mut b = Self::new(C::PushButton, n, false);
                    let mut client = Self::new(C::Label, "Client", false);
                    client.tooltip_name = step.to_string();
                    b.add_child(client);
                    x.add_child(b);
                }
                let mut e = Self::composite(C::Editbox, "Editbox", false);
                if let Some(eb) = e.editbox_mut() {
                    eb.integer_only = true;
                }
                x.add_child(e);
                x.control = Some(Box::new(k::Control::Numeric(1)));
            }
            // 0x5EDAA0: ItemPanel (+0x6C)
            C::Viewport => {
                x.add_child(Self::new(C::Panel, "ItemPanel", false));
                x.control = Some(Box::new(k::Control::Viewport(k::Viewport::default())));
            }
            _ => {}
        }
        x
    }

    /// AddChild appending at the tail (panel vtable+0xD0).
    pub fn add_child(&mut self, child: Widget) -> &mut Widget {
        let key = child.key;
        let at = self.children.iter().rposition(|c| c.key <= key).map(|i| i + 1).unwrap_or(0);
        self.children.insert(at, child);
        &mut self.children[at]
    }

    /// AddChild at the head of its key group (vtable+0xD4).
    pub fn add_child_front(&mut self, child: Widget) -> &mut Widget {
        let key = child.key;
        let at = self.children.iter().position(|c| c.key >= key).unwrap_or(self.children.len());
        self.children.insert(at, child);
        &mut self.children[at]
    }

    pub fn child(&self, name: &str) -> Option<&Widget> {
        self.children.iter().find(|c| c.name.trim_start_matches('/').eq_ignore_ascii_case(name.trim_start_matches('/')))
    }

    pub fn child_mut(&mut self, name: &str) -> Option<&mut Widget> {
        self.children
            .iter_mut()
            .find(|c| c.name.trim_start_matches('/').eq_ignore_ascii_case(name.trim_start_matches('/')))
    }

    /// Find a descendant by `/`-separated path of names.
    pub fn find(&self, path: &str) -> Option<&Widget> {
        let mut at = self;
        for p in path.split('/').filter(|p| !p.is_empty()) {
            at = at.child(p)?;
        }
        Some(at)
    }

    pub fn find_mut(&mut self, path: &str) -> Option<&mut Widget> {
        let mut at = self;
        for p in path.split('/').filter(|p| !p.is_empty()) {
            at = at.child_mut(p)?;
        }
        Some(at)
    }

    /// `SetVisible` (flag bit 0).
    pub fn set_visible(&mut self, v: bool) {
        if v {
            self.flags |= 1;
        } else {
            self.flags &= !1;
        }
    }

    /// `0x5EFB30`: enable/disable (flag bit 2).
    pub fn set_enabled(&mut self, v: bool) {
        if v {
            self.flags |= 4;
        } else {
            self.flags &= !4;
        }
    }

    /// Propagate the effective visible (bit 1, `0x5EFB00`) and effective
    /// enabled (bit 19, `0x5EFB30`) flags from the parent.
    pub fn update_effective(&mut self, parent_visible: bool, parent_enabled: bool) {
        let vis = parent_visible && self.flags & 1 != 0;
        let en = parent_enabled && self.flags & 4 != 0;
        self.flags = (self.flags & !(widget_flag::SHOWN | widget_flag::ENABLED))
            | if vis { widget_flag::SHOWN } else { 0 }
            | if en { widget_flag::ENABLED } else { 0 };
        for c in &mut self.children {
            c.update_effective(vis, en);
        }
    }

    /// Load this widget and its code-created children.
    ///
    /// `parent`: the parent's data node and that node's file root.
    pub fn load(
        &mut self,
        store: &mut TgiStore,
        parent: Option<(&UiNode, &Arc<UiNode>)>,
        parent_threshold: Option<i32>,
        strings: &dyn Fn(&str) -> String,
    ) {
        let (node, file_root): (UiNode, Arc<UiNode>) = match (&self.file, parent) {
            (Some(file), _) => match store.load_file(file) {
                Ok(root) => match find_path(&root, &self.name) {
                    Some(n) => (n.clone(), root.clone()),
                    None => {
                        self.diagnostics.push(format!("Missing data node '{}' loading widget", self.name));
                        return;
                    }
                },
                Err(e) => {
                    self.diagnostics.push(e);
                    return;
                }
            },
            (None, Some((pnode, proot))) => {
                let found = if self.name.starts_with('/') { find_path(proot, &self.name) } else { find_path(pnode, &self.name) };
                match found {
                    Some(n) => (n.clone(), proot.clone()),
                    None => {
                        self.diagnostics.push(format!("SubNode not found: '{}' under '{}'", self.name, pnode.name));
                        return;
                    }
                }
            }
            (None, None) => {
                self.diagnostics.push(format!("Unable to determine skinning hierarchy for widget '{}'", self.name));
                return;
            }
        };
        self.load_node(&node, &file_root, store, parent_threshold, strings);
    }

    fn load_node(
        &mut self,
        node: &UiNode,
        file_root: &Arc<UiNode>,
        store: &mut TgiStore,
        parent_threshold: Option<i32>,
        strings: &dyn Fn(&str) -> String,
    ) {
        self.tooltip_name = strings(&node.string_or_empty("tooltip_name"));
        self.tooltip = strings(&node.string_or_empty("ToolTip"));
        if node.bool_or("intercept_pick", false).unwrap_or(false) {
            self.flags |= 0x200000;
        } else {
            self.flags &= !0x200000;
        }
        match node.child("RenderThreshold") {
            Some(rt) => {
                self.flags |= 0x40;
                self.render_threshold = rt.int("render_threshold").unwrap_or(self.render_threshold);
            }
            None => {
                if let Some(t) = parent_threshold {
                    self.render_threshold = t;
                }
            }
        }
        match node.child("View") {
            Some(vn) => match ViewElement::load_localized(vn, strings) {
                Ok(mut view) => {
                    // 0x5E29B0 / 0x5EEB80: the root takes the view's bounds,
                    // the view fills the root.
                    self.root.outer = view.layout.outer;
                    self.root.client = view.layout.client;
                    let mut full = Bounds::default();
                    parse_bounds(&mut full, "0", "0", "0r", "0b");
                    view.layout.outer = full;
                    view.layout.client = full;
                    self.view = Some(view);
                }
                Err(e) => self.diagnostics.push(e),
            },
            None => self.diagnostics.push(format!("SubNode not found: 'view' in '{}'", node.name)),
        }
        if self.flags & 1 != 0 {
            let v = node.bool_or("visible", true).unwrap_or(true);
            self.set_visible(v);
        }
        // subclass data
        match self.class {
            WidgetClass::Panel | WidgetClass::TopLevel | WidgetClass::Interface | WidgetClass::SelectablePanel => {
                self.set_sound = node.string_or_empty("set_sound");
                self.unset_sound = node.string_or_empty("unset_sound");
                if self.class == WidgetClass::Interface {
                    self.interface = Some(InterfaceProps {
                        music: node.string_or_empty("interface_music"),
                        show_menu_background: node.bool_or("show_menu_background", false).unwrap_or(false),
                        show_logo_and_build_stamp: node.bool_or("show_logo_and_build_stamp", false).unwrap_or(false),
                    });
                }
            }
            WidgetClass::PushButton | WidgetClass::ToggleButton => {
                self.set_sound = node.string_or_empty("set_sound");
                self.unset_sound = node.string_or_empty("unset_sound");
                let get = |k: &str| node.string_or_empty(k);
                let mut pressed = Bounds::default();
                if parse_bounds(
                    &mut pressed,
                    &get("pressed_client_L"),
                    &get("pressed_client_T"),
                    &get("pressed_client_R"),
                    &get("pressed_client_B"),
                ) {
                    self.pressed_client = Some(pressed);
                } else {
                    self.diagnostics.push(format!("Unable to parse pressed client bounds for '{}'", self.name));
                }
            }
            WidgetClass::Label | WidgetClass::ListItem => {
                if let Some(ln) = node.child("Label") {
                    match ViewElement::load_localized(ln, strings) {
                        Ok(ve) => {
                            if ve.ve_type != crate::ve::VeType::Label {
                                self.diagnostics.push(format!("View element '{}' should be type 'LABEL'", ln.name));
                            }
                            self.sub_ve = Some(ve);
                        }
                        Err(e) => self.diagnostics.push(e),
                    }
                } else {
                    self.diagnostics.push(format!("SubNode not found: 'label' in '{}'", node.name));
                }
            }
            WidgetClass::StatusBar => {
                match node.child("StatusBar") {
                    Some(sn) => match ViewElement::load_localized(sn, strings) {
                        Ok(ve) => self.sub_ve = Some(ve),
                        Err(e) => self.diagnostics.push(e),
                    },
                    None => self.diagnostics.push(format!("SubNode not found: 'StatusBar' in '{}'", node.name)),
                }
                // has_target_percentage (+0x7C): the TargetPercentage decal
                // widget is created and added, else removed
                let has_target = node.bool_or("has_target_percentage", false).unwrap_or(false);
                if has_target {
                    if self.child("TargetPercentage").is_none() {
                        self.add_child(Self::new(WidgetClass::Decal, "TargetPercentage", false));
                    }
                } else {
                    self.children.retain(|c| !c.name.eq_ignore_ascii_case("TargetPercentage"));
                }
            }
            WidgetClass::Model => {
                if let Some(mn) = node.child("Model") {
                    match ViewElement::load_localized(mn, strings) {
                        Ok(ve) => self.sub_ve = Some(ve),
                        Err(e) => self.diagnostics.push(e),
                    }
                } else {
                    self.diagnostics.push(format!("SubNode not found: 'Model' in '{}'", node.name));
                }
            }
            WidgetClass::Decal => {
                if let Some(dn) = node.child("Decal") {
                    match ViewElement::load_localized(dn, strings) {
                        Ok(ve) => self.sub_ve = Some(ve),
                        Err(e) => self.diagnostics.push(e),
                    }
                } else {
                    self.diagnostics.push(format!("SubNode not found: 'decal' in '{}'", node.name));
                }
            }
            WidgetClass::Cinematic => {
                self.videos =
                    node.children_named("VideoResource").map(|v| v.string_or_empty("video_resource")).collect();
                self.repeating_video = node.bool_or("repeating_video", false).unwrap_or(false);
            }
            _ => {}
        }
        // Viewport LoadData 0x5EDB50: a [Scrollbar] node adds a scrollbar
        if matches!(self.class, WidgetClass::Viewport | WidgetClass::MessageLog) && node.child("Scrollbar").is_some() && self.child("Scrollbar").is_none() {
            let mut sb = Self::composite_with(WidgetClass::Scrollbar, "Scrollbar", false, true);
            if let Some(sl) = sb.child_mut("Slider").and_then(|s| s.slider_mut()) {
                sl.callback = crate::controls::SliderCallback::Owner;
            }
            self.add_child(sb);
        }
        crate::control_load::load_control(self, node, strings);
        self.loaded = true;
        let threshold = self.render_threshold;
        for child in &mut self.children {
            child.load(store, Some((node, file_root)), Some(threshold), strings);
        }
        crate::control_load::after_children(self);
        // button client bounds saved after the client loaded (0x5EF8A0)
        if self.class.is_button() {
            let client = self.client_name.clone();
            if let Some(c) = self.children.iter().find(|c| c.name == client) {
                self.client_bounds = Some(c.root.outer);
            }
        }
    }

    /// Visit every widget depth-first (pre-order).
    pub fn walk<'a>(&'a self, path: &str, f: &mut dyn FnMut(&str, &'a Widget)) {
        let p = if path.is_empty() { self.name.trim_start_matches('/').to_string() } else { format!("{path}/{}", self.name.trim_start_matches('/')) };
        f(&p, self);
        for c in &self.children {
            c.walk(&p, f);
        }
    }

    /// Collect diagnostics of the whole tree.
    pub fn all_diagnostics(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.walk("", &mut |p, w| {
            for d in &w.diagnostics {
                out.push(format!("{p}: {d}"));
            }
            for ve in w.view.iter().chain(w.sub_ve.iter()) {
                ve.walk(&mut |v| {
                    for d in &v.diagnostics {
                        out.push(format!("{p} ({}): {d}", v.name));
                    }
                });
            }
        });
        out
    }
}
