use aa_runtime::execution_context::{ApplicationThread, OsThreadIds};
struct Os {
    id: u32,
    reads: u32,
    fail: bool,
}
impl OsThreadIds for Os {
    fn current_thread_id(&mut self) -> Result<u32, String> {
        self.reads += 1;
        if self.fail {
            Err("required real OS thread ID".into())
        } else {
            Ok(self.id)
        }
    }
}
#[test]
fn native_app_identity_is_captured_once_and_does_not_follow_later_current_thread_changes() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-execution-context/native-execution-context.json"
    ))
    .unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let id = row["input"]["identity"].as_u64().unwrap() as u32;
        let mut os = Os {
            id,
            reads: 0,
            fail: false,
        };
        let owner = ApplicationThread::capture(&mut os).unwrap();
        assert_eq!(
            owner.id(),
            row["output"]["identity"].as_u64().unwrap() as u32
        );
        assert_eq!(os.reads, 1);
        os.id = id.wrapping_add(1);
        assert_ne!(os.current_thread_id().unwrap(), owner.id());
        assert_eq!(os.reads, 2);
        assert_eq!(owner.id(), id);
    }
    let mut required = Os {
        id: 0,
        reads: 0,
        fail: true,
    };
    assert!(ApplicationThread::capture(&mut required).is_err());
    assert_eq!(required.reads, 1);
}
