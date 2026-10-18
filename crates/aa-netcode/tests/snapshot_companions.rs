//! Whole native session snapshots, including same-count kingdom replacement,
//! now pass through one owner of wire settings and non-wire companions.
use aa_netcode::launch_assignment::SettingsStore;
use aa_netcode::lobby::{self, ClientAdmin, LobbyAction, LobbyData, Session};
use aa_netcode::lobby_staging::{Effects, MapStatus, SnapshotOwner};
use aa_netcode::staging::{LaunchStep, StagingNode, StateEvent};
use aa_netcode::{Error, Result};
use aa_orders::{
    Cursor, Position,
    payload::{Field, Registry},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};

type Log = Rc<RefCell<Vec<Value>>>;
fn de<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-net-integration/native-snapshot-companions.json"
    ))
    .unwrap()
}
fn owner(row: &Value) -> SnapshotOwner {
    let b = &row["before"];
    SnapshotOwner::new(
        de(&b["session"]),
        SettingsStore {
            launch: de(&b["retained_launch"]),
            launch_88: de(&b["retained_launch_88"]),
            single: de(&b["single"]),
            multi: de(&b["multi"]),
        },
    )
    .unwrap()
}
fn node(row: &Value) -> StagingNode {
    StagingNode {
        view: de(&row["node"]),
        took_over: 0,
        ordering_player: None,
        local_player: None,
        internet: false,
    }
}
fn snapshot(owner: &SnapshotOwner) -> Value {
    let settings = owner.settings().unwrap();
    json!({"session": owner.session(), "retained_launch": settings.launch, "retained_launch_88": settings.launch_88,
        "single": settings.single, "multi": settings.multi})
}
struct Data {
    log: Log,
}
impl LobbyData for Data {
    fn registry_count(&self, registry: Registry) -> Result<usize> {
        if registry == lobby::WORLD_RULES {
            return Ok(5);
        }
        if [
            lobby::MAP_DEF_44,
            lobby::MAP_DEF_48,
            lobby::CONDITIONS,
            lobby::NATIONS,
            lobby::FACTIONS,
            lobby::COLORS,
            lobby::AI_HANDICAPS,
            lobby::START_PROPERTIES,
            lobby::OBJECT_DEFINITIONS,
        ]
        .contains(&registry)
        {
            Ok(4)
        } else {
            Err(Error::Unsupported("fixture registry missing"))
        }
    }
    fn definition(&self, registry: Registry, index: u32) -> bool {
        self.registry_count(registry)
            .is_ok_and(|n| (index as usize) < n)
    }
    fn world_rule_kind(&self, index: u32) -> Result<u32> {
        [0, 1, 2, 3, 7]
            .get(index as usize)
            .copied()
            .ok_or(Error::Unsupported("fixture world rule missing"))
    }
    fn world_rule_name(&self, _: u32) -> Result<Vec<u16>> {
        Err(Error::Unsupported(
            "snapshot codec must not query world-rule names",
        ))
    }
    fn scenario(&mut self, key: &[u16]) -> Option<Vec<u16>> {
        self.log
            .borrow_mut()
            .push(json!({"event": "scenario", "key": key}));
        (key == "scenario".encode_utf16().collect::<Vec<_>>()).then(|| key.to_vec())
    }
    fn random_map_config(&mut self, key: &[u16]) -> Option<Vec<u16>> {
        self.log
            .borrow_mut()
            .push(json!({"event": "random_map", "key": key}));
        (key == "rmc".encode_utf16().collect::<Vec<_>>()).then(|| key.to_vec())
    }
    fn zero_width_list_budget(&self) -> usize {
        64
    }
}
struct EventSink {
    log: Log,
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
        self.log.borrow_mut().push(
            json!({"event": "state_event", "text": event.text, "state": event.state,
            "flag": event.flag, "launch_state": event.launch_state}),
        );
        Ok(())
    }
}
struct MapChecks {
    log: Log,
    fail: bool,
}
impl MapStatus for MapChecks {
    fn update_map_status(
        &mut self,
        _: &mut Session,
        rmc: bool,
        file: &[u16],
        crc: u32,
        source_check: bool,
    ) -> Result<bool> {
        self.log.borrow_mut().push(json!({"event": "map_status", "rmc": rmc, "file": file, "crc": crc, "source_check": u8::from(source_check)}));
        if self.fail {
            Err(Error::Unsupported("fixture map checker unavailable"))
        } else {
            Ok(true)
        }
    }
}

#[test]
fn full_native_snapshots_rebuild_companions_and_preserve_other_private_fields() {
    let fixture = fixture();
    assert_eq!(
        fixture["constructor"],
        json!({"text_0c": [], "flag_68": false})
    );
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 12);
    for row in rows {
        let mut owned = owner(row);
        let bytes: Vec<u8> = de(&row["wire"]);
        let end: Position = de(&row["end"]);
        let mut reader =
            Cursor::with_bounds(&bytes, Position::PACKET_START, end.absolute().unwrap()).unwrap();
        let mut admin: ClientAdmin = de(&row["admin_before"]);
        let log = Log::default();
        let mut data = Data { log: log.clone() };
        let mut events = EventSink { log: log.clone() };
        let mut maps = MapChecks {
            log: log.clone(),
            fail: false,
        };
        let actions = owned
            .read_snapshot(
                &mut reader,
                &mut admin,
                &mut data,
                &node(row),
                &mut events,
                &mut maps,
            )
            .unwrap();
        assert_eq!(reader.position(), end);
        assert_eq!(
            snapshot(&owned),
            row["after"],
            "native snapshot {}",
            row["index"]
        );
        assert_eq!(json!(admin), row["admin_after"]);
        assert!(actions.contains(&LobbyAction::ResetSessionInfo));
        let expected: Vec<_> = row["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|event| event["event"] != "reset_session_info")
            .cloned()
            .collect();
        assert_eq!(*log.borrow(), expected, "callback order {}", row["index"]);
        // A subsequent native leave-state copy must use the decoded companions,
        // never the previous local kingdom fields that happened to share an index.
        let current = owned.session().launch.clone();
        let retained = owned.settings().unwrap().launch.clone();
        owned
            .change_to_state(2, false, &node(row), &mut events)
            .unwrap();
        assert_eq!(owned.session().launch_88, current);
        assert_eq!(owned.settings().unwrap().launch_88, retained);
    }
}

#[test]
fn incomplete_snapshot_invalidates_companions_and_refuses_replay_or_copy() {
    let fixture = fixture();
    let row = &fixture["cases"][3]; // staging -> staging, equal-count replacement
    let bytes: Vec<u8> = de(&row["wire"]);
    let end: Position = de(&row["end"]);
    for bit_end in [
        3,
        64,
        end.absolute().unwrap() / 2,
        end.absolute().unwrap() - 1,
    ] {
        let mut owned = owner(row);
        let mut admin: ClientAdmin = de(&row["admin_before"]);
        let log = Log::default();
        let mut data = Data { log: log.clone() };
        let mut events = EventSink { log: log.clone() };
        let mut maps = MapChecks {
            log: log.clone(),
            fail: false,
        };
        let mut reader = Cursor::with_bounds(&bytes, Position::PACKET_START, bit_end).unwrap();
        assert!(
            owned
                .read_snapshot(
                    &mut reader,
                    &mut admin,
                    &mut data,
                    &node(row),
                    &mut events,
                    &mut maps
                )
                .is_err()
        );
        assert_eq!(owned.session().state, 1); // committed native transition remains
        assert!(owned.settings().is_err());
        let partial = owned.session().clone();
        let events_before = log.borrow().len();
        assert!(
            owned
                .change_to_state(2, false, &node(row), &mut events)
                .is_err()
        );
        let mut retry =
            Cursor::with_bounds(&bytes, Position::PACKET_START, end.absolute().unwrap()).unwrap();
        assert!(
            owned
                .read_snapshot(
                    &mut retry,
                    &mut admin,
                    &mut data,
                    &node(row),
                    &mut events,
                    &mut maps
                )
                .is_err()
        );
        assert_eq!(retry.position(), Position::PACKET_START);
        assert_eq!(owned.session(), &partial);
        assert_eq!(log.borrow().len(), events_before);
        assert!(owned.into_complete().is_err());
    }
}

#[test]
fn failed_map_status_cannot_publish_complete_snapshot_ownership() {
    let fixture = fixture();
    let row = &fixture["cases"][1]; // scenario mode invokes map status
    let mut owned = owner(row);
    let bytes: Vec<u8> = de(&row["wire"]);
    let end: Position = de(&row["end"]);
    let mut reader =
        Cursor::with_bounds(&bytes, Position::PACKET_START, end.absolute().unwrap()).unwrap();
    let mut admin: ClientAdmin = de(&row["admin_before"]);
    let log = Log::default();
    let mut data = Data { log: log.clone() };
    let mut events = EventSink { log: log.clone() };
    let mut maps = MapChecks {
        log: log.clone(),
        fail: true,
    };
    assert!(
        owned
            .read_snapshot(
                &mut reader,
                &mut admin,
                &mut data,
                &node(row),
                &mut events,
                &mut maps
            )
            .is_err()
    );
    assert!(owned.settings().is_err());
    assert!(
        log.borrow()
            .iter()
            .any(|event| event["event"] == "map_status")
    );
}
