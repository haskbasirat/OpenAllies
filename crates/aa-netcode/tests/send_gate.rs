//! Replays research/phase8-net/native-send-gate.json: original
//! GetWantedPacketType 0x74BDE0 (with 0x74B600 and 0x74BD80) on both images
//! at PC24 and PC53.
use aa_netcode::send::{wanted_packet_type, GateConn, GateNode};
use aa_orders::net::transport::X87Precision;
use serde_json::Value;

fn fixture() -> Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-send-gate.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}
fn b(v: &Value) -> bool {
    v.as_u64().unwrap() != 0
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

#[test]
fn native_send_gate() {
    let v = fixture();
    let mut total = 0;
    let mut seen = std::collections::BTreeSet::new();
    for image in ["original", "v14"] {
        for c in v["images"][image].as_array().unwrap() {
            let node = GateNode {
                connecting: b(&c["connecting"]),
                we_are_host: b(&c["we_are_host"]),
                we_are_server: b(&c["we_are_server"]),
                leaving: b(&c["leaving"]),
                leave_now: b(&c["leave_now"]),
                rank: c["node_rank"].as_i64().unwrap() as i32,
                admin_clock: b(&c["admin_clock"]),
                backup_since: u(&c["backup_since"]),
            };
            // The fixture's pointer layout: a node that is host/server is not the conn.
            let conn = GateConn {
                is_host: !node.we_are_host && b(&c["conn_is_host"]),
                is_server: !node.we_are_server && b(&c["conn_is_server"]),
                disconnecting: b(&c["disconnecting"]),
                connected: b(&c["connected"]),
                rank: c["conn_rank"].as_i64().unwrap() as i32,
                player: b(&c["player"]),
                sent_this_pass: u(&c["sent_this_pass"]),
                last_send: u(&c["last_send"]),
            };
            let precision = match c["precision"].as_str().unwrap() {
                "Single" => X87Precision::Single,
                _ => X87Precision::Double,
            };
            let mut calls = 0;
            let backlog = b(&c["backlog"]);
            let got = wanted_packet_type(&node, &conn, u(&c["now"]), &mut || {
                calls += 1;
                Ok(backlog)
            }, precision)
            .unwrap();
            assert_eq!(got.packet_type, u(&c["packet_type"]), "{image} {c}");
            assert_eq!(calls, c["backlog_called"].as_u64().unwrap() as usize, "{image} {c}");
            let logs: Vec<u32> = c["log"].as_array().unwrap().iter().map(u).collect();
            assert_eq!(got.log_line.into_iter().collect::<Vec<_>>(), logs, "{image} {c}");
            seen.insert(got.packet_type);
            total += 1;
        }
    }
    assert_eq!(total, 4096);
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), vec![0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
}
