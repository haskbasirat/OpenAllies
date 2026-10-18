// Rig baking for animated instanced models; bound again once the native
// world drives the renderer.
#[allow(dead_code)]
mod anim_bake;
mod animation_preview;
mod audio;
mod battle_audio;
mod authored_rules;
mod camera;
mod economy_lab;
mod frontend;
mod gpu;
mod input;
mod inspection;
mod live_preview;
mod live_scene;
mod loading;
mod menu_ui;
mod menu_video;
mod map_world_check;
mod mechanics_check;
mod runtime_fp;
mod windows_film_time;
mod model_fixes;
mod native_map_check;
mod terrain_render;
mod tgi_frontend;
mod ui_script;
mod vfx;
mod ui_tex;
mod visibility;
mod world_check;
#[cfg(windows)]
mod windows_reset_os;

use aa_assets::AssetStore;
use anyhow::{Context, Result, bail};
use camera::Camera;
use eframe::egui;
use gpu::{CpuScene, SceneRenderer};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    time::Instant,
};

const TITLE: &str = "Axis & Allies — Rust Development 10";

#[derive(Default)]
struct Options {
    assets: Option<PathBuf>,
    map: Option<String>,
    model: Option<String>,
    animated: Option<String>,
    live_preview: Option<String>,
    clip: Option<String>,
    clip_index: Option<usize>,
    time: f32,
    render_check: Option<PathBuf>,
    render_frames: Option<usize>,
    verify: bool,
    economy_check: bool,
    mechanics_check: bool,
    world_check: bool,
    map_world_check: Option<PathBuf>,
    map_restore_check: Option<String>,
    map_restore_report: Option<PathBuf>,
    actor_check: Option<String>,
    workers: Option<usize>,
    help: bool,
    memory_headroom_percent: Option<u32>,
    backend: Option<wgpu::Backends>,
    viewer: bool,
    auto: frontend::AutoStart,
}
impl Options {
    fn parse() -> Result<Self> {
        let mut result = Self::default();
        let mut args = std::env::args_os().skip(1);
        while let Some(arg) = args.next() {
            match arg.to_str().context("Arguments must be Unicode")? {
                "--assets" => {
                    result.assets = Some(args.next().context("--assets needs a path")?.into())
                }
                "--map" => {
                    result.map = Some(
                        args.next()
                            .context("--map needs the full map ID")?
                            .into_string()
                            .map_err(|_| anyhow::anyhow!("Invalid map ID"))?,
                    )
                }
                "--model" => {
                    result.model = Some(
                        args.next()
                            .context("--model needs an original resource path")?
                            .into_string()
                            .map_err(|_| anyhow::anyhow!("Invalid model path"))?,
                    )
                }
                "--animated" => {
                    result.animated = Some(
                        args.next()
                            .context("--animated needs a recovered rig name")?
                            .into_string()
                            .map_err(|_| anyhow::anyhow!("Invalid rig name"))?,
                    )
                }
                "--clip" => {
                    result.clip = Some(
                        args.next()
                            .context("--clip needs a clip name")?
                            .into_string()
                            .map_err(|_| anyhow::anyhow!("Invalid clip name"))?,
                    )
                }
                "--live-preview" => {
                    result.live_preview = Some(
                        args.next()
                            .context(
                                "--live-preview needs an infantry Thing ID (e.g. usa_infantry)",
                            )?
                            .into_string()
                            .map_err(|_| anyhow::anyhow!("Invalid preview Thing ID"))?,
                    );
                }
                "--time" => {
                    result.time = args
                        .next()
                        .context("--time needs seconds")?
                        .to_str()
                        .context("Invalid sample time")?
                        .parse()?;
                    if !result.time.is_finite() || result.time < 0.0 {
                        bail!("Sample time must be finite and nonnegative");
                    }
                }
                "--render-check" => {
                    result.render_check = Some(
                        args.next()
                            .context("--render-check needs a new PNG path")?
                            .into(),
                    )
                }
                "--render-frames" => {
                    let count: usize = args
                        .next()
                        .context("--render-frames needs a positive count")?
                        .to_str()
                        .context("Invalid frame count")?
                        .parse()?;
                    if count == 0 {
                        bail!("Render frame count must be positive");
                    }
                    result.render_frames = Some(count);
                }
                "--clip-index" => {
                    result.clip_index = Some(
                        args.next()
                            .context("--clip-index needs a zero-based clip index")?
                            .to_str()
                            .context("Invalid clip index")?
                            .parse()?,
                    )
                }
                "--verify-assets" => result.verify = true,
                "--viewer" => result.viewer = true,
                "--shot" => {
                    result.auto.shot = Some(args.next().context("--shot needs a PNG path")?.into())
                }
                "--ui-script" => {
                    result.auto.script = Some(args.next().context("--ui-script needs a JSON file")?.into())
                }
                "--economy-check" => result.economy_check = true,
                "--mechanics-check" => result.mechanics_check = true,
                "--world-check" => result.world_check = true,
                "--map-world-check" => {
                    result.map_world_check = Some(
                        args.next()
                            .context("--map-world-check needs an original .RMP path")?
                            .into(),
                    );
                }
                "--map-restore-check" => {
                    result.map_restore_check = Some(
                        args.next().context("--map-restore-check needs a map ID or .RMP path")?
                            .into_string().map_err(|_| anyhow::anyhow!("Invalid map ID or path"))?,
                    );
                }
                "--map-restore-report" => {
                    result.map_restore_report = Some(
                        args.next().context("--map-restore-report needs a new JSON path")?.into(),
                    );
                }
                "--actor-check" => {
                    result.actor_check = Some(
                        args.next()
                            .context("--actor-check needs an original Thing IDS")?
                            .into_string()
                            .map_err(|_| anyhow::anyhow!("Invalid Thing IDS"))?,
                    )
                }
                "--memory-headroom-percent" => {
                    let value: u32 = args
                        .next()
                        .context("--memory-headroom-percent needs 0..100")?
                        .to_str()
                        .context("Invalid memory reserve")?
                        .parse()?;
                    if value > 100 {
                        bail!("Memory headroom must be between 0 and 100 percent");
                    }
                    result.memory_headroom_percent = Some(value);
                }
                "--backend" => {
                    result.backend = Some(
                        match args
                            .next()
                            .context("--backend needs auto, dx12 or vulkan")?
                            .to_str()
                            .context("Invalid backend")?
                        {
                            "auto" => wgpu::Backends::DX12 | wgpu::Backends::VULKAN,
                            "dx12" => wgpu::Backends::DX12,
                            "vulkan" => wgpu::Backends::VULKAN,
                            _ => bail!("Backend must be auto, dx12 or vulkan"),
                        },
                    );
                }
                "--workers" => {
                    let n: usize = args
                        .next()
                        .context("--workers needs a positive count")?
                        .to_str()
                        .context("Invalid worker count")?
                        .parse()?;
                    if n == 0 {
                        bail!("Worker count must be positive");
                    }
                    result.workers = Some(n);
                }
                "--help" | "-h" => result.help = true,
                _ => bail!("Unknown option {}", arg.to_string_lossy()),
            }
        }
        if result.map_restore_report.is_some() && result.map_restore_check.is_none() {
            bail!("--map-restore-report requires --map-restore-check");
        }
        if result.map_restore_check.is_some()
            && (result.mechanics_check || result.world_check || result.map_world_check.is_some()
                || result.verify || result.economy_check || result.actor_check.is_some()
                || result.viewer || result.map.is_some() || result.model.is_some()
                || result.animated.is_some() || result.live_preview.is_some()
                || result.render_check.is_some() || result.auto.script.is_some() || result.auto.shot.is_some())
        {
            bail!("Use --map-restore-check separately from other diagnostics and viewer/render options");
        }
        if result.clip.is_some() && result.clip_index.is_some() {
            bail!("Use either --clip or --clip-index");
        }
        if result.render_frames.is_some() && result.render_check.is_none() {
            bail!("--render-frames requires --render-check");
        }
        if u8::from(result.mechanics_check)
            + u8::from(result.world_check)
            + u8::from(result.map_world_check.is_some())
            > 1
        {
            bail!("Use only one of --mechanics-check, --world-check or --map-world-check");
        }
        if result.live_preview.is_some()
            && (result.animated.is_some()
                || result.model.is_some()
                || result.map.is_some()
                || result.clip_index.is_some())
        {
            bail!("Use --live-preview separately from --animated, --model, --map and --clip-index");
        }
        if (result.clip.is_some() || result.clip_index.is_some())
            && result.animated.is_none()
            && result.live_preview.is_none()
        {
            bail!("Clip selection requires --animated");
        }
        Ok(result)
    }
}

fn discover_assets(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return path
            .canonicalize()
            .context("Asset directory does not exist");
    }
    let mut bases = vec![std::env::current_dir()?];
    if let Some(parent) = std::env::current_exe()?.parent() {
        bases.push(parent.to_owned());
    }
    for base in bases {
        for parent in base.ancestors() {
            for candidate in [parent.to_owned(), parent.join("reconstruction")] {
                if candidate.join("game/data/maps/index.json").is_file() {
                    return Ok(candidate);
                }
            }
        }
    }
    bail!(
        "Recovered original assets were not found. Use --assets with the reconstruction directory. This development executable requires those assets."
    )
}

fn device_descriptor(adapter: &wgpu::Adapter) -> wgpu::DeviceDescriptor<'static> {
    wgpu::DeviceDescriptor {
        label: Some("64-bit reconstruction device"),
        // Ask for actual selected-adapter capabilities. No legacy 2/4 GiB allowance.
        required_limits: adapter.limits(),
        ..Default::default()
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let options = Options::parse()?;
    if options.help {
        println!(
            "{TITLE}\nStarts the game. Development viewer options below open the asset viewer.\n--viewer (open the map/model viewer)\n\
            --assets <reconstruction directory>\n--map <full ID, default Campaign/tutorial01>\n\
            --workers <positive count>\n--backend <auto|dx12|vulkan>\n--memory-headroom-percent <0..100, default 10>\n\
            --model <original model resource>\n--animated <recovered rig name> --clip <unique name> --time <seconds>\n--clip-index <zero-based index, instead of --clip>\n--live-preview <Thing ID, e.g. usa_infantry> [--clip jog] [--time seconds]\n\
            --verify-assets\n--economy-check\n--mechanics-check (asset-free native-reference diagnostic)\n--world-check (asset-free native world-fragment diagnostic)\n--map-world-check <original RMP path>\n--map-restore-check <map ID or RMP path> [--map-restore-report <new JSON path>]\n  Actual native record loading diagnostic; exit 0 means the check ran, not a playable battle.\n--actor-check <Thing IDS>\n--render-check <new PNG path> [--render-frames <positive count>]\n\
            Default: open the native renderer. Original game files are read only."
        );
        return Ok(());
    }
    if options.mechanics_check {
        let report = mechanics_check::run()?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        if report["passed"] != true {
            bail!("Mechanics diagnostic did not match the selected native reference cases");
        }
        return Ok(());
    }
    if options.world_check {
        let report = world_check::run()?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        if report["passed"] != true {
            bail!("World-fragment diagnostic did not match the selected native reference cases");
        }
        return Ok(());
    }
    if let Some(path) = &options.map_world_check {
        let policy = aa_resources::MemoryPolicy {
            physical_headroom_bps: options.memory_headroom_percent.unwrap_or(10) * 100,
            commit_headroom_bps: options.memory_headroom_percent.unwrap_or(10) * 100,
            ..Default::default()
        };
        let report = map_world_check::run(path, policy)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        if report["passed"] != true {
            bail!("Native map inputs include unresolved or invalid records; see report");
        }
        return Ok(());
    }
    if let Some(n) = options.workers {
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .build_global()?;
    }
    let root = discover_assets(options.assets.as_deref())?;
    if let Some(map) = &options.map_restore_check {
        let policy = aa_resources::MemoryPolicy {
            physical_headroom_bps: options.memory_headroom_percent.unwrap_or(10) * 100,
            commit_headroom_bps: options.memory_headroom_percent.unwrap_or(10) * 100,
            ..Default::default()
        };
        let report = native_map_check::run(&root, map, policy)?;
        if let Some(path) = &options.map_restore_report {
            native_map_check::write_new(path, &report)?;
        }
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    let store = Arc::new(AssetStore::open(&root)?);
    if options.verify {
        let start = Instant::now();
        let mut placements = 0usize;
        for map in store.maps() {
            placements += store.load_map(&map.id)?.placements.actors.len();
        }
        println!(
            "{}",
            serde_json::json!({"asset_root":root,"maps":store.maps().len(),
            "placements":placements,"model_catalog":store.model_infos().len(),
            "seconds":start.elapsed().as_secs_f64(),"status":"map structures verified; not gameplay parity"})
        );
        return Ok(());
    }
    let auto = options.auto.clone();
    let viewer_mode = options.viewer
        || options.live_preview.is_some()
        || options.animated.is_some()
        || options.model.is_some()
        || options.map.is_some()
        || options.render_check.is_some();
    let request = if let Some(id) = options.live_preview {
        PreviewRequest::LiveActor {
            id,
            epoch: 1,
            clip: options.clip,
            time: options.time,
        }
    } else if let Some(name) = options.animated {
        PreviewRequest::Animated {
            path: animation_preview::resolve(&store, &name)?,
            clip: options.clip,
            clip_index: options.clip_index,
            time: options.time,
        }
    } else {
        match options.model {
            Some(source) => PreviewRequest::Model(source),
            None => {
                PreviewRequest::Map(options.map.unwrap_or_else(|| "Campaign/tutorial01".into()))
            }
        }
    };
    let memory_policy = aa_resources::MemoryPolicy {
        physical_headroom_bps: options.memory_headroom_percent.unwrap_or(10) * 100,
        commit_headroom_bps: options.memory_headroom_percent.unwrap_or(10) * 100,
        ..Default::default()
    };
    if options.economy_check {
        println!(
            "{}",
            serde_json::to_string_pretty(&economy_lab::run(&root, memory_policy)?)?
        );
        return Ok(());
    }
    if let Some(id) = options.actor_check {
        let catalog = authored_rules::load(&root, memory_policy)?;
        let actor = catalog
            .find(&id)
            .with_context(|| format!("No authored Thing definition for {id}"))?;
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "scope":"Authored original/v14 fields only; no live state, expression evaluation or gameplay execution",
                "definitions":catalog.actors.len(), "source_files":catalog.sources.len(),
                "source_notices":catalog.diagnostics.len(), "actor":actor
            }))?
        );
        return Ok(());
    }
    let backends = options
        .backend
        .unwrap_or(wgpu::Backends::DX12 | wgpu::Backends::VULKAN);
    if let Some(path) = options.render_check {
        return render_check(
            &store,
            &request,
            &path,
            backends,
            memory_policy,
            options.render_frames.unwrap_or(1),
        );
    }
    let setup = egui_wgpu::WgpuSetupCreateNew {
        instance_descriptor: wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        },
        device_descriptor: Arc::new(device_descriptor),
        ..Default::default()
    };
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([640.0, 480.0]),
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: egui_wgpu::WgpuConfiguration {
            wgpu_setup: setup.into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let native = eframe::NativeOptions {
        viewport: native
            .viewport
            .with_title(TITLE)
            .with_icon(std::sync::Arc::new(egui::IconData::default())),
        ..native
    };
    eframe::run_native(
        TITLE,
        native,
        Box::new(move |cc| {
            let state = cc
                .wgpu_render_state
                .clone()
                .context("The wgpu renderer was not initialized")?;
            if viewer_mode {
                Ok(Box::new(App::new(state, cc.egui_ctx.clone(), store, request, memory_policy)?)
                    as Box<dyn eframe::App>)
            } else {
                let mut shell = frontend::Shell::new(state, &cc.egui_ctx, store, memory_policy)?;
                shell.auto = auto;
                Ok(Box::new(shell) as Box<dyn eframe::App>)
            }
        }),
    )
    .map_err(|e| anyhow::anyhow!("Native renderer: {e}"))
}

fn render_check(
    store: &AssetStore,
    request: &PreviewRequest,
    output: &Path,
    backends: wgpu::Backends,
    memory_policy: aa_resources::MemoryPolicy,
    frames: usize,
) -> Result<()> {
    if output.exists() {
        bail!("Render check output already exists; choose a new path");
    }
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends,
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&device_descriptor(&adapter)))?;
    let egui_renderer =
        egui_wgpu::Renderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, Default::default());
    let state = egui_wgpu::RenderState {
        adapter,
        available_adapters: Vec::new(),
        device,
        queue,
        target_format: wgpu::TextureFormat::Rgba8Unorm,
        renderer: Arc::new(egui::mutex::RwLock::new(egui_renderer)),
    };
    let load_settings = loading::LoadSettings::new(
        store,
        state.device.limits().max_texture_dimension_2d,
        memory_policy,
    )?;
    let mut renderer = SceneRenderer::new(state)?;
    let mut preview = request.load(store, load_settings)?;
    let mut scene = preview.scene;
    renderer.upload(&scene)?;
    if let Some(live) = &mut scene.live {
        live.attach(renderer.scene_epoch())?;
    }
    let _live_actor = if let Some(prepared) = preview.live_actor.take() {
        Some(
            prepared.activate(
                &mut renderer,
                scene
                    .live
                    .as_mut()
                    .context("Preview live binding missing")?,
                scene.objects.as_mut().context("Preview picking missing")?,
            )?,
        )
    } else {
        None
    };
    let first_frame = Instant::now();
    renderer.render(&preview.camera, [1600, 900])?;
    let first_frame_ms = first_frame.elapsed().as_secs_f64() * 1000.0;
    let repeat_start = Instant::now();
    for _ in 1..frames {
        renderer.render(&preview.camera, [1600, 900])?;
    }
    let repeated_frame_ms = if frames > 1 {
        Some(repeat_start.elapsed().as_secs_f64() * 1000.0 / (frames - 1) as f64)
    } else {
        None
    };
    renderer.save_view(output)?;
    println!(
        "{}",
        serde_json::json!({"asset":scene.map_id,"image":output,"adapter":renderer.adapter_name,
        "draw_calls":renderer.draw_calls,"triangles":renderer.triangles,"uploaded_bytes":renderer.uploaded_bytes,
        "placements":scene.placements,"rendered_instances":scene.rendered_instances,
        "cpu_load_seconds":scene.load_seconds,"workers":rayon::current_num_threads(),"warnings":scene.warnings,
        "render_frames":frames,"first_encode_submit_ms":first_frame_ms,"repeated_encode_submit_mean_ms":repeated_frame_ms,
        "draw_order_rebuilds":renderer.ordering_rebuilds,
        "timing_scope":"CPU encode/submit plus validation scope; not isolated GPU time or in-game FPS. Final image readback completes before success.",
        "status":"original-asset render only; no gameplay parity claim"})
    );
    Ok(())
}

#[derive(Clone)]
pub(crate) enum PreviewRequest {
    LiveActor {
        id: String,
        epoch: u64,
        clip: Option<String>,
        time: f32,
    },
    Map(String),
    Model(String),
    Animated {
        path: PathBuf,
        clip: Option<String>,
        clip_index: Option<usize>,
        time: f32,
    },
}
struct LoadedPreview {
    scene: CpuScene,
    camera: Camera,
    animation: Option<animation_preview::AnimationPreview>,
    live_actor: Option<live_preview::Prepared>,
}
impl PreviewRequest {
    fn load(&self, store: &AssetStore, settings: loading::LoadSettings) -> Result<LoadedPreview> {
        match self {
            Self::LiveActor {
                id,
                epoch,
                clip,
                time,
            } => {
                let (live_actor, scene, camera) =
                    live_preview::prepare(store, settings, id, *epoch, clip.as_deref(), *time)?;
                Ok(LoadedPreview {
                    scene,
                    camera,
                    animation: None,
                    live_actor: Some(live_actor),
                })
            }
            Self::Map(id) => {
                let scene = loading::load(store, id, settings)?;
                let camera = Camera::new(scene.size);
                Ok(LoadedPreview {
                    scene,
                    camera,
                    animation: None,
                    live_actor: None,
                })
            }
            Self::Model(source) => {
                let (scene, bounds) = loading::load_model(store, source, settings)?;
                Ok(LoadedPreview {
                    scene,
                    camera: Camera::for_model(&bounds),
                    animation: None,
                    live_actor: None,
                })
            }
            Self::Animated {
                path,
                clip,
                clip_index,
                time,
            } => {
                let (mut animation, mut scene, camera) =
                    animation_preview::AnimationPreview::load(store, path, settings)?;
                if let Some(name) = clip {
                    animation.select_clip(name)?;
                }
                if let Some(index) = clip_index {
                    animation.select_clip_index(*index)?;
                }
                animation.time = *time;
                animation.pose()?;
                for (draw, vertices) in scene.draws.iter_mut().zip(&animation.vertices) {
                    draw.vertices.clone_from(vertices);
                }
                Ok(LoadedPreview {
                    scene,
                    camera,
                    animation: Some(animation),
                    live_actor: None,
                })
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum BrowserKind {
    Maps,
    Models,
    Animations,
    LivePreview,
}

struct LoadedInfo {
    name: String,
    id: String,
    placements: usize,
    rendered: usize,
    seconds: f64,
    warnings: Vec<String>,
}
pub(crate) struct App {
    store: Arc<AssetStore>,
    renderer: SceneRenderer,
    camera: Camera,
    pending: Option<mpsc::Receiver<Result<LoadedPreview>>>,
    loaded: Option<LoadedInfo>,
    error: Option<String>,
    selected: String,
    selected_model: String,
    browser: BrowserKind,
    animated_choices: Vec<(String, PathBuf)>,
    selected_animation: Option<PathBuf>,
    animation: Option<animation_preview::AnimationPreview>,
    filter: String,
    objects: Option<inspection::SceneObjects>,
    live_scene: Option<live_scene::LiveScene>,
    live_actor: Option<live_preview::Active>,
    selected_live_actor: String,
    selection: Option<inspection::Selection>,
    authored_rules: authored_rules::AuthoredRules,
    native_map_check: native_map_check::Panel,
    fullscreen: bool,
    last_frame: Instant,
    frame_ms: f32,
    ui_scale: f32,
    load_settings: loading::LoadSettings,
}
impl App {
    pub(crate) fn new(
        state: egui_wgpu::RenderState,
        egui_ctx: egui::Context,
        store: Arc<AssetStore>,
        request: PreviewRequest,
        memory_policy: aa_resources::MemoryPolicy,
    ) -> Result<Self> {
        let load_settings = loading::LoadSettings::new(
            &store,
            state.device.limits().max_texture_dimension_2d,
            memory_policy,
        )?;
        let animated_choices = animation_preview::available(&store);
        let selected_animation = match &request {
            PreviewRequest::Animated { path, .. } => Some(path.clone()),
            _ => animated_choices
                .iter()
                .find(|(name, _)| name == "USAInfantry")
                .or(animated_choices.first())
                .map(|(_, path)| path.clone()),
        };
        let (selected, selected_model, browser) = match &request {
            PreviewRequest::LiveActor { .. } => (
                "Campaign/tutorial01".into(),
                String::new(),
                BrowserKind::LivePreview,
            ),
            PreviewRequest::Map(id) => (
                id.clone(),
                store
                    .model_infos()
                    .first()
                    .map(|m| m.source.clone())
                    .unwrap_or_default(),
                BrowserKind::Maps,
            ),
            PreviewRequest::Model(source) => (
                "Campaign/tutorial01".into(),
                source.clone(),
                BrowserKind::Models,
            ),
            PreviewRequest::Animated { .. } => (
                "Campaign/tutorial01".into(),
                String::new(),
                BrowserKind::Animations,
            ),
        };
        let mut app = Self {
            selected_live_actor: match &request {
                PreviewRequest::LiveActor { id, .. } => id.clone(),
                _ => "usa_infantry".into(),
            },
            store,
            renderer: SceneRenderer::new(state)?,
            camera: Camera::new([1024.0; 2]),
            pending: None,
            loaded: None,
            error: None,
            selected,
            selected_model,
            browser,
            animated_choices,
            selected_animation,
            animation: None,
            filter: String::new(),
            objects: None,
            live_scene: None,
            live_actor: None,
            selection: None,
            authored_rules: authored_rules::AuthoredRules::default(),
            native_map_check: native_map_check::Panel::default(),
            fullscreen: false,
            last_frame: Instant::now(),
            frame_ms: 0.0,
            ui_scale: 1.0,
            load_settings,
        };
        app.start_request(egui_ctx, request);
        Ok(app)
    }
    fn start_load(&mut self, ctx: egui::Context) {
        let request = match self.browser {
            BrowserKind::LivePreview => PreviewRequest::LiveActor {
                id: self.selected_live_actor.clone(),
                epoch: self.renderer.scene_epoch() + 1,
                clip: None,
                time: 0.,
            },
            BrowserKind::Maps => PreviewRequest::Map(self.selected.clone()),
            BrowserKind::Models => PreviewRequest::Model(self.selected_model.clone()),
            BrowserKind::Animations => {
                let Some(path) = self.selected_animation.clone() else {
                    return;
                };
                PreviewRequest::Animated {
                    path,
                    clip: None,
                    clip_index: None,
                    time: 0.0,
                }
            }
        };
        self.start_request(ctx, request);
    }
    fn start_request(&mut self, ctx: egui::Context, request: PreviewRequest) {
        if self.pending.is_some() || self.native_map_check.busy() {
            return;
        }
        let (send, receive) = mpsc::sync_channel(1);
        let store = self.store.clone();
        let settings = self.load_settings.clone();
        self.error = None;
        self.pending = Some(receive);
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                request.load(&store, settings)
            }))
            .unwrap_or_else(|_| {
                Err(anyhow::anyhow!(
                    "Asset loading failed unexpectedly; see development logs"
                ))
            });
            let _ = send.send(result);
            ctx.request_repaint();
        });
    }
    fn poll_load(&mut self) {
        let result = self.pending.as_ref().and_then(|r| match r.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err(anyhow::anyhow!("Asset loader stopped")))
            }
            Err(mpsc::TryRecvError::Empty) => None,
        });
        if let Some(result) = result {
            self.pending = None;
            match result.and_then(|mut preview| {
                self.renderer.upload(&preview.scene)?;
                //Upload replaces the epoch. Even if later preview activation
                //fails, old animation/picking bindings must not survive it.
                self.live_actor = None;
                self.live_scene = None;
                self.objects = None;
                self.animation = None;
                self.selection = None;
                self.loaded = None;
                if let Some(live) = &mut preview.scene.live {
                    live.attach(self.renderer.scene_epoch())?;
                }
                let live_actor = if let Some(prepared) = preview.live_actor.take() {
                    Some(
                        prepared.activate(
                            &mut self.renderer,
                            preview
                                .scene
                                .live
                                .as_mut()
                                .context("Preview live binding missing")?,
                            preview
                                .scene
                                .objects
                                .as_mut()
                                .context("Preview picking missing")?,
                        )?,
                    )
                } else {
                    None
                };
                self.live_actor = live_actor;
                Ok(preview)
            }) {
                Ok(preview) => {
                    let scene = preview.scene;
                    self.camera = preview.camera;
                    self.animation = preview.animation;
                    self.objects = scene.objects;
                    self.live_scene = scene.live;
                    self.selection = None;
                    self.loaded = Some(LoadedInfo {
                        name: scene.name,
                        id: scene.map_id,
                        placements: scene.placements,
                        rendered: scene.rendered_instances,
                        seconds: scene.load_seconds,
                        warnings: scene.warnings,
                    });
                }
                Err(e) => self.error = Some(format!("{e:#}")),
            }
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.poll_load();
        self.authored_rules.poll();
        self.native_map_check.poll();
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.frame_ms += (elapsed * 1000.0 - self.frame_ms) * 0.08;
        if let Some(animation) = &mut self.animation {
            animation.advance(elapsed);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
            self.fullscreen = !self.fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
        }
        egui::TopBottomPanel::top("status").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("AXIS & ALLIES / RUST RECONSTRUCTION");
                ui.label("Development 10 · original asset inspection");
                if ui.checkbox(&mut self.fullscreen,"Fullscreen (F11)").changed() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
                }
            });
            ui.colored_label(egui::Color32::from_rgb(236,193,109),
                "The full game is unfinished: combat, AI, campaign progression, original saves and multiplayer are not implemented here.");
        });
        let mut inspect_model = None;
        egui::SidePanel::left("maps").default_width(280.0).min_width(180.0).resizable(true).show(ctx, |ui| {
            let map_list_height=(ui.available_height()*0.45).max(100.0);
            egui::ScrollArea::vertical().id_salt("all-controls").show(ui,|ui|{
            ui.horizontal_wrapped(|ui|{
                if ui.selectable_value(&mut self.browser,BrowserKind::Maps,"Maps").changed(){self.filter.clear();}
                if ui.selectable_value(&mut self.browser,BrowserKind::Models,"Models").changed(){self.filter.clear();}
                if ui.selectable_value(&mut self.browser,BrowserKind::Animations,"Animations").changed(){self.filter.clear();}
                if ui.selectable_value(&mut self.browser,BrowserKind::LivePreview,"Live preview").changed(){self.filter.clear();}
            });
            ui.heading(match self.browser{BrowserKind::Maps=>"Original maps",BrowserKind::Models=>"Original models",BrowserKind::Animations=>"Original animation rigs",BrowserKind::LivePreview=>"Live actor preview"});
            if self.browser==BrowserKind::LivePreview {
                ui.small("Spawn an original infantry model into an isolated visual test scene.");
                for (id,label) in live_preview::ACTORS {ui.selectable_value(&mut self.selected_live_actor,id.into(),label);}
            } else {
            ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Filter original assets"));
            let needle = self.filter.to_lowercase();
            egui::ScrollArea::vertical().id_salt("map-list").max_height(map_list_height).show(ui, |ui| {
                if self.browser==BrowserKind::Maps {for map in self.store.maps() {
                    if needle.is_empty() || format!("{} {}",map.id,map.name).to_lowercase().contains(&needle) {
                        ui.selectable_value(&mut self.selected,map.id.clone(),&map.id).on_hover_text(&map.name);
                    }
                }
                }else if self.browser==BrowserKind::Models {for model in self.store.model_infos(){
                    if needle.is_empty()||model.source.to_lowercase().contains(&needle){
                        ui.selectable_value(&mut self.selected_model,model.source.clone(),&model.name).on_hover_text(&model.source);
                    }
                }}else{for (name,path) in &self.animated_choices{
                    if needle.is_empty()||name.to_lowercase().contains(&needle){ui.selectable_value(&mut self.selected_animation,Some(path.clone()),name);}
                }
                    if self.animated_choices.is_empty(){ui.label("Recovered rig cache is not available.");}
                }
            });
            }
            let load_label=if self.browser==BrowserKind::LivePreview {"Spawn preview infantry"}else{"Load selected asset"};
            if ui.add_enabled(self.pending.is_none() && !self.native_map_check.busy(),egui::Button::new(load_label)).clicked() { self.start_load(ctx.clone()); }
            if self.pending.is_some() { ui.horizontal(|ui|{ui.spinner();ui.label("Loading original assets…");}); }
            if let Some(error) = &self.error { ui.colored_label(egui::Color32::LIGHT_RED,error); }
            if self.browser==BrowserKind::Maps {
                self.native_map_check.show(ui,self.store.root(),&self.selected,self.load_settings.memory_policy,self.pending.is_some());
            }
            if let Some(actor)=&mut self.live_actor
                && let Err(error)=actor.ui(ui) {actor.stop_on_error();self.error=Some(format!("Live preview stopped: {error:#}"));}
            if let Some(animation)=&mut self.animation {
                ui.separator();
                ui.label("Original clip playback");
                egui::ComboBox::from_id_salt("animation-clip").selected_text(animation_preview::clip_label(&animation.asset.clips,animation.clip)).show_ui(ui,|ui|{
                    for i in 0..animation.asset.clips.len(){
                        if ui.selectable_value(&mut animation.clip,i,animation_preview::clip_label(&animation.asset.clips,i)).changed(){animation.time=0.0;}
                    }
                });
                ui.horizontal(|ui|{ui.checkbox(&mut animation.playing,"Play");ui.checkbox(&mut animation.looping,"Loop");});
                let duration=animation.asset.clips[animation.clip].duration;
                if ui.add(egui::Slider::new(&mut animation.time,0.0..=duration).suffix(" s")).dragged(){animation.playing=false;}
                ui.small("Isolated visual playback; no unit orders or battlefield movement.");
            }
            ui.separator();
            ui.label("WASD / arrows: pan\nMouse wheel: zoom\nHome / End: zoom endpoints\nF11: fullscreen");
            if self.camera.inspection {
                ui.label("Q / E: rotate model view");
            }
            if self.live_actor.is_none() {ui.horizontal(|ui|{
                ui.label("Team color");
                ui.color_edit_button_rgb(&mut self.renderer.team_color);
                if ui.button("Default").clicked(){self.renderer.team_color=[1.0;3];}
            });}
            ui.label(format!("Zoom {:.2}  ·  {} workers",self.camera.zoom,rayon::current_num_threads()));
            egui::ComboBox::from_label("UI size").selected_text(format!("{:.0}%",self.ui_scale*100.0)).show_ui(ui,|ui|{
                for (n,label) in [(1.0,"100%"),(1.25,"125%"),(1.5,"150%"),(2.0,"200%")] {
                    if ui.selectable_value(&mut self.ui_scale,n,label).changed() {ctx.set_zoom_factor(n);}
                }
            });
            ui.small("Presentation is synced to the display. UI size multiplies Windows DPI scale.");
            ui.separator();
            ui.label(&self.renderer.adapter_name);
            ui.label(format!("{:.1} ms/frame · {:.1} MiB scene data",self.frame_ms,self.renderer.uploaded_bytes as f64/1048576.0));
            ui.label(format!("{} draw calls · {} triangles",self.renderer.draw_calls,self.renderer.triangles));
            if let Some(loaded) = &self.loaded {
                ui.label(format!("{}\n{}\n{} / {} placements · {:.2}s CPU load",loaded.name,loaded.id,loaded.rendered,loaded.placements,loaded.seconds));
                ui.collapsing(format!("Reconstruction notes ({})",loaded.warnings.len()),|ui|{
                    egui::ScrollArea::vertical().max_height(180.0).show(ui,|ui|{for w in &loaded.warnings {ui.small(w);}});
                });
            }
            if let Some(objects) = &self.objects {
                ui.separator();
                ui.label("Click a map object to inspect it");
                ui.small("Geometry inspection only; this does not issue game orders. Transparent texels are not part of the hit test.");
                if let Some(selection) = self.selection {
                    let object = &objects.objects[selection.object];
                    ui.strong(&object.name);
                    ui.label(format!("{} · instance {}", object.thing_id, object.instance_id));
                    if let Some(category) = &object.category { ui.label(category); }
                    let position_label=if self.live_actor.is_some(){"Preview position"}else{"Original position"};
                    ui.label(format!("{position_label}: {:.2}, {:.2}, {:.2}", object.native_position[0], object.native_position[1], object.native_position[2]));
                    ui.small(&object.model_source);
                    self.authored_rules.show(ui, &object.thing_id, self.store.root(), self.load_settings.memory_policy);
                    if object.record_length!=0 { ui.collapsing("Original record", |ui| {
                        if let Some(source) = &object.rule_source { ui.label(source); }
                        ui.label(format!("Placement {} · record {} + {} bytes", object.source_index, object.record_offset, object.record_length));
                    }); } else {ui.small("Preview instance; no original saved placement record.");}
                    ui.horizontal(|ui| {
                        if ui.button("Focus").clicked() { self.camera.target = objects.focus(selection.object); }
                        if ui.add_enabled(self.pending.is_none() && !self.native_map_check.busy(), egui::Button::new("Inspect model")).clicked() { inspect_model = Some(object.model_source.clone()); }
                        if ui.button("Clear").clicked() { self.selection = None; }
                    });
                }
            }
            });
        });
        if let Some(source) = inspect_model {
            self.selected_model = source.clone();
            self.browser = BrowserKind::Models;
            self.filter.clear();
            self.start_request(ctx.clone(), PreviewRequest::Model(source));
        }
        if let Some(animation) = &mut self.animation
            && let Err(error) = animation
                .pose()
                .and_then(|()| self.renderer.update_vertices(&animation.vertices))
        {
            animation.playing = false;
            self.error = Some(format!("Animation playback stopped: {error:#}"));
        }
        if let (Some(actor), Some(live), Some(objects)) = (
            &mut self.live_actor,
            &mut self.live_scene,
            &mut self.objects,
        ) && let Err(error) = actor.tick(elapsed, &mut self.renderer, live, objects)
        {
            actor.stop_on_error();
            self.error = Some(format!("Live preview stopped: {error:#}"));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let size = ui.available_size().max(egui::vec2(1.0, 1.0));
                let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
                if response.clicked() || response.secondary_clicked() {
                    response.request_focus();
                }
                if !ctx.wants_keyboard_input() || response.has_focus() {
                    let (axes, home, end) = ctx.input(|i| {
                        (
                            input::camera_axes(i),
                            i.key_pressed(egui::Key::Home),
                            i.key_pressed(egui::Key::End),
                        )
                    });
                    // Held keyboard state is independent of mouse buttons.
                    self.camera.pan(axes[0], axes[1], elapsed.min(0.05));
                    if self.camera.inspection {
                        let turn = ctx.input(|i| {
                            f32::from(i.key_down(egui::Key::E))
                                - f32::from(i.key_down(egui::Key::Q))
                        });
                        self.camera.yaw += turn * elapsed.min(0.05);
                    }
                    if home {
                        self.camera.zoom = 0.7;
                    }
                    if end {
                        self.camera.zoom = 2.1;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.selection = None;
                    }
                }
                if response.hovered() {
                    self.camera.zoom_by(ctx.input(|i| i.smooth_scroll_delta.y));
                }
                let pixels = [
                    (size.x * ctx.pixels_per_point()).round().max(1.0) as u32,
                    (size.y * ctx.pixels_per_point()).round().max(1.0) as u32,
                ];
                if response.clicked()
                    && let Some(objects) = &self.objects
                    && let Some(pointer) = response.interact_pointer_pos()
                {
                    self.selection = objects.pick_at(&self.camera, rect, pointer, pixels);
                }
                match self.renderer.render(&self.camera, pixels) {
                    Ok(texture) => {
                        ui.painter().image(
                            texture,
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        if let Some(selection) = self.selection
                            && let Some(point) = inspection::SceneObjects::marker(
                                &self.camera,
                                rect,
                                pixels,
                                selection.position,
                            )
                        {
                            let painter = ui.painter().with_clip_rect(rect);
                            painter.circle_stroke(
                                point,
                                8.0,
                                egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(255, 220, 120)),
                            );
                            painter.circle_filled(point, 2.0, egui::Color32::WHITE);
                        }
                    }
                    Err(error) => self.error = Some(format!("Rendering failed: {error:#}")),
                }
            });
        ctx.request_repaint();
    }
}
