use crate::{
    camera::Camera,
    gpu::{CpuDraw, CpuScene, Instance, Vertex},
    loading::{self, LoadSettings},
};
use aa_animation::{AnimatedAsset, PlaybackMode};
use aa_assets::AssetStore;
use aa_resources::{MemoryEstimate, count_bytes, preflight};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

/// Visual playback only. Commands, simulation ticks and authoritative positions
/// never derive from this inspector's wall-clock time.
pub struct AnimationPreview {
    pub asset: AnimatedAsset,
    pub clip: usize,
    pub time: f32,
    pub playing: bool,
    pub looping: bool,
    pub vertices: Vec<Vec<Vertex>>,
}

pub fn cache_dir(store: &AssetStore) -> Option<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        roots.extend(parent.ancestors().map(Path::to_path_buf));
    }
    if let Some(install) = store.root().parent() {
        roots.push(install.join("rust-rebuild"));
    }
    roots
        .into_iter()
        .map(|r| r.join("assets-cache/animation"))
        .find(|p| p.join("manifest.json").is_file())
}

pub fn available(store: &AssetStore) -> Vec<(String, PathBuf)> {
    let Some(dir) = cache_dir(store) else {
        return Vec::new();
    };
    let mut paths = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            p.file_name()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_suffix(".rig.json"))
                .filter(|_| p.is_file())
                .map(|s| (s.to_owned(), p.clone()))
        })
        .collect::<Vec<_>>();
    paths.sort_by(|a, b| a.0.cmp(&b.0));
    paths
}

pub fn resolve(store: &AssetStore, name: &str) -> Result<PathBuf> {
    available(store)
        .into_iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, p)| p)
        .context("Unknown recovered animation rig; use a name listed under Animations")
}

fn verify_mesh_mapping(
    indices: &[Option<usize>],
    retained: &[usize],
    removed: usize,
) -> Result<()> {
    if indices.iter().all(Option::is_none) {
        if removed != 0 || indices.len() != retained.len() {
            bail!("Animation export needs original mesh indices to bind shadow-helper filtering");
        }
    } else if indices
        .iter()
        .copied()
        .collect::<Option<Vec<_>>>()
        .as_deref()
        != Some(retained)
    {
        bail!("Animation original-mesh mapping differs from audited retained geometry");
    }
    Ok(())
}

pub fn clip_label(clips: &[aa_animation::Clip], index: usize) -> String {
    let clip = &clips[index];
    if clips
        .iter()
        .filter(|other| other.name.eq_ignore_ascii_case(&clip.name))
        .count()
        > 1
    {
        format!(
            "{} · {} [{}]",
            clip.name,
            clip.source.rsplit('/').next().unwrap_or(&clip.source),
            index
        )
    } else {
        clip.name.clone()
    }
}

fn resolve_clip_name(clips: &[aa_animation::Clip], name: &str) -> Result<usize> {
    let matches: Vec<_> = clips
        .iter()
        .enumerate()
        .filter(|(_, c)| c.name.eq_ignore_ascii_case(name))
        .map(|(i, _)| i)
        .collect();
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => bail!(
            "Unknown clip name {name}; available: {}",
            clips
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => bail!(
            "Ambiguous clip name {name}; select --clip-index instead: {}",
            matches
                .iter()
                .map(|&i| clip_label(clips, i))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

pub(crate) fn verified_asset(
    store: &AssetStore,
    path: &Path,
    settings: &LoadSettings,
) -> Result<AnimatedAsset> {
    let bytes = std::fs::metadata(path)?.len();
    preflight(
        MemoryEstimate {
            temporary_cpu_bytes: count_bytes(bytes, 12)?,
            ..Default::default()
        },
        settings.memory_policy,
    )?;
    let mut asset = AnimatedAsset::load(path)?;
    let nif_path = store.resolve_url(&format!("/assets/original/Data/{}", asset.source))?;
    if format!("{:x}", Sha256::digest(std::fs::read(nif_path)?)) != asset.source_sha256 {
        bail!("Animation original NIF identity mismatch");
    }
    if let Some(expected) = &asset.static_model_sha256 {
        let model_path = store.resolve_url(store.model_url(&asset.source)?)?;
        if format!("{:x}", Sha256::digest(std::fs::read(model_path)?)) != *expected {
            bail!("Animation converted static model identity mismatch");
        }
    } else if asset
        .meshes
        .iter()
        .any(|mesh| mesh.source_mesh_index.is_some())
    {
        bail!("Indexed animation exports require a converted static model hash");
    }
    let mut original = store.load_model(&asset.source)?.prepare()?;
    let (stats, retained_indices) = settings
        .model_fixes
        .apply_with_mesh_mapping(store, &mut original)?;
    verify_mesh_mapping(
        &asset
            .meshes
            .iter()
            .map(|mesh| mesh.source_mesh_index)
            .collect::<Vec<_>>(),
        &retained_indices,
        stats.shadow_helpers_removed,
    )?;
    if original.meshes.len() != asset.meshes.len() {
        bail!("Original material correction mesh count differs from animation rig");
    }
    for (rig, mesh) in asset.meshes.iter_mut().zip(&original.meshes) {
        if rig.name != mesh.name
            || rig.indices != mesh.indices
            || rig.positions.len() != mesh.vertices.len()
        {
            bail!("Animation material geometry identity mismatch");
        }
        rig.material = mesh.material.clone();
    }
    if asset.clips.is_empty() {
        bail!("Animation rig has no supported clips");
    }
    Ok(asset)
}

impl AnimationPreview {
    pub fn load(
        store: &AssetStore,
        path: &Path,
        settings: LoadSettings,
    ) -> Result<(Self, CpuScene, Camera)> {
        let start = Instant::now();
        let asset = verified_asset(store, path, &settings)?;
        let clip = asset
            .clips
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case("Idle"))
            .unwrap_or(0);
        let pose = asset.sample(clip, 0.0, PlaybackMode::Clamp)?;
        if pose.meshes.len() != asset.meshes.len() {
            bail!("Animated pose mesh count mismatch");
        }
        let mut draws = Vec::new();
        for (mesh_index, (mesh, posed)) in asset.meshes.iter().zip(&pose.meshes).enumerate() {
            if mesh.indices.is_empty() || posed.positions.is_empty() {
                bail!("Animated rig contains an empty drawable mesh");
            }
            if mesh.uvs.len() != posed.positions.len()
                || posed.normals.len() != posed.positions.len()
            {
                bail!("Animated mesh attributes have different lengths");
            }
            let m = &mesh.material;
            let color = [m.color[0], m.color[1], m.color[2], m.opacity];
            let vertices = posed
                .positions
                .iter()
                .zip(&posed.normals)
                .zip(&mesh.uvs)
                .map(|((position, normal), uv)| Vertex {
                    terrain_uv: [0.0; 2],
                    position: *position,
                    normal: *normal,
                    uv: loading::model_uv(*uv),
                    color,
                })
                .collect();
            draws.push(CpuDraw {
                vertices,
                indices: mesh.indices.clone(),
                instances: vec![Instance::identity()],
                material: loading::material(m),
                source_order: Some(vec![[0, mesh_index]]),
                // Dynamic skinned sphere propagation remains unrecovered;
                // the renderer follows the actual posed vertex bounds here.
                sort_center: None,
            });
        }
        let textures = loading::load_textures(store, &draws, &settings)?;
        let vertices = draws.iter().map(|d| d.vertices.clone()).collect();
        let camera = Camera::for_model(&pose.bounds);
        let scene=CpuScene{live:None,objects:None,name:asset.source.clone(),map_id:asset.source.clone(),size:[pose.bounds.size[0],pose.bounds.size[2]],
            draws,textures,placements:1,rendered_instances:1,load_seconds:start.elapsed().as_secs_f64(),
            warnings:vec!["Original rig/clip inspection only. Runtime animation selection, blending, gameplay events and unit movement are not reconstructed. Transparent animated meshes use posed vertex-box centers and binary32 preview sort keys; native skinned world spheres remain unrecovered.".into()]};
        Ok((
            Self {
                asset,
                clip,
                time: 0.0,
                playing: true,
                looping: true,
                vertices,
            },
            scene,
            camera,
        ))
    }
    pub fn select_clip(&mut self, name: &str) -> Result<()> {
        let index = resolve_clip_name(&self.asset.clips, name)?;
        self.select_clip_index(index)
    }
    pub fn select_clip_index(&mut self, index: usize) -> Result<()> {
        self.asset
            .clips
            .get(index)
            .context("Clip index is outside this rig's clip list")?;
        self.clip = index;
        self.time = 0.0;
        Ok(())
    }
    pub fn pose(&mut self) -> Result<()> {
        let pose = self.asset.sample(
            self.clip,
            self.time,
            if self.looping {
                PlaybackMode::Loop
            } else {
                PlaybackMode::Clamp
            },
        )?;
        if pose.meshes.len() != self.vertices.len() {
            bail!("Animated pose mesh count changed");
        }
        for (mesh, vertices) in pose.meshes.iter().zip(&self.vertices) {
            if mesh.positions.len() != vertices.len() || mesh.normals.len() != vertices.len() {
                bail!("Animated vertex count changed");
            }
        }
        for (mesh, vertices) in pose.meshes.iter().zip(&mut self.vertices) {
            for (vertex, (position, normal)) in vertices
                .iter_mut()
                .zip(mesh.positions.iter().zip(&mesh.normals))
            {
                vertex.position = *position;
                vertex.normal = *normal;
            }
        }
        Ok(())
    }
    pub fn advance(&mut self, seconds: f32) {
        if self.playing {
            let duration = self.asset.clips[self.clip].duration;
            self.time += seconds;
            if self.looping && duration > 0.0 {
                self.time = self.time.rem_euclid(duration);
            } else if self.time >= duration {
                self.time = duration;
                self.playing = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_native_clip_labels_require_explicit_index() {
        let make = |name: &str, source: &str| aa_animation::Clip {
            name: name.into(),
            source: source.into(),
            source_sha256: String::new(),
            duration: 1.0,
            cycle_type: None,
            tracks: vec![],
            events: vec![],
        };
        let clips = [
            make("Recoil1", "BunkerDisabledRecoil1.kf"),
            make("Recoil1", "BunkerRecoil1.kf"),
            make("Idle", "BunkerIdle.kf"),
        ];
        assert!(resolve_clip_name(&clips, "recoil1").is_err());
        assert_eq!(resolve_clip_name(&clips, "IDLE").unwrap(), 2);
        assert!(resolve_clip_name(&clips, "missing").is_err());
        assert_ne!(clip_label(&clips, 0), clip_label(&clips, 1));
        assert!(clip_label(&clips, 0).contains("Disabled"));
    }
    #[test]
    fn animation_mesh_mapping_rejects_hidden_missing_and_reordered_geometry() {
        assert!(verify_mesh_mapping(&[None, None], &[0, 1], 0).is_ok());
        assert!(verify_mesh_mapping(&[Some(0), Some(3)], &[0, 3], 2).is_ok());
        for indices in [
            vec![None, None],
            vec![Some(3), Some(0)],
            vec![Some(0), Some(1)],
            vec![Some(0), None],
            vec![Some(0)],
        ] {
            assert!(verify_mesh_mapping(&indices, &[0, 3], 2).is_err());
        }
    }
}
