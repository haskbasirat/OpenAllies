//! TGC_AdminConn packet bodies (writer dispatch 0x7521F0, reader 0x75CD20,
//! 4-bit type written by 0x763A60 and read by 0x764950 = 0x411FA0(13)).
//!
//! | type | name                    | writer   | reader   |
//! |------|-------------------------|----------|----------|
//! | 2    | Connect                 | 0x750330 | 0x75BF90 | (session.rs)
//! | 3    | Reconnect               | 0x74C2D0 | 0x753820 |
//! | 4    | ClientToServer          | 0x74E0F0 | 0x74E170 |
//! | 5    | ServerToClient          | 0x74E280 | 0x75A350 |
//! | 6    | ClientToBackupServer    | 0x74E3C0 | 0x74E430 |
//! | 7    | BackupServerToClient    | 0x74C310 | 0x75A5C0 |
//! | 8    | ServerDisconnect        | 0x74B680 | 0x75A730 |
//! | 9    | (no body, no reader case)                     |
//! | 10   | ClientDisconnectRequest | 0x74B6B0 | 0x74F7C0 |
//! | 11   | ClientDisconnectDecree  | 0x74C380 | 0x75A780 |
//!
//! Section limits are absolute stream positions: a writer passes
//! `L - (8*byte + bit)` with L = 0x403/0x603 (client->host) and
//! 0x803/0xC03 (host->client), i.e. bit targets 0x400/0x600/0x800/0xC00
//! including the transport header written into the same stream. These are
//! wire-compatibility values, kept. Types 6/7 pass 0x400 directly.
//!
//! Readers return [`ConnEvent`]s for node-level effects (SetHost 0x757010,
//! SetServer 0x759790, sender resets 0x7525B0/0x7523A0, backup mode 0x757250,
//! disconnects) instead of reaching into a global admin node.
use crate::channel::{BlockReceiver, BlockSender, BlockSink, BlockSource};
use crate::file::{FileReceiver, FileSender, FileStore};
use crate::history::{clear_history, HistoryReceiver, HistorySender};
use crate::session::PlayerId;
use crate::{Error, NativeImage, Result};
use aa_orders::net::transport::Sequencing;
use aa_orders::commands::WireWriter;
use aa_orders::{Cursor, History};
use serde::{Deserialize, Serialize};

pub const TYPE_CONNECT: u32 = 2;
pub const TYPE_RECONNECT: u32 = 3;
pub const TYPE_CLIENT_TO_SERVER: u32 = 4;
pub const TYPE_SERVER_TO_CLIENT: u32 = 5;
pub const TYPE_CLIENT_TO_BACKUP: u32 = 6;
pub const TYPE_BACKUP_TO_CLIENT: u32 = 7;
pub const TYPE_SERVER_DISCONNECT: u32 = 8;
pub const TYPE_BACKUP_DISCONNECT: u32 = 9;
pub const TYPE_CLIENT_DISCONNECT_REQUEST: u32 = 10;
pub const TYPE_CLIENT_DISCONNECT_DECREE: u32 = 11;
/// 0x764950: 0x411FA0 with 13 possibilities (4 bits).
pub const PACKET_TYPES: usize = 13;
/// ClientDisconnectDecree reason, 0x411FA0(13) (0x75A780).
pub const DISCONNECT_REASONS: usize = 13;

/// Disconnect reasons (UI keys used by 0x75A780).
pub const REASON_KEYS: [&str; 13] = [
    "networking_disconnect_game_full",
    "password",
    "networking_disconnect_wrong_version",
    "networking_disconnect_banned",
    "kicked",
    "quit",
    "bad_depot_checksums",
    "bad_data_checksums",
    "wrong_mods",
    "internet_lan_mismatch",
    "game_in_progress",
    "cdkey_auth_failed",
    "prompt_not_original_host",
];

/// +0x44 patience maximum and AdminConn::Reset value (0x7521E6: 15.0).
pub const PATIENCE_MAX: f32 = 15.0;
/// 0x751536: AdminPeer+0x58 reset value (-10000.0).
pub const FIELD_58_RESET: u32 = 0xC61C_3C00;

/// TGC_AdminPeer base shared by AdminNode and AdminConn (ctor 0x752A60,
/// reset 0x7514F0). Byte fields are kept as bytes because the native copies
/// them byte-wise (+0x30 = +0x31, +0x49 = +0xA9 / NetNode+0x64).
pub struct PeerBase {
    /// +0x04 (reset 0x751530; meaning not established).
    pub field_04: u32,
    /// +0x08 SessionPlayer, as its +0x20 id; None = null pointer.
    pub player: Option<u32>,
    /// +0x0C queued request packets (0x74B790(0x400)).
    pub requests: History,
    /// +0x1C backup request packets (0x74B790(0x100)).
    pub backup_requests: History,
    /// +0x2C ordering player (SessionPlayer id).
    pub ordering_player: Option<u32>,
    /// +0x30 lag flag (wants-backup bit 1), copied from +0x31 by 0x74B2F0.
    pub flag_30: u8,
    /// +0x31 pending lag flag (set by 0x751D00).
    pub flag_31: u8,
    pub field_32: u8,
    pub field_34: i32,
    /// +0x38 map transfer status (2 = sending, 1 = done; 0x74F310).
    pub transfer_status: i32,
    pub field_3c: i32,
    pub field_40: i32,
    /// +0x44 lag patience in seconds (15.0 initially).
    pub patience: f32,
    /// +0x48 host decree history not fully acknowledged (0x74D5B0).
    pub flag_48: u8,
    /// +0x49 falling behind (wants-backup bit 8).
    pub flag_49: u8,
    /// +0x4A not connected (wants-backup bit 2).
    pub flag_4a: u8,
    /// +0x4B decree channels behind (wants-backup bit 4).
    pub flag_4b: u8,
    /// +0x4C smoothed RTT copy (NetConn+0x38).
    pub rtt: f32,
    /// +0x50 received fraction (0x7642D0).
    pub received_fraction: f32,
    pub field_54: u8,
    /// +0x55 vote to disconnect the lagging players.
    pub vote: u8,
    pub field_56: u8,
    pub field_58: f32,
    pub field_5c: i32,
    /// +0x60 client frame ratio (0x752AE0), raw bits copied.
    pub frame_ratio: f32,
    /// +0x64/+0x68 vote-to-kick list (0x74EF90 adds / removes peers; the
    /// sweep 0x74F160 counts them through 0x74D320): peers voted against,
    /// head first.
    pub vote_to_kick: Vec<crate::heartbeat::PeerRef>,
    pub field_6c: u8,
    /// +0x70 join order (set by the PlayerJoin decree 0x758FD0; the backup
    /// rank compared by the backup readers and the send gate).
    pub join_order: i32,
    /// +0x74 player id object (0x77BA70; reset by 0x77B8D0 to an empty name
    /// and zero words; written by the PlayerJoin decree and the snapshot).
    pub player_id: PlayerId,
}

impl PeerBase {
    /// Constructor 0x752A60 (histories +0xC / +0x1C with initial capacities
    /// 0x400 / 0x100, +0x64/+0x68 list empty) followed by the peer reset
    /// 0x7514F0, as every AdminPeer constructor ends; +0x44 patience as
    /// AdminConn::Reset 0x752110 leaves it (15.0). Fields the original leaves
    /// to the allocator are 0.
    pub fn new() -> Result<Self> {
        let mut b = Self {
            field_04: 0,
            player: None,
            requests: History::new(i32::MIN, 0x400)?,
            backup_requests: History::new(i32::MIN, 0x100)?,
            ordering_player: None,
            flag_30: 0,
            flag_31: 0,
            field_32: 0,
            field_34: 0,
            transfer_status: 0,
            field_3c: 0,
            field_40: 0,
            patience: PATIENCE_MAX,
            flag_48: 0,
            flag_49: 0,
            flag_4a: 0,
            flag_4b: 0,
            rtt: 0.0,
            received_fraction: 0.0,
            field_54: 0,
            vote: 0,
            field_56: 0,
            field_58: 0.0,
            field_5c: 0,
            frame_ratio: 0.0,
            vote_to_kick: Vec::new(),
            field_6c: 0,
            join_order: 0,
            player_id: PlayerId::default(),
        };
        b.reset_head();
        b.player_id = PlayerId::default();
        b.reset_tail()?;
        Ok(b)
    }
    /// 0x7514F0 up to the +0x74 sub-object reset 0x77B8D0.
    pub fn reset_head(&mut self) {
        self.field_32 = 0;
        self.field_34 = 0;
        self.field_6c = 1;
        self.field_40 = 0;
        self.field_3c = 0;
        self.transfer_status = 0;
        self.ordering_player = None;
        self.field_04 = 0;
        self.player = None;
    }
    /// 0x7514F0 after 0x77B8D0.
    pub fn reset_tail(&mut self) -> Result<()> {
        self.field_58 = f32::from_bits(FIELD_58_RESET);
        self.field_56 = 0;
        self.vote_to_kick.clear();
        clear_history(&mut self.requests)?;
        clear_history(&mut self.backup_requests)?;
        self.flag_30 = 0;
        self.flag_31 = 0;
        self.flag_48 = 0;
        self.flag_49 = 0;
        self.flag_4a = 0;
        self.flag_4b = 0;
        self.rtt = 0.0;
        self.received_fraction = 0.0;
        self.field_54 = 0;
        self.vote = 0;
        self.join_order = 0;
        self.field_5c = 0;
        Ok(())
    }
    /// Wants-backup byte `8*(+0x49) + 4*(+0x4B) + 2*(+0x4A) + (+0x30)` tested
    /// for zero by 0x74DD80, 0x74DE10, 0x74DED0, 0x759530, 0x759680, 0x75BDC4.
    pub fn lag_bits(&self) -> u8 {
        (u8::from(self.flag_49 != 0) << 3)
            | (u8::from(self.flag_4b != 0) << 2)
            | (u8::from(self.flag_4a != 0) << 1)
            | u8::from(self.flag_30 != 0)
    }
}

/// One TGC_AdminConn (0x424 bytes, ctor 0x756BC0) for the packet layer, the
/// heartbeat port (crate::heartbeat) and the lobby port (crate::lobby).
pub struct AdminConn {
    /// TGC_AdminPeer base +0x04..+0x84 (+8 session player, +0xC request
    /// history received from this peer, +0x1C backup requests, +0x2C
    /// ordering player, +0x44 patience, +0x60 frame ratio = the status word
    /// of packet types 4/6, +0x70 join order = backup rank, +0x74 player id).
    pub base: PeerBase,
    /// +0x88 connected, +0x89 disconnecting, +0x8C disconnect reason, +0x90
    /// NetNode time (binary32 bits) when disconnecting was set (0x74B620).
    pub connected: bool,
    pub disconnecting: bool,
    pub disconnect_reason: u32,
    pub disconnect_started: u32,
    pub field_94: u32,
    pub field_98: u32,
    /// +0x9C disconnect seen (ServerDisconnect 0x75A730 / client request
    /// 0x74F7C0).
    pub server_disconnected: bool,
    /// +0xA0 backup index, +0xA4 reverse start (last reverse decree
    /// acknowledged; -1 after reset).
    pub backup_index: i32,
    pub reverse_start: i32,
    /// +0xA8 request backlog and +0xA9 falling behind (status block 0x74C7B0).
    pub status_backlog: bool,
    pub status_falling_behind: bool,
    pub field_aa: u8,
    /// +0xAC private decree history, +0xBC reverse decree history.
    pub private_history: History,
    pub reverse_history: History,
    /// +0xCC/+0x114 broadcast decree channel.
    pub decree_out: HistorySender,
    pub decree_in: HistoryReceiver,
    /// +0x144/+0x18C reverse decree channel (host migration).
    pub reverse_out: HistorySender,
    pub reverse_in: HistoryReceiver,
    /// +0x1BC/+0x204 broadcast request channel.
    pub request_out: HistorySender,
    pub request_in: HistoryReceiver,
    /// +0x234/+0x27C private decree channel.
    pub private_out: HistorySender,
    pub private_in: HistoryReceiver,
    /// +0x2AC/+0x2F4 backup decree channel.
    pub backup_decree_out: HistorySender,
    pub backup_decree_in: HistoryReceiver,
    /// +0x324/+0x36C backup request channel.
    pub backup_request_out: HistorySender,
    pub backup_request_in: HistoryReceiver,
    /// Channel queries the heartbeat reads, refreshed from the channel
    /// objects by [`AdminConn::refresh_channel_status`]: 0x7626A0 on +0xCC
    /// and +0x2AC; 0x7612D0 / 0x7A6610 on the FileSender +0x39C.
    pub broadcast_decree_acked: i32,
    pub backup_decree_acked: i32,
    pub file_sent: i32,
    pub file_size: i32,
    /// +0x410 time of the previous Update 0x75BD70.
    pub last_update: f32,
    /// +0x414 NetConn sequencing state (+0x1C received window, +0x2C
    /// liveness, +0x38 RTT) as the heartbeat reads it.
    pub net: Sequencing,
    /// +0x418 file-transfer generation, +0x420 connect accepted.
    pub file_generation: i32,
    pub accepted: bool,
}

impl AdminConn {
    /// 0x756BC0: base 0x752A60 (histories +0xC/+0x1C, capacities 0x400 /
    /// 0x100), +0xAC/+0xBC (0x100), twelve channels (span 32, copies 2),
    /// FileSender/FileReceiver, then Reset 0x752110. Every history starts
    /// as [INT_MIN, INT_MIN) (0x74B790 and 0x7507D0). The capacities are
    /// initial sizes only; [`History`] grows.
    pub fn new() -> Result<Self> {
        let mut c = Self {
            base: PeerBase::new()?,
            connected: false,
            disconnecting: false,
            disconnect_reason: 0xd,
            disconnect_started: 0,
            field_94: 0,
            field_98: 0,
            server_disconnected: false,
            backup_index: 0,
            reverse_start: -1,
            status_backlog: false,
            status_falling_behind: false,
            field_aa: 0,
            private_history: History::new(i32::MIN, 0x100)?,
            reverse_history: History::new(i32::MIN, 0x100)?,
            decree_out: HistorySender::default(),
            decree_in: HistoryReceiver::new()?,
            reverse_out: HistorySender::default(),
            reverse_in: HistoryReceiver::new()?,
            request_out: HistorySender::default(),
            request_in: HistoryReceiver::new()?,
            private_out: HistorySender::default(),
            private_in: HistoryReceiver::new()?,
            backup_decree_out: HistorySender::default(),
            backup_decree_in: HistoryReceiver::new()?,
            backup_request_out: HistorySender::default(),
            backup_request_in: HistoryReceiver::new()?,
            broadcast_decree_acked: 0,
            backup_decree_acked: 0,
            file_sent: 0,
            file_size: 0,
            last_update: 0.0,
            net: Sequencing::default(),
            file_generation: 0,
            accepted: false,
        };
        c.reset_conn_fields();
        c.reset_conn_tail()?;
        Ok(c)
    }

    /// 0x752110 field part before the channel resets (after the peer reset
    /// 0x7514F0): connected, +0x9C, disconnecting cleared, reason 0xD, +0x90,
    /// +0x70, +0x94, +0x98, +0xA8..+0xAA, +0x3C.
    pub fn reset_conn_fields(&mut self) {
        self.connected = false;
        self.server_disconnected = false;
        self.disconnecting = false;
        self.disconnect_reason = 0xd;
        self.disconnect_started = 0;
        self.base.join_order = 0;
        self.field_94 = 0;
        self.field_98 = 0;
        self.status_backlog = false;
        self.status_falling_behind = false;
        self.field_aa = 0;
        self.base.field_3c = 0;
    }
    /// 0x752110 after the channel resets: +0xAC cleared, backup index 0,
    /// +0x9C, file generation 0, +0x420, reverse start -1, patience 15.0.
    pub fn reset_conn_tail(&mut self) -> Result<()> {
        clear_history(&mut self.private_history)?;
        self.backup_index = 0;
        self.server_disconnected = false;
        self.file_generation = 0;
        self.accepted = false;
        self.reverse_start = -1;
        self.base.patience = PATIENCE_MAX;
        Ok(())
    }

    /// 0x752110 AdminConn::Reset applied directly: peer reset 0x7514F0
    /// (including 0x77B8D0 on the +0x74 player id), the connection fields,
    /// the decree / request / private channel resets in native order
    /// (receiver +0x114, sender +0xCC, receiver +0x204, sender +0x1BC,
    /// sender +0x234, receiver +0x27C; backup and reverse channels are NOT
    /// touched), FileSender 0x761400 and FileReceiver 0x761500, then the
    /// tail. The heartbeat port runs the same steps with its channel effects
    /// (crate::heartbeat).
    pub fn reset(&mut self, send: &mut dyn FileSending, receive: &mut dyn FileReceiving) -> Result<()> {
        self.base.reset_head();
        self.base.player_id = PlayerId::default();
        self.base.reset_tail()?;
        self.reset_conn_fields();
        self.decree_in.reset();
        self.decree_out.reset();
        self.request_in.reset();
        self.request_out.reset();
        self.private_out.reset();
        self.private_in.reset();
        send.reset()?;
        receive.reset()?;
        self.reset_conn_tail()
    }

    /// Refreshes the channel queries the heartbeat reads: 0x7626A0 on the
    /// broadcast decree sender (+0xCC, bound to AdminNode+0x214) and on the
    /// backup decree sender (+0x2AC, AdminNode+0x224); file progress
    /// 0x7612D0 and size 0x7A6610 from the caller's FileSender.
    pub fn refresh_channel_status(&mut self, node: &NodeHistories<'_>, file_sent: i32, file_size: i32) {
        self.broadcast_decree_acked = self.decree_out.oldest_pending_index(node.decrees);
        self.backup_decree_acked = self.backup_decree_out.oldest_pending_index(node.backup_decrees);
        self.file_sent = file_sent;
        self.file_size = file_size;
    }
}

/// Node-level histories and flags the packet bodies read (TGC_AdminNode).
pub struct NodeHistories<'a> {
    /// +0xC request history, +0x1C backup request history.
    pub requests: &'a mut History,
    pub backup_requests: &'a mut History,
    /// +0x214 decree history, +0x224 backup decree history.
    pub decrees: &'a mut History,
    pub backup_decrees: &'a mut History,
    /// +0x234 next backup decree to process (ProcessBackupDecrees 0x75A0C0;
    /// -1 = start at the backup decree history begin).
    pub backup_decree_next: &'a mut i32,
}

/// Network AVars of the admin layer (table [0xB39990], registered by
/// 0x401BD0 with these integer defaults; none is present in Data/AVars*.tgi).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkAvars {
    /// 0x48 (+0x120) NetworkFileTransferBlocksPerFrame = 20 (0x403762).
    pub file_blocks_per_frame: i32,
    /// 0x49 (+0x124) NetworkFileTransferBlockSize = 64 (0x4037A5).
    pub file_block_size: i32,
    /// 0x4A (+0x128) NetworkFileTransferMaxPendingBlocks = 256 (0x4037E8).
    pub file_max_pending_blocks: i32,
    /// 0x4B (+0x12C) NetworkBroadcastDecreeBehindThreshold = 32 (0x40382E).
    pub broadcast_decree_behind: i32,
    /// 0x4C (+0x130) NetworkBroadcastRequestBehindThreshold = 32 (0x403871).
    pub broadcast_request_behind: i32,
    /// 0x4D (+0x134) NetworkBroadcastUseMoreBandwidthThreshold = 2 (0x4038B4).
    pub broadcast_use_more_bandwidth: i32,
    /// 0x4E (+0x138) NetworkBackupUseMoreBandwidthThreshold = 4 (0x4038F7).
    pub backup_use_more_bandwidth: i32,
    /// 0x4F (+0x13C) NetworkBackupDecreeBehindThreshold = 64 (0x40393A).
    pub backup_decree_behind: i32,
    /// 0x50 (+0x140) NetworkBackupRequestBehindThreshold = 32 (0x40397D).
    pub backup_request_behind: i32,
}
impl Default for NetworkAvars {
    fn default() -> Self {
        Self {
            file_blocks_per_frame: 20,
            file_block_size: 64,
            file_max_pending_blocks: 256,
            broadcast_decree_behind: 32,
            broadcast_request_behind: 32,
            broadcast_use_more_bandwidth: 2,
            backup_use_more_bandwidth: 4,
            backup_decree_behind: 64,
            backup_request_behind: 32,
        }
    }
}

/// AdminConn+0x39C FileSender as seen by the packet layer: a
/// TGC_BlockSender (span AVar 0x48, one copy) over the file source.
pub trait FileSending {
    /// 0x760A80 on +0x39C.
    fn serialize(&mut self, w: &mut WireWriter, sequence: i32, limit: i32) -> Result<()>;
    /// 0x7608E0 on +0x39C (from the ack dispatcher 0x74B4F0).
    fn packet_result(&mut self, sequence: i32, acked: bool) -> Result<()>;
    /// 0x761400 (from AdminConn::Reset 0x752110).
    fn reset(&mut self) -> Result<()>;
    /// v14 cave 0x9EFF40 file test: FileSender+0x48 size >= 0 and the
    /// channel's first unsent block (0x760590 on +0x39C) is below its block
    /// window end (+0x3B4).
    fn unsent_blocks_pending(&self) -> bool;
}
/// AdminConn+0x3E8 FileReceiver as seen by 0x74C8B0.
pub trait FileReceiving {
    /// 0x760F20 on +0x3E8.
    fn receive(&mut self, r: &mut Cursor<'_>) -> Result<()>;
    /// 0x75FEA0 on +0x3E8.
    fn skip(&mut self, r: &mut Cursor<'_>) -> Result<()>;
    /// 0x761500.
    fn reset(&mut self) -> Result<()>;
}

struct NoBlocks;
impl BlockSource for NoBlocks {
    fn available(&mut self) -> Result<i32> {
        Ok(0)
    }
    fn release(&mut self, _index: i32) -> Result<()> {
        Ok(())
    }
    fn estimate(&mut self, _index: i32) -> Result<i32> {
        Ok(0)
    }
    fn serialize(&mut self, _w: &mut WireWriter, _index: i32) -> Result<()> {
        Ok(())
    }
}
impl BlockSink for NoBlocks {
    fn receive(&mut self, _r: &mut Cursor<'_>, index: i32) -> Result<()> {
        Err(Error::Protocol(format!("file block {index} received without a file receiver")))
    }
    fn skip(&mut self, _r: &mut Cursor<'_>, index: i32) -> Result<()> {
        Err(Error::Protocol(format!("file block {index} skipped without a file receiver")))
    }
}

/// File channel of a connection that never starts a transfer: the
/// constructed FileSender (0x7611B0: block window +0x2C empty, so
/// available 0x761920 = 0) writes the single absent bit of 0x760A80; a
/// section carrying blocks is a typed error instead of being queued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoFileTransfer {
    pub sender: BlockSender,
    pub receiver: BlockReceiver,
}
impl NoFileTransfer {
    pub fn new(avars: &NetworkAvars) -> Self {
        Self {
            sender: BlockSender::new(avars.file_blocks_per_frame, 1),
            receiver: BlockReceiver { span: avars.file_blocks_per_frame },
        }
    }
}
impl FileSending for NoFileTransfer {
    fn serialize(&mut self, w: &mut WireWriter, sequence: i32, limit: i32) -> Result<()> {
        BlockSender::serialize(&mut self.sender, &mut NoBlocks, w, sequence, limit)
    }
    fn packet_result(&mut self, sequence: i32, acked: bool) -> Result<()> {
        self.sender.packet_result(&mut NoBlocks, sequence, acked)
    }
    fn reset(&mut self) -> Result<()> {
        self.sender.reset();
        Ok(())
    }
    fn unsent_blocks_pending(&self) -> bool {
        false
    }
}

/// AdminConn+0x39C: the ported FileSender (crate::file) with its
/// TGC_BlockSender base.
#[derive(Debug, Clone)]
pub struct FileChannel<F: FileStore> {
    pub channel: BlockSender,
    pub sender: FileSender<F>,
}
impl<F: FileStore> FileSending for FileChannel<F> {
    fn serialize(&mut self, w: &mut WireWriter, sequence: i32, limit: i32) -> Result<()> {
        BlockSender::serialize(&mut self.channel, &mut self.sender, w, sequence, limit)
    }
    fn packet_result(&mut self, sequence: i32, acked: bool) -> Result<()> {
        self.channel.packet_result(&mut self.sender, sequence, acked)
    }
    fn reset(&mut self) -> Result<()> {
        self.sender.reset(&mut self.channel);
        Ok(())
    }
    fn unsent_blocks_pending(&self) -> bool {
        self.sender.size >= 0 && self.channel.first_unsent() < self.channel.blocks.end()
    }
}
/// AdminConn+0x3E8: the ported FileReceiver (its BlockReceiver base is
/// `channel`).
impl<F: FileStore> FileReceiving for FileReceiver<F> {
    fn receive(&mut self, r: &mut Cursor<'_>) -> Result<()> {
        let channel = self.channel;
        channel.receive(self, r)
    }
    fn skip(&mut self, r: &mut Cursor<'_>) -> Result<()> {
        let channel = self.channel;
        channel.skip(self, r)
    }
    fn reset(&mut self) -> Result<()> {
        FileReceiver::reset(self)
    }
}

/// 0x74E36E: absolute bit target of the type-5 file section (the original
/// 0xC03; the accepted v14 image raises it to 0x2403 at 0x74E370).
pub fn file_target(image: NativeImage) -> i32 {
    match image {
        NativeImage::Original => 0xc03,
        NativeImage::V14 => 0x2403,
    }
}
impl FileReceiving for NoFileTransfer {
    fn receive(&mut self, r: &mut Cursor<'_>) -> Result<()> {
        let receiver = self.receiver;
        receiver.receive(&mut NoBlocks, r)
    }
    fn skip(&mut self, r: &mut Cursor<'_>) -> Result<()> {
        let receiver = self.receiver;
        receiver.skip(&mut NoBlocks, r)
    }
    fn reset(&mut self) -> Result<()> {
        Ok(())
    }
}

impl AdminConn {
    /// 0x752490 ResetPermanentSendersToClient(start) (host side, after the
    /// PlayerJoin decree 0x753580): private history +0xAC cleared to
    /// [0, 0); private sender bound to it at 0; private receiver reset;
    /// broadcast decree sender bound to AdminNode+0x214 at `start`; decree
    /// receiver reset; request sender reset; request receiver bound to the
    /// per-connection request history +0xC (cleared).
    pub fn reset_senders_to_client(&mut self, start: i32) -> Result<()> {
        clear_history(&mut self.private_history)?;
        self.private_history.retire_before(0)?;
        self.private_out.bind(0)?;
        self.private_in.reset();
        self.decree_out.bind(start)?;
        self.decree_in.reset();
        self.request_out.reset();
        self.request_in.bind(&mut self.base.requests)
    }

    /// 0x7525B0 ResetPermanentSendersToServer (client side): node request
    /// history +0xC cleared to [0, 0); private sender reset; private
    /// receiver bound to +0xAC; decree sender reset; decree receiver bound
    /// to AdminNode+0x214 (cleared); request sender bound to AdminNode+0xC
    /// at 0; request receiver reset.
    pub fn reset_senders_to_server(&mut self, node: &mut NodeHistories<'_>) -> Result<()> {
        clear_history(node.requests)?;
        node.requests.retire_before(0)?;
        self.private_out.reset();
        self.private_in.bind(&mut self.private_history)?;
        self.decree_out.reset();
        self.decree_in.bind(node.decrees)?;
        self.request_out.bind(0)?;
        self.request_in.reset();
        Ok(())
    }

    /// 0x752300 ResetBackupSendersToClient (backup server side): backup
    /// index +0xA0 incremented; backup decree sender bound to
    /// AdminNode+0x224 at its current end; backup decree receiver reset;
    /// per-connection backup requests +0x1C cleared; backup request sender
    /// reset and receiver bound to +0x1C; reverse sender reset and receiver
    /// bound to the reverse history +0xBC.
    pub fn reset_backup_senders_to_client(&mut self, node: &mut NodeHistories<'_>) -> Result<()> {
        self.backup_index = self.backup_index.wrapping_add(1);
        self.backup_decree_out.bind(node.backup_decrees.end())?;
        self.backup_decree_in.reset();
        clear_history(&mut self.base.backup_requests)?;
        self.backup_request_out.reset();
        self.backup_request_in.bind(&mut self.base.backup_requests)?;
        self.reverse_out.reset();
        self.reverse_in.bind(&mut self.reverse_history)
    }

    /// 0x7523A0 ResetBackupSendersToServer(reverse_start) (client side):
    /// node backup requests +0x1C cleared to [0, 0); backup decree sender
    /// reset; backup decree receiver bound to AdminNode+0x224 (cleared);
    /// AdminNode+0x234 = -1; per-connection +0x1C cleared; backup request
    /// sender bound to AdminNode+0x1C at 0; backup request receiver reset;
    /// reverse sender bound to AdminNode+0x214 at `reverse_start` unless it
    /// is -1 (then reset); reverse receiver reset.
    pub fn reset_backup_senders_to_server(
        &mut self,
        node: &mut NodeHistories<'_>,
        reverse_start: i32,
    ) -> Result<()> {
        clear_history(node.backup_requests)?;
        node.backup_requests.retire_before(0)?;
        self.backup_decree_out.reset();
        self.backup_decree_in.bind(node.backup_decrees)?;
        *node.backup_decree_next = -1;
        clear_history(&mut self.base.backup_requests)?;
        self.backup_request_out.bind(0)?;
        self.backup_request_in.reset();
        if reverse_start != -1 {
            self.reverse_out.bind(reverse_start)?;
        } else {
            self.reverse_out.reset();
        }
        self.reverse_in.reset();
        Ok(())
    }

    /// 0x74B4F0: transport result for `sequence`, in native order: decree
    /// (+0xCC, AdminNode+0x214), request (+0x1BC, AdminNode+0xC), backup
    /// decree (+0x2AC, AdminNode+0x224), backup request (+0x324,
    /// AdminNode+0x1C), private (+0x234, +0xAC), file (+0x39C), reverse
    /// (+0x144, AdminNode+0x214). Receivers get 0x4D1E10 (no-op).
    pub fn packet_result(
        &mut self,
        node: &NodeHistories<'_>,
        sequence: i32,
        acked: bool,
        file: &mut dyn FileSending,
    ) -> Result<()> {
        self.decree_out.packet_result(node.decrees, sequence, acked)?;
        self.request_out.packet_result(node.requests, sequence, acked)?;
        self.backup_decree_out.packet_result(node.backup_decrees, sequence, acked)?;
        self.backup_request_out.packet_result(node.backup_requests, sequence, acked)?;
        self.private_out.packet_result(&self.private_history, sequence, acked)?;
        file.packet_result(sequence, acked)?;
        self.reverse_out.packet_result(node.decrees, sequence, acked)
    }

    /// Send-gate backlog test. Original 0x74BCB0: (AdminNode+0x228 backup
    /// decree end - oldest unsent backup decree 0x762640) > AVar 0x4E; on the
    /// host additionally (AdminNode+0x218 - oldest unsent decree) > AVar 0x4D
    /// or the oldest pending private block (0x7626A0) is below +0xB0.
    /// v14 (0x74BEC9 -> 0x9EFF40 -> 0x9EFEC0): if that is false, the bound
    /// decree history (+0xF8) with begin >= 0 and next (+0x110) inside it
    /// counts when end - next > AVar 0x4D; then, with map transfer status
    /// +0x38 == 2, a file with unsent blocks counts.
    pub fn backlogged(
        &mut self,
        node: &NodeHistories<'_>,
        we_are_host: bool,
        avars: &NetworkAvars,
        image: NativeImage,
        map_transfer_status: i32,
        file: &dyn FileSending,
    ) -> Result<bool> {
        let unsent = self.backup_decree_out.oldest_unsent_index(node.backup_decrees)?;
        let mut backlog = node.backup_decrees.end().wrapping_sub(unsent) > avars.backup_use_more_bandwidth;
        if we_are_host {
            let unsent = self.decree_out.oldest_unsent_index(node.decrees)?;
            backlog |= node.decrees.end().wrapping_sub(unsent) > avars.broadcast_use_more_bandwidth;
            backlog |= self.private_out.oldest_pending_index(&self.private_history) < self.private_history.end();
        }
        if image == NativeImage::Original || backlog {
            return Ok(backlog);
        }
        if self.decree_out.bound {
            let (begin, end, next) = (node.decrees.begin(), node.decrees.end(), self.decree_out.next);
            if begin >= 0 && next >= begin && next < end && end.wrapping_sub(next) > avars.broadcast_use_more_bandwidth {
                return Ok(true);
            }
        }
        Ok(map_transfer_status == 2 && file.unsent_blocks_pending())
    }

    /// 0x74C6D0 inputs: backlog = (AdminNode+0x1C end - oldest pending
    /// backup request 0x7626A0) > AVar 0x50, or, for the host conn
    /// (AdminNode+0x2C4), (AdminNode+0xC end - oldest pending request) >
    /// AVar 0x4C; falling behind = NetNode+0x64; word = +0x60.
    pub fn status_inputs(
        &mut self,
        node: &NodeHistories<'_>,
        conn_is_host: bool,
        falling_behind: bool,
        avars: &NetworkAvars,
    ) -> StatusInputs {
        let pending = self.backup_request_out.oldest_pending_index(node.backup_requests);
        let mut backlog = node.backup_requests.end().wrapping_sub(pending) > avars.backup_request_behind;
        if conn_is_host {
            let pending = self.request_out.oldest_pending_index(node.requests);
            backlog |= node.requests.end().wrapping_sub(pending) > avars.broadcast_request_behind;
        }
        StatusInputs { backlog, falling_behind, word: self.base.frame_ratio.to_bits() }
    }
}

/// 0x74C8B0: 32-bit generation, then the file section: older than +0x418
/// -> skipped, equal -> received, newer -> receiver reset 0x761500,
/// received, +0x418 = generation.
pub fn read_file_section(conn: &mut AdminConn, r: &mut Cursor<'_>, file: &mut dyn FileReceiving) -> Result<()> {
    let generation = r.read_uint(32)? as i32;
    if generation < conn.file_generation {
        return file.skip(r);
    }
    if generation == conn.file_generation {
        return file.receive(r);
    }
    file.reset()?;
    file.receive(r)?;
    conn.file_generation = generation;
    Ok(())
}

/// Inputs of the status block 0x74C6D0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusInputs {
    /// (backup request history end) - (oldest unacked backup request) >
    /// AVar NetworkBackupRequestBehindThreshold (0xB39990+0x140, 32), or, when
    /// this conn is the host conn, the same for the broadcast request channel
    /// (AVar +0x130, 32).
    pub backlog: bool,
    /// NetNode+0x64 falling behind (0x765410 frame statistics).
    pub falling_behind: bool,
    /// AdminConn+0x60 value written.
    pub word: u32,
}

/// Node-level effects of a received packet, in native order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnEvent {
    /// Native log line (AdminLayer.cpp line) for an ignored packet.
    Ignored(u32),
    /// 0x75A350: from the server conn that is not the host: AdminNode+0x2E0
    /// = 1, SetHost(conn) 0x757010, UpdateTaskStatus request 0x74E970(0);
    /// ResetPermanentSendersToServer 0x7525B0 was applied by the reader.
    MigratedToServerAsHost,
    /// 0x75A350: from the host while it is not our server: connected = 1
    /// (applied), SetServer(conn) 0x759790; when still connecting: 0x40B450
    /// on 0xB399B0 and AdminNode+0x2CC = 0 (ResetPermanentSendersToServer
    /// 0x7525B0 was applied by the reader).
    HostBecameServer { was_connecting: bool },
    /// New backup index: ResetBackupSendersToServer(reverse start) 0x7523A0
    /// and 0x74B6E0(index) were applied by the reader (AdminNode+0x234 = -1).
    NewBackupIndex { index: i32, reverse_start: i32 },
    /// Backup mode changed (0x757250).
    BackupMode(bool),
    /// 0x74C850 after the broadcast decree section, only with an admin
    /// clock (AdminNode+0x158): 0x75FB40 on AdminNode+0x16C with binary32
    /// `fild(decree_end - AdminNode+0x20C - 1) * 0.0625 [0xA8C084] +
    /// AdminNode+0x28C` (x87 statement, precision of the caller's thread).
    ClientClockUpdate { decree_end: i32 },
    /// 0x75A730 from our server: Timeout 0x75A250 on the conn (heartbeat
    /// port), then +0x9C = 1 (applied).
    ServerDisconnected,
    /// 0x74F7C0 on the host for a conn with a session player: PlayerLeave
    /// request 0x74EAE0 {player = conn+8, reason 5 "quit"} (emitter argument 0).
    ClientLeaveRequested { reason: u32 },
    /// 0x75A780: 0x74B980 stops the admin node (before the reason is read),
    /// then with a session state (+0xE4 != 0) SetServer(node) 0x759790,
    /// SetHost(node) 0x757010 and AdminNode+0x2E0 = 2 (`take_over`); then the
    /// reason dialog (UI key in [`REASON_KEYS`]) and ClientLeave
    /// 0x755840(node, reason, 1, 1).
    DisconnectedByServer { reason: u32, take_over: bool },
    /// 0x753820: reconnect from a client while we are the backup server
    /// (reverse start +0xA4 stored, connected = 1 and
    /// ResetBackupSendersToClient 0x752300 applied by the reader).
    Reconnected { reverse_start: i32, was_connected: bool },
    /// 0x75A5C0 adopting a backup server: connected = 1 (applied),
    /// SetServer(conn) 0x759790.
    BackupServerAdopted,
}

/// Roles of the local node relative to the connection (AdminNode +0x2C4/+0x2C8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Roles {
    /// Local node is the host (node == +0x2C4).
    pub we_are_host: bool,
    /// Local node is the server (node == +0x2C8).
    pub we_are_server: bool,
    /// This conn is the host (+0x2C4) / server (+0x2C8) conn.
    pub conn_is_host: bool,
    pub conn_is_server: bool,
    /// AdminNode+0x2CC connecting.
    pub connecting: bool,
    /// AdminNode+0x158 admin clock present and +8 local player present
    /// (false while joining).
    pub joined: bool,
    /// Session backup-mode bit as written/compared by 0x74E280/0x75A350
    /// (session && session+0xF4 && session+0xE4 && AdminNode+0x2D4).
    pub backup_mode: bool,
    /// AdminNode+0x70 local rank (backup ordering).
    pub rank: i32,
    /// 0x74B600 sender is a session member (for Reconnect).
    pub session_member: bool,
    /// Session +0xE4 != 0 (game state set), for 0x75A780.
    pub session_state_set: bool,
    /// AdminNode+0x158 admin clock present (0x74C850 client clock update).
    pub admin_clock: bool,
}

fn limit(w: &WireWriter, target: i32) -> Result<i32> {
    Ok(target - w.position().absolute()? as i32)
}

/// 0x74C6D0.
pub fn write_status(w: &mut WireWriter, s: StatusInputs) -> Result<()> {
    w.write_uint(u32::from(s.backlog), 1)?;
    w.write_uint(u32::from(s.falling_behind), 1)?;
    w.write_uint(s.word, 32)?;
    Ok(())
}
/// 0x74C7B0: bits stored only from the most recent sequence.
pub fn read_status(conn: &mut AdminConn, r: &mut Cursor<'_>, most_recent: bool) -> Result<()> {
    let backlog = r.read_uint(1)? == 1;
    let behind = r.read_uint(1)? == 1;
    conn.base.frame_ratio = f32::from_bits(r.read_uint(32)?);
    if most_recent {
        conn.status_backlog = backlog;
        conn.status_falling_behind = behind;
    }
    Ok(())
}

/// 0x74E0F0 ClientToServer.
pub fn write_client_to_server(
    conn: &mut AdminConn,
    node: &NodeHistories<'_>,
    w: &mut WireWriter,
    sequence: i32,
    status: StatusInputs,
) -> Result<()> {
    let l = limit(w, 0x403)?;
    conn.request_out.serialize(node.requests, w, sequence, l)?;
    w.write_uint(conn.backup_index as u32, 32)?;
    write_status(w, status)?;
    let l = limit(w, 0x603)?;
    conn.backup_request_out.serialize(node.backup_requests, w, sequence, l)
}

/// 0x74E170 ClientToServer reader (host only).
pub fn read_client_to_server(
    conn: &mut AdminConn,
    r: &mut Cursor<'_>,
    roles: Roles,
    most_recent: bool,
) -> Result<Vec<ConnEvent>> {
    if !roles.we_are_host {
        return Ok(vec![ConnEvent::Ignored(0x127c)]);
    }
    if conn.disconnecting {
        return Ok(vec![ConnEvent::Ignored(0x1284)]);
    }
    conn.request_in.receive(&mut conn.base.requests, r)?;
    if !conn.connected {
        return Ok(vec![ConnEvent::Ignored(0x1292)]);
    }
    let index = r.read_uint(32)? as i32;
    read_status(conn, r, most_recent)?;
    if index != conn.backup_index {
        conn.backup_request_in.skip(&mut conn.base.backup_requests, r)?;
        return Ok(vec![ConnEvent::Ignored(0x129d)]);
    }
    conn.backup_request_in.receive(&mut conn.base.backup_requests, r)?;
    Ok(Vec::new())
}

/// 0x74E280 ServerToClient. The file channel (FileSender +0x39C) gets the
/// limit computed before the 32-bit generation word, against
/// [`file_target`] of `image`.
#[allow(clippy::too_many_arguments)]
pub fn write_server_to_client(
    conn: &mut AdminConn,
    node: &NodeHistories<'_>,
    w: &mut WireWriter,
    sequence: i32,
    roles: Roles,
    image: NativeImage,
    file: &mut dyn FileSending,
) -> Result<()> {
    w.write_uint(u32::from(roles.backup_mode), 1)?;
    w.write_uint(conn.backup_index as u32, 32)?;
    let l = limit(w, 0x803)?;
    conn.private_out.serialize(&conn.private_history, w, sequence, l)?;
    let l = limit(w, 0xc03)?;
    conn.backup_decree_out.serialize(node.backup_decrees, w, sequence, l)?;
    let l = limit(w, 0xc03)?;
    conn.decree_out.serialize(node.decrees, w, sequence, l)?;
    // The file budget is taken before the 32-bit generation word (uncharged).
    let file_limit = limit(w, file_target(image))?;
    w.write_uint(conn.file_generation as u32, 32)?;
    file.serialize(w, sequence, file_limit)
}

/// 0x75A350 ServerToClient reader; the file section is read by 0x74C8B0.
/// Channel bindings happen in native order inside the reader (they decide
/// how the following sections are read); node-level effects are returned.
pub fn read_server_to_client(
    conn: &mut AdminConn,
    node: &mut NodeHistories<'_>,
    r: &mut Cursor<'_>,
    roles: Roles,
    most_recent: bool,
    file: &mut dyn FileReceiving,
) -> Result<Vec<ConnEvent>> {
    let mut events = Vec::new();
    if roles.conn_is_server {
        if !roles.conn_is_host {
            events.push(ConnEvent::MigratedToServerAsHost);
            conn.reset_senders_to_server(node)?;
        }
    } else {
        if !roles.conn_is_host {
            return Ok(vec![ConnEvent::Ignored(0x12c4)]);
        }
        if !most_recent {
            return Ok(vec![ConnEvent::Ignored(0x12c9)]);
        }
        conn.connected = true;
        events.push(ConnEvent::HostBecameServer { was_connecting: roles.connecting });
        if roles.connecting {
            conn.reset_senders_to_server(node)?;
        }
    }
    let backup_mode = r.read_uint(1)? == 1;
    let index = r.read_uint(32)? as i32;
    if most_recent {
        if conn.backup_index != index {
            let reverse_start = conn.reverse_start.wrapping_add(1);
            conn.reset_backup_senders_to_server(node, reverse_start)?;
            conn.backup_index = index;
            events.push(ConnEvent::NewBackupIndex { index, reverse_start });
        }
        if roles.backup_mode != backup_mode {
            events.push(ConnEvent::BackupMode(backup_mode));
        }
    }
    conn.private_in.receive(&mut conn.private_history, r)?;
    if index != conn.backup_index {
        conn.backup_decree_in.skip(node.backup_decrees, r)?;
    } else {
        conn.backup_decree_in.receive(node.backup_decrees, r)?;
    }
    conn.decree_in.receive(node.decrees, r)?;
    if roles.admin_clock {
        events.push(ConnEvent::ClientClockUpdate { decree_end: node.decrees.end() });
    }
    read_file_section(conn, r, file)?;
    Ok(events)
}

/// 0x74E3C0 ClientToBackupServer: backup index, status, backup-request
/// channel (+0x324, bound to AdminNode+0x1C) and reverse-decree channel
/// (+0x144, bound to the decree history at the reverse start by 0x7523A0),
/// each with the fixed budget 0x400.
pub fn write_client_to_backup(
    conn: &mut AdminConn,
    node: &NodeHistories<'_>,
    w: &mut WireWriter,
    sequence: i32,
    status: StatusInputs,
) -> Result<()> {
    w.write_uint(conn.backup_index as u32, 32)?;
    write_status(w, status)?;
    conn.backup_request_out.serialize(node.backup_requests, w, sequence, 0x400)?;
    conn.reverse_out.serialize(node.decrees, w, sequence, 0x400)
}

/// 0x74E430 ClientToBackupServer reader (backup server only).
pub fn read_client_to_backup(
    conn: &mut AdminConn,
    r: &mut Cursor<'_>,
    roles: Roles,
    most_recent: bool,
) -> Result<Vec<ConnEvent>> {
    if !roles.we_are_server {
        return Ok(vec![ConnEvent::Ignored(0x1323)]);
    }
    if roles.we_are_host {
        return Ok(vec![ConnEvent::Ignored(0x1329)]);
    }
    if conn.disconnecting {
        return Ok(vec![ConnEvent::Ignored(0x132f)]);
    }
    if !conn.connected {
        return Ok(vec![ConnEvent::Ignored(0x1335)]);
    }
    let index = r.read_uint(32)? as i32;
    if index != conn.backup_index {
        return Ok(vec![ConnEvent::Ignored(0x133e)]);
    }
    read_status(conn, r, most_recent)?;
    conn.backup_request_in.receive(&mut conn.base.backup_requests, r)?;
    conn.reverse_in.receive(&mut conn.reverse_history, r)?;
    Ok(Vec::new())
}

/// 0x75A5C0 BackupServerToClient reader (bindings applied in native order).
pub fn read_backup_to_client(
    conn: &mut AdminConn,
    node: &mut NodeHistories<'_>,
    r: &mut Cursor<'_>,
    roles: Roles,
    most_recent: bool,
) -> Result<Vec<ConnEvent>> {
    let mut events = Vec::new();
    if !roles.conn_is_server {
        if roles.conn_is_host {
            return Ok(vec![ConnEvent::Ignored(0x135c)]);
        }
        if !roles.joined {
            return Ok(vec![ConnEvent::Ignored(0x1361)]);
        }
        if !most_recent {
            return Ok(vec![ConnEvent::Ignored(0x1366)]);
        }
        if conn.base.join_order >= roles.rank {
            return Ok(vec![ConnEvent::Ignored(0x136b)]);
        }
        conn.connected = true;
        events.push(ConnEvent::BackupServerAdopted);
    }
    let index = r.read_uint(32)? as i32;
    let decree_end = r.read_uint(32)? as i32;
    if most_recent && conn.backup_index != index {
        conn.reset_backup_senders_to_server(node, decree_end)?;
        conn.backup_index = index;
        events.push(ConnEvent::NewBackupIndex { index, reverse_start: decree_end });
    }
    if index != conn.backup_index {
        conn.backup_decree_in.skip(node.backup_decrees, r)?;
    } else {
        conn.backup_decree_in.receive(node.backup_decrees, r)?;
    }
    Ok(events)
}

/// 0x74C310 BackupServerToClient.
pub fn write_backup_to_client(
    conn: &mut AdminConn,
    w: &mut WireWriter,
    sequence: i32,
    decree_end: i32,
    backup_decrees: &History,
) -> Result<()> {
    w.write_uint(conn.backup_index as u32, 32)?;
    w.write_uint(decree_end as u32, 32)?;
    conn.backup_decree_out.serialize(backup_decrees, w, sequence, 0x400)
}

/// 0x74C2D0 Reconnect: admin+0x218 - 1.
pub fn write_reconnect(w: &mut WireWriter, decree_end: i32) -> Result<()> {
    w.write_uint(decree_end.wrapping_sub(1) as u32, 32).map_err(Error::from)
}

/// 0x753820 Reconnect reader (ResetBackupSendersToClient applied).
pub fn read_reconnect(
    conn: &mut AdminConn,
    node: &mut NodeHistories<'_>,
    r: &mut Cursor<'_>,
    roles: Roles,
    most_recent: bool,
) -> Result<Vec<ConnEvent>> {
    if !roles.joined {
        return Ok(vec![ConnEvent::Ignored(0x1225)]);
    }
    if conn.disconnecting {
        return Ok(vec![ConnEvent::Ignored(0x122a)]);
    }
    if !most_recent {
        return Ok(vec![ConnEvent::Ignored(0x122f)]);
    }
    if !roles.we_are_server {
        return Ok(vec![ConnEvent::Ignored(0x1234)]);
    }
    if !roles.session_member {
        return Ok(vec![ConnEvent::Ignored(0x123c)]);
    }
    if conn.base.join_order < roles.rank {
        return Ok(vec![ConnEvent::Ignored(0x1242)]);
    }
    conn.reverse_start = r.read_uint(32)? as i32;
    let was_connected = conn.connected;
    conn.connected = true;
    conn.reset_backup_senders_to_client(node)?;
    Ok(vec![ConnEvent::Reconnected { reverse_start: conn.reverse_start, was_connected }])
}

/// 0x75A730 ServerDisconnect reader (empty body). The heartbeat port's
/// Timeout (0x75A250) must run before +0x9C is set, as natively.
pub fn read_server_disconnect(
    conn: &mut AdminConn,
    roles: Roles,
    timeout: &mut dyn FnMut(&mut AdminConn) -> Result<()>,
) -> Result<Vec<ConnEvent>> {
    if roles.conn_is_server {
        timeout(conn)?;
        conn.server_disconnected = true;
        Ok(vec![ConnEvent::ServerDisconnected])
    } else {
        Ok(Vec::new())
    }
}

/// 0x74F7C0 ClientDisconnectRequest reader (empty body).
pub fn read_client_disconnect_request(conn: &mut AdminConn, roles: Roles) -> Vec<ConnEvent> {
    if !roles.we_are_server {
        return vec![ConnEvent::Ignored(0x1424)];
    }
    conn.server_disconnected = true;
    if !roles.we_are_host {
        return vec![ConnEvent::Ignored(0x1434)];
    }
    if conn.base.player.is_none() {
        return vec![ConnEvent::Ignored(0x143b)];
    }
    vec![ConnEvent::ClientLeaveRequested { reason: 5 }]
}

/// 0x74C380 ClientDisconnectDecree: 4-bit reason.
pub fn write_disconnect_decree(conn: &AdminConn, w: &mut WireWriter) -> Result<()> {
    w.write_uint(conn.disconnect_reason, 4).map_err(Error::from)
}

/// 0x75A780 ClientDisconnectDecree reader.
pub fn read_disconnect_decree(r: &mut Cursor<'_>, roles: Roles) -> Result<Vec<ConnEvent>> {
    if roles.we_are_server {
        return Ok(vec![ConnEvent::Ignored(0x13b4)]);
    }
    if (!roles.conn_is_server && !roles.connecting) || (!roles.conn_is_host && roles.connecting) {
        return Ok(vec![ConnEvent::Ignored(0x13bc)]);
    }
    let reason = r.read_possibility(DISCONNECT_REASONS)?;
    Ok(vec![ConnEvent::DisconnectedByServer { reason, take_over: roles.session_state_set }])
}

/// Applies a channel call emitted by the heartbeat port
/// ([`crate::heartbeat::Effect::Channel`], from SetServer 0x759790 /
/// SetHost 0x757010 / AdminConn reset 0x752110) to this connection's
/// channel objects. The packet writers use fixed bindings (sender/receiver
/// to history, as 0x752300/0x7523A0/0x752490/0x7525B0 set them); a call
/// naming any other history is a typed error rather than a silent rebind.
pub fn apply_channel_op(
    conn: &mut AdminConn,
    conn_index: usize,
    node: &mut NodeHistories<'_>,
    channel: crate::heartbeat::Channel,
    op: crate::heartbeat::ChannelOp,
    file_send: &mut dyn FileSending,
    file_receive: &mut dyn FileReceiving,
) -> Result<()> {
    use crate::heartbeat::{Channel as C, ChannelOp as O, HistoryRef as H};
    let own = |h: H| -> bool {
        matches!(h, H::ConnRequests(i) | H::ConnBackupRequests(i) | H::ConnPrivateDecrees(i) | H::ConnReverseDecrees(i) if i == conn_index)
            || matches!(h, H::NodeDecrees | H::NodeBackupDecrees)
    };
    let mismatch = || Error::Unsupported("channel bound to a history other than its fixed packet-layer binding");
    match (channel, op) {
        (C::FileSender, O::FileSenderReset) => return file_send.reset(),
        (C::FileReceiver, O::FileReceiverReset) => return file_receive.reset(),
        (_, O::SenderBind { history, .. }) | (_, O::ReceiverBind { history }) if !own(history) => return Err(mismatch()),
        _ => {}
    }
    let sender = match channel {
        C::BroadcastDecreeSender => Some((&mut conn.decree_out, H::NodeDecrees)),
        C::ReverseDecreeSender => Some((&mut conn.reverse_out, H::NodeDecrees)),
        C::PrivateDecreeSender => Some((&mut conn.private_out, H::ConnPrivateDecrees(conn_index))),
        C::BackupDecreeSender => Some((&mut conn.backup_decree_out, H::NodeBackupDecrees)),
        // Request senders bind node histories (+0xC / +0x1C) that the
        // heartbeat port never names.
        C::BroadcastRequestSender | C::BackupRequestSender => None,
        _ => None,
    };
    if let Some((s, fixed)) = sender {
        return match op {
            O::SenderReset => {
                s.reset();
                Ok(())
            }
            O::SenderBind { history, start } if history == fixed => s.bind(start),
            _ => Err(mismatch()),
        };
    }
    if let (C::BroadcastRequestSender, O::SenderReset) = (channel, op) {
        conn.request_out.reset();
        return Ok(());
    }
    if let (C::BackupRequestSender, O::SenderReset) = (channel, op) {
        conn.backup_request_out.reset();
        return Ok(());
    }
    let receiver = match channel {
        C::BroadcastDecreeReceiver => Some((&mut conn.decree_in, H::NodeDecrees)),
        C::ReverseDecreeReceiver => Some((&mut conn.reverse_in, H::ConnReverseDecrees(conn_index))),
        C::BroadcastRequestReceiver => Some((&mut conn.request_in, H::ConnRequests(conn_index))),
        C::PrivateDecreeReceiver => Some((&mut conn.private_in, H::ConnPrivateDecrees(conn_index))),
        C::BackupDecreeReceiver => Some((&mut conn.backup_decree_in, H::NodeBackupDecrees)),
        C::BackupRequestReceiver => Some((&mut conn.backup_request_in, H::ConnBackupRequests(conn_index))),
        _ => None,
    };
    let Some((r, fixed)) = receiver else { return Err(mismatch()) };
    match op {
        O::ReceiverReset => {
            r.reset();
            Ok(())
        }
        O::ReceiverBind { history } if history == fixed => {
            let target: &mut History = match history {
                H::NodeDecrees => node.decrees,
                H::NodeBackupDecrees => node.backup_decrees,
                H::ConnRequests(_) => &mut conn.base.requests,
                H::ConnBackupRequests(_) => &mut conn.base.backup_requests,
                H::ConnPrivateDecrees(_) => &mut conn.private_history,
                H::ConnReverseDecrees(_) => &mut conn.reverse_history,
            };
            r.bind(target)
        }
        _ => Err(mismatch()),
    }
}
