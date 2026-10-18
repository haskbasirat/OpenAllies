//! Baked vertex animation for GPU-instanced unit playback.
//!
//! [`bake`] samples every gameplay-role clip of an original rig cache
//! (`aa-original-rig-v1`, see `aa_animation::AnimatedAsset`) at a fixed rate and
//! stores the posed vertices of every rig mesh, so an instanced draw only needs
//! a per-instance frame index ([`BakedRig::frame`]) and one storage buffer per
//! mesh. Positions/normals are model-local in the render basis `(x, z, -y)`,
//! identical to the converted model JSON vertices loaded by
//! `aa_assets::AssetStore::load_model`. Mesh `i` of a baked rig corresponds to
//! converted mesh `source_mesh_index` (game rigs) or, for the legacy five
//! Phase 2 rigs that carry no index, to converted mesh `i`.
//!
//! Timing: `time` arguments are gameplay seconds since the clip started. Each
//! clip covers its catalog gameplay length (`gameplay_length`, e.g. Idle 3 s,
//! Die3 4 s) and maps it onto the KF controller duration exactly like the
//! native `kf_duration / gameplay_length` rate. Move clips additionally scale by
//! `speed / (visual_scale * walk_distance)` natively; callers that want
//! stride-matched feet pass `elapsed * speed / (visual_scale * walk_distance)`.
//! Idle/Move loop, Attack/Die clamp on their final pose.
//!
//! Model paths are matched case-insensitively ([`model_key`]): the gameplay
//! manifest stores aa-game's catalog spelling (e.g.
//! `Units/USA/USAInfantryMG/USAInfantryMG.nif`) while some original files are
//! spelled differently on disk (`USAInfantryMG.NIF`).
#![allow(dead_code)] // Public baking API; wired into the renderer separately.

use aa_animation::{AnimatedAsset, PlaybackMode};
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    Idle,
    Move,
    Attack,
    Die,
}

impl Role {
    pub const ALL: [Role; 4] = [Role::Idle, Role::Move, Role::Attack, Role::Die];

    /// Native `canonical_looping_type`: idle and move loop; the rest clamp.
    pub fn loops(self) -> bool {
        matches!(self, Role::Idle | Role::Move)
    }

    /// Role of a clip name. Game rigs use `idle`, `move`, `attack`,
    /// `die1`..`die3`; the legacy Phase 2 rigs use `InfantryIdle`,
    /// `InfantryJog`, `InfantryAttack1` and `InfantryDie1`.
    pub fn from_clip_name(name: &str) -> Option<Role> {
        let lower = name.trim().to_ascii_lowercase();
        let n = lower.strip_prefix("infantry").unwrap_or(&lower);
        match n {
            "idle" => Some(Role::Idle),
            "move" | "jog" | "walk" => Some(Role::Move),
            "attack" | "attack1" => Some(Role::Attack),
            _ if n.starts_with("die") && n[3..].chars().all(|c| c.is_ascii_digit()) => {
                Some(Role::Die)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BakedClip {
    pub role: Role,
    /// Clip name in the rig (`idle`, `move`, `attack`, `die1`, ... or legacy names).
    pub name: String,
    /// First frame of this clip in the concatenated frame list.
    pub first_frame: u32,
    pub frames: u32,
    /// Frames per gameplay second for this clip. A shader can reproduce
    /// [`BakedClip::local_frame`]: `i = floor(t * fps + 0.5)`, then
    /// `i mod frames` when looping, else `min(i, frames - 1)`.
    pub fps: f32,
    pub looping: bool,
    /// Gameplay seconds covered by the clip (catalog `length`, else KF duration).
    pub length: f32,
    /// Original KF path the clip was exported from.
    pub source: String,
    /// Catalog `walk_distance` (1.0 when absent); see module docs for Move.
    pub walk_distance: f32,
}

impl BakedClip {
    /// Frame within this clip (0-based) at gameplay seconds `time`.
    pub fn local_frame(&self, time: f32) -> u32 {
        if self.frames <= 1 || !time.is_finite() {
            return 0;
        }
        let index = (f64::from(time) * f64::from(self.fps) + 0.5).floor();
        if self.looping {
            let frames = i64::from(self.frames);
            // Saturating cast is fine: huge times only lose sub-frame phase.
            (index as i64).rem_euclid(frames) as u32
        } else {
            index.clamp(0.0, f64::from(self.frames - 1)) as u32
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BakedMesh {
    pub name: String,
    /// Index into the converted model's `meshes`; `None` for legacy rigs
    /// (whose mesh order equals the converted model's).
    pub source_mesh_index: Option<usize>,
    pub vertex_count: u32,
    /// frames * vertex_count entries, frame-major: [pos.x,pos.y,pos.z, nrm.x,nrm.y,nrm.z]
    pub frames: Vec<[f32; 6]>,
}

impl BakedMesh {
    /// Vertices of one absolute frame (see [`BakedRig::frame`]).
    pub fn frame(&self, frame: u32) -> &[[f32; 6]] {
        let n = self.vertex_count as usize;
        let start = frame as usize * n;
        &self.frames[start..start + n]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BakedRig {
    /// Original model path of the rig (`AnimatedAsset::source`, on-disk spelling).
    pub model: String,
    pub meshes: Vec<BakedMesh>,
    pub clips: Vec<BakedClip>,
    pub total_frames: u32,
}

impl BakedRig {
    /// Frame index (into the concatenated frame list) for a role at a playback time; picks a variant
    /// deterministically from `variant` when several clips share the role (e.g. die1..3); clamps or loops per clip.
    ///
    /// A missing role falls back to Idle, then to the first clip (frame 0 of
    /// an empty rig); use [`BakedRig::has_role`] to detect that.
    pub fn frame(&self, role: Role, time: f32, variant: u32) -> u32 {
        self.clip(role, variant)
            .or_else(|| self.clip(Role::Idle, 0))
            .or_else(|| self.clips.first())
            .map_or(0, |clip| clip.first_frame + clip.local_frame(time))
    }

    pub fn has_role(&self, role: Role) -> bool {
        self.clips.iter().any(|c| c.role == role)
    }

    /// Number of clips sharing `role` (e.g. 3 for die1..die3).
    pub fn variants(&self, role: Role) -> u32 {
        self.clips.iter().filter(|c| c.role == role).count() as u32
    }

    /// Clip chosen for `role` and `variant` (`variant % variants(role)`, rig order).
    pub fn clip(&self, role: Role, variant: u32) -> Option<&BakedClip> {
        let count = self.variants(role);
        if count == 0 {
            return None;
        }
        self.clips
            .iter()
            .filter(|c| c.role == role)
            .nth((variant % count) as usize)
    }

    /// CPU bytes held by the baked vertex frames.
    pub fn bytes(&self) -> usize {
        self.meshes
            .iter()
            .map(|m| m.frames.len() * std::mem::size_of::<[f32; 6]>())
            .sum()
    }

    pub fn vertex_count(&self) -> u32 {
        self.meshes.iter().map(|m| m.vertex_count).sum()
    }
}

/// Lowercase, forward-slash, no leading slash: the key used to compare model
/// resource paths (the same normalization as `AssetStore::model_url`).
pub fn model_key(model: &str) -> String {
    model
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_lowercase()
}

#[derive(Deserialize)]
struct GameManifest {
    format: String,
    rigs: Vec<GameEntry>,
}
#[derive(Deserialize)]
struct GameEntry {
    model: String,
    file: String,
}
#[derive(Deserialize)]
struct LegacyManifest {
    assets: Vec<LegacyEntry>,
}
#[derive(Deserialize)]
struct LegacyEntry {
    source: String,
    file: String,
}

fn animation_dir(cache_dir: &Path) -> PathBuf {
    let nested = cache_dir.join("animation");
    if nested.is_dir() {
        nested
    } else {
        cache_dir.to_path_buf()
    }
}

fn safe_relative(file: &str) -> Option<PathBuf> {
    let path = PathBuf::from(file.replace('\\', "/"));
    path.components()
        .all(|c| matches!(c, Component::Normal(_)))
        .then_some(path)
}

fn read_manifest<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Lists gameplay rigs: (converted model resource path as used by aa_assets::AssetStore::load_model, rig file path).
/// Reads assets-cache/animation/game-rigs.json, falling back to the original 5-rig manifest.json (map their clips: InfantryIdle->Idle, InfantryJog->Move, InfantryAttack1->Attack, InfantryDie1->Die).
///
/// Sorted by [`model_key`]; entries whose rig file is missing are skipped.
/// Compare model paths with [`model_key`] (or use [`find`]).
pub fn available(cache_dir: &Path) -> Vec<(String, PathBuf)> {
    let dir = animation_dir(cache_dir);
    let entries: Vec<(String, String)> =
        match read_manifest::<GameManifest>(&dir.join("game-rigs.json")) {
            Some(m) if m.format == "aa-game-rig-manifest-v1" => {
                m.rigs.into_iter().map(|r| (r.model, r.file)).collect()
            }
            _ => read_manifest::<LegacyManifest>(&dir.join("manifest.json"))
                .map(|m| m.assets.into_iter().map(|a| (a.source, a.file)).collect())
                .unwrap_or_default(),
        };
    let mut out: Vec<(String, PathBuf)> = entries
        .into_iter()
        .filter_map(|(model, file)| {
            let path = dir.join(safe_relative(&file)?);
            path.is_file().then_some((model, path))
        })
        .collect();
    out.sort_by_cached_key(|(model, _)| model_key(model));
    out.dedup_by(|a, b| model_key(&a.0) == model_key(&b.0));
    out
}

/// Case-insensitive lookup of a model resource path in [`available`].
pub fn find(cache_dir: &Path, model: &str) -> Option<PathBuf> {
    let key = model_key(model);
    available(cache_dir)
        .into_iter()
        .find(|(m, _)| model_key(m) == key)
        .map(|(_, path)| path)
}

/// Gameplay fields the exporter adds next to the aa_animation clip schema.
#[derive(Deserialize)]
struct RigExtras {
    #[serde(default)]
    clips: Vec<ClipExtras>,
}
#[derive(Deserialize)]
struct ClipExtras {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    gameplay_length: Option<f32>,
    #[serde(default)]
    walk_distance: Option<f32>,
    #[serde(default)]
    looping: Option<bool>,
}

struct Plan {
    clip_index: usize,
    steps: u32,
    clip: BakedClip,
}

/// Load a rig and bake every clip at `fps` (e.g. 15). Positions/normals are in the same model-local render basis as the converted model's vertices.
///
/// Clips whose name maps to no [`Role`] are not baked. A looping clip gets
/// `round(length * fps)` frames (the end pose equals the start and is not
/// repeated); a clamped clip gets one more so its final pose is exact.
pub fn bake(rig_path: &Path, fps: f32) -> Result<BakedRig> {
    ensure!(
        fps.is_finite() && fps > 0.0 && fps <= 240.0,
        "Bake rate must be within (0, 240] frames per second"
    );
    let bytes =
        std::fs::read(rig_path).with_context(|| format!("Reading rig {}", rig_path.display()))?;
    let asset: AnimatedAsset = serde_json::from_slice(&bytes)
        .with_context(|| format!("Parsing rig {}", rig_path.display()))?;
    // `sample` validates again on every call; this up-front check only gives a
    // clearer error before allocation.
    asset
        .validate()
        .with_context(|| format!("Validating rig {}", rig_path.display()))?;
    let extras: RigExtras = serde_json::from_slice(&bytes)?;
    ensure!(
        extras.clips.is_empty() || extras.clips.len() == asset.clips.len(),
        "Rig clip extras do not match clip list"
    );

    let mut plans = Vec::new();
    let mut first_frame = 0u32;
    for (clip_index, clip) in asset.clips.iter().enumerate() {
        let extra = extras.clips.get(clip_index);
        let role = Role::from_clip_name(&clip.name).or_else(|| {
            extra
                .and_then(|e| e.role.as_deref())
                .and_then(Role::from_clip_name)
        });
        let Some(role) = role else {
            continue;
        };
        let length = extra
            .and_then(|e| e.gameplay_length)
            .filter(|l| l.is_finite() && *l > 0.0)
            .unwrap_or(clip.duration);
        let looping = extra.and_then(|e| e.looping).unwrap_or(role.loops());
        let steps_f = (f64::from(length) * f64::from(fps)).round().max(1.0);
        ensure!(
            steps_f <= 100_000.0,
            "Clip {} is too long to bake",
            clip.name
        );
        let steps = steps_f as u32;
        let frames = if looping { steps } else { steps + 1 };
        plans.push(Plan {
            clip_index,
            steps,
            clip: BakedClip {
                role,
                name: clip.name.clone(),
                first_frame,
                frames,
                fps: (f64::from(steps) / f64::from(length)) as f32,
                looping,
                length,
                source: clip.source.clone(),
                walk_distance: extra
                    .and_then(|e| e.walk_distance)
                    .filter(|w| w.is_finite() && *w > 0.0)
                    .unwrap_or(1.0),
            },
        });
        first_frame = first_frame
            .checked_add(frames)
            .context("Baked frame count overflow")?;
    }
    if plans.is_empty() {
        bail!("Rig {} has no gameplay role clips", rig_path.display());
    }
    let total_frames = first_frame;
    let mut meshes = Vec::with_capacity(asset.meshes.len());
    for mesh in &asset.meshes {
        let vertex_count = u32::try_from(mesh.positions.len())?;
        let entries = (total_frames as usize)
            .checked_mul(vertex_count as usize)
            .context("Baked vertex count overflow")?;
        meshes.push(BakedMesh {
            name: mesh.name.clone(),
            source_mesh_index: mesh.source_mesh_index,
            vertex_count,
            frames: Vec::with_capacity(entries),
        });
    }
    for plan in &plans {
        let duration = asset.clips[plan.clip_index].duration;
        for frame in 0..plan.clip.frames {
            // Native KF time = gameplay time * duration / length; frame/steps
            // is the fraction of the gameplay length (1.0 on a clamp's end).
            let kf_time = (f64::from(frame) / f64::from(plan.steps) * f64::from(duration)) as f32;
            let pose = asset
                .sample(plan.clip_index, kf_time, PlaybackMode::Clamp)
                .with_context(|| format!("Sampling {} at {kf_time}", plan.clip.name))?;
            ensure!(
                pose.meshes.len() == meshes.len(),
                "Sampled mesh count changed"
            );
            for (out, posed) in meshes.iter_mut().zip(&pose.meshes) {
                ensure!(
                    posed.positions.len() == out.vertex_count as usize
                        && posed.normals.len() == out.vertex_count as usize,
                    "Sampled vertex count changed"
                );
                out.frames.extend(
                    posed
                        .positions
                        .iter()
                        .zip(&posed.normals)
                        .map(|(p, n)| [p[0], p[1], p[2], n[0], n[1], n[2]]),
                );
            }
        }
    }
    Ok(BakedRig {
        model: asset.source.clone(),
        meshes,
        clips: plans.into_iter().map(|p| p.clip).collect(),
        total_frames,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "aa-anim-bake-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn translation_clip(name: &str, extra: &str) -> String {
        format!(
            r#"{{"name":"{name}","source":"Shared/{name}.kf","source_sha256":"0","duration":1.0,"cycle_type":null,
            "tracks":[{{"node":1,"target":"Bone","controller":"NiKeyframeController","frequency":1.0,"phase":0.0,
            "start":0.0,"stop":1.0,"flags":40,
            "translation":{{"interpolation":1,"keys":[{{"time":0.0,"value":[0.0,0.0,0.0]}},{{"time":1.0,"value":[1.0,0.0,2.0]}}]}},
            "scale":{{"interpolation":1,"keys":[]}},
            "rotation":{{"kind":"quaternion","channel":{{"interpolation":1,"keys":[]}}}}}}],
            "events":[]{extra}}}"#
        )
    }

    /// One rigid triangle on a node translated from (0,0,0) to (1,0,2) over 1 KF second.
    fn synthetic_rig(dir: &Path) -> PathBuf {
        let clips = [
            translation_clip(
                "idle",
                r#","role":"idle","gameplay_length":2.0,"looping":true"#,
            ),
            translation_clip(
                "die1",
                r#","role":"die","gameplay_length":1.0,"looping":false"#,
            ),
            translation_clip(
                "die2",
                r#","role":"die","gameplay_length":0.5,"looping":false"#,
            ),
            translation_clip("Fidget1", ""),
        ]
        .join(",");
        let json = format!(
            r#"{{"format":"aa-original-rig-v1","source":"Units/T/Test.nif","source_sha256":"0",
            "coordinates":"test","nif_version":"10.1.0.0",
            "nodes":[{{"name":"Root","parent":null,"translation":[0,0,0],"rotation":[[1,0,0],[0,1,0],[0,0,1]],"scale":1.0}},
                     {{"name":"Bone","parent":0,"translation":[0,0,0],"rotation":[[1,0,0],[0,1,0],[0,0,1]],"scale":1.0}}],
            "meshes":[{{"name":"Tri","source_mesh_index":3,"node":1,"positions":[[0,0,0],[1,0,0],[0,1,0]],
                        "normals":[[0,0,1],[0,0,1],[0,0,1]],"uvs":[],"indices":[0,1,2],"material":{{}},"skin":null}}],
            "clips":[{clips}],"limitations":[]}}"#
        );
        let path = dir.join("Test.rig.json");
        std::fs::write(&path, json).unwrap();
        path
    }

    #[test]
    fn role_names() {
        assert_eq!(Role::from_clip_name("InfantryIdle"), Some(Role::Idle));
        assert_eq!(Role::from_clip_name("InfantryJog"), Some(Role::Move));
        assert_eq!(Role::from_clip_name("InfantryAttack1"), Some(Role::Attack));
        assert_eq!(Role::from_clip_name("InfantryDie1"), Some(Role::Die));
        assert_eq!(Role::from_clip_name("die3"), Some(Role::Die));
        assert_eq!(Role::from_clip_name("move"), Some(Role::Move));
        assert_eq!(Role::from_clip_name("Recoil"), None);
        assert_eq!(Role::from_clip_name("Diesel"), None);
        assert_eq!(
            model_key("/Units\\USA/USAInfantry.NIF"),
            "units/usa/usainfantry.nif"
        );
    }

    #[test]
    fn bake_synthetic_rig_and_select_frames() {
        let dir = scratch("bake");
        let rig = bake(&synthetic_rig(&dir), 10.0).unwrap();
        // idle: 2 s loop -> 20 frames; die1: 1 s clamp -> 11; die2: 0.5 s -> 6.
        let shape: Vec<_> = rig
            .clips
            .iter()
            .map(|c| (c.role, c.first_frame, c.frames, c.looping))
            .collect();
        assert_eq!(
            shape,
            [
                (Role::Idle, 0, 20, true),
                (Role::Die, 20, 11, false),
                (Role::Die, 31, 6, false)
            ]
        );
        assert_eq!(rig.total_frames, 37);
        assert_eq!(rig.meshes[0].source_mesh_index, Some(3));
        assert_eq!(rig.meshes[0].frames.len(), 37 * 3);
        assert_eq!(rig.bytes(), 37 * 3 * 24);
        // Idle at 1 gameplay second is KF 0.5: node at native (0.5,0,1) ->
        // render (x, z, -y) = (0.5, 1, 0) for vertex 0; normal (0,0,1) -> (0,1,0).
        let f = rig.frame(Role::Idle, 1.0, 0);
        assert_eq!(f, 10);
        let v = rig.meshes[0].frame(f)[0];
        assert!((v[0] - 0.5).abs() < 1e-6 && (v[1] - 1.0).abs() < 1e-6 && v[2].abs() < 1e-6);
        assert!((v[4] - 1.0).abs() < 1e-6);
        // Looping wraps, negative times wrap backwards.
        assert_eq!(rig.frame(Role::Idle, 2.0, 0), 0);
        assert_eq!(rig.frame(Role::Idle, 2.1, 7), 1);
        assert_eq!(rig.frame(Role::Idle, -0.1, 0), 19);
        // Clamped die variants: final pose is exact and held.
        assert_eq!(rig.frame(Role::Die, 0.0, 0), 20);
        assert_eq!(rig.frame(Role::Die, 50.0, 0), 30);
        assert_eq!(rig.frame(Role::Die, 50.0, 1), 36);
        assert_eq!(rig.frame(Role::Die, 50.0, 2), 30);
        assert_eq!(rig.frame(Role::Die, -1.0, 1), 31);
        let end = rig.meshes[0].frame(30)[1];
        assert!((end[0] - 2.0).abs() < 1e-6 && (end[1] - 2.0).abs() < 1e-6);
        // Missing roles fall back to idle; unknown clips are not baked.
        assert!(!rig.has_role(Role::Move) && !rig.has_role(Role::Attack));
        assert!(rig.has_role(Role::Die) && rig.variants(Role::Die) == 2);
        assert_eq!(rig.frame(Role::Move, 1.0, 0), 10);
        assert!(rig.clips.iter().all(|c| c.name != "Fidget1"));
        assert_eq!(rig.frame(Role::Idle, f32::NAN, 0), 0);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn bake_rejects_bad_rate() {
        let dir = scratch("rate");
        let path = synthetic_rig(&dir);
        assert!(bake(&path, 0.0).is_err() && bake(&path, f32::NAN).is_err());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn available_prefers_game_manifest_then_legacy() {
        let dir = scratch("available");
        let anim = dir.join("animation");
        std::fs::create_dir_all(anim.join("game")).unwrap();
        for file in ["game/B.rig.json", "game/A.rig.json", "USAInfantry.rig.json"] {
            std::fs::write(anim.join(file), "{}").unwrap();
        }
        std::fs::write(
            anim.join("manifest.json"),
            r#"{"assets":[{"source":"Units/USA/USAInfantry/USAInfantry.nif","file":"USAInfantry.rig.json"}]}"#,
        )
        .unwrap();
        let legacy = available(&dir);
        assert_eq!(legacy.len(), 1);
        assert_eq!(legacy[0].1, anim.join("USAInfantry.rig.json"));
        std::fs::write(
            anim.join("game-rigs.json"),
            r#"{"format":"aa-game-rig-manifest-v1","rigs":[
              {"model":"Units/X/B.nif","file":"game/B.rig.json"},
              {"model":"Units/X/a.nif","file":"game/A.rig.json"},
              {"model":"Units/X/Missing.nif","file":"game/Missing.rig.json"},
              {"model":"Units/X/Evil.nif","file":"../../outside.rig.json"}]}"#,
        )
        .unwrap();
        let game = available(&dir);
        let models: Vec<_> = game.iter().map(|(m, _)| m.as_str()).collect();
        assert_eq!(models, ["Units/X/a.nif", "Units/X/B.nif"]);
        assert_eq!(
            find(&dir, "units/x/A.NIF").unwrap(),
            anim.join("game/A.rig.json")
        );
        assert_eq!(available(&anim), game, "animation dir itself is accepted");
        std::fs::remove_dir_all(dir).ok();
    }

    fn local_cache() -> PathBuf {
        std::env::var_os("AA_ASSETS_CACHE")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets-cache"))
    }

    /// Bakes every gameplay rig at 15 fps; checks vertex layout against the
    /// converted models and reports time/memory per rig.
    #[test]
    #[ignore = "needs local assets-cache and reconstruction data"]
    fn bake_all_local_game_rigs() {
        let cache = local_cache();
        let store = aa_assets::AssetStore::open(cache.join("../../reconstruction"))
            .expect("open reconstruction asset store");
        let rigs = available(&cache);
        assert!(!rigs.is_empty(), "no rigs under {}", cache.display());
        let (mut total_bytes, mut total_ms) = (0usize, 0f64);
        for (model, path) in &rigs {
            let started = Instant::now();
            let rig = bake(path, 15.0).unwrap_or_else(|e| panic!("{model}: {e:#}"));
            let ms = started.elapsed().as_secs_f64() * 1e3;
            let converted = store.load_model(model).expect("converted model");
            for (i, mesh) in rig.meshes.iter().enumerate() {
                let source = mesh.source_mesh_index.unwrap_or(i);
                assert_eq!(
                    converted.meshes[source].positions.len(),
                    3 * mesh.vertex_count as usize,
                    "{model} mesh {}",
                    mesh.name
                );
                assert_eq!(converted.meshes[source].name, mesh.name, "{model}");
                assert_eq!(
                    mesh.frames.len(),
                    (rig.total_frames * mesh.vertex_count) as usize
                );
            }
            assert!(rig.has_role(Role::Idle), "{model}");
            for role in Role::ALL {
                let f = rig.frame(role, 0.37, 1);
                assert!(f < rig.total_frames);
            }
            total_bytes += rig.bytes();
            total_ms += ms;
            println!(
                "{model}: {} verts, {} frames, clips [{}], {:.1} ms, {:.0} KiB",
                rig.vertex_count(),
                rig.total_frames,
                rig.clips
                    .iter()
                    .map(|c| format!("{}:{}", c.name, c.frames))
                    .collect::<Vec<_>>()
                    .join(" "),
                ms,
                rig.bytes() as f64 / 1024.0
            );
        }
        println!(
            "{} rigs: {:.1} ms total ({:.1} ms/rig), {:.1} MiB total ({:.0} KiB/rig)",
            rigs.len(),
            total_ms,
            total_ms / rigs.len() as f64,
            total_bytes as f64 / 1048576.0,
            total_bytes as f64 / 1024.0 / rigs.len() as f64
        );
    }
}
