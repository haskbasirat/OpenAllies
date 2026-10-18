//! Canonical Character state -> original source KF -> exclusively owned actor
//! mesh poses. No simulation clock, animation selection, or FX is synthesized.
use super::PreparedSpawn;
use crate::{
    gpu::{SceneRenderer, Vertex},
    inspection::{PoseMesh, SceneObjects},
    live_scene::LiveScene,
    loading::LoadSettings,
};
use aa_animation::{
    AnimatedAsset,
    controller_clock::{ClockCache, Controller, ControllerClock, sequence_time},
    definitions::BoundAnimation,
    runtime::{self, Character},
};
use aa_assets::AssetStore;
use aa_body::F32;
use aa_numeric::Precision;
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone)]
struct BoundClip {
    clip: usize,
    definition: BoundAnimation,
}
pub struct ActorRig {
    actor_id: u32,
    epoch: u64,
    thing_id: String,
    source: String,
    asset: AnimatedAsset,
    vertices: Vec<Vec<Vertex>>,
    double_sided: Vec<bool>,
    clips: BTreeMap<String, BoundClip>,
    active: Option<String>,
    clocks: Vec<ControllerClock>,
}
#[derive(Debug)]
pub struct PoseReport {
    pub actor_id: u32,
    pub source_kf: String,
    pub controller_times: Vec<F32>,
    pub vertex_count: usize,
}
impl PreparedSpawn {
    ///Prepare against the current scene epoch before consuming the spawn.
    ///Exact NIF identity is looked up in the source-hashed rig manifest.
    pub fn prepare_rig(
        &self,
        store: &AssetStore,
        settings: &LoadSettings,
        epoch: u64,
    ) -> Result<ActorRig> {
        ensure!(epoch != 0, "No scene to prepare an animated actor against");
        let folder = crate::animation_preview::cache_dir(store)
            .context("Recovered animation cache unavailable")?;
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(folder.join("manifest.json"))?)?;
        let rows = manifest["assets"]
            .as_array()
            .context("Invalid animation manifest")?;
        let matching = rows
            .iter()
            .filter(|r| {
                r["source"]
                    .as_str()
                    .is_some_and(|s| s.eq_ignore_ascii_case(&self.binding.model_path))
            })
            .collect::<Vec<_>>();
        ensure!(
            matching.len() == 1,
            "Original NIF has no unique recovered rig"
        );
        let row = matching[0];
        let file = row["file"]
            .as_str()
            .context("Rig manifest filename absent")?;
        ensure!(
            std::path::Path::new(file)
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
            "Invalid rig manifest path"
        );
        let path = folder.join(file);
        let bytes = std::fs::read(&path)?;
        ensure!(
            row["sha256"].as_str() == Some(format!("{:x}", Sha256::digest(&bytes)).as_str()),
            "Recovered rig manifest hash mismatch"
        );
        let asset = crate::animation_preview::verified_asset(store, &path, settings)?;
        ensure!(
            asset.source.eq_ignore_ascii_case(&self.binding.model_path),
            "Rig source differs from actual actor aspect"
        );
        ensure!(
            asset.meshes.len() == self.scene.draws.len(),
            "Rig mesh count differs from spawned source model"
        );
        for (mesh, draw) in asset.meshes.iter().zip(&self.scene.draws) {
            ensure!(
                mesh.indices == draw.indices
                    && mesh.positions.len() == draw.vertices.len()
                    && !draw.indices.is_empty(),
                "Rig topology differs from spawned source model"
            );
        }
        Ok(ActorRig {
            actor_id: self.request.visual.actor_id,
            epoch,
            thing_id: self.binding.thing_id.clone(),
            source: asset.source.clone(),
            vertices: self
                .scene
                .draws
                .iter()
                .map(|d| d.vertices.clone())
                .collect(),
            double_sided: self
                .scene
                .draws
                .iter()
                .map(|d| d.material.double_sided)
                .collect(),
            asset,
            clips: BTreeMap::new(),
            active: None,
            clocks: Vec::new(),
        })
    }
}
impl ActorRig {
    ///Explicit preview reactivation; does not alter the authoritative Character.
    pub(crate) fn reset_controllers(&mut self) {
        self.active = None;
        self.clocks.clear();
    }
    ///Caller obtains this definition by resolving Character.animation through
    ///the canonical scoped bank. Authored clip labels are never lookup keys.
    pub fn bind_clip(
        &mut self,
        store: &AssetStore,
        key: &str,
        definition: &BoundAnimation,
    ) -> Result<()> {
        ensure!(
            !self.clips.contains_key(key),
            "Canonical animation key is already bound"
        );
        //All supported infantry definitions use absolute original KF resources.
        //Relative/format expressions need their native set-prefix resolution.
        let resource = definition
            .kf_expression
            .strip_prefix('/')
            .context("Relative KF expression requires native animation-set prefix binding")?;
        ensure!(
            !resource.contains('%')
                && !resource.contains('\\')
                && !resource
                    .split('/')
                    .any(|s| s.is_empty() || s == ".." || s == "."),
            "Unsupported original KF resource expression"
        );
        let matches = self
            .asset
            .clips
            .iter()
            .enumerate()
            .filter(|(_, c)| c.source.eq_ignore_ascii_case(resource))
            .collect::<Vec<_>>();
        ensure!(
            matches.len() == 1,
            "Actual source KF has no unique supported clip in this original rig"
        );
        let (clip, source) = matches[0];
        ensure!(
            source.tracks.iter().all(|t| t.flags & 32 != 0),
            "Inactive source controller needs native target update policy"
        );
        let path = store.resolve_url(&format!("/assets/original/Data/{resource}"))?;
        ensure!(
            format!("{:x}", Sha256::digest(std::fs::read(path)?)) == source.source_sha256,
            "Original KF identity mismatch"
        );
        runtime::canonical_looping_type(definition.timing.kind)?;
        ensure!(
            f32::from_bits(definition.timing.length.0) > 0.,
            "Invalid authored animation length"
        );
        self.clips.insert(
            key.into(),
            BoundClip {
                clip,
                definition: definition.clone(),
            },
        );
        Ok(())
    }
    ///Keep one shared clock cache for all posed actors in actual presentation
    ///traversal order. State is borrowed directly from its authoritative box.
    ///Sequence/controller state and picking publish only after GPU acceptance.
    #[allow(clippy::too_many_arguments)] // Separate native state, clock, picking and GPU authorities.
    pub fn pose(
        &mut self,
        live: &LiveScene,
        renderer: &mut SceneRenderer,
        objects: &mut SceneObjects,
        state: &Character<String>,
        client_time: F32,
        cache: &mut ClockCache,
        p: Precision,
    ) -> Result<PoseReport> {
        ensure!(
            renderer.scene_epoch() == self.epoch && live.epoch == Some(self.epoch),
            "Rig belongs to a previous scene"
        );
        ensure!(
            state.header.owner.raw() == self.actor_id,
            "Character belongs to a different native actor"
        );
        let key = state
            .animation
            .as_ref()
            .context("Character has no current native animation")?;
        let bound = self
            .clips
            .get(key)
            .context("Current canonical animation has not been source-bound")?;
        let binding = live
            .bindings
            .get(&self.actor_id)
            .context("Animated actor has not been published to this scene")?;
        let object = objects
            .objects
            .get(binding.object)
            .context("Animated picking actor absent")?;
        ensure!(
            object.thing_id.eq_ignore_ascii_case(&self.thing_id)
                && object.model_source.eq_ignore_ascii_case(&self.source),
            "Rig does not match actor's current source aspect"
        );
        ensure!(
            binding.slots.len() == self.asset.meshes.len(),
            "Animated actor mesh slot mapping changed"
        );
        let clip = &self.asset.clips[bound.clip];
        let parameters = runtime::playback_parameters(
            state,
            client_time,
            F32(clip.duration.to_bits()),
            bound.definition.timing.length,
            p,
        )?;
        let input = sequence_time(client_time, parameters.start, p)?;
        let mut clocks = if self.active.as_ref() == Some(key) {
            self.clocks.clone()
        } else {
            vec![ControllerClock::default(); clip.tracks.len()]
        };
        ensure!(
            clocks.len() == clip.tracks.len(),
            "Controller state layout changed"
        );
        let mut staged_cache = cache.clone();
        let mut times = Vec::with_capacity(clocks.len());
        for (clock, track) in clocks.iter_mut().zip(&clip.tracks) {
            let c = Controller {
                flags: u16::try_from(track.flags)?,
                frequency: parameters.rate,
                phase: F32(track.phase.to_bits()),
                start: F32(track.start.to_bits()),
                stop: F32(track.stop.to_bits()),
            };
            times.push(clock.update(input, c, &mut staged_cache, p)?);
        }
        let pose = self.asset.sample_controller_times(
            bound.clip,
            &times
                .iter()
                .map(|v| f32::from_bits(v.0))
                .collect::<Vec<_>>(),
        )?;
        ensure!(
            pose.meshes.len() == self.vertices.len(),
            "Native sampled mesh count changed"
        );
        let mut vertices = self.vertices.clone();
        for (mesh, output) in pose.meshes.iter().zip(&mut vertices) {
            ensure!(
                mesh.positions.len() == output.len() && mesh.normals.len() == output.len(),
                "Native pose vertex layout changed"
            );
            for (v, (position, normal)) in output
                .iter_mut()
                .zip(mesh.positions.iter().zip(&mesh.normals))
            {
                v.position = *position;
                v.normal = *normal;
            }
        }
        let picking = objects.prepare_pose(
            binding.object,
            &vertices
                .iter()
                .zip(&self.asset.meshes)
                .zip(&self.double_sided)
                .map(|((vertices, m), &double_sided)| PoseMesh {
                    vertices,
                    indices: &m.indices,
                    double_sided,
                })
                .collect::<Vec<_>>(),
        )?;
        let updates = binding
            .slots
            .iter()
            .copied()
            .zip(vertices)
            .collect::<Vec<_>>();
        renderer.update_actor_vertices(self.epoch, &updates)?;
        objects.apply_pose(picking);
        self.clocks = clocks;
        self.active = Some(key.clone());
        *cache = staged_cache;
        self.vertices = updates.into_iter().map(|(_, vertices)| vertices).collect();
        Ok(PoseReport {
            actor_id: self.actor_id,
            source_kf: clip.source.clone(),
            controller_times: times,
            vertex_count: self.vertices.iter().map(Vec::len).sum(),
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        inspection::Ray,
        live_scene::{
            ActorVisual,
            spawn::{self, SpawnActor},
        },
    };
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
    };
    use std::sync::Arc;
    #[test]
    #[ignore = "requires original assets/source and graphics adapter via AA_GAME_ROOT"]
    fn live_animation_uses_borrowed_character_source_kf_and_atomic_actor_pose() {
        let game =
            std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"));
        let store = AssetStore::open(game.join("reconstruction")).unwrap();
        let mut sources = V14Sources::open(
            game.join("Data.rwd"),
            game.join("reconstruction/native-game/runtime-expanded-v14"),
            SourceLimits {
                max_member_bytes: 64 * 1024 * 1024,
            },
        )
        .unwrap();
        let catalog = ActorCatalog::load_v14(&mut sources).unwrap();
        let definitions =
            aa_animation::definitions::character(&catalog, "usa_infantry", Precision::Double53)
                .unwrap()
                .unwrap();
        let jog = definitions.base.iter().find(|v| v.id == "jog").unwrap();
        let settings =
            LoadSettings::new(&store, 4096, aa_resources::MemoryPolicy::default()).unwrap();
        let mut scene =
            crate::loading::load(&store, "Campaign/tutorial01", settings.clone()).unwrap();
        let mut live = scene.live.take().unwrap();
        let mut objects = scene.objects.take().unwrap();
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
            available_adapters: vec![],
            device,
            queue,
            target_format: wgpu::TextureFormat::Rgba8UnormSrgb,
            renderer: Arc::new(eframe::egui::mutex::RwLock::new(egui_renderer)),
        })
        .unwrap();
        renderer.upload(&scene).unwrap();
        let epoch = renderer.scene_epoch();
        live.attach(epoch).unwrap();
        let actor_id = u32::MAX - 24;
        let visual = ActorVisual {
            actor_id,
            native_position: [scene.size[0] / 2., scene.size[1] / 2., 300.],
            native_facing: [0., -1., 0.],
            scale: 40.,
            visible: true,
            team_color: Some([0.25, 0.8, 0.4]),
        };
        let prepared = spawn::prepare(
            &store,
            &catalog,
            settings,
            SpawnActor {
                thing_id: "usa_infantry".into(),
                object_variant: -1,
                resolved_aspect: None,
                visual,
            },
            Precision::Double53,
        )
        .unwrap();
        let mut rig = prepared
            .prepare_rig(
                &store,
                &LoadSettings::new(&store, 4096, aa_resources::MemoryPolicy::default()).unwrap(),
                epoch,
            )
            .unwrap();
        let key = "usa_infantry:set3:jog-row".to_owned();
        rig.bind_clip(&store, &key, jog).unwrap();
        assert!(rig.bind_clip(&store, &key, jog).is_err());
        let mut wrong = jog.clone();
        wrong.kf_expression = "%sJog.kf".into();
        assert!(rig.bind_clip(&store, "relative", &wrong).is_err());
        let missing = definitions
            .base
            .iter()
            .find(|v| v.id == "projectileattack1")
            .unwrap();
        assert!(rig.bind_clip(&store, "missing_recovery", missing).is_err());
        let report = live.append(&mut renderer, &mut objects, prepared).unwrap();
        let mut state = Box::new(Character::new(
            aa_actors::ActorId::from_raw(actor_id),
            &[Some(key.clone())],
        ));
        let address = state.as_ref() as *const _ as usize;
        let mut cache = ClockCache::default();
        let mut camera = crate::camera::Camera::new(scene.size);
        camera.target.y = 325.;
        camera.zoom = 0.7;
        let folder = std::env::temp_dir().join(format!(
            "aa-live-animation-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&folder).unwrap();
        let capture = |r: &mut SceneRenderer, name: &str| {
            r.render(&camera, [384, 256]).unwrap();
            let path = folder.join(format!("{name}.png"));
            r.save_view(&path).unwrap();
            image::open(path).unwrap().to_rgba8()
        };
        let f = |n: f32| F32(n.to_bits());
        let first = rig
            .pose(
                &live,
                &mut renderer,
                &mut objects,
                &state,
                f(0.),
                &mut cache,
                Precision::Double53,
            )
            .unwrap();
        assert_eq!(first.actor_id, actor_id);
        assert!(first.source_kf.ends_with("/Jog.kf"));
        assert_eq!(first.vertex_count, 230);
        assert!(first.controller_times.iter().all(|v| *v == F32::ZERO));
        let image0 = capture(&mut renderer, "jog-start");
        rig.pose(
            &live,
            &mut renderer,
            &mut objects,
            &state,
            f(0.5),
            &mut cache,
            Precision::Double53,
        )
        .unwrap();
        let image1 = capture(&mut renderer, "jog-frame");
        assert!(
            image0 != image1,
            "Original jog must visibly change actor pose"
        );
        let before_clocks = serde_json::to_value(&rig.clocks).unwrap();
        let oldslot = live.bindings[&actor_id].slots[0];
        live.bindings.get_mut(&actor_id).unwrap().slots[0].draw = usize::MAX;
        assert!(
            rig.pose(
                &live,
                &mut renderer,
                &mut objects,
                &state,
                f(0.7),
                &mut cache,
                Precision::Double53
            )
            .is_err()
        );
        live.bindings.get_mut(&actor_id).unwrap().slots[0] = oldslot;
        assert_eq!(serde_json::to_value(&rig.clocks).unwrap(), before_clocks);
        assert!(capture(&mut renderer, "rejected-pose") == image1);
        //Picking follows the current posed original triangle, not the old bind pose.
        let matrix = glam::Mat4::from_cols_array_2d(&visual.instance().unwrap().model);
        let indices = &rig.asset.meshes[0].indices;
        let points: [glam::Vec3; 3] = std::array::from_fn(|i| {
            matrix.transform_point3(glam::Vec3::from(
                rig.vertices[0][indices[i] as usize].position,
            ))
        });
        let center = (points[0] + points[1] + points[2]) / 3.;
        let normal = (points[1] - points[0])
            .cross(points[2] - points[0])
            .normalize();
        assert_eq!(
            objects
                .pick(Ray {
                    origin: center + normal * 5.,
                    direction: -normal
                })
                .map(|s| s.object),
            Some(report.object)
        );
        runtime::pause(&mut state, f(0.5)).unwrap();
        rig.pose(
            &live,
            &mut renderer,
            &mut objects,
            &state,
            f(2.5),
            &mut cache,
            Precision::Double53,
        )
        .unwrap();
        assert!(capture(&mut renderer, "paused") == image1);
        runtime::resume(&mut state, f(3.5), Precision::Double53).unwrap();
        rig.pose(
            &live,
            &mut renderer,
            &mut objects,
            &state,
            f(3.75),
            &mut cache,
            Precision::Double53,
        )
        .unwrap();
        let resumed = capture(&mut renderer, "resumed");
        assert!(resumed != image1);
        assert_eq!(state.as_ref() as *const _ as usize, address);
        assert_eq!(state.animation.as_ref(), Some(&key));
        let bytes = renderer.uploaded_bytes;
        let moved = ActorVisual {
            native_position: [
                visual.native_position[0] + 25.,
                visual.native_position[1],
                300.,
            ],
            ..visual
        };
        live.apply(&mut renderer, &mut objects, 1, &[moved])
            .unwrap();
        rig.pose(
            &live,
            &mut renderer,
            &mut objects,
            &state,
            f(3.75),
            &mut cache,
            Precision::Double53,
        )
        .unwrap();
        assert!(capture(&mut renderer, "moved") != resumed);
        assert_eq!(renderer.uploaded_bytes, bytes);
        renderer.upload(&scene).unwrap();
        assert!(
            rig.pose(
                &live,
                &mut renderer,
                &mut objects,
                &state,
                f(3.7),
                &mut cache,
                Precision::Double53
            )
            .is_err()
        );
        println!(
            "Live original actor animation captures: {}",
            folder.display()
        );
    }
}
