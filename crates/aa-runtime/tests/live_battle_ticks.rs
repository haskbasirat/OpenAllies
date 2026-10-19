//! Headless live-battle tick census on a real selected skirmish.
//! Real selected-map Host -> main launch -> queued resource task -> actual
//! 775DB0 activation (finish_completed_world) on the SAME simulation thread,
//! then native 75CA70 product frames and direct GWorld 4F9970 updates.
//! Every error is recorded; nothing here mints readiness or turns a refusal
//! into success. Run with AA_GAME_ROOT and optional AA_PLAYABLE_MAP,
//! AA_LIVE_TICKS, AA_LIVE_FRAMES, AA_PLAYABLE_REPORT.
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
use aa_runtime::{
    catalog::Catalog,
    live_first_tick::{AudioOutput, MusicRequest, Playback},
    live_world_frame::TickInputs,
    localization::Localization,
    settings::Settings,
};
use std::{cell::RefCell, collections::BTreeMap, path::PathBuf, sync::Arc};

fn report_base() -> PathBuf {
    std::env::var_os("AA_PLAYABLE_REPORT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../../research/live-battle-ticks/run-{}",
                std::process::id()
            ))
        })
}
fn record(stage: &str, value: &serde_json::Value) {
    let base = report_base();
    std::fs::create_dir_all(base.parent().unwrap()).unwrap();
    let path = base.with_extension(format!("{stage}.json"));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap();
    serde_json::to_writer_pretty(file, value).unwrap();
    println!("{}", path.display());
}
fn env_count(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Headless process: there is no audio device. 747630's result is ignored by
/// the native caller; Declined truthfully reports that no output accepted it.
struct HeadlessAudio {
    requests: Vec<String>,
}
impl AudioOutput for HeadlessAudio {
    fn play_747630(&mut self, request: &MusicRequest) -> Result<Playback, String> {
        self.requests.push(request.audio.id().to_owned());
        Ok(Playback::Declined)
    }
}
impl local_bootstrap::live_frame::Frontend for HeadlessAudio {
    fn dev_mode_current(&mut self) -> Result<u8, String> {
        Err("headless harness has no Ui.uvars DevMode backend".into())
    }
}

/// Simulation-thread state: the application AppState and, after activation,
/// the one retained ActualActivatedSession. It never leaves this thread.
struct SimState {
    application: aa_runtime::application_state::State,
    session: Option<Activated>,
}
struct Activated {
    session: ActualActivatedSession,
    source: Arc<ActorCatalog>,
    notifications: Notifications,
    time: NodeTime<windows_reset_os::PerformanceTimer>,
    /// The unit given the harness Move order, when AA_LIVE_MOVE is set.
    move_unit: Option<u32>,
}

fn catalog_for<'a>(
    source: &'a Arc<ActorCatalog>,
    staged_resources: Option<&Arc<ResourceCatalog>>,
    settings: Settings,
    load_mode: Option<u32>,
) -> Result<Catalog<'a>, String> {
    let mut catalog = Catalog::new(source);
    let resources = staged_resources.ok_or("activation actual Resource source owner absent")?;
    catalog.bind_resource_catalog(Arc::clone(resources))?;
    catalog.settings = Some(settings);
    catalog.native_load_mode = load_mode;
    Ok(catalog)
}

/// Read-only per-tick observation of the SAME retained World.
fn observe(world: &aa_runtime::live_world_frame::World<'_>) -> serde_json::Value {
    let clock = world.clock();
    let queues: Vec<_> = world
        .queues()
        .snapshot()
        .iter()
        .map(|q| {
            serde_json::json!({"id":q.id,"current":q.current,
            "entries":q.buckets.iter().map(|b|b.len()).sum::<usize>()})
        })
        .collect();
    let mut positions = BTreeMap::new();
    if let Ok(spatial) = world.spatial() {
        for identity in world.actors().identities().iter() {
            if let Ok(p) = world.actors().placement(identity.value, spatial) {
                positions.insert(identity.id.raw(), p.position_bits);
            }
        }
    }
    //Kingdom+1C4 stockpiles and +338 snapshot counts of the SAME owners.
    let mut kingdoms = BTreeMap::new();
    for identity in world.actors().identities().iter() {
        if let Ok(entity) = world.actors().entity(identity.value) {
            if let aa_runtime::actor_world::Source::Special(special) = &entity.source {
                if let (aa_runtime::actor_world::SpecialSource::Kingdom(_), Some(shell)) =
                    (&special.source, special.derived.as_ref())
                {
                    let stock: Vec<f32> = shell
                        .vectors
                        .get(&0x1c4)
                        .map(|v| v.values.iter().map(|b| f32::from_bits(*b)).collect())
                        .unwrap_or_default();
                    let snapshots = world
                        .evidence()
                        .kingdom_economy
                        .get(&identity.id.raw())
                        .map(|e| e.snapshots.len());
                    kingdoms.insert(
                        identity.id.raw(),
                        serde_json::json!({"stockpiles":stock,"snapshots":snapshots}),
                    );
                }
            }
        }
    }
    serde_json::json!({
        "tick":clock.tick,"begin_bits":clock.begin.0,"end_bits":clock.end.0,
        "stopped":world.stopped(),"queues":queues,"placed_actors":positions.len(),
        "kingdoms":kingdoms,"positions":positions,
    })
}
/// Every queued event's scheduler target; Mind targets add kind/current state.
fn queue_targets(world: &aa_runtime::live_world_frame::World<'_>) -> serde_json::Value {
    use aa_runtime::scheduler_owners::Target;
    let mut rows = vec![];
    for queue in world.queues().snapshot() {
        for bucket in &queue.buckets {
            for (owner, data) in bucket {
                let target = world.owners().target(*owner);
                let mut row = serde_json::json!({"queue":queue.id,"owner":owner,"data":data,
                    "target":format!("{target:?}")});
                if let Ok(Target::Component { object, slot }) = target {
                    if let Ok(object_ref) = world.actors().object(object) {
                        row["definition"] = object_ref.definition.proof.id.clone().into();
                        if slot == 4 {
                            if let Ok(components) = object_ref.components() {
                                if let Some(aa_runtime::actor_world::RuntimeComponent::Mind(m)) =
                                    components.get(aa_actors::components::ComponentSlot::Mind)
                                {
                                    let current = world
                                        .actors()
                                        .minds
                                        .mind(m.mind_id)
                                        .ok()
                                        .and_then(|core| core.stack.front().copied().flatten());
                                    let offset = current.and_then(|s| {
                                        m.state_bindings.iter().find(|(_, v)| **v == s).map(|(k, _)| *k)
                                    });
                                    row["mind"] = serde_json::json!({"kind":format!("{:?}",m.kind),
                                        "current_state_offset":offset,
                                        "word_keys":m.words.keys().collect::<Vec<_>>()});
                                }
                            }
                        }
                    }
                }
                rows.push(row);
            }
        }
    }
    serde_json::Value::Array(rows)
}
fn moved(before: &serde_json::Value, after: &serde_json::Value) -> Vec<u64> {
    let (Some(a), Some(b)) = (
        before["positions"].as_object(),
        after["positions"].as_object(),
    ) else {
        return vec![];
    };
    b.iter()
        .filter(|(k, v)| a.get(*k).is_some_and(|old| old != *v))
        .filter_map(|(k, _)| k.parse().ok())
        .collect()
}
fn strip_positions(mut value: serde_json::Value) -> serde_json::Value {
    if let Some(object) = value.as_object_mut() {
        object.remove("positions");
    }
    value
}

/// The UI order stream context (as aa-client's OrderSchema): the
/// ActorCommand registry is the predefined command set; no other registry is
/// reached by the right-click Move request.
struct OrderSchema;
impl aa_orders::payload::SchemaContext for OrderSchema {
    fn registry_count(&self, registry: aa_orders::payload::Registry) -> aa_orders::Result<usize> {
        if registry == aa_runtime::player_order_input::requests::ACTOR_COMMANDS {
            return Ok(aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS.len());
        }
        Err(aa_orders::Error::SchemaContext("harness order stream reached an unbound registry"))
    }
    fn team_command_flags(&self, _: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext("harness order stream has no TeamCommand source"))
    }
    fn definition_kind(&self, _: aa_orders::payload::Registry, _: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext("harness order stream has no definition kind source"))
    }
    fn zero_width_list_budget(&self) -> usize {
        0
    }
}
/// Current Mind stack (state offsets) and active component of one actor.
fn mind_state(world: &aa_runtime::live_world_frame::World<'_>, gtid: u32) -> serde_json::Value {
    let Some(key) = world.actors().key(aa_actors::ActorId::from_raw(gtid)) else { return serde_json::Value::Null };
    let Ok(object) = world.actors().object(key) else { return serde_json::Value::Null };
    let Ok(components) = object.components() else { return serde_json::Value::Null };
    let Some(aa_runtime::actor_world::RuntimeComponent::Mind(m)) =
        components.get(aa_actors::components::ComponentSlot::Mind) else { return serde_json::Value::Null };
    let stack: Vec<_> = world.actors().minds.mind(m.mind_id).ok().map(|core| {
        core.stack.iter().map(|s| s.and_then(|s| m.state_bindings.iter().find(|(_, v)| **v == s).map(|(k, _)| *k))).collect()
    }).unwrap_or_default();
    serde_json::json!({"stack_offsets":stack,"active":format!("{:?}",object.active_component),
        "position":world.spatial().ok().and_then(|sp| world.actors().placement(key, sp).ok()).map(|p| p.position_bits)})
}
/// The real UI request path (0x436C50 clear, 0x436AD0 add, then 0x61E960
/// TellSelectedCommand) emitted into the SAME AdminNode request stream.
/// `move`: 0x620760's right-click command with the 0x620957 location, to the
/// local player's first Mover/Element/Mind Actor. `pack`: the explicit
/// command 34 (KKC "pack", component 19) to its first MobileDeployment Actor.
/// `cmdN`: explicit command N to its first owned class3 Actor.
/// Harness Globals for the direct World-level decree probe: every reached
/// gate read is reported as an explicit error, except Session+F8 phase.
struct ProbeGlobals { phase: u32 }
impl aa_runtime::player_order_world::Globals for ProbeGlobals {
    fn can_process_40eaa0(&mut self) -> aa_runtime::player_order_world::Result<bool> {
        Err(aa_runtime::player_order_world::Error::Required("harness probe reached 40EAA0".into()))
    }
    fn session_phase_76ac90(&mut self) -> aa_runtime::player_order_world::Result<u32> { Ok(self.phase) }
}
fn issue_order(session: &mut ActualActivatedSession, kind: &str, catalog: Option<&Catalog<'_>>) -> Result<(serde_json::Value, Option<u32>), String> {
    use aa_actors::components::ComponentSlot as Slot;
    use aa_runtime::player_order_input::{command, requests::{AdminRequests, Counts, Requests}};
    let (host, world, _) = session.parts();
    let (running, local_player, players, phase) = {
        let s = host.snapshot();
        (s.node_active, s.local_player, s.players.clone(), s.phase)
    };
    let mut kingdoms = BTreeMap::new();
    for identity in world.actors().identities().iter() {
        if let Ok(object) = world.actors().object(identity.value) {
            if object.definition.native_class == 4 {
                kingdoms.insert(identity.id.raw(), (object.definition.proof.id.clone(),
                    String::from_utf16_lossy(&object.thing.name_utf16)));
            }
        }
    }
    let local = players.iter().find(|p| p["id"].as_u64().map(|v| v as u32) == local_player).cloned();
    let local_kingdom_text = local.as_ref().and_then(|p| p["kingdom"].as_array()).map(|units| {
        String::from_utf16_lossy(&units.iter().filter_map(|u| u.as_u64().map(|v| v as u16)).take_while(|&u| u != 0).collect::<Vec<_>>())
    });
    let wanted_owner = std::env::var("AA_MOVE_OWNER").ok().and_then(|v| v.parse::<u32>().ok()).or_else(|| {
        let text = local_kingdom_text.as_ref()?;
        kingdoms.iter().find(|(_, (id, name))| id.eq_ignore_ascii_case(text) || name.eq_ignore_ascii_case(text)).map(|(k, _)| *k)
    });
    let mut owned = vec![];
    let mut chosen = None;
    let index = env_count("AA_MOVE_INDEX", 0);
    let mut seen = 0;
    let spatial = world.spatial()?;
    for identity in world.actors().identities().iter() {
        let Ok(object) = world.actors().object(identity.value) else { continue };
        let owner = object.actor.as_ref().and_then(|a| a.words.get(&0xe8)).copied().unwrap_or(0);
        if object.definition.native_class != 3 || wanted_owner.is_some_and(|o| o != owner) { continue }
        let Ok(components) = object.components() else { continue };
        let slots: Vec<String> = components.iter().map(|(s, _)| format!("{s:?}")).collect();
        owned.push(serde_json::json!({"gtid":identity.id.raw(),"id":object.definition.proof.id,"slots":slots}));
        let eligible = match kind {
            "move" => components.present(Slot::Mover) && components.present(Slot::Element) && components.present(Slot::Mind),
            "pack" => components.present(Slot::MobileDeployment),
            _ => true,
        };
        if eligible && chosen.is_none() {
            if seen == index {
                let p = world.actors().placement(identity.value, spatial).map_err(|e| e.to_string())?;
                chosen = Some((identity.id.raw(), object.definition.proof.id.clone(), p.position_bits));
            }
            seen += 1;
        }
    }
    let mut report = serde_json::json!({"kind":kind,"owned":owned,"running_2c0":running,"local_player":local_player,
        "players":players,"local_kingdom_text":local_kingdom_text,"kingdoms":format!("{kingdoms:?}"),
        "wanted_owner":wanted_owner,"chosen":format!("{chosen:?}")});
    let Some((gtid, _, position)) = chosen else { return Ok((report, None)) };
    let mut wire = command::new_right_click_command(0);
    match kind {
        "move" => {
            let dx: f32 = std::env::var("AA_MOVE_DX").ok().and_then(|v| v.parse().ok()).unwrap_or(40.0);
            let dy: f32 = std::env::var("AA_MOVE_DY").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let target = [(f32::from_bits(position[0]) + dx).to_bits(), (f32::from_bits(position[1]) + dy).to_bits()];
            command::set_location(&mut wire, target);
            report["target_bits"] = serde_json::json!(target);
        }
        other => {
            let index: u32 = if other == "pack" { 34 } else {
                other.strip_prefix("cmd").and_then(|n| n.parse().ok()).ok_or("unknown harness order kind")?
            };
            //An explicit command (4D9A90): flag bit 6 and +08.
            wire = aa_orders::commands::CommandWire::default();
            wire.flags |= 0x40;
            wire.command = Some(index);
        }
    }
    if let (Some(catalog), true) = (catalog, std::env::var("AA_ORDER_PATH").is_ok_and(|v| v == "direct")) {
        //Direct 61F970 decree body on the SAME retained World (probe only).
        let actor_wire = aa_orders::commands::ActorCommandWire { command: wire.clone(), actor: aa_orders::commands::ActorSlotReference(gtid as u16) };
        let mut globals = ProbeGlobals { phase };
        let outcome = world.borrow_frame().with_gameplay_order(|parts| {
            aa_runtime::player_order_frame::actor(catalog, parts, &actor_wire, &mut globals)
        });
        report["direct"] = format!("{outcome:?}").into();
        report["wire"] = format!("{wire:?}").into();
        return Ok((report, Some(gtid)));
    }
    let admin = host.order_node_mut().map_err(|e| e.to_string())?;
    let mut requests = AdminRequests {
        admin,
        running_2c0: running,
        local_player,
        context: &OrderSchema,
        actor_slot: Ok::<u16, String> as fn(u16) -> Result<u16, String>,
    };
    let counts = Counts { commands: aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS.len(), objects: 0 };
    let sent = if std::env::var("AA_ORDER_PATH").is_ok_and(|v| v == "selected") {
        Requests::<u16>::clear_436c50(&mut requests, None)?;
        requests.add_436ad0(gtid as u16, None)?;
        requests.tell_selected_61e960(&wire, counts, None)?
    } else {
        //85 KKC_TellActorCommandOrderRequest: the 0x61E6B0 command then the
        //0x76A5A0 actor slot (B4 check), through the same request emitter.
        let mut fields = aa_runtime::player_order_input::requests::command_fields(&wire, counts)?;
        fields.push(aa_orders::payload::Field::Thing { check: aa_orders::payload::ThingCheck::B4, slot: gtid as u16 });
        let player = local_player.ok_or("0x76AC60 local Node+8 player is NULL")?;
        requests.admin.emit_request(running, aa_orders::commands::ACTOR_REQUEST_ID, &fields, player, &OrderSchema)
            .map_err(|e| e.to_string())?
    };
    report["sent"] = sent.into();
    report["wire"] = format!("{wire:?}").into();
    Ok((report, Some(gtid)))
}

#[test]
#[ignore = "requires original AA_GAME_ROOT assets, accepted v14 source receipt, real Windows and the complete native startup chain"]
fn live_skirmish_ticks_census() {
    let app_thread = Arc::new(
        aa_runtime::execution_context::ApplicationThread::capture(
            &mut windows_reset_os::WindowsThreadIds,
        )
        .unwrap(),
    );
    let mut simulation = aa_runtime::simulation_execution::SimulationThread::start(
        Arc::clone(&app_thread),
        windows_reset_os::WindowsThreadIds,
        |_| {
            Ok(SimState {
                application: aa_runtime::application_state::State::from_constructor(),
                session: None,
            })
        },
    )
    .unwrap();
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = Arc::new(ActorCatalog::load_v14(&mut files).unwrap());
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(Settings::load_v14(&mut files, Limits::default()).unwrap());
    let text = Localization::load_v14(&mut files, Limits::default()).unwrap();
    catalog.definition_text = Some(text.clone());
    *catalog.localization.borrow_mut() = Some(text.clone());
    let proof =
        aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
            &catalog, &mut files,
        )
        .unwrap();
    let creation_startup =
        aa_runtime::actor_creation_startup::StartupInputs::from_verified_startup_reference(&proof)
            .unwrap();
    aa_runtime::combat_source_completion::install(
        &catalog,
        &mut files,
        proof.context(),
        proof.csv(),
        proof.globals(),
    )
    .unwrap();
    aa_runtime::combat_source_completion::compute(&catalog).unwrap();
    aa_runtime::replacement_flags_source::complete_source_suffix(&catalog).unwrap();
    let view_inputs = aa_runtime::view_startup_inputs::Inputs::from_fresh_sources(
        &catalog,
        &proof,
        &mut files,
        aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
    )
    .unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut files).unwrap());
    let variables = aa_runtime::creator_variables::State::from_fresh_sources(
        &catalog,
        &mut files,
        aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
    )
    .unwrap();
    let clock = windows_reset_os::PerformanceTimer::from_native_constructor().unwrap();
    let mut lifecycle = aa_runtime::restore_lifecycle::ScopedManagedLoad::from_sources(
        &mut files,
        clock.milliseconds().unwrap(),
    )
    .unwrap();
    lifecycle
        .select_local_offline(
            &catalog,
            &mut files,
            aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
            &mut windows_reset_os::WindowsStandaloneClock,
        )
        .unwrap();
    let output_root = report_base().with_extension("output");
    std::fs::create_dir_all(&output_root).unwrap();
    let mut services = aa_runtime::world_ready_services::Application::from_fresh_sources(
        &catalog,
        files,
        aa_runtime::application_mode::GameAppState::from_native_constructor(),
        creation_startup,
        &output_root,
        aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
    )
    .unwrap();
    services.install_view_inputs(&catalog, view_inputs).unwrap();
    let map = std::env::var("AA_PLAYABLE_MAP")
        .unwrap_or_else(|_| "Maps/DeveloperMaps/(3v3)Firefight.RMP".into());
    let selected = services
        .depot_mut()
        .select_map(&catalog, &map.encode_utf16().collect::<Vec<_>>())
        .unwrap();
    let ui_precision = Precision::Single24;
    let mut startup =
        match Startup::construct(&catalog, &variables, Arc::clone(&selected), ui_precision) {
            Ok(v) => v,
            Err((_, e)) => panic!("{e}"),
        };
    let confirmation = simulation
        .run(|state, _| {
            local_bootstrap::selected_map_confirm::OrdinaryEntry::capture_current_application(
                &state.application,
            )
            .map_err(|error| error.to_string())
        })
        .unwrap();
    let host_input = confirmation
        .into_host_input(
            &catalog,
            &catalog.definition_bank.borrow(),
            services.application(),
            &selected,
            aa_netcode::lobby::GameOptions {
                name: "Native local skirmish".encode_utf16().collect(),
                password: vec![],
                max_players: -1,
                field_d8: u32::MAX,
                allow_observers: true,
            },
        )
        .unwrap();
    let choices = Choices {
        host: host_input,
        player_name: "Local human".encode_utf16().collect(),
        seed: 0,
    };
    let mut gate = || {
        simulation
            .identity()
            .sync_gate(&mut windows_reset_os::WindowsThreadIds, || {
                Err("actual same SAI flags required on simulation thread".into())
            })
            .map_err(aa_netcode::Error::Protocol)
    };
    let mut reset = windows_reset_os::WindowsResetOs::new(&mut gate);
    let mut time = NodeTime::from_native_constructor(
        windows_reset_os::PerformanceTimer::from_native_constructor().unwrap(),
    );
    time.begin_update_prefix(ui_precision).unwrap();
    let launch = startup.host_and_launch_current_text(
        &catalog,
        choices,
        &mut reset,
        &mut time,
        &mut lifecycle,
        &mut services,
        &simulation.identity(),
        || windows_reset_os::WindowsThreadIds,
        ui_precision,
    );
    drop(reset);
    record(
        "launch",
        &serde_json::json!({"map":map,"error":launch.as_ref().err().map(|e|e.to_string())}),
    );
    launch.unwrap();
    let parts = services.parts(&catalog).unwrap();
    let transfer = match catalog.into_source_transfer(Arc::clone(&actors)) {
        Ok(v) => v,
        Err((e, _)) => panic!("{e}"),
    };
    let started = std::time::Instant::now();
    let result = startup.run_queued_resource_task(
        transfer,
        lifecycle,
        parts.depot,
        parts.application,
        parts.startup,
        parts.orientation,
        resources,
        Arc::clone(&app_thread),
        simulation.identity(),
        || windows_reset_os::WindowsThreadIds,
        runtime_fp::sample,
        Precision::Double53,
    );
    record(
        "worker",
        &serde_json::json!({"map":map,"seconds":started.elapsed().as_secs_f64(),
            "error":result.as_ref().err().map(|f|f.error().to_owned())}),
    );
    let task = match result {
        Ok(task) => task,
        Err(failure) => panic!("resource task failed: {}", failure.error()),
    };
    let (host, notifications) = startup.into_host_and_notifications();
    let source = Arc::clone(&actors);
    //775DB0 on the SAME simulation thread, exactly as the client controller.
    let activation = simulation
        .run(move |state, _execution| {
            let staged = task.mission().staged();
            let catalog = catalog_for(
                &source,
                staged.resource_source.as_ref(),
                staged.settings.clone(),
                staged.native_load_mode,
            )?;
            let mut ready = vec![];
            let mut success = vec![];
            let started = std::time::Instant::now();
            let activation = host.finish_completed_world(
                task,
                &catalog,
                services,
                Arc::clone(&source),
                &notifications,
                &mut time,
                Precision::Double53,
                &mut windows_reset_os::WindowsLocalFiletime::current,
                &mut ready,
                &mut success,
            );
            let seconds = started.elapsed().as_secs_f64();
            drop(catalog);
            let ready = format!("{ready:?}");
            let success = format!("{success:?}");
            Ok(match activation {
                Ok(session) => {
                    let observation = observe(session.world());
                    let targets = queue_targets(session.world());
                    state.session = Some(Activated {
                        session,
                        source,
                        notifications,
                        time,
                        move_unit: None,
                    });
                    serde_json::json!({"activated":true,"seconds":seconds,"ready":ready,
                        "success":success,"world":strip_positions(observation),"queue_targets":targets})
                }
                Err(failure) => serde_json::json!({"activated":false,"seconds":seconds,
                    "error":failure.error(),"ready":ready,"success":success}),
            })
        })
        .unwrap();
    record("activation", &activation);
    assert_eq!(activation["activated"], true, "{activation}");

    let frames = env_count("AA_LIVE_FRAMES", 8);
    let ticks = env_count("AA_LIVE_TICKS", 64);
    //strict: stop at the first unported leaf exactly as the native latch.
    //census: record and skip every unported leaf (contaminates the World).
    let census_mode = std::env::var("AA_LIVE_MODE").is_ok_and(|v| v == "census");
    //Product 75CA70 frames first: these keep every native gate, including
    //Sim+29 freeze and the admin timeline; errors latch exactly as native.
    let product = simulation
        .run(move |state, execution| {
            let activated = state.session.as_mut().ok_or("activation was not retained")?;
            let staged_settings = activated
                .session
                .world()
                .presentation_parts()?
                .settings
                .clone();
            let resources = activated
                .session
                .world()
                .evidence()
                .resource_source
                .clone();
            let load_mode = activated.session.world().evidence().native_load_mode;
            let catalog = catalog_for(
                &activated.source,
                resources.as_ref(),
                staged_settings,
                load_mode,
            )?;
            let window =
                aa_runtime::application_window_state::ApplicationWindowState::from_native_constructor();
            let focus = window.reader();
            let identity = execution.identity();
            let os = RefCell::new(windows_reset_os::WindowsThreadIds);
            let mut audio = HeadlessAudio { requests: vec![] };
            let mut diagnostics = local_bootstrap::live_frame::Diagnostics::default();
            let mut rows = vec![];
            let mut move_report = serde_json::Value::Null;
            //aa-client battle_notifications: after the kind0 application
            //state change the game screen sends the one 766D40 version ack;
            //requests with version_check are dropped until it is accepted.
            activated.session.parts().0.request_version_ack_766d40().map_err(|e| e.to_string())?;
            if std::env::var_os("AA_LIVE_MOVE").is_some() {
                let (report, unit) = issue_order(&mut activated.session, "move", None)?;
                move_report = report;
                activated.move_unit = unit;
            }
            for frame in 0..frames {
                let before = observe(activated.session.world());
                let result = activated.session.run_native_frame(
                    &catalog,
                    &mut activated.time,
                    &focus,
                    &activated.notifications,
                    &identity,
                    &os,
                    &mut audio,
                    &mut diagnostics,
                    Precision::Double53,
                );
                let after = observe(activated.session.world());
                let moved = moved(&before, &after);
                let error = result.as_ref().err().map(|e| e.to_string());
                let unit = activated.move_unit.map(|g| mind_state(activated.session.world(), g));
                rows.push(serde_json::json!({"frame":frame,"action":result.as_ref().ok().map(|a|format!("{a:?}")),
                    "error":error,"moved":moved,"unit":unit,"world":strip_positions(after)}));
                if error.is_some() {
                    break;
                }
            }
            Ok(serde_json::json!({"move":move_report,"frames":rows,"effects":diagnostics.effects,
                "tasks":format!("{:?}",diagnostics.tasks),"audio":audio.requests}))
        })
        .unwrap();
    record("product-frames", &product);

    //Direct GWorld 4F9970 updates on the same retained owners (harness entry,
    //bypassing only the admin timeline/Sim freeze gate). First error stops
    //the World exactly as the native failure latch does.
    let census = simulation
        .run(move |state, execution| {
            let activated = state.session.as_mut().ok_or("activation was not retained")?;
            let settings = activated
                .session
                .world()
                .presentation_parts()?
                .settings
                .clone();
            let resources = activated
                .session
                .world()
                .evidence()
                .resource_source
                .clone();
            let load_mode = activated.session.world().evidence().native_load_mode;
            let identity = execution.identity();
            let os = RefCell::new(windows_reset_os::WindowsThreadIds);
            let mut audio = HeadlessAudio { requests: vec![] };
            let mut rows = vec![];
            let mut total_moved = std::collections::BTreeSet::new();
            let sink = RefCell::new(Vec::<aa_runtime::live_world_frame::CensusEntry>::new());
            let first = observe(activated.session.world());
            //AA_LIVE_ORDERS="tick:kind;tick:kind": issue each order at that tick
            //through the real request stream, then run AA_ORDER_FRAMES product
            //75CA70 frames so the Admin executes the requests as decrees.
            let schedule: Vec<(usize, String)> = std::env::var("AA_LIVE_ORDERS").unwrap_or_default()
                .split(';').filter_map(|e| { let (t, k) = e.split_once(':')?; Some((t.trim().parse().ok()?, k.trim().to_owned())) }).collect();
            let order_frames = env_count("AA_ORDER_FRAMES", 6);
            let mut order_rows = vec![];
            for tick in 0..ticks {
                for (_, kind) in schedule.iter().filter(|(t, _)| *t == tick) {
                    let history = |s: &mut ActualActivatedSession| s.parts().0.order_node_mut().map(|a|
                        serde_json::json!({"requests":[a.requests.begin(),a.requests.end()],"decrees":[a.decrees.begin(),a.decrees.end()]}))
                        .unwrap_or(serde_json::Value::Null);
                    let before_history = history(&mut activated.session);
                    let probe_catalog = catalog_for(&activated.source, resources.as_ref(), settings.clone(), load_mode)?;
                    let (report, unit) = issue_order(&mut activated.session, kind, Some(&probe_catalog))?;
                    drop(probe_catalog);
                    if unit.is_some() { activated.move_unit = unit; }
                    let frame_catalog = catalog_for(&activated.source, resources.as_ref(), settings.clone(), load_mode)?;
                    let window = aa_runtime::application_window_state::ApplicationWindowState::from_native_constructor();
                    let focus = window.reader();
                    let mut diagnostics = local_bootstrap::live_frame::Diagnostics::default();
                    let mut frames_out = vec![];
                    for frame in 0..order_frames {
                        //Product frames are paced by real time (75CA70 waits for
                        //the Admin158 clock): emulate the game's frame interval.
                        std::thread::sleep(std::time::Duration::from_millis(env_count("AA_ORDER_FRAME_MS", 70) as u64));
                        let result = activated.session.run_native_frame(&frame_catalog, &mut activated.time, &focus,
                            &activated.notifications, &identity, &os, &mut audio, &mut diagnostics, Precision::Double53);
                        let unit = activated.move_unit.map(|g| mind_state(activated.session.world(), g));
                        frames_out.push(serde_json::json!({"frame":frame,"action":result.as_ref().ok().map(|a|format!("{a:?}")),
                            "error":result.as_ref().err().map(|e|e.to_string()),"unit":unit}));
                    }
                    let after_history = history(&mut activated.session);
                    let acks: Vec<String> = activated.session.host().version_ack_diagnostics().iter().map(|r| format!("{} {}", r.function, String::from_utf16_lossy(&r.message))).collect();
                    order_rows.push(serde_json::json!({"tick":tick,"order":report,"frames":frames_out,
                        "history":[before_history,after_history],"version_ack":acks,
                        "effects":diagnostics.effects,"tasks":format!("{:?}",diagnostics.tasks)}));
                }
                let before = observe(activated.session.world());
                let result = {
                    let (host, world, services) = activated.session.parts();
                    //The SAME current AVar25 RendererDefaultOrientation owner
                    //(552DD0 reads [B39990]+64) loaned for this update.
                    let mut catalog = catalog_for(&activated.source, resources.as_ref(), settings.clone(), load_mode)?;
                    catalog.bind_renderer_orientation(services.view_inputs()?.orientation())?;
                    let cells = local_bootstrap::frame_host::Cells::from_runtime(
                        host.runtime_borrow().map_err(|e| e.to_string())?,
                        &mut activated.time,
                    )
                    .map_err(|e| e.to_string())?;
                    let frame = world.borrow_frame();
                    let mut gate = || {
                        identity.sync_gate(
                            &mut *os.try_borrow_mut().map_err(|e| e.to_string())?,
                            || {
                                let f = frame.sai_sync_flags();
                                Ok(Some([f[0], f[1]]))
                            },
                        )
                    };
                    let gate_loan: &mut dyn FnMut() -> Result<bool, String> = &mut gate;
                    let gate_cell = RefCell::new(gate_loan);
                    let audio_cell = RefCell::new(&mut audio);
                    let inputs = TickInputs {
                        session: cells.players(),
                        sync_a: cells.same_a(),
                        sync_gate: &gate_cell,
                        audio: &audio_cell,
                    };
                    if census_mode {
                        frame.world_update_census(&catalog, inputs, &sink)
                    } else {
                        frame.world_update_4f9970(&catalog, inputs)
                    }
                    .map(|clock| clock.tick)
                    .map_err(|e| e.to_string())
                };
                let after = observe(activated.session.world());
                let moved = moved(&before, &after);
                total_moved.extend(moved.iter().copied());
                let unit = activated.move_unit.map(|g| mind_state(activated.session.world(), g));
                rows.push(serde_json::json!({"tick":tick,"result":result,"moved":moved,"unit":unit,
                    "world":strip_positions(after)}));
                if result.is_err() {
                    break;
                }
            }
            let last = observe(activated.session.world());
            let native_log = activated.session.parts().1.borrow_frame().native_log()?;
            let entries = sink.into_inner();
            let mut distinct: BTreeMap<String, serde_json::Value> = BTreeMap::new();
            let digits = |text: &str| -> String {
                let mut out = String::new();
                let mut in_digits = false;
                for c in text.chars() {
                    if c.is_ascii_digit() {
                        if !in_digits { out.push('#'); }
                        in_digits = true;
                    } else {
                        in_digits = false;
                        out.push(c);
                    }
                }
                out
            };
            for entry in &entries {
                let key = format!("{} => {}", digits(&entry.site), digits(&entry.error));
                let row = distinct.entry(key).or_insert_with(|| serde_json::json!({
                    "count":0,"first_tick":entry.tick,"example_site":entry.site}));
                row["count"] = (row["count"].as_u64().unwrap() + 1).into();
            }
            Ok(serde_json::json!({"orders":order_rows,"mode":if census_mode {"census"} else {"strict"},
                "census_distinct":distinct,"census_total":entries.len(),
                "ticks":rows,"actors_moved_total":total_moved.len(),
                "moved_ids":total_moved,"first":strip_positions(first),"last":strip_positions(last),
                "audio":audio.requests,"native_log_total":native_log.len(),
                "native_log_head":native_log.iter().take(20).collect::<Vec<_>>()}))
        })
        .unwrap();
    record("world-ticks", &census);
}
