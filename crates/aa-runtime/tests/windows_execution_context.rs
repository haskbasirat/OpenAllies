//! Real Windows identity and current-thread control producers.
#[cfg(windows)]
#[path="../../aa-client/src/windows_reset_os.rs"]
mod windows_reset_os;
#[path="../../aa-client/src/runtime_fp.rs"]
mod runtime_fp;
#[cfg(windows)]
#[test]
fn actual_os_ids_distinguish_live_application_and_worker_threads() {
    use aa_runtime::execution_context::{ApplicationThread,OsThreadIds};
    let mut os=windows_reset_os::WindowsThreadIds;
    let application=ApplicationThread::capture(&mut os).unwrap();
    assert_eq!(application.id(),os.current_thread_id().unwrap());
    let child=std::thread::spawn(|| {
        let mut os=windows_reset_os::WindowsThreadIds;
        let captured=ApplicationThread::capture(&mut os).unwrap();
        assert_eq!(captured.id(),os.current_thread_id().unwrap());
        captured.id()
    }).join().unwrap();
    assert_ne!(application.id(),child);
    assert_eq!(application.id(),os.current_thread_id().unwrap());
    //This tests real ID capture only, not native ASYNC scheduling/readiness.
}
