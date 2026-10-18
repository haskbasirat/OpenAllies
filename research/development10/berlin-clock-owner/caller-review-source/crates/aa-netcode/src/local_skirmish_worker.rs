//!Shared actual ASYNC task body and its owned nonblocking transport.
//!The GUI creates/registers the real ResourceWorker before release. Only Send
//!preactivation owners move; Host launch remains on its application thread.
use super::*;
use aa_runtime::{
    execution_context::{LoadExecution, ResourceWorker},
    load_mission_start_world::StartedMission,
};

pub(super) struct Prepared {
    streams: aa_runtime::map_resource_streams::Owner,
    counter: aa_runtime::film_process_metadata::HeaderConstructorCounter,
    graph_source: aa_runtime::resource_depot::SourceGraphsRead,
    inactive: aa_runtime::restore_lifecycle::InactiveLoad,
    source: Arc<aa_rules::actors::ActorCatalog>,
    data: aa_runtime::catalog::SourceData,
}
pub(super) fn simple(error: String) -> WorkerFailure {
    WorkerFailure {
        error,
        prefix: None,
    }
}
pub(super) fn prepare(
    startup: &mut Startup,
    transfer: &mut Option<SourceTransfer>,
    lifecycle: ScopedManagedLoad,
    depot: &mut aa_runtime::resource_depot::Application,
) -> Result<(Prepared, Arc<local_bootstrap::load_task::Job>), WorkerFailure> {
    let streams = depot.open_map_streams(&startup.selected).map_err(simple)?;
    let counter = depot
        .process_metadata()
        .map_err(simple)?
        .constructor_counter();
    let graph_source = depot.source_graphs();
    let inactive = lifecycle
        .begin(aa_runtime::restore_lifecycle::TaskKind::Normal)
        .map_err(simple)?;
    let job = Arc::new(
        startup
            .host
            .take_queued_load_job()
            .map_err(|e| simple(e.to_string()))?,
    );
    let (source, data) = transfer
        .take()
        .ok_or_else(|| simple("load source transfer already consumed".into()))?
        .into_parts();
    Ok((
        Prepared {
            streams,
            counter,
            graph_source,
            inactive,
            source,
            data,
        },
        job,
    ))
}
#[allow(clippy::too_many_arguments)]
pub(super) fn run<O: OsThreadIds>(
    startup: &mut Startup,
    prepared: Prepared,
    depot: &mut aa_runtime::resource_depot::Application,
    application: &aa_runtime::application_mode::GameAppState,
    startup_inputs: &aa_runtime::actor_creation_startup::StartupInputs,
    orientation: Option<&aa_runtime::view_renderer_variables::State>,
    resources: Arc<ResourceCatalog>,
    simulation: SimulationIdentity,
    os: fn() -> O,
    environment: aa_runtime::load_performer_value_refresh::EnvironmentSampler,
    precision: Precision,
    execution: &mut LoadExecution<'_>,
    job: &local_bootstrap::load_task::Job,
    prefix: &Mutex<Option<WorkerPrefix>>,
) -> Result<StartedMission, String> {
    let Prepared {
        streams,
        counter,
        graph_source,
        inactive,
        source,
        data,
    } = prepared;
    let parts = startup.host.resource_worker_parts();
    let files = depot.sources_mut();
    let startup = startup_inputs;
    job.on_run_start(execution.identity())?;
    let catalog = match data.bind(&source) {
        Ok(v) => v,
        Err((e, v)) => {
            *prefix.lock().map_err(|_| "worker failure lock poisoned")? =
                Some(WorkerPrefix::Source(v));
            return Err(e);
        }
    };
    let selected = parts
        .selected_creator
        .ok_or("actual selected Creator loan missing")?;
    job.inputs()
        .validate_creator(&catalog, &catalog.definition_bank.borrow(), selected)?;
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
    let attempt = match aa_runtime::restoration::Attempt::load_selected_owned_runtime(
        streams,
        catalog,
        selected,
        precision,
        u64::MAX,
        aa_mapworld::restore::RestoreMode::StopOnError,
        aa_runtime::restoration::LoadBindings {
            native_load_mode: Some(job.inputs().kind().map_resource_load_mode()),
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
            *prefix.lock().map_err(|_| "worker failure lock poisoned")? = Some(
                WorkerPrefix::LowerLevelDidNotReturnGraph("load_selected_owned_runtime"),
            );
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
        *prefix.lock().map_err(|_| "worker failure lock poisoned")? =
            Some(WorkerPrefix::Immediate(Box::new(attempt)));
        return Err(e);
    }
    let mut staged = match attempt.into_staged() {
        Ok(v) => v,
        Err(e) => {
            *prefix.lock().map_err(|_| "worker failure lock poisoned")? =
                Some(WorkerPrefix::LowerLevelDidNotReturnGraph("into_staged"));
            return Err(e.to_string());
        }
    };
    if let Err(e) = staged.post_load(&source, precision) {
        *prefix.lock().map_err(|_| "worker failure lock poisoned")? =
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
            *prefix.lock().map_err(|_| "worker failure lock poisoned")? =
                Some(WorkerPrefix::PostLoad(Box::new(staged)));
            return Err("mission actual Resource source owner absent".into());
        }
    };
    if let Err(error) = read.bind_resource_catalog(resources) {
        *prefix.lock().map_err(|_| "worker failure lock poisoned")? =
            Some(WorkerPrefix::PostLoad(Box::new(staged)));
        return Err(error);
    }
    if let Err(error) = read.bind_source_graphs(&graph_source) {
        *prefix.lock().map_err(|_| "worker failure lock poisoned")? =
            Some(WorkerPrefix::PostLoad(Box::new(staged)));
        return Err(error);
    }
    if let Some(orientation) = orientation {
        if let Err(error) = read.bind_renderer_orientation(orientation) {
            *prefix.lock().map_err(|_| "worker failure lock poisoned")? =
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
    if let Err(error) = read.bind_load_execution(execution) {
        *prefix.lock().map_err(|_| "worker failure lock poisoned")? =
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
            *prefix.lock().map_err(|_| "worker failure lock poisoned")? =
                Some(WorkerPrefix::Tail(Box::new(f.staged)));
            return Err(f.error);
        }
    };
    if let Err(e) = loaded.finish_resource_load(&read) {
        *prefix.lock().map_err(|_| "worker failure lock poisoned")? =
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
            *prefix.lock().map_err(|_| "worker failure lock poisoned")? =
                Some(WorkerPrefix::Mission(Box::new(loaded)));
            Err(e)
        }
    }
}

pub(super) fn accept(
    startup: &mut Startup,
    job: Arc<local_bootstrap::load_task::Job>,
    completed: aa_runtime::execution_context::CompletedWorker<StartedMission>,
    prefix: Option<WorkerPrefix>,
) -> Result<local_bootstrap::load_task::SuccessfulTask, WorkerFailure> {
    let error = completed.outcome().err().map(str::to_owned);
    startup.worker_threads = Some((
        completed.identity().parent_os_thread_id(),
        completed.identity().child_os_thread_id(),
    ));
    let job = Arc::try_unwrap(job)
        .map_err(|_| simple("completed worker still owns its job loan".into()))?;
    job.accept_completed(completed).map_err(simple)?;
    if let Some(error) = error {
        return Err(WorkerFailure { error, prefix });
    }
    startup
        .host
        .begin_finished_task_callbacks()
        .map_err(|e| simple(e.to_string()))
}

struct Owners {
    startup: Startup,
    services: Application,
    prepared: Option<Prepared>,
}
///A failed launch still owns the original Host/Application. It is not a new
///empty attempt and cannot certify that its ASYNC work returned successfully.
pub struct OwnedStartFailure {
    pub error: String,
    owners: Owners,
    _source: Option<SourceTransfer>,
    _lifecycle: Option<ScopedManagedLoad>,
}
impl std::fmt::Debug for OwnedStartFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedStartFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}
///Only preactivation Send owners cross to the actual ASYNC thread. The GUI
///retains this move-only transport, polls completion and accepts SAME task124.
pub struct PendingResourceTask {
    owners: Option<Arc<Mutex<Owners>>>,
    prefix: Arc<Mutex<Option<WorkerPrefix>>>,
    job: Option<Arc<local_bootstrap::load_task::Job>>,
    worker: Option<ResourceWorker<StartedMission>>,
}
impl PendingResourceTask {
    #[allow(clippy::too_many_arguments)]
    pub fn start<O: OsThreadIds + Send + 'static>(
        mut startup: Startup,
        transfer: SourceTransfer,
        lifecycle: ScopedManagedLoad,
        mut services: Application,
        resources: Arc<ResourceCatalog>,
        app_thread: Arc<ApplicationThread>,
        simulation: SimulationIdentity,
        os: fn() -> O,
        environment: aa_runtime::load_performer_value_refresh::EnvironmentSampler,
        precision: Precision,
    ) -> Result<Self, OwnedStartFailure> {
        if let Err(error) = simulation.require_application(&app_thread) {
            return Err(OwnedStartFailure {
                error,
                owners: Owners {
                    startup,
                    services,
                    prepared: None,
                },
                _source: Some(transfer),
                _lifecycle: Some(lifecycle),
            });
        }
        let current = match os().current_thread_id() {
            Ok(value) => value,
            Err(error) => {
                return Err(OwnedStartFailure {
                    error,
                    owners: Owners {
                        startup,
                        services,
                        prepared: None,
                    },
                    _source: Some(transfer),
                    _lifecycle: Some(lifecycle),
                });
            }
        };
        if current != app_thread.id() {
            return Err(OwnedStartFailure {
                error: "owned load startup must run on its retained application thread".into(),
                owners: Owners {
                    startup,
                    services,
                    prepared: None,
                },
                _source: Some(transfer),
                _lifecycle: Some(lifecycle),
            });
        }
        let mut transfer = Some(transfer);
        let prepared = match prepare(&mut startup, &mut transfer, lifecycle, services.depot_mut()) {
            Ok(value) => value,
            Err(error) => {
                return Err(OwnedStartFailure {
                    error: error.error,
                    owners: Owners {
                        startup,
                        services,
                        prepared: None,
                    },
                    _source: transfer,
                    _lifecycle: None,
                });
            }
        };
        let (prepared, job) = prepared;
        let owners = Arc::new(Mutex::new(Owners {
            startup,
            services,
            prepared: Some(prepared),
        }));
        let prefix = Arc::new(Mutex::new(None));
        let child_owners = Arc::clone(&owners);
        let child_job = Arc::clone(&job);
        let child_prefix = Arc::clone(&prefix);
        let worker = ResourceWorker::start(
            app_thread,
            os(),
            |id| job.on_created(id),
            move |execution| {
                let mut guard = child_owners
                    .lock()
                    .map_err(|_| "owned load owners lock poisoned")?;
                let Owners {
                    startup,
                    services,
                    prepared,
                } = &mut *guard;
                let prepared = prepared.take().ok_or("owned load body already consumed")?;
                //Read-only source view exists only inside this actual worker loan.
                let source = Arc::clone(&prepared.source);
                let read = Catalog::new(&source);
                let parts = services.parts(&read)?;
                run(
                    startup,
                    prepared,
                    parts.depot,
                    parts.application,
                    parts.startup,
                    parts.orientation,
                    resources,
                    simulation,
                    os,
                    environment,
                    precision,
                    execution,
                    &child_job,
                    &child_prefix,
                )
            },
        );
        match worker {
            Ok(worker) => Ok(Self {
                owners: Some(owners),
                prefix,
                job: Some(job),
                worker: Some(worker),
            }),
            Err(error) => {
                let owners = Arc::try_unwrap(owners)
                    .ok()
                    .expect("failed worker joined before returning")
                    .into_inner()
                    .unwrap_or_else(|e| e.into_inner());
                Err(OwnedStartFailure {
                    error,
                    owners,
                    _source: None,
                    _lifecycle: None,
                })
            }
        }
    }
    ///Nonblocking; join happens only after the same OS handle is finished.
    ///The original partial failure graph is returned beside the same owners.
    pub fn poll(
        &mut self,
    ) -> Result<
        Option<(
            Startup,
            Application,
            Result<local_bootstrap::load_task::SuccessfulTask, WorkerFailure>,
        )>,
        String,
    > {
        let worker = self
            .worker
            .as_ref()
            .ok_or("resource task outcome was already consumed")?;
        if !worker.is_finished() {
            return Ok(None);
        }
        let completed = self.worker.take().unwrap().join_complete();
        let mut owners = self
            .owners
            .as_ref()
            .ok_or("resource owners already consumed")?
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let prefix = self.prefix.lock().unwrap_or_else(|e| e.into_inner()).take();
        let result = accept(
            &mut owners.startup,
            self.job.take().unwrap(),
            completed,
            prefix,
        );
        //Both Arc captures have expired after join. Move each owner once out of
        //the SAME allocation; no empty replacement Host/Application is made.
        drop(owners);
        let retained = self.owners.take().expect("same retained owner");
        let Owners {
            startup, services, ..
        } = Arc::try_unwrap(retained)
            .ok()
            .expect("joined worker released its owner loan")
            .into_inner()
            .unwrap_or_else(|e| e.into_inner());
        Ok(Some((startup, services, result)))
    }
}
