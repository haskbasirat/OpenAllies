use aa_animation::*;
use serde::Deserialize;
use std::path::PathBuf;
#[derive(Deserialize)]
struct Manifest {
    exports: Vec<Family>,
}
#[derive(Deserialize)]
struct Family {
    name: String,
    file: String,
    reference: String,
}
#[derive(Deserialize)]
struct Reference {
    poses: Vec<Pose>,
}
#[derive(Deserialize)]
struct Pose {
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
fn indexed_rig_identity_allows_repeated_labels_but_not_duplicate_mesh_indices() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets-cache/animation");
    let mut asset = AnimatedAsset::load(root.join("USAInfantry.rig.json")).unwrap();
    let original = asset.sample(1, 0.2, PlaybackMode::Clamp).unwrap();
    for node in &mut asset.nodes {
        node.name = "repeated authored label".into();
    }
    for clip in &mut asset.clips {
        for track in &mut clip.tracks {
            track.target = asset.nodes[track.node].name.clone();
        }
    }
    asset.meshes[0].source_mesh_index = Some(0);
    asset.meshes[1].source_mesh_index = Some(1);
    asset.validate().unwrap();
    assert_eq!(asset.sample(1, 0.2, PlaybackMode::Clamp).unwrap(), original);
    asset.meshes[1].source_mesh_index = Some(0);
    assert!(asset.validate().is_err());
}
#[test]
#[ignore = "Phase3 staged families must be explicitly validated before promotion"]
fn staged_families_match_original_reference() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets-cache/animation");
    let manifest: Manifest =
        serde_json::from_slice(&std::fs::read(root.join("families-manifest.json")).unwrap())
            .unwrap();
    let mut failed = Vec::new();
    let mut poses = 0;
    let mut max_position = 0.0_f32;
    let mut max_normal = 0.0_f32;
    for family in manifest.exports {
        let result = (|| -> std::result::Result<(), String> {
            let asset = AnimatedAsset::load(root.join(&family.file)).map_err(|e| e.to_string())?;
            let references: Reference = serde_json::from_slice(
                &std::fs::read(root.join(&family.reference)).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            if asset.meshes.iter().any(|m| m.source_mesh_index.is_none()) {
                return Err("missing original mesh identity".into());
            }
            for r in references.poses {
                let pose = match r.clip {
                    Some(i) => asset.sample(i, r.time, PlaybackMode::Clamp),
                    None => asset.sample_bind(),
                }
                .map_err(|e| e.to_string())?;
                poses += 1;
                if pose.meshes.len() != r.meshes.len() {
                    return Err("mesh count".into());
                }
                for ((actual, expected), source) in
                    pose.meshes.iter().zip(&r.meshes).zip(&asset.meshes)
                {
                    if source.name != expected.name
                        || actual.positions.len() != expected.positions.len()
                        || actual.normals.len() != expected.normals.len()
                    {
                        return Err("mesh identity/count".into());
                    }
                    let pe = actual
                        .positions
                        .iter()
                        .flatten()
                        .zip(expected.positions.iter().flatten())
                        .map(|(a, b)| (a - b).abs())
                        .fold(0.0_f32, f32::max);
                    let ne = actual
                        .normals
                        .iter()
                        .flatten()
                        .zip(expected.normals.iter().flatten())
                        .map(|(a, b)| (a - b).abs())
                        .fold(0.0_f32, f32::max);
                    max_position = max_position.max(pe);
                    max_normal = max_normal.max(ne);
                    if pe > 0.0001 || ne > 0.0001 {
                        return Err(format!(
                            "clip{:?} t{} mesh{} position{pe} normal{ne}",
                            r.clip, r.time, source.name
                        ));
                    }
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            failed.push(format!("{}: {error}", family.name));
        }
    }
    println!("Validated {poses} poses; max_position={max_position}, max_normal={max_normal}");
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}
