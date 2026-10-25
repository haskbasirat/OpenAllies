//! View elements (`TGC_ViewElement` and subclasses,
//! `.\Source\Ui\System\ViewElement.cpp`, `ViewElementDecal.cpp`).
//!
//! Factory `0x5E4B40`: reads the node's `type` and maps it through the table
//! built by `0x5E45A0` (DUMMY 0, GROUP 1, DECAL 2, DECAL_TILED 3,
//! DECAL_TILED_DIRECTIONAL 4, VIDEO 5, LABEL 6, STATUS_BAR 7, MODEL 8,
//! MINIMAP 9, SCORE_GRAPH 10, NIF 11), constructs the class (`0x5E4670`),
//! calls its `Load` (vtable slot 0) and inserts it into the parent group
//! (`0x5E37A0`, sorted insert `0x5EF900`: ascending key, equal keys keep
//! insertion order; plain view elements use the key -FLT_MAX).
//!
//! Loaders:
//! * base `0x5E5780`: L/T/R/B, client_L/T/R/B (`0x5E3150`), `when` (`0x5E54A0`);
//! * group `0x5E5A60`: `validate_decal_children` (default true), every
//!   `Child`, `transparent`;
//! * decal `0x5E81E0`: `color`, optional `Animated`, `texture`, `tiled_x`,
//!   `tiled_y`, `alpha_testing`, `alpha_blending`, `stretch`;
//! * decal base `0x5E68D0` (tiled kinds): `color`, `alpha_testing`,
//!   `alpha_blending`;
//! * tiled `0x5E8510`: corners/left/right/top/bottom/center textures;
//! * directional `0x5E8730`: ends/center textures, `direction`;
//! * label `0x5E5BA0`: `text`, `font` (`0x5E4220`), `justify` (`0x5E4000`),
//!   `color`, `background_color`, `multiline`, `Override` list.

use crate::layout::{Bounds, VeLayout, parse_bounds};
use crate::node::UiNode;
use crate::state::{parse_attributes, parse_when};
use serde::{Deserialize, Serialize};

/// Native view-element type ids (`0x5E45A0`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VeType {
    Dummy = 0,
    Group = 1,
    Decal = 2,
    DecalTiled = 3,
    DecalTiledDirectional = 4,
    Video = 5,
    Label = 6,
    StatusBar = 7,
    Model = 8,
    MiniMap = 9,
    ScoreGraph = 10,
    Nif = 11,
}

impl VeType {
    pub fn from_type_name(name: &str) -> Option<Self> {
        Some(match name.trim().to_ascii_uppercase().as_str() {
            "DUMMY" => Self::Dummy,
            "GROUP" => Self::Group,
            "DECAL" => Self::Decal,
            "DECAL_TILED" => Self::DecalTiled,
            "DECAL_TILED_DIRECTIONAL" => Self::DecalTiledDirectional,
            "VIDEO" => Self::Video,
            "LABEL" => Self::Label,
            "STATUS_BAR" => Self::StatusBar,
            "MODEL" => Self::Model,
            "MINIMAP" => Self::MiniMap,
            "SCORE_GRAPH" => Self::ScoreGraph,
            "NIF" => Self::Nif,
            _ => return None,
        })
    }
}

/// Text justification (`0x5E4000`; table values per the label renderer).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Justify {
    #[default]
    Left,
    Center,
    Right,
}

impl Justify {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "LEFT" => Some(Self::Left),
            "CENTER" => Some(Self::Center),
            "RIGHT" => Some(Self::Right),
            _ => None,
        }
    }
}

/// Animated decal (`[optional Animated]`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Animation {
    pub fps: f32,
    pub loops: bool,
    /// Frame texture paths `"%s%d.%s"` of prefix, 1-based index, file type.
    pub frames: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecalData {
    pub color: [f32; 4],
    /// Raw `texture` value (`%s` unresolved); `None` when empty.
    pub texture: Option<String>,
    pub animation: Option<Animation>,
    pub tiled_x: bool,
    pub tiled_y: bool,
    pub alpha_testing: bool,
    pub alpha_blending: bool,
    pub stretch: bool,
    /// Texture effect mode set by code (`0x5E66E0`: 1 while the owning
    /// button is disabled), see `draw::Quad::effect`.
    #[serde(default)]
    pub effect: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TiledData {
    pub color: [f32; 4],
    pub alpha_testing: bool,
    pub alpha_blending: bool,
    /// corners, left, right, top, bottom, center (native order at VE+200).
    pub textures: [Option<String>; 6],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectionalData {
    pub color: [f32; 4],
    pub alpha_testing: bool,
    pub alpha_blending: bool,
    /// ends, center.
    pub textures: [Option<String>; 2],
    /// `direction = horizontal` (VE+0xE8 = 1) or vertical (0).
    pub horizontal: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LabelOverride {
    pub when: u32,
    pub attributes: u32,
    pub text: Option<String>,
    pub font: Option<String>,
    pub justify: Option<Justify>,
    pub color: Option<[f32; 4]>,
    pub background_color: Option<[f32; 4]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LabelData {
    pub text: String,
    pub font: String,
    pub justify: Justify,
    pub color: [f32; 4],
    pub background_color: [f32; 4],
    pub multiline: bool,
    /// Native list order: each override is pushed to the front
    /// (`0x5E5BA0` links the new entry before the previous head).
    pub overrides: Vec<LabelOverride>,
    /// Horizontal text scroll in pixels (text object +0x74, `0x72F410`),
    /// set by the edit box (`0x5CF770`).
    #[serde(default)]
    pub scroll_px: f32,
}

/// Status bar (VE type 7, ctor `0x5E3DE0`, vtable 0xA50C48; LoadData
/// `0x5E61C0`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StatusBarData {
    pub texture: Option<String>,
    /// `mid_percentage` (+0xCC, ctor default 0.5).
    pub mid: f32,
    pub start_color: [f32; 3],
    pub mid_color: [f32; 3],
    pub end_color: [f32; 3],
    pub horizontal: bool,
    /// +0x124, -1 until set.
    pub value: f32,
}

impl StatusBarData {
    /// Clamp of `0x5E2FE0`.
    pub fn clamped(&self) -> f32 {
        if self.value < 0.0 || self.value.is_nan() {
            0.0
        } else if self.value > 1.0 {
            1.0
        } else {
            self.value
        }
    }

    /// Colour at the current value (`0x5E2FE0` with the LoadData deltas:
    /// +0x114 = (mid - start) / (mid * 100), +0x104 = (end - mid) /
    /// ((1 - mid) * 100)).
    pub fn color(&self) -> [f32; 4] {
        let v = self.clamped();
        let mut out = [0.0f32; 4];
        for k in 0..3 {
            out[k] = if v <= self.mid {
                let d = (self.mid_color[k] - self.start_color[k]) * (1.0 / (self.mid * 100.0));
                d * (v * 100.0) + self.start_color[k]
            } else {
                let d = (self.end_color[k] - self.mid_color[k]) * (1.0 / ((1.0 - self.mid) * 100.0));
                d * ((v - self.mid) * 100.0) + self.mid_color[k]
            };
        }
        // alpha: (1 - 1) deltas keep 1
        out[3] = 1.0;
        // mid 0 (or 1) divides by zero; the bar is then empty (or full at
        // the end colour) and the NaN colour invisible: keep it finite
        for c in &mut out {
            if !c.is_finite() {
                *c = 0.0;
            }
        }
        out
    }

    /// Geometry `0x5E2100`: horizontal bars grow from the left, vertical
    /// ones from the bottom.
    pub fn rect(&self, r: crate::layout::Rect) -> crate::layout::Rect {
        let v = self.clamped();
        let mut out = r;
        if self.horizontal {
            out.r = (r.r - r.l) * v + r.l;
        } else {
            out.t = r.b - (r.b - r.t) * v;
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NifEntry {
    pub model: String,
    pub camera: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum VeKind {
    Dummy,
    Group { validate_decal_children: bool, transparent: bool },
    Decal(DecalData),
    DecalTiled(TiledData),
    DecalTiledDirectional(DirectionalData),
    Video,
    Label(LabelData),
    Nif { menu_background: bool, nifs: Vec<NifEntry> },
    StatusBar(StatusBarData),
    /// Kinds whose rendering is outside this crate's scope for now (status
    /// bar, model, minimap, score graph); their node is kept for the caller.
    Unsupported(VeType),
}

/// A loaded view element. Children are kept in native list order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewElement {
    /// Data node name (`View`, `Child`, `Label`, `Decal`, ...).
    pub name: String,
    pub ve_type: VeType,
    pub kind: VeKind,
    pub layout: VeLayout,
    /// Authored bound strings (for diagnostics).
    pub bounds_text: [String; 4],
    pub when: u32,
    pub children: Vec<ViewElement>,
    /// Load diagnostics (the original's "Unable to parse bounds", "Unknown
    /// when value" etc. log lines).
    pub diagnostics: Vec<String>,
}

fn color4(node: &UiNode, key: &str, diags: &mut Vec<String>) -> [f32; 4] {
    match node.point3(key) {
        Ok([r, g, b]) => [r, g, b, 1.0],
        Err(e) => {
            diags.push(e.to_string());
            [1.0, 1.0, 1.0, 1.0]
        }
    }
}

fn flag(node: &UiNode, key: &str, diags: &mut Vec<String>) -> bool {
    match node.bool(key) {
        Ok(v) => v,
        Err(e) => {
            diags.push(e.to_string());
            false
        }
    }
}

fn texture_field(node: &UiNode, key: &str) -> Option<String> {
    let t = node.string(key).ok()?;
    let t = t.trim().trim_end_matches(';').trim();
    if t.is_empty() || t.eq_ignore_ascii_case("null") {
        None
    } else {
        Some(t.to_string())
    }
}

/// Load the base bounds/when part (`0x5E5780`).
fn load_base(node: &UiNode, diags: &mut Vec<String>) -> (VeLayout, [String; 4], u32) {
    let get = |k: &str| node.string(k).unwrap_or_default();
    let text = [get("L"), get("T"), get("R"), get("B")];
    // VE constructor 0x5E3BF0: flags 0x1004 (bit 12 = clip to parent)
    let mut layout = VeLayout { flags: 0x1004, ..Default::default() };
    if !parse_bounds(&mut layout.outer, &text[0], &text[1], &text[2], &text[3]) {
        diags.push(format!(
            "Unable to parse bounds for '{}': L={}, T={}, R={}, B={}",
            node.name, text[0], text[1], text[2], text[3]
        ));
    }
    let ctext = [get("client_L"), get("client_T"), get("client_R"), get("client_B")];
    if !parse_bounds(&mut layout.client, &ctext[0], &ctext[1], &ctext[2], &ctext[3]) {
        diags.push(format!(
            "Unable to parse client bounds for '{}': L={}, T={}, R={}, B={}",
            node.name, ctext[0], ctext[1], ctext[2], ctext[3]
        ));
    }
    let when_text = node.string("when").unwrap_or_default();
    let parsed = if when_text.eq_ignore_ascii_case("null") { Default::default() } else { parse_when(&when_text) };
    for u in &parsed.unknown {
        diags.push(format!("Unknown when value '{u}' for '{}'", node.name));
    }
    layout.flags |= crate::layout::ve_flag::LOADED;
    (layout, text, parsed.bits)
}

impl ViewElement {
    /// `0x5E4B40` + the class loader, without localization.
    pub fn load(node: &UiNode) -> Result<Self, String> {
        Self::load_localized(node, &|s: &str| s.to_string())
    }

    /// `0x5E4B40` + the class loader; `strings` resolves localized values
    /// (`#key` lookups, `0x5B5620` -> `0x4B9F40`).
    pub fn load_localized(node: &UiNode, strings: &dyn Fn(&str) -> String) -> Result<Self, String> {
        let type_name = node.string("type").map_err(|e| e.to_string())?;
        let ve_type = VeType::from_type_name(&type_name)
            .ok_or_else(|| format!("Unknown view element type: {type_name} ('{}')", node.name))?;
        let mut diags = Vec::new();
        let (layout, bounds_text, when) = load_base(node, &mut diags);
        let mut children = Vec::new();
        let kind = match ve_type {
            VeType::Dummy => VeKind::Dummy,
            VeType::Group => {
                let validate = node.bool_or("validate_decal_children", true).unwrap_or(true);
                for child in node.children_named("Child") {
                    match ViewElement::load_localized(child, strings) {
                        Ok(ve) => children.push(ve),
                        Err(e) => diags.push(e),
                    }
                }
                if validate {
                    for (i, a) in children.iter().enumerate() {
                        let decal = |t: VeType| matches!(t, VeType::Decal | VeType::DecalTiled | VeType::DecalTiledDirectional);
                        if decal(a.ve_type)
                            && children[..i].iter().any(|b| decal(b.ve_type) && b.when == a.when)
                        {
                            diags.push(format!(
                                "Child decal with duplicate when flag '{}' for widget '{}'",
                                a.when, node.name
                            ));
                        }
                    }
                }
                VeKind::Group { validate_decal_children: validate, transparent: node.bool_or("transparent", false).unwrap_or(false) }
            }
            VeType::Decal => {
                let color = color4(node, "color", &mut diags);
                let animation = node.child("Animated").map(|a| {
                    let prefix = a.string("texture_prefix").unwrap_or_default();
                    let file_type = a.string("file_type").unwrap_or_default();
                    let count = a.int("frame_count").unwrap_or(0);
                    Animation {
                        fps: a.float("fps").unwrap_or(0.0),
                        loops: a.bool("loops").unwrap_or(false),
                        frames: (1..=count.max(0)).map(|i| format!("{prefix}{i}.{file_type}")).collect(),
                    }
                });
                VeKind::Decal(DecalData {
                    color,
                    texture: if animation.is_none() { texture_field(node, "texture") } else { None },
                    animation,
                    tiled_x: flag(node, "tiled_x", &mut diags),
                    tiled_y: flag(node, "tiled_y", &mut diags),
                    alpha_testing: flag(node, "alpha_testing", &mut diags),
                    alpha_blending: flag(node, "alpha_blending", &mut diags),
                    stretch: flag(node, "stretch", &mut diags),
                    effect: 0,
                })
            }
            VeType::DecalTiled => {
                let color = color4(node, "color", &mut diags);
                let keys = [
                    "corners_texture",
                    "left_border_texture",
                    "right_border_texture",
                    "top_border_texture",
                    "bottom_border_texture",
                    "center_texture",
                ];
                let textures = keys.map(|k| {
                    let t = texture_field(node, k);
                    if t.is_none() {
                        diags.push(format!("Decal texture not specified for {} ({k})", node.name));
                    }
                    t
                });
                VeKind::DecalTiled(TiledData {
                    color,
                    alpha_testing: flag(node, "alpha_testing", &mut diags),
                    alpha_blending: flag(node, "alpha_blending", &mut diags),
                    textures,
                })
            }
            VeType::DecalTiledDirectional => {
                let color = color4(node, "color", &mut diags);
                let textures = ["ends_texture", "center_texture"].map(|k| {
                    let t = texture_field(node, k);
                    if t.is_none() {
                        diags.push(format!("Decal texture not specified for {} ({k})", node.name));
                    }
                    t
                });
                let dir = node.string("direction").unwrap_or_default();
                let horizontal = if dir.eq_ignore_ascii_case("horizontal") {
                    true
                } else {
                    if !dir.eq_ignore_ascii_case("vertical") {
                        diags.push(format!("Unknow direction '{dir}' for '{}'", node.name));
                    }
                    false
                };
                VeKind::DecalTiledDirectional(DirectionalData {
                    color,
                    alpha_testing: flag(node, "alpha_testing", &mut diags),
                    alpha_blending: flag(node, "alpha_blending", &mut diags),
                    textures,
                    horizontal,
                })
            }
            VeType::Label => VeKind::Label(load_label(node, &mut diags, strings)),
            VeType::Video => VeKind::Video,
            VeType::Nif => VeKind::Nif {
                menu_background: node.bool_or("menu_background_nifve", false).unwrap_or(false),
                nifs: node
                    .children_named("NIF")
                    .map(|n| NifEntry { model: n.string_or_empty("model"), camera: n.string_or_empty("camera") })
                    .collect(),
            },
            VeType::StatusBar => {
                let c3 = |k: &str, d: [f32; 3], diags: &mut Vec<String>| match node.point3(k) {
                    Ok(v) => v,
                    Err(e) => {
                        if node.string(k).is_ok() {
                            diags.push(e.to_string());
                        }
                        d
                    }
                };
                VeKind::StatusBar(StatusBarData {
                    texture: texture_field(node, "texture"),
                    mid: node.float("mid_percentage").unwrap_or(0.5),
                    start_color: c3("start_color", [0.0; 3], &mut diags),
                    mid_color: c3("mid_color", [0.0; 3], &mut diags),
                    end_color: c3("end_color", [0.0; 3], &mut diags),
                    horizontal: node.bool_or("horizontal", false).unwrap_or(false),
                    value: -1.0,
                })
            }
            other => VeKind::Unsupported(other),
        };
        let mut layout = layout;
        if let VeKind::Decal(d) = &kind {
            if d.tiled_x {
                layout.flags |= crate::layout::ve_flag::TILED_X;
            }
            if d.tiled_y {
                layout.flags |= crate::layout::ve_flag::TILED_Y;
            }
        }
        if let VeKind::Label(l) = &kind {
            if l.multiline {
                layout.flags |= crate::layout::ve_flag::MULTILINE;
            }
        }
        Ok(ViewElement { name: node.name.clone(), ve_type, kind, layout, bounds_text, when, children, diagnostics: diags })
    }

    /// Replace the outer bounds (used for widget root/view bound moves,
    /// `0x5E29B0`).
    pub fn set_outer(&mut self, b: Bounds) {
        self.layout.outer = b;
    }

    /// Walk every element depth-first in list order.
    pub fn walk<'a>(&'a self, f: &mut dyn FnMut(&'a ViewElement)) {
        f(self);
        for c in &self.children {
            c.walk(f);
        }
    }
}

fn load_label(node: &UiNode, diags: &mut Vec<String>, strings: &dyn Fn(&str) -> String) -> LabelData {
    let font = node.string("font").unwrap_or_default();
    let justify_text = node.string("justify").unwrap_or_default();
    let justify = Justify::parse(&justify_text).unwrap_or_else(|| {
        diags.push(format!("Unknown justify value '{justify_text}' for '{}'", node.name));
        Justify::Left
    });
    let mut overrides = Vec::new();
    for o in node.children_named("Override") {
        let when_text = o.string("when").unwrap_or_default();
        let when = if when_text.eq_ignore_ascii_case("null") { Default::default() } else { parse_when(&when_text) };
        for u in &when.unknown {
            diags.push(format!("Unknown when value '{u}' for '{}'", node.name));
        }
        let attr_text = o.string("attributes").unwrap_or_default();
        let attrs = if attr_text.eq_ignore_ascii_case("null") { Default::default() } else { parse_attributes(&attr_text) };
        for u in &attrs.unknown {
            diags.push(format!("Unknown attribute value '{u}' for '{}'", node.name));
        }
        let a = attrs.bits;
        let mut d = Vec::new();
        overrides.insert(
            0,
            LabelOverride {
                when: when.bits,
                attributes: a,
                text: (a & 1 != 0).then(|| strings(&o.string("text").unwrap_or_default())),
                font: (a & 2 != 0).then(|| o.string("font").unwrap_or_default()),
                justify: (a & 4 != 0).then(|| Justify::parse(&o.string("justify").unwrap_or_default()).unwrap_or_default()),
                color: (a & 8 != 0).then(|| color4(o, "color", &mut d)),
                background_color: (a & 0x10 != 0).then(|| color4(o, "background_color", &mut d)),
            },
        );
        diags.extend(d);
    }
    LabelData {
        text: strings(&node.string("text").unwrap_or_default()),
        font,
        justify,
        color: color4(node, "color", diags),
        background_color: color4(node, "background_color", diags),
        multiline: node.bool_or("multiline", false).unwrap_or(false),
        overrides,
        scroll_px: 0.0,
    }
}
