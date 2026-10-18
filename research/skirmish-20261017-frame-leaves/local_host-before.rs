//! Fresh local Node751770 -> append-self ->759790 ->757010 ->77D900.
//! There is exactly one OrderAdmin request+0C/decree+214 allocation. This
//! bounded owner does not instantiate the heartbeat laboratory's PeerBase.
//! Selected settings/77A950 and full live transport remain separate producers.
use crate::{Error, Result, history::clear_history};
use aa_numeric::{Binary, Precision};
use aa_orders::{History, Position, commands::WireWriter, net::admin::AdminNode};
use aa_runtime::{
    catalog::Catalog,
    server_clock::{ServerClock, TimeSource},
};
use serde::Serialize;

fn error(e: impl std::fmt::Display) -> Error {
    Error::Protocol(e.to_string())
}

/// The retained59AB60/QPC timer, not Reset's GetTickCount. The platform owns
/// the actual anchor/frequency; fixtures explicitly inject this OS input.
pub trait MillisecondTimer {
    fn milliseconds(&mut self) -> std::result::Result<u32, String>;
}
/// Reached NetNode765AB0 timing fields. +58 is constructor1e12, +64 is0;
/// uninitialized +5C is intentionally absent. The same timer lives throughout.
pub struct NodeTime<T> {
    timer: T,
    cached_58: u32,
    behind_64: u8,
}
impl<T: MillisecondTimer> NodeTime<T> {
    pub fn from_native_constructor(timer: T) -> Self {
        Self {
            timer,
            cached_58: 0x5368d4a5,
            behind_64: 0,
        }
    }
    pub fn cached_seconds_bits(&self) -> u32 {
        self.cached_58
    }
    pub fn falling_behind(&self) -> u8 {
        self.behind_64
    }
    /// Exact765410..765452 prefix, ending at its first cached-time write.
    /// This does not run or claim the remainder of the NetNode frame.
    pub fn begin_update_prefix(&mut self, p: Precision) -> Result<()> {
        let ms = self.timer.milliseconds().map_err(error)?;
        let mut value = Binary::from_i32(ms as i32);
        if (ms as i32) < 0 {
            value = value
                .add(&Binary::from_f32_bits(0x4f800000).map_err(error)?, p)
                .map_err(error)?;
        }
        // Unlike ServerClock's ms helper, this native prefix DOES NOT spill
        // the integer conversion before FMUL9F4580 (binary32 .001).
        self.cached_58 = value
            .mul(&Binary::from_f32_bits(0x3a83126f).map_err(error)?, p)
            .map_err(error)?
            .store_f32()
            .map_err(error)?;
        Ok(())
    }
}
impl<T: MillisecondTimer> TimeSource for NodeTime<T> {
    fn cached_seconds_bits(&mut self) -> std::result::Result<u32, String> {
        Ok(self.cached_58)
    }
    fn milliseconds(&mut self) -> std::result::Result<u32, String> {
        self.timer.milliseconds()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Stage {
    BeforeSessionReset,
    Reset,
    StartingSelfHost,
    SelfHost,
}

/// Only fields reached by the proved fresh-self path. No Clone/Default and no
/// artificial network/player/packet cap. Other positive peer services must be
/// bound before a future producer can add such peers to this path.
pub struct Node {
    pub(crate) orders: AdminNode,
    ///752A60 constructs the actual AdminPeer+64/+68 vote-to-kick list empty.
    vote_to_kick: std::collections::VecDeque<crate::lobby::PeerLink>,
    backup_requests: History,
    backup_decrees: History,
    next_backup_decree: i32,
    pub(crate) server_is_self: bool,
    pub(crate) has_clock: bool,
    pub(crate) clock: ServerClock,
    patience: Option<u32>,
    flags: [u8; 6], //30,31,48,49,4A,4B
    pub(crate) order_version: u8,
    pub(crate) mirror_268: MirrorAttachment,
    vote: u8,
    votes: i32,
    max_patience: u32,
    rtt: u32,
    received_fraction: u32,
    pub(crate) backup_mode: bool,
    pub(crate) backup_since: u32,
    pub(crate) admin_time_28c: u32,
    pub(crate) disconnecting_2cd: bool,
    pub(crate) disconnect_started_2d0: u32,
    pub(crate) heartbeat_2dc: u32,
    pub(crate) playback_26c: PlaybackSource,
    stage: Stage,
    pub(crate) vworld_init_required_8c:Option<u8>,
    pub(crate) peer_task_34:u32,
    ///No constructor value is assumed;74FEC0/74FF50 must actually store it.
    pub(crate) task_status_90:Option<u32>,
    pub(crate) task_history_238:crate::local_bootstrap::world_ready::tasks::TaskHistory,
    pub(crate) order_abort_2e5:crate::local_bootstrap::order_gate::Byte,
    ///75CE80/75D1AE initializes the retained host-migration word to zero.
    pub(crate) takeover_2e0:u32,
    pub(crate) commands_118:crate::local_bootstrap::node_commands::State,
}
struct PrivatePlayback;
/// Actual75D07A null playback pointer; no positive replay owner is invented.
pub struct PlaybackSource {
    target: Option<PrivatePlayback>,
}
impl PlaybackSource {
    pub fn is_attached(&self) -> bool {
        self.target.is_some()
    }
}
/// Direct sibling loans used by the shared frame kernel. Every mutable word
/// refers to the retained Node field; no detached Driver state or copyback.
pub struct FrameFields<'a> {
    pub running_2c0: &'a mut bool,
    pub has_clock_158: &'a bool,
    pub server_is_self_2c8: &'a bool,
    pub admin_time_28c: &'a mut u32,
    pub disconnecting_2cd: &'a mut bool,
    pub disconnect_started_2d0: &'a mut u32,
    pub heartbeat_2dc: &'a mut u32,
    pub backup_mode_2d4: &'a mut bool,
    pub backup_since_2d8: &'a mut u32,
    pub playback_26c: &'a PlaybackSource,
}
/// Native75D074 constructor-null recorder attachment. A positive handle is
/// nonowning and always points to the SAME Session1C film stream.
pub struct MirrorAttachment {
    target: Option<aa_runtime::film_resource::RecorderHandle>,
}
impl MirrorAttachment {
    pub fn is_attached(&self) -> bool {
        self.target.is_some()
    }
    pub fn recorder(&self) -> Option<&aa_runtime::film_resource::RecorderHandle> {
        self.target.as_ref()
    }
}
fn zero_history(h: &mut History) -> Result<()> {
    clear_history(h)?;
    h.retire_before(0)?;
    Ok(())
}
fn reset_stream(s: &mut WireWriter) -> Result<()> {
    *s = WireWriter::new(Position::PACKET_START)?;
    Ok(())
}
impl Node {
    ///74DA10 removes each64-list node, clearing68 when the last is removed.
    ///Typed growable storage owns both ends; no raw zero mirrors survive.
    pub(crate) fn clear_votes_74da10(&mut self) {
        while self.vote_to_kick.pop_front().is_some() {}
    }
    pub(crate) fn vote_count_64(&self) -> usize {
        self.vote_to_kick.len()
    }
    #[cfg(test)]
    pub(crate) fn fixture_votes(&mut self, values: Vec<crate::lobby::PeerLink>) {
        self.vote_to_kick = values.into();
    }
    pub fn has_replay_source(&self) -> bool {
        self.playback_26c.is_attached()
    }
    ///74D5F0 stores the actual268 pointer before resetting only the SAME
    ///274/278/27C mirror writer. Client250/resume fields are not touched.
    pub fn attach_recorder_74d5f0(
        &mut self,
        recorder: aa_runtime::film_resource::RecorderHandle,
    ) -> Result<()> {
        self.mirror_268.target = Some(recorder);
        reset_stream(&mut self.orders.mirror_stream)
    }
    ///74B310 only clears the borrowed recorder pointer; queued mirror bits
    ///and the Session1C resource remain owned by their original allocations.
    pub fn detach_recorder_74b310(&mut self) {
        self.mirror_268.target = None;
    }
    pub fn recorder_268(&self) -> Option<&aa_runtime::film_resource::RecorderHandle> {
        self.mirror_268.recorder()
    }
    /// Source75CE80 constructor dependencies and reached751770 prefix before
    ///76EC70. Fields intentionally have no native-use access while incomplete.
    pub(crate) fn before_session_reset() -> Result<Self> {
        let mut orders = AdminNode::new(i32::MIN)?;
        // Actual ctor+20C=0, overwritten by751770 suffix only after Reset.
        orders.client.next_index = 0;
        zero_history(&mut orders.requests)?;
        reset_stream(&mut orders.request_stream)?;
        zero_history(&mut orders.decrees)?;
        orders.client.resume_byte = -1;
        orders.client.resume_bit = -1;
        orders.client.field_250 = 0;
        reset_stream(&mut orders.decree_stream)?;
        Ok(Self {
            orders,
            vote_to_kick: std::collections::VecDeque::new(),
            backup_requests: History::new(i32::MIN, 0x100)?,
            backup_decrees: History::new(i32::MIN, 0x100)?,
            next_backup_decree: 0,
            server_is_self: false,
            has_clock: false,
            clock: ServerClock::from_native_constructor(),
            patience: None,
            flags: [0; 6],
            order_version: 1,
            mirror_268: MirrorAttachment { target: None },
            vote: 0,
            votes: 0,
            max_patience: 0,
            rtt: 0,
            received_fraction: 0,
            backup_mode: false,
            backup_since: 0,
            admin_time_28c: 0,                             //75D0DB
            disconnecting_2cd: false,                      //75D190
            disconnect_started_2d0: 0,                     //75D196
            heartbeat_2dc: 0,                              //75D1A8
            playback_26c: PlaybackSource { target: None }, //75D07A
            stage: Stage::BeforeSessionReset,
            vworld_init_required_8c:None, //No reached constructor writer proved.
            peer_task_34:0, //752A60 ->7514F0 actual client Reset.
            task_status_90:None,
            task_history_238:crate::local_bootstrap::world_ready::tasks::TaskHistory::from_native_constructor(),
            order_abort_2e5:crate::local_bootstrap::order_gate::Byte::from_native_constructor_zero(),
            takeover_2e0:0, //75CEB2 xor EBX;75D1AE stores EBX (no Reset substitute).
            commands_118:crate::local_bootstrap::node_commands::State::from_native_constructor(),
        })
    }
    pub(crate) fn after_session_reset(&mut self) -> Result<()> {
        if self.stage != Stage::BeforeSessionReset {
            return Err(error("Node Reset suffix is out of order"));
        }
        reset_stream(&mut self.orders.mirror_stream)?;
        self.has_clock = false;
        self.admin_time_28c = 0; //7518BB
        self.disconnecting_2cd = false; //7518CD
        self.heartbeat_2dc = 0; //751907;751770 leaves2D0 unchanged
        reset_stream(&mut self.orders.decree_stream)?;
        self.orders.client.next_index = -1;
        self.orders.last_request_player = None;
        self.orders.client.field_284 = 0;
        self.orders.film_ordering_player = None;
        self.backup_mode = false;
        self.backup_since = 0;
        self.server_is_self = false;
        self.patience = Some(0x41700000);
        self.stage = Stage::Reset;
        Ok(())
    }
    pub(crate) fn require_reset(&self) -> Result<()> {
        if self.stage == Stage::Reset {
            Ok(())
        } else {
            Err(error(
                "fresh self Host cannot retry a started/incomplete Node",
            ))
        }
    }
    /// Caller has just appended this same Node to its formerly empty list.
    /// No unrelated eager heartbeat Env/config/default is read here.
    pub(crate) fn set_self_server<T: MillisecondTimer>(
        &mut self,
        time: &NodeTime<T>,
    ) -> Result<()> {
        self.require_reset()?;
        self.stage = Stage::StartingSelfHost;
        self.server_is_self = true;
        zero_history(&mut self.backup_decrees)?;
        self.next_backup_decree = 0;
        zero_history(&mut self.backup_requests)?;
        self.patience = Some(0x41700000);
        //74F1A0: one self peer; max helper74DD80 skips self virtual+4==null.
        self.votes = i32::from(self.vote != 0);
        self.max_patience = 0;
        self.rtt = 0;
        self.received_fraction = 0;
        self.flags[5] = 0;
        self.flags[3] = time.falling_behind();
        self.flags[4] = 0;
        self.flags[0] = self.flags[1];
        self.flags[2] = 0;
        // Fresh SessionF4/E4/Node2D4 make759680 take its ordinary branch.
        // A positive native condition reaches757250; retain the complete
        // preceding refresh but do not pretend its services were executed.
        if self.flags[0] != 0 || self.flags[3] != 0 {
            return Err(Error::Unsupported(
                "757250 positive backup-mode entry on fresh local Host",
            ));
        }
        Ok(())
    }
    pub(crate) fn set_self_host<T: MillisecondTimer>(
        &mut self,
        time: &mut NodeTime<T>,
        rate: impl FnOnce() -> Result<u32>,
        p: Precision,
    ) -> Result<()> {
        if self.stage != Stage::StartingSelfHost || !self.server_is_self {
            return Err(error("SetHost requires actual completed SetServer"));
        }
        //74DBC0 writes this real peer6C before reading SVar or QPC.
        self.order_version = 1;
        let rate = rate()?;
        let period = crate::heartbeat::x87_div_store(1.0, f32::from_bits(rate)).to_bits();
        // Host caller force-stores null2C4 before757010. Thus no old-clock
        // read or NewHost message occurs; start is the native positive zero.
        self.clock.restart(period, 0, time, p).map_err(error)?;
        self.has_clock = true;
        Ok(())
    }
    pub(crate) fn after_host_bound<T: MillisecondTimer>(
        &mut self,
        time: &NodeTime<T>,
    ) -> Result<()> {
        if !self.has_clock || self.stage != Stage::StartingSelfHost {
            return Err(error("Host clock binding incomplete"));
        }
        self.backup_mode = false;
        self.backup_since = time.cached_seconds_bits();
        self.stage = Stage::SelfHost;
        Ok(())
    }
    pub fn clock(&self) -> &ServerClock {
        &self.clock
    }
    pub fn clock_mut(&mut self) -> &mut ServerClock {
        &mut self.clock
    }
    pub fn stage(&self) -> Stage {
        self.stage
    }
    pub fn has_admin_clock(&self) -> bool {
        self.has_clock
    }
    pub fn server_is_self(&self) -> bool {
        self.server_is_self
    }
    pub fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({"stage":self.stage,"requests":{"begin":self.orders.requests.begin(),"end":self.orders.requests.end()},
            "backup_requests":{"begin":self.backup_requests.begin(),"end":self.backup_requests.end()},
            "decrees":{"begin":self.orders.decrees.begin(),"end":self.orders.decrees.end()},
            "backup_decrees":{"begin":self.backup_decrees.begin(),"end":self.backup_decrees.end()},
            "client_next":self.orders.client.next_index,"last_request_player":self.orders.last_request_player,
            "ordering_player":self.orders.client.field_284,"clock_selected":self.has_clock,"clock_null":!self.has_clock,
            "server_is_node":self.server_is_self,"backup_mode":u8::from(self.backup_mode),"backup_since":self.backup_since,
            "vote_count":self.votes,"max_patience":self.max_patience,"own_patience":self.patience,
            "order_version":self.order_version,"own_flags":self.flags,"own_rtt":self.rtt,"own_received_fraction":self.received_fraction})
    }
}

/// Read the actual source-created stored SVar at its reached757185 boundary.
/// Its conversion PC belongs to the completed Bank; host arithmetic PC is a
/// separate argument. Current Catalog/source ownership is checked by the same
/// numeric binder, not a caller's guessed Double/Single setting.
pub(crate) fn admin_rate(catalog: &Catalog<'_>) -> Result<u32> {
    aa_runtime::load_host_settings::admin_ticks_per_second(catalog).map_err(error)
}

#[cfg(test)]
#[path = "local_host_tests.rs"]
mod tests;
