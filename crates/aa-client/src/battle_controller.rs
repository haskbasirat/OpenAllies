//! Client ownership of the real local-skirmish startup facade. No inspection
//!scene, test fixture, raw StartedMission, or successful loader alone can
//!enable game orders. Mutable native World stays on the one SimulationThread.
use aa_runtime::application_window_state::Reader;
#[path="battle_startup_job.rs"]
mod startup_job;
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
    ui_bridge:crate::battle_ui_bridge::Client,
    request: Option<Request>,
    status: Status,
    message: String,
    #[cfg(windows)]
    engine: Option<windows::Engine>,
    #[cfg(windows)]
    failed: Option<windows::Failure>,
}
impl Controller {
    pub fn new(root: &Path, focus: Reader,ui_bridge:crate::battle_ui_bridge::Client) -> Self {
        Self {
            root: root.to_owned(),
            focus,
            ui_bridge,
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
    pub fn ui_backend(&self)->crate::battle_ui_bridge::Client{self.ui_bridge.clone()}
    ///Application+10C as read by the last completed notification/frame job on
    ///the simulation thread (0 before the first one). AppState4 is the Game
    ///state entered by Loading682EE0 -> 40CE10(Loading+C).
    pub fn application_state(&self)->u32{
        #[cfg(windows)]{self.engine.as_ref().map(|e|e.application_state).unwrap_or(0)}
        #[cfg(not(windows))]{0}
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
        if !matches!(self.status,Status::Loading|Status::Activating|Status::AwaitingNativeView) {
            return;
        }
        #[cfg(windows)]
        {
            let Some(engine) = &mut self.engine else {
                return;
            };
            if self.status==Status::Loading {
                let result=self.request.as_ref().ok_or_else(||windows::Failure::early("loading has no retained request"))
                    .and_then(|request|engine.poll_start(request));
                match result {
                    Ok(windows::StartPoll::Pending(message))=>self.message=message.into(),
                    Ok(windows::StartPoll::Activating)=>{self.status=Status::Activating;self.message="Initializing the native local session".into();},
                    Err(failure)=>{self.message=failure.error.clone();self.failed=Some(failure);self.status=Status::Failed;},
                }
                return;
            }
            if self.status==Status::AwaitingNativeView {
                match engine.poll_notifications(self.request.as_ref().map(|r|r.map_id.as_str()).unwrap_or("")){
                    Ok(Some(Some(error)))|Err(error)=>{
                        //The same retained notification/frame operation also
                        //runs the AppState4 game frames after Loading.
                        let what=if engine.application_state==4{"Native game frame stopped"}else{"Native view initialization stopped"};
                        if !self.message.starts_with(what){log::error!("{what}: {error}");}
                        self.message=format!("{what}: {error}")
                    },
                    Ok(Some(None))=>self.message=if engine.application_state==4{
                        "In game: AppState 4, Game interface shown (40CA80/5C0440); its live-World update 612010 is not bound yet".into()
                    }else{"Application notifications processed; native view/task protocol remains authoritative.".into()},
                    Ok(None)=>{},
                }
                return;
            }
            match engine.poll_activation() {
                Ok(None) => {}
                Ok(Some(None)) => {
                    self.status = Status::AwaitingNativeView;
                    let start=self.request.as_ref().ok_or_else(||"activated mission has no retained selected-map request".to_owned())
                        .and_then(|request|engine.start_notifications(&request.map_id));
                    match start{
                        Ok(())=>self.message="Processing native application notifications".into(),
                        Err(error)=>{log::error!("native view initialization stopped: {error}");self.message=format!("Native view initialization stopped: {error}")},
                    }
                }
                Ok(Some(Some(error))) | Err(error) => {
                    self.status = Status::Failed;
                    self.message = format!("Native activation stopped: {error}");
                }
            }
        }
    }
    pub fn user_ready_command(&mut self)->Result<(),String>{
        #[cfg(windows)]{self.engine.as_mut().ok_or("native application owner is absent")?.queue_user_ready()}
        #[cfg(not(windows))]{Err("native loading UserReady requires Windows application".into())}
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
    ///Called after the loading page has been painted. Application/simulation
    ///thread capture remains on this GUI thread; source work and the real
    ///ASYNC task are polled while the normal event loop keeps running.
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
            match windows::Engine::new(self.focus.clone(),&self.ui_bridge) {
                Err(error) => {
                    self.message = error;
                    self.status = Status::Failed;
                }
                Ok(mut engine) => {
                    match engine.start_preparation(&self.root, &request) {
                        Ok(())=>self.message="Reading and validating game data".into(),
                        Err(error)=>{self.message=error;self.status=Status::Failed;},
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
        _owners: Vec<Box<dyn Any+Send>>,
    }
    impl Failure {
        pub(super) fn early(error: impl ToString) -> Self {
            Self {
                error: error.to_string(),
                _owners: vec![],
            }
        }
        fn retained(error: impl ToString, owners: impl Any+Send) -> Self {
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
        owners: impl Any+Send,
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
    struct SourceOwners {
        source:Arc<ActorCatalog>,resources:Arc<ResourceCatalog>,services:Application,
        lifecycle:aa_runtime::restore_lifecycle::ScopedManagedLoad,
        variables:aa_runtime::creator_variables::State,
        selected:Arc<aa_runtime::resource_creator::SourceResource>,transfer:SourceTransfer,
    }
    struct PendingMain {
        source:Arc<ActorCatalog>,time:NodeTime<PerformanceTimer>,
        creator_variables:aa_runtime::creator_variables::State,
    }
    pub(super) enum StartPoll {Pending(&'static str),Activating}
    enum MissionState {
        Empty,
        SourceFailed(Failure),
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
            view_startup:Option<crate::battle_view_startup::State>,
        },
    }
    ///Application view allocation identities survive mission transitions on
    ///this same simulation OS thread. Mission owners remain move-only below.
    struct SimulationState {
        application_views:crate::live_scene::world_owner::ApplicationViews,
        application_events:crate::battle_notifications::State,
        mission:MissionState,
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
        pub application_views:&'a mut crate::live_scene::world_owner::ApplicationViews,
        pub application_events:&'a mut crate::battle_notifications::State,
        pub view_startup:&'a mut Option<crate::battle_view_startup::State>,
    }
    pub(super) struct Engine {
        application: Arc<ApplicationThread>,
        simulation: SimulationThread<SimulationState>,
        focus: Reader,
        source_job:Option<super::startup_job::Job<Result<SourceOwners,Failure>>>,
        native_source_job:Option<aa_runtime::simulation_execution::JobTicket<Result<SourceOwners,Failure>>>,
        resource_task:Option<aa_netcode::local_skirmish_startup::PendingResourceTask>,
        pending_main:Option<PendingMain>,
        activation: Option<aa_runtime::simulation_execution::JobTicket<Option<String>>>,
        view_startup:Option<aa_runtime::simulation_execution::JobTicket<(Option<String>,u32)>>,
        notifications_stopped:bool,
        ///Each ReadyButton press is one native 6E80F0 command, in order.
        user_ready_tickets:std::collections::VecDeque<aa_runtime::simulation_execution::JobTicket<()>>,
        pub(super) application_state:u32,
        ui_bridge:crate::battle_ui_bridge::Client,
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
            let wake=self.ui_bridge.clone();
            self.simulation.submit(move |state, execution| {let result=match &mut state.mission {
                MissionState::Activated {
                    session,
                    source,
                    notifications,
                    time,
                    focus,
                    ready_diagnostics,
                    success_diagnostics,
                    creator_variables,
                    view_startup,
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
                        application_views:&mut state.application_views,
                        application_events:&mut state.application_events,
                        view_startup,
                    },
                    execution,
                ),
                MissionState::SourceFailed(failure) => Err(failure.error.clone()),
                MissionState::Failed { failure, .. } => Err(failure.error().to_owned()),
                MissionState::Empty => {
                    Err("actual7757/7738 activation has not completed".into())
                }
            };wake.wake();result})
        }
        pub(super) fn start_notifications(&mut self,map_id:&str)->Result<(),String>{
            if self.view_startup.is_some(){return Err("application notification operation already submitted".into())}
            if self.notifications_stopped{return Ok(())}
            let bridge=self.ui_bridge.clone();let map_id=map_id.to_owned();
            self.view_startup=Some(self.submit_activated(move|loans,execution|{
                let mut catalog=Catalog::new(loans.source);
                let resources=loans.session.world().evidence().resource_source.as_ref()
                    .ok_or("notification actual Resource source owner absent")?;
                catalog.bind_resource_catalog(Arc::clone(resources))?;
                catalog.settings=Some(loans.session.world().presentation_parts()?.settings.clone());
                //The original load argument survives in this SAME activated
                //World's evidence; MapResource+A8 has already restored its
                //prior value at resource return and is not this provenance.
                catalog.native_load_mode=loans.session.world().evidence().native_load_mode;
                //The frame's Catalog: the same bindings, rebuilt inside the
                //frame's Application loan with the AVar25 owner bound.
                let frame_catalog=crate::battle_notifications::FrameCatalog{
                    source:loans.source,resources:Arc::clone(resources),
                    settings:catalog.settings.as_ref().ok_or("notification Catalog settings absent")?,
                    load_mode:catalog.native_load_mode,
                };
                let result=loans.application_events.drain(loans.session,&catalog,loans.notifications,loans.time,
                    loans.application_views,loans.view_startup,&bridge,execution,&map_id)
                    .and_then(|()|loans.application_events.run_frame(loans.session,&catalog,&frame_catalog,loans.notifications,loans.time,loans.focus,&bridge,execution));
                Ok((result.err(),loans.application_events.current_state()))
            })?);Ok(())
        }
        pub(super) fn queue_user_ready(&mut self)->Result<(),String>{
            if self.notifications_stopped{return Err("native application stopped after retained error".into())}
            let ticket=self.submit_activated(|loans,_|loans.application_events.user_ready_command(loans.session))?;
            self.user_ready_tickets.push_back(ticket);Ok(())
        }
        pub(super) fn poll_notifications(&mut self,map_id:&str)->Result<Option<Option<String>>,String>{
            if self.notifications_stopped{return Ok(None)}
            while let Some(ticket)=self.user_ready_tickets.front_mut(){
                match ticket.try_result(){
                    Ok(None)=>break,Ok(Some(()))=>{self.user_ready_tickets.pop_front();},
                    Err(error)=>{self.notifications_stopped=true;return Err(error)},
                }
            }
            let result=match self.view_startup.as_mut(){Some(ticket)=>ticket.try_result(),None=>Ok(None)};
            let result=result.map(|done|done.map(|(error,state)|{self.application_state=state;error}));
            if !matches!(result,Ok(None)){
                self.view_startup=None;
                if matches!(&result,Err(_)|Ok(Some(Some(_)))){self.notifications_stopped=true;}
            }
            if self.view_startup.is_none()&&!self.notifications_stopped{self.start_notifications(map_id)?;}
            result
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
        pub fn new(focus: Reader,ui:&crate::battle_ui_bridge::Client) -> Result<Self, String> {
            let application = Arc::new(ApplicationThread::capture(&mut WindowsThreadIds)?);
            ui.bind_application(&application)?;
            let simulation =
                SimulationThread::start(Arc::clone(&application), WindowsThreadIds, |_| {
                    Ok(SimulationState{
                        application_views:crate::live_scene::world_owner::ApplicationViews::from_application_constructor().map_err(|e|e.to_string())?,
                        application_events:crate::battle_notifications::State::from_application_constructor()?,
                        mission:MissionState::Empty,
                    })
                })?;
            Ok(Self {
                application,
                simulation,
                focus,
                source_job:None,native_source_job:None,resource_task:None,pending_main:None,
                activation: None,
                view_startup:None,
                notifications_stopped:false,
                user_ready_tickets:Default::default(),
                application_state:0,
                ui_bridge:ui.clone(),
            })
        }
        fn prepare_sources(
            root: &Path,
            request: &Request,
        ) -> Result<SourceOwners, Failure> {
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
            aa_runtime::replacement_flags_source::complete_source_suffix(&catalog)
                .map_err(Failure::early)?;
            let view_inputs = match aa_runtime::view_startup_inputs::Inputs::from_fresh_sources(
                &catalog,
                &proof,
                &mut files,
                aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
            ) {
                Ok(inputs) => inputs,
                Err(failure) => {
                    let error = failure.error.clone();
                    return Err(retain_source_failure(
                        catalog,
                        Arc::clone(&source),
                        error,
                        (failure, files, startup_inputs),
                    ));
                }
            };
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
                        (error, view_inputs, lifecycle, variables, resources),
                    ));
                }
            };
            if let Err(failure) = services.install_view_inputs(&catalog, view_inputs) {
                let error = failure.error.clone();
                return Err(retain_source_failure(
                    catalog,
                    Arc::clone(&source),
                    error,
                    (failure, services, lifecycle, variables, resources),
                ));
            }
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
            let transfer=catalog.into_source_transfer(Arc::clone(&source))
                .map_err(|(error,_)|Failure::early(error))?;
            Ok(SourceOwners{source,resources,services,lifecycle,variables,selected,transfer})
        }
        ///Runs before Host launch or World construction. The SAME retained
        ///simulation execution can request actual UI policy reads while the
        ///GUI keeps pumping; the ASYNC source worker cannot forge this role.
        fn prepare_native_sources(owners:SourceOwners,bridge:&crate::battle_ui_bridge::Client,
            execution:&mut aa_runtime::simulation_execution::Execution<'_>)->Result<SourceOwners,Failure>{
            let SourceOwners{source,resources,mut services,lifecycle,variables,selected,transfer}=owners;
            let(actual_source,data)=transfer.into_parts();
            if !Arc::ptr_eq(&source,&actual_source){
                return Err(Failure::retained("native source preparation owner changed",
                    (source,actual_source,data,services,lifecycle,variables,resources,selected)));
            }
            let mut catalog=match data.bind(&source){
                Ok(value)=>value,
                Err((error,data))=>return Err(Failure::retained(error,
                    (actual_source,data,services,lifecycle,variables,resources,selected))),
            };
            if let Err(error)=catalog.bind_resource_catalog(Arc::clone(&resources)){
                return Err(retain_source_failure(catalog,Arc::clone(&source),error,
                    (actual_source,services,lifecycle,variables,resources,selected)));
            }
            match crate::battle_source_preparation::prepare(&catalog,&mut services,bridge,execution){
                Ok(report)=>log::info!("native pre-World Object/Actor source children completed for {} definitions",report.definitions),
                Err(error)=>return Err(retain_source_failure(catalog,Arc::clone(&source),error,
                    (actual_source,services,lifecycle,variables,resources,selected))),
            }
            let transfer=match catalog.into_source_transfer(Arc::clone(&source)){
                Ok(value)=>value,
                Err((error,_))=>return Err(Failure::retained(error,
                    (actual_source,services,lifecycle,variables,resources,selected))),
            };
            Ok(SourceOwners{source,resources,services,lifecycle,variables,selected,transfer})
        }
        fn start_native_source_preparation(&mut self,owners:SourceOwners)->Result<(),Failure>{
            //Retain ownership outside the submitted closure until the queue
            //accepts it. A closed simulation queue cannot discard the prefix.
            let pending=Arc::new(std::sync::Mutex::new(Some(owners)));
            let worker=Arc::clone(&pending);let bridge=self.ui_bridge.clone();let wake=bridge.clone();
            let submitted=self.simulation.submit(move|_,execution|{
                let owners=worker.lock().map_err(|_|"native source transport owner is poisoned")?
                    .take().ok_or("native source transport owner was already consumed")?;
                let result=Self::prepare_native_sources(owners,&bridge,execution);
                if let Err(failure)=&result{
                    log::error!("native pre-World source preparation failed: {}",failure.error);
                }
                wake.wake();Ok(result)
            });
            match submitted{
                Ok(ticket)=>{self.native_source_job=Some(ticket);Ok(())},
                Err(error)=>Err(Failure::retained(error,pending)),
            }
        }
        ///All native Host/reset/launch writes still execute on the captured GUI
        ///application thread after the source-only result returns here.
        fn launch_sources(&mut self,owners:SourceOwners,request:&Request)->Result<(MainOwners,Option<String>),Failure>{
            let current=match aa_runtime::execution_context::OsThreadIds::current_thread_id(&mut WindowsThreadIds){
                Ok(value)=>value,Err(error)=>return Err(Failure::retained(error,owners)),
            };
            if current!=self.application.id(){return Err(Failure::retained("Host launch must run on its retained GUI application thread",owners))}
            let SourceOwners{source,resources,mut services,mut lifecycle,variables,selected,transfer}=owners;
            let(actual_source,data)=transfer.into_parts();
            if !Arc::ptr_eq(&source,&actual_source){return Err(Failure::retained("source preparation owner changed",(source,actual_source,data,services,lifecycle,variables,resources,selected)))}
            let catalog=match data.bind(&source){Ok(value)=>value,Err((error,data))=>return Err(Failure::retained(error,(data,actual_source,services,lifecycle,variables,resources,selected)))};
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
                let host_input=(||->Result<_,String>{
                    //Capture the actual retained AppState on its owner thread.
                    //This is native ESI's value read before the confirmed-map
                    //LaunchState constructor; no UI callback runs in this job.
                    let confirmation=self.simulation.submit(|state,_|state.application_events.selected_map_confirmation())?.wait()?;
                    let bank=catalog.definition_bank.try_borrow().map_err(|_|"selected-map confirmation source Bank is already borrowed")?;
                    confirmation.into_host_input(&catalog,&bank,services.application(),&selected,
                        aa_netcode::lobby::GameOptions {
                            name:request.game_name.encode_utf16().collect(),password:vec![],max_players:-1,
                            field_d8:u32::MAX,allow_observers:request.observers,
                        }).map_err(|e|e.to_string())
                })();
                let host=match host_input{
                    Ok(value)=>value,
                    Err(error)=>return Err(retain_source_failure(catalog,Arc::clone(&source),error,
                        (startup,services,lifecycle,variables,resources,time))),
                };
                let choices = Choices {
                    host,
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
        pub fn start_preparation(&mut self,root:&Path,request:&Request)->Result<(),String>{
            if self.source_job.is_some()||self.native_source_job.is_some()||self.resource_task.is_some(){return Err("startup work already retained".into())}
            let root=root.to_owned();let request=request.clone();let wake=self.ui_bridge.clone();
            self.source_job=Some(super::startup_job::Job::start("BattleSourcePreparation",move||wake.wake(),
                move||Self::prepare_sources(&root,&request))?);
            Ok(())
        }
        pub(super) fn poll_start(&mut self,request:&Request)->Result<StartPoll,Failure>{
            if let Some(job)=self.source_job.as_mut(){
                let source=match job.poll().map_err(Failure::early)?{
                    None=>return Ok(StartPoll::Pending("Reading and validating game data")),Some(value)=>value,
                };
                self.source_job=None;
                self.start_native_source_preparation(source?)?;
                return Ok(StartPoll::Pending("Preparing native object and actor resources"));
            }
            if let Some(job)=self.native_source_job.as_mut(){
                let source=match job.try_result().map_err(Failure::early)?{
                    None=>return Ok(StartPoll::Pending("Preparing native object and actor resources")),
                    Some(value)=>value,
                };
                self.native_source_job=None;
                let(mut owners,error)=self.launch_sources(source?,request)?;
                if let Some(error)=error{return Err(Failure::retained(error,owners))}
                let transfer=owners.transfer.take().ok_or_else(||Failure::early("source transfer already consumed"))?;
                let lifecycle=owners.lifecycle.take().ok_or_else(||Failure::early("managed lifecycle already consumed"))?;
                let MainOwners{startup,services,source,resources,time,_variables:creator_variables,..}=owners;
                self.pending_main=Some(PendingMain{source,time,creator_variables});
                self.resource_task=Some(aa_netcode::local_skirmish_startup::PendingResourceTask::start(
                    startup,transfer,lifecycle,services,resources,Arc::clone(&self.application),self.simulation.identity(),
                    ||WindowsThreadIds,crate::runtime_fp::sample,Precision::Double53)
                    .map_err(|failure|Failure::retained(failure.error.clone(),failure))?);
                return Ok(StartPoll::Pending("Loading the selected world"));
            }
            if let Some(task)=self.resource_task.as_mut(){
                let(startup,services,result)=match task.poll().map_err(Failure::early)?{
                    None=>return Ok(StartPoll::Pending("Loading the selected world")),Some(value)=>value,
                };
                self.resource_task=None;
                let pending=self.pending_main.take().ok_or_else(||Failure::early("resource completion lost its retained main owners"))?;
                let task=match result{Ok(value)=>value,Err(failure)=>return Err(Failure::retained(failure.error().to_owned(),(startup,services,pending,failure)))};
                self.submit_activation(startup,services,pending,task)?;
                return Ok(StartPoll::Activating);
            }
            Err(Failure::early("no startup operation is retained"))
        }
        fn submit_activation(&mut self,startup:Startup,services:Application,pending:PendingMain,
            task:local_bootstrap::load_task::SuccessfulTask)->Result<(),Failure>{
            let PendingMain{source,time,creator_variables}=pending;
            let (host, notifications) = startup.into_host_and_notifications();
            let focus = self.focus.clone();
            //Only Send owners cross. ActualActivatedSession/World are built
            //inside and retained inside this same native simulation thread.
            let outcome = self
                .simulation
                .submit(move |state, _execution| {
                    let mut catalog = Catalog::new(&source);
                    let binding = task.mission().staged().resource_source.as_ref()
                        .ok_or_else(|| "activation actual Resource source owner absent".to_owned())
                        .and_then(|resources| catalog.bind_resource_catalog(Arc::clone(resources)));
                    if let Err(error) = binding {
                        drop(catalog);
                        state.mission = MissionState::SourceFailed(Failure::retained(
                            error.clone(),
                            (host, task, services, source, notifications, time, creator_variables, focus),
                        ));
                        return Ok(Some(error));
                    }
                    catalog.settings = Some(task.mission().staged().settings.clone());
                    //Keep the actual task's retained load argument for SAI
                    //settings. Never substitute the post-return MapResource
                    //mode or an assumed new-game value.
                    catalog.native_load_mode = task.mission().staged().native_load_mode;
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
                            state.mission = MissionState::Activated {
                                session,
                                source,
                                notifications,
                                time,
                                focus,
                                ready_diagnostics: ready,
                                success_diagnostics: success,
                                creator_variables,
                                view_startup:None,
                            };
                            Ok(None)
                        }
                        Err(failure) => {
                            let error = failure.error().to_owned();
                            state.mission = MissionState::Failed {
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
    #[cfg(test)]
    mod startup_tests { include!("battle_controller_startup_tests.rs"); }
    #[cfg(test)]
    mod native_source_transport_tests {
        use super::*;
        use std::{sync::{mpsc,atomic::{AtomicBool,Ordering}},time::{Duration,Instant}};
        struct Prefix(Arc<AtomicBool>);
        impl Drop for Prefix {fn drop(&mut self){self.0.store(true,Ordering::Release);}}
        #[test]
        fn native_source_ticket_keeps_gui_polling_and_retains_failed_prefix() {
            let window=aa_runtime::application_window_state::ApplicationWindowState::from_native_constructor();
            let(mut queue,bridge)=crate::battle_ui_bridge::Queue::new();
            let mut controller=Controller::new(Path::new("."),window.reader(),bridge.clone());
            let request=Request{map_id:"native-source-transport-only".into(),source_path:"not-loaded".into(),
                player_name:"transport".into(),game_name:"transport".into(),seed:0,observers:false};
            controller.queue(request).unwrap();
            let mut engine=Engine::new(window.reader(),&bridge).unwrap();
            let parent=engine.application.id();
            let(started,entered)=mpsc::channel();let(release,gate)=mpsc::channel();
            let retired=Arc::new(AtomicBool::new(false));let prefix=Prefix(Arc::clone(&retired));
            engine.native_source_job=Some(engine.simulation.submit(move|_,execution|{
                assert_eq!(execution.application_os_thread_id(),parent);
                assert_ne!(execution.current_os_thread_id()?,parent);
                started.send(()).unwrap();gate.recv().unwrap();
                Ok(Err::<SourceOwners,_>(Failure::retained("controlled actual source child failure",prefix)))
            }).unwrap());
            controller.engine=Some(engine);controller.status=Status::Loading;
            entered.recv_timeout(Duration::from_secs(5)).unwrap();
            for _ in 0..3{
                controller.poll();
                assert_eq!(controller.status(),&Status::Loading);
                assert_eq!(controller.message(),"Preparing native object and actor resources");
            }
            assert!(!retired.load(Ordering::Acquire));release.send(()).unwrap();
            let deadline=Instant::now()+Duration::from_secs(5);
            while controller.status()==&Status::Loading{
                controller.poll();assert!(Instant::now()<deadline);std::thread::yield_now();
            }
            assert_eq!(controller.status(),&Status::Failed);
            assert_eq!(controller.message(),"controlled actual source child failure");
            assert!(!retired.load(Ordering::Acquire));
            queue.close();controller.discard_stopped_attempt().unwrap();
            assert!(retired.load(Ordering::Acquire));
        }
    }
    const _: fn() = || {
        fn send<T: Send>() {}
        send::<Startup>();
        send::<Application>();
        send::<local_bootstrap::load_task::SuccessfulTask>();
        send::<WorkerFailure>();
    };
}
