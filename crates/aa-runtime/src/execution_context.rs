//!85FA70 MainThread+48 and the source-proven Application+1F8 ASYNC worker.
//! Capture the actual frontend OS thread once.4296B0 must read the current
//! OS ID again at each event and compare it to this retained identity.
use std::sync::{Arc, mpsc};
pub trait OsThreadIds {
    /// The current Win32 GetCurrentThreadId result. Precision and logical
    /// simulation roles cannot substitute for this operating-system service.
    fn current_thread_id(&mut self) -> Result<u32, String>;
}
/// The single captured MainThread identity. No raw-ID/default/role constructor
/// is exposed; sharing this immutable owner does not create another OS thread.
#[derive(Debug)]
pub struct ApplicationThread {
    id: u32,
}
impl ApplicationThread {
    ///85FA70/85FAA9..AF: retain the actual current-ID callback result. The
    ///remaining TGC_MainThread constructor/handle ownership is separate work.
    pub fn capture(os: &mut impl OsThreadIds) -> Result<Self, String> {
        Ok(Self {
            id: os.current_thread_id()?,
        })
    }
    pub fn id(&self) -> u32 {
        self.id
    }
}

/// The same captured application owner and actual parent/child OS identities.
/// Only the worker startup handshake can construct this immutable receipt.
#[derive(Debug)]
pub struct WorkerIdentity {
    job: Arc<()>,
    application: Arc<ApplicationThread>,
    parent: u32,
    child: u32,
}
impl WorkerIdentity {
    pub fn witness(&self) -> WorkerWitness {
        WorkerWitness(Arc::downgrade(&self.job))
    }
    pub fn parent_os_thread_id(&self) -> u32 {
        self.parent
    }
    pub fn child_os_thread_id(&self) -> u32 {
        self.child
    }
    pub fn application(&self) -> &ApplicationThread {
        &self.application
    }
}
///Stable logical identity of this exact OS job, including after its thread
///exits. Reused OS IDs and reused allocation addresses cannot match it.
#[derive(Clone)]
pub struct WorkerWitness(std::sync::Weak<()>);
impl WorkerWitness {
    pub(crate) fn is_live(&self)->bool {self.0.strong_count()!=0}
    pub fn is_for(&self, identity: &WorkerIdentity) -> bool {
        self.0.strong_count() != 0 && self.0.ptr_eq(&Arc::downgrade(&identity.job))
    }
}

/// Borrowed, current OS-ID service for a running load task. Every current-ID
/// request calls the OS provider again, as4296B0 does; it never returns a role
/// constant or a cached startup ID. Time remains owned by the caller's actual
/// View+8C/World+E0 services, selected after comparing the two real identities.
pub struct LoadExecution<'a> {
    identity: &'a WorkerIdentity,
    os: &'a mut dyn OsThreadIds,
}
/// Object-safe borrow of the current execution. Production implements this
/// with LoadExecution over its actual OS provider and retained application.
pub trait ExecutionIds {
    fn current_os_thread_id(&mut self) -> Result<u32, String>;
    fn application_os_thread_id(&self) -> u32;
}
impl ExecutionIds for LoadExecution<'_> {
    fn current_os_thread_id(&mut self) -> Result<u32, String> {
        LoadExecution::current_os_thread_id(self)
    }
    fn application_os_thread_id(&self) -> u32 {
        LoadExecution::application_os_thread_id(self)
    }
}
impl LoadExecution<'_> {
    pub fn identity(&self) -> &WorkerIdentity {
        self.identity
    }
    pub fn current_os_thread_id(&mut self) -> Result<u32, String> {
        self.os.current_thread_id()
    }
    pub fn application_os_thread_id(&self) -> u32 {
        self.identity.application.id()
    }
}

/// One native-source-proven ASYNC LoadGame job on a real operating-system
/// thread.85E430 creates the child suspended, registers it with the actual
/// parent and calls OnCreated before resume; this driver admits only OS-ID
/// capture before its parent registration callback and release barrier.
///
/// This is a single-job driver, not a claim to port all TGC_Thread signals,
/// recurring manager queue operations, Win32 priority or exception handling.
/// It does not infer any x87 precision profile from the thread's name or role.
pub struct ResourceWorker<T> {
    identity: Arc<WorkerIdentity>,
    handle: Option<std::thread::JoinHandle<Result<T, String>>>,
}
impl<T: Send + 'static> ResourceWorker<T> {
    pub fn start<O, C, F>(
        application: Arc<ApplicationThread>,
        mut os: O,
        on_created: C,
        task: F,
    ) -> Result<Self, String>
    where
        O: OsThreadIds + Send + 'static,
        C: FnOnce(&WorkerIdentity) -> Result<(), String>,
        F: FnOnce(&mut LoadExecution<'_>) -> Result<T, String> + Send + 'static,
    {
        let parent = os.current_thread_id()?;
        let (started_tx, started_rx) = mpsc::sync_channel(0);
        let (release_tx, release_rx) = mpsc::sync_channel(0);
        let handle = std::thread::Builder::new()
            .name("ASYNC".into())
            .spawn(move || {
                let child = match os.current_thread_id() {
                    Ok(id) => id,
                    Err(error) => {
                        let _ = started_tx.send(Err(error.clone()));
                        return Err(error);
                    }
                };
                let identity = Arc::new(WorkerIdentity {
                    job: Arc::new(()),
                    application,
                    parent,
                    child,
                });
                started_tx
                    .send(Ok(Arc::clone(&identity)))
                    .map_err(|_| "ASYNC parent disappeared during startup".to_string())?;
                release_rx.recv().map_err(|_| {
                    "ASYNC parent did not release its startup barrier".to_string()
                })??;
                task(&mut LoadExecution {
                    identity: &identity,
                    os: &mut os,
                })
            })
            .map_err(|error| format!("creating ASYNC operating-system thread: {error}"))?;
        let identity = match started_rx.recv() {
            Ok(Ok(identity)) => identity,
            Ok(Err(error)) => {
                let _ = handle.join();
                return Err(error);
            }
            Err(_) => {
                return match handle.join() {
                    Err(error) => Err(panic_error(error)),
                    Ok(_) => Err("ASYNC exited without publishing its captured identity".into()),
                };
            }
        };
        let registration =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| on_created(&identity)))
                .map_err(panic_error)
                .and_then(|result| result);
        if let Err(error) = registration {
            let _ = release_tx.send(Err(error.clone()));
            let _ = handle.join();
            return Err(error);
        }
        if release_tx.send(Ok(())).is_err() {
            return match handle.join() {
                Err(error) => Err(panic_error(error)),
                Ok(_) => Err("ASYNC exited before parent registration was released".into()),
            };
        }
        Ok(Self {
            identity,
            handle: Some(handle),
        })
    }
    pub fn identity(&self) -> &WorkerIdentity {
        &self.identity
    }
    pub fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(|h| h.is_finished())
    }
    pub fn join(mut self) -> Result<T, String> {
        self.handle
            .take()
            .expect("ResourceWorker handle consumed once")
            .join()
            .map_err(panic_error)?
    }
    ///The owned sibling of ScopedResourceWorker::join_complete. Poll
    ///is_finished first on an event-loop caller; the same actual job identity
    ///and outcome are retained, including an error or panic.
    pub fn join_complete(mut self) -> CompletedWorker<T> {
        let result = self.handle.take().expect("worker is joined once")
            .join().map_err(panic_error).and_then(|result| result);
        CompletedWorker { identity: Arc::clone(&self.identity), result }
    }
}
impl<T> Drop for ResourceWorker<T> {
    fn drop(&mut self) {
        // Reap the same child; explicit join is required to observe task errors.
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
fn panic_error(error: Box<dyn std::any::Any + Send>) -> String {
    let message = error
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| error.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic");
    format!("ASYNC worker panicked: {message}")
}

///Receipt of a real joined job, never a native map/task success certificate.
///Only a genuine StartedMission result can satisfy the native map caller.
pub struct CompletedWorker<T> {
    identity: Arc<WorkerIdentity>,
    result: Result<T, String>,
}
impl<T> CompletedWorker<T> {
    pub fn identity(&self) -> &WorkerIdentity {
        &self.identity
    }
    pub fn outcome(&self) -> Result<&T, &str> {
        self.result.as_ref().map_err(String::as_str)
    }
    pub fn into_outcome(self) -> Result<T, String> {
        self.result
    }
}
///Scoped form of the same actual OS worker/barrier. The application may lend
///its real source Catalog, Sim and disjoint Session fields without an unsafe
///lifetime extension or a replacement snapshot. The scope joins before return.
pub struct ScopedResourceWorker<'scope, T> {
    identity: Arc<WorkerIdentity>,
    handle: Option<std::thread::ScopedJoinHandle<'scope, Result<T, String>>>,
}
impl<'scope, T: Send + 'scope> ScopedResourceWorker<'scope, T> {
    pub fn start<'env: 'scope, O, C, F>(
        scope: &'scope std::thread::Scope<'scope, 'env>,
        application: Arc<ApplicationThread>,
        mut os: O,
        on_created: C,
        task: F,
    ) -> Result<Self, String>
    where
        O: OsThreadIds + Send + 'scope,
        C: FnOnce(&WorkerIdentity) -> Result<(), String>,
        F: FnOnce(&mut LoadExecution<'_>) -> Result<T, String> + Send + 'scope,
    {
        let parent = os.current_thread_id()?;
        let (started_tx, started_rx) = mpsc::sync_channel(0);
        let (release_tx, release_rx) = mpsc::sync_channel(0);
        let handle = std::thread::Builder::new()
            .name("ASYNC".into())
            .spawn_scoped(scope, move || {
                let child = match os.current_thread_id() {
                    Ok(id) => id,
                    Err(error) => {
                        let _ = started_tx.send(Err(error.clone()));
                        return Err(error);
                    }
                };
                let identity = Arc::new(WorkerIdentity {
                    job: Arc::new(()),
                    application,
                    parent,
                    child,
                });
                started_tx
                    .send(Ok(Arc::clone(&identity)))
                    .map_err(|_| "scoped ASYNC parent disappeared during startup".to_owned())?;
                release_rx.recv().map_err(|_| {
                    "scoped ASYNC parent did not release startup barrier".to_owned()
                })??;
                task(&mut LoadExecution {
                    identity: &identity,
                    os: &mut os,
                })
            })
            .map_err(|error| format!("creating scoped ASYNC operating-system thread: {error}"))?;
        let identity = match started_rx.recv() {
            Ok(Ok(identity)) => identity,
            Ok(Err(error)) => {
                let _ = handle.join();
                return Err(error);
            }
            Err(_) => {
                return match handle.join() {
                    Err(error) => Err(panic_error(error)),
                    Ok(_) => Err("scoped ASYNC exited before publishing its OS identity".into()),
                };
            }
        };
        let registration =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| on_created(&identity)))
                .map_err(panic_error)
                .and_then(|value| value);
        if let Err(error) = registration {
            let _ = release_tx.send(Err(error.clone()));
            let _ = handle.join();
            return Err(error);
        }
        if release_tx.send(Ok(())).is_err() {
            return match handle.join() {
                Err(error) => Err(panic_error(error)),
                Ok(_) => Err("scoped ASYNC exited before parent registration was released".into()),
            };
        }
        Ok(Self {
            identity,
            handle: Some(handle),
        })
    }
    pub fn identity(&self) -> &WorkerIdentity {
        &self.identity
    }
    pub fn is_finished(&self) -> bool {
        self.handle
            .as_ref()
            .is_none_or(|handle| handle.is_finished())
    }
    ///Always records the actual join outcome. A task Err/panic is retained,
    ///never reinterpreted as a completed native map operation.
    pub fn join_complete(mut self) -> CompletedWorker<T> {
        let result = self
            .handle
            .take()
            .expect("scoped worker is joined once")
            .join()
            .map_err(panic_error)
            .and_then(|result| result);
        CompletedWorker {
            identity: Arc::clone(&self.identity),
            result,
        }
    }
}
impl<T> Drop for ScopedResourceWorker<'_, T> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
