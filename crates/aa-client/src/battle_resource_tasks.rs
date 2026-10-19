//!Real ResourceTask services on the retained simulation execution. Native
//!guards are same-Application recursive locks; Texture4C delegates the real
//!current UI/source/GPU adapter. Unsupported payloads remain required.
use crate::{battle_texture_services, battle_ui_bridge};
use aa_netcode::local_bootstrap::LocalBootstrap;
use aa_numeric::Precision;
use aa_runtime::{
    resource_depot::{release::ReleaseLog, Backend, Resources},
    resource_objects::{Handle, Kind},
    resource_task::{self, Services, ThreadHandle},
    resource_task_guards::{Capability, Kind as GuardKind, Lease},
    simulation_execution::Execution,
};
type Result<T> = std::result::Result<T, String>;
struct Held {
    resource: Option<Handle>,
    leases: Vec<Lease>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Dispatch,
    Load,
    Request,
    ///5AC3D0/5ABFF0 manager+3C around an unload.
    Unload,
    ///5ABB30..5ABB50 (+4, +20, +3C) in 5B1970.
    Manager,
}
impl Drop for Held {
    fn drop(&mut self) {
        while let Some(lease) = self.leases.pop() {
            drop(lease)
        }
    }
}

pub struct Current<'a, 'execution> {
    bridge: &'a battle_ui_bridge::Client,
    execution: &'a mut Execution<'execution>,
    guards: Capability,
    precision: Precision,
    host: Option<&'a LocalBootstrap>,
    held: Vec<(Phase, Held)>,
    #[cfg(windows)]
    timer: Option<crate::windows_reset_os::PerformanceTimer>,
    #[cfg(windows)]
    model_timer: Option<crate::windows_reset_os::PerformanceTimer>,
}
impl<'a, 'execution> Current<'a, 'execution> {
    pub fn new(
        bridge: &'a battle_ui_bridge::Client,
        execution: &'a mut Execution<'execution>,
        guards: Capability,
        precision: Precision,
    ) -> Self {
        Self {
            bridge,
            execution,
            guards,
            precision,
            host: None,
            held: vec![],
            #[cfg(windows)]
            timer: None,
            #[cfg(windows)]
            model_timer: None,
        }
    }
    ///The SAME actual Session supplies40EAA0 lazily. Construction alone does
    ///not authorize a timing/default variable value or source completion.
    pub fn with_session(mut self, host: &'a LocalBootstrap) -> Self {
        self.host = Some(host);
        self
    }
    fn elapsed(&self) -> Result<u32> {
        #[cfg(windows)]
        {
            self.timer
                .as_ref()
                .ok_or("Resource block timer has not started")?
                .milliseconds()
                .map_err(|e| e.to_string())
        }
        #[cfg(not(windows))]
        {
            Err("Resource block timer requires actual Windows QPC service".into())
        }
    }
}
impl Current<'_, '_> {
    fn push_guards(&mut self, phase: Phase, resource: &Handle, kinds: &[GuardKind]) -> Result<()> {
        let thread = self.current_thread()?;
        let mut held = Held {
            resource: Some(resource.clone()),
            leases: vec![],
        };
        for kind in kinds {
            held.leases.push(self.guards.acquire(resource, *kind, &thread)?);
        }
        self.held.push((phase, held));
        Ok(())
    }
    ///Held drops its leases in reverse acquisition order.
    fn pop_guards(&mut self, phase: Phase, resource: &Handle) -> Result<()> {
        if !self.held.last().is_some_and(|(p, v)| {
            *p == phase && v.resource.as_ref().is_some_and(|r| r.same_identity(resource))
        }) {
            return Err("Resource guard release differs from the acquired allocation/phase".into());
        }
        drop(self.held.pop());
        Ok(())
    }
}
impl Services for Current<'_, '_> {
    fn current_thread(&mut self) -> Result<ThreadHandle> {
        ThreadHandle::on_simulation(self.execution)
    }
    fn enter_manager_5abb30(&mut self, resource: &Handle) -> Result<()> {
        self.push_guards(
            Phase::Manager,
            resource,
            &[GuardKind::Load04, GuardKind::Registry20, GuardKind::Decode3c],
        )
    }
    fn leave_manager_5abb50(&mut self, resource: &Handle) -> Result<()> {
        self.pop_guards(Phase::Manager, resource)
    }
    fn enter_load(&mut self, resource: &Handle) -> Result<()> {
        let thread = self.current_thread()?;
        let mut held = Held {
            resource: Some(resource.clone()),
            leases: vec![],
        };
        held.leases
            .push(self.guards.acquire(resource, GuardKind::Load04, &thread)?);
        //5B1A10 walks the actual descriptor chain forB3D990. Sequence is
        //a sibling underB3D968, so it does not take this optional lock.
        if resource.kind() == Kind::Model {
            held.leases.push(
                self.guards
                    .acquire(resource, GuardKind::Registry20, &thread)?,
            );
        }
        held.leases.push(
            self.guards
                .acquire(resource, GuardKind::Decode3c, &thread)?,
        );
        self.held.push((Phase::Load, held));
        Ok(())
    }
    fn leave_load(&mut self, resource: &Handle) -> Result<()> {
        if !self
            .held
            .last()
            .is_some_and(|(phase, v)| *phase == Phase::Load && v.resource.as_ref().is_some_and(|r|r.same_identity(resource)))
        {
            return Err("Resource load guard release differs from acquired allocation".into());
        }
        drop(self.held.pop());
        Ok(())
    }
    fn enter_request(&mut self, resource: &Handle) -> Result<()> {
        let thread = self.current_thread()?;
        let lease = self
            .guards
            .acquire(resource, GuardKind::Registry20, &thread)?;
        self.held.push((
            Phase::Request,
            Held {
                resource: Some(resource.clone()),
                leases: vec![lease],
            },
        ));
        Ok(())
    }
    fn leave_request(&mut self, resource: &Handle) -> Result<()> {
        if !self.held.last().is_some_and(|(phase, v)| {
            *phase == Phase::Request && v.resource.as_ref().is_some_and(|r|r.same_identity(resource))
        }) {
            return Err("Resource request guard release differs from acquired allocation".into());
        }
        drop(self.held.pop());
        Ok(())
    }
    fn loaded_diagnostic(&mut self, resource: &Handle) -> Result<()> {
        log::info!(
            "native ResourceTask Loaded {}",
            String::from_utf16_lossy(resource.logical_ids())
        );
        Ok(())
    }
    fn request_diagnostic(&mut self, resource: &Handle, immediate: bool) -> Result<()> {
        log::info!(
            "native ResourceTask {} {}",
            if immediate {
                "load immediately"
            } else {
                "add to load queue"
            },
            String::from_utf16_lossy(resource.logical_ids())
        );
        Ok(())
    }
    fn removing_from_queue_diagnostic(&mut self, resource: &Handle) -> Result<()> {
        log::info!(
            "native ResourceTask RemoveFromTaskQueue {}",
            String::from_utf16_lossy(resource.logical_ids())
        );
        Ok(())
    }
    fn block_timer_reset_59aa70(&mut self) -> Result<()> {
        #[cfg(windows)]
        {
            crate::windows_reset_os::PerformanceTimer::initialize_frequency()
        }
        #[cfg(not(windows))]
        {
            Err("Resource task timer frequency requires Windows QPC".into())
        }
    }
    fn block_timer_start_59ab60(&mut self) -> Result<()> {
        #[cfg(windows)]
        {
            self.timer = Some(
                crate::windows_reset_os::PerformanceTimer::from_native_constructor()
                    .map_err(|e| e.to_string())?,
            );
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err("Resource task timer start requires Windows QPC".into())
        }
    }
    fn timing_enabled_40eaa0(&mut self) -> Result<bool> {
        let host = self
            .host
            .ok_or("Resource block40EAA0 requires SAME actual Session owner")?;
        host.can_process_40eaa0(|| {
            self.bridge.call(self.execution, |context| {
                context.ui.uvars.dev_mode_current()
            })
        })
    }
    fn block_diagnostic(
        &mut self,
        resource: &Handle,
        reason: resource_task::BlockDiagnostic,
    ) -> Result<()> {
        log::info!(
            "native ResourceTask {:?} {}: {}ms",
            reason,
            String::from_utf16_lossy(resource.logical_ids()),
            self.elapsed()?
        );
        Ok(())
    }
    //59BFA0 cache resurrection and5B1FD0's actual running-loader waiter
    //retain precise required services until their shared owners are bound.
}
impl resource_task::DispatchServices for Current<'_, '_>{
    fn enter_dispatch(&mut self,application:&resource_task::ApplicationKey,thread:&ThreadHandle)->Result<()>{
        //Borrow the SAME Task18 captured at5B18C0, avoiding a second native
        //thread lookup solely for the modern recursive-lock validation.
        let lease=self.guards.acquire_task(application,thread)?;
        self.held.push((Phase::Dispatch,Held{resource:None,leases:vec![lease]}));Ok(())
    }
    fn dispatch_diagnostic(&mut self,kind:resource_task::DispatchKind<'_>)->Result<()>{
        match kind{
            resource_task::DispatchKind::Resource(r)=>log::info!("native ResourceManager task Load {}",String::from_utf16_lossy(r.logical_ids())),
            resource_task::DispatchKind::View=>log::info!("native ResourceManager task UI"),
        }Ok(())
    }
    fn leave_dispatch(&mut self)->Result<()>{
        if !self.held.last().is_some_and(|(phase,held)|*phase==Phase::Dispatch&&held.resource.is_none()){
            return Err("native queued task dispatch guard is not the current outer loan".into())
        }
        drop(self.held.pop());Ok(())
    }
}
impl Backend for Current<'_, '_> {
    fn load_resource_4c_in(
        &mut self,
        resources: &mut Resources<'_>,
        resource: &Handle,
    ) -> Result<()> {
        match resource.kind() {
            Kind::Texture => resources
                .load_texture_4c(
                    resource,
                    self.precision,
                    &mut battle_texture_services::Current::for_controlled_fresh_start(
                        self.bridge,
                        self.execution,
                    ),
                )
                .map(|_| ()),
            Kind::Model => resources.load_model_4c(resource,self.precision,self).map(|_|()),
            Kind::Sequence => {
                Err("required native service:5A5520 actual same Sequence payload/KF decoder".into())
            }
            Kind::Sound => {
                Err("required native service:59B550 actual same Sound/Miles decoder owner".into())
            }
        }
    }
    fn double_request_warning(&mut self, resource: &Handle, count: i32) -> Result<()> {
        log::warn!(
            "native Resource double request {} count={count}",
            String::from_utf16_lossy(resource.logical_ids())
        );
        Ok(())
    }
    fn model_after_base_3c_in(&mut self,resources:&mut Resources<'_>,resource:&Handle)->Result<()>{
        resources.ni_after_base_3c_5a5410(resource,self.precision)
    }
    fn resource_caching_54(&mut self, resource: &Handle) -> Result<u8> {
        let value = self.bridge.call(self.execution, |context| context.ui.uvars.resource_caching_current())?;
        //Modern census aid (native 5AC3D0 is silent when it caches): the
        //first released Resources reaching CacheOrUnload, then every 1000th.
        static REACHED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = REACHED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if n < 32 || n % 1000 == 0 {
            log::info!(
                "native 5AC3D0 release #{n} reached CacheOrUnload {} (UVar54 ResourceCaching={value})",
                String::from_utf16_lossy(resource.logical_ids())
            );
        }
        Ok(value)
    }
    fn enter_unload_3c(&mut self, resource: &Handle) -> Result<()> {
        self.push_guards(Phase::Unload, resource, &[GuardKind::Decode3c])
    }
    fn leave_unload_3c(&mut self, resource: &Handle) -> Result<()> {
        self.pop_guards(Phase::Unload, resource)
    }
    ///404F60/404F90 with the native format text and arguments.
    fn release_log(&mut self, resource: &Handle, log: &ReleaseLog) -> Result<()> {
        let native = log.native();
        let name = String::from_utf16_lossy(resource.logical_ids());
        let text = match log {
            ReleaseLog::AbortRemovedFromQueue { count }
            | ReleaseLog::AbortAlreadyPopped { count }
            | ReleaseLog::AbortNotRun { count }
            | ReleaseLog::AbortTooLate { count } => {
                native.format.replacen("%s", &name, 1).replacen("%d", &count.to_string(), 1)
            }
            ReleaseLog::NiUnloading { count } => native
                .format
                .replacen("%s", std::thread::current().name().unwrap_or("unnamed"), 1)
                .replacen("%s", &name, 1)
                .replacen("%d", &count.to_string(), 1),
            ReleaseLog::Unloading | ReleaseLog::Unloaded | ReleaseLog::TextureUnloading => {
                native.format.replacen("%s", &name, 1)
            }
        };
        log::info!("native {}::{} line {:#x}: {text}", native.file, native.function, native.line);
        Ok(())
    }
}
impl aa_runtime::model_resource_load::Backend for Current<'_,'_>{
    fn resource_no_ni_objects_current(&mut self)->Result<u8>{
        self.bridge.call(self.execution,|context|context.ui.uvars.resource_no_ni_objects_current())
    }
    fn begin_model_load(&mut self,_resource:&Handle)->Result<()>{
        #[cfg(windows)]{
            crate::windows_reset_os::PerformanceTimer::initialize_frequency()?;
            self.model_timer=Some(crate::windows_reset_os::PerformanceTimer::from_native_constructor().map_err(|e|e.to_string())?);Ok(())
        }
        #[cfg(not(windows))]{Err("Model load timer requires actual Windows QPC service".into())}
    }
    fn decode_selected_model(&mut self,input:&aa_runtime::resource_model_payload::Input)->Result<aa_runtime::resource_model_payload::Payload>{
        //Only an immutable AssetStore Arc crosses this UI reply. The selected
        //bytes and graph work remain on this retained simulation execution.
        let store=self.bridge.call(self.execution,|context|Ok(std::sync::Arc::clone(context.store)))?;
        crate::model_payload_backend::prepare(input,&store,self.precision)
    }
    fn process_model_metadata(&mut self,payload:&aa_runtime::resource_model_payload::Payload,has_skin:bool)->Result<aa_runtime::resource_model_payload::Metadata>{
        payload.process_metadata_with_policy(has_skin,self)
    }
    fn skip_loaded_model_bounds(&mut self)->Result<u8>{
        self.bridge.call(self.execution,|context|context.ui.uvars.resource_caching_current())
    }
}
impl aa_runtime::resource_model_payload::MetadataPolicy for Current<'_,'_>{
    fn renderer_remove_normals_current(&mut self)->Result<u8>{
        self.bridge.call(self.execution,|context|context.ui.uvars.renderer_remove_normals_current())
    }
    fn preprocessed_data_missing_warning_current(&mut self)->Result<u8>{self.bridge.call(self.execution,|context|context.ui.avars.preprocessed_data_missing_warning_current())}
    fn preprocessed_data_wrong_version_warning_current(&mut self)->Result<u8>{self.bridge.call(self.execution,|context|context.ui.avars.preprocessed_data_wrong_version_warning_current())}
}
impl aa_runtime::resource_objects::TexturePolicy for Current<'_, '_> {
    fn data_compatibility_mode_current(&mut self) -> Result<u8> {
        aa_runtime::resource_objects::TexturePolicy::data_compatibility_mode_current(
            &mut battle_texture_services::Current::for_controlled_fresh_start(
                self.bridge,
                self.execution,
            ),
        )
    }
    fn data_missing_resource_warning_current(&mut self) -> Result<u8> {
        aa_runtime::resource_objects::TexturePolicy::data_missing_resource_warning_current(
            &mut battle_texture_services::Current::for_controlled_fresh_start(
                self.bridge,
                self.execution,
            ),
        )
    }
    fn missing_texture_diagnostic_404fc0(&mut self, ids: &[u16], level: u32) -> Result<()> {
        aa_runtime::resource_objects::TexturePolicy::missing_texture_diagnostic_404fc0(
            &mut battle_texture_services::Current::for_controlled_fresh_start(
                self.bridge,
                self.execution,
            ),
            ids,
            level,
        )
    }
    fn texture_cache_source_overrides(
        &mut self,
    ) -> Result<aa_runtime::load_view_variables::NativeOverrides> {
        aa_runtime::resource_objects::TexturePolicy::texture_cache_source_overrides(
            &mut battle_texture_services::Current::for_controlled_fresh_start(
                self.bridge,
                self.execution,
            ),
        )
    }
}
impl aa_runtime::view_assets::Services for Current<'_, '_> {
    fn warning_404ea0(&mut self, line: u32, message: &'static str) -> Result<()> {
        log::warn!("native404EA0 line={line} {message}");
        Ok(())
    }
}
impl aa_runtime::resource_objects::ModelPolicy for Current<'_, '_> {
    fn use_hardware_model_73a560(&mut self) -> Result<bool> {
        aa_runtime::resource_objects::ModelPolicy::use_hardware_model_73a560(
            &mut crate::live_scene::model_policy::UiPolicy::new(self.bridge, self.execution),
        )
    }
    fn hardware_model_suffix_40f7b0(&mut self) -> Result<Vec<u16>> {
        aa_runtime::resource_objects::ModelPolicy::hardware_model_suffix_40f7b0(
            &mut crate::live_scene::model_policy::UiPolicy::new(self.bridge, self.execution),
        )
    }
}
impl Drop for Current<'_, '_> {
    fn drop(&mut self) {
        //Portable cleanup only. Normal/native paths release explicitly in
        //their native order; neither panic nor Err leaves host locks held.
        while let Some(frame) = self.held.pop() {
            drop(frame)
        }
    }
}
