//Included inside battle_controller::windows. This tests the real Controller
//status/error reducer with a controlled SOURCE-only failure. The real Engine
//creates its actual Windows simulation thread; no window, GPU or game World
//is constructed, and no native readiness/activation state is manufactured.
use super::*;
use std::{
    sync::{mpsc, atomic::{AtomicBool, Ordering}},
    time::{Duration, Instant},
};

struct RetainedSourcePrefix(Arc<AtomicBool>);
impl Drop for RetainedSourcePrefix {
    fn drop(&mut self) { self.0.store(true, Ordering::Release); }
}

#[test]
fn pending_source_poll_returns_to_gui_then_real_controller_preserves_failure() {
    let window = aa_runtime::application_window_state::ApplicationWindowState::from_native_constructor();
    let (mut gui_queue, bridge) = crate::battle_ui_bridge::Queue::new();
    let mut controller = Controller::new(Path::new("."), window.reader(), bridge.clone());
    let request = Request {
        map_id: "controlled-source-transport-fixture".into(),
        source_path: "no-map-is-loaded-by-this-test".into(),
        player_name: "transport test".into(), game_name: "transport test".into(),
        seed: 0, observers: false,
    };
    controller.queue(request.clone()).unwrap();
    assert_eq!(controller.status(), &Status::Queued);

    let (started, entered) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let retired = Arc::new(AtomicBool::new(false));
    let prefix = RetainedSourcePrefix(Arc::clone(&retired));
    let wake = bridge.clone();
    let mut engine = Engine::new(window.reader(), &bridge).unwrap();
    engine.source_job = Some(super::super::startup_job::Job::start(
        "controlled Controller source stop", move || wake.wake(), move || {
            started.send(()).unwrap(); gate.recv().unwrap();
            Err::<SourceOwners, _>(Failure::retained("controlled source child is unavailable", prefix))
        }).unwrap());
    controller.engine = Some(engine);
    //This is the explicit pending-transport input. The actual poll reducer
    //below determines every later status; it cannot reach a fake World.
    controller.status = Status::Loading;
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    for _ in 0..3 {
        controller.poll();
        assert_eq!(controller.status(), &Status::Loading);
        assert_eq!(controller.message(), "Reading and validating game data");
    }
    assert!(controller.queue(request).unwrap_err().contains("already retained"));
    assert!(!retired.load(Ordering::Acquire));
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while controller.status() == &Status::Loading {
        controller.poll();
        assert!(Instant::now() < deadline, "Controller did not observe the source outcome");
        std::thread::yield_now();
    }
    assert_eq!(controller.status(), &Status::Failed);
    assert_eq!(controller.message(), "controlled source child is unavailable");
    let failure = controller.failed.as_ref().expect("same failure owner must remain installed");
    assert_eq!(failure.error, controller.message());
    assert_eq!(failure._owners.len(), 1);
    assert!(!retired.load(Ordering::Acquire));
    controller.poll();
    assert_eq!(controller.status(), &Status::Failed);
    assert!(!retired.load(Ordering::Acquire));
    gui_queue.close();
    controller.discard_stopped_attempt().unwrap();
    assert!(retired.load(Ordering::Acquire));
    assert_eq!(controller.status(), &Status::Idle);
    assert!(controller.request().is_none());
    assert!(controller.message().is_empty());
}
