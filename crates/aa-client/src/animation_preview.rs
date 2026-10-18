use crate::{
    camera::Camera,
    gpu::{CpuDraw, CpuScene, Instance, SceneRenderer, Vertex},
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
    pub visible: Vec<bool>,
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

pub fn resolve(store: &AssetStore, name: &str) -> Result<PathBuf> {
    crate::animation_catalog::resolve(store, name)
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
    verified_asset_domain(store, path, settings, false)
}

fn verified_asset_domain(
    store: &AssetStore,
    path: &Path,
    settings: &LoadSettings,
    inspector: bool,
) -> Result<AnimatedAsset> {
    let bytes = std::fs::metadata(path)?.len();
    preflight(
        MemoryEstimate {
            temporary_cpu_bytes: count_bytes(bytes, 12)?,
            ..Default::default()
        },
        settings.memory_policy,
    )?;
    let mut asset = if inspector {
        AnimatedAsset::load_preview(path)?
    } else {
        AnimatedAsset::load(path)?
    };
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
        crate::animation_catalog::verify_path(store, path)?;
        let asset = verified_asset_domain(store, path, &settings, true)?;
        let clip = asset
            .clips
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case("Idle"))
            .unwrap_or(0);
        let pose = asset.sample_preview(clip, 0.0, PlaybackMode::Clamp)?;
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
            let color = [
                m.color[0],
                m.color[1],
                m.color[2],
                posed.opacity.unwrap_or(m.opacity),
            ];
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
                material: preview_material(
                    m,
                    asset.clips.iter().any(|clip| {
                        clip.preview
                            .alpha_tracks
                            .iter()
                            .any(|track| track.meshes.contains(&mesh_index))
                    }),
                ),
                source_order: Some(vec![[0, mesh_index]]),
                // Dynamic skinned sphere propagation remains unrecovered;
                // the renderer follows the actual posed vertex bounds here.
                sort_center: None,
            });
        }
        let textures = loading::load_textures(store, &draws, &settings)?;
        let vertices = draws.iter().map(|d| d.vertices.clone()).collect();
        let visible = pose.meshes.iter().map(|mesh| mesh.visible).collect();
        let camera = Camera::for_model(&pose.bounds);
        let warnings = preview_warnings(&asset);
        let scene = CpuScene {
            live: None,
            objects: None,
            name: asset.source.clone(),
            map_id: asset.source.clone(),
            size: [pose.bounds.size[0], pose.bounds.size[2]],
            draws,
            textures,
            placements: 1,
            rendered_instances: 1,
            load_seconds: start.elapsed().as_secs_f64(),
            warnings,
        };
        Ok((
            Self {
                asset,
                clip,
                time: 0.0,
                playing: true,
                looping: true,
                vertices,
                visible,
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
        let pose = self.asset.sample_preview(
            self.clip,
            self.time,
            if self.looping {
                PlaybackMode::Loop
            } else {
                PlaybackMode::Clamp
            },
        )?;
        apply_sampled_pose(
            &self.asset.meshes,
            &pose.meshes,
            &mut self.vertices,
            &mut self.visible,
        )
    }
    ///A single inspector call keeps rendered geometry, alpha and collapsed-mesh
    ///visibility on the same sampled frame. It never advances simulation state.
    pub fn upload_pose(&mut self, renderer: &mut SceneRenderer) -> Result<()> {
        self.pose()?;
        renderer.update_preview_pose(&self.vertices, &self.visible)
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

fn apply_sampled_pose(
    materials: &[aa_animation::RigMesh],
    meshes: &[aa_animation::PosedMesh],
    vertices: &mut [Vec<Vertex>],
    visible: &mut [bool],
) -> Result<()> {
    if meshes.len() != vertices.len()
        || meshes.len() != materials.len()
        || meshes.len() != visible.len()
    {
        bail!("Animated pose mesh count changed");
    }
    for ((mesh, vertices), material) in meshes.iter().zip(&*vertices).zip(materials) {
        if mesh.positions.len() != vertices.len() || mesh.normals.len() != vertices.len() {
            bail!("Animated vertex count changed");
        }
        if !mesh
            .opacity
            .unwrap_or(material.material.opacity)
            .is_finite()
            || mesh
                .positions
                .iter()
                .chain(&mesh.normals)
                .flatten()
                .any(|v| !v.is_finite())
        {
            bail!("Animated pose has non-finite geometry or alpha");
        }
    }
    for (((mesh, vertices), material), visible) in
        meshes.iter().zip(vertices).zip(materials).zip(visible)
    {
        //NiAlphaController sets material opacity. Each sample starts from the
        //base material when there is no override, never last frame's alpha.
        let opacity = mesh.opacity.unwrap_or(material.material.opacity);
        for (vertex, (position, normal)) in vertices
            .iter_mut()
            .zip(mesh.positions.iter().zip(&mesh.normals))
        {
            vertex.position = *position;
            vertex.normal = *normal;
            vertex.color[3] = opacity;
        }
        *visible = mesh.visible;
    }
    Ok(())
}

fn preview_warnings(asset: &AnimatedAsset) -> Vec<String> {
    let mut warnings = vec!["Original animation inspection only. Gameplay animation selection, blending and unit movement are not driven by this viewer. Transparent meshes use posed vertex bounds; native skinned world spheres remain unrecovered.".into()];
    for limitation in &asset.limitations {
        if !warnings.contains(limitation) {
            warnings.push(limitation.clone());
        }
    }
    for clip in &asset.clips {
        for controller in &clip.preview.unsupported_controllers {
            warnings.push(format!(
                "{}: unsupported animation controller {controller}",
                clip.name
            ));
        }
    }
    if asset.clips.iter().any(|clip| !clip.events.is_empty()) {
        warnings.push("Clip markers are listed only; their effects, sounds and gameplay events are not executed.".into());
    }
    warnings
}

fn preview_material(
    material: &aa_assets::Material,
    alpha_animated: bool,
) -> crate::gpu::CpuMaterial {
    let mut result = loading::material(material);
    if alpha_animated {
        //RepColor texture alpha selects the replacement color, not opacity.
        //The shader already converts that texture contribution to alpha1.
        //A sourced opacity controller still uses the authored blend state.
        result.blend = material
            .native_render_state
            .map_or(material.alpha_blend || material.opacity < 1.0, |state| {
                state.alpha_flags & 1 != 0
            });
    }
    result
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
            preview: Default::default(),
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

    fn mesh(opacity: f32) -> aa_animation::RigMesh {
        aa_animation::RigMesh {
            name: "preview fixture".into(),
            source_mesh_index: None,
            node: 0,
            positions: vec![[0.0; 3]],
            normals: vec![[0.0, 1.0, 0.0]],
            uvs: vec![[0.0; 2]],
            indices: vec![0, 0, 0],
            skin: None,
            material: aa_assets::Material {
                opacity,
                ..Default::default()
            },
        }
    }
    fn vertex(alpha: f32) -> Vertex {
        Vertex {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            color: [0.2, 0.3, 0.4, alpha],
            terrain_uv: [0.0; 2],
        }
    }
    #[test]
    fn sampled_alpha_replaces_base_once_and_visibility_recovers_between_clips() {
        let materials = [mesh(0.25), mesh(0.8)];
        let mut vertices = [vec![vertex(0.25)], vec![vertex(0.8)]];
        let mut visible = [true, true];
        let mut pose = [
            aa_animation::PosedMesh {
                positions: vec![[1.0, 2.0, 3.0]],
                normals: vec![[0.0, 1.0, 0.0]],
                opacity: Some(0.5),
                visible: true,
            },
            aa_animation::PosedMesh {
                positions: vec![[4.0, 5.0, 6.0]],
                normals: vec![[1.0, 0.0, 0.0]],
                opacity: Some(0.0),
                visible: false,
            },
        ];
        for _ in 0..3 {
            apply_sampled_pose(&materials, &pose, &mut vertices, &mut visible).unwrap();
            assert_eq!(vertices[0][0].color, [0.2, 0.3, 0.4, 0.5]);
            assert_eq!(vertices[1][0].color[3], 0.0);
            assert_eq!(visible, [true, false]);
        }
        pose[0].opacity = None;
        pose[1].opacity = None;
        pose[1].visible = true;
        apply_sampled_pose(&materials, &pose, &mut vertices, &mut visible).unwrap();
        assert_eq!(vertices[0][0].color[3], 0.25);
        assert_eq!(vertices[1][0].color[3], 0.8);
        assert_eq!(visible, [true, true]);
        assert_eq!(vertices[1][0].position, [4.0, 5.0, 6.0]);
    }
    #[test]
    fn invalid_later_mesh_cannot_partially_apply_a_preview_pose() {
        let materials = [mesh(1.0), mesh(1.0)];
        let mut vertices = [vec![vertex(1.0)], vec![vertex(1.0)]];
        let mut visible = [true, true];
        let mut pose = [
            aa_animation::PosedMesh {
                positions: vec![[1.0; 3]],
                normals: vec![[0.0, 1.0, 0.0]],
                opacity: Some(0.25),
                visible: false,
            },
            aa_animation::PosedMesh {
                positions: vec![[2.0; 3]],
                normals: vec![],
                opacity: Some(0.5),
                visible: false,
            },
        ];
        assert!(apply_sampled_pose(&materials, &pose, &mut vertices, &mut visible).is_err());
        pose[1].normals.push([0.0, 1.0, 0.0]);
        pose[1].opacity = Some(f32::NAN);
        assert!(apply_sampled_pose(&materials, &pose, &mut vertices, &mut visible).is_err());
        assert_eq!(visible, [true, true]);
        assert!(
            vertices
                .iter()
                .all(|mesh| mesh[0].position == [0.0; 3] && mesh[0].color[3] == 1.0)
        );
    }
    #[test]
    fn alpha_controller_uses_authored_blend_without_squaring_repcolor_opacity() {
        let mut material = aa_assets::Material {
            opacity: 0.25,
            team_color_mask: true,
            native_render_state: Some(aa_assets::NativeRenderState {
                alpha_flags: 237,
                alpha_threshold: 0,
                zbuffer_flags: 1,
                zbuffer_function: 3,
            }),
            ..Default::default()
        };
        assert!(!preview_material(&material, false).blend);
        let animated = preview_material(&material, true);
        assert!(animated.blend);
        assert!(animated.team_color_mask);
        assert_eq!(animated.color, [1.0; 4]);
        assert_eq!(animated.native_render_state, material.native_render_state);
        material.native_render_state.as_mut().unwrap().alpha_flags &= !1;
        assert!(!preview_material(&material, true).blend);
    }

    #[test]
    fn unsupported_controllers_and_effect_markers_remain_visible_warnings() {
        let mut clip = aa_animation::Clip {
            name: "Death".into(),
            source: "HQDeath.nif".into(),
            source_sha256: String::new(),
            duration: 1.0,
            cycle_type: None,
            tracks: Vec::new(),
            events: vec![aa_animation::TextEvent {
                time: 0.5,
                text: "PlaySFX collapse".into(),
            }],
            preview: Default::default(),
        };
        clip.preview
            .unsupported_controllers
            .push(serde_json::json!({"controller":"NiParticleSystemController","target":"Smoke"}));
        let asset = AnimatedAsset {
            format: "test metadata only".into(),
            source: "HQ.nif".into(),
            source_sha256: String::new(),
            static_model_sha256: None,
            coordinates: "test".into(),
            nif_version: "test".into(),
            nodes: Vec::new(),
            meshes: Vec::new(),
            clips: vec![clip],
            limitations: vec!["Source particle geometry is not rendered.".into()],
        };
        let warnings = preview_warnings(&asset);
        assert!(
            warnings
                .iter()
                .any(|v| v.contains("Death") && v.contains("NiParticleSystemController"))
        );
        assert!(
            warnings
                .iter()
                .any(|v| v.contains("Source particle geometry"))
        );
        assert!(
            warnings
                .iter()
                .any(|v| v.contains("effects, sounds and gameplay events are not executed"))
        );
    }
}
