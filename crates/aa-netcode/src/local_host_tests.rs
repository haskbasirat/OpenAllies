use super::*;
use serde_json::{Value, json};

struct Timer {
    value: u32,
    calls: usize,
    fail: bool,
}
impl MillisecondTimer for Timer {
    fn milliseconds(&mut self) -> std::result::Result<u32, String> {
        self.calls += 1;
        if self.fail {
            Err("explicit OS fixture failure".into())
        } else {
            Ok(self.value)
        }
    }
}
fn p(row: &Value) -> Precision {
    match row["precision"].as_str().unwrap() {
        "Single" => Precision::Single24,
        "Double" => Precision::Double53,
        "Extended" => Precision::Extended64,
        _ => panic!(),
    }
}
fn compare(got: &Value, expected: &Value) {
    for (key, value) in got.as_object().unwrap() {
        if key == "stage" {
            continue;
        }
        if key == "last_request_player" && value.is_null() {
            assert_eq!(expected[key], 0, "native null pointer must really be zero");
            continue; //portable Option::None is the same proved null identity
        }
        assert_eq!(value, &expected[key], "field{key}");
    }
}
#[test]
fn native_fresh_self_server_host_and_partial_backup_boundaries() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-local-bootstrap/host/native-local-host.json"
    ))
    .unwrap();
    assert_eq!(fixture["passed"], true);
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 24);
    for row in fixture["cases"].as_array().unwrap() {
        let mut node = Node::before_session_reset().unwrap();
        node.after_session_reset().unwrap();
        compare(&node.snapshot(), &row["reset"]);
        let orders = &node.orders as *const AdminNode;
        let request = &node.orders.requests as *const History;
        let mut time = NodeTime::from_native_constructor(Timer {
            value: 1000,
            calls: 0,
            fail: false,
        });
        assert_eq!(time.cached_seconds_bits(), 0x5368d4a5);
        assert_eq!(time.falling_behind(), 0);
        time.begin_update_prefix(p(row)).unwrap();
        assert_eq!(time.cached_seconds_bits(), 0x3f800000);
        time.behind_64 = row["behind"].as_u64().unwrap() as u8; //explicit native input probe
        let server = node.set_self_server(&time);
        if time.behind_64 != 0 {
            assert!(server.unwrap_err().to_string().contains("757250"));
            assert_eq!(row["cutoff"], "00757250");
            assert_eq!(time.timer.calls, 1); //neither SVar nor QPC clock was reached
        } else {
            server.unwrap();
            node.set_self_host(
                &mut time,
                || Ok(row["rate"].as_u64().unwrap() as u32),
                p(row),
            )
            .unwrap();
            node.after_host_bound(&time).unwrap();
            assert_eq!(row["cutoff"], "0075B20E");
            let restart = row["events"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["kind"] == "server_clock_restart")
                .unwrap();
            assert_eq!(
                node.clock.words()[0],
                restart["period"].as_u64().unwrap() as u32
            );
            assert_eq!(restart["start"], 0);
            assert_eq!(node.clock.words()[1], 0x3dcccccd);
            assert_eq!(node.clock.words()[2], 0x3f800000);
            assert_eq!(time.timer.calls, 2); //prefix QPC then native clock restart QPC
        }
        compare(&node.snapshot(), &row["after"]);
        assert_eq!(orders, &node.orders as *const AdminNode);
        assert_eq!(request, &node.orders.requests as *const History);
        assert!(node.require_reset().is_err()); //no automatic retry after partial success
    }
}
#[test]
fn native_node_constructor_and_first_cached_time_write() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-local-bootstrap/host/native-node-time.json"
    ))
    .unwrap();
    assert_eq!(fixture["passed"], true);
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 48);
    for row in fixture["constructors"].as_array().unwrap() {
        let time = NodeTime::from_native_constructor(Timer {
            value: 0,
            calls: 0,
            fail: false,
        });
        assert_eq!(json!(time.cached_seconds_bits()), row["cached58"]);
        assert_eq!(json!(time.falling_behind()), row["behind64"]);
    }
    for row in fixture["cases"].as_array().unwrap() {
        let mut time = NodeTime::from_native_constructor(Timer {
            value: row["milliseconds"].as_u64().unwrap() as u32,
            calls: 0,
            fail: false,
        });
        time.begin_update_prefix(p(row)).unwrap();
        assert_eq!(json!(time.cached_seconds_bits()), row["cached58"]);
        assert_eq!(time.timer.calls, 1);
    }
}
#[test]
fn required_rate_or_qpc_keeps_actual_reset_version_and_clock_prefix() {
    for qpc_failure in [false, true] {
        let mut node = Node::before_session_reset().unwrap();
        node.after_session_reset().unwrap();
        let mut time = NodeTime::from_native_constructor(Timer {
            value: 1000,
            calls: 0,
            fail: qpc_failure,
        });
        node.set_self_server(&time).unwrap();
        node.order_version = 7; //actual owner mutation probe
        let result = node.set_self_host(
            &mut time,
            || {
                if qpc_failure {
                    Ok(0x41800000)
                } else {
                    Err(error("required actual SVar"))
                }
            },
            Precision::Double53,
        );
        assert!(result.is_err());
        assert_eq!(node.order_version, 1);
        assert!(!node.has_clock);
        assert!(node.server_is_self);
        assert_eq!(node.stage, Stage::StartingSelfHost);
        assert_eq!(
            node.clock.words(),
            if qpc_failure {
                [0x3d800000, 0, 0]
            } else {
                [0, 0, 0]
            }
        );
        assert_eq!(time.timer.calls, usize::from(qpc_failure));
        assert!(node.require_reset().is_err());
    }
}

#[test]
fn native_node_frame_field_writers_and_same_resource_attachment() {
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261016-local-bootstrap/begin-playing/native-node-frame-fields.json")).unwrap();
    assert_eq!(fixture["passed"], true);
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 12);
    for row in fixture["cases"].as_array().unwrap() {
        let mut node = Node::before_session_reset().unwrap();
        assert_eq!(
            node.admin_time_28c,
            row["constructor"]["admin_time"].as_u64().unwrap() as u32
        );
        assert_eq!(
            node.disconnect_started_2d0,
            row["constructor"]["disconnect_started"].as_u64().unwrap() as u32
        );
        assert!(
            !node.disconnecting_2cd && !node.has_replay_source() && node.recorder_268().is_none()
        );
        let poison = row["poison"].as_u64().unwrap() as u32;
        node.admin_time_28c = poison;
        node.disconnecting_2cd = true;
        node.disconnect_started_2d0 = poison;
        node.heartbeat_2dc = poison;
        node.after_session_reset().unwrap();
        assert_eq!(
            node.admin_time_28c,
            row["reset"]["admin_time"].as_u64().unwrap() as u32
        );
        assert_eq!(
            node.disconnect_started_2d0,
            row["reset"]["disconnect_started"].as_u64().unwrap() as u32
        );
        assert!(!node.disconnecting_2cd);
        assert_eq!(node.heartbeat_2dc, 0);
        let mut resource = aa_runtime::film_resource::Resource::open_memory(
            "explicit-backend-test-not-real-startup",
        );
        resource.open_chunk("Orders", 0).unwrap();
        resource.flush().unwrap();
        let handle = resource.recorder_handle().unwrap();
        node.orders.client.field_250 = 0x12345678;
        node.orders.client.resume_byte = 0x11223344;
        node.orders.client.resume_bit = 0x55667788;
        node.orders
            .mirror_stream
            .write_uint(0xffffffff, 32)
            .unwrap();
        node.attach_recorder_74d5f0(handle).unwrap();
        assert!(node.recorder_268().unwrap().is_live());
        assert_eq!(node.orders.mirror_stream.position(), Position::PACKET_START);
        assert!(node.orders.mirror_stream.bytes().iter().all(|b| *b == 0));
        assert_eq!(
            node.orders.client.field_250,
            row["attach"]["field250"].as_u64().unwrap() as u32
        );
        assert_eq!(
            node.orders.client.resume_byte,
            row["attach"]["resume_byte"].as_i64().unwrap() as i32
        );
        assert_eq!(
            node.orders.client.resume_bit,
            row["attach"]["resume_bit"].as_i64().unwrap() as i32
        );
        node.orders.mirror_stream.write_uint(0x91, 8).unwrap();
        let pos = node.orders.mirror_stream.position();
        let bytes = node.orders.mirror_stream.bytes().to_vec();
        let borrowed = node.recorder_268().unwrap().clone();
        node.detach_recorder_74b310();
        assert!(node.recorder_268().is_none());
        assert_eq!(node.orders.mirror_stream.position(), pos);
        assert_eq!(node.orders.mirror_stream.bytes(), bytes);
        borrowed.write(&[1, 2, 3]).unwrap();
        resource.close().unwrap();
        assert!(borrowed.write(&[4]).is_err());
    }
}
