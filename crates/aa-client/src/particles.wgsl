// Effect particles and sprite quads (original Gamebryo effect textures).
// One instanced draw, premultiplied-alpha blending: alpha-blended and
// additive particles share the pipeline and the camera-sorted order.
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
@group(1) @binding(0) var atlas: texture_2d_array<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

struct Instance {
    @location(0) position: vec3<f32>,
    @location(1) size: f32,
    @location(2) color: vec4<f32>,
    // x: texture layer, y: rotation, z: blend (0 alpha, 1 additive, 2 one/one), w: mode
    @location(3) params: vec4<f32>,
    @location(4) c0: vec4<f32>,
    @location(5) c1: vec4<f32>,
    @location(6) c2: vec4<f32>,
    @location(7) c3: vec4<f32>,
    @location(8) cv: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) layer: u32,
    @location(3) @interpolate(flat) blend: f32,
    @location(4) world: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VertexOutput {
    var order = array<u32, 6>(0u, 1u, 2u, 1u, 3u, 2u);
    let k = order[vi % 6u];
    var corners = array<vec4<f32>, 4>(inst.c0, inst.c1, inst.c2, inst.c3);
    var vs = array<f32, 4>(inst.cv.x, inst.cv.y, inst.cv.z, inst.cv.w);
    let vp = camera.view_projection * camera.model_root;
    let local_eye=(camera.root_inverse*camera.eye).xyz;
    // Camera basis from the view-projection rows (perspective_rh * look_at_rh).
    let right = normalize(vec3<f32>(vp[0][0], vp[1][0], vp[2][0]));
    let up = normalize(vec3<f32>(vp[0][1], vp[1][1], vp[2][1]));
    let back = normalize(cross(right, up));
    let mode = u32(inst.params.w + 0.5);
    let fx = f32(k & 1u);
    let fy = f32(k >> 1u);
    var world = inst.position;
    var uv = vec2<f32>(fx, 1.0 - fy);
    switch mode {
        case 0u: {
            // camera-facing particle, rotated in the view plane
            let sx = fx * 2.0 - 1.0;
            let sy = fy * 2.0 - 1.0;
            let c = cos(inst.params.y);
            let s = sin(inst.params.y);
            let o = vec2<f32>(c * sx - s * sy, s * sx + c * sy) * inst.size;
            world = inst.position + right * o.x + up * o.y;
        }
        case 1u: {
            let c = corners[k];
            world = inst.position + c.xyz;
            uv = vec2<f32>(c.w, vs[k]);
        }
        case 2u: {
            // streak from position (head) back to position + c0 (tail)
            let tail = inst.position + inst.c0.xyz;
            let to_cam = normalize(local_eye - inst.position);
            var dir = inst.c0.xyz;
            if dot(dir, dir) < 1e-8 {
                dir = right;
            }
            var side = cross(dir, to_cam);
            if dot(side, side) < 1e-10 {
                side = up;
            }
            side = normalize(side) * inst.size;
            let along = mix(tail, inst.position, fx);
            world = along + side * (fy * 2.0 - 1.0);
            uv = vec2<f32>(fx, 1.0 - fy);
        }
        case 3u: {
            let c = corners[k];
            world = inst.position + right * c.x + up * c.y + back * c.z;
            uv = vec2<f32>(c.w, vs[k]);
        }
        default: {
            // billboard rotating about the world up axis
            var h = local_eye - inst.position;
            h.y = 0.0;
            var back_h = back;
            if dot(h, h) > 1e-6 {
                back_h = normalize(h);
            }
            let up_w = vec3<f32>(0.0, 1.0, 0.0);
            let right_h = cross(up_w, back_h);
            let c = corners[k];
            world = inst.position + right_h * c.x + up_w * c.y + back_h * c.z;
            uv = vec2<f32>(c.w, vs[k]);
        }
    }
    var out: VertexOutput;
    out.clip = camera.view_projection * camera.model_root * vec4<f32>(world, 1.0);
    out.uv = uv;
    out.color = inst.color;
    out.layer = u32(max(inst.params.x, 0.0) + 0.5);
    out.blend = inst.params.z;
    out.world = world;
    return out;
}

@fragment
fn fs_main(v: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(atlas, atlas_sampler, v.uv, v.layer);
    var rgb = tex.rgb * v.color.rgb;
    let a = clamp(tex.a * v.color.a, 0.0, 1.0);
    if camera.fog.z > 0.5 {
        let fog_uv = vec2<f32>(v.world.x * camera.fog.x, -v.world.z * camera.fog.y);
        rgb = rgb * textureSampleLevel(fog_texture, fog_sampler, fog_uv, 0.0).r;
    }
    var out: vec4<f32>;
    if v.blend < 0.5 {
        out = vec4<f32>(rgb * a, a);
    } else if v.blend < 1.5 {
        out = vec4<f32>(rgb * a, 0.0);
    } else {
        out = vec4<f32>(rgb, 0.0);
    }
    if max(max(out.r, out.g), max(out.b, out.a)) < 0.002 {
        discard;
    }
    return out;
}
