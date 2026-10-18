//! Replays research/phase8-net/native-packets.json: two emulated original
//! nodes (host and client) with native TGC_AdminConn objects (ctor 0x756BC0,
//! bindings 0x752490 / 0x7525B0) exchanging ServerToClient (0x74E280 /
//! 0x75A350) and ClientToServer (0x74E0F0 / 0x74E170) bodies, with losses,
//! late (not most recent) datagrams, file generations (0x74C8B0) and ack
//! results through 0x74B4F0.
use aa_netcode::packets::{
    read_client_to_server, read_server_to_client, write_client_to_server, write_server_to_client, AdminConn,
    NetworkAvars, NoFileTransfer, NodeHistories, Roles,
};
use aa_netcode::NativeImage;
use aa_orders::commands::WireWriter;
use aa_orders::{Cursor, History, Packet, Position};
use serde_json::Value;

fn fixture() -> Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-packets.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}
fn bytes(v: &Value) -> Vec<u8> {
    serde_json::from_value(v.clone()).unwrap()
}
fn pos(v: &Value) -> Position {
    serde_json::from_value(v.clone()).unwrap()
}
fn int(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}

struct Node {
    requests: History,
    backup_requests: History,
    decrees: History,
    backup_decrees: History,
    backup_decree_next: i32,
}
impl Node {
    fn new(decree_start: i32) -> Self {
        Self {
            requests: History::new(0, 64).unwrap(),
            backup_requests: History::new(0, 64).unwrap(),
            decrees: History::new(decree_start, 64).unwrap(),
            backup_decrees: History::new(0, 64).unwrap(),
            backup_decree_next: 0,
        }
    }
    fn view(&mut self) -> NodeHistories<'_> {
        NodeHistories {
            requests: &mut self.requests,
            backup_requests: &mut self.backup_requests,
            decrees: &mut self.decrees,
            backup_decrees: &mut self.backup_decrees,
            backup_decree_next: &mut self.backup_decree_next,
        }
    }
}

fn roles() -> Roles {
    Roles {
        we_are_host: false,
        we_are_server: false,
        conn_is_host: false,
        conn_is_server: false,
        connecting: false,
        joined: true,
        backup_mode: false,
        rank: 0,
        session_member: false,
        session_state_set: false,
        admin_clock: false,
    }
}

fn append(h: &mut History, packet: &Value) {
    let i = h.end();
    h.put(i, Some(Packet::new(bytes(packet)).unwrap())).unwrap();
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

fn written(w: &WireWriter, native: &Value, end: &Value, what: &str) {
    let native = bytes(native);
    assert_eq!(w.position(), pos(end), "{what} end");
    assert_eq!(&w.bytes()[..native.len()], &native[..], "{what} bytes");
}

fn reader(data: &[u8]) -> Vec<u8> {
    let mut d = data.to_vec();
    d.extend([0u8; 8]);
    d
}

#[test]
fn native_admin_packet_bodies() {
    let v = fixture();
    let avars = NetworkAvars::default();
    let mut steps = 0;
    let (mut late, mut skipped_generation, mut backlog_seen) = (0, 0, 0);
    let mut send_backlog = [0usize; 2];
    for (image_name, image) in [("original", NativeImage::Original), ("v14", NativeImage::V14)] {
    for sc in v["images"][image_name].as_array().unwrap() {
        let name = sc["name"].as_str().unwrap();
        let start = int(&sc["start"]);
        let mut host = Node::new(start);
        let mut client = Node::new(0);
        let mut hc = AdminConn::new().unwrap();
        let mut cc = AdminConn::new().unwrap();
        hc.connected = true;
        cc.connected = true;
        let mut hf = NoFileTransfer::new(&avars);
        let mut cf = NoFileTransfer::new(&avars);
        // As the host join completion 0x753580: 0x752300 then 0x752490(start).
        hc.reset_backup_senders_to_client(&mut host.view()).unwrap();
        hc.reset_senders_to_client(start).unwrap();
        cc.reset_senders_to_server(&mut client.view()).unwrap();
        let host_write = roles();
        let client_read = Roles { conn_is_host: true, conn_is_server: true, ..roles() };
        let host_read = Roles { we_are_host: true, we_are_server: true, ..roles() };
        let mut generation = 0;
        for step in sc["steps"].as_array().unwrap() {
            let seq = int(&step["sequence"]);
            let what = format!("{name} seq {seq}");
            for d in step["added_decrees"].as_array().unwrap() {
                append(&mut host.decrees, d);
            }
            if !step["added_private"].is_null() {
                append(&mut hc.private_history, &step["added_private"]);
            }
            if !step["added_request"].is_null() {
                append(&mut client.requests, &step["added_request"]);
            }
            let g = int(&step["generation"]);
            if g < generation {
                skipped_generation += 1;
            }
            generation = g;
            hc.file_generation = g;
            cc.base.frame_ratio = f32::from_bits(step["status_word"].as_u64().unwrap() as u32);
            let behind = step["falling_behind"].as_bool().unwrap();

            let before = hc.backlogged(&host.view(), true, &avars, image, 0, &hf).unwrap();
            assert_eq!(u64::from(before), step["backlog_before"].as_u64().unwrap(), "{what} {image_name} backlog before");
            send_backlog[usize::from(image == NativeImage::V14)] += usize::from(before);
            let mut w = WireWriter::new(Position { byte: 0, bit: 3 }).unwrap();
            write_server_to_client(&mut hc, &host.view(), &mut w, seq, host_write, image, &mut hf).unwrap();
            written(&w, &step["down"], &step["down_end"], &format!("{what} ServerToClient"));

            let status = cc.status_inputs(&client.view(), true, behind, &avars);
            backlog_seen += usize::from(status.backlog);
            let mut w = WireWriter::new(Position { byte: 0, bit: 3 }).unwrap();
            write_client_to_server(&mut cc, &client.view(), &mut w, seq, status).unwrap();
            written(&w, &step["up"], &step["up_end"], &format!("{what} ClientToServer"));

            // Bodies by sequence, as recorded for this and earlier steps.
            for d in step["deliveries"].as_array().unwrap() {
                let s = int(&d["sequence"]);
                let source = sc["steps"].as_array().unwrap().iter().find(|x| int(&x["sequence"]) == s).unwrap();
                if !d["down_read_end"].is_null() {
                    let recent = d["down_most_recent"].as_bool().unwrap();
                    late += usize::from(!recent);
                    let data = reader(&bytes(&source["down"]));
                    let mut r = Cursor::with_bounds(&data, Position { byte: 0, bit: 3 }, data.len() * 8).unwrap();
                    read_server_to_client(&mut cc, &mut client.view(), &mut r, client_read, recent, &mut cf).unwrap();
                    assert_eq!(r.position(), pos(&d["down_read_end"]), "{what} read ServerToClient {s}");
                }
                if !d["up_read_end"].is_null() {
                    let recent = d["up_most_recent"].as_bool().unwrap();
                    let data = reader(&bytes(&source["up"]));
                    let mut r = Cursor::with_bounds(&data, Position { byte: 0, bit: 3 }, data.len() * 8).unwrap();
                    read_client_to_server(&mut hc, &mut r, host_read, recent).unwrap();
                    assert_eq!(r.position(), pos(&d["up_read_end"]), "{what} read ClientToServer {s}");
                }
            }
            for res in step["host_results"].as_array().unwrap() {
                hc.packet_result(&host.view(), int(&res[0]), res[1].as_bool().unwrap(), &mut hf).unwrap();
            }
            for res in step["client_results"].as_array().unwrap() {
                cc.packet_result(&client.view(), int(&res[0]), res[1].as_bool().unwrap(), &mut cf).unwrap();
            }
            let after = hc.backlogged(&host.view(), true, &avars, image, 0, &hf).unwrap();
            assert_eq!(u64::from(after), step["backlog_after"].as_u64().unwrap(), "{what} {image_name} backlog after");
            assert_history(&client.decrees, &step["client_decrees"], &format!("{what} client decrees"));
            assert_history(&cc.private_history, &step["client_private"], &format!("{what} client private"));
            assert_history(&hc.base.requests, &step["host_conn_requests"], &format!("{what} host conn requests"));
            assert_eq!(cc.file_generation, int(&step["client_file_generation"]), "{what} generation");
            assert_eq!(cc.backup_index, int(&step["client_backup_index"]), "{what} client backup index");
            assert_eq!(client.backup_decree_next, int(&step["client_backup_decree_next"]), "{what} AdminNode+0x234");
            let st = &step["host_conn_status"];
            assert_eq!(hc.base.frame_ratio.to_bits() as u64, st["word"].as_u64().unwrap(), "{what} status word");
            assert_eq!(u64::from(hc.status_backlog), st["backlog"].as_u64().unwrap(), "{what} backlog");
            assert_eq!(u64::from(hc.status_falling_behind), st["falling_behind"].as_u64().unwrap(), "{what} behind");
            steps += 1;
        }
    }
    }
    assert_eq!(steps, 292, "{steps}");
    assert!(send_backlog[0] > 0 && send_backlog[1] > send_backlog[0], "{send_backlog:?}");
    assert!(late > 0 && skipped_generation > 0 && backlog_seen > 0, "{late} {skipped_generation} {backlog_seen}");
}
