//! Read-only geometry selection for the reconstruction inspector.
//! This is not the original game's order/selection rules or collision system.
use crate::{camera::Camera, gpu::Instance};
use aa_assets::PreparedScene;
use aa_resources::{MemoryEstimate, MemoryPolicy, count_bytes, preflight};
use anyhow::{Context, Result, ensure};
use eframe::egui;
use glam::{Mat4, Vec3};
use rayon::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

/// Pointer coordinates and viewport are both in egui's logical points. Only
/// projection aspect comes from the rounded physical render target dimensions.
pub fn pointer_ray(
    camera: &Camera,
    rect: egui::Rect,
    pointer: egui::Pos2,
    pixels: [u32; 2],
) -> Option<Ray> {
    if !rect.contains(pointer) || rect.width() <= 0.0 || rect.height() <= 0.0 || pixels.contains(&0)
    {
        return None;
    }
    let normalized = (pointer - rect.min) / rect.size();
    let inverse = camera
        .view_projection(pixels[0] as f32 / pixels[1] as f32)
        .as_dmat4()
        .inverse();
    let ndc = Vec3::new(normalized.x * 2.0 - 1.0, 1.0 - normalized.y * 2.0, 0.0);
    let origin = inverse.project_point3(ndc.as_dvec3());
    let far = inverse.project_point3(ndc.with_z(1.0).as_dvec3());
    let direction = (far - origin).try_normalize()?.as_vec3();
    let origin = origin.as_vec3();
    (origin.is_finite() && direction.is_finite()).then_some(Ray { origin, direction })
}

#[derive(Clone, Copy, Debug)]
struct Bounds {
    min: Vec3,
    max: Vec3,
}
impl Bounds {
    fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }
    fn include(&mut self, p: Vec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }
    fn near(&self, ray: Ray, limit: f32) -> Option<f32> {
        let mut near = 0.0_f32;
        let mut far = limit;
        for axis in 0..3 {
            if ray.direction[axis] == 0.0 {
                if ray.origin[axis] < self.min[axis] || ray.origin[axis] > self.max[axis] {
                    return None;
                }
            } else {
                let a = (self.min[axis] - ray.origin[axis]) / ray.direction[axis];
                let b = (self.max[axis] - ray.origin[axis]) / ray.direction[axis];
                near = near.max(a.min(b));
                far = far.min(a.max(b));
                if near > far {
                    return None;
                }
            }
        }
        Some(near)
    }
    fn transformed(self, matrix: Mat4) -> Self {
        let mut result = Self::empty();
        for x in [self.min.x, self.max.x] {
            for y in [self.min.y, self.max.y] {
                for z in [self.min.z, self.max.z] {
                    result.include(matrix.transform_point3(Vec3::new(x, y, z)));
                }
            }
        }
        result
    }
}

#[derive(Clone, Copy)]
struct Triangle {
    points: [Vec3; 3],
    double_sided: bool,
}
impl Triangle {
    fn hit(self, ray: Ray, limit: f32) -> Option<f32> {
        // UI geometry calculations only; this does not implement native physics.
        let a = self.points[0].as_dvec3();
        let e1 = self.points[1].as_dvec3() - a;
        let e2 = self.points[2].as_dvec3() - a;
        let direction = ray.direction.as_dvec3();
        let p = direction.cross(e2);
        let determinant = e1.dot(p);
        if determinant == 0.0 || (!self.double_sided && determinant < 0.0) {
            return None;
        }
        let inverse = 1.0 / determinant;
        let t = ray.origin.as_dvec3() - a;
        let u = t.dot(p) * inverse;
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let q = t.cross(e1);
        let v = direction.dot(q) * inverse;
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let distance = e2.dot(q) * inverse;
        (distance >= 0.0 && distance <= f64::from(limit)).then_some(distance as f32)
    }
}

struct Node {
    bounds: Bounds,
    start: usize,
    end: usize,
    children: Option<[usize; 2]>,
}
#[derive(Default)]
struct Geometry {
    triangles: Vec<Triangle>,
    nodes: Vec<Node>,
}
impl Geometry {
    fn build(triangles: Vec<Triangle>) -> Result<Self> {
        ensure!(
            triangles.iter().flat_map(|t| t.points).all(Vec3::is_finite),
            "Nonfinite inspection geometry"
        );
        let mut geometry = Self {
            triangles,
            nodes: Vec::new(),
        };
        geometry
            .nodes
            .try_reserve(geometry.triangles.len() / 2 + 1)?;
        if !geometry.triangles.is_empty() {
            geometry.partition(0, geometry.triangles.len());
        }
        Ok(geometry)
    }
    fn partition(&mut self, start: usize, end: usize) -> usize {
        let mut bounds = Bounds::empty();
        for triangle in &self.triangles[start..end] {
            for p in triangle.points {
                bounds.include(p);
            }
        }
        let index = self.nodes.len();
        self.nodes.push(Node {
            bounds,
            start,
            end,
            children: None,
        });
        if end - start > 8 {
            let extent = bounds.max - bounds.min;
            let axis = if extent.x >= extent.y && extent.x >= extent.z {
                0
            } else if extent.y >= extent.z {
                1
            } else {
                2
            };
            let middle = start + (end - start) / 2;
            self.triangles[start..end].select_nth_unstable_by(middle - start, |a, b| {
                let center =
                    |t: &Triangle| t.points.iter().map(|p| f64::from(p[axis])).sum::<f64>();
                center(a).total_cmp(&center(b))
            });
            self.nodes[index].children =
                Some([self.partition(start, middle), self.partition(middle, end)]);
        }
        index
    }
    fn hit(&self, ray: Ray, limit: f32) -> Option<f32> {
        if self.nodes.is_empty() {
            return None;
        }
        self.hit_node(0, ray, limit)
    }
    fn hit_node(&self, index: usize, ray: Ray, mut limit: f32) -> Option<f32> {
        let node = &self.nodes[index];
        node.bounds.near(ray, limit)?;
        if let Some([left, right]) = node.children {
            let first = self.hit_node(left, ray, limit);
            if let Some(distance) = first {
                limit = distance;
            }
            self.hit_node(right, ray, limit).or(first)
        } else {
            let mut found = None;
            for &triangle in &self.triangles[node.start..node.end] {
                if let Some(distance) = triangle.hit(ray, limit) {
                    limit = distance;
                    found = Some(distance);
                }
            }
            found
        }
    }
}

pub struct Object {
    pub instance_id: u32,
    pub thing_id: String,
    pub name: String,
    pub category: Option<String>,
    pub rule_source: Option<String>,
    pub model_source: String,
    pub native_position: [f32; 3],
    pub record_offset: u64,
    pub record_length: u64,
    pub source_index: usize,
    pub visible: bool,
    model_index: usize,
    inverse: Mat4,
    bounds: Bounds,
}
pub struct SceneObjects {
    pub objects: Vec<Object>,
    models: Vec<Geometry>,
    terrain: Geometry,
}
impl SceneObjects {
    pub(crate) fn empty() -> Result<Self> {
        Ok(Self {
            objects: Vec::new(),
            models: Vec::new(),
            terrain: Geometry::build(Vec::new())?,
        })
    }
}
#[derive(Clone, Copy)]
pub(crate) struct ObjectTransformUpdate {
    pub object: usize,
    pub instance: Instance,
    pub native_position: [f32; 3],
    pub visible: bool,
}
pub(crate) struct PreparedObjectTransforms(Vec<(usize, Mat4, Bounds, [f32; 3], bool)>);
#[derive(Clone, Copy, Debug)]
pub struct Selection {
    pub object: usize,
    pub position: Vec3,
}

/// Staged local picking geometry for an actor with no saved placement record.
pub(crate) struct PreparedObjectAppend {
    object: Object,
    geometry: Geometry,
}
pub(crate) struct SpawnObject {
    pub actor_id: u32,
    pub thing_id: String,
    pub model_source: String,
    pub rule_source: String,
    pub instance: Instance,
    pub native_position: [f32; 3],
    pub visible: bool,
}
pub(crate) struct PoseMesh<'a> {
    pub vertices: &'a [crate::gpu::Vertex],
    pub indices: &'a [u32],
    pub double_sided: bool,
}
pub(crate) struct PreparedObjectPose {
    object: usize,
    model: usize,
    geometry: Geometry,
    bounds: Bounds,
}
impl SceneObjects {
    pub(crate) fn prepare_pose(
        &self,
        object: usize,
        meshes: &[PoseMesh<'_>],
    ) -> Result<PreparedObjectPose> {
        let item = self
            .objects
            .get(object)
            .context("Animated picking actor is absent")?;
        ensure!(
            self.objects
                .iter()
                .filter(|o| o.model_index == item.model_index)
                .count()
                == 1,
            "Animated picking geometry is shared with another actor"
        );
        let mut triangles = Vec::new();
        for mesh in meshes {
            ensure!(
                mesh.indices.len() % 3 == 0,
                "Invalid animated picking topology"
            );
            triangles.try_reserve(mesh.indices.len() / 3)?;
            for indices in mesh.indices.chunks_exact(3) {
                let mut points = [Vec3::ZERO; 3];
                for i in 0..3 {
                    points[i] = Vec3::from(
                        mesh.vertices
                            .get(indices[i] as usize)
                            .context("Animated picking index is absent")?
                            .position,
                    );
                    ensure!(points[i].is_finite(), "Invalid animated picking position");
                }
                triangles.push(Triangle {
                    points,
                    double_sided: mesh.double_sided,
                });
            }
        }
        let geometry = Geometry::build(triangles)?;
        let matrix = item.inverse.inverse();
        let bounds = geometry
            .nodes
            .first()
            .context("Animated picking geometry empty")?
            .bounds
            .transformed(matrix);
        ensure!(
            bounds.min.is_finite() && bounds.max.is_finite(),
            "Invalid animated picking bounds"
        );
        Ok(PreparedObjectPose {
            object,
            model: item.model_index,
            geometry,
            bounds,
        })
    }
    pub(crate) fn apply_pose(&mut self, pose: PreparedObjectPose) {
        self.models[pose.model] = pose.geometry;
        self.objects[pose.object].bounds = pose.bounds;
    }
    pub(crate) fn prepare_append(
        &mut self,
        draws: &[crate::gpu::CpuDraw],
        source: SpawnObject,
    ) -> Result<PreparedObjectAppend> {
        ensure!(
            !self
                .objects
                .iter()
                .any(|o| o.instance_id == source.actor_id),
            "Duplicate native actor ID in picking scene"
        );
        let mut triangles = Vec::new();
        for draw in draws {
            ensure!(
                draw.instances.len() == 1 && !draw.material.terrain_layer,
                "Invalid actor picking draw"
            );
            ensure!(
                draw.indices.len() % 3 == 0,
                "Incomplete actor picking triangle"
            );
            triangles.try_reserve(draw.indices.len() / 3)?;
            for indices in draw.indices.chunks_exact(3) {
                let mut points = [Vec3::ZERO; 3];
                for i in 0..3 {
                    points[i] = Vec3::from(
                        draw.vertices
                            .get(indices[i] as usize)
                            .context("Invalid actor picking index")?
                            .position,
                    );
                    ensure!(points[i].is_finite(), "Nonfinite actor picking vertex");
                }
                triangles.push(Triangle {
                    points,
                    double_sided: draw.material.double_sided,
                });
            }
        }
        let geometry = Geometry::build(triangles)?;
        let matrix = Mat4::from_cols_array_2d(&source.instance.model);
        let inverse = matrix.inverse();
        let bounds = geometry
            .nodes
            .first()
            .context("Spawn has no picking geometry")?
            .bounds
            .transformed(matrix);
        ensure!(
            inverse.is_finite() && bounds.min.is_finite() && bounds.max.is_finite(),
            "Invalid spawn picking transform"
        );
        self.objects.try_reserve(1)?;
        self.models.try_reserve(1)?;
        let object = Object {
            instance_id: source.actor_id,
            name: source.thing_id.clone(),
            thing_id: source.thing_id,
            category: None,
            rule_source: Some(source.rule_source),
            model_source: source.model_source,
            native_position: source.native_position,
            record_offset: 0,
            record_length: 0,
            // No original map record exists for a live-created actor.
            source_index: usize::MAX,
            visible: source.visible,
            model_index: self.models.len(),
            inverse,
            bounds,
        };
        Ok(PreparedObjectAppend { object, geometry })
    }
    pub(crate) fn append(&mut self, prepared: PreparedObjectAppend) -> usize {
        let index = self.objects.len();
        self.models.push(prepared.geometry);
        self.objects.push(prepared.object);
        index
    }
    /// Validate and stage CPU picking changes before the associated GPU update.
    /// Models and their local triangle hierarchies remain resident and unchanged.
    pub(crate) fn prepare_transforms(
        &self,
        updates: &[ObjectTransformUpdate],
    ) -> Result<PreparedObjectTransforms> {
        let mut staged = Vec::new();
        staged.try_reserve(updates.len())?;
        let mut seen = std::collections::BTreeSet::new();
        for update in updates {
            ensure!(
                seen.insert(update.object),
                "Duplicate picking object in snapshot"
            );
            let object = self
                .objects
                .get(update.object)
                .context("Picking object slot is out of range")?;
            let matrix = Mat4::from_cols_array_2d(&update.instance.model);
            let inverse = matrix.inverse();
            let bounds = self.models[object.model_index]
                .nodes
                .first()
                .context("Picking geometry is empty")?
                .bounds
                .transformed(matrix);
            ensure!(
                matrix.is_finite()
                    && inverse.is_finite()
                    && bounds.min.is_finite()
                    && bounds.max.is_finite()
                    && update.native_position.iter().all(|n| n.is_finite()),
                "Invalid live transform for picking"
            );
            staged.push((
                update.object,
                inverse,
                bounds,
                update.native_position,
                update.visible,
            ));
        }
        Ok(PreparedObjectTransforms(staged))
    }
    pub(crate) fn apply_transforms(&mut self, updates: PreparedObjectTransforms) {
        for (index, inverse, bounds, native_position, visible) in updates.0 {
            let object = &mut self.objects[index];
            object.inverse = inverse;
            object.bounds = bounds;
            object.native_position = native_position;
            object.visible = visible;
        }
    }
    pub fn pick_at(
        &self,
        camera: &Camera,
        rect: egui::Rect,
        pointer: egui::Pos2,
        pixels: [u32; 2],
    ) -> Option<Selection> {
        let selection = self.pick(pointer_ray(camera, rect, pointer, pixels)?)?;
        // The CPU index can contain outlier models beyond the render far plane.
        Self::marker(camera, rect, pixels, selection.position).map(|_| selection)
    }
    pub fn build(scene: &PreparedScene, policy: MemoryPolicy) -> Result<Self> {
        let triangle_count =
            scene
                .models
                .values()
                .flat_map(|m| &m.meshes)
                .try_fold(0u64, |sum, mesh| {
                    sum.checked_add(mesh.indices.len() as u64 / 3)
                        .context("Inspection geometry count overflow")
                })?;
        let terrain_count = scene
            .terrain
            .batches
            .iter()
            .map(|b| b.indices.len() as u64 / 3)
            .sum::<u64>();
        preflight(
            MemoryEstimate {
                resident_cpu_bytes: count_bytes(
                    triangle_count
                        .checked_add(terrain_count)
                        .context("Inspection count overflow")?,
                    100,
                )?
                .checked_add(count_bytes(scene.instances.len() as u64, 2048)?)
                .context("Inspection size overflow")?,
                ..Default::default()
            },
            policy,
        )?;
        let keys: Vec<_> = scene.models.keys().collect();
        let model_indices: BTreeMap<_, _> = keys
            .iter()
            .enumerate()
            .map(|(index, key)| (key.as_str(), index))
            .collect();
        let models: Vec<_> = keys
            .par_iter()
            .map(|key| {
                let mut triangles = Vec::new();
                for mesh in &scene.models[*key].meshes {
                    triangles.try_reserve(mesh.indices.len() / 3)?;
                    for indices in mesh.indices.chunks_exact(3) {
                        triangles.push(Triangle {
                            points: std::array::from_fn(|i| {
                                Vec3::from(mesh.vertices[indices[i] as usize].position)
                            }),
                            double_sided: mesh.material.double_sided,
                        });
                    }
                }
                Geometry::build(triangles)
            })
            .collect::<Result<_>>()?;
        let mut objects = Vec::new();
        objects.try_reserve(scene.instances.len())?;
        for instance in &scene.instances {
            let Some(key) = &instance.model_key else {
                continue;
            };
            let Some(&model_index) = model_indices.get(key.as_str()) else {
                continue;
            };
            let Some(root) = models[model_index].nodes.first() else {
                continue;
            };
            let matrix = Mat4::from_cols_array_2d(
                &Instance::at(instance.position, instance.yaw_radians, instance.scale).model,
            );
            let inverse = matrix.inverse();
            ensure!(
                inverse.is_finite(),
                "Invalid instance transform for inspection"
            );
            let placement = &scene.map.placements.actors[instance.source_index];
            let definition = scene.map.placements.types.get(&instance.thing_id);
            objects.push(Object {
                instance_id: instance.instance_id,
                thing_id: instance.thing_id.clone(),
                name: definition
                    .and_then(|d| d.name.clone())
                    .unwrap_or_else(|| instance.thing_id.clone()),
                category: definition.and_then(|d| d.category.clone()),
                rule_source: definition.and_then(|d| d.source.clone()),
                model_source: scene.models[key].source.clone(),
                native_position: placement.native_position(),
                record_offset: placement.record_offset,
                record_length: placement.record_length,
                source_index: instance.source_index,
                visible: true,
                model_index,
                inverse,
                bounds: root.bounds.transformed(matrix),
            });
        }
        let mut triangles = Vec::new();
        triangles.try_reserve(usize::try_from(terrain_count)?)?;
        for batch in &scene.terrain.batches {
            for indices in batch.indices.chunks_exact(3) {
                triangles.push(Triangle {
                    points: std::array::from_fn(|i| {
                        Vec3::from(scene.terrain.vertices[indices[i] as usize].position)
                    }),
                    double_sided: false,
                });
            }
        }
        Ok(Self {
            objects,
            models,
            terrain: Geometry::build(triangles)?,
        })
    }
    pub fn pick(&self, ray: Ray) -> Option<Selection> {
        if !ray.origin.is_finite()
            || !ray.direction.is_finite()
            || ray.direction.length_squared() == 0.0
        {
            return None;
        }
        let mut limit = self
            .terrain
            .hit(ray, f32::INFINITY)
            .unwrap_or(f32::INFINITY);
        let mut selected = None;
        // Stable source order resolves coincident triangles. Inverse direction
        // deliberately is NOT normalized: t remains in world-ray units.
        for (index, object) in self.objects.iter().enumerate() {
            if !object.visible {
                continue;
            }
            if object.bounds.near(ray, limit).is_none() {
                continue;
            }
            let local = Ray {
                origin: object.inverse.transform_point3(ray.origin),
                direction: object.inverse.transform_vector3(ray.direction),
            };
            if let Some(distance) = self.models[object.model_index].hit(local, limit)
                && (selected.is_none() || distance < limit)
            {
                limit = distance;
                selected = Some(Selection {
                    object: index,
                    position: ray.origin + ray.direction * distance,
                });
            }
        }
        selected
    }
    pub fn focus(&self, index: usize) -> Vec3 {
        let bounds = self.objects[index].bounds;
        (bounds.min + bounds.max) * 0.5
    }
    pub fn marker(
        camera: &Camera,
        rect: egui::Rect,
        pixels: [u32; 2],
        position: Vec3,
    ) -> Option<egui::Pos2> {
        if pixels.contains(&0) {
            return None;
        }
        let p = camera.view_projection(pixels[0] as f32 / pixels[1] as f32) * position.extend(1.0);
        if p.w <= 0.0 || p.z < 0.0 || p.z > p.w {
            return None;
        }
        let ndc = p.truncate() / p.w;
        let point = rect.min + rect.size() * egui::vec2(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
        rect.contains(point).then_some(point)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plane(z: f32, double_sided: bool) -> Triangle {
        Triangle {
            points: [
                Vec3::new(-2.0, -2.0, z),
                Vec3::new(2.0, -2.0, z),
                Vec3::new(0.0, 2.0, z),
            ],
            double_sided,
        }
    }
    #[test]
    fn triangle_intersection_obeys_geometry_and_culling() {
        let ray = Ray {
            origin: Vec3::new(0.0, 0.0, 4.0),
            direction: -Vec3::Z,
        };
        assert_eq!(plane(1.0, false).hit(ray, 10.0), Some(3.0));
        assert_eq!(plane(1.0, false).hit(ray, 2.0), None);
        assert_eq!(
            plane(1.0, false).hit(
                Ray {
                    origin: Vec3::ZERO,
                    direction: Vec3::Z
                },
                10.0
            ),
            None
        );
        assert_eq!(
            plane(1.0, true).hit(
                Ray {
                    origin: Vec3::ZERO,
                    direction: Vec3::Z
                },
                10.0
            ),
            Some(1.0)
        );
        assert_eq!(
            plane(1.0, true).hit(
                Ray {
                    origin: Vec3::new(1.9, 1.9, 4.0),
                    ..ray
                },
                10.0
            ),
            None
        );
    }
    #[test]
    fn hierarchy_matches_bruteforce_including_parallel_and_inside_rays() {
        let triangles: Vec<_> = (0..50).map(|i| plane(i as f32 * 0.3, true)).collect();
        let geometry = Geometry::build(triangles.clone()).unwrap();
        for z in [-4.0, 0.1, 5.0, 30.0] {
            for direction in [Vec3::Z, -Vec3::Z, Vec3::X] {
                let ray = Ray {
                    origin: Vec3::new(0.0, 0.0, z),
                    direction,
                };
                let expected = triangles
                    .iter()
                    .filter_map(|t| t.hit(ray, f32::INFINITY))
                    .min_by(f32::total_cmp);
                assert_eq!(geometry.hit(ray, f32::INFINITY), expected);
            }
        }
        assert!(
            Geometry::build(vec![Triangle {
                points: [Vec3::NAN; 3],
                double_sided: true
            }])
            .is_err()
        );
        assert!(
            Geometry::default()
                .hit(
                    Ray {
                        origin: Vec3::ZERO,
                        direction: Vec3::Z
                    },
                    10.0
                )
                .is_none()
        );
    }
    #[test]
    fn scaled_rotated_instance_keeps_world_distance() {
        let triangle = plane(0.0, true);
        let transform = Mat4::from_cols_array_2d(&Instance::at([3.0, 2.0, -5.0], 0.7, 4.0).model);
        let inverse = transform.inverse();
        let ray = Ray {
            origin: transform.transform_point3(Vec3::new(0.0, 0.0, 10.0)),
            direction: transform.transform_vector3(-Vec3::Z).normalize(),
        };
        let local = Ray {
            origin: inverse.transform_point3(ray.origin),
            direction: inverse.transform_vector3(ray.direction),
        };
        assert!((triangle.hit(local, 100.0).unwrap() - 40.0).abs() < 0.0001);
    }
    fn object_at(geometry: &Geometry, z: f32, id: u32) -> Object {
        let transform = Mat4::from_translation(Vec3::new(0.0, 0.0, z));
        Object {
            instance_id: id,
            thing_id: format!("fixture{id}"),
            name: "Fixture".into(),
            category: None,
            rule_source: None,
            model_source: "fixture.nif".into(),
            native_position: [0.0, -z, 0.0],
            record_offset: 0,
            record_length: 0,
            source_index: id as usize,
            visible: true,
            model_index: 0,
            inverse: transform.inverse(),
            bounds: geometry.nodes[0].bounds.transformed(transform),
        }
    }
    #[test]
    fn selection_obeys_nearest_geometry_terrain_and_stable_source_order() {
        let geometry = Geometry::build(vec![plane(0.0, true)]).unwrap();
        let objects = vec![
            object_at(&geometry, 1.0, 42),
            object_at(&geometry, 3.0, 17),
            object_at(&geometry, 3.0, 6),
        ];
        let mut scene = SceneObjects {
            objects,
            models: vec![geometry],
            terrain: Geometry::default(),
        };
        let ray = Ray {
            origin: Vec3::new(0.0, 0.0, 10.0),
            direction: -Vec3::Z,
        };
        let selected = scene.pick(ray).unwrap();
        assert_eq!(scene.objects[selected.object].instance_id, 17);
        assert_eq!(selected.position.z, 3.0);
        scene.terrain = Geometry::build(vec![plane(4.0, false)]).unwrap();
        assert!(scene.pick(ray).is_none(), "Terrain must occlude objects");
        assert!(
            scene
                .pick(Ray {
                    origin: Vec3::NAN,
                    ..ray
                })
                .is_none()
        );
        assert!(
            scene
                .pick(Ray {
                    direction: Vec3::ZERO,
                    ..ray
                })
                .is_none()
        );
    }

    #[test]
    fn live_transform_and_visibility_keep_picking_at_rendered_position() {
        let geometry = Geometry::build(vec![plane(0.0, true)]).unwrap();
        let objects = vec![object_at(&geometry, 1.0, 42)];
        let mut scene = SceneObjects {
            objects,
            models: vec![geometry],
            terrain: Geometry::default(),
        };
        let ray = Ray {
            origin: Vec3::new(0.0, 0.0, 10.0),
            direction: -Vec3::Z,
        };
        assert!(scene.pick(ray).is_some());
        let update = ObjectTransformUpdate {
            object: 0,
            instance: Instance::at([7.0, 0.0, 1.0], 0.0, 1.0),
            native_position: [7.0, -1.0, 0.0],
            visible: true,
        };
        let prepared = scene.prepare_transforms(&[update]).unwrap();
        assert!(
            scene.pick(ray).is_some(),
            "staging must not publish before GPU acceptance"
        );
        scene.apply_transforms(prepared);
        assert!(scene.pick(ray).is_none());
        let moved_ray = Ray {
            origin: Vec3::new(7.0, 0.0, 10.0),
            ..ray
        };
        assert_eq!(
            scene.objects[scene.pick(moved_ray).unwrap().object].instance_id,
            42
        );
        assert_eq!(scene.objects[0].native_position, [7.0, -1.0, 0.0]);
        let mut invalid = update;
        invalid.instance.model[3][0] = f32::NAN;
        assert!(scene.prepare_transforms(&[invalid]).is_err());
        assert!(scene.pick(moved_ray).is_some());
        let hidden = scene
            .prepare_transforms(&[ObjectTransformUpdate {
                visible: false,
                ..update
            }])
            .unwrap();
        scene.apply_transforms(hidden);
        assert!(scene.pick(moved_ray).is_none());
    }

    #[test]
    #[ignore = "requires the local recovered original game assets"]
    fn original_map_selection_retains_identity_and_matches_brute_geometry() {
        let root = std::env::var_os("AA_ASSET_ROOT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../reconstruction")
            });
        let store = aa_assets::AssetStore::open(root).unwrap();
        let fixes = crate::model_fixes::ModelFixes::discover(&store).unwrap();
        let mut prepared = store
            .load_scene(
                "Campaign/tutorial01",
                aa_assets::HeightMode::NativeConditional,
            )
            .unwrap();
        for model in prepared.models.values_mut() {
            fixes.apply(&store, model).unwrap();
        }
        let scene = SceneObjects::build(&prepared, MemoryPolicy::default()).unwrap();
        assert!(!scene.objects.is_empty());
        for object in &scene.objects {
            let source = &prepared.map.placements.actors[object.source_index];
            assert_eq!(object.instance_id, source.instance_id);
            assert_eq!(object.thing_id, source.thing_id);
            assert_eq!(object.native_position, source.native_position());
            assert_eq!(object.record_offset, source.record_offset);
        }
        let camera = Camera::new([
            prepared.map.terrain.width as f32 * 2.0,
            prepared.map.terrain.height as f32 * 2.0,
        ]);
        let mut hit_count = 0;
        for object in scene
            .objects
            .iter()
            .step_by((scene.objects.len() / 20).max(1))
        {
            let target = (object.bounds.min + object.bounds.max) * 0.5;
            let ray = Ray {
                origin: camera.eye(),
                direction: (target - camera.eye()).normalize(),
            };
            let mut distance = scene
                .terrain
                .triangles
                .iter()
                .filter_map(|t| t.hit(ray, f32::INFINITY))
                .min_by(f32::total_cmp)
                .unwrap_or(f32::INFINITY);
            let mut expected = None;
            for (index, object) in scene.objects.iter().enumerate() {
                let local = Ray {
                    origin: object.inverse.transform_point3(ray.origin),
                    direction: object.inverse.transform_vector3(ray.direction),
                };
                for triangle in &scene.models[object.model_index].triangles {
                    if let Some(hit) = triangle.hit(local, distance)
                        && (expected.is_none() || hit < distance)
                    {
                        distance = hit;
                        expected = Some(index);
                    }
                }
            }
            let selected = scene.pick(ray);
            assert_eq!(selected.map(|s| s.object), expected);
            hit_count += usize::from(selected.is_some());
        }
        assert!(
            hit_count > 0,
            "Original geometry test must include positive hits"
        );
    }
    #[test]
    fn pointer_projection_roundtrips_at_windows_and_ui_scales() {
        let camera = Camera::new([1024.0, 1024.0]);
        for scale in [1.0_f32, 1.5, 2.25, 3.0] {
            let rect =
                egui::Rect::from_min_size(egui::pos2(281.5, 49.25), egui::vec2(1017.3, 647.1));
            let pixels = [
                (rect.width() * scale).round() as u32,
                (rect.height() * scale).round() as u32,
            ];
            let target = camera.target + Vec3::new(25.0, 0.0, -12.0);
            let pointer = SceneObjects::marker(&camera, rect, pixels, target).unwrap();
            let ray = pointer_ray(&camera, rect, pointer, pixels).unwrap();
            let t = (target.y - ray.origin.y) / ray.direction.y;
            assert!((ray.origin + ray.direction * t).distance(target) < 0.04);
            assert!(pointer_ray(&camera, rect, egui::pos2(10.0, 20.0), pixels).is_none());
        }
    }
    #[test]
    fn viewport_selection_rejects_geometry_beyond_the_render_far_plane() {
        let camera = Camera::new([1024.0, 1024.0]);
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0));
        let ray = pointer_ray(&camera, rect, rect.center(), [1600, 900]).unwrap();
        let center = ray.origin + ray.direction * 20000.0;
        let right = ray.direction.cross(Vec3::Y).normalize() * 100.0;
        let up = right.normalize().cross(ray.direction) * 100.0;
        let geometry = Geometry::build(vec![Triangle {
            points: [center - right - up, center + right - up, center + up],
            double_sided: true,
        }])
        .unwrap();
        let object = object_at(&geometry, 0.0, 1);
        let scene = SceneObjects {
            objects: vec![object],
            models: vec![geometry],
            terrain: Geometry::default(),
        };
        assert!(scene.pick(ray).is_some());
        assert!(
            scene
                .pick_at(&camera, rect, rect.center(), [1600, 900])
                .is_none()
        );
    }
}
