//! Original NIF/KF inspector playback and bounded native Character runtime.
//! Runtime timing/selection is explicit; world scheduling, rendered blending,
//! resource acquisition and concrete FX dispatch remain integration boundaries.
use aa_assets::{Bounds, Material, native_to_render};
use glam::{Mat3, Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub mod controller_clock;
pub mod definitions;
pub mod runtime;

pub type Result<T> = std::result::Result<T, AnimationError>;
#[derive(Debug, thiserror::Error)]
pub enum AnimationError {
    #[error("animation file {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("animation JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid/unsupported original animation: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RigNode {
    pub name: String,
    pub parent: Option<usize>,
    pub translation: [f32; 3],
    /// Original NIF row vectors become columns for Rust's column-vector math.
    pub rotation: [[f32; 3]; 3],
    pub scale: f32,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkinWeight {
    pub vertex: usize,
    pub weight: f32,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkinBone {
    pub node: usize,
    pub inverse_bind: [[f32; 4]; 4],
    pub weights: Vec<SkinWeight>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Skin {
    pub skeleton_root: usize,
    pub skin_offset: [[f32; 4]; 4],
    pub bones: Vec<SkinBone>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RigMesh {
    pub name: String,
    /// Original flattened-converter mesh index; optional for legacy Phase2 caches.
    #[serde(default)]
    pub source_mesh_index: Option<usize>,
    pub node: usize,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub material: Material,
    pub skin: Option<Skin>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Key<T> {
    pub time: f32,
    pub value: T,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Channel<T> {
    pub interpolation: u32,
    pub keys: Vec<Key<T>>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RotationChannel {
    Quaternion {
        channel: Channel<[f32; 4]>,
    },
    /// A constant Euler spline proven equal to the original bind rotation.
    /// Nonconstant/unequal Euler data is rejected by the exporter.
    ConstantBind {
        euler_xyz: [f32; 3],
        max_bind_matrix_delta: f32,
    },
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeTrack {
    pub node: usize,
    pub target: String,
    pub controller: String,
    pub frequency: f32,
    pub phase: f32,
    pub start: f32,
    pub stop: f32,
    pub flags: u32,
    pub translation: Channel<[f32; 3]>,
    pub scale: Channel<f32>,
    pub rotation: RotationChannel,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextEvent {
    pub time: f32,
    pub text: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Clip {
    pub name: String,
    pub source: String,
    pub source_sha256: String,
    pub duration: f32,
    /// None when this NIF version does not serialize a sequence cycle field.
    /// Individual controller flags are retained separately; inspector playback
    /// mode is explicit and does not claim the native controller state machine.
    pub cycle_type: Option<u32>,
    pub tracks: Vec<NodeTrack>,
    pub events: Vec<TextEvent>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AnimatedAsset {
    pub format: String,
    pub source: String,
    pub source_sha256: String,
    #[serde(default)]
    pub static_model_sha256: Option<String>,
    pub coordinates: String,
    pub nif_version: String,
    pub nodes: Vec<RigNode>,
    pub meshes: Vec<RigMesh>,
    pub clips: Vec<Clip>,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackMode {
    Clamp,
    Loop,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PosedMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SampledPose {
    pub time_seconds: f32,
    pub meshes: Vec<PosedMesh>,
    pub bounds: Bounds,
}

fn invalid(message: impl Into<String>) -> AnimationError {
    AnimationError::Invalid(message.into())
}
fn finite(values: impl IntoIterator<Item = f32>) -> bool {
    values.into_iter().all(f32::is_finite)
}
fn matrix_valid(matrix: &[[f32; 4]; 4]) -> bool {
    finite(matrix.iter().flatten().copied())
        && Mat4::from_cols_array_2d(matrix).determinant().abs() > 1e-12
}
fn channel_valid<T>(channel: &Channel<T>, check: impl Fn(&T) -> bool) -> bool {
    (channel.keys.is_empty() || channel.interpolation == 1)
        && channel
            .keys
            .iter()
            .all(|k| k.time.is_finite() && check(&k.value))
        && channel.keys.windows(2).all(|p| p[0].time < p[1].time)
}
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
fn sample_scalar(channel: &Channel<f32>, time: f32) -> Option<f32> {
    interval(channel, time).map(|(&a, &b, t)| a + (b - a) * t)
}
fn sample_vector(channel: &Channel<[f32; 3]>, time: f32) -> Option<Vec3> {
    interval(channel, time).map(|(a, b, t)| Vec3::from(*a).lerp(Vec3::from(*b), t))
}
fn sample_quaternion(channel: &Channel<[f32; 4]>, time: f32) -> Option<[f32; 4]> {
    interval(channel, time).map(|(a, b, t)| {
        // Preserve values in clamped intervals (including the final key). Exact
        // first/interior key times still enter interpolation and normalization.
        // The source Attack1 calf final key is slightly non-unit; silently
        // normalizing that final value changes the source/reference pose.
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
        let q = if dot < 0.9995 {
            let theta = dot.acos();
            let sin = theta.sin();
            std::array::from_fn(|i| {
                (((1.0 - t) * theta).sin() * a[i] + (t * theta).sin() * b[i]) / sin
            })
        } else {
            std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
        };
        let length = q.iter().map(|v| v * v).sum::<f64>().sqrt();
        q.map(|v| (v / length) as f32)
    })
}
fn quaternion_matrix([x, y, z, w]: [f32; 4]) -> Mat3 {
    // NIF's row-vector quaternion matrix, transposed by supplying its rows as
    // columns. Unlike a rotation constructor this preserves non-unit endpoints.
    Mat3::from_cols(
        Vec3::new(
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y + w * z),
            2.0 * (x * z - w * y),
        ),
        Vec3::new(
            2.0 * (x * y - w * z),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z + w * x),
        ),
        Vec3::new(
            2.0 * (x * z + w * y),
            2.0 * (y * z - w * x),
            1.0 - 2.0 * (x * x + y * y),
        ),
    )
}
fn local_matrix(node: &RigNode, track: Option<&NodeTrack>, time: f32) -> Mat4 {
    let mut translation = Vec3::from(node.translation);
    let mut scale = node.scale;
    let mut rotation = Mat3::from_cols_array_2d(&node.rotation);
    if let Some(track) = track {
        let time = time.clamp(track.start, track.stop);
        if let Some(v) = sample_vector(&track.translation, time) {
            translation = v;
        }
        if let Some(v) = sample_scalar(&track.scale, time) {
            scale = v;
        }
        if let RotationChannel::Quaternion { channel } = &track.rotation
            && let Some(q) = sample_quaternion(channel, time)
        {
            rotation = quaternion_matrix(q);
        }
    }
    Mat4::from_cols(
        (rotation.x_axis * scale).extend(0.0),
        (rotation.y_axis * scale).extend(0.0),
        (rotation.z_axis * scale).extend(0.0),
        translation.extend(1.0),
    )
}
fn rotate_without_scale(matrix: Mat4, normal: Vec3) -> Vec3 {
    // The original rig uses uniform positive scale. Remove it from the skin
    // matrix before normal accumulation, matching the reference skin routine.
    matrix.transform_vector3(normal) / Mat3::from_mat4(matrix).determinant().cbrt()
}

impl AnimatedAsset {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| AnimationError::Io {
            path: path.into(),
            source,
        })?;
        let asset: Self = serde_json::from_slice(&bytes)?;
        asset.validate()?;
        Ok(asset)
    }
    pub fn validate(&self) -> Result<()> {
        if self.format != "aa-original-rig-v1" || self.nodes.is_empty() || self.meshes.is_empty() {
            return Err(invalid("format or empty rig"));
        }
        for (i, node) in self.nodes.iter().enumerate() {
            if node.parent.is_some_and(|p| p >= i)
                || !finite(node.translation)
                || !finite(node.rotation.iter().flatten().copied())
                || !node.scale.is_finite()
                || node.scale <= 0.0
                || Mat3::from_cols_array_2d(&node.rotation).determinant().abs() < 1e-8
            {
                return Err(invalid(format!("node hierarchy/transform: {}", node.name)));
            }
        }
        let mut source_mesh_indices = std::collections::BTreeSet::new();
        for mesh in &self.meshes {
            if mesh
                .source_mesh_index
                .is_some_and(|index| !source_mesh_indices.insert(index))
            {
                return Err(invalid("duplicate original converted-mesh index"));
            }
            let count = mesh.positions.len();
            if mesh.node >= self.nodes.len()
                || count == 0
                || mesh.indices.is_empty()
                || !mesh.indices.len().is_multiple_of(3)
                || mesh.indices.iter().any(|&i| i as usize >= count)
                || (!mesh.normals.is_empty() && mesh.normals.len() != count)
                || (!mesh.uvs.is_empty() && mesh.uvs.len() != count)
                || !finite(mesh.positions.iter().flatten().copied())
                || !finite(mesh.normals.iter().flatten().copied())
                || !finite(mesh.uvs.iter().flatten().copied())
            {
                return Err(invalid(format!("mesh arrays: {}", mesh.name)));
            }
            if let Some(skin) = &mesh.skin {
                if skin.skeleton_root >= self.nodes.len()
                    || !matrix_valid(&skin.skin_offset)
                    || skin.bones.is_empty()
                {
                    return Err(invalid("skin root/offset"));
                }
                let mut weights = vec![0.0_f64; count];
                for bone in &skin.bones {
                    if bone.node >= self.nodes.len()
                        || bone.node == skin.skeleton_root
                        || !matrix_valid(&bone.inverse_bind)
                    {
                        return Err(invalid("skin bone/inverse bind"));
                    }
                    let mut parent = Some(bone.node);
                    let mut connected = false;
                    while let Some(p) = parent {
                        if p == skin.skeleton_root {
                            connected = true;
                            break;
                        }
                        parent = self.nodes[p].parent;
                    }
                    if !connected {
                        return Err(invalid("bone is not beneath skeleton root"));
                    }
                    for w in &bone.weights {
                        if w.vertex >= count
                            || !w.weight.is_finite()
                            || !(0.0..=1.0).contains(&w.weight)
                        {
                            return Err(invalid("skin vertex/weight"));
                        }
                        weights[w.vertex] += f64::from(w.weight);
                    }
                }
                if weights.iter().any(|sum| (sum - 1.0).abs() > 0.01) {
                    return Err(invalid("skin weights do not sum to one"));
                }
            }
        }
        for clip in &self.clips {
            if !clip.duration.is_finite()
                || clip.duration <= 0.0
                || clip.events.iter().any(|e| !e.time.is_finite())
            {
                return Err(invalid("clip duration/events"));
            }
            let mut targets = std::collections::BTreeSet::new();
            for t in &clip.tracks {
                if t.node >= self.nodes.len()
                    || self.nodes[t.node].name != t.target
                    || !targets.insert(t.node)
                    || t.controller != "NiKeyframeController"
                    || t.frequency != 1.0
                    || t.phase != 0.0
                    || !finite([t.start, t.stop])
                    || t.stop < t.start
                    || !channel_valid(&t.translation, |v| finite(*v))
                    || !channel_valid(&t.scale, |v| v.is_finite() && *v > 0.0)
                {
                    return Err(invalid(format!("unsupported track {}", t.target)));
                }
                match &t.rotation {
                    RotationChannel::Quaternion { channel } => {
                        if !channel_valid(channel, |q| {
                            let norm = Quat::from_array(*q).length_squared();
                            finite(*q) && norm.is_finite() && norm > 1e-10
                        }) {
                            return Err(invalid("quaternion keys"));
                        }
                    }
                    RotationChannel::ConstantBind {
                        euler_xyz,
                        max_bind_matrix_delta,
                    } => {
                        let computed = Mat3::from_rotation_z(euler_xyz[2])
                            * Mat3::from_rotation_y(euler_xyz[1])
                            * Mat3::from_rotation_x(euler_xyz[0]);
                        let actual = Mat3::from_cols_array_2d(&self.nodes[t.node].rotation);
                        let delta = computed
                            .to_cols_array()
                            .into_iter()
                            .zip(actual.to_cols_array())
                            .map(|(a, b)| (a - b).abs())
                            .fold(0.0_f32, f32::max);
                        if !finite(*euler_xyz)
                            || !max_bind_matrix_delta.is_finite()
                            || !(0.0..=1e-6).contains(max_bind_matrix_delta)
                            || delta > 1e-6
                        {
                            return Err(invalid("unproven constant Euler bind equivalence"));
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub fn sample(
        &self,
        clip_index: usize,
        time_seconds: f32,
        mode: PlaybackMode,
    ) -> Result<SampledPose> {
        self.validate()?;
        if !time_seconds.is_finite() {
            return Err(invalid("non-finite playback time"));
        }
        let clip = self
            .clips
            .get(clip_index)
            .ok_or_else(|| invalid("clip index"))?;
        let time = match mode {
            PlaybackMode::Clamp => time_seconds.clamp(0.0, clip.duration),
            PlaybackMode::Loop => time_seconds.rem_euclid(clip.duration),
        };
        self.pose(Some(clip), time)
    }
    pub fn sample_bind(&self) -> Result<SampledPose> {
        self.validate()?;
        self.pose(None, 0.0)
    }
    /// Evaluated native controller times, one per track. This does not choose
    /// clamp/loop policy or derive gameplay time from the inspector clock.
    pub fn sample_controller_times(&self, clip_index: usize, times: &[f32]) -> Result<SampledPose> {
        self.validate()?;
        let clip = self
            .clips
            .get(clip_index)
            .ok_or_else(|| invalid("clip index"))?;
        if times.len() != clip.tracks.len() || times.iter().any(|t| !t.is_finite()) {
            return Err(invalid("controller times differ from clip tracks"));
        }
        self.pose_times(Some(clip), 0.0, Some(times))
    }
    fn pose(&self, clip: Option<&Clip>, time: f32) -> Result<SampledPose> {
        self.pose_times(clip, time, None)
    }
    fn pose_times(
        &self,
        clip: Option<&Clip>,
        time: f32,
        times: Option<&[f32]>,
    ) -> Result<SampledPose> {
        // Public entry points validate mutable input before indexing.
        let mut tracks = vec![None; self.nodes.len()];
        if let Some(clip) = clip {
            for (i, track) in clip.tracks.iter().enumerate() {
                tracks[track.node] = Some((track, times.map_or(time, |v| v[i])));
            }
        }
        let mut globals: Vec<Mat4> = Vec::with_capacity(self.nodes.len());
        for (index, node) in self.nodes.iter().enumerate() {
            let selected = tracks[index];
            let local = local_matrix(node, selected.map(|v| v.0), selected.map_or(time, |v| v.1));
            globals.push(node.parent.map_or(local, |parent| globals[parent] * local));
        }
        let mut meshes = Vec::with_capacity(self.meshes.len());
        for mesh in &self.meshes {
            let global = globals[mesh.node];
            let mut positions = vec![Vec3::ZERO; mesh.positions.len()];
            let mut normals = vec![Vec3::ZERO; mesh.positions.len()];
            if let Some(skin) = &mesh.skin {
                let root_inverse = globals[skin.skeleton_root].inverse();
                let offset = Mat4::from_cols_array_2d(&skin.skin_offset);
                for bone in &skin.bones {
                    let bone_transform = offset
                        * root_inverse
                        * globals[bone.node]
                        * Mat4::from_cols_array_2d(&bone.inverse_bind);
                    for weight in &bone.weights {
                        positions[weight.vertex] += bone_transform
                            .transform_point3(Vec3::from(mesh.positions[weight.vertex]))
                            * weight.weight;
                        if !mesh.normals.is_empty() {
                            normals[weight.vertex] += rotate_without_scale(
                                bone_transform,
                                Vec3::from(mesh.normals[weight.vertex]),
                            ) * weight.weight;
                        }
                    }
                }
            } else {
                for (target, &source) in positions.iter_mut().zip(&mesh.positions) {
                    *target = Vec3::from(source);
                }
                for (target, &source) in normals.iter_mut().zip(&mesh.normals) {
                    *target = Vec3::from(source);
                }
            }
            let positions: Vec<_> = positions
                .into_iter()
                .map(|p| native_to_render(global.transform_point3(p).to_array()))
                .collect();
            let normals: Vec<_> = normals
                .into_iter()
                .map(|n| {
                    native_to_render(
                        rotate_without_scale(global, n)
                            .normalize_or_zero()
                            .to_array(),
                    )
                })
                .collect();
            if !finite(positions.iter().flatten().copied())
                || !finite(normals.iter().flatten().copied())
            {
                return Err(invalid("non-finite sampled geometry"));
            }
            meshes.push(PosedMesh { positions, normals });
        }
        let bounds = Bounds::from_positions(meshes.iter().flat_map(|m| m.positions.iter()))
            .ok_or_else(|| invalid("empty sampled model"))?;
        Ok(SampledPose {
            time_seconds: time,
            meshes,
            bounds,
        })
    }
}
