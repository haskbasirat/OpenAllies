//Exact-source CPU adapter probe; GPU interface types only, no GPU execution.
use anyhow::{Result,Context,bail};
use aa_animation::AnimatedAsset;
mod terrain_render { #[derive(Clone)] pub struct TerrainMaskMaterial; }
mod gpu { #[derive(Clone,Copy,Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    /// Cell-local native alpha UV. Zero for models; independent of diffuse UV.
    pub terrain_uv: [f32; 2],
}
#[derive(Clone)]
pub struct CpuMaterial {
    pub texture: Option<String>,
    pub color: [f32; 4],
    pub emissive: [f32; 3],
    pub cutoff: f32,
    pub blend: bool,
    pub double_sided: bool,
    pub team_color_mask: bool,
    pub native_render_state: Option<aa_assets::NativeRenderState>,
    pub terrain_layer: bool,
    pub terrain_mask: Option<crate::terrain_render::TerrainMaskMaterial>,
}
}
use gpu::Vertex;
mod loading { use crate::gpu::CpuMaterial;
pub fn material(m: &aa_assets::Material) -> CpuMaterial {
    let blend = m
        .native_render_state
        .map_or(m.alpha_blend || m.opacity < 1.0, |state| {
            state.alpha_flags & 1 != 0
        });
    CpuMaterial {
        texture: m.texture.clone(),
        color: [1.0; 4],
        emissive: m.emissive,
        cutoff: if m.team_color_mask { 0.0 } else { m.alpha_test },
        blend: !m.team_color_mask && blend,
        double_sided: m.double_sided,
        team_color_mask: m.team_color_mask,
        native_render_state: m.native_render_state,
        terrain_layer: false,
        terrain_mask: None,
    }
}
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

