//! Authoritative frame scheduler: the original admin/world tick driver.
//!
//! Ported from AA.exe (image base 0x400000); evidence and oracle fixtures are in
//! `research/phase8-tick` (`native-tick-driver.json`, README.txt).
//!
//! Native structure (one call of [`Driver::frame`] == one 0x75CA70 call made by
//! `TGC_NetNode::Update` 0x765410 per rendered frame):
//!
//! * At most ONE simulation step per frame: either a world tick
//!   (0x752AE0 -> 0x76F790 -> GWorld update 0x4F9970) or an admin tick
//!   (0x755290). They never run in the same frame.
//! * Admin time (`TGC_AdminNode+0x28C`, binary32) advances by exactly 1/16
//!   (constant 0xA8C084) per admin tick. Player decrees are applied inside the
//!   admin tick: host 0x752E60 (each client's queued requests, list order) then
//!   0x752EA0 records history; client 0x74FD10 replays history entry
//!   `processed`. Replay playback alone applies decrees inside the world-tick
//!   wrapper, before 0x4F9970 (0x751A40). No decree runs inside 0x4F9970.
//! * The `KKC_Sim` timeline (0xB36554) maps game time to admin time over the
//!   last admin tick. A world tick runs when the admin time at which the
//!   world's current end time is reached (0x76AF90) is strictly below the
//!   current admin time; otherwise an admin tick is attempted. The resulting
//!   admin/world interleaving is a deterministic function of the timeline and
//!   x87 precision; wall-clock only delays frames (real-time gate).
//!
//! Wall clock never enters the simulation: it only gates whether a frame acts.
//! Every network/presentation leaf is a required seam ([`NetworkSeams`]); the
//! world update is [`WorldSeams`], implemented for [`crate::clocked::Runtime`].
//! A failed seam stops the driver: effects already committed are not rolled
//! back and the frame is not retried automatically.
//!
//! Threading: the whole frame is sequential. Nothing in it is proven
//! order-independent (shared RNG, linked-list traversal, decree order); see
//! research/phase8-tick/README.txt for the analysis. Natively the frame runs
//! on its own TGC_Thread (callback thunk NetNode+0x40, started at 0x41ABDC),
//! not on the Direct3D thread; see research/phase8-tick/fpu-precision.md.
use crate::clocked::{Runtime, Services};

pub mod speed;
use aa_numeric::{Binary, Precision};
use aa_simulation::{FloatBits, NativeClock};
use serde::{Deserialize, Serialize};
pub use speed::{PowPath, SpeedChange, SpeedContext, SpeedSettings};

/// 0xA8C084: admin time added by every admin tick (binary32 1/16).
pub const ADMIN_TICK_SECONDS_BITS: u32 = 0x3d80_0000;
/// 0xA92F84: binary32 1e12 returned by 0x76AF90 when no world tick is due.
pub const NO_TICK_EVENT_BITS: u32 = 0x5368_d4a5;
/// 0x76DD6E: binary32 64.0 stored as the timeline maximum speed.
pub const MAX_SPEED_BITS: u32 = 0x4280_0000;
/// 0x75543E: argument pushed to the host rate helper 0x75FD80 (binary32 0.1).
pub const HOST_RATE_ARGUMENT_BITS: u32 = 0x3dcc_cccd;
/// 0xA874AC: disconnect timeout (2.5 s) checked at 0x75CA96.
pub const DISCONNECT_TIMEOUT_BITS: u32 = 0x4020_0000;
/// Sync-log flush counters 0xB40908/0xB40904 fire when they exceed 16.
pub const SYNC_FLUSH_AFTER: i32 = 0x10;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    Runtime(#[from] crate::clocked::Error),
    #[error("required frame seam failed: {0}")]
    Seam(String),
    #[error("native path is not ported: {0}")]
    Unsupported(&'static str),
    #[error("x87 value has no exact extended encoding")]
    Extended,
    #[error("ResetAdminTime 0x76DCE0 requires g_gworld")]
    MissingWorld,
    #[error("driver is stopped after a partially committed frame")]
    Stopped,
}
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn num(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}

/// Inputs read by the timeline/driver gates. Session object 0xB36550 is
/// dereferenced unconditionally by 0x76AF90, so it always exists here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Gates {
    /// Session+0xF4 (networked session flag).
    pub session_f4: bool,
    /// Session+0xE4 (session mode; 2 enables host sync-log flushes).
    pub session_mode: i32,
    /// Global 0xB396C4; bit 1 forces the timeline on.
    pub flags_b396c4: u32,
    /// UVar SinglePlayerAutoPause (UVars+0x350).
    pub single_player_auto_pause: bool,
    /// UVar RenderDumpAVI (UVars+0x1C8): frame-locked capture mode.
    pub render_dump_avi: bool,
    /// UVar RenderAVIFramesPerTick (UVars+0x1CC).
    pub avi_frames_per_tick: i32,
}
impl Gates {
    /// Shared prefix of 0x76AF30/0x76AF90/0x76AEE0/0x76B120.
    pub fn timeline_enabled(&self) -> bool {
        self.session_f4 || (self.flags_b396c4 >> 1) & 1 != 0 || !self.single_player_auto_pause
    }
}

/// `KKC_Sim` timeline fields (object 0xB36554, constructor 0x76F240).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timeline {
    /// +0x10/+0x14: game/admin time at the start of the last admin tick.
    pub begin_game: FloatBits,
    pub begin_admin: FloatBits,
    /// +0x18/+0x1C: game/admin time at the end of the last admin tick.
    pub end_game: FloatBits,
    pub end_admin: FloatBits,
    /// +0x20 GameSpeed exponent and +0x24 maximum speed (64.0).
    pub speed: FloatBits,
    pub max_speed: FloatBits,
    /// +0x28 paused and +0x29 frozen (constructor sets frozen).
    pub paused: bool,
    pub frozen: bool,
    /// +0x2C game seconds per admin tick (= speed factor / AdminTicksPerSecond).
    pub game_per_admin_tick: FloatBits,
    /// +0x38 RenderDumpAVI frame counter.
    pub avi_frames: i32,
}

impl Timeline {
    /// Constructor 0x76F240: frozen, unpaused, speed 0, max 64.0 and
    /// +0x2C = 1.0 / AdminTicksPerSecond (SVars+0x20, x87 FDIV then f32 store).
    /// Remaining time fields are allocation-zeroed in the native object.
    pub fn new(admin_ticks_per_second_bits: u32, precision: Precision) -> Result<Self> {
        let rate = Binary::from_i32(1)
            .div(&num(admin_ticks_per_second_bits)?, precision)?
            .store_f32()?;
        Ok(Self {
            begin_game: FloatBits::ZERO,
            begin_admin: FloatBits::ZERO,
            end_game: FloatBits::ZERO,
            end_admin: FloatBits::ZERO,
            speed: FloatBits::ZERO,
            max_speed: FloatBits(MAX_SPEED_BITS),
            paused: false,
            frozen: true,
            game_per_admin_tick: FloatBits(rate),
            avi_frames: 0,
        })
    }

    /// 0x76AF30: game time per admin tick, or +0.0 when disabled/paused/frozen.
    pub fn game_time_per_admin_tick(&self, gates: &Gates) -> FloatBits {
        if gates.timeline_enabled() && !self.paused && !self.frozen {
            self.game_per_admin_tick
        } else {
            FloatBits::ZERO
        }
    }

    /// 0x76AF90 GetNextTickEventAdminTime: admin time at which the world's
    /// current end time (GWorld+0xE4) is reached, as the extended ST0 value.
    /// The slope (end_admin-begin_admin)/(end_game-begin_game) is stored to
    /// binary32 before use; the final multiply/add stay at x87 precision.
    pub fn next_tick_event_admin_time(
        &self,
        gates: &Gates,
        world_end: Option<FloatBits>,
        precision: Precision,
    ) -> Result<Binary> {
        let none = || num(NO_TICK_EVENT_BITS);
        if !(gates.timeline_enabled() && !self.paused && !self.frozen) {
            return none();
        }
        let (begin_game, end_game) = (num(self.begin_game.0)?, num(self.end_game.0)?);
        if begin_game.compare(&end_game).is_eq() {
            return none(); // FUCOMPP equal -> 0x76B011
        }
        // Native stores the slope before testing g_gworld; a missing world
        // returns the constant either way, so the test is done first here.
        let Some(world_end) = world_end else {
            return none();
        };
        let admin_span = num(self.end_admin.0)?.sub(&num(self.begin_admin.0)?, precision)?;
        let game_span = end_game.sub(&begin_game, precision)?;
        let slope = num(admin_span.div(&game_span, precision)?.store_f32()?)?;
        Ok(num(world_end.0)?
            .sub(&begin_game, precision)?
            .mul(&slope, precision)?
            .add(&num(self.begin_admin.0)?, precision)?)
    }

    /// 0x76B780 (argument = new admin time): unless frozen, shift the end of
    /// the last admin tick to its beginning and extend game time by 0x76AF30.
    /// Returns the sync-log entries written (IsFrozen, then end game/admin).
    pub fn end_admin_tick(
        &mut self,
        admin_time: FloatBits,
        gates: &Gates,
        precision: Precision,
    ) -> Result<Vec<SyncLog>> {
        let mut log = vec![SyncLog::IsFrozen(self.frozen)];
        if self.frozen {
            return Ok(log);
        }
        let rate = self.game_time_per_admin_tick(gates);
        let end_game = num(rate.0)?
            .add(&num(self.end_game.0)?, precision)?
            .store_f32()?;
        self.begin_game = self.end_game;
        self.begin_admin = self.end_admin;
        self.end_admin = admin_time;
        self.end_game = FloatBits(end_game);
        log.push(SyncLog::AdminTickEndGameTime(self.end_game));
        log.push(SyncLog::AdminTickEndAdminTime(self.end_admin));
        Ok(log)
    }

    /// 0x76DCE0 KKC_Sim::ResetAdminTime(t), called by SetFrozen(false)
    /// 0x76DD90 when leaving the frozen state. The native assertion requires
    /// g_gworld; its begin time (+0xE0) anchors both game ends.
    pub fn reset_admin_time(
        &mut self,
        admin_time: FloatBits,
        world_begin: Option<FloatBits>,
        precision: Precision,
    ) -> Result<()> {
        let world_begin = world_begin.ok_or(Error::MissingWorld)?;
        let begin_admin = num(admin_time.0)?
            .sub(&Binary::from_i32(1), precision)?
            .store_f32()?;
        self.begin_game = world_begin;
        self.begin_admin = FloatBits(begin_admin);
        self.end_game = world_begin;
        self.end_admin = admin_time;
        self.max_speed = FloatBits(MAX_SPEED_BITS);
        Ok(())
    }

    /// 0x76AEE0: gate used by the host sync-log cadence in 0x755290.
    pub fn host_sync_gate(&self, gates: &Gates) -> bool {
        gates.timeline_enabled() && !self.paused && !self.frozen
    }
}

/// Values the native code writes to its sync logs 0xB3655C/0xB36560.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncLog {
    /// 0x75CBF2/0x75CC9C: min(next tick event, admin time) stored binary32.
    NextTickEventAdminTime(FloatBits),
    /// 0x76F816 (log 0xB3655C) and 0x76F835 (log 0xB36560): GWorld+0xE0.
    GameTimeBeginTick {
        second_log: bool,
        value: FloatBits,
    },
    /// 0x76B797.
    IsFrozen(bool),
    /// 0x76B7D9/0x76B7EF.
    AdminTickEndGameTime(FloatBits),
    AdminTickEndAdminTime(FloatBits),
    /// 0x76F467 "m_game_time_per_tick" after SetGameSpeed.
    GameTimePerTick(FloatBits),
    /// 0x76DE57 "_frozen" in SetFrozen.
    Frozen(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// This node is the host (local node 0xB36558 +0x2C4, SetHost 0x757010):
    /// it orders decrees and runs admin ticks on its own.
    Host,
    /// Admin ticks only replay received decree history.
    Client,
}

/// Per-connection flags read by the server's backup-mode decision 0x759680
/// (client objects on AdminNode+0x2AC): wants-backup bits +0x49 (8), +0x4B
/// (4), +0x4A (2), +0x30 (1), and +0x48 which also keeps backup mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BackupClient {
    pub flag_30: bool,
    pub flag_48: bool,
    pub flag_49: bool,
    pub flag_4a: bool,
    pub flag_4b: bool,
}
impl BackupClient {
    fn wants_backup(&self) -> u32 {
        (u32::from(self.flag_49) << 3)
            + (u32::from(self.flag_4b) << 2)
            + (u32::from(self.flag_4a) << 1)
            + u32::from(self.flag_30)
    }
}

/// `TGC_AdminNode` fields used by the driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminNode {
    pub role: Role,
    /// +0x28C admin time.
    pub admin_time: FloatBits,
    /// +0x2C0 running and +0x158 admin clock present.
    pub running: bool,
    pub has_admin_clock: bool,
    /// +0x2CD disconnecting and +0x2D0 its start time.
    pub disconnecting: bool,
    pub disconnect_started: FloatBits,
    /// +0x20C history entries applied and +0x218 entries available.
    pub processed: i32,
    pub received: i32,
    /// +0x2D4 backup mode (TGC_AdminNode::SetBackupMode 0x757250) and +0x2D8
    /// the net time it was last set. With session f4 and a session mode the
    /// host stops ordering admin ticks while it is set.
    pub backup_mode: bool,
    pub backup_since: FloatBits,
    /// +0x2DC next heartbeat time.
    pub heartbeat_at: FloatBits,
    /// +0x268 replay recorder and +0x26C replay source attached.
    pub replay_recorder: bool,
    pub replay_source: bool,
    /// This node is the server (local node +0x2C8, SetServer 0x759790): it
    /// runs the backup-mode decision 0x759680 at the end of every frame.
    pub is_server: bool,
}
impl AdminNode {
    /// Node state after 0x751770 reset, apart from the running/clock flags.
    pub fn new(role: Role) -> Self {
        Self {
            role,
            admin_time: FloatBits::ZERO,
            running: false,
            has_admin_clock: false,
            disconnecting: false,
            disconnect_started: FloatBits::ZERO,
            processed: -1,
            received: 0,
            backup_mode: false,
            backup_since: FloatBits::ZERO,
            heartbeat_at: FloatBits::ZERO,
            replay_recorder: false,
            replay_source: false,
            is_server: false,
        }
    }
    fn backup_active(&self, gates: &Gates) -> bool {
        gates.session_f4 && gates.session_mode != 0 && self.backup_mode
    }
}

/// What a frame did. Native code does not report this; it is derived from the
/// branch taken in 0x75CA70.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// +0x2C0 clear or no admin clock: return before the heartbeat.
    NotRunning,
    /// Real admin clock below min(next tick event, admin time).
    WaitingForRealTime,
    /// RenderDumpAVI frame count below RenderAVIFramesPerTick.
    WaitingForAviFrames,
    /// World tick not due and no admin tick allowed (host in backup mode or a
    /// client without received history).
    WaitingForAdminTick,
    WorldTick,
    AdminTick,
}

/// Network, decree and presentation seams of the frame. Every method is
/// required: there is no default success for an unported native leaf.
pub trait NetworkSeams {
    /// 0x776540 network pump.
    fn net_pump(&mut self) -> Result<()>;
    /// 0x75C9A0 TGC_AdminNode::ProcessCommands.
    fn process_commands(&mut self) -> Result<()>;
    /// AdminNode+0x158 virtual +8 with argument 1: wall-clock admin time.
    fn real_admin_clock(&mut self) -> Result<FloatBits>;
    /// KKC_Sim+4 virtual +4 (1,0) during RenderDumpAVI: frames produced.
    fn avi_pump(&mut self) -> Result<i32>;
    /// Sync-log writes (0x77D930/0x77D9D0) in native order.
    fn sync_log(&mut self, entry: SyncLog) -> Result<()>;
    /// 0x74D640 (AdminNode+0x268 set): record replay before a world tick.
    fn replay_record(&mut self) -> Result<()>;
    /// 0x751A40 (AdminNode+0x26C set): apply replayed decrees before a world tick.
    fn replay_playback(&mut self) -> Result<()>;
    /// 0x41C180 timing smoothing on 0xB39BB8 (presentation).
    fn timing_smoothing(&mut self) -> Result<()>;
    /// 0x76F600 end-of-session check after the world update.
    fn end_condition_check(&mut self) -> Result<()>;
    /// 0x74E8B0 / 0x74E670 sync-log flushes.
    fn client_sync_flush(&mut self) -> Result<()>;
    fn host_sync_flush(&mut self) -> Result<()>;
    /// 0x752DA0 admin history (skipped in backup mode).
    fn admin_history(&mut self) -> Result<()>;
    /// 0x75FD80 on AdminNode+0x15C with 0.1.
    fn host_rate(&mut self, seconds: FloatBits) -> Result<()>;
    /// 0x752E60: host applies each client's queued decree requests.
    fn host_execute_requests(&mut self) -> Result<()>;
    /// 0x752EA0: host records this admin tick into the decree history.
    fn host_record_history(&mut self) -> Result<()>;
    /// 0x74FD10: client applies decree history entry `index` (+0x20C). The
    /// driver performs the native final `+0x20C += 1` after success.
    fn client_replay_history(&mut self, index: i32) -> Result<()>;
    /// 0x75B820 heartbeat: backup-mode chat/vote requests and decrees,
    /// connection statistics, lag detection, file-transfer status (network
    /// only; it does not touch the simulation).
    fn heartbeat(&mut self) -> Result<()>;
    /// Server only: the connections on AdminNode+0x2AC, in list order.
    fn backup_clients(&mut self) -> Result<Vec<BackupClient>>;
    /// 0x757290..0x7573B3 diagnostics and the GameSpy report 0x791800 when
    /// SetBackupMode(true) leaves normal mode.
    fn backup_mode_entered(&mut self) -> Result<()>;
}

/// The authoritative world update 0x4F9970 and its clock.
pub trait WorldSeams {
    /// g_gworld clock, or None when there is no world.
    fn world_clock(&self) -> Option<NativeClock>;
    /// One complete GWorld update; returns the new clock.
    fn world_update(&mut self) -> Result<NativeClock>;
}

impl<S: Services> WorldSeams for Runtime<S> {
    fn world_clock(&self) -> Option<NativeClock> {
        Some(self.world.clock)
    }
    fn world_update(&mut self) -> Result<NativeClock> {
        Ok(self.step()?)
    }
}

/// Sync-log flush counters (globals 0xB40908 client, 0xB40904 host).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SyncCounters {
    pub client: i32,
    pub host: i32,
}

/// One node's tick driver: admin node, sim timeline and gates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Driver {
    pub node: AdminNode,
    pub timeline: Timeline,
    pub gates: Gates,
    pub counters: SyncCounters,
    /// View SVar NetworkAdminAsyncTicksPerSecond (0xB39990+0x11C).
    pub async_ticks_per_second: FloatBits,
    /// x87 precision control in effect for this code.
    pub precision: Precision,
    stopped: bool,
}

impl Driver {
    pub fn new(
        node: AdminNode,
        timeline: Timeline,
        gates: Gates,
        async_ticks_per_second: FloatBits,
        precision: Precision,
    ) -> Self {
        Self {
            node,
            timeline,
            gates,
            counters: SyncCounters::default(),
            async_ticks_per_second,
            precision,
            stopped: false,
        }
    }
    pub fn stopped(&self) -> bool {
        self.stopped
    }

    /// 0x75CA70 followed by nothing else: one frame of the native driver.
    /// `now` is the net node's wall time (NetNode+0x58, seconds) for the
    /// disconnect timeout and heartbeat; neither feeds the simulation.
    pub fn frame<N: NetworkSeams, W: WorldSeams>(
        &mut self,
        now: FloatBits,
        net: &mut N,
        world: &mut W,
    ) -> Result<Action> {
        if self.stopped {
            return Err(Error::Stopped);
        }
        self.stopped = true;
        let action = self.frame_inner(now, net, world)?;
        self.stopped = false;
        Ok(action)
    }

    fn next_event<W: WorldSeams>(&self, world: &W) -> Result<Binary> {
        self.timeline.next_tick_event_admin_time(
            &self.gates,
            world.world_clock().map(|c| c.end),
            self.precision,
        )
    }

    /// 0x75CAE3..0x75CB1E and 0x75CBA9..0x75CBE4: min(next event, admin
    /// time); the event value is stored to binary32 when it is not greater.
    fn gate<W: WorldSeams>(&self, world: &W) -> Result<FloatBits> {
        let next = self.next_event(world)?;
        let admin = num(self.node.admin_time.0)?;
        Ok(if next.compare(&admin).is_le() {
            FloatBits(next.store_f32()?)
        } else {
            self.node.admin_time
        })
    }

    fn frame_inner<N: NetworkSeams, W: WorldSeams>(
        &mut self,
        now: FloatBits,
        net: &mut N,
        world: &mut W,
    ) -> Result<Action> {
        let pc = self.precision;
        if self.node.disconnecting {
            let elapsed = num(now.0)?.sub(&num(self.node.disconnect_started.0)?, pc)?;
            if elapsed.compare(&num(DISCONNECT_TIMEOUT_BITS)?).is_gt() {
                return Err(Error::Unsupported(
                    "disconnect timeout 0x751770/0x7642A0 admin reset",
                ));
            }
        }
        net.net_pump()?;
        net.process_commands()?;
        if !self.node.running || !self.node.has_admin_clock {
            return Ok(Action::NotRunning);
        }
        let action = self.decide(net, world)?;
        // 0x75CCA1 heartbeat: now > heartbeat_at.
        if num(now.0)?.compare(&num(self.node.heartbeat_at.0)?).is_gt() {
            net.heartbeat()?;
            let next = Binary::from_i32(1)
                .div(&num(self.async_ticks_per_second.0)?, pc)?
                .add(&num(now.0)?, pc)?
                .store_f32()?;
            self.node.heartbeat_at = FloatBits(next);
        }
        if self.node.is_server {
            let clients = net.backup_clients()?;
            self.server_backup_update(&clients, now, net)?;
        }
        Ok(action)
    }

    fn decide<N: NetworkSeams, W: WorldSeams>(
        &mut self,
        net: &mut N,
        world: &mut W,
    ) -> Result<Action> {
        let gate = self.gate(world)?;
        // 0x75CB2B: FCOMP with C0 test; a clock below the gate waits.
        let real = net.real_admin_clock()?;
        if num(real.0)?.compare(&num(gate.0)?).is_lt() {
            return Ok(Action::WaitingForRealTime);
        }
        if self.gates.render_dump_avi {
            self.avi_pump(net)?;
            if self.timeline.avi_frames < self.gates.avi_frames_per_tick {
                return Ok(Action::WaitingForAviFrames);
            }
        }
        // 0x75CB8B: strictly earlier event -> world tick (0x6A71F0 is false).
        let next = self.next_event(world)?;
        if next.compare(&num(self.node.admin_time.0)?).is_lt() {
            net.sync_log(SyncLog::NextTickEventAdminTime(self.gate(world)?))?;
            self.world_tick(net, world)?;
            return Ok(Action::WorldTick);
        }
        let allowed = match self.node.role {
            Role::Host => !self.node.backup_active(&self.gates),
            Role::Client => self.node.processed < self.node.received,
        };
        if !allowed {
            return Ok(Action::WaitingForAdminTick);
        }
        self.admin_tick(net)?;
        net.sync_log(SyncLog::NextTickEventAdminTime(self.gate(world)?))?;
        Ok(Action::AdminTick)
    }

    /// 0x76B030: pump up to RenderAVIFramesPerTick frames, stopping after a
    /// zero result; each result is added to KKC_Sim+0x38.
    fn avi_pump<N: NetworkSeams>(&mut self, net: &mut N) -> Result<()> {
        let mut index = 0;
        let mut stop = false;
        while index < self.gates.avi_frames_per_tick {
            if stop {
                break;
            }
            let produced = net.avi_pump()?;
            if produced == 0 {
                stop = true;
            }
            self.timeline.avi_frames = self.timeline.avi_frames.wrapping_add(produced);
            index += 1;
        }
        Ok(())
    }

    /// 0x752AE0 -> 0x76F790 -> 0x4F9970 (wall-clock statistics of 0x752AE0
    /// at +0x238..+0x244/+0x60 are telemetry and not modelled).
    pub fn world_tick<N: NetworkSeams, W: WorldSeams>(
        &mut self,
        net: &mut N,
        world: &mut W,
    ) -> Result<()> {
        if self.node.replay_recorder {
            net.replay_record()?;
        }
        if self.node.replay_source {
            net.replay_playback()?;
        }
        if self.gates.render_dump_avi {
            self.timeline.avi_frames = 0; // 0x76F7DB
        }
        let begin = world
            .world_clock()
            .map(|c| c.begin)
            .ok_or(Error::Unsupported(
                "0x76F790 dereferences g_gworld; a world tick without a world is undefined",
            ))?;
        net.sync_log(SyncLog::GameTimeBeginTick {
            second_log: false,
            value: begin,
        })?;
        net.sync_log(SyncLog::GameTimeBeginTick {
            second_log: true,
            value: begin,
        })?;
        net.timing_smoothing()?;
        world.world_update()?;
        net.end_condition_check()
    }

    /// 0x755290 admin tick: sync-log cadence, decrees, admin time += 1/16,
    /// then 0x76B780 with the new admin time.
    pub fn admin_tick<N: NetworkSeams>(&mut self, net: &mut N) -> Result<()> {
        let backup = self.node.backup_active(&self.gates);
        let host = self.node.role == Role::Host;
        if !host && !backup {
            self.counters.client = self.counters.client.wrapping_add(1);
            if self.counters.client > SYNC_FLUSH_AFTER {
                net.client_sync_flush()?;
                self.counters.client = 0;
            }
        }
        if host
            && self.timeline.host_sync_gate(&self.gates)
            && !backup
            && self.gates.session_mode == 2
        {
            self.counters.host = self.counters.host.wrapping_add(1);
            if self.counters.host > SYNC_FLUSH_AFTER {
                net.host_sync_flush()?;
                self.counters.host = 0;
            }
        }
        if !backup {
            net.admin_history()?;
        }
        if host {
            net.host_rate(FloatBits(HOST_RATE_ARGUMENT_BITS))?;
            net.host_execute_requests()?;
            net.host_record_history()?;
        } else {
            net.client_replay_history(self.node.processed)?;
            self.node.processed = self.node.processed.wrapping_add(1);
        }
        let admin_time = num(self.node.admin_time.0)?
            .add(&num(ADMIN_TICK_SECONDS_BITS)?, self.precision)?
            .store_f32()?;
        self.node.admin_time = FloatBits(admin_time);
        for entry in
            self.timeline
                .end_admin_tick(self.node.admin_time, &self.gates, self.precision)?
        {
            net.sync_log(entry)?;
        }
        Ok(())
    }

    /// 0x759680 (server node, frame tail): outside an active backup mode any
    /// connection wanting backup enters it; inside it, it is left once no
    /// connection wants backup or holds +0x48 (also with no connections).
    pub fn server_backup_update<N: NetworkSeams>(
        &mut self,
        clients: &[BackupClient],
        now: FloatBits,
        net: &mut N,
    ) -> Result<()> {
        if self.node.backup_active(&self.gates) {
            let holding = clients
                .iter()
                .filter(|c| c.wants_backup() != 0 || c.flag_48)
                .count();
            if holding == 0 {
                self.set_backup_mode(false, now, net)?;
            }
        } else if clients.iter().any(|c| c.wants_backup() != 0) {
            self.set_backup_mode(true, now, net)?;
        }
        Ok(())
    }

    /// 0x757250 TGC_AdminNode::SetBackupMode on the local node.
    pub fn set_backup_mode<N: NetworkSeams>(
        &mut self,
        enabled: bool,
        now: FloatBits,
        net: &mut N,
    ) -> Result<()> {
        if enabled && !self.node.backup_mode {
            net.backup_mode_entered()?;
        }
        self.node.backup_mode = enabled;
        self.node.backup_since = now;
        Ok(())
    }

    /// 0x74B180(1): the admin node's current admin time, min(real clock
    /// stored as binary32, min(0x76AF90, admin time)), as the x87 ST0 value.
    pub fn admin_clock_estimate<W: WorldSeams>(
        &self,
        real: FloatBits,
        world: &W,
    ) -> Result<Binary> {
        let next = self.next_event(world)?;
        let admin = num(self.node.admin_time.0)?;
        let cap = if next.compare(&admin).is_le() {
            next
        } else {
            admin
        };
        let real = num(real.0)?;
        Ok(if real.compare(&cap).is_lt() {
            real
        } else {
            cap
        })
    }

    /// 0x76DD90 KKC_Sim::SetFrozen. Unfreezing a frozen timeline calls
    /// ResetAdminTime 0x76DCE0 with f32(0x74B180(1)) on the local node.
    pub fn set_frozen<N: NetworkSeams, W: WorldSeams>(
        &mut self,
        frozen: bool,
        net: &mut N,
        world: &W,
    ) -> Result<()> {
        if self.timeline.frozen && !frozen {
            let real = net.real_admin_clock()?;
            let t = FloatBits(self.admin_clock_estimate(real, world)?.store_f32()?);
            let begin = world.world_clock().map(|c| c.begin);
            self.timeline.reset_admin_time(t, begin, self.precision)?;
        }
        net.sync_log(SyncLog::Frozen(frozen))?;
        self.timeline.frozen = frozen;
        Ok(())
    }
}

/// Clean entry point for the client and network layer: one call per rendered
/// frame. The world is the authoritative [`Runtime`]; `network` supplies the
/// admin/decree/presentation seams.
pub struct Simulation<S, N> {
    pub runtime: Runtime<S>,
    pub driver: Driver,
    pub network: N,
}
impl<S: Services, N: NetworkSeams> Simulation<S, N> {
    pub fn new(runtime: Runtime<S>, driver: Driver, network: N) -> Self {
        Self {
            runtime,
            driver,
            network,
        }
    }
    /// One native frame (0x75CA70). Returns which step, if any, ran.
    pub fn step(&mut self, now: FloatBits) -> Result<Action> {
        self.driver.frame(now, &mut self.network, &mut self.runtime)
    }
    /// 0x76DD90 KKC_Sim::SetFrozen through the frame seams.
    pub fn set_frozen(&mut self, frozen: bool) -> Result<()> {
        self.driver
            .set_frozen(frozen, &mut self.network, &self.runtime)
    }
}

/// Convert a seam error message into the driver error.
pub fn seam_error(message: impl Into<String>) -> Error {
    Error::Seam(message.into())
}
