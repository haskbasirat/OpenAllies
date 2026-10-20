//! The original look for every model and terrain layer: one material type
//! whose shader (original.wgsl) applies the original fixed-function lighting,
//! alpha test and terrain mask. Owner: lead.
use bevy::asset::uuid_handle;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Face, ShaderType};
use bevy::shader::{Shader, ShaderRef};

const LOOK_SHADER: Handle<Shader> = uuid_handle!("3b8e1f64-0a2c-4d7e-9b51-6c0f2e9d4a77");

pub type OriginalMaterial = ExtendedMaterial<StandardMaterial, OriginalLook>;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct OriginalLook {
    #[uniform(100)]
    pub params: LookParams,
    #[texture(101)]
    #[sampler(102)]
    pub mask: Option<Handle<Image>>,
}

#[derive(ShaderType, Debug, Clone)]
pub struct LookParams {
    pub mask_u: Vec4,
    pub mask_v: Vec4,
    pub flags: Vec4,
    pub ambient: Vec4,
    pub diffuse: Vec4,
    pub emissive: Vec4,
    pub sun: Vec4,
}

impl MaterialExtension for OriginalLook {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(LOOK_SHADER)
    }
}

pub struct LookPlugin;
impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<OriginalMaterial>::default());
        let _ = app.world_mut().resource_mut::<Assets<Shader>>().insert(LOOK_SHADER.id(), Shader::from_wgsl(include_str!("original.wgsl"), "aa_bevy/original.wgsl"));
    }
}

/// Sun of Data/Sun/sun_default.tgi and Game/SVars_AA.tgi: diffuse 1,1,1,
/// ambient 1,1,1, SunDirection 1,1,-2 (native, the way the light travels).
pub const SUN_AMBIENT: [f32; 3] = [1.0, 1.0, 1.0];
pub const SUN_DIFFUSE: [f32; 3] = [1.0, 1.0, 1.0];
pub fn sun_to_light() -> Vec3 {
    // native direction to the sun (-1,-1,2) -> render (x, z, -y)
    Vec3::new(-1.0, 2.0, 1.0).normalize()
}

/// Fixed-function material colours of a textured original mesh: the
/// shipped NIFs use NiMaterialProperty ambient 0.502, diffuse 1.0 (measured
/// on the Corps, trees, infantry, tanks, the Reichstag, high grass) with the
/// texture MODULATE; untextured meshes use their colour for both.
fn material_colours(m: &aa_assets::Material) -> ([f32; 3], [f32; 3]) {
    if m.texture.is_some() {
        ([0.502; 3], [1.0; 3])
    } else {
        (m.color, m.color)
    }
}

/// Lighting style: Bevy's lit shading with shadows (the Development 12 look,
/// default) or the original fixed-function formula (AA_BEVY_FIXED_FUNCTION=1).
pub fn fixed_function() -> bool {
    std::env::var_os("AA_BEVY_FIXED_FUNCTION").is_some()
}

fn params(ambient: [f32; 3], diffuse: [f32; 3], emissive: [f32; 3], alpha: (f32, f32)) -> LookParams {
    let mul = |c: [f32; 3], s: [f32; 3]| Vec4::new(c[0] * s[0], c[1] * s[1], c[2] * s[2], 1.0);
    LookParams {
        mask_u: Vec4::ZERO,
        mask_v: Vec4::ZERO,
        flags: Vec4::new(0.0, alpha.0, alpha.1, if fixed_function() { 1.0 } else { 0.0 }),
        ambient: mul(ambient, SUN_AMBIENT),
        diffuse: mul(diffuse, SUN_DIFFUSE),
        emissive: Vec4::new(emissive[0], emissive[1], emissive[2], 0.0),
        sun: sun_to_light().extend(0.0),
    }
}

/// Native alpha test: (function code + 1 or 0, threshold 0..1), as the
/// client's `alpha_test` reads NiAlphaProperty (flag 512 enables it).
fn alpha_test(m: &aa_assets::Material) -> (f32, f32) {
    if m.team_color_mask {
        return (0.0, 0.0);
    }
    if let Some(s) = m.native_render_state {
        let func = if s.alpha_flags & 512 == 0 { 0.0 } else { f32::from(((s.alpha_flags >> 10) & 7) + 1) };
        return (func, f32::from(s.alpha_threshold) / 255.0);
    }
    if m.alpha_test > 0.0 { (7.0, m.alpha_test) } else { (0.0, 0.0) }
}

/// An original model mesh material (after the audited model fixes).
pub fn model_material(m: &aa_assets::Material, texture: Option<Handle<Image>>) -> OriginalMaterial {
    let blend = !m.team_color_mask && m.native_render_state.map_or(m.alpha_blend || m.opacity < 1.0, |s| s.alpha_flags & 1 != 0);
    let alpha = alpha_test(m);
    let alpha_mode = if m.team_color_mask {
        AlphaMode::Opaque
    } else if blend {
        AlphaMode::Blend
    } else if alpha.0 > 0.0 {
        // The depth prepass needs the cutout; the shader applies the exact test.
        AlphaMode::Mask(alpha.1.max(1.0 / 255.0))
    } else {
        AlphaMode::Opaque
    };
    let (ambient, diffuse) = material_colours(m);
    OriginalMaterial {
        base: StandardMaterial {
            base_color: if m.texture.is_some() { Color::srgba(1.0, 1.0, 1.0, m.opacity) } else { Color::srgba(1.0, 1.0, 1.0, m.opacity) },
            base_color_texture: texture,
            alpha_mode,
            double_sided: m.double_sided,
            cull_mode: (!m.double_sided).then_some(Face::Back),
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            metallic: 0.0,
            ..default()
        },
        extension: OriginalLook { params: params(ambient, diffuse, m.emissive, alpha), mask: None },
    }
}

/// One terrain layer: the variation texture, and for transitions the
/// original mask with its compositor rows. `order` stacks layers.
pub fn terrain_material(texture: Option<Handle<Image>>, color: [f32; 4], mask: Option<(Handle<Image>, [[f32; 4]; 2])>, order: usize) -> OriginalMaterial {
    let overlay = mask.is_some();
    let mut p = params([0.502; 3], [1.0; 3], [0.0; 3], (0.0, 0.0));
    if let Some((_, rows)) = &mask {
        p.mask_u = Vec4::from_array(rows[0]);
        p.mask_v = Vec4::from_array(rows[1]);
        p.flags.x = 1.0;
    }
    OriginalMaterial {
        base: StandardMaterial {
            base_color: Color::srgba(color[0], color[1], color[2], 1.0),
            base_color_texture: texture,
            alpha_mode: if overlay { AlphaMode::Blend } else { AlphaMode::Opaque },
            // Overlays draw before every other transparent thing, in layer order.
            depth_bias: -1.0e8 + order as f32 * 1000.0,
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        },
        extension: OriginalLook { params: p, mask: mask.map(|m| m.0) },
    }
}
