//!Standalone Windows ownership tests using the exact two production modules.
//!No full runtime linker; no native recurring-loop/readiness claim.
#![allow(dead_code)]
#[path = "../../crates/aa-runtime/src/execution_context.rs"]
mod execution_context;
#[path = "../../crates/aa-runtime/src/simulation_execution.rs"]
mod simulation_execution;
use execution_context::ApplicationThread;
use simulation_execution::SimulationThread;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
};
struct WindowsIds;
impl execution_context::OsThreadIds for WindowsIds {
    fn current_thread_id(&mut self) -> Result<u32, String> {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetCurrentThreadId() -> u32;
        }
        //SAFETY: actual Win32 no-argument current-thread observation.
        Ok(unsafe { GetCurrentThreadId() })
    }
}
fn simulation() -> SimulationThread<Rc<RefCell<u32>>> {
    let application = Arc::new(ApplicationThread::capture(&mut WindowsIds).unwrap());
    SimulationThread::start(application, WindowsIds, |_| Ok(Rc::new(RefCell::new(10)))).unwrap()
}
#[test]
fn application_can_acknowledge_work_while_same_non_send_state_is_lent() {
    let mut sim = simulation();
    let child = sim.identity().os_thread_id().unwrap();
    let (request_tx, request_rx) = mpsc::sync_channel(0);
    let (ack_tx, ack_rx) = mpsc::sync_channel(0);
    let mut ticket = sim
        .submit(move |state, execution| {
            *state.borrow_mut() += 1;
            let identity = (
                execution.current_os_thread_id()?,
                Rc::as_ptr(state) as usize,
            );
            request_tx.send(identity).map_err(|e| e.to_string())?;
            ack_rx.recv().map_err(|e| e.to_string())?;
            Ok((identity, *state.borrow()))
        })
        .unwrap();
    let requested = request_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    assert_eq!(requested.0, child);
    assert_eq!(ticket.try_result().unwrap(), None);
    //This call is made by the application thread, while the child is waiting
    //inside its same operation. A blocking run here would prevent this ack.
    ack_tx.send(()).unwrap();
    assert_eq!(ticket.wait().unwrap(), (requested, 11));
    let next = sim
        .run(|state, execution| {
            Ok((
                (
                    execution.current_os_thread_id()?,
                    Rc::as_ptr(state) as usize,
                ),
                *state.borrow(),
            ))
        })
        .unwrap();
    assert_eq!(next, (requested, 11));
}
#[test]
fn dropped_delivery_and_required_errors_preserve_same_serial_state() {
    let mut sim = simulation();
    let ticket = sim
        .submit(|state, _| {
            *state.borrow_mut() += 2;
            Ok(7u32)
        })
        .unwrap();
    drop(ticket);
    assert_eq!(sim.run(|state, _| Ok(*state.borrow())).unwrap(), 12);
    let ticket = sim
        .submit(|state, _| {
            *state.borrow_mut() += 3;
            Err::<u32, _>("required native backend".into())
        })
        .unwrap();
    assert_eq!(ticket.wait().unwrap_err(), "required native backend");
    assert_eq!(sim.run(|state, _| Ok(*state.borrow())).unwrap(), 15);
}
#[test]
fn polling_consumes_a_completed_outcome_exactly_once() {
    let mut sim = simulation();
    let mut ticket = sim.submit(|_, _| Ok(19u32)).unwrap();
    let stop = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Some(value) = ticket.try_result().unwrap() {
            assert_eq!(value, 19);
            break;
        }
        assert!(std::time::Instant::now() < stop);
        std::thread::yield_now();
    }
    assert_eq!(
        ticket.try_result().unwrap_err(),
        "simulation job outcome was already consumed"
    );
    assert_eq!(
        ticket.wait().unwrap_err(),
        "simulation job outcome was already consumed"
    );
}
