//! The order/decree/sync-log part of the per-frame network seams of the tick
//! driver (aa-runtime `simulation::NetworkSeams`), with aa-orders types so this
//! crate does not depend on aa-runtime. A thin adapter maps the trait onto
//! [`NetworkCore`] (see research/phase8-orders/network-seams-adapter.rs.txt).
//!
//! Synchronizer routing of the driver's entries (checked in the original
//! code): every tick entry goes to A ([0xB3655C]) except the second
//! `g_gworld->GetGameTimeBeginTick()` (0x76F835), which goes to S
//! ([0xB36560]); floats use 0x77D930 (raw bits), bools 0x77D9D0.
use super::admin::{AdminNode, Peer};
use super::sync::{DEFAULT_MAX_ENTRIES, Synchronizer};
use crate::flow::{Log, OrderHost, SyncEntry, Validity};
use crate::payload::{Field, NameKind, Registry, Resolver, SchemaContext, ThingCheck};
use crate::{Gate, Result};
use serde::{Deserialize, Serialize};

/// KKC_SetSimSyncValueOrderRequest (host, synchronizer S, 0x74E670).
pub const SIM_SYNC_REQUEST: usize = 299;
/// TGC_SetSyncValueOrderRequest (client, synchronizer A, 0x74E8B0).
pub const SYNC_VALUE_REQUEST: usize = 302;

/// Entries the tick driver logs (aa-runtime `SyncLog`), values as raw bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TickLog {
    /// 0x75CBF2 / 0x75CC9C.
    NextTickEventAdminTime(u32),
    /// 0x76F816 (A) and 0x76F835 (S).
    GameTimeBeginTick { second_log: bool, value: u32 },
    /// 0x76B797.
    IsFrozen(bool),
    /// 0x76B7D9 / 0x76B7EF.
    AdminTickEndGameTime(u32),
    AdminTickEndAdminTime(u32),
    /// 0x76F467.
    GameTimePerTick(u32),
    /// 0x76DE57.
    Frozen(bool),
}

/// Game/lobby side of decree execution (the native process functions).
/// Reference resolution and decoding game data are passed separately as an
/// immutable [`GameData`].
pub trait GameEffects {
    fn has_ordering_client(&mut self) -> bool;
    fn version_acknowledged(&mut self) -> bool;
    fn film_session(&mut self) -> bool;
    fn valid(&mut self, validity: Validity, request: usize, fields: &[Field]) -> Result<bool>;
    fn process(&mut self, decree: usize, fields: &mut Vec<Field>) -> Result<()>;
    fn mirror_enabled(&mut self) -> bool;
}

/// Read-only game data for decoding and request->decree resolution.
pub trait GameData: Resolver + SchemaContext {}
impl<T: Resolver + SchemaContext> GameData for T {}

pub struct NetworkCore {
    pub node: AdminNode,
    /// Host only: admin+0x2AC peers in list order (this node first).
    pub peers: Vec<Peer>,
    pub sync_a: Synchronizer,
    pub sync_s: Synchronizer,
    /// Simulation-thread and application-flag gate of the loggers.
    pub sync_gate: bool,
    /// 0x74D710 abort pair (session+0x24, admin+0x2E5).
    pub gate: Gate,
    /// admin+0x2C0.
    pub active: bool,
    /// admin+8 local player id (default player of the emitters).
    pub local_player: u32,
}

/// Routes synchronizer entries into the core and everything else to the game.
struct Bridge<'a, G: GameEffects + ?Sized> {
    a: &'a mut Synchronizer,
    s: &'a mut Synchronizer,
    gate: bool,
    game: &'a mut G,
}
impl<G: GameEffects + ?Sized> OrderHost for Bridge<'_, G> {
    fn has_ordering_client(&mut self) -> bool {
        self.game.has_ordering_client()
    }
    fn version_acknowledged(&mut self) -> bool {
        self.game.version_acknowledged()
    }
    fn film_session(&mut self) -> bool {
        self.game.film_session()
    }
    fn valid(&mut self, v: Validity, r: usize, f: &[Field]) -> Result<bool> {
        self.game.valid(v, r, f)
    }
    fn sync(&mut self, entry: SyncEntry) -> Result<()> {
        let pick = |log: Log| -> bool { log == Log::A };
        match entry {
            SyncEntry::Int { log, label, value } => {
                let s = if pick(log) { &mut *self.a } else { &mut *self.s };
                s.log_int(self.gate, label, value)
            }
            SyncEntry::Text { log, label, value } => {
                let units: Vec<u16> = value.encode_utf16().collect();
                let s = if pick(log) { &mut *self.a } else { &mut *self.s };
                s.log_text(self.gate, label, &units)
            }
        }
        Ok(())
    }
    fn process(&mut self, d: usize, f: &mut Vec<Field>) -> Result<()> {
        self.game.process(d, f)
    }
    fn mirror_enabled(&mut self) -> bool {
        self.game.mirror_enabled()
    }
}
impl NetworkCore {
    pub fn new(first_index: i32, local_player: u32) -> Result<Self> {
        Ok(Self {
            node: AdminNode::new(first_index)?,
            peers: Vec::new(),
            sync_a: Synchronizer::new(DEFAULT_MAX_ENTRIES),
            sync_s: Synchronizer::new(DEFAULT_MAX_ENTRIES),
            sync_gate: true,
            gate: Gate::default(),
            active: true,
            local_player,
        })
    }

    /// `sync_log` seam.
    pub fn tick_log(&mut self, entry: TickLog) {
        let g = self.sync_gate;
        let a = &mut self.sync_a;
        match entry {
            TickLog::NextTickEventAdminTime(v) => a.log_float_bits(g, "GetNextTickEventAdminTime()", v),
            TickLog::GameTimeBeginTick { second_log: false, value } => {
                a.log_float_bits(g, "g_gworld->GetGameTimeBeginTick()", value)
            }
            TickLog::GameTimeBeginTick { second_log: true, value } => {
                self.sync_s.log_float_bits(g, "g_gworld->GetGameTimeBeginTick()", value)
            }
            TickLog::IsFrozen(b) => a.log_bool(g, "IsFrozen()", b),
            TickLog::AdminTickEndGameTime(v) => a.log_float_bits(g, "m_admin_tick_end_game_time", v),
            TickLog::AdminTickEndAdminTime(v) => a.log_float_bits(g, "m_admin_tick_end_admin_time", v),
            TickLog::GameTimePerTick(v) => a.log_float_bits(g, "m_game_time_per_tick", v),
            TickLog::Frozen(b) => a.log_bool(g, "_frozen", b),
        }
    }

    /// 0x74E8B0: TGC_SetSyncValueOrderRequest {A last index, A hash}.
    pub fn client_sync_flush(&mut self, context: &dyn SchemaContext) -> Result<bool> {
        let fields = [
            Field::Uint { width: 32, value: (self.sync_a.end() - 1) as u32 },
            Field::Uint { width: 32, value: self.sync_a.hash as u32 },
        ];
        self.node.emit_request(self.active, SYNC_VALUE_REQUEST, &fields, self.local_player, context)
    }
    /// 0x74E670: KKC_SetSimSyncValueOrderRequest {S last index, S hash}.
    pub fn host_sync_flush(&mut self, context: &dyn SchemaContext) -> Result<bool> {
        let fields = [
            Field::Uint { width: 32, value: (self.sync_s.end() - 1) as u32 },
            Field::Uint { width: 32, value: self.sync_s.hash as u32 },
        ];
        self.node.emit_request(self.active, SIM_SYNC_REQUEST, &fields, self.local_player, context)
    }
    /// 0x752DA0.
    pub fn admin_history(&mut self) -> Result<i32> {
        self.node.seal_requests()
    }
    /// 0x752E60.
    pub fn host_execute_requests<G: GameEffects + ?Sized, D: GameData + ?Sized>(
        &mut self,
        game: &mut G,
        data: &D,
    ) -> Result<()> {
        let mut peers = std::mem::take(&mut self.peers);
        let mut bridge = Bridge { a: &mut self.sync_a, s: &mut self.sync_s, gate: self.sync_gate, game };
        let result = self.node.host_execute(&mut peers, &mut bridge, &DataRef(data), &DataRef(data), &mut self.gate);
        self.peers = peers;
        result
    }
    /// 0x752EA0.
    pub fn host_record_history<G: GameEffects + ?Sized>(&mut self, game: &mut G) -> Result<i32> {
        let mut bridge = Bridge { a: &mut self.sync_a, s: &mut self.sync_s, gate: self.sync_gate, game };
        self.node.seal_decrees(&mut bridge)
    }
    /// 0x74FD10.
    pub fn client_replay_history<G: GameEffects + ?Sized, D: GameData + ?Sized>(
        &mut self,
        index: i32,
        game: &mut G,
        data: &D,
    ) -> Result<()> {
        let mut bridge = Bridge { a: &mut self.sync_a, s: &mut self.sync_s, gate: self.sync_gate, game };
        self.node.client_execute(index, &mut bridge, &DataRef(data), &DataRef(data), &mut self.gate)
    }
    /// 0x74D640 (film recorder attached).
    pub fn replay_record(&mut self) -> Result<Vec<u8>> {
        self.node.record_film()
    }
    /// 0x751A40 (film source attached): one record.
    pub fn replay_playback<G: GameEffects + ?Sized, D: GameData + ?Sized>(
        &mut self,
        record: &[u8],
        game: &mut G,
        data: &D,
    ) -> Result<()> {
        let mut bridge = Bridge { a: &mut self.sync_a, s: &mut self.sync_s, gate: self.sync_gate, game };
        self.node.play_film_record(record, &mut bridge, &DataRef(data), &DataRef(data), &mut self.gate)
    }
}

/// Sized view of an unsized [`GameData`] for the `dyn` interfaces.
struct DataRef<'a, D: GameData + ?Sized>(&'a D);
impl<D: GameData + ?Sized> Resolver for DataRef<'_, D> {
    fn thing(&self, c: ThingCheck, s: u16) -> Option<u16> {
        self.0.thing(c, s)
    }
    fn player(&self, id: u32) -> bool {
        self.0.player(id)
    }
    fn definition(&self, r: Registry, i: u32) -> bool {
        self.0.definition(r, i)
    }
    fn named(&self, k: NameKind, key: &[u16]) -> Option<Vec<u16>> {
        self.0.named(k, key)
    }
}
impl<D: GameData + ?Sized> SchemaContext for DataRef<'_, D> {
    fn registry_count(&self, r: Registry) -> Result<usize> {
        self.0.registry_count(r)
    }
    fn team_command_flags(&self, i: u32) -> Result<u32> {
        self.0.team_command_flags(i)
    }
    fn definition_kind(&self, r: Registry, i: u32) -> Result<u32> {
        self.0.definition_kind(r, i)
    }
    fn zero_width_list_budget(&self) -> usize {
        self.0.zero_width_list_budget()
    }
}
