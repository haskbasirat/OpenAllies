//! Renderer-neutral draw list produced from laid-out widgets.
//!
//! Commands are in the original's draw order (widget root view elements are
//! drawn parent first, children in their sorted list order). Coordinates are
//! window pixels, y down; UVs are normalised texture coordinates with v down
//! (row 0 of the upright image is v = 0).

use crate::layout::Rect;
use crate::ve::Justify;
use serde::{Deserialize, Serialize};

/// Blend state of a quad (`alpha_blending` / `alpha_testing`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Blend {
    /// Neither flag: the texture is drawn opaque (alpha ignored).
    #[default]
    Opaque,
    /// `alpha_blending = true`: source-over blending.
    Alpha,
    /// `alpha_testing = true` without blending: texels with alpha below the
    /// test reference are discarded.
    AlphaTest,
}

/// Texture addressing along one axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Address {
    #[default]
    Clamp,
    Wrap,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quad {
    /// Data-relative texture path after `%s` resolution, original spelling
    /// where the file exists (`/UI/Default/Menus/MainMenu/MainMenuTop.TGA`).
    /// `None` draws the vertex colour only.
    pub texture: Option<String>,
    pub rect: Rect,
    /// u0, v0, u1, v1.
    pub uv: [f32; 4],
    /// Vertex colour RGBA.
    pub color: [f32; 4],
    pub blend: Blend,
    pub address: [Address; 2],
    /// Scissor rectangle (window pixels) or none.
    pub clip: Option<Rect>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Text {
    pub text: String,
    /// Font IDS (`large`, `medium`, `small`, `tiny`, ...).
    pub font: String,
    /// Face and pixel size chosen for the current screen height
    /// (`KKC_RFont::SetupFont` 0x708B30).
    pub face: Option<crate::text::FontFace>,
    /// Faces of the fonts the glyphs use (markup can switch fonts).
    pub faces: Vec<TextFace>,
    /// Glyph quads of the native rebuild (`rdraw.rs`) in draw order: each
    /// is clipped to the label rect and floored to pixels by AddQuad.
    pub glyphs: Vec<GlyphQuad>,
    /// Label rect (clipped) in window pixels.
    pub rect: Rect,
    pub justify: Justify,
    pub multiline: bool,
    pub color: [f32; 4],
    pub clip: Option<Rect>,
}

/// A font used by a label.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextFace {
    /// Font IDS.
    pub ids: String,
    pub face: Option<crate::text::FontFace>,
}

/// One glyph quad (`0x72FA00`). The glyph cell's top-left is
/// `(rect.l - cell[0], rect.t - cell[1])`; the rendered cell holds the
/// glyph with its baked one-pixel shadow / outline, and the quad shows only
/// `rect` of it (the advance wide, so the shadow column is cut).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GlyphQuad {
    /// Glyph index (UTF-16 unit after stand-in / missing-glyph substitution).
    pub c: u32,
    /// Index into [`Text::faces`].
    pub face: usize,
    pub rect: Rect,
    /// Pixels clipped from the cell at the left and top.
    pub cell: [f32; 2],
    pub color: [f32; 4],
    /// `<h_on>` quad of the batch's second list (+0x10), drawn in the
    /// label's `background_color` (RText+0x58). Rendering it as a solid
    /// background behind the glyphs is INFERRED (renderer not traced).
    pub highlight: bool,
    /// Special glyph image (data path) replacing the font cell.
    pub image: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Video {
    /// Data-relative video resources in authored order.
    pub resources: Vec<String>,
    pub repeating: bool,
    pub rect: Rect,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DrawKind {
    Quad(Quad),
    Text(Text),
    Video(Video),
    /// A 3D model / NIF / minimap view element: the caller renders it into
    /// `rect` (not drawn by this crate).
    External {
        ve_type: String,
        rect: Rect,
        /// Model id a ModelWidget shows (set by game code through the HUD
        /// world), if any.
        #[serde(default)]
        model: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawCmd {
    /// Widget path (`MainMenu/MainMenuButtonSinglePlayer`).
    pub widget: String,
    /// View element path inside the widget (`View/Child[2]`).
    pub ve: String,
    #[serde(flatten)]
    pub kind: DrawKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct DrawList {
    pub screen: [i32; 2],
    /// Nation skin whose `Skins/<NATION>` files take precedence for texture
    /// paths (in-game screens), if any.
    pub skin: Option<String>,
    pub commands: Vec<DrawCmd>,
}
