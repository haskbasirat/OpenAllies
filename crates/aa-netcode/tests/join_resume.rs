//! Replays research/phase8-net/native-join-resume.json: the original host
//! request dispatch 0x74D710 over a request packet holding a PlayerJoin
//! request (306) among speed requests, and the original client replay
//! 0x74FD10 of the resulting decree packet, once resumed at the admin-state
//! pair (AdminNode+0x200/+0x204 captured inside the PlayerJoin process,
//! copied to the client's +0x248/+0x24C) and once from the packet start.
//!
//! Checked against the native run: the decree packet bytes, every
//! synchronizer entry (log, label, value, hash before) on both sides, the
//! process order, the resume pair (= the decree stream position after the
//! PlayerJoin decree), and that only the resumed client continues the
//! host's synchronizer A sequence.
use aa_orders::flow::{Log, OrderHost, SyncEntry, Validity};
use aa_orders::net::admin::AdminNode;
use aa_orders::net::sync::Synchronizer;
use aa_orders::net::transport::finalize;
use aa_orders::payload::{self, Field, NameKind, Registry, Resolver, SchemaContext, ThingCheck};
use aa_orders::{commands::WireWriter, dispatch_observed, flow, Gate, Packet, Position};
use serde_json::{json, Value};

const PLAYER_JOIN_DECREE: usize = 317;
const SPEED_DECREE: usize = 73;
const SET_PLAYER_DECREE: usize = 298;

struct Ctx;
impl SchemaContext for Ctx {
    fn registry_count(&self, _r: Registry) -> aa_orders::Result<usize> {
        Ok(7)
    }
    fn team_command_flags(&self, _i: u32) -> aa_orders::Result<u32> {
        Ok(0)
    }
    fn definition_kind(&self, _r: Registry, _i: u32) -> aa_orders::Result<u32> {
        Ok(0)
    }
    fn zero_width_list_budget(&self) -> usize {
        0
    }
}
impl Resolver for Ctx {
    fn thing(&self, _c: ThingCheck, slot: u16) -> Option<u16> {
        Some(slot)
    }
    fn player(&self, _id: u32) -> bool {
        true
    }
    fn definition(&self, _r: Registry, i: u32) -> bool {
        i < 7
    }
    fn named(&self, _k: NameKind, key: &[u16]) -> Option<Vec<u16>> {
        Some(key.to_vec())
    }
}

/// Synchronizers A and S with every entry recorded as the native harness
/// records the original loggers (hash before the entry).
struct Node {
    a: Synchronizer,
    s: Synchronizer,
    log: Vec<Value>,
    processes: Vec<&'static str>,
    stream_bits: u32,
    capture: Option<Value>,
}
impl Node {
    fn new(a_index: i64, a_hash: i64) -> Self {
        let mut a = Synchronizer::new(0x10000);
        a.reset(a_index as i32, a_hash as i32);
        let mut s = Synchronizer::new(0x10000);
        s.reset(-1, -1);
        Self { a, s, log: Vec::new(), processes: Vec::new(), stream_bits: 0, capture: None }
    }
}
impl OrderHost for Node {
    fn has_ordering_client(&mut self) -> bool {
        true
    }
    fn version_acknowledged(&mut self) -> bool {
        true
    }
    fn film_session(&mut self) -> bool {
        false
    }
    fn valid(&mut self, _v: Validity, _r: usize, _f: &[Field]) -> aa_orders::Result<bool> {
        Ok(true)
    }
    fn sync(&mut self, e: SyncEntry) -> aa_orders::Result<()> {
        match e {
            SyncEntry::Int { log, label, value } => {
                let s = if log == Log::A { &mut self.a } else { &mut self.s };
                self.log.push(json!({"sync": if log == Log::A {"A"} else {"S"}, "kind": "int", "label": label,
                                     "value": value, "hash_before": s.hash}));
                if log == Log::A && label == "o.GetBitSize()" {
                    self.stream_bits = value;
                }
                s.log_int(true, label, value);
            }
            SyncEntry::Text { log, label, value } => {
                let s = if log == Log::A { &mut self.a } else { &mut self.s };
                self.log.push(json!({"sync": if log == Log::A {"A"} else {"S"}, "kind": "text", "label": label,
                                     "value": value, "hash_before": s.hash}));
                let units: Vec<u16> = value.encode_utf16().collect();
                s.log_text(true, label, &units);
            }
        }
        Ok(())
    }
    fn process(&mut self, d: usize, f: &mut Vec<Field>) -> aa_orders::Result<()> {
        self.processes.push(match d {
            SPEED_DECREE => "speed",
            PLAYER_JOIN_DECREE => "player_join",
            SET_PLAYER_DECREE => "set_player",
            _ => panic!("process {d}"),
        });
        if d == PLAYER_JOIN_DECREE {
            // AdminNode+0x200/+0x204: decree stream position after this decree.
            let mut w = WireWriter::new(Position::PACKET_START)?;
            payload::encode(PLAYER_JOIN_DECREE, f, &mut w, &Ctx)?;
            let payload_bits = w.position().absolute()? - 3;
            let abs = 3 + self.stream_bits as usize + 9 + payload_bits;
            self.capture = Some(json!({
                "admin_200": abs / 8, "admin_204": abs % 8, "log_count": self.log.len(),
                "sync_a_index": self.a.end() - 1, "sync_a_hash": self.a.hash,
            }));
        }
        Ok(())
    }
    fn mirror_enabled(&mut self) -> bool {
        false
    }
}

fn strip_this(processes: &Value) -> Vec<String> {
    processes.as_array().unwrap().iter().map(|p| p["process"].as_str().unwrap().to_string()).collect()
}

/// Native log entries, as the Rust ones are recorded (text values as strings).
fn native_log(log: &Value) -> Vec<Value> {
    log.as_array().unwrap().iter().map(|e| {
        json!({"sync": e["sync"], "kind": e["kind"], "label": e["label"], "value": e["value"], "hash_before": e["hash_before"]})
    }).collect()
}

fn client(decree: &[u8], index: i32, a_index: i64, a_hash: i64, resume: Option<(i32, i32)>) -> Node {
    let mut node = AdminNode::new(index).unwrap();
    node.decrees.put(index, Some(Packet::new(decree.to_vec()).unwrap())).unwrap();
    if let Some((byte, bit)) = resume {
        node.client.resume_byte = byte;
        node.client.resume_bit = bit;
    }
    let mut g = Node::new(a_index, a_hash);
    node.client_execute(index, &mut g, &Ctx, &Ctx, &mut Gate::default()).unwrap();
    g
}

#[test]
fn native_join_resume_replay() {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-join-resume.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    let cases = v["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 4);
    for c in cases {
        let case = &c["case"];
        let name = case["name"].as_str().unwrap();
        let host = &c["host"];
        // Host: 0x74D710 over the request packet (ordering player set by the caller).
        let request: Vec<u8> = serde_json::from_value(host["request_packet"].clone()).unwrap();
        let packet = Packet::new(request).unwrap();
        let mut g = Node::new(case["host_sync_index"].as_i64().unwrap(), case["host_sync_hash"].as_i64().unwrap());
        let mut decree = WireWriter::new(Position::PACKET_START).unwrap();
        let mut mirror = WireWriter::new(Position::PACKET_START).unwrap();
        let registry = aa_orders::native::registry().unwrap();
        let mut gate = Gate::default();
        dispatch_observed(
            Some(&packet),
            None,
            &registry,
            false,
            &mut gate,
            |d, cursor, _| {
                let fields = payload::decode(d, cursor, &Ctx).map_err(|e| e.to_string())?;
                flow::process_request(d, &fields, &mut g, &Ctx, &Ctx, &mut decree, &mut mirror)
                    .map(drop)
                    .map_err(|e| e.to_string())
            },
            |_| {},
        )
        .unwrap();
        let decree_bytes = finalize(&decree).unwrap();
        let native_decree: Vec<u8> = serde_json::from_value(host["decree_packet"].clone()).unwrap();
        assert_eq!(decree_bytes, native_decree, "{name}: decree packet");
        assert_eq!(g.log, native_log(&host["log"]), "{name}: host synchronizer entries");
        assert_eq!(g.processes, strip_this(&host["processes"]), "{name}: host processes");
        let cap = &host["captures"][0];
        let mine = g.capture.clone().unwrap();
        for k in ["admin_200", "admin_204", "log_count", "sync_a_index", "sync_a_hash"] {
            assert_eq!(mine[k], cap[k], "{name}: capture {k}");
        }
        assert_eq!((&cap["admin_200"], &cap["admin_204"]), (&cap["stream_byte"], &cap["stream_bit"]));
        assert_eq!(i64::from(g.a.hash), host["sync_a"]["hash"].as_i64().unwrap(), "{name}: host A hash");
        // Client: 0x74FD10 resumed at the pair, and from the packet start.
        let index = case["packet_index"].as_i64().unwrap() as i32;
        let (ai, ah) = (cap["sync_a_index"].as_i64().unwrap(), cap["sync_a_hash"].as_i64().unwrap());
        let pair = (cap["admin_200"].as_i64().unwrap() as i32, cap["admin_204"].as_i64().unwrap() as i32);
        let resumed = client(&native_decree, index, ai, ah, Some(pair));
        let r = &c["client_resumed"];
        assert_eq!(resumed.log, native_log(&r["log"]), "{name}: resumed client entries");
        assert_eq!(resumed.processes, strip_this(&r["processes"]), "{name}: resumed client processes");
        assert_eq!(i64::from(resumed.a.hash), r["sync_a"]["hash"].as_i64().unwrap());
        let started = client(&native_decree, index, ai, ah, None);
        let s = &c["client_from_start"];
        assert_eq!(started.log, native_log(&s["log"]), "{name}: client from the packet start");
        assert_eq!(started.processes, strip_this(&s["processes"]));
        // Only the resumed client continues the host's A sequence.
        let host_after: Vec<&Value> = g.log[cap["log_count"].as_u64().unwrap() as usize..].iter().filter(|e| e["sync"] == "A").collect();
        let client_a: Vec<&Value> = resumed.log.iter().filter(|e| e["sync"] == "A").collect();
        let pairs = |l: &[&Value]| l.iter().map(|e| (e["value"].clone(), e["hash_before"].clone())).collect::<Vec<_>>();
        assert_eq!(pairs(&host_after), pairs(&client_a[..client_a.len() - 1]), "{name}");
        assert_eq!(client_a.last().unwrap()["hash_before"].as_i64().unwrap(), i64::from(g.a.hash), "{name}");
        let start_a: Vec<&Value> = started.log.iter().filter(|e| e["sync"] == "A").collect();
        assert_ne!(start_a.last().unwrap()["hash_before"].as_i64().unwrap(), i64::from(g.a.hash), "{name}");
        assert!(c["aligned_with_resume"].as_bool().unwrap() && !c["aligned_from_start"].as_bool().unwrap());
    }
}
