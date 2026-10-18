use crate::gpu::{CpuDraw, CpuMaterial, CpuScene, CpuTexture, Instance, Vertex};
use aa_assets::{AssetStore, HeightMode};
use aa_resources::{MemoryEstimate, MemoryPolicy, count_bytes, preflight, rgba8_mip_chain_bytes};
use anyhow::{Context, Result, bail, ensure};
use rayon::prelude::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
    time::Instant,
};

#[path = "water_render.rs"]
mod water_render;
#[path = "battle_map_upload.rs"]
pub mod battle_map;

#[derive(Clone)]
pub struct LoadSettings {
    pub max_texture_dimension: u32,
    pub memory_policy: MemoryPolicy,
    pub model_fixes: Arc<crate::model_fixes::ModelFixes>,
    pub visibility: Arc<crate::visibility::Visibility>,
}
impl LoadSettings {
    pub fn new(
        store: &AssetStore,
        max_texture_dimension: u32,
        memory_policy: MemoryPolicy,
    ) -> Result<Self> {
        let bytes = crate::model_fixes::ModelFixes::metadata_bytes(store)?;
        preflight(
            MemoryEstimate {
                temporary_cpu_bytes: count_bytes(bytes, 12)?,
                ..Default::default()
            },
            memory_policy,
        )?;
        let model_fixes = Arc::new(crate::model_fixes::ModelFixes::discover(store)?);
        log::info!(
            "Model metadata {} SHA256 {}",
            model_fixes.metadata_path().display(),
            model_fixes.metadata_sha256()
        );
        Ok(Self {
            max_texture_dimension,
            memory_policy,
            model_fixes,
            visibility: Arc::new(crate::visibility::Visibility::new(store)),
        })
    }
}

/// The recovered mesh files targeted Three.js and store (u, 1-native_v).
/// wgpu uploads PNG rows top-first; restore native Direct3D coordinates here.
/// Terrain UVs are generated for this renderer and do not use this conversion.
pub fn model_uv(uv: [f32; 2]) -> [f32; 2] {
    [uv[0], 1.0 - uv[1]]
}

fn checked_sum(values: impl IntoIterator<Item = u64>) -> Result<u64> {
    values.into_iter().try_fold(0u64, |sum, n| {
        sum.checked_add(n).context("Asset byte estimate overflow")
    })
}

fn preflight_source(store: &AssetStore, id: &str, settings: LoadSettings) -> Result<()> {
    let info = store
        .maps()
        .iter()
        .find(|m| m.id == id)
        .context("Unknown full map ID")?;
    let json_bytes = checked_sum([
        std::fs::metadata(store.resolve_url(&format!("/data/maps/{id}.json"))?)?.len(),
        std::fs::metadata(store.resolve_url(&format!("/data/map-actors/{id}.json"))?)?.len(),
    ])?;
    // A conservative estimate for JSON parsing, terrain construction, temporary
    // index remapping and overlapping prepared/renderer vertex arrays. The policy
    // scales with observed physical/commit headroom; it is not a reservation.
    let cells = u64::from(info.width) * u64::from(info.height);
    let base = checked_sum([count_bytes(json_bytes, 12)?, count_bytes(cells, 800)?])?;
    preflight(
        MemoryEstimate {
            temporary_cpu_bytes: base,
            ..Default::default()
        },
        settings.memory_policy,
    )?;
    let map = store.load_map(id)?;
    let models: BTreeSet<_> = map
        .placements
        .types
        .values()
        .flat_map(|t| t.models.iter().map(|m| m.path.as_str()))
        .collect();
    let mut model_bytes = 0u64;
    for path in models {
        if let Ok(url) = store.model_url(path) {
            model_bytes = checked_sum([
                model_bytes,
                std::fs::metadata(store.resolve_url(url)?)?.len(),
            ])?;
        }
    }
    drop(map);
    preflight(
        MemoryEstimate {
            temporary_cpu_bytes: checked_sum([base, count_bytes(model_bytes, 12)?])?,
            ..Default::default()
        },
        settings.memory_policy,
    )?;
    Ok(())
}

pub fn load(store: &AssetStore, id: &str, settings: LoadSettings) -> Result<CpuScene> {
    load_impl(store, id, settings, None, None)
}

/// Static map scene for gameplay: placements owned by the running game
/// (cities, start markers, placeholders) are left to the dynamic layer and
/// the inspection/picking structures are not built.
pub fn load_for_game(
    store: &AssetStore,
    id: &str,
    settings: LoadSettings,
    exclude: &BTreeSet<u32>,
) -> Result<CpuScene> {
    load_impl(store, id, settings, Some(exclude), None)
}

///Static rendering plus a source-certified dynamic layer. Exclusions come
///from the same loaded graph, including original records already destroyed.
///The presenter must subsequently bind the current native renderables.
pub fn load_for_battle(store:&AssetStore,id:&str,settings:LoadSettings,
    staged:&aa_runtime::restoration::Staged)->Result<CpuScene> {
    let inventory=&staged.inventory;
    ensure!(inventory.dynamic_things.map_source==inventory.map_source,
        "battle inventory DynamicThings belongs to another map");
    ensure!(inventory.dynamic_things.unsupported.is_empty() && inventory.generation_mismatches.is_empty()
        && inventory.team_generation_mismatches.is_empty() && inventory.kingdom_generation_mismatches.is_empty(),
        "battle exclusions require complete same-generation native restore records");
    let teams=inventory.teams.as_ref().context("battle exclusion Team records are unbound")?;
    let kingdoms=inventory.kingdoms.as_ref().context("battle exclusion Kingdom records are unbound")?;
    let mut exclude=inventory.dynamic_things.placements.iter().map(|p|p.serialized_id).collect::<BTreeSet<_>>();
    exclude.extend(teams.slots.iter().filter_map(|s|s.record.as_ref()).map(|r|r.serialized_id));
    exclude.extend(kingdoms.slots.iter().filter_map(|s|s.record.as_ref()).map(|r|r.serialized_id));
    //Verify immutable record identities against the selected archive. This is
    //a source check, never a substitute runtime/ActorStore or copied live state.
    let info=store.maps().iter().find(|m|m.id==id).context("battle selected map is absent")?;
    let raw=aa_mapworld::MapArchive::from_bytes(std::fs::read(store.resolve_url(&format!("/assets/original/Data/{}",info.source))?)?)?;
    ensure!(raw.source()==&inventory.map_source,"battle restore inventory belongs to another selected map");
    let dynamic=aa_mapworld::inventory::Inventory::decode(&raw,u64::MAX)?;
    let raw_teams=aa_mapworld::kingdoms::Teams::decode(&raw.decode_chunk("TEAMS",u64::MAX)?)?;
    let raw_kingdoms=aa_mapworld::kingdoms::Kingdoms::decode(&raw.decode_chunk("KINGDOMS",u64::MAX)?,inventory.world.version)?;
    let mut source_ids=dynamic.placements.iter().map(|p|p.serialized_id).collect::<BTreeSet<_>>();
    source_ids.extend(raw_teams.slots.iter().filter_map(|s|s.record.as_ref()).map(|r|r.serialized_id));
    source_ids.extend(raw_kingdoms.slots.iter().filter_map(|s|s.record.as_ref()).map(|r|r.serialized_id));
    ensure!(source_ids==exclude,"battle restore record identities differ from the selected native archive");
    exclude.extend(staged.actors.identities().iter().map(|e|e.id.raw()));
    load_impl(store,id,settings,Some(&exclude),Some(&inventory.map_source))
}

fn load_impl(
    store: &AssetStore,
    id: &str,
    settings: LoadSettings,
    exclude: Option<&BTreeSet<u32>>,
    battle: Option<&aa_mapworld::SourceIdentity>,
) -> Result<CpuScene> {
    let start = Instant::now();
    preflight_source(store, id, settings.clone())?;
    let mut scene = store.load_scene(id, HeightMode::NativeConditional)?;
    let rendered_map_source =
        crate::live_scene::battle::RenderedMapSource::bind_exact(store, &mut scene);
    if let Some(source)=battle {
        rendered_map_source.as_ref().map_err(|e|anyhow::anyhow!("battle map source binding failed: {e:#}"))?
            .require_identity(source)?;
        //Prepared rendering data only. Original archive/restore records and
        //source ordinals remain intact; dynamic models come from current owners.
        scene.instances.retain(|p|!exclude.is_some_and(|set|set.contains(&p.instance_id)));
    }
    let visibility = settings
        .visibility
        .apply(store, settings.memory_policy, &mut scene);
    let corrected: Vec<_> = scene
        .models
        .par_iter_mut()
        .map(|(_, model)| settings.model_fixes.apply(store, model))
        .collect();
    for result in corrected {
        result?;
    }
    let mut warnings: Vec<_> = scene
        .warnings
        .iter()
        .map(|w| format!("{}: {}", w.source, w.message))
        .collect();
    match visibility {
        Ok(stats) => {
            log::info!(
                "Authored visibility hid {} map instances, source hashes {:?}",
                stats.hidden_instances,
                stats.sources
            );
            if stats.hidden_instances != 0 {
                warnings.push(format!("Authored Flags.invisible hides {} editor/audio marker instances; all source placement records are preserved.", stats.hidden_instances));
            }
            if !stats.unresolved_ids.is_empty() {
                warnings.push(format!(
                    "Visibility remains unresolved for {} Thing IDs; their geometry is unchanged.",
                    stats.unresolved_ids.len()
                ));
                log::warn!("Unresolved authored visibility: {:?}", stats.unresolved_ids);
            }
        }
        Err(error) => warnings.push(format!(
            "Authored visibility source loading failed; map geometry is unchanged: {error:#}"
        )),
    }
    let (mut draws, terrain_warnings) =
        crate::terrain_render::build(&scene, settings.memory_policy)?;
    // Preserve the source JSON, while removing its obsolete viewer limitation.
    warnings.retain(|warning| !warning.contains("Terrain blending, tile orientation flags, scenario actors and mission scripts are not applied in the viewer."));
    warnings.extend(terrain_warnings);
    match water_render::build(store, &scene, settings.memory_policy) {
        Ok((water_draws, water_warnings)) => {
            draws.extend(water_draws);
            warnings.extend(water_warnings);
        }
        Err(error) => warnings.push(format!(
            "Original water source binding failed; water omitted: {error:#}"
        )),
    }
    let mut grouped = BTreeMap::<String, Vec<(Instance, usize)>>::new();
    for instance in &scene.instances {
        if exclude.is_some_and(|e| e.contains(&instance.instance_id)) {
            continue;
        }
        if let Some(key) = &instance.model_key {
            if scene
                .models
                .get(key)
                .is_none_or(|model| model.meshes.is_empty())
            {
                continue;
            }
            grouped.entry(key.clone()).or_default().push((
                Instance::at(instance.position, instance.yaw_radians, instance.scale),
                instance.source_index,
            ));
        }
    }
    let rendered_instances = grouped.values().map(Vec::len).sum();
    for (key, instances) in grouped {
        let Some(model) = scene.models.get(&key) else {
            continue;
        };
        for (mesh_index, mesh) in model.meshes.iter().enumerate() {
            let m = &mesh.material;
            draws.push(CpuDraw {
                vertices: mesh
                    .vertices
                    .iter()
                    .map(|v| Vertex {
                        terrain_uv: [0.0; 2],
                        position: v.position,
                        normal: v.normal,
                        uv: model_uv(v.uv),
                        color: v.color,
                    })
                    .collect(),
                indices: mesh.indices.clone(),
                instances: instances.iter().map(|(instance, _)| *instance).collect(),
                // aa-assets already placed the material diffuse/opacity in each vertex.
                material: material(m),
                source_order: Some(
                    instances
                        .iter()
                        .map(|(_, source)| [*source, mesh_index])
                        .collect(),
                ),
                sort_center: m.native_sort_center,
            });
        }
    }
    let name = scene.map.terrain.name.clone();
    let size = [
        scene.map.terrain.width as f32 * scene.map.terrain.cell_size,
        scene.map.terrain.height as f32 * scene.map.terrain.cell_size,
    ];
    let placements = scene.map.placements.actors.len();
    let (objects, live) = if exclude.is_some() && battle.is_none() {
        (None, None)
    } else {
        let objects = crate::inspection::SceneObjects::build(&scene, settings.memory_policy)?;
        let mut live = crate::live_scene::LiveScene::from_scene(&scene, &draws, &objects)?;
        if battle.is_some() {
            //Remaining original objects are outside the native graph. They
            //remain static GPU/picking geometry, not mutable native bindings.
            live.detach_static_bindings();
        }
        match rendered_map_source {
            Ok(source)=>live.seal_map_source(source)?,
            Err(error)=>warnings.push(format!("Map source correspondence is unverified; native battle binding is unavailable: {error:#}")),
        }
        if !live.ambiguous_actor_ids().is_empty() {
            warnings.push(format!(
                "Live presentation leaves {} ambiguous native actor IDs unbound.",
                live.ambiguous_actor_ids().len()
            ));
        }
        (Some(objects), Some(live))
    };
    // Release prepared geometry and parsed JSON before the parallel texture decode.
    drop(scene);
    let textures = load_textures(store, &draws, &settings)?;
    warnings.push("Visual reconstruction is incomplete: static model poses, planar placement facing only, approximate lighting/FOV, incomplete shore water and no shadows. Transparency uses recovered geometry sorting with binary32 preview keys and recorded map source order; original live rendering precision/traversal remain unproven.".into());
    Ok(CpuScene {
        objects,
        live,
        name,
        map_id: id.into(),
        size,
        draws,
        textures,
        placements,
        rendered_instances,
        warnings,
        load_seconds: start.elapsed().as_secs_f64(),
    })
}

pub fn load_model(
    store: &AssetStore,
    source: &str,
    settings: LoadSettings,
) -> Result<(CpuScene, aa_assets::Bounds)> {
    load_model_named(store, source, settings).map(|(scene, bounds, _)| (scene, bounds))
}

/// As `load_model`, also returning each draw's source mesh name and node
/// hierarchy (draw order), so baked animation and turrets can be matched
/// after helper meshes are removed.
pub fn load_model_named(
    store: &AssetStore,
    source: &str,
    settings: LoadSettings,
) -> Result<(CpuScene, aa_assets::Bounds, Vec<(String, Vec<String>)>)> {
    load_model_named_inner(store,source,settings,None)
}
///Native battle model transport: the retained Resource's selected byte hash
///must match the existing paired conversion artifact before preparation.
pub(crate) fn load_selected_model(store:&AssetStore,source:&str,settings:LoadSettings,input_sha256:&str)->Result<(CpuScene,aa_assets::Bounds)>{
    load_model_named_inner(store,source,settings,Some(input_sha256)).map(|(scene,bounds,_)|(scene,bounds))
}
fn load_model_named_inner(store:&AssetStore,source:&str,settings:LoadSettings,selected_input:Option<&str>)->Result<(CpuScene,aa_assets::Bounds,Vec<(String,Vec<String>)>)>{
    let start = Instant::now();
    let bytes = std::fs::metadata(store.resolve_url(store.model_url(source)?)?)?.len();
    preflight(
        MemoryEstimate {
            temporary_cpu_bytes: count_bytes(bytes, 12)?,
            ..Default::default()
        },
        settings.memory_policy,
    )?;
    let asset=match selected_input{
        Some(digest)=>settings.model_fixes.load_selected_model(store,source,digest)?,
        None=>store.load_model(source)?,
    };
    let mut model=asset.prepare()?;
    settings.model_fixes.apply(store, &mut model)?;
    map_prepared_model(store,source,&model,settings,start)
}
///The native Model's source40 owner already retains its verified modern
///geometry. Map that exact allocation without another model-file read.
pub(crate) fn load_retained_model(store:&AssetStore,source:&str,model:&aa_assets::PreparedModel,settings:LoadSettings)->Result<(CpuScene,aa_assets::Bounds)>{
    let packet_bytes=checked_sum(model.meshes.iter().map(|mesh|checked_sum([
        count_bytes(mesh.vertices.len() as u64,std::mem::size_of::<Vertex>() as u64)?,
        count_bytes(mesh.indices.len() as u64,4)?,std::mem::size_of::<Instance>() as u64,
    ])).collect::<Result<Vec<_>>>()?)?;
    preflight(MemoryEstimate{temporary_cpu_bytes:packet_bytes,..Default::default()},settings.memory_policy)?;
    map_prepared_model(store,source,model,settings,Instant::now()).map(|(scene,bounds,_)|(scene,bounds))
}
fn map_prepared_model(store:&AssetStore,source:&str,model:&aa_assets::PreparedModel,settings:LoadSettings,start:Instant)->Result<(CpuScene,aa_assets::Bounds,Vec<(String,Vec<String>)>)>{
    let draws = model
        .meshes
        .iter()
        .enumerate()
        .map(|(mesh_index, mesh)| CpuDraw {
            vertices: mesh
                .vertices
                .iter()
                .map(|v| Vertex {
                    terrain_uv: [0.0; 2],
                    position: v.position,
                    normal: v.normal,
                    uv: model_uv(v.uv),
                    color: v.color,
                })
                .collect(),
            indices: mesh.indices.clone(),
            instances: vec![Instance::identity()],
            material: material(&mesh.material),
            source_order: Some(vec![[0, mesh_index]]),
            sort_center: mesh.material.native_sort_center,
        })
        .collect::<Vec<_>>();
    let textures = load_textures(store, &draws, &settings)?;
    let bounds = model.bounds;
    let names: Vec<(String, Vec<String>)> = model
        .meshes
        .iter()
        .map(|m| (m.name.clone(), m.hierarchy.clone()))
        .collect();
    Ok((
        CpuScene {
            live: None,
            objects: None,
            name: source.rsplit('/').next().unwrap_or(source).into(),
            map_id: source.into(),
            size: [bounds.size[0], bounds.size[2]],
            draws,
            textures,
            placements: 1,
            rendered_instances: 1,
            warnings: model
                .warnings
                .iter()
                .map(|w| format!("{}: {}", w.source, w.message))
                .chain(std::iter::once("Transparency uses recovered geometry sorting with binary32 preview keys; original live rendering precision remains unproven.".into()))
                .collect(),
            load_seconds: start.elapsed().as_secs_f64(),
        },
        bounds,
        names,
    ))
}

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

pub fn load_textures(
    store: &AssetStore,
    draws: &[CpuDraw],
    settings: &LoadSettings,
) -> Result<BTreeMap<String, CpuTexture>> {
    let names: Vec<_> = draws
        .iter()
        .flat_map(|d| {
            d.material
                .texture
                .iter()
                .cloned()
                .chain(d.material.terrain_mask.iter().map(|m| m.texture.clone()))
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let plan: Vec<_> = names
        .into_iter()
        .map(|name| -> Result<_> {
            let path = store.resolve_url(&name)?;
            let dimensions = image::ImageReader::open(&path)?.into_dimensions()?;
            if dimensions.0.max(dimensions.1) > settings.max_texture_dimension {
                bail!(
                    "Texture {name} is {}x{}, beyond this GPU's {}-pixel capability",
                    dimensions.0,
                    dimensions.1,
                    settings.max_texture_dimension
                );
            }
            let bytes = rgba8_mip_chain_bytes(dimensions.0, dimensions.1)?;
            Ok((name, path, dimensions, bytes))
        })
        .collect::<Result<_>>()?;
    let texture_bytes = checked_sum(plan.iter().map(|p| p.3))?;
    let mesh_bytes = checked_sum(
        draws
            .iter()
            .map(|d| {
                checked_sum([
                    count_bytes(
                        d.vertices.len() as u64,
                        std::mem::size_of::<Vertex>() as u64,
                    )?,
                    count_bytes(d.indices.len() as u64, 4)?,
                    count_bytes(
                        d.instances.len() as u64,
                        std::mem::size_of::<Instance>() as u64,
                    )?,
                ])
            })
            .collect::<Result<Vec<_>>>()?,
    )?;
    let report = preflight(
        MemoryEstimate {
            resident_cpu_bytes: texture_bytes,
            temporary_cpu_bytes: count_bytes(texture_bytes, 2)?,
            upload_staging_bytes: checked_sum([texture_bytes, mesh_bytes])?,
            gpu_bytes: checked_sum([texture_bytes, mesh_bytes])?,
        },
        settings.memory_policy,
    )?;
    let decoded: Vec<_> = plan
        .par_iter()
        .map(|(name, path, dimensions, bytes)| {
            // A per-image bound based on its dimensions and the observed budget,
            // replacing image's implicit fixed 512 MiB allocation allowance.
            let decode_budget = bytes
                .saturating_mul(4)
                .min(report.effective_cpu_budget_bytes);
            let result = decode_texture(
                path,
                *dimensions,
                decode_budget,
                settings.max_texture_dimension,
            )
            .with_context(|| format!("Texture {name}"));
            (name.clone(), result)
        })
        .collect();
    let mut textures = BTreeMap::new();
    for (name, result) in decoded {
        // A budget, decode or I/O failure aborts the candidate scene. Do not
        // silently erase textures to make a low-memory load look successful.
        textures.insert(name, result?);
    }
    Ok(textures)
}

fn decode_texture(
    path: &Path,
    dimensions: (u32, u32),
    allocation_budget: u64,
    max_dimension: u32,
) -> Result<CpuTexture> {
    // Only recovered PNGs are accepted by the compiled decoder. Original files
    // stay read-only; generated mipmaps exist only for this renderer session.
    let mut reader = image::ImageReader::open(path)?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(max_dimension);
    limits.max_image_height = Some(max_dimension);
    limits.max_alloc = Some(allocation_budget);
    reader.limits(limits);
    let rgba = reader.decode()?.into_rgba8();
    if rgba.dimensions() != dimensions {
        bail!("Texture dimensions changed during loading");
    }
    let mut mips = vec![rgba];
    while mips.last().unwrap().width() > 1 || mips.last().unwrap().height() > 1 {
        let last = mips.last().unwrap();
        let next = image::imageops::resize(
            last,
            (last.width() / 2).max(1),
            (last.height() / 2).max(1),
            image::imageops::FilterType::Triangle,
        );
        mips.push(next);
    }
    Ok(CpuTexture { mips })
}
