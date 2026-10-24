//! The effect table (`nif::NifTable`) built at run time from the original
//! effect NIFs with the ported NiStream reader (`aa_formats::nif`), so the
//! effects work from any checkout with the user's install. A port of the
//! wgpu client's `vfx/extract_nif_effects.py` (the same walk, fields and
//! output layout): node hierarchy with keyframe and visibility tracks,
//! billboard modes, blended sprite geometry paired into quads (rigid
//! skinning per quad), material alpha/colour/flip controllers, NiPSys
//! emitters and modifiers, and old-style NiParticleSystemController
//! particles. Embedded textures (NiPixelData) are decoded to PNG files.
//! No Bevy types beyond glam (also built by a standalone probe).
use super::nif::*;
use aa_formats::nif::{Field, Fields, NifFile, NifObject, Value, is_kind_of};
use bevy::math::{Mat3, Mat4, Quat, Vec3};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn fl(w: u32) -> f32 {
    f32::from_bits(w)
}
fn floats(w: &[u32]) -> Vec<f32> {
    w.iter().map(|&x| fl(x)).collect()
}
fn rec(fields: &[Field]) -> Fields {
    Fields(fields.to_vec())
}
fn str_field(f: &Fields, name: &str) -> String {
    f.string(name).flatten().map(|b| String::from_utf8_lossy(b).into_owned()).unwrap_or_default()
}

/// The column-vector rotation of a stored NiMatrix3: the nine words are
/// PyFFI's row-vector matrix in column order (m11 m21 m31 m12 ...), whose
/// rows become columns (the client extractor's and rig exporter's `rows()`),
/// so each stored triple is one row of the column-vector matrix.
pub fn rotation(w: &[u32]) -> Mat3 {
    if w.len() < 9 {
        return Mat3::IDENTITY;
    }
    Mat3::from_cols_array(&[fl(w[0]), fl(w[1]), fl(w[2]), fl(w[3]), fl(w[4]), fl(w[5]), fl(w[6]), fl(w[7]), fl(w[8])]).transpose()
}
/// NiTransform words (rotation 9, translation 3, scale) as a matrix.
fn transform(w: &[u32]) -> Mat4 {
    if w.len() < 13 {
        return Mat4::IDENTITY;
    }
    let r = rotation(&w[0..9]);
    let t = Vec3::new(fl(w[9]), fl(w[10]), fl(w[11]));
    let s = fl(w[12]);
    Mat4::from_cols((r.x_axis * s).extend(0.0), (r.y_axis * s).extend(0.0), (r.z_axis * s).extend(0.0), t.extend(1.0))
}
fn quat_cols(w: f32, x: f32, y: f32, z: f32) -> Mat3 {
    Mat3::from_quat(Quat::from_xyzw(x, y, z, w).normalize())
}

/// Effect textures of the whole table: keys and their layer order.
#[derive(Default)]
pub struct Textures {
    pub keys: Vec<String>,
    index: HashMap<String, i32>,
    embedded: HashMap<u64, String>,
    /// Lower-case basename -> Data-relative paths of every texture file.
    by_name: HashMap<String, Vec<String>>,
    files: HashMap<String, String>,
    pub out_dir: PathBuf,
}

const TEXTURE_EXT: [&str; 5] = ["dds", "tga", "png", "jpg", "bmp"];

impl Textures {
    pub fn new(data_root: &Path, out_dir: PathBuf) -> Self {
        let mut t = Self { out_dir, ..Self::default() };
        let mut stack = vec![data_root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("").to_ascii_lowercase();
                if !TEXTURE_EXT.contains(&ext.as_str()) {
                    continue;
                }
                let Ok(rel) = p.strip_prefix(data_root) else { continue };
                let rel = rel.to_string_lossy().replace('\\', "/");
                let name = rel.rsplit('/').next().unwrap_or("").to_ascii_lowercase();
                t.files.insert(rel.to_ascii_lowercase(), rel.clone());
                t.by_name.entry(name).or_default().push(rel);
            }
        }
        t
    }

    fn intern(&mut self, key: String) -> i32 {
        if let Some(&i) = self.index.get(&key) {
            return i;
        }
        let i = self.keys.len() as i32;
        self.index.insert(key.clone(), i);
        self.keys.push(key);
        i
    }

    /// The NIF's texture file: next to the NIF, in the VFX texture search
    /// path (/Shared/SharedTextures), else the file of that name sharing
    /// the longest folder prefix with the NIF (the client converter's rule).
    fn resolve(&self, nif_rel: &str, file: &str) -> Option<String> {
        let raw = file.replace('\\', "/").trim_start_matches('/').to_owned();
        if raw.is_empty() {
            return None;
        }
        let folder = nif_rel.rsplit_once('/').map_or("", |(d, _)| d);
        let base = raw.rsplit('/').next().unwrap_or(&raw).to_ascii_lowercase();
        for cand in [format!("{folder}/{raw}"), raw.clone(), format!("Shared/SharedTextures/{base}")] {
            if let Some(f) = self.files.get(&cand.to_ascii_lowercase()) {
                return Some(f.clone());
            }
        }
        let list = self.by_name.get(&base)?;
        let parts: Vec<String> = nif_rel.to_ascii_lowercase().split('/').map(str::to_owned).collect();
        list.iter()
            .min_by_key(|p| {
                let pp: Vec<String> = p.to_ascii_lowercase().split('/').map(str::to_owned).collect();
                let common = parts.iter().zip(&pp).take_while(|(a, b)| a == b).count();
                (std::cmp::Reverse(common), pp.len(), p.to_ascii_lowercase())
            })
            .cloned()
    }

    fn source(&mut self, file: &NifFile, nif_rel: &str, source: Option<usize>, tag: &mut u32) -> i32 {
        let Some(o) = source.map(|i| &file.objects[i]) else { return -1 };
        *tag += 1;
        if o.fields.u8("use_external").unwrap_or(1) != 0 {
            let name = str_field(&o.fields, "file_name");
            return match self.resolve(nif_rel, &name) {
                Some(rel) => self.intern(rel),
                None => -1,
            };
        }
        let Some(pix) = o.fields.link("pixel_data").and_then(|l| file.target(l)).map(|i| &file.objects[i]) else { return -1 };
        let Some(img) = decode_pixels(&pix.fields) else { return -1 };
        let digest = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            img.as_raw().hash(&mut h);
            h.finish()
        };
        if let Some(k) = self.embedded.get(&digest).cloned() {
            return self.intern(k);
        }
        let stem: String = nif_rel
            .trim_start_matches("Effects/")
            .rsplit_once('.')
            .map_or(nif_rel, |(s, _)| s)
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
            .collect();
        let name = format!("{stem}_{tag}.png");
        let _ = std::fs::create_dir_all(&self.out_dir);
        if img.save(self.out_dir.join(&name)).is_err() {
            return -1;
        }
        let key = format!("embedded:{name}");
        self.embedded.insert(digest, key.clone());
        self.intern(key)
    }
}

/// The first mip level of an NiPixelData: RGB8 (0), RGBA8 (1), DXT1/3/5 (4/5/6).
fn decode_pixels(f: &Fields) -> Option<image::RgbaImage> {
    let mips = match f.get("mipmaps")? {
        Value::U32s(v) => v,
        _ => return None,
    };
    let pixels = match f.get("pixels")? {
        Value::Bytes(b) => b,
        _ => return None,
    };
    let (w, h, off) = (*mips.first()? as usize, *mips.get(1)? as usize, *mips.get(2)? as usize);
    let format = f.u32("format")?;
    let data = pixels.get(off..)?;
    let mut out = vec![0u8; w * h * 4];
    match format {
        1 => out.copy_from_slice(data.get(..w * h * 4)?),
        0 => {
            for (i, px) in data.get(..w * h * 3)?.chunks_exact(3).enumerate() {
                out[i * 4..i * 4 + 4].copy_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
        4..=6 => dxt(format, data, w, h, &mut out)?,
        _ => return None,
    }
    image::RgbaImage::from_raw(w as u32, h as u32, out)
}

fn rgb565(c: u16) -> [u8; 3] {
    let r = ((c >> 11) & 31) as u32;
    let g = ((c >> 5) & 63) as u32;
    let b = (c & 31) as u32;
    [((r * 255 + 15) / 31) as u8, ((g * 255 + 31) / 63) as u8, ((b * 255 + 15) / 31) as u8]
}

/// BC1/BC2/BC3 block decoding.
fn dxt(format: u32, data: &[u8], w: usize, h: usize, out: &mut [u8]) -> Option<()> {
    let block = if format == 4 { 8 } else { 16 };
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    for by in 0..bh {
        for bx in 0..bw {
            let at = (by * bw + bx) * block;
            let b = data.get(at..at + block)?;
            let (alpha, color) = if format == 4 { (None, b) } else { (Some(&b[..8]), &b[8..]) };
            let c0 = u16::from_le_bytes([color[0], color[1]]);
            let c1 = u16::from_le_bytes([color[2], color[3]]);
            let (p0, p1) = (rgb565(c0), rgb565(c1));
            let mut pal = [[0u8; 4]; 4];
            pal[0] = [p0[0], p0[1], p0[2], 255];
            pal[1] = [p1[0], p1[1], p1[2], 255];
            let mix = |a: u8, b: u8, wa: u32, wb: u32, d: u32| ((a as u32 * wa + b as u32 * wb) / d) as u8;
            if c0 > c1 || format != 4 {
                for k in 0..3 {
                    pal[2][k] = mix(p0[k], p1[k], 2, 1, 3);
                    pal[3][k] = mix(p0[k], p1[k], 1, 2, 3);
                }
                pal[2][3] = 255;
                pal[3][3] = 255;
            } else {
                for k in 0..3 {
                    pal[2][k] = mix(p0[k], p1[k], 1, 1, 2);
                }
                pal[2][3] = 255;
                pal[3] = [0, 0, 0, 0];
            }
            let bits = u32::from_le_bytes([color[4], color[5], color[6], color[7]]);
            // Alpha of BC2 (explicit 4-bit) and BC3 (interpolated).
            let mut alphas = [255u8; 16];
            if let Some(a) = alpha {
                if format == 5 {
                    for i in 0..16 {
                        let v = (a[i / 2] >> ((i % 2) * 4)) & 15;
                        alphas[i] = v * 17;
                    }
                } else {
                    let (a0, a1) = (a[0] as u32, a[1] as u32);
                    let mut ap = [0u32; 8];
                    ap[0] = a0;
                    ap[1] = a1;
                    if a0 > a1 {
                        for k in 1..7 {
                            ap[k + 1] = ((7 - k as u32) * a0 + k as u32 * a1) / 7;
                        }
                    } else {
                        for k in 1..5 {
                            ap[k + 1] = ((5 - k as u32) * a0 + k as u32 * a1) / 5;
                        }
                        ap[6] = 0;
                        ap[7] = 255;
                    }
                    let mut abits = 0u64;
                    for k in 0..6 {
                        abits |= (a[2 + k] as u64) << (8 * k);
                    }
                    for (i, slot) in alphas.iter_mut().enumerate() {
                        *slot = ap[((abits >> (3 * i)) & 7) as usize] as u8;
                    }
                }
            }
            for i in 0..16 {
                let (x, y) = (bx * 4 + i % 4, by * 4 + i / 4);
                if x >= w || y >= h {
                    continue;
                }
                let mut c = pal[((bits >> (2 * i)) & 3) as usize];
                if alpha.is_some() {
                    c[3] = alphas[i];
                }
                out[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&c);
            }
        }
    }
    Some(())
}

/// The output rows of a key list: [t, value..., (forward..., backward...)].
fn key_group(keys: Option<&[Value]>, key_type: u32, n: usize) -> KeyGroup {
    let interp = if keys.is_some_and(|k| !k.is_empty()) { key_type as u8 } else { 1 };
    let mut out = Vec::new();
    for k in keys.unwrap_or(&[]) {
        let Value::F32s(w) = k else { continue };
        let w = floats(w);
        if w.len() < 1 + n {
            continue;
        }
        let mut row = w[..1 + n].to_vec();
        if interp == 2 && w.len() >= 1 + 3 * n {
            row.extend_from_slice(&w[1 + n..1 + 3 * n]);
        }
        out.push(row);
    }
    KeyGroup { i: interp, k: out }
}

fn keys_of<'a>(f: &'a Fields, name: &str) -> Option<&'a [Value]> {
    f.list(name)
}

fn timing(c: &Fields) -> Timing {
    Timing {
        start: fl(c.f32("start").unwrap_or(0)),
        stop: fl(c.f32("stop").unwrap_or(0)),
        freq: fl(c.f32("frequency").unwrap_or(1.0f32.to_bits())),
        phase: fl(c.f32("phase").unwrap_or(0)),
        cycle: ((c.u16("flags").unwrap_or(0) >> 1) & 3) as u8,
    }
}

fn bool_keys(f: &Fields, name: &str) -> Vec<[f32; 2]> {
    f.list(name)
        .unwrap_or(&[])
        .iter()
        .filter_map(|v| match v {
            Value::Record(r) => {
                let r = rec(r);
                Some([fl(r.f32("time")?), r.u8("value")? as f32])
            }
            _ => None,
        })
        .collect()
}

pub struct Extract<'a> {
    file: &'a NifFile,
    nif_rel: String,
    textures: &'a mut Textures,
    nodes: Vec<Node>,
    node_index: HashMap<usize, usize>,
    sprites: Vec<Sprite>,
    emitters: Vec<Emitter>,
    duration: f32,
    tag: u32,
    psys: Vec<(usize, HashMap<&'static str, usize>, usize)>,
    /// Scene walk only (animated models): every drawn geometry object and
    /// its node, instead of sprite extraction.
    geometry: Option<Vec<(usize, usize)>>,
}

impl<'a> Extract<'a> {
    fn obj(&self, i: usize) -> &'a NifObject {
        &self.file.objects[i]
    }
    fn target(&self, f: &Fields, name: &str) -> Option<usize> {
        f.link(name).and_then(|l| self.file.target(l)).filter(|&i| i < self.file.objects.len())
    }
    fn ctrl_duration(&mut self, c: &Fields) {
        self.duration = self.duration.max(fl(c.f32("stop").unwrap_or(0)));
    }
    /// The controller chain from `controller`.
    fn controllers(&self, f: &Fields) -> Vec<usize> {
        let mut out = Vec::new();
        let mut c = self.target(f, "controller");
        while let Some(i) = c {
            if out.contains(&i) || out.len() > 64 {
                break;
            }
            out.push(i);
            c = self.target(&self.obj(i).fields, "next");
        }
        out
    }

    fn keyframe_track(&self, data: &Fields, bind: Mat3) -> Option<Keyframes> {
        let mut k = Keyframes::default();
        if data.u32("translation_count").unwrap_or(0) > 0 {
            k.t = Some(key_group(keys_of(data, "translations"), data.u32("translation_type").unwrap_or(1), 3));
        }
        if data.u32("scale_count").unwrap_or(0) > 0 {
            k.s = Some(key_group(keys_of(data, "scales"), data.u32("scale_type").unwrap_or(1), 1));
        }
        let rcount = data.u32("rotation_count").unwrap_or(0);
        let rtype = data.u32("rotation_type").unwrap_or(1);
        if rcount > 0 && rtype != 4 {
            let mut rows = Vec::new();
            for v in keys_of(data, "rotations").unwrap_or(&[]) {
                let Value::F32s(w) = v else { continue };
                let w = floats(w);
                if w.len() >= 5 {
                    rows.push((w[0], w[1], w[2], w[3], w[4]));
                }
            }
            if let Some(&(_, w, x, y, z)) = rows.first() {
                // The stored convention against the bind rotation at the first key.
                let err = |m: Mat3| (m.x_axis - bind.x_axis).abs().element_sum() + (m.y_axis - bind.y_axis).abs().element_sum() + (m.z_axis - bind.z_axis).abs().element_sum();
                let sign = if err(quat_cols(w, -x, -y, -z)) + 1e-4 < err(quat_cols(w, x, y, z)) { -1.0 } else { 1.0 };
                k.q = Some(KeyGroup { i: 1, k: rows.iter().map(|&(t, w, x, y, z)| vec![t, sign * x, sign * y, sign * z, w]).collect() });
            }
        } else if rcount > 0 {
            // XYZ Euler float tracks.
            if let Some(Value::Record(r)) = keys_of(data, "rotations").and_then(|l| l.first()) {
                let r = rec(r);
                let axes = r.list("axes").unwrap_or(&[]);
                let mut groups = Vec::new();
                for a in axes {
                    let Value::Record(a) = a else { continue };
                    let a = rec(a);
                    groups.push(key_group(a.list("keys"), a.u32("type").unwrap_or(1), 1));
                }
                if !groups.is_empty() {
                    k.e = Some(groups);
                }
            }
        }
        (k.t.is_some() || k.q.is_some() || k.s.is_some() || k.e.is_some()).then_some(k)
    }

    fn add_node(&mut self, i: usize, parent: i32) -> usize {
        let o = self.obj(i);
        let f = &o.fields;
        let idx = self.nodes.len();
        self.node_index.insert(i, idx);
        let t = f.f32s("translation").map(floats).filter(|v| v.len() >= 3).map_or([0.0; 3], |v| [v[0], v[1], v[2]]);
        let rot = rotation(f.f32s("rotation").unwrap_or(&[]));
        let q = Quat::from_mat3(&rot).normalize();
        let mut node = Node {
            name: str_field(f, "name"),
            p: parent,
            t,
            q: [q.x, q.y, q.z, q.w],
            s: fl(f.f32("scale").unwrap_or(1.0f32.to_bits())),
            bb: None,
            kf: None,
            vis: None,
            hidden: false,
        };
        let raw_flags = match f.field("flags") {
            Some(Field { read: Some(Value::U16(r)), .. }) => *r,
            _ => f.u16("flags").unwrap_or(0),
        };
        if is_kind_of(o.class, "NiBillboardNode") {
            node.bb = Some(if self.file.version >= 0x0a01_0000 { f.u16("billboard_mode").unwrap_or(0) as u8 } else { ((raw_flags >> 5) & 3) as u8 });
        }
        for c in self.controllers(f) {
            let co = self.obj(c);
            let cf = &co.fields;
            if is_kind_of(co.class, "NiKeyframeController") {
                if let Some(d) = self.target(cf, "data")
                    && let Some(mut track) = self.keyframe_track(&self.obj(d).fields, rot)
                {
                    track.timing = timing(cf);
                    node.kf = Some(track);
                    self.ctrl_duration(cf);
                }
            } else if is_kind_of(co.class, "NiVisController")
                && let Some(d) = self.target(cf, "data")
            {
                node.vis = Some(VisTrack { timing: timing(cf), k: bool_keys(&self.obj(d).fields, "keys") });
                self.ctrl_duration(cf);
            }
        }
        if raw_flags & 1 != 0 {
            node.hidden = true;
        }
        self.nodes.push(node);
        idx
    }

    fn walk(&mut self, i: usize, parent: i32, props: &HashMap<&'static str, usize>, hidden: bool) {
        if self.node_index.contains_key(&i) {
            return;
        }
        let o = self.obj(i);
        let mut props = props.clone();
        for l in o.fields.links("properties").unwrap_or(&[]) {
            if let Some(p) = self.file.target(*l).filter(|&p| p < self.file.objects.len()) {
                props.insert(self.obj(p).class, p);
            }
        }
        let idx = self.add_node(i, parent);
        let flags = o.fields.u16("flags").unwrap_or(0);
        let hidden = hidden || flags & 1 != 0;
        let name = str_field(&o.fields, "name").to_ascii_lowercase();
        if is_kind_of(o.class, "NiParticles") {
            self.psys.push((i, props, idx));
            return;
        }
        if is_kind_of(o.class, "NiTriBasedGeom") {
            if let Some(g) = self.geometry.as_mut() {
                if !hidden {
                    g.push((i, idx));
                }
                return;
            }
            if self.target(&o.fields, "data").is_some() && !hidden && !name.starts_with("selectionbox") && !name.starts_with("selection box") && !name.starts_with("collision") {
                self.sprite(i, &props, idx);
            }
            return;
        }
        let children: Vec<usize> = self.file.children.get(i).map(|c| c.iter().flatten().copied().collect()).unwrap_or_default();
        for c in children {
            if c < self.file.objects.len() && is_kind_of(self.obj(c).class, "NiAVObject") {
                self.walk(c, idx as i32, &props, hidden);
            }
        }
    }

    fn material(&mut self, props: &HashMap<&'static str, usize>) -> Material {
        let mut m = Material { color: [1.0; 3], emissive: [0.0; 3], alpha: 1.0, blend: 0, blended: false, tex: -1, vc_emissive: false, akeys: None, ckeys: None, flip: None, atest: None };
        if let Some(&p) = props.get("NiMaterialProperty") {
            let f = &self.obj(p).fields;
            let c3 = |n: &str| f.f32s(n).map(floats).filter(|v| v.len() >= 3).map_or([1.0; 3], |v| [v[0], v[1], v[2]]);
            m.color = c3("diffuse");
            m.emissive = f.f32s("emissive").map(floats).filter(|v| v.len() >= 3).map_or([0.0; 3], |v| [v[0], v[1], v[2]]);
            m.alpha = fl(f.f32("alpha").unwrap_or(1.0f32.to_bits()));
            for c in self.controllers(f) {
                let co = self.obj(c);
                let cf = &co.fields;
                if is_kind_of(co.class, "NiAlphaController") {
                    if let Some(d) = self.target(cf, "data") {
                        let df = &self.obj(d).fields;
                        m.akeys = Some(FloatTrack { timing: timing(cf), keys: key_group(df.list("keys"), df.u32("key_type").unwrap_or(1), 1) });
                        self.ctrl_duration(cf);
                    }
                } else if is_kind_of(co.class, "NiMaterialColorController")
                    && let Some(d) = self.target(cf, "link")
                {
                    let df = &self.obj(d).fields;
                    m.ckeys = Some(ColorTrack {
                        track: FloatTrack { timing: timing(cf), keys: key_group(df.list("keys"), df.u32("key_type").unwrap_or(1), 3) },
                        target: cf.u16("controller_flags").unwrap_or(0) as u8,
                    });
                    self.ctrl_duration(cf);
                }
            }
        }
        if let Some(&p) = props.get("NiAlphaProperty") {
            let f = &self.obj(p).fields;
            let flags = f.u16("flags").unwrap_or(0);
            let src = (flags >> 1) & 15;
            let dst = (flags >> 5) & 15;
            if flags & 1 != 0 {
                m.blended = true;
                m.blend = if dst == 0 && src == 0 {
                    2
                } else if dst == 0 {
                    1
                } else {
                    0
                };
            }
            if flags & 512 != 0 {
                m.atest = Some(f.u8("threshold").unwrap_or(0) as f32 / 255.0);
            }
        }
        if let Some(&p) = props.get("NiVertexColorProperty") {
            m.vc_emissive = self.obj(p).fields.u32("vertex_mode") == Some(1);
        }
        if let Some(&p) = props.get("NiTexturingProperty") {
            let f = &self.obj(p).fields;
            let base = f.list("maps").and_then(|l| l.first()).and_then(|v| match v {
                Value::Record(r) => Some(rec(r)),
                _ => None,
            });
            if let Some(base) = base {
                let src = self.target(&base, "source");
                if src.is_some() {
                    let (file, rel) = (self.file, self.nif_rel.clone());
                    m.tex = self.textures.source(file, &rel, src, &mut self.tag);
                    for c in self.controllers(f) {
                        let co = self.obj(c);
                        let cf = &co.fields;
                        if is_kind_of(co.class, "NiFlipController") {
                            let mut frames = Vec::new();
                            for l in cf.links("sources").unwrap_or(&[]) {
                                let s = self.file.target(*l);
                                frames.push(self.textures.source(file, &rel, s, &mut self.tag));
                            }
                            m.flip = Some(Flip { timing: timing(cf), delta: fl(cf.f32("delta").unwrap_or(0)), frames });
                            self.ctrl_duration(cf);
                        } else if is_kind_of(co.class, "NiTextureTransformController") {
                            self.ctrl_duration(cf);
                        }
                    }
                }
            }
        }
        m
    }

    fn triangles(&self, data: &NifObject) -> Vec<[u16; 3]> {
        let f = &data.fields;
        let mut out = Vec::new();
        if let Some(t) = f.u16s("triangles") {
            for c in t.chunks_exact(3) {
                out.push([c[0], c[1], c[2]]);
            }
        } else if let (Some(lengths), Some(points)) = (f.u16s("strip_lengths"), f.u16s("points")) {
            let mut at = 0usize;
            for &len in lengths {
                let len = len as usize;
                let strip = points.get(at..at + len).unwrap_or(&[]);
                at += len;
                let mut j = false;
                for k in 2..strip.len() {
                    j = !j;
                    let (t0, t1, t2) = (strip[k - 2], strip[k - 1], strip[k]);
                    if t0 == t1 || t1 == t2 || t2 == t0 {
                        continue;
                    }
                    out.push(if j { [t0, t1, t2] } else { [t0, t2, t1] });
                }
            }
        }
        out.retain(|t| t[0] != t[1] && t[1] != t[2] && t[0] != t[2]);
        out
    }

    /// The world (root-space) transform of an extracted node.
    fn world(&self, obj: Option<usize>) -> Mat4 {
        let mut chain = Vec::new();
        let mut i = obj.and_then(|o| self.node_index.get(&o).copied()).map_or(-1, |i| i as i32);
        while i >= 0 {
            chain.push(i as usize);
            i = self.nodes[i as usize].p;
        }
        let mut m = Mat4::IDENTITY;
        for &n in chain.iter().rev() {
            let nd = &self.nodes[n];
            m *= Mat4::from_scale_rotation_translation(Vec3::splat(nd.s), Quat::from_xyzw(nd.q[0], nd.q[1], nd.q[2], nd.q[3]), Vec3::from(nd.t));
        }
        m
    }

    /// The transform of an object relative to the scene root (its parent
    /// chain through the linked parents, root excluded).
    fn to_root(&self, i: usize) -> Mat4 {
        let root = self.file.top_level.first().copied().flatten();
        let mut m = Mat4::IDENTITY;
        let mut cur = Some(i);
        let mut guard = 0;
        while let Some(c) = cur {
            if Some(c) == root || guard > 64 {
                break;
            }
            guard += 1;
            let f = &self.obj(c).fields;
            let t = f.f32s("translation").map(floats).filter(|v| v.len() >= 3).map_or(Vec3::ZERO, |v| Vec3::new(v[0], v[1], v[2]));
            let r = rotation(f.f32s("rotation").unwrap_or(&[]));
            let s = fl(f.f32("scale").unwrap_or(1.0f32.to_bits()));
            let local = Mat4::from_cols((r.x_axis * s).extend(0.0), (r.y_axis * s).extend(0.0), (r.z_axis * s).extend(0.0), t.extend(1.0));
            m = local * m;
            cur = self.file.parents.get(c).copied().flatten();
        }
        m
    }

    fn sprite(&mut self, geom: usize, props: &HashMap<&'static str, usize>, idx: usize) {
        let g = self.obj(geom);
        let Some(d) = self.target(&g.fields, "data") else { return };
        let data = self.obj(d);
        let verts: Vec<Vec3> = data.fields.f32s("vertices").map(|w| w.chunks_exact(3).map(|c| Vec3::new(fl(c[0]), fl(c[1]), fl(c[2]))).collect()).unwrap_or_default();
        if verts.is_empty() {
            return;
        }
        let mat = self.material(props);
        if !mat.blended {
            return;
        }
        if mat.alpha <= 0.0 && mat.akeys.is_none() && mat.ckeys.is_none() {
            return;
        }
        let n = verts.len();
        let uvs: Vec<[f32; 2]> = data.fields.f32s("uv_sets").map(|w| w.chunks_exact(2).take(n).map(|c| [fl(c[0]), fl(c[1])]).collect()).unwrap_or_default();
        let colors: Vec<[f32; 4]> = data.fields.f32s("colors").map(|w| w.chunks_exact(4).map(|c| [fl(c[0]), fl(c[1]), fl(c[2]), fl(c[3])]).collect()).unwrap_or_default();
        let tris = self.triangles(data);
        if tris.is_empty() {
            return;
        }
        // Pair triangles sharing an edge into quads (o1, e1, e2, o2).
        let mut edges: HashMap<(u16, u16), Vec<usize>> = HashMap::new();
        let key = |a: u16, b: u16| if a < b { (a, b) } else { (b, a) };
        for (ti, t) in tris.iter().enumerate() {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                edges.entry(key(a, b)).or_default().push(ti);
            }
        }
        let mut used = vec![false; tris.len()];
        let mut quads: Vec<[u16; 4]> = Vec::new();
        for ti in 0..tris.len() {
            if used[ti] {
                continue;
            }
            let t = tris[ti];
            let mut paired = false;
            'edges: for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                for &tj in &edges[&key(a, b)] {
                    if tj == ti || used[tj] {
                        continue;
                    }
                    let u = tris[tj];
                    let o1 = *t.iter().find(|v| **v != a && **v != b).unwrap_or(&t[0]);
                    let o2 = *u.iter().find(|v| **v != a && **v != b).unwrap_or(&u[0]);
                    quads.push([o1, a, b, o2]);
                    used[ti] = true;
                    used[tj] = true;
                    paired = true;
                    break 'edges;
                }
            }
            if !paired {
                used[ti] = true;
                quads.push([t[0], t[1], t[2], t[2]]);
            }
        }
        let uv = |i: u16| uvs.get(i as usize).copied().unwrap_or([0.0, 0.0]);
        let mut groups: Vec<(usize, Vec<Vec<f32>>)> = Vec::new();
        let push = |node: usize, row: Vec<f32>, groups: &mut Vec<(usize, Vec<Vec<f32>>)>| match groups.iter_mut().find(|(n, _)| *n == node) {
            Some((_, list)) => list.push(row),
            None => groups.push((node, vec![row])),
        };
        let skin = self.target(&g.fields, "skin_instance").and_then(|s| Some((s, self.target(&self.obj(s).fields, "data")?)));
        let mut post = None;
        if let Some((si, sd)) = skin {
            // Rigid skinning: each quad follows the bone of the largest weight.
            let sdata = &self.obj(sd).fields;
            let so = transform(sdata.f32s("root_transform").unwrap_or(&[]));
            let gw = self.to_root(geom);
            post = Some((gw * so).to_cols_array());
            let bones: Vec<Option<usize>> = self.obj(si).fields.links("bones").unwrap_or(&[]).iter().map(|l| self.file.target(*l)).collect();
            let mut offsets = Vec::new();
            let mut weights: HashMap<u16, Vec<(usize, f32)>> = HashMap::new();
            for (bi, b) in sdata.list("bone_list").unwrap_or(&[]).iter().enumerate() {
                let Value::Record(b) = b else { continue };
                let b = rec(b);
                offsets.push(transform(b.f32s("transform").unwrap_or(&[])));
                if let (Some(ix), Some(ws)) = (b.u16s("vertex_indices"), b.f32s("vertex_weights")) {
                    for (i, w) in ix.iter().zip(ws) {
                        weights.entry(*i).or_default().push((bi, fl(*w)));
                    }
                }
            }
            for q in &quads {
                let mut score: Vec<(usize, f32)> = Vec::new();
                for vi in q {
                    for &(bi, w) in weights.get(vi).map_or(&[][..], |v| v) {
                        match score.iter_mut().find(|(b, _)| *b == bi) {
                            Some(s) => s.1 += w,
                            None => score.push((bi, w)),
                        }
                    }
                }
                let Some(&(bi, _)) = score.iter().max_by(|a, b| a.1.total_cmp(&b.1)) else { continue };
                let Some(node) = bones.get(bi).copied().flatten().and_then(|b| self.node_index.get(&b).copied()) else { continue };
                let off = offsets.get(bi).copied().unwrap_or(Mat4::IDENTITY);
                let mut row = Vec::with_capacity(20);
                for &vi in q {
                    let v = off.transform_point3(verts[vi as usize]);
                    row.extend_from_slice(&[v.x, v.y, v.z]);
                    row.extend_from_slice(&uv(vi));
                }
                push(node, row, &mut groups);
            }
        } else {
            for q in &quads {
                let mut row = Vec::with_capacity(20);
                for &vi in q {
                    let v = verts[vi as usize];
                    row.extend_from_slice(&[v.x, v.y, v.z]);
                    row.extend_from_slice(&uv(vi));
                }
                push(idx, row, &mut groups);
            }
        }
        let vcolor = (!colors.is_empty() && colors.len() == n).then(|| {
            let mut a = [0.0f32; 4];
            for c in &colors {
                for k in 0..4 {
                    a[k] += c[k];
                }
            }
            a.map(|x| x / n as f32)
        });
        for (node, quads) in groups {
            self.sprites.push(Sprite { node, quads, material: mat.clone(), vcolor, post });
        }
    }

    fn particle_system(&mut self, psys: usize, props: &HashMap<&'static str, usize>, idx: usize) {
        let mat = self.material(props);
        if is_kind_of(self.obj(psys).class, "NiParticleSystem") {
            self.npsys(psys, mat, idx);
        } else {
            self.legacy(psys, mat, idx);
        }
    }

    fn npsys(&mut self, psys: usize, mat: Material, idx: usize) {
        let pf = &self.obj(psys).fields;
        let max = self.target(pf, "data").and_then(|d| self.obj(d).fields.u16("vertex_count")).map(|n| n as u32);
        let mut ctl = Controllers::default();
        let mut active: HashMap<String, VisTrack> = HashMap::new();
        for c in self.controllers(pf) {
            let co = self.obj(c);
            let cf = &co.fields;
            self.ctrl_duration(cf);
            let data = self.target(cf, "data").map(|d| &self.obj(d).fields);
            match co.class {
                "NiPSysEmitterCtlr" => {
                    let mut e = EmitCtl { timing: timing(cf), rate: None, vis: Vec::new() };
                    if let Some(d) = data {
                        e.rate = Some(key_group(d.list("float_keys"), d.u32("float_key_type").unwrap_or(1), 1));
                        e.vis = bool_keys(d, "visibility_keys");
                    }
                    ctl.emit = Some(e);
                }
                "NiPSysModifierActiveCtlr" => {
                    if let Some(d) = data {
                        active.insert(str_field(cf, "modifier_name"), VisTrack { timing: timing(cf), k: bool_keys(d, "keys") });
                    }
                }
                "NiPSysEmitterInitialRadiusCtlr" => {
                    if let Some(d) = data {
                        ctl.radius = Some(FloatTrack { timing: timing(cf), keys: key_group(d.list("keys"), d.u32("key_type").unwrap_or(1), 1) });
                    }
                }
                "NiPSysEmitterSpeedCtlr" => {
                    if let Some(d) = data {
                        ctl.speed = Some(FloatTrack { timing: timing(cf), keys: key_group(d.list("keys"), d.u32("key_type").unwrap_or(1), 1) });
                    }
                }
                _ => {}
            }
        }
        let mut out = Emitter {
            node: idx,
            world: pf.u8("world_space").unwrap_or(0) != 0,
            max,
            material: mat,
            ctl,
            emitter: EmitterShape {
                speed: 0.0,
                speed_var: 0.0,
                decl: 0.0,
                decl_var: 0.0,
                planar: 0.0,
                planar_var: 0.0,
                color: [1.0; 4],
                radius: 0.0,
                life: 0.0,
                life_var: 0.0,
                shape: String::new(),
                obj: -1,
                size: None,
                points: None,
                vel_type: None,
                axis: None,
                active: None,
            },
            grow: None,
            fade: None,
            colors: None,
            rot_speed: None,
            gravity: Vec::new(),
            drag: Vec::new(),
            collider: None,
        };
        let mut has_emitter = false;
        let modifiers: Vec<usize> = pf.links("modifiers").unwrap_or(&[]).iter().filter_map(|l| self.file.target(*l)).filter(|&i| i < self.file.objects.len()).collect();
        for m in modifiers {
            let mo = self.obj(m);
            let mf = &mo.fields;
            let name = str_field(mf, "modifier_name");
            let act = active.get(&name).cloned();
            let node_ref = |o: Option<usize>| o.and_then(|o| self.node_index.get(&o).copied()).map_or(-1, |i| i as i32);
            if is_kind_of(mo.class, "NiPSysEmitter") {
                let w = mf.f32s("emitter_words").map(floats).unwrap_or_default();
                if w.len() < 13 {
                    continue;
                }
                let mut e = EmitterShape {
                    speed: w[0],
                    speed_var: w[1],
                    decl: w[2],
                    decl_var: w[3],
                    planar: w[4],
                    planar_var: w[5],
                    color: [w[6], w[7], w[8], w[9]],
                    radius: w[10],
                    life: w[11],
                    life_var: w[12],
                    shape: String::new(),
                    obj: -1,
                    size: None,
                    points: None,
                    vel_type: None,
                    axis: None,
                    active: act,
                };
                let f = |n: &str| fl(mf.f32(n).unwrap_or(0));
                match mo.class {
                    "NiPSysBoxEmitter" => {
                        e.shape = "box".into();
                        e.obj = node_ref(self.target(mf, "emitter_object"));
                        e.size = Some([f("width"), f("height"), f("depth")]);
                    }
                    "NiPSysSphereEmitter" => {
                        e.shape = "sphere".into();
                        e.obj = node_ref(self.target(mf, "emitter_object"));
                        e.size = Some([f("radius"), 0.0, 0.0]);
                    }
                    "NiPSysCylinderEmitter" => {
                        e.shape = "cylinder".into();
                        e.obj = node_ref(self.target(mf, "emitter_object"));
                        e.size = Some([f("radius"), f("height"), 0.0]);
                    }
                    "NiPSysMeshEmitter" => {
                        let mut pts = Vec::new();
                        for l in mf.links("meshes").unwrap_or(&[]) {
                            let Some(mesh) = self.file.target(*l).filter(|&i| i < self.file.objects.len()) else { continue };
                            let Some(md) = self.target(&self.obj(mesh).fields, "data") else { continue };
                            let mdf = &self.obj(md).fields;
                            let world = self.world(Some(mesh));
                            let vs: Vec<Vec3> = mdf.f32s("vertices").map(|w| w.chunks_exact(3).map(|c| Vec3::new(fl(c[0]), fl(c[1]), fl(c[2]))).collect()).unwrap_or_default();
                            let ns: Vec<Vec3> = mdf.f32s("normals").map(|w| w.chunks_exact(3).map(|c| Vec3::new(fl(c[0]), fl(c[1]), fl(c[2]))).collect()).unwrap_or_default();
                            let step = (vs.len() / 48).max(1);
                            for i in (0..vs.len()).step_by(step) {
                                let p = world.transform_point3(vs[i]);
                                let n = ns.get(i).map_or(Vec3::Z, |n| world.transform_vector3(*n).normalize_or(Vec3::Z));
                                pts.push([p.x, p.y, p.z, n.x, n.y, n.z]);
                            }
                        }
                        let a = mf.f32s("emission_axis").map(floats).filter(|v| v.len() >= 3).map_or([0.0, 0.0, 1.0], |v| [v[0], v[1], v[2]]);
                        e.shape = "mesh".into();
                        e.points = Some(pts);
                        e.vel_type = Some(mf.u32("initial_velocity_type").unwrap_or(0) as u8);
                        e.axis = Some(a);
                    }
                    _ => continue,
                }
                out.emitter = e;
                has_emitter = true;
            } else if mo.class == "NiPSysGrowFadeModifier" {
                out.grow = Some(fl(mf.f32("grow_time").unwrap_or(0)));
                out.fade = Some(fl(mf.f32("fade_time").unwrap_or(0)));
            } else if mo.class == "NiPSysColorModifier" {
                if let Some(d) = self.target(mf, "link") {
                    let df = &self.obj(d).fields;
                    out.colors = Some(key_group(df.list("keys"), df.u32("key_type").unwrap_or(1), 4));
                }
            } else if mo.class == "NiPSysRotationModifier" {
                out.rot_speed = Some(fl(mf.f32("initial_speed").unwrap_or(0)));
            } else if mo.class == "NiPSysGravityModifier" {
                let go = self.target(mf, "gravity_object");
                let w = self.world(go);
                let a = mf.f32s("axis").map(floats).filter(|v| v.len() >= 3).map_or(Vec3::Z, |v| Vec3::new(v[0], v[1], v[2]));
                let axis = Mat3::from_mat4(w) * a;
                let pos = w.w_axis.truncate();
                out.gravity.push(Gravity {
                    axis: axis.to_array(),
                    strength: fl(mf.f32("strength").unwrap_or(0)),
                    decay: fl(mf.f32("decay").unwrap_or(0)),
                    kind: mf.u32("force_type").unwrap_or(0) as u8,
                    turb: fl(mf.f32("turbulence").unwrap_or(0)),
                    pos: pos.to_array(),
                    active: act,
                });
            } else if mo.class == "NiPSysDragModifier" {
                let w = self.world(self.target(mf, "parent"));
                let a = mf.f32s("axis").map(floats).filter(|v| v.len() >= 3).map_or(Vec3::Z, |v| Vec3::new(v[0], v[1], v[2]));
                out.drag.push(Drag { axis: (Mat3::from_mat4(w) * a).to_array(), pct: fl(mf.f32("percentage").unwrap_or(0)), active: act });
            } else if mo.class == "NiPSysColliderManager"
                && let Some(c) = self.target(mf, "link")
                && self.obj(c).class == "NiPSysPlanarCollider"
            {
                let cf = &self.obj(c).fields;
                let w = self.world(self.target(cf, "collider_object"));
                out.collider = Some(Collider { bounce: fl(cf.f32("bounce").unwrap_or(0)), die: cf.u8("die_on_collide").unwrap_or(0) != 0, z: w.w_axis.z });
            }
        }
        if has_emitter {
            self.emitters.push(out);
        }
    }

    /// NetImmerse 10.0 NiParticles + NiParticleSystemController.
    fn legacy(&mut self, parts: usize, mat: Material, idx: usize) {
        let pf = &self.obj(parts).fields;
        for c in self.controllers(pf) {
            let co = self.obj(c);
            if co.class != "NiParticleSystemController" {
                continue;
            }
            let cf = &co.fields;
            self.ctrl_duration(cf);
            let w = cf.f32s("emitter_words").map(floats).unwrap_or_default();
            let b = cf.f32s("birth_words").map(floats).unwrap_or_default();
            if w.len() < 16 || b.len() < 3 {
                continue;
            }
            // speed, speed_random, vertical dir/angle, horizontal dir/angle,
            // normal(3), colour(4), size, emit start, emit stop.
            let (size, start, stop) = (w[13], w[14], w[15]);
            let (rate, life, life_random) = (b[0], b[1], b[2]);
            let obj = self.target(cf, "emitter").and_then(|o| self.node_index.get(&o).copied()).map_or(-1, |i| i as i32);
            let mut out = Emitter {
                node: idx,
                world: true,
                max: None,
                material: mat.clone(),
                ctl: Controllers {
                    emit: Some(EmitCtl { timing: timing(cf), rate: Some(KeyGroup { i: 1, k: vec![vec![0.0, rate]] }), vis: vec![[start, 1.0], [stop, 0.0]] }),
                    radius: None,
                    speed: None,
                },
                emitter: EmitterShape {
                    speed: w[0],
                    speed_var: w[1],
                    decl: w[2],
                    decl_var: w[3],
                    planar: w[4],
                    planar_var: w[5],
                    color: [1.0; 4],
                    radius: size,
                    life,
                    life_var: life_random,
                    shape: "box".into(),
                    obj,
                    size: Some([0.0; 3]),
                    points: None,
                    vel_type: None,
                    axis: None,
                    active: None,
                },
                grow: None,
                fade: None,
                colors: None,
                rot_speed: None,
                gravity: Vec::new(),
                drag: Vec::new(),
                collider: None,
            };
            // The modifier chain (PyFFI's particle_extra): the first linked
            // of the two modifier slots.
            let mut m = self.target(cf, "modifier").or_else(|| self.target(cf, "collider"));
            let mut guard = 0;
            while let Some(mi) = m {
                guard += 1;
                if guard > 32 {
                    break;
                }
                let mo = self.obj(mi);
                let mf = &mo.fields;
                match mo.class {
                    "NiParticleGrowFade" => {
                        out.grow = Some(fl(mf.f32("grow").unwrap_or(0)));
                        out.fade = Some(fl(mf.f32("fade").unwrap_or(0)));
                    }
                    "NiParticleColorModifier" => {
                        if let Some(d) = self.target(mf, "color_data") {
                            let df = &self.obj(d).fields;
                            let mut g = key_group(df.list("keys"), df.u32("key_type").unwrap_or(1), 4);
                            // Legacy colour keys run over the lifetime in seconds.
                            if g.k.last().is_some_and(|k| k[0] > 1.01) {
                                let l = life.max(1e-3);
                                for k in &mut g.k {
                                    k[0] = (k[0] / l).min(1.0);
                                }
                            }
                            out.colors = Some(g);
                        }
                    }
                    "NiParticleRotation" => out.rot_speed = Some(fl(mf.f32("rotation_speed").unwrap_or(0))),
                    "NiGravity" => {
                        let d = mf.f32s("direction").map(floats).filter(|v| v.len() >= 3).map_or([0.0, 0.0, -1.0], |v| [v[0], v[1], v[2]]);
                        let p = mf.f32s("position").map(floats).filter(|v| v.len() >= 3).map_or([0.0; 3], |v| [v[0], v[1], v[2]]);
                        out.gravity.push(Gravity { axis: d, strength: fl(mf.f32("force").unwrap_or(0)), decay: 0.0, kind: mf.u32("field_type").unwrap_or(0) as u8, turb: 0.0, pos: p, active: None });
                    }
                    _ => {}
                }
                m = self.target(mf, "next");
            }
            self.emitters.push(out);
        }
    }
}

/// One effect NIF (`nif_rel` Data-relative, for texture lookup).
pub fn extract(file: &NifFile, nif_rel: &str, textures: &mut Textures) -> Option<NifEffect> {
    let mut ex = Extract {
        file,
        nif_rel: nif_rel.to_owned(),
        textures,
        nodes: Vec::new(),
        node_index: HashMap::new(),
        sprites: Vec::new(),
        emitters: Vec::new(),
        duration: 0.0,
        tag: 0,
        psys: Vec::new(),
        geometry: None,
    };
    let roots: Vec<usize> = file.top_level.iter().flatten().copied().filter(|&i| i < file.objects.len()).collect();
    for r in roots {
        if is_kind_of(file.objects[r].class, "NiAVObject") {
            ex.walk(r, -1, &HashMap::new(), false);
        }
    }
    for (p, props, idx) in std::mem::take(&mut ex.psys) {
        ex.particle_system(p, &props, idx);
    }
    if ex.sprites.is_empty() && ex.emitters.is_empty() {
        return None;
    }
    Some(NifEffect { duration: ex.duration, nodes: ex.nodes, sprites: ex.sprites, emitters: ex.emitters })
}

/// The particle-player folders (as the client extractor): UI markers,
/// flags, transports, waypoints and the special-operation set pieces are
/// animated models, not particle effects. Death debris adds its smoke.
pub const INCLUDE: [&str; 21] = [
    "gunshot/",
    "tankshot/",
    "artilleryshot/",
    "mortarshot/",
    "rocketattacks/",
    "explosions/",
    "burningeffects/",
    "flakshot/",
    "moveeffect/",
    "crusheddoodad/",
    "construction/pack",
    "specialops/smokebarrage/",
    "specialops/atomicbomb/atomicsparks",
    "specialops/vweapons/v1explosion",
    "specialops/vweapons/v1groundfire",
    "specialops/vweapons/v1shock",
    "specialops/courage/couragecloud",
    "specialops/fear/fearcloud",
    "specialops/economicaid/grounddust",
    "specialops/resupply/",
    "deathdebris/",
];

/// Build the table for the VFX models (`effects/...` / `projectiles/...`,
/// lower case Data-relative paths). Keys are relative to Data/Effects like
/// the client's table.
pub fn build(data_root: &Path, models: &[String], texture_dir: PathBuf) -> NifTable {
    let mut textures = Textures::new(data_root, texture_dir);
    let mut effects = HashMap::new();
    let mut failures = HashMap::new();
    let mut seen = std::collections::HashSet::new();
    for m in models {
        let key = m.strip_prefix("effects/").unwrap_or(m).to_ascii_lowercase();
        if !INCLUDE.iter().any(|p| key.starts_with(p)) || !seen.insert(key.clone()) {
            continue;
        }
        let Some(path) = super::bones::find_file(data_root, m) else {
            failures.insert(key, "missing".into());
            continue;
        };
        let rel = path.strip_prefix(data_root).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_else(|_| m.clone());
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                failures.insert(key, e.to_string());
                continue;
            }
        };
        match aa_formats::nif::load(&bytes) {
            Ok(file) => match extract(&file, &rel, &mut textures) {
                Some(fx) => {
                    effects.insert(key, fx);
                }
                None => {
                    failures.insert(key, "empty".into());
                }
            },
            Err(e) => {
                failures.insert(key, e.to_string());
            }
        }
    }
    NifTable { format: "aa-bevy-nif-effects-1".into(), textures: textures.keys, effects, failures }
}

/// The scene graph of a model NIF: the node table (bind transforms with
/// their keyframe and visibility tracks), object -> node, and the drawn
/// geometry objects with their nodes, in walk order.
pub struct Scene {
    pub nodes: Vec<Node>,
    pub node_of: HashMap<usize, usize>,
    pub geometry: Vec<(usize, usize)>,
    pub duration: f32,
}

pub fn scene(file: &NifFile) -> Scene {
    let mut textures = Textures::default();
    let mut ex = Extract {
        file,
        nif_rel: String::new(),
        textures: &mut textures,
        nodes: Vec::new(),
        node_index: HashMap::new(),
        sprites: Vec::new(),
        emitters: Vec::new(),
        duration: 0.0,
        tag: 0,
        psys: Vec::new(),
        geometry: Some(Vec::new()),
    };
    let roots: Vec<usize> = file.top_level.iter().flatten().copied().filter(|&i| i < file.objects.len()).collect();
    for r in roots {
        if is_kind_of(file.objects[r].class, "NiAVObject") {
            ex.walk(r, -1, &HashMap::new(), false);
        }
    }
    Scene { geometry: ex.geometry.take().unwrap_or_default(), nodes: ex.nodes, node_of: ex.node_index, duration: ex.duration }
}

/// NiTransform words as a matrix (skin data and bone offsets).
pub fn skin_transform(w: &[u32]) -> Mat4 {
    transform(w)
}
