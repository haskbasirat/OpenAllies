//! Real-data terrain fog census on the Berlin skirmish (Maps/(2v2)Berlin.RMP,
//! or AA_PLAYABLE_MAP). The same activation chain as tests/live_recruit_order.rs
//! (real selected-map Host -> main launch -> queued resource task -> 775DB0
//! activation on the SAME simulation thread), then, on the activated World:
//!
//! * the FogOfWar SVar the original reads (`[[B39AB0]+108]`); with 1 the
//!   mission start ran 0x551F90 reveal_all(visible = false);
//! * the local Kingdom (0x76AC70) and its +190/+228 words, and per GMap tile
//!   (+14 fog word) the explored (low 16 bits) and visible (high 16 bits)
//!   counts for that Kingdom;
//! * the ported 0x7208D0 fog half for EVERY TerrainBlock of the map as if each
//!   were entering visibility state 2 in the same pass (0x720670 cells,
//!   0x720AD0 grid, 0x71CCB0 +141 = 1, then 0x7203F0 for the (d0+2)^2 cells and
//!   0x71C290(t = 1, rebuild)), after one 0x725340 fog step (B285D4), composed
//!   like TerrainFog::compose. The luminance histogram and a PGM of the
//!   image are recorded.
//!
//! Then AA_FOG_FRAMES native product frames run (the live Detection tick
//! updates the tile fog words) and the census repeats.
//! Inputs not read from a source here, stated: UVar0 TerrainQuadSize is its
//! registered value 2 (no profile in this process), DevMode (UVar62) is 0 so
//! 0x40FCA0's debug gate is false, and the debug UVars are 0. Run with
//! AA_GAME_ROOT.
#![cfg(windows)]
#[path = "../../aa-client/src/runtime_fp.rs"]
mod runtime_fp;
#[path = "../../aa-client/src/windows_reset_os.rs"]
mod windows_reset_os;
use aa_actors::ActorId;
use aa_netcode::{
    local_bootstrap::{self, Notifications, world_ready::ActualActivatedSession},
    local_host::NodeTime,
    local_skirmish_startup::{Choices, Startup},
};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{
    catalog::Catalog,
    live_first_tick::{AudioOutput, MusicRequest, Playback},
    localization::Localization,
    settings::Settings,
    view_terrain_fog::{
        BlockFog, CellLayout, FogVariables, Globals, MapFog, Reads, cells_720670, grid_720ad0,
        live::{FrameWords, LiveReads},
    },
};
use std::{cell::RefCell, path::PathBuf, sync::Arc};

fn report_base() -> PathBuf {
    std::env::var_os("AA_FOG_REPORT").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../research/phase8-fog/berlin-run-{}", std::process::id()))
    })
}
fn record(stage: &str, value: &serde_json::Value) {
    let base = report_base();
    std::fs::create_dir_all(base.parent().unwrap()).unwrap();
    let path = base.with_extension(format!("{stage}.json"));
    std::fs::write(&path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    println!("{}", path.display());
}
fn env_count(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// Headless frontend, as tests/live_recruit_order.rs: no audio device, a
/// real 59AB60/59ABA0 QPC owner for B396E0, [B39B1C] 0, DevMode 0.
struct HeadlessFrontend {
    requests: Vec<String>,
    application_timer: windows_reset_os::PerformanceTimer,
}
impl AudioOutput for HeadlessFrontend {
    fn play_747630(&mut self, request: &MusicRequest) -> Result<Playback, String> {
        self.requests.push(request.audio.id().to_owned());
        Ok(Playback::Declined)
    }
}
impl local_bootstrap::live_frame::Frontend for HeadlessFrontend {
    fn dev_mode_current(&mut self) -> Result<u8, String> {
        Ok(0)
    }
    fn application_milliseconds_b396e0_current(&mut self) -> Result<u32, String> {
        self.application_timer.milliseconds().map_err(|e| e.to_string())
    }
    fn view_time_b39b1c_current(&mut self) -> Result<u32, String> {
        Ok(0)
    }
}

struct SimState {
    application: aa_runtime::application_state::State,
    session: Option<Activated>,
}
struct Activated {
    session: ActualActivatedSession,
    source: Arc<ActorCatalog>,
    notifications: Notifications,
    time: NodeTime<windows_reset_os::PerformanceTimer>,
}

fn catalog_for<'a>(source: &'a Arc<ActorCatalog>, staged_resources: Option<&Arc<ResourceCatalog>>, settings: Settings,
    load_mode: Option<u32>) -> Result<Catalog<'a>, String> {
    let mut catalog = Catalog::new(source);
    let resources = staged_resources.ok_or("activation actual Resource source owner absent")?;
    catalog.bind_resource_catalog(Arc::clone(resources))?;
    catalog.settings = Some(settings);
    catalog.native_load_mode = load_mode;
    Ok(catalog)
}

/// The words of one VWorld update in this headless process.
fn frame_words(session: &ActualActivatedSession) -> FrameWords {
    let e4 = session.session_state_e4();
    let kingdom = if e4 == 3 {
        Ok(None)
    } else {
        match session.local_player_kingdom_76ac70() {
            Ok(Some(k)) => Ok(Some(k)),
            Ok(None) => Err("5C0E50 UI+1E0 fallback is not lent headless".to_string()),
            Err(e) => Err(e.to_string()),
        }
    };
    let unread = || Err::<u8, String>("debug UVar record read although the 40FCA0 gate is false".into());
    FrameWords {
        kingdom_5c0e50: kingdom,
        application_state_e4: Ok(Some(e4)),
        debug_gate_40fca0: Ok(e4 == 3),
        debug_object_blocking: unread(),
        debug_water_blocking: unread(),
        show_blocking_22: Ok(0),
        settlement_debug_25: Ok(0),
        guard_debug_27: Ok(0),
        control_debug_29: Ok(0),
        fog_debug_31: Ok(0),
        terrain_grid_100: Ok(0),
        alpha_transitions_102: Ok(0),
        local_player_5c0e10: Ok(session.local_player_present_76ac60()),
        performance_disable_doodads: Ok(Some(0)),
    }
}

/// One census of the activated World (see the module docs).
fn census(session: &ActualActivatedSession, block_size_a0: i32, milliseconds: u32, label: &str) -> Result<serde_json::Value, String> {
    let p = Precision::Double53;
    let world = session.world();
    let parts = world.presentation_parts()?;
    let words = frame_words(session);
    let kingdom = words.kingdom_5c0e50.clone()?;
    let vars = FogVariables::from_settings(parts.settings)?;
    let mut reads = LiveReads::new(&parts, &words, &vars);
    let fog_of_war = reads.fog_of_war()?;
    let g = parts.zones.geometry;
    let (w, h) = (g.width, g.height);
    // Tile census for the local Kingdom (0x54F3E0 / 0x54ED10 masks).
    let (k190, k228) = match kingdom {
        Some(k) => (Some(reads.kingdom_190(k)?), Some(reads.kingdom_228(k)?)),
        None => (None, None),
    };
    let (mut explored, mut visible, mut neither) = (0usize, 0usize, 0usize);
    for t in 0..parts.zones.tiles.len() {
        let fog = reads.tile_fog_14(t)?;
        let mask = k228.unwrap_or(0);
        if mask & fog & 0xffff_0000 != 0 {
            visible += 1;
        } else if mask & fog & 0xffff != 0 {
            explored += 1;
        } else {
            neither += 1;
        }
    }
    // 0x725340's fog step: B285D4 from AVar64.
    let mut globals = Globals::from_fresh_process_image();
    let mut clock = MapFog::<ActorId>::from_721330();
    clock.update_725340(&mut globals, Ok(milliseconds), Ok(kingdom), vars.gray_percentage_64, Ok(0), p)?;
    // TerrainMap geometry (0x723E00 with UVar0 = 2): +A8 = A0 / AC, +B4 =
    // gtile / AC, +D0 = +A8 / +B4.
    let quad = 2;
    let gtile = f32::from_bits(parts.terrain.grid.cell_size) as i32;
    let layout = CellLayout { tiles: [w, h], blocks_x: 0, quads_a8: block_size_a0 / quad, quads_per_gtile_b4: gtile / quad };
    let d0 = layout.quads_a8 / layout.quads_per_gtile_b4;
    if d0 <= 0 || w % d0 != 0 || h % d0 != 0 {
        return Err(format!("GMap {w}x{h} is not tiled by d0 {d0}"));
    }
    let blocks = [w / d0, h / d0];
    let layout = CellLayout { blocks_x: blocks[0], ..layout };
    let mut n = 0;
    let mut image = Vec::new();
    let mut cell_flags = std::collections::BTreeMap::<u8, usize>::new();
    for r in 0..blocks[1] {
        for c in 0..blocks[0] {
            let mut block = BlockFog::from_723960([c, r]);
            block.cells = Some(cells_720670(d0, [c, r], layout, &mut |t| reads.tile_variation_0(t))?);
            block.grid = Some(grid_720ad0(d0, quad, vars.pixels_per_meter_61, p)?);
            block.loaded_141 = 1;
            block.update_7208d0(2, d0, &globals, clock.delta_13c, &mut reads, p)?;
            for cell in block.cells.as_ref().unwrap() {
                if cell.block_8.is_some() {
                    *cell_flags.entry(cell.flags_c).or_default() += 1;
                }
            }
            let grid = block.grid.unwrap();
            n = grid.n_12c;
            if image.is_empty() {
                image = vec![0u8; (blocks[0] * n * blocks[1] * n) as usize];
            }
            let stride = (blocks[0] * n) as usize;
            for y in 0..n {
                for x in 0..n {
                    image[(r * n + y) as usize * stride + (c * n + x) as usize] = grid.pixels[(y * n + x) as usize][0];
                }
            }
        }
    }
    let mut histogram = std::collections::BTreeMap::<u8, usize>::new();
    for v in &image {
        *histogram.entry(*v).or_default() += 1;
    }
    let pgm = report_base().with_extension(format!("{label}.pgm"));
    let mut bytes = format!("P5\n{} {}\n255\n", blocks[0] * n, blocks[1] * n).into_bytes();
    bytes.extend_from_slice(&image);
    std::fs::write(&pgm, bytes).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "fog_of_war_svar": fog_of_war,
        "session_e4": session.session_state_e4(),
        "kingdom_5c0e50": kingdom.map(|k| k.raw()),
        "kingdom_190": k190, "kingdom_228": k228.map(|v| format!("{v:#010x}")),
        "gmap": [w, h], "gtile": gtile, "terrain_block_size_a0": block_size_a0, "d0": d0, "blocks": blocks,
        "fog_pixels_per_block": n,
        "tiles": {"visible": visible, "explored_only": explored, "unexplored": neither},
        "b285d4": globals.explored_b285d4, "w138": clock.seconds_138, "w13c": clock.delta_13c,
        "in_map_cell_flags_7203f0": cell_flags.iter().map(|(f, c)| (format!("{f:#04x}"), *c)).collect::<std::collections::BTreeMap<_, _>>(),
        "luminance_histogram": histogram.iter().map(|(v, c)| (v.to_string(), *c)).collect::<std::collections::BTreeMap<_, _>>(),
        "image": pgm.display().to_string(),
    }))
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets, accepted v14 source receipt, real Windows and the complete native startup chain"]
fn berlin_terrain_fog_census() {
    let app_thread = Arc::new(aa_runtime::execution_context::ApplicationThread::capture(&mut windows_reset_os::WindowsThreadIds).unwrap());
    let mut simulation = aa_runtime::simulation_execution::SimulationThread::start(Arc::clone(&app_thread),
        windows_reset_os::WindowsThreadIds, |_| Ok(SimState { application: aa_runtime::application_state::State::from_constructor(), session: None }))
        .unwrap();
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut files = V14Sources::open(root.join("Data.rwd"), root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: u64::MAX }).unwrap();
    let actors = Arc::new(ActorCatalog::load_v14(&mut files).unwrap());
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(Settings::load_v14(&mut files, Limits::default()).unwrap());
    let text = Localization::load_v14(&mut files, Limits::default()).unwrap();
    catalog.definition_text = Some(text.clone());
    *catalog.localization.borrow_mut() = Some(text.clone());
    let proof = aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
    let creation_startup = aa_runtime::actor_creation_startup::StartupInputs::from_verified_startup_reference(&proof).unwrap();
    aa_runtime::combat_source_completion::install(&catalog, &mut files, proof.context(), proof.csv(), proof.globals()).unwrap();
    aa_runtime::combat_source_completion::compute(&catalog).unwrap();
    aa_runtime::replacement_flags_source::complete_source_suffix(&catalog).unwrap();
    let view_inputs = aa_runtime::view_startup_inputs::Inputs::from_fresh_sources(&catalog, &proof, &mut files,
        aa_runtime::load_view_variables::NativeOverrides::ControlledNone).unwrap();
    let block_size_a0 = view_inputs.grid().terrain_block_size();
    // The three fog AVars of the v14 sources (FogVariables::from_sources)
    // agree with the Settings projection the census uses.
    let from_sources = FogVariables::from_sources(&catalog, &mut files, aa_runtime::load_view_variables::NativeOverrides::ControlledNone).unwrap();
    assert_eq!(from_sources, FogVariables::from_settings(catalog.settings.as_ref().unwrap()).unwrap());
    let resources = Arc::new(ResourceCatalog::load_v14(&mut files).unwrap());
    let variables = aa_runtime::creator_variables::State::from_fresh_sources(&catalog, &mut files,
        aa_runtime::load_view_variables::NativeOverrides::ControlledNone).unwrap();
    let clock = windows_reset_os::PerformanceTimer::from_native_constructor().unwrap();
    let mut lifecycle = aa_runtime::restore_lifecycle::ScopedManagedLoad::from_sources(&mut files, clock.milliseconds().unwrap()).unwrap();
    lifecycle.select_local_offline(&catalog, &mut files, aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
        &mut windows_reset_os::WindowsStandaloneClock).unwrap();
    let output_root = report_base().with_extension("output");
    std::fs::create_dir_all(&output_root).unwrap();
    let mut services = aa_runtime::world_ready_services::Application::from_fresh_sources(&catalog, files,
        aa_runtime::application_mode::GameAppState::from_native_constructor(), creation_startup, &output_root,
        aa_runtime::load_view_variables::NativeOverrides::ControlledNone).unwrap();
    services.install_view_inputs(&catalog, view_inputs).unwrap();
    let map = std::env::var("AA_PLAYABLE_MAP").unwrap_or_else(|_| "Maps/(2v2)Berlin.RMP".into());
    let selected = services.depot_mut().select_map(&catalog, &map.encode_utf16().collect::<Vec<_>>()).unwrap();
    let ui_precision = Precision::Single24;
    let mut startup = match Startup::construct(&catalog, &variables, Arc::clone(&selected), ui_precision) {
        Ok(v) => v,
        Err((_, e)) => panic!("{e}"),
    };
    let confirmation = simulation
        .run(|state, _| local_bootstrap::selected_map_confirm::OrdinaryEntry::capture_current_application(&state.application).map_err(|e| e.to_string()))
        .unwrap();
    let host_input = confirmation.into_host_input(&catalog, &catalog.definition_bank.borrow(), services.application(), &selected,
        aa_netcode::lobby::GameOptions { name: "Native local skirmish".encode_utf16().collect(), password: vec![], max_players: -1,
            field_d8: u32::MAX, allow_observers: true }).unwrap();
    let choices = Choices { host: host_input, player_name: "Local human".encode_utf16().collect(), seed: 0 };
    let mut gate = || {
        simulation.identity().sync_gate(&mut windows_reset_os::WindowsThreadIds, || Err("actual same SAI flags required on simulation thread".into()))
            .map_err(aa_netcode::Error::Protocol)
    };
    let mut reset = windows_reset_os::WindowsResetOs::new(&mut gate);
    let mut time = NodeTime::from_native_constructor(windows_reset_os::PerformanceTimer::from_native_constructor().unwrap());
    time.begin_update_prefix(ui_precision).unwrap();
    let launch = startup.host_and_launch_current_text(&catalog, choices, &mut reset, &mut time, &mut lifecycle, &mut services,
        &simulation.identity(), || windows_reset_os::WindowsThreadIds, ui_precision);
    drop(reset);
    record("launch", &serde_json::json!({"map":map,"error":launch.as_ref().err().map(|e|e.to_string())}));
    launch.unwrap();
    let parts = services.parts(&catalog).unwrap();
    let transfer = match catalog.into_source_transfer(Arc::clone(&actors)) {
        Ok(v) => v,
        Err((e, _)) => panic!("{e}"),
    };
    let result = startup.run_queued_resource_task(transfer, lifecycle, parts.depot, parts.application, parts.startup, parts.orientation,
        resources, Arc::clone(&app_thread), simulation.identity(), || windows_reset_os::WindowsThreadIds, runtime_fp::sample, Precision::Double53);
    let task = match result {
        Ok(task) => task,
        Err(failure) => panic!("resource task failed: {}", failure.error()),
    };
    let (host, notifications) = startup.into_host_and_notifications();
    let source = Arc::clone(&actors);
    let activation = simulation
        .run(move |state, _execution| {
            let staged = task.mission().staged();
            let catalog = catalog_for(&source, staged.resource_source.as_ref(), staged.settings.clone(), staged.native_load_mode)?;
            let (mut ready, mut success) = (vec![], vec![]);
            let activation = host.finish_completed_world(task, &catalog, services, Arc::clone(&source), &notifications, &mut time,
                Precision::Double53, &mut windows_reset_os::WindowsLocalFiletime::current, &mut ready, &mut success);
            drop(catalog);
            Ok(match activation {
                Ok(session) => {
                    state.session = Some(Activated { session, source, notifications, time });
                    serde_json::json!({"activated":true})
                }
                Err(failure) => serde_json::json!({"activated":false,"error":failure.error()}),
            })
        })
        .unwrap();
    record("activation", &activation);
    assert_eq!(activation["activated"], true, "{activation}");

    let report = simulation
        .run(move |state, execution| {
            let activated = state.session.as_mut().ok_or("activation was not retained")?;
            let timer = windows_reset_os::PerformanceTimer::from_native_constructor().map_err(|e| e.to_string())?;
            let ms = || timer.milliseconds().map_err(|e| e.to_string());
            let start = census(&activated.session, block_size_a0, ms()?, "start");
            let settings = activated.session.world().presentation_parts()?.settings.clone();
            let resources = activated.session.world().evidence().resource_source.clone();
            let load_mode = activated.session.world().evidence().native_load_mode;
            let source = Arc::clone(&activated.source);
            let catalog = catalog_for(&source, resources.as_ref(), settings, load_mode)?;
            let window = aa_runtime::application_window_state::ApplicationWindowState::from_native_constructor();
            let focus = window.reader();
            let identity = execution.identity();
            let os = RefCell::new(windows_reset_os::WindowsThreadIds);
            let mut frontend = HeadlessFrontend { requests: vec![], application_timer: windows_reset_os::PerformanceTimer::from_native_constructor().map_err(|e| e.to_string())? };
            activated.session.parts().0.request_version_ack_766d40().map_err(|e| e.to_string())?;
            let mut frames = vec![];
            for frame in 0..env_count("AA_FOG_FRAMES", 12) {
                std::thread::sleep(std::time::Duration::from_millis(70));
                let mut diagnostics = local_bootstrap::live_frame::Diagnostics::default();
                let result = activated.session.run_native_frame(&catalog, &mut activated.time, &focus, &activated.notifications,
                    &identity, &os, &mut frontend, &mut diagnostics, Precision::Double53);
                let error = result.as_ref().err().map(|e| e.to_string());
                frames.push(serde_json::json!({"frame":frame,"error":error}));
                if error.is_some() {
                    break;
                }
            }
            let after = census(&activated.session, block_size_a0, ms()?, "after-frames");
            let value = |r: Result<serde_json::Value, String>| r.unwrap_or_else(|e| serde_json::json!({"error": e}));
            Ok(serde_json::json!({"start":value(start),"frames":frames,"after_frames":value(after)}))
        })
        .unwrap();
    record("census", &report);
    let start = &report["start"];
    assert!(start.get("fog_of_war_svar").is_some(), "{report}");
    assert_eq!(start["fog_of_war_svar"], 1, "Berlin RULES FogOfWar (1 = explored)");
}
