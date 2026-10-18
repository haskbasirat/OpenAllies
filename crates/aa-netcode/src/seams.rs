//! `aa_runtime::simulation::NetworkSeams` for the native order/decree layer
//! (aa_orders::net::seams::NetworkCore). Ported and oracle-backed in
//! aa-orders: sync-log routing and hashing, client/host sync flushes
//! (302/299 emitters), admin history 0x752DA0, host request execution
//! 0x752E60/0x751B60 with every ProcessRequest, host history recording
//! 0x752EA0, client replay 0x74FD10, film record 0x74D640 and playback
//! 0x751A40.
//!
//! Supplied by the session through [`SessionSeams`] (each must be a real
//! implementation, never a default success): net pump 0x776540, command
//! processing 0x75C9A0, the real admin clock (AdminNode+0x158 vfunc +8),
//! AVI pump, timing smoothing 0x41C180, end-condition check 0x76F600, host
//! rate 0x75FD80, heartbeat 0x75B820, backup clients, backup-mode entry and
//! the film recorder/source streams (AdminNode+0x268 / +0x26C).
use aa_orders::net::seams::{GameData, GameEffects, NetworkCore, TickLog};
use aa_runtime::simulation::{BackupClient, Error, NetworkSeams, Result, SyncLog};
use aa_simulation::FloatBits;

pub trait SessionSeams {
    fn net_pump(&mut self) -> Result<()>;
    fn process_commands(&mut self) -> Result<()>;
    fn real_admin_clock(&mut self) -> Result<FloatBits>;
    fn avi_pump(&mut self) -> Result<i32>;
    fn timing_smoothing(&mut self) -> Result<()>;
    fn end_condition_check(&mut self) -> Result<()>;
    fn host_rate(&mut self, seconds: FloatBits) -> Result<()>;
    fn heartbeat(&mut self) -> Result<()>;
    fn backup_clients(&mut self) -> Result<Vec<BackupClient>>;
    fn backup_mode_entered(&mut self) -> Result<()>;
    /// AdminNode+0x268 recorder vfunc +4 receives `[u32 LE length][bytes]`.
    fn film_record(&mut self, record: Vec<u8>) -> Result<()>;
    /// AdminNode+0x26C source: the next `[u32 LE length][bytes]` record.
    fn film_next(&mut self) -> Result<Vec<u8>>;
}

pub struct Network<'a, G: GameEffects, D: GameData, S: SessionSeams> {
    pub core: &'a mut NetworkCore,
    pub game: &'a mut G,
    pub data: &'a D,
    pub session: &'a mut S,
}

fn seam(e: aa_orders::Error) -> Error {
    Error::Seam(e.to_string())
}

/// SyncLog (aa-runtime) -> TickLog (aa-orders): same native log points.
pub fn tick_log(entry: SyncLog) -> TickLog {
    match entry {
        SyncLog::NextTickEventAdminTime(v) => TickLog::NextTickEventAdminTime(v.0),
        SyncLog::GameTimeBeginTick { second_log, value } => TickLog::GameTimeBeginTick { second_log, value: value.0 },
        SyncLog::IsFrozen(b) => TickLog::IsFrozen(b),
        SyncLog::AdminTickEndGameTime(v) => TickLog::AdminTickEndGameTime(v.0),
        SyncLog::AdminTickEndAdminTime(v) => TickLog::AdminTickEndAdminTime(v.0),
        SyncLog::GameTimePerTick(v) => TickLog::GameTimePerTick(v.0),
        SyncLog::Frozen(b) => TickLog::Frozen(b),
    }
}

impl<G: GameEffects, D: GameData, S: SessionSeams> NetworkSeams for Network<'_, G, D, S> {
    fn net_pump(&mut self) -> Result<()> {
        self.session.net_pump()
    }
    fn process_commands(&mut self) -> Result<()> {
        self.session.process_commands()
    }
    fn real_admin_clock(&mut self) -> Result<FloatBits> {
        self.session.real_admin_clock()
    }
    fn avi_pump(&mut self) -> Result<i32> {
        self.session.avi_pump()
    }
    fn sync_log(&mut self, entry: SyncLog) -> Result<()> {
        self.core.tick_log(tick_log(entry));
        Ok(())
    }
    fn replay_record(&mut self) -> Result<()> {
        let record = self.core.replay_record().map_err(seam)?;
        self.session.film_record(record)
    }
    fn replay_playback(&mut self) -> Result<()> {
        let record = self.session.film_next()?;
        self.core.replay_playback(&record, self.game, self.data).map_err(seam)
    }
    fn timing_smoothing(&mut self) -> Result<()> {
        self.session.timing_smoothing()
    }
    fn end_condition_check(&mut self) -> Result<()> {
        self.session.end_condition_check()
    }
    fn client_sync_flush(&mut self) -> Result<()> {
        self.core.client_sync_flush(self.data).map(drop).map_err(seam)
    }
    fn host_sync_flush(&mut self) -> Result<()> {
        self.core.host_sync_flush(self.data).map(drop).map_err(seam)
    }
    fn admin_history(&mut self) -> Result<()> {
        self.core.admin_history().map(drop).map_err(seam)
    }
    fn host_rate(&mut self, seconds: FloatBits) -> Result<()> {
        self.session.host_rate(seconds)
    }
    fn host_execute_requests(&mut self) -> Result<()> {
        self.core.host_execute_requests(self.game, self.data).map_err(seam)
    }
    fn host_record_history(&mut self) -> Result<()> {
        self.core.host_record_history(self.game).map(drop).map_err(seam)
    }
    fn client_replay_history(&mut self, index: i32) -> Result<()> {
        self.core.client_replay_history(index, self.game, self.data).map_err(seam)
    }
    fn heartbeat(&mut self) -> Result<()> {
        self.session.heartbeat()
    }
    fn backup_clients(&mut self) -> Result<Vec<BackupClient>> {
        self.session.backup_clients()
    }
    fn backup_mode_entered(&mut self) -> Result<()> {
        self.session.backup_mode_entered()
    }
}
