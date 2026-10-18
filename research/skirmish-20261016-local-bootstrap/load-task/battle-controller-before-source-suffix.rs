//! Client ownership of the real local-skirmish startup facade. No inspection
//!scene, test fixture, raw StartedMission, or successful loader alone can
//!enable game orders. Mutable native World stays on the one SimulationThread.
use aa_runtime::application_window_state::Reader;
use std::path::{Path, PathBuf};
#[cfg(windows)]
pub use windows::ActiveLoans;

#[derive(Clone, Debug)]
pub struct Request {
    pub map_id: String,
    pub source_path: String,
    pub player_name: String,
    pub game_name: String,
    pub seed: u32,
    pub observers: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Idle,
    Queued,
    Loading,
    Activating,
    Failed,
    AwaitingNativeView,
}
pub struct Controller {
    root: PathBuf,
    focus: Reader,
    request: Option<Request>,
    status: Status,
    message: String,
    #[cfg(windows)]
    engine: Option<windows::Engine>,
    #[cfg(windows)]
    failed: Option<windows::Failure>,
}
impl Controller {
    pub fn new(root: &Path, focus: Reader) -> Self {
        Self {
            root: root.to_owned(),
            focus,
            request: None,
            status: Status::Idle,
            message: String::new(),
            #[cfg(windows)]
            engine: None,
            #[cfg(windows)]
            failed: None,
        }
    }
    pub fn status(&self) -> &Status {
        &self.status
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn request(&self) -> Option<&Request> {
        self.request.as_ref()
    }
    ///Only the Send result returns to the UI. The callback receives short real
    ///owner loans on the same retained simulation OS thread; no Frame/World
    ///can escape through this API. Activation is not view/task readiness.
    #[cfg(windows)]
    pub fn with_activated<T: Send + 'static>(
        &mut self,
        call: impl FnOnce(
            ActiveLoans<'_>,
            &mut aa_runtime::simulation_execution::Execution<'_>,
        ) -> Result<T, String>
        + Send
        + 'static,
    ) -> Result<T, String> {
        if self.status != Status::AwaitingNativeView {
            return Err("actual7757/7738 activation has not completed".into());
        }
        self.engine
            .as_mut()
            .ok_or("no retained local simulation thread")?
            .with_activated(call)
    }
    #[cfg(windows)]
    pub fn submit_activated<T: Send + 'static>(
        &mut self,
        call: impl FnOnce(
            ActiveLoans<'_>,
            &mut aa_runtime::simulation_execution::Execution<'_>,
        ) -> Result<T, String>
        + Send
        + 'static,
    ) -> Result<aa_runtime::simulation_execution::JobTicket<T>, String> {
        if self.status != Status::AwaitingNativeView {
            return Err("actual7757/7738 activation has not completed".into());
        }
        self.engine
            .as_mut()
            .ok_or("no retained local simulation thread")?
            .submit_activated(call)
    }
    ///Keep UI event/backend work available while the same native activation
    ///operation is running on its retained child thread.
    pub fn poll(&mut self) {
        if self.status != Status::Activating {
            return;
        }
        #[cfg(windows)]
        {
            let Some(engine) = &mut self.engine else {
                return;
            };
            match engine.poll_activation() {
                Ok(None) => {}
                Ok(Some(None)) => {
                    self.status = Status::AwaitingNativeView;
                    self.message="Map lifecycle completed. Waiting for the native view/task activation before orders and simulation frames can run.".into();
                }
                Ok(Some(Some(error))) | Err(error) => {
                    self.status = Status::Failed;
                    self.message = format!("Native activation stopped: {error}");
                }
            }
        }
    }
    ///Explicit new-attempt action retires any prior stopped owners. Merely
    ///returning to the menu leaves the same failure graph retained here.
    pub fn queue(&mut self, request: Request) -> Result<(), String> {
        if matches!(
            self.status,
            Status::Queued | Status::Loading | Status::Activating | Status::AwaitingNativeView
        ) {
            return Err(
                "a local session is already retained; leave it before starting another".into(),
            );
        }
        if request.player_name.trim().is_empty() {
            return Err("Enter a player name before starting skirmish".into());
        }
        if request.game_name.trim().is_empty() {
            return Err("Enter a game name before starting skirmish".into());
        }
        #[cfg(windows)]
        {
            self.failed = None;
            self.engine = None;
        }
        self.request = Some(request);
        self.status = Status::Queued;
        self.message = "Preparing local skirmish".into();
        Ok(())
    }
    ///Called after the original loading page has been painted. The existing
    ///facade joins its real scoped ASYNC job synchronously; this UI currently
    ///waits at that boundary instead of fabricating a background ready result.
    pub fn start_queued(&mut self) {
        if self.status != Status::Queued {
            return;
        }
        let Some(request) = self.request.clone() else {
            return;
        };
        self.status = Status::Loading;
        #[cfg(windows)]
        {
            match windows::Engine::new(self.focus.clone()) {
                Err(error) => {
                    self.message = error;
                    self.status = Status::Failed;
                }
                Ok(mut engine) => {
                    match engine.start(&self.root, &request) {
                        Ok(()) => {
                            self.status = Status::Activating;
                            self.message = "Initializing the native local session".into();
                        }
                        Err(failure) => {
                            self.message = failure.error.clone();
                            self.failed = Some(failure);
                            self.status = Status::Failed;
                        }
                    }
                    self.engine = Some(engine);
                }
            }
        }
        #[cfg(not(windows))]
        {
            let _ = request;
            self.message = "The native local battle backend requires Windows".into();
            self.status = Status::Failed;
        }
    }
    ///Dispose an already stopped/failed preparation only. A successfully
    ///activated session needs the actual native leave path before retirement.
    pub fn discard_stopped_attempt(&mut self) -> Result<(), String> {
        if matches!(
            self.status,
            Status::Loading | Status::Activating | Status::AwaitingNativeView
        ) {
            return Err("current session requires the actual native leave-game service".into());
        }
        #[cfg(windows)]
        {
            self.engine = None;
            self.failed = None;
        }
        self.request = None;
        self.status = Status::Idle;
        self.message.clear();
        Ok(())
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use crate::windows_reset_os::{
        PerformanceTimer, WindowsLocalFiletime, WindowsResetOs, WindowsStandaloneClock,
        WindowsThreadIds,
    };
    use aa_netcode::{
        local_bootstrap,
        local_host::NodeTime,
        local_skirmish_startup::{Choices, Startup, WorkerFailure},
    };
    use aa_numeric::Precision;
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use aa_runtime::{
        catalog::{Catalog, SourceTransfer},
        execution_context::ApplicationThread,
        localization::Localization,
        settings::Settings,
        simulation_execution::SimulationThread,
        world_ready_services::Application,
    };
    use std::{any::Any, sync::Arc};

    ///Opaque but retained actual prefixes, never a replacement clean world.
    pub(super) struct Failure {
        pub error: String,
        _owners: Vec<Box<dyn Any>>,
    }
    impl Failure {
        fn early(error: impl ToString) -> Self {
            Self {
                error: error.to_string(),
                _owners: vec![],
            }
        }
        fn retained(error: impl ToString, owners: impl Any) -> Self {
            Self {
                error: error.to_string(),
                _owners: vec![Box::new(owners)],
            }
        }
    }
    fn retain_source_failure(
        catalog: Catalog<'_>,
        source: Arc<ActorCatalog>,
        error: impl ToString,
        owners: impl Any,
    ) -> Failure {
        let message = error.to_string();
        match catalog.into_source_transfer(source) {
            Ok(source) => Failure::retained(message, (source, owners)),
            //A refused source transfer cannot be made self-referential or
            //rebound to a fabricated Catalog. Preserve the owned services and
            //report that this lower-level source scope did not return itself.
            Err((reason, _)) => Failure::retained(
                format!("{message}; source scope retention refused: {reason}"),
                owners,
            ),
        }
    }
    struct MainOwners {
        startup: Startup,
        services: Application,
        source: Arc<ActorCatalog>,
        resources: Arc<ResourceCatalog>,
        lifecycle: Option<aa_runtime::restore_lifecycle::ScopedManagedLoad>,
        transfer: Option<SourceTransfer>,
        _variables: aa_runtime::creator_variables::State,
        time: NodeTime<PerformanceTimer>,
        source_settings: Settings,
    }
    enum SimulationState {
        Empty,
        Failed {
            failure: local_bootstrap::world_ready::ActivationFailure,
            notifications: local_bootstrap::Notifications,
            time: NodeTime<PerformanceTimer>,
            ready_diagnostics: Vec<local_bootstrap::world_ready::Diagnostic>,
            success_diagnostics: Vec<local_bootstrap::world_ready::SuccessDiagnostic>,
            creator_variables: aa_runtime::creator_variables::State,
        },
        Activated {
            session: local_bootstrap::world_ready::ActualActivatedSession,
            source: Arc<ActorCatalog>,
            notifications: local_bootstrap::Notifications,
            time: NodeTime<PerformanceTimer>,
            focus: Reader,
            ready_diagnostics: Vec<local_bootstrap::world_ready::Diagnostic>,
            success_diagnostics: Vec<local_bootstrap::world_ready::SuccessDiagnostic>,
            creator_variables: aa_runtime::creator_variables::State,
        },
    }
    pub struct ActiveLoans<'a> {
        pub session: &'a mut local_bootstrap::world_ready::ActualActivatedSession,
        pub source: &'a Arc<ActorCatalog>,
        pub notifications: &'a local_bootstrap::Notifications,
        pub time: &'a mut NodeTime<PerformanceTimer>,
        pub focus: &'a Reader,
        pub ready_diagnostics: &'a mut Vec<local_bootstrap::world_ready::Diagnostic>,
        pub success_diagnostics: &'a mut Vec<local_bootstrap::world_ready::SuccessDiagnostic>,
        pub creator_variables: &'a mut aa_runtime::creator_variables::State,
    }
    pub(super) struct Engine {
        application: Arc<ApplicationThread>,
        simulation: SimulationThread<SimulationState>,
        focus: Reader,
        activation: Option<aa_runtime::simulation_execution::JobTicket<Option<String>>>,
    }
    impl Engine {
        pub fn with_activated<T: Send + 'static>(
            &mut self,
            call: impl FnOnce(
                ActiveLoans<'_>,
                &mut aa_runtime::simulation_execution::Execution<'_>,
            ) -> Result<T, String>
            + Send
            + 'static,
        ) -> Result<T, String> {
            self.submit_activated(call)?.wait()
        }
        pub fn submit_activated<T: Send + 'static>(
            &mut self,
            call: impl FnOnce(
                ActiveLoans<'_>,
                &mut aa_runtime::simulation_execution::Execution<'_>,
            ) -> Result<T, String>
            + Send
            + 'static,
        ) -> Result<aa_runtime::simulation_execution::JobTicket<T>, String> {
            self.simulation.submit(move |state, execution| match state {
                SimulationState::Activated {
                    session,
                    source,
                    notifications,
                    time,
                    focus,
                    ready_diagnostics,
                    success_diagnostics,
                    creator_variables,
                } => call(
                    ActiveLoans {
                        session,
                        source,
                        notifications,
                        time,
                        focus,
                        ready_diagnostics,
                        success_diagnostics,
                        creator_variables,
                    },
                    execution,
                ),
                SimulationState::Failed { failure, .. } => Err(failure.error().to_owned()),
                SimulationState::Empty => {
                    Err("actual7757/7738 activation has not completed".into())
                }
            })
        }
        pub(super) fn poll_activation(&mut self) -> Result<Option<Option<String>>, String> {
            let result = self
                .activation
                .as_mut()
                .ok_or("native activation operation has not been submitted")?
                .try_result();
            if !matches!(result, Ok(None)) {
                self.activation = None;
            }
            result
        }
        pub fn new(focus: Reader) -> Result<Self, String> {
            let application = Arc::new(ApplicationThread::capture(&mut WindowsThreadIds)?);
            let simulation =
                SimulationThread::start(Arc::clone(&application), WindowsThreadIds, |_| {
                    Ok(SimulationState::Empty)
                })?;
            Ok(Self {
                application,
                simulation,
                focus,
                activation: None,
            })
        }
        fn prepare(
            &self,
            root: &Path,
            request: &Request,
        ) -> Result<(MainOwners, Option<String>), Failure> {
            let install = root
                .parent()
                .ok_or_else(|| Failure::early("reconstruction directory has no install parent"))?;
            let mut files = V14Sources::open(
                install.join("Data.rwd"),
                root.join("native-game/runtime-expanded-v14"),
                SourceLimits {
                    max_member_bytes: u64::MAX,
                },
            )
            .map_err(Failure::early)?;
            let source = Arc::new(ActorCatalog::load_v14(&mut files).map_err(Failure::early)?);
            let mut catalog = Catalog::new(&source);
            catalog.settings =
                Some(Settings::load_v14(&mut files, Limits::default()).map_err(Failure::early)?);
            let text =
                Localization::load_v14(&mut files, Limits::default()).map_err(Failure::early)?;
            catalog.definition_text = Some(text.clone());
            *catalog.localization.borrow_mut() = Some(text);
            let proof=aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).map_err(Failure::early)?;
            let startup_inputs =
                aa_runtime::actor_creation_startup::StartupInputs::from_verified_startup_reference(
                    &proof,
                )
                .map_err(Failure::early)?;
            aa_runtime::combat_source_completion::install(
                &catalog,
                &mut files,
                proof.context(),
                proof.csv(),
                proof.globals(),
            )
            .map_err(Failure::early)?;
            aa_runtime::combat_source_completion::compute(&catalog).map_err(Failure::early)?;
            let resources =
                Arc::new(ResourceCatalog::load_v14(&mut files).map_err(Failure::early)?);
            let variables = aa_runtime::creator_variables::State::from_fresh_sources(
                &catalog,
                &mut files,
                aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
            )
            .map_err(Failure::early)?;
            let clock = PerformanceTimer::from_native_constructor().map_err(Failure::early)?;
            let mut lifecycle = aa_runtime::restore_lifecycle::ScopedManagedLoad::from_sources(
                &mut files,
                clock.milliseconds().map_err(Failure::early)?,
            )
            .map_err(Failure::early)?;
            lifecycle
                .select_local_offline(
                    &catalog,
                    &mut files,
                    aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
                    &mut WindowsStandaloneClock,
                )
                .map_err(Failure::early)?;
            let output = crate::frontend::user_dir().join("battles").join(format!(
                "{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(Failure::early)?
                    .as_nanos()
            ));
            std::fs::create_dir_all(&output).map_err(Failure::early)?;
            let mut services = match Application::from_fresh_sources(
                &catalog,
                files,
                aa_runtime::application_mode::GameAppState::from_native_constructor(),
                startup_inputs,
                &output,
                aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
            ) {
                Ok(v) => v,
                Err(error) => {
                    let message = error.error.clone();
                    return Err(retain_source_failure(
                        catalog,
                        Arc::clone(&source),
                        message,
                        (error, lifecycle, variables, resources),
                    ));
                }
            };
            let selected = match services.depot_mut().select_map(
                &catalog,
                &request.source_path.encode_utf16().collect::<Vec<_>>(),
            ) {
                Ok(value) => value,
                Err(error) => {
                    return Err(retain_source_failure(
                        catalog,
                        Arc::clone(&source),
                        error,
                        (services, lifecycle, variables, resources),
                    ));
                }
            };
            let (mut startup, mut error) = match Startup::construct(
                &catalog,
                &variables,
                Arc::clone(&selected),
                Precision::Single24,
            ) {
                Ok(v) => (v, None),
                Err((v, e)) => (v, Some(e)),
            };
            let timer = match PerformanceTimer::from_native_constructor() {
                Ok(value) => value,
                Err(error) => {
                    return Err(retain_source_failure(
                        catalog,
                        Arc::clone(&source),
                        error,
                        (startup, services, lifecycle, variables, resources),
                    ));
                }
            };
            let mut time = NodeTime::from_native_constructor(timer);
            if let Err(error) = time.begin_update_prefix(Precision::Single24) {
                return Err(retain_source_failure(
                    catalog,
                    Arc::clone(&source),
                    error,
                    (startup, services, lifecycle, variables, resources, time),
                ));
            }
            if error.is_none() {
                let mut launch = local_bootstrap::constructor_launch_state();
                launch.kind = 12;
                let choices = Choices {
                    host: local_bootstrap::launch::HostInput::from_ui_choices(
                        launch,
                        2,
                        2,
                        false,
                        aa_netcode::lobby::GameOptions {
                            name: request.game_name.encode_utf16().collect(),
                            password: vec![],
                            max_players: -1,
                            field_d8: u32::MAX,
                            allow_observers: request.observers,
                        },
                        Some(selected.ids().to_vec()),
                    ),
                    player_name: request.player_name.encode_utf16().collect(),
                    seed: request.seed,
                };
                let identity = self.simulation.identity();
                let mut gate = || {
                    identity
                        .sync_gate(&mut WindowsThreadIds, || {
                            Err("same actual SAI required on simulation thread".into())
                        })
                        .map_err(aa_netcode::Error::Protocol)
                };
                let mut reset = WindowsResetOs::new(&mut gate);
                //The facade borrows current Text only at the actual player
                //creation read, releasing it before47EA40 replaces MAPSTRINGS.
                error = startup
                    .host_and_launch_current_text(
                        &catalog,
                        choices,
                        &mut reset,
                        &mut time,
                        &mut lifecycle,
                        &mut services,
                        &identity,
                        || WindowsThreadIds,
                        Precision::Single24,
                    )
                    .err();
            }
            let source_settings = catalog
                .settings
                .as_ref()
                .ok_or_else(|| Failure::early("actual Settings owner missing"))?
                .clone();
            let transfer = catalog
                .into_source_transfer(Arc::clone(&source))
                .map_err(|(e, _)| Failure::early(e))?;
            Ok((
                MainOwners {
                    startup,
                    services,
                    source,
                    resources,
                    lifecycle: Some(lifecycle),
                    transfer: Some(transfer),
                    _variables: variables,
                    time,
                    source_settings,
                },
                error,
            ))
        }
        pub fn start(&mut self, root: &Path, request: &Request) -> Result<(), Failure> {
            let (mut owners, error) = self.prepare(root, request)?;
            if let Some(error) = error {
                return Err(Failure::retained(error, owners));
            }
            let mut read = Catalog::new(&owners.source);
            read.settings = Some(owners.source_settings.clone());
            let transfer = match owners.transfer.take() {
                Some(value) => value,
                None => {
                    drop(read);
                    return Err(Failure::retained(
                        "source transfer already consumed",
                        owners,
                    ));
                }
            };
            let lifecycle = match owners.lifecycle.take() {
                Some(value) => value,
                None => {
                    drop(read);
                    return Err(Failure::retained(
                        "managed lifecycle already consumed",
                        (owners, transfer),
                    ));
                }
            };
            let parts = match owners.services.parts(&read) {
                Ok(value) => value,
                Err(error) => {
                    drop(read);
                    return Err(Failure::retained(error, (owners, transfer, lifecycle)));
                }
            };
            let result = owners.startup.run_queued_resource_task(
                transfer,
                lifecycle,
                parts.depot,
                parts.application,
                parts.startup,
                Arc::clone(&owners.resources),
                Arc::clone(&self.application),
                self.simulation.identity(),
                || WindowsThreadIds,
                crate::runtime_fp::sample,
                Precision::Double53,
            );
            drop(read);
            let task = match result {
                Ok(v) => v,
                Err(failure) => {
                    let error = failure.error().to_owned();
                    return Err(Failure::retained(error, (owners, failure)));
                }
            };
            let MainOwners {
                startup,
                services,
                source,
                time,
                _variables: creator_variables,
                ..
            } = owners;
            let (host, notifications) = startup.into_host_and_notifications();
            let focus = self.focus.clone();
            //Only Send owners cross. ActualActivatedSession/World are built
            //inside and retained inside this same native simulation thread.
            let outcome = self
                .simulation
                .submit(move |state, _execution| {
                    let mut catalog = Catalog::new(&source);
                    catalog.settings = Some(task.mission().staged().settings.clone());
                    let mut time = time;
                    let mut ready = vec![];
                    let mut success = vec![];
                    let activation = host.finish_completed_world(
                        task,
                        &catalog,
                        services,
                        Arc::clone(&source),
                        &notifications,
                        &mut time,
                        Precision::Double53,
                        &mut WindowsLocalFiletime::current,
                        &mut ready,
                        &mut success,
                    );
                    drop(catalog);
                    match activation {
                        Ok(session) => {
                            *state = SimulationState::Activated {
                                session,
                                source,
                                notifications,
                                time,
                                focus,
                                ready_diagnostics: ready,
                                success_diagnostics: success,
                                creator_variables,
                            };
                            Ok(None)
                        }
                        Err(failure) => {
                            let error = failure.error().to_owned();
                            *state = SimulationState::Failed {
                                failure,
                                notifications,
                                time,
                                ready_diagnostics: ready,
                                success_diagnostics: success,
                                creator_variables,
                            };
                            Ok(Some(error))
                        }
                    }
                })
                .map_err(Failure::early)?;
            self.activation = Some(outcome);
            Ok(())
        }
    }
    //Compile-time ownership checks, with no unsafe Send implementation.
    const _: fn() = || {
        fn send<T: Send>() {}
        send::<Startup>();
        send::<Application>();
        send::<local_bootstrap::load_task::SuccessfulTask>();
        send::<WorkerFailure>();
    };
}
