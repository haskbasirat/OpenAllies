// Original effect quads (Gamebryo effect textures in one texture array).
// Premultiplied-alpha output so alpha-blended (0), additive src-alpha/one
// (1) and additive one/one (2) quads share one pipeline and one sorted draw,
// as the wgpu client's particles.wgsl. The texture and vertex colours are
// the original's gamma-space values; the result goes to Bevy's sRGB target
// as linear colour.
#import bevy_pbr::forward_io::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var atlas: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var atlas_sampler: sampler;

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((max(c, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
#ifdef VERTEX_UVS_B
    let layer = i32(in.uv_b.x + 0.5);
    let blend = in.uv_b.y;
#else
    let layer = 0;
    let blend = 0.0;
#endif
#ifdef VERTEX_COLORS
    let color = in.color;
#else
    let color = vec4<f32>(1.0);
#endif
#ifdef VERTEX_UVS_A
    let tex = textureSample(atlas, atlas_sampler, in.uv, layer);
#else
    let tex = vec4<f32>(1.0);
#endif
    let rgb = to_linear(clamp(tex.rgb * color.rgb, vec3<f32>(0.0), vec3<f32>(1.0)));
    let a = clamp(tex.a * color.a, 0.0, 1.0);
    var out: vec4<f32>;
    if blend < 0.5 {
        out = vec4<f32>(rgb * a, a);
    } else if blend < 1.5 {
        out = vec4<f32>(rgb * a, 0.0);
    } else {
        out = vec4<f32>(rgb, 0.0);
    }
    if max(max(out.r, out.g), max(out.b, out.a)) < 0.002 {
        discard;
    }
    return out;
}
