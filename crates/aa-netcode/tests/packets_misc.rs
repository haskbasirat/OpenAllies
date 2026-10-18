//! Replays research/phase8-net/native-packets-misc.json: admin packet types
//! 3 (0x74C2D0/0x753820), 6 (0x74E3C0/0x74E430), 7 (0x74C310/0x75A5C0),
//! 8 (0x74B680/0x75A730), 10 (0x74B6B0/0x74F7C0) and 11 (0x74C380/0x75A780).
use aa_netcode::packets::{
    read_backup_to_client, read_client_disconnect_request, read_client_to_backup, read_disconnect_decree,
    read_reconnect, read_server_disconnect, write_backup_to_client, write_client_to_backup, write_disconnect_decree,
    write_reconnect, AdminConn, ConnEvent, NetworkAvars, NodeHistories, Roles,
};
use aa_orders::commands::WireWriter;
use aa_orders::{Cursor, History, Packet, Position};
use serde_json::Value;

fn fixture() -> Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-packets-misc.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}
fn f(c: &Value, k: &str) -> bool {
    c.get(k).and_then(Value::as_u64).unwrap_or(0) != 0
}
fn i(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}
fn bytes(v: &Value) -> Vec<u8> {
    serde_json::from_value(v.clone()).unwrap()
}
fn pos(v: &Value) -> Position {
    serde_json::from_value(v.clone()).unwrap()
}
fn reader(data: &[u8]) -> Vec<u8> {
    let mut d = data.to_vec();
    d.extend([0u8; 8]);
    d
}
fn written(w: &WireWriter, row: &Value, what: &str) {
    let native = bytes(&row["bytes"]);
    assert_eq!(w.position(), pos(&row["end"]), "{what} end");
    assert_eq!(&w.bytes()[..native.len()], &native[..], "{what} bytes");
}
fn logs(row: &Value) -> Vec<u32> {
    row["events"].as_array().unwrap().iter().filter(|e| e["event"] == "log").map(|e| e["line"].as_u64().unwrap() as u32).collect()
}
fn named(row: &Value, name: &str) -> Vec<Value> {
    row["events"].as_array().unwrap().iter().filter(|e| e["event"] == name).cloned().collect()
}
fn assert_history(h: &History, native: &Value, what: &str) {
    assert_eq!(h.begin() as i64, native["begin"].as_i64().unwrap(), "{what} begin");
    assert_eq!(h.end() as i64, native["end"].as_i64().unwrap(), "{what} end");
    for (k, p) in native["packets"].as_array().unwrap().iter().enumerate() {
        let got = h.get(h.begin() + k as i32).unwrap().map(|p| p.bytes().to_vec());
        let want: Option<Vec<u8>> = serde_json::from_value(p.clone()).unwrap();
        assert_eq!(got, want, "{what} packet {k}");
    }
}
fn append(h: &mut History, p: &Value) {
    let e = h.end();
    h.set_end(e + 1).unwrap();
    h.put(e, Some(Packet::new(bytes(p)).unwrap())).unwrap();
}

struct Node {
    requests: History,
    backup_requests: History,
    decrees: History,
    backup_decrees: History,
    next: i32,
}
impl Node {
    fn new() -> Self {
        Self {
            requests: History::new(0, 64).unwrap(),
            backup_requests: History::new(0, 64).unwrap(),
            decrees: History::new(0, 64).unwrap(),
            backup_decrees: History::new(0, 64).unwrap(),
            next: 0,
        }
    }
    fn view(&mut self) -> NodeHistories<'_> {
        NodeHistories {
            requests: &mut self.requests,
            backup_requests: &mut self.backup_requests,
            decrees: &mut self.decrees,
            backup_decrees: &mut self.backup_decrees,
            backup_decree_next: &mut self.next,
        }
    }
}

/// Gate inputs as the oracle's `roles()` applied them (a node that is the
/// host/server is not the conn).
fn roles(c: &Value) -> Roles {
    let we_host = f(c, "we_are_host");
    let we_server = f(c, "we_are_server");
    Roles {
        we_are_host: we_host,
        we_are_server: we_server,
        conn_is_host: !we_host && f(c, "conn_is_host"),
        conn_is_server: !we_server && f(c, "conn_is_server"),
        connecting: f(c, "connecting"),
        joined: f(c, "admin_clock") && f(c, "local_player"),
        backup_mode: false,
        rank: c.get("node_rank").map_or(0, i),
        session_member: f(c, "admin_clock") && f(c, "conn_player"),
        session_state_set: f(c, "session_state"),
        admin_clock: f(c, "admin_clock"),
    }
}
fn conn_for(c: &Value) -> AdminConn {
    let mut conn = AdminConn::new().unwrap();
    conn.connected = f(c, "connected");
    conn.disconnecting = f(c, "disconnecting");
    conn.base.join_order = c.get("conn_rank").map_or(0, i);
    conn.base.player = f(c, "conn_player").then_some(1);
    conn
}
fn ignored(events: &[ConnEvent]) -> Option<u32> {
    match events {
        [ConnEvent::Ignored(l)] => Some(*l),
        _ => None,
    }
}

#[test]
fn reconnect() {
    let v = fixture();
    let rows = v["reconnect"].as_array().unwrap();
    let mut ok = 0;
    for row in rows {
        let c = &row["case"];
        let mut w = WireWriter::new(Position::PACKET_START).unwrap();
        write_reconnect(&mut w, 0x1122_3344).unwrap();
        written(&w, row, "reconnect");
        let mut node = Node::new();
        let mut conn = conn_for(c);
        let data = reader(&bytes(&row["bytes"]));
        let mut r = Cursor::with_bounds(&data, Position::PACKET_START, data.len() * 8).unwrap();
        let ev = read_reconnect(&mut conn, &mut node.view(), &mut r, roles(c), f(c, "most_recent")).unwrap();
        assert_eq!(r.position(), pos(&row["read_end"]), "{c}");
        let st = &row["conn"];
        if let Some(line) = ignored(&ev) {
            assert_eq!(logs(row), vec![line], "{c}");
        } else {
            ok += 1;
            assert!(matches!(ev[..], [ConnEvent::Reconnected { .. }]));
            assert_eq!(conn.reverse_start, i(&st["reverse_start"]));
            assert_eq!(conn.base.backup_requests.begin(), i(&row["conn_requests_begin_backup"]));
        }
        assert_eq!(u64::from(conn.connected), st["connected"].as_u64().unwrap(), "{c}");
        assert_eq!(conn.backup_index, i(&st["backup_index"]), "{c}");
    }
    assert!(ok >= 2 && rows.len() == 62);
}

#[test]
fn client_to_backup_server() {
    let v = fixture();
    let avars = NetworkAvars::default();
    let rows = v["client_to_backup"].as_array().unwrap();
    let mut delivered = 0;
    for row in rows {
        let c = &row["case"];
        // Client side (writer).
        let mut cn = Node::new();
        let mut cc = AdminConn::new().unwrap();
        cc.reset_senders_to_server(&mut cn.view()).unwrap();
        cn.decrees.retire_before(0).unwrap(); // header arrived (0x750880(0))
        for p in c["client_decrees"]["packets"].as_array().unwrap() {
            append(&mut cn.decrees, p);
        }
        cc.reset_backup_senders_to_server(&mut cn.view(), i(&c["reverse_start"])).unwrap();
        for p in c["client_backup_requests"]["packets"].as_array().unwrap() {
            append(&mut cn.backup_requests, p);
        }
        cc.backup_index = if f(c, "index_ok") { 1 } else { 7 };
        cc.base.frame_ratio = f32::from_bits(c["client_status_word"].as_u64().unwrap() as u32);
        let status = cc.status_inputs(&cn.view(), false, f(c, "falling_behind"), &avars);
        let mut w = WireWriter::new(Position::PACKET_START).unwrap();
        write_client_to_backup(&mut cc, &cn.view(), &mut w, 0, status).unwrap();
        written(&w, row, "type 6");
        // Backup server side (reader), bound by 0x752300.
        let mut sn = Node::new();
        let mut sc = AdminConn::new().unwrap();
        sc.reset_backup_senders_to_client(&mut sn.view()).unwrap();
        sc.connected = f(c, "connected");
        sc.disconnecting = f(c, "disconnecting");
        let data = reader(&bytes(&row["bytes"]));
        let mut r = Cursor::with_bounds(&data, Position::PACKET_START, data.len() * 8).unwrap();
        let ev = read_client_to_backup(&mut sc, &mut r, roles(c), f(c, "most_recent")).unwrap();
        assert_eq!(r.position(), pos(&row["read_end"]), "{c}");
        if let Some(line) = ignored(&ev) {
            assert_eq!(logs(row), vec![line], "{c}");
        } else {
            delivered += 1;
        }
        let st = &row["conn"];
        assert_eq!(sc.base.frame_ratio.to_bits() as u64, st["status_word"].as_u64().unwrap(), "{c}");
        assert_eq!(u64::from(sc.status_backlog), st["backlog"].as_u64().unwrap(), "{c}");
        assert_eq!(u64::from(sc.status_falling_behind), st["falling_behind"].as_u64().unwrap(), "{c}");
        assert_history(&sc.base.backup_requests, &row["server_backup_requests"], "backup requests");
        assert_history(&sc.reverse_history, &row["server_reverse"], "reverse decrees");
    }
    assert!(delivered >= 4 && rows.len() == 44, "{delivered}");
}

#[test]
fn backup_server_to_client() {
    let v = fixture();
    let rows = v["backup_to_client"].as_array().unwrap();
    let (mut adopted, mut skipped_past_end) = (0, 0);
    for row in rows {
        let c = &row["case"];
        // Backup server: two backup decrees, 0x752300 (binds at end), two more.
        let mut sn = Node::new();
        let native = row["server_backup_decrees"]["packets"].as_array().unwrap();
        append(&mut sn.backup_decrees, &native[0]);
        append(&mut sn.backup_decrees, &native[1]);
        let mut sc = AdminConn::new().unwrap();
        sc.reset_backup_senders_to_client(&mut sn.view()).unwrap();
        append(&mut sn.backup_decrees, &native[2]);
        append(&mut sn.backup_decrees, &native[3]);
        let data = bytes(&row["bytes"]);
        let mut peek = Cursor::for_packet(&data, None).unwrap();
        peek.read_uint(32).unwrap();
        let decree_end = peek.read_uint(32).unwrap() as i32;
        let mut w = WireWriter::new(Position::PACKET_START).unwrap();
        write_backup_to_client(&mut sc, &mut w, 0, decree_end, &sn.backup_decrees).unwrap();
        written(&w, row, "type 7");
        // Client.
        let mut cn = Node::new();
        let mut cc = AdminConn::new().unwrap();
        cc.reset_senders_to_server(&mut cn.view()).unwrap();
        if f(c, "index_same") {
            cc.reset_backup_senders_to_server(&mut cn.view(), 0).unwrap();
            cc.backup_index = 1;
        }
        cc.base.join_order = i(&c["conn_rank"]);
        let native_end = pos(&row["read_end"]).absolute().unwrap();
        let data_end = pos(&row["end"]).absolute().unwrap();
        let mut r = Cursor::with_bounds(&data, Position::PACKET_START, data_end).unwrap();
        let result = read_backup_to_client(&mut cc, &mut cn.view(), &mut r, roles(c), f(c, "most_recent"));
        if native_end > data_end {
            // Native hazard: the skip 0x7638B0 parses the byte-buffer header
            // block 0 as a bit block (0x7633C0) and reads past the datagram
            // into whatever memory follows; the port stops with a typed error.
            assert!(result.is_err(), "{c}");
            skipped_past_end += 1;
            continue;
        }
        let ev = result.unwrap_or_else(|e| panic!("{c} {}: {e}", row["read_end"]));
        assert_eq!(r.position(), pos(&row["read_end"]), "{c}");
        if let Some(line) = ignored(&ev) {
            assert_eq!(logs(row), vec![line], "{c}");
            assert!(named(row, "set_server").is_empty());
        } else {
            let adopt = ev.iter().any(|e| matches!(e, ConnEvent::BackupServerAdopted));
            assert_eq!(adopt, !named(row, "set_server").is_empty(), "{c}");
            adopted += usize::from(adopt);
        }
        let st = &row["conn"];
        assert_eq!(u64::from(cc.connected), st["connected"].as_u64().unwrap(), "{c}");
        assert_eq!(cc.backup_index, i(&st["backup_index"]), "{c}");
        assert_eq!(cn.next, i(&st["backup_decree_next"]), "{c}");
        assert_history(&cn.backup_decrees, &row["client_backup_decrees"], "client backup decrees");
        assert_history(&cn.backup_requests, &row["client_backup_requests"], "client backup requests");
    }
    assert!(adopted >= 1 && skipped_past_end >= 1 && rows.len() == 54, "{adopted} {skipped_past_end}");
}

#[test]
fn empty_bodies_and_disconnects() {
    let v = fixture();
    for row in v["empty_bodies"].as_array().unwrap() {
        let c = &row["case"];
        let w = WireWriter::new(Position::PACKET_START).unwrap();
        written(&w, row, "empty body");
        let mut conn = conn_for(c);
        let ev = if row["type"] == 8 {
            let mut timeouts = 0;
            let ev = read_server_disconnect(&mut conn, roles(c), &mut |_| {
                timeouts += 1;
                Ok(())
            })
            .unwrap();
            assert_eq!(timeouts, named(row, "timeout").len(), "{c}");
            ev
        } else {
            let ev = read_client_disconnect_request(&mut conn, roles(c));
            let leave = named(row, "player_leave_request");
            match &ev[..] {
                [ConnEvent::ClientLeaveRequested { reason }] => {
                    assert_eq!(leave.len(), 1);
                    assert_eq!(leave[0]["reason"].as_u64().unwrap() as u32, *reason);
                    assert_eq!(leave[0]["player"], "player");
                }
                _ => assert!(leave.is_empty(), "{c}"),
            }
            ev
        };
        if let Some(line) = ignored(&ev) {
            assert_eq!(logs(row), vec![line], "{c}");
        }
        assert_eq!(u64::from(conn.server_disconnected), row["conn"]["server_disconnected"].as_u64().unwrap(), "{c}");
    }
    let rows = v["disconnect_decree"].as_array().unwrap();
    let mut accepted = 0;
    for row in rows {
        let c = &row["case"];
        let mut conn = AdminConn::new().unwrap();
        conn.disconnect_reason = row["reason_written"].as_u64().unwrap() as u32;
        let mut w = WireWriter::new(Position::PACKET_START).unwrap();
        write_disconnect_decree(&conn, &mut w).unwrap();
        written(&w, row, "type 11");
        let data = reader(&bytes(&row["bytes"]));
        let mut r = Cursor::with_bounds(&data, Position::PACKET_START, data.len() * 8).unwrap();
        let ev = read_disconnect_decree(&mut r, roles(c)).unwrap();
        assert_eq!(r.position(), pos(&row["read_end"]), "{c}");
        match &ev[..] {
            [ConnEvent::Ignored(line)] => {
                assert_eq!(logs(row), vec![*line], "{c}");
                assert!(!row["reached_dialog"].as_bool().unwrap());
            }
            [ConnEvent::DisconnectedByServer { reason, take_over }] => {
                accepted += 1;
                assert!(row["reached_dialog"].as_bool().unwrap());
                assert_eq!(*reason, row["reason_written"].as_u64().unwrap() as u32);
                assert_eq!(named(row, "admin_stop").len(), 1);
                assert_eq!(!named(row, "set_server").is_empty(), *take_over, "{c}");
                assert_eq!(!named(row, "set_host").is_empty(), *take_over, "{c}");
                assert_eq!(row["node_2e0"].as_u64().unwrap(), if *take_over { 2 } else { 0 });
            }
            other => panic!("{other:?}"),
        }
    }
    assert!(accepted >= 4, "{accepted}");
}
