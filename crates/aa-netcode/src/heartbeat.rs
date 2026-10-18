//! TGC_AdminNode async heartbeat and TGC_AdminConn::Update (network part),
//! ported from the original AA.exe. Oracle: research/phase8-net/
//! native_heartbeat.py -> native-heartbeat.json, replayed by
//! tests/heartbeat.rs.
//!
//! Ported functions (native order is kept everywhere):
//! * 0x75B820 [`AdminNode::heartbeat`] (called every 1/AsyncTicksPerSecond s by
//!   the frame update 0x75CA70, already scheduled in aa-runtime):
//!   - server only: 0x7593E0 backup requests of every peer (chat 0x753940 ->
//!     chat decree 0x74FA20, vote 0x74F890), then 0x75A2C0
//!     DisconnectLostPlayers (helpers 0x74DE10, 0x74DED0, 0x74DD50, 0x74DD20,
//!     0x74DD80; action 0x759530 with 0x74E970 TaskStatus request 304,
//!     0x750CB0 PlayerLeave decree 309, GameSpy 0x791840, reverse decree
//!     transfer 0x7501D0);
//!   - global counter DAT_00B40934: on the server with counter >= 16 (the 17th
//!     tick) 0x74F1A0 lag refresh and 0x752C60 status decree, counter = 0;
//!     otherwise counter += 1 (every node);
//!   - 0x75A0C0 ProcessBackupDecrees (status 0x74E5C0/0x74C590, chat 0x756E00);
//!   - host only: 0x74F310 UpdateFileTransferStatus for every connection
//!     (UpdateMapStatus request 305 via 0x74EA20/0x74CC20).
//! * 0x74F1A0 [`AdminNode::refresh_lag_state`] with every peer vtable slot
//!   (AdminNode 0xA913B0: 0x5AA2D0, 0x74B270, 0x74B2A0, 0x74B2B0, 0x74B2D0,
//!   0x74B2F0, 0x74B300; AdminConn 0xA8D964: 0x74B260, 0x74B280 + 0x7642D0,
//!   0x74D490, 0x74B2C0, 0x74B2E0, 0x74B2F0, 0x74D5B0). Its flags are the
//!   inputs of the backup-mode decision 0x759680 ([`AdminNode::backup_clients`]).
//! * 0x75BD70 [`AdminNode::update_conn`]: dt clamp, patience, disconnecting
//!   reset (0x752110 + 0x7514F0), liveness timeouts -> 0x75A250 Timeout,
//!   private decrees 0x7531A0 (effect).
//! * 0x759790 [`AdminNode::set_server`] (0x752300 backup channel binding),
//!   0x757010 [`AdminNode::set_host`] (0x752490 host channel binding).
//! * Client senders 0x753410 [`AdminNode::send_backup_chat`] and 0x7532D0
//!   [`AdminNode::send_backup_vote`] (writers 0x74F950 / 0x74C3D0).
//! * 0x750CB0 sequence [`emit_player_leave_decree`] for the integrator.
//!
//! Every external effect is passed in native order to [`Seams::effect`] with
//! the node, so an integrator can execute it inline exactly where the
//! original does (this matters for the PlayerLeave decree: its process
//! 0x75DAA0 -> 0x755D70 -> 0x755840 changes connections that the 0x759530 loop
//! reads afterwards). [`Recorder`] only records.
//!
//! x87: every float statement takes [`X87Precision`] explicitly. Statements
//! that are a single operation on binary32 operands followed by a binary32
//! store are precision independent (double rounding is innocuous for p >=
//! 2*24+2); the patience increase (two operations), the in-register
//! comparisons of 0x75BD70 and the received fraction (integer operands) are
//! computed exactly for PC24, PC53 and PC64. NaN results follow the x87
//! rules (quieted operand, larger significand, default NaN 0xFFC00000).
//!
//! Native caps found here (none is a size limit; all kept as protocol rules
//! or removed): static backup decree / request build streams 0xB408DC,
//! 0xB408E4, 0xB408F4, 0xB408FC start at 0x400 bytes (0x767110(0x400)) and
//! grow -> WireWriter; backup decree history keeps end-200 (0x753AF0,
//! 0x752CC4) and the own backup request history end-1000 (0x7533D2,
//! 0x7534D4): protocol rules, kept; history rings 0x100/0x400 initial
//! (0x752A60) grow -> aa_orders::History.
use aa_orders::commands::WireWriter;
use aa_orders::flow::{self, Log, OrderHost, SyncEntry};
use aa_orders::net::admin::AdminNode as OrderNode;
use aa_orders::net::transport::{finalize, X87Precision};
use aa_orders::payload::{self, Field, SchemaContext};
use aa_orders::{Cursor, History, Packet, Position};
use aa_runtime::simulation::BackupClient;

use crate::{Error, Result};

/// 0x75B867: the lag refresh runs when the counter is >= 16 (signed), so on
/// every 17th async tick of a server.
pub const REFRESH_COUNTER: i32 = 16;
/// 0xA8C090: patience maximum and reset value (0x41700000).
pub use crate::packets::PATIENCE_MAX;
/// 0xA8C0BC: patience gain per second without lag flags, and the
/// DisconnectLostPlayers patience threshold (0x75A32F).
pub const PATIENCE_RATE: f32 = 0.1;
/// 0xA8C09C: AdminConn::Update dt clamp; also the numerator of the admin
/// tick period in SetHost (0x75718B).
pub const DT_CLAMP: f32 = 1.0;
/// 0x9FDD44: disconnecting reset delay and liveness timeout of every
/// connection except the upstream server.
pub const PEER_TIMEOUT: f32 = 2.0;
/// 0xA32690: liveness timeout of the upstream server connection.
pub const SERVER_TIMEOUT: f32 = 3.0;
/// 0x753AF0 / 0x752CC4: backup decree history keeps sequences >= end-200.
pub const BACKUP_DECREE_RETENTION: i32 = 200;
/// 0x7533D2 / 0x7534D4: own backup request history keeps end-1000.
pub const BACKUP_REQUEST_RETENTION: i32 = 1000;
/// 0x752133: disconnect reason after a reset (13 = none, the 4-bit range).
pub const NO_DISCONNECT_REASON: u32 = 13;
/// 0x75A2E6 / 0x75A33C: PlayerLeave reasons used by DisconnectLostPlayers.
pub const REASON_KICKED: u32 = 4;
pub const REASON_QUIT: u32 = 5;
/// Registry ordinals (phase4 registration, descriptor objects 0xB40980,
/// 0xB4098C, 0xB409BC).
pub const TASK_STATUS_REQUEST: usize = 304;
pub const MAP_STATUS_REQUEST: usize = 305;
pub const PLAYER_LEAVE_DECREE: usize = 309;
/// x87 real indefinite (default NaN of an invalid operation).
const X87_INDEFINITE: u32 = 0xFFC0_0000;

/// AVar configuration read by these functions, with the native defaults.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    /// NetworkAdminAsyncTicksPerSecond (AVar 0x47 at [0xB39990]+0x11C,
    /// registered 0x40371A, default 16.0). Used by the scheduler 0x75CCA1.
    pub async_ticks_per_second: f32,
    /// NetworkBroadcastDecreeBehindThreshold (AVar 0x4B, [0xB39990]+0x12C,
    /// registered 0x40382C, default 32), read by 0x74D4FE.
    pub broadcast_decree_behind: i32,
    /// NetworkBackupDecreeBehindThreshold (AVar 0x4F, [0xB39990]+0x13C,
    /// registered 0x403938, default 64), read by 0x74D4A5.
    pub backup_decree_behind: i32,
    /// AdminTicksPerSecond ([0xB39AB0]+0x20, registered 0x5069F9, default
    /// 16.0), read by SetHost 0x757185.
    pub admin_ticks_per_second: f32,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            async_ticks_per_second: 16.0,
            broadcast_decree_behind: 32,
            backup_decree_behind: 64,
            admin_ticks_per_second: 16.0,
        }
    }
}

/// An entry of the AdminNode+0x2AC peer list: the node itself (vtable +4
/// returns 0, 0x4C9FA0) or one of its connections (0x8D6180 returns this).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PeerRef {
    Node,
    Conn(usize),
}

pub use crate::packets::PeerBase;

/// One TGC_AdminConn type for the packet layer and these functions.
pub use crate::packets::AdminConn;

/// TGC_AdminNode fields used here.
pub struct AdminNode {
    pub base: PeerBase,
    /// +0x2AC peer list (the node itself is normally first).
    pub list: Vec<PeerRef>,
    pub conns: Vec<AdminConn>,
    /// +0x90 task status (0x74FEC0 / 0x74FF50), sent by 0x74E970.
    pub task_status: u32,
    /// +0x158 admin clock present, and whether it is the own clock +0x15C.
    pub has_admin_clock: bool,
    pub admin_clock_own: bool,
    /// +0x224 backup decree history, +0x234 next backup decree to process
    /// (-1: start at the first one available).
    pub backup_decrees: History,
    pub next_backup_decree: i32,
    /// +0x2B8 votes and +0x2BC maximum patience of lagging connections.
    pub vote_count: i32,
    pub max_patience: f32,
    /// +0x2C4 host and +0x2C8 (upstream) server.
    pub host: Option<PeerRef>,
    pub server: Option<PeerRef>,
    /// +0x2E0 set to 1 when this node took over as host (0x75955D).
    pub took_over: i32,
    /// Global DAT_00B40934 async tick counter.
    pub async_counter: i32,
    /// +0x8C byte cleared by ClearAllTasks 0x74DAA0.
    pub field_8c: u8,
    /// +0x2A0 / +0x2A4: ClientLeave moves a leaving +0x2A0 peer to +0x2A4.
    pub field_2a0: Option<PeerRef>,
    pub field_2a4: Option<PeerRef>,
    /// +0x2B4 set when the +0x2AC peer list changes.
    pub peers_changed: bool,
    /// Successor each peer had when 0x4F1430 last unlinked it from +0x2AC:
    /// the freed list node keeps its link (the engine pool free 0x85C2B0
    /// only clears a bitmap bit) and the vote-kick sweep 0x74F160 reads it
    /// after the ClientLeave that removed the node.
    pub removed_links: Vec<(PeerRef, Option<PeerRef>)>,
}

/// Inputs that are not AdminNode state.
pub struct Env<'a> {
    /// NetNode+0x58 (binary32 seconds).
    pub now: f32,
    /// NetNode+0x64 byte (this node falls behind).
    pub falling_behind: u8,
    /// Session +0xE4 state (1 = staging).
    pub session_state: i32,
    /// [0xB3654C]+0xBC and [0xB40C9C]+0xB8 (GameSpy connection checks).
    pub gamespy_verify: bool,
    /// Value returned by the old admin clock virtual (AdminNode+0x16C, +8)
    /// after `fstp dword`; read only when a node becomes host with a previous host.
    pub old_clock_time: f32,
    pub config: Config,
    pub precision: X87Precision,
    /// AdminNode+0x214 decree history (aa_orders::net::admin::AdminNode::decrees).
    pub decrees: &'a mut History,
    /// 0x76B8F0 live SessionPlayer lookup (id and film byte 0).
    pub players: &'a dyn Fn(u32) -> bool,
}

/// AdminConn channel objects (ctor 0x756BC0) by offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    BroadcastDecreeSender,
    BroadcastDecreeReceiver,
    ReverseDecreeSender,
    ReverseDecreeReceiver,
    BroadcastRequestSender,
    BroadcastRequestReceiver,
    PrivateDecreeSender,
    PrivateDecreeReceiver,
    BackupDecreeSender,
    BackupDecreeReceiver,
    BackupRequestSender,
    BackupRequestReceiver,
    FileSender,
    FileReceiver,
}
impl Channel {
    pub fn offset(self) -> u32 {
        match self {
            Self::BroadcastDecreeSender => 0xCC,
            Self::BroadcastDecreeReceiver => 0x114,
            Self::ReverseDecreeSender => 0x144,
            Self::ReverseDecreeReceiver => 0x18C,
            Self::BroadcastRequestSender => 0x1BC,
            Self::BroadcastRequestReceiver => 0x204,
            Self::PrivateDecreeSender => 0x234,
            Self::PrivateDecreeReceiver => 0x27C,
            Self::BackupDecreeSender => 0x2AC,
            Self::BackupDecreeReceiver => 0x2F4,
            Self::BackupRequestSender => 0x324,
            Self::BackupRequestReceiver => 0x36C,
            Self::FileSender => 0x39C,
            Self::FileReceiver => 0x3E8,
        }
    }
}

/// Histories a channel is bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryRef {
    /// AdminNode+0x214.
    NodeDecrees,
    /// AdminNode+0x224.
    NodeBackupDecrees,
    /// AdminConn+0xC, +0x1C, +0xAC, +0xBC.
    ConnRequests(usize),
    ConnBackupRequests(usize),
    ConnPrivateDecrees(usize),
    ConnReverseDecrees(usize),
}

/// Channel object calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelOp {
    /// 0x763600 MessageHistorySender bind (history, first index).
    SenderBind { history: HistoryRef, start: i32 },
    /// 0x762B20 sender reset.
    SenderReset,
    /// 0x763080 receiver reset.
    ReceiverReset,
    /// 0x763130 MessageHistoryReceiver bind.
    ReceiverBind { history: HistoryRef },
    /// 0x761400 FileSender reset, 0x761500 FileReceiver reset.
    FileSenderReset,
    FileReceiverReset,
}

/// External effects, passed to [`Seams::effect`] in native order.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Request emitter template (0x74E970, 0x74EA20) with emitter argument 0:
    /// gate 0x766CD0, issuing player = local player 0x76AC60 (0x766C10
    /// SetPlayer), 9-bit descriptor 0x766D00 and payload into the request
    /// stream (aa_orders::net::admin::AdminNode::emit_request).
    Request { descriptor: usize, fields: Vec<Field> },
    /// Host decree written and executed at once (0x750CB0); see
    /// [`emit_player_leave_decree`].
    Decree { descriptor: usize, fields: Vec<Field> },
    /// 0x756E00: "public_chat|ss" (player display name 0x7667D0, text) shown
    /// with 0x40BC50(&DAT_00B399B0, 0xF).
    PublicChat { player: u32, text: Vec<u16> },
    /// 0x757010: "networking_new_host|s" with the new host's player name.
    NewHostMessage { player: Option<u32> },
    Channel { conn: usize, channel: Channel, op: ChannelOp },
    /// 0x7514F0: 0x77B8D0 on AdminPeer+0x74 (the +0x64 list is cleared here).
    PeerSubobjectReset { peer: PeerRef },
    /// 0x74DBC0(1) ResetOrderVersion.
    ResetOrderVersion,
    /// 0x75FBF0 on AdminNode+0x15C (period, start); +0x158 then points to it.
    AdminClockRestart { period: f32, start: f32 },
    /// 0x759680 backup-mode decision (aa-runtime simulation.rs
    /// server_backup_update, inputs [`AdminNode::backup_clients`]).
    BackupModeDecision,
    /// 0x7531A0 private decrees of the host connection.
    PrivateDecrees { conn: usize },
    /// 0x791840 GameSpy "verify connection end".
    GameSpyVerifyEnd,
    /// ClientLeave 0x755840 UI message (0x40BC50(&0xB399B0, 0xF, ...)):
    /// [`LEAVE_KICKED_SELF_KEY`] without argument, the others with the
    /// display name (0x7667D0) of the leaving connection's player.
    LeaveMessage { key: &'static str, player: Option<u32> },
    /// 0x76CFA0 on the session (crate::staging::abort_countdown).
    AbortCountdown,
    /// 0x76AA80(0) on [0xB36554] (+0x28 = 0) when the leaving peer was +0x2A0.
    TimelineFlag28Cleared,
    /// 0x788290(player id) on [0xB40C24] (host, internet, multiplayer).
    GameSpyPlayerLeft { player: u32 },
    /// 0x755B58: every session player whose connection (+4, 0x8569A0) is
    /// the leaving peer is destroyed with KKC_Session::DestroyPlayer
    /// 0x7737E0 (crate::lobby::destroy_player), in session list order.
    DestroyPeerPlayers { peer: PeerRef },
    /// 0x76EE10 on the session (state 2 = in game).
    InGameLeave,
    /// 0x751F70 / ClientLeave tail when every peer's task status +0x34 is
    /// past 2 (0x74DA50): 0x750940 and 0x750990(0) on AdminNode+0x238, then
    /// KKC_Session 0x776560(`no_status_4`) (0x775BF0 when set, else
    /// 0x773AE0); ClearAllTasks 0x74DAA0 follows (applied).
    TasksComplete { no_status_4: bool },
}

/// ClientLeave UI keys (0xA8EC74, 0xA8EC3C, 0xA8EBF8, 0xA8EBC0).
pub const LEAVE_KICKED_SELF_KEY: &str = "networking_player_was_kicked";
pub const LEAVE_KICKED_KEY: &str = "networking_player_kicked|s";
pub const LEAVE_INVALID_CDKEY_KEY: &str = "networking_player_invalid_cdkey|s";
pub const LEAVE_QUIT_KEY: &str = "networking_player_quit|s";

/// Inputs of ClientLeave that are not AdminNode state.
pub struct LeaveEnv<'a> {
    /// NetNode+0x58 (binary32 bits): SetDisconnecting time.
    pub now: u32,
    /// Session +0xE4 state.
    pub session_state: u32,
    /// [0xB3654C]+0xBC internet game and session +0xF4 multiplayer.
    pub internet: bool,
    pub multiplayer: bool,
    /// [0xB36554]+0x29: every listed peer votes (0x74D380 / 0x74D320);
    /// otherwise only peers whose SessionPlayer +0x28 is set.
    pub all_peers_vote: bool,
    /// SessionPlayer +0x28 by id (read through the peer's +8 player).
    pub player_field_28: &'a dyn Fn(u32) -> Result<u32>,
}

/// Executor of the external effects.
pub trait Seams {
    fn effect(&mut self, node: &mut AdminNode, effect: Effect) -> Result<()>;
}

/// Records effects without executing them.
#[derive(Debug, Default)]
pub struct Recorder {
    pub effects: Vec<Effect>,
}
impl Seams for Recorder {
    fn effect(&mut self, _: &mut AdminNode, effect: Effect) -> Result<()> {
        self.effects.push(effect);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// x87 arithmetic on binary32 operands
// ---------------------------------------------------------------------------

fn quiet(v: f32) -> f32 {
    f32::from_bits(v.to_bits() | 0x0040_0000)
}

/// x87 NaN propagation of a two-operand instruction (SDM vol. 1 table 4-7):
/// a QNaN beats an SNaN, otherwise the larger significand wins (ties: the
/// positive one); the result is quiet.
fn x87_nan(a: f32, b: f32) -> f32 {
    match (a.is_nan(), b.is_nan()) {
        (true, false) => quiet(a),
        (false, true) => quiet(b),
        _ => {
            let qa = a.to_bits() & 0x0040_0000 != 0;
            let qb = b.to_bits() & 0x0040_0000 != 0;
            if qa != qb {
                return quiet(if qa { a } else { b });
            }
            let (sa, sb) = (a.to_bits() & 0x007F_FFFF, b.to_bits() & 0x007F_FFFF);
            if sa > sb {
                quiet(a)
            } else if sb > sa {
                quiet(b)
            } else if a.to_bits() >> 31 <= b.to_bits() >> 31 {
                quiet(a)
            } else {
                quiet(b)
            }
        }
    }
}

/// Result of a binary operation on binary32 operands, NaN handled the x87 way.
fn x87_op(a: f32, b: f32, r: f32) -> f32 {
    if a.is_nan() || b.is_nan() {
        x87_nan(a, b)
    } else if r.is_nan() {
        f32::from_bits(X87_INDEFINITE)
    } else {
        r
    }
}

/// `fld a; fsub b; fstp dword`: one operation on binary32 operands, precision
/// independent (0x75BD83, 0x75BE26).
pub fn x87_sub_store(a: f32, b: f32) -> f32 {
    x87_op(a, b, a - b)
}

/// `fld a; fdiv b; fstp dword` (0x74F3EF, 0x75718B): precision independent.
pub fn x87_div_store(a: f32, b: f32) -> f32 {
    x87_op(a, b, a / b)
}

/// Exact sum `s + e` of two f64 values (Knuth two-sum).
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    let e = (a - (s - bb)) + (b - bb);
    (s, e)
}

/// Round a finite nonzero f64 to 24 significant bits (round to nearest
/// even, unbounded exponent); returns (rounded, was exactly a midpoint).
fn round24(v: f64) -> (f64, bool) {
    if v == 0.0 || !v.is_finite() {
        return (v, false);
    }
    let bits = v.to_bits();
    const DROP: u32 = 29;
    let mask = (1u64 << DROP) - 1;
    let half = 1u64 << (DROP - 1);
    let low = bits & mask;
    let mut base = bits & !mask;
    if low > half || (low == half && (base >> DROP) & 1 == 1) {
        base += 1 << DROP;
    }
    (f64::from_bits(base), low == half)
}

/// round24 (unbounded exponent) of the exact value `s + e` where
/// `s = fl53(s + e)`: only a midpoint `s` can round differently from `s`,
/// and then the exact value lies on the side of `e`.
fn round24_exact(s: f64, e: f64) -> f64 {
    let (r, midpoint) = round24(s);
    if !midpoint || e == 0.0 {
        return r;
    }
    let base = s.to_bits() & !((1u64 << 29) - 1);
    let toward_zero = f64::from_bits(base);
    let away = f64::from_bits(base + (1u64 << 29));
    let (below, above) = if s > 0.0 { (toward_zero, away) } else { (away, toward_zero) };
    if e > 0.0 { above } else { below }
}

/// Binary32 neighbours of a value that `as f32` rounded to `r`.
fn f32_neighbour(r: f32, up: bool) -> f32 {
    if up {
        if r == 0.0 {
            f32::from_bits(1)
        } else if r > 0.0 {
            f32::from_bits(r.to_bits() + 1)
        } else {
            f32::from_bits(r.to_bits() - 1)
        }
    } else if r == 0.0 {
        -f32::from_bits(1)
    } else if r > 0.0 {
        f32::from_bits(r.to_bits() - 1)
    } else {
        f32::from_bits(r.to_bits() + 1)
    }
}

/// Binary32 store (fstp dword) of round64(s + e), `s = fl53(s + e)`.
fn store_from_extended(s: f64, e: f64) -> f32 {
    let r = s as f32;
    if e == 0.0 || f64::from(r) == s {
        return r;
    }
    // Overflow midpoint MAX + ulp/2: RNE goes to infinity (MAX is odd).
    let limit = f64::from(f32::MAX) + f64::from_bits(((103 + 1023) as u64) << 52);
    if !r.is_finite() {
        let exp = ((s.to_bits() >> 52) & 0x7FF) as i64 - 1023;
        let half_ulp64 = f64::from_bits(((exp - 64 + 1023) as u64) << 52);
        if s.abs() == limit && e.abs() > half_ulp64 && (e < 0.0) == (s > 0.0) {
            return f32::MAX.copysign(s as f32);
        }
        return r;
    }
    // s is not representable: is it the midpoint of two binary32 values?
    let other = f32_neighbour(r, f64::from(r) < s);
    if (f64::from(r) + f64::from(other)) / 2.0 != s {
        return r; // round64 cannot land on a binary32 midpoint
    }
    // round64(s+e) == s iff |e| <= ulp64(s)/2.
    let exp = ((s.to_bits() >> 52) & 0x7FF) as i64 - 1023;
    let half_ulp64 = f64::from_bits(((exp - 64 + 1023) as u64) << 52);
    if e.abs() <= half_ulp64 {
        r
    } else {
        let toward = if e > 0.0 { f64::from(r).max(f64::from(other)) } else { f64::from(r).min(f64::from(other)) };
        toward as f32
    }
}

/// 0x75BDF4..0x75BE01: `fld dt; fmul [0.1f]; fadd patience; fstp dword` at
/// precision `p` (the product and the sum are both rounded to `p`).
pub fn patience_increase(dt: f32, patience: f32, p: X87Precision) -> f32 {
    if !dt.is_finite() || !patience.is_finite() {
        // NaN or infinite operands: the result is NaN or infinite whatever
        // the rounding of a finite product (0.1 is finite and nonzero).
        let product = if dt.is_nan() { quiet(dt) } else { dt * PATIENCE_RATE };
        return x87_op(product, patience, product + patience);
    }
    let exact = f64::from(dt) * f64::from(PATIENCE_RATE); // 48 bits: exact
    match p {
        X87Precision::Single => {
            let (prod24, _) = round24(exact);
            let (s, e) = two_sum(prod24, f64::from(patience));
            let r = if s == 0.0 && e == 0.0 { s } else { round24_exact(s, e) };
            r as f32
        }
        X87Precision::Double => (exact + f64::from(patience)) as f32,
        X87Precision::Extended => {
            let (s, e) = two_sum(exact, f64::from(patience));
            store_from_extended(s, e)
        }
    }
}

/// `fld a; fsub b; fcomp c; test ah, 0x41` taken as "greater": the difference
/// stays in the register at precision `p` (0x75BE67, 0x75BECF, 0x75BEFE).
pub fn x87_sub_greater(a: f32, b: f32, c: f32, p: X87Precision) -> bool {
    if a.is_nan() || b.is_nan() {
        return false;
    }
    if a.is_infinite() || b.is_infinite() {
        let d = a - b;
        return !d.is_nan() && d > c;
    }
    let (s, e) = two_sum(f64::from(a), -f64::from(b));
    let c = f64::from(c);
    match p {
        X87Precision::Single => round24_exact(s, e) > c,
        X87Precision::Double => s > c,
        X87Precision::Extended => {
            if s != c {
                s > c
            } else {
                let exp = ((c.to_bits() >> 52) & 0x7FF) as i64 - 1023;
                e > f64::from_bits(((exp - 64 + 1023) as u64) << 52)
            }
        }
    }
}

/// Round `num / den` (num >= 0, den > 0) to `bits` significant bits, round to
/// nearest even; returns (significand, k) with value = significand * 2^-k.
fn round_ratio(num: u64, den: u64, bits: u32) -> (u128, i32) {
    let ln = 64 - num.leading_zeros() as i32;
    let ld = 64 - den.leading_zeros() as i32;
    let mut k = bits as i32 - (ln - ld);
    loop {
        let (n, d) = if k >= 0 {
            ((num as u128) << k, den as u128)
        } else {
            (num as u128, (den as u128) << (-k))
        };
        let q = n / d;
        if q >= (1u128 << bits) {
            k -= 1;
            continue;
        }
        if q < (1u128 << (bits - 1)) {
            k += 1;
            continue;
        }
        let r = n - q * d;
        let mut q = q;
        if 2 * r > d || (2 * r == d && q & 1 == 1) {
            q += 1;
        }
        if q == (1u128 << bits) {
            return (q >> 1, k - 1);
        }
        return (q, k);
    }
}

/// 0x7642D0 + `fstp dword [esi+0x50]` (0x74B28E): `fild count; fidiv n` at
/// precision `p`, stored as binary32. Empty window: the 0.0 constant 0x9F32DC.
pub fn received_fraction(received: &aa_orders::net::transport::ReceivedFlags, p: X87Precision) -> f32 {
    let n = received.flags.len();
    if n == 0 {
        return 0.0;
    }
    let count = received.flags.iter().filter(|&&f| f).count() as u64;
    // The native divisor is the 32-bit difference end - begin.
    let n = (received.begin.wrapping_add(n as i32)).wrapping_sub(received.begin);
    let negative = n < 0;
    if count == 0 {
        return if negative { -0.0 } else { 0.0 };
    }
    let den = u64::from(n.unsigned_abs());
    let first = match p {
        X87Precision::Single => 24,
        X87Precision::Double => 53,
        X87Precision::Extended => 64,
    };
    let (mut q, mut k) = round_ratio(count, den, first);
    if first > 24 {
        let drop = first - 24;
        let low = q & ((1u128 << drop) - 1);
        let half = 1u128 << (drop - 1);
        q >>= drop;
        k -= drop as i32;
        if low > half || (low == half && q & 1 == 1) {
            q += 1;
        }
    }
    let scale = f64::from_bits(((1023 - k) as u64) << 52);
    let v = (q as f64 * scale) as f32;
    if negative { -v } else { v }
}

// ---------------------------------------------------------------------------
// Backup request / decree packets
// ---------------------------------------------------------------------------

pub(crate) fn new_stream() -> Result<WireWriter> {
    Ok(WireWriter::new(Position::PACKET_START)?)
}

/// CString write `movzx len16; write 16; write len16*16 bits` (0x74F9A3,
/// 0x74FA79): only the low 16 bits of the length are used.
fn write_text(w: &mut WireWriter, text: &[u16]) -> Result<()> {
    let count = text.len() as u16;
    w.write_uint(u32::from(count), 16)?;
    let bytes: Vec<u8> = text[..usize::from(count)].iter().flat_map(|u| u.to_le_bytes()).collect();
    w.write_raw_bits(&bytes, usize::from(count) * 16)?;
    Ok(())
}

fn read_text(c: &mut Cursor<'_>) -> Result<Vec<u16>> {
    let count = c.read_uint(16)? as usize;
    let raw = c.read_raw_bits(count * 16)?;
    Ok(raw.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect())
}

/// 0x74F950 SerializeBackupRequestChat (writer): 1 bit 0, CString.
pub fn write_backup_request_chat(w: &mut WireWriter, text: &[u16]) -> Result<()> {
    w.write_uint(0, 1)?;
    write_text(w, text)
}

/// 0x74C3D0 SerializeBackupRequestVoteToDisconnect (writer): 1 bit 1, 1 bit vote.
pub fn write_backup_request_vote(w: &mut WireWriter, vote: bool) -> Result<()> {
    w.write_uint(1, 1)?;
    w.write_uint(u32::from(vote), 1)?;
    Ok(())
}

/// 0x74FA20 SerializeBackupDecreeChat (writer): 2 bits 1, CString, player
/// (0x765F10: SessionPlayer+0x20, 0 for null).
pub fn write_backup_decree_chat(w: &mut WireWriter, text: &[u16], player: Option<u32>) -> Result<()> {
    w.write_uint(1, 2)?;
    write_text(w, text)?;
    w.write_uint(player.unwrap_or(0), 32)?;
    Ok(())
}

/// 0x74C450 per-peer status record.
fn write_peer_status(w: &mut WireWriter, b: &PeerBase) -> Result<()> {
    write_status_fields(w,|o|Ok(match o{0x48=>b.flag_48,0x49=>b.flag_49,0x4a=>b.flag_4a,0x4b=>b.flag_4b,0x30=>b.flag_30,0x55=>b.vote,_=>unreachable!()}),
        |o|Ok(match o{0x4c=>b.rtt.to_bits(),0x50=>b.received_fraction.to_bits(),0x60=>b.frame_ratio.to_bits(),_=>unreachable!()}))
}
///Shared74C450 serializer with lazy current-field reads, in native order.
pub(crate) fn write_status_fields(w:&mut WireWriter,mut byte:impl FnMut(u32)->Result<u8>,mut word:impl FnMut(u32)->Result<u32>)->Result<()> {
    for o in [0x48,0x49,0x4a,0x4b]{w.write_uint(u32::from(byte(o)?!=0),1)?;}
    for o in [0x4c,0x50]{w.write_uint(word(o)?,32)?;}
    for o in [0x30,0x55]{w.write_uint(u32::from(byte(o)?!=0),1)?;}
    w.write_uint(word(0x60)?,32)?;
    Ok(())
}
///Shared74E530 prefix; actual peer owners serialize into the same writer.
pub(crate) fn write_status_decree(w:&mut WireWriter,mut word:impl FnMut(u32)->Result<u32>,peers:impl FnOnce(&mut WireWriter)->Result<()>)->Result<()> {
    w.write_uint(0,2)?;
    w.write_uint(word(0x2b8)?,32)?;
    w.write_uint(word(0x2bc)?,32)?;
    peers(w)
}

/// 0x74C590 per-peer status record (reader); fields are stored as read.
fn read_peer_status(c: &mut Cursor<'_>, b: &mut PeerBase) -> Result<()> {
    b.flag_48 = u8::from(c.read_uint(1)? == 1);
    b.flag_49 = u8::from(c.read_uint(1)? == 1);
    b.flag_4a = u8::from(c.read_uint(1)? == 1);
    b.flag_4b = u8::from(c.read_uint(1)? == 1);
    b.rtt = f32::from_bits(c.read_uint(32)?);
    b.received_fraction = f32::from_bits(c.read_uint(32)?);
    b.flag_30 = u8::from(c.read_uint(1)? == 1);
    b.vote = u8::from(c.read_uint(1)? == 1);
    b.frame_ratio = f32::from_bits(c.read_uint(32)?);
    Ok(())
}

/// Append a sealed packet at `end` (0x74EC30 + 0x766FC0) and retire to
/// `end - retention` when that is above begin (0x750880).
pub(crate) fn append(history: &mut History, packet: Packet, retention: i32) -> Result<()> {
    let index = history.end();
    history.set_end(index.checked_add(1).ok_or(aa_orders::Error::IndexExhausted)?)?;
    history.put(index, Some(packet))?;
    let floor = history.end().wrapping_sub(retention);
    if history.begin() < floor {
        history.retire_before(floor)?;
    }
    Ok(())
}

/// 0x7507D0: release every entry; begin = end = 0x80000000.
fn clear(history: &mut History) -> Result<()> {
    *history = History::new(i32::MIN, history.capacity())?;
    Ok(())
}

/// 0x7507D0 then 0x750880(0): empty at 0.
fn clear_to_zero(history: &mut History) -> Result<()> {
    clear(history)?;
    history.retire_before(0)?;
    Ok(())
}

pub(crate) fn packet_at(history: &History, index: i32) -> Result<Packet> {
    history
        .get(index)?
        .cloned()
        .ok_or(Error::Protocol(format!("history entry {index} holds no packet (native reads a null packet)")))
}

pub(crate) fn reader(packet: &Packet) -> Result<Cursor<'_>> {
    // Readers start at byte 0 bit 3 (local_14 = 0, local_10 = 3).
    Ok(Cursor::for_packet(packet.bytes(), None)?)
}

/// 0x750CB0 executed against the order layer: A "o.GetBitSize()", 9-bit
/// descriptor 309 and payload into the decree stream admin+0x1F8, A
/// "m_decree_type.m_name", A "m_decree_type.m_id", process 0x75DAA0 at once,
/// A "o.GetBitSize()". No S log and no film mirror.
pub fn emit_player_leave_decree(
    admin: &mut OrderNode,
    fields: &[Field],
    host: &mut dyn OrderHost,
    context: &dyn SchemaContext,
) -> Result<()> {
    let bits = |w: &WireWriter| -> Result<u32> { Ok((w.position().absolute()? - 3) as u32) };
    host.sync(SyncEntry::Int { log: Log::A, label: "o.GetBitSize()", value: bits(&admin.decree_stream)? })?;
    admin.decree_stream.write_uint(PLAYER_LEAVE_DECREE as u32, flow::DESCRIPTOR_BITS)?;
    payload::encode(PLAYER_LEAVE_DECREE, fields, &mut admin.decree_stream, context)?;
    host.sync(SyncEntry::Text { log: Log::A, label: "m_decree_type.m_name", value: "TGC_PlayerLeaveOrderDecree" })?;
    host.sync(SyncEntry::Int { log: Log::A, label: "m_decree_type.m_id", value: PLAYER_LEAVE_DECREE as u32 })?;
    let mut order = fields.to_vec();
    host.process(PLAYER_LEAVE_DECREE, &mut order)?;
    host.sync(SyncEntry::Int { log: Log::A, label: "o.GetBitSize()", value: bits(&admin.decree_stream)? })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// AdminNode
// ---------------------------------------------------------------------------

impl AdminNode {
    pub fn peer(&self, r: PeerRef) -> Result<&PeerBase> {
        match r {
            PeerRef::Node => Ok(&self.base),
            PeerRef::Conn(i) => self.conns.get(i).map(|c| &c.base).ok_or_else(|| missing(i)),
        }
    }
    pub fn peer_mut(&mut self, r: PeerRef) -> Result<&mut PeerBase> {
        match r {
            PeerRef::Node => Ok(&mut self.base),
            PeerRef::Conn(i) => self.conns.get_mut(i).map(|c| &mut c.base).ok_or_else(|| missing(i)),
        }
    }
    fn conn(&mut self, i: usize) -> Result<&mut AdminConn> {
        self.conns.get_mut(i).ok_or_else(|| missing(i))
    }
    fn is_server(&self) -> bool {
        self.server == Some(PeerRef::Node)
    }
    fn is_host(&self) -> bool {
        self.host == Some(PeerRef::Node)
    }

    /// Inputs of the backup-mode decision 0x759680: every entry of the peer
    /// list, the node included, in list order.
    pub fn backup_clients(&self) -> Result<Vec<BackupClient>> {
        self.list
            .iter()
            .map(|&r| {
                let b = self.peer(r)?;
                Ok(BackupClient {
                    flag_30: b.flag_30 != 0,
                    flag_48: b.flag_48 != 0,
                    flag_49: b.flag_49 != 0,
                    flag_4a: b.flag_4a != 0,
                    flag_4b: b.flag_4b != 0,
                })
            })
            .collect()
    }

    /// 0x75B820 async heartbeat.
    pub fn heartbeat(&mut self, env: &mut Env<'_>, seams: &mut dyn Seams) -> Result<()> {
        pulse(&mut BoundPulse {node:self,env,seams})
    }

    /// 0x7593E0 ServerDispatchBackupRequestsAndBuildDecree for one peer.
    fn dispatch_backup_requests(&mut self, r: PeerRef) -> Result<()> {
        loop {
            let history = &self.peer(r)?.backup_requests;
            let index = history.begin();
            if index >= history.end() {
                return Ok(());
            }
            let packet = packet_at(history, index)?;
            let mut c = reader(&packet)?;
            // 0x411FA0(2): one bit.
            match c.read_possibility(2)? {
                0 => {
                    // 0x753940: chat request -> chat decree 0x74FA20.
                    let text = read_text(&mut c)?;
                    let player = self.peer(r)?.player;
                    let mut w = new_stream()?;
                    write_backup_decree_chat(&mut w, &text, player)?;
                    let packet = Packet::new(finalize(&w)?)?;
                    append(&mut self.backup_decrees, packet, BACKUP_DECREE_RETENTION)?;
                }
                _ => {
                    // 0x74F890: vote request.
                    let vote = c.read_uint(1)?;
                    self.peer_mut(r)?.vote = u8::from(vote == 1);
                }
            }
            let history = &mut self.peer_mut(r)?.backup_requests;
            let begin = history.begin();
            history.retire_before(begin.checked_add(1).ok_or(aa_orders::Error::IndexExhausted)?)?;
        }
    }

    /// 0x75A2C0 DisconnectLostPlayers.
    fn disconnect_lost_players(&mut self, env: &mut Env<'_>, seams: &mut dyn Seams) -> Result<()> {
        if !self.is_host() && !self.local_decree_history_current()? {
            return Ok(());
        }
        if self.all_lagging_requested_disconnect()? {
            return self.drop_lagging_players(REASON_QUIT, env, seams);
        }
        if env.session_state == 1 {
            if self.base.vote != 0 {
                return self.drop_lagging_players(REASON_KICKED, env, seams);
            }
            return Ok(());
        }
        // 0x74DD50: peers with +0x4A clear, /2 + 1; 0x74DD20: votes.
        let mut live = 0i32;
        let mut votes = 0i32;
        for &r in &self.list {
            let b = self.peer(r)?;
            if b.flag_4a == 0 {
                live += 1;
            }
            if b.vote != 0 {
                votes += 1;
            }
        }
        let required = live / 2 + 1;
        // 0x75A32F: fcomp [0.1f]; C0|C3 -> max <= 0.1 (ordered).
        if required <= votes && self.max_lagging_patience()? <= PATIENCE_RATE {
            return self.drop_lagging_players(REASON_KICKED, env, seams);
        }
        Ok(())
    }

    /// 0x74DE10 IsLocalDecreeHistoryCurrent.
    fn local_decree_history_current(&self) -> Result<bool> {
        for &r in &self.list {
            if let PeerRef::Conn(i) = r {
                let c = self.conns.get(i).ok_or_else(|| missing(i))?;
                if self.has_admin_clock
                    && c.base.player.is_some()
                    && c.base.lag_bits() == 0
                    && c.reverse_history.end() <= c.reverse_start
                {
                    return Ok(false); // "NET: Reverse history not up to date"
                }
            }
        }
        Ok(true)
    }

    /// 0x74DED0: no lagging connection without +0x9C.
    fn all_lagging_requested_disconnect(&self) -> Result<bool> {
        for &r in &self.list {
            if let PeerRef::Conn(i) = r {
                let c = self.conns.get(i).ok_or_else(|| missing(i))?;
                if self.has_admin_clock && c.base.player.is_some() && c.base.lag_bits() != 0 && !c.server_disconnected {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// 0x74DD80: maximum patience of lagging connections, from 0.0.
    fn max_lagging_patience(&self) -> Result<f32> {
        let mut max = 0.0f32;
        for &r in &self.list {
            if let PeerRef::Conn(i) = r {
                let c = self.conns.get(i).ok_or_else(|| missing(i))?;
                // fld max; fcomp patience; test ah,5; jp -> take when max < patience.
                if c.base.lag_bits() != 0 && max < c.base.patience {
                    max = c.base.patience;
                }
            }
        }
        Ok(max)
    }

    /// 0x759530: take over as host if needed and drop the lagging players.
    fn drop_lagging_players(&mut self, reason: u32, env: &mut Env<'_>, seams: &mut dyn Seams) -> Result<()> {
        if !self.is_host() {
            // 0x74E9EF writes the 3 low bits of +0x90; schema 304 is bounded(5).
            let status = self.task_status & 7;
            if status >= 5 {
                return Err(Error::Unsupported("AdminNode+0x90 task status above 4 has no 304 schema value"));
            }
            self.set_host(PeerRef::Node, env, seams)?;
            self.took_over = 1;
            seams.effect(self, Effect::Request {
                descriptor: TASK_STATUS_REQUEST,
                fields: vec![Field::Bounded { n: 5, value: status }],
            })?;
        }
        let mut dropped = false;
        let mut k = 0;
        while k < self.list.len() {
            let r = self.list[k];
            k += 1;
            self.peer_mut(r)?.vote = 0;
            if let PeerRef::Conn(i) = r {
                let c = self.conns.get(i).ok_or_else(|| missing(i))?;
                if self.has_admin_clock
                    && c.base.lag_bits() != 0
                    && let Some(player) = c.base.player
                {
                    seams.effect(self, Effect::Decree {
                        descriptor: PLAYER_LEAVE_DECREE,
                        fields: vec![Field::Player(player), Field::Bounded { n: 13, value: reason }],
                    })?;
                    dropped = true;
                }
            }
        }
        if dropped && env.gamespy_verify {
            seams.effect(self, Effect::GameSpyVerifyEnd)?;
        }
        self.check_reverse_decrees(env)
    }

    /// 0x7501D0 CheckForReverseDecrees: append every reverse decree a
    /// connected client holds beyond the local decree history.
    fn check_reverse_decrees(&mut self, env: &mut Env<'_>) -> Result<()> {
        let mut k = 0;
        while k < self.list.len() {
            let r = self.list[k];
            k += 1;
            let PeerRef::Conn(i) = r else { continue };
            let c = self.conns.get(i).ok_or_else(|| missing(i))?;
            if !c.connected {
                continue;
            }
            while env.decrees.end() < c.reverse_history.end() {
                let index = env.decrees.end();
                let packet = c.reverse_history.get(index)?.cloned();
                env.decrees.set_end(index.checked_add(1).ok_or(aa_orders::Error::IndexExhausted)?)?;
                env.decrees.put(index, packet)?;
            }
        }
        Ok(())
    }

    /// 0x74F1A0 lag-state refresh.
    pub fn refresh_lag_state(&mut self, env: &mut Env<'_>) -> Result<()> {
        let mut votes = 0i32;
        for &r in &self.list {
            votes += i32::from(self.peer(r)?.vote != 0);
        }
        self.vote_count = votes;
        self.max_patience = self.max_lagging_patience()?;
        let host = self.is_host();
        let decree_end = env.decrees.end();
        let backup_end = self.backup_decrees.end();
        for k in 0..self.list.len() {
            match self.list[k] {
                PeerRef::Node => {
                    let b = &mut self.base;
                    b.rtt = 0.0; // 0x5AA2D0
                    b.received_fraction = 0.0; // 0x74B270
                    b.flag_4b = 0; // 0x74B2A0
                    b.flag_49 = env.falling_behind; // 0x74B2B0
                    b.flag_4a = 0; // 0x74B2D0
                    b.flag_30 = b.flag_31; // 0x74B2F0
                    b.flag_48 = 0; // 0x74B300
                }
                PeerRef::Conn(i) => {
                    let c = self.conns.get_mut(i).ok_or_else(|| missing(i))?;
                    c.base.rtt = c.net.rtt; // 0x74B260
                    c.base.received_fraction = received_fraction(&c.net.received, env.precision); // 0x74B280
                    // 0x74D490
                    if !c.connected {
                        c.base.flag_4b = 0;
                    } else {
                        let behind = backup_end.wrapping_sub(c.backup_decree_acked);
                        let mut f = behind > env.config.backup_decree_behind || c.status_backlog;
                        if host {
                            f |= decree_end.wrapping_sub(c.broadcast_decree_acked) > env.config.broadcast_decree_behind;
                        }
                        c.base.flag_4b = u8::from(f);
                    }
                    c.base.flag_49 = u8::from(c.status_falling_behind); // 0x74B2C0
                    c.base.flag_4a = u8::from(!c.connected); // 0x74B2E0
                    c.base.flag_30 = c.base.flag_31; // 0x74B2F0
                    // 0x74D5B0
                    c.base.flag_48 = u8::from(host && decree_end != c.broadcast_decree_acked);
                }
            }
            // 0x74F221..0x74F239: max over +0x60 into a dead local (no effect).
        }
        Ok(())
    }

    /// 0x752C60 build the backup status decree (0x74E530) into +0x224.
    fn build_backup_status_decree(&mut self) -> Result<()> {
        let mut w = new_stream()?;
        write_status_decree(&mut w,|o|Ok(match o{0x2b8=>self.vote_count as u32,0x2bc=>self.max_patience.to_bits(),_=>unreachable!()}),
            |w|{for &r in &self.list{write_peer_status(w,self.peer(r)?)?;}Ok(())})?;
        let packet = Packet::new(finalize(&w)?)?;
        append(&mut self.backup_decrees, packet, BACKUP_DECREE_RETENTION)
    }

    /// 0x75A0C0 ProcessBackupDecrees.
    fn process_backup_decrees(&mut self, env: &mut Env<'_>, seams: &mut dyn Seams) -> Result<()> {
        process_decree_history(&mut BoundDecrees{node:self,env,seams})
    }

    /// 0x74F310 UpdateFileTransferStatus (host side).
    fn update_file_transfer_status(&mut self, i: usize, seams: &mut dyn Seams) -> Result<()> {
        let c = self.conn(i)?;
        if c.base.transfer_status != 2 {
            return Ok(());
        }
        if c.file_sent == c.file_size {
            c.base.transfer_status = 1;
        }
        let fraction = x87_div_store(c.file_sent as f32, c.file_size as f32);
        let fields = vec![
            Field::Player(c.base.player.unwrap_or(0)),
            Field::Bounded { n: 4, value: c.base.transfer_status as u32 & 3 },
            Field::Uint { width: 32, value: fraction.to_bits() },
        ];
        seams.effect(self, Effect::Request { descriptor: MAP_STATUS_REQUEST, fields })
    }

    /// 0x75BD70 TGC_AdminConn::Update for connection `i`.
    pub fn update_conn(&mut self, i: usize, env: &mut Env<'_>, seams: &mut dyn Seams) -> Result<()> {
        let server = self.is_server();
        let p = env.precision;
        let now = env.now;
        let c = self.conn(i)?;
        let mut dt = x87_sub_store(now, c.last_update);
        if dt > DT_CLAMP {
            dt = DT_CLAMP;
        }
        c.last_update = now;
        if server {
            if c.base.lag_bits() == 0 {
                let x = patience_increase(dt, c.base.patience, p);
                c.base.patience = x;
                if x > PATIENCE_MAX {
                    c.base.patience = PATIENCE_MAX;
                }
            } else {
                let y = x87_sub_store(c.base.patience, dt);
                c.base.patience = y;
                if y < 0.0 {
                    c.base.patience = 0.0;
                }
            }
        }
        if c.disconnecting && x87_sub_greater(now, f32::from_bits(c.disconnect_started), PEER_TIMEOUT, p) {
            self.reset_conn(i, seams)?;
        }
        let c = self.conns.get(i).ok_or_else(|| missing(i))?;
        if self.has_admin_clock && self.base.player.is_some() && c.connected {
            let limit = if self.server == Some(PeerRef::Conn(i)) { SERVER_TIMEOUT } else { PEER_TIMEOUT };
            if x87_sub_greater(now, c.net.last_advance, limit, p) {
                self.timeout(i, env, seams)?;
            }
        }
        if self.host == Some(PeerRef::Conn(i)) {
            seams.effect(self, Effect::PrivateDecrees { conn: i })?;
        }
        Ok(())
    }

    /// 0x75A250 TGC_AdminConn::Timeout.
    fn timeout(&mut self, i: usize, env: &mut Env<'_>, seams: &mut dyn Seams) -> Result<()> {
        self.conn(i)?.connected = false;
        if self.server == Some(PeerRef::Conn(i)) {
            self.set_server(PeerRef::Node, env, seams)?;
        }
        Ok(())
    }

    /// 0x755840 TGC_AdminNode::ClientLeave(peer, reason, message,
    /// check_votes). The node itself leaving (`PeerRef::Node`): "was
    /// kicked" message for reason 4, ClientLeave(conn, 5, 0, 0) for every
    /// connection in the +0x2AC list (successor read before each call), the
    /// vote-kick sweep 0x74F160 and the task check 0x751F70. A connection
    /// leaving: message (kicked / invalid CD key / quit by reason), launch
    /// countdown abort, list removal (+0x2B4 = 1), +0x2A0 -> +0x2A4, vote
    /// removal 0x74EF90(peer, 0) on every remaining peer, GameSpy report,
    /// its session players destroyed, +8 / +0x2C cleared, the in-game hook,
    /// then on the host (reason != 5) SetDisconnecting inline, else
    /// AdminConn::Reset 0x752110; with `check_votes` the sweep; the task
    /// check (inline copy of 0x751F70).
    pub fn client_leave(
        &mut self,
        peer: PeerRef,
        reason: u32,
        message: bool,
        check_votes: bool,
        env: &LeaveEnv<'_>,
        seams: &mut dyn Seams,
    ) -> Result<()> {
        // log 0xBED "NET: ClientLeave(%s)" / 0xBEF "<no player>"
        let PeerRef::Conn(i) = peer else {
            if reason == 4 {
                seams.effect(self, Effect::LeaveMessage { key: LEAVE_KICKED_SELF_KEY, player: None })?;
            }
            for r in self.list.clone() {
                if let PeerRef::Conn(_) = r {
                    self.client_leave(r, 5, false, false, env, seams)?;
                }
            }
            self.vote_kick_sweep(env, seams)?;
            return self.check_tasks(seams);
        };
        let player = self.conn(i)?.base.player;
        if message {
            let key = match reason {
                4 => LEAVE_KICKED_KEY,
                0xB => LEAVE_INVALID_CDKEY_KEY,
                _ => LEAVE_QUIT_KEY,
            };
            let p = player.ok_or_else(|| Error::Protocol("0x7667D0 on the null player of a leaving connection".into()))?;
            seams.effect(self, Effect::LeaveMessage { key, player: Some(p) })?;
        }
        seams.effect(self, Effect::AbortCountdown)?;
        self.unlink_peer(peer);
        self.peers_changed = true;
        if self.field_2a0 == Some(peer) {
            self.field_2a4 = Some(peer);
            self.field_2a0 = None;
            seams.effect(self, Effect::TimelineFlag28Cleared)?;
        }
        for r in self.list.clone() {
            // 0x74EF90(peer, 0): log, then 0x4F1430 removes the first entry.
            let votes = &mut self.peer_mut(r)?.vote_to_kick;
            if let Some(k) = votes.iter().position(|&v| v == peer) {
                votes.remove(k);
            }
        }
        if self.is_host() && env.internet && env.multiplayer && let Some(p) = player {
            seams.effect(self, Effect::GameSpyPlayerLeft { player: p })?;
        }
        seams.effect(self, Effect::DestroyPeerPlayers { peer })?;
        {
            let c = self.conn(i)?;
            c.base.player = None;
            c.base.ordering_player = None;
        }
        if env.session_state == 2 {
            seams.effect(self, Effect::InGameLeave)?;
        }
        if self.is_host() && reason != 5 {
            // SetDisconnecting inline (log 0x102F).
            let c = self.conn(i)?;
            c.connected = false;
            c.disconnecting = true;
            c.disconnect_started = env.now;
            c.disconnect_reason = reason;
        } else {
            self.reset_conn(i, seams)?;
        }
        if check_votes {
            self.vote_kick_sweep(env, seams)?;
        }
        self.check_tasks(seams)
    }

    /// 0x4F1430 on +0x2AC: unlinks the first entry for `peer`, remembering
    /// its successor (see `removed_links`).
    fn unlink_peer(&mut self, peer: PeerRef) {
        if let Some(k) = self.list.iter().position(|&r| r == peer) {
            let next = self.list.get(k + 1).copied();
            self.list.remove(k);
            self.removed_links.retain(|(r, _)| *r != peer);
            self.removed_links.push((peer, next));
        }
    }

    /// Voter test of 0x74D380 / 0x74D320 for a listed peer.
    fn votes_counted(&self, r: PeerRef, env: &LeaveEnv<'_>) -> Result<bool> {
        if env.all_peers_vote {
            return Ok(true);
        }
        let player = self
            .peer(r)?
            .player
            .ok_or_else(|| Error::Protocol("0x74D3A9 reads +0x28 of a null peer player".into()))?;
        Ok((env.player_field_28)(player)? != 0)
    }

    /// 0x74F160: every peer (list order) with at least a majority (0x74D380:
    /// voters / 2 + 1, computed once) of voters having it in their +0x64
    /// list (0x74D320) leaves with ClientLeave(peer, 4, 1, 1).
    fn vote_kick_sweep(&mut self, env: &LeaveEnv<'_>, seams: &mut dyn Seams) -> Result<()> {
        let mut voters = 0i32;
        for &r in &self.list {
            voters += i32::from(self.votes_counted(r, env)?);
        }
        let majority = voters / 2 + 1;
        let mut cur = self.list.first().copied();
        while let Some(r) = cur {
            let mut votes = 0i32;
            for &q in &self.list {
                if self.votes_counted(q, env)? && self.peer(q)?.vote_to_kick.contains(&r) {
                    votes += 1;
                }
            }
            if votes >= majority {
                self.client_leave(r, 4, true, true, env, seams)?;
            }
            // [node+4] after the call: the live successor, or the link the
            // unlinked node kept.
            cur = match self.list.iter().position(|&x| x == r) {
                Some(k) => self.list.get(k + 1).copied(),
                None => self.removed_links.iter().find(|(x, _)| *x == r).and_then(|(_, n)| *n),
            };
        }
        Ok(())
    }

    /// 0x74DA50 + (0x750940, 0x750990(0), 0x776560) + ClearAllTasks 0x74DAA0.
    fn check_tasks(&mut self, seams: &mut dyn Seams) -> Result<()> {
        let mut no_status_4 = true;
        for &r in &self.list {
            let t = self.peer(r)?.field_34;
            no_status_4 = no_status_4 && t != 4;
            if matches!(t, 0..=2) {
                return Ok(());
            }
        }
        seams.effect(self, Effect::TasksComplete { no_status_4 })?;
        // 0x74DAA0 ClearAllTasks (log).
        for r in self.list.clone() {
            self.peer_mut(r)?.field_34 = 0;
        }
        self.field_8c = 0;
        Ok(())
    }

    /// 0x752110 connection reset (with the peer reset 0x7514F0); the field
    /// steps are the shared ones of `packets::AdminConn` (also used by
    /// `AdminConn::reset`), the channel resets are emitted as effects.
    fn reset_conn(&mut self, i: usize, seams: &mut dyn Seams) -> Result<()> {
        self.conn(i)?.base.reset_head();
        // 0x77B8D0 on +0x74: applied to the merged AdminConn and reported.
        self.conn(i)?.base.player_id = crate::session::PlayerId::default();
        seams.effect(self, Effect::PeerSubobjectReset { peer: PeerRef::Conn(i) })?;
        self.conn(i)?.base.reset_tail()?;
        self.conn(i)?.reset_conn_fields();
        use Channel as C;
        use ChannelOp as O;
        for (channel, op) in [
            (C::BroadcastDecreeReceiver, O::ReceiverReset),
            (C::BroadcastDecreeSender, O::SenderReset),
            (C::BroadcastRequestReceiver, O::ReceiverReset),
            (C::BroadcastRequestSender, O::SenderReset),
            (C::PrivateDecreeSender, O::SenderReset),
            (C::PrivateDecreeReceiver, O::ReceiverReset),
            (C::FileSender, O::FileSenderReset),
            (C::FileReceiver, O::FileReceiverReset),
        ] {
            seams.effect(self, Effect::Channel { conn: i, channel, op })?;
        }
        self.conn(i)?.reset_conn_tail()
    }

    /// 0x759790 TGC_AdminNode::SetServer.
    pub fn set_server(&mut self, server: PeerRef, env: &mut Env<'_>, seams: &mut dyn Seams) -> Result<()> {
        self.server = Some(server);
        clear_to_zero(&mut self.backup_decrees)?;
        self.next_backup_decree = 0;
        clear_to_zero(&mut self.base.backup_requests)?;
        if server != PeerRef::Node {
            return Ok(()); // "SetServer() Becoming client"
        }
        let mut k = 0;
        while k < self.list.len() {
            let r = self.list[k];
            k += 1;
            self.peer_mut(r)?.patience = PATIENCE_MAX;
            if let PeerRef::Conn(i) = r
                && self.conns.get(i).ok_or_else(|| missing(i))?.base.join_order >= self.base.join_order
            {
                self.bind_backup_server(i, seams)?;
            }
        }
        self.refresh_lag_state(env)?;
        seams.effect(self, Effect::BackupModeDecision)
    }

    /// 0x752300: bind connection `i` to this node acting as backup server.
    fn bind_backup_server(&mut self, i: usize, seams: &mut dyn Seams) -> Result<()> {
        use Channel as C;
        use ChannelOp as O;
        let start = self.backup_decrees.end();
        let c = self.conn(i)?;
        c.backup_index = c.backup_index.wrapping_add(1);
        seams.effect(self, Effect::Channel {
            conn: i,
            channel: C::BackupDecreeSender,
            op: O::SenderBind { history: HistoryRef::NodeBackupDecrees, start },
        })?;
        seams.effect(self, Effect::Channel { conn: i, channel: C::BackupDecreeReceiver, op: O::ReceiverReset })?;
        clear(&mut self.conn(i)?.base.backup_requests)?;
        for (channel, op) in [
            (C::BackupRequestSender, O::SenderReset),
            (C::BackupRequestReceiver, O::ReceiverBind { history: HistoryRef::ConnBackupRequests(i) }),
            (C::ReverseDecreeSender, O::SenderReset),
            (C::ReverseDecreeReceiver, O::ReceiverBind { history: HistoryRef::ConnReverseDecrees(i) }),
        ] {
            seams.effect(self, Effect::Channel { conn: i, channel, op })?;
        }
        Ok(())
    }

    /// 0x757010 TGC_AdminNode::SetHost.
    pub fn set_host(&mut self, host: PeerRef, env: &mut Env<'_>, seams: &mut dyn Seams) -> Result<()> {
        let player = self.peer(host)?.player;
        if self.host.is_some() {
            seams.effect(self, Effect::NewHostMessage { player })?;
        }
        if host != PeerRef::Node {
            self.host = Some(host);
            return Ok(());
        }
        seams.effect(self, Effect::ResetOrderVersion)?;
        let start = if self.host.is_some() { env.old_clock_time } else { 0.0 };
        let period = x87_div_store(DT_CLAMP, env.config.admin_ticks_per_second);
        seams.effect(self, Effect::AdminClockRestart { period, start })?;
        self.has_admin_clock = true;
        self.admin_clock_own = true;
        let had_host = self.host.is_some();
        let mut k = 0;
        while k < self.list.len() {
            let r = self.list[k];
            k += 1;
            if let PeerRef::Conn(i) = r {
                let c = self.conns.get(i).ok_or_else(|| missing(i))?;
                let first = if had_host { c.reverse_start.wrapping_add(1) } else { 0 };
                self.bind_host_channels(i, first, seams)?;
                let c = self.conn(i)?;
                c.base.ordering_player = c.base.player;
            }
        }
        self.host = Some(host);
        Ok(())
    }

    /// 0x752490: bind connection `i` to this node acting as host.
    fn bind_host_channels(&mut self, i: usize, start: i32, seams: &mut dyn Seams) -> Result<()> {
        use Channel as C;
        use ChannelOp as O;
        clear_to_zero(&mut self.conn(i)?.private_history)?;
        for (channel, op) in [
            (C::PrivateDecreeSender, O::SenderBind { history: HistoryRef::ConnPrivateDecrees(i), start: 0 }),
            (C::PrivateDecreeReceiver, O::ReceiverReset),
            (C::BroadcastDecreeSender, O::SenderBind { history: HistoryRef::NodeDecrees, start }),
            (C::BroadcastDecreeReceiver, O::ReceiverReset),
            (C::BroadcastRequestSender, O::SenderReset),
            (C::BroadcastRequestReceiver, O::ReceiverBind { history: HistoryRef::ConnRequests(i) }),
        ] {
            seams.effect(self, Effect::Channel { conn: i, channel, op })?;
        }
        Ok(())
    }

    /// 0x753410 SendBackupChat: 0x74F950 packet appended to the own backup
    /// request history (+0x1C), which keeps end-1000.
    pub fn send_backup_chat(&mut self, text: &[u16]) -> Result<()> {
        let mut w = new_stream()?;
        write_backup_request_chat(&mut w, text)?;
        append(&mut self.base.backup_requests, Packet::new(finalize(&w)?)?, BACKUP_REQUEST_RETENTION)
    }

    /// 0x7532D0 SendBackupVote: 0x74C3D0 packet appended like the chat.
    pub fn send_backup_vote(&mut self, vote: bool) -> Result<()> {
        let mut w = new_stream()?;
        write_backup_request_vote(&mut w, vote)?;
        append(&mut self.base.backup_requests, Packet::new(finalize(&w)?)?, BACKUP_REQUEST_RETENTION)
    }
}

fn missing(i: usize) -> Error {
    Error::Protocol(format!("peer list names connection {i} which does not exist"))
}

///The ONE75B820 coordinator. Concrete local fields and the existing packet
///model supply their own borrowed storage; neither builds a second Node.
pub(crate) trait Pulse {
    fn is_server(&self)->bool;
    fn is_host(&self)->bool;
    fn counter(&self)->i32;
    fn increment_counter(&mut self);
    fn reset_counter(&mut self);
    fn dispatch_backup_requests(&mut self)->Result<()>;
    fn disconnect_lost_players(&mut self)->Result<()>;
    fn refresh_lag_state(&mut self)->Result<()>;
    fn build_backup_status_decree(&mut self)->Result<()>;
    fn process_backup_decrees(&mut self)->Result<()>;
    fn update_file_transfer_status(&mut self)->Result<()>;
}
pub(crate) fn pulse(world:&mut impl Pulse)->Result<()> {
    if world.is_server(){
        world.dispatch_backup_requests()?;
        world.disconnect_lost_players()?;
    }
    if world.is_server()&&world.counter()>=REFRESH_COUNTER {
        world.refresh_lag_state()?;
        world.build_backup_status_decree()?;
        world.reset_counter();
    }else{world.increment_counter();}
    world.process_backup_decrees()?;
    if world.is_host(){world.update_file_transfer_status()?;}
    Ok(())
}
struct BoundPulse<'a,'env>{node:&'a mut AdminNode,env:&'a mut Env<'env>,seams:&'a mut dyn Seams}
impl Pulse for BoundPulse<'_,'_>{
    fn is_server(&self)->bool{self.node.is_server()}
    fn is_host(&self)->bool{self.node.is_host()}
    fn counter(&self)->i32{self.node.async_counter}
    fn increment_counter(&mut self){self.node.async_counter=self.node.async_counter.wrapping_add(1);}
    fn reset_counter(&mut self){self.node.async_counter=0;}
    fn dispatch_backup_requests(&mut self)->Result<()>{
        let mut k=0;
        while k<self.node.list.len(){let r=self.node.list[k];k+=1;self.node.dispatch_backup_requests(r)?;}
        Ok(())
    }
    fn disconnect_lost_players(&mut self)->Result<()>{self.node.disconnect_lost_players(self.env,self.seams)}
    fn refresh_lag_state(&mut self)->Result<()>{self.node.refresh_lag_state(self.env)}
    fn build_backup_status_decree(&mut self)->Result<()>{self.node.build_backup_status_decree()}
    fn process_backup_decrees(&mut self)->Result<()>{self.node.process_backup_decrees(self.env,self.seams)}
    fn update_file_transfer_status(&mut self)->Result<()>{
        let mut k=0;
        while k<self.node.list.len(){let r=self.node.list[k];k+=1;if let PeerRef::Conn(i)=r{self.node.update_file_transfer_status(i,self.seams)?;}}
        Ok(())
    }
}

///ONE75A0C0 cursor/parser loop. The concrete application must bind diagnostics;
///the pre-existing packet model keeps its historically unobserved log scope.
pub(crate) trait Decrees {
    fn next(&self)->i32;
    fn begin(&self)->i32;
    fn end(&self)->i32;
    fn set_next(&mut self,value:i32);
    fn packet(&self,index:i32)->Result<Packet>;
    fn diagnostic(&mut self,index:Option<i32>)->Result<()>;
    fn apply(&mut self,kind:u32,reader:&mut Cursor<'_>)->Result<()>;
}
pub(crate) fn process_decree_history(world:&mut impl Decrees)->Result<()> {
    if world.next()==-1 {
        if world.begin()==world.end(){world.diagnostic(None)?;return Ok(());}
        world.set_next(world.begin());
    }
    while world.next()<world.end(){
        world.diagnostic(Some(world.next()))?;
        let packet=world.packet(world.next())?;
        let mut cursor=reader(&packet)?;
        let kind=cursor.read_possibility(3)?;
        world.apply(kind,&mut cursor)?;
        world.set_next(world.next().wrapping_add(1));
    }
    Ok(())
}
struct BoundDecrees<'a,'env>{node:&'a mut AdminNode,env:&'a mut Env<'env>,seams:&'a mut dyn Seams}
impl Decrees for BoundDecrees<'_,'_>{
    fn next(&self)->i32{self.node.next_backup_decree}
    fn begin(&self)->i32{self.node.backup_decrees.begin()}
    fn end(&self)->i32{self.node.backup_decrees.end()}
    fn set_next(&mut self,v:i32){self.node.next_backup_decree=v;}
    fn packet(&self,i:i32)->Result<Packet>{packet_at(&self.node.backup_decrees,i)}
    fn diagnostic(&mut self,_:Option<i32>)->Result<()>{Ok(())} //Unchanged packet-model scope.
    fn apply(&mut self,kind:u32,c:&mut Cursor<'_>)->Result<()>{
        match kind {
            0=>if !self.node.is_server(){
                self.node.vote_count=c.read_uint(32)? as i32;
                self.node.max_patience=f32::from_bits(c.read_uint(32)?);
                for k in 0..self.node.list.len(){let r=self.node.list[k];read_peer_status(c,self.node.peer_mut(r)?)?;}
            },
            1=>{
                let text=read_text(c)?;let id=c.read_uint(32)?;
                if (self.env.players)(id){self.seams.effect(self.node,Effect::PublicChat{player:id,text})?;}
            },
            _=>{},
        }
        Ok(())
    }
}
