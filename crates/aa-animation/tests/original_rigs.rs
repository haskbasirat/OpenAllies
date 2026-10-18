use aa_animation::*;
use serde::Deserialize;
use std::path::PathBuf;
fn cache() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets-cache/animation")
}
fn asset(nation: &str) -> AnimatedAsset {
    AnimatedAsset::load(cache().join(format!("{nation}Infantry.rig.json"))).unwrap()
}
#[derive(Deserialize)]
struct Reference {
    poses: Vec<ReferencePose>,
}
#[derive(Deserialize)]
struct ReferencePose {
    clip: Option<usize>,
    time: f32,
    meshes: Vec<ReferenceMesh>,
}
#[derive(Deserialize)]
struct ReferenceMesh {
    name: String,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
}

#[test]
fn original_rigs_match_independent_pyffi_skinning_reference() {
    let mut maximum_position_error = 0.0_f32;
    let mut maximum_normal_error = 0.0_f32;
    for nation in ["USA", "GER", "GBR", "JPN", "RUS"] {
        let asset = asset(nation);
        let reference: Reference = serde_json::from_slice(
            &std::fs::read(cache().join(format!("{nation}Infantry.reference.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(asset.meshes.len(), 2);
        assert_eq!(asset.clips.len(), 4);
        for reference in reference.poses {
            let actual = match reference.clip {
                Some(clip) => asset
                    .sample(clip, reference.time, PlaybackMode::Clamp)
                    .unwrap(),
                None => asset.sample_bind().unwrap(),
            };
            assert_eq!(actual.meshes.len(), reference.meshes.len());
            for ((mesh, source), rigmesh) in actual
                .meshes
                .iter()
                .zip(reference.meshes)
                .zip(&asset.meshes)
            {
                assert_eq!(source.name, rigmesh.name);
                assert_eq!(mesh.positions.len(), source.positions.len());
                assert_eq!(mesh.normals.len(), source.normals.len());
                let position_error = mesh
                    .positions
                    .iter()
                    .flatten()
                    .zip(source.positions.iter().flatten())
                    .map(|(a, b)| (a - b).abs())
                    .fold(0_f32, f32::max);
                let normal_error = mesh
                    .normals
                    .iter()
                    .flatten()
                    .zip(source.normals.iter().flatten())
                    .map(|(a, b)| (a - b).abs())
                    .fold(0_f32, f32::max);
                maximum_position_error = maximum_position_error.max(position_error);
                maximum_normal_error = maximum_normal_error.max(normal_error);
                assert!(
                    position_error < 0.00001,
                    "{nation} {} clip{:?} t{} positionerror{position_error}",
                    source.name,
                    reference.clip,
                    reference.time
                );
                assert!(
                    normal_error < 0.00001,
                    "{nation} {} clip{:?} t{} normalerror{normal_error}",
                    source.name,
                    reference.clip,
                    reference.time
                );
            }
        }
    }
    println!(
        "105 original reference poses: max_position_error={maximum_position_error}, max_normal_error={maximum_normal_error}"
    );
}

#[test]
fn real_clips_move_original_vertices_without_changing_topology() {
    let a = asset("USA");
    for (i, clip) in a.clips.iter().enumerate() {
        let first = a.sample(i, 0.0, PlaybackMode::Clamp).unwrap();
        let mid = a
            .sample(i, clip.duration * 0.5, PlaybackMode::Clamp)
            .unwrap();
        let movement = first
            .meshes
            .iter()
            .zip(mid.meshes)
            .flat_map(|(x, y)| x.positions.iter().copied().zip(y.positions))
            .flat_map(|(x, y)| x.into_iter().zip(y).map(|(v, w)| (v - w).abs()))
            .fold(0_f32, f32::max);
        assert!(
            movement > 0.001,
            "{} did not move authored vertices",
            clip.name
        );
        for (posed, rig) in first.meshes.iter().zip(&a.meshes) {
            assert_eq!(posed.positions.len(), rig.positions.len());
            assert_eq!(posed.normals.len(), rig.positions.len());
        }
    }
}

#[test]
fn explicit_preview_loop_and_clamp_are_deterministic() {
    let a = asset("USA");
    let duration = a.clips[1].duration;
    assert_eq!(
        a.sample(1, duration, PlaybackMode::Loop).unwrap(),
        a.sample(1, 0.0, PlaybackMode::Loop).unwrap()
    );
    assert_eq!(
        a.sample(1, -1.0, PlaybackMode::Clamp).unwrap(),
        a.sample(1, 0.0, PlaybackMode::Clamp).unwrap()
    );
    assert_eq!(
        a.sample(1, duration + 10.0, PlaybackMode::Clamp).unwrap(),
        a.sample(1, duration, PlaybackMode::Clamp).unwrap()
    );
    assert!(a.sample(999, 0.0, PlaybackMode::Loop).is_err());
    assert!(a.sample(0, f32::NAN, PlaybackMode::Loop).is_err());
}

#[test]
fn malformed_hierarchy_weight_and_unsupported_curve_are_rejected() {
    let mut a = asset("USA");
    a.nodes[0].parent = Some(0);
    assert!(a.validate().is_err());
    let mut a = asset("USA");
    a.meshes
        .iter_mut()
        .find_map(|m| m.skin.as_mut())
        .unwrap()
        .bones[0]
        .weights[0]
        .vertex = usize::MAX;
    assert!(a.validate().is_err());
    let mut a = asset("USA");
    a.clips[0].tracks[0].translation.interpolation = 2;
    assert!(a.validate().is_err());
    let mut a = asset("USA");
    a.clips[0].duration = f32::NAN;
    assert!(a.sample(0, 0.0, PlaybackMode::Clamp).is_err());
    let mut a = asset("USA");
    let rotation = a.clips[0]
        .tracks
        .iter_mut()
        .find_map(|t| match &mut t.rotation {
            RotationChannel::ConstantBind { euler_xyz, .. } => Some(euler_xyz),
            _ => None,
        })
        .unwrap();
    rotation[0] += 0.1;
    assert!(a.validate().is_err());
}
