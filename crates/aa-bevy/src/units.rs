//! Display-only units: original rigs posed by `aa_animation` every frame.
//! Placement and paths are for viewing the engine, not gameplay.
use aa_animation::{AnimatedAsset, PlaybackMode};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::sync::Arc;

/// Soldiers sharing one clip and phase share their posed meshes.
#[derive(Component)]
pub struct PoseGroup {
    pub asset: Arc<AnimatedAsset>,
    pub clip: usize,
    pub phase: f32,
    pub meshes: Vec<Handle<Mesh>>,
}

/// Moves along a closed loop around `center` at `speed` m/s.
#[derive(Component)]
pub struct LoopPath {
    pub center: Vec2,
    pub radius: f32,
    pub speed: f32,
    pub angle: f32,
    /// Formation offset: (sideways, behind) in metres.
    pub offset: Vec2,
}

pub fn clip_index(asset: &AnimatedAsset, name: &str) -> Option<usize> {
    asset.clips.iter().position(|c| c.name.eq_ignore_ascii_case(name))
}

/// Bind-pose meshes for one pose group.
pub fn group_meshes(asset: &AnimatedAsset, meshes: &mut Assets<Mesh>) -> anyhow::Result<Vec<Handle<Mesh>>> {
    let pose = asset.sample_bind()?;
    Ok(asset
        .meshes
        .iter()
        .zip(&pose.meshes)
        .map(|(rig, posed)| {
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, posed.positions.clone());
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, posed.normals.clone());
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, rig.uvs.clone());
            mesh.insert_indices(Indices::U32(rig.indices.clone()));
            meshes.add(mesh)
        })
        .collect())
}

pub fn animate(time: Res<Time>, groups: Query<&PoseGroup>, mut meshes: ResMut<Assets<Mesh>>) {
    if std::env::var_os("AA_BEVY_NO_ANIM").is_some() { return; }
    let t = time.elapsed_secs();
    for group in &groups {
        let Ok(pose) = group.asset.sample(group.clip, t + group.phase, PlaybackMode::Loop) else { continue };
        for (handle, posed) in group.meshes.iter().zip(pose.meshes) {
            if let Some(mut mesh) = meshes.get_mut(handle) {
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, posed.positions);
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, posed.normals);
            }
        }
    }
}

pub fn walk(time: Res<Time>, ground: Res<crate::camera::Ground>, mut movers: Query<(&mut LoopPath, &mut Transform)>) {
    let dt = time.delta_secs().min(0.1);
    for (mut path, mut transform) in &mut movers {
        path.angle += path.speed / path.radius * dt;
        let dir = Vec2::new(path.angle.cos(), path.angle.sin());
        let tangent = Vec2::new(-dir.y, dir.x);
        let p = path.center + dir * (path.radius + path.offset.x) - tangent * path.offset.y;
        let y = (ground.0)(p.x, p.y);
        transform.translation = Vec3::new(p.x, y, p.y);
        // Original models face render +Z at identity (native -Y).
        transform.rotation = Quat::from_rotation_y(tangent.x.atan2(tangent.y));
    }
}
