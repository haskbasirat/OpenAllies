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
    let simulation = aa_runtime::simulation_execution::SimulationThread::start(
        Arc::clone(&app_thread),
        windows_reset_os::WindowsThreadIds,
        |_| Ok(()),
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
    let mut startup = match Startup::construct(
        &catalog,
        &variables,
        Arc::clone(&selected),
        ui_precision,
    ) {
        Ok(v) => v,
        Err((_, e)) => panic!("{e}"),
    };
    let selected_allocation = startup.host().selected_load_creator().unwrap() as *const _ as usize;
    let mut state = local_bootstrap::constructor_launch_state();
    state.kind = 12;
    let choices = Choices {
        host: local_bootstrap::launch::HostInput::from_ui_choices(
            state,
            2,
            2,
            false,
            aa_netcode::lobby::GameOptions {
                name: "Native local skirmish".encode_utf16().collect(),
                password: vec![],
                max_players: -1,
                field_d8: u32::MAX,
                allow_observers: true,
            },
            Some(selected.ids().to_vec()),
        ),
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
        &serde_json::json!({"map":map,"ui_precision":24,"worker_precision":53,"error":launch.as_ref().err(),"scope":"actual reusable native Host/main-launch facade; no worker/battle activation","session":startup.host().snapshot(),"calls":startup.diagnostics(),"header_counter":counter.value(),"main_thread":app_thread.id(),"simulation_thread":simulation.identity().os_thread_id().unwrap()}),
    );
    launch.unwrap();
    use aa_runtime::load_world_globals::SessionPhase;
    assert_eq!(startup.host().application_state().unwrap(), 2);
    assert_eq!(startup.host().current_phase().unwrap(), 12);
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
        resources,
        app_thread,
        simulation.identity(),
        || windows_reset_os::WindowsThreadIds,
        runtime_fp::sample,
        Precision::Double53,
    );
    if let Err(failure) = &result {
        if let Some(WorkerPrefix::Immediate(attempt)) = failure.prefix() {
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
                Some(WorkerPrefix::Mission(_)) => ("mission", None),
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
