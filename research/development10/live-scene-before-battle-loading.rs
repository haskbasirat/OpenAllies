//! Snapshot-to-presentation binding for actual original map actors. This module
//! does not advance simulation, issue orders, synthesize units, or choose clips.
//! Existing model poses remain intact; facing is the verified native planar axis.
#[path = "live_battle_scene.rs"]
pub mod battle;
#[path = "battle_presenter.rs"]
pub mod presenter;
use crate::{
    gpu::{CpuDraw, Instance, InstanceSlot, InstanceUpdate, InstanceUpdateStats, SceneRenderer},
    inspection::{ObjectTransformUpdate, SceneObjects},
};
#[path = "live_spawn.rs"]
pub mod spawn;
use aa_assets::PreparedScene;
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorVisual {
    /// Native actor ID, including its generation bits; never a vector ordinal.
    pub actor_id: u32,
    pub native_position: [f32; 3],
    pub native_facing: [f32; 3],
    /// Final original model placement scale, not a new gameplay size modifier.
    pub scale: f32,
    pub visible: bool,
    /// None preserves the existing inspector-wide team-color control.
    pub team_color: Option<[f32; 3]>,
}

#[cfg(test)]
mod independent_review_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    #[ignore = "independent review: requires original tutorial assets and a graphics adapter"]
    fn live_apply_preserves_gpu_and_picking_on_invalid_snapshot_and_stale_epoch() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../reconstruction");
        let store = aa_assets::AssetStore::open(root).unwrap();
        let settings =
            crate::loading::LoadSettings::new(&store, 4096, aa_resources::MemoryPolicy::default())
                .unwrap();
        let mut scene = crate::loading::load(&store, "Campaign/tutorial01", settings).unwrap();
        let mut live = scene.live.take().unwrap();
        let mut objects = scene.objects.take().unwrap();
        let original = live
            .bindings
            .values()
            .filter_map(|b| b.original.instance().ok().map(|_| b.original))
            .take(2)
            .collect::<Vec<_>>();
        assert_eq!(original.len(), 2);
        let object_indices = original
            .iter()
            .map(|a| live.bindings[&a.actor_id].object)
            .collect::<Vec<_>>();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .unwrap();
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            Default::default(),
        );
        let mut renderer = SceneRenderer::new(egui_wgpu::RenderState {
            adapter,
            available_adapters: Vec::new(),
            device,
            queue,
            target_format: wgpu::TextureFormat::Rgba8UnormSrgb,
            renderer: Arc::new(eframe::egui::mutex::RwLock::new(egui_renderer)),
        })
        .unwrap();
        renderer.upload(&scene).unwrap();
        live.attach(renderer.scene_epoch()).unwrap();
        let camera = crate::camera::Camera::new(scene.size);
        let folder = std::env::temp_dir().join(format!(
            "aa-live-apply-review-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&folder).unwrap();
        let capture = |renderer: &mut SceneRenderer, name: &str| {
            renderer.render(&camera, [192, 128]).unwrap();
            let path = folder.join(format!("{name}.png"));
            renderer.save_view(&path).unwrap();
            image::open(path).unwrap().to_rgba8()
        };
        let before = capture(&mut renderer, "before");
        let initial_positions = object_indices
            .iter()
            .map(|&i| objects.objects[i].native_position)
            .collect::<Vec<_>>();
        let mut moved = original[0];
        moved.native_position[0] += 5.0;
        moved.native_position[1] += 7.0;
        let invalid = ActorVisual {
            scale: 0.0,
            ..original[1]
        };
        assert!(
            live.apply(&mut renderer, &mut objects, 10, &[moved, invalid])
                .is_err()
        );
        assert_eq!(live.last_sequence, None);
        assert_eq!(capture(&mut renderer, "invalid_actor"), before);
        assert_eq!(
            object_indices
                .iter()
                .map(|&i| objects.objects[i].native_position)
                .collect::<Vec<_>>(),
            initial_positions
        );
        let hidden = ActorVisual {
            visible: false,
            ..original[1]
        };
        let accepted = live
            .apply(&mut renderer, &mut objects, 10, &[moved, hidden])
            .unwrap();
        assert_eq!(accepted.actors_applied, 2);
        assert!(accepted.gpu.changed >= 2);
        assert_eq!(
            objects.objects[object_indices[0]].native_position,
            moved.native_position
        );
        assert!(!objects.objects[object_indices[1]].visible);
        let accepted_image = capture(&mut renderer, "accepted");
        let duplicate = live
            .apply(&mut renderer, &mut objects, 10, &original)
            .unwrap();
        assert!(duplicate.duplicate_snapshot);
        assert_eq!(duplicate.actors_applied, 0);
        assert!(
            live.apply(&mut renderer, &mut objects, 9, &original)
                .is_err()
        );
        assert!(
            live.apply(&mut renderer, &mut objects, 11, &[original[0], original[0]])
                .is_err()
        );
        //Force a GPU validation failure after CPU picking has successfully staged.
        //An earlier valid row must not publish either side of the failed snapshot.
        let slot = live.bindings.get_mut(&original[1].actor_id).unwrap().slots[0];
        live.bindings.get_mut(&original[1].actor_id).unwrap().slots[0].draw = usize::MAX;
        assert!(
            live.apply(&mut renderer, &mut objects, 11, &original)
                .is_err()
        );
        live.bindings.get_mut(&original[1].actor_id).unwrap().slots[0] = slot;
        assert_eq!(live.last_sequence, Some(10));
        assert_eq!(
            objects.objects[object_indices[0]].native_position,
            moved.native_position
        );
        assert!(!objects.objects[object_indices[1]].visible);
        assert_eq!(capture(&mut renderer, "invalid_gpu_slot"), accepted_image);
        let unknown_id = (0..=u32::MAX)
            .rev()
            .find(|id| !live.bindings.contains_key(id))
            .unwrap();
        let unknown = ActorVisual {
            actor_id: unknown_id,
            ..original[0]
        };
        let report = live
            .apply(&mut renderer, &mut objects, 11, &[unknown])
            .unwrap();
        assert_eq!(report.unbound_actor_ids, vec![unknown_id]);
        assert_eq!(report.actors_applied, 0);
        renderer.upload(&scene).unwrap();
        assert!(
            live.apply(&mut renderer, &mut objects, 12, &original)
                .is_err()
        );
        assert_eq!(
            objects.objects[object_indices[0]].native_position,
            moved.native_position
        );
        println!("Independent live snapshot captures: {}", folder.display());
    }
}
impl ActorVisual {
    fn instance(self) -> Result<Instance> {
        ensure!(
            self.native_position
                .iter()
                .chain(&self.native_facing)
                .all(|n| n.is_finite())
                && self.scale.is_finite()
                && self.scale > 0.0,
            "Invalid native visual transform for actor {}",
            self.actor_id
        );
        let [x, y, _] = self.native_facing;
        ensure!(
            x != 0.0 || y != 0.0,
            "Native planar facing is zero for actor {}",
            self.actor_id
        );
        //Original model forward is native-Y; this matches static placement load.
        let mut instance = Instance::at(
            aa_assets::native_to_render(self.native_position),
            x.atan2(-y),
            self.scale,
        );
        if let Some([r, g, b]) = self.team_color {
            ensure!(
                [r, g, b]
                    .iter()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
                "Invalid native team color for actor {}",
                self.actor_id
            );
            instance.team_color = [r, g, b, 1.0];
        }
        ensure!(
            glam::Mat4::from_cols_array_2d(&instance.model)
                .inverse()
                .is_finite(),
            "Native actor transform is not invertible"
        );
        Ok(instance)
    }
}

struct Binding {
    slots: Vec<InstanceSlot>,
    object: usize,
    original: ActorVisual,
}
pub struct LiveScene {
    map_id: Option<String>,
    rendered_map_source: Option<battle::RenderedMapSource>,
    bindings: BTreeMap<u32, Binding>,
    ambiguous_ids: Vec<u32>,
    epoch: Option<u64>,
    last_sequence: Option<u64>,
}
#[derive(Debug, Default)]
pub struct LiveUpdateReport {
    pub sequence: u64,
    pub actors_applied: usize,
    pub unbound_actor_ids: Vec<u32>,
    pub duplicate_snapshot: bool,
    pub gpu: InstanceUpdateStats,
}
impl LiveScene {
    ///An isolated presentation scene has no simulation actors until appended.
    pub(crate) fn empty() -> Self {
        Self {
            map_id: None,
            rendered_map_source: None,
            bindings: BTreeMap::new(),
            ambiguous_ids: Vec::new(),
            epoch: None,
            last_sequence: None,
        }
    }
    ///Bind using the same nonempty-draw compaction that SceneRenderer::upload
    ///uses. One actor may occupy several mesh draws; every slot is retained.
    pub fn from_scene(
        scene: &PreparedScene,
        draws: &[CpuDraw],
        objects: &SceneObjects,
    ) -> Result<Self> {
        ensure!(scene.map.terrain.id==scene.map.placements.id,"Live map source identities differ");
        let mut identities = BTreeMap::<u32, BTreeSet<usize>>::new();
        let mut by_source = BTreeMap::new();
        for placement in &scene.instances {
            identities
                .entry(placement.instance_id)
                .or_default()
                .insert(placement.source_index);
            ensure!(
                by_source
                    .insert(placement.source_index, placement)
                    .is_none(),
                "Repeated original placement source ordinal"
            );
        }
        let ambiguous_ids = identities
            .iter()
            .filter(|(_, sources)| sources.len() != 1)
            .map(|(&id, _)| id)
            .collect::<Vec<_>>();
        let object_indices = objects
            .objects
            .iter()
            .enumerate()
            .map(|(index, object)| (object.source_index, index))
            .collect::<BTreeMap<_, _>>();
        let mut bindings = BTreeMap::<u32, Binding>::new();
        for (draw_index, draw) in draws
            .iter()
            .filter(|draw| !draw.indices.is_empty() && !draw.instances.is_empty())
            .enumerate()
        {
            let Some(order) = &draw.source_order else {
                continue;
            };
            ensure!(
                order.len() == draw.instances.len(),
                "Live source order differs from instance count"
            );
            for (instance, [source_index, _]) in order.iter().enumerate() {
                let placement = by_source
                    .get(source_index)
                    .context("Live draw references an absent original placement")?;
                if ambiguous_ids.binary_search(&placement.instance_id).is_ok() {
                    continue;
                }
                let Some(&object) = object_indices.get(source_index) else {
                    continue;
                };
                let original = &scene.map.placements.actors[*source_index];
                let facing = original.orientation.map(|v| v.unwrap_or(0.0));
                let binding = bindings
                    .entry(placement.instance_id)
                    .or_insert_with(|| Binding {
                        slots: Vec::new(),
                        object,
                        original: ActorVisual {
                            actor_id: placement.instance_id,
                            native_position: original.native_position(),
                            native_facing: facing,
                            scale: placement.scale,
                            visible: true,
                            team_color: None,
                        },
                    });
                ensure!(
                    binding.object == object,
                    "One native actor ID refers to distinct picking objects"
                );
                binding.slots.push(InstanceSlot {
                    draw: draw_index,
                    instance: u32::try_from(instance)?,
                });
            }
        }
        Ok(Self {
            map_id: Some(scene.map.terrain.id.clone()),
            rendered_map_source: None,
            bindings,
            ambiguous_ids,
            epoch: None,
            last_sequence: None,
        })
    }
    pub(crate) fn seal_map_source(&mut self,source:battle::RenderedMapSource)->Result<()> {
        ensure!(self.rendered_map_source.is_none(),"rendered map source is already sealed");
        ensure!(self.map_id.as_deref()==Some(source.id()),"rendered map receipt belongs to another scene");
        self.rendered_map_source=Some(source);
        Ok(())
    }
    pub fn actor_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.bindings.keys().copied()
    }
    pub fn ambiguous_actor_ids(&self) -> &[u32] {
        &self.ambiguous_ids
    }
    pub fn original_visual(&self, actor_id: u32) -> Option<ActorVisual> {
        self.bindings.get(&actor_id).map(|b| b.original)
    }
    ///Call once after this scene's successful GPU upload. A stale binding cannot
    ///silently mutate a different map or an isolated model later loaded into it.
    pub fn attach(&mut self, epoch: u64) -> Result<()> {
        ensure!(epoch != 0, "No uploaded scene to attach live actors");
        self.epoch = Some(epoch);
        self.last_sequence = None;
        Ok(())
    }
    ///Named actors are deltas; omitted actors stay unchanged. Deletion/hiding
    ///must be explicit via visible=false. Sequence is the caller's monotonic
    ///snapshot serial, not a guessed engine tick duration or wrapped clock.
    pub fn apply(
        &mut self,
        renderer: &mut SceneRenderer,
        objects: &mut SceneObjects,
        sequence: u64,
        actors: &[ActorVisual],
    ) -> Result<LiveUpdateReport> {
        let epoch = self
            .epoch
            .context("Live actor bindings are not attached to an uploaded scene")?;
        ensure!(
            renderer.scene_epoch() == epoch,
            "Live actor bindings belong to a previous scene"
        );
        if let Some(previous) = self.last_sequence {
            ensure!(sequence >= previous, "Stale live presentation snapshot");
            if sequence == previous {
                return Ok(LiveUpdateReport {
                    sequence,
                    duplicate_snapshot: true,
                    ..Default::default()
                });
            }
        }
        let mut seen = BTreeSet::new();
        let mut instances = Vec::new();
        let mut picking = Vec::new();
        let mut report = LiveUpdateReport {
            sequence,
            ..Default::default()
        };
        for actor in actors {
            ensure!(
                seen.insert(actor.actor_id),
                "Repeated actor ID in one visual snapshot"
            );
            let value = actor.instance()?;
            let Some(binding) = self.bindings.get(&actor.actor_id) else {
                report.unbound_actor_ids.push(actor.actor_id);
                continue;
            };
            instances.try_reserve(binding.slots.len())?;
            instances.extend(binding.slots.iter().map(|&slot| InstanceUpdate {
                slot,
                value,
                visible: actor.visible,
            }));
            picking.push(ObjectTransformUpdate {
                object: binding.object,
                instance: value,
                native_position: actor.native_position,
                visible: actor.visible,
            });
            report.actors_applied += 1;
        }
        //Picking cannot move unless the corresponding GPU snapshot is accepted.
        let prepared = objects.prepare_transforms(&picking)?;
        report.gpu = renderer.update_instances(epoch, &instances)?;
        objects.apply_transforms(prepared);
        self.last_sequence = Some(sequence);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_planar_basis_and_team_color_are_explicit() {
        let actor = ActorVisual {
            actor_id: 42,
            native_position: [3.0, 5.0, 7.0],
            native_facing: [0.0, -1.0, 0.0],
            scale: 2.0,
            visible: true,
            team_color: Some([0.2, 0.4, 0.8]),
        };
        let value = actor.instance().unwrap();
        let matrix = glam::Mat4::from_cols_array_2d(&value.model);
        assert_eq!(
            matrix.transform_point3(glam::Vec3::ZERO),
            glam::Vec3::new(3.0, 7.0, -5.0)
        );
        assert_eq!(
            matrix.transform_vector3(glam::Vec3::Z),
            glam::Vec3::new(0.0, 0.0, 2.0)
        );
        assert_eq!(value.team_color, [0.2, 0.4, 0.8, 1.0]);
        assert_eq!(
            ActorVisual {
                team_color: None,
                ..actor
            }
            .instance()
            .unwrap()
            .team_color,
            [0.0; 4]
        );
        assert!(
            ActorVisual {
                scale: 0.0,
                ..actor
            }
            .instance()
            .is_err()
        );
        assert!(
            ActorVisual {
                native_facing: [0.0; 3],
                ..actor
            }
            .instance()
            .is_err()
        );
        assert!(
            ActorVisual {
                team_color: Some([f32::NAN, 0.0, 0.0]),
                ..actor
            }
            .instance()
            .is_err()
        );
    }

    #[test]
    #[ignore = "requires original recovered assets; run explicitly"]
    fn original_map_ids_bind_all_retained_mesh_slots_after_draw_compaction() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../reconstruction");
        let store = aa_assets::AssetStore::open(root).unwrap();
        let settings =
            crate::loading::LoadSettings::new(&store, 4096, aa_resources::MemoryPolicy::default())
                .unwrap();
        let scene = crate::loading::load(&store, "Campaign/tutorial01", settings).unwrap();
        let live = scene.live.as_ref().unwrap();
        let objects = scene.objects.as_ref().unwrap();
        let draws = scene
            .draws
            .iter()
            .filter(|d| !d.indices.is_empty() && !d.instances.is_empty())
            .collect::<Vec<_>>();
        assert!(live.actor_ids().count() > 0);
        assert!(live.bindings.values().any(|b| b.slots.len() > 1));
        let mut occupied = BTreeSet::new();
        for (&id, binding) in &live.bindings {
            assert_eq!(objects.objects[binding.object].instance_id, id);
            for slot in &binding.slots {
                assert!(
                    occupied.insert(*slot),
                    "distinct actor IDs share a GPU slot"
                );
                let draw = draws[slot.draw];
                assert_eq!(
                    draw.source_order.as_ref().unwrap()[slot.instance as usize][0],
                    objects.objects[binding.object].source_index
                );
                if let Ok(expected) = binding.original.instance() {
                    for (a, b) in expected.model.iter().flatten().zip(
                        draw.instances[slot.instance as usize]
                            .model
                            .iter()
                            .flatten(),
                    ) {
                        assert!(
                            (a - b).abs() < 0.0001,
                            "native basis differs from original static placement"
                        );
                    }
                }
            }
        }
    }
}
