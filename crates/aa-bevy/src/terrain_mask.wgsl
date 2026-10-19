// Terrain layer: Bevy's standard lit material with the original transition
// mask alpha (same mask UV rows the wgpu client uses in scene.wgsl).
#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}

struct TerrainMask {
    mask_u: vec4<f32>,
    mask_v: vec4<f32>,
    flags: vec4<f32>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> terrain_mask: TerrainMask;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var mask_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var mask_sampler: sampler;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
#ifdef VERTEX_UVS_B
    if terrain_mask.flags.x > 0.5 {
        let local = vec3<f32>(in.uv_b, 1.0);
        let uv = vec2<f32>(dot(local, terrain_mask.mask_u.xyz), dot(local, terrain_mask.mask_v.xyz));
        pbr_input.material.base_color.a = textureSampleLevel(mask_texture, mask_sampler, uv, 0.0).a;
    }
#endif
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
