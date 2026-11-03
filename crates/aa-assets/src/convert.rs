//! The art of a loose data folder (a profile's `data`, a mod), converted the way the
//! stock art is: its NIFs to `aa-nif-mesh-v1` JSON, its DDS and TGA textures to PNG, into
//! `<output>/assets/converted/...`, when [`crate::AssetStore::set_art_overlay`] is asked
//! for them ([`FolderArt`]).
//!
//! This is the project's own exporter (`convert_models.py` and `archive_textures.py`,
//! which made the stock `assets/converted`) in Rust, on the ported NiStream reader
//! [`aa_formats::nif`], so a release, which has no Python, draws a mod's art.
//! `tests/convert_stock.rs` holds it against the whole stock art. The original game has
//! no such step: it draws the NIFs directly. So the numbers here are the exporter's, not
//! the game's: the walk, the hidden, shadow and selection helpers it leaves out, the
//! nearest LOD, the bind pose of a skin, `(x, z, -y)`, `1 - v`, six decimals.
//!
//! The stock models get their audited corrections from a sidecar (`model_fixes` of the
//! client, `research/texture-phase2/export_model_materials.py`). A folder's models have no
//! audit, so [`ModelFile::corrected`] applies the same rules here: the tags of 879A90, the
//! inherited alpha and depth state, the authored sphere centre.
//!
//! A file the folder holds byte for byte like the stock data is not converted: the stock
//! conversion already is its result, and the store falls through to it. A mod is mostly a
//! copy of the game, so that is most of its files.
use aa_formats::nif::{self, Field, NifFile, NifObject, Value, is_kind_of};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Stale plurals in the original NIFs; the matching atlas lies beside each mesh.
const TEXTURE_ALIASES: [(&str, &str); 2] = [("savannahrocksbase.dds", "savannahrockbase.dds"), ("temperatetreebase.dds", "temperatetreesbase.dds")];

/// A 4x4 in the row-vector convention of the exporter: `m[row][column]`, `v' = v * M`.
#[derive(Clone, Copy)]
struct M44([[f64; 4]; 4]);
/// The upper left 3x3.
#[derive(Clone, Copy)]
struct M33([[f64; 3]; 3]);

impl M44 {
    /// `scale * rotation` over the translation. The nine rotation words are stored
    /// `m11 m21 m31 m12 m22 m32 m13 m23 m33`.
    fn compose(scale: f64, r: &[u32], t: &[u32]) -> Self {
        let w = |i: usize| f64::from(f32::from_bits(r[i])) * scale;
        Self([
            [w(0), w(3), w(6), 0.0],
            [w(1), w(4), w(7), 0.0],
            [w(2), w(5), w(8), 0.0],
            [f64::from(f32::from_bits(t[0])), f64::from(f32::from_bits(t[1])), f64::from(f32::from_bits(t[2])), 1.0],
        ])
    }
    fn mul(&self, x: &Self) -> Self {
        let mut m = [[0.0; 4]; 4];
        for (r, row) in m.iter_mut().enumerate() {
            for (c, cell) in row.iter_mut().enumerate() {
                *cell = self.0[r][0] * x.0[0][c] + self.0[r][1] * x.0[1][c] + self.0[r][2] * x.0[2][c] + self.0[r][3] * x.0[3][c];
            }
        }
        Self(m)
    }
    /// The rotation of `get_scale_rotation_translation`: the 3x3 over the signed cube root
    /// of its determinant. A zero scale is a failure of the whole model, as it was.
    fn rotation(&self) -> Result<M33, String> {
        let m = &self.0;
        let (a, b, c, d, e, f, g, h, i) = (m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0], m[2][1], m[2][2]);
        let det = a * e * i + b * f * g + c * d * h - g * e * c - d * b * i - a * h * f;
        let scale = if det < 0.0 { -((-det).powf(1.0 / 3.0)) } else { det.powf(1.0 / 3.0) };
        if scale == 0.0 {
            return Err("float division by zero".into());
        }
        Ok(M33([[a / scale, b / scale, c / scale], [d / scale, e / scale, f / scale], [g / scale, h / scale, i / scale]]))
    }
    /// The skin's transforms hold `rotation, translation, scale` (13 words).
    fn of_skin_transform(w: &[u32]) -> Result<Self, String> {
        if w.len() < 13 {
            return Err("short skin transform".into());
        }
        Ok(Self::compose(f64::from(f32::from_bits(w[12])), &w[0..9], &w[9..12]))
    }
}

fn word3(w: &[u32], i: usize) -> [f64; 3] {
    [f64::from(f32::from_bits(w[i * 3])), f64::from(f32::from_bits(w[i * 3 + 1])), f64::from(f32::from_bits(w[i * 3 + 2]))]
}

/// Python's `round(value, 6)`: the exact decimal expansion, so not `(v * 1e6).round()`.
fn round6(value: f64) -> Result<f64, String> {
    if !value.is_finite() {
        return Err("Non-finite mesh coordinate".into());
    }
    format!("{value:.6}").parse().map_err(|_| "Non-finite mesh coordinate".to_owned())
}

fn text(v: Option<&[u8]>) -> String {
    v.map_or_else(String::new, |b| String::from_utf8_lossy(b).into_owned())
}

fn field<'a>(fields: &'a [Field], name: &str) -> Option<&'a Value> {
    fields.iter().find(|f| f.name == name).map(|f| &f.value)
}

/// The word the file holds, where the native loader stored a converted one.
fn file_u16(o: &NifObject, name: &str) -> u16 {
    match o.fields.field(name) {
        Some(Field { read: Some(Value::U16(v)), .. }) | Some(Field { value: Value::U16(v), .. }) => *v,
        _ => 0,
    }
}

/// `files` of the exporter: every file of a folder by lower case relative path, and the
/// texture files by lower case name.
struct FileIndex {
    root: PathBuf,
    /// lower case relative path -> relative path as it lies on disk.
    by_rel: HashMap<String, String>,
    by_name: HashMap<String, Vec<String>>,
}

impl FileIndex {
    fn walk(root: &Path) -> Self {
        let mut index = Self { root: root.to_path_buf(), by_rel: HashMap::new(), by_name: HashMap::new() };
        let mut stack = vec![(root.to_path_buf(), String::new())];
        while let Some((dir, rel)) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let path = entry.path();
                let child = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
                if path.is_dir() {
                    stack.push((path, child));
                    continue;
                }
                let lower = name.to_lowercase();
                if [".dds", ".tga", ".png", ".bmp", ".jpg"].iter().any(|x| lower.ends_with(x)) {
                    index.by_name.entry(lower).or_default().push(child.clone());
                }
                index.by_rel.insert(child.to_lowercase(), child);
            }
        }
        index
    }

    /// The candidate sharing the longest folder prefix with the model: climate variants
    /// stay local.
    fn nearest(&self, model_rel: &str, name: &str) -> Option<&str> {
        let model: Vec<String> = model_rel.split('/').map(str::to_lowercase).collect();
        let rank = |path: &str| {
            let parts: Vec<String> = path.split('/').map(str::to_lowercase).collect();
            let matching = model.iter().zip(&parts).take_while(|(a, b)| a == b).count();
            (std::cmp::Reverse(matching), parts.len(), path.to_lowercase())
        };
        self.by_name.get(name)?.iter().min_by_key(|p| rank(p)).map(String::as_str)
    }
}

/// `<parent of the model>/<raw>`, as the path arithmetic gave it.
fn beside_model(model_rel: &str, raw: &str) -> String {
    let parent = model_rel.rsplit_once('/').map_or("", |(d, _)| d);
    let mut parts: Vec<&str> = parent.split('/').filter(|p| !p.is_empty()).collect();
    parts.extend(raw.split('/').filter(|p| !p.is_empty() && *p != "."));
    parts.join("/").to_lowercase()
}

/// `with_suffix('.png')`.
fn with_suffix(rel: &str, suffix: &str) -> String {
    let (dir, name) = rel.rsplit_once('/').map_or(("", rel), |(d, n)| (d, n));
    let stem = match name.rfind('.') {
        Some(dot) if dot > 0 => &name[..dot],
        _ => name,
    };
    if dir.is_empty() { format!("{stem}{suffix}") } else { format!("{dir}/{stem}{suffix}") }
}

/// What a model's texture names resolve against: the folder, then the stock data.
pub struct ArtIndex {
    own: FileIndex,
    stock: Option<FileIndex>,
    missing: Mutex<BTreeSet<String>>,
}

impl ArtIndex {
    pub fn new(data: &Path, stock: Option<&Path>) -> Self {
        Self { own: FileIndex::walk(data), stock: stock.map(FileIndex::walk), missing: Mutex::default() }
    }

    fn resolve_texture(&self, model_rel: &str, filename: &str) -> Option<String> {
        let raw = filename.replace('\\', "/");
        let raw = raw.trim_start_matches('/');
        if raw.is_empty() {
            return None;
        }
        let name = raw.rsplit('/').next().unwrap_or(raw).to_lowercase();
        let beside = if raw.contains(':') { raw.to_lowercase() } else { beside_model(model_rel, raw) };
        let own = &self.own;
        let found = own.by_rel.get(&beside).or_else(|| own.by_rel.get(&raw.to_lowercase())).map(String::as_str).or_else(|| {
            own.nearest(model_rel, &name).or_else(|| {
                let alias = TEXTURE_ALIASES.iter().find(|(from, _)| *from == name)?.1;
                own.nearest(model_rel, alias)
            })
        });
        if let Some(rel) = found {
            let lower = rel.to_lowercase();
            if lower.ends_with(".png") || lower.ends_with(".jpg") {
                return Some(format!("/assets/original/Data/{rel}"));
            }
            return Some(format!("/assets/converted/textures/{}", with_suffix(rel, ".png")));
        }
        // A texture the folder does not hold is the game's: the stock URL.
        if let Some(stock) = &self.stock {
            let rel = stock.by_rel.get(&beside).or_else(|| stock.by_rel.get(&raw.to_lowercase())).map(String::as_str).or_else(|| stock.nearest(model_rel, &name));
            if let Some(rel) = rel {
                return Some(format!("/assets/converted/textures/{}", with_suffix(rel, ".png")));
            }
        }
        if let Ok(mut missing) = self.missing.lock() {
            missing.insert(raw.to_owned());
        }
        None
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Bounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub size: [f64; 3],
}

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct Material {
    color: Option<[f64; 3]>,
    opacity: Option<f64>,
    double_sided: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    emissive: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base_texture_state: Option<BaseTextureState>,
    /// A key with `null` when the name resolves to nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    texture: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_texture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alpha_property_state: Option<AlphaState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alpha_blend: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alpha_test: Option<f64>,
    team_color_mask: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    native_render_state: Option<RenderState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    native_sort_center: Option<[f64; 3]>,
}

/// The native render state of a mesh: what `aa_assets::NativeRenderState` reads.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderState {
    pub alpha_flags: u16,
    pub alpha_threshold: u8,
    pub zbuffer_flags: u16,
    pub zbuffer_function: u32,
}

/// What the audit of the native tags says of one mesh.
#[derive(Debug, Clone, Copy)]
pub struct Correction {
    /// A shadow helper (`Sg`, `Sg2`): not drawn.
    pub skip: bool,
    pub team_color_mask: bool,
    pub render_state: RenderState,
    pub sort_center: [f64; 3],
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct BaseTextureState {
    clamp_mode: u32,
    filter_mode: u32,
    uv_set: u32,
}

#[derive(Serialize, Debug)]
struct AlphaState {
    flags: u16,
    threshold: u8,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Mesh {
    name: String,
    hierarchy: Vec<String>,
    positions: Vec<f64>,
    normals: Vec<f64>,
    uvs: Vec<f64>,
    indices: Vec<u32>,
    material: Material,
    skinned_pose: bool,
    #[serde(skip)]
    correction: Option<Correction>,
}

#[derive(Serialize, Debug)]
struct Skipped {
    name: String,
    reason: &'static str,
}

/// One converted model: the file the store reads.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModelFile {
    format: &'static str,
    source: String,
    nif_version: String,
    up_axis: &'static str,
    pose: &'static str,
    bounds: Bounds,
    meshes: Vec<Mesh>,
    skipped_helpers: Vec<Skipped>,
}

impl ModelFile {
    /// The audited rows of the meshes, in order.
    pub fn corrections(&self) -> Vec<Correction> {
        self.meshes.iter().filter_map(|m| m.correction).collect()
    }

    /// The corrections applied: shadow helpers gone, the team colour mask and the native
    /// render state set, the bounds over what is left (`ModelFixes::apply`).
    pub fn corrected(mut self) -> Result<Self, String> {
        self.meshes.retain(|m| !m.correction.is_some_and(|c| c.skip));
        let mut min = [0.0f64; 3];
        let mut max = [0.0f64; 3];
        let mut first = true;
        for mesh in &mut self.meshes {
            if let Some(c) = mesh.correction {
                mesh.material.team_color_mask = c.team_color_mask;
                mesh.material.native_render_state = Some(c.render_state);
                mesh.material.native_sort_center = Some(c.sort_center);
            }
            for p in mesh.positions.chunks_exact(3) {
                for axis in 0..3 {
                    min[axis] = if first { p[axis] } else { min[axis].min(p[axis]) };
                    max[axis] = if first { p[axis] } else { max[axis].max(p[axis]) };
                }
                first = false;
            }
        }
        self.bounds = Bounds { min, max, size: [round6(max[0] - min[0])?, round6(max[1] - min[1])?, round6(max[2] - min[2])?] };
        Ok(self)
    }
}

/// `RepColor`, `Sg`, `Sg2` on an object: 8790B0 normalizes its string extras, 877C50 compares
/// the name by exact length, ignoring ASCII case, and 871CC0 looks only for presence.
fn own_tags(nif: &NifFile, o: &NifObject) -> [bool; 3] {
    const TAG_TABLE: [&str; 10] = ["mod", "eq", "ambient_repcolor", "emrc", "repcolor", "fastpath", "tred", "sg", "sg2", "mkf"];
    const MATERIAL_TAGS: [&str; 3] = ["RepColor", "Sg", "Sg2"];
    let mut extras: Vec<usize> = o.fields.links("extra_data_list").unwrap_or(&[]).iter().filter_map(|&l| nif.target(l)).collect();
    let mut linked = o.fields.link("extra_data").and_then(|l| nif.target(l));
    while let Some(e) = linked {
        if extras.contains(&e) {
            break;
        }
        extras.push(e);
        linked = nif.objects[e].fields.link("next").and_then(|l| nif.target(l));
    }
    struct Row {
        name: String,
        value: String,
        is_string: bool,
        tags: Vec<String>,
        removed: bool,
    }
    let until_nul = |s: String| s.split('\0').next().unwrap_or("").to_owned();
    let mut rows = Vec::new();
    for e in extras {
        let extra = &nif.objects[e];
        let is_string = is_kind_of(extra.class, "NiStringExtraData");
        let name = until_nul(text(extra.fields.string("extra_name").flatten()));
        let value = until_nul(text(extra.fields.string("string").flatten()));
        let (mut tags, mut leftover) = (Vec::new(), false);
        if is_string {
            let lines: Vec<&str> = value.split("\r\n").collect();
            for (i, line) in lines.iter().enumerate() {
                // The scan keeps the last `=` and does not flush a final one.
                if i + 1 == lines.len() && line.ends_with('=') {
                    leftover = true;
                    continue;
                }
                let keyword = line.rsplit_once('=').map_or(*line, |(k, _)| k);
                if keyword.is_ascii() && TAG_TABLE.contains(&keyword.to_ascii_lowercase().as_str()) {
                    tags.push(keyword.to_ascii_lowercase());
                } else if !keyword.is_empty() {
                    leftover = true;
                }
            }
        }
        let removed = !tags.is_empty() && !leftover;
        rows.push(Row { name: if removed { String::new() } else { name }, value, is_string, tags, removed });
    }
    MATERIAL_TAGS.map(|tag| {
        let lower = tag.to_ascii_lowercase();
        rows.iter().any(|r| r.name == tag || (r.is_string && (r.tags.contains(&lower) || (!r.removed && r.value.is_ascii() && r.value.to_ascii_lowercase() == lower))))
    })
}

/// The tags in force on an object: 879A90 resets them at any object that is not a geometry.
#[derive(Clone, Copy, Default)]
struct TagContext {
    shadow: bool,
    team: bool,
}

/// The properties a node inherits, one per class, nearest wins.
#[derive(Clone, Copy, Default)]
struct Properties {
    material: Option<usize>,
    texturing: Option<usize>,
    alpha: Option<usize>,
    stencil: Option<usize>,
    zbuffer: Option<usize>,
}

struct Walk<'a> {
    nif: &'a NifFile,
    rel: &'a str,
    index: &'a ArtIndex,
    seen: Vec<bool>,
    meshes: Vec<Mesh>,
    skipped: Vec<Skipped>,
}

impl Walk<'_> {
    fn object(&self, link: Option<u32>) -> Option<usize> {
        link.and_then(|l| self.nif.target(l)).filter(|&t| t < self.nif.objects.len())
    }

    fn local(&self, o: &NifObject) -> Result<M44, String> {
        let (Some(r), Some(t), Some(s)) = (o.fields.f32s("rotation"), o.fields.f32s("translation"), o.fields.f32("scale")) else {
            return Err("object without a transform".into());
        };
        Ok(M44::compose(f64::from(f32::from_bits(s)), r, t))
    }

    fn material(&self, properties: &Properties, team: bool) -> Result<(Material, usize), String> {
        let mut mat = Material { color: Some([1.0, 1.0, 1.0]), opacity: Some(1.0), team_color_mask: team, ..Material::default() };
        let mut uv_set = 0usize;
        if let Some(p) = properties.material.map(|i| &self.nif.objects[i]) {
            mat.name = Some(text(p.fields.string("name").flatten()));
            let colour = |name: &str| -> Result<[f64; 3], String> {
                let w = p.fields.f32s(name).ok_or("material without a colour")?;
                let c = word3(w, 0);
                Ok([round6(c[0])?, round6(c[1])?, round6(c[2])?])
            };
            mat.color = Some(colour("diffuse")?);
            mat.opacity = Some(round6(f64::from(f32::from_bits(p.fields.f32("alpha").unwrap_or(0))))?);
            mat.emissive = Some(colour("emissive")?);
        }
        if let Some(p) = properties.texturing.map(|i| &self.nif.objects[i]) {
            if let Some(Value::Record(base)) = p.fields.list("maps").and_then(|m| m.first()) {
                let word = |name: &str| match field(base, name) {
                    Some(Value::U32(v)) => *v,
                    _ => 0,
                };
                mat.base_texture_state = Some(BaseTextureState { clamp_mode: word("clamp"), filter_mode: word("filter"), uv_set: word("uv_set") });
                let source = match field(base, "source") {
                    Some(Value::Link(l)) => self.object(Some(*l)),
                    _ => None,
                };
                if let Some(source) = source {
                    let name = text(self.nif.objects[source].fields.string("file_name").flatten());
                    mat.texture = Some(self.index.resolve_texture(self.rel, &name));
                    mat.source_texture = Some(name);
                    uv_set = word("uv_set") as usize;
                }
            }
        }
        if let Some(p) = properties.alpha.map(|i| &self.nif.objects[i]) {
            let flags = file_u16(p, "flags");
            let threshold = p.fields.u8("threshold").unwrap_or(0);
            mat.alpha_property_state = Some(AlphaState { flags, threshold });
            mat.alpha_blend = Some(flags & 1 != 0);
            mat.alpha_test = Some(if flags & 512 != 0 { round6(f64::from(threshold) / 255.0)? } else { 0.0 });
        }
        if let Some(p) = properties.stencil.map(|i| &self.nif.objects[i]) {
            mat.double_sided = p.fields.u32("draw_mode") == Some(3);
        }
        Ok((mat, uv_set))
    }

    /// `bone.get_transform(skeleton root)`: the local transforms up to, not including, the
    /// root (the bone's own when it is the root).
    fn relative_to(&self, bone: usize, root: usize) -> Result<M44, String> {
        let mut m = self.local(&self.nif.objects[bone])?;
        let mut at = bone;
        let mut guard = 0;
        while at != root {
            let parent = self.nif.parents.get(at).copied().flatten().ok_or("a bone outside its skeleton root")?;
            if parent == root {
                break;
            }
            m = m.mul(&self.local(&self.nif.objects[parent])?);
            at = parent;
            guard += 1;
            if guard > self.nif.objects.len() {
                return Err("a cycle in the bones".into());
            }
        }
        Ok(m)
    }

    /// `get_skin_deformation`: vertices and normals in their bind pose, in geometry space.
    fn deform(&self, geometry: &NifObject, data: &NifObject, count: usize) -> Result<(Vec<f64>, Vec<f64>), String> {
        let skin = self.object(geometry.fields.link("skin_instance")).map(|i| &self.nif.objects[i]).ok_or("no skin")?;
        let skin_data = self.object(skin.fields.link("data")).map(|i| &self.nif.objects[i]).ok_or("NiGeometry has NiSkinInstance without NiSkinData")?;
        let root = self.object(skin.fields.link("root_parent")).ok_or("NiGeometry has NiSkinInstance without skeleton root")?;
        let vertices = data.fields.f32s("vertices").unwrap_or(&[]);
        let source_normals = data.fields.f32s("normals").unwrap_or(&[]);
        let has_normals = data.fields.u32("has_normals").unwrap_or(0) != 0;
        let mut out = vec![0.0; count * 3];
        let mut normals = vec![0.0; count * 3];
        let skin_offset = M44::of_skin_transform(skin_data.fields.f32s("root_transform").ok_or("skin without a transform")?)?;
        let bones = skin.fields.links("bones").unwrap_or(&[]);
        let list = skin_data.fields.list("bone_list").unwrap_or(&[]);
        for (i, &link) in bones.iter().enumerate() {
            let Some(Value::Record(bone_data)) = list.get(i) else {
                return Err("list index out of range".into());
            };
            let bone = self.object(Some(link)).ok_or("a bone that is not there")?;
            let Some(Value::F32s(offset)) = field(bone_data, "transform") else {
                return Err("bone without a transform".into());
            };
            let transform = M44::of_skin_transform(offset)?.mul(&self.relative_to(bone, root)?).mul(&skin_offset);
            // A bone scaled to nothing has no rotation: its normals stay out.
            let rotation = transform.rotation().unwrap_or(M33([[0.0; 3]; 3]));
            let (indices, weights) = match (field(bone_data, "vertex_indices"), field(bone_data, "vertex_weights")) {
                (Some(Value::U16s(i)), Some(Value::F32s(w))) => (i.as_slice(), w.as_slice()),
                _ => (&[][..], &[][..]),
            };
            for (&index, &weight) in indices.iter().zip(weights) {
                let index = index as usize;
                if index >= count || (index + 1) * 3 > vertices.len() {
                    return Err("list index out of range".into());
                }
                let weight = f64::from(f32::from_bits(weight));
                let [x, y, z] = word3(vertices, index);
                let m = &transform.0;
                for axis in 0..3 {
                    out[index * 3 + axis] += weight * (x * m[0][axis] + y * m[1][axis] + z * m[2][axis] + m[3][axis]);
                }
                if has_normals {
                    if (index + 1) * 3 > source_normals.len() {
                        return Err("list index out of range".into());
                    }
                    let [x, y, z] = word3(source_normals, index);
                    let r = &rotation.0;
                    for axis in 0..3 {
                        normals[index * 3 + axis] += weight * (x * r[0][axis] + y * r[1][axis] + z * r[2][axis]);
                    }
                }
            }
        }
        Ok((out, normals))
    }

    fn triangles(data: &NifObject) -> Result<Vec<u32>, String> {
        let mut out = Vec::new();
        if data.is("NiTriShapeData") {
            let count = data.fields.u16("triangle_count").unwrap_or(0) as usize;
            if let Some(words) = data.fields.u16s("triangles") {
                out.extend(words.iter().take(count * 3).map(|&i| u32::from(i)));
            }
        } else if data.is("NiTriStripsData") {
            // Degenerate triangles of a strip are left out; every other one is wound
            // the other way.
            let (Some(lengths), Some(points)) = (data.fields.u16s("strip_lengths"), data.fields.u16s("points")) else {
                return Ok(out);
            };
            let mut at = 0;
            for &length in lengths {
                let length = length as usize;
                let strip = points.get(at..at + length).ok_or("strip outside its points")?;
                at += length;
                if length < 3 {
                    continue;
                }
                let mut even = false;
                for w in strip.windows(3) {
                    even = !even;
                    let (t0, t1, t2) = (u32::from(w[0]), u32::from(w[1]), u32::from(w[2]));
                    if t0 == t1 || t1 == t2 || t2 == t0 {
                        continue;
                    }
                    out.extend(if even { [t0, t1, t2] } else { [t0, t2, t1] });
                }
            }
        } else {
            return Err("geometry data that is not triangles".into());
        }
        Ok(out)
    }

    /// The audited row of a mesh about to be listed.
    fn correction(&self, context: TagContext, properties: &Properties, meshdata: &NifObject, m: &[[f64; 4]; 4]) -> Result<Correction, String> {
        let alpha = properties.alpha.map(|i| &self.nif.objects[i]);
        let depth = properties.zbuffer.map(|i| &self.nif.objects[i]);
        // The native default NiAlphaProperty is 0xEC (913B58).
        let render_state = RenderState {
            alpha_flags: alpha.map_or(0xEC, |a| file_u16(a, "flags")),
            alpha_threshold: alpha.and_then(|a| a.fields.u8("threshold")).unwrap_or(0),
            zbuffer_flags: depth.map_or(3, |d| file_u16(d, "flags")),
            zbuffer_function: depth.and_then(|d| d.fields.u32("function")).unwrap_or(3),
        };
        let centre = word3(meshdata.fields.f32s("center").ok_or("geometry data without a centre")?, 0);
        let tx = centre[0] * m[0][0] + centre[1] * m[1][0] + centre[2] * m[2][0] + m[3][0];
        let ty = centre[0] * m[0][1] + centre[1] * m[1][1] + centre[2] * m[2][1] + m[3][1];
        let tz = centre[0] * m[0][2] + centre[1] * m[1][2] + centre[2] * m[2][2] + m[3][2];
        Ok(Correction { skip: context.shadow, team_color_mask: !context.shadow && context.team, render_state, sort_center: [round6(tx)?, round6(tz)?, round6(-ty)?] })
    }

    #[allow(clippy::too_many_arguments)]
    fn walk(&mut self, node: Option<usize>, matrix: Option<&M44>, inherited: &Properties, hierarchy: &[String], inherited_team: bool, inherited_tags: TagContext) -> Result<(), String> {
        let Some(index) = node else { return Ok(()) };
        let nif = self.nif;
        let o = &nif.objects[index];
        if !is_kind_of(o.class, "NiAVObject") || self.seen[index] {
            return Ok(());
        }
        self.seen[index] = true;
        let name = text(o.fields.string("name").flatten());
        let mut path = hierarchy.to_vec();
        path.push(name.clone());
        let mut transform = self.local(o)?;
        if let Some(m) = matrix {
            transform = transform.mul(m);
        }
        let mut properties = *inherited;
        for &link in o.fields.links("properties").unwrap_or(&[]) {
            let Some(p) = self.object(Some(link)) else { continue };
            match nif.objects[p].class {
                "NiMaterialProperty" => properties.material = Some(p),
                "NiTexturingProperty" => properties.texturing = Some(p),
                "NiAlphaProperty" => properties.alpha = Some(p),
                "NiStencilProperty" => properties.stencil = Some(p),
                "NiZBufferProperty" => properties.zbuffer = Some(p),
                _ => {}
            }
        }
        let hidden = file_u16(o, "flags") & 1 != 0;
        let extras: Vec<String> = o.fields.links("extra_data_list").unwrap_or(&[]).iter().filter_map(|&l| self.object(Some(l))).map(|e| text(nif.objects[e].fields.string("extra_name").flatten())).collect();
        let team = inherited_team || extras.iter().any(|e| e == "RepColor");
        let [own_team, own_sg, own_sg2] = own_tags(nif, o);
        let mut tags = if is_kind_of(o.class, "NiGeometry") { inherited_tags } else { TagContext::default() };
        tags.shadow |= own_sg || own_sg2;
        tags.team |= own_team && !tags.shadow;
        let lower = name.to_lowercase();
        let shadow = extras.iter().any(|e| e == "Sg" || e == "Sgv") || lower.contains(" sg");
        let selection = lower.starts_with("selectionbox") || lower.starts_with("selection box") || lower.starts_with("collision");
        let data = self.object(o.fields.link("data"));
        if is_kind_of(o.class, "NiTriBasedGeom")
            && let Some(data) = data
        {
            if hidden || shadow || selection {
                self.skipped.push(Skipped { name, reason: "helper/shadow/hidden" });
                return Ok(());
            }
            let meshdata = &nif.objects[data];
            let count = meshdata.fields.u16("vertex_count").unwrap_or(0) as usize;
            let skinned = self.object(o.fields.link("skin_instance")).is_some();
            let has_normals = meshdata.fields.u32("has_normals").unwrap_or(0) != 0;
            let (vertices, normals): (Vec<f64>, Vec<f64>) = if skinned {
                self.deform(o, meshdata, count)?
            } else {
                let words = |name: &str| -> Vec<f64> { meshdata.fields.f32s(name).unwrap_or(&[]).iter().take(count * 3).map(|&w| f64::from(f32::from_bits(w))).collect() };
                (words("vertices"), words("normals"))
            };
            let m = &transform.0;
            let mut positions = Vec::with_capacity(vertices.len());
            for v in vertices.chunks_exact(3) {
                let tx = v[0] * m[0][0] + v[1] * m[1][0] + v[2] * m[2][0] + m[3][0];
                let ty = v[0] * m[0][1] + v[1] * m[1][1] + v[2] * m[2][1] + m[3][1];
                let tz = v[0] * m[0][2] + v[1] * m[1][2] + v[2] * m[2][2] + m[3][2];
                positions.extend([round6(tx)?, round6(tz)?, round6(-ty)?]);
            }
            // A node scaled to nothing draws nothing: the mesh goes, not the whole model.
            let Ok(rotation) = transform.rotation().map(|r| r.0) else {
                self.skipped.push(Skipped { name, reason: "scale 0" });
                return Ok(());
            };
            let mut normal_values = Vec::new();
            if has_normals {
                normal_values.reserve(normals.len());
                for v in normals.chunks_exact(3) {
                    let tx = v[0] * rotation[0][0] + v[1] * rotation[1][0] + v[2] * rotation[2][0];
                    let ty = v[0] * rotation[0][1] + v[1] * rotation[1][1] + v[2] * rotation[2][1];
                    let tz = v[0] * rotation[0][2] + v[1] * rotation[1][2] + v[2] * rotation[2][2];
                    normal_values.extend([round6(tx)?, round6(tz)?, round6(-ty)?]);
                }
            }
            let (material, uv_set) = self.material(&properties, team)?;
            let mut uvs = Vec::new();
            let sets = (meshdata.fields.u16("data_flags").unwrap_or(0) & 0x3f) as usize;
            if sets > 0 {
                let uv_set = uv_set.min(sets - 1);
                let words = meshdata.fields.f32s("uv_sets").unwrap_or(&[]);
                let start = uv_set * count * 2;
                let slice = words.get(start..start + count * 2).ok_or("short uv sets")?;
                uvs.reserve(slice.len());
                for uv in slice.chunks_exact(2) {
                    uvs.push(round6(f64::from(f32::from_bits(uv[0])))?);
                    uvs.push(round6(1.0 - f64::from(f32::from_bits(uv[1])))?);
                }
            }
            let indices = Self::triangles(meshdata)?;
            let vertex_count = (vertices.len() / 3) as u32;
            if indices.iter().any(|&i| i >= vertex_count) {
                return Err("Triangle index outside vertex buffer".into());
            }
            if !positions.is_empty() && !indices.is_empty() {
                let correction = Some(self.correction(tags, &properties, meshdata, m)?);
                self.meshes.push(Mesh { name, hierarchy: path.clone(), positions, normals: normal_values, uvs, indices, material, skinned_pose: skinned, correction });
            }
        }
        let mut children: Vec<u32> = o.fields.links("children").map(<[u32]>::to_vec).unwrap_or_default();
        // The nearest authored LOD is the full detail appearance.
        if is_kind_of(o.class, "NiLODNode") {
            children.truncate(1);
        }
        for link in children {
            let child = self.object(Some(link));
            self.walk(child, Some(&transform), &properties, &path, team, tags)?;
        }
        Ok(())
    }
}

/// One NIF to its [`ModelFile`], `rel` its path below the folder.
pub fn convert_model(rel: &str, bytes: &[u8], index: &ArtIndex) -> Result<ModelFile, String> {
    let nif = nif::load(bytes).map_err(|e| e.to_string())?;
    let mut walk = Walk { nif: &nif, rel, index, seen: vec![false; nif.objects.len()], meshes: Vec::new(), skipped: Vec::new() };
    for &root in &nif.top_level {
        walk.walk(root, None, &Properties::default(), &[], false, TagContext::default())?;
    }
    if walk.meshes.is_empty() {
        return Err("No visible triangle geometry".into());
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for mesh in &walk.meshes {
        for p in mesh.positions.chunks_exact(3) {
            for axis in 0..3 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
        }
    }
    let size = [round6(max[0] - min[0])?, round6(max[1] - min[1])?, round6(max[2] - min[2])?];
    let v = nif.version;
    Ok(ModelFile {
        format: "aa-nif-mesh-v1",
        source: rel.to_owned(),
        nif_version: format!("{}.{}.{}.{}", (v >> 24) & 255, (v >> 16) & 255, (v >> 8) & 255, v & 255),
        up_axis: "Y",
        pose: "authored static pose",
        bounds: Bounds { min, max, size },
        meshes: walk.meshes,
        skipped_helpers: walk.skipped,
    })
}

/// A `(width, height, RGBA)` image.
pub type Rgba = (u32, u32, Vec<u8>);

/// A texture file's first level, as the converted stock textures hold it.
pub fn decode_texture(rel: &str, bytes: &[u8]) -> Result<Rgba, String> {
    if rel.to_lowercase().ends_with(".dds") { decode_dds(bytes) } else { decode_tga(bytes) }
}

/// A TGA goes by its vertical origin bit and its pixel depth: 32 bits hold alpha whatever
/// the descriptor declares.
fn decode_tga(bytes: &[u8]) -> Result<Rgba, String> {
    let mut patched;
    let source = if bytes.len() >= 18 {
        patched = bytes.to_vec();
        patched[17] = (bytes[17] & 32) | if bytes[16] == 32 { 8 } else { 0 };
        &patched[..]
    } else {
        bytes
    };
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(source), image::ImageFormat::Tga);
    reader.no_limits();
    let image = reader.decode().map_err(|e| e.to_string())?.into_rgba8();
    Ok((image.width(), image.height(), image.into_raw()))
}

fn le32(bytes: &[u8], at: usize) -> Result<u32, String> {
    bytes.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| "truncated DDS".to_owned())
}

fn decode_dds(bytes: &[u8]) -> Result<Rgba, String> {
    if bytes.get(..4) != Some(b"DDS ") || le32(bytes, 4)? != 124 {
        return Err("not a DDS".into());
    }
    let (height, width) = (le32(bytes, 12)?, le32(bytes, 16)?);
    let (w, h) = (width as usize, height as usize);
    let flags = le32(bytes, 80)?;
    let fourcc = bytes.get(84..88).ok_or("truncated DDS")?;
    let body = bytes.get(128..).ok_or("truncated DDS")?;
    let mut out = vec![0u8; w * h * 4];
    if flags & 4 != 0 {
        let kind = match fourcc {
            b"DXT1" => 1,
            b"DXT3" => 2,
            b"DXT5" => 3,
            other => return Err(format!("unsupported DDS compression {:?}", String::from_utf8_lossy(other))),
        };
        decode_blocks(kind, body, w, h, &mut out)?;
        return Ok((width, height, out));
    }
    let bits = le32(bytes, 88)? as usize;
    if bits == 0 || bits % 8 != 0 || bits > 32 {
        return Err(format!("unsupported DDS depth {bits}"));
    }
    let masks = [le32(bytes, 92)?, le32(bytes, 96)?, le32(bytes, 100)?, if flags & 1 != 0 { le32(bytes, 104)? } else { 0 }];
    let luminance = flags & 0x20000 != 0;
    if flags & 0x40 == 0 && !luminance {
        return Err("unsupported DDS pixel format".into());
    }
    let stride = bits / 8;
    if body.len() < w * h * stride {
        return Err("truncated DDS".into());
    }
    let channel = |p: u32, mask: u32| -> u32 {
        let max = mask >> mask.trailing_zeros();
        ((p & mask) >> mask.trailing_zeros()) * 255 / max
    };
    for (i, pixel) in body.chunks_exact(stride).take(w * h).enumerate() {
        let mut p = 0u32;
        for (b, &byte) in pixel.iter().enumerate() {
            p |= u32::from(byte) << (8 * b);
        }
        let d = &mut out[i * 4..i * 4 + 4];
        if luminance {
            let l = if masks[0] != 0 { channel(p, masks[0]) } else { p & 255 } as u8;
            d.copy_from_slice(&[l, l, l, if masks[3] != 0 { channel(p, masks[3]) as u8 } else { 255 }]);
        } else {
            for c in 0..4 {
                d[c] = if masks[c] != 0 { channel(p, masks[c]) as u8 } else { 255 };
            }
        }
    }
    Ok((width, height, out))
}

fn decode_565(v: u16) -> [u16; 3] {
    let v = v as i32;
    let r = (v & 0xf800) >> 8;
    let g = (v & 0x07e0) >> 3;
    let b = (v & 0x001f) << 3;
    [(r | (r >> 5)) as u16, (g | (g >> 6)) as u16, (b | (b >> 5)) as u16]
}

/// The DXT blocks: the same integer interpolation as the converted stock textures.
fn decode_blocks(kind: u8, body: &[u8], w: usize, h: usize, out: &mut [u8]) -> Result<(), String> {
    let block_size = if kind == 1 { 8 } else { 16 };
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    if body.len() < bw * bh * block_size {
        return Err("truncated DDS".into());
    }
    for by in 0..bh {
        for bx in 0..bw {
            let block = &body[(by * bw + bx) * block_size..(by * bw + bx + 1) * block_size];
            let colour = &block[if kind == 1 { 0 } else { 8 }..];
            let c0 = u16::from_le_bytes([colour[0], colour[1]]);
            let c1 = u16::from_le_bytes([colour[2], colour[3]]);
            let lut = u32::from_le_bytes([colour[4], colour[5], colour[6], colour[7]]);
            let (a, b) = (decode_565(c0), decode_565(c1));
            let mut palette = [[0u8; 4]; 4];
            for ch in 0..3 {
                palette[0][ch] = a[ch] as u8;
                palette[1][ch] = b[ch] as u8;
            }
            palette[0][3] = 255;
            palette[1][3] = 255;
            palette[2][3] = 255;
            if c0 > c1 || kind != 1 {
                for ch in 0..3 {
                    palette[2][ch] = ((2 * a[ch] + b[ch]) / 3) as u8;
                    palette[3][ch] = ((a[ch] + 2 * b[ch]) / 3) as u8;
                }
                palette[3][3] = 255;
            } else {
                for ch in 0..3 {
                    palette[2][ch] = ((a[ch] + b[ch]) / 2) as u8;
                }
            }
            let mut alpha = [255u8; 16];
            if kind == 2 {
                for (i, a) in alpha.iter_mut().enumerate() {
                    let v = (block[i >> 1] >> ((i & 1) * 4)) & 15;
                    *a = v | (v << 4);
                }
            } else if kind == 3 {
                let (a0, a1) = (u16::from(block[0]), u16::from(block[1]));
                let mut values = [a0 as u8, a1 as u8, 0, 0, 0, 0, 0, 0];
                if a0 > a1 {
                    for i in 1..7u16 {
                        values[i as usize + 1] = (((7 - i) * a0 + i * a1) / 7) as u8;
                    }
                } else {
                    for i in 1..5u16 {
                        values[i as usize + 1] = (((5 - i) * a0 + i * a1) / 5) as u8;
                    }
                    values[6] = 0;
                    values[7] = 255;
                }
                let lut1 = u32::from(block[2]) | (u32::from(block[3]) << 8) | (u32::from(block[4]) << 16);
                let lut2 = u32::from(block[5]) | (u32::from(block[6]) << 8) | (u32::from(block[7]) << 16);
                for i in 0..8 {
                    alpha[i] = values[((lut1 >> (3 * i)) & 7) as usize];
                    alpha[i + 8] = values[((lut2 >> (3 * i)) & 7) as usize];
                }
            }
            for y in 0..4 {
                for x in 0..4 {
                    let (px, py) = (bx * 4 + x, by * 4 + y);
                    if px >= w || py >= h {
                        continue;
                    }
                    let n = y * 4 + x;
                    let mut pixel = palette[((lut >> (2 * n)) & 3) as usize];
                    if kind != 1 {
                        pixel[3] = alpha[n];
                    }
                    out[(py * w + px) * 4..(py * w + px) * 4 + 4].copy_from_slice(&pixel);
                }
            }
        }
    }
    Ok(())
}

fn write_png(path: &Path, image: &Rgba) -> Result<(), String> {
    use image::ImageEncoder;
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let mut bytes = Vec::new();
    PngEncoder::new_with_quality(&mut bytes, CompressionType::Fast, FilterType::Adaptive)
        .write_image(&image.2, image.0, image.1, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    write_whole(path, &bytes)
}

/// The art of one loose data folder, converted when it is asked for: a mod holds thousands of
/// models and tens of thousands of textures, a battle draws a few hundred. Each result stays
/// in `output` for the next battle, by its path below the folder. The folder is only read.
pub struct FolderArt {
    data: PathBuf,
    output: PathBuf,
    index: ArtIndex,
    /// What a file of the folder compares like with the stock data's, once per file.
    same: Mutex<HashMap<String, bool>>,
}

impl std::fmt::Debug for FolderArt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FolderArt({} -> {})", self.data.display(), self.output.display())
    }
}

impl FolderArt {
    pub fn new(data: &Path, output: &Path, stock: Option<&Path>) -> Self {
        Self { data: data.to_path_buf(), output: output.to_path_buf(), index: ArtIndex::new(data, stock), same: Mutex::default() }
    }

    /// The folder's file at `rel` as it lies on disk, unless it is byte for byte the stock
    /// data's: then the stock conversion is its result.
    fn own_file(&self, rel: &str) -> Option<String> {
        let found = self.index.own.by_rel.get(&rel.to_lowercase())?;
        let key = found.to_lowercase();
        if let Some(same) = self.same.lock().ok().and_then(|s| s.get(&key).copied()) {
            return (!same).then(|| found.clone());
        }
        let same = self.index.stock.as_ref().and_then(|stock| {
            let theirs = stock.root.join(stock.by_rel.get(&key)?);
            let ours = self.data.join(found);
            let (a, b) = (fs::metadata(&ours).ok()?, fs::metadata(&theirs).ok()?);
            Some(a.len() == b.len() && fs::read(&ours).ok()? == fs::read(&theirs).ok()?)
        });
        let same = same.unwrap_or(false);
        if let Ok(mut map) = self.same.lock() {
            map.insert(key, same);
        }
        (!same).then(|| found.clone())
    }

    /// The NIF a resource path names in the folder: the path as the data has it, with or
    /// without its extension.
    fn own_model(&self, resource_path: &str) -> Option<String> {
        let path = resource_path.replace('\\', "/");
        self.own_file(&path).or_else(|| self.own_file(&format!("{path}.nif")))
    }

    /// The model is the folder's own, not the stock data's.
    pub fn holds_model(&self, resource_path: &str) -> bool {
        self.own_model(resource_path).is_some()
    }

    /// `None`: the model is the stock data's. Otherwise its converted file, made now if it
    /// is not there or the NIF is newer.
    pub fn model(&self, resource_path: &str) -> Option<Result<PathBuf, String>> {
        let rel = self.own_model(resource_path)?;
        let target = self.output.join("assets/converted/models").join(with_suffix(&rel, ".json"));
        if up_to_date(&target, &self.data.join(&rel)) {
            return Some(Ok(target));
        }
        let made = fs::read(self.data.join(&rel)).map_err(|e| e.to_string()).and_then(|bytes| {
            let model = convert_model(&rel, &bytes, &self.index)?.corrected()?;
            let text = serde_json::to_vec(&model).map_err(|e| e.to_string())?;
            write_whole(&target, &text)
        });
        Some(made.map(|()| target))
    }

    /// `rel` below `assets/converted/textures`: the file, made now from the folder's DDS or
    /// TGA of that name. `None` when the folder holds none or the stock data holds the same.
    pub fn texture(&self, rel: &str) -> Option<PathBuf> {
        let source = [".dds", ".tga"].iter().find_map(|ext| self.own_file(&with_suffix(rel, ext)))?;
        let target = self.output.join("assets/converted/textures").join(with_suffix(&source, ".png"));
        if up_to_date(&target, &self.data.join(&source)) {
            return Some(target);
        }
        let bytes = fs::read(self.data.join(&source)).ok()?;
        let image = decode_texture(&source, &bytes).ok()?;
        write_png(&target, &image).ok()?;
        Some(target)
    }

    /// A file of the folder itself: a PNG a model names, the NIF of a model.
    pub fn original(&self, rel: &str) -> Option<PathBuf> {
        let found = self.index.own.by_rel.get(&rel.to_lowercase())?;
        Some(self.data.join(found))
    }
}

/// `target` exists and is not older than `source`.
fn up_to_date(target: &Path, source: &Path) -> bool {
    let time = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok();
    matches!((time(target), time(source)), (Some(t), Some(s)) if t >= s)
}

/// By a rename, so a reader finds the whole file or none. Two threads may make one file.
fn write_whole(target: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let part = target.with_extension(format!("part{:?}", std::thread::current().id()).replace(['(', ')'], ""));
    fs::write(&part, bytes).map_err(|e| e.to_string())?;
    fs::rename(&part, target).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_is_the_exact_decimal_expansion() {
        assert_eq!(round6(0.1234565).unwrap(), 0.123456);
        assert_eq!(round6(0.1234575).unwrap(), 0.123457);
        assert_eq!(round6(0.0078125).unwrap(), 0.007812);
        assert_eq!(round6(-1.0000004).unwrap(), -1.0);
        assert!(round6(f64::NAN).is_err());
    }

    #[test]
    fn suffixes_replace_the_last_one_only() {
        assert_eq!(with_suffix("a/b.c.DDS", ".png"), "a/b.c.png");
        assert_eq!(with_suffix("a/b", ".png"), "a/b.png");
        assert_eq!(with_suffix("b.nif", ""), "b");
    }

    #[test]
    fn a_dxt1_block_keeps_its_transparent_entry() {
        // c0 < c1: the third colour is the mean, the fourth transparent black.
        let mut block = vec![0u8; 8];
        block[..2].copy_from_slice(&0x0000u16.to_le_bytes());
        block[2..4].copy_from_slice(&0xffffu16.to_le_bytes());
        block[4] = 0b1110_0100;
        let mut out = vec![0u8; 4 * 4 * 4];
        decode_blocks(1, &block, 4, 4, &mut out).unwrap();
        assert_eq!(&out[0..4], &[0, 0, 0, 255]);
        assert_eq!(&out[4..8], &[255, 255, 255, 255]);
        assert_eq!(&out[8..12], &[127, 127, 127, 255]);
        assert_eq!(&out[12..16], &[0, 0, 0, 0]);
    }
}
