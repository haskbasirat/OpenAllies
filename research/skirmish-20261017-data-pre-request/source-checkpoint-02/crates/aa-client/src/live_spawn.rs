//! Source-resolved presentation of newly registered native actors. Does not
//! allocate simulation identities or choose a gameplay variant/state.
#[path = "live_animation.rs"]
pub mod animation;
use super::{ActorVisual, Binding, LiveScene};
use crate::{
    gpu::{CpuScene, SceneRenderer},
    inspection::{SceneObjects, SpawnObject},
    loading::LoadSettings,
};
use aa_assets::AssetStore;
use aa_numeric::Precision;
use aa_rules::{
    actors::{ActorCatalog, ResolvedNode},
    native::TypedValue,
    resources::Provenance,
};
use anyhow::{Context, Result, ensure};

#[derive(Clone, Debug)]
pub struct SpawnActor {
    /// Canonical Thing definition, not a NIF name or inspector label.
    pub thing_id: String,
    pub object_variant: i32,
    /// Already resolved native aspect index (e.g.45E5E0/4EAAA0), NOT Actor+15C
    /// state index. None asserts the ordinary/default view is currently active.
    pub resolved_aspect: Option<u32>,
    pub visual: ActorVisual,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct ModelBinding {
    pub thing_id: String,
    pub aspect_index: u32,
    pub aspect_id: String,
    pub source_folder: String,
    pub model_path: String,
    pub source: Provenance,
    pub fields: std::collections::BTreeMap<String, TypedValue>,
}
pub struct PreparedSpawn {
    request: SpawnActor,
    binding: ModelBinding,
    scene: CpuScene,
}
impl PreparedSpawn {
    pub fn binding(&self) -> &ModelBinding {
        &self.binding
    }
}
#[derive(Debug)]
pub struct SpawnReport {
    pub actor_id: u32,
    pub object: usize,
    pub draw_slots: usize,
    pub model_path: String,
    pub warnings: Vec<String>,
}
fn field(n: &ResolvedNode<'_>, key: &str, p: Precision) -> Result<TypedValue> {
    n.field(key, p)?
        .with_context(|| format!("Missing source visual field {key}"))
}
fn text(v: &TypedValue) -> Result<&str> {
    if let TypedValue::String { value, .. } = v {
        Ok(value)
    } else {
        anyhow::bail!("Expected native visual string")
    }
}
///The shared source-path domain; this wrapper preserves the existing client
///error transport and all caller/fixture ownership.
fn expand(format: &str, prefix: &str) -> Result<String> {
    aa_runtime::data_pre_request::path::expand(format, prefix).map_err(anyhow::Error::msg)
}
fn model_path(folder: &str, resource: &str, aspect: &str, format: &str) -> Result<String> {
    aa_runtime::data_pre_request::path::model_path(folder, resource, aspect, format)
        .map_err(anyhow::Error::msg)
}
///Original ObjectData loads AspectNormal first when variations are disabled,
///then authored Aspect rows. Native4EAAA0 falls back to49B480 normal index for
///ordinary variant=-1. Explicit state/construction overrides must be resolved
///by the caller; this function never treats a state index as an aspect index.
pub fn bind(catalog: &ActorCatalog, request: &SpawnActor, p: Precision) -> Result<ModelBinding> {
    let source = catalog
        .find(&request.thing_id)
        .context("Unknown canonical Thing ID")?;
    let actor = catalog.resolved_actor(&source.id)?;
    let view = actor.child("View")?.context("Source actor has no View")?;
    let variations = field(&view, "aspects_as_variations", p)?.boolean()?;
    let mut aspects = Vec::new();
    if !variations {
        aspects.push(
            view.child("AspectNormal")?
                .context("Missing native normal aspect")?,
        );
    }
    aspects.extend(view.children("Aspect")?);
    let selected = if let Some(index) = request.resolved_aspect {
        index
    } else if request.object_variant >= 0 {
        ensure!(
            variations,
            "Nonvariation actor requires a resolved native aspect when variant is nonnegative"
        );
        request.object_variant as u32
    } else {
        ensure!(
            request.object_variant == -1 && !variations,
            "Unresolved original visual variant"
        );
        0
    };
    let aspect = aspects
        .get(selected as usize)
        .context("Native visual aspect index outside source list")?;
    let mut fields = std::collections::BTreeMap::new();
    fields.insert(
        "resource_prefix".into(),
        field(&view, "resource_prefix", p)?,
    );
    for key in ["IDS", "aspect_prefix", "model", "skin", "glow"] {
        fields.insert(key.into(), field(aspect, key, p)?);
    }
    ensure!(
        text(&fields["skin"])?.is_empty() && text(&fields["glow"])?.is_empty(),
        "Source aspect requires unsupported texture/glow replacement"
    );
    ensure!(
        aspect.children("TextureRename")?.is_empty(),
        "Source aspect requires native TextureRename application"
    );
    let path = source.source.path.replace('\\', "/");
    let folder = path.rsplit_once('/').map_or("", |(folder, _)| folder);
    let model_path = model_path(
        folder,
        text(&fields["resource_prefix"])?,
        text(&fields["aspect_prefix"])?,
        text(&fields["model"])?,
    )?;
    Ok(ModelBinding {
        thing_id: source.id.clone(),
        aspect_index: selected,
        aspect_id: text(&fields["IDS"])?.into(),
        source_folder: folder.into(),
        model_path,
        source: source.source.clone(),
        fields,
    })
}
pub fn prepare(
    store: &AssetStore,
    catalog: &ActorCatalog,
    settings: LoadSettings,
    request: SpawnActor,
    p: Precision,
) -> Result<PreparedSpawn> {
    let binding = bind(catalog, &request, p)?;
    prepare_bound(store, settings, request, binding)
}
///The binding was resolved on the authoritative simulation thread. This
///render-only half never needs to transfer Catalog or the mutable ActorStore.
pub(super) fn prepare_bound(
    store: &AssetStore,
    settings: LoadSettings,
    request: SpawnActor,
    binding: ModelBinding,
) -> Result<PreparedSpawn> {
    ensure!(
        request.thing_id.eq_ignore_ascii_case(&binding.thing_id),
        "render binding Thing identity differs"
    );
    ensure!(
        request
            .resolved_aspect
            .is_none_or(|v| v == binding.aspect_index),
        "render binding aspect identity differs"
    );
    let instance = request.visual.instance()?;
    // Exact resource identity, full source material validation and existing UV
    //correction. load_model has no first-model or missing-resource fallback.
    let (mut scene, _) = crate::loading::load_model(store, &binding.model_path, settings)?;
    ensure!(
        scene.draws.iter().any(|d| !d.indices.is_empty()),
        "Original actor model has no retained geometry"
    );
    for draw in &mut scene.draws {
        draw.instances = vec![instance];
    }
    Ok(PreparedSpawn {
        request,
        binding,
        scene,
    })
}
impl LiveScene {
    ///A synchronous staged publication. Existing slots and scene epoch remain
    ///stable; failure before GPU acceptance cannot publish CPU picking/bindings.
    ///New actor resource residency is independent from old posed mesh buffers.
    pub fn append(
        &mut self,
        renderer: &mut SceneRenderer,
        objects: &mut SceneObjects,
        spawn: PreparedSpawn,
    ) -> Result<SpawnReport> {
        let epoch = self.epoch.context("Live scene is not attached")?;
        ensure!(
            epoch == renderer.scene_epoch(),
            "Spawn belongs to a previous scene"
        );
        let id = spawn.request.visual.actor_id;
        ensure!(
            !self.bindings.contains_key(&id) && !self.ambiguous_ids.contains(&id),
            "Duplicate native actor ID in live scene"
        );
        let visual = spawn.request.visual;
        let picking = objects.prepare_append(
            &spawn.scene.draws,
            SpawnObject {
                actor_id: id,
                thing_id: spawn.binding.thing_id.clone(),
                model_source: spawn.binding.model_path.clone(),
                rule_source: spawn.binding.source.path.clone(),
                instance: visual.instance()?,
                native_position: visual.native_position,
                visible: visual.visible,
            },
        )?;
        let slots = renderer.append_actor(epoch, &spawn.scene, visual.visible)?;
        let count = slots.len();
        let object = objects.append(picking);
        self.bindings.insert(
            id,
            Binding {
                slots,
                object,
                original: visual,
            },
        );
        Ok(SpawnReport {
            actor_id: id,
            object,
            draw_slots: count,
            model_path: spawn.binding.model_path,
            warnings: spawn.scene.warnings,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_source_resource_format_rejects_unresolved_inputs() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/phase7-live-spawn/native-model-binding.json"
        ))
        .unwrap();
        for row in fixture["paths"].as_array().unwrap() {
            assert_eq!(
                model_path(
                    row["folder"].as_str().unwrap(),
                    row["resource_prefix"].as_str().unwrap(),
                    row["aspect_prefix"].as_str().unwrap(),
                    row["model"].as_str().unwrap()
                )
                .unwrap(),
                row["result"].as_str().unwrap()
            );
        }
        assert_eq!(
            model_path("Units/USA", "USAInfantry/USAInfantry", "", "%s.nif").unwrap(),
            "Units/USA/USAInfantry/USAInfantry.nif"
        );
        assert_eq!(
            model_path("Units/USA", "ignored", "/Doodads/Test", "%s.nif").unwrap(),
            "Doodads/Test.nif"
        );
        assert_eq!(
            model_path("Units/USA", "ignored", "", "/Doodads/Test.nif").unwrap(),
            "Doodads/Test.nif"
        );
        assert_eq!(
            model_path("Units/USA", "../Shared/Foo", "Bar", "%s.nif").unwrap(),
            "Units/Shared/FooBar.nif"
        );
        for expression in ["%d.nif", "%s%s.nif", "%", "/../../bad.nif"] {
            assert!(model_path("Units/USA", "base", "", expression).is_err());
        }
    }
}

#[cfg(test)]
mod source_tests {
    use super::*;
    use std::sync::Arc;
    fn originals() -> (AssetStore, ActorCatalog, LoadSettings) {
        let game =
            std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"));
        let store = AssetStore::open(game.join("reconstruction")).unwrap();
        let mut sources = aa_rules::sources::V14Sources::open(
            game.join("Data.rwd"),
            game.join("reconstruction/native-game/runtime-expanded-v14"),
            aa_rules::sources::SourceLimits {
                max_member_bytes: 64 * 1024 * 1024,
            },
        )
        .unwrap();
        let catalog = ActorCatalog::load_v14(&mut sources).unwrap();
        let settings =
            LoadSettings::new(&store, 4096, aa_resources::MemoryPolicy::default()).unwrap();
        (store, catalog, settings)
    }
    fn request(id: u32, thing: &str) -> SpawnActor {
        SpawnActor {
            thing_id: thing.into(),
            object_variant: -1,
            resolved_aspect: None,
            visual: ActorVisual {
                actor_id: id,
                native_position: [0., 0., 0.],
                native_facing: [0., -1., 0.],
                scale: 1.,
                visible: true,
                team_color: Some([0.3, 0.7, 0.9]),
            },
        }
    }
    #[test]
    #[ignore = "requires original source/cache via AA_GAME_ROOT"]
    fn source_spawn_uses_original_models_and_normal_aspect_without_inspector_fallback() {
        let (store, catalog, settings) = originals();
        let mut rows = Vec::new();
        for (id, prefix) in [
            ("usa_infantry", "USA"),
            ("ger_infantry", "GER"),
            ("gbr_infantry", "GBR"),
            ("jpn_infantry", "JPN"),
            ("rus_infantry", "RUS"),
        ] {
            let r = request(42, id);
            let normal = prepare(
                &store,
                &catalog,
                settings.clone(),
                r.clone(),
                Precision::Double53,
            )
            .unwrap();
            assert_eq!(
                normal.binding.model_path,
                format!("Units/{prefix}/{prefix}Infantry/{prefix}Infantry.nif")
            );
            assert_eq!(normal.binding().aspect_index, 0);
            assert!(!normal.scene.draws.is_empty());
            rows.push(normal.binding);
            let count = catalog
                .native_creation_definition(id, Precision::Double53)
                .unwrap()
                .aspects
                .len();
            for index in 0..count {
                let mut r = r.clone();
                r.resolved_aspect = Some(index as u32);
                let selected =
                    prepare(&store, &catalog, settings.clone(), r, Precision::Double53).unwrap();
                assert_eq!(selected.binding.aspect_index, index as u32);
                rows.push(selected.binding);
            }
            let mut invalid = r.clone();
            invalid.resolved_aspect = Some(count as u32);
            assert!(bind(&catalog, &invalid, Precision::Double53).is_err());
            invalid.resolved_aspect = None;
            invalid.object_variant = -2;
            assert!(bind(&catalog, &invalid, Precision::Double53).is_err());
        }
        assert!(bind(&catalog, &request(43, "absent_thing"), Precision::Double53).is_err());
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../research/phase7-live-spawn/source-model-bindings.json");
        std::fs::write(out, serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
    }
    #[test]
    #[ignore = "requires original assets and graphics adapter via AA_GAME_ROOT"]
    fn live_spawn_stages_gpu_picking_and_preserves_old_epoch_slots_on_error() {
        let (store, catalog, settings) = originals();
        let mut scene =
            crate::loading::load(&store, "Campaign/tutorial01", settings.clone()).unwrap();
        let mut live = scene.live.take().unwrap();
        let mut objects = scene.objects.take().unwrap();
        let old = live
            .bindings
            .values()
            .find(|b| b.original.instance().is_ok())
            .unwrap()
            .original;
        let old_slots = live.bindings[&old.actor_id].slots.clone();
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
        let initial_count = objects.objects.len();
        let initial_bytes = renderer.uploaded_bytes;
        let mut camera = crate::camera::Camera::new(scene.size);
        camera.target.y = 325.0;
        let folder = std::env::temp_dir().join(format!(
            "aa-live-spawn-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&folder).unwrap();
        let capture = |renderer: &mut SceneRenderer, name: &str| {
            renderer.render(&camera, [320, 200]).unwrap();
            let path = folder.join(format!("{name}.png"));
            renderer.save_view(&path).unwrap();
            image::open(path).unwrap().to_rgba8()
        };
        let before = capture(&mut renderer, "before");
        let mut r = request(u32::MAX - 15, "usa_infantry");
        r.visual.native_position = [scene.size[0] * 0.5, scene.size[1] * 0.5, 300.];
        r.visual.scale = 25.;
        r.visual.visible = false;
        let mut malformed = prepare(
            &store,
            &catalog,
            settings.clone(),
            r.clone(),
            Precision::Double53,
        )
        .unwrap();
        //Late GPU validation failure, after picking staged successfully.
        malformed
            .scene
            .textures
            .values_mut()
            .next()
            .unwrap()
            .mips
            .clear();
        assert!(live.append(&mut renderer, &mut objects, malformed).is_err());
        assert_eq!(objects.objects.len(), initial_count);
        assert_eq!(renderer.uploaded_bytes, initial_bytes);
        assert!(!live.bindings.contains_key(&r.visual.actor_id));
        assert!(capture(&mut renderer, "invalid_gpu") == before);
        let spawn = prepare(
            &store,
            &catalog,
            settings.clone(),
            r.clone(),
            Precision::Double53,
        )
        .unwrap();
        let draw = spawn
            .scene
            .draws
            .iter()
            .find(|d| !d.indices.is_empty())
            .unwrap();
        let matrix = glam::Mat4::from_cols_array_2d(&draw.instances[0].model);
        let points: [glam::Vec3; 3] = std::array::from_fn(|i| {
            matrix.transform_point3(glam::Vec3::from(
                draw.vertices[draw.indices[i] as usize].position,
            ))
        });
        let point = (points[0] + points[1] + points[2]) / 3.0;
        let normal = (points[1] - points[0])
            .cross(points[2] - points[0])
            .normalize();
        let actor_ray = crate::inspection::Ray {
            origin: point + normal * 5.0,
            direction: -normal,
        };
        let report = live.append(&mut renderer, &mut objects, spawn).unwrap();
        assert_eq!(report.actor_id, r.visual.actor_id);
        assert_eq!(live.original_visual(report.actor_id), Some(r.visual));
        assert!(report.draw_slots > 0);
        assert_eq!(report.object, initial_count);
        assert_eq!(renderer.scene_epoch(), epoch);
        assert_eq!(live.bindings[&old.actor_id].slots, old_slots);
        assert_eq!(objects.objects.len(), initial_count + 1);
        assert!(!objects.objects[report.object].visible);
        assert!(capture(&mut renderer, "hidden_spawn") == before);
        let resident = renderer.uploaded_bytes;
        let duplicate = prepare(
            &store,
            &catalog,
            settings.clone(),
            r.clone(),
            Precision::Double53,
        )
        .unwrap();
        assert!(live.append(&mut renderer, &mut objects, duplicate).is_err());
        assert_eq!(objects.objects.len(), initial_count + 1);
        assert_eq!(resident, renderer.uploaded_bytes);
        r.visual.visible = true;
        let updated = live
            .apply(&mut renderer, &mut objects, 1, &[old, r.visual])
            .unwrap();
        assert_eq!(updated.sequence, 1);
        assert_eq!(updated.actors_applied, 2);
        assert!(objects.objects[report.object].visible);
        assert_eq!(renderer.uploaded_bytes, resident);
        let visible = capture(&mut renderer, "visible_spawn");
        assert!(
            visible != before,
            "Visible original spawned model must change pixels"
        );
        //Probe transformed triangles directly, without depending on arbitrary camera UI coordinates.
        let ray = actor_ray;
        assert_eq!(objects.pick(ray).map(|s| s.object), Some(report.object));
        r.visual.visible = false;
        live.apply(&mut renderer, &mut objects, 2, &[r.visual])
            .unwrap();
        assert_ne!(objects.pick(ray).map(|s| s.object), Some(report.object));
        let mut next = r.clone();
        next.visual.actor_id -= 1;
        let stale = prepare(&store, &catalog, settings, next, Precision::Double53).unwrap();
        renderer.upload(&scene).unwrap();
        assert!(live.append(&mut renderer, &mut objects, stale).is_err());
        assert_eq!(objects.objects.len(), initial_count + 1);
        assert!(!report.warnings.is_empty());
        assert!(report.model_path.ends_with("USAInfantry.nif"));
        println!("Live spawn GPU captures: {}", folder.display());
    }
}
