//! Real selected-map Host -> main launch -> exact queued task on a real worker.
//! Every reached native gate must pass; an expected refusal never passes this test.
#![cfg(windows)]
#[path = "../../aa-client/src/runtime_fp.rs"]
mod runtime_fp;
#[path = "../../aa-client/src/windows_reset_os.rs"]
mod windows_reset_os;
use aa_netcode::{
    local_bootstrap,
    local_host::NodeTime,
    local_skirmish_startup::{Choices, Startup, WorkerPrefix},
};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources},
    tgi::Limits,
};
use aa_runtime::{catalog::Catalog, localization::Localization, settings::Settings};
use std::{path::PathBuf, sync::Arc};
fn report_base() -> PathBuf {
    std::env::var_os("AA_PLAYABLE_REPORT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../../research/development10/playable-startup/run-{}",
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
        .create_new(true)
        .open(&path)
        .unwrap();
    serde_json::to_writer_pretty(file, value).unwrap();
    println!("{}", path.display());
}
/// Read only the genuine failed mission's retained allocations. This does not
/// manufacture a Staged world or turn the diagnostic into an entry callback.
fn fresh_actor_prefix(staged: &aa_runtime::restoration::Staged) -> Option<serde_json::Value> {
    let mut records = Vec::new();
    for identity in staged.actors.identities().iter() {
        let entity = staged.actors.entity(identity.value).unwrap();
        let aa_runtime::actor_world::Source::New {
            record: creation,
            phase,
            failure,
        } = &entity.source
        else {
            continue;
        };
        let placement = match staged.load_world.as_ref().map(|world| world.live_spatial()) {
            Some(Ok(spatial)) => serde_json::json!({
                "getter":"ActorStore::placement with SAME retained World spatial owner",
                "result":format!("{:?}",staged.actors.placement(identity.value, spatial)),
            }),
            spatial => serde_json::json!({
                "getter":"ActorStore::staged_placement; no usable live spatial owner",
                "result":format!("{:?}",staged.actors.staged_placement(identity.value)),
                "spatial_error":spatial.map(|result|result.err().map(|error|error.to_string())),
            }),
        };
        let property_instances = match entity.runtime.as_ref()
            .and_then(|runtime| runtime.actor.as_ref())
            .and_then(|actor| actor.components.get(aa_actors::components::ComponentSlot::Properties)) {
            Some(aa_runtime::actor_world::RuntimeComponent::Properties(properties)) => {
                let state = &staged.actors.property_allocations;
                let instances: Vec<_> = properties.effects.iter().map(|handle| {
                    match state.instances().get(handle) {
                        Some(instance) => serde_json::json!({
                            "effect_handle":handle.0,
                            "instance":instance,
                            "definition_ids":state.facts(instance.definition).map(|facts|facts.id()),
                        }),
                        None => serde_json::json!({
                            "effect_handle":handle.0,
                            "error":"effect handle is absent from SAME PropertyState instances",
                        }),
                    }
                }).collect();
                Some(serde_json::json!({
                    "scope":"read-only ordered SAME Properties effects and PropertyState instance/facts allocations",
                    "owner":properties.owner.0,"instances":instances,
                }))
            },
            Some(_) => Some(serde_json::json!({"error":"actual Properties slot payload differs"})),
            None => None,
        };
        records.push(serde_json::json!({
            "gtid":identity.id.raw(),
            "object_key":identity.value.raw(),
            "actual_entity_address":format!("{:p}",entity),
            "actual_binding_address":entity.bindings.as_ref().map(|binding|format!("{:p}",Arc::as_ptr(binding))),
            "actual_denizen_source_guards":entity.bindings.as_ref()
                .and_then(|binding|binding.source_ground.denizen_guards.as_ref()),
            "actual_runtime_address":entity.runtime.as_ref().map(|runtime|format!("{:p}",runtime)),
            "actual_actor_address":entity.runtime.as_ref().and_then(|runtime|runtime.actor.as_ref()).map(|actor|format!("{:p}",actor)),
            "new_phase":format!("{phase:?}"),
            "new_failure":failure,
            "actual_creation_record_debug":format!("{creation:#?}"),
            "actual_runtime_debug":format!("{:#?}",entity.runtime),
            "actual_property_instances":property_instances,
            "placement":placement,
        }));
    }
    if records.is_empty() {
        return None;
    }
    let queues = staged.queues.snapshot();
    let queued_targets: std::collections::BTreeMap<_, _> = queues
        .iter()
        .flat_map(|queue| &queue.buckets)
        .flatten()
        .map(|(owner, _)| (*owner, format!("{:?}", staged.owners.target(*owner))))
        .collect();
    let presentation = staged.load_world.as_ref().map(|world| {
        let owner = world.presentation_messages();
        let pending: Vec<_> = owner.pending().map(|command| command.as_ref().map(|command| {
            //Only constructor words exposed by the actual command are read.
            //Typed source/target identities stay separate from raw words.
            let words: std::collections::BTreeMap<_, _> = (0..=0x54u32).step_by(4)
                .filter_map(|offset| command.word(offset).map(|value|(format!("0x{offset:02x}"),value)))
                .collect();
            serde_json::json!({
                "source":command.source().map(aa_actors::ActorId::raw),
                "target":command.target().map(aa_actors::ActorId::raw),
                "kingdom":command.kingdom().map(aa_actors::ActorId::raw),
                "root_count":command.roots().len(),
                "graph_owner":command.graph().owner(),
                "graph_kind":format!("{:?}",command.graph().kind()),
                "words":words,
                "byte_58":command.byte(0x58),
            })
        })).collect();
        serde_json::json!({
            "scope":"read-only SAME World presentation manager; pending null commands remain null",
            "muted":owner.muted(),"attached_view":owner.attached_view(),
            "pending_commands":pending,
        })
    });
    Some(serde_json::json!({
        "scope":"read-only registered Source::New allocations in the SAME failed Mission prefix, after actual queued map preparation; snapshot is not an Init entry hook",
        "limitations":[
            "original mutable replacement Params are not retained in full; creation record is the actual constructed record",
            "unregistered factory allocations are not exposed by the public ActorStore iterator and are unmeasured",
            "addresses describe only this retained diagnostic owner lifetime and do not certify native pointer identity",
            "no new EconomyLimitedResourceMax UVar read or unavailable component field is synthesized",
            "scheduler target resolutions cover only the owners present in the actual queue snapshot",
        ],
        "actual_staged_address":format!("{:p}",staged),
        "actual_actor_store_address":format!("{:p}",&staged.actors),
        "actual_bank_address":format!("{:p}",&staged.definitions),
        "actual_world_address":staged.load_world.as_ref().map(|world|format!("{:p}",world)),
        "actual_world_revision":staged.load_world.as_ref().map(|world|world.revision().value()),
        "actual_presentation_messages":presentation,
        "actual_inactive_observer_active":staged.inactive_load.as_ref()
            .map(|load|load.observer_active()),
        "source_precision":staged.definitions.combat_source().map(|source|format!("{:?}",source.source_precision())),
        "requested_worker_precision":53,
        "clock":staged.clock,
        "rng":staged.rng,
        "queues_snapshot":queues,
        "queued_scheduler_targets":queued_targets,
        "mind_arena":staged.actors.minds,
        "actual_kingdom_economy_debug":format!("{:#?}",staged.kingdom_economy),
        "restore_report":staged.report,
        "records":records,
    }))
}
#[test]
#[ignore = "requires original AA_GAME_ROOT assets, accepted v14 source receipt, real Windows and every reached native startup service"]
fn actual_local_firefight_host_launch_and_same_resource_task_reach_started_mission() {
    let app_thread = Arc::new(
        aa_runtime::execution_context::ApplicationThread::capture(
            &mut windows_reset_os::WindowsThreadIds,
        )
        .unwrap(),
    );
    //Retained App+150 identity. The native recurring Net/Sim loop is separate;
    //future live World initialization/jobs must use this SAME thread/state queue.
    let mut simulation = aa_runtime::simulation_execution::SimulationThread::start(
        Arc::clone(&app_thread),
        windows_reset_os::WindowsThreadIds,
        |_| Ok(aa_runtime::application_state::State::from_constructor()),
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
    let counter = services.header_constructor_counter().unwrap();
    assert_eq!(counter.value(), 1);
    //Actual UI execution role. Definition-time provenance remains the
    //verified source receipt, and the resource worker runs at PC53 below.
    let ui_precision = Precision::Single24;
    let mut startup =
        match Startup::construct(&catalog, &variables, Arc::clone(&selected), ui_precision) {
            Ok(v) => v,
            Err((_, e)) => panic!("{e}"),
        };
    let selected_allocation = startup.host().selected_load_creator().unwrap() as *const _ as usize;
    //The replacement frontend's explicit selected-map confirmation borrows
    //the SAME retained App+10C owner on its actual thread. Native staging
    //menu navigation is outside this bounded startup test, as in the factory.
    let confirmation=simulation.run(|state,_|
        local_bootstrap::selected_map_confirm::OrdinaryEntry::capture_current_application(state)
            .map_err(|error|error.to_string())).unwrap();
    let host_input=confirmation.into_host_input(
        &catalog,&catalog.definition_bank.borrow(),services.application(),&selected,
        aa_netcode::lobby::GameOptions {
            name:"Native local skirmish".encode_utf16().collect(),password:vec![],
            max_players:-1,field_d8:u32::MAX,allow_observers:true,
        },
    ).unwrap();
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
    record(
        "main-launch",
        &serde_json::json!({"map":map,"ui_precision":24,"worker_precision":53,"error":launch.as_ref().err(),"scope":"actual reusable native Host/main-launch facade; no worker/battle activation","session":startup.host().snapshot(),"actual_notifications":startup.notifications().diagnostic_snapshot().map_err(|error|error.to_string()),"calls":startup.diagnostics(),"header_counter":counter.value(),"main_thread":app_thread.id(),"simulation_thread":simulation.identity().os_thread_id().unwrap()}),
    );
    launch.unwrap();
    use aa_runtime::load_world_globals::SessionPhase;
    assert_eq!(startup.host().application_state().unwrap(), 2);
    //77A750 takes field18 when the real confirmation LaunchState.kind is6.
    assert_eq!(startup.host().current_phase().unwrap(), 4);
    assert_eq!(
        startup.host().selected_load_creator().unwrap() as *const _ as usize,
        selected_allocation
    );
    assert_eq!(startup.host().task_status_14().unwrap(), Some(0));
    assert!(Arc::ptr_eq(
        startup
            .host()
            .selected_load_creator()
            .unwrap()
            .source_resource()
            .unwrap(),
        &selected
    ));
    let parts = services.parts(&catalog).unwrap();
    let transfer = match catalog.into_source_transfer(Arc::clone(&actors)) {
        Ok(v) => v,
        Err((e, _)) => panic!("{e}"),
    };
    let result = startup.run_queued_resource_task(
        transfer,
        lifecycle,
        parts.depot,
        parts.application,
        parts.startup,
        parts.orientation,
        resources,
        app_thread,
        simulation.identity(),
        || windows_reset_os::WindowsThreadIds,
        runtime_fp::sample,
        Precision::Double53,
    );
    if let Err(failure) = &result {
        if let Some(WorkerPrefix::Mission(mission)) = failure.prefix() {
            if let Some(snapshot) = fresh_actor_prefix(mission.staged()) {
                record("fresh-actor-prefix", &snapshot);
            }
        }
        if let Some(WorkerPrefix::Immediate(attempt)) = failure.prefix() {
            if let Some(event) = attempt.report.lifecycle.iter().rev().find(|e|e.attempted&&!e.completed) {
                let actual = event.output_id.and_then(|id|attempt.registry.get(aa_actors::ActorId::from_raw(id)));
                record("failed-record-prefix", &serde_json::json!({
                    "scope":"read-only actual failed Immediate allocation; not a fresh Actor or reconstructed runtime",
                    "error":failure.error(),"lifecycle":event,
                    "actual_record_debug":actual.map(|entry|format!("{:#?}",entry.value)),
                    "queues_snapshot":attempt.queues.snapshot(),
                    "native_load_mode":attempt.native_load_mode,
                }));
            }
            //Actual held owner values only. This snapshot is diagnostic input
            //for reached native removals, never a reconstruction/default seed.
            if attempt.report.configuration_attempted && !attempt.report.configuration_completed {
                record(
                    "removal-owned",
                    &serde_json::json!({"scope":"actual same failed Attempt owners, read-only diagnostic", "result":
                    aa_runtime::configure_kingdom_removal::diagnostic_snapshot(attempt,aa_actors::ActorId::from_raw(1677))}),
                );
                let records: Vec<_> = attempt
                    .registry
                    .iter()
                    .map(|entry| {
                        serde_json::json!({
                            "id":entry.id.raw(),"actual_runtime_debug":format!("{:#?}",entry.value)
                        })
                    })
                    .collect();
                record(
                    "removal-prefix",
                    &serde_json::json!({"scope":"actual failed Immediate Attempt owners before strict test drop", "error":failure.error(),"records":records,
                    "kingdom_registry":attempt.load_world.as_ref().map(|world|world.kingdoms()),
                    "inactive_observer_debug":format!("{:#?}",attempt.inactive_load),
                    "queues_snapshot":attempt.queues.snapshot(),
                    "updatable_table_debug":format!("{:#?}",attempt.updatables),
                    "native_load_mode":attempt.native_load_mode,"session":startup.host().snapshot()}),
                );
            }
        }
    }
    let (error, prefix, restore) = match &result {
        Ok(task) => (
            None,
            None,
            Some(serde_json::to_value(&task.mission().staged().report).unwrap()),
        ),
        Err(f) => {
            let (prefix, restore) = match f.prefix() {
                Some(WorkerPrefix::Immediate(v)) => {
                    ("immediate", Some(serde_json::to_value(&v.report).unwrap()))
                }
                Some(WorkerPrefix::PostLoad(v)) => {
                    ("post-load", Some(serde_json::to_value(&v.report).unwrap()))
                }
                Some(WorkerPrefix::Tail(v)) => {
                    ("tail", Some(serde_json::to_value(&v.report).unwrap()))
                }
                Some(WorkerPrefix::Resource(_)) => ("resource", None),
                Some(WorkerPrefix::Mission(v)) => (
                    "mission",
                    Some(serde_json::to_value(&v.staged().report).unwrap()),
                ),
                Some(WorkerPrefix::Source(_)) => ("source", None),
                Some(WorkerPrefix::LowerLevelDidNotReturnGraph(s)) => (*s, None),
                None => ("no-returned-prefix", None),
            };
            (Some(f.error()), Some(prefix), restore)
        }
    };
    record(
        "worker",
        &serde_json::json!({"map":map,"ui_precision":24,"worker_precision":53,"error":error,"retained_prefix":prefix,"restore_report":restore,"threads":startup.worker_threads(),"scope":"actual same queued task and source Bank; no application activation"}),
    );
    let task = result.unwrap();
    task.validate_owner(startup.host()).unwrap();
    assert!(task.mission().staged().load_world.is_some());
    assert!(!startup.host().task_is_null_124());
}
