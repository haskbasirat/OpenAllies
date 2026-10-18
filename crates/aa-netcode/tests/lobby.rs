//! Replays research/phase8-net/native-lobby.json (native_lobby.py): the
//! original host join completion 0x753580 (session snapshot 0x74C270 /
//! 0x76D0B0 and admin state 0x74C130) on a host node, the original private
//! dispatcher 0x7531A0 (0x74DF50 / 0x77AAC0) on a client node, and the
//! PlayerJoin decree 0x75DA80 -> 0x758FD0 on host and client nodes.
use aa_netcode::lobby::*;
use aa_netcode::packets::{AdminConn, NodeHistories};
use aa_netcode::session::PeerAddress;
use aa_orders::net::sync::Synchronizer;
use aa_orders::net::transport::X87Precision;
use aa_orders::payload::Registry;
use aa_orders::{History, Packet, Position};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

fn fixture() -> Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-lobby.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}
fn de<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap_or_else(|e| panic!("{e}: {v}"))
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn i(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}
fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

const REGISTRIES: [(&str, u32); 10] = [
    ("MAP_DEF_44", 0xb3aa24),
    ("MAP_DEF_48", 0xb3aa34),
    ("CONDITIONS", 0xb3aa70),
    ("WORLD_RULES", 0xb3aa88),
    ("NATIONS", 0xb3aa44),
    ("FACTIONS", 0xb3aa4c),
    ("COLORS", 0xb3aa60),
    ("AI_HANDICAPS", 0xb3aa68),
    ("START_PROPERTIES", 0xb3aa2c),
    ("OBJECT_DEFINITIONS", 0xb39f58),
];

struct Reg {
    name: &'static str,
    count: usize,
    null: Vec<u32>,
    reject: Vec<u32>,
    kinds: HashMap<u32, u32>,
    names: HashMap<u32, String>,
}

/// Game data of a fixture case; records the lookups in call order.
struct Data {
    regs: HashMap<u32, Reg>,
    scenarios: HashMap<Vec<u16>, Vec<u16>>,
    random_maps: HashMap<Vec<u16>, Vec<u16>>,
    budget: usize,
    log: Rc<RefCell<Vec<Value>>>,
}
impl Data {
    fn new(v: &Value, log: Rc<RefCell<Vec<Value>>>) -> Self {
        let mut regs = HashMap::new();
        for (name, addr) in REGISTRIES {
            let c = &v["registries"][name];
            let list = |k: &str| -> Vec<u32> { c[k].as_array().map(|a| a.iter().map(u).collect()).unwrap_or_default() };
            let map = |k: &str| -> Vec<(u32, Value)> {
                c[k].as_object().map(|o| o.iter().map(|(k, v)| (k.parse().unwrap(), v.clone())).collect()).unwrap_or_default()
            };
            regs.insert(
                addr,
                Reg {
                    name,
                    count: c["count"].as_u64().unwrap() as usize,
                    null: list("null"),
                    reject: list("reject"),
                    kinds: map("kinds").into_iter().map(|(k, v)| (k, u(&v))).collect(),
                    names: map("names").into_iter().map(|(k, v)| (k, v.as_str().unwrap().to_owned())).collect(),
                },
            );
        }
        let table = |k: &str| -> HashMap<Vec<u16>, Vec<u16>> {
            v[k].as_object().unwrap().iter().map(|(k, v)| (units(k), units(v.as_str().unwrap()))).collect()
        };
        Self {
            regs,
            scenarios: table("scenarios"),
            random_maps: table("random_maps"),
            budget: v["zero_width_list_budget"].as_u64().unwrap() as usize,
            log,
        }
    }
    fn reg(&self, r: Registry) -> &Reg {
        &self.regs[&r.0]
    }
}
impl LobbyData for Data {
    fn registry_count(&self, registry: Registry) -> aa_netcode::Result<usize> {
        Ok(self.reg(registry).count)
    }
    fn definition(&self, registry: Registry, index: u32) -> bool {
        let r = self.reg(registry);
        (index as usize) < r.count
            && !r.null.contains(&index)
            && !(registry == OBJECT_DEFINITIONS && r.reject.contains(&index))
    }
    fn world_rule_kind(&self, index: u32) -> aa_netcode::Result<u32> {
        Ok(*self.reg(WORLD_RULES).kinds.get(&index).unwrap_or(&0))
    }
    fn world_rule_name(&self, index: u32) -> aa_netcode::Result<Vec<u16>> {
        let r = self.reg(WORLD_RULES);
        Ok(units(&r.names.get(&index).cloned().unwrap_or_else(|| format!("{}_{index}", r.name.to_lowercase()))))
    }
    fn scenario(&mut self, key: &[u16]) -> Option<Vec<u16>> {
        self.log.borrow_mut().push(json!({"event": "scenario", "key": key}));
        self.scenarios.get(key).cloned()
    }
    fn random_map_config(&mut self, key: &[u16]) -> Option<Vec<u16>> {
        self.log.borrow_mut().push(json!({"event": "random_map", "key": key}));
        self.random_maps.get(key).cloned()
    }
    fn zero_width_list_budget(&self) -> usize {
        self.budget
    }
}

/// SnapshotHost recording into the same ordered log as the data lookups.
struct Host {
    log: Rc<RefCell<Vec<Value>>>,
    map_status: Vec<bool>,
}
impl SnapshotHost for Host {
    fn change_to_state(&mut self, _session: &mut Session, state: u32, arg: u32) -> aa_netcode::Result<()> {
        self.log.borrow_mut().push(json!({"event": "change_to_state", "state": state, "arg": arg}));
        Ok(())
    }
    fn update_map_status(
        &mut self,
        _session: &mut Session,
        rmc: bool,
        file: &[u16],
        crc: u32,
        source_check: bool,
    ) -> aa_netcode::Result<bool> {
        self.log.borrow_mut().push(
            json!({"event": "map_status", "rmc": rmc, "file": file, "crc": crc, "source_check": u8::from(source_check)}),
        );
        Ok(if self.map_status.is_empty() { true } else { self.map_status.remove(0) })
    }
}

struct Peers {
    node: PeerView,
    conns: Vec<(PeerAddress, PeerIdentity)>,
}
impl PeerDirectory for Peers {
    fn peer(&self, link: &PeerLink) -> aa_netcode::Result<PeerView> {
        Ok(match link {
            PeerLink::Node => self.node.clone(),
            PeerLink::Conn(a) => PeerView {
                address: Some(*a),
                identity: self.conns.iter().find(|(b, _)| b == a).map(|(_, i)| i.clone()).unwrap_or_default(),
            },
            PeerLink::Null => panic!("null peer"),
        })
    }
}

fn node_peers(node: &Value) -> Peers {
    let mut conns = vec![(de::<PeerAddress>(&node["conn_address"]), de::<PeerIdentity>(&node["conn_identity"]))];
    for p in node["peers"].as_array().unwrap() {
        conns.push((de(&p["address"]), de(&p["identity"])));
    }
    Peers { node: PeerView { address: de(&node["own_address"]), identity: de(&node["identity"]) }, conns }
}

fn history_value(h: &History) -> Value {
    let snap = h.snapshot();
    json!({"begin": snap.begin, "end": snap.end, "packets": snap.entries})
}

fn precision(v: &Value) -> X87Precision {
    match v.as_str().unwrap() {
        "Single" => X87Precision::Single,
        "Double" => X87Precision::Double,
        _ => X87Precision::Extended,
    }
}

/// Peer state of the client node, driven by the reader's actions.
#[derive(Clone)]
struct PeerState {
    identity: PeerIdentity,
    joined: Option<u32>,
    ordering: Option<u32>,
}
struct PeerModel {
    node: PeerState,
    conns: BTreeMap<(u32, u16), PeerState>,
    list: Vec<PeerLink>,
    changed: bool,
}
impl PeerModel {
    fn new(node: &Value, session: &Value) -> Self {
        let blank = |identity: PeerIdentity| PeerState { identity, joined: None, ordering: None };
        let mut conns = BTreeMap::new();
        let a: PeerAddress = de(&node["conn_address"]);
        conns.insert((a.ip, a.port), blank(de(&node["conn_identity"])));
        for p in node["peers"].as_array().unwrap() {
            let a: PeerAddress = de(&p["address"]);
            conns.insert((a.ip, a.port), blank(de(&p["identity"])));
        }
        // Connections of players already in the reading session (the oracle
        // interns them when it builds that session).
        for p in session.get("players").and_then(Value::as_array).into_iter().flatten() {
            if let Some(a) = p["link"].get("Conn") {
                let a: PeerAddress = de(a);
                conns.entry((a.ip, a.port)).or_insert_with(|| blank(PeerIdentity::default()));
            }
        }
        Self { node: blank(de(&node["identity"])), conns, list: Vec::new(), changed: false }
    }
    fn get(&mut self, link: &PeerLink) -> &mut PeerState {
        match link {
            PeerLink::Node => &mut self.node,
            PeerLink::Conn(a) => self.conns.get_mut(&(a.ip, a.port)).expect("connection reported before use"),
            PeerLink::Null => panic!("null link"),
        }
    }
    fn apply(&mut self, action: &LobbyAction, log: &mut Vec<Value>) {
        match action {
            LobbyAction::ConnectToPeer { address } => {
                log.push(json!({"event": "connect", "address": address}));
                self.conns.entry((address.ip, address.port)).or_insert(PeerState {
                    identity: PeerIdentity::default(),
                    joined: None,
                    ordering: None,
                });
            }
            LobbyAction::BindPlayer { link, player } => {
                let p = self.get(link);
                p.joined = Some(*player);
                p.ordering = Some(*player);
            }
            LobbyAction::AddPeer { link } => {
                self.list.push(*link);
                self.changed = true;
            }
            LobbyAction::SetPeerIdentity { link, identity } => self.get(link).identity = identity.clone(),
            LobbyAction::ResetSessionInfo => log.push(json!({"event": "reset_session_info"})),
            LobbyAction::ApplySessionInfo { text } => log.push(json!({"event": "apply_session_info", "text": text})),
            other => panic!("unexpected reader action {other:?}"),
        }
    }
    fn state(&self) -> Value {
        let row = |link: Value, p: &PeerState| {
            json!({"link": link, "identity": p.identity, "joined_player": p.joined, "ordering_player": p.ordering})
        };
        let mut out = vec![row(json!("Node"), &self.node)];
        for ((ip, port), p) in &self.conns {
            out.push(row(json!({"Conn": {"ip": ip, "port": port}}), p));
        }
        Value::Array(out)
    }
}

const CALLBACKS: [&str; 4] = ["scenario", "random_map", "change_to_state", "map_status"];
const ACTIONS: [&str; 3] = ["connect", "reset_session_info", "apply_session_info"];
fn only(events: &Value, kinds: &[&str]) -> Vec<Value> {
    events
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| kinds.contains(&e["event"].as_str().unwrap()))
        .cloned()
        .collect()
}

#[test]
fn native_join_messages() {
    let v = fixture();
    let cases = v["join_messages"].as_array().unwrap();
    assert!(v["v14_result_differences"].as_array().unwrap().is_empty());
    let mut checked = (0, 0);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let host = &case["host"];
        let result = &case["result"];
        // ---- host: 0x753580
        let session: Session = de(&host["session"]);
        let peers = node_peers(&host["node"]);
        let data = Data::new(&case["data"], Rc::default());
        let mut conn = AdminConn::new().unwrap();
        conn.backup_index = i(&result["host"]["backup_index_before"]);
        let (mut requests, mut backup_requests) = (History::new(0, 64).unwrap(), History::new(0, 64).unwrap());
        let mut decrees = History::new(0, 64).unwrap();
        let bd = &host["backup_decrees"];
        let mut backup_decrees = History::new(i(&bd["begin"]), 64).unwrap();
        backup_decrees.set_end(i(&bd["end"])).unwrap();
        let mut next = -1;
        let mut node = NodeHistories {
            requests: &mut requests,
            backup_requests: &mut backup_requests,
            decrees: &mut decrees,
            backup_decrees: &mut backup_decrees,
            backup_decree_next: &mut next,
        };
        let admin: AdminStateSource = de(&host["admin"]);
        let mut sync = Synchronizer::new(0x10000);
        sync.reset(i(&host["sync"]["index"]), i(&host["sync"]["hash"]));
        let internet = host["internet"].as_bool().unwrap();
        let out = complete_host_join(&mut conn, &mut node, &session, &peers, &data, &admin, &sync, internet);
        let native_ok = result["host"]["status"] == "ok";
        assert_eq!(out.is_ok(), native_ok, "{name}: host status {:?} vs {}", out.as_ref().err(), result["host"]["status"]);
        assert_eq!(history_value(&conn.private_history), result["host"]["private"], "{name}: private history");
        assert_eq!(conn.connected, result["host"]["connected"].as_bool().unwrap(), "{name}");
        assert_eq!(conn.backup_index, i(&result["host"]["backup_index"]), "{name}");
        if let Ok(messages) = &out {
            let p = &result["host"]["private"]["packets"];
            assert_eq!(json!(messages.session), p[0], "{name}");
            assert_eq!(json!(messages.admin_state), p[1], "{name}");
        }
        checked.0 += 1;
        // ---- client: 0x7531A0
        let c = &result["client"];
        if c.is_null() {
            continue;
        }
        let client = &case["client"];
        let mut session: Session = de(&client["session"]);
        let a = &client["admin"];
        let mut admin = ClientAdmin {
            own_address: de(&client["node"]["own_address"]),
            resume_byte: u(&a["resume_byte"]),
            resume_bit: u(&a["resume_bit"]),
            resume_player: None,
            admin_time: u(&a["admin_time"]),
            decree_next: i(&a["decree_next"]),
            decree_end: i(&a["decree_end"]),
            clock_selected: false,
            clock: de(&a["clock"]),
        };
        let mut sync = Synchronizer::new(0x10000);
        sync.reset(i(&client["sync"]["index"]), i(&client["sync"]["hash"]));
        let mut conn = AdminConn::new().unwrap();
        let begin = i(&client["private_begin"]);
        conn.private_history = History::new(begin, 4).unwrap();
        for (k, p) in result["host"]["private"]["packets"].as_array().unwrap().iter().enumerate() {
            conn.private_history.put(begin + k as i32, Some(Packet::new(de(p)).unwrap())).unwrap();
        }
        let view = NodeView { host: de(&client["node"]["host"]) };
        let env = ClockEnv { now: f32::from_bits(u(&client["now"])), precision: precision(&client["precision"]) };
        let map_status: Vec<bool> =
            case["data"]["map_status_results"].as_array().unwrap().iter().map(|b| b.as_bool().unwrap()).collect();
        let log: Rc<RefCell<Vec<Value>>> = Rc::default();
        let mut data = Data::new(&case["data"], log.clone());
        let mut cb = Host { log: log.clone(), map_status };
        let res = process_private_messages(&mut conn, &mut admin, &mut session, &mut sync, &mut data, &mut cb, &view, env);
        let native_ok = c["status"] == "ok";
        assert_eq!(res.is_ok(), native_ok, "{name}: client status {:?} vs {}", res.as_ref().err(), c["status"]);
        let mut log_actions = Vec::new();
        let mut model = PeerModel::new(&client["node"], &client["session"]);
        let mut ends = Vec::new();
        if let Ok(messages) = &res {
            for m in messages {
                for a in &m.actions {
                    model.apply(a, &mut log_actions);
                }
                ends.push(json!(m.end));
            }
            assert_eq!(Value::Array(ends), c["ends"], "{name}: end positions");
        }
        assert_eq!(json!(session), c["session"], "{name}: client session");
        assert_eq!(json!(admin), c["admin"], "{name}: client admin");
        let s = json!({"enabled": sync.enabled, "file_dump": sync.file_dump, "hash": sync.hash, "begin": sync.begin(), "end": sync.end()});
        assert_eq!(s, c["sync"], "{name}: synchronizer A");
        let h = conn.private_history.snapshot();
        assert_eq!(json!({"begin": h.begin, "end": h.end}), c["private"], "{name}: private window");
        assert_eq!(Value::Array(log.borrow().clone()), Value::Array(only(&c["events"], &CALLBACKS)), "{name}: callbacks");
        if res.is_ok() {
            assert_eq!(Value::Array(log_actions), Value::Array(only(&c["events"], &ACTIONS)), "{name}: actions");
            assert_eq!(model.state(), c["peers"], "{name}: peers");
            assert_eq!(json!(model.list), c["peer_list"], "{name}: peer list");
            assert_eq!(model.changed, c["peers_changed"].as_bool().unwrap(), "{name}");
        }
        checked.1 += 1;
    }
    assert!(checked.0 >= 30 && checked.1 >= 25, "{checked:?}");
}


// ---------------------------------------------------------------------------
// PlayerJoin decree 0x758FD0
// ---------------------------------------------------------------------------

struct ConnModel {
    admin: AdminConn,
}
struct JoinHarness {
    conns: BTreeMap<(u32, u16), ConnModel>,
    kingdom_open: HashMap<Vec<u16>, bool>,
    events: Vec<Value>,
    duplicate_calls: usize,
}
impl JoinHarness {
    fn new(case: &Value) -> Self {
        let node = &case["node"];
        let mut conns = BTreeMap::new();
        let mut add = |a: PeerAddress, identity: PeerIdentity| {
            let mut admin = AdminConn::new().unwrap();
            admin.accepted = true; // fixture: +0x420 = 1, +0x8C = 0xD, +0x90 = 0 before the decree
            admin.base.set_identity(&identity);
            conns.insert((a.ip, a.port), ConnModel { admin });
        };
        add(de(&node["conn_address"]), de(&node["conn_identity"]));
        for p in node["peers"].as_array().unwrap() {
            add(de(&p["address"]), de(&p["identity"]));
        }
        for j in node["joined"].as_array().unwrap() {
            let link: PeerLink = de(&j[0]);
            if let PeerLink::Conn(a) = link {
                let c = conns.get_mut(&(a.ip, a.port)).unwrap();
                c.admin.base.player = Some(u(&j[1]));
                c.admin.base.ordering_player = Some(u(&j[1]));
            }
        }
        let kingdom_open = case["data"]["kingdom_open"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (units(k), v.as_bool().unwrap()))
            .collect();
        Self { conns, kingdom_open, events: Vec::new(), duplicate_calls: 0 }
    }
}
impl JoinHost for JoinHarness {
    fn conn(&mut self, link: &PeerLink) -> aa_netcode::Result<&mut AdminConn> {
        let PeerLink::Conn(a) = link else { panic!("decree target {link:?}") };
        let c = self.conns.entry((a.ip, a.port)).or_insert_with(|| {
            // Connection created by the 0x764970 seam during the decree: zeroed harness memory.
            let mut admin = AdminConn::new().unwrap();
            admin.disconnect_reason = 0;
            ConnModel { admin }
        });
        Ok(&mut c.admin)
    }
    fn auto_assign_kingdom(&mut self, _session: &mut Session, player: u32) -> aa_netcode::Result<()> {
        self.events.push(json!({"event": "auto_assign_kingdom", "player": player}));
        Ok(())
    }
    fn duplicate_name(&mut self, name: &[u16], n: u32) -> aa_netcode::Result<Vec<u16>> {
        // Fixture format "%s (%d)" (0x405D60 runs natively on it).
        self.duplicate_calls += 1;
        let mut out = name.iter().copied().take_while(|&c| c != 0).collect::<Vec<u16>>();
        out.extend(units(&format!(" ({n})")));
        Ok(out)
    }
    fn kingdom_open(&mut self, kingdom: &[u16]) -> aa_netcode::Result<bool> {
        self.events.push(json!({"event": "kingdom_info", "kingdom": kingdom}));
        self.kingdom_open
            .get(kingdom)
            .copied()
            .ok_or(aa_netcode::Error::Protocol("0x76C8C4 dereferences a null kingdom info".into()))
    }
}

fn outcome_logs(outcome: &JoinOutcome) -> Vec<&'static str> {
    match outcome {
        JoinOutcome::FullIgnored => vec!["full"],
        JoinOutcome::FullDisconnected { .. } => vec!["full", "full_host", "set_disconnecting"],
        JoinOutcome::FullAlreadyJoined { .. } => vec!["full", "full_host", "full_already_joined"],
        JoinOutcome::RejectedSelf => vec!["rejected_self"],
        JoinOutcome::RejectedDuplicate { .. } => vec!["rejected_duplicate"],
        JoinOutcome::Joined { .. } => vec![],
    }
}

#[test]
fn native_player_join() {
    let v = fixture();
    let names: HashMap<u32, String> = v["admin_log_lines"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, n)| (u32::from_str_radix(k.trim_start_matches("0x"), 16).unwrap(), n.as_str().unwrap().to_owned()))
        .collect();
    let cases = v["player_join"].as_array().unwrap();
    let mut joined = 0;
    let mut harness_gaps = Vec::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let r = &case["result"];
        // The oracle harness stopped early (its own call assertion, not a
        // native assertion): no native evidence for these cases.
        if r["status"].as_str().unwrap().starts_with("error: AssertionError 0x") {
            harness_gaps.push(name.to_owned());
            continue;
        }
        let data = Data::new(&case["data"], Rc::default());
        let mut session: Session = de(&case["session"]);
        let nv = &case["node"];
        let internet = case["internet"].as_bool().unwrap();
        let mut node = JoinNode {
            view: NodeView { host: de(&nv["host"]) },
            own_address: de(&nv["own_address"]),
            now: u(&nv["now"]),
            internet,
            peers_changed: false,
            multiplayer_assert_ignored: false,
        };
        let info: JoinInfo = de(&case["info"]);
        let mut h = JoinHarness::new(case);
        let res = process_player_join(&info, &mut session, &mut node, &data, &mut h);
        // AdminLayer.cpp lines of the host join completion 0x753580 (0x752300,
        // 0x752490 x2, 0x74C270), which the oracle runs natively.
        const COMPLETION_LOGS: [u32; 4] = [0x15ac, 0x15d5, 0x15dd, 0x1211];
        let all_logs: Vec<u32> = r["logs"].as_array().unwrap().iter().map(u).collect();
        let completion_logs: Vec<u32> = all_logs.iter().copied().filter(|l| COMPLETION_LOGS.contains(l)).collect();
        let native_logs: Vec<String> =
            all_logs.iter().filter(|l| !COMPLETION_LOGS.contains(l)).map(|l| names[l].clone()).collect();
        let status = r["status"].as_str().unwrap();
        let res = match res {
            Ok(res) => res,
            Err(e) => {
                assert!(status.starts_with("error"), "{name}: {e:?} vs {status}");
                continue;
            }
        };
        assert_eq!(native_logs, outcome_logs(&res.outcome), "{name}: outcome {:?}", res.outcome);
        let host_joined = matches!(res.outcome, JoinOutcome::Joined { .. }) && node.view.host == PeerLink::Node;
        assert_eq!(completion_logs, if host_joined { COMPLETION_LOGS.to_vec() } else { vec![] }, "{name}: completion");
        // Node-level actions.
        let mut connects = Vec::new();
        let mut list = Vec::new();
        for a in &res.actions {
            match a {
                LobbyAction::ConnectToPeer { address } => connects.push(json!({"event": "connect", "address": address})),
                LobbyAction::AddPeer { link } => list.push(*link),
                LobbyAction::UiMessage { kind, key, args } => {
                    assert_eq!(key, PLAYER_JOINED_KEY);
                    h.events.push(json!({"event": "ui_message", "kind": kind, "text": args[0]}));
                }
                LobbyAction::Request { descriptor, fields } => {
                    assert_eq!(*descriptor, POST_PLAYER_JOIN_REQUEST);
                    let aa_orders::payload::Field::Player(id) = fields[0] else { panic!() };
                    let mut w = aa_orders::commands::WireWriter::new(Position::PACKET_START).unwrap();
                    w.write_uint(307, aa_orders::registry_bits(357).unwrap()).unwrap();
                    w.write_uint(id, 32).unwrap();
                    let end = w.position();
                    let bits = end.absolute().unwrap();
                    assert_eq!(json!(w.bytes()[..bits.div_ceil(8)]), r["request"]["bytes"], "{name}: request");
                    assert_eq!(json!(end), r["request"]["end"], "{name}");
                    h.events.push(json!({"event": "request_player", "player": 0}));
                }
                other => panic!("{name}: unexpected {other:?}"),
            }
        }
        assert_eq!(Value::Array(connects), Value::Array(only(&r["events"], &["connect"])), "{name}: connects");
        assert_eq!(json!(list), r["peer_list"], "{name}: peer list");
        assert_eq!(node.peers_changed, r["peers_changed"].as_bool().unwrap(), "{name}");
        let key = |e: &Value| e["event"].as_str().unwrap().to_owned();
        let mut expected_events = h.events.clone();
        expected_events.sort_by_key(key);
        let mut native_events = only(&r["events"], &["ui_message", "auto_assign_kingdom", "kingdom_info", "request_player"]);
        native_events.sort_by_key(key);
        // The oracle's CString::Format 0x405D60 is a no-op seam, so the native
        // UI text is empty; only the message kind is native evidence here (the
        // Rust action carries the localization key and the player name).
        for e in expected_events.iter_mut().chain(native_events.iter_mut()) {
            if e["event"] == "ui_message" {
                e["text"] = Value::Null;
            }
        }
        for e in only(&r["events"], &["ui_message"]) {
            assert_eq!(e["text"], json!([]), "{name}: Format seam output");
        }
        // Same events per kind, in native order within a kind (stable sort).
        assert_eq!(expected_events, native_events, "{name}: events");
        let dups = r["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["event"] == "localize" && e["key"] == DUPLICATE_NAME_KEY)
            .count();
        assert_eq!(h.duplicate_calls, dups, "{name}: duplicate-name formats");
        // Host completion 0x753580 on the joined connection (inside the native decree).
        if let JoinOutcome::Joined { link: PeerLink::Conn(a), host_completion: true, .. } = res.outcome {
            joined += 1;
            let peers = Peers {
                node: PeerView { address: node.own_address, identity: de(&nv["identity"]) },
                conns: h
                    .conns
                    .iter()
                    .map(|((ip, port), c)| (PeerAddress { ip: *ip, port: *port }, c.admin.base.identity()))
                    .collect(),
            };
            let c = h.conns.get_mut(&(a.ip, a.port)).unwrap();
            c.admin.backup_index = i(&r["backup_index_before"]);
            let (mut q, mut bq, mut d) = (History::new(0, 64).unwrap(), History::new(0, 64).unwrap(), History::new(0, 64).unwrap());
            let mut bd = History::new(0, 64).unwrap();
            let mut next = -1;
            let mut hist = NodeHistories {
                requests: &mut q,
                backup_requests: &mut bq,
                decrees: &mut d,
                backup_decrees: &mut bd,
                backup_decree_next: &mut next,
            };
            let ad = &case["admin"];
            let source = AdminStateSource {
                conn_address: Some(a),
                field_200: u(&ad["field_200"]),
                field_204: u(&ad["field_204"]),
                player: None,
                admin_time: u(&ad["admin_time"]),
                decree_end: i(&ad["decree_end"]),
            };
            let mut sync = Synchronizer::new(0x10000);
            sync.reset(i(&case["sync"]["index"]), i(&case["sync"]["hash"]));
            let out = complete_host_join(&mut c.admin, &mut hist, &session, &peers, &data, &source, &sync, internet);
            assert_eq!(out.is_ok(), status == "ok", "{name}: completion {:?} vs {status}", out.err());
            assert_eq!(history_value(&c.admin.private_history), r["private"], "{name}: private history");
            assert_eq!(c.admin.backup_index, i(&r["backup_index"]), "{name}");
        } else {
            assert_eq!(status, "ok", "{name}");
        }
        assert_eq!(json!(session), r["session"], "{name}: session");
        for (key, c) in &h.conns {
            let native = &r["conns"][format!("{}:{}", key.0, key.1)];
            let mine = json!({"accepted": c.admin.accepted, "joined_player": c.admin.base.player,
                "ordering_player": c.admin.base.ordering_player, "disconnecting": c.admin.disconnecting,
                "disconnect_reason": c.admin.disconnect_reason, "disconnect_started": c.admin.disconnect_started,
                "connected": c.admin.connected, "identity": c.admin.base.identity()});
            assert_eq!(&mine, native, "{name}: conn {key:?}");
        }
        assert_eq!(h.conns.len(), r["conns"].as_object().unwrap().len(), "{name}: connection count");
    }
    assert!(joined >= 5, "{joined}");
    // Known oracle gaps (duplicate-name rule 0x766560 with harness-built
    // players): recorded in research/phase8-net/lobby-notes.txt, not proven.
    assert_eq!(harness_gaps, ["host_join_duplicate_names", "host_join_empty_name", "host_join_nul_name"]);
}

#[test]
fn native_address_interning() {
    let v = fixture();
    let rows = v["addresses"].as_array().unwrap();
    let mut seen: Vec<(PeerAddress, u64)> = Vec::new();
    for row in rows {
        let mut w = aa_orders::commands::WireWriter::new(Position::PACKET_START).unwrap();
        w.write_uint(u(&row["ip"]), 32).unwrap();
        w.write_uint(u(&row["port"]), 32).unwrap();
        let mut bytes = w.bytes().to_vec();
        bytes.truncate(9);
        bytes[0] |= 5; // 67 bits: 5 unused bits in the last byte
        let mut r = aa_orders::Cursor::for_packet(&bytes, None).unwrap();
        match read_address(&mut r).unwrap() {
            None => assert!(u(&row["ip"]) == 0 && u(&row["port"]) == 0, "{row}"),
            Some(a) => {
                assert_eq!(json!(a), row["stored"], "{row}");
                let obj = row["object"].as_u64().unwrap();
                match seen.iter().find(|(b, _)| *b == a) {
                    Some((_, o)) => assert_eq!(*o, obj, "same key, same object: {row}"),
                    None => {
                        assert!(seen.iter().all(|(_, o)| *o != obj), "different key, different object: {row}");
                        seen.push((a, obj));
                    }
                }
            }
        }
    }
}
