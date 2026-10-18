//!Exact production ResourceWorker plus its existing actual-Windows tests.
extern crate self as aa_runtime;
#[path="../../../crates/aa-runtime/src/execution_context.rs"]
pub mod execution_context;
#[path="../../../crates/aa-runtime/tests/resource_worker.rs"]
mod existing_worker_tests;

#[cfg(windows)]
#[test]
fn owned_completion_is_polled_after_parent_pumps_reply_and_retains_error_identity() {
    use execution_context::{ApplicationThread,OsThreadIds,ResourceWorker};
    use std::sync::{Arc,mpsc};
    struct Os;
    #[link(name="kernel32")]
    unsafe extern "system" {fn GetCurrentThreadId()->u32;}
    impl OsThreadIds for Os {
        fn current_thread_id(&mut self)->Result<u32,String>{Ok(unsafe{GetCurrentThreadId()})}
    }
    let application=Arc::new(ApplicationThread::capture(&mut Os).unwrap());
    let parent=application.id();
    let(request,requests)=mpsc::channel();
    let(reply,replies)=mpsc::channel();
    let worker=ResourceWorker::<()>::start(Arc::clone(&application),Os,
        |identity|{assert_eq!(Os.current_thread_id()?,parent);assert_eq!(identity.parent_os_thread_id(),parent);Ok(())},
        move|execution|{
            request.send(execution.current_os_thread_id()?).unwrap();
            let error=replies.recv().unwrap();Err(error)
        }).unwrap();
    let child=requests.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
    assert_ne!(child,parent);
    assert!(!worker.is_finished());
    //The event-loop caller can service the reply instead of joining here.
    assert_eq!(Os.current_thread_id().unwrap(),parent);
    reply.send("retained actual task error".to_owned()).unwrap();
    while !worker.is_finished(){std::thread::yield_now();}
    let complete=worker.join_complete();
    assert_eq!(complete.identity().parent_os_thread_id(),parent);
    assert_eq!(complete.identity().child_os_thread_id(),child);
    assert!(std::ptr::eq(complete.identity().application(),&*application));
    assert_eq!(complete.outcome().unwrap_err(),"retained actual task error");
}
