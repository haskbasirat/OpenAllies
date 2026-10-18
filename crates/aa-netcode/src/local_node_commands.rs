//! Same Node+118 command queue created by75C600 and owned until destruction.
//! Its native count+34 is Node14C. The constructor-empty75C9A0 path is exact;
//! positive command records/75C920 remain a required handler, never discarded.
use super::*;
use std::sync::{Condvar, Mutex};
///A future positive producer must bind the real full command record before
///it can enqueue it. Missing payload words are never filled with zero.
struct Pending {
    required_native_record: (),
}
pub(crate) struct State {
    queue: Mutex<VecDeque<Pending>>,
    changed: Condvar,
    signaled: bool,
}
impl State {
    ///75C600 creates its critical section/event(false) then stores pointer,
    ///capacity,head,count zero. Rust Mutex/Condvar is the backend equivalent
    ///for the admitted empty path; no native kernel handle identity is claimed.
    pub(crate) fn from_native_constructor() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            changed: Condvar::new(),
            signaled: false,
        }
    }
    pub(crate) fn pending_count(&self) -> Result<usize> {
        Ok(self
            .queue
            .lock()
            .map_err(|_| missing("same Node command queue is poisoned"))?
            .len())
    }
    ///Native75C9BE..C6 returns without logger/temp-command/signal effects when
    ///the actual queue count is zero. A false result requires those children.
    pub(crate) fn constructor_empty_prefix(&self) -> Result<bool> {
        Ok(self.pending_count()? == 0)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_constructor_queue_and_empty_process_command_branch() {
        let d: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/development10/live-frame-host/native-command-queue.json"
        ))
        .unwrap();
        assert_eq!(d["cases"].as_array().unwrap().len(), 18);
        for c in d["cases"].as_array().unwrap() {
            for o in ["0x28", "0x2c", "0x30", "0x34"] {
                assert_eq!(c["constructor"][o], 0)
            }
            assert_eq!(
                c["constructor_events"],
                serde_json::json!([{"kind":"critical_ctor","argument":null},{"kind":"event_ctor","argument":0}])
            );
            let state = State::from_native_constructor();
            assert_eq!(state.pending_count().unwrap(), 0);
            assert!(!state.signaled);
            assert!(state.constructor_empty_prefix().unwrap());
            let count = c["positive_count"].as_u64().unwrap() as usize;
            if count == 0 {
                assert!(c["stop"].is_null());
                assert_eq!(c["events"], serde_json::json!([]));
            } else {
                //Explicit unbound-record laboratory probes: positive counts
                //never inherit the constructor-empty successful response.
                for _ in 0..count {
                    state.queue.lock().unwrap().push_back(Pending {
                        required_native_record: (),
                    });
                }
                assert!(!state.constructor_empty_prefix().unwrap());
                assert_eq!(state.pending_count().unwrap(), count);
                assert_eq!(c["stop"], "positive_diagnostic");
                assert_eq!(c["count_after"], count);
            }
        }
    }
}
