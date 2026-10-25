// Effect mode 1 of the original's UI textures: a disabled button icon
// (0x5E66E0 -> 0x73EAF0). These are the Direct3D 9 fixed-function stages that
// 0x742CC0 / 0x742630 set for it (the GrayscaleShader 0x73B6B0 sets the
// same ones):
//   TEXTUREFACTOR 0x7F7F7F;
//   stage 0: COLOROP MULTIPLYADD, ARG0 TFACTOR, ARG1 TEXTURE, ARG2 TFACTOR
//            (TFACTOR + TEXTURE * TFACTOR);
//   stage 1: COLOROP DOTPRODUCT3, ARG1 TEXTURE (the renderer's 2x2 RGBA32
//            image filled with 0x008EA885 in 0x71032E, DAT_00B40698:
//            R 0x85, G 0xA8, B 0x8E), ARG2 CURRENT; the dot product is
//            replicated to all four channels;
//   alpha test GREATER 0 (0x742200). A texture without alpha_blending
//   (UI texture +0x80 != 2, 0x73F660) is drawn with blending off
//   (0x742CC0); an alpha-blended one blends SRCALPHA / INVSRCALPHA
//   (0x742630), its alpha being the dot product too.
// Direct3D works on the gamma-encoded texel, so the shader goes back to sRGB
// first.
#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_render::color_operations::{linear_to_srgb, srgb_to_linear}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var texture_sampler: sampler;
// x: 1 for an alpha-blended texture
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> params: vec4<u32>;

const TFACTOR: f32 = 127.0 / 255.0;
const WEIGHTS: vec3<f32> = vec3<f32>(133.0 / 255.0, 168.0 / 255.0, 142.0 / 255.0);

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let t = linear_to_srgb(textureSample(texture, texture_sampler, mesh.uv).rgb);
    let current = clamp(vec3<f32>(TFACTOR) + t * TFACTOR, vec3<f32>(0.0), vec3<f32>(1.0));
    let grey = clamp(4.0 * dot(WEIGHTS - vec3<f32>(0.5), current - vec3<f32>(0.5)), 0.0, 1.0);
    if grey <= 0.0 {
        discard;
    }
    let alpha = select(1.0, grey, params.x != 0u);
    var out = vec4<f32>(srgb_to_linear(vec3<f32>(grey)), alpha);
#ifdef SRGB_OUTPUT
    out = vec4<f32>(linear_to_srgb(out.rgb), out.a);
#endif
    return out;
}
