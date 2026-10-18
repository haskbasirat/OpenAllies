use super::*;
use std::{sync::{mpsc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
const DEADLINE: Duration = Duration::from_secs(5);
#[derive(Debug)]
struct Prefix { identity: Arc<()>, retired: Arc<AtomicBool> }
impl Drop for Prefix { fn drop(&mut self) { self.retired.store(true, Ordering::Release); } }
#[derive(Debug)]
struct ReachedError { message: String, prefix: Prefix }
fn poll_until<T: Send + 'static>(job: &mut Job<T>) -> Result<T> {
    let deadline = Instant::now() + DEADLINE;
    loop {
        if let Some(value) = job.poll()? { return Ok(value); }
        assert!(Instant::now() < deadline, "source result was not delivered");
        thread::yield_now();
    }
}

#[test]
fn pending_source_job_allows_gui_pump_and_returns_same_failure_owner() {
    let (request_tx, request_rx) = mpsc::channel::<mpsc::Sender<()>>();
    let identity = Arc::new(());
    let retired = Arc::new(AtomicBool::new(false));
    let prefix = Prefix { identity: Arc::clone(&identity), retired: Arc::clone(&retired) };
    let woke = Arc::new(AtomicBool::new(false)); let wake_flag = Arc::clone(&woke);
    let mut job = Job::start("source handshake test", move || { wake_flag.store(true, Ordering::Release); }, move || {
        let (reply, wait) = mpsc::channel();
        request_tx.send(reply).unwrap();
        wait.recv().unwrap();
        Err::<(), _>(ReachedError { message: "reached source child unavailable".into(), prefix })
    }).unwrap();
    //The message barrier proves that the worker is waiting for GUI work.
    //This is a transport fixture, not a game World or a readiness owner.
    let gui_request = request_rx.recv_timeout(DEADLINE).unwrap();
    let mut gui_frames = 0;
    for _ in 0..3 { assert!(job.poll().unwrap().is_none()); gui_frames += 1; }
    assert_eq!(gui_frames, 3); assert!(!retired.load(Ordering::Acquire));
    gui_request.send(()).unwrap();
    let failure = match poll_until(&mut job).unwrap() { Err(failure) => failure, Ok(()) => panic!("source stop became success") };
    assert_eq!(failure.message, "reached source child unavailable");
    assert!(Arc::ptr_eq(&failure.prefix.identity, &identity));
    assert!(!retired.load(Ordering::Acquire));
    assert!(job.poll().unwrap_err().contains("already consumed"));
    drop(failure); assert!(retired.load(Ordering::Acquire));
    //A result may be observed before the harmless trailing wake returns.
    let deadline = Instant::now() + DEADLINE;
    while !woke.load(Ordering::Acquire) { assert!(Instant::now() < deadline); thread::yield_now(); }
}

#[test]
fn poll_delivers_ready_result_without_joining_blocked_wake() {
    let (wake_entered, entered) = mpsc::channel();
    let (release, wake_release) = mpsc::channel();
    let (poll_entered, polling) = mpsc::channel();
    let (outcome, observed) = mpsc::channel();
    let gui = thread::spawn(move || {
        let mut job = Job::start("blocked wake test", move || {
            wake_entered.send(()).unwrap(); wake_release.recv().unwrap();
        }, || 17u32).unwrap();
        //Wait only in this harness. The production GUI never does this.
        entered.recv_timeout(DEADLINE).unwrap();
        assert!(!job.thread.as_ref().unwrap().is_finished());
        poll_entered.send(()).unwrap();
        outcome.send(job.poll()).unwrap();
    });
    polling.recv_timeout(DEADLINE).unwrap();
    let result = observed.recv_timeout(DEADLINE);
    //Always release/join before asserting, so a regression fails instead of
    //leaving a hanging test process. No fixed sleep drives the ordering.
    release.send(()).unwrap(); gui.join().unwrap();
    assert_eq!(result.unwrap().unwrap(), Some(17));
}

#[test]
fn blocking_wait_pattern_prevents_gui_reply_until_external_rescue() {
    let (request, gui_requests) = mpsc::channel();
    let (waiting, wait_entered) = mpsc::channel();
    let (outcome, completed) = mpsc::channel();
    let gui = thread::spawn(move || {
        let mut job = Job::start("blocking predecessor fixture", || {}, move || {
            let (reply, receive) = mpsc::channel(); request.send(reply).unwrap();
            receive.recv().unwrap(); "retained source result"
        }).unwrap();
        waiting.send(()).unwrap();
        //Explicit predecessor-pattern probe: the original GUI's scoped join
        //cannot pump while waiting here. This is NOT a claim that Berlin's
        //observed preactivation starvation already reached a UI callback cycle.
        job.thread.take().unwrap().join().unwrap();
        outcome.send(job.poll()).unwrap();
    });
    wait_entered.recv_timeout(DEADLINE).unwrap();
    let reply = gui_requests.recv_timeout(DEADLINE).unwrap();
    assert!(matches!(completed.try_recv(), Err(mpsc::TryRecvError::Empty)));
    //The harness rescues the deliberately blocked predecessor; it does not
    //pretend another thread performed a real GUI operation.
    reply.send(()).unwrap();
    assert_eq!(completed.recv_timeout(DEADLINE).unwrap().unwrap(), Some("retained source result"));
    gui.join().unwrap();
}

#[test]
fn dropping_unfinished_source_job_does_not_wait_for_source_work() {
    let (started, ready) = mpsc::channel(); let (release, run) = mpsc::channel();
    let (dropped, observed) = mpsc::channel(); let (finished, done) = mpsc::channel();
    let gui = thread::spawn(move || {
        let job = Job::start("abandoned source test", move || { finished.send(()).unwrap(); }, move || {
            started.send(()).unwrap(); run.recv().unwrap(); vec![1u8, 2, 3]
        }).unwrap();
        ready.recv_timeout(DEADLINE).unwrap(); drop(job); dropped.send(()).unwrap();
    });
    let result = observed.recv_timeout(DEADLINE);
    release.send(()).unwrap(); gui.join().unwrap(); done.recv_timeout(DEADLINE).unwrap();
    result.unwrap();
}

#[test]
fn work_panic_and_failed_wakeup_do_not_erase_transport_outcome() {
    let mut job = Job::<()>::start("panic source test", || {}, || panic!("source parser stopped")).unwrap();
    assert!(poll_until(&mut job).unwrap_err().contains("source parser stopped"));
    let mut job = Job::start("panic wake test", || panic!("repaint unavailable"), || 23).unwrap();
    assert_eq!(poll_until(&mut job).unwrap(), 23);
}
