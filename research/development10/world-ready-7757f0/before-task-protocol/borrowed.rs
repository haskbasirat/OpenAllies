//! One frame kernel over direct loans of the retained Node/Sim fields.
//! The laboratory Driver adapter and a live Host use this SAME algorithm.
//! No mutable Node/OrderAdmin state is copied into a second Driver object.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeFlag {
    Running,
    HasAdminClock,
    Disconnecting,
    BackupMode,
    ReplayRecorder,
    ReplaySource,
    IsServer,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeWord {
    AdminTime,
    DisconnectStarted,
    BackupSince,
    HeartbeatAt,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateFlag {
    SessionF4,
    SinglePlayerAutoPause,
    RenderDumpAvi,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateWord {
    SessionMode,
    FlagsB396C4,
    AviFramesPerTick,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeWord {
    BeginGame,
    BeginAdmin,
    EndGame,
    EndAdmin,
}

/// All getters read the actual current sibling owner at this instruction
/// boundary. Unknown fields return a required error when reached.
pub trait Host: NetworkSeams {
    fn role(&self) -> Result<Role>;
    fn flag(&self, field: NodeFlag) -> Result<bool>;
    fn word(&self, field: NodeWord) -> Result<FloatBits>;
    fn store_flag(&mut self, field: NodeFlag, value: bool) -> Result<()>;
    fn store_word(&mut self, field: NodeWord, value: FloatBits) -> Result<()>;
    fn processed(&self) -> Result<i32>;
    fn received(&self) -> Result<i32>;
    fn gate_flag(&self, field: GateFlag) -> Result<bool>;
    fn gate_word(&self, field: GateWord) -> Result<u32>;
    fn async_ticks_per_second(&self) -> Result<FloatBits>;
    /// Whole74FD10, including its final+20C increment, on the SAME actual
    /// OrderAdmin. The laboratory adapter appends that store to its seam.
    fn replay_client_tick(&mut self) -> Result<()>;
}
/// Same Sim fields; absent native time words remain absent until read by an
/// enabled/unfrozen path. No raw, zero-filled Timeline conversion is required.
pub trait SimFields {
    fn frozen(&self) -> bool;
    /// The existing SyncLog API covers canonical Bool bytes. A raw-byte owner
    /// must refuse a noncanonical log value rather than silently map128 to1.
    fn frozen_for_log(&self) -> Result<bool> {
        Ok(self.frozen())
    }
    fn paused(&self) -> bool;
    fn time(&self, field: TimeWord) -> Result<FloatBits>;
    fn store_time(&mut self, field: TimeWord, value: FloatBits) -> Result<()>;
    fn game_per_admin_tick(&self) -> Result<FloatBits>;
    fn avi_frames(&self) -> i32;
    fn store_avi_frames(&mut self, value: i32);
    fn store_frozen(&mut self, value: bool);
    fn store_maximum(&mut self, value: FloatBits);
}
impl SimFields for Timeline {
    fn frozen(&self) -> bool {
        self.frozen
    }
    fn paused(&self) -> bool {
        self.paused
    }
    fn time(&self, f: TimeWord) -> Result<FloatBits> {
        Ok(match f {
            TimeWord::BeginGame => self.begin_game,
            TimeWord::BeginAdmin => self.begin_admin,
            TimeWord::EndGame => self.end_game,
            TimeWord::EndAdmin => self.end_admin,
        })
    }
    fn store_time(&mut self, f: TimeWord, v: FloatBits) -> Result<()> {
        match f {
            TimeWord::BeginGame => self.begin_game = v,
            TimeWord::BeginAdmin => self.begin_admin = v,
            TimeWord::EndGame => self.end_game = v,
            TimeWord::EndAdmin => self.end_admin = v,
        };
        Ok(())
    }
    fn game_per_admin_tick(&self) -> Result<FloatBits> {
        Ok(self.game_per_admin_tick)
    }
    fn avi_frames(&self) -> i32 {
        self.avi_frames
    }
    fn store_avi_frames(&mut self, v: i32) {
        self.avi_frames = v
    }
    fn store_frozen(&mut self, v: bool) {
        self.frozen = v
    }
    fn store_maximum(&mut self, v: FloatBits) {
        self.max_speed = v
    }
}
/// Persistent process sync counters and frame failure latch. Construction
/// from actual process-global image/reset belongs to its application owner;
/// this loan never resets or replaces either field.
pub struct Control<'a> {
    pub counters: &'a mut SyncCounters,
    pub stopped: &'a mut bool,
}
pub struct Kernel<'a, T: SimFields + ?Sized> {
    pub sim: &'a mut T,
    pub control: Control<'a>,
    pub precision: Precision,
}
impl<T: SimFields + ?Sized> Kernel<'_, T> {
    fn enabled<H: Host + ?Sized>(host: &H) -> Result<bool> {
        Ok(host.gate_flag(GateFlag::SessionF4)?
            || host.gate_word(GateWord::FlagsB396C4)? & 2 != 0
            || !host.gate_flag(GateFlag::SinglePlayerAutoPause)?)
    }
    fn backup<H: Host + ?Sized>(host: &H) -> Result<bool> {
        Ok(host.gate_flag(GateFlag::SessionF4)?
            && host.gate_word(GateWord::SessionMode)? != 0
            && host.flag(NodeFlag::BackupMode)?)
    }
    fn next_event<H: Host + ?Sized, W: WorldSeams>(&self, host: &H, world: &W) -> Result<Binary> {
        if !(Self::enabled(host)? && !self.sim.paused() && !self.sim.frozen()) {
            return num(NO_TICK_EVENT_BITS);
        }
        let begin = num(self.sim.time(TimeWord::BeginGame)?.0)?;
        let end = num(self.sim.time(TimeWord::EndGame)?.0)?;
        if begin.compare(&end).is_eq() {
            return num(NO_TICK_EVENT_BITS);
        }
        let admin_span = num(self.sim.time(TimeWord::EndAdmin)?.0)?.sub(
            &num(self.sim.time(TimeWord::BeginAdmin)?.0)?,
            self.precision,
        )?;
        let span = end.sub(&begin, self.precision)?;
        let slope = num(admin_span.div(&span, self.precision)?.store_f32()?)?;
        let Some(current) = world.world_clock() else {
            return num(NO_TICK_EVENT_BITS);
        };
        Ok(num(current.end.0)?
            .sub(&begin, self.precision)?
            .mul(&slope, self.precision)?
            .add(
                &num(self.sim.time(TimeWord::BeginAdmin)?.0)?,
                self.precision,
            )?)
    }
    fn gate<H: Host + ?Sized, W: WorldSeams>(&self, host: &H, world: &W) -> Result<FloatBits> {
        let admin = host.word(NodeWord::AdminTime)?;
        let next = self.next_event(host, world)?;
        if next.compare(&num(admin.0)?).is_le() {
            Ok(FloatBits(self.next_event(host, world)?.store_f32()?))
        } else {
            host.word(NodeWord::AdminTime)
        }
    }
    pub fn frame<H: Host + ?Sized, W: WorldSeams>(
        &mut self,
        now: FloatBits,
        host: &mut H,
        world: &mut W,
    ) -> Result<Action> {
        if *self.control.stopped {
            return Err(Error::Stopped);
        }
        *self.control.stopped = true;
        let result = self.frame_inner(now, host, world)?;
        *self.control.stopped = false;
        Ok(result)
    }
    fn frame_inner<H: Host + ?Sized, W: WorldSeams>(
        &mut self,
        now: FloatBits,
        host: &mut H,
        world: &mut W,
    ) -> Result<Action> {
        if host.flag(NodeFlag::Disconnecting)?
            && num(now.0)?
                .sub(
                    &num(host.word(NodeWord::DisconnectStarted)?.0)?,
                    self.precision,
                )?
                .compare(&num(DISCONNECT_TIMEOUT_BITS)?)
                .is_gt()
        {
            return Err(Error::Unsupported(
                "disconnect timeout 0x751770/0x7642A0 admin reset",
            ));
        }
        host.net_pump()?;
        host.process_commands()?;
        if !host.flag(NodeFlag::Running)? || !host.flag(NodeFlag::HasAdminClock)? {
            return Ok(Action::NotRunning);
        }
        let action = self.decide(host, world)?;
        if num(now.0)?
            .compare(&num(host.word(NodeWord::HeartbeatAt)?.0)?)
            .is_gt()
        {
            host.heartbeat()?;
            let next = Binary::from_i32(1)
                .div(&num(host.async_ticks_per_second()?.0)?, self.precision)?
                .add(&num(now.0)?, self.precision)?
                .store_f32()?;
            host.store_word(NodeWord::HeartbeatAt, FloatBits(next))?;
        }
        if host.flag(NodeFlag::IsServer)? {
            let clients = host.backup_clients()?;
            self.server_backup_update(&clients, now, host)?
        }
        Ok(action)
    }
    fn decide<H: Host + ?Sized, W: WorldSeams>(
        &mut self,
        host: &mut H,
        world: &mut W,
    ) -> Result<Action> {
        let gate = self.gate(host, world)?;
        if num(host.real_admin_clock()?.0)?
            .compare(&num(gate.0)?)
            .is_lt()
        {
            return Ok(Action::WaitingForRealTime);
        }
        if host.gate_flag(GateFlag::RenderDumpAvi)? {
            let mut index = 0i32;
            let mut stop = false;
            while index < (host.gate_word(GateWord::AviFramesPerTick)? as i32) {
                if stop {
                    break;
                }
                let produced = host.avi_pump()?;
                if produced == 0 {
                    stop = true
                }
                self.sim
                    .store_avi_frames(self.sim.avi_frames().wrapping_add(produced));
                index += 1;
            }
            if self.sim.avi_frames() < (host.gate_word(GateWord::AviFramesPerTick)? as i32) {
                return Ok(Action::WaitingForAviFrames);
            }
        }
        let admin = host.word(NodeWord::AdminTime)?;
        if self
            .next_event(host, world)?
            .compare(&num(admin.0)?)
            .is_lt()
        {
            let gate = self.gate(host, world)?;
            host.sync_log(SyncLog::NextTickEventAdminTime(gate))?;
            self.world_tick(host, world)?;
            return Ok(Action::WorldTick);
        }
        let allowed = match host.role()? {
            Role::Host => !Self::backup(host)?,
            Role::Client => host.processed()? < host.received()?,
        };
        if !allowed {
            return Ok(Action::WaitingForAdminTick);
        }
        self.admin_tick(host)?;
        let gate = self.gate(host, world)?;
        host.sync_log(SyncLog::NextTickEventAdminTime(gate))?;
        Ok(Action::AdminTick)
    }
    pub fn world_tick<H: Host + ?Sized, W: WorldSeams>(
        &mut self,
        host: &mut H,
        world: &mut W,
    ) -> Result<()> {
        if host.flag(NodeFlag::ReplayRecorder)? {
            host.replay_record()?
        }
        if host.flag(NodeFlag::ReplaySource)? {
            host.replay_playback()?
        }
        if host.gate_flag(GateFlag::RenderDumpAvi)? {
            self.sim.store_avi_frames(0)
        }
        let begin = world.world_clock().ok_or(Error::MissingWorld)?.begin;
        host.sync_log(SyncLog::GameTimeBeginTick {
            second_log: false,
            value: begin,
        })?;
        let begin = world.world_clock().ok_or(Error::MissingWorld)?.begin;
        host.sync_log(SyncLog::GameTimeBeginTick {
            second_log: true,
            value: begin,
        })?;
        host.timing_smoothing()?;
        world.world_update()?;
        host.end_condition_check()
    }
    pub fn admin_tick<H: Host + ?Sized>(&mut self, host: &mut H) -> Result<()> {
        if host.role()? != Role::Host && !Self::backup(host)? {
            self.control.counters.client = self.control.counters.client.wrapping_add(1);
            if self.control.counters.client > SYNC_FLUSH_AFTER {
                host.client_sync_flush()?;
                self.control.counters.client = 0;
            }
        }
        if host.role()? == Role::Host
            && Self::enabled(host)?
            && !self.sim.paused()
            && !self.sim.frozen()
            && !Self::backup(host)?
            && host.gate_word(GateWord::SessionMode)? == 2
        {
            self.control.counters.host = self.control.counters.host.wrapping_add(1);
            if self.control.counters.host > SYNC_FLUSH_AFTER {
                host.host_sync_flush()?;
                self.control.counters.host = 0;
            }
        }
        if !Self::backup(host)? {
            host.admin_history()?
        }
        if host.role()? == Role::Host {
            host.host_rate(FloatBits(HOST_RATE_ARGUMENT_BITS))?;
            host.host_execute_requests()?;
            host.host_record_history()?
        } else {
            host.replay_client_tick()?
        }
        let admin = FloatBits(
            num(host.word(NodeWord::AdminTime)?.0)?
                .add(&num(ADMIN_TICK_SECONDS_BITS)?, self.precision)?
                .store_f32()?,
        );
        host.store_word(NodeWord::AdminTime, admin)?;
        //76B780 logs BEFORE reading/storing any timeline words.
        host.sync_log(SyncLog::IsFrozen(self.sim.frozen_for_log()?))?;
        if !self.sim.frozen() {
            //76B7A3/A6 capture BOTH register values before either A9/AE store.
            //An unavailable second word must not commit a BeginGame write.
            let previous_end_game = self.sim.time(TimeWord::EndGame)?;
            let previous_end_admin = self.sim.time(TimeWord::EndAdmin)?;
            self.sim
                .store_time(TimeWord::BeginGame, previous_end_game)?;
            self.sim
                .store_time(TimeWord::BeginAdmin, previous_end_admin)?;
            let rate = if Self::enabled(host)? && !self.sim.paused() && !self.sim.frozen() {
                self.sim.game_per_admin_tick()?
            } else {
                FloatBits::ZERO
            };
            let end = FloatBits(
                num(rate.0)?
                    .add(&num(self.sim.time(TimeWord::EndGame)?.0)?, self.precision)?
                    .store_f32()?,
            );
            self.sim.store_time(TimeWord::EndAdmin, admin)?;
            self.sim.store_time(TimeWord::EndGame, end)?;
            host.sync_log(SyncLog::AdminTickEndGameTime(end))?;
            host.sync_log(SyncLog::AdminTickEndAdminTime(
                self.sim.time(TimeWord::EndAdmin)?,
            ))?;
        }
        Ok(())
    }
    pub fn server_backup_update<H: Host + ?Sized>(
        &mut self,
        clients: &[BackupClient],
        now: FloatBits,
        host: &mut H,
    ) -> Result<()> {
        if Self::backup(host)? {
            if !clients.iter().any(|c| c.wants_backup() != 0 || c.flag_48) {
                self.set_backup_mode(false, now, host)?
            }
        } else if clients.iter().any(|c| c.wants_backup() != 0) {
            self.set_backup_mode(true, now, host)?
        }
        Ok(())
    }
    pub fn set_backup_mode<H: Host + ?Sized>(
        &mut self,
        enabled: bool,
        now: FloatBits,
        host: &mut H,
    ) -> Result<()> {
        if enabled && !host.flag(NodeFlag::BackupMode)? {
            host.backup_mode_entered()?
        }
        host.store_flag(NodeFlag::BackupMode, enabled)?;
        host.store_word(NodeWord::BackupSince, now)
    }
    pub fn admin_clock_estimate<H: Host + ?Sized, W: WorldSeams>(
        &self,
        real: FloatBits,
        host: &H,
        world: &W,
    ) -> Result<Binary> {
        let admin = host.word(NodeWord::AdminTime)?;
        let next = self.next_event(host, world)?;
        let cap = if next.compare(&num(admin.0)?).is_le() {
            self.next_event(host, world)?
        } else {
            num(host.word(NodeWord::AdminTime)?.0)?
        };
        let real = num(real.0)?;
        Ok(if real.compare(&cap).is_lt() {
            real
        } else {
            cap
        })
    }
    pub fn set_frozen<H: Host + ?Sized, W: WorldSeams>(
        &mut self,
        frozen: bool,
        host: &mut H,
        world: &W,
    ) -> Result<()> {
        if !frozen && world.world_clock().is_none() {
            return Err(Error::MissingWorld);
        }
        if self.sim.frozen() && !frozen {
            let real = host.real_admin_clock()?;
            let admin = FloatBits(self.admin_clock_estimate(real, host, world)?.store_f32()?);
            let prior = FloatBits(
                num(admin.0)?
                    .sub(&Binary::from_i32(1), self.precision)?
                    .store_f32()?,
            );
            self.sim.store_time(
                TimeWord::BeginGame,
                world.world_clock().ok_or(Error::MissingWorld)?.begin,
            )?;
            self.sim.store_time(TimeWord::BeginAdmin, prior)?;
            self.sim.store_time(
                TimeWord::EndGame,
                world.world_clock().ok_or(Error::MissingWorld)?.begin,
            )?;
            self.sim.store_time(TimeWord::EndAdmin, admin)?;
            self.sim.store_maximum(FloatBits(MAX_SPEED_BITS));
        }
        host.sync_log(SyncLog::Frozen(frozen))?;
        self.sim.store_frozen(frozen);
        Ok(())
    }
}

/// Legacy seam-bounded adapter. The actual live Host instead implements the
/// trait on direct field loans and the whole retained OrderAdmin74FD10 call.
pub(super) struct Laboratory<'a, N> {
    pub node: &'a mut AdminNode,
    pub gates: &'a Gates,
    pub async_rate: FloatBits,
    pub net: &'a mut N,
}
impl<N: NetworkSeams> Host for Laboratory<'_, N> {
    fn role(&self) -> Result<Role> {
        Ok(self.node.role)
    }
    fn flag(&self, f: NodeFlag) -> Result<bool> {
        Ok(match f {
            NodeFlag::Running => self.node.running,
            NodeFlag::HasAdminClock => self.node.has_admin_clock,
            NodeFlag::Disconnecting => self.node.disconnecting,
            NodeFlag::BackupMode => self.node.backup_mode,
            NodeFlag::ReplayRecorder => self.node.replay_recorder,
            NodeFlag::ReplaySource => self.node.replay_source,
            NodeFlag::IsServer => self.node.is_server,
        })
    }
    fn word(&self, f: NodeWord) -> Result<FloatBits> {
        Ok(match f {
            NodeWord::AdminTime => self.node.admin_time,
            NodeWord::DisconnectStarted => self.node.disconnect_started,
            NodeWord::BackupSince => self.node.backup_since,
            NodeWord::HeartbeatAt => self.node.heartbeat_at,
        })
    }
    fn store_flag(&mut self, f: NodeFlag, v: bool) -> Result<()> {
        match f {
            NodeFlag::Running => self.node.running = v,
            NodeFlag::HasAdminClock => self.node.has_admin_clock = v,
            NodeFlag::Disconnecting => self.node.disconnecting = v,
            NodeFlag::BackupMode => self.node.backup_mode = v,
            NodeFlag::ReplayRecorder => self.node.replay_recorder = v,
            NodeFlag::ReplaySource => self.node.replay_source = v,
            NodeFlag::IsServer => self.node.is_server = v,
        };
        Ok(())
    }
    fn store_word(&mut self, f: NodeWord, v: FloatBits) -> Result<()> {
        match f {
            NodeWord::AdminTime => self.node.admin_time = v,
            NodeWord::DisconnectStarted => self.node.disconnect_started = v,
            NodeWord::BackupSince => self.node.backup_since = v,
            NodeWord::HeartbeatAt => self.node.heartbeat_at = v,
        };
        Ok(())
    }
    fn processed(&self) -> Result<i32> {
        Ok(self.node.processed)
    }
    fn received(&self) -> Result<i32> {
        Ok(self.node.received)
    }
    fn gate_flag(&self, f: GateFlag) -> Result<bool> {
        Ok(match f {
            GateFlag::SessionF4 => self.gates.session_f4,
            GateFlag::SinglePlayerAutoPause => self.gates.single_player_auto_pause,
            GateFlag::RenderDumpAvi => self.gates.render_dump_avi,
        })
    }
    fn gate_word(&self, f: GateWord) -> Result<u32> {
        Ok(match f {
            GateWord::SessionMode => self.gates.session_mode as u32,
            GateWord::FlagsB396C4 => self.gates.flags_b396c4,
            GateWord::AviFramesPerTick => self.gates.avi_frames_per_tick as u32,
        })
    }
    fn async_ticks_per_second(&self) -> Result<FloatBits> {
        Ok(self.async_rate)
    }
    fn replay_client_tick(&mut self) -> Result<()> {
        self.net.client_replay_history(self.node.processed)?;
        self.node.processed = self.node.processed.wrapping_add(1);
        Ok(())
    }
}
macro_rules! forward {($($name:ident($($arg:ident:$type:ty),*)->$out:ty;)+)=>{$(fn $name(&mut self,$($arg:$type),*)->Result<$out>{self.net.$name($($arg),*)})+}}
impl<N: NetworkSeams> NetworkSeams for Laboratory<'_, N> {
    forward! {net_pump()->();process_commands()->();real_admin_clock()->FloatBits;avi_pump()->i32;
    sync_log(entry:SyncLog)->();replay_record()->();replay_playback()->();timing_smoothing()->();
    end_condition_check()->();client_sync_flush()->();host_sync_flush()->();admin_history()->();
    host_rate(seconds:FloatBits)->();host_execute_requests()->();host_record_history()->();
    client_replay_history(index:i32)->();heartbeat()->();backup_clients()->Vec<BackupClient>;backup_mode_entered()->();}
}
