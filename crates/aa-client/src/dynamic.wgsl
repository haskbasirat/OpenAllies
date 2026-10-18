// Gameplay actor layer: instanced original models; skinned infantry use
// baked original animation frames fetched by vertex index.
struct Camera {
    view_projection: mat4x4<f32>,
    eye: vec4<f32>,
    light: vec4<f32>,
    team_color: vec4<f32>,
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
struct Material { color: vec4<f32>, emissive_cutoff: vec4<f32>, render_flags: vec4<f32>, terrain_u: vec4<f32>, terrain_v: vec4<f32> };
@group(1) @binding(2) var<uniform> material: Material;
@group(1) @binding(3) var terrain_mask: texture_2d<f32>;
@group(1) @binding(4) var terrain_mask_sampler: sampler;

struct AnimInfo { vertex_count: u32, frames: u32, pad0: u32, pad1: u32 };
@group(2) @binding(0) var<storage, read> frames: array<f32>;
@group(2) @binding(1) var<uniform> anim: AnimInfo;

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
    @location(10) frame: vec4<f32>,
};
struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(4) @interpolate(flat) team_color: vec4<f32>,
    @location(5) world: vec3<f32>,
    @location(6) @interpolate(flat) tint: f32,
};

fn finish(v: VertexInput, position: vec3<f32>, normal: vec3<f32>) -> VertexOutput {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    var o: VertexOutput;
    let world = model * vec4<f32>(position, 1.0);
    o.clip = camera.view_projection * camera.model_root * world;
    o.world = world.xyz;
    o.normal = normalize((camera.root_normal * model * vec4<f32>(normal, 0.0)).xyz);
    o.uv = v.uv;
    o.color = v.color;
    o.team_color = select(camera.team_color, v.team_color, v.team_color.w > 0.5);
    o.tint = v.frame.w;
    return o;
}

@vertex fn vs_static(v: VertexInput) -> VertexOutput {
    return finish(v, v.position, v.normal);
}

fn fetch(frame: u32, vertex: u32) -> array<f32, 6> {
    let base = (frame * anim.vertex_count + vertex) * 6u;
    return array<f32, 6>(frames[base], frames[base + 1u], frames[base + 2u], frames[base + 3u], frames[base + 4u], frames[base + 5u]);
}

@vertex fn vs_anim(v: VertexInput, @builtin(vertex_index) vi: u32) -> VertexOutput {
    let last = max(anim.frames, 1u) - 1u;
    let fa = min(u32(v.frame.x), last);
    let fb = min(u32(v.frame.y), last);
    let a = fetch(fa, vi);
    let b = fetch(fb, vi);
    let t = clamp(v.frame.z, 0.0, 1.0);
    let p = mix(vec3<f32>(a[0], a[1], a[2]), vec3<f32>(b[0], b[1], b[2]), t);
    let n = mix(vec3<f32>(a[3], a[4], a[5]), vec3<f32>(b[3], b[4], b[5]), t);
    return finish(v, p, n);
}

@fragment fn fs_main(v: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(diffuse, diffuse_sampler, v.uv);
    var sampled = texel;
    if material.render_flags.y > 0.5 {
        sampled = vec4<f32>(mix(v.team_color.rgb, texel.rgb, texel.a), 1.0);
    }
    var base = sampled * material.color * v.color;
    let threshold = material.emissive_cutoff.w;
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
    let output_alpha = select(1.0, base.a, material.render_flags.x > 0.5);
    var fog = 1.0;
    if camera.fog.z > 0.5 {
        let fog_uv = vec2<f32>(v.world.x * camera.fog.x, -v.world.z * camera.fog.y);
        fog = textureSampleLevel(fog_texture, fog_sampler, fog_uv, 0.0).r;
    }
    // Instance tint: >0 brightens (selection/hover), <0 darkens (ghost/disabled).
    var rgb = (base.rgb * illumination + material.emissive_cutoff.xyz) * fog;
    rgb = rgb * (1.0 + v.tint);
    return vec4<f32>(rgb, output_alpha);
}
