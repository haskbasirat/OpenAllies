//! Replays research/phase8-net/native-connect.json: original Connect writer
//! 0x750330 on a client node, reader/validation 0x75BF90 on a host node and,
//! when accepted, the PlayerJoin request payload 0x77B680.
use aa_netcode::packets::AdminConn;
use aa_netcode::session::{
    encode_player_join, read_and_validate_connect, write_connect, ConnectInfo, ConnectVerdict, HostConnectState,
    PeerAddress, PlayerId,
};
use aa_orders::commands::WireWriter;
use aa_orders::{Cursor, Position};
use serde_json::Value;

fn fixture() -> Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-connect.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}
fn units(v: &Value) -> Vec<u16> {
    v.as_str().unwrap().encode_utf16().collect()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn b(v: &Value) -> bool {
    v.as_bool().unwrap()
}
fn bytes(v: &Value) -> Vec<u8> {
    serde_json::from_value(v.clone()).unwrap()
}
fn pos(v: &Value) -> Position {
    serde_json::from_value(v.clone()).unwrap()
}

#[test]
fn native_connect_and_validation() {
    let v = fixture();
    let cases = v["cases"].as_array().unwrap();
    let (mut accepted, mut rejected, mut ignored) = (0, 0, 0);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let cl = &c["client"];
        let pid = &cl["player_id"];
        let info = ConnectInfo {
            name: units(&cl["name"]),
            password: units(&cl["password"]),
            version: units(&cl["version"]),
            build: units(&cl["build"]),
            data_checksum: u(&cl["checksum"]),
            depot_checksums: units(&cl["depot"]),
            mods: units(&cl["mods"]),
            player_id: PlayerId {
                name: units(&pid["name"]),
                serial: u(&pid["serial"]),
                a: u(&pid["a"]),
                b: u(&pid["b"]),
                c: u(&pid["c"]),
            },
            internet: b(&cl["internet"]),
        };
        let mut w = WireWriter::new(Position { byte: 0, bit: 3 }).unwrap();
        write_connect(&mut w, &info).unwrap();
        let native = bytes(&c["bytes"]);
        assert_eq!(w.position(), pos(&c["end"]), "{name} end");
        assert_eq!(&w.bytes()[..native.len()], &native[..], "{name} bytes");

        let h = &c["host"];
        let host = HostConnectState {
            we_are_host: b(&h["we_are_host"]),
            migrated_host: b(&h["migrated_host"]),
            internet: b(&h["internet"]),
            session_mode: h["session_mode"].as_i64().unwrap() as i32,
            launch_countdown: h["session_players_in_game"].as_i64().unwrap() as i32,
            password: units(&h["password"]),
            version: units(&h["version"]),
            build: units(&h["build"]),
            data_checksum: u(&h["checksum"]),
            depot_checksums: units(&h["depot"]),
            mods: units(&h["mods"]),
        };
        let mut conn = AdminConn::new().unwrap();
        conn.accepted = b(&h["conn_accepted"]);
        conn.connected = b(&h["conn_connected"]);
        conn.disconnecting = b(&h["conn_disconnecting"]);
        let now = (h["now"].as_f64().unwrap() as f32).to_bits();
        let peer = PeerAddress { ip: u(&h["peer_ip"]), port: u(&h["peer_port"]) as u16 };
        let mut lookups = Vec::new();
        let ban = b(&h["banned"]);
        let mut data = native.clone();
        data.extend([0u8; 8]);
        let mut r = Cursor::with_bounds(&data, Position { byte: 0, bit: 3 }, data.len() * 8).unwrap();
        let verdict = read_and_validate_connect(&mut conn, &mut r, &host, peer, now, &mut |key: &[u16]| {
            lookups.push(String::from_utf16(key).unwrap());
            ban
        })
        .unwrap();
        assert_eq!(r.position(), pos(&c["read_end"]), "{name} read end");

        // Native events in order.
        let mut events: Vec<String> = lookups.iter().map(|_| "ban_lookup".to_string()).collect();
        let native_events: Vec<&Value> = c["events"].as_array().unwrap().iter().collect();
        for e in &native_events {
            if e["event"] == "ban_lookup" {
                assert_eq!(e["key"].as_str().unwrap(), lookups[0], "{name} ban key");
            }
        }
        match &verdict {
            ConnectVerdict::Ignored(_) => ignored += 1,
            ConnectVerdict::Accepted { request, .. } => {
                accepted += 1;
                events.push("player_join_request".into());
                let ev = native_events.last().unwrap();
                let mut pw = WireWriter::new(Position { byte: 0, bit: 3 }).unwrap();
                encode_player_join(request, &mut pw).unwrap();
                let payload = bytes(&ev["payload"]);
                assert_eq!(pw.position(), pos(&ev["payload_end"]), "{name} payload end");
                assert_eq!(&pw.bytes()[..payload.len()], &payload[..], "{name} payload");
                assert_eq!(ev["vtable"], "0xa70e3c");
            }
            ConnectVerdict::Rejected { banned_prompt, .. } => {
                rejected += 1;
                if *banned_prompt {
                    events.push("banned_prompt".into());
                }
            }
        }
        let native_names: Vec<String> =
            native_events.iter().map(|e| e["event"].as_str().unwrap().to_string()).collect();
        assert_eq!(events, native_names, "{name} events");
        let nc = &c["conn"];
        assert_eq!(u32::from(conn.accepted), u(&nc["accepted"]), "{name} accepted");
        assert_eq!(u32::from(conn.disconnecting), u(&nc["disconnecting"]), "{name} disconnecting");
        assert_eq!(conn.disconnect_reason, u(&nc["reason"]), "{name} reason");
        assert_eq!(conn.disconnect_started, u(&nc["disconnect_time_bits"]), "{name} time");
    }
    assert_eq!(cases.len(), 31);
    assert!(accepted >= 9 && rejected >= 15 && ignored == 4, "{accepted} {rejected} {ignored}");
}
