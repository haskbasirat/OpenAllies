//! Owning local-startup prefix of the original Session/Node.
//!
//! This is deliberately not `Session::default()`: the wire projection's zeroes
//! are not native constructor values. Only reached, proved fields exist here.
//! In particular Session+E8 and Sim+10..1C are uninitialized by their native
//! constructors. Reset76EC70 supplies E8; no game-clock values are invented.
//! Native Host75AEE0 continues beyond this prefix into server/clock/settings
//! and launch services. This owner does not advertise a playable Host yet.
//!
//! Evidence: research/skirmish-20261016-local-bootstrap. Native allocation
//! failures and arbitrary callback reentry are outside the successful domain.
use crate::lobby::{GameOptions, LaunchState, NodeView, PeerLink, PropertyTable, SessionPlayer};
use crate::session::PlayerId;
use crate::{Error, Result};
use aa_orders::net::sync::{Synchronizer, DEFAULT_MAX_ENTRIES};
use aa_runtime::load_world_globals::SessionPhase;
use aa_runtime::localization::Localization;
use aa_simulation::NativeRng;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, Weak};

fn missing(message: impl Into<String>) -> Error { Error::Protocol(message.into()) }

/// The exact GUID member widths passed to the eleven `%X` conversions in
/// 76ED71..76EDBE. CoCreateGuid is an OS input, not a simulation RNG draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}
impl Guid {
    pub fn native_text(self) -> Vec<u16> {
        let mut s = format!("{:X}{:X}{:X}", self.data1, self.data2, self.data3);
        for b in self.data4 { use std::fmt::Write; write!(&mut s, "{b:X}").unwrap(); }
        s.encode_utf16().collect()
    }
}

/// Called at the native boundaries, rather than sampling both values before
/// EndSession. The platform adapter must call the real OS; fixtures explicitly
/// label their injected values. There is no entropy/timer fallback.
pub trait ResetOs {
    fn get_tick_count(&mut self) -> Result<u32>;
    fn co_create_guid(&mut self) -> Result<Guid>;
    /// Reached only if the actual synchronizer is enabled and unsuspended.
    /// This is the native simulation-thread/application-flag logger gate.
    fn sync_log_gate(&mut self) -> Result<bool>;
}

/// 40F7D0 -> 7841E0. The `Default` wire value has different +C/+10 words.
pub fn constructor_launch_state() -> LaunchState {
    LaunchState { kind: 0, flag_04: false, text: Vec::new(), field_0c: u32::MAX,
        field_10: 1, flag_14: false, flag_15: false, flag_16: false,
        field_18: 0, field_1c: 0, flag_20: false, field_24: 0 }
}

/// An actual queued 3C-byte UI notification value (7844F0/7519C0/784760).
/// EndSession overwrites the constructor's kind9 with kind2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Notification {
    pub kind: u32,
    pub text: Vec<u16>,
    pub state: u32,
    pub launch_state: LaunchState,
    pub flag_34: bool,
    pub word_38: u32,
}
impl Notification {
    fn end_session() -> Self {
        Self { kind: 2, text: Vec::new(), state: 0,
            launch_state: constructor_launch_state(), flag_34: false, word_38: 0 }
    }
}

struct QueueState { application_kind: u32, entries: VecDeque<Notification> }
/// 7850E0/784EF0 owns a critical section, signal and growable queue. Push7851C0
/// drops in application-kind3, otherwise locks, copies to the tail and signals.
/// The owner of B396C0 supplies and updates that application-kind input; it is
/// not Session+E4. No fixed native ring capacity is imposed.
pub struct Notifications { state: Mutex<QueueState>, changed: Condvar }
impl Notifications {
    pub fn new(application_kind: u32) -> Self {
        Self { state: Mutex::new(QueueState { application_kind, entries: VecDeque::new() }), changed: Condvar::new() }
    }
    pub fn set_application_kind(&self, value: u32) -> Result<()> {
        self.state.lock().map_err(|_| missing("notification critical section poisoned"))?.application_kind = value;
        Ok(())
    }
    pub fn push(&self, value: Notification) -> Result<()> {
        let mut state = self.state.lock().map_err(|_| missing("notification critical section poisoned"))?;
        if state.application_kind == 3 { return Ok(()); }
        state.entries.try_reserve(1).map_err(|_| missing("notification allocation failed"))?;
        state.entries.push_back(value);
        self.changed.notify_one();
        Ok(())
    }
    pub fn pop(&self) -> Result<Option<Notification>> {
        Ok(self.state.lock().map_err(|_| missing("notification critical section poisoned"))?.entries.pop_front())
    }
    pub fn len(&self) -> Result<usize> {
        Ok(self.state.lock().map_err(|_| missing("notification critical section poisoned"))?.entries.len())
    }
    pub fn is_empty(&self) -> Result<bool> { Ok(self.len()? == 0) }
}

/// Native 41AD7E..41AD9A creates the source B36568 table with79C6E0.
/// This owns the real initial source table, rather than pretending the source
/// pointer is null. Internet rated-map population is a separate required port.
pub struct InitialProperties { table: PropertyTable }
impl InitialProperties {
    pub fn from_native_constructor() -> Self {
        Self { table: PropertyTable { flag: false, count: 0, entries: Vec::new() } }
    }
}

/// Player+50 owns a VWorldState even when its Kingdom pointer is null. +C is
/// not initialized by4ED6D0, so there is no raw zero/default for that word.
/// The pointer/list fields shown here are genuinely initialized by the ctor.
#[derive(Debug)]
struct VWorldState {
    kingdom: Option<u32>,
    word_0c: Option<u32>,
    wildcard: StaticGroup,
}
#[derive(Debug)]
struct StaticGroup {
    key: Vec<u16>, title: Vec<u16>, text: Vec<u16>,
    members: Vec<u32>, flag_18: bool, word_1c: u32, flag_20: bool,
}
impl VWorldState {
    fn new_null(localization: &Localization) -> Result<Self> {
        // 4ED6D0 ->4B9F40, default/diagnostic resolution is not fabricated.
        let key: Vec<u16> = "static_group_wildcard_name".encode_utf16().collect();
        let title = localization.display(&key).map_err(|e| missing(e.to_string()))?;
        Ok(Self { kingdom: None, word_0c: None,
            wildcard: StaticGroup { key: Vec::new(), title, text: Vec::new(),
                members: Vec::new(), flag_18: false, word_1c: 0, flag_20: false } })
    }
}

struct OwnedPlayer { core: SessionPlayer, vector_44: Vec<u32>, view: VWorldState }
/// Player identity is native ++Session+E8, scoped to its actual owner. It is
/// never an ActorId/GTID and cannot be transferred to another bootstrap owner.
#[derive(Clone, Debug)]
pub struct PlayerHandle { owner: Weak<()>, id: u32 }
impl PlayerHandle { pub fn id(&self) -> u32 { self.id } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Progress { Constructed, Resetting, Reset, CreatingPlayer, LocalPlayerCreated }

/// Only fields written by76F890 or the actual Reset prefix are represented.
/// There is no complete LaunchSettings/Session conversion that fills omitted
/// fields with Default. Subsequent services must bind their real producers.
struct SessionFields {
    launch_mode: u32, launch_game_type: u32, launch_state: LaunchState,
    options: GameOptions, properties: PropertyTable,
    players: Vec<OwnedPlayer>, created: Option<u32>,
    changed: [bool; 3], counter_e0: u32, state: u32, phase: u32,
    last_player_id: Option<u32>, rng: NativeRng, multiplayer: bool,
    launch_countdown: i32, host_countdown: i32,
    field_08: u32, field_18: u32, field_154: Option<u32>,
    field_158: u32, field_15c: u32, text_160: Vec<u16>, field_164: u32,
    flags_168_16e: [bool; 7],
}

/// The real local Node/Session prefix owner. Host remains null while the first
/// player is constructed; the resulting human status is0, not2.
pub struct LocalBootstrap {
    identity: Arc<()>, session: SessionFields,
    /// Actual Node+8/+2C, not a detached SessionFacts snapshot.
    local_player: Option<u32>, ordering_player: Option<u32>,
    node_host: PeerLink, node_active: bool, peer_list: Vec<PeerLink>, peers_changed: bool,
    local_identity: PlayerId,
    /// Actual Sim+29 and world+3C projection. Uninitialized timeline words
    /// deliberately have no storage until their actual producer is bound.
    sim_frozen: bool, sim_world: Option<u32>, world_lock: Mutex<()>,
    sync_a: Synchronizer, sync_s: Synchronizer,
    progress: Progress,
}

impl LocalBootstrap {
    /// Successful-allocation constructor field projection76F890/76F240,
    /// options77B7B0 and properties79C6E0. Not a full unported Node ctor.
    pub fn from_native_constructors() -> Self {
        Self { identity: Arc::new(()), session: SessionFields {
            launch_mode: 3, launch_game_type: 0, launch_state: constructor_launch_state(),
            options: GameOptions { name: Vec::new(), password: Vec::new(),
                max_players: -1, field_d8: u32::MAX, allow_observers: true },
            properties: PropertyTable { flag: false, count: 0, entries: Vec::new() },
            players: Vec::new(), created: None, changed: [false; 3], counter_e0: 0,
            state: 0, phase: 0, last_player_id: None, rng: NativeRng::default(), multiplayer: false,
            launch_countdown: 0, host_countdown: 0, field_08: 0, field_18: 0, field_154: None,
            field_158: 0, field_15c: 0, text_160: Vec::new(), field_164: 0,
            flags_168_16e: [false; 7],
        }, local_player: None, ordering_player: None, node_host: PeerLink::Null,
            node_active: false, peer_list: Vec::new(), peers_changed: true,
            local_identity: PlayerId { name: Vec::new(), serial: 0, a: 0, b: 0, c: 0 },
            sim_frozen: true, sim_world: None, world_lock: Mutex::new(()),
            sync_a: Synchronizer::new(DEFAULT_MAX_ENTRIES), sync_s: Synchronizer::new(DEFAULT_MAX_ENTRIES),
            progress: Progress::Constructed }
    }

    /// Real fresh-owner76EC70 Reset, including76E0F0 EndSession and the
    /// no-world branch of76AB80. Existing live worlds/replays/players are not
    /// silently discarded: those constructors/destructors require further ports.
    /// A failed OS/queue service retains all effects already committed.
    pub fn reset_fresh(&mut self, os: &mut dyn ResetOs,
        source: &InitialProperties, notifications: &Notifications) -> Result<()> {
        if self.progress != Progress::Constructed {
            return Err(missing("fresh Reset cannot destroy a started/incomplete native owner"));
        }
        self.progress = Progress::Resetting;
        // Session+1C is null by ctor76FA?;76E0F0 still locks the real timeline
        // World slot through76AB80, then posts the native kind2 value.
        {
            let _guard = self.world_lock.lock().map_err(|_| missing("world critical section poisoned"))?;
            if self.sim_world.is_some() { return Err(Error::Unsupported("76AB80 live GWorld destruction")); }
        }
        notifications.push(Notification::end_session())?;
        let tick = os.get_tick_count()?;
        self.session.rng.reseed(tick); // actual85D880, not a new seed algorithm
        self.session.last_player_id = Some(0);
        // 74D460 actual peer list is empty in this fresh owner.
        if !self.peer_list.is_empty() { return Err(missing("fresh Reset has nonempty Node peer owner")); }
        if !self.session.players.is_empty() { return Err(Error::Unsupported("76EC70 SessionPlayer destruction")); }
        self.session.changed = [true; 3];
        self.session.flags_168_16e[2] = false;
        self.session.state = 0; self.session.phase = 0;
        self.session.launch_countdown = 0; self.session.host_countdown = 0;
        self.session.field_18 = 0; self.session.field_158 = 0; self.session.field_15c = 0;
        // 76DD90(true) never reads undefined time words or ResetAdminTime.
        // Honor lazy logger gating, then reuse the actual synchronizer kernel.
        let gate = if self.sync_a.enabled && self.sync_a.suspend < 1 { os.sync_log_gate()? } else { false };
        self.sync_a.log_bool(gate, "_frozen", true);
        self.sim_frozen = true;
        self.session.flags_168_16e[4] = false;
        self.session.flags_168_16e[0] = false;
        let guid = os.co_create_guid()?;
        self.session.text_160 = guid.native_text();
        self.session.field_164 = 0; self.session.field_08 = 0;
        self.session.flags_168_16e[3] = false;
        self.session.flags_168_16e[5] = false;
        self.session.flags_168_16e[6] = false;
        self.session.multiplayer = false; self.session.field_154 = Some(0);
        // 79C910 preserves destination+10 flag; it copies count/entries only.
        self.session.properties.entries.clear();
        self.session.properties.count = source.table.count;
        self.session.properties.entries.clone_from(&source.table.entries);
        self.progress = Progress::Reset;
        Ok(())
    }

    /// 75B126..75B1B1 local Host prefix. `nickname` is UserData+2C; the local
    /// identity key is [B40C24]+1C copied by77B8F0. Neither is a Kingdom name.
    /// Host/server/clock setup comes later, in its native order.
    pub fn create_first_local_player(&mut self, nickname: &[u16],
        local_identity_key: &[u16], localization: &Localization) -> Result<PlayerHandle> {
        if self.progress != Progress::Reset || !self.session.players.is_empty() || self.node_host != PeerLink::Null {
            return Err(missing("first local player requires the actual fresh Reset owner and null Host"));
        }
        self.progress = Progress::CreatingPlayer;
        self.node_active = true;
        let id = self.session.last_player_id.ok_or_else(|| missing("Session+E8 has no Reset producer"))?.wrapping_add(1);
        self.session.last_player_id = Some(id);
        let core = SessionPlayer::new(PeerLink::Node, id, false, false, false,
            &NodeView { host: self.node_host });
        let view = VWorldState::new_null(localization)?;
        self.session.players.push(OwnedPlayer { core, vector_44: Vec::new(), view });
        self.session.created = Some(id); self.session.changed[0] = true;
        // 770280 reads the nonnull properties owner then mode3: no definition,
        // condition, CRC, rated-map, map-header or arbitrary default is read.
        if self.session.launch_mode != 3 { return Err(Error::Unsupported("first local player rated update outside ctor mode3")); }
        // 766560 on the sole player has no other duplicate, including the
        // empty-name case where76CE10 returns this player itself.
        let p = &mut self.session.players[0].core;
        p.name = nickname.to_vec(); p.flag_2c = true;
        self.local_player = Some(id); self.ordering_player = Some(id);
        self.local_identity = PlayerId { name: local_identity_key.to_vec(), serial: 0, a: 0, b: 0, c: 0 };
        self.progress = Progress::LocalPlayerCreated;
        Ok(PlayerHandle { owner: Arc::downgrade(&self.identity), id })
    }

    fn player(&self, handle: &PlayerHandle) -> Result<&OwnedPlayer> {
        if !Weak::ptr_eq(&handle.owner, &Arc::downgrade(&self.identity)) {
            return Err(missing("SessionPlayer handle belongs to another native owner"));
        }
        self.session.players.iter().find(|p| p.core.id == handle.id)
            .ok_or_else(|| missing("SessionPlayer is no longer owned by this Session"))
    }
    pub fn local_player(&self) -> Result<Option<PlayerHandle>> {
        self.local_player.map(|id| {
            let h = PlayerHandle { owner: Arc::downgrade(&self.identity), id };
            self.player(&h)?; Ok(h)
        }).transpose()
    }
    /// Native player+24. This reads the actual constructor/staging-owned
    /// CString; it never substitutes the profile nickname or a map label.
    pub fn kingdom_name(&self, player: &PlayerHandle) -> Result<&[u16]> { Ok(&self.player(player)?.core.kingdom) }
    pub fn player_name(&self, player: &PlayerHandle) -> Result<&[u16]> { Ok(&self.player(player)?.core.name) }
    pub fn player_status(&self, player: &PlayerHandle) -> Result<u32> { Ok(self.player(player)?.core.status) }
    pub fn progress(&self) -> Progress { self.progress }
    pub fn launch_mode(&self) -> u32 { self.session.launch_mode }
    pub fn launch_game_type(&self) -> u32 { self.session.launch_game_type }
    pub fn options(&self) -> &GameOptions { &self.session.options }
    pub fn last_player_id(&self) -> Result<u32> { self.session.last_player_id.ok_or_else(|| missing("native ctor leaves Session+E8 uninitialized")) }
    pub fn rng_state(&self) -> u32 { self.session.rng.state }
    pub fn reset_guid_text(&self) -> &[u16] { &self.session.text_160 }
    pub fn host(&self) -> PeerLink { self.node_host }
    /// No native SetHost has run and no ready state is advertised by this prefix.
    pub fn require_host_launch(&self) -> Result<()> {
        Err(Error::Unsupported("75B1B1 onwards: SetServer/SetHost/selected settings/kingdom assignment/77A950 launch"))
    }
}

impl SessionPhase for LocalBootstrap {
    fn current_phase(&self) -> std::result::Result<u32, String> { Ok(self.session.phase) }
    fn application_state(&self) -> std::result::Result<u32, String> { Ok(self.session.state) }
}

/// A read-only diagnostic snapshot, never used to update the retained owner.
#[derive(Debug, Serialize)]
pub struct Snapshot<'a> {
    pub progress: Progress, pub launch_mode: u32, pub launch_game_type: u32,
    pub launch_state: &'a LaunchState, pub options: &'a GameOptions,
    pub changed: [bool; 3], pub counter_e0: u32, pub state: u32, pub phase: u32,
    pub last_player_id: Option<u32>, pub rng_state: u32, pub multiplayer: bool,
    pub field_154: Option<u32>, pub text_160: &'a [u16], pub sim_frozen: bool,
    pub players: Vec<&'a SessionPlayer>, pub local_player: Option<u32>, pub ordering_player: Option<u32>,
    pub host: PeerLink, pub node_active: bool, pub peer_list: &'a [PeerLink], pub peers_changed: bool,
    pub local_identity: &'a PlayerId, pub properties: &'a PropertyTable,
    pub sync_a_enabled: bool, pub sync_a_hash: i32, pub sync_a_end: i32,
    pub sync_s_enabled: bool,
}
impl LocalBootstrap {
    pub fn snapshot(&self) -> Snapshot<'_> {
        Snapshot { progress: self.progress, launch_mode: self.session.launch_mode,
            launch_game_type: self.session.launch_game_type, launch_state: &self.session.launch_state,
            options: &self.session.options, changed: self.session.changed, counter_e0: self.session.counter_e0,
            state: self.session.state, phase: self.session.phase, last_player_id: self.session.last_player_id,
            rng_state: self.session.rng.state, multiplayer: self.session.multiplayer,
            field_154: self.session.field_154, text_160: &self.session.text_160, sim_frozen: self.sim_frozen,
            players: self.session.players.iter().map(|p| &p.core).collect(),
            local_player: self.local_player, ordering_player: self.ordering_player,
            host: self.node_host, node_active: self.node_active, peer_list: &self.peer_list,
            peers_changed: self.peers_changed, local_identity: &self.local_identity,
            properties: &self.session.properties, sync_a_enabled: self.sync_a.enabled,
            sync_a_hash: self.sync_a.hash, sync_a_end: self.sync_a.end(), sync_s_enabled: self.sync_s.enabled }
    }
}
