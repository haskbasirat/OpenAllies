//! Retained App+150 simulation-thread identity and an owned thread-local job queue.
//!
//! Native41ABCF..41ABDC constructs/starts that embedded thread through85E7B0
//! and85E430. Its actual OS ID at+48 is globalB39828. App+1F8 resource
//! workers and the main-thread owner are distinct allocations/roles.
//! This portable queue does not implement the native recurring networking/
//! simulation loop. It preserves one real OS thread and thread-local state
//! for subsequently bound work; state need not be Send or leave that thread.
use crate::execution_context::{ApplicationThread, OsThreadIds};
use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

#[derive(Debug)]
struct Identity {
    application: Arc<ApplicationThread>,
    parent: u32,
    child: u32,
    alive: AtomicBool,
}
//Rust lifetime validation only: do not authorize a reused OS ID after exit.
struct Running(Arc<Identity>);
impl Drop for Running {
    fn drop(&mut self) {
        self.0.alive.store(false, Ordering::Release);
    }
}

///Weak reference to this exact retained thread allocation, never a raw role ID.
#[derive(Clone, Debug)]
pub struct SimulationIdentity(Weak<Identity>);
impl SimulationIdentity {
    fn owner(&self) -> Result<Arc<Identity>, String> {
        let owner = self
            .0
            .upgrade()
            .ok_or_else(|| "simulation thread has stopped".to_string())?;
        if !owner.alive.load(Ordering::Acquire) {
            return Err("simulation thread has stopped".into());
        }
        Ok(owner)
    }
    pub fn os_thread_id(&self) -> Result<u32, String> {
        Ok(self.owner()?.child)
    }
    pub fn parent_os_thread_id(&self) -> Result<u32, String> {
        Ok(self.owner()?.parent)
    }
    ///Reject another application allocation even when an OS ID happens to match.
    pub fn require_application(&self, application: &ApplicationThread) -> Result<(), String> {
        if !std::ptr::eq(self.owner()?.application.as_ref(), application) {
            return Err("simulation identity belongs to another application owner".into());
        }
        Ok(())
    }
    ///Native77DA70/77DB10: current-ID comparison precedes both SAI reads.
    ///The closure must borrow the actual retained SAI or certify its absence.
    pub fn sync_gate(
        &self,
        os: &mut dyn OsThreadIds,
        sai_flags: impl FnOnce() -> Result<Option<[u8; 2]>, String>,
    ) -> Result<bool, String> {
        let owner = self.owner()?;
        if os.current_thread_id()? != owner.child {
            return Ok(false);
        }
        Ok(match sai_flags()? {
            None => true,
            Some([a, b]) => a == 0 && b == 0,
        })
    }
}

///Borrowed only on the actual retained simulation OS thread.
pub struct Execution<'a> {
    identity: &'a Arc<Identity>,
    os: &'a mut dyn OsThreadIds,
}
impl Execution<'_> {
    pub fn identity(&self) -> SimulationIdentity {
        SimulationIdentity(Arc::downgrade(self.identity))
    }
    pub fn current_os_thread_id(&mut self) -> Result<u32, String> {
        self.os.current_thread_id()
    }
    pub fn application_os_thread_id(&self) -> u32 {
        self.identity.application.id()
    }
    pub fn sync_gate(
        &mut self,
        sai_flags: impl FnOnce() -> Result<Option<[u8; 2]>, String>,
    ) -> Result<bool, String> {
        self.identity().sync_gate(self.os, sai_flags)
    }
}
type Job<S> = Box<dyn FnOnce(&mut S, &mut Execution<'_>) + Send + 'static>;

///Move-only owner. Dropping closes the queue and joins the same actual thread.
///Only Send jobs/results cross the channel; S is initialized and kept inside.
pub struct SimulationThread<S: 'static> {
    identity: Arc<Identity>,
    sender: Option<mpsc::Sender<Job<S>>>,
    handle: Option<std::thread::JoinHandle<()>>,
}
impl<S: 'static> SimulationThread<S> {
    pub fn start<O, F>(
        application: Arc<ApplicationThread>,
        mut os: O,
        initialize: F,
    ) -> Result<Self, String>
    where
        O: OsThreadIds + Send + 'static,
        F: FnOnce(&mut Execution<'_>) -> Result<S, String> + Send + 'static,
    {
        let parent = os.current_thread_id()?;
        if parent != application.id() {
            return Err("simulation startup must run on its retained application thread".into());
        }
        let (ready_tx, ready_rx) = mpsc::sync_channel(0);
        let (release_tx, release_rx) = mpsc::sync_channel(0);
        let (initialized_tx, initialized_rx) = mpsc::sync_channel(0);
        let (sender, receiver) = mpsc::channel::<Job<S>>();
        let handle = std::thread::Builder::new()
            .name("Simulation".into())
            .spawn(move || {
                let child = match os.current_thread_id() {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                let identity = Arc::new(Identity {
                    application,
                    parent,
                    child,
                    alive: AtomicBool::new(true),
                });
                let _running = Running(Arc::clone(&identity));
                if ready_tx.send(Ok(Arc::clone(&identity))).is_err() || release_rx.recv().is_err() {
                    return;
                }
                let mut execution = Execution {
                    identity: &identity,
                    os: &mut os,
                };
                let mut state = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    initialize(&mut execution)
                })) {
                    Ok(Ok(v)) => v,
                    Ok(Err(e)) => {
                        let _ = initialized_tx.send(Err(e));
                        return;
                    }
                    Err(_) => {
                        let _ = initialized_tx.send(Err("simulation initializer panicked".into()));
                        return;
                    }
                };
                if initialized_tx.send(Ok(())).is_err() {
                    return;
                }
                for job in receiver {
                    job(&mut state, &mut execution);
                }
            })
            .map_err(|e| format!("creating simulation OS thread: {e}"))?;
        let identity = match ready_rx.recv() {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => {
                let _ = handle.join();
                return Err(e);
            }
            Err(_) => {
                let _ = handle.join();
                return Err("simulation thread did not publish its OS identity".into());
            }
        };
        //85E430: actual child identity and parent registration exist before resume.
        //The Arc relation is portable lifetime metadata, not a native thread handle.
        if identity.child == parent {
            drop(release_tx);
            let _ = handle.join();
            return Err("created simulation OS thread aliases its parent".into());
        }
        if release_tx.send(()).is_err() {
            let _ = handle.join();
            return Err("simulation thread stopped before release".into());
        }
        match initialized_rx.recv() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                let _ = handle.join();
                return Err(e);
            }
            Err(_) => {
                let _ = handle.join();
                return Err("simulation initializer stopped".into());
            }
        }
        Ok(Self {
            identity,
            sender: Some(sender),
            handle: Some(handle),
        })
    }
    pub fn identity(&self) -> SimulationIdentity {
        SimulationIdentity(Arc::downgrade(&self.identity))
    }
    ///Execute a bound operation on the SAME state; only its Send result returns.
    pub fn run<T: Send + 'static>(
        &mut self,
        job: impl FnOnce(&mut S, &mut Execution<'_>) -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        let (tx, rx) = mpsc::sync_channel(0);
        self.sender
            .as_ref()
            .ok_or("simulation queue is closed")?
            .send(Box::new(move |state, execution| {
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    job(state, execution)
                }))
                .unwrap_or_else(|_| Err("simulation job panicked; same state is retained".into()));
                let _ = tx.send(outcome);
            }))
            .map_err(|_| "simulation OS thread has stopped".to_string())?;
        rx.recv()
            .map_err(|_| "simulation job stopped without an outcome".to_string())?
    }
}
impl<S: 'static> Drop for SimulationThread<S> {
    fn drop(&mut self) {
        drop(self.sender.take());
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
