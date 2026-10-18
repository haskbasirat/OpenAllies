//! Replays research/phase8-net/native-heartbeat.json (original AA.exe code
//! executed under Unicorn by native_heartbeat.py) against the port in
//! aa_netcode::heartbeat: every case's effects (in order), request and decree
//! stream bytes and the complete final AdminNode / AdminConn state.
use aa_netcode::heartbeat::*;
use aa_orders::commands::WireWriter;
use aa_orders::flow::{Log, OrderHost, SyncEntry, Validity};
use aa_orders::net::admin::AdminNode as OrderNode;
use aa_orders::net::transport::{ReceivedFlags, Sequencing, X87Precision};
use aa_orders::payload::{self, Field, Registry, SchemaContext};
use aa_orders::{History, Packet, Position};
use serde_json::{json, Value};

fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-heartbeat.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json")
}

fn u(v: &Value) -> u32 {
    v.as_u64().map(|x| x as u32).or_else(|| v.as_i64().map(|x| x as u32)).expect("int")
}
fn i(v: &Value) -> i32 {
    v.as_i64().expect("int") as i32
}
fn b8(v: &Value) -> u8 {
    v.as_u64().expect("byte") as u8
}
fn f(v: &Value) -> f32 {
    f32::from_bits(u(v))
}
fn opt(v: &Value) -> Option<u32> {
    if v.is_null() { None } else { Some(u(v)) }
}
fn peer_ref(v: &Value) -> Option<PeerRef> {
    if v.is_null() {
        None
    } else if v.as_str() == Some("node") {
        Some(PeerRef::Node)
    } else {
        Some(PeerRef::Conn(v.as_u64().unwrap() as usize))
    }
}
fn ref_json(r: Option<PeerRef>) -> Value {
    match r {
        None => Value::Null,
        Some(PeerRef::Node) => json!("node"),
        Some(PeerRef::Conn(i)) => json!(i),
    }
}
fn precision(v: &Value) -> X87Precision {
    match v.as_str().unwrap() {
        "Single" => X87Precision::Single,
        "Double" => X87Precision::Double,
        _ => X87Precision::Extended,
    }
}

fn history(v: &Value) -> History {
    let begin = i(&v["begin"]);
    let packets = v["packets"].as_array().unwrap();
    let capacity = v.get("capacity").map(|c| c.as_u64().unwrap() as usize).unwrap_or(4).max(packets.len()).max(1);
    let mut h = History::new(begin, capacity).unwrap();
    h.set_end(begin + packets.len() as i32).unwrap();
    for (k, p) in packets.iter().enumerate() {
        let packet = if p.is_null() {
            None
        } else {
            Some(Packet::new(p.as_array().unwrap().iter().map(b8).collect()).unwrap())
        };
        h.put(begin + k as i32, packet).unwrap();
    }
    h
}
fn history_json(h: &History) -> Value {
    let s = h.snapshot();
    json!({"begin": s.begin, "packets": s.entries})
}

fn base(v: &Value) -> PeerBase {
    PeerBase {
        field_04: u(&v["field_04"]),
        player: opt(&v["player"]),
        requests: history(&v["requests"]),
        backup_requests: history(&v["backup_requests"]),
        ordering_player: opt(&v["ordering_player"]),
        flag_30: b8(&v["flag_30"]),
        flag_31: b8(&v["flag_31"]),
        field_32: b8(&v["field_32"]),
        field_34: i(&v["field_34"]),
        transfer_status: i(&v["transfer_status"]),
        field_3c: i(&v["field_3c"]),
        field_40: i(&v["field_40"]),
        patience: f(&v["patience"]),
        flag_48: b8(&v["flag_48"]),
        flag_49: b8(&v["flag_49"]),
        flag_4a: b8(&v["flag_4a"]),
        flag_4b: b8(&v["flag_4b"]),
        rtt: f(&v["rtt"]),
        received_fraction: f(&v["received_fraction"]),
        field_54: b8(&v["field_54"]),
        vote: b8(&v["vote"]),
        field_56: b8(&v["field_56"]),
        field_58: f(&v["field_58"]),
        field_5c: i(&v["field_5c"]),
        frame_ratio: f(&v["frame_ratio"]),
        // native-heartbeat.json fills these lists with opaque tokens (only
        // their clearing by 0x7514F0 is observable there).
        vote_to_kick: v
            .get("vote_to_kick")
            .map(|l| l.as_array().unwrap().iter().map(|x| PeerRef::Conn(u(x) as usize)).collect())
            .unwrap_or_default(),
        field_6c: b8(&v["field_6c"]),
        join_order: i(&v["join_order"]),
        player_id: Default::default(),
    }
}
fn base_json(b: &PeerBase) -> Value {
    json!({
        "field_04": b.field_04, "player": b.player, "requests": history_json(&b.requests),
        "backup_requests": history_json(&b.backup_requests), "ordering_player": b.ordering_player,
        "flag_30": b.flag_30, "flag_31": b.flag_31, "field_32": b.field_32, "flag_48": b.flag_48,
        "flag_49": b.flag_49, "flag_4a": b.flag_4a, "flag_4b": b.flag_4b, "field_54": b.field_54,
        "vote": b.vote, "field_56": b.field_56, "field_6c": b.field_6c, "field_34": b.field_34,
        "transfer_status": b.transfer_status, "field_3c": b.field_3c, "field_40": b.field_40,
        "patience": b.patience.to_bits(), "rtt": b.rtt.to_bits(), "received_fraction": b.received_fraction.to_bits(),
        "field_58": b.field_58.to_bits(), "field_5c": b.field_5c, "frame_ratio": b.frame_ratio.to_bits(),
        "join_order": b.join_order,
        "vote_to_kick": b.vote_to_kick.iter().map(|r| match r { PeerRef::Conn(i) => json!(i), PeerRef::Node => json!("node") }).collect::<Vec<_>>(),
    })
}

fn conn(v: &Value) -> AdminConn {
    let n = &v["net"];
    let net = Sequencing {
        last_advance: f(&n["last_advance"]),
        rtt: f(&n["rtt"]),
        received: ReceivedFlags {
            begin: i(&n["received_begin"]),
            flags: n["received_flags"].as_array().unwrap().iter().map(|x| u(x) != 0).collect(),
        },
        ..Sequencing::default()
    };
    // The fixture's conns carry the heartbeat fields; channel objects are
    // the constructed ones (the heartbeat only reads the queries below).
    let mut c = AdminConn::new().unwrap();
    c.base = base(&v["base"]);
    c.connected = b8(&v["connected"]) != 0;
    c.disconnecting = b8(&v["disconnecting"]) != 0;
    c.disconnect_reason = u(&v["disconnect_reason"]);
    c.disconnect_started = f(&v["disconnect_time"]).to_bits();
    c.field_94 = u(&v["field_94"]);
    c.field_98 = u(&v["field_98"]);
    c.server_disconnected = b8(&v["disconnect_requested"]) != 0;
    c.backup_index = i(&v["backup_index"]);
    c.reverse_start = i(&v["reverse_acked"]);
    c.status_backlog = b8(&v["request_backlog"]) != 0;
    c.status_falling_behind = b8(&v["falling_behind"]) != 0;
    c.field_aa = b8(&v["field_aa"]);
    c.private_history = history(&v["private_decrees"]);
    c.reverse_history = history(&v["reverse_decrees"]);
    c.broadcast_decree_acked = i(&v["broadcast_decree_acked"]);
    c.backup_decree_acked = i(&v["backup_decree_acked"]);
    c.file_sent = i(&v["file_sent"]);
    c.file_size = i(&v["file_size"]);
    c.last_update = f(&v["last_update"]);
    c.net = net;
    c.file_generation = i(&v["file_generation"]);
    c.accepted = b8(&v["field_420"]) != 0;
    c
}
fn conn_json(c: &AdminConn) -> Value {
    json!({
        "base": base_json(&c.base), "connected": u8::from(c.connected), "disconnecting": u8::from(c.disconnecting),
        "disconnect_reason": c.disconnect_reason, "disconnect_time": c.disconnect_started,
        "field_94": c.field_94, "field_98": c.field_98, "disconnect_requested": u8::from(c.server_disconnected),
        "backup_index": c.backup_index, "reverse_acked": c.reverse_start, "request_backlog": u8::from(c.status_backlog),
        "falling_behind": u8::from(c.status_falling_behind), "field_aa": c.field_aa, "private_decrees": history_json(&c.private_history),
        "reverse_decrees": history_json(&c.reverse_history), "last_update": c.last_update.to_bits(),
        "file_generation": c.file_generation, "field_420": u8::from(c.accepted),
    })
}

fn node(st: &Value) -> AdminNode {
    let n = &st["node"];
    AdminNode {
        base: base(&n["base"]),
        list: st["list"].as_array().unwrap().iter().map(|r| peer_ref(r).unwrap()).collect(),
        conns: st["conns"].as_array().unwrap().iter().map(conn).collect(),
        task_status: u(&n["task_status"]),
        has_admin_clock: u(&n["has_admin_clock"]) != 0,
        admin_clock_own: u(&n["has_admin_clock"]) != 0,
        backup_decrees: history(&n["backup_decrees"]),
        next_backup_decree: i(&n["next_backup_decree"]),
        vote_count: i(&n["vote_count"]),
        max_patience: f(&n["max_patience"]),
        host: peer_ref(&n["host"]),
        server: peer_ref(&n["server"]),
        took_over: i(&n["took_over"]),
        async_counter: i(&st["env"]["heartbeat_counter"]),
        field_8c: 0,
        field_2a0: None,
        field_2a4: None,
        peers_changed: false,
        removed_links: Vec::new(),
    }
}

struct NoContext;
impl SchemaContext for NoContext {
    fn registry_count(&self, _: Registry) -> aa_orders::Result<usize> {
        Err(aa_orders::Error::SchemaContext("not used"))
    }
    fn team_command_flags(&self, _: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext("not used"))
    }
    fn definition_kind(&self, _: Registry, _: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext("not used"))
    }
    fn zero_width_list_budget(&self) -> usize {
        0
    }
}

/// OrderHost recording the 0x750CB0 sync entries and the process call.
struct DecreeHost<'a> {
    events: &'a mut Vec<Value>,
}
impl OrderHost for DecreeHost<'_> {
    fn has_ordering_client(&mut self) -> bool {
        unreachable!()
    }
    fn version_acknowledged(&mut self) -> bool {
        unreachable!()
    }
    fn film_session(&mut self) -> bool {
        unreachable!()
    }
    fn valid(&mut self, _: Validity, _: usize, _: &[Field]) -> aa_orders::Result<bool> {
        unreachable!()
    }
    fn sync(&mut self, entry: SyncEntry) -> aa_orders::Result<()> {
        let log = |l: Log| if l == Log::A { "A" } else { "S" };
        self.events.push(match entry {
            SyncEntry::Int { log: l, label, value } => json!({"kind": "sync_int", "log": log(l), "label": label, "value": value}),
            SyncEntry::Text { log: l, label, value } => json!({"kind": "sync_string", "log": log(l), "label": label, "value": value}),
        });
        Ok(())
    }
    fn process(&mut self, decree: usize, fields: &mut Vec<Field>) -> aa_orders::Result<()> {
        let (Field::Player(player), Field::Bounded { value: reason, .. }) = (&fields[0], &fields[1]) else {
            panic!("309 fields")
        };
        self.events.push(json!({"kind": "process", "descriptor": decree, "player": player, "reason": reason}));
        Ok(())
    }
    fn mirror_enabled(&mut self) -> bool {
        unreachable!()
    }
}

/// Converts effects to the oracle's event records and writes the request
/// stream exactly like the stubbed template (0x766C10 recorded, not written).
struct TestSeams {
    events: Vec<Value>,
    active: bool,
    local_player: u32,
    requests: WireWriter,
    orders: OrderNode,
}
fn history_name(h: HistoryRef) -> String {
    match h {
        HistoryRef::NodeDecrees => "node.decrees".into(),
        HistoryRef::NodeBackupDecrees => "node.backup_decrees".into(),
        HistoryRef::ConnRequests(i) => format!("conn{i}.requests"),
        HistoryRef::ConnBackupRequests(i) => format!("conn{i}.backup_requests"),
        HistoryRef::ConnPrivateDecrees(i) => format!("conn{i}.private_decrees"),
        HistoryRef::ConnReverseDecrees(i) => format!("conn{i}.reverse_decrees"),
    }
}
fn channel_name(c: Channel) -> &'static str {
    match c {
        Channel::BroadcastDecreeSender => "broadcast_decree_sender",
        Channel::BroadcastDecreeReceiver => "broadcast_decree_receiver",
        Channel::ReverseDecreeSender => "reverse_decree_sender",
        Channel::ReverseDecreeReceiver => "reverse_decree_receiver",
        Channel::BroadcastRequestSender => "broadcast_request_sender",
        Channel::BroadcastRequestReceiver => "broadcast_request_receiver",
        Channel::PrivateDecreeSender => "private_decree_sender",
        Channel::PrivateDecreeReceiver => "private_decree_receiver",
        Channel::BackupDecreeSender => "backup_decree_sender",
        Channel::BackupDecreeReceiver => "backup_decree_receiver",
        Channel::BackupRequestSender => "backup_request_sender",
        Channel::BackupRequestReceiver => "backup_request_receiver",
        Channel::FileSender => "file_sender",
        Channel::FileReceiver => "file_receiver",
    }
}
fn name(prefix: &str, player: Option<u32>) -> String {
    match player {
        None => format!("{prefix}null"),
        Some(p) => format!("{prefix}{p}"),
    }
}
impl Seams for TestSeams {
    fn effect(&mut self, _node: &mut AdminNode, effect: Effect) -> aa_netcode::Result<()> {
        match effect {
            Effect::Request { descriptor, fields } => {
                self.events.push(json!({"kind": "request_gate", "active": u8::from(self.active)}));
                if self.active {
                    self.events.push(json!({"kind": "request_player", "player": self.local_player}));
                    self.requests.write_uint(descriptor as u32, 9)?;
                    payload::encode(descriptor, &fields, &mut self.requests, &NoContext)?;
                }
            }
            Effect::Decree { descriptor, fields } => {
                assert_eq!(descriptor, PLAYER_LEAVE_DECREE);
                let mut host = DecreeHost { events: &mut self.events };
                emit_player_leave_decree(&mut self.orders, &fields, &mut host, &NoContext)?;
            }
            Effect::PublicChat { player, text } => self.events.push(json!({
                "kind": "ui_message", "format": "public_chat|ss",
                "args": [name("D", Some(player)), String::from_utf16(&text).unwrap()]
            })),
            Effect::NewHostMessage { player } => self.events.push(json!({
                "kind": "ui_message", "format": "networking_new_host|s", "args": [name("P", player)]
            })),
            Effect::Channel { conn, channel, op } => {
                let mut e = json!({"kind": "channel", "conn": conn, "channel": channel_name(channel)});
                let m = e.as_object_mut().unwrap();
                match op {
                    ChannelOp::SenderBind { history, start } => {
                        m.insert("op".into(), json!("sender_bind"));
                        m.insert("history".into(), json!(history_name(history)));
                        m.insert("start".into(), json!(start));
                    }
                    ChannelOp::SenderReset => {
                        m.insert("op".into(), json!("sender_reset"));
                    }
                    ChannelOp::ReceiverReset => {
                        m.insert("op".into(), json!("receiver_reset"));
                    }
                    ChannelOp::ReceiverBind { history } => {
                        m.insert("op".into(), json!("receiver_bind"));
                        m.insert("history".into(), json!(history_name(history)));
                    }
                    ChannelOp::FileSenderReset => {
                        m.insert("op".into(), json!("file_sender_reset"));
                    }
                    ChannelOp::FileReceiverReset => {
                        m.insert("op".into(), json!("file_receiver_reset"));
                    }
                }
                self.events.push(e);
            }
            Effect::PeerSubobjectReset { peer } => {
                self.events.push(json!({"kind": "peer_subobject_reset", "peer": ref_json(Some(peer))}))
            }
            Effect::ResetOrderVersion => self.events.push(json!({"kind": "reset_order_version", "arg": 1})),
            Effect::AdminClockRestart { period, start } => self.events.push(json!({
                "kind": "clock_restart", "clock": "0x15c", "period": period.to_bits(), "start": start.to_bits()
            })),
            Effect::BackupModeDecision => self.events.push(json!({"kind": "backup_mode_decision"})),
            Effect::PrivateDecrees { conn } => self.events.push(json!({"kind": "private_decrees", "conn": conn})),
            Effect::GameSpyVerifyEnd => self.events.push(json!({"kind": "gamespy_verify_end"})),
            other => panic!("heartbeat paths do not emit {other:?}"),
        }
        Ok(())
    }
}

fn stream_json(w: &WireWriter) -> Value {
    let p = w.position();
    let n = (p.absolute().unwrap()).div_ceil(8);
    json!({"bytes": w.bytes()[..n].to_vec(), "end": {"byte": p.byte, "bit": p.bit}})
}

fn config(v: &Value) -> Config {
    Config {
        async_ticks_per_second: f(&v["async_ticks_per_second"]),
        broadcast_decree_behind: i(&v["broadcast_decree_behind"]),
        backup_decree_behind: i(&v["backup_decree_behind"]),
        admin_ticks_per_second: f(&v["admin_ticks_per_second"]),
    }
}

/// Runs one fixture case; returns (events, after).
fn replay(case: &Value) -> aa_netcode::Result<(Vec<Value>, Value)> {
    let st = &case["before"];
    let e = &st["env"];
    let mut node = node(st);
    let mut decrees = history(&st["node"]["decrees"]);
    let known: Vec<u32> = e["known_players"].as_array().unwrap().iter().map(u).collect();
    let film: Vec<u32> = e["film_players"].as_array().unwrap().iter().map(u).collect();
    let players = move |id: u32| known.contains(&id) && !film.contains(&id);
    let mut env = Env {
        now: f(&e["now"]),
        falling_behind: b8(&e["falling_behind"]),
        session_state: i(&e["session_state"]),
        gamespy_verify: u(&e["gamespy"]) != 0,
        old_clock_time: f(&e["old_clock_time"]),
        config: config(&e["config"]),
        precision: precision(&case["precision"]),
        decrees: &mut decrees,
        players: &players,
    };
    let mut seams = TestSeams {
        events: Vec::new(),
        active: u(&e["request_active"]) != 0,
        local_player: u(&e["local_player"]),
        requests: WireWriter::new(Position::PACKET_START).unwrap(),
        orders: OrderNode::new(0).unwrap(),
    };
    match case["kind"].as_str().unwrap() {
        "refresh" => node.refresh_lag_state(&mut env)?,
        "update" => node.update_conn(case["conn"].as_u64().unwrap() as usize, &mut env, &mut seams)?,
        "heartbeat" => node.heartbeat(&mut env, &mut seams)?,
        "set_host" => node.set_host(peer_ref(&st["target"]).unwrap(), &mut env, &mut seams)?,
        "set_server" => node.set_server(peer_ref(&st["target"]).unwrap(), &mut env, &mut seams)?,
        "send_chat" => {
            let text: Vec<u16> = st["text"].as_array().unwrap().iter().map(|x| u(x) as u16).collect();
            node.send_backup_chat(&text)?
        }
        "send_vote" => node.send_backup_vote(u(&st["vote"]) != 0)?,
        k => panic!("unknown kind {k}"),
    }
    let after = json!({
        "node": {
            "base": base_json(&node.base), "task_status": node.task_status,
            "has_admin_clock": node.has_admin_clock, "admin_clock_own": node.admin_clock_own,
            "decrees": history_json(env.decrees), "backup_decrees": history_json(&node.backup_decrees),
            "next_backup_decree": node.next_backup_decree, "vote_count": node.vote_count,
            "max_patience": node.max_patience.to_bits(), "host": ref_json(node.host),
            "server": ref_json(node.server), "took_over": node.took_over,
        },
        "conns": node.conns.iter().map(conn_json).collect::<Vec<_>>(),
        "heartbeat_counter": node.async_counter,
        "request_stream": stream_json(&seams.requests),
        "decree_stream": stream_json(&seams.orders.decree_stream),
    });
    Ok((seams.events, after))
}

fn diff(path: &str, a: &Value, b: &Value, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (k, v) in x {
                diff(&format!("{path}.{k}"), v, y.get(k).unwrap_or(&Value::Null), out);
            }
            for k in y.keys().filter(|k| !x.contains_key(*k)) {
                out.push(format!("{path}.{k}: missing in port"));
            }
        }
        _ if a != b => out.push(format!("{path}: native {a} port {b}")),
        _ => {}
    }
}

#[test]
fn native_heartbeat_cases() {
    let fx = fixture();
    let cases = fx["cases"].as_array().unwrap();
    let mut counts = std::collections::BTreeMap::new();
    let mut failures = Vec::new();
    for (n, case) in cases.iter().enumerate() {
        let kind = case["kind"].as_str().unwrap();
        if kind == "fraction" {
            continue;
        }
        *counts.entry(kind.to_string()).or_insert(0) += 1;
        let label = format!("#{n} {kind} {} {}", case["label"], case["precision"]);
        match replay(case) {
            Err(e) => failures.push(format!("{label}: port error {e}")),
            Ok((events, after)) => {
                let native_events = case["events"].as_array().unwrap();
                if native_events != &events {
                    failures.push(format!("{label}: events\n native {native_events:?}\n port   {events:?}"));
                }
                let mut d = Vec::new();
                diff("", &case["after"], &after, &mut d);
                if !d.is_empty() {
                    failures.push(format!("{label}: state\n  {}", d.join("\n  ")));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{} failing cases:\n{}", failures.len(), failures.iter().take(12).cloned().collect::<Vec<_>>().join("\n"));
    assert!(counts.values().sum::<usize>() > 1000, "{counts:?}");
}

#[test]
fn native_received_fraction_large_windows() {
    let fx = fixture();
    let mut seen = 0;
    for case in fx["cases"].as_array().unwrap().iter().filter(|c| c["kind"] == "fraction") {
        let begin = i(&case["begin"]);
        let n = case["n"].as_u64().unwrap() as usize;
        let pattern: Vec<u32> = case["pattern"].as_array().unwrap().iter().map(u).collect();
        let flags: Vec<bool> = (0..n)
            .map(|k| {
                let index = (begin.wrapping_add(k as i32) as u32).wrapping_add(0x8000_0000) as usize % pattern.len();
                pattern[index] != 0
            })
            .collect();
        let got = received_fraction(&ReceivedFlags { begin, flags }, precision(&case["precision"]));
        assert_eq!(got.to_bits(), u(&case["result"]), "begin {begin} n {n} {:?}", case["precision"]);
        seen += 1;
    }
    assert_eq!(seen, 15);
}

#[test]
fn executed_windows_recorded() {
    let fx = fixture();
    assert_eq!(fx["original_sha256"], "5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26");
    assert_eq!(fx["v14_sha256"], "3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1");
    let windows = fx["executed_windows"].as_array().unwrap();
    let covers = |a: u64| {
        windows.iter().any(|w| {
            let s = u64::from_str_radix(w["start"].as_str().unwrap().trim_start_matches("0x"), 16).unwrap();
            let e = u64::from_str_radix(w["end"].as_str().unwrap().trim_start_matches("0x"), 16).unwrap();
            s <= a && a < e
        })
    };
    for a in [
        0x75b820u64, 0x75bd70, 0x7593e0, 0x753940, 0x74fa20, 0x74f890, 0x75a2c0, 0x74de10, 0x74ded0, 0x74dd50,
        0x74dd20, 0x74dd80, 0x759530, 0x74e970, 0x750cb0, 0x7501d0, 0x74f1a0, 0x5aa2d0, 0x74b270, 0x74b2a0,
        0x74b2b0, 0x74b2d0, 0x74b2f0, 0x74b300, 0x74b260, 0x74b280, 0x7642d0, 0x74d490, 0x74b2c0, 0x74b2e0,
        0x74d5b0, 0x752c60, 0x74c450, 0x75a0c0, 0x74c590, 0x756e00, 0x74f310, 0x74ea20, 0x74cc20, 0x75a250,
        0x759790, 0x757010, 0x752110, 0x7514f0, 0x752300, 0x752490, 0x753410, 0x74f950, 0x7532d0, 0x74c3d0,
        0x74e5c0, 0x74e530,
    ] {
        assert!(covers(a), "{a:#x} not executed by the oracle");
    }
}

#[test]
fn backup_clients_follow_the_list() {
    let fx = fixture();
    let case = fx["cases"].as_array().unwrap().iter().find(|c| c["kind"] == "refresh" && c["before"]["conns"].as_array().unwrap().len() > 1).unwrap();
    let mut st = case["before"].clone();
    st["node"] = case["after"]["node"].clone();
    st["node"]["has_admin_clock"] = json!(1);
    for (k, c) in case["after"]["conns"].as_array().unwrap().iter().enumerate() {
        st["conns"][k]["base"] = c["base"].clone();
    }
    let n = node(&st);
    let clients = n.backup_clients().unwrap();
    assert_eq!(clients.len(), n.list.len());
    for (c, &r) in clients.iter().zip(&n.list) {
        let b = n.peer(r).unwrap();
        assert_eq!((c.flag_30, c.flag_48, c.flag_49, c.flag_4a, c.flag_4b),
                   (b.flag_30 != 0, b.flag_48 != 0, b.flag_49 != 0, b.flag_4a != 0, b.flag_4b != 0));
    }
}


// ---------------------------------------------------------------------------
// ClientLeave 0x755840 (research/phase8-net/native-leave.json)
// ---------------------------------------------------------------------------

use aa_netcode::lobby::{destroy_link_players, NodeView, PeerLink, Session, SessionPlayer};
use aa_netcode::session::PeerAddress;

fn link_of(r: PeerRef) -> PeerLink {
    match r {
        PeerRef::Node => PeerLink::Node,
        PeerRef::Conn(i) => PeerLink::Conn(PeerAddress { ip: i as u32, port: 0 }),
    }
}

struct LeaveSeams {
    events: Vec<Value>,
    session: Session,
    timeline_28: u8,
}
impl Seams for LeaveSeams {
    fn effect(&mut self, node: &mut AdminNode, effect: Effect) -> aa_netcode::Result<()> {
        match effect {
            Effect::LeaveMessage { key, player } => {
                let args: Vec<String> = player.map(|p| name("D", Some(p))).into_iter().collect();
                self.events.push(json!({"kind": "ui_message", "format": key, "args": args}));
            }
            Effect::AbortCountdown => self.events.push(json!({"kind": "abort_countdown"})),
            // 0x76AA80 is a recording seam in the oracle (its byte store is not made there).
            Effect::TimelineFlag28Cleared => self.events.push(json!({"kind": "timeline_28", "arg": 0})),
            Effect::GameSpyPlayerLeft { player } => self.events.push(json!({"kind": "gamespy_player_left", "player": player})),
            Effect::DestroyPeerPlayers { peer } => {
                let events = &mut self.events;
                destroy_link_players(&mut self.session, &link_of(peer), |_s, id| {
                    events.push(json!({"kind": "delete_player", "player": id, "arg": 1}));
                    events.push(json!({"kind": "update_rated"}));
                    Ok(())
                })?;
            }
            Effect::InGameLeave => self.events.push(json!({"kind": "in_game_leave"})),
            Effect::TasksComplete { no_status_4 } => {
                self.events.push(json!({"kind": "task_history_clear", "this_is_238": true}));
                self.events.push(json!({"kind": "task_history_first", "arg": 0}));
                self.events.push(json!({"kind": "tasks_complete", "flag": u8::from(no_status_4)}));
            }
            // AdminConn::Reset 0x752110 is one recorded seam in the oracle.
            Effect::PeerSubobjectReset { peer } => {
                let PeerRef::Conn(i) = peer else { panic!("node reset") };
                self.events.push(json!({"kind": "reset_conn", "conn": i}));
            }
            Effect::Channel { .. } => {}
            other => panic!("ClientLeave does not emit {other:?}"),
        }
        let _ = node;
        Ok(())
    }
}

fn leave_ref(key: &str) -> PeerRef {
    if key == "node" { PeerRef::Node } else { PeerRef::Conn(key.parse().unwrap()) }
}

#[test]
fn native_client_leave() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-leave.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = v["cases"].as_array().unwrap();
    let (mut swept, mut errors, mut self_leaves) = (0, 0, 0);
    for (n, c) in cases.iter().enumerate() {
        let st = &c["before"];
        let l = &st["leave"];
        let mut node = node(st);
        for (k, list) in l["votes"].as_object().unwrap() {
            node.peer_mut(leave_ref(k)).unwrap().vote_to_kick =
                list.as_array().unwrap().iter().map(|x| peer_ref(x).unwrap()).collect();
        }
        for (k, t) in l["tasks"].as_object().unwrap() {
            node.peer_mut(leave_ref(k)).unwrap().field_34 = i(t);
        }
        node.field_2a0 = peer_ref(&l["field_2a0"]);
        node.field_2a4 = peer_ref(&l["field_2a4"]);
        node.field_8c = 1;
        let view = NodeView { host: PeerLink::Node };
        let mut session = Session::default();
        let mut field_28 = std::collections::HashMap::new();
        for p in l["players"].as_array().unwrap() {
            let id = u(&p["id"]);
            let link = link_of(peer_ref(&p["link"]).unwrap());
            session.players.push(SessionPlayer::new(link, id, false, false, false, &view));
            field_28.insert(id, u(&p["field_28"]));
        }
        let f28 = |id: u32| -> aa_netcode::Result<u32> { Ok(field_28.get(&id).copied().unwrap_or(0)) };
        let env = LeaveEnv {
            now: u(&st["env"]["now"]),
            session_state: u(&l["session_state"]),
            internet: u(&st["env"]["gamespy"]) != 0,
            multiplayer: u(&l["multiplayer"]) != 0,
            all_peers_vote: u(&l["timeline_29"]) != 0,
            player_field_28: &f28,
        };
        let mut seams = LeaveSeams { events: Vec::new(), session, timeline_28: 1 };
        let target = if l["target"] == "node" { PeerRef::Node } else { PeerRef::Conn(u(&l["target"]) as usize) };
        let r = node.client_leave(target, u(&l["reason"]), u(&l["message"]) != 0, u(&l["check_votes"]) != 0, &env, &mut seams);
        if c["status"] != "ok" {
            // Native null reads: 0x74D3A9 (a voter peer without a player) or
            // 0x7667D0 (message for a leaving connection without a player).
            let status = c["status"].as_str().unwrap();
            let pc = ["0x74d3a9", "0x7667d0"].into_iter().find(|pc| status.contains(pc)).expect(status);
            let e = r.expect_err("typed error expected");
            assert!(e.to_string().to_lowercase().contains(pc), "{n}: {e}");
            errors += 1;
            continue;
        }
        r.unwrap();
        assert_eq!(Value::Array(seams.events.clone()), c["events"], "{n}: events");
        let refs: Vec<PeerRef> = std::iter::once(PeerRef::Node).chain((0..node.conns.len()).map(PeerRef::Conn)).collect();
        let mut peers = serde_json::Map::new();
        for r in refs {
            let b = node.peer(r).unwrap();
            let mut e = json!({"player": b.player, "ordering_player": b.ordering_player, "task": b.field_34,
                "votes": b.vote_to_kick.iter().map(|&x| ref_json(Some(x))).collect::<Vec<_>>()});
            if let PeerRef::Conn(k) = r {
                let cn = &node.conns[k];
                let m = e.as_object_mut().unwrap();
                m.insert("connected".into(), json!(u8::from(cn.connected)));
                m.insert("disconnecting".into(), json!(u8::from(cn.disconnecting)));
                m.insert("disconnect_reason".into(), json!(cn.disconnect_reason));
                m.insert("disconnect_time".into(), json!(cn.disconnect_started));
            }
            let key = match r { PeerRef::Node => "node".to_string(), PeerRef::Conn(k) => k.to_string() };
            peers.insert(key, e);
        }
        let mut native_after = c["after"].clone();
        for p in native_after["peers"].as_object_mut().unwrap().values_mut() {
            p.as_object_mut().unwrap().remove("vote_tail");
        }
        // AdminConn::Reset 0x752110 is a seam in this oracle (its stores are
        // proven by native-heartbeat.json): fields it rewrites are compared
        // only for connections that were not reset.
        let reset: Vec<String> = c["events"].as_array().unwrap().iter()
            .filter(|e| e["kind"] == "reset_conn").map(|e| e["conn"].to_string()).collect();
        for k in &reset {
            for f in ["task", "votes", "connected", "disconnecting", "disconnect_reason", "disconnect_time"] {
                native_after["peers"][k].as_object_mut().unwrap().remove(f);
                peers[k].as_object_mut().unwrap().remove(f);
            }
        }
        let mine = json!({
            "list": node.list.iter().map(|&x| ref_json(Some(x))).collect::<Vec<_>>(),
            "peers": peers,
            "field_2a0": ref_json(node.field_2a0), "field_2a4": ref_json(node.field_2a4),
            "peers_changed": u8::from(node.peers_changed), "node_8c": node.field_8c,
            "timeline_28": seams.timeline_28,
            "session_players": seams.session.players.iter().map(|p| p.id).collect::<Vec<_>>(),
        });
        let mut out = Vec::new();
        diff("after", &native_after, &mine, &mut out);
        assert!(out.is_empty(), "{n}: {out:#?}");
        swept += usize::from(c["events"].as_array().unwrap().iter().filter(|e| e["kind"] == "ui_message").count() > 1);
        self_leaves += usize::from(target == PeerRef::Node);
    }
    assert_eq!(cases.len(), 400);
    assert!(errors > 0 && swept > 5 && self_leaves > 20, "{errors} {swept} {self_leaves}");
}
