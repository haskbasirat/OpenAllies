// Effect modes 1-3 of the original's UI textures (UI texture +0x74, set by
// 0x73EAF0 from 0x5E66E0 / 0x5E6740): the Direct3D 9 fixed-function stages
// that 0x742CC0 (blending off) and 0x742630 (blended) set for them.
//
// Mode 1, a disabled button icon (the GrayscaleShader 0x73B6B0 sets the same
// stages):
//   TEXTUREFACTOR 0x7F7F7F;
//   stage 0: COLOROP MULTIPLYADD, ARG0 TFACTOR, ARG1 TEXTURE, ARG2 TFACTOR
//            (TFACTOR + TEXTURE * TFACTOR);
//   stage 1: COLOROP DOTPRODUCT3, ARG1 TEXTURE (the renderer's 2x2 RGBA32
//            image filled with 0x008EA885 in 0x71032E, DAT_00B40698:
//            R 0x85, G 0xA8, B 0x8E), ARG2 CURRENT; the dot product is
//            replicated to all four channels.
// Mode 2, a tinted icon (the widget's +0x101 flag, 0x5E6740; the tint is
// the texture's +0x30 colour, 0x73EB30, drawn as the vertices' specular):
//   stage 0: COLOROP BLENDTEXTUREALPHA, ARG1 TEXTURE, ARG2 SPECULAR
//            (texture over the tint by the texture's alpha), ALPHAOP
//            DISABLE (the diffuse alpha goes on);
//   stage 1: COLOROP MODULATE, ARG1 CURRENT, ARG2 DIFFUSE.
// Mode 3, tinted and disabled (with at least 4 texture stages, else mode 1):
//   the two mode-2 stages, then the two mode-1 stages on CURRENT
//   (stage 2 MULTIPLYADD CURRENT * TFACTOR + TFACTOR, stage 3 DOTPRODUCT3
//   with the weights image).
// The grey modes test alpha GREATER 0 (0x742200), their alpha being the dot
// product; a texture without alpha_blending (UI texture +0x80 != 2,
// 0x73F660) is drawn with blending off (0x742CC0), an alpha-blended one
// blends SRCALPHA / INVSRCALPHA (0x742630).
// Direct3D works on gamma-encoded values, so the shader goes back to sRGB
// first.
#import bevy_sprite::mesh2d_vertex_output::VertexOutput
#import bevy_render::color_operations::{linear_to_srgb, srgb_to_linear}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var texture_sampler: sampler;
// x: 1 for an alpha-blended texture; y: the effect mode (1, 2, 3);
// z: the tint (modes 2 and 3) as R | G << 8 | B << 16
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> params: vec4<u32>;

const TFACTOR: f32 = 127.0 / 255.0;
const WEIGHTS: vec3<f32> = vec3<f32>(133.0 / 255.0, 168.0 / 255.0, 142.0 / 255.0);

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(texture, texture_sampler, mesh.uv);
    let t = linear_to_srgb(texel.rgb);
    var diffuse = vec4<f32>(1.0);
#ifdef VERTEX_COLORS
    diffuse = vec4<f32>(linear_to_srgb(mesh.color.rgb), mesh.color.a);
#endif
    let blended = params.x != 0u;
    var c = t;
    if params.y >= 2u {
        let tint = vec3<f32>(f32(params.z & 0xffu), f32((params.z >> 8u) & 0xffu), f32((params.z >> 16u) & 0xffu)) / 255.0;
        c = mix(tint, t, texel.a) * diffuse.rgb;
    }
    var out: vec4<f32>;
    if params.y == 2u {
        // blending off draws it opaque (the vertex alpha is 1 there)
        out = vec4<f32>(srgb_to_linear(c), select(1.0, diffuse.a, blended));
    } else {
        let current = clamp(vec3<f32>(TFACTOR) + c * TFACTOR, vec3<f32>(0.0), vec3<f32>(1.0));
        let grey = clamp(4.0 * dot(WEIGHTS - vec3<f32>(0.5), current - vec3<f32>(0.5)), 0.0, 1.0);
        if grey <= 0.0 {
            discard;
        }
        out = vec4<f32>(srgb_to_linear(vec3<f32>(grey)), select(1.0, grey, blended));
    }
#ifdef SRGB_OUTPUT
    out = vec4<f32>(linear_to_srgb(out.rgb), out.a);
#endif
    return out;
}
