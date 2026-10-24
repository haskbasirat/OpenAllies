//! Animated original model effects: the waving flags (keyframed bones
//! skinning the cloth) and the flying death debris (keyframed pieces),
//! played from the NIF's own NiKeyframeController tracks and NiSkinInstance
//! weights, read with the ported NiStream reader. Like battle_view's unit
//! frames, every model is baked once at 20 Hz into per-frame vertex sets
//! (shared by all its instances); the meshes keep the converted model's
//! UVs, indices and audited materials.
//! Skinning follows aa-animation's proven rule: G(mesh) * skin offset *
//! G(skeleton root)^-1 * G(bone) * bone offset.
use super::extract;
use super::nif::Node;
use aa_formats::nif::{NifFile, Value};
use bevy::math::{Mat3, Mat4, Vec3};

pub const RATE: f32 = 20.0;

#[allow(dead_code)]
pub struct BakedPart {
    /// Index into the converted model's meshes.
    pub mesh: usize,
    /// Per frame: render-space positions and normals (None: static part).
    pub frames: Option<Vec<(Vec<[f32; 3]>, Vec<[f32; 3]>)>>,
}

#[allow(dead_code)]
pub struct Baked {
    pub parts: Vec<BakedPart>,
    pub frames: usize,
    pub looping: bool,
    pub duration: f32,
}

fn to_render(v: Vec3) -> [f32; 3] {
    [v.x, v.z, -v.y]
}

/// Node globals (NIF space) at time `t`.
fn globals(nodes: &[Node], t: f32) -> Vec<Mat4> {
    let mut out: Vec<Mat4> = Vec::with_capacity(nodes.len());
    for (i, n) in nodes.iter().enumerate() {
        let local = n.local(t);
        let g = if n.p >= 0 && (n.p as usize) < i { out[n.p as usize] * local } else { local };
        out.push(g);
    }
    out
}

struct Geo {
    node: usize,
    verts: Vec<Vec3>,
    normals: Vec<Vec3>,
    /// (bone node, bone offset), the skin offset and skeleton root node.
    skin: Option<(Vec<(Option<usize>, Mat4)>, Mat4, Option<usize>, Vec<Vec<(usize, f32)>>)>,
}

fn vec3s(w: Option<&[u32]>) -> Vec<Vec3> {
    w.map(|w| w.chunks_exact(3).map(|c| Vec3::new(f32::from_bits(c[0]), f32::from_bits(c[1]), f32::from_bits(c[2]))).collect()).unwrap_or_default()
}

/// Bake a model NIF against its converted meshes (`(name, vertex count)` in
/// the converted order). None when nothing in it is animated.
pub fn bake(file: &NifFile, converted: &[(String, usize)]) -> Option<Baked> {
    let scene = extract::scene(file);
    let animated_nodes = scene.nodes.iter().any(|n| n.kf.is_some());
    if !animated_nodes || scene.duration <= 0.0 {
        return None;
    }
    let target = |f: &aa_formats::nif::Fields, name: &str| f.link(name).and_then(|l| file.target(l)).filter(|&i| i < file.objects.len());
    // The NIF's drawn geometry, by name.
    let mut geos: Vec<(String, Geo)> = Vec::new();
    for &(obj, node) in &scene.geometry {
        let o = &file.objects[obj];
        let Some(d) = target(&o.fields, "data") else { continue };
        let df = &file.objects[d].fields;
        let verts = vec3s(df.f32s("vertices"));
        let normals = vec3s(df.f32s("normals"));
        let mut skin = None;
        if let Some(si) = target(&o.fields, "skin_instance")
            && let Some(sd) = target(&file.objects[si].fields, "data")
        {
            let sif = &file.objects[si].fields;
            let sdf = &file.objects[sd].fields;
            let offset = extract::skin_transform(sdf.f32s("root_transform").unwrap_or(&[]));
            let root = target(sif, "root_parent").and_then(|r| scene.node_of.get(&r).copied());
            let bone_nodes: Vec<Option<usize>> = sif.links("bones").unwrap_or(&[]).iter().map(|l| file.target(*l).and_then(|b| scene.node_of.get(&b).copied())).collect();
            let mut bones = Vec::new();
            let mut weights = vec![Vec::new(); verts.len()];
            for (bi, b) in sdf.list("bone_list").unwrap_or(&[]).iter().enumerate() {
                let Value::Record(b) = b else { continue };
                let b = aa_formats::nif::Fields(b.to_vec());
                bones.push((bone_nodes.get(bi).copied().flatten(), extract::skin_transform(b.f32s("transform").unwrap_or(&[]))));
                if let (Some(ix), Some(ws)) = (b.u16s("vertex_indices"), b.f32s("vertex_weights")) {
                    for (i, w) in ix.iter().zip(ws) {
                        if let Some(slot) = weights.get_mut(*i as usize) {
                            slot.push((bi, f32::from_bits(*w)));
                        }
                    }
                }
            }
            skin = Some((bones, offset, root, weights));
        }
        let name = o.fields.string("name").flatten().map(|b| String::from_utf8_lossy(b).into_owned()).unwrap_or_default();
        geos.push((name, Geo { node, verts, normals, skin }));
    }
    let looping = scene.nodes.iter().filter_map(|n| n.kf.as_ref()).any(|k| k.timing.cycle == 0);
    let duration = scene.duration;
    let frames = if looping { ((duration * RATE).round() as usize).max(1) } else { (duration * RATE).ceil() as usize + 1 };
    let poses: Vec<Vec<Mat4>> = (0..frames).map(|f| globals(&scene.nodes, f as f32 / RATE)).collect();
    let mut used = vec![false; geos.len()];
    let mut parts = Vec::new();
    for (mi, (name, count)) in converted.iter().enumerate() {
        let found = geos.iter().enumerate().position(|(gi, (n, g))| !used[gi] && n.eq_ignore_ascii_case(name) && g.verts.len() == *count);
        let Some(gi) = found else {
            parts.push(BakedPart { mesh: mi, frames: None });
            continue;
        };
        used[gi] = true;
        let g = &geos[gi].1;
        let mut out = Vec::with_capacity(frames);
        for pose in &poses {
            let mut ps = Vec::with_capacity(g.verts.len());
            let mut ns = Vec::with_capacity(g.verts.len());
            match &g.skin {
                Some((bones, offset, root, weights)) => {
                    let g_root = root.map_or(Mat4::IDENTITY, |r| pose[r]);
                    let base = pose[g.node] * *offset * if g_root.determinant().abs() > 1e-12 { g_root.inverse() } else { Mat4::IDENTITY };
                    let mats: Vec<Mat4> = bones.iter().map(|(b, off)| base * b.map_or(Mat4::IDENTITY, |b| pose[b]) * *off).collect();
                    for (vi, v) in g.verts.iter().enumerate() {
                        let mut p = Vec3::ZERO;
                        let mut n = Vec3::ZERO;
                        let src_n = g.normals.get(vi).copied().unwrap_or(Vec3::Z);
                        let ws = &weights[vi];
                        if ws.is_empty() {
                            p = pose[g.node].transform_point3(*v);
                            n = Mat3::from_mat4(pose[g.node]) * src_n;
                        }
                        for &(bi, w) in ws {
                            let m = mats[bi];
                            p += m.transform_point3(*v) * w;
                            n += Mat3::from_mat4(m) * src_n * w;
                        }
                        ps.push(to_render(p));
                        ns.push(to_render(n.normalize_or(Vec3::Z)));
                    }
                }
                None => {
                    let m = pose[g.node];
                    let r = Mat3::from_mat4(m);
                    for (vi, v) in g.verts.iter().enumerate() {
                        ps.push(to_render(m.transform_point3(*v)));
                        ns.push(to_render((r * g.normals.get(vi).copied().unwrap_or(Vec3::Z)).normalize_or(Vec3::Z)));
                    }
                }
            }
            out.push((ps, ns));
        }
        parts.push(BakedPart { mesh: mi, frames: Some(out) });
    }
    parts.iter().any(|p| p.frames.is_some()).then_some(Baked { parts, frames, looping, duration })
}

/// The frame index shown `age` seconds after the start.
pub fn frame_at(b_frames: usize, looping: bool, age: f32) -> usize {
    let f = (age.max(0.0) * RATE).floor() as usize;
    if looping { f % b_frames.max(1) } else { f.min(b_frames.saturating_sub(1)) }
}
