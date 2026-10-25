//! Paints aa-ui draw lists with Bevy: one 2D overlay camera, the list's
//! commands in their original paint order as textured quad meshes.
//!
//! Draw-list conventions (aa-ui `draw.rs`): window pixels, y down, pixel
//! centres at i + 0.5; the list is already in the original's paint order
//! (tree order for menus and dialogs, opaque-then-blended for the loading
//! screen), so painter's order reproduces the original's depth layering.
//!
//! The overlay camera composites in sRGB space (`CompositingSpace::Srgb`):
//! like the original Direct3D 9 renderer, blending happens on
//! gamma-encoded values, so translucent panels and antialiased glyph edges
//! keep the original's weight. The overlay is written over the battle view
//! with premultiplied alpha.
//!
//! Consecutive commands that sample the same image form one mesh; each
//! mesh is drawn at its own depth (painter's order). Clip rectangles are
//! applied on the CPU (quads are axis-aligned: the UVs are cut with them).
//!
//! A quad in texture effect mode 1 (a disabled button icon, `0x5E66E0`) is
//! drawn with `GreyUiMaterial` (`ui_grey.wgsl`, the Direct3D stages of
//! `0x742CC0`). Effect modes 2 and 3 (the texture's +0x101 flag: the
//! specular colour blended in by texture alpha) are not ported.
use super::glyphs::{GlyphAtlas, PAGE};
use super::tex::{Key, UiTextures, Variant};
use super::video::VideoPlayer;
use aa_ui::draw::{Address, Blend, DrawKind, DrawList};
use aa_ui::layout::Rect;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{CameraOutputMode, ClearColorConfig, CompositingSpace, ScalingMode};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::BlendState;
use std::collections::HashMap;
use std::path::PathBuf;

/// Render layer of the UI meshes (only the overlay camera draws it).
pub const UI_LAYER: usize = 7;

#[derive(Component)]
pub struct UiCamera;

#[derive(Component)]
pub struct UiBatch;

/// What the front end wants painted this frame.
#[derive(Resource, Default)]
pub struct UiDraw {
    pub list: DrawList,
    /// The minimap content (drawn at the GameMiniMap view element).
    pub minimap: Option<super::minimap::MinimapDraw>,
    pub minimap_image: Option<Handle<Image>>,
    /// The object display's portrait render and banner image.
    pub portrait: Option<Handle<Image>>,
    pub banner: Option<Handle<Image>>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum MatKey {
    Plain,
    Image(AssetId<Image>),
    /// Effect mode 1 (a disabled icon, `ui_grey.wgsl`); alpha-blended.
    Grey(AssetId<Image>, bool),
}

/// The material of a batch: the plain colour material, or the grey effect.
#[derive(Clone)]
enum UiMat {
    Color(Handle<ColorMaterial>),
    Grey(Handle<GreyUiMaterial>),
}

pub const GREY_SHADER: Handle<bevy::shader::Shader> = bevy::asset::uuid_handle!("5f0c3b8a-2d71-4e96-8a4b-c71e09d36f52");

/// A UI texture in effect mode 1 (`0x73EAF0`): the fixed-function stages of
/// `0x742CC0` / `0x742630` as a shader (`ui_grey.wgsl`).
#[derive(Asset, TypePath, bevy::render::render_resource::AsBindGroup, Debug, Clone)]
pub struct GreyUiMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
    /// x: 1 when the texture is alpha-blended (UI texture +0x80 == 2).
    #[uniform(2)]
    pub params: UVec4,
}

impl bevy::sprite_render::Material2d for GreyUiMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        bevy::shader::ShaderRef::Handle(GREY_SHADER)
    }
    fn alpha_mode(&self) -> bevy::sprite_render::AlphaMode2d {
        bevy::sprite_render::AlphaMode2d::Blend
    }
}

/// Register the grey effect material and its shader.
pub fn add_grey(app: &mut App) {
    app.add_plugins(bevy::sprite_render::Material2dPlugin::<GreyUiMaterial>::default());
    let shader = bevy::shader::Shader::from_wgsl(include_str!("ui_grey.wgsl"), "aa_bevy/ui/ui_grey.wgsl");
    let _ = app.world_mut().resource_mut::<Assets<bevy::shader::Shader>>().insert(GREY_SHADER.id(), shader);
}

struct Batch {
    key: MatKey,
    image: Option<Handle<Image>>,
    pos: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    col: Vec<[f32; 4]>,
    idx: Vec<u32>,
}

#[derive(Resource)]
pub struct UiRenderer {
    pub tex: UiTextures,
    pub glyphs: GlyphAtlas,
    videos: HashMap<String, VideoPlayer>,
    video_root: PathBuf,
    seed: usize,
    last: Option<(DrawList, Vec<Option<AssetId<Image>>>, [u32; 2], Option<super::minimap::MinimapDraw>, [Option<AssetId<Image>>; 2])>,
    pool: Vec<(Entity, Handle<Mesh>)>,
    materials: HashMap<MatKey, UiMat>,
}

impl UiRenderer {
    pub fn new(reconstruction: &std::path::Path, video_root: PathBuf, data: aa_ui::fs::DataRoot) -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as usize)
            .unwrap_or(0);
        Self {
            tex: UiTextures::new(reconstruction),
            glyphs: GlyphAtlas::new(data),
            videos: HashMap::new(),
            video_root,
            seed,
            last: None,
            pool: Vec::new(),
            materials: HashMap::new(),
        }
    }

    /// The baked frame folder of a cinematic resource list.
    fn video(&mut self, resources: &[String]) -> &mut VideoPlayer {
        let globe = resources.iter().any(|s| s.to_ascii_lowercase().contains("aaglobe"));
        let folder = if globe { "MainMenuGlobe" } else { "MainMenu" };
        let root = self.video_root.join(folder);
        let seed = self.seed;
        self.videos.entry(folder.to_string()).or_insert_with(|| VideoPlayer::new(&root, seed))
    }
}

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 100,
            // the overlay's own texture starts transparent every frame
            clear_color: ClearColorConfig::Custom(Color::NONE),
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                clear_color: ClearColorConfig::Custom(Color::BLACK),
            },
            ..default()
        },
        Msaa::Off,
        CompositingSpace::Srgb,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        RenderLayers::layer(UI_LAYER),
        UiCamera,
    ));
}

fn screen_size(window: &Window) -> [u32; 2] {
    [window.physical_width().max(1), window.physical_height().max(1)]
}

/// Fit the overlay projection to the window's physical pixels and clear the
/// window to black only while no other camera draws (menus, loading).
pub fn update_camera(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut cameras: Query<(&mut Camera, &mut Projection), With<UiCamera>>,
    others: Query<&Camera, Without<UiCamera>>,
) {
    let Ok(window) = windows.single() else { return };
    let [w, h] = screen_size(window);
    let alone = !others.iter().any(|c| c.is_active);
    for (mut cam, mut proj) in &mut cameras {
        let want = CameraOutputMode::Write {
            blend_state: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            clear_color: if alone { ClearColorConfig::Custom(Color::BLACK) } else { ClearColorConfig::None },
        };
        let differs = match (&cam.output_mode, &want) {
            (CameraOutputMode::Write { clear_color: a, .. }, CameraOutputMode::Write { clear_color: b, .. }) => {
                std::mem::discriminant(a) != std::mem::discriminant(b)
            }
            _ => true,
        };
        if differs {
            cam.output_mode = want;
        }
        if let Projection::Orthographic(o) = proj.as_mut() {
            let mode = ScalingMode::Fixed { width: w as f32, height: h as f32 };
            if !matches!(o.scaling_mode, ScalingMode::Fixed { width, height } if width == w as f32 && height == h as f32) {
                o.scaling_mode = mode;
            }
        }
    }
}

fn srgb_to_linear(c: [f32; 4]) -> [f32; 4] {
    let l = Color::srgba(c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0), c[3].clamp(0.0, 1.0)).to_linear();
    [l.red, l.green, l.blue, l.alpha]
}

/// Cut a quad by a clip rectangle, cutting its UVs in proportion.
fn clip_quad(r: Rect, uv: [f32; 4], clip: Option<Rect>) -> Option<(Rect, [f32; 4])> {
    let (mut l, mut t, mut rr, mut b) = (r.l, r.t, r.r, r.b);
    let mut uv = uv;
    if let Some(c) = clip {
        let (w, h) = (r.r - r.l, r.b - r.t);
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        let du = (uv[2] - uv[0]) / w;
        let dv = (uv[3] - uv[1]) / h;
        if c.l > l {
            uv[0] += (c.l - l) * du;
            l = c.l;
        }
        if c.r < rr {
            uv[2] -= (rr - c.r) * du;
            rr = c.r;
        }
        if c.t > t {
            uv[1] += (c.t - t) * dv;
            t = c.t;
        }
        if c.b < b {
            uv[3] -= (b - c.b) * dv;
            b = c.b;
        }
    }
    (rr > l && b > t).then_some((Rect { l, t, r: rr, b }, uv))
}

struct Builder {
    batches: Vec<Batch>,
    half: [f32; 2],
}

impl Builder {
    /// An arbitrary quad (corners in order), no clipping.
    fn poly(&mut self, image: Option<Handle<Image>>, pts: [(f32, f32); 4], uvs: [[f32; 2]; 4], color: [f32; 4]) {
        let key = match &image {
            Some(h) => MatKey::Image(h.id()),
            None => MatKey::Plain,
        };
        if self.batches.last().is_none_or(|b| b.key != key) {
            self.batches.push(Batch { key, image, pos: Vec::new(), uv: Vec::new(), col: Vec::new(), idx: Vec::new() });
        }
        let b = self.batches.last_mut().unwrap();
        let base = b.pos.len() as u32;
        let (hx, hy) = (self.half[0], self.half[1]);
        for p in pts {
            b.pos.push([p.0 - hx, hy - p.1, 0.0]);
        }
        b.uv.extend(uvs);
        let c = srgb_to_linear(color);
        b.col.extend([c; 4]);
        b.idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    fn quad(&mut self, image: Option<Handle<Image>>, rect: Rect, uv: [f32; 4], color: [f32; 4], clip: Option<Rect>) {
        self.quad_fx(image, rect, uv, color, clip, None);
    }
    /// A quad; `grey` = Some(blended) draws it in effect mode 1.
    fn quad_fx(&mut self, image: Option<Handle<Image>>, rect: Rect, uv: [f32; 4], color: [f32; 4], clip: Option<Rect>, grey: Option<bool>) {
        let Some((r, uv)) = clip_quad(rect, uv, clip) else { return };
        let key = match (&image, grey) {
            (Some(h), Some(blended)) => MatKey::Grey(h.id(), blended),
            (Some(h), None) => MatKey::Image(h.id()),
            (None, _) => MatKey::Plain,
        };
        if self.batches.last().is_none_or(|b| b.key != key) {
            self.batches.push(Batch { key, image, pos: Vec::new(), uv: Vec::new(), col: Vec::new(), idx: Vec::new() });
        }
        let b = self.batches.last_mut().unwrap();
        let base = b.pos.len() as u32;
        let (hx, hy) = (self.half[0], self.half[1]);
        let x = |v: f32| v - hx;
        let y = |v: f32| hy - v;
        b.pos.extend([[x(r.l), y(r.t), 0.0], [x(r.r), y(r.t), 0.0], [x(r.r), y(r.b), 0.0], [x(r.l), y(r.b), 0.0]]);
        b.uv.extend([[uv[0], uv[1]], [uv[2], uv[1]], [uv[2], uv[3]], [uv[0], uv[3]]]);
        let c = srgb_to_linear(color);
        b.col.extend([c; 4]);
        b.idx.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }
}

fn variant(blend: Blend, address: [Address; 2]) -> Variant {
    Variant { alpha_test: blend != Blend::Alpha, wrap_u: address[0] == Address::Wrap, wrap_v: address[1] == Address::Wrap }
}

/// Build the meshes for this frame's draw list (only when it changed).
#[allow(clippy::too_many_arguments)]
pub fn paint(
    mut commands: Commands,
    draw: Res<UiDraw>,
    mut renderer: ResMut<UiRenderer>,
    time: Res<Time>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut grey_materials: ResMut<Assets<GreyUiMaterial>>,
    mut visibility: Query<&mut Visibility, With<UiBatch>>,
) {
    let Ok(window) = windows.single() else { return };
    let size = screen_size(window);
    let list = &draw.list;
    let r = &mut *renderer;
    // cinematics advance every frame; their image handles change only once
    let now = time.elapsed_secs_f64();
    let mut video_frames: Vec<Option<Handle<Image>>> = Vec::new();
    for cmd in &list.commands {
        if let DrawKind::Video(v) = &cmd.kind {
            video_frames.push(r.video(&v.resources).frame(now, &mut images));
        }
    }
    let video_ids: Vec<Option<AssetId<Image>>> = video_frames.iter().map(|h| h.as_ref().map(|h| h.id())).collect();
    let extra = [draw.portrait.as_ref().map(|h| h.id()), draw.banner.as_ref().map(|h| h.id())];
    if r.last.as_ref().is_some_and(|(l, v, s, m, e)| l == list && *v == video_ids && *s == size && *m == draw.minimap && *e == extra) {
        return;
    }
    // textures first (decoded in parallel)
    let skin = list.skin.clone();
    let mut keys = Vec::new();
    for cmd in &list.commands {
        match &cmd.kind {
            DrawKind::Quad(q) => {
                if let Some(t) = &q.texture {
                    keys.push(Key { path: t.clone(), skin: skin.clone(), variant: variant(q.blend, q.address) });
                }
            }
            DrawKind::Text(t) => {
                for g in &t.glyphs {
                    if let Some(img) = &g.image {
                        keys.push(Key { path: img.clone(), skin: skin.clone(), variant: Variant::default() });
                    }
                }
            }
            _ => {}
        }
    }
    r.tex.prepare(keys, &mut images);
    let mut b = Builder { batches: Vec::new(), half: [size[0] as f32 * 0.5, size[1] as f32 * 0.5] };
    let mut video_i = 0;
    for cmd in &list.commands {
        match &cmd.kind {
            DrawKind::Quad(q) => {
                let opaque = q.blend != Blend::Alpha;
                let mut color = q.color;
                if opaque {
                    // ALPHATEST > 0 with blending off: drawn opaque
                    if color[3] <= 0.0 {
                        continue;
                    }
                    color[3] = 1.0;
                }
                match &q.texture {
                    Some(t) => {
                        let key = Key { path: t.clone(), skin: skin.clone(), variant: variant(q.blend, q.address) };
                        let Some(e) = r.tex.get(&key) else { continue };
                        let image = e.image.clone();
                        let grey = (q.effect == 1).then_some(q.blend == Blend::Alpha);
                        b.quad_fx(Some(image), q.rect, q.uv, color, q.clip, grey);
                    }
                    None => b.quad(None, q.rect, [0.0, 0.0, 1.0, 1.0], color, q.clip),
                }
            }
            DrawKind::Text(t) => {
                for g in t.glyphs.iter().filter(|g| g.highlight) {
                    // <h_on> background list as a solid fill (INFERRED in aa-ui)
                    b.quad(None, g.rect, [0.0, 0.0, 1.0, 1.0], g.color, t.clip);
                }
                for g in t.glyphs.iter().filter(|g| !g.highlight) {
                    let (gw, gh) = (g.rect.r - g.rect.l, g.rect.b - g.rect.t);
                    if let Some(img) = &g.image {
                        let key = Key { path: img.clone(), skin: skin.clone(), variant: Variant::default() };
                        let Some(e) = r.tex.get(&key) else { continue };
                        let (iw, ih) = (e.size[0].max(1) as f32, e.size[1].max(1) as f32);
                        let uv = [g.cell[0] / iw, g.cell[1] / ih, (g.cell[0] + gw) / iw, (g.cell[1] + gh) / ih];
                        let image = e.image.clone();
                        b.quad(Some(image), g.rect, uv, g.color, t.clip);
                        continue;
                    }
                    let Some(face) = t.faces.get(g.face).and_then(|f| f.face.as_ref()) else { continue };
                    if char::from_u32(g.c).is_none_or(|c| c.is_whitespace()) {
                        continue;
                    }
                    let Some(cell) = r.glyphs.cell(face, g.c, &mut images) else { continue };
                    let Some(page) = r.glyphs.page_image(cell.page) else { continue };
                    let p = PAGE as f32;
                    let (u0, v0) = ((cell.x as f32 + g.cell[0]) / p, (cell.y as f32 + g.cell[1]) / p);
                    let uv = [u0, v0, u0 + gw / p, v0 + gh / p];
                    b.quad(Some(page), g.rect, uv, g.color, t.clip);
                }
            }
            DrawKind::Video(v) => {
                let frame = video_frames.get(video_i).cloned().flatten();
                video_i += 1;
                match frame {
                    Some(h) => b.quad(Some(h), v.rect, [0.0, 0.0, 1.0, 1.0], [1.0; 4], None),
                    // no baked frames (yet): the frame stays black
                    None => b.quad(None, v.rect, [0.0, 0.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0], None),
                }
            }
            // 3D model view elements are drawn by the battle renderer
            DrawKind::External { ve_type, .. } if ve_type == "MiniMap" => {
                if let Some(m) = &draw.minimap {
                    let [p0, p1, p2] = m.quad;
                    // point(u, v) = p0 + u (p2 - p1) + v (p1 - p0); image row 0
                    // is the map's top (v = 1). The image holds every layer,
                    // the objects included.
                    let pt = |u: f32, v: f32| (p0.0 + u * (p2.0 - p1.0) + v * (p1.0 - p0.0), p0.1 + u * (p2.1 - p1.1) + v * (p1.1 - p0.1));
                    if let (Some(img), true) = (&draw.minimap_image, m.has_terrain) {
                        b.poly(Some(img.clone()), [pt(0.0, 0.0), pt(1.0, 0.0), pt(1.0, 1.0), pt(0.0, 1.0)], [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]], [1.0; 4]);
                    }
                    for (a, e, c) in &m.lines {
                        // a one pixel line as a thin quad
                        let (dx, dy) = (e.0 - a.0, e.1 - a.1);
                        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                        let (nx, ny) = (-dy / len * 0.5, dx / len * 0.5);
                        b.poly(None, [(a.0 + nx, a.1 + ny), (e.0 + nx, e.1 + ny), (e.0 - nx, e.1 - ny), (a.0 - nx, a.1 - ny)], [[0.0, 0.0]; 4], *c);
                    }
                }
            }
            // the object display's portrait render and banner image
            DrawKind::External { rect, .. } if cmd.widget.ends_with("PortraitButton/Portrait") => {
                if let Some(img) = &draw.portrait {
                    b.quad(Some(img.clone()), *rect, [0.0, 0.0, 1.0, 1.0], [1.0; 4], None);
                }
            }
            DrawKind::External { rect, .. } if cmd.widget.ends_with("BannerButton/Banner") => {
                if let Some(img) = &draw.banner {
                    // the banner's square, centred in the view
                    let (w, h) = (rect.r - rect.l, rect.b - rect.t);
                    let s = w.min(h);
                    let (l, t) = (rect.l + (w - s) * 0.5, rect.t + (h - s) * 0.5);
                    b.quad(Some(img.clone()), Rect { l, t, r: l + s, b: t + s }, [0.0, 0.0, 1.0, 1.0], [1.0; 4], None);
                }
            }
            DrawKind::External { .. } => {}
        }
    }
    r.glyphs.flush(&mut images);
    // write the batches into pooled entities
    let used = b.batches.len();
    for (i, batch) in b.batches.into_iter().enumerate() {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, batch.pos);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, batch.uv);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, batch.col);
        mesh.insert_indices(Indices::U32(batch.idx));
        let material = r
            .materials
            .entry(batch.key.clone())
            .or_insert_with(|| match (&batch.key, &batch.image) {
                (MatKey::Grey(_, blended), Some(image)) => UiMat::Grey(grey_materials.add(GreyUiMaterial { texture: image.clone(), params: UVec4::new(u32::from(*blended), 0, 0, 0) })),
                _ => UiMat::Color(materials.add(ColorMaterial { color: Color::WHITE, texture: batch.image.clone(), alpha_mode: bevy::sprite_render::AlphaMode2d::Blend, ..default() })),
            })
            .clone();
        let z = i as f32 * 0.001;
        let entity = if let Some((entity, handle)) = r.pool.get(i).cloned() {
            let _ = meshes.insert(handle.id(), mesh);
            commands.entity(entity).insert((Transform::from_xyz(0.0, 0.0, z), Visibility::Visible));
            entity
        } else {
            let handle = meshes.add(mesh);
            let entity = commands
                .spawn((
                    Mesh2d(handle.clone()),
                    Transform::from_xyz(0.0, 0.0, z),
                    Visibility::Visible,
                    RenderLayers::layer(UI_LAYER),
                    bevy::camera::visibility::NoFrustumCulling,
                    UiBatch,
                ))
                .id();
            r.pool.push((entity, handle));
            entity
        };
        // a pooled entity keeps one material component of the right kind
        match material {
            UiMat::Color(m) => {
                commands.entity(entity).remove::<MeshMaterial2d<GreyUiMaterial>>().insert(MeshMaterial2d(m));
            }
            UiMat::Grey(m) => {
                commands.entity(entity).remove::<MeshMaterial2d<ColorMaterial>>().insert(MeshMaterial2d(m));
            }
        }
    }
    r.last = Some((list.clone(), video_ids, size, draw.minimap.clone(), extra));
    // hide the pool entries this frame did not use
    for (entity, _) in r.pool.iter().skip(used) {
        if let Ok(mut v) = visibility.get_mut(*entity) {
            *v = Visibility::Hidden;
        }
    }
}
