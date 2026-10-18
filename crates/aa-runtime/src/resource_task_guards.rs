//!Portable backend for the SAME ResourceManager's native recursive guards.
//!5B1A10 acquires+4, optional+20(Model ancestry), then+3C;5B0270 uses+20.
//!These are real waiting locks. They neither load a resource nor certify a
//!native cache/decoder, and they do not hold the Manager's field mutex.
use crate::{resource_objects::Handle, resource_task::ThreadHandle};
use std::{
    marker::PhantomData,
    rc::Rc,
    sync::{Arc, Condvar, Mutex, Weak},
    thread::ThreadId,
};
type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Load04,
    Registry20,
    Decode3c,
}
struct Fields {
    owner: Option<ThreadId>,
    depth: usize,
}
struct Recursive {
    fields: Mutex<Fields>,
    wake: Condvar,
}
impl Recursive {
    fn new() -> Self {
        Self {
            fields: Mutex::new(Fields {
                owner: None,
                depth: 0,
            }),
            wake: Condvar::new(),
        }
    }
    fn enter(&self) -> Result<()> {
        let id = std::thread::current().id();
        let mut fields = self
            .fields
            .lock()
            .map_err(|_| "Resource guard state poisoned")?;
        fields = self
            .wake
            .wait_while(fields, |v| v.owner.is_some_and(|owner| owner != id))
            .map_err(|_| "Resource guard wait poisoned")?;
        fields.depth = fields
            .depth
            .checked_add(1)
            .ok_or("Resource recursive guard depth overflow")?;
        fields.owner = Some(id);
        Ok(())
    }
    fn leave(&self) {
        //No outward callback occurs under this tiny metadata mutex. Recover
        //only for cleanup if host unwinding poisoned it; never mint success.
        let mut fields = self.fields.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(
            fields.owner,
            Some(std::thread::current().id()),
            "Resource guard released on another OS thread"
        );
        assert!(fields.depth > 0);
        fields.depth -= 1;
        if fields.depth == 0 {
            fields.owner = None;
            self.wake.notify_one();
        }
    }
}
struct Inner {
    application: Weak<()>,
    guards: [Recursive; 3],
}
pub(crate) struct State {
    inner: Arc<Inner>,
}
impl State {
    pub(crate) fn from_fresh_application(origin: &Arc<()>) -> Self {
        Self {
            inner: Arc::new(Inner {
                application: Arc::downgrade(origin),
                guards: std::array::from_fn(|_| Recursive::new()),
            }),
        }
    }
    pub(crate) fn capability(&self) -> Capability {
        Capability {
            inner: Arc::clone(&self.inner),
        }
    }
}
///Identity-only loan issued by the actual application before borrowing its
///Resource facade. Clones share the SAME three locks; there is no raw ctor.
#[derive(Clone)]
pub struct Capability {
    inner: Arc<Inner>,
}
impl Capability {
    ///The heterogeneous queue passes its opaque SAME-Application key. Both
    ///admitted task vtables have vf10/vf14=false and lock manager+4.
    pub fn acquire_task(&self,application:&crate::resource_task::ApplicationKey,thread:&ThreadHandle)->Result<Lease>{
        application.require(&self.inner.application)?;thread.require_current()?;
        self.inner.guards[index(Kind::Load04)].enter()?;
        Ok(Lease{inner:Arc::clone(&self.inner),kind:Kind::Load04,_same_thread:PhantomData})
    }
    pub fn acquire(&self, resource: &Handle, kind: Kind, thread: &ThreadHandle) -> Result<Lease> {
        let application = self
            .inner
            .application
            .upgrade()
            .ok_or("Resource guard Application is retired")?;
        resource.require_application(&application)?;
        thread.require_current()?;
        self.inner.guards[index(kind)].enter()?;
        Ok(Lease {
            inner: Arc::clone(&self.inner),
            kind,
            _same_thread: PhantomData,
        })
    }
}
fn index(kind: Kind) -> usize {
    match kind {
        Kind::Load04 => 0,
        Kind::Registry20 => 1,
        Kind::Decode3c => 2,
    }
}
///A held recursive lock. !Send prevents releasing on another thread. Drop
///unlocks on ordinary errors/unwind without copying a native field snapshot.
pub struct Lease {
    inner: Arc<Inner>,
    kind: Kind,
    _same_thread: PhantomData<Rc<()>>,
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.inner.guards[index(self.kind)].leave()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recursive_scope_and_real_other_thread_wait() {
        let guard = Arc::new(Recursive::new());
        guard.enter().unwrap();
        guard.enter().unwrap();
        assert_eq!(guard.fields.lock().unwrap().depth, 2);
        let (other, rx) = std::sync::mpsc::channel();
        let worker_guard = Arc::clone(&guard);
        let worker = std::thread::spawn(move || {
            worker_guard.enter().unwrap();
            other.send(()).unwrap();
            worker_guard.leave();
        });
        assert!(matches!(
            rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        guard.leave();
        assert_eq!(guard.fields.lock().unwrap().depth, 1);
        assert!(matches!(
            rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        guard.leave();
        rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        worker.join().unwrap();
        assert!(guard.fields.lock().unwrap().owner.is_none());
    }
    #[cfg(windows)]
    #[test]
    fn controlled_ids_on_real_simulation_thread_same_manager_and_retirement() {
        //Controlled two-ID OS transport. Real Rust threads/identity lifetimes
        //are exercised; this test does not claim native Windows OS capture.
        struct Os {
            main: std::thread::ThreadId,
        }
        impl crate::execution_context::OsThreadIds for Os {
            fn current_thread_id(&mut self) -> Result<u32> {
                Ok(if std::thread::current().id() == self.main {
                    1
                } else {
                    2
                })
            }
        }
        let main = std::thread::current().id();
        let application = Arc::new(
            crate::execution_context::ApplicationThread::capture(&mut Os { main }).unwrap(),
        );
        let origin = Arc::new(());
        let cap = State::from_fresh_application(&origin).capability();
        //Explicit metadata-only Resource constructor input; no file load or
        //native manager startup is certified by this guard lifetime test.
        let resource = Handle::new(
            &origin,
            crate::resource_objects::Kind::Texture,
            vec![65],
            crate::resource_objects::SourceFile::from_catalog(&aa_rules::sources::CatalogEntry {
                logical_path: "laboratory/guard.dds".into(),
                size: 0,
                origin: aa_rules::sources::Origin::V14Loose,
            }),
        );
        let transported = Arc::new(Mutex::new(None));
        let send = Arc::clone(&transported);
        let mut simulation = crate::simulation_execution::SimulationThread::start(
            application,
            Os { main },
            move |execution| {
                let thread = ThreadHandle::on_simulation(execution)?;
                thread.require_current()?;
                *send.lock().unwrap() = Some(thread.clone());
                Ok((thread, resource, cap))
            },
        )
        .unwrap();
        assert!(
            transported
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .require_current()
                .is_err()
        );
        simulation
            .run(|(thread, resource, cap), execution| {
                assert_eq!(thread.os_id(), execution.current_os_thread_id()?);
                let first = cap.acquire(resource, Kind::Registry20, thread)?;
                let second = cap.acquire(resource, Kind::Registry20, thread)?;
                drop(second);
                drop(first);
                Ok(())
            })
            .unwrap();
        drop(origin);
        simulation
            .run(|(thread, resource, cap), _| {
                assert!(cap.acquire(resource, Kind::Load04, thread).is_err());
                Ok(())
            })
            .unwrap();
        drop(simulation);
    }
}
