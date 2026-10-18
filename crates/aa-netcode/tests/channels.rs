//! Replays research/phase8-net/native-channels.json: original
//! TGC_MessageHistorySender/Receiver over TGC_BlockSender/Receiver.
use aa_netcode::history::{HistoryReceiver, HistorySender};
use aa_orders::commands::WireWriter;
use aa_orders::{Cursor, History, Packet, Position};
use serde_json::Value;

fn fixture() -> Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-channels.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}
fn bytes(v: &Value) -> Vec<u8> {
    serde_json::from_value(v.clone()).unwrap()
}
fn pos(v: &Value) -> Position {
    serde_json::from_value(v.clone()).unwrap()
}

#[test]
fn native_message_history_channels() {
    let v = fixture();
    let mut steps = 0;
    for sc in v["scenarios"].as_array().unwrap() {
        let name = sc["name"].as_str().unwrap();
        let start = sc["start"].as_i64().unwrap() as i32;
        let mut h = History::new(start, 64).unwrap();
        for p in sc["initial"].as_array().unwrap() {
            let i = h.end();
            h.put(i, Some(Packet::new(bytes(p)).unwrap())).unwrap();
        }
        let mut h2 = History::new(0, 64).unwrap();
        let mut sender = HistorySender::default();
        sender.bind(start).unwrap();
        let mut receiver = HistoryReceiver::new().unwrap();
        receiver.bind(&mut h2).unwrap();
        for step in sc["steps"].as_array().unwrap() {
            let seq = step["sequence"].as_i64().unwrap() as i32;
            if step["grow"].as_bool().unwrap() {
                for p in sc["later"].as_array().unwrap() {
                    let i = h.end();
                    h.put(i, Some(Packet::new(bytes(p)).unwrap())).unwrap();
                }
            }
            let mut w = WireWriter::new(Position { byte: 0, bit: 3 }).unwrap();
            sender.serialize(&h, &mut w, seq, step["limit"].as_i64().unwrap() as i32).unwrap();
            let native = bytes(&step["bytes"]);
            let end = pos(&step["end"]);
            assert_eq!(w.position(), end, "{name} seq {seq}");
            assert_eq!(&w.bytes()[..native.len()], &native[..], "{name} seq {seq}");
            if step["delivered"].as_bool().unwrap() {
                let mut data = native.clone();
                data.extend([0u8; 8]);
                let mut r = Cursor::with_bounds(&data, Position { byte: 0, bit: 3 }, data.len() * 8).unwrap();
                receiver.receive(&mut h2, &mut r).unwrap();
                assert_eq!(r.position(), pos(&step["read_end"]), "{name} seq {seq} read end");
            }
            for res in step["results"].as_array().unwrap() {
                let s = res[0].as_i64().unwrap() as i32;
                sender.packet_result(&h, s, res[1].as_bool().unwrap()).unwrap();
            }
            // Sender records.
            let st = &step["sender"];
            assert_eq!(sender.sender.packets.begin as i64, st["packets_begin"].as_i64().unwrap(), "{name} {seq}");
            let packets: Vec<[i64; 4]> = sender
                .sender
                .packets
                .items
                .iter()
                .map(|p| [p.state as i64, p.first as i64, p.end as i64, p.mask as i64])
                .collect();
            let native_packets: Vec<[i64; 4]> = serde_json::from_value(st["packets"].clone()).unwrap();
            assert_eq!(packets, native_packets, "{name} seq {seq} packet records");
            assert_eq!(sender.sender.blocks.begin as i64, st["blocks_begin"].as_i64().unwrap());
            let blocks: Vec<[i64; 2]> =
                sender.sender.blocks.items.iter().map(|b| [b.inflight as i64, b.estimate as i64]).collect();
            let native_blocks: Vec<[i64; 2]> = serde_json::from_value(st["blocks"].clone()).unwrap();
            assert_eq!(blocks, native_blocks, "{name} seq {seq} block records");
            assert_eq!(sender.blocks.begin as i64, st["history_blocks_begin"].as_i64().unwrap());
            let hb: Vec<[i64; 3]> = sender
                .blocks
                .items
                .iter()
                .map(|b| [b.index as i64, i64::from(b.last), b.bits as i64])
                .collect();
            let native_hb: Vec<[i64; 3]> = serde_json::from_value(st["history_blocks"].clone()).unwrap();
            assert_eq!(hb, native_hb, "{name} seq {seq} history blocks");
            assert_eq!(sender.next as i64, st["next"].as_i64().unwrap());
            // Receiver window and reassembled history.
            let rs = &step["receiver"];
            assert_eq!(receiver.blocks.begin as i64, rs["blocks_begin"].as_i64().unwrap(), "{name} {seq}");
            assert_eq!(receiver.blocks.end() as i64, rs["blocks_end"].as_i64().unwrap(), "{name} {seq}");
            let nh = &step["history"];
            assert_eq!(h2.begin() as i64, nh["begin"].as_i64().unwrap(), "{name} seq {seq} history begin");
            assert_eq!(h2.end() as i64, nh["end"].as_i64().unwrap(), "{name} seq {seq} history end");
            for (k, p) in nh["packets"].as_array().unwrap().iter().enumerate() {
                let got = h2.get(h2.begin() + k as i32).unwrap().map(|p| p.bytes().to_vec());
                let want: Option<Vec<u8>> = serde_json::from_value(p.clone()).unwrap();
                assert_eq!(got, want, "{name} seq {seq} packet {k}");
            }
            steps += 1;
        }
    }
    assert!(steps > 90);
}
