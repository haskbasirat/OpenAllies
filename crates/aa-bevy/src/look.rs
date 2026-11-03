//! The original look for every model and terrain layer: one material type
//! whose shader (original.wgsl) applies the original fixed-function lighting,
//! alpha test and terrain mask. Owner: lead.
use bevy::asset::uuid_handle;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Face, ShaderType};
use bevy::shader::{Shader, ShaderRef};

const LOOK_SHADER: Handle<Shader> = uuid_handle!("3b8e1f64-0a2c-4d7e-9b51-6c0f2e9d4a77");
/// The terrain and unit fog texture every original material samples
/// (written by `ui::fog`): one R8 texel per `FOG_TEXEL` metres of the native
/// ground from the map origin, 0 = visible, 255 = black.
pub const FOG_IMAGE: Handle<Image> = uuid_handle!("8d2c5a3e-6f1b-4e07-a9c4-2b7d0e5f9a61");
/// Fog texels per side (covers 4096 m: larger than any original map).
pub const FOG_SIZE: u32 = 1024;
/// Native metres per fog texel (the GMap tile).
pub const FOG_TEXEL: f32 = 4.0;

pub type OriginalMaterial = ExtendedMaterial<StandardMaterial, OriginalLook>;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct OriginalLook {
    #[uniform(100)]
    pub params: LookParams,
    #[texture(101)]
    #[sampler(102)]
    pub mask: Option<Handle<Image>>,
    /// The fog texture (always `FOG_IMAGE`).
    #[texture(103)]
    pub fog: Option<Handle<Image>>,
    /// The terrain's two NiTextureEffects (0x723E00; `params.ground`).
    #[texture(104)]
    #[sampler(105)]
    pub detail: Option<Handle<Image>>,
    #[texture(106)]
    #[sampler(107)]
    pub macro_map: Option<Handle<Image>>,
    /// The Biome's cloud_texture (`params.cloud`).
    #[texture(108)]
    #[sampler(109)]
    pub cloud: Option<Handle<Image>>,
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
    /// rgb: the stencil shadow multiplier (AVar C0
    /// StencilShadowColorMultiplier, the camera's shadow quad 7016A0).
    pub shadow: Vec4,
    /// The terrain's texture effects: x and y repeats per metre of the detail and
    /// macro maps (0 = none).
    pub ground: Vec4,
    /// A `lirp` VFXTintModifier (717E40 -> 716870, the model's last stage): rgb the colour,
    /// a how far the final colour goes to it (0 = none).
    pub lirp: Vec4,
    /// The terrain's cloud shadow: x, y repeats per metre (0 = none), z, w its motion
    /// in repeats per second.
    pub cloud: Vec4,
}

impl MaterialExtension for OriginalLook {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(LOOK_SHADER)
    }

    /// StandardMaterial turns `depth_bias` into the rasterizer's constant
    /// depth bias as well as the draw order. The terrain layers' -1e8 +
    /// 1000 * order is meant for the order only: as a depth bias it pushed
    /// the whole terrain to the far plane, so anything under the ground (the
    /// parts of an unpacking HQ that rise out of it) drew over it. Keep the
    /// order and only the layer step (one unit per layer) in the depth: the
    /// base layer at its own depth, each overlay just in front of the one
    /// below (appended by the effects owner, for the unpack aspects).
    fn specialize(
        _pipeline: &bevy::pbr::MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        if let Some(ds) = descriptor.depth_stencil.as_mut()
            && ds.bias.constant <= -10_000_000
        {
            ds.bias.constant = (ds.bias.constant + 100_000_000) / 1000;
        }
        Ok(())
    }
}

pub struct LookPlugin;
impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<OriginalMaterial>::default());
        let _ = app.world_mut().resource_mut::<Assets<Shader>>().insert(LOOK_SHADER.id(), Shader::from_wgsl(include_str!("original.wgsl"), "aa_bevy/original.wgsl"));
        // no fog until a battle writes it; created at its final size so the
        // GPU texture (and every material's binding of it) is reused
        let image = Image::new(
            bevy::render::render_resource::Extent3d { width: FOG_SIZE, height: FOG_SIZE, depth_or_array_layers: 1 },
            bevy::render::render_resource::TextureDimension::D2,
            vec![0; (FOG_SIZE * FOG_SIZE) as usize],
            bevy::render::render_resource::TextureFormat::R8Unorm,
            bevy::asset::RenderAssetUsages::default(),
        );
        let _ = app.world_mut().resource_mut::<Assets<Image>>().insert(FOG_IMAGE.id(), image);
    }
}

/// The light of the HUD's own model views (portraits): white. The battle's
/// materials take the sun's colours from the scene lights instead (see
/// [`apply_sun`]), so their params hold the material colours alone.
pub const SUN_AMBIENT: [f32; 3] = [1.0, 1.0, 1.0];
pub const SUN_DIFFUSE: [f32; 3] = [1.0, 1.0, 1.0];
pub fn sun_to_light() -> Vec3 {
    // native direction to the sun (-1,-1,2) -> render (x, z, -y)
    Vec3::new(-1.0, 2.0, 1.0).normalize()
}

/// Fixed-function material colours of an original mesh: its
/// NiMaterialProperty ambient (read from the original NIF by the model
/// fixes, [`record_ambient`]; most shipped materials have 0.502, the high
/// grass 0.745, some units 0.588 or 0.42) and diffuse (the converted
/// colour: 1.0 on most, 0.588 on the tank treads), with the texture
/// MODULATE. Without a recorded ambient a textured mesh keeps the common
/// 0.502 and an untextured one its colour.
fn material_colours(m: &aa_assets::Material) -> ([f32; 3], [f32; 3]) {
    let recorded = AMBIENT.read().ok().and_then(|t| t.get(&material_key(m)).copied());
    let ambient = recorded.unwrap_or(if m.texture.is_some() { [0.502; 3] } else { m.color });
    (ambient, m.color)
}

/// NiMaterialProperty ambient by converted material (the converted meshes
/// keep only the diffuse colour), filled before the materials are made.
static AMBIENT: std::sync::RwLock<std::collections::BTreeMap<u64, [f32; 3]>> = std::sync::RwLock::new(std::collections::BTreeMap::new());

fn material_key(m: &aa_assets::Material) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (&m.name, &m.texture, &m.source_texture, m.double_sided, m.team_color_mask).hash(&mut h);
    for v in m.color.iter().chain(&m.emissive).chain(std::iter::once(&m.opacity)) {
        v.to_bits().hash(&mut h);
    }
    h.finish()
}

/// The original NIF ambient of a converted mesh material.
pub fn record_ambient(m: &aa_assets::Material, ambient: [f32; 3]) {
    if let Ok(mut t) = AMBIENT.write() {
        t.insert(material_key(m), ambient);
    }
}

/// Lighting style: the original Direct3D fixed-function formula with the
/// battle's sun and the stencil shadow multiplier (default), or Bevy's lit
/// PBR shading of the Development 12 test (AA_BEVY_PBR_LOOK=1, kept for
/// comparison).
pub fn fixed_function() -> bool {
    std::env::var_os("AA_BEVY_PBR_LOOK").is_none()
}

fn params(ambient: [f32; 3], diffuse: [f32; 3], emissive: [f32; 3], alpha: (f32, f32)) -> LookParams {
    let mul = |c: [f32; 3], s: [f32; 3]| Vec4::new(c[0] * s[0], c[1] * s[1], c[2] * s[2], 1.0);
    let shadow = vars().stencil_shadow_color;
    LookParams {
        mask_u: Vec4::ZERO,
        mask_v: Vec4::ZERO,
        flags: Vec4::new(0.0, alpha.0, alpha.1, if fixed_function() { 1.0 } else { 0.0 }),
        ambient: mul(ambient, SUN_AMBIENT),
        diffuse: mul(diffuse, SUN_DIFFUSE),
        emissive: Vec4::new(emissive[0], emissive[1], emissive[2], 0.0),
        sun: sun_to_light().extend(0.0),
        shadow: Vec4::new(shadow[0], shadow[1], shadow[2], 1.0),
        ground: Vec4::ZERO,
        lirp: Vec4::ZERO,
        cloud: Vec4::ZERO,
    }
}

static NIGHT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 4EE070: the sun's diffuse (+38..+40) adds up below SVar SunNightThreshold. A `nightonly`
/// VFX is hidden while it is not (4446F0).
pub fn is_night() -> bool {
    NIGHT.load(std::sync::atomic::Ordering::Relaxed)
}

/// The world's Sun as 4EE070 reads it.
pub fn set_sun_for_night(diffuse: [f32; 3]) {
    NIGHT.store(diffuse[0] + diffuse[1] + diffuse[2] < vars().sun_night_threshold, std::sync::atomic::Ordering::Relaxed);
}

/// The original's lighting variables (Data/AVars*.tgi, Data/Game/SVars*.tgi
/// in loader order, a later file replacing an earlier value), read once.
#[derive(Clone, Debug)]
pub struct OriginalVars {
    /// AVar fast +0C / +08 (723E00: the terrain NiMaterialProperty ambient
    /// and diffuse are white B322A4 times these).
    pub terrain_ambient: f32,
    pub terrain_diffuse: f32,
    /// AVar fast +10: 43A400 multiplies the sun dimmer by it while paused.
    pub sun_dimmer_when_paused: f32,
    /// AVar C0 (7016A0: the stencil shadow quad's material emissive; its
    /// NiAlphaProperty 0x243 blends ZERO / SRCCOLOR, so a shadowed pixel is
    /// multiplied by this colour).
    pub stencil_shadow_color: [f32; 3],
    /// SVar SunNightThreshold (4EE070).
    pub sun_night_threshold: f32,
    /// The new world's Sun from the SVars (4EE1F0 at world init 4F27E0):
    /// SunDirection (normalised by 4453A0), SunDimmer, SunDiffuse,
    /// SunAmbient, SunSpecular.
    pub sun: SceneSun,
}

impl Default for OriginalVars {
    /// The shipped values (AVars.tgi, AVars_AA.tgi, SVars.tgi, SVars_AA.tgi).
    fn default() -> Self {
        Self {
            terrain_ambient: 0.15,
            terrain_diffuse: 1.0,
            sun_dimmer_when_paused: 0.8,
            stencil_shadow_color: [0.7, 0.7, 0.6],
            sun_night_threshold: 1.6,
            sun: SceneSun { direction: Vec3::new(1.0, 1.0, -2.0).normalize(), diffuse: [1.0; 3], ambient: [1.0; 3], specular: [0.0; 3], dimmer: 1.0 },
        }
    }
}

static VARS: std::sync::OnceLock<OriginalVars> = std::sync::OnceLock::new();

/// The lighting variables (the shipped values until [`load_vars`] ran).
pub fn vars() -> OriginalVars {
    VARS.get().cloned().unwrap_or_default()
}

/// Read the lighting variables from the install (`reconstruction` is the
/// converted-assets root, its parent the install). The v14 runtime's
/// replacement files come before the original Data copies.
pub fn load_vars(reconstruction: &std::path::Path) {
    let Some(vars) = read_vars(reconstruction) else { return };
    info!("original lighting variables: {vars:?}");
    let _ = VARS.set(vars);
}

/// Every numeric variable of the four files, by lower-case name.
fn read_values(reconstruction: &std::path::Path) -> Option<std::collections::HashMap<String, Vec<f32>>> {
    let install = reconstruction.parent()?;
    let runtime = install.join("reconstruction/native-game/runtime-expanded-v14/Data");
    let original = reconstruction.join("assets/original/Data");
    let mut values: std::collections::HashMap<String, Vec<f32>> = std::collections::HashMap::new();
    for (file, header) in [("AVars.tgi", "Vars"), ("AVars_AA.tgi", "Vars"), ("Game/SVars.tgi", "SVars"), ("Game/SVars_AA.tgi", "SVars")] {
        let path = [runtime.join(file), original.join(file)].into_iter().find(|p| p.exists());
        let Some(bytes) = path.and_then(|p| std::fs::read(p).ok()) else { continue };
        let Ok(doc) = aa_rules::tgi::Document::parse_native(bytes, aa_rules::tgi::Limits::default()) else { continue };
        for item in &doc.items {
            let aa_rules::tgi::Item::Node(node) = item else { continue };
            if !doc.header_name(&node.header).is_some_and(|h| h.eq_ignore_ascii_case(header)) {
                continue;
            }
            for item in &node.items {
                let aa_rules::tgi::Item::Property(p) = item else { continue };
                let (Some(name), Some(raw)) = (doc.property_name(p), doc.property_value(p)) else { continue };
                let parts: Option<Vec<f32>> = raw.split(';').next().unwrap_or("").split(',').map(|v| number(v.trim())).collect();
                if let Some(parts) = parts.filter(|p| !p.is_empty()) {
                    values.insert(name.to_ascii_lowercase(), parts);
                }
            }
        }
    }
    Some(values)
}

fn read_vars(reconstruction: &std::path::Path) -> Option<OriginalVars> {
    let values = read_values(reconstruction)?;
    let mut vars = OriginalVars::default();
    let one = |name: &str, v: &mut f32| {
        if let Some(x) = values.get(name).and_then(|p| p.first()) {
            *v = *x;
        }
    };
    let three = |name: &str, v: &mut [f32; 3]| {
        if let Some(p) = values.get(name).filter(|p| p.len() == 3) {
            *v = [p[0], p[1], p[2]];
        }
    };
    one("terrainambient", &mut vars.terrain_ambient);
    one("terraindiffuse", &mut vars.terrain_diffuse);
    one("sundimmermultiplierwhenpaused", &mut vars.sun_dimmer_when_paused);
    one("sunnightthreshold", &mut vars.sun_night_threshold);
    three("stencilshadowcolormultiplier", &mut vars.stencil_shadow_color);
    let mut direction = [1.0, 1.0, -2.0];
    three("sundirection", &mut direction);
    vars.sun.direction = Vec3::from_array(direction).normalize_or_zero();
    one("sundimmer", &mut vars.sun.dimmer);
    three("sundiffuse", &mut vars.sun.diffuse);
    three("sunambient", &mut vars.sun.ambient);
    three("sunspecular", &mut vars.sun.specular);
    Some(vars)
}

/// A TGI number (`15%` = 0.15).
fn number(s: &str) -> Option<f32> {
    let t = s.trim().trim_matches('"');
    match t.strip_suffix('%') {
        Some(p) => p.trim().parse::<f32>().ok().map(|v| v / 100.0),
        None => t.parse::<f32>().ok(),
    }
}

/// The battle's sun as 43A400 hands it to the scene light (VWorld+108):
/// NiLight dimmer +F4, ambient +F8, diffuse +104, specular +110 and the
/// direction the light travels (native coordinates).
#[derive(Resource, Clone, Debug)]
pub struct SceneSun {
    pub direction: Vec3,
    pub diffuse: [f32; 3],
    pub ambient: [f32; 3],
    pub specular: [f32; 3],
    pub dimmer: f32,
}

impl SceneSun {
    /// The map's saved Sun (MAP chunk "SUN", GWorld+4C Load 4EDBD0): the
    /// direction, three current and three target colours (diffuse, ambient,
    /// specular), the colour duration and the dimmer. With no colour or
    /// dimmer transition running (duration <= 0) 43A400 lights the scene
    /// with the target colours and the current dimmer.
    pub fn from_map_chunk(bytes: &[u8]) -> Option<Self> {
        let mut at = 0usize;
        let mut word = || -> Option<u32> {
            let w = u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?);
            at += 4;
            Some(w)
        };
        let version = word()? as i32;
        let updatable = word()? as i32;
        if !(0..=2).contains(&version) || !(0..=1).contains(&updatable) {
            return None;
        }
        if updatable >= 1 {
            word()?;
        }
        let mut f = || word().map(f32::from_bits);
        let direction = Vec3::new(f()?, f()?, f()?);
        let mut triple = || -> Option<[f32; 3]> { Some([f()?, f()?, f()?]) };
        let current = [triple()?, triple()?, triple()?];
        let target = [triple()?, triple()?, triple()?];
        let mut f = || word().map(f32::from_bits);
        let color_duration = f()?;
        let current_dimmer = f()?;
        // a running transition starts from the current colours (43A390 at
        // no elapsed time); the dimmer lerp starts from the current dimmer
        let colors = if color_duration > 0.0 { current } else { target };
        Some(Self { direction: direction.normalize_or_zero(), diffuse: colors[0], ambient: colors[1], specular: colors[2], dimmer: current_dimmer })
    }

    /// Render-space direction to the sun.
    pub fn to_light(&self) -> Vec3 {
        let d = -self.direction;
        Vec3::new(d.x, d.z, -d.y).normalize_or_zero()
    }
}

/// Hand the battle's sun to the scene lights every frame (43A400): the
/// directional light carries diffuse x dimmer, the ambient light ambient x
/// dimmer (Gamebryo scales every light colour by its dimmer), both at unit
/// intensity so the fixed-function shader reads the original colours. While
/// the game is paused the dimmer is multiplied by
/// SunDimmerMultiplierWhenPaused.
pub fn apply_sun(
    sun: Option<Res<SceneSun>>,
    battle: Option<Res<crate::session::Battle>>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut lights: Query<(&mut DirectionalLight, &mut Transform)>,
) {
    let Some(sun) = sun.filter(|_| fixed_function()) else { return };
    let paused = battle.as_ref().is_some_and(|b| b.paused);
    // The World+4C Sun the map's triggers drive (sun_colors / sun_dimmer:
    // 4EDB10 / 4ED820, moved by 4EDDC0), when the scenario runs them; else
    // the battle's starting sun.
    let (diffuse, ambient_colour, dimmer) = match battle.as_ref().and_then(|b| aa_play::triggers::sun(&b.world)) {
        Some(([diffuse, ambient, _specular], dimmer)) => (diffuse, ambient, dimmer),
        None => (sun.diffuse, sun.ambient, sun.dimmer),
    };
    set_sun_for_night(diffuse);
    let dimmer = dimmer * if paused { vars().sun_dimmer_when_paused } else { 1.0 };
    let scaled = |c: [f32; 3]| Color::linear_rgb(c[0] * dimmer, c[1] * dimmer, c[2] * dimmer);
    let colour = scaled(ambient_colour);
    if ambient.color != colour || ambient.brightness != 1.0 {
        ambient.color = colour;
        ambient.brightness = 1.0;
    }
    let to_light = sun.to_light();
    for (mut light, mut transform) in &mut lights {
        let colour = scaled(diffuse);
        if light.color != colour || light.illuminance != 1.0 {
            light.color = colour;
            light.illuminance = 1.0;
        }
        let want = Transform::default().looking_to(-to_light, Vec3::Y);
        if transform.rotation != want.rotation {
            transform.rotation = want.rotation;
        }
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
    // NiAlphaProperty bits 1-4 source, 5-8 destination: ONE / ONE adds (the strategic
    // map's lamp lights, flags 0x1001)
    let additive = m.native_render_state.is_some_and(|s| (s.alpha_flags >> 1) & 15 == 0 && (s.alpha_flags >> 5) & 15 == 0);
    let alpha_mode = if m.team_color_mask {
        AlphaMode::Opaque
    } else if blend && additive {
        AlphaMode::Add
    } else if blend {
        AlphaMode::Blend
    } else if alpha.0 > 0.0 {
        // The depth prepass needs the cutout; the shader applies the exact test.
        AlphaMode::Mask(alpha.1.max(1.0 / 255.0))
    } else {
        AlphaMode::Opaque
    };
    let (ambient, diffuse) = material_colours(m);
    let mut look_params = params(ambient, diffuse, m.emissive, alpha);
    look_params.emissive.w = f32::from(u8::from(alpha_mode == AlphaMode::Add));
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
        extension: OriginalLook { params: look_params, mask: None, fog: Some(FOG_IMAGE), detail: None, macro_map: None, cloud: None },
    }
}

/// The biome's two texture effects (Biome 0x4B8E80: `detail_texture` +0x88 with
/// `detail_repetition` +0x90, `macro_texture` +0x8C with `macro_repetition` +0x94).
/// 0x723E00 makes a NiTextureEffect of each: filter 2, wrap 3 (S and T), mode 0
/// (projected light), coordinates 0 (world parallel), the matrix `1 / repetition`
/// on X and Y: one repeat per `repetition` metres of the native ground. A repetition
/// of 0 is 0.8 m (0xA16F30) and 80 m (0xA88E50); the shipped biomes give none.
/// 0x71FAC0 -> 0x71D6A0 attaches them to the terrain's quads while the UVar
/// TerrainExtraQuality holds (the renderer's stage count gates them too: five stages
/// for the detail, six for the macro beside it; every card has those now).
#[derive(Clone, Debug, Default)]
pub struct GroundMaps {
    pub detail: Option<Handle<Image>>,
    pub macro_map: Option<Handle<Image>>,
    pub detail_per_metre: f32,
    pub macro_per_metre: f32,
    /// TerrainExtraQuality.
    pub on: bool,
    /// The cloud shadow of the Biome's CloudInfo (0x723E00, the effect at Terrain +0x34):
    /// a projected shadow, its texture, repeats per metre (1 / cloud_scale, 50 m without
    /// one) and its motion in repeats per second.
    pub cloud: Option<Handle<Image>>,
    pub cloud_per_metre: Vec2,
    pub cloud_motion: Vec2,
}

/// `<Data>/Terrain/<biome>.tgi` as 0x4B8E80 reads it, with the v14 runtime's files
/// before the original's; textures are the DDS mip chains of `Data/Textures/Beauty`.
pub fn load_ground_maps(reconstruction: &std::path::Path, biome: &str, images: &mut Assets<Image>, on: bool) -> GroundMaps {
    let install = reconstruction.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
    let roots = [install.join("reconstruction/native-game/runtime-expanded-v14/Data"), reconstruction.join("assets/original/Data")];
    // A battle on a data folder reads its own first (`effects::bones::folder`).
    let folder = crate::effects::bones::folder();
    let read = |rel: &str| folder.iter().chain(roots.iter()).find_map(|r| std::fs::read(r.join(rel.trim_start_matches('/'))).ok());
    let mut out = GroundMaps { detail_per_metre: 1.0 / 0.8, macro_per_metre: 1.0 / 80.0, on, ..GroundMaps::default() };
    let Some(text) = read(&format!("Terrain/{biome}.tgi")).map(|b| String::from_utf8_lossy(&b).into_owned()) else { return out };
    let value = |key: &str| {
        text.lines().find_map(|l| {
            let (k, v) = l.split(";;").next()?.split_once('=')?;
            (k.trim().eq_ignore_ascii_case(key)).then(|| v.trim().to_owned())
        })
    };
    let mut image = |key: &str| {
        let path = value(key)?;
        let bytes = read(&path).or_else(|| read(&path.to_ascii_lowercase()))?;
        let image = dds_image(&bytes)?;
        Some(images.add(image))
    };
    out.detail = image("detail_texture");
    out.macro_map = image("macro_texture");
    let repetition = |key: &str, default: f32| value(key).and_then(|v| v.parse::<f32>().ok()).filter(|r| *r != 0.0).unwrap_or(default);
    out.detail_per_metre = 1.0 / repetition("detail_repetition", 0.8);
    out.macro_per_metre = 1.0 / repetition("macro_repetition", 80.0);
    // 0x723E00: direction normalized, over the scale, per axis; 0x71E020 moves the map by
    // that times a speed that wanders between cloud_min_velocity and cloud_max_velocity on
    // the view's own random numbers. Here it holds their middle (a labelled rendition: the
    // wander has nothing to compare with).
    let pair = |key: &str| value(key).and_then(|v| {
        let mut parts = v.split(',').map(|p| p.trim().parse::<f32>().ok());
        Some(Vec2::new(parts.next()??, parts.next()??))
    });
    let number = |key: &str| value(key).and_then(|v| v.parse::<f32>().ok());
    if let Some(path) = value("cloud_texture")
        && let Some(bytes) = read(&path).or_else(|| read(&path.to_ascii_lowercase()))
        && let Some(cloud) = cloud_image(&bytes)
    {
        out.cloud = Some(images.add(cloud));
        let scale = pair("cloud_scale").unwrap_or(Vec2::ZERO);
        let scale = Vec2::new(if scale.x == 0.0 { 50.0 } else { scale.x }, if scale.y == 0.0 { 50.0 } else { scale.y });
        let direction = pair("cloud_movement_direction").filter(|d| *d != Vec2::ZERO).unwrap_or(Vec2::X).normalize();
        let speed = (number("cloud_min_velocity").unwrap_or(0.0) + number("cloud_max_velocity").unwrap_or(0.0)) * 0.5;
        out.cloud_per_metre = Vec2::ONE / scale;
        out.cloud_motion = direction / scale * speed;
    }
    out
}

/// The cloud map, a TGA: repeating (wrap 3), filtered without mips (filter 1).
fn cloud_image(bytes: &[u8]) -> Option<Image> {
    let rgba = image::load_from_memory_with_format(bytes, image::ImageFormat::Tga).ok()?.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut image = Image::new(
        bevy::render::render_resource::Extent3d { width, height, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        rgba.into_raw(),
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        ..bevy::image::ImageSamplerDescriptor::default()
    });
    Some(image)
}

/// A DXT3 DDS with its mips as an RGBA image that repeats, filtered through its
/// mips (the effects' filter 2).
fn dds_image(b: &[u8]) -> Option<Image> {
    let u = |o: usize| b.get(o..o + 4).map(|v| u32::from_le_bytes([v[0], v[1], v[2], v[3]]));
    if b.get(0..4)? != b"DDS " || b.get(84..88)? != b"DXT3" {
        return None;
    }
    let (height, width, mips) = (u(12)? as usize, u(16)? as usize, u(28)?.max(1) as usize);
    let mut data = Vec::new();
    let mut at = 128;
    let (mut w, mut h) = (width, height);
    let mut levels = 0;
    for _ in 0..mips {
        let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
        let size = bw * bh * 16;
        let blocks = b.get(at..at + size)?;
        let mut level = vec![0u8; w * h * 4];
        for by in 0..bh {
            for bx in 0..bw {
                let block = &blocks[(by * bw + bx) * 16..][..16];
                let c0 = u16::from_le_bytes([block[8], block[9]]);
                let c1 = u16::from_le_bytes([block[10], block[11]]);
                let rgb = |c: u16| [i32::from(c >> 11 & 31) * 255 / 31, i32::from(c >> 5 & 63) * 255 / 63, i32::from(c & 31) * 255 / 31];
                let (a, d) = (rgb(c0), rgb(c1));
                let colours = [a, d, [(2 * a[0] + d[0]) / 3, (2 * a[1] + d[1]) / 3, (2 * a[2] + d[2]) / 3], [(a[0] + 2 * d[0]) / 3, (a[1] + 2 * d[1]) / 3, (a[2] + 2 * d[2]) / 3]];
                let bits = u32::from_le_bytes([block[12], block[13], block[14], block[15]]);
                let alpha = u64::from_le_bytes([block[0], block[1], block[2], block[3], block[4], block[5], block[6], block[7]]);
                for py in 0..4 {
                    for px in 0..4 {
                        let (x, y) = (bx * 4 + px, by * 4 + py);
                        if x >= w || y >= h {
                            continue;
                        }
                        let i = py * 4 + px;
                        let c = colours[(bits >> (2 * i) & 3) as usize];
                        let o = (y * w + x) * 4;
                        level[o] = c[0] as u8;
                        level[o + 1] = c[1] as u8;
                        level[o + 2] = c[2] as u8;
                        level[o + 3] = ((alpha >> (4 * i) & 15) * 17) as u8;
                    }
                }
            }
        }
        data.extend_from_slice(&level);
        at += size;
        levels += 1;
        if w == 1 && h == 1 {
            break;
        }
        (w, h) = ((w / 2).max(1), (h / 2).max(1));
    }
    use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
    let mut image = Image::new(
        bevy::render::render_resource::Extent3d { width: width as u32, height: height as u32, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        data[..width * height * 4].to_vec(),
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(data);
    image.texture_descriptor.mip_level_count = levels;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..ImageSamplerDescriptor::default()
    });
    Some(image)
}

/// `LookParams::ground` of these maps.
pub fn ground_params(ground: &GroundMaps) -> Vec4 {
    Vec4::new(
        if ground.on && ground.detail.is_some() { ground.detail_per_metre } else { 0.0 },
        if ground.on && ground.macro_map.is_some() { ground.macro_per_metre } else { 0.0 },
        0.0,
        0.0,
    )
}

/// One terrain layer: the variation texture, and for transitions the
/// original mask with its compositor rows. `order` stacks layers.
pub fn terrain_material(texture: Option<Handle<Image>>, color: [f32; 4], mask: Option<(Handle<Image>, [[f32; 4]; 2])>, order: usize, ground: &GroundMaps) -> OriginalMaterial {
    let overlay = mask.is_some();
    // 723E00: the terrain's NiMaterialProperty is white (B322A4) times
    // TerrainAmbient / TerrainDiffuse, no specular or emissive (B43E68)
    let v = vars();
    let mut p = params([v.terrain_ambient; 3], [v.terrain_diffuse; 3], [0.0; 3], (0.0, 0.0));
    if let Some((_, rows)) = &mask {
        p.mask_u = Vec4::from_array(rows[0]);
        p.mask_v = Vec4::from_array(rows[1]);
        p.flags.x = 1.0;
    }
    p.ground = ground_params(ground);
    if ground.cloud.is_some() {
        p.cloud = Vec4::new(ground.cloud_per_metre.x, ground.cloud_per_metre.y, ground.cloud_motion.x, ground.cloud_motion.y);
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
        extension: OriginalLook { params: p, mask: mask.map(|m| m.0), fog: Some(FOG_IMAGE), detail: ground.detail.clone(), macro_map: ground.macro_map.clone(), cloud: ground.cloud.clone() },
    }
}

/// The material of the terrain's water (0x7377F0): no NiMaterialProperty, the vertex
/// colour is the ambient and the diffuse (NiVertexColorProperty source 2, lighting 1), the
/// texture MODULATE, NiAlphaProperty 0xED, z test without z write. `order` keeps it after
/// the terrain's layers and before every other transparent thing.
pub fn water_material(texture: Option<Handle<Image>>, order: usize) -> OriginalMaterial {
    let mut p = params([1.0; 3], [1.0; 3], [0.0; 3], (0.0, 0.0));
    // The shader's mark for a colour that is the ambient and the diffuse.
    p.ground.z = 1.0;
    OriginalMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: texture,
            alpha_mode: AlphaMode::Blend,
            depth_bias: -1.0e8 + order as f32 * 1000.0,
            cull_mode: None,
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        },
        extension: OriginalLook { params: p, mask: None, fog: Some(FOG_IMAGE), detail: None, macro_map: None, cloud: None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(version: u32, updatable: u32, color_duration: f32) -> Vec<u8> {
        let mut out = Vec::new();
        let mut w = |v: u32| out.extend_from_slice(&v.to_le_bytes());
        w(version);
        w(updatable);
        if updatable >= 1 {
            w(7);
        }
        for v in [0.408_248_3f32, 0.408_248_3, -0.816_496_6] {
            w(v.to_bits());
        }
        // current diffuse / ambient / specular, then the targets
        for c in [[0.4f32, 0.4, 0.7], [0.4, 0.4, 0.7], [0.1, 0.1, 0.25], [1.0, 1.0, 1.0], [0.8, 0.8, 0.8], [0.0, 0.0, 0.0]] {
            for v in c {
                w(v.to_bits());
            }
        }
        for v in [color_duration, 1.0, 1.0, 0.0, -1.0] {
            w(v.to_bits());
        }
        if version >= 1 {
            out.push(0);
            out.extend_from_slice(&[0; 8]);
            out.push(0);
            out.extend_from_slice(&[0; 8]);
        }
        out
    }

    #[test]
    fn saved_sun_chunk_lights_with_targets_and_current_dimmer() {
        let sun = SceneSun::from_map_chunk(&chunk(2, 1, 0.0)).unwrap();
        assert_eq!(sun.diffuse, [1.0; 3]);
        assert_eq!(sun.ambient, [0.8; 3]);
        assert_eq!(sun.specular, [0.0; 3]);
        assert_eq!(sun.dimmer, 1.0);
        // native (1,1,-2) travel direction -> render direction to the sun
        assert!((sun.to_light() - Vec3::new(-1.0, 2.0, 1.0).normalize()).length() < 1e-5);
        // a running transition starts from the current colours
        let sun = SceneSun::from_map_chunk(&chunk(2, 0, 5.0)).unwrap();
        assert_eq!(sun.diffuse, [0.4, 0.4, 0.7]);
        // the Load's version gates (4EDBD0 accepts outer 0..2, Updatable 0..1)
        assert!(SceneSun::from_map_chunk(&chunk(3, 1, 0.0)).is_none());
        assert!(SceneSun::from_map_chunk(&chunk(2, 1, 0.0)[..20]).is_none());
    }

    /// The shipped files through the reader (when the data is present).
    #[test]
    fn shipped_lighting_variables() {
        let Some(root) = crate::ui::reconstruction_root() else { return };
        // the files themselves, not the defaults (they hold the same values)
        let values = read_values(&root).unwrap();
        let value = |name: &str| values.get(name).cloned().unwrap_or_else(|| panic!("{name} not read"));
        assert_eq!(value("terrainambient"), [0.15]);
        assert_eq!(value("terraindiffuse"), [1.0]);
        assert_eq!(value("sundimmermultiplierwhenpaused"), [0.8]);
        assert_eq!(value("stencilshadowcolormultiplier"), [0.7, 0.7, 0.6]);
        assert_eq!(value("sundirection"), [1.0, 1.0, -2.0]);
        assert_eq!(value("sundimmer"), [1.0]);
        assert_eq!(value("sunambient"), [1.0, 1.0, 1.0]);
        let v = read_vars(&root).unwrap();
        assert_eq!(v.terrain_ambient, 0.15);
        assert_eq!(v.terrain_diffuse, 1.0);
        assert_eq!(v.sun_dimmer_when_paused, 0.8);
        assert_eq!(v.stencil_shadow_color, [0.7, 0.7, 0.6]);
        assert!((v.sun.direction - Vec3::new(1.0, 1.0, -2.0).normalize()).length() < 1e-6);
        assert_eq!(v.sun.dimmer, 1.0);
        // SVars_AA replaces SVars' 0.8
        assert_eq!(v.sun.ambient, [1.0; 3]);
        assert_eq!(v.sun.diffuse, [1.0; 3]);
        assert_eq!(v.sun.specular, [0.0; 3]);
    }

    /// Every shipped map file decodes its saved Sun (when the data is present).
    #[test]
    fn shipped_maps_have_a_sun() {
        let Some(root) = crate::ui::reconstruction_root() else { return };
        let dir = root.join("assets/original/Data/Maps");
        let Ok(entries) = std::fs::read_dir(&dir) else { return };
        let mut maps = 0;
        for e in entries.flatten() {
            let path = e.path();
            if !path.extension().is_some_and(|x| x.eq_ignore_ascii_case("rmp")) {
                continue;
            }
            let archive = aa_mapworld::MapArchive::from_bytes(std::fs::read(&path).unwrap()).unwrap();
            let chunk = archive.decode_chunk("SUN", 1 << 16).unwrap_or_else(|e| panic!("{path:?}: {e}"));
            let sun = SceneSun::from_map_chunk(&chunk.bytes).unwrap_or_else(|| panic!("{path:?}: SUN does not parse"));
            assert!((sun.direction.length() - 1.0).abs() < 1e-3, "{path:?}: {sun:?}");
            assert!(sun.dimmer > 0.0 && sun.dimmer <= 1.0, "{path:?}: {sun:?}");
            // the trigger Sun reads the current colours, 43A400 the targets
            // while no transition runs: the shipped maps save them equal
            let word = |i: usize| f32::from_le_bytes(chunk.bytes[i * 4..i * 4 + 4].try_into().unwrap());
            let at = if u32::from_le_bytes(chunk.bytes[4..8].try_into().unwrap()) >= 1 { 3 } else { 2 } + 3;
            let (current, target, duration) = ((at..at + 9).map(word).collect::<Vec<_>>(), (at + 9..at + 18).map(word).collect::<Vec<_>>(), word(at + 18));
            eprintln!("{:?}: {sun:?} current {current:?} target {target:?} duration {duration}", path.file_name().unwrap());
            assert_eq!(current, target, "{path:?}");
            assert!(duration <= 0.0, "{path:?}");
            maps += 1;
        }
        assert!(maps > 0);
    }

    #[test]
    fn tgi_numbers_take_percent() {
        assert_eq!(number("15%"), Some(0.15));
        assert_eq!(number("0.8"), Some(0.8));
        assert_eq!(number(" 1 "), Some(1.0));
        assert_eq!(number("x"), None);
    }
}
