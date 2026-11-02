// The original look: Direct3D 9 fixed-function lighting as AA.exe's
// Gamebryo renderer sets it up (NiMaterialProperty ambient/diffuse/emissive,
// NiTexturingProperty MODULATE, NiVertexColorProperty SOURCE_IGNORE, the
// battle's sun of 43A400 handed over as the scene lights), computed on
// gamma-space colours like the original's non-sRGB pipeline, the stencil
// shadow multiply of the camera's shadow quad (7016A0), plus the original
// alpha test and the terrain transition mask alpha.
#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings as view_bindings,
    mesh_view_types,
    shadows,
    mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT,
}

struct Look {
    mask_u: vec4<f32>,
    mask_v: vec4<f32>,
    // x: terrain mask present, y: alpha test function (0 off, else NIF code + 1),
    // z: alpha test threshold (0..1), w: 1 = fixed-function lighting, 0 = Bevy
    // lit shading with shadows (the Development 12 look)
    flags: vec4<f32>,
    // fixed-function material colours (the battle's sun colours come from the
    // scene lights; the HUD's model views keep white light)
    ambient: vec4<f32>,
    diffuse: vec4<f32>,
    emissive: vec4<f32>,
    // xyz: render-space direction to the sun; w: 1 = no fog (the HUD's
    // portrait models, lit by this direction and white light)
    sun: vec4<f32>,
    // rgb: StencilShadowColorMultiplier (AVar C0)
    shadow: vec4<f32>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> look: Look;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var mask_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var mask_sampler: sampler;
// The terrain / unit fog (KKC_Terrain fog blocks 0x7203F0 / 0x71BD00 /
// 0x71C290): one texel per 4 m GMap tile of the native ground, 0 visible,
// TerrainFogGrayPercentage * 255 explored, 255 never explored.
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var fog_texture: texture_2d<f32>;

// Native metres per fog texel and the original texture density
// (AVar TerrainFogPixelsPerMeter = 2): the original fog texture holds two
// texels per metre, each the value of the tile under it, bilinearly
// filtered, so a tile edge blends over half a metre.
const FOG_TEXEL: f32 = 4.0;
const FOG_PIXELS_PER_METER: f32 = 2.0;

fn fog_texel(i: vec2<i32>) -> f32 {
    let size = vec2<i32>(textureDimensions(fog_texture));
    let c = clamp(i, vec2<i32>(0), size - vec2<i32>(1));
    return textureLoad(fog_texture, c, 0).r;
}

/// Fog darkening at a render-space position: the terrain colour is
/// multiplied by (1 - fog), the way a black fog layer of that opacity
/// covers it.
fn fog_at(world: vec3<f32>) -> f32 {
    if look.sun.w > 0.5 {
        return 0.0;
    }
    // render (x, z, -y) -> native ground (x, y), in texels
    let p = vec2<f32>(world.x, -world.z) / FOG_TEXEL - vec2<f32>(0.5);
    let i = vec2<i32>(floor(p));
    var f = fract(p);
    let sharp = FOG_TEXEL * FOG_PIXELS_PER_METER;
    f = clamp((f - vec2<f32>(0.5)) * sharp + vec2<f32>(0.5), vec2<f32>(0.0), vec2<f32>(1.0));
    let a = mix(fog_texel(i), fog_texel(i + vec2<i32>(1, 0)), f.x);
    let b = mix(fog_texel(i + vec2<i32>(0, 1)), fog_texel(i + vec2<i32>(1, 1)), f.x);
    return mix(a, b, f.y);
}

fn to_gamma(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((max(c, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

fn alpha_passes(a: f32, func: u32, threshold: f32) -> bool {
    switch func {
        case 2u: { return a < threshold; }
        case 3u: { return a == threshold; }
        case 4u: { return a <= threshold; }
        case 5u: { return a > threshold; }
        case 6u: { return a != threshold; }
        case 7u: { return a >= threshold; }
        case 8u: { return false; }
        default: { return true; }
    }
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    // Texture (sRGB-decoded) times the material colour.
    var base = pbr_input.material.base_color;
#ifdef VERTEX_UVS_B
    if look.flags.x > 0.5 {
        // Native texture stage 1 keeps RGB and takes the mask texture alpha.
        let local = vec3<f32>(in.uv_b, 1.0);
        let uv = vec2<f32>(dot(local, look.mask_u.xyz), dot(local, look.mask_v.xyz));
        base.a = textureSampleLevel(mask_texture, mask_sampler, uv, 0.0).a;
    }
#endif
    if !alpha_passes(base.a, u32(look.flags.y), look.flags.z) {
        discard;
    }
    var out: FragmentOutput;
    if look.flags.w < 0.5 {
        // Bevy's lit shading and shadows, as Development 12 drew it.
        pbr_input.material.base_color = base;
        out.color = apply_pbr_lighting(pbr_input);
        // the fog darkens on gamma values like the original's pipeline
        let keep = to_linear(vec3<f32>(1.0 - fog_at(in.world_position.xyz)));
        out.color = vec4<f32>(out.color.rgb * keep, out.color.a);
        out.color = main_pass_post_lighting_processing(pbr_input, out.color);
        return out;
    }
    let n = normalize(select(-pbr_input.world_normal, pbr_input.world_normal, is_front));
    var to_sun = normalize(look.sun.xyz);
    var sun_ambient = vec3<f32>(1.0);
    var sun_diffuse = vec3<f32>(1.0);
    var shadow = vec3<f32>(1.0);
    if look.sun.w < 0.5 {
        // The battle's sun (43A400 -> VWorld+108 NiLight): its ambient and
        // dimmed diffuse colours and direction, handed over as Bevy's
        // ambient and first directional light at unit intensity.
        sun_ambient = view_bindings::lights.ambient_color.rgb;
        if view_bindings::lights.n_directional_lights > 0u {
            let light = &view_bindings::lights.directional_lights[0];
            sun_diffuse = (*light).color.rgb;
            to_sun = normalize((*light).direction_to_light);
            if (pbr_input.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
                && ((*light).flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
                // The stencil shadows: the camera's full-screen quad (7016A0)
                // blends ZERO / SRCCOLOR with emissive
                // StencilShadowColorMultiplier where the shadow volumes
                // marked the stencil; the sun's shadow map marks it here.
                let view_z = dot(vec4<f32>(
                    view_bindings::view.view_from_world[0].z,
                    view_bindings::view.view_from_world[1].z,
                    view_bindings::view.view_from_world[2].z,
                    view_bindings::view.view_from_world[3].z
                ), pbr_input.world_position);
                let lit = shadows::fetch_directional_shadow(0u, pbr_input.world_position, n, view_z, pbr_input.frag_coord.xy);
                shadow = mix(look.shadow.rgb, vec3<f32>(1.0), lit);
            }
        }
    }
    let lambert = max(dot(n, to_sun), 0.0);
    // D3D fixed function: clamp(emissive + ambient * light ambient + diffuse
    // * light diffuse * N.L), then MODULATE with the texture, all on gamma
    // values; the shadow quad and the fog layer multiply the result.
    let light = clamp(look.emissive.rgb + look.ambient.rgb * sun_ambient + look.diffuse.rgb * sun_diffuse * lambert, vec3<f32>(0.0), vec3<f32>(1.0));
    let colour = to_gamma(base.rgb) * light * shadow * (1.0 - fog_at(in.world_position.xyz));
    // emissive.w: AlphaMode::Add (ONE / ONE), whose blend state takes alpha 0
    out.color = vec4<f32>(to_linear(colour), select(base.a, 0.0, look.emissive.w > 0.5));
    return out;
}
