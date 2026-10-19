//! Headless recruit order on the real Berlin skirmish (Maps/(2v2)Berlin.RMP).
//!
//! The same activation chain as tests/live_battle_ticks.rs (real selected-map
//! Host -> main launch -> queued resource task -> 775DB0 activation on the
//! SAME simulation thread), then the real order path of a TellSelected
//! recruit from the local player's rus_corps:
//!   CanSend  UIWorldState 0x436950's per-actor vt+150 (4CF430(kingdom, &cmd,
//!            1)) between frames over the retained World,
//!   Send     0x436C50 clear, 0x436AD0 add, 0x61E960 request 84 with the
//!            0x61E6B0 fields (flag bit6 ActorCommand 15 recruit, flag bit1
//!            the 0x76A880 Object-definition reference),
//!   frames   product 75CA70 frames: the Admin seals 84 -> decree 86 (the
//!            request's 76A960 references re-resolved on the host), the
//!            decree decodes (61EBA0), 61F210 describes, 4ED360 dumps
//!            4D9A40 on A=[B3655C], 4CF430/4CE680 permits and 4D9B60 routes
//!            to the actor's Recruit component vt+24 (583A70).
//! Every error is recorded; nothing here mints readiness. Run with
//! AA_GAME_ROOT (and optionally AA_RECRUIT_TARGET, AA_RECRUIT_REPORT,
//! AA_RECRUIT_WARMUP, AA_ORDER_FRAMES, AA_ORDER_FRAME_MS).
#![cfg(windows)]
#[path = "../../aa-client/src/runtime_fp.rs"]
mod runtime_fp;
#[path = "../../aa-client/src/windows_reset_os.rs"]
mod windows_reset_os;
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
use aa_runtime::{catalog::Catalog, live_first_tick::{AudioOutput, MusicRequest, Playback}, localization::Localization, settings::Settings};
use std::{cell::RefCell, path::PathBuf, sync::Arc};

fn report_base() -> PathBuf {
    std::env::var_os("AA_RECRUIT_REPORT").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../research/phase8-orders-input/recruit/live-run-{}", std::process::id()))
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

/// Headless frontend. No audio device (747630's result is ignored natively).
/// The B396E0 application timer is a real 59AB60/59ABA0 QPC owner started
/// with this process' frontend. [B39B1C] is the .bss view time the main
/// thread writes only around its view update (410F7A..410FFC); no view
/// update runs headless, so its current value is the zero it starts with.
/// UVar62 DevMode is the read-only flags2 Bool (aa-ui view_variables: no
/// profile/UI write can change it, 4B29B6); with no native override writer
/// its current byte is the zero it is created with.
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

/// The request stream context on the sending UI: the 39 predefined
/// ActorCommands (B3AA58) and the native Thing registry (B39F58, 76A880).
struct RecruitSchema {
    objects: usize,
}
impl aa_orders::payload::SchemaContext for RecruitSchema {
    fn registry_count(&self, registry: aa_orders::payload::Registry) -> aa_orders::Result<usize> {
        use aa_runtime::player_order_input::requests::{ACTOR_COMMANDS, OBJECT_DEFINITIONS};
        if registry == ACTOR_COMMANDS {
            return Ok(aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS.len());
        }
        if registry == OBJECT_DEFINITIONS {
            return Ok(self.objects);
        }
        Err(aa_orders::Error::SchemaContext("recruit order stream reached an unbound registry"))
    }
    fn team_command_flags(&self, _: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext("recruit order stream has no TeamCommand source"))
    }
    fn definition_kind(&self, _: aa_orders::payload::Registry, _: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext("recruit order stream has no definition kind source"))
    }
    fn zero_width_list_budget(&self) -> usize {
        0
    }
}
/// CanSend's application gate: single player, so 40EAA0 = DevMode != 0
/// (UVar62, see HeadlessFrontend); phase is Session+F8.
struct CanSendGlobals {
    phase: u32,
    multiplayer: bool,
}
impl aa_runtime::player_order_world::Globals for CanSendGlobals {
    fn can_process_40eaa0(&mut self) -> aa_runtime::player_order_world::Result<bool> {
        if self.multiplayer {
            return Err(aa_runtime::player_order_world::Error::Required("multiplayer 40EAA0 nickname gate".into()));
        }
        Ok(false)
    }
    fn session_phase_76ac90(&mut self) -> aa_runtime::player_order_world::Result<u32> {
        Ok(self.phase)
    }
}

/// CanSend + Send of the recruit request (84) from the local rus_corps.
fn issue_recruit(session: &mut ActualActivatedSession, catalog: &Catalog<'_>) -> Result<serde_json::Value, String> {
    use aa_runtime::player_order_decree::object_reference;
    use aa_runtime::player_order_input::requests::{AdminRequests, Counts, Requests};
    let target = std::env::var("AA_RECRUIT_TARGET").unwrap_or_else(|_| "rus_truck_engineer".into());
    let (host, world, _) = session.parts();
    let (running, local_player, players, phase, multiplayer) = {
        let s = host.snapshot();
        (s.node_active, s.local_player, s.players.clone(), s.phase, s.multiplayer)
    };
    let local = players.iter().find(|p| p["id"].as_u64().map(|v| v as u32) == local_player).cloned().ok_or("local player")?;
    let kingdom = local["field_28"].as_u64().map(|v| aa_actors::ActorId::from_raw(v as u32));
    // The local player's own rus_corps (Actor+E8 == Player+28).
    let mut corps = None;
    for identity in world.actors().identities().iter() {
        let Ok(object) = world.actors().object(identity.value) else { continue };
        let owner = object.actor.as_ref().and_then(|a| a.words.get(&0xe8)).copied();
        if object.definition.native_class == 3 && object.definition.proof.id == "rus_corps"
            && owner.map(aa_actors::ActorId::from_raw) == kingdom {
            let words = object.actor.as_ref().map(|a| [a.words.get(&0xf0).copied(), a.words.get(&0xf4).copied()]);
            corps = Some((identity.id, words));
            break;
        }
    }
    let (actor, actions) = corps.ok_or("no local rus_corps in the World")?;
    let objects = object_reference::object_registry_count_b39f58(catalog)?;
    let (object, bits) = object_reference::object_reference_76a880(catalog, Some(&target))?;
    let resolved = object_reference::resolve_object_76a960(catalog, object)?.map(|d| d.id.clone());
    // 6157B0's command: the recruit command button's 4D9E20 (bit6 + def)
    // then 4D9AB0(ActorData) (bit1 + +14). A target with ActorData+2F0
    // (an Organization layout) would add 4D9E90's bit5 ParameterSet.
    let layout = catalog.layout_data(Precision::Double53)
        .and_then(|d| d.organization_component(catalog.actors, &target, Precision::Double53).map_err(|e| e.to_string()))
        .map(|o| o.map(|o| o.parameters.len()));
    let mut wire = aa_orders::commands::CommandWire::default();
    wire.flags = 0x42;
    wire.command = Some(15);
    wire.object = Some(object);
    // CanSend: 0x436950's vt+150 between frames over the retained World.
    let can_send = {
        let parts = world.presentation_parts()?;
        let bank = aa_runtime::player_order_world::ReadBank::new(parts.definitions);
        let d = aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS[15];
        let definition_id = parts.definitions.token("ActorCommand", d.id);
        let object_token = resolved.as_ref().and_then(|id| parts.definitions.token("ActorDefinition", id));
        match definition_id {
            None => Err("CanSend: ActorCommand recruit token is not yet interned in the World's Bank (read-only)".to_string()),
            Some(definition_id) => {
                let mut plain = aa_runtime::mind_world::PlainCommand {
                    definition_id,
                    definition: Some(aa_mind::command::Definition { component_slot: d.component_slot, mind_type: d.mind_type, behavior_flags: d.behavior_flags }),
                    flags: 0x42, word10: 0, object_definition: object_token, position_bits: [0; 2], auxiliary_bits: [0; 2],
                    scalar_bits: 0, parameter_definition: None, extra_definition: None, boolean: 0, trailing: 0,
                };
                aa_runtime::player_order_world::authorize_selected_command_read(catalog, &bank, parts.actors, parts.spatial,
                    actor, kingdom, &mut plain, &mut CanSendGlobals { phase, multiplayer })
                    .map_err(|e| e.to_string())
            }
        }
    };
    let admin = host.order_node_mut().map_err(|e| e.to_string())?;
    let schema = RecruitSchema { objects };
    let mut requests = AdminRequests {
        admin,
        running_2c0: running,
        local_player,
        context: &schema,
        actor_slot: Ok::<u16, String> as fn(u16) -> Result<u16, String>,
    };
    let counts = Counts { commands: aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS.len(), objects };
    let fields = aa_runtime::player_order_input::requests::command_fields(&wire, counts)?;
    let sent = if can_send == Ok(true) {
        Requests::<u16>::clear_436c50(&mut requests, None)?;
        requests.add_436ad0(actor.slot(), None)?;
        Some(requests.tell_selected_61e960(&wire, counts, None)?)
    } else {
        None
    };
    Ok(serde_json::json!({"actor":actor.raw(),"actor_slot":actor.slot(),"actions_f0_f4":format!("{actions:?}"),
        "kingdom":kingdom.map(|k|k.raw()),"target":target,"objects_b39f58":objects,"object_index":object,"object_bits":bits,
        "resolved_76a960":resolved,"target_organization_layout_parameters":format!("{layout:?}"),"can_send_436950":format!("{can_send:?}"),
        "fields_61e6b0":format!("{fields:?}"),"sent_61e960":sent,"wire":format!("{wire:?}"),"phase":phase,"running_2c0":running}))
}

fn sync_tail(session: &mut ActualActivatedSession, from: i32) -> serde_json::Value {
    let (host, _, _) = session.parts();
    let Ok(rt) = host.runtime_borrow() else { return serde_json::Value::Null };
    let Ok(a) = rt.sync_a.try_borrow() else { return serde_json::Value::Null };
    let mut out = vec![];
    let mut i = from.max(a.begin());
    while i < a.end() {
        if let Some(e) = a.entry(i) {
            out.push(serde_json::json!([i, e.label, format!("{:?}", e.kind), e.value, e.hash]));
        }
        i += 1;
    }
    serde_json::json!({"enabled":a.enabled,"suspend":a.suspend,"end":a.end(),"entries":out})
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 assets, accepted v14 source receipt, real Windows and the complete native startup chain"]
fn live_berlin_rus_corps_recruit_order() {
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
            let settings = activated.session.world().presentation_parts()?.settings.clone();
            let resources = activated.session.world().evidence().resource_source.clone();
            let load_mode = activated.session.world().evidence().native_load_mode;
            let catalog = catalog_for(&activated.source, resources.as_ref(), settings, load_mode)?;
            let window = aa_runtime::application_window_state::ApplicationWindowState::from_native_constructor();
            let focus = window.reader();
            let identity = execution.identity();
            let os = RefCell::new(windows_reset_os::WindowsThreadIds);
            let mut frontend = HeadlessFrontend { requests: vec![], application_timer: windows_reset_os::PerformanceTimer::from_native_constructor().map_err(|e| e.to_string())? };
            activated.session.parts().0.request_version_ack_766d40().map_err(|e| e.to_string())?;
            let frame_ms = env_count("AA_ORDER_FRAME_MS", 70) as u64;
            let mut run = |activated: &mut Activated, frontend: &mut HeadlessFrontend, n: usize| {
                let mut rows = vec![];
                for frame in 0..n {
                    std::thread::sleep(std::time::Duration::from_millis(frame_ms));
                    let mut diagnostics = local_bootstrap::live_frame::Diagnostics::default();
                    let result = activated.session.run_native_frame(&catalog, &mut activated.time, &focus, &activated.notifications,
                        &identity, &os, frontend, &mut diagnostics, Precision::Double53);
                    let error = result.as_ref().err().map(|e| e.to_string());
                    rows.push(serde_json::json!({"frame":frame,"action":result.as_ref().ok().map(|a|format!("{a:?}")),
                        "error":error,"effects":diagnostics.effects}));
                    if error.is_some() {
                        break;
                    }
                }
                rows
            };
            let warmup = run(activated, &mut frontend, env_count("AA_RECRUIT_WARMUP", 8));
            let before = sync_tail(&mut activated.session, i32::MIN);
            let sync_from = before["end"].as_i64().unwrap_or(0) as i32;
            let order = issue_recruit(&mut activated.session, &catalog);
            let history = {
                let a = activated.session.parts().0.order_node_mut().map_err(|e| e.to_string())?;
                serde_json::json!({"requests":[a.requests.begin(),a.requests.end()],"decrees":[a.decrees.begin(),a.decrees.end()]})
            };
            let frames = run(activated, &mut frontend, env_count("AA_ORDER_FRAMES", 6));
            let after_history = {
                let a = activated.session.parts().0.order_node_mut().map_err(|e| e.to_string())?;
                serde_json::json!({"requests":[a.requests.begin(),a.requests.end()],"decrees":[a.decrees.begin(),a.decrees.end()]})
            };
            let acks: Vec<String> = activated.session.host().version_ack_diagnostics().iter()
                .map(|r| format!("{} {}", r.function, String::from_utf16_lossy(&r.message))).collect();
            Ok(serde_json::json!({"warmup":warmup,"order":order.map_err(|e|e.to_string()),"history":[history,after_history],
                "frames":frames,"sync_a_after_order":sync_tail(&mut activated.session, sync_from),"version_ack":acks,
                "audio":frontend.requests}))
        })
        .unwrap();
    record("recruit", &report);
}
