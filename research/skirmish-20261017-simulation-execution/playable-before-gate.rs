//! Strict actual local Host -> native main launch -> same queued resource job.
//! Required native gates fail this ignored acceptance test and are recorded;
//! no expected-error string or status flag can turn a stopped load into play.
#![cfg(windows)]
#[path = "../../aa-client/src/runtime_fp.rs"]
mod runtime_fp;
#[path = "../../aa-client/src/windows_reset_os.rs"]
mod windows_reset_os;
use aa_netcode::{
    local_bootstrap::{self, InitialProperties, LocalBootstrap, Notifications},
    local_host::NodeTime,
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

fn record(stage: &str, value: &serde_json::Value) {
    let base = std::env::var_os("AA_PLAYABLE_REPORT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../../research/development10/playable-startup/run-{}",
                std::process::id()
            ))
        });
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
struct Effects<'a, 'source> {
    catalog: &'a Catalog<'source>,
    calls: Vec<u32>,
}
impl local_bootstrap::launch::Effects for Effects<'_, '_> {
    fn mode_rejected_counting_down(&mut self) -> aa_netcode::Result<()> {
        Err(aa_netcode::Error::Protocol(
            "fresh local countdown unexpectedly positive".into(),
        ))
    }
    fn set_new_state_diagnostic(&mut self, _: u32) -> aa_netcode::Result<()> {
        self.calls.push(0x7745c0);
        Ok(())
    }
}
impl local_bootstrap::assignment::Effects for Effects<'_, '_> {
    fn assigning(
        &mut self,
        _: u32,
        _: &[u16],
        _: &[u16],
        _: &[u16],
        _: &[u16],
    ) -> aa_netcode::Result<()> {
        self.calls.push(0x778d10);
        Ok(())
    }
    fn check_nation(&mut self, _: &[u16], _: &[u16]) -> aa_netcode::Result<()> {
        self.calls.push(0x777cd0);
        Ok(())
    }
    fn check_faction(&mut self, _: &[u16], _: &[u16]) -> aa_netcode::Result<()> {
        self.calls.push(0x777610);
        Ok(())
    }
}
impl local_bootstrap::implicit_ai::Effects for Effects<'_, '_> {
    fn no_profiles_404ed0(&mut self) -> aa_netcode::Result<()> {
        self.calls.push(0x404ed0);
        Ok(())
    }
    fn creating_player(&mut self, _: u32, _: bool, _: bool) -> aa_netcode::Result<()> {
        self.calls.push(0x773710);
        Ok(())
    }
    fn kingdom_diagnostic(&mut self, _: usize, _: &[u16]) -> aa_netcode::Result<()> {
        self.calls.push(0x776580);
        Ok(())
    }
    fn player_diagnostic(&mut self, _: &[u16], _: &[u16], _: bool) -> aa_netcode::Result<()> {
        self.calls.push(0x77684f);
        Ok(())
    }
}
impl local_bootstrap::finalize::Effects for Effects<'_, '_> {
    fn finalize_diagnostic(&mut self, _: &[u16]) -> aa_netcode::Result<()> {
        self.calls.push(0x776b20);
        Ok(())
    }
}
impl local_bootstrap::load_task::Effects for Effects<'_, '_> {
    fn checking_old_task(&mut self) -> aa_netcode::Result<()> {
        self.calls.push(0x775ee0);
        Ok(())
    }
    fn launching_map_type(&mut self, _: u32) -> aa_netcode::Result<()> {
        self.calls.push(0x775f6a);
        Ok(())
    }
    fn creating_load_game_task(&mut self) -> aa_netcode::Result<()> {
        self.calls.push(0x76be80);
        Ok(())
    }
    fn begin_task_diagnostic(&mut self, _: bool) -> aa_netcode::Result<()> {
        self.calls.push(0x74daf0);
        Ok(())
    }
}
impl local_bootstrap::launch_live::Effects for Effects<'_, '_> {
    fn dump_launch_settings(
        &mut self,
        owner: &LocalBootstrap,
        label: &'static str,
    ) -> aa_netcode::Result<()> {
        self.calls.push(0x76d6c0);
        owner.dump_actual_launch_settings(self.catalog, label, &mut || {
            Err("actual thread/application sync gate is required if logging enabled".into())
        })
    }
    fn sync_log_gate(&mut self) -> aa_netcode::Result<bool> {
        Err(aa_netcode::Error::Unsupported(
            "actual thread/application sync gate is required if logging enabled",
        ))
    }
    fn reset_order_version_diagnostic(&mut self) -> aa_netcode::Result<()> {
        self.calls.push(0x74dbc0);
        Ok(())
    }
}

#[test]
#[ignore = "requires original AA_GAME_ROOT assets, accepted v14 receipt, real Windows and every reached native startup service"]
fn actual_local_firefight_host_launch_and_same_resource_task_reach_started_mission() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = Arc::new(ActorCatalog::load_v14(&mut sources).unwrap());
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(Settings::load_v14(&mut sources, Limits::default()).unwrap());
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    catalog.definition_text = Some(text.clone());
    *catalog.localization.borrow_mut() = Some(text.clone());
    let proof =
        aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
            &catalog,
            &mut sources,
        )
        .unwrap();
    aa_runtime::combat_source_completion::install(
        &catalog,
        &mut sources,
        proof.context(),
        proof.csv(),
        proof.globals(),
    )
    .unwrap();
    aa_runtime::combat_source_completion::compute(&catalog).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let process = aa_runtime::film_process_metadata::State::from_fresh_sources(
        &catalog,
        &mut sources,
        aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
    )
    .unwrap();
    let variables = aa_runtime::creator_variables::State::from_fresh_sources(
        &catalog,
        &mut sources,
        aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
    )
    .unwrap();
    let app_clock = windows_reset_os::PerformanceTimer::from_native_constructor().unwrap();
    let mut application = aa_runtime::restore_lifecycle::ScopedManagedLoad::from_sources(
        &mut sources,
        app_clock.milliseconds().unwrap(),
    )
    .unwrap();
    application
        .select_local_offline(
            &catalog,
            &mut sources,
            aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
            &mut windows_reset_os::WindowsStandaloneClock,
        )
        .unwrap();
    let game_app = aa_runtime::application_mode::GameAppState::from_native_constructor();
    let mut depot = aa_runtime::resource_depot::Application::from_sources(sources);
    if let Err((_, error)) = depot.attach_process_metadata(process) {
        panic!("{error}")
    }
    let map = std::env::var("AA_PLAYABLE_MAP")
        .unwrap_or_else(|_| "Maps/DeveloperMaps/(3v3)Firefight.RMP".into());
    let selected = depot
        .select_map(&catalog, &map.encode_utf16().collect::<Vec<_>>())
        .unwrap();
    let counter = depot.process_metadata().unwrap().constructor_counter();
    assert_eq!(counter.value(), 1);
    let mut owner = LocalBootstrap::from_native_constructors();
    owner
        .sim_mut()
        .bind_constructor_rate(&catalog, Precision::Double53)
        .unwrap();
    owner
        .bind_load_creator_constructor(&catalog, &variables)
        .unwrap();
    let selected_allocation = owner.selected_load_creator().unwrap() as *const _ as usize;
    let mut state = local_bootstrap::constructor_launch_state();
    state.kind = 12;
    let input = local_bootstrap::launch::HostInput::from_ui_choices(
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
    );
    let notifications = Notifications::new(0);
    let mut actual_gate = || {
        Err(aa_netcode::Error::Unsupported(
            "disabled fresh sync must not read unbound gate",
        ))
    };
    let mut os = windows_reset_os::WindowsResetOs::new(&mut actual_gate);
    owner
        .reset_node_fresh_with_host_input(
            input,
            &mut os,
            &InitialProperties::from_native_constructor(),
            &notifications,
        )
        .unwrap();
    owner
        .create_first_local_player_with_game_spy(
            &"Local human".encode_utf16().collect::<Vec<_>>(),
            &[],
            &text,
            &catalog,
            &catalog.definition_bank.borrow(),
            application.game_spy_manager_gate().unwrap(),
        )
        .unwrap();
    let mut node_time = NodeTime::from_native_constructor(
        windows_reset_os::PerformanceTimer::from_native_constructor().unwrap(),
    );
    node_time.begin_update_prefix(Precision::Double53).unwrap();
    owner
        .finish_fresh_self_host(&catalog, &mut node_time, Precision::Double53)
        .unwrap();
    let mut effects = Effects {
        catalog: &catalog,
        calls: Vec::new(),
    };
    owner.install_host_launch_fields(&mut effects).unwrap();
    owner.install_host_selected_file(&mut effects).unwrap();
    owner
        .install_host_selected_game_settings(&catalog, &selected)
        .unwrap();
    owner
        .refresh_host_players_7791f0(&catalog, &mut effects)
        .unwrap();
    owner.set_seed_7710e0(0, &mut effects).unwrap(); //Explicit frontend auto-seed choice; not a ctor default.
    let launch = application.with_source_profiles_and_game_spy(
        &catalog,
        &catalog.definition_bank.borrow(),
        |profiles, manager| {
            owner
                .launch_ordinary_host_77a950(
                    &catalog,
                    &catalog.definition_bank.borrow(),
                    &mut depot,
                    &game_app,
                    manager,
                    &text,
                    profiles,
                    &notifications,
                    &mut effects,
                )
                .map_err(|e| e.to_string())
        },
    );
    record(
        "main-launch",
        &serde_json::json!({"map":map,"error":launch.as_ref().err(),"scope":"actual native Host/main launch; no worker or battle activation", "session":owner.snapshot(),"calls":effects.calls,"header_counter":counter.value()}),
    );
    launch.unwrap();
    use aa_runtime::load_world_globals::SessionPhase;
    assert_eq!(owner.application_state().unwrap(), 2);
    assert_eq!(owner.current_phase().unwrap(), 12);
    assert_eq!(
        owner.selected_load_creator().unwrap() as *const _ as usize,
        selected_allocation
    );
    assert_eq!(owner.task_status_14().unwrap(), Some(0));
    assert!(Arc::ptr_eq(
        owner
            .selected_load_creator()
            .unwrap()
            .source_resource()
            .unwrap(),
        &selected
    ));
    drop(effects);
    let transfer = match catalog.into_source_transfer(Arc::clone(&actors)) {
        Ok(v) => v,
        Err((error, _)) => panic!("{error}"),
    };
    let (worker_actors, source_data) = transfer.into_parts();
    let inactive = application
        .begin(aa_runtime::restore_lifecycle::TaskKind::Normal)
        .unwrap();
    let streams = depot.open_map_streams(&selected).unwrap();
    let job = owner.take_queued_load_job().unwrap();
    let app_thread = Arc::new(
        aa_runtime::execution_context::ApplicationThread::capture(
            &mut windows_reset_os::WindowsThreadIds,
        )
        .unwrap(),
    );
    let worker_map=map.clone();
    let complete = std::thread::scope(|scope| {
        let parts = owner.resource_worker_parts();
        let job_ref = &job;
        let app_ref = &game_app;
        let source_reader = depot.sources_mut();
        let worker=aa_runtime::execution_context::ScopedResourceWorker::start(scope,app_thread,windows_reset_os::WindowsThreadIds,
            |id|job_ref.on_created(id),move|execution|{
                job_ref.on_run_start(execution.identity())?;
                let catalog=source_data.bind(&worker_actors).map_err(|(e,_)|e)?;
                let selected=parts.selected_creator.ok_or("actual selected Creator loan absent")?;
                job_ref.inputs().validate_creator(&catalog,&catalog.definition_bank.borrow(),selected)?;
                let mut gate_a=||Err("actual worker/application SyncA gate required if logger opens".into());
                let mut gate_s=||Err("actual worker/application SyncS gate required if logger opens".into());
                let attempt=aa_runtime::restoration::Attempt::load_selected_owned_runtime(streams,catalog,selected,Precision::Double53,u64::MAX,
                    aa_mapworld::restore::RestoreMode::StopOnError,aa_runtime::restoration::LoadBindings{native_load_mode:Some(job_ref.inputs().kind().map_resource_load_mode()),inactive_load:Some(inactive),resources:Some(resources)},
                    source_reader,&parts.session,false,execution,runtime_fp::sample,parts.sim,
                    aa_runtime::live_clocked::SyncBinding::Direct(aa_runtime::live_clocked::SyncLog{owner:parts.sync_a,gate:&mut gate_a}),
                    aa_runtime::live_clocked::SyncBinding::Direct(aa_runtime::live_clocked::SyncLog{owner:parts.sync_s,gate:&mut gate_s}),&counter).map_err(|e|e.to_string())?;
                record("immediate",&serde_json::json!({"map":worker_map,"report":attempt.report,"scope":"strict native immediate callbacks; stopped rows cannot satisfy this test"}));
                if attempt.report.events.iter().any(|e|e.error.is_some())||attempt.report.lifecycle.iter().any(|e|!e.completed||e.omitted){
                    return Err(format!("native immediate lifecycle stopped: {}",serde_json::to_string(&attempt.report).unwrap()));
                }
                let mut staged=attempt.into_staged().map_err(|e|e.to_string())?;
                staged.post_load(&worker_actors,Precision::Double53).map_err(|e|e.to_string())?;
                let mut read=Catalog::new(&worker_actors);read.settings=Some(staged.settings.clone());
                read.definition_text=Some(staged.definition_text.clone());
                let mut loaded=aa_runtime::load_mission_start_world::finish_load(staged,&read,&parts.session,Precision::Double53).map_err(|f|f.error)?;
                loaded.finish_resource_load(&read)?;
                loaded.start_mission(&read,app_ref,&parts.session,
                    &mut aa_runtime::live_clocked::SyncLog{owner:parts.sync_a,gate:&mut gate_a},
                    &mut aa_runtime::live_clocked::SyncLog{owner:parts.sync_s,gate:&mut gate_s},Precision::Double53).map_err(|(_,e)|e)
            }).unwrap();
        worker.join_complete()
    });
    let outcome = complete.outcome().err().map(str::to_owned);
    record(
        "worker",
        &serde_json::json!({"map":map,"error":outcome,"scope":"actual joined same resource job; no application activation", "parent_thread":complete.identity().parent_os_thread_id(),"child_thread":complete.identity().child_os_thread_id()}),
    );
    job.accept_completed(complete).unwrap();
    if let Some(error) = outcome {
        panic!("actual queued resource task blocked: {error}");
    }
    assert_eq!(owner.task_status_14().unwrap(), Some(2));
    let task = owner.begin_finished_task_callbacks().unwrap();
    task.validate_owner(&owner).unwrap();
    assert!(task.mission().staged().load_world.is_some());
    //7757F0/7738B0 and application activation remain separate required work.
    assert!(!owner.task_is_null_124());
}
