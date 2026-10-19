struct Camera {
    view_projection: mat4x4<f32>,
    eye: vec4<f32>,
    light: vec4<f32>,
    team_color:vec4<f32>,
    fog: vec4<f32>,
    model_root:mat4x4<f32>,
    root_inverse:mat4x4<f32>,
    root_normal:mat4x4<f32>,
    light_ambient:vec4<f32>,
    light_diffuse:vec4<f32>,
    light_specular:vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var fog_texture: texture_2d<f32>;
@group(0) @binding(2) var fog_sampler: sampler;
@group(1) @binding(0) var diffuse: texture_2d<f32>;
@group(1) @binding(1) var diffuse_sampler: sampler;
struct Material { color: vec4<f32>, emissive_cutoff: vec4<f32>, render_flags:vec4<f32>, terrain_u:vec4<f32>, terrain_v:vec4<f32> };
@group(1) @binding(2) var<uniform> material: Material;
@group(1) @binding(3) var terrain_mask: texture_2d<f32>;
@group(1) @binding(4) var terrain_mask_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(8) terrain_uv: vec2<f32>,
    @location(4) m0: vec4<f32>,
    @location(5) m1: vec4<f32>,
    @location(6) m2: vec4<f32>,
    @location(7) m3: vec4<f32>,
    @location(9) team_color: vec4<f32>,
};
struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) terrain_uv: vec2<f32>,
    @location(4) @interpolate(flat) team_color: vec4<f32>,
    @location(5) world: vec3<f32>,
};
@vertex fn vs_main(v: VertexInput) -> VertexOutput {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    var o: VertexOutput;
    let world = model * vec4<f32>(v.position, 1.0);
    o.clip = camera.view_projection * camera.model_root * world;
    o.world = world.xyz;
    // Converted instances have only a uniform scale and yaw.
    o.normal = normalize((camera.root_normal * model * vec4<f32>(v.normal, 0.0)).xyz);
    o.uv = v.uv;
    o.color = v.color;
    o.terrain_uv = v.terrain_uv;
    o.team_color = select(camera.team_color,v.team_color,v.team_color.w>0.5);
    return o;
}
@fragment fn fs_main(v: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(diffuse, diffuse_sampler, v.uv);
    // Native RepColor base stage (73A880) uses BLENDTEXTUREALPHA with
    // texture ARG1/team factor ARG2, followed by diffuse modulation.
    var sampled=texel;
    if material.render_flags.y>0.5 {
        sampled=vec4<f32>(mix(v.team_color.rgb,texel.rgb,texel.a),1.0);
    }
    var base = sampled * material.color * v.color;
    if material.render_flags.w > 0.0 {
        let local_uv = vec3<f32>(v.terrain_uv, 1.0);
        let mask_uv = vec2<f32>(dot(local_uv,material.terrain_u.xyz),dot(local_uv,material.terrain_v.xyz));
        // Native texture stage1 preserves RGB and selects mask texture alpha.
        base.a = textureSampleLevel(terrain_mask,terrain_mask_sampler,mask_uv,0.0).a;
    }
    let threshold = material.emissive_cutoff.w;
    // Native 73A5B0 selects all eight comparison functions, not always GEQUAL.
    // Values are NIF compare + 1; zero is disabled by the authored flag.
    let alpha_mode = u32(material.render_flags.z);
    var pass_alpha = true;
    switch alpha_mode {
        case 2u: { pass_alpha = base.a < threshold; }
        case 3u: { pass_alpha = base.a == threshold; }
        case 4u: { pass_alpha = base.a <= threshold; }
        case 5u: { pass_alpha = base.a > threshold; }
        case 6u: { pass_alpha = base.a != threshold; }
        case 7u: { pass_alpha = base.a >= threshold; }
        case 8u: { pass_alpha = false; }
        default: {}
    }
    if !pass_alpha { discard; }
    let lambert=max(dot(normalize(v.normal), normalize(camera.light.xyz)), 0.0);
    var illumination=vec3<f32>(0.48+0.52*lambert);
    if camera.light.w>0.5 { illumination=camera.light_ambient.rgb+camera.light_diffuse.rgb*lambert; }
    // DDS alpha can be a replacement-color mask. Opaque/cutout geometry must
    // leave an opaque viewport pixel; only authored blend materials use alpha.
    let output_alpha=select(1.0,base.a,material.render_flags.x>0.5);
    // Native terrain fog of war (7208D0 -> 71C290 per TerrainBlock). Only
    // TerrainBlock geometry (terrain_v.w) receives it, like the 720AD0
    // NiTextureEffect attached under the block node. Render x = native x,
    // render z = -native y. Block (c, r) owns texels [c*n, c*n+n): the
    // effect maps uv = (p / A0 - c) * (n-1)/n + 0.5/(n-1) (88A910 scale
    // 1/A0, 88A990 -c, 88A910 (n-1)/n, 88A990 0.5/(n-1)), clamped inside the
    // block's own n texels.
    var fog = 1.0;
    if camera.fog.w > 0.5 && material.terrain_v.w > 0.5 {
        let a0 = camera.fog.x;
        let n = camera.fog.y;
        let p = vec2<f32>(v.world.x, -v.world.z) / a0;
        let block = floor(p);
        let uv = (p - block) * ((n - 1.0) / n) + vec2<f32>(0.5 / (n - 1.0));
        let texel = clamp(uv * n, vec2<f32>(0.5), vec2<f32>(n - 0.5));
        let dims = vec2<f32>(textureDimensions(fog_texture));
        fog = textureSampleLevel(fog_texture, fog_sampler, (block * n + texel) / dims, 0.0).r;
    }
    return vec4<f32>((base.rgb * illumination + material.emissive_cutoff.xyz) * fog, output_alpha);
}
