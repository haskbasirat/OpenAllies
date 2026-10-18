//! Native ChangeToState with its settings assignment executing end to end.
use aa_netcode::launch_assignment::SettingsStore;
use aa_netcode::lobby::{Session, SnapshotHost};
use aa_netcode::lobby_staging::{Effects, MapStatus, Snapshot, Staging};
use aa_netcode::staging::{LaunchStep, StagingNode, StateEvent};
use aa_netcode::{Error, Result};
use aa_orders::payload::Field;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

fn de<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-net-integration/native-staging-assignment.json"
    ))
    .unwrap()
}
fn store(value: &Value) -> SettingsStore {
    SettingsStore {
        launch: de(&value["retained_launch"]),
        launch_88: de(&value["retained_launch_88"]),
        single: de(&value["single"]),
        multi: de(&value["multi"]),
    }
}
fn state(session: &Session, store: &SettingsStore) -> Value {
    json!({"session": session, "retained_launch": store.launch, "retained_launch_88": store.launch_88,
           "single": store.single, "multi": store.multi})
}
fn node(value: &Value) -> StagingNode {
    StagingNode {
        view: de(&value["view"]),
        took_over: de(&value["took_over"]),
        ordering_player: None,
        local_player: None,
        internet: false,
    }
}

#[derive(Default)]
struct EventSink {
    events: Vec<Value>,
    fail: bool,
}
impl Effects for EventSink {
    fn ui_message(&mut self, _: u32, _: &'static str, _: Option<i32>) -> Result<()> {
        panic!("unexpected UI message")
    }
    fn request(&mut self, _: usize, _: Vec<Field>) -> Result<()> {
        panic!("unexpected request")
    }
    fn countdown_time(&mut self) -> Result<i32> {
        panic!("unexpected countdown")
    }
    fn start_timer(&mut self, _: &mut Session) -> Result<()> {
        panic!("unexpected timer")
    }
    fn not_ready(&mut self, _: &mut Session) -> Result<()> {
        panic!("unexpected readiness")
    }
    fn launch_step(&mut self, _: &mut Session, _: LaunchStep) -> Result<()> {
        panic!("unexpected world load")
    }
    fn load(&mut self, _: &mut Session) -> Result<bool> {
        panic!("unexpected world load")
    }
    fn dump_launch_settings(&mut self, _: &mut Session, _: &'static str) -> Result<()> {
        panic!("unexpected sync dump")
    }
    fn timeline_freeze(&mut self, _: bool) -> Result<()> {
        panic!("unexpected freeze")
    }
    fn reset_order_version(&mut self) -> Result<()> {
        panic!("unexpected order reset")
    }
    fn state_changed(&mut self, _: &mut Session, event: StateEvent) -> Result<()> {
        self.events.push(json!({"event": "state_event", "text": event.text, "state": event.state, "flag": event.flag,
            "launch_state": {"kind": event.launch_state.kind, "flag_04": event.launch_state.flag_04,
            "text": event.launch_state.text, "field_0c": event.launch_state.field_0c, "field_10": event.launch_state.field_10,
            "flag_14": event.launch_state.flag_14, "flag_15": event.launch_state.flag_15, "flag_16": event.launch_state.flag_16,
            "field_18": event.launch_state.field_18, "field_1c": event.launch_state.field_1c,
            "flag_20": event.launch_state.flag_20, "field_24": event.launch_state.field_24}}));
        if self.fail {
            Err(Error::Unsupported("test event queue unavailable"))
        } else {
            Ok(())
        }
    }
}
struct MissingMapStatus;
impl MapStatus for MissingMapStatus {
    fn update_map_status(
        &mut self,
        _: &mut Session,
        _: bool,
        _: &[u16],
        _: u32,
        _: bool,
    ) -> Result<bool> {
        Err(Error::Unsupported("test map checker unavailable"))
    }
}

#[test]
fn snapshot_bridge_replays_native_state_and_real_settings_copies() {
    let fixture = fixture();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 48);
    for (index, row) in cases.iter().enumerate() {
        let mut session: Session = de(&row["before"]["session"]);
        let mut settings = store(&row["before"]);
        let mut events = EventSink::default();
        let n = node(&row["node"]);
        let mut maps = MissingMapStatus;
        let mut bridge = Snapshot {
            node: &n,
            staging: Staging {
                settings: &mut settings,
                effects: &mut events,
            },
            map_status: &mut maps,
        };
        bridge
            .change_to_state(
                &mut session,
                de(&row["new_state"]),
                u32::from(de::<bool>(&row["flag"])),
            )
            .unwrap();
        assert_eq!(
            state(&session, &settings),
            row["after"],
            "native state {index}"
        );
        assert_eq!(json!(events.events), row["events"], "native events {index}");
    }
}

#[test]
fn publication_error_keeps_native_committed_prefix_and_never_fakes_map_readiness() {
    let fixture = fixture();
    // 0 -> 1 publishes before restoring Launch88. Failed publication must
    // retain the new state/flags but must not perform the later copy/clear.
    let row = &fixture["cases"][0];
    let mut session: Session = de(&row["before"]["session"]);
    let before = session.clone();
    let mut settings = store(&row["before"]);
    let mut events = EventSink {
        fail: true,
        ..Default::default()
    };
    let n = node(&row["node"]);
    let mut maps = MissingMapStatus;
    let mut bridge = Snapshot {
        node: &n,
        staging: Staging {
            settings: &mut settings,
            effects: &mut events,
        },
        map_status: &mut maps,
    };
    assert!(bridge.change_to_state(&mut session, 1, 0).is_err());
    assert_eq!(session.state, 1);
    assert_eq!(session.field_100, 0);
    assert!(session.changed && session.changed_c9);
    assert_eq!(session.launch, before.launch);
    assert_eq!(session.launch_88, before.launch_88);
    assert!(
        bridge
            .update_map_status(&mut session, false, &[65], 0x12345678, true)
            .is_err()
    );
    assert_eq!(events.events.len(), 1);
}
