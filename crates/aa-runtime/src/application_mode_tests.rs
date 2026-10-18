use super::*;
use serde_json::Value;
struct Services {
    a: bool,
    b: bool,
    fail: Option<&'static str>,
    calls: Vec<&'static str>,
}
impl Services {
    fn step(&mut self, call: &'static str) -> Result<(), String> {
        self.calls.push(call);
        if self.fail == Some(call) {
            Err(call.into())
        } else {
            Ok(())
        }
    }
}
impl ModeServices for Services {
    fn manager_a_active(&mut self) -> Result<bool, String> {
        self.step("read_a")?;
        Ok(self.a)
    }
    fn post_manager_a_reset(&mut self) -> Result<(), String> {
        self.step("post_a")
    }
    fn manager_b_active(&mut self) -> Result<bool, String> {
        self.step("read_b")?;
        Ok(self.b)
    }
    fn post_manager_b_reset(&mut self) -> Result<(), String> {
        self.step("post_b")
    }
}
#[test]
fn native_constructor_and_651840_setter_replay() {
    let rows: Value=serde_json::from_str(include_str!("../../../research/skirmish-20261016-local-bootstrap/application/native-application-mode.json")).unwrap();
    assert_eq!(rows["passed"], true);
    let ctor = GameAppState::from_native_constructor();
    for c in rows["constructors"].as_array().unwrap() {
        assert_eq!(ctor.mode(), c["mode"].as_u64().unwrap() as u32);
        assert_eq!(ctor.changed(), c["changed"] == 1);
        assert_eq!(
            ctor.constructor_flags(),
            (
                c["base4"].as_u64().unwrap() as u32,
                c["flag10"] == 1,
                c["flag11"] == 1
            )
        );
        assert_eq!(c["getter_identity"], true);
    }
    let mut count = 0;
    for row in rows["cases"].as_array().unwrap() {
        let mut owner = GameAppState::from_native_constructor();
        let mut s = Services {
            a: row["manager_a_field4"].as_u64().unwrap_or(0) != 0,
            b: row["manager_b_field14"].as_u64().unwrap_or(0) != 0,
            fail: None,
            calls: Vec::new(),
        };
        let mode = row["mode"].as_u64().unwrap() as u32;
        owner.set_mode(mode, &mut s).unwrap();
        assert_eq!(owner.mode(), mode);
        assert_eq!(owner.changed(), row["changed"] == 1);
        let actual: Vec<_> = s
            .calls
            .iter()
            .filter_map(|c| match *c {
                "post_a" => Some("00794B70"),
                "post_b" => Some("0079A340"),
                _ => None,
            })
            .collect();
        let expected: Vec<_> = row["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["call"].as_str().unwrap())
            .collect();
        assert_eq!(actual, expected);
        for event in row["events"].as_array().unwrap() {
            assert_eq!(event["mode_before"], mode);
            assert_eq!(event["changed_before"], 0);
        }
        count += 1;
    }
    assert_eq!(count, 270);
}
#[test]
fn reached_service_errors_preserve_mode_and_native_lazy_prefix() {
    for (fail, calls) in [
        ("read_a", vec!["read_a"]),
        ("post_a", vec!["read_a", "post_a"]),
        ("read_b", vec!["read_a", "post_a", "read_b"]),
        ("post_b", vec!["read_a", "post_a", "read_b", "post_b"]),
    ] {
        let mut owner = GameAppState::from_native_constructor();
        let mut s = Services {
            a: true,
            b: true,
            fail: Some(fail),
            calls: Vec::new(),
        };
        assert_eq!(owner.set_mode(4, &mut s).unwrap_err(), fail);
        assert_eq!(owner.mode(), 4);
        assert!(!owner.changed());
        assert_eq!(s.calls, calls);
    }
    let mut owner = GameAppState::from_native_constructor();
    let mut s = Services {
        a: false,
        b: false,
        fail: Some("post_a"),
        calls: Vec::new(),
    };
    owner.set_mode(2, &mut s).unwrap();
    assert_eq!(s.calls, ["read_a", "read_b"]);
    assert!(owner.changed());
    // Native does not clear an already-set +8 before a later setter error.
    s.fail = Some("read_a");
    assert!(owner.set_mode(7, &mut s).is_err());
    assert_eq!(owner.mode(), 7);
    assert!(owner.changed());
}
