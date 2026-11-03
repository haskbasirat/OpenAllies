//! LAN multiplayer session of the original AA.exe, composed from the ported
//! admin layer of this crate (research/phase8-net/README.txt):
//!
//! * hosting, TGC_AdminNode::Host 0x75AEE0: the UDP socket on the first free
//!   port of `HostPortDefault .. + NetworkNumPortsToTry` (0x7641E0), the
//!   host's own SessionPlayer (0x773710, name 0x766560), SetServer 0x759790,
//!   SetHost 0x757010 (server clock 0x75FBF0), synchronizer A reset 0x77D900,
//!   then KKC_Session::Launch 0x77A950 into the staging state;
//! * joining: Connect (type 2, 0x750330) every 0.25 s until the host's
//!   ServerToClient arrives, the host's validation 0x75BF90 and its
//!   PlayerJoin request 0x74C9C0 (306), the PlayerJoin decree 0x758FD0 on
//!   every node, the host join completion 0x753580 (session snapshot
//!   0x76D0B0, admin state 0x74C130) and the client's private-history
//!   dispatcher 0x7531A0 (0x77AAC0 / 0x74DF50);
//! * every order as its native request (client stream admin+0x254, sealed by
//!   0x752DA0) and decree (host 0x752E60 / 0x751B60, sealed 0x752EA0, client
//!   replay 0x74FD10), through aa-orders' native flow with the order
//!   processes of `lan::orders`;
//! * the frame of both roles is the original tick driver 0x75CA70 (aa-runtime
//!   `simulation::Driver`): admin ticks of 1/16 s, world ticks from the
//!   timeline, the client gated by its received decree history and its
//!   client clock (0x75FB40, vf+8 0x75FD70), the host by its server clock
//!   (0x75FCB0, host rate 0x75FD80);
//! * the original sync check: every 17th admin tick a client sends
//!   TGC_SetSyncValueOrderRequest (302, 0x74E8B0) with synchronizer A's last
//!   index and hash, the host validates it in 0x751D00 (0x77D050) and on a
//!   mismatch writes TGC_SyncFailureOrderDecree (303, 0x750BA0) that every
//!   node executes (0x75BBC0). In game the host also sends synchronizer S
//!   (299, 0x74E670); 0x752F60 compares it for film players only.
//!
//! The datagrams come from the ported codecs of this crate (transport header,
//! admin packet types 2/4/5/8/10/11, channels, order payloads), each proven on
//! its own by its oracle (research/phase8-net/README.txt). This composition has
//! been tested on loopback: tests/lan_session.rs (two nodes, one process, a
//! stand-in world), tests/lan_battle.rs (two processes, an aa-play battle), and
//! with the original itself as a joining client as far as its staging screen
//! (tests/lan_original.rs, research/phase8-net/README.txt section 13). The
//! client's front end is crates/aa-bevy/src/net.
//!
//! Not ported, and what happens instead:
//! * heartbeat 0x75B820 and the lag votes (`crate::heartbeat` has them proven,
//!   nothing here calls it). [`LanNode::frame`] runs the rule of 0x75BD70 in
//!   short: a silent connection is no longer connected and its datagrams
//!   restore it (the original's network thread does not stall with a frame,
//!   ours can), and after the 15 s of patience the host drops its player
//!   (reason 4, without the dialog that asks for a vote); no backup server
//!   takes over: a client that lost its host has lost the session;
//! * the end-of-session check 0x76F600 and the session timing 0x41C180;
//! * the connect checksums and the map check: [`LanConfig`] carries what the
//!   caller computed, a client reports it has the map without looking
//!   (`SnapshotEffects`), a file transfer (status 2) is refused;
//! * of FinalizeLaunchSettings 0x776B20 everything but the seed, of 0x7791F0
//!   everything but 0x778D10 and the removal of the AIs;
//! * GameSpy, backup servers, a film recorder inside the node (the client
//!   records through `aa_parity::recording`), every order without a process
//!   (`orders`, refused by name). The team format ([session+0x14]) is its text
//!   alone: the records it seats come from [`LanData::kingdom_open`].
//!
//! Synchronizer A here holds the clock and timeline entries, the order streams
//! and the world-RNG getter values. The original's other A writers (785 int
//! sites, the world dump 0x4F6280, the launch dump 0x76D6C0:
//! research/byte-exact/SYNC_LOG.md) are not ported, so a world divergence that
//! draws nothing from the world RNG goes unnoticed, and an original peer's
//! hash would never match.
use crate::lobby::{
    self, ClientAdmin, ClientClock, ClockEnv, LaunchSettings, PeerIdentity, PeerLink, PeerView,
    Session,
};
use crate::packets::{
    self, AdminConn, ConnEvent, NetworkAvars, NoFileTransfer, NodeHistories, Roles,
};
use crate::send::{wanted_packet_type, GateConn, GateNode};
use crate::session::{read_and_validate_connect, write_connect, ConnectInfo, ConnectVerdict, HostConnectState, PeerAddress, PlayerId};
use crate::udp::UdpNode;
use crate::{Error, Result};
use aa_numeric::Precision;
use aa_orders::net::admin::AdminNode;
use aa_orders::net::sync::Synchronizer;
use aa_orders::net::transport::X87Precision;
use aa_orders::payload::{Field, ThingCheck};
use aa_orders::{Gate, History};
use aa_runtime::server_clock::{ServerClock, TimeSource};
use aa_runtime::simulation::{self as sim, Action, Driver, Gates, Role as DriverRole, Timeline};
use aa_simulation::{FloatBits, NativeClock};
use std::cell::RefCell;
use std::net::{SocketAddr, UdpSocket};
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub mod orders;
pub mod registries;

pub use orders::descriptor;
use registries::NativeRegistries;

/// AVars_AA `HostPortDefault`.
pub const HOST_PORT_DEFAULT: u16 = 7290;
/// AVars `NetworkNumPortsToTry` (0x75AF8F reads it; ports port..port+n-1).
pub const PORTS_TO_TRY: u16 = 10;
/// SVars AdminTicksPerSecond (16.0) and View SVar
/// NetworkAdminAsyncTicksPerSecond (16.0) as binary32.
pub const ADMIN_TICKS_PER_SECOND_BITS: u32 = 0x4180_0000;
/// 0x765452: FMUL 0x9F4580 (binary32 0.001) of the NetNode timer.
const MS_TO_SECONDS_BITS: u32 = 0x3a83_126f;
/// Engineering bound on driver frames run by one [`LanNode::frame`] call
/// (the native thread loops 0x75CA70 continuously; one call here stands for
/// the loop iterations of one rendered frame).
pub const MAX_DRIVER_STEPS: usize = 64;
/// 0x75BD70: liveness timeout of the upstream server connection and of
/// every other connection (seconds since NetConn+0x2C).
pub const SERVER_TIMEOUT: f32 = 3.0;
pub const PEER_TIMEOUT: f32 = 2.0;
/// [0xA874AC]: a leaving node shuts down after 2.5 s.
pub const LEAVE_DELAY: f32 = 2.5;
/// AdminConn +0x44 starts at 15 s of patience (0x75BD70 takes 1/s while lagging).
pub const LOST_PATIENCE: f32 = 15.0;

/// The local player and build identity a node connects or hosts with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanConfig {
    /// UVar NetworkLanNickname (AdminNode+0xEC).
    pub name: Vec<u16>,
    /// Password typed when joining (AdminNode+0xF0).
    pub password: Vec<u16>,
    /// [0xB3888C] version and [0xB388A0] application strings of 0x750330.
    pub version: Vec<u16>,
    pub build: Vec<u16>,
    /// [0xB36530]+8 data checksum, 0x5AF810 depot and 0x5AF970 mod strings.
    pub data_checksum: u32,
    pub depot_checksums: Vec<u16>,
    pub mods: Vec<u16>,
    /// LAN player id object (0x77B8F0).
    pub player_id: PlayerId,
}

/// The CreateGameDialog choices of a host (0x6EBAB0, session +0xCC..+0xDC).
/// The dialog fills every field; there are no default choices here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostOptions {
    pub game_name: Vec<u16>,
    pub password: Vec<u16>,
    pub max_players: i32,
    pub allow_observers: bool,
    /// First port tried and the count ([`HOST_PORT_DEFAULT`], [`PORTS_TO_TRY`]).
    pub port: u16,
    pub ports_to_try: u16,
    /// Bind address (0.0.0.0 natively).
    pub bind: std::net::Ipv4Addr,
}

/// What happened in a frame, for the front end.
#[derive(Debug, Clone, PartialEq)]
pub enum LanEvent {
    /// Client: the session snapshot and admin state arrived (0x7531A0); the
    /// node is a session member with this player.
    Joined { player: u32 },
    /// The host ended our connection: ClientDisconnectDecree (type 11,
    /// 0x75A780, reason index into `packets::REASON_KEYS`) or
    /// ServerDisconnect (type 8, 0x75A730: reason `None`).
    Disconnected { reason: Option<u32> },
    /// 0x40BC50(&0xB399B0, kind, text) of a process: a localization key with
    /// its arguments (`|s` strings) or number (`|d`).
    Message { kind: u32, key: String, args: Vec<Vec<u16>>, number: Option<i32> },
    /// TGC_PublicChatOrderDecree (77, 0x75E070 -> 0x755E00): its fields.
    Chat { player: Option<u32>, fields: Vec<Field> },
    /// KKC_Session::ChangeToState 0x77A750 posted its state event 0x7851C0.
    StateChanged { state: u32 },
    /// KKC_Session::Launch 0x77A950 created the launch task (0x775EE0): the
    /// node must now load the session's world and report it with
    /// [`LanNode::report_world_loaded`].
    LoadWorld,
    /// 0x775BF0: every peer finished its task successfully; the timeline
    /// was unfrozen (0x76DD90(0)) and world ticks run.
    BeginPlay,
    /// 0x773AE0: a peer's task failed.
    TasksFailed,
    /// TGC_SyncFailureOrderDecree (303) process 0x75BBC0: `player` is out
    /// of sync with the host.
    SyncFailure { player: u32 },
    /// The session's player list changed (join, leave, kingdom, ready).
    PlayersChanged,
    /// 0x754880: the game was paused or resumed by this player's order.
    Paused { paused: bool, player: Option<u32> },
    /// The node left the session (the leave delay 0xA874AC ran out).
    Left,
    /// Host: 0x75BF90 refused a connect (the reason index of
    /// `packets::REASON_KEYS`, what it said of itself).
    ConnectRefused { reason: u32, info: ConnectInfo },
    /// No datagram from a connection for [`PEER_TIMEOUT`] /
    /// [`SERVER_TIMEOUT`] (0x75BD70).
    PeerTimedOut { address: PeerAddress },
}

/// Game data a session's codecs read (registry counts, name lookups,
/// localization): the same on every node of the session.
pub trait LanData {
    fn registries(&self) -> &NativeRegistries;
    /// 0x41EEC0 random-map configuration by name: its own name.
    fn random_map_config(&self, key: &[u16]) -> Option<Vec<u16>>;
    /// 0x59F570 scenario by name: its own name.
    fn scenario(&self, key: &[u16]) -> Option<Vec<u16>>;
    /// `CString::Format(0x4B9F40(key), args)` (e.g. the duplicate-name rule
    /// "networking_duplicate_name_formate|sd").
    fn format(&self, key: &str, args: &[Vec<u16>]) -> Vec<u16>;
    /// AVar LaunchCountdownTime (0x41D430, 5 in AVars.tgi).
    fn countdown_time(&self) -> i32;
    /// Nation +0x48/+0x4C (`factions`) of a Nation IDS, as Faction IDS.
    fn nation_factions(&self, nation: &[u16]) -> Result<Vec<Vec<u16>>>;
    /// SVars GameSpeedMin/Max, GameSpeedFilmMin/Max, AdminTicksPerSecond
    /// (0x76F370 reads them).
    fn speed_settings(&self) -> sim::SpeedSettings;
    /// 0x77E430: the entry of a kingdom record in the session's team format
    /// (+0x14, 0x781250): `Some(open)`, `None` when the lists are empty.
    fn kingdom_open(&self, _session: &lobby::Session, _record: usize) -> Option<bool> {
        None
    }
    /// The kingdom records of a random-map configuration (0x41EEC0, its
    /// kingdoms in file order), `None` for an unknown name.
    fn rmc_header(&self, _rmc: &[u16]) -> Option<lobby::MapHeader> {
        None
    }
}

/// The world side of a session (aa-play in the client, a fake in tests).
pub trait LanGame {
    /// A decree with a gameplay effect, executed in an admin tick in the same
    /// order on every node: its descriptor, payload (references resolved as
    /// the host's reader resolved them) and the ordering SessionPlayer.
    fn decree(&mut self, player: Option<u32>, descriptor: usize, fields: &[Field]) -> Result<()>;
    /// Thing reference readers 0x76A4B0.. (type check `check`): the slot of a
    /// live thing, `None` for a reference the reader resolves to null.
    fn thing(&self, check: ThingCheck, slot: u16) -> Option<u16>;
    /// g_gworld's clock, `None` before the world exists.
    fn world_clock(&self) -> Option<NativeClock>;
    /// One GWorld update 0x4F9970 (applying the commands of the decrees
    /// executed since the last one first). Returns the new clock and the
    /// world-RNG getter values GWorld::GetRNG 0x4F02B0 logged to
    /// synchronizer A ("(int)m_rng.GetSeed()"), in draw order.
    fn world_update(&mut self) -> Result<(NativeClock, Vec<u32>)>;
    /// 0x76EE10 MidgameCreateImplicitAIs: `player` left in the game; its
    /// kingdom goes on under an implicit computer player.
    fn implicit_ai(&mut self, _player: u32) -> Result<()> {
        Ok(())
    }
}

/// A session without a world (the lobby and staging).
pub struct NoWorld;
impl LanGame for NoWorld {
    fn decree(&mut self, _player: Option<u32>, descriptor: usize, _fields: &[Field]) -> Result<()> {
        Err(Error::Protocol(format!("gameplay decree {descriptor} without a world")))
    }
    fn thing(&self, _check: ThingCheck, _slot: u16) -> Option<u16> {
        None
    }
    fn world_clock(&self) -> Option<NativeClock> {
        None
    }
    fn world_update(&mut self) -> Result<(NativeClock, Vec<u32>)> {
        Err(Error::Protocol("world update without a world".into()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Host,
    Client,
}

/// One remote peer: TGC_NetConn (in [`UdpNode`]) and its TGC_AdminConn.
pub(crate) struct Conn {
    pub addr: PeerAddress,
    pub admin: AdminConn,
    pub file: NoFileTransfer,
    /// AdminPeer +0x34 task status (0x752FC0).
    pub task: u32,
    /// The liveness timeout was reported.
    pub timed_out: bool,
    /// When it was (the patience of [`LOST_PATIENCE`] runs from there).
    pub lost_at: Option<f32>,
}

/// The backup histories of the node (AdminNode +0x1C / +0x224 / +0x234):
/// no backup server is ever chosen in a LAN session of this port, but the
/// channel codecs read them.
pub(crate) struct Spare {
    pub backup_requests: History,
    pub backup_decrees: History,
    pub backup_next: i32,
}

/// Engine-side effects of processes run inside a driver frame, applied when
/// the frame returns (the driver is borrowed while they run).
#[derive(Default)]
pub(crate) struct Deferred {
    /// 0x766D40 OrderVersionAck after a session state event.
    pub version_ack: bool,
    /// 0x750BA0 SyncFailure decrees the host owes (player ids).
    pub sync_failures: Vec<u32>,
    /// Requests emitted by processes outside a request-emission loan.
    pub requests: Vec<(usize, Vec<Field>)>,
}

/// A PlayerJoin decree the host processed this admin tick: its connection,
/// the decree-stream position after it (AdminNode+0x200/+0x204) and
/// synchronizer A at that point (0x753580 runs inside the process).
pub(crate) struct PendingJoin {
    pub link: PeerLink,
    pub resume_bits: usize,
    pub sync_a: Synchronizer,
    pub player: Option<u32>,
}

/// Node state other than the order streams and the driver.
pub(crate) struct State {
    pub role: Role,
    pub udp: UdpNode,
    pub conns: Vec<Conn>,
    pub start: Instant,
    /// NetNode+0x58 (seconds, binary32) and the timer milliseconds it came from.
    pub now: f32,
    pub ms: u32,
    pub precision: X87Precision,
    pub pc: Precision,
    pub avars: NetworkAvars,
    pub spare: Spare,
    pub sync_a: Synchronizer,
    pub sync_s: Synchronizer,
    pub session: Session,
    /// The stored single/multiplayer launch settings ([[0xB39D04]+0x104/+0x108]).
    pub stored_single: Option<LaunchSettings>,
    pub stored_multi: Option<LaunchSettings>,
    /// AdminNode+8 local player and +0x2C (the node's ordering player).
    pub local_player: Option<u32>,
    pub own_ordering: Option<u32>,
    /// AdminNode+0x29C our address as the host sees it.
    pub own_address: Option<PeerAddress>,
    /// The node's own AdminPeer fields: +0x6C order version, +0x34 task, and
    /// the identity (+0x3C map status, +0x70, +0x74).
    pub own_6c: u8,
    pub own_task: u32,
    pub own_identity: PeerIdentity,
    /// AdminNode+0x2AC: the session peers in list order.
    pub peers: Vec<PeerLink>,
    /// Client: the host connection's address; joining state.
    pub host_addr: Option<PeerAddress>,
    pub connect: Option<ConnectInfo>,
    pub connecting: bool,
    pub server_set: bool,
    pub joined: bool,
    pub client: ClientAdmin,
    /// Host: the connect checks and the server clock (AdminNode+0x15C).
    pub host_connect: Option<HostConnectState>,
    pub server_clock: ServerClock,
    /// Session+0x110 countdown timer start (milliseconds).
    pub countdown_started: Option<u32>,
    pub pending_joins: Vec<PendingJoin>,
    pub deferred: Deferred,
    /// "o.GetBitSize()" last logged to A (decree-stream bits).
    pub stream_bits: u32,
    pub events: Vec<LanEvent>,
    /// AdminNode+0x2A0 the client who paused, +0x2A4 the last one, +0x2A8 when
    /// it ended, the clients' votes (+0x56) and last pauses (+0x58): 0x754880.
    pub pause_holder: Option<PeerLink>,
    pub pause_last: Option<PeerLink>,
    pub pause_ended: f32,
    pub pause_votes: Vec<PeerLink>,
    pub pause_last_paused: Vec<(PeerLink, f32)>,
    /// AdminNode+0x2CD: when the node started to leave.
    pub leaving: Option<f32>,
    /// The leave delay ran out: the node sends nothing more.
    pub left: bool,
}

/// One session node (host or client).
pub struct LanNode {
    pub(crate) admin: AdminNode,
    pub(crate) driver: Driver,
    pub(crate) gate: Gate,
    pub(crate) st: State,
    pub(crate) data: Box<dyn LanData + Send + Sync>,
    /// The game's world clock after the last driver step.
    pub(crate) world_clock_hint: Option<NativeClock>,
    /// The frame kernel's failure latch (a frame that failed part way).
    pub(crate) stopped: bool,
}

fn peer_address(a: SocketAddr) -> Result<PeerAddress> {
    match a {
        SocketAddr::V4(v4) => Ok(PeerAddress { ip: u32::from(*v4.ip()), port: v4.port() }),
        SocketAddr::V6(_) => Err(Error::Protocol("the original network layer is IPv4 only".into())),
    }
}
fn socket_of(a: PeerAddress) -> SocketAddr {
    SocketAddr::from((std::net::Ipv4Addr::from(a.ip), a.port))
}

/// The NetNode timer behind NetNode+0x58: one origin per process, as the
/// original's timer outlives its sessions.
fn node_timer() -> Instant {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    *ORIGIN.get_or_init(Instant::now)
}
/// NetNode+0xB0 at Init 0x764190: trunc(NetNode clock * 10.0).
fn node_session_id(origin: Instant, pc: Precision) -> Result<i32> {
    let now = node_seconds(origin.elapsed().as_millis() as u32, pc)?;
    Ok((f64::from(now) * 10.0) as i32)
}

/// 0x765410 prefix: NetNode+0x58 = binary32(ms * 0.001f) at the thread's
/// precision (the integer is not spilled before the multiply).
fn node_seconds(ms: u32, pc: Precision) -> Result<f32> {
    let mut v = aa_numeric::Binary::from_i32(ms as i32);
    if (ms as i32) < 0 {
        v = v.add(&aa_numeric::Binary::from_f32_bits(0x4f80_0000).map_err(err)?, pc).map_err(err)?;
    }
    let s = v.mul(&aa_numeric::Binary::from_f32_bits(MS_TO_SECONDS_BITS).map_err(err)?, pc).map_err(err)?.store_f32().map_err(err)?;
    Ok(f32::from_bits(s))
}
pub(crate) fn err(e: impl std::fmt::Display) -> Error {
    Error::Protocol(e.to_string())
}
fn sim_err(e: impl std::fmt::Display) -> sim::Error {
    sim::seam_error(e.to_string())
}

/// The server clock's time source: NetNode+0x58 cached, the 59ABA0
/// millisecond timer otherwise.
struct NodeClock {
    now: f32,
    ms: u32,
}
impl TimeSource for NodeClock {
    fn cached_seconds_bits(&mut self) -> std::result::Result<u32, String> {
        Ok(self.now.to_bits())
    }
    fn milliseconds(&mut self) -> std::result::Result<u32, String> {
        Ok(self.ms)
    }
}

fn new_driver(role: DriverRole, pc: Precision) -> Result<Driver> {
    let timeline = Timeline::new(ADMIN_TICKS_PER_SECOND_BITS, pc).map_err(err)?;
    let gates = Gates {
        // Session+0xF4 multiplayer: the timeline runs without the
        // single-player auto pause.
        session_f4: true,
        session_mode: 0,
        flags_b396c4: 0,
        single_player_auto_pause: true,
        render_dump_avi: false,
        avi_frames_per_tick: 0,
    };
    Ok(Driver::new(sim::AdminNode::new(role), timeline, gates, FloatBits(ADMIN_TICKS_PER_SECOND_BITS), pc))
}

impl State {
    fn new(role: Role, udp: UdpNode) -> Result<Self> {
        let mut sync_a = Synchronizer::new(aa_orders::net::sync::DEFAULT_MAX_ENTRIES);
        sync_a.enabled = false;
        Ok(Self {
            role,
            udp,
            conns: Vec::new(),
            start: Instant::now(),
            now: 0.0,
            ms: 0,
            precision: X87Precision::Double,
            pc: Precision::Double53,
            avars: NetworkAvars::default(),
            spare: Spare {
                backup_requests: History::new(0, 0x100)?,
                backup_decrees: History::new(0, 0x100)?,
                backup_next: 0,
            },
            sync_a,
            sync_s: Synchronizer::new(aa_orders::net::sync::DEFAULT_MAX_ENTRIES),
            session: Session { properties: Some(Default::default()), ..Default::default() },
            stored_single: None,
            stored_multi: None,
            local_player: None,
            own_ordering: None,
            own_address: None,
            own_6c: 1,
            own_task: 0,
            own_identity: PeerIdentity::default(),
            peers: Vec::new(),
            host_addr: None,
            connect: None,
            connecting: false,
            server_set: false,
            joined: false,
            client: ClientAdmin {
                own_address: None,
                resume_byte: 0,
                resume_bit: 0,
                resume_player: None,
                admin_time: 0,
                decree_next: 0,
                decree_end: 0,
                clock_selected: false,
                clock: ClientClock::default(),
            },
            host_connect: None,
            server_clock: ServerClock::from_native_constructor(),
            countdown_started: None,
            pending_joins: Vec::new(),
            deferred: Deferred::default(),
            stream_bits: 0,
            events: Vec::new(),
            pause_holder: None,
            pause_last: None,
            pause_ended: -10000.0,
            pause_votes: Vec::new(),
            pause_last_paused: Vec::new(),
            leaving: None,
            left: false,
        })
    }

    pub(crate) fn conn_index(&self, addr: PeerAddress) -> Option<usize> {
        self.conns.iter().position(|c| c.addr == addr)
    }
    pub(crate) fn conn_of_link(&mut self, link: &PeerLink) -> Option<&mut Conn> {
        match link {
            PeerLink::Conn(a) => self.conns.iter_mut().find(|c| c.addr == *a),
            _ => None,
        }
    }
    /// 0x764970: the connection for an address, created on first use.
    pub(crate) fn ensure_conn(&mut self, addr: PeerAddress) -> Result<usize> {
        if let Some(i) = self.conn_index(addr) {
            return Ok(i);
        }
        self.udp.conn_for(socket_of(addr));
        self.conns.push(Conn { addr, admin: AdminConn::new()?, file: NoFileTransfer::new(&self.avars), task: 0, timed_out: false, lost_at: None });
        Ok(self.conns.len() - 1)
    }
    pub(crate) fn we_are_host(&self) -> bool {
        self.role == Role::Host
    }
    /// AdminNode+0x2C4 as a [`lobby::NodeView`].
    pub(crate) fn view(&self) -> lobby::NodeView {
        match (self.role, self.host_addr) {
            (Role::Host, _) => lobby::NodeView { host: PeerLink::Node },
            (Role::Client, Some(a)) => lobby::NodeView { host: PeerLink::Conn(a) },
            (Role::Client, None) => lobby::NodeView { host: PeerLink::Null },
        }
    }
    /// The SessionPlayer's AdminPeer link (player +4).
    pub(crate) fn player_link(&self, id: u32) -> Option<PeerLink> {
        self.session.players.iter().find(|p| p.id == id).map(|p| p.link)
    }
    /// AdminPeer +0x6C of a link (order version acknowledged).
    pub(crate) fn link_6c(&self, link: &PeerLink) -> u8 {
        match link {
            PeerLink::Node => self.own_6c,
            PeerLink::Conn(a) => self.conns.iter().find(|c| c.addr == *a).map_or(0, |c| c.admin.base.field_6c),
            PeerLink::Null => 0,
        }
    }
    pub(crate) fn set_link_6c(&mut self, link: &PeerLink, value: u8) {
        match link {
            PeerLink::Node => self.own_6c = value,
            PeerLink::Conn(a) => {
                if let Some(c) = self.conns.iter_mut().find(|c| c.addr == *a) {
                    c.admin.base.field_6c = value;
                }
            }
            PeerLink::Null => {}
        }
    }
    pub(crate) fn link_task(&self, link: &PeerLink) -> u32 {
        match link {
            PeerLink::Node => self.own_task,
            PeerLink::Conn(a) => self.conns.iter().find(|c| c.addr == *a).map_or(0, |c| c.task),
            PeerLink::Null => 0,
        }
    }
    pub(crate) fn set_link_task(&mut self, link: &PeerLink, value: u32) {
        match link {
            PeerLink::Node => self.own_task = value,
            PeerLink::Conn(a) => {
                if let Some(c) = self.conns.iter_mut().find(|c| c.addr == *a) {
                    c.task = value;
                }
            }
            PeerLink::Null => {}
        }
    }
    pub(crate) fn event(&mut self, e: LanEvent) {
        self.events.push(e);
    }
}

/// AdminPeer views (vfunc 0 address and +0x3C/+0x70/+0x74) of this node.
pub(crate) struct Directory<'a> {
    pub st: &'a State,
}
impl lobby::PeerDirectory for Directory<'_> {
    fn peer(&self, link: &PeerLink) -> Result<PeerView> {
        match link {
            // 0x74B4D0: AdminNode+0x29C (null on the host).
            PeerLink::Node => Ok(PeerView { address: self.st.own_address, identity: self.st.own_identity.clone() }),
            PeerLink::Conn(a) => {
                let c = self
                    .st
                    .conns
                    .iter()
                    .find(|c| c.addr == *a)
                    .ok_or_else(|| Error::Protocol(format!("no connection {a:?}")))?;
                Ok(PeerView { address: Some(*a), identity: c.admin.base.identity() })
            }
            PeerLink::Null => Err(Error::Protocol("null AdminPeer".into())),
        }
    }
}

impl LanNode {
    /// TGC_AdminNode::Host 0x75AEE0 for a multiplayer session staging
    /// `launch` (the map / RMC and its kingdom records).
    pub fn host(config: &LanConfig, options: &HostOptions, launch: LaunchSettings, data: Box<dyn LanData + Send + Sync>) -> Result<Self> {
        // 0x75AFE8..0x75B00B: the first port of the range that binds; an empty
        // range fails like an exhausted one (the message and event 0x12 are
        // this error).
        let mut bound = None;
        let mut last = None;
        for i in 0..options.ports_to_try {
            let port = options.port.wrapping_add(i);
            match UdpSocket::bind((options.bind, port)) {
                Ok(s) => {
                    bound = Some(s);
                    break;
                }
                Err(e) => last = Some(e),
            }
        }
        let socket = match bound {
            Some(s) => s,
            None => return Err(last.map(Error::from).unwrap_or_else(|| Error::Protocol("no port to bind".into()))),
        };
        let pc = Precision::Double53;
        let start = node_timer();
        let session_id = node_session_id(start, pc)?;
        let udp = UdpNode::from_socket(socket, session_id, X87Precision::Double)?;
        let mut st = State::new(Role::Host, udp)?;
        st.start = start;
        let mut node = Self { admin: AdminNode::new(0)?, driver: new_driver(DriverRole::Host, pc)?, gate: Gate::default(), st, data, world_clock_hint: None, stopped: false };
        node.tick_time()?;
        node.start_host(config, options, launch)?;
        Ok(node)
    }

    /// Connect to a host (LobbyJoinDirectExecute "ip port" 0x648030 ->
    /// 0x787750): an ephemeral client socket, then Connect packets.
    pub fn join(config: &LanConfig, host: SocketAddr, data: Box<dyn LanData + Send + Sync>) -> Result<Self> {
        let pc = Precision::Double53;
        // 0.0.0.0 natively; a host on this machine keeps the socket on loopback.
        let bind: SocketAddr = match host {
            SocketAddr::V4(v4) if v4.ip().is_loopback() => "127.0.0.1:0".parse().map_err(err)?,
            _ => "0.0.0.0:0".parse().map_err(err)?,
        };
        let start = node_timer();
        let session_id = node_session_id(start, pc)?;
        let udp = UdpNode::bind(bind, session_id, X87Precision::Double)?;
        let mut st = State::new(Role::Client, udp)?;
        st.start = start;
        let host_addr = peer_address(host)?;
        st.ensure_conn(host_addr)?;
        st.host_addr = Some(host_addr);
        st.connecting = true;
        st.session.multiplayer = true;
        st.connect = Some(ConnectInfo {
            name: config.name.clone(),
            password: config.password.clone(),
            version: config.version.clone(),
            build: config.build.clone(),
            data_checksum: config.data_checksum,
            depot_checksums: config.depot_checksums.clone(),
            mods: config.mods.clone(),
            player_id: config.player_id.clone(),
            internet: false,
        });
        let mut node = Self { admin: AdminNode::new(0)?, driver: new_driver(DriverRole::Client, pc)?, gate: Gate::default(), st, data, world_clock_hint: None, stopped: false };
        node.tick_time()?;
        Ok(node)
    }

    fn start_host(&mut self, config: &LanConfig, options: &HostOptions, launch: LaunchSettings) -> Result<()> {
        let port = self.st.udp.local_addr()?.port();
        let st = &mut self.st;
        st.session.multiplayer = true;
        // KKC_Session::Reset 0x76EC70: 0x85D880(GetTickCount) seeds the session
        // stream +0x128 (this clock stands in for the tick count: the value
        // only has to differ between sessions, and clients get the state).
        let tick = SystemTime::now().duration_since(UNIX_EPOCH).map_err(err)?.as_millis() as u32;
        st.session.field_128 = aa_simulation::NativeRng::from_native_seed(tick).state;
        // 0x75B142: running, ++session+0xE8, CreatePlayer 0x773710 on the
        // node, the LAN name through 0x766560, node+8 = node+0x2C = player.
        let id = st.session.last_player_id.wrapping_add(1);
        st.session.last_player_id = id;
        let join_node = lobby::JoinNode {
            view: st.view(),
            own_address: None,
            now: st.now.to_bits(),
            internet: false,
            peers_changed: false,
            multiplayer_assert_ignored: false,
        };
        let data_view = orders::DataView { data: &*self.data };
        lobby::create_player(&mut st.session, id, PeerLink::Node, false, false, &join_node, &data_view)?;
        let index = st.session.players.len() - 1;
        {
            let mut names = orders::NameFormat { data: &*self.data };
            lobby::set_player_name(&mut st.session, index, &config.name, &mut names)?;
        }
        st.local_player = Some(id);
        st.own_ordering = Some(id);
        st.own_identity = PeerIdentity { field_3c: 0, join_order: 0, player_id: config.player_id.clone() };
        // 0x45F240 the node joins +0x2AC; SetServer 0x759790; SetHost
        // 0x757010: ResetOrderVersion(1), the server clock restart with
        // 1 / AdminTicksPerSecond from the native positive zero.
        st.peers.push(PeerLink::Node);
        st.own_6c = 1;
        let period = crate::heartbeat::x87_div_store(1.0, f32::from_bits(ADMIN_TICKS_PER_SECOND_BITS)).to_bits();
        let mut clock = NodeClock { now: st.now, ms: st.ms };
        st.server_clock.restart(period, 0, &mut clock, st.pc).map_err(err)?;
        // 0x75B20E: synchronizer A continues from (0, 0).
        st.sync_a.reset(0, 0);
        // Session game options (+0xCC name, +0xD0 password, +0xD4 max
        // players, +0xDC observers).
        st.session.options.name = options.game_name.clone();
        st.session.options.password = options.password.clone();
        st.session.options.max_players = options.max_players;
        // Session +0xD8 = AdminNode +0xD8, the port that bound (0x75B11D).
        st.session.options.field_d8 = u32::from(port);
        st.session.options.allow_observers = options.allow_observers;
        st.host_connect = Some(HostConnectState {
            we_are_host: true,
            migrated_host: false,
            internet: false,
            session_mode: 0,
            launch_countdown: 0,
            password: options.password.clone(),
            version: config.version.clone(),
            build: config.build.clone(),
            data_checksum: config.data_checksum,
            depot_checksums: config.depot_checksums.clone(),
            mods: config.mods.clone(),
        });
        // 0x75B228 SetNewState(1, launch state) on the settings the staging
        // launch restores (0x77A750's restore path copies Launch88 into
        // Launch, so both hold the settings to stage).
        let mut staged = launch;
        staged.game_type = 1;
        st.session.launch = staged.clone();
        st.session.launch_88 = staged;
        // 0x75B387 KKC_Session::Launch into the staging state.
        let view = st.view();
        let node = orders::staging_node(st, &view, st.local_player);
        let mut fx = orders::StagingEffects::new(st, &*self.data);
        crate::staging::launch(&mut st.session, &node, &mut fx)?;
        let freeze = std::mem::take(&mut fx.freeze);
        fx.apply(st, None, &*self.data)?;
        // 0x76DD90(1) of that launch: the timeline stays frozen.
        for f in freeze {
            st.sync_a.log_bool(true, "_frozen", f);
            self.driver.timeline.frozen = f;
        }
        // 0x7791F0 (0x75B37C): the players sit down (0x778D10 for the host's
        // player; the AI fill of the seats is the staging screen's).
        let mut assign = orders::KingdomAssign { data: &*self.data, view, local: st.local_player, now: st.now.to_bits() };
        crate::staging::auto_assign_kingdom(&mut st.session, id, &mut assign)?;
        // [INF] The launch settings a staging session launches with carry
        // m_session_state 2 (the restored last settings of 0x77A000; not
        // traced): KKC_Session::Launch 0x77A950 then changes to the game.
        st.session.launch.game_type = 2;
        // The driver: the node runs and has its admin clock.
        self.driver.node.running = true;
        self.driver.node.has_admin_clock = true;
        self.driver.node.backup_since = FloatBits(self.st.now.to_bits());
        Ok(())
    }

    // ---- accessors ------------------------------------------------------

    pub fn role(&self) -> Role {
        self.st.role
    }
    pub fn session(&self) -> &Session {
        &self.st.session
    }
    /// The local SessionPlayer id (AdminNode+8).
    pub fn local_player(&self) -> Option<u32> {
        self.st.local_player
    }
    /// True while the session runs (the driver's node).
    pub fn running(&self) -> bool {
        self.driver.node.running && !self.st.left
    }
    /// Leave the session: every connection is told (type 8 from the host,
    /// type 10 from a client, 0x74BDE0 with AdminNode+0x2CD) until the delay
    /// 0xA874AC of 2.5 s ends; then [`LanEvent::Left`].
    pub fn leave(&mut self) {
        if self.st.leaving.is_none() {
            self.st.leaving = Some(self.st.now);
        }
    }
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.st.udp.local_addr()
    }
    pub fn joined(&self) -> bool {
        self.st.role == Role::Host || self.st.joined
    }
    pub fn sync_a(&self) -> &Synchronizer {
        &self.st.sync_a
    }
    /// Admin time (AdminNode+0x28C).
    pub fn admin_time(&self) -> f32 {
        self.driver.node.admin_time.as_f32()
    }
    pub fn timeline_frozen(&self) -> bool {
        self.driver.timeline.frozen
    }
    /// Game speed exponent of the timeline (`KKC_Sim+0x20`).
    pub fn timeline(&self) -> &Timeline {
        &self.driver.timeline
    }
    pub fn data(&self) -> &dyn LanData {
        &*self.data
    }
    /// Testing aid: perturb synchronizer A (a node whose admin log differs
    /// from the host's must be reported by the original sync check).
    pub fn perturb_sync_a(&mut self, value: u32) {
        let gate = true;
        self.st.sync_a.log_int(gate, "lan perturbation", value);
    }

    /// Emit a request of the local player (0x766C10 template with player 0 =
    /// the local player 0x76AC60; dropped when the node is not active). An
    /// order without a process in this port is refused here, by name.
    pub fn request(&mut self, descriptor: usize, fields: &[Field]) -> Result<bool> {
        orders::check_request(descriptor)?;
        let Some(player) = self.st.local_player else { return Ok(false) };
        let ctx = orders::DataView { data: &*self.data };
        Ok(self.admin.emit_request(true, descriptor, fields, player, &ctx)?)
    }

    /// SetGWorldSuccess / SetVWorldSuccess 0x74FEC0 / 0x74FF50 after the
    /// launch task: node+0x90 = 3 (4 on failure) and
    /// TGC_UpdateTaskStatusOrderRequest (304, 0x74E970). A node with a view
    /// sends 2 first natively (+0x8C); one report stands for both here.
    pub fn report_world_loaded(&mut self, success: bool) -> Result<()> {
        let status = if success { 3 } else { 4 };
        self.request(descriptor::TASK_STATUS_REQUEST, &[Field::Bounded { n: 5, value: status }])?;
        Ok(())
    }

    // ---- the frame -------------------------------------------------------

    fn tick_time(&mut self) -> Result<()> {
        let ms = self.st.start.elapsed().as_millis() as u32;
        self.st.ms = ms;
        self.st.now = node_seconds(ms, self.st.pc)?;
        Ok(())
    }

    /// One frame: receive, the joining client's private messages, the
    /// driver frames, the host's countdown, send.
    pub fn frame(&mut self, game: &mut dyn LanGame) -> Result<Vec<LanEvent>> {
        self.tick_time()?;
        if self.st.left {
            return Ok(std::mem::take(&mut self.st.events));
        }
        self.receive()?;
        self.private_messages()?;
        self.flush_deferred()?;
        if self.st.leaving.is_none() {
            self.run_driver(game)?;
            if self.st.role == Role::Host {
                self.update_countdown()?;
            }
            self.liveness()?;
        }
        self.send()?;
        if let Some(at) = self.st.leaving
            && self.st.now - at > LEAVE_DELAY
        {
            self.st.left = true;
            self.st.event(LanEvent::Left);
        }
        Ok(std::mem::take(&mut self.st.events))
    }

    fn roles(&self, conn: &Conn) -> Roles {
        let st = &self.st;
        let is_host_conn = st.host_addr == Some(conn.addr);
        Roles {
            we_are_host: st.role == Role::Host,
            we_are_server: st.role == Role::Host,
            conn_is_host: st.role == Role::Client && is_host_conn,
            conn_is_server: st.role == Role::Client && is_host_conn && st.server_set,
            connecting: st.connecting,
            joined: st.joined,
            backup_mode: false,
            rank: st.own_identity.join_order as i32,
            session_member: conn.admin.base.player.is_some(),
            session_state_set: st.session.state != 0,
            admin_clock: if st.role == Role::Host { true } else { st.client.clock_selected },
        }
    }

    /// The reply to a GameSpy QR2 query on the game socket (qr2 of the SDK in
    /// the original, the keys of KKC_GameSpyQR2 0x79A000's session): the type
    /// and request key, server keys as name and value strings ended by an empty
    /// string, then empty player and team lists (a 16-bit count and an empty
    /// key list each). Only a host answers.
    fn answer_queries(&mut self) -> Result<()> {
        let queries = std::mem::take(&mut self.st.udp.queries);
        if self.st.role != Role::Host {
            return Ok(());
        }
        for (to, key) in queries {
            let s = &self.st.session;
            let humans = s.players.iter().filter(|p| !p.ai && !p.film).count();
            let string = |u: &[u16]| String::from_utf16_lossy(u);
            let map = s.launch.random_map.as_deref().map_or_else(|| string(&s.launch.file), string);
            let port = self.st.udp.local_addr()?.port();
            let keys = [
                ("hostname", string(&s.options.name)),
                ("gamever", string(&self.st.host_connect.as_ref().map(|h| h.version.clone()).unwrap_or_default())),
                ("hostport", port.to_string()),
                ("mapname", map),
                ("gametype", String::new()),
                ("numplayers", humans.to_string()),
                ("maxplayers", s.options.max_players.to_string()),
                ("gamemode", if s.state == 1 { "openstaging".to_string() } else { "closedplaying".to_string() }),
                ("password", u8::from(!s.options.password.is_empty()).to_string()),
            ];
            let mut out = vec![0u8];
            out.extend_from_slice(&key);
            for (k, v) in keys {
                out.extend_from_slice(k.as_bytes());
                out.push(0);
                out.extend_from_slice(v.as_bytes());
                out.push(0);
            }
            out.push(0);
            // players, then teams: count 0, empty key list
            out.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
            self.st.udp.send_raw(to, &out)?;
        }
        Ok(())
    }

    fn receive(&mut self) -> Result<()> {
        let now = self.st.now;
        let incoming = self.st.udp.receive_all(now)?;
        self.st.udp.dropped.clear();
        self.answer_queries()?;
        for inc in incoming {
            let addr = peer_address(self.st.udp.conns[inc.conn].addr)?;
            // A client talks to its session's peers only.
            let index = match self.st.role {
                Role::Host => self.st.ensure_conn(addr)?,
                Role::Client => match self.st.conn_index(addr) {
                    Some(i) => i,
                    None => continue,
                },
            };
            // A datagram ends a reported silence: the original's network thread
            // does not stall with a frame, ours does (module documentation).
            if self.st.conns[index].timed_out {
                self.st.conns[index].timed_out = false;
                self.st.conns[index].lost_at = None;
                self.st.conns[index].admin.connected = true;
            }
            let roles = self.roles(&self.st.conns[index]);
            let st = &mut self.st;
            let admin = &mut self.admin;
            let Conn { admin: conn, file, .. } = &mut st.conns[index];
            let mut view = NodeHistories {
                requests: &mut admin.requests,
                backup_requests: &mut st.spare.backup_requests,
                decrees: &mut admin.decrees,
                backup_decrees: &mut st.spare.backup_decrees,
                backup_decree_next: &mut st.spare.backup_next,
            };
            for &(s, ok) in &inc.acks {
                conn.packet_result(&view, s, ok, file)?;
            }
            conn.net = st.udp.conns[inc.conn].seq.clone();
            let mut r = inc.body()?;
            let mut events = Vec::new();
            match (st.role, inc.packet_type) {
                (Role::Host, packets::TYPE_CONNECT) => {
                    let mut hc = st.host_connect.clone().ok_or_else(|| Error::Protocol("host without connect state".into()))?;
                    hc.session_mode = st.session.state as i32;
                    hc.launch_countdown = st.session.launch_countdown;
                    let verdict = read_and_validate_connect(conn, &mut r, &hc, addr, now.to_bits(), &mut |_| false)?;
                    match verdict {
                        ConnectVerdict::Accepted { request, .. } => {
                            // 0x74C9C0 with the local player (0x76AC60).
                            let player = st.local_player.ok_or_else(|| Error::Protocol("host without a player".into()))?;
                            let ctx = orders::DataView { data: &*self.data };
                            admin.emit_request(true, crate::session::PLAYER_JOIN_REQUEST, &request, player, &ctx)?;
                        }
                        ConnectVerdict::Rejected { info, reason, .. } => st.events.push(LanEvent::ConnectRefused { reason, info }),
                        ConnectVerdict::Ignored(_) => {}
                    }
                }
                (Role::Host, packets::TYPE_CLIENT_TO_SERVER) => {
                    events = packets::read_client_to_server(conn, &mut r, roles, inc.most_recent)?;
                }
                (Role::Host, packets::TYPE_CLIENT_DISCONNECT_REQUEST) => {
                    events = packets::read_client_disconnect_request(conn, roles);
                }
                (Role::Client, packets::TYPE_SERVER_TO_CLIENT) => {
                    events = packets::read_server_to_client(conn, &mut view, &mut r, roles, inc.most_recent, file)?;
                }
                (Role::Client, packets::TYPE_SERVER_DISCONNECT) => {
                    // Timeout 0x75A250 on the conn first: connected cleared
                    // (the fail-over to a backup server is not run).
                    events = packets::read_server_disconnect(conn, roles, &mut |c| {
                        c.connected = false;
                        Ok(())
                    })?;
                }
                (Role::Client, packets::TYPE_CLIENT_DISCONNECT_DECREE) => {
                    events = packets::read_disconnect_decree(&mut r, roles)?;
                }
                // Types 3, 6, 7, 9 are backup-server traffic: no backup server
                // is ever chosen in a LAN session of this port.
                _ => {}
            }
            for e in events {
                self.conn_event(index, e)?;
            }
        }
        Ok(())
    }

    fn conn_event(&mut self, index: usize, e: ConnEvent) -> Result<()> {
        match e {
            // 0x75A350: SetServer(conn) 0x759790 and AdminNode+0x2CC = 0.
            ConnEvent::HostBecameServer { .. } => {
                self.st.connecting = false;
                self.st.server_set = true;
            }
            // 0x74C850 -> 0x75FB40: binary32(fild(decree end - +0x20C - 1)
            // * 0.0625 + admin time).
            ConnEvent::ClientClockUpdate { decree_end } => {
                if self.st.client.clock_selected {
                    let k = decree_end.wrapping_sub(self.driver.node.processed).wrapping_sub(1);
                    let target = lobby::clock_target(k, self.driver.node.admin_time.as_f32(), self.st.precision)?;
                    let now = self.st.now;
                    self.st.client.clock.update(target, now);
                }
            }
            ConnEvent::ServerDisconnected => self.st.event(LanEvent::Disconnected { reason: None }),
            ConnEvent::DisconnectedByServer { reason, .. } => {
                // 0x74B980 stops the admin node before the reason is read.
                self.driver.node.running = false;
                self.st.event(LanEvent::Disconnected { reason: Some(reason) });
            }
            // 0x74F7C0: PlayerLeave request {conn+8, reason}.
            ConnEvent::ClientLeaveRequested { reason } => {
                if let Some(player) = self.st.conns[index].admin.base.player {
                    self.request(descriptor::PLAYER_LEAVE_REQUEST, &[Field::Player(player), Field::Bounded { n: 13, value: reason }])?;
                }
            }
            // Log lines of ignored packets, and events of the backup-server
            // packet types `receive` does not read.
            _ => {}
        }
        Ok(())
    }

    /// AdminConn::Update -> 0x7531A0 on the host connection's private
    /// history (client only).
    fn private_messages(&mut self) -> Result<()> {
        let Some(host) = self.st.host_addr else { return Ok(()) };
        let Some(index) = self.st.conn_index(host) else { return Ok(()) };
        if self.st.conns[index].admin.private_history.begin() >= self.st.conns[index].admin.private_history.end() {
            return Ok(());
        }
        self.st.client.decree_end = self.admin.decrees.end();
        let env = ClockEnv { now: self.st.now, precision: self.st.precision };
        let view = self.st.view();
        let messages = {
            let st = &mut self.st;
            let State { conns, client, session, sync_a, .. } = st;
            let conn = &mut conns[index].admin;
            let mut data = orders::DataView { data: &*self.data };
            let mut snap = orders::SnapshotEffects { deferred: Vec::new(), view };
            let out = lobby::process_private_messages(conn, client, session, sync_a, &mut data, &mut snap, &view, env)?;
            (out, snap.deferred)
        };
        let (messages, effects) = messages;
        for m in messages {
            for a in m.actions {
                self.lobby_action(a)?;
            }
            if m.kind == lobby::PRIVATE_ADMIN_STATE {
                // 0x74DF50: +0x20C, the resume pair +0x248/+0x24C, +0x250,
                // +0x28C, the client clock (+0x158 = +0x16C).
                let c = &self.st.client;
                self.admin.client.next_index = c.decree_next;
                self.admin.client.resume_byte = c.resume_byte as i32;
                self.admin.client.resume_bit = c.resume_bit as i32;
                self.admin.client.field_250 = c.resume_player.unwrap_or(0);
                self.st.own_address = c.own_address;
                self.driver.node.processed = c.decree_next;
                self.driver.node.admin_time = FloatBits(c.admin_time);
                self.driver.node.has_admin_clock = true;
                self.driver.node.running = true;
                self.st.joined = true;
                if let Some(p) = self.st.local_player {
                    self.st.event(LanEvent::Joined { player: p });
                }
            }
        }
        for e in effects {
            self.snapshot_effect(e)?;
        }
        self.st.event(LanEvent::PlayersChanged);
        Ok(())
    }

    pub(crate) fn lobby_action(&mut self, a: lobby::LobbyAction) -> Result<()> {
        use lobby::LobbyAction as A;
        match a {
            A::ConnectToPeer { address } => {
                self.st.ensure_conn(address)?;
            }
            A::BindPlayer { link, player } => match link {
                PeerLink::Node => {
                    self.st.local_player = Some(player);
                    self.st.own_ordering = Some(player);
                }
                l => {
                    if let Some(c) = self.st.conn_of_link(&l) {
                        c.admin.base.player = Some(player);
                        c.admin.base.ordering_player = Some(player);
                    }
                }
            },
            A::AddPeer { link } => {
                if !self.st.peers.contains(&link) {
                    self.st.peers.push(link);
                }
            }
            A::SetPeerIdentity { link, identity } => match link {
                PeerLink::Node => self.st.own_identity = identity,
                l => {
                    if let Some(c) = self.st.conn_of_link(&l) {
                        c.admin.base.set_identity(&identity);
                    }
                }
            },
            A::UiMessage { kind, key, args } => self.st.event(LanEvent::Message { kind, key, args, number: None }),
            A::Request { descriptor, fields } => {
                self.request(descriptor, &fields)?;
            }
            // The format text is what [+0x14] holds on this port: the reader
            // stored it, the records it seats come from `LanData::kingdom_open`.
            A::ResetSessionInfo => self.st.session.info_text.clear(),
            A::ApplySessionInfo { text } => self.st.session.info_text = text,
        }
        Ok(())
    }

    fn snapshot_effect(&mut self, e: orders::SnapshotEffect) -> Result<()> {
        match e {
            orders::SnapshotEffect::State(state) => {
                self.st.event(LanEvent::StateChanged { state });
                self.st.deferred.version_ack = true;
            }
            // SendUpdateRMCMapStatus 0x7770B0 / SendUpdateMapStatus 0x770B70:
            // TGC_SetMapTransferStatusOrderRequest (3) {status 1, file, +0x154}.
            orders::SnapshotEffect::MapStatus { file } => {
                let word = self.st.session.field_154;
                self.st.deferred.requests.push((
                    descriptor::SET_MAP_TRANSFER_STATUS_REQUEST,
                    vec![Field::Bounded { n: 4, value: 1 }, Field::Text(file), Field::Uint { width: 32, value: word }],
                ));
            }
        }
        Ok(())
    }

    /// Requests and order-version acks left by processes and the snapshot
    /// (0x766D40 writes the direct OrderVersionAckRequest without the
    /// SetPlayer guard of the ordinary emitter).
    fn flush_deferred(&mut self) -> Result<()> {
        let ack = std::mem::take(&mut self.st.deferred.version_ack);
        let requests = std::mem::take(&mut self.st.deferred.requests);
        if ack {
            aa_orders::native::write_version_ack_request(&self.admin.registry, &mut self.admin.request_stream)?;
        }
        for (descriptor, fields) in requests {
            self.request(descriptor, &fields)?;
        }
        Ok(())
    }

    /// The +0x218 decree packets the client has, contiguous from +0x20C.
    fn received_decrees(&self) -> i32 {
        let mut received = self.driver.node.processed;
        while received < self.admin.decrees.end() && self.admin.decrees.begin() <= received {
            if !matches!(self.admin.decrees.get(received), Ok(Some(_))) {
                break;
            }
            received += 1;
        }
        received
    }

    /// Driver frames (0x75CA70) until the driver waits: the frame kernel of
    /// aa-runtime over this node's owners, the timeline shared with the
    /// order processes (0x76DD90 SetFrozen, 0x76F370 SetGameSpeed and the
    /// pause run inside their decrees, as natively).
    fn run_driver(&mut self, game: &mut dyn LanGame) -> Result<()> {
        self.driver.gates.session_mode = self.st.session.state as i32;
        let game = RefCell::new(game);
        let rng = RefCell::new(Vec::new());
        let precision = self.driver.precision;
        let gates = self.driver.gates;
        let async_rate = self.driver.async_ticks_per_second;
        for _ in 0..MAX_DRIVER_STEPS {
            if !self.driver.node.running {
                break;
            }
            let received = self.received_decrees();
            self.driver.node.received = received;
            let now = FloatBits(self.st.now.to_bits());
            let action = {
                let Driver { node, timeline, counters, .. } = &mut self.driver;
                let shared = RefCell::new(timeline);
                let mut cell = TimelineCell(&shared);
                let admin_time = node.admin_time;
                let mut net = Seams {
                    admin: &mut self.admin,
                    st: &mut self.st,
                    gate: &mut self.gate,
                    game: &game,
                    rng: &rng,
                    data: &*self.data,
                    admin_time,
                    node,
                    gates: &gates,
                    async_rate,
                    timeline: &shared,
                    precision,
                };
                let mut world = WorldSide { game: &game, rng: &rng, clock: None };
                let mut kernel = sim::borrowed::Kernel {
                    sim: &mut cell,
                    control: sim::borrowed::Control { counters, stopped: &mut self.stopped },
                    precision,
                };
                kernel.frame(now, &mut net, &mut world)
            };
            let action = action.map_err(err)?;
            self.world_clock_hint = game.try_borrow().map_err(err)?.world_clock();
            if !matches!(action, Action::WorldTick | Action::AdminTick) {
                break;
            }
        }
        self.flush_deferred()
    }

    /// KKC_Session::UpdateCountdown 0x76E3D0 (host).
    fn update_countdown(&mut self) -> Result<()> {
        if self.st.session.host_countdown <= 0 {
            return Ok(());
        }
        let view = self.st.view();
        let node = orders::staging_node(&self.st, &view, None);
        let all_ready = crate::staging::all_ready(&self.st.session, &node, &Directory { st: &self.st })?;
        // 0x74B220: multiplayer session state with backup mode (never set here).
        if !all_ready {
            let mut fx = orders::StagingEffects::new(&self.st, &*self.data);
            crate::staging::abort_countdown(&mut self.st.session, &node, &mut fx)?;
            let reqs = fx.take_requests();
            for (d, f) in reqs {
                self.request(d, &f)?;
            }
            return Ok(());
        }
        let started = self.st.countdown_started.ok_or_else(|| Error::Protocol("0x76E3D0 countdown without its timer +0x110".into()))?;
        // 0x40AFE0: f32(ms) * 0.001, _ftol.
        let elapsed = (self.st.ms.wrapping_sub(started) as f32 * 0.001) as i32;
        let time = self.data.countdown_time();
        if time - elapsed == self.st.session.host_countdown {
            return Ok(());
        }
        self.st.session.host_countdown -= 1;
        if self.st.session.host_countdown > 0 {
            // 0x76CBF0(value, player 0).
            let value = self.st.session.host_countdown;
            self.request(crate::staging::LAUNCH_COUNTDOWN_PROGRESS_REQUEST, &[Field::Uint { width: 32, value: value as u32 }])?;
        } else {
            // 0x40DF00(0): TGC_LaunchOrderRequest.
            self.request(descriptor::LAUNCH_REQUEST, &[])?;
        }
        Ok(())
    }

    /// AdminConn::Update 0x75BD70: a connection silent past its limit is no
    /// longer connected (0x75A250). The host then drops its player when the
    /// patience of 15 s (+0x44) ran out (0x75A2C0 with reason 4; the vote
    /// dialog that asks for it first is not built); a client whose host stays
    /// silent that long has lost the session (no backup server takes over).
    fn liveness(&mut self) -> Result<()> {
        let now = self.st.now;
        let host_addr = self.st.host_addr;
        let mut timed = Vec::new();
        let mut lost = Vec::new();
        for c in &mut self.st.conns {
            if c.admin.net.highest < 0 {
                continue;
            }
            if c.timed_out {
                if c.lost_at.is_some_and(|at| now - at > LOST_PATIENCE) {
                    c.lost_at = None;
                    lost.push((c.addr, c.admin.base.player));
                }
                continue;
            }
            if !c.admin.connected {
                continue;
            }
            let limit = if Some(c.addr) == host_addr { SERVER_TIMEOUT } else { PEER_TIMEOUT };
            if now - c.admin.net.last_advance > limit {
                c.timed_out = true;
                c.lost_at = Some(now);
                c.admin.connected = false;
                timed.push(c.addr);
            }
        }
        for address in timed {
            self.st.event(LanEvent::PeerTimedOut { address });
        }
        for (address, player) in lost {
            if self.st.role == Role::Client {
                if Some(address) == host_addr {
                    self.st.event(LanEvent::Disconnected { reason: None });
                }
            } else if let Some(player) = player {
                self.request(descriptor::PLAYER_LEAVE_REQUEST, &[Field::Player(player), Field::Bounded { n: 13, value: 4 }])?;
            }
        }
        Ok(())
    }

    /// SendPackets 0x764FA0: ask the gate 0x74BDE0 of every connection until
    /// it answers 0 and send what it asks for.
    fn send(&mut self) -> Result<()> {
        let now = self.st.now;
        self.st.udp.begin_pass();
        for nc in 0..self.st.udp.conns.len() {
            let addr = peer_address(self.st.udp.conns[nc].addr)?;
            let Some(index) = self.st.conn_index(addr) else { continue };
            for _ in 0..64 {
                let roles = self.roles(&self.st.conns[index]);
                let st = &mut self.st;
                let seq = &st.udp.conns[nc].seq;
                let is_host_conn = st.host_addr == Some(addr);
                let gn = GateNode {
                    connecting: st.connecting,
                    we_are_host: st.role == Role::Host,
                    we_are_server: st.role == Role::Host,
                    leaving: st.leaving.is_some(),
                    leave_now: false,
                    rank: st.own_identity.join_order as i32,
                    admin_clock: if st.role == Role::Host { true } else { st.client.clock_selected },
                    backup_since: 0,
                };
                let conn = &st.conns[index];
                let gc = GateConn {
                    is_host: st.role == Role::Client && is_host_conn,
                    is_server: st.role == Role::Client && is_host_conn && st.server_set,
                    disconnecting: conn.admin.disconnecting,
                    connected: conn.admin.connected,
                    rank: conn.admin.base.join_order,
                    player: conn.admin.base.player.is_some(),
                    sent_this_pass: seq.sent_this_pass,
                    last_send: seq.last_send.to_bits(),
                };
                let admin = &mut self.admin;
                let Conn { admin: aconn, file, .. } = &mut st.conns[index];
                let view = NodeHistories {
                    requests: &mut admin.requests,
                    backup_requests: &mut st.spare.backup_requests,
                    decrees: &mut admin.decrees,
                    backup_decrees: &mut st.spare.backup_decrees,
                    backup_decree_next: &mut st.spare.backup_next,
                };
                let avars = st.avars;
                let we_are_host = st.role == Role::Host;
                let wanted = {
                    let mut backlog = || aconn.backlogged(&view, we_are_host, &avars, crate::NativeImage::V14, 0, file);
                    wanted_packet_type(&gn, &gc, now.to_bits(), &mut backlog, st.precision)?
                };
                let udp = &mut st.udp;
                match wanted.packet_type {
                    0 => break,
                    packets::TYPE_CONNECT => {
                        let info = st.connect.clone().ok_or_else(|| Error::Protocol("connect without info".into()))?;
                        udp.send(nc, packets::TYPE_CONNECT, now, &mut |w, _| write_connect(w, &info))?;
                    }
                    packets::TYPE_CLIENT_TO_SERVER => {
                        let status = aconn.status_inputs(&view, true, false, &avars);
                        udp.send(nc, packets::TYPE_CLIENT_TO_SERVER, now, &mut |w, s| packets::write_client_to_server(aconn, &view, w, s, status))?;
                    }
                    packets::TYPE_SERVER_TO_CLIENT => {
                        udp.send(nc, packets::TYPE_SERVER_TO_CLIENT, now, &mut |w, s| {
                            packets::write_server_to_client(aconn, &view, w, s, roles, crate::NativeImage::V14, file)
                        })?;
                    }
                    packets::TYPE_CLIENT_DISCONNECT_DECREE => {
                        udp.send(nc, packets::TYPE_CLIENT_DISCONNECT_DECREE, now, &mut |w, _| packets::write_disconnect_decree(aconn, w))?;
                    }
                    packets::TYPE_SERVER_DISCONNECT | packets::TYPE_CLIENT_DISCONNECT_REQUEST => {
                        let t = wanted.packet_type;
                        udp.send(nc, t, now, &mut |_, _| Ok(()))?;
                    }
                    _ => break,
                }
            }
        }
        Ok(())
    }

    // ---- host staging requests ------------------------------------------

    /// The host's StagingLaunch (0x65DE80 -> 0x651950): with more than one
    /// human, TGC_StartLaunchCountdownOrderRequest (96, 0x6518A0), else the
    /// launch request 0x40DF00 directly.
    pub fn start_launch(&mut self) -> Result<()> {
        if self.st.session.multiplayer && self.st.session.human_count() > 1 {
            self.request(descriptor::START_COUNTDOWN_REQUEST, &[])?;
        } else {
            self.request(descriptor::LAUNCH_REQUEST, &[])?;
        }
        Ok(())
    }
}

use aa_orders::commands::WireWriter;

/// The fields of TGC_SetSessionLoadMapOrder (91 / 93) from a file, its header
/// (0x5A1E90, the codec of [`lobby::GameSettings`]) and the crc: the payload
/// written by that codec and read back by the order's schema.
pub fn load_map_fields(file: &[u16], game: &lobby::GameSettings, crc: u32, data: &dyn LanData) -> Result<Vec<Field>> {
    let view = orders::DataView { data };
    let mut w = WireWriter::new(aa_orders::Position::PACKET_START)?;
    crate::session::write_text(&mut w, file)?;
    game.write(&mut w, &view)?;
    w.write_uint(crc, 32)?;
    let bytes = aa_orders::net::transport::finalize(&w)?;
    let mut r = aa_orders::Cursor::for_packet(&bytes, None)?;
    Ok(aa_orders::payload::decode(descriptor::SET_SESSION_LOAD_MAP_REQUEST, &mut r, &view)?)
}

/// The reverse of [`load_map_fields`] for the decree's process.
pub(crate) fn read_load_map_fields(fields: &[Field], data: &dyn LanData, previous: &lobby::GameSettings) -> Result<(Vec<u16>, lobby::GameSettings, u32)> {
    let mut view = orders::DataView { data };
    let mut w = WireWriter::new(aa_orders::Position::PACKET_START)?;
    aa_orders::payload::encode(descriptor::SET_SESSION_LOAD_MAP_DECREE, fields, &mut w, &view)?;
    let bytes = aa_orders::net::transport::finalize(&w)?;
    let mut r = aa_orders::Cursor::for_packet(&bytes, None)?;
    let file = crate::session::read_text(&mut r)?;
    let mut game = previous.clone();
    game.read(&mut r, &mut view)?;
    let crc = r.read_uint(32)?;
    Ok((file, game, crc))
}

/// The world seams of the driver (0x4F9970 through [`LanGame`]).
struct WorldSide<'a, 'g> {
    game: &'a RefCell<&'g mut dyn LanGame>,
    rng: &'a RefCell<Vec<u32>>,
    /// A clock to answer for when no live game is borrowed (SetFrozen
    /// outside a frame).
    clock: Option<NativeClock>,
}
impl sim::WorldSeams for WorldSide<'_, '_> {
    fn world_clock(&self) -> Option<NativeClock> {
        self.game.try_borrow().ok().and_then(|g| g.world_clock()).or(self.clock)
    }
    fn world_update(&mut self) -> sim::Result<NativeClock> {
        let (clock, rng) = self.game.try_borrow_mut().map_err(sim_err)?.world_update().map_err(sim_err)?;
        self.rng.try_borrow_mut().map_err(sim_err)?.extend(rng);
        Ok(clock)
    }
}

/// The timeline (`KKC_Sim` 0xB36554) shared by the frame kernel and the
/// order processes of its admin tick.
pub(crate) type SharedTimeline<'a, 't> = &'a RefCell<&'t mut Timeline>;

/// [`sim::borrowed::SimFields`] over the shared timeline: every access is a
/// short borrow, so a process of the same admin tick can change it.
struct TimelineCell<'a, 't>(SharedTimeline<'a, 't>);
impl sim::borrowed::SimFields for TimelineCell<'_, '_> {
    fn frozen(&self) -> bool {
        self.0.borrow().frozen
    }
    fn paused(&self) -> bool {
        self.0.borrow().paused
    }
    fn time(&self, field: sim::borrowed::TimeWord) -> sim::Result<FloatBits> {
        sim::borrowed::SimFields::time(&**self.0.try_borrow().map_err(sim_err)?, field)
    }
    fn store_time(&mut self, field: sim::borrowed::TimeWord, value: FloatBits) -> sim::Result<()> {
        sim::borrowed::SimFields::store_time(&mut **self.0.try_borrow_mut().map_err(sim_err)?, field, value)
    }
    fn game_per_admin_tick(&self) -> sim::Result<FloatBits> {
        Ok(self.0.try_borrow().map_err(sim_err)?.game_per_admin_tick)
    }
    fn avi_frames(&self) -> i32 {
        self.0.borrow().avi_frames
    }
    fn store_avi_frames(&mut self, value: i32) {
        self.0.borrow_mut().avi_frames = value;
    }
    fn store_frozen(&mut self, value: bool) {
        self.0.borrow_mut().frozen = value;
    }
    fn store_maximum(&mut self, value: FloatBits) {
        self.0.borrow_mut().max_speed = value;
    }
}

/// The real admin clock (AdminNode+0x158 vfunc +8 with argument 1).
pub(crate) fn real_admin_clock(st: &mut State) -> Result<FloatBits> {
    match st.role {
        // AdminNode+0x158 = the server clock +0x15C: 0x75FCB0(cached).
        Role::Host => {
            let mut clock = NodeClock { now: st.now, ms: st.ms };
            st.server_clock.bounded_time(true, &mut clock, st.pc).map_err(err)
        }
        // +0x158 = the client clock +0x16C: vf+8 0x75FD70 -> 0x75FCF0(1):
        // binary32(NetNode+0x58 - +8).
        Role::Client => {
            let offset = f32::from_bits(st.client.clock.offset);
            Ok(FloatBits((st.now - offset).to_bits()))
        }
    }
}

/// The network seams of the driver over the node's owners.
pub(crate) struct Seams<'a, 'g, 't> {
    pub admin: &'a mut AdminNode,
    pub st: &'a mut State,
    pub gate: &'a mut Gate,
    pub game: &'a RefCell<&'g mut dyn LanGame>,
    pub rng: &'a RefCell<Vec<u32>>,
    pub data: &'a dyn LanData,
    /// AdminNode+0x28C at the start of this frame (one admin tick at most
    /// per frame, so the value inside it).
    pub admin_time: FloatBits,
    /// The driver's node fields (+0x28C, +0x2C0, +0x20C...), gates and the
    /// async heartbeat rate.
    pub node: &'a mut sim::AdminNode,
    pub gates: &'a Gates,
    pub async_rate: FloatBits,
    pub timeline: SharedTimeline<'a, 't>,
    pub precision: Precision,
}
impl Seams<'_, '_, '_> {
    fn emit(&mut self, descriptor: usize, fields: &[Field]) -> sim::Result<()> {
        let Some(player) = self.st.local_player else { return Ok(()) };
        let ctx = orders::DataView { data: self.data };
        self.admin.emit_request(true, descriptor, fields, player, &ctx).map_err(sim_err)?;
        Ok(())
    }
}
/// The frame kernel's node reads (the Laboratory adapter of aa-runtime over
/// this node's fields).
impl sim::borrowed::Host for Seams<'_, '_, '_> {
    fn begin_world_duration(&mut self, _: Precision) -> sim::Result<Option<u32>> {
        // 0x752AE0's duration history (+0x238) is not kept here (nothing in
        // the frame reads it).
        Ok(None)
    }
    fn finish_world_duration(&mut self, _: u32, _: Precision) -> sim::Result<()> {
        Ok(())
    }
    fn role(&self) -> sim::Result<DriverRole> {
        Ok(self.node.role)
    }
    fn flag(&self, f: sim::borrowed::NodeFlag) -> sim::Result<bool> {
        use sim::borrowed::NodeFlag as F;
        Ok(match f {
            F::Running => self.node.running,
            F::HasAdminClock => self.node.has_admin_clock,
            F::Disconnecting => self.node.disconnecting,
            F::BackupMode => self.node.backup_mode,
            F::ReplayRecorder => self.node.replay_recorder,
            F::ReplaySource => self.node.replay_source,
            F::IsServer => self.node.is_server,
        })
    }
    fn word(&self, f: sim::borrowed::NodeWord) -> sim::Result<FloatBits> {
        use sim::borrowed::NodeWord as W;
        Ok(match f {
            W::AdminTime => self.node.admin_time,
            W::DisconnectStarted => self.node.disconnect_started,
            W::BackupSince => self.node.backup_since,
            W::HeartbeatAt => self.node.heartbeat_at,
        })
    }
    fn store_flag(&mut self, f: sim::borrowed::NodeFlag, v: bool) -> sim::Result<()> {
        use sim::borrowed::NodeFlag as F;
        match f {
            F::Running => self.node.running = v,
            F::HasAdminClock => self.node.has_admin_clock = v,
            F::Disconnecting => self.node.disconnecting = v,
            F::BackupMode => self.node.backup_mode = v,
            F::ReplayRecorder => self.node.replay_recorder = v,
            F::ReplaySource => self.node.replay_source = v,
            F::IsServer => self.node.is_server = v,
        };
        Ok(())
    }
    fn store_word(&mut self, f: sim::borrowed::NodeWord, v: FloatBits) -> sim::Result<()> {
        use sim::borrowed::NodeWord as W;
        match f {
            W::AdminTime => self.node.admin_time = v,
            W::DisconnectStarted => self.node.disconnect_started = v,
            W::BackupSince => self.node.backup_since = v,
            W::HeartbeatAt => self.node.heartbeat_at = v,
        };
        Ok(())
    }
    fn processed(&self) -> sim::Result<i32> {
        Ok(self.node.processed)
    }
    fn received(&self) -> sim::Result<i32> {
        Ok(self.node.received)
    }
    fn gate_flag(&self, f: sim::borrowed::GateFlag) -> sim::Result<bool> {
        use sim::borrowed::GateFlag as G;
        Ok(match f {
            G::SessionF4 => self.gates.session_f4,
            G::SinglePlayerAutoPause => self.gates.single_player_auto_pause,
            G::RenderDumpAvi => self.gates.render_dump_avi,
        })
    }
    fn gate_word(&self, f: sim::borrowed::GateWord) -> sim::Result<u32> {
        use sim::borrowed::GateWord as G;
        Ok(match f {
            G::SessionMode => self.gates.session_mode as u32,
            G::FlagsB396C4 => self.gates.flags_b396c4,
            G::AviFramesPerTick => self.gates.avi_frames_per_tick as u32,
        })
    }
    fn async_ticks_per_second(&self) -> sim::Result<FloatBits> {
        Ok(self.async_rate)
    }
    fn replay_client_tick(&mut self) -> sim::Result<()> {
        // 0x74FD10 on decree packet +0x20C, then +0x20C += 1.
        let index = self.node.processed;
        orders::client_replay(self, index).map_err(sim_err)?;
        self.node.processed = self.node.processed.wrapping_add(1);
        Ok(())
    }
}
impl sim::NetworkSeams for Seams<'_, '_, '_> {
    fn net_pump(&mut self) -> sim::Result<()> {
        // 0x776540: LanNode::frame received before the driver runs and sends
        // after it.
        Ok(())
    }
    fn process_commands(&mut self) -> sim::Result<()> {
        // 0x75C9A0: no console commands reach a LAN session.
        Ok(())
    }
    fn real_admin_clock(&mut self) -> sim::Result<FloatBits> {
        real_admin_clock(self.st).map_err(sim_err)
    }
    fn avi_pump(&mut self) -> sim::Result<i32> {
        Ok(0)
    }
    fn sync_log(&mut self, entry: sim::SyncLog) -> sim::Result<()> {
        // Routing of aa_orders::net::seams::NetworkCore::tick_log: the second
        // GetGameTimeBeginTick goes to S, everything else to A.
        let g = true;
        let a = &mut self.st.sync_a;
        match entry {
            sim::SyncLog::NextTickEventAdminTime(v) => a.log_float_bits(g, "GetNextTickEventAdminTime()", v.0),
            sim::SyncLog::GameTimeBeginTick { second_log: false, value } => a.log_float_bits(g, "g_gworld->GetGameTimeBeginTick()", value.0),
            sim::SyncLog::GameTimeBeginTick { second_log: true, value } => {
                self.st.sync_s.log_float_bits(g, "g_gworld->GetGameTimeBeginTick()", value.0)
            }
            sim::SyncLog::IsFrozen(b) => a.log_bool(g, "IsFrozen()", b),
            sim::SyncLog::AdminTickEndGameTime(v) => a.log_float_bits(g, "m_admin_tick_end_game_time", v.0),
            sim::SyncLog::AdminTickEndAdminTime(v) => a.log_float_bits(g, "m_admin_tick_end_admin_time", v.0),
            sim::SyncLog::GameTimePerTick(v) => a.log_float_bits(g, "m_game_time_per_tick", v.0),
            sim::SyncLog::Frozen(b) => a.log_bool(g, "_frozen", b),
        }
        Ok(())
    }
    fn replay_record(&mut self) -> sim::Result<()> {
        Err(sim::Error::Unsupported("film recorder (AdminNode+0x268) in a LAN session"))
    }
    fn replay_playback(&mut self) -> sim::Result<()> {
        Err(sim::Error::Unsupported("film source (AdminNode+0x26C) in a LAN session"))
    }
    fn timing_smoothing(&mut self) -> sim::Result<()> {
        // 0x41C180: presentation timing, not ported (module documentation).
        Ok(())
    }
    fn end_condition_check(&mut self) -> sim::Result<()> {
        // The check 0x76F600 itself is not ported (module documentation). This
        // is where the world update ended: GetRNG 0x4F02B0 logged every draw
        // of it to A, in draw order.
        let draws = std::mem::take(&mut *self.rng.try_borrow_mut().map_err(sim_err)?);
        for v in draws {
            self.st.sync_a.log_int(true, "(int)m_rng.GetSeed()", v);
        }
        Ok(())
    }
    fn client_sync_flush(&mut self) -> sim::Result<()> {
        // 0x74E8B0 TGC_SetSyncValueOrderRequest {A last index, A hash}.
        if !self.st.sync_a.enabled {
            return Ok(());
        }
        let fields = [
            Field::Uint { width: 32, value: self.st.sync_a.end().wrapping_sub(1) as u32 },
            Field::Uint { width: 32, value: self.st.sync_a.hash as u32 },
        ];
        self.emit(descriptor::SYNC_VALUE_REQUEST, &fields)
    }
    fn host_sync_flush(&mut self) -> sim::Result<()> {
        // 0x74E670 KKC_SetSimSyncValueOrderRequest {S last index, S hash}.
        if !self.st.sync_s.enabled {
            return Ok(());
        }
        let fields = [
            Field::Uint { width: 32, value: self.st.sync_s.end().wrapping_sub(1) as u32 },
            Field::Uint { width: 32, value: self.st.sync_s.hash as u32 },
        ];
        self.emit(descriptor::SIM_SYNC_REQUEST, &fields)
    }
    fn admin_history(&mut self) -> sim::Result<()> {
        self.admin.seal_requests().map(drop).map_err(sim_err)
    }
    fn host_rate(&mut self, seconds: FloatBits) -> sim::Result<()> {
        let mut clock = NodeClock { now: self.st.now, ms: self.st.ms };
        self.st.server_clock.host_rate(seconds.0, &mut clock, self.st.pc).map_err(sim_err)
    }
    fn host_execute_requests(&mut self) -> sim::Result<()> {
        orders::host_execute(self).map_err(sim_err)
    }
    fn host_record_history(&mut self) -> sim::Result<()> {
        orders::host_record(self).map_err(sim_err)
    }
    fn client_replay_history(&mut self, index: i32) -> sim::Result<()> {
        orders::client_replay(self, index).map_err(sim_err)
    }
    fn heartbeat(&mut self) -> sim::Result<()> {
        // 0x75B820 is not run (module documentation): LanNode::liveness reports.
        Ok(())
    }
    fn backup_clients(&mut self) -> sim::Result<Vec<sim::BackupClient>> {
        Ok(Vec::new())
    }
    fn backup_mode_entered(&mut self) -> sim::Result<()> {
        Ok(())
    }
}
