//! Replays research/phase8-net/native-transport.json: the original NetConn
//! header writer (0x764DA0/0x764CA0/0x764AF0) and reader (0x764BB0/0x764600/
//! 0x764550) on synthetic connection states, PC24 and PC53.
use aa_orders::commands::WireWriter;
use aa_orders::net::transport::{
    ReceivedFlags, Received, SentTimes, Sequencing, X87Precision,
};
use aa_orders::{Cursor, Position};
use serde_json::Value;

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-transport.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn state(v: &Value) -> Sequencing {
    let f = |k: &str| v[k].as_f64().unwrap() as f32;
    let i = |k: &str| v[k].as_i64().unwrap() as i32;
    Sequencing {
        peer_session: i("peer_session"),
        sent: SentTimes {
            begin: v["sent"]["begin"].as_i64().unwrap() as i32,
            times: v["sent"]["times"].as_array().unwrap().iter().map(|t| t.as_f64().unwrap() as f32).collect(),
        },
        received: ReceivedFlags {
            begin: v["received"]["begin"].as_i64().unwrap() as i32,
            flags: v["received"]["flags"].as_array().unwrap().iter().map(|t| t.as_bool().unwrap()).collect(),
        },
        last_advance: f("last_advance"),
        last_sent: i("last_sent"),
        highest: i("highest"),
        rtt: f("rtt"),
        sent_this_pass: v["sent_this_pass"].as_u64().unwrap() as u32,
        last_send: f("last_send"),
    }
}
fn precision(v: &Value) -> X87Precision {
    match v.as_str().unwrap() {
        "Single" => X87Precision::Single,
        "Double" => X87Precision::Double,
        _ => X87Precision::Extended,
    }
}
fn same(a: &Sequencing, b: &Sequencing) -> bool {
    // Compare floats bitwise (fixture floats round-trip exactly through f64).
    a.peer_session == b.peer_session
        && a.sent.begin == b.sent.begin
        && a.sent.times.iter().map(|x| x.to_bits()).eq(b.sent.times.iter().map(|x| x.to_bits()))
        && a.received == b.received
        && a.last_advance.to_bits() == b.last_advance.to_bits()
        && a.last_sent == b.last_sent
        && a.highest == b.highest
        && a.rtt.to_bits() == b.rtt.to_bits()
        && a.sent_this_pass == b.sent_this_pass
        && a.last_send.to_bits() == b.last_send.to_bits()
}

#[test]
fn native_datagram_header_writer_and_reader() {
    let v = fixture();
    let (mut writes, mut reads) = (0, 0);
    for case in v["cases"].as_array().unwrap() {
        let mut s = state(&case["before"]);
        let after = state(&case["after"]);
        let own = case["own_session"].as_i64().unwrap() as i32;
        let now = case["now"].as_f64().unwrap() as f32;
        let p = precision(&case["precision"]);
        let bytes: Vec<u8> = serde_json::from_value(case["bytes"].clone()).unwrap();
        let end: Position = serde_json::from_value(case["end"].clone()).unwrap();
        if case["kind"] == "write" {
            let mut w = WireWriter::new(Position { byte: 0, bit: 3 }).unwrap();
            let seq = s.write_header(&mut w, own).unwrap();
            assert_eq!(case["payload_sequence"][0].as_i64().unwrap() as i32, seq);
            s.sent(seq, now);
            assert_eq!(w.position(), end, "{case}");
            assert_eq!(&w.bytes()[..bytes.len()], &bytes[..], "{case}");
            assert!(same(&s, &after), "{case}\n{s:?}");
            writes += 1;
        } else {
            let mut data = bytes.clone();
            data.extend([0u8; 8]);
            let mut r = Cursor::with_bounds(&data, Position { byte: 0, bit: 3 }, data.len() * 8).unwrap();
            let result = s.receive(&mut r, own, now, p).unwrap();
            let lines: Vec<u64> = serde_json::from_value(case["log_lines"].clone()).unwrap();
            let acks: Vec<(i32, u8)> = serde_json::from_value(case["acks"].clone()).unwrap();
            let dispatched: Vec<i32> = serde_json::from_value(case["dispatched"].clone()).unwrap();
            match &result {
                Received::NoSessionIds => assert_eq!(lines, [0x391]),
                Received::InvalidSender => assert_eq!(lines, [0x377]),
                Received::InvalidTarget => assert_eq!(lines, [0x37e]),
                Received::OutOfOrder { .. } => assert_eq!(lines, [0x3fd]),
                Received::Accepted { sequence, first, acks: got } => {
                    assert_eq!(lines, if *first { vec![0x3e6] } else { vec![] });
                    assert_eq!(dispatched, [*sequence]);
                    let got: Vec<(i32, u8)> = got.iter().map(|&(a, b)| (a, u8::from(b))).collect();
                    assert_eq!(got, acks, "{case}");
                    assert_eq!(r.position(), end, "{case}");
                }
            }
            if !matches!(result, Received::Accepted { .. }) {
                assert!(dispatched.is_empty() && acks.is_empty());
            }
            assert!(same(&s, &after), "{case}\n{s:?}");
            reads += 1;
        }
    }
    assert!(writes > 10 && reads > 100, "{writes} {reads}");
}
