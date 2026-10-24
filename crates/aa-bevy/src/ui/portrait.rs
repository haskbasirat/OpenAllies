//! The object display's 3D views (the CompanyDisplay `PortraitButton/
//! Portrait` and `BannerButton/Banner` model widgets, `0x624270` /
//! `0x624DA0`):
//! * the portrait: the model the HUD names (a leader's `<thing>_portrait`
//!   Thing, e.g. Units/GER/GERInfantryCo/GERInfantryCOPortrait.nif, else the
//!   selected object's own model) rendered into an image the UI draws in
//!   the widget's rectangle. A portrait NIF's own NiCamera (its transform
//!   and frustum) frames it; any other model is framed as the ModelVE
//!   fit_model does it (`0x5EA730`): from the VE position 1,-1,1 (front
//!   right, above), its bounds filling the view;
//! * the banner: the selected organization's composed banner (the same
//!   layers the world banner shows) drawn as an image.
//!
//! Not done: the portraits' idle / fidget / talk animations (the KF files
//! beside the portrait NIF); the portrait is drawn in its bind pose.
use super::render::UiDraw;
use crate::look::OriginalMaterial;
use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use std::collections::HashMap;
use std::sync::Arc;

/// Render layer of the portrait models (only the portrait camera draws it).
pub const PORTRAIT_LAYER: usize = 5;

/// A NiCamera of a portrait NIF, in render space.
#[derive(Clone, Copy, Debug)]
struct NifCamera {
    eye: Vec3,
    dir: Vec3,
    up: Vec3,
    /// Vertical field of view (rad) from the frustum top / bottom.
    fov: f32,
    near: f32,
    far: f32,
}

struct PortraitModel {
    parts: Vec<(Handle<Mesh>, Handle<OriginalMaterial>)>,
    camera: Option<NifCamera>,
    min: Vec3,
    max: Vec3,
}

#[derive(Resource, Default)]
pub struct Portraits {
    store: Option<Option<aa_assets::AssetStore>>,
    models: HashMap<String, Option<Arc<PortraitModel>>>,
    shown: Option<(String, UVec2)>,
    entities: Vec<Entity>,
    image: Option<Handle<Image>>,
}

/// Native (x, y, z up) -> render (x, z, -y).
fn to_render(p: Vec3) -> Vec3 {
    Vec3::new(p.x, p.z, -p.y)
}

fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// The first NiCamera of a NIF with its world transform (the parent chain
/// composed: p' = R s p + t).
fn nif_camera(bytes: &[u8]) -> Option<NifCamera> {
    let file = aa_formats::nif::load(bytes).ok()?;
    let ci = file.objects.iter().position(|o| o.is("NiCamera"))?;
    let local = |i: usize| -> Option<(Mat3, f32, Vec3)> {
        let o = &file.objects[i];
        let t = o.fields.f32s("translation")?;
        let r = o.fields.f32s("rotation")?;
        let s = o.fields.f32("scale").map(f).unwrap_or(1.0);
        // NiMatrix3 rows in stream order: column k = M * e_k
        let m = Mat3::from_cols(Vec3::new(f(r[0]), f(r[3]), f(r[6])), Vec3::new(f(r[1]), f(r[4]), f(r[7])), Vec3::new(f(r[2]), f(r[5]), f(r[8])));
        Some((m, s, Vec3::new(f(t[0]), f(t[1]), f(t[2]))))
    };
    let mut chain = vec![ci];
    let mut at = ci;
    while let Some(Some(p)) = file.parents.get(at) {
        chain.push(*p);
        at = *p;
        if chain.len() > 64 {
            break;
        }
    }
    // world = parent ... * local
    let (mut rot, mut scale, mut pos) = (Mat3::IDENTITY, 1.0f32, Vec3::ZERO);
    for &i in chain.iter().rev() {
        let (m, s, t) = local(i)?;
        pos += rot * (t * scale);
        rot *= m;
        scale *= s;
    }
    let fr = file.objects[ci].fields.f32s("frustum")?;
    let (top, bottom, near, far) = (f(fr[2]), f(fr[3]), f(fr[4]), f(fr[5]));
    // Gamebryo cameras look down their local +X with +Y up
    let dir = to_render(rot * Vec3::X).normalize_or_zero();
    let up = to_render(rot * Vec3::Y).normalize_or_zero();
    let fov = (top.atan() - bottom.atan()).abs().max(0.05);
    Some(NifCamera { eye: to_render(pos), dir, up, fov, near: near.max(0.01), far: far.max(near + 1.0) })
}

impl Portraits {
    fn load(&mut self, path: &str, data: &aa_ui::fs::DataRoot, meshes: &mut Assets<Mesh>, materials: &mut Assets<OriginalMaterial>, images: &mut Assets<Image>) -> Option<Arc<PortraitModel>> {
        if let Some(m) = self.models.get(path) {
            return m.clone();
        }
        let store = self.store.get_or_insert_with(|| super::reconstruction_root().and_then(|r| aa_assets::AssetStore::open(r).ok())).clone();
        let loaded = (|| {
            let store = store?;
            let model = store.load_model(path).ok()?.prepare().ok()?;
            let mut parts = Vec::new();
            let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for mesh in model.meshes.iter().filter(|m| !m.indices.is_empty()) {
                for v in &mesh.vertices {
                    let p = Vec3::from(v.position);
                    min = min.min(p);
                    max = max.max(p);
                }
                let texture = mesh.material.texture.as_ref().and_then(|t| crate::textures::load(&store, t, crate::scene::kind(&mesh.material, crate::scene::NEUTRAL_TEAM)).ok()).map(|i| images.add(i));
                let mut mat = crate::scene::standard(&mesh.material, texture);
                // the HUD's own lighting, no fog
                mat.extension.params.flags.w = 1.0;
                mat.extension.params.sun.w = 1.0;
                parts.push((meshes.add(crate::scene::model_mesh(mesh)), materials.add(mat)));
            }
            if parts.is_empty() {
                return None;
            }
            // a portrait model's own NiCamera frames it; other models (a
            // truck: USATruck.nif carries an unrelated camera) are framed
            // from their bounds
            let portrait = path.to_ascii_lowercase().contains("portrait");
            let camera = data.find(path).filter(|_| portrait).and_then(|p| std::fs::read(p).ok()).and_then(|b| nif_camera(&b));
            debug!("portrait {path}: {} parts, bounds {min} .. {max}, camera {camera:?}", parts.len());
            Some(Arc::new(PortraitModel { parts, camera, min, max }))
        })();
        if loaded.is_none() {
            warn!("portrait model {path} did not load");
        }
        self.models.insert(path.to_string(), loaded.clone());
        loaded
    }

    fn clear(&mut self, commands: &mut Commands) {
        for e in self.entities.drain(..) {
            commands.entity(e).despawn();
        }
        self.shown = None;
    }
}

/// The portrait and banner views of this frame's draw list.
#[allow(clippy::too_many_arguments)]
pub fn update(
    mut commands: Commands,
    mut p: ResMut<Portraits>,
    mut draw: ResMut<UiDraw>,
    front: Res<super::front::Front>,
    banners: Res<super::banners::BannerArt>,
    std_materials: Res<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OriginalMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // the banner preview: the selected organization's composed banner
    draw.banner = banners.selected.as_ref().and_then(|m| std_materials.get(m)).and_then(|m| m.base_color_texture.clone());
    let found = draw.list.commands.iter().find_map(|c| match &c.kind {
        aa_ui::draw::DrawKind::External { rect, model: Some(model), .. } if c.widget.ends_with("PortraitButton/Portrait") => Some((*rect, model.clone())),
        _ => None,
    });
    let Some((rect, model)) = found.filter(|_| front.page == super::front::Page::Game) else {
        if p.shown.is_some() {
            p.clear(&mut commands);
        }
        draw.portrait = None;
        return;
    };
    let size = UVec2::new(((rect.r - rect.l).round() as u32).max(1), ((rect.b - rect.t).round() as u32).max(1));
    let key = (model.clone(), size);
    if p.shown.as_ref() != Some(&key) {
        p.clear(&mut commands);
        let data = front.ui.store.root.clone();
        let Some(m) = p.load(&model, &data, &mut meshes, &mut materials, &mut images) else {
            draw.portrait = None;
            p.shown = Some(key);
            return;
        };
        // the render target, recreated for a new size
        let image = match p.image.clone().filter(|h| images.get(h).is_some_and(|i| i.width() == size.x && i.height() == size.y)) {
            Some(h) => h,
            None => {
                let h = images.add(Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None));
                p.image = Some(h.clone());
                h
            }
        };
        let layer = RenderLayers::layer(PORTRAIT_LAYER);
        let aspect = size.x as f32 / size.y as f32;
        let (eye, look, up, fov, near, far) = match m.camera {
            Some(c) => (c.eye, c.eye + c.dir, if c.up.length_squared() > 0.5 { c.up } else { Vec3::Y }, c.fov, c.near, c.far),
            None => {
                // ModelVE fit_model (0x5EA730): the camera on the VE
                // `position` direction (template_ve.tgi 1,-1,1: native +x,
                // -y in front of the model, above; render (1,1,1)), looking
                // at the model origin raised by half the model height. The
                // distance puts the bounds' top corner (horizontal radius R,
                // half height h) on the frame edge: sqrt(R^2 + h^2)
                // sin(pi - atan(h/R) - a - e) / sin(a), with a the half view
                // angle and e the elevation of the position.
                let h = ((m.max.y - m.min.y) * 0.5).max(0.05);
                let r = m.min.x.abs().max(m.max.x.abs()).max(m.min.z.abs()).max(m.max.z.abs()).max(0.05);
                let fov = 30f32.to_radians();
                let half = fov * 0.5;
                let pos = Vec3::new(1.0, -1.0, 1.0);
                let elevation = std::f32::consts::FRAC_PI_2 - (pos.x.hypot(pos.y) / pos.z).atan();
                let a = std::f32::consts::PI - (h / r).atan() - half - elevation;
                let dist = (r * r + h * h).sqrt() * a.sin() / half.sin();
                let target = Vec3::new(0.0, h, 0.0);
                let dir = to_render(pos).normalize();
                let radius = (r * r + h * h).sqrt();
                (target + dir * dist, target, Vec3::Y, fov, (dist - radius * 2.0).max(0.05), dist + radius * 2.0)
            }
        };
        // models face the camera's light: from the front, above
        let to_light = ((eye - look).normalize_or_zero() + up * 0.6).normalize_or_zero();
        for (_, mat) in &m.parts {
            if let Some(mut mm) = materials.get_mut(mat) {
                mm.extension.params.sun = to_light.extend(1.0);
            }
        }
        let root = commands.spawn((Transform::IDENTITY, Visibility::Visible, layer.clone())).id();
        p.entities.push(root);
        for (mesh, mat) in &m.parts {
            commands.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(mat.clone()), layer.clone(), bevy::light::NotShadowCaster, bevy::light::NotShadowReceiver, ChildOf(root)));
        }
        let cam = commands
            .spawn((
                Camera3d::default(),
                Camera { order: -10, clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() },
                RenderTarget::Image(image.clone().into()),
                Projection::Perspective(PerspectiveProjection { fov, aspect_ratio: aspect, near, far, ..default() }),
                Transform::from_translation(eye).looking_at(look, up),
                Msaa::Off,
                layer,
            ))
            .id();
        p.entities.push(cam);
        p.shown = Some(key);
    }
    draw.portrait = p.image.clone().filter(|_| !p.entities.is_empty());
}
