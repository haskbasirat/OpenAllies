use super::*;
use serde_json::{json, Value};

struct Os { tick: u32, guid: Guid, calls: Vec<&'static str>, fail: Option<&'static str> }
impl ResetOs for Os {
    fn get_tick_count(&mut self) -> Result<u32> {
        self.calls.push("tick");
        if self.fail == Some("tick") { return Err(missing("injected OS tick failure")); }
        Ok(self.tick)
    }
    fn co_create_guid(&mut self) -> Result<Guid> {
        self.calls.push("guid");
        if self.fail == Some("guid") { return Err(missing("injected CoCreateGuid failure")); }
        Ok(self.guid)
    }
    fn sync_log_gate(&mut self) -> Result<bool> {
        panic!("disabled fresh synchronizer must not read a fabricated logger gate")
    }
}
fn os(tick: u32, bytes: &[u8]) -> Os {
    Os { tick, guid: Guid { data1: u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
        data2: u16::from_le_bytes(bytes[4..6].try_into().unwrap()),
        data3: u16::from_le_bytes(bytes[6..8].try_into().unwrap()), data4: bytes[8..16].try_into().unwrap() },
        calls: Vec::new(), fail: None }
}
fn assert_projection(got: &Value, expected: &Value) {
    for (key, value) in expected.as_object().unwrap() {
        assert_eq!(&got[key], value, "native projected field {key}");
    }
}

#[test]
fn native_whole_fresh_reset_and_first_local_player_replay() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-local-bootstrap/native-bootstrap.json")).unwrap();
    assert_eq!(f["passed"], true);
    assert_eq!(f["cases"].as_array().unwrap().len(), 24);
    for row in f["cases"].as_array().unwrap() {
        let mut owner = LocalBootstrap::from_native_constructors();
        assert!(owner.last_player_id().is_err());
        assert_eq!(owner.launch_mode(), 3);
        assert_eq!(owner.launch_game_type(), 0);
        assert_eq!(owner.options().max_players, -1);
        assert_eq!(owner.options().field_d8, u32::MAX);
        assert!(owner.options().allow_observers);
        assert_eq!(serde_json::to_value(constructor_launch_state()).unwrap(), row["constructor_launch_state"]);
        let mut host = os(row["tick"].as_u64().unwrap() as u32,
            &serde_json::from_value::<Vec<u8>>(row["guid_bytes"].clone()).unwrap());
        let notifications = Notifications::new(row["application_kind"].as_u64().unwrap() as u32);
        owner.reset_fresh(&mut host, &InitialProperties::from_native_constructor(), &notifications).unwrap();
        assert_eq!(host.calls, ["tick", "guid"]);
        assert_projection(&serde_json::to_value(owner.snapshot()).unwrap(), &row["reset"]);
        assert_eq!(notifications.len().unwrap() as u64, row["queued_count"].as_u64().unwrap());
        let expected_event = &row["notification"][0];
        if row["application_kind"] != 3 {
            assert_eq!(serde_json::to_value(notifications.pop().unwrap().unwrap()).unwrap(), *expected_event);
            assert_eq!(row["queued_values"], json!([expected_event]));
        } else {
            assert_eq!(row["queued_values"], json!([]));
        }
        assert!(notifications.is_empty().unwrap());
        let nickname: Vec<u16> = serde_json::from_value(row["nickname"].clone()).unwrap();
        let key: Vec<u16> = serde_json::from_value(row["identity_key"].clone()).unwrap();
        // Explicit native text-table fixture seam. Production entry point uses
        // the real Localization::display source instead of this closure.
        let p = owner.create_first_local_player_with(&nickname, &key,
            || Ok(VWorldState::with_caption("All Units".encode_utf16().collect()))).unwrap();
        assert_eq!(p.id(), 1);
        assert_eq!(owner.player_status(&p).unwrap(), 0);
        assert_eq!(owner.host(), PeerLink::Null);
        assert!(owner.kingdom_name(&p).unwrap().is_empty());
        assert_eq!(owner.player_name(&p).unwrap(), nickname);
        assert_projection(&serde_json::to_value(owner.snapshot()).unwrap(), &row["after_player"]);
        let got = owner.player_view_snapshot(&p).unwrap();
        let mut expected = row["player_view"].clone();
        assert_eq!(expected["view_word_0c"], json!(0xa5a5a5a5u32));
        // Allocation capacity/unused bytes are not authored gameplay values.
        expected.as_object_mut().unwrap().remove("player_vector_capacity");
        expected["group"].as_object_mut().unwrap().remove("capacity");
        expected["view_word_0c"] = Value::Null; // typed uninitialized, never allocator-zero claim
        expected["view_kingdom"] = Value::Null; // native null pointer
        assert_eq!(got, expected);
        assert!(owner.require_host_launch().is_err());
    }
}

#[test]
fn os_failures_keep_the_native_committed_prefix_without_supplying_defaults() {
    let source = InitialProperties::from_native_constructor();
    for failure in ["tick", "guid"] {
        let mut owner = LocalBootstrap::from_native_constructors();
        let queue = Notifications::new(0);
        let mut os = os(0x12345678, &[0; 16]); os.fail = Some(failure);
        assert!(owner.reset_fresh(&mut os, &source, &queue).is_err());
        assert_eq!(owner.progress(), Progress::Resetting);
        assert_eq!(queue.pop().unwrap().unwrap().kind, 2);
        assert!(queue.is_empty().unwrap());
        if failure == "tick" {
            assert!(owner.last_player_id().is_err());
            assert_eq!(owner.rng_state(), NativeRng::default().state);
        } else {
            assert_eq!(owner.last_player_id().unwrap(), 0);
            assert_eq!(owner.rng_state(), NativeRng::from_native_seed(0x12345678).state);
            assert_eq!(owner.snapshot().changed, [true; 3]);
        }
        assert!(owner.reset_fresh(&mut os, &source, &queue).is_err());
    }
}

#[test]
fn actual_localization_is_required_and_player_handles_belong_to_the_live_owner() {
    let source = InitialProperties::from_native_constructor(); let queue = Notifications::new(0);
    let mut owner = LocalBootstrap::from_native_constructors();
    owner.reset_fresh(&mut os(1,&[0;16]), &source, &queue).unwrap();
    assert!(owner.create_first_local_player(&[65], &[66], &Localization::default()).is_err());
    assert_eq!(owner.progress(), Progress::CreatingPlayer);
    assert_eq!(owner.last_player_id().unwrap(), 1); // E8 increment precedes failing ctor
    assert!(owner.local_player().unwrap().is_none());
    assert_eq!(owner.snapshot().players.len(), 0); // constructor never appended
    assert!(owner.snapshot().node_active);
    let mut a = LocalBootstrap::from_native_constructors();
    let mut b = LocalBootstrap::from_native_constructors();
    a.reset_fresh(&mut os(0,&[0;16]), &source, &queue).unwrap();
    b.reset_fresh(&mut os(0,&[0;16]), &source, &queue).unwrap();
    let p = a.create_first_local_player_with(&[], &[], || Ok(VWorldState::with_caption(Vec::new()))).unwrap();
    b.create_first_local_player_with(&[], &[], || Ok(VWorldState::with_caption(Vec::new()))).unwrap();
    assert!(a.kingdom_name(&p).unwrap().is_empty());
    assert!(b.kingdom_name(&p).is_err());
    assert_eq!(a.current_phase().unwrap(), 0); assert_eq!(a.application_state().unwrap(), 0);
    assert_eq!(serde_json::to_value(constructor_launch_state()).unwrap()["field_0c"], json!(u32::MAX));
}
