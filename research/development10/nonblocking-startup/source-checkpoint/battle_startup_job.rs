//!Nonblocking transport for owned, Send source preparation only. Engine,
//!World and captured ApplicationThread owners must stay in their existing
//!startup stages. This ticket is never a native load or readiness receipt.
use std::{
    any::Any,
    sync::{Arc, Mutex, TryLockError},
    thread::{self, JoinHandle},
};
type Result<T> = std::result::Result<T, String>;

enum Outcome<T> { Pending, Ready(Result<T>), Consumed }
pub(super) struct Job<T: Send + 'static> {
    outcome: Arc<Mutex<Outcome<T>>>,
    thread: Option<JoinHandle<()>>,
}
fn panic_message(payload: Box<dyn Any + Send>) -> String {
    let message = payload.downcast_ref::<&str>().copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic");
    format!("source preparation panicked: {message}")
}
impl<T: Send + 'static> Job<T> {
    ///The worker owns source-only inputs. The result slot retains the actual
    ///returned source/failure owner even when a repaint notification is lost.
    pub(super) fn start(name: &str, wake: impl Fn() + Send + 'static,
        work: impl FnOnce() -> T + Send + 'static) -> Result<Self> {
        let outcome = Arc::new(Mutex::new(Outcome::Pending));
        let worker_outcome = Arc::clone(&outcome);
        let thread = thread::Builder::new().name(name.to_owned()).spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work))
                .map_err(panic_message);
            //There is no result-channel send whose disconnected receiver can
            //discard an otherwise retained partial source owner.
            *worker_outcome.lock().unwrap_or_else(|e| e.into_inner()) = Outcome::Ready(result);
            //A repaint is only a notification. Its failure cannot erase the
            //owned result; the GUI also polls while Loading.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(wake));
        }).map_err(|e| format!("cannot start source preparation: {e}"))?;
        Ok(Self { outcome, thread: Some(thread) })
    }
    ///Never waits for either computation or a worker's trailing wake call.
    ///An outcome is delivered once; a native status is owned by Controller.
    pub(super) fn poll(&mut self) -> Result<Option<T>> {
        if self.thread.as_ref().is_some_and(JoinHandle::is_finished) {
            self.thread.take().ok_or("source worker handle was consumed")?
                .join().map_err(panic_message)?;
        }
        let mut slot = match self.outcome.try_lock() {
            Ok(slot) => slot,
            Err(TryLockError::WouldBlock) => return Ok(None),
            Err(TryLockError::Poisoned(error)) => error.into_inner(),
        };
        match std::mem::replace(&mut *slot, Outcome::Consumed) {
            Outcome::Pending => { *slot = Outcome::Pending; Ok(None) }
            Outcome::Ready(value) => value.map(Some),
            Outcome::Consumed => Err("source preparation outcome was already consumed".into()),
        }
    }
}
impl<T: Send + 'static> Drop for Job<T> {
    fn drop(&mut self) {
        if self.thread.as_ref().is_some_and(JoinHandle::is_finished) {
            if let Some(thread) = self.thread.take() { let _ = thread.join(); }
        }
        //Dropping an unfinished source-only JoinHandle detaches it. Explicit
        //GUI/application abandonment retires its eventual source result on
        //that worker; shutdown never waits for source computation. Native
        //World/Application owners are forbidden by this transport's contract.
    }
}
#[cfg(test)]
#[path="battle_startup_job_tests.rs"]
mod tests;
