// The original look: Direct3D 9 fixed-function lighting as AA.exe's
// Gamebryo renderer sets it up (NiMaterialProperty ambient/diffuse/emissive,
// NiTexturingProperty MODULATE, NiVertexColorProperty SOURCE_IGNORE, the sun
// of Data/Sun and SVars_AA), computed on gamma-space colours like the
// original's non-sRGB pipeline, plus the original alpha test and the
// terrain transition mask alpha.
#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}

struct Look {
    mask_u: vec4<f32>,
    mask_v: vec4<f32>,
    // x: terrain mask present, y: alpha test function (0 off, else NIF code + 1),
    // z: alpha test threshold (0..1), w: 1 = fixed-function lighting, 0 = Bevy
    // lit shading with shadows (the Development 12 look)
    flags: vec4<f32>,
    // fixed-function material colours, already multiplied by the sun colours
    ambient: vec4<f32>,
    diffuse: vec4<f32>,
    emissive: vec4<f32>,
    // xyz: render-space direction to the sun
    sun: vec4<f32>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> look: Look;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var mask_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var mask_sampler: sampler;

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
        out.color = main_pass_post_lighting_processing(pbr_input, out.color);
        return out;
    }
    let n = normalize(select(-pbr_input.world_normal, pbr_input.world_normal, is_front));
    let lambert = max(dot(n, normalize(look.sun.xyz)), 0.0);
    // D3D fixed function: clamp(emissive + ambient + diffuse * N.L), then
    // MODULATE with the texture, all on gamma values.
    let light = clamp(look.emissive.rgb + look.ambient.rgb + look.diffuse.rgb * lambert, vec3<f32>(0.0), vec3<f32>(1.0));
    let colour = to_gamma(base.rgb) * light;
    out.color = vec4<f32>(to_linear(colour), base.a);
    return out;
}
