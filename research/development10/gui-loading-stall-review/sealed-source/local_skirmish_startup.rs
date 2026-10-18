//! Concrete selected-map local Host and its SAME queued resource job.
//! The returned task is not activation:7757F0/7738B0 remain subsequent real
//! callbacks. Required service failures stay failures and retain returned
//! partial graphs; no diagnostic viewer/error path becomes a battle.
use crate::{
    Error,
    local_bootstrap::{self, InitialProperties, LocalBootstrap, Notifications, ResetOs},
    local_host::{MillisecondTimer, NodeTime},
};
use aa_numeric::Precision;
use aa_rules::resources::ResourceCatalog;
use aa_runtime::{
    catalog::{Catalog, SourceTransfer},
    execution_context::{ApplicationThread, OsThreadIds, ScopedResourceWorker},
    localization::Localization,
    resource_creator::SourceResource,
    restore_lifecycle::ScopedManagedLoad,
    simulation_execution::SimulationIdentity,
    world_ready_services::Application,
};
use std::sync::{Arc, Mutex};

///User choices, not constructor defaults or source-derived game rules.
pub struct Choices {
    pub host: local_bootstrap::launch::HostInput,
    pub player_name: Vec<u16>,
    pub seed: u32,
}
///The real owners survive a preparation/launch refusal in this same object.
pub struct Startup {
    host: LocalBootstrap,
    notifications: Notifications,
    selected: Arc<SourceResource>,
    diagnostics: Vec<u32>,
    worker_threads: Option<(u32, u32)>,
}
impl Startup {
    ///Bind actual constructors once; selected resource and application remain
    ///the caller's real shared allocations. No launched/ready flag is minted.
    pub fn construct(
        catalog: &Catalog<'_>,
        variables: &aa_runtime::creator_variables::State,
        selected: Arc<SourceResource>,
        precision: Precision,
    ) -> Result<Self, (Self, String)> {
        let mut this = Self {
            host: LocalBootstrap::from_native_constructors(),
            notifications: Notifications::from_native_interactive_post_init(),
            selected,
            diagnostics: Vec::new(),
            worker_threads: None,
        };
        let result = (|| {
            this.host
                .sim_mut()
                .bind_constructor_rate(catalog, precision)?;
            this.host
                .bind_load_creator_constructor(catalog, variables)
                .map_err(|e| e.to_string())
        })();
        match result {
            Ok(()) => Ok(this),
            Err(e) => Err((this, e)),
        }
    }
    pub fn host(&self) -> &LocalBootstrap {
        &self.host
    }
    pub fn notifications(&self) -> &Notifications {
        &self.notifications
    }
    pub fn diagnostics(&self) -> &[u32] {
        &self.diagnostics
    }
    pub fn worker_threads(&self) -> Option<(u32, u32)> {
        self.worker_threads
    }
    pub fn into_host_and_notifications(self) -> (LocalBootstrap, Notifications) {
        (self.host, self.notifications)
    }
    ///Native Host constructor/reset/self-host/settings/refresh/finalize/77A950
    ///in order. Real platform timer/entropy and same application owners only.
    #[allow(clippy::too_many_arguments)]
    pub fn host_and_launch<O: OsThreadIds>(
        &mut self,
        catalog: &Catalog<'_>,
        text: &Localization,
        choices: Choices,
        reset_os: &mut impl ResetOs,
        time: &mut NodeTime<impl MillisecondTimer>,
        lifecycle: &mut ScopedManagedLoad,
        services: &mut Application,
        simulation: &SimulationIdentity,
        os: fn() -> O,
        precision: Precision,
    ) -> Result<(), String> {
        self.host_and_launch_with_text(catalog,Some(text),choices,reset_os,time,lifecycle,services,simulation,os,precision)
    }
    ///Actual current TextManager loans end at each reached text operation;
    ///source47EA40 later mutates that same owner without a detached clone.
    #[allow(clippy::too_many_arguments)]
    pub fn host_and_launch_current_text<O: OsThreadIds>(
        &mut self,
        catalog: &Catalog<'_>,
        choices: Choices,
        reset_os: &mut impl ResetOs,
        time: &mut NodeTime<impl MillisecondTimer>,
        lifecycle: &mut ScopedManagedLoad,
        services: &mut Application,
        simulation: &SimulationIdentity,
        os: fn() -> O,
        precision: Precision,
    ) -> Result<(), String> {
        self.host_and_launch_with_text(catalog,None,choices,reset_os,time,lifecycle,services,simulation,os,precision)
    }
    #[allow(clippy::too_many_arguments)]
    fn host_and_launch_with_text<O: OsThreadIds>(
        &mut self,
        catalog: &Catalog<'_>,
        text: Option<&Localization>,
        choices: Choices,
        reset_os: &mut impl ResetOs,
        time: &mut NodeTime<impl MillisecondTimer>,
        lifecycle: &mut ScopedManagedLoad,
        services: &mut Application,
        simulation: &SimulationIdentity,
        os: fn() -> O,
        precision: Precision,
    ) -> Result<(), String> {
        self.host
            .reset_node_fresh_with_host_input(
                choices.host,
                reset_os,
                &InitialProperties::from_native_constructor(),
                &self.notifications,
            )
            .map_err(|e| e.to_string())?;
        local_bootstrap::with_launch_text(catalog,text,|text|self.host
            .create_first_local_player_with_game_spy(
                &choices.player_name,
                &[],
                text,
                catalog,
                &catalog.definition_bank.borrow(),
                lifecycle.game_spy_manager_gate().map_err(Error::Protocol)?,
            )
        ).map_err(|e| e.to_string())?;
        self.host
            .finish_fresh_self_host(catalog, time, precision)
            .map_err(|e| e.to_string())?;
        let mut effects = Effects {
            catalog,
            simulation,
            os,
            calls: &mut self.diagnostics,
            sai_flags: None,
        };
        self.host
            .install_host_launch_fields(&mut effects)
            .map_err(|e| e.to_string())?;
        self.host
            .install_host_selected_file(&mut effects)
            .map_err(|e| e.to_string())?;
        self.host
            .install_host_selected_game_settings(catalog, &self.selected)
            .map_err(|e| e.to_string())?;
        self.host
            .refresh_host_players_7791f0(catalog, &mut effects)
            .map_err(|e| e.to_string())?;
        self.host
            .set_seed_7710e0(choices.seed, &mut effects)
            .map_err(|e| e.to_string())?;
        drop(effects);
        let parts = services.parts(catalog)?;
        lifecycle.with_source_profiles_game_spy_and_sync_flags(
            catalog,
            &catalog.definition_bank.borrow(),
            |profiles, manager, sai_flags| {
                let mut effects = Effects {
                    catalog,
                    simulation,
                    os,
                    calls: &mut self.diagnostics,
                    sai_flags: Some(sai_flags),
                };
                self.host
                    .launch_ordinary_host_with_text_77a950(
                        catalog,
                        &catalog.definition_bank.borrow(),
                        parts.depot,
                        parts.application,
                        manager,
                        text,
                        profiles,
                        &self.notifications,
                        &mut effects,
                    )
                    .map_err(|e| e.to_string())
            },
        )
    }
}

struct Effects<'a, 'source, O> {
    catalog: &'a Catalog<'source>,
    simulation: &'a SimulationIdentity,
    os: fn() -> O,
    calls: &'a mut Vec<u32>,
    sai_flags: Option<&'a [u8; 3]>,
}
impl<O: OsThreadIds> Effects<'_, '_, O> {
    fn gate(&self) -> Result<bool, String> {
        self.simulation.sync_gate(&mut (self.os)(), || {
            let flags = self
                .sai_flags
                .ok_or("actual same SAI flags are required on the simulation thread")?;
            Ok(Some([flags[0], flags[1]]))
        })
    }
}
impl<O: OsThreadIds> local_bootstrap::launch::Effects for Effects<'_, '_, O> {
    fn mode_rejected_counting_down(&mut self) -> crate::Result<()> {
        self.calls.push(0x771160);
        Ok(())
    }
    fn set_new_state_diagnostic(&mut self, _: u32) -> crate::Result<()> {
        self.calls.push(0x7745c0);
        Ok(())
    }
}
impl<O: OsThreadIds> local_bootstrap::assignment::Effects for Effects<'_, '_, O> {
    fn assigning(
        &mut self,
        _: u32,
        _: &[u16],
        _: &[u16],
        _: &[u16],
        _: &[u16],
    ) -> crate::Result<()> {
        self.calls.push(0x778d10);
        Ok(())
    }
    fn check_nation(&mut self, _: &[u16], _: &[u16]) -> crate::Result<()> {
        self.calls.push(0x777cd0);
        Ok(())
    }
    fn check_faction(&mut self, _: &[u16], _: &[u16]) -> crate::Result<()> {
        self.calls.push(0x777610);
        Ok(())
    }
}
impl<O: OsThreadIds> local_bootstrap::implicit_ai::Effects for Effects<'_, '_, O> {
    fn no_profiles_404ed0(&mut self) -> crate::Result<()> {
        Err(Error::Protocol(
            "native implicit-AI profile assertion reached".into(),
        ))
    }
    fn creating_player(&mut self, _: u32, _: bool, _: bool) -> crate::Result<()> {
        self.calls.push(0x773710);
        Ok(())
    }
    fn kingdom_diagnostic(&mut self, _: usize, _: &[u16]) -> crate::Result<()> {
        self.calls.push(0x776580);
        Ok(())
    }
    fn player_diagnostic(&mut self, _: &[u16], _: &[u16], _: bool) -> crate::Result<()> {
        self.calls.push(0x77684f);
        Ok(())
    }
}
impl<O: OsThreadIds> local_bootstrap::finalize::Effects for Effects<'_, '_, O> {
    fn finalize_diagnostic(&mut self, _: &[u16]) -> crate::Result<()> {
        self.calls.push(0x776b20);
        Ok(())
    }
}
impl<O: OsThreadIds> local_bootstrap::load_task::Effects for Effects<'_, '_, O> {
    fn checking_old_task(&mut self) -> crate::Result<()> {
        self.calls.push(0x775ee0);
        Ok(())
    }
    fn launching_map_type(&mut self, _: u32) -> crate::Result<()> {
        self.calls.push(0x775f6a);
        Ok(())
    }
    fn creating_load_game_task(&mut self) -> crate::Result<()> {
        self.calls.push(0x76be80);
        Ok(())
    }
    fn begin_task_diagnostic(&mut self, _: bool) -> crate::Result<()> {
        self.calls.push(0x74daf0);
        Ok(())
    }
}
impl<O: OsThreadIds> local_bootstrap::launch_live::Effects for Effects<'_, '_, O> {
    fn dump_launch_settings(
        &mut self,
        owner: &LocalBootstrap,
        label: &'static str,
    ) -> crate::Result<()> {
        self.calls.push(0x76d6c0);
        owner.dump_actual_launch_settings(self.catalog, label, &mut || self.gate())
    }
    fn sync_log_gate(&mut self) -> crate::Result<bool> {
        self.gate().map_err(Error::Protocol)
    }
    fn reset_order_version_diagnostic(&mut self) -> crate::Result<()> {
        self.calls.push(0x74dbc0);
        Ok(())
    }
}

///Returned partial owners at an explicit reached stop. Bare lower-level
///loader/staging errors presently do not return their consumed graph; this
///adapter records that limitation rather than substituting another graph.
pub enum WorkerPrefix {
    Immediate(Box<aa_runtime::restoration::Attempt>),
    PostLoad(Box<aa_runtime::restoration::Staged>),
    Tail(Box<aa_runtime::restoration::Staged>),
    Resource(Box<aa_runtime::load_mission_start_world::LoadedMap>),
    Mission(Box<aa_runtime::load_mission_start_world::LoadedMap>),
    Source(aa_runtime::catalog::SourceData),
    LowerLevelDidNotReturnGraph(&'static str),
}
pub struct WorkerFailure {
    error: String,
    prefix: Option<WorkerPrefix>,
}
impl WorkerFailure {
    pub fn error(&self) -> &str {
        &self.error
    }
    pub fn prefix(&self) -> Option<&WorkerPrefix> {
        self.prefix.as_ref()
    }
}
impl std::fmt::Debug for WorkerFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl Startup {
    ///Drain the exact native queued task after the completed main-thread
    ///stores, on a real scoped OS worker. The Host remains in self on error.
    #[allow(clippy::too_many_arguments)]
    pub fn run_queued_resource_task<O: OsThreadIds + Send + 'static>(
        &mut self,
        transfer: SourceTransfer,
        lifecycle: ScopedManagedLoad,
        depot: &mut aa_runtime::resource_depot::Application,
        application: &aa_runtime::application_mode::GameAppState,
        startup: &aa_runtime::actor_creation_startup::StartupInputs,
        orientation: Option<&aa_runtime::view_renderer_variables::State>,
        resources: Arc<ResourceCatalog>,
        app_thread: Arc<ApplicationThread>,
        simulation: SimulationIdentity,
        os: fn() -> O,
        environment: aa_runtime::load_performer_value_refresh::EnvironmentSampler,
        precision: Precision,
    ) -> Result<local_bootstrap::load_task::SuccessfulTask, WorkerFailure> {
        let simple = |error: String| WorkerFailure {
            error,
            prefix: None,
        };
        simulation
            .require_application(&app_thread)
            .map_err(simple)?;
        let streams = depot.open_map_streams(&self.selected).map_err(simple)?;
        let counter = depot
            .process_metadata()
            .map_err(simple)?
            .constructor_counter();
        //Same Application graph arena, with a weak application identity.
        //This owned read capability permits the separate mutable file loan.
        let graph_source = depot.source_graphs();
        let inactive = lifecycle
            .begin(aa_runtime::restore_lifecycle::TaskKind::Normal)
            .map_err(simple)?;
        let job = self
            .host
            .take_queued_load_job()
            .map_err(|e| simple(e.to_string()))?;
        let (source, data) = transfer.into_parts();
        let prefix = Mutex::new(None);
        let completed = std::thread::scope(|scope| {
            let parts = self.host.resource_worker_parts();
            let job_ref = &job;
            let failed = &prefix;
            let files = depot.sources_mut();
            let worker = ScopedResourceWorker::start(
                scope,
                app_thread,
                os(),
                |id| job_ref.on_created(id),
                move |execution| {
                    job_ref.on_run_start(execution.identity())?;
                    let catalog = match data.bind(&source) {
                        Ok(v) => v,
                        Err((e, v)) => {
                            *failed.lock().map_err(|_| "worker failure lock poisoned")? =
                                Some(WorkerPrefix::Source(v));
                            return Err(e);
                        }
                    };
                    let selected = parts
                        .selected_creator
                        .ok_or("actual selected Creator loan missing")?;
                    job_ref.inputs().validate_creator(
                        &catalog,
                        &catalog.definition_bank.borrow(),
                        selected,
                    )?;
                    let mut gate_a = || {
                        simulation.sync_gate(&mut os(), || {
                            Err("actual SAI loan required on simulation thread".into())
                        })
                    };
                    let mut gate_s = || {
                        simulation.sync_gate(&mut os(), || {
                            Err("actual SAI loan required on simulation thread".into())
                        })
                    };
                    use aa_runtime::live_clocked::{SyncBinding, SyncLog};
                    let attempt =
                        match aa_runtime::restoration::Attempt::load_selected_owned_runtime(
                            streams,
                            catalog,
                            selected,
                            precision,
                            u64::MAX,
                            aa_mapworld::restore::RestoreMode::StopOnError,
                            aa_runtime::restoration::LoadBindings {
                                native_load_mode: Some(
                                    job_ref.inputs().kind().map_resource_load_mode(),
                                ),
                                inactive_load: Some(inactive),
                                resources: Some(resources),
                            },
                            files,
                            &parts.session,
                            false,
                            execution,
                            environment,
                            parts.sim,
                            SyncBinding::Direct(SyncLog {
                                owner: parts.sync_a,
                                gate: &mut gate_a,
                            }),
                            SyncBinding::Direct(SyncLog {
                                owner: parts.sync_s,
                                gate: &mut gate_s,
                            }),
                            &counter,
                        ) {
                            Ok(v) => v,
                            Err(e) => {
                                *failed.lock().map_err(|_| "worker failure lock poisoned")? =
                                    Some(WorkerPrefix::LowerLevelDidNotReturnGraph(
                                        "load_selected_owned_runtime",
                                    ));
                                return Err(e.to_string());
                            }
                        };
                    if attempt.report.events.iter().any(|e| e.error.is_some())
                        || attempt
                            .report
                            .lifecycle
                            .iter()
                            .any(|e| !e.completed || e.omitted)
                    {
                        let e = attempt
                            .report
                            .events
                            .iter()
                            .find_map(|e| e.error.as_ref())
                            .cloned()
                            .or_else(|| {
                                attempt
                                    .report
                                    .lifecycle
                                    .iter()
                                    .find_map(|e| e.error.as_ref())
                                    .cloned()
                            })
                            .unwrap_or_else(|| "native immediate lifecycle stopped".into());
                        *failed.lock().map_err(|_| "worker failure lock poisoned")? =
                            Some(WorkerPrefix::Immediate(Box::new(attempt)));
                        return Err(e);
                    }
                    let mut staged = match attempt.into_staged() {
                        Ok(v) => v,
                        Err(e) => {
                            *failed.lock().map_err(|_| "worker failure lock poisoned")? =
                                Some(WorkerPrefix::LowerLevelDidNotReturnGraph("into_staged"));
                            return Err(e.to_string());
                        }
                    };
                    if let Err(e) = staged.post_load(&source, precision) {
                        *failed.lock().map_err(|_| "worker failure lock poisoned")? =
                            Some(WorkerPrefix::PostLoad(Box::new(staged)));
                        return Err(e.to_string());
                    }
                    let mut read = Catalog::new(&source);
                    //The immediate load moved this SAME immutable Resource
                    //source into Staged. The readonly mission Catalog must
                    //retain that allocation for reached Kingdom Economy reads.
                    let resources = match staged.resource_source.as_ref() {
                        Some(resources) => Arc::clone(resources),
                        None => {
                            *failed.lock().map_err(|_| "worker failure lock poisoned")? =
                                Some(WorkerPrefix::PostLoad(Box::new(staged)));
                            return Err("mission actual Resource source owner absent".into());
                        }
                    };
                    if let Err(error) = read.bind_resource_catalog(resources) {
                        *failed.lock().map_err(|_| "worker failure lock poisoned")? =
                            Some(WorkerPrefix::PostLoad(Box::new(staged)));
                        return Err(error);
                    }
                    if let Err(error) = read.bind_source_graphs(&graph_source) {
                        *failed.lock().map_err(|_| "worker failure lock poisoned")? =
                            Some(WorkerPrefix::PostLoad(Box::new(staged)));
                        return Err(error);
                    }
                    if let Some(orientation) = orientation {
                        if let Err(error) = read.bind_renderer_orientation(orientation) {
                            *failed.lock().map_err(|_| "worker failure lock poisoned")? =
                                Some(WorkerPrefix::PostLoad(Box::new(staged)));
                            return Err(error);
                        }
                    }
                    //Carry the actual load binding into this readonly source
                    //view; the authoritative mutable mode stays in SAME World.
                    read.native_load_mode = staged.native_load_mode;
                    //Fresh InitExisting reads the SAME Session lazily during
                    //map preparation, after the immediate-load scope ends.
                    read.bind_load_session(&parts.session);
                    //Retain the actual worker sampler; native refresh samples
                    //its current thread only if the arithmetic path reaches it.
                    read.bind_runtime_environment(environment);
                    if let Err(error)=read.bind_load_execution(execution) {
                        *failed.lock().map_err(|_|"worker failure lock poisoned")?=
                            Some(WorkerPrefix::PostLoad(Box::new(staged)));
                        return Err(error);
                    }
                    read.settings = Some(staged.settings.clone());
                    read.definition_text = Some(staged.definition_text.clone());
                    let mut loaded = match aa_runtime::load_mission_start_world::finish_load(
                        staged,
                        &read,
                        &parts.session,
                        precision,
                    ) {
                        Ok(v) => v,
                        Err(f) => {
                            *failed.lock().map_err(|_| "worker failure lock poisoned")? =
                                Some(WorkerPrefix::Tail(Box::new(f.staged)));
                            return Err(f.error);
                        }
                    };
                    if let Err(e) = loaded.finish_resource_load(&read) {
                        *failed.lock().map_err(|_| "worker failure lock poisoned")? =
                            Some(WorkerPrefix::Resource(Box::new(loaded)));
                        return Err(e);
                    }
                    match loaded.start_mission(
                        &read,
                        application,
                        startup,
                        &parts.session,
                        &mut SyncLog {
                            owner: parts.sync_a,
                            gate: &mut gate_a,
                        },
                        &mut SyncLog {
                            owner: parts.sync_s,
                            gate: &mut gate_s,
                        },
                        precision,
                    ) {
                        Ok(v) => Ok(v),
                        Err((loaded, e)) => {
                            *failed.lock().map_err(|_| "worker failure lock poisoned")? =
                                Some(WorkerPrefix::Mission(Box::new(loaded)));
                            Err(e)
                        }
                    }
                },
            )
            .map_err(simple)?;
            Ok::<_, WorkerFailure>(worker.join_complete())
        })?;
        let error = completed.outcome().err().map(str::to_owned);
        self.worker_threads = Some((
            completed.identity().parent_os_thread_id(),
            completed.identity().child_os_thread_id(),
        ));
        job.accept_completed(completed).map_err(simple)?;
        if let Some(error) = error {
            return Err(WorkerFailure {
                error,
                prefix: prefix
                    .into_inner()
                    .map_err(|_| simple("worker failure lock poisoned".into()))?,
            });
        }
        self.host
            .begin_finished_task_callbacks()
            .map_err(|e| simple(e.to_string()))
    }
}
