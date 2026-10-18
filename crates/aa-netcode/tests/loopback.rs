//! LOOPBACK ONLY (one machine, 127.0.0.1): a host and a client in separate
//! threads, each with its own `std::net::UdpSocket`, talking only through
//! real UDP datagrams built by the ported layers:
//!
//! * transport header / acks / RTT: aa_orders::net::transport (oracle-proven);
//! * admin packet types 2 (Connect 0x750330 / 0x75BF90), 4 (0x74E0F0 /
//!   0x74E170) and 5 (0x74E280 / 0x75A350) with the message-history channels,
//!   bindings and ack dispatcher (native-connect/-packets/-channels.json);
//! * send scheduling 0x74BDE0 with the backlog test (native-send-gate.json);
//! * request sealing 0x752DA0, host execution 0x752E60/0x751B60, decree
//!   sealing 0x752EA0, client replay 0x74FD10 with the resume pair, and
//!   synchronizer A (native order flow / sync fixtures);
//! * the lobby join path (native-lobby.json): PlayerJoin decree process
//!   0x758FD0 on the host, host join completion 0x753580 with the session
//!   snapshot 0x76D0B0 and the admin-state message 0x74C130 (decree stream
//!   position AdminNode+0x200/+0x204 and synchronizer A captured at the
//!   PlayerJoin process, as native-join-resume.json shows the original
//!   does), and on the client the private-history dispatcher 0x7531A0 with
//!   0x77AAC0 / 0x74DF50 (synchronizer reset, decree resume);
//! * the host's TGC_SetSyncValueOrderDecree process 0x751D00 decision: the
//!   client's (index, hash) from TGC_SetSyncValueOrderRequest (302,
//!   0x74E8B0) is validated against the host synchronizer with 0x77D050.
//!
//! The host runs admin ticks (and logs to its synchronizer) from the start,
//! long before the client connects; the client resumes from the admin-state
//! message. Test-harness choices (not native): the session and game data are
//! the lobby fixture's "host_join" case; order processes other than
//! PlayerJoin / SetPlayer / SetSyncValue only record; the client sends its
//! synchronizer value every 4th admin tick (natively every 17th); the
//! kingdom auto-assignment 0x778D10 and ChangeToState 0x77A750 are recorded
//! (crate traits). This is a two-thread loopback test, not a two-machine test.
use aa_netcode::lobby::{
    complete_host_join, process_player_join, process_private_messages, AdminStateSource, ClientAdmin, ClientClock,
    ClockEnv, JoinHost, JoinInfo, JoinNode, JoinOutcome, LobbyAction, LobbyData, NodeView, PeerDirectory,
    PeerIdentity, PeerLink, PeerView, Session, SnapshotHost,
};
use aa_netcode::packets::{
    read_client_to_server, read_server_to_client, write_client_to_server, write_server_to_client, AdminConn,
    ConnEvent, NetworkAvars, NoFileTransfer, NodeHistories, Roles,
};
use aa_netcode::send::{wanted_packet_type, GateConn, GateNode};
use aa_netcode::session::{
    read_and_validate_connect, write_connect, ConnectInfo, ConnectVerdict, HostConnectState, LobbyContext, PeerAddress,
    PlayerId,
};
use aa_netcode::udp::UdpNode;
use aa_netcode::NativeImage;
use aa_orders::commands::WireWriter;
use aa_orders::flow::{Log, OrderHost, SyncEntry, Validity};
use aa_orders::net::admin::{AdminNode, Peer};
use aa_orders::net::sync::Synchronizer;
use aa_orders::net::transport::X87Precision;
use aa_orders::payload::{Field, NameKind, Registry, Resolver, SchemaContext, ThingCheck};
use aa_orders::{Gate, History, Position};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

const SET_PLAYER_DECREE: usize = 298;
const SYNC_VALUE_REQUEST: usize = 302;
const SYNC_VALUE_DECREE: usize = 314;
const PLAYER_JOIN_REQUEST: usize = 306;
const PLAYER_JOIN_DECREE: usize = 317;
const POST_PLAYER_JOIN_DECREE: usize = 318;
const MOVE_REQUEST: usize = 47;
const SPEED_REQUEST: usize = 54;
const HOST_PLAYER: u32 = 1;
const TICK: f32 = 1.0 / 16.0; // AVar NetworkAdminTicksPerSecond 16.0 (0x403721)

// ---------------------------------------------------------------------------
// Game data (order payloads and lobby codecs)
// ---------------------------------------------------------------------------

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
    fn player(&self, id: u32) -> bool {
        id == 1 || id == 2
    }
    fn definition(&self, _r: Registry, i: u32) -> bool {
        i < 7
    }
    fn named(&self, _k: NameKind, key: &[u16]) -> Option<Vec<u16>> {
        Some(key.to_vec())
    }
}

fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// The lobby fixture's "host_join" case: host session (one host player)
/// and the definition registries its snapshot codecs consult.
fn lobby_case() -> Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-lobby.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    v["player_join"].as_array().unwrap().iter().find(|c| c["name"] == "host_join").unwrap().clone()
}

#[derive(Clone)]
struct Data {
    counts: HashMap<u32, usize>,
    nulls: HashMap<u32, Vec<u32>>,
    budget: usize,
}
impl Data {
    fn new(case: &Value) -> Self {
        let regs = &case["data"]["registries"];
        let addr = |n: &str| -> u32 {
            match n {
                "MAP_DEF_44" => 0xb3aa24,
                "MAP_DEF_48" => 0xb3aa34,
                "CONDITIONS" => 0xb3aa70,
                "WORLD_RULES" => 0xb3aa88,
                "NATIONS" => 0xb3aa44,
                "FACTIONS" => 0xb3aa4c,
                "COLORS" => 0xb3aa60,
                "AI_HANDICAPS" => 0xb3aa68,
                "START_PROPERTIES" => 0xb3aa2c,
                "OBJECT_DEFINITIONS" => 0xb39f58,
                other => panic!("registry {other}"),
            }
        };
        let mut counts = HashMap::new();
        let mut nulls = HashMap::new();
        for (name, r) in regs.as_object().unwrap() {
            counts.insert(addr(name), r["count"].as_u64().unwrap() as usize);
            let n: Vec<u32> = r["null"].as_array().map(|a| a.iter().map(|x| x.as_u64().unwrap() as u32).collect()).unwrap_or_default();
            nulls.insert(addr(name), n);
        }
        Self { counts, nulls, budget: case["data"]["zero_width_list_budget"].as_u64().unwrap() as usize }
    }
}
impl LobbyData for Data {
    fn registry_count(&self, registry: Registry) -> aa_netcode::Result<usize> {
        Ok(self.counts[&registry.0])
    }
    fn definition(&self, registry: Registry, index: u32) -> bool {
        (index as usize) < self.counts[&registry.0] && !self.nulls[&registry.0].contains(&index)
    }
    fn world_rule_kind(&self, _index: u32) -> aa_netcode::Result<u32> {
        Ok(0)
    }
    fn world_rule_name(&self, index: u32) -> aa_netcode::Result<Vec<u16>> {
        Ok(units(&format!("world_rules_{index}")))
    }
    fn scenario(&mut self, key: &[u16]) -> Option<Vec<u16>> {
        Some(key.to_vec())
    }
    fn random_map_config(&mut self, key: &[u16]) -> Option<Vec<u16>> {
        Some(key.to_vec())
    }
    fn zero_width_list_budget(&self) -> usize {
        self.budget
    }
}

/// ChangeToState 0x77A750 and the map-status sends are recorded.
#[derive(Default)]
struct SnapHost {
    states: Vec<u32>,
}
impl SnapshotHost for SnapHost {
    fn change_to_state(&mut self, _s: &mut Session, state: u32, _arg: u32) -> aa_netcode::Result<()> {
        self.states.push(state);
        Ok(())
    }
    fn update_map_status(&mut self, _s: &mut Session, _rmc: bool, _f: &[u16], _crc: u32, _src: bool) -> aa_netcode::Result<bool> {
        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// Connections of a node
// ---------------------------------------------------------------------------

struct Conn {
    addr: PeerAddress,
    admin: AdminConn,
    file: NoFileTransfer,
}
fn peer_address(a: SocketAddr) -> PeerAddress {
    match a {
        SocketAddr::V4(v4) => PeerAddress { ip: u32::from(*v4.ip()), port: v4.port() },
        SocketAddr::V6(_) => panic!("IPv4 only"),
    }
}

/// JoinHost over the node's connections (0x764970 has created every
/// connection that sent a datagram).
struct Joins<'a> {
    conns: &'a mut Vec<Conn>,
    kingdoms_assigned: &'a mut Vec<u32>,
}
impl JoinHost for Joins<'_> {
    fn conn(&mut self, link: &PeerLink) -> aa_netcode::Result<&mut AdminConn> {
        let PeerLink::Conn(a) = link else { panic!("join target {link:?}") };
        Ok(&mut self.conns.iter_mut().find(|c| c.addr == *a).expect("connection of the joining address").admin)
    }
    fn auto_assign_kingdom(&mut self, _s: &mut Session, player: u32) -> aa_netcode::Result<()> {
        self.kingdoms_assigned.push(player);
        Ok(())
    }
    fn duplicate_name(&mut self, name: &[u16], n: u32) -> aa_netcode::Result<Vec<u16>> {
        let mut out = name.to_vec();
        out.extend(units(&format!(" ({n})")));
        Ok(out)
    }
    fn kingdom_open(&mut self, _k: &[u16]) -> aa_netcode::Result<bool> {
        Ok(true)
    }
}

/// AdminPeer views of the writing node (vfunc 0 address and +0x3C/+0x70/+0x74).
struct Dir {
    node: PeerView,
    conns: Vec<(PeerAddress, PeerIdentity)>,
}
impl PeerDirectory for Dir {
    fn peer(&self, link: &PeerLink) -> aa_netcode::Result<PeerView> {
        Ok(match link {
            PeerLink::Node => self.node.clone(),
            PeerLink::Conn(a) => {
                let (_, identity) = self.conns.iter().find(|(b, _)| b == a).expect("peer connection");
                PeerView { address: Some(*a), identity: identity.clone() }
            }
            PeerLink::Null => panic!("null peer"),
        })
    }
}

// ---------------------------------------------------------------------------
// Order processes of a node
// ---------------------------------------------------------------------------

struct PendingJoin {
    link: PeerLink,
    resume: Position,
    sync_a: Synchronizer,
    player: Option<u32>,
}

/// Game side of one node: synchronizer A, executed decrees, and the
/// network-relevant decree processes.
struct Game {
    host: bool,
    a: Synchronizer,
    executed: Vec<(usize, Vec<Field>)>,
    ordering_player: Option<u32>,
    /// The previous tick's last ordering player (AdminNode+0x208 during
    /// host execution).
    previous_ordering_player: Option<u32>,
    /// Decree-stream bits before the decree being processed ("o.GetBitSize()").
    stream_bits: u32,
    session: Session,
    data: Data,
    conns: Vec<Conn>,
    /// AdminNode+0x29C our address as the host sees it (admin-state message).
    own_address: Option<PeerAddress>,
    now: u32,
    kingdoms_assigned: Vec<u32>,
    pending_joins: Vec<PendingJoin>,
    requests: Vec<(usize, Vec<Field>)>,
    validations: Vec<(u32, i32, i32, bool)>,
}
impl OrderHost for Game {
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
            SyncEntry::Int { log: Log::A, label, value } => {
                if label == "o.GetBitSize()" {
                    self.stream_bits = value;
                }
                self.a.log_int(true, label, value)
            }
            SyncEntry::Text { log: Log::A, label, value } => {
                let units: Vec<u16> = value.encode_utf16().collect();
                self.a.log_text(true, label, &units)
            }
            _ => {}
        }
        Ok(())
    }
    fn process(&mut self, d: usize, f: &mut Vec<Field>) -> aa_orders::Result<()> {
        self.executed.push((d, f.clone()));
        let fail = |e: aa_netcode::Error| aa_orders::Error::SchemaContext(Box::leak(e.to_string().into_boxed_str()));
        match (d, f.as_slice()) {
            (SET_PLAYER_DECREE, [Field::Uint { value, .. }]) => self.ordering_player = Some(*value),
            (PLAYER_JOIN_DECREE, fields) => {
                let info = JoinInfo::from_fields(fields).map_err(fail)?;
                // Decree stream position after this decree (AdminNode+0x200/+0x204).
                let mut w = WireWriter::new(Position::PACKET_START)?;
                aa_orders::payload::encode(PLAYER_JOIN_DECREE, fields, &mut w, &LobbyContext)?;
                let payload = w.position().absolute()? - 3;
                let abs = 3 + self.stream_bits as usize + 9 + payload;
                let resume = Position { byte: abs / 8, bit: (abs % 8) as u8 };
                let snapshot = self.a.clone();
                let mut node = JoinNode {
                    view: NodeView { host: if self.host { PeerLink::Node } else { PeerLink::Null } },
                    own_address: self.own_address,
                    now: self.now,
                    internet: false,
                    peers_changed: false,
                    multiplayer_assert_ignored: false,
                };
                let mut joins = Joins { conns: &mut self.conns, kingdoms_assigned: &mut self.kingdoms_assigned };
                let r = process_player_join(&info, &mut self.session, &mut node, &self.data, &mut joins).map_err(fail)?;
                for a in r.actions {
                    if let LobbyAction::Request { descriptor, fields } = a {
                        self.requests.push((descriptor, fields));
                    }
                }
                if let JoinOutcome::Joined { link, host_completion: true, .. } = r.outcome {
                    self.pending_joins.push(PendingJoin {
                        link,
                        resume,
                        sync_a: snapshot,
                        player: self.previous_ordering_player,
                    });
                }
            }
            (SYNC_VALUE_DECREE, [Field::Uint { value: index, .. }, Field::Uint { value: hash, .. }]) if self.host => {
                // 0x751D00: only the host validates, for an ordering player
                // with a client (the joined peer), with 0x77D050.
                let player = self.ordering_player.expect("ordering player");
                let ok = self.a.validate(*index as i32, *hash as i32);
                self.validations.push((player, *index as i32, *hash as i32, ok));
            }
            _ => {}
        }
        Ok(())
    }
    fn mirror_enabled(&mut self) -> bool {
        false
    }
}

fn roles() -> Roles {
    Roles {
        we_are_host: false,
        we_are_server: false,
        conn_is_host: false,
        conn_is_server: false,
        connecting: false,
        joined: false,
        backup_mode: false,
        rank: 0,
        session_member: false,
        session_state_set: false,
        admin_clock: false,
    }
}

struct Spare {
    backup_requests: History,
    backup_decrees: History,
    backup_next: i32,
}
impl Spare {
    fn new() -> Self {
        Self { backup_requests: History::new(0, 16).unwrap(), backup_decrees: History::new(0, 16).unwrap(), backup_next: 0 }
    }
}
fn view<'a>(node: &'a mut AdminNode, spare: &'a mut Spare) -> NodeHistories<'a> {
    NodeHistories {
        requests: &mut node.requests,
        backup_requests: &mut spare.backup_requests,
        decrees: &mut node.decrees,
        backup_decrees: &mut spare.backup_decrees,
        backup_decree_next: &mut spare.backup_next,
    }
}

fn now_of(start: Instant) -> f32 {
    start.elapsed().as_secs_f32()
}

fn game(host: bool, case: &Value) -> Game {
    let mut session: Session = serde_json::from_value(case["session"].clone()).unwrap();
    if !host {
        // The client's own session before the snapshot: no players yet.
        session.players.clear();
        session.last_player_id = 0;
        session.created = None;
    }
    let mut a = Synchronizer::new(0x10000);
    a.reset(-1, -1);
    Game {
        host,
        a,
        executed: Vec::new(),
        ordering_player: None,
        previous_ordering_player: None,
        stream_bits: 0,
        session,
        data: Data::new(case),
        conns: Vec::new(),
        own_address: None,
        now: 0,
        kingdoms_assigned: Vec::new(),
        pending_joins: Vec::new(),
        requests: Vec::new(),
        validations: Vec::new(),
    }
}

struct HostResult {
    executed: Vec<(usize, Vec<Field>)>,
    validations: Vec<(u32, i32, i32, bool)>,
    join_packet: i32,
    ticks_before_join: u32,
    kingdoms_assigned: Vec<u32>,
    datagrams_sent: usize,
}
struct ClientResult {
    executed: Vec<(usize, Vec<Field>)>,
    first_packet: i32,
    sync_flushes: Vec<(i32, i32)>,
    local_player: Option<u32>,
    snapshot_players: Vec<u32>,
    datagrams_sent: usize,
}

fn host_thread(socket_tx: mpsc::Sender<SocketAddr>, stop: Arc<AtomicBool>, case: Value) -> HostResult {
    let precision = X87Precision::Double; // network thread PC53
    let mut net = UdpNode::bind("127.0.0.1:0", 0x1234, precision).unwrap();
    socket_tx.send(net.local_addr().unwrap()).unwrap();
    let avars = NetworkAvars::default();
    let mut node = AdminNode::new(0).unwrap();
    let mut spare = Spare::new();
    let mut g = game(true, &case);
    let host_identity: PeerIdentity = serde_json::from_value(case["node"]["identity"].clone()).unwrap();
    let mut gate = Gate::default();
    let state = HostConnectState {
        we_are_host: true,
        migrated_host: false,
        internet: false,
        session_mode: 1,
        launch_countdown: 0,
        password: Vec::new(),
        version: units("1.0.0.2"),
        build: units("AA2004"),
        data_checksum: 0x1234_abcd,
        depot_checksums: units("DepotSums"),
        mods: Vec::new(),
    };
    let start = Instant::now();
    let mut next_tick = 0.0f32;
    let mut ticks = 0u32;
    let (mut join_packet, mut ticks_before_join) = (-1, 0);
    let mut sent = 0;
    while !stop.load(Ordering::Relaxed) {
        let now = now_of(start);
        g.now = now.to_bits();
        for inc in net.receive_all(now).unwrap() {
            if g.conns.iter().all(|c| c.addr != peer_address(net.conns[inc.conn].addr)) {
                g.conns.push(Conn {
                    addr: peer_address(net.conns[inc.conn].addr),
                    admin: AdminConn::new().unwrap(),
                    file: NoFileTransfer::new(&avars),
                });
            }
            let addr = peer_address(net.conns[inc.conn].addr);
            let c = g.conns.iter_mut().find(|c| c.addr == addr).unwrap();
            for &(s, ok) in &inc.acks {
                c.admin.packet_result(&view(&mut node, &mut spare), s, ok, &mut c.file).unwrap();
            }
            let mut r = inc.body().unwrap();
            match inc.packet_type {
                2 => {
                    let verdict =
                        read_and_validate_connect(&mut c.admin, &mut r, &state, addr, now.to_bits(), &mut |_| false).unwrap();
                    if let ConnectVerdict::Accepted { request, .. } = verdict {
                        // 0x74C9C0 with the local player (0x76AC60).
                        assert!(node.emit_request(true, PLAYER_JOIN_REQUEST, &request, HOST_PLAYER, &Ctx).unwrap());
                    }
                }
                4 => {
                    let roles = Roles { we_are_host: true, we_are_server: true, ..roles() };
                    read_client_to_server(&mut c.admin, &mut r, roles, inc.most_recent).unwrap();
                }
                t => panic!("host got packet type {t}"),
            }
        }
        if now >= next_tick {
            next_tick += TICK;
            ticks += 1;
            if ticks % 5 == 2 {
                let fields = [Field::Uint { width: 32, value: ticks % 3 }];
                node.emit_request(true, SPEED_REQUEST, &fields, HOST_PLAYER, &Ctx).unwrap();
            }
            // Admin tick: seal own requests (0x752DA0), execute every peer in
            // list order (0x752E60: this node first), record the decree packet.
            node.seal_requests().unwrap();
            g.previous_ordering_player = node.previous_ordering_player;
            let mut peers = vec![Peer {
                requests: std::mem::replace(&mut node.requests, History::new(0, 1).unwrap()),
                ordering_player: Some(HOST_PLAYER),
            }];
            let joined: Vec<usize> = (0..g.conns.len()).filter(|&i| g.conns[i].admin.base.ordering_player.is_some()).collect();
            for &i in &joined {
                let c = &mut g.conns[i];
                peers.push(Peer {
                    requests: std::mem::replace(&mut c.admin.base.requests, History::new(0, 1).unwrap()),
                    ordering_player: c.admin.base.ordering_player,
                });
            }
            node.host_execute(&mut peers, &mut g, &Ctx, &Ctx, &mut gate).unwrap();
            let mut it = peers.into_iter();
            node.requests = it.next().unwrap().requests;
            for &i in &joined {
                g.conns[i].admin.base.requests = it.next().unwrap().requests;
            }
            // Host join completion 0x753580 (runs inside the PlayerJoin
            // process natively; no datagram is built in between here).
            for j in std::mem::take(&mut g.pending_joins) {
                let PeerLink::Conn(a) = j.link else { panic!() };
                let admin = AdminStateSource {
                    conn_address: Some(a),
                    field_200: j.resume.byte as u32,
                    field_204: u32::from(j.resume.bit),
                    player: j.player,
                    admin_time: (ticks as f32 * TICK).to_bits(),
                    decree_end: node.decrees.end(),
                };
                join_packet = node.decrees.end();
                ticks_before_join = ticks;
                let dir = Dir {
                    node: PeerView { address: None, identity: host_identity.clone() },
                    conns: g.conns.iter().map(|c| (c.addr, c.admin.base.identity())).collect(),
                };
                let c = g.conns.iter_mut().find(|c| c.addr == a).unwrap();
                complete_host_join(&mut c.admin, &mut view(&mut node, &mut spare), &g.session, &dir, &g.data, &admin, &j.sync_a, false)
                    .unwrap();
            }
            for (d, f) in std::mem::take(&mut g.requests) {
                node.emit_request(true, d, &f, HOST_PLAYER, &Ctx).unwrap();
            }
            node.seal_decrees(&mut g).unwrap();
        }
        // SendPackets: ask the gate until it answers 0 for every conn.
        net.begin_pass();
        for nc in 0..net.conns.len() {
            let addr = peer_address(net.conns[nc].addr);
            let Some(c) = g.conns.iter_mut().find(|c| c.addr == addr) else { continue };
            loop {
                let seq = &net.conns[nc].seq;
                let gn = GateNode {
                    connecting: false,
                    we_are_host: true,
                    we_are_server: true,
                    leaving: false,
                    leave_now: false,
                    rank: 0,
                    admin_clock: true,
                    backup_since: 0,
                };
                let gc = GateConn {
                    is_host: false,
                    is_server: false,
                    disconnecting: c.admin.disconnecting,
                    connected: c.admin.connected,
                    rank: c.admin.base.join_order,
                    player: c.admin.base.player.is_some(),
                    sent_this_pass: seq.sent_this_pass,
                    last_send: seq.last_send.to_bits(),
                };
                let mut backlog =
                    || c.admin.backlogged(&view(&mut node, &mut spare), true, &avars, NativeImage::V14, 0, &c.file);
                let wanted = wanted_packet_type(&gn, &gc, now.to_bits(), &mut backlog, precision).unwrap();
                match wanted.packet_type {
                    0 => break,
                    5 => {
                        net.send(nc, 5, now, &mut |w, s| {
                            write_server_to_client(&mut c.admin, &view(&mut node, &mut spare), w, s, roles(), NativeImage::V14, &mut c.file)
                        })
                        .unwrap();
                        sent += 1;
                    }
                    t => panic!("host gate asked for type {t}"),
                }
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    HostResult {
        executed: g.executed,
        validations: g.validations,
        join_packet,
        ticks_before_join,
        kingdoms_assigned: g.kingdoms_assigned,
        datagrams_sent: sent,
    }
}

fn client_thread(host: SocketAddr, stop: Arc<AtomicBool>, case: Value) -> ClientResult {
    let precision = X87Precision::Double;
    let mut net = UdpNode::bind("127.0.0.1:0", 0x5678, precision).unwrap();
    let hc = net.conn_for(host);
    let host_addr = peer_address(host);
    let avars = NetworkAvars::default();
    let mut node = AdminNode::new(0).unwrap();
    let mut spare = Spare::new();
    let mut g = game(false, &case);
    let mut conn = AdminConn::new().unwrap();
    let mut file = NoFileTransfer::new(&avars);
    let mut gate = Gate::default();
    let key = units("loopback-client");
    let info = ConnectInfo {
        name: units("Client"),
        password: Vec::new(),
        version: units("1.0.0.2"),
        build: units("AA2004"),
        data_checksum: 0x1234_abcd,
        depot_checksums: units("DEPOTSUMS"),
        mods: Vec::new(),
        player_id: PlayerId { name: key, serial: 7, a: 1, b: 2, c: 3 },
        internet: false,
    };
    let mut admin = ClientAdmin {
        own_address: None,
        resume_byte: 0,
        resume_bit: 0,
        resume_player: None,
        admin_time: 0,
        decree_next: 0,
        decree_end: 0,
        clock_selected: false,
        clock: ClientClock::default(),
    };
    let mut snap = SnapHost::default();
    let start = Instant::now();
    let (mut connecting, mut server) = (true, false);
    let mut resumed = false;
    let mut processed = 0i32;
    let mut first_packet = -1;
    let mut local_player = None;
    let mut snapshot_players = Vec::new();
    let mut ticks = 0u32;
    let mut next_tick = 0.0f32;
    let mut flushes = Vec::new();
    let mut sent = 0;
    while !stop.load(Ordering::Relaxed) {
        let now = now_of(start);
        for inc in net.receive_all(now).unwrap() {
            assert_eq!(inc.conn, hc);
            for &(s, ok) in &inc.acks {
                conn.packet_result(&view(&mut node, &mut spare), s, ok, &mut file).unwrap();
            }
            let mut r = inc.body().unwrap();
            match inc.packet_type {
                5 => {
                    let roles = Roles { conn_is_host: true, conn_is_server: server, connecting, ..roles() };
                    let events = read_server_to_client(&mut conn, &mut view(&mut node, &mut spare), &mut r, roles, inc.most_recent, &mut file)
                        .unwrap();
                    for e in events {
                        if let ConnEvent::HostBecameServer { .. } = e {
                            // SetServer(conn) 0x759790 and AdminNode+0x2CC = 0.
                            connecting = false;
                            server = true;
                        }
                    }
                }
                t => panic!("client got packet type {t}"),
            }
        }
        // AdminConn::Update -> 0x7531A0 on the host connection's private history.
        if conn.private_history.begin() < conn.private_history.end() {
            admin.decree_end = node.decrees.end();
            let env = ClockEnv { now, precision };
            let view_node = NodeView { host: PeerLink::Conn(host_addr) };
            let messages = process_private_messages(&mut conn, &mut admin, &mut g.session, &mut g.a, &mut g.data, &mut snap, &view_node, env)
                .unwrap();
            for m in messages {
                for a in m.actions {
                    if let LobbyAction::BindPlayer { link: PeerLink::Node, player } = a {
                        local_player = Some(player);
                    }
                }
                if m.kind == 0 {
                    // 0x74DF50: +0x20C, resume pair +0x248/+0x24C, +0x250.
                    processed = admin.decree_next;
                    first_packet = processed;
                    node.client.resume_byte = admin.resume_byte as i32;
                    node.client.resume_bit = admin.resume_bit as i32;
                    g.ordering_player = admin.resume_player;
                    resumed = true;
                }
            }
            snapshot_players = g.session.players.iter().map(|p| p.id).collect();
            g.own_address = admin.own_address;
        }
        if resumed && now >= next_tick {
            next_tick = now + TICK;
            ticks += 1;
            // Replay every decree packet that arrived (0x74FD10).
            while processed < node.decrees.end() && node.decrees.begin() <= processed {
                if node.decrees.get(processed).unwrap().is_none() {
                    break;
                }
                node.client_execute(processed, &mut g, &Ctx, &Ctx, &mut gate).unwrap();
                processed += 1;
            }
            if let Some(p) = local_player {
                if ticks.is_multiple_of(4) {
                    // 0x74E8B0 TGC_SetSyncValueOrderRequest {A last index, A hash}.
                    let (index, hash) = (g.a.end() - 1, g.a.hash);
                    let fields = [Field::Uint { width: 32, value: index as u32 }, Field::Uint { width: 32, value: hash as u32 }];
                    assert!(node.emit_request(true, SYNC_VALUE_REQUEST, &fields, p, &Ctx).unwrap());
                    flushes.push((index, hash));
                }
                if ticks % 6 == 1 {
                    let fields = [Field::Uint { width: 32, value: ticks % 3 }];
                    node.emit_request(true, SPEED_REQUEST, &fields, p, &Ctx).unwrap();
                }
                if ticks % 10 == 3 {
                    let fields = vec![
                        Field::Text(units("usa_infantry")),
                        Field::Uint { width: 32, value: 0x4300_0000 + ticks },
                        Field::Uint { width: 32, value: 0xc298_0000 },
                        Field::Uint { width: 32, value: 0 },
                    ];
                    node.emit_request(true, MOVE_REQUEST, &fields, p, &Ctx).unwrap();
                }
            }
            node.seal_requests().unwrap();
        }
        net.begin_pass();
        loop {
            let seq = &net.conns[hc].seq;
            let gn = GateNode {
                connecting,
                we_are_host: false,
                we_are_server: false,
                leaving: false,
                leave_now: false,
                rank: 1,
                admin_clock: admin.clock_selected,
                backup_since: 0,
            };
            let gc = GateConn {
                is_host: true,
                is_server: server,
                disconnecting: conn.disconnecting,
                connected: conn.connected,
                rank: conn.base.join_order,
                player: true,
                sent_this_pass: seq.sent_this_pass,
                last_send: seq.last_send.to_bits(),
            };
            let wanted = wanted_packet_type(&gn, &gc, now.to_bits(), &mut || Ok(false), precision).unwrap();
            match wanted.packet_type {
                0 => break,
                2 => {
                    net.send(hc, 2, now, &mut |w, _| write_connect(w, &info)).unwrap();
                }
                4 => {
                    let status = conn.status_inputs(&view(&mut node, &mut spare), true, false, &avars);
                    net.send(hc, 4, now, &mut |w, s| write_client_to_server(&mut conn, &view(&mut node, &mut spare), w, s, status))
                        .unwrap();
                }
                t => panic!("client gate asked for type {t}"),
            }
            sent += 1;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    ClientResult { executed: g.executed, first_packet, sync_flushes: flushes, local_player, snapshot_players, datagrams_sent: sent }
}

/// A UDP relay between client and host that drops every fifth datagram in
/// each direction and delays every eleventh one behind the next (network
/// loss and reordering on the loopback interface; no change to either node).
fn relay(host: SocketAddr, stop: Arc<AtomicBool>, addr_tx: mpsc::Sender<SocketAddr>) -> (usize, usize) {
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.set_read_timeout(Some(Duration::from_millis(5))).unwrap();
    addr_tx.send(socket.local_addr().unwrap()).unwrap();
    let mut client: Option<SocketAddr> = None;
    let mut buf = vec![0u8; 65_535];
    let (mut count, mut dropped, mut delayed) = ([0usize; 2], 0, 0);
    let mut held: [Option<Vec<u8>>; 2] = [None, None];
    while !stop.load(Ordering::Relaxed) {
        let (len, from) = match socket.recv_from(&mut buf) {
            Ok(x) => x,
            Err(_) => continue,
        };
        let dir = if from == host { 0 } else { 1 };
        if dir == 1 {
            client = Some(from);
        }
        let Some(to) = (if dir == 0 { client } else { Some(host) }) else { continue };
        count[dir] += 1;
        if count[dir] % 5 == 3 {
            dropped += 1;
            continue;
        }
        let data = buf[..len].to_vec();
        if count[dir] % 11 == 7 && held[dir].is_none() {
            held[dir] = Some(data);
            delayed += 1;
            continue;
        }
        socket.send_to(&data, to).unwrap();
        if let Some(late) = held[dir].take() {
            socket.send_to(&late, to).unwrap();
        }
    }
    (dropped, delayed)
}

fn run(lossy: bool) -> (HostResult, ClientResult, (usize, usize)) {
    let case = lobby_case();
    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let s1 = stop.clone();
    let hcase = case.clone();
    let host = std::thread::spawn(move || host_thread(tx, s1, hcase));
    let mut addr = rx.recv().unwrap();
    // The host ticks alone first: the client joins a running session.
    std::thread::sleep(Duration::from_millis(700));
    let mut relay_thread = None;
    if lossy {
        let (tx, rx) = mpsc::channel();
        let s = stop.clone();
        relay_thread = Some(std::thread::spawn(move || relay(addr, s, tx)));
        addr = rx.recv().unwrap();
    }
    let s2 = stop.clone();
    let client = std::thread::spawn(move || client_thread(addr, s2, case));
    std::thread::sleep(Duration::from_secs(if lossy { 8 } else { 6 }));
    stop.store(true, Ordering::Relaxed);
    let h = host.join().unwrap();
    let c = client.join().unwrap();
    let r = relay_thread.map(|t| t.join().unwrap()).unwrap_or((0, 0));
    (h, c, r)
}

fn check(h: &HostResult, c: &ClientResult, label: &str) {
    assert!(h.ticks_before_join > 5, "the host ticked {} times before the join", h.ticks_before_join);
    assert_eq!(c.local_player, Some(2), "the session snapshot bound the client to the second session player");
    assert_eq!(c.snapshot_players, vec![1, 2]);
    assert_eq!(h.kingdoms_assigned, vec![2]);
    assert_eq!(c.first_packet, h.join_packet, "the client resumes in the packet holding its PlayerJoin decree");
    // The client resumes right after its own PlayerJoin decree: it executes
    // exactly the host's decrees that follow that decree, in order.
    let join_at = h
        .executed
        .iter()
        .position(|e| e.0 == PLAYER_JOIN_DECREE)
        .expect("host executed the PlayerJoin decree");
    let host_after = &h.executed[join_at + 1..];
    assert!(c.executed.len() > 20, "client executed {} decrees", c.executed.len());
    assert!(host_after.len() >= c.executed.len());
    assert_eq!(&host_after[..c.executed.len()], &c.executed[..], "same decrees, fields and order after the resume");
    let kinds: std::collections::BTreeSet<usize> = c.executed.iter().map(|e| e.0).collect();
    for d in [SET_PLAYER_DECREE, POST_PLAYER_JOIN_DECREE, SYNC_VALUE_DECREE, 66, 73] {
        assert!(kinds.contains(&d), "decree {d} executed; got {kinds:?}");
    }
    // Every client synchronizer value the host executed matched (0x751D00 /
    // 0x77D050) although the host logged for many ticks before the join.
    assert!(h.validations.len() >= 5, "host validated {} sync values", h.validations.len());
    assert!(h.validations.iter().all(|v| v.0 == 2 && v.3), "{:?}", h.validations);
    let validated: Vec<(i32, i32)> = h.validations.iter().map(|v| (v.1, v.2)).collect();
    assert_eq!(&c.sync_flushes[..validated.len()], &validated[..]);
    assert!(h.datagrams_sent > 10 && c.datagrams_sent > 10);
    eprintln!(
        "{label}: host ticked {} times before the join (packet {}); host sent {} datagrams, client {}; {} decrees replayed; {} synchronizer values validated",
        h.ticks_before_join,
        h.join_packet,
        h.datagrams_sent,
        c.datagrams_sent,
        c.executed.len(),
        h.validations.len()
    );
}

#[test]
fn loopback_host_and_client_over_udp() {
    let (h, c, _) = run(false);
    check(&h, &c, "loopback");
}

#[test]
fn loopback_through_lossy_reordering_relay() {
    let (h, c, (dropped, delayed)) = run(true);
    assert!(dropped > 10 && delayed > 3, "relay dropped {dropped}, delayed {delayed}");
    check(&h, &c, &format!("lossy loopback ({dropped} dropped, {delayed} delayed)"));
}
