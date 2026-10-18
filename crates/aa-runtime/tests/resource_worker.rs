//! Actual Windows OS identities and parent-before-task barrier/cleanup checks.
#![cfg(windows)]
use aa_runtime::execution_context::{ApplicationThread, OsThreadIds, ResourceWorker};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
#[derive(Clone, Copy)]
struct WindowsOs;
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentThreadId() -> u32;
}
impl OsThreadIds for WindowsOs {
    fn current_thread_id(&mut self) -> Result<u32, String> {
        // Same actual Kernel32 service used by the original's ID getter.
        Ok(unsafe { GetCurrentThreadId() })
    }
}
#[test]
fn actual_os_worker_waits_for_parent_registration_and_reads_current_ids() {
    let mut os = WindowsOs;
    let application = Arc::new(ApplicationThread::capture(&mut os).unwrap());
    let app_id = application.id();
    let registered = Arc::new(AtomicBool::new(false));
    let child_registered = Arc::clone(&registered);
    let parent_registered = Arc::clone(&registered);
    let worker = ResourceWorker::start(
        application,
        os,
        move |identity| {
            assert_eq!(identity.parent_os_thread_id(), app_id);
            assert_eq!(WindowsOs.current_thread_id().unwrap(), app_id);
            assert_ne!(identity.child_os_thread_id(), app_id);
            assert!(!parent_registered.swap(true, Ordering::SeqCst));
            Ok(())
        },
        move |execution| {
            assert!(child_registered.load(Ordering::SeqCst));
            let first = execution.current_os_thread_id()?;
            assert_ne!(first, app_id);
            assert_eq!(first, execution.identity().child_os_thread_id());
            assert_eq!(execution.application_os_thread_id(), app_id);
            assert_eq!(execution.current_os_thread_id()?, first);
            Ok(first)
        },
    )
    .unwrap();
    let child_id = worker.identity().child_os_thread_id();
    assert_eq!(worker.join().unwrap(), child_id);
    assert!(registered.load(Ordering::SeqCst));
}
#[test]
fn startup_and_registration_failures_do_not_release_the_task() {
    let app = Arc::new(ApplicationThread::capture(&mut WindowsOs).unwrap());
    struct FailedChild {
        reads: u32,
    }
    impl OsThreadIds for FailedChild {
        fn current_thread_id(&mut self) -> Result<u32, String> {
            self.reads += 1;
            if self.reads == 1 {
                WindowsOs.current_thread_id()
            } else {
                Err("required child OS getter failed".into())
            }
        }
    }
    let called = Arc::new(AtomicBool::new(false));
    let copy = Arc::clone(&called);
    let result = ResourceWorker::start(
        Arc::clone(&app),
        FailedChild { reads: 0 },
        |_| panic!("unreached registration"),
        move |_| {
            copy.store(true, Ordering::SeqCst);
            Ok(())
        },
    );
    assert!(matches!(result,Err(e)if e=="required child OS getter failed"));
    assert!(!called.load(Ordering::SeqCst));
    let copy = Arc::clone(&called);
    let result = ResourceWorker::start(
        Arc::clone(&app),
        WindowsOs,
        |_| Err("parent registration failed".into()),
        move |_| {
            copy.store(true, Ordering::SeqCst);
            Ok(())
        },
    );
    assert!(matches!(result,Err(e)if e=="parent registration failed"));
    assert!(!called.load(Ordering::SeqCst));
    let copy = Arc::clone(&called);
    let result = ResourceWorker::start(
        app,
        WindowsOs,
        |_| panic!("parent registration panic"),
        move |_| {
            copy.store(true, Ordering::SeqCst);
            Ok(())
        },
    );
    assert!(matches!(result,Err(e)if e.contains("parent registration panic")));
    assert!(!called.load(Ordering::SeqCst));
}
#[test]
fn task_errors_and_panics_are_returned_by_join() {
    let app = Arc::new(ApplicationThread::capture(&mut WindowsOs).unwrap());
    let worker = ResourceWorker::<()>::start(
        Arc::clone(&app),
        WindowsOs,
        |_| Ok(()),
        |_| Err("load failed after native partial stores".into()),
    )
    .unwrap();
    assert_eq!(
        worker.join().unwrap_err(),
        "load failed after native partial stores"
    );
    let worker =
        ResourceWorker::<()>::start(app, WindowsOs, |_| Ok(()), |_| panic!("worker task panic"))
            .unwrap();
    assert!(worker.join().unwrap_err().contains("worker task panic"));
}

#[test]
fn scoped_actual_worker_mutates_same_borrowed_owner_and_retains_join_result() {
    use aa_runtime::execution_context::ScopedResourceWorker;
    let application=Arc::new(ApplicationThread::capture(&mut WindowsOs).unwrap());
    let mut same=Vec::from([7u32]);
    let allocation=same.as_ptr() as usize;
    let registered=AtomicBool::new(false);
    std::thread::scope(|scope| {
        let flag=&registered;
        let worker=ScopedResourceWorker::start(scope,Arc::clone(&application),WindowsOs,
            |identity| {assert_ne!(identity.child_os_thread_id(),application.id());flag.store(true,Ordering::SeqCst);Ok(())},
            |execution| {
                assert!(flag.load(Ordering::SeqCst));
                assert_eq!(same.as_ptr() as usize,allocation);
                assert_eq!(execution.current_os_thread_id()?,execution.identity().child_os_thread_id());
                same[0]=23;Ok(91u32)
            }).unwrap();
        let child=worker.identity().child_os_thread_id();
        let complete=worker.join_complete();
        assert_eq!(complete.identity().child_os_thread_id(),child);
        assert_eq!(complete.outcome(),Ok(&91));
        assert_eq!(complete.into_outcome().unwrap(),91);
    });
    assert_eq!(same,[23]);assert_eq!(same.as_ptr() as usize,allocation);
    std::thread::scope(|scope| {
        let worker=ScopedResourceWorker::start(scope,application,WindowsOs,|_|Ok(()),|_| {
            same[0]=31;Err::<(),_>("actual failed prefix".into())
        }).unwrap();
        let completed=worker.join_complete();
        assert_eq!(completed.outcome(),Err("actual failed prefix"));
    });
    assert_eq!(same,[31]);
}
