//! FX attachment locations (Data/Game/bones*.tgi: Bone IDS -> NIF node
//! name) on the drawn actors. Animated actors use their original rig at the
//! very clip and frame battle_view draws (same clip choice, same 20 Hz frame
//! cache time); other models use the NIF node's bind transform read with the
//! native NiStream port (aa-formats::nif). Everything is render space
//! (native x, y, z -> x, z, -y).
use aa_animation::{AnimatedAsset, Channel, NodeTrack, RotationChannel};
use aa_play::{Activity, ActorId};
use bevy::math::{Mat3, Mat4, Vec3};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// battle_view's animation frame cache rate.
const FRAME_RATE: f32 = 20.0;

/// Native (x, y, z) -> render (x, z, -y) as a matrix.
const TO_RENDER: Mat4 = super::player::NIF_TO_RENDER;

pub struct RigBones {
    asset: AnimatedAsset,
    nodes: HashMap<String, usize>,
    /// First skinned mesh: (mesh node, skin offset, skeleton root).
    skin: Option<(usize, Mat4, usize)>,
    idle: Option<usize>,
    moving: Option<usize>,
    attack: Option<usize>,
    deaths: Vec<usize>,
}

/// Bind-pose node transforms of a static model (render-space model matrices).
pub struct NifBones {
    nodes: HashMap<String, Mat4>,
}

pub enum Source {
    Rig(Box<RigBones>),
    Nif(NifBones),
    None,
}

#[derive(Default)]
pub struct Bones {
    /// Lower-case original NIF path -> rig file (game/ rigs preferred), the
    /// same index battle_view builds.
    rig_files: HashMap<String, PathBuf>,
    data_root: PathBuf,
    cache: HashMap<String, Arc<Source>>,
}

fn rig_source(text: &str) -> Option<String> {
    let at = text.find("\"source\"")?;
    let rest = &text[at + 8..];
    let start = rest.find('"')? + 1;
    let end = rest[start..].find('"')? + start;
    Some(rest[start..end].to_owned())
}

impl Bones {
    pub fn new(rig_dir: Option<&Path>, data_root: PathBuf) -> Self {
        let mut rig_files = HashMap::new();
        if let Some(rig_dir) = rig_dir {
            for dir in [rig_dir.to_path_buf(), rig_dir.join("game")] {
                let Ok(entries) = std::fs::read_dir(&dir) else { continue };
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.to_string_lossy().ends_with(".rig.json") {
                        continue;
                    }
                    // Only the head of the file is needed for the source field.
                    let Ok(text) = read_head(&path) else { continue };
                    if let Some(source) = rig_source(&text) {
                        let key = source.to_ascii_lowercase().replace('\\', "/");
                        let is_game = dir.ends_with("game");
                        if is_game || !rig_files.contains_key(&key) {
                            rig_files.insert(key, path);
                        }
                    }
                }
            }
        }
        Self { rig_files, data_root, cache: HashMap::new() }
    }

    /// The bone source of a model (loaded once).
    pub fn source(&mut self, model: &str) -> Arc<Source> {
        let key = model.to_ascii_lowercase().replace('\\', "/");
        if let Some(s) = self.cache.get(&key) {
            return s.clone();
        }
        let source = if let Some(file) = self.rig_files.get(&key) {
            match AnimatedAsset::load(file) {
                Ok(asset) => Source::Rig(Box::new(RigBones::new(asset))),
                Err(_) => Source::None,
            }
        } else {
            match nif_bones(&self.data_root, &key) {
                Some(b) => Source::Nif(b),
                None => Source::None,
            }
        };
        let source = Arc::new(source);
        self.cache.insert(key, source.clone());
        source
    }
}

fn read_head(path: &Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 4096];
    let n = f.read(&mut buf)?;
    buf.truncate(n);
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

impl RigBones {
    fn new(asset: AnimatedAsset) -> Self {
        let nodes = asset.nodes.iter().enumerate().map(|(i, n)| (n.name.to_ascii_lowercase(), i)).collect();
        let skin = asset.meshes.iter().find_map(|m| m.skin.as_ref().map(|s| (m.node, Mat4::from_cols_array_2d(&s.skin_offset), s.skeleton_root)));
        let find = |names: &[&str]| asset.clips.iter().position(|c| names.iter().any(|n| c.name.eq_ignore_ascii_case(n)));
        let idle = find(&["idle", "InfantryIdle"]);
        let moving = find(&["move", "InfantryJog", "InfantryWalk"]);
        let attack = find(&["attack", "InfantryAttack1"]);
        let deaths = asset
            .clips
            .iter()
            .enumerate()
            .filter(|(_, c)| c.name.to_ascii_lowercase().starts_with("die") || c.name.contains("Die"))
            .map(|(i, _)| i)
            .collect();
        Self { asset, nodes, skin, idle, moving, attack, deaths }
    }

    /// battle_view's `Rig::clip_for` and frame quantisation.
    fn clip_time(&self, actor: ActorId, activity: Activity, since: f32, now: f32) -> Option<(usize, f32)> {
        let phase = (actor as f32 * 0.618_034).fract() * 4.0;
        let (clip, t, looped) = match activity {
            Activity::Moving => (self.moving.or(self.idle)?, now + phase, true),
            Activity::Attacking => (self.attack.or(self.idle)?, now - since, true),
            Activity::Dying | Activity::Dead => {
                let clip = *self.deaths.get(actor as usize % self.deaths.len().max(1))?;
                let t = if activity == Activity::Dead { f32::MAX / 4.0 } else { now - since };
                (clip, t, false)
            }
            _ => (self.idle?, now + phase, true),
        };
        let duration = self.asset.clips.get(clip)?.duration.max(1.0 / FRAME_RATE);
        let t = if looped { t.rem_euclid(duration) } else { t.clamp(0.0, duration) };
        let index = (t * FRAME_RATE).floor();
        Some((clip, (index / FRAME_RATE).clamp(0.0, duration)))
    }

    /// Native-space global matrices of every node at a clip time.
    fn globals(&self, clip: Option<usize>, time: f32) -> Vec<Mat4> {
        let mut tracks: Vec<Option<&NodeTrack>> = vec![None; self.asset.nodes.len()];
        if let Some(c) = clip.and_then(|c| self.asset.clips.get(c)) {
            for t in &c.tracks {
                if t.node < tracks.len() {
                    tracks[t.node] = Some(t);
                }
            }
        }
        let mut globals: Vec<Mat4> = Vec::with_capacity(self.asset.nodes.len());
        for (i, node) in self.asset.nodes.iter().enumerate() {
            let local = local_matrix(node, tracks[i], time);
            let g = node.parent.filter(|&p| p < i).map_or(local, |p| globals[p] * local);
            globals.push(g);
        }
        globals
    }

    /// Render-space model matrix of a node for what battle_view draws.
    pub fn node(&self, name: &str, actor: ActorId, activity: Activity, since: f32, now: f32) -> Option<Mat4> {
        let index = *self.nodes.get(&name.to_ascii_lowercase())?;
        let (clip, t) = self.clip_time(actor, activity, since, now).map_or((None, 0.0), |(c, t)| (Some(c), t));
        let g = self.globals(clip, t);
        let mut m = g[index];
        // Skinned geometry is drawn as G(mesh) * offset * G(root)^-1 * G(bone).
        if let Some((mesh, offset, root)) = self.skin
            && mesh < g.len()
            && root < g.len()
            && g[root].determinant().abs() > 1e-12
        {
            m = g[mesh] * offset * g[root].inverse() * m;
        }
        Some(TO_RENDER * m * TO_RENDER.inverse())
    }
}

/// aa-animation's linear key sampling (lib.rs `interval` / `local_matrix`).
fn interval<T>(channel: &Channel<T>, time: f32) -> Option<(&T, &T, f32)> {
    let keys = &channel.keys;
    let first = keys.first()?;
    let next = keys.partition_point(|k| k.time <= time);
    if next == 0 {
        return Some((&first.value, &first.value, 0.0));
    }
    if next == keys.len() {
        let last = &keys[next - 1].value;
        return Some((last, last, 0.0));
    }
    let a = &keys[next - 1];
    let b = &keys[next];
    Some((&a.value, &b.value, (time - a.time) / (b.time - a.time)))
}
fn quaternion_matrix([x, y, z, w]: [f32; 4]) -> Mat3 {
    Mat3::from_cols(
        Vec3::new(1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y + w * z), 2.0 * (x * z - w * y)),
        Vec3::new(2.0 * (x * y - w * z), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z + w * x)),
        Vec3::new(2.0 * (x * z + w * y), 2.0 * (y * z - w * x), 1.0 - 2.0 * (x * x + y * y)),
    )
}
fn slerp(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
    if std::ptr::eq(a, b) {
        return *a;
    }
    let a = a.map(f64::from);
    let mut b = b.map(f64::from);
    let t = f64::from(t);
    let mut dot = a.iter().zip(b).map(|(v, w)| v * w).sum::<f64>();
    if dot < 0.0 {
        b = b.map(|v| -v);
        dot = -dot;
    }
    dot = dot.clamp(-1.0, 1.0);
    let q: [f64; 4] = if dot < 0.9995 {
        let theta = dot.acos();
        let sin = theta.sin();
        std::array::from_fn(|i| (((1.0 - t) * theta).sin() * a[i] + (t * theta).sin() * b[i]) / sin)
    } else {
        std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
    };
    let length = q.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-12);
    q.map(|v| (v / length) as f32)
}
fn local_matrix(node: &aa_animation::RigNode, track: Option<&NodeTrack>, time: f32) -> Mat4 {
    let mut translation = Vec3::from(node.translation);
    let mut scale = node.scale;
    let mut rotation = Mat3::from_cols_array_2d(&node.rotation);
    if let Some(track) = track {
        let time = time.clamp(track.start, track.stop);
        if let Some((a, b, t)) = interval(&track.translation, time) {
            translation = Vec3::from(*a).lerp(Vec3::from(*b), t);
        }
        if let Some((a, b, t)) = interval(&track.scale, time) {
            scale = a + (b - a) * t;
        }
        if let RotationChannel::Quaternion { channel } = &track.rotation
            && let Some((a, b, t)) = interval(channel, time)
        {
            rotation = quaternion_matrix(slerp(a, b, t));
        }
    }
    Mat4::from_cols((rotation.x_axis * scale).extend(0.0), (rotation.y_axis * scale).extend(0.0), (rotation.z_axis * scale).extend(0.0), translation.extend(1.0))
}

/// Case-insensitive file lookup below `root` for a Data-relative path.
pub fn find_file(root: &Path, rel: &str) -> Option<PathBuf> {
    let direct = root.join(rel);
    if direct.exists() {
        return Some(direct);
    }
    let mut dir = root.to_path_buf();
    for part in rel.split(['/', '\\']).filter(|p| !p.is_empty()) {
        let entries = std::fs::read_dir(&dir).ok()?;
        let found = entries.flatten().find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
        dir = found.path();
    }
    Some(dir)
}

/// Bind-pose node transforms of an original NIF (native NiStream port).
fn nif_bones(data_root: &Path, rel: &str) -> Option<NifBones> {
    use aa_formats::nif::Value;
    let path = find_file(data_root, rel)?;
    let bytes = std::fs::read(path).ok()?;
    let file = aa_formats::nif::load(&bytes).ok()?;
    let floats = |v: Option<&Value>| -> Option<Vec<f32>> {
        match v? {
            Value::F32s(w) => Some(w.iter().map(|b| f32::from_bits(*b)).collect()),
            Value::F32(b) => Some(vec![f32::from_bits(*b)]),
            _ => None,
        }
    };
    let mut nodes = HashMap::new();
    let mut stack: Vec<(usize, Mat4)> = file.top_level.iter().flatten().map(|&i| (i, Mat4::IDENTITY)).collect();
    let mut seen = vec![false; file.objects.len()];
    while let Some((i, parent)) = stack.pop() {
        if i >= file.objects.len() || seen[i] {
            continue;
        }
        seen[i] = true;
        let o = &file.objects[i];
        let t = floats(o.fields.get("translation")).filter(|v| v.len() >= 3);
        let r = floats(o.fields.get("rotation")).filter(|v| v.len() >= 9);
        let s = floats(o.fields.get("scale")).and_then(|v| v.first().copied()).unwrap_or(1.0);
        let local = match (t, r) {
            (Some(t), Some(r)) => {
                let rot = Mat3::from_cols_array(&[r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7], r[8]]);
                Mat4::from_cols((rot.x_axis * s).extend(0.0), (rot.y_axis * s).extend(0.0), (rot.z_axis * s).extend(0.0), Vec3::new(t[0], t[1], t[2]).extend(1.0))
            }
            _ => Mat4::IDENTITY,
        };
        let world = parent * local;
        if let Some(Value::Str(Some(name))) = o.fields.get("name") {
            let name = String::from_utf8_lossy(name).to_ascii_lowercase();
            nodes.entry(name).or_insert(TO_RENDER * world * TO_RENDER.inverse());
        }
        if let Some(children) = file.children.get(i) {
            for c in children.iter().flatten() {
                stack.push((*c, world));
            }
        }
    }
    (!nodes.is_empty()).then_some(NifBones { nodes })
}

impl Source {
    /// Render-space model matrix of the bone `name` (NIF node name).
    pub fn node(&self, name: &str, actor: ActorId, activity: Activity, since: f32, now: f32) -> Option<Mat4> {
        match self {
            Source::Rig(r) => r.node(name, actor, activity, since, now),
            Source::Nif(n) => n.nodes.get(&name.to_ascii_lowercase()).copied(),
            Source::None => None,
        }
    }
}
