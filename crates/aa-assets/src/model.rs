use crate::terrain::normalize;
use crate::{AssetError, AssetWarning, Bounds, Material, ModelAsset, RawMesh, Result};
use rayon::prelude::*;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[repr(C)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PreparedMesh {
    pub name: String,
    pub hierarchy: Vec<String>,
    pub vertices: Vec<ModelVertex>,
    pub indices: Vec<u32>,
    pub material: Material,
    pub skinned_pose: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PreparedModel {
    pub source: String,
    pub pose: String,
    pub meshes: Vec<PreparedMesh>,
    pub bounds: Bounds,
    pub warnings: Vec<AssetWarning>,
}

impl RawMesh {
    pub fn prepare(&self) -> Result<(PreparedMesh, Vec<AssetWarning>)> {
        let count = self.positions.len() / 3;
        if count == 0
            || !self.positions.len().is_multiple_of(3)
            || !self.indices.len().is_multiple_of(3)
            || self.indices.is_empty()
            || (!self.normals.is_empty() && self.normals.len() != count * 3)
            || (!self.uvs.is_empty() && self.uvs.len() != count * 2)
            || self.indices.iter().any(|&i| i as usize >= count)
            || self
                .positions
                .iter()
                .chain(&self.normals)
                .chain(&self.uvs)
                .any(|v| !v.is_finite())
            || self
                .material
                .color
                .iter()
                .chain(&self.material.emissive)
                .any(|v| !v.is_finite())
            || !self.material.opacity.is_finite()
            || !self.material.alpha_test.is_finite()
        {
            return Err(AssetError::Invalid(format!(
                "{}: invalid mesh arrays/material",
                self.name
            )));
        }
        let positions: Vec<[f32; 3]> = self
            .positions
            .chunks_exact(3)
            .map(|p| [p[0], p[1], p[2]])
            .collect();
        let mut warnings = Vec::new();
        let normals: Vec<[f32; 3]> = if self.normals.is_empty() {
            warnings.push(AssetWarning {
                source: self.name.clone(),
                message:
                    "Normals absent: generated deterministically from authored triangle order."
                        .into(),
            });
            let mut normals = vec![[0.0; 3]; count];
            // Serial accumulation deliberately avoids scheduling-dependent sums.
            for tri in self.indices.chunks_exact(3) {
                let a = positions[tri[0] as usize];
                let b = positions[tri[1] as usize];
                let c = positions[tri[2] as usize];
                let u = std::array::from_fn::<_, 3, _>(|i| b[i] - a[i]);
                let v = std::array::from_fn::<_, 3, _>(|i| c[i] - a[i]);
                let n = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                for &index in tri {
                    for (target, value) in normals[index as usize].iter_mut().zip(n) {
                        *target += value;
                    }
                }
            }
            normals.into_iter().map(normalize).collect()
        } else {
            self.normals
                .chunks_exact(3)
                .map(|n| normalize([n[0], n[1], n[2]]))
                .collect()
        };
        if self.uvs.is_empty() {
            warnings.push(AssetWarning {
                source: self.name.clone(),
                message: "UV coordinates absent; using zero UVs.".into(),
            });
        }
        let color = [
            self.material.color[0],
            self.material.color[1],
            self.material.color[2],
            self.material.opacity,
        ];
        let vertices = positions
            .into_iter()
            .enumerate()
            .map(|(i, position)| ModelVertex {
                position,
                normal: normals[i],
                uv: if self.uvs.is_empty() {
                    [0.0; 2]
                } else {
                    [self.uvs[i * 2], self.uvs[i * 2 + 1]]
                },
                color,
            })
            .collect();
        Ok((
            PreparedMesh {
                name: self.name.clone(),
                hierarchy: self.hierarchy.clone(),
                vertices,
                indices: self.indices.clone(),
                material: self.material.clone(),
                skinned_pose: self.skinned_pose,
            },
            warnings,
        ))
    }
}

impl ModelAsset {
    pub fn prepare(&self) -> Result<PreparedModel> {
        if self.format != "aa-nif-mesh-v1" || self.up_axis != "Y" || self.meshes.is_empty() {
            return Err(AssetError::Invalid(format!(
                "{}: unsupported model format/axis or empty mesh list",
                self.source
            )));
        }
        let prepared: Vec<_> = self.meshes.par_iter().map(RawMesh::prepare).collect();
        let mut meshes = Vec::with_capacity(prepared.len());
        let mut warnings = Vec::new();
        for result in prepared {
            let (mesh, w) = result?;
            meshes.push(mesh);
            warnings.extend(w);
        }
        let bounds = Bounds::from_positions(
            meshes
                .iter()
                .flat_map(|m| m.vertices.iter().map(|v| &v.position)),
        )
        .expect("nonempty validated mesh");
        warnings.push(AssetWarning{source:self.source.clone(),message:"Converted static authored pose; animation, native LOD switching, collision and excluded helper/shadow meshes are not reconstructed.".into()});
        Ok(PreparedModel {
            source: self.source.clone(),
            pose: self.pose.clone(),
            meshes,
            bounds,
            warnings,
        })
    }
}
