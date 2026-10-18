//!Private native callback composition. Public construction consumes only a
//!genuine same-Host completed task and every live owner exactly once.
use super::*;
use aa_runtime::{
    catalog::Catalog,
    world_ready_services::{Application, SourceCatalog},
};
use std::sync::Arc;

enum Prefix {
    WorldReady(super::actual::Failure),
    Success {
        ready: super::actual::Ready,
        error: Error,
    },
    Frame(aa_runtime::live_world_frame::BindFailure),
}
pub struct ActivationFailure {
    host: LocalBootstrap,
    services: Application,
    source: Arc<SourceCatalog>,
    prefix: Prefix,
    error: String,
}
impl ActivationFailure {
    pub fn error(&self) -> &str {
        &self.error
    }
    pub fn host(&self) -> &LocalBootstrap {
        &self.host
    }
    pub fn services(&self) -> &Application {
        &self.services
    }
}
impl std::fmt::Debug for ActivationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActivationFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}
///Successful775DB0 callbacks own the same Host, real live World and current
///application services. No Clone/Default/public field or raw-ready constructor.
///Native Sim29 can still be frozen while the real view/task status protocol
///finishes; this owner never substitutes an unconditional unfreeze.
pub struct ActualActivatedSession {
    host: LocalBootstrap,
    world: aa_runtime::live_world_frame::World<'static>,
    services: Application,
}
impl ActualActivatedSession {
    pub fn host(&self) -> &LocalBootstrap {
        &self.host
    }
    pub fn world(&self) -> &aa_runtime::live_world_frame::World<'static> {
        &self.world
    }
    pub fn services(&self) -> &Application {
        &self.services
    }
    ///Short sibling loans; owners stay in this allocation between frames.
    pub fn parts(
        &mut self,
    ) -> (
        &mut LocalBootstrap,
        &mut aa_runtime::live_world_frame::World<'static>,
        &mut Application,
    ) {
        (&mut self.host, &mut self.world, &mut self.services)
    }
}
impl LocalBootstrap {
    ///775DB0 status2: actual7757F0, actual7738B0, clear SAME124, then move the
    ///actual initialized World into its persistent frame owner. The Filetime
    ///callback supplies only the real OS input to6FB1F0; all game services are
    ///concrete. No injected success callback can mint this return type.
    pub fn finish_completed_world<T, C>(
        mut self,
        task: super::super::load_task::SuccessfulTask,
        catalog: &Catalog<'_>,
        mut services: Application,
        source: Arc<SourceCatalog>,
        notifications: &Notifications,
        time: &mut crate::local_host::NodeTime<T>,
        precision: aa_numeric::Precision,
        current_local_filetime: &mut C,
        diagnostics: &mut Vec<super::actual::Diagnostic>,
        success_diagnostics: &mut Vec<super::success::SuccessDiagnostic>,
    ) -> std::result::Result<ActualActivatedSession, ActivationFailure>
    where
        T: crate::local_host::MillisecondTimer,
        C: FnMut() -> std::result::Result<[u32; 2], String>,
    {
        let ready = match self.finish_world_ready_task(
            task,
            catalog,
            &mut services,
            current_local_filetime,
            diagnostics,
        ) {
            Ok(ready) => ready,
            Err(failure) => {
                return Err(ActivationFailure {
                    error: failure.error().into(),
                    host: self,
                    services,
                    source,
                    prefix: Prefix::WorldReady(failure),
                });
            }
        };
        let complete = match self.finish_world_success_callback(
            ready,
            catalog,
            notifications,
            time,
            precision,
            success_diagnostics,
        ) {
            Ok(complete) => complete,
            Err((ready, error)) => {
                return Err(ActivationFailure {
                    error: error.to_string(),
                    host: self,
                    services,
                    source,
                    prefix: Prefix::Success { ready, error },
                });
            }
        };
        let world = match aa_runtime::live_world_frame::World::from_sai_complete_owned(
            complete.frame,
            catalog,
            services.depot_mut().sources_mut(),
            Arc::clone(&source),
            precision,
        ) {
            Ok(world) => world,
            Err(failure) => {
                return Err(ActivationFailure {
                    error: failure.error().into(),
                    host: self,
                    services,
                    source,
                    prefix: Prefix::Frame(failure),
                });
            }
        };
        Ok(ActualActivatedSession {
            host: self,
            world,
            services,
        })
    }
}
