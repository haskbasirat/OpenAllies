#![cfg(windows)]
#[path = "../../aa-client/src/windows_reset_os.rs"]
mod windows_reset_os;
use aa_runtime::{execution_context::ApplicationThread, simulation_execution::SimulationThread};
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[test]
fn same_real_thread_retains_non_send_state_and_lazy_sync_gate() {
    let application =
        Arc::new(ApplicationThread::capture(&mut windows_reset_os::WindowsThreadIds).unwrap());
    let mut simulation = SimulationThread::start(
        Arc::clone(&application),
        windows_reset_os::WindowsThreadIds,
        |execution| {
            assert_ne!(
                execution.current_os_thread_id()?,
                execution.application_os_thread_id()
            );
            Ok(Rc::new(RefCell::new(10u32)))
        },
    )
    .unwrap();
    let identity = simulation.identity();
    identity.require_application(&application).unwrap();
    let foreign_application =
        ApplicationThread::capture(&mut windows_reset_os::WindowsThreadIds).unwrap();
    assert!(identity.require_application(&foreign_application).is_err());
    assert_ne!(identity.os_thread_id().unwrap(), application.id());
    assert!(
        !identity
            .sync_gate(&mut windows_reset_os::WindowsThreadIds, || panic!(
                "foreign thread must not read SAI"
            ))
            .unwrap()
    );
    let first = simulation
        .run(|state, execution| {
            *state.borrow_mut() += 1;
            assert!(execution.sync_gate(|| Ok(Some([0, 0])))?);
            assert!(!execution.sync_gate(|| Ok(Some([128, 0])))?);
            assert!(!execution.sync_gate(|| Ok(Some([0, 255])))?);
            assert!(execution.sync_gate(|| Ok(None))?);
            let error = execution
                .sync_gate(|| Err("actual SAI unavailable".into()))
                .unwrap_err();
            assert_eq!(error, "actual SAI unavailable");
            Ok((
                execution.current_os_thread_id()?,
                Rc::as_ptr(state) as usize,
                *state.borrow(),
            ))
        })
        .unwrap();
    let second = simulation
        .run(|state, execution| {
            Ok((
                execution.current_os_thread_id()?,
                Rc::as_ptr(state) as usize,
                *state.borrow(),
            ))
        })
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.2, 11);
    drop(simulation);
    assert!(identity.os_thread_id().is_err());
}

#[test]
fn initializer_failure_joins_without_publishing_a_live_owner() {
    let application =
        Arc::new(ApplicationThread::capture(&mut windows_reset_os::WindowsThreadIds).unwrap());
    let failure = SimulationThread::<Rc<RefCell<()>>>::start(
        application,
        windows_reset_os::WindowsThreadIds,
        |_| Err("genuine init failure".into()),
    );
    assert!(matches!(failure,Err(e)if e=="genuine init failure"));
}

#[test]
fn replay_all_native_gate_rows_using_actual_parent_and_child_threads() {
    let native: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-simulation-execution/native-gate.json"
    ))
    .unwrap();
    let application =
        Arc::new(ApplicationThread::capture(&mut windows_reset_os::WindowsThreadIds).unwrap());
    //These flag values are explicit native-oracle test inputs, not a production SAI.
    let mut simulation =
        SimulationThread::start(application, windows_reset_os::WindowsThreadIds, |_| Ok(()))
            .unwrap();
    for row in native["rows"].as_array().unwrap() {
        let flags = row["sai_flags"]
            .as_array()
            .map(|v| [v[0].as_u64().unwrap() as u8, v[1].as_u64().unwrap() as u8]);
        let gate = if row["current"] == row["simulation"] {
            simulation
                .run(move |_, execution| execution.sync_gate(|| Ok(flags)))
                .unwrap()
        } else {
            simulation
                .identity()
                .sync_gate(&mut windows_reset_os::WindowsThreadIds, || {
                    panic!("native thread mismatch must skip SAI flags")
                })
                .unwrap()
        };
        assert_eq!(gate, row["gate"].as_bool().unwrap(), "{row}");
        let mut sync = aa_orders::net::sync::Synchronizer::new(32);
        sync.enabled = true;
        sync.log_int(gate, "thread-gate", 17);
        assert_eq!(
            sync.hash,
            i32::try_from(row["after"]["hash"].as_i64().unwrap()).unwrap()
        );
    }
}
