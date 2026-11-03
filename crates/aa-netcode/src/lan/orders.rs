//! The order processes of a LAN session (the `shape.process` functions of
//! aa-orders' flow table) on the session model of `crate::lobby`, and the
//! host/client execution steps of the admin tick.
//!
//! Ported process functions (native order kept; addresses per process):
//! * 0x75D710 TGC_SetPlayerOrder (aa_orders `try_process_set_player`);
//! * 0x75DA80 -> 0x758FD0 PlayerJoin (`lobby::process_player_join`) with the
//!   host join completion 0x753580 (`lobby::complete_host_join`);
//! * 0x75D980 -> 0x751D00 SetSyncValue (host validation 0x77D050) and
//!   0x750BA0 / 0x75D9A0 -> 0x75BBC0 SyncFailure; 0x75D7F0 -> 0x752F60
//!   SetSimSyncValue (film players only: refused);
//! * 0x75DB90 / 0x75DBA0 / 0x75DBB0 / 0x75D8D0 countdown and launch
//!   (`crate::staging`), the seed of FinalizeLaunchSettings 0x776B20, the
//!   not-ready report 0x7725A0, 0x75D8A0 -> 0x7745C0 SetNewState, 0x75DB40 ->
//!   0x7712E0 / 0x765EA0 SetPlayerReadyStatus, 0x75DA40 -> 0x7710E0 SetMapSeed;
//! * 0x75E140 -> 0x778170 SetPlayerKingdom (0x777F00, 0x777CD0, 0x777610,
//!   0x7744C0), 0x75E030 -> 0x754F50 CreateExplicitAI, 0x75E1C0 / 0x75E1F0 /
//!   0x75E340 kingdom nation / faction / AI handicap (0x777D90 / 0x777700 /
//!   0x771880: `local_bootstrap::staging_requests`, proven by its oracle,
//!   over [`SessionHost`]), 0x75E220 -> 0x771C70 kingdom color;
//! * 0x75DFF0 -> 0x7554B0 SetMapTransferStatus (host; transfer refused) and
//!   0x75DA60 -> 0x74BBB0 UpdateMapStatus;
//! * 0x75DA50 -> 0x752FC0 UpdateTaskStatus with 0x74DA50 / 0x776560 ->
//!   0x775BF0 / 0x773AE0 and ClearAllTasks 0x74DAA0;
//! * 0x75D810 SetGameSpeed (`crate::speed` gate);
//! * 0x75DAA0 -> 0x755D70 PlayerLeave -> ClientLeave 0x755840 (reduced: see
//!   [`Orders::player_leave`]);
//! * 0x75E070 -> 0x755E00 public chat (the event; its display is the UI's);
//! * gameplay decrees (selection, generic commands, actor orders, faction
//!   abilities) go to [`super::LanGame::decree`].
//!
//! Every other decree is refused ([`Error::Unsupported`] with its name): the
//! pause vote 0x754880 (75, 76), private and team chat (78, 79), kicks, swaps,
//! teams and the rest of the 358 descriptors have no process here.
//!
//! Nothing here is proven against the original unless the module named says
//! so; the processes written in this file have no oracle of their own, only
//! the two-node tests tests/lan_session.rs and tests/lan_battle.rs.
use super::{LanData, LanEvent, LanGame, PendingJoin, Role, Seams, State};
use crate::local_bootstrap::staging_requests::{self as requests, Effects as RequestEffects, Host as RequestHost, Row};
use crate::lobby::{self, LaunchSettings, PeerDirectory, PeerLink, PeerView, Session};
use crate::session::PeerAddress;
use crate::staging::{self, LaunchStep, SettingsSlot, StagingHost, StagingNode, StateEvent};
use crate::{Error, Result};
use aa_orders::commands::{self as cmds, TryPlayerContext};
use aa_orders::flow::{Log, SyncEntry, Validity};
use aa_orders::net::admin::{ClientIdentity, CurrentClient, IdentifiedPeer, OrderingHost, Peer, RequestEmission};
use aa_orders::payload::{self, Field, NameKind, Registry, Resolver, SchemaContext, ThingCheck};
use aa_orders::{History, Position};
use std::cell::RefCell;

/// [0xA8C088]: seconds between a client's pauses (0x74B080, 0x754880).
const PAUSE_DELAY: f32 = 300.0;

/// Native descriptor ordinals (crates/aa-orders/src/native_registry.rs).
pub mod descriptor {
    pub const LAUNCH_REQUEST: usize = 0;
    pub const SET_MAP_TRANSFER_STATUS_REQUEST: usize = 3;
    pub const LAUNCH_DECREE: usize = 5;
    pub const SET_MAP_TRANSFER_STATUS_DECREE: usize = 8;
    pub const SET_NEW_STATE_REQUEST: usize = 4;
    pub const SET_NEW_STATE_DECREE: usize = 9;
    pub const SELECTION_ADD_REQUEST: usize = 12;
    pub const SELECTION_REMOVE_REQUEST: usize = 13;
    pub const SELECTION_CLEAR_REQUEST: usize = 14;
    pub const SELECTION_ADD_DECREE: usize = 16;
    pub const SELECTION_REMOVE_DECREE: usize = 17;
    pub const SELECTION_CLEAR_DECREE: usize = 18;
    pub const SELECTION_CLEAR_NAMED_DECREE: usize = 19;
    pub const TEAM_COMMAND_DECREE: usize = 23;
    pub const TELL_ACTOR_ATTACK_DECREE: usize = 65;
    pub const TELL_ACTOR_MOVE_DECREE: usize = 66;
    pub const TELL_ACTOR_ABILITY_DECREE: usize = 67;
    pub const GAME_PLAYER_KINGDOM_DECREE: usize = 68;
    pub const FACTION_USE_ABILITY_REQUEST: usize = 51;
    pub const FACTION_USE_ABILITY_DECREE: usize = 70;
    pub const SET_GAME_SPEED_REQUEST: usize = 54;
    pub const SET_GAME_SPEED_DECREE: usize = 73;
    pub const TOGGLE_PAUSED_REQUEST: usize = 56;
    pub const SET_PAUSED_REQUEST: usize = 57;
    pub const PUBLIC_CHAT_REQUEST: usize = 58;
    pub const PUBLIC_CHAT_DECREE: usize = 77;
    pub const TELL_SELECTED_REQUEST: usize = 84;
    pub const TELL_ACTOR_REQUEST: usize = 85;
    pub const TELL_SELECTED_DECREE: usize = 86;
    pub const TELL_ACTOR_DECREE: usize = 87;
    pub const SET_PLAYER_NAME_REQUEST: usize = 10;
    pub const SET_PLAYER_NAME_DECREE: usize = 11;
    pub const TOGGLE_PAUSED_DECREE: usize = 75;
    pub const SET_PAUSED_DECREE: usize = 76;
    pub const SPEED_UP_REQUEST: usize = 61;
    pub const SPEED_DOWN_REQUEST: usize = 62;
    pub const SPEED_UP_DECREE: usize = 80;
    pub const SPEED_DOWN_DECREE: usize = 81;
    pub const SET_SESSION_LOAD_MAP_REQUEST: usize = 91;
    pub const SET_SESSION_LOAD_MAP_DECREE: usize = 93;
    pub const SET_TEAMS_REQUEST: usize = 100;
    pub const SET_TEAMS_DECREE: usize = 104;
    pub const SET_MAX_PLAYERS_REQUEST: usize = 106;
    pub const SET_GAME_NAME_REQUEST: usize = 107;
    pub const SET_GAME_PASSWORD_REQUEST: usize = 108;
    pub const SET_ALLOW_OBSERVERS_REQUEST: usize = 109;
    pub const SET_MAX_PLAYERS_DECREE: usize = 110;
    pub const SET_GAME_NAME_DECREE: usize = 111;
    pub const SET_GAME_PASSWORD_DECREE: usize = 112;
    pub const SET_ALLOW_OBSERVERS_DECREE: usize = 113;
    pub const SET_MAP_SIZE_REQUEST: usize = 114;
    pub const SET_RMC_REQUEST: usize = 115;
    pub const SET_RMC_SETTINGS_REQUEST: usize = 118;
    pub const SET_MAP_SIZE_DECREE: usize = 119;
    pub const SET_RMC_DECREE: usize = 120;
    pub const SET_RMC_SETTINGS_DECREE: usize = 123;
    pub const KICK_PLAYER_REQUEST: usize = 133;
    pub const KICK_PLAYER_DECREE: usize = 144;
    pub const SET_MAP_SEED_REQUEST: usize = 90;
    pub const SET_MAP_SEED_DECREE: usize = 92;
    pub const START_COUNTDOWN_REQUEST: usize = 96;
    pub const START_COUNTDOWN_DECREE: usize = 97;
    pub const SET_READY_REQUEST: usize = 99;
    pub const STOP_COUNTDOWN_REQUEST: usize = 101;
    pub const SET_READY_DECREE: usize = 103;
    pub const STOP_COUNTDOWN_DECREE: usize = 105;
    pub const CREATE_EXPLICIT_AI_REQUEST: usize = 132;
    pub const SET_PLAYER_KINGDOM_REQUEST: usize = 135;
    pub const SET_KINGDOM_NATION_REQUEST: usize = 137;
    pub const SET_KINGDOM_FACTION_REQUEST: usize = 138;
    pub const SET_AI_HANDICAP_REQUEST: usize = 140;
    pub const CREATE_EXPLICIT_AI_DECREE: usize = 143;
    pub const SET_PLAYER_KINGDOM_DECREE: usize = 146;
    pub const SET_KINGDOM_NATION_DECREE: usize = 148;
    pub const SET_KINGDOM_FACTION_DECREE: usize = 149;
    pub const SET_AI_HANDICAP_DECREE: usize = 151;
    pub const SET_PLAYER_DECREE: usize = 298;
    pub const SIM_SYNC_REQUEST: usize = 299;
    pub const SYNC_VALUE_REQUEST: usize = 302;
    pub const SYNC_FAILURE_DECREE: usize = 303;
    pub const TASK_STATUS_REQUEST: usize = 304;
    pub const UPDATE_MAP_STATUS_REQUEST: usize = 305;
    pub const PLAYER_JOIN_REQUEST: usize = 306;
    pub const POST_PLAYER_JOIN_REQUEST: usize = 307;
    pub const PLAYER_LEAVE_REQUEST: usize = 308;
    pub const PLAYER_LEAVE_DECREE: usize = 309;
    pub const SIM_SYNC_DECREE: usize = 311;
    pub const SYNC_VALUE_DECREE: usize = 314;
    pub const TASK_STATUS_DECREE: usize = 315;
    pub const UPDATE_MAP_STATUS_DECREE: usize = 316;
    pub const PLAYER_JOIN_DECREE: usize = 317;
    pub const POST_PLAYER_JOIN_DECREE: usize = 318;
    pub const SET_PLAYER_REQUEST: usize = 320;
    pub const VERSION_ACK_REQUEST: usize = 323;
    pub const UPDATE_KINGDOM_NAME_DECREE: usize = 328;
    pub const COUNTDOWN_PROGRESS_DECREE: usize = 330;
    pub const SELECTION_SET_REQUEST: usize = 332;
    pub const SELECTION_SET_DECREE: usize = 333;
    pub const SET_KINGDOM_COLOR_REQUEST: usize = 342;
    pub const SET_KINGDOM_COLOR_DECREE: usize = 348;
}
use descriptor as d;

/// Decrees whose effect is the game's (the world and its players' views).
pub fn is_gameplay_decree(decree: usize) -> bool {
    matches!(
        decree,
        d::SELECTION_ADD_DECREE
            | d::SELECTION_REMOVE_DECREE
            | d::SELECTION_CLEAR_DECREE
            | d::SELECTION_CLEAR_NAMED_DECREE
            | d::TEAM_COMMAND_DECREE
            | d::TELL_ACTOR_ATTACK_DECREE
            | d::TELL_ACTOR_MOVE_DECREE
            | d::TELL_ACTOR_ABILITY_DECREE
            | d::GAME_PLAYER_KINGDOM_DECREE
            | d::FACTION_USE_ABILITY_DECREE
            | d::TELL_SELECTED_DECREE
            | d::TELL_ACTOR_DECREE
            | d::UPDATE_KINGDOM_NAME_DECREE
            | d::SELECTION_SET_DECREE
    )
}

/// The decrees this port has a process for ([`Orders::run`]).
fn has_process(decree: usize) -> bool {
    is_gameplay_decree(decree)
        || matches!(
            decree,
            d::SET_PLAYER_DECREE
                | d::PLAYER_JOIN_DECREE
                | d::POST_PLAYER_JOIN_DECREE
                | d::PLAYER_LEAVE_DECREE
                | d::SYNC_VALUE_DECREE
                | d::SYNC_FAILURE_DECREE
                | d::SIM_SYNC_DECREE
                | d::START_COUNTDOWN_DECREE
                | d::STOP_COUNTDOWN_DECREE
                | d::COUNTDOWN_PROGRESS_DECREE
                | d::LAUNCH_DECREE
                | d::SET_NEW_STATE_DECREE
                | d::SET_READY_DECREE
                | d::PUBLIC_CHAT_DECREE
                | d::SET_MAP_SEED_DECREE
                | d::SET_PLAYER_KINGDOM_DECREE
                | d::CREATE_EXPLICIT_AI_DECREE
                | d::SET_KINGDOM_NATION_DECREE
                | d::SET_KINGDOM_FACTION_DECREE
                | d::SET_AI_HANDICAP_DECREE
                | d::SET_KINGDOM_COLOR_DECREE
                | d::SET_MAP_TRANSFER_STATUS_DECREE
                | d::UPDATE_MAP_STATUS_DECREE
                | d::TASK_STATUS_DECREE
                | d::SET_GAME_SPEED_DECREE
                | d::SET_PLAYER_NAME_DECREE
                | d::TOGGLE_PAUSED_DECREE
                | d::SET_PAUSED_DECREE
                | d::SPEED_UP_DECREE
                | d::SPEED_DOWN_DECREE
                | d::SET_SESSION_LOAD_MAP_DECREE
                | d::SET_TEAMS_DECREE
                | d::SET_MAX_PLAYERS_DECREE
                | d::SET_GAME_NAME_DECREE
                | d::SET_GAME_PASSWORD_DECREE
                | d::SET_ALLOW_OBSERVERS_DECREE
                | d::SET_MAP_SIZE_DECREE
                | d::SET_RMC_DECREE
                | d::SET_RMC_SETTINGS_DECREE
                | d::KICK_PLAYER_DECREE
        )
}

/// A decree without a process here: the explicit refusal, with its name.
fn refused(decree: usize) -> Error {
    Error::Unsupported(aa_orders::native::DESCRIPTORS.get(decree).map_or("unknown order descriptor", |o| o.name))
}

/// Refuse a request whose decree has no process here before it enters the
/// stream (a request without a flow shape is the emitter's error).
pub(crate) fn check_request(descriptor: usize) -> Result<()> {
    match aa_orders::flow::shape(descriptor) {
        Ok(shape) if !has_process(usize::from(shape.decree)) => Err(refused(usize::from(shape.decree))),
        _ => Ok(()),
    }
}

fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
/// 0x506560 random-map settings as a process reads them: word, two lists of
/// (name, word), word.
fn rmc_settings_of(fields: &[Field]) -> Result<lobby::RmcSettings> {
    let mut i = 0;
    let word = |i: &mut usize| -> Result<u32> {
        let v = uint(fields, *i)?;
        *i += 1;
        Ok(v)
    };
    let mut out = lobby::RmcSettings { field_0: word(&mut i)?, ..Default::default() };
    for list in 0..2 {
        let n = word(&mut i)? as usize;
        let mut rows = Vec::with_capacity(n.min(fields.len()));
        for _ in 0..n {
            let name = text_field(fields, i)?;
            i += 1;
            let value = word(&mut i)?;
            rows.push(lobby::NamedWord { name, value });
        }
        if list == 0 {
            out.list_4 = rows;
        } else {
            out.list_10 = rows;
        }
    }
    out.field_1c = word(&mut i)?;
    Ok(out)
}
fn nonnull(raw: u32) -> Option<u32> {
    (raw != 0).then_some(raw)
}
fn orders_err(e: impl std::fmt::Display) -> aa_orders::Error {
    aa_orders::Error::Handler { index: 0, position: Position::PACKET_START, message: e.to_string() }
}
fn uint(fields: &[Field], i: usize) -> Result<u32> {
    match fields.get(i) {
        Some(Field::Uint { value, .. }) => Ok(*value),
        Some(Field::Bounded { value, .. }) => Ok(*value),
        Some(Field::Player(v)) => Ok(*v),
        Some(Field::Def { index, .. }) => Ok(*index),
        other => Err(Error::Protocol(format!("payload field {i}: {other:?}"))),
    }
}
fn text_field(fields: &[Field], i: usize) -> Result<Vec<u16>> {
    match fields.get(i) {
        Some(Field::Text(u)) | Some(Field::Named { units: u, .. }) => Ok(u.clone()),
        other => Err(Error::Protocol(format!("payload field {i}: {other:?}"))),
    }
}
fn def_field(fields: &[Field], i: usize, count: usize) -> Result<Option<u32>> {
    match fields.get(i) {
        Some(Field::Def { index, .. }) => Ok((*index as usize != count).then_some(*index)),
        other => Err(Error::Protocol(format!("payload field {i}: {other:?}"))),
    }
}

// ---------------------------------------------------------------------------
// Game data views
// ---------------------------------------------------------------------------

/// Registry counts and lookups of the session's game data.
pub(crate) struct DataView<'a> {
    pub data: &'a dyn LanData,
}
impl SchemaContext for DataView<'_> {
    fn registry_count(&self, registry: Registry) -> aa_orders::Result<usize> {
        self.data.registries().count(registry).ok_or_else(|| {
            aa_orders::Error::SchemaSource(format!("count of definition registry {:#x} not loaded", registry.0))
        })
    }
    fn team_command_flags(&self, _index: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext("TeamCommand definition flags are not loaded"))
    }
    fn definition_kind(&self, registry: Registry, index: u32) -> aa_orders::Result<u32> {
        if registry == super::registries::WORLD_RULES {
            return self
                .data
                .registries()
                .world_rule_kinds
                .get(index as usize)
                .copied()
                .ok_or(aa_orders::Error::SchemaContext("WorldRule kind out of range"));
        }
        Err(aa_orders::Error::SchemaContext("definition kinds of this registry are not loaded"))
    }
    /// The kingdom list of a header (0x5104A0) has no width of its own in the
    /// schema: more records than any map has is the explicit limit.
    fn zero_width_list_budget(&self) -> usize {
        1024
    }
}
impl lobby::LobbyData for DataView<'_> {
    fn registry_count(&self, registry: Registry) -> Result<usize> {
        self.data
            .registries()
            .count(registry)
            .ok_or_else(|| Error::Protocol(format!("count of definition registry {:#x} not loaded", registry.0)))
    }
    fn definition(&self, registry: Registry, index: u32) -> bool {
        self.data.registries().count(registry).is_some_and(|n| (index as usize) < n)
    }
    fn world_rule_kind(&self, index: u32) -> Result<u32> {
        self.data
            .registries()
            .world_rule_kinds
            .get(index as usize)
            .copied()
            .ok_or_else(|| Error::Protocol(format!("WorldRule {index} kind")))
    }
    fn world_rule_name(&self, index: u32) -> Result<Vec<u16>> {
        self.data
            .registries()
            .ids(super::registries::WORLD_RULES, index)
            .map(units)
            .ok_or_else(|| Error::Protocol(format!("WorldRule {index} name")))
    }
    fn scenario(&mut self, key: &[u16]) -> Option<Vec<u16>> {
        self.data.scenario(key)
    }
    fn random_map_config(&mut self, key: &[u16]) -> Option<Vec<u16>> {
        self.data.random_map_config(key)
    }
    fn zero_width_list_budget(&self) -> usize {
        0
    }
}

/// The duplicate-name format of 0x766560 (the rest of JoinHost is unused
/// by `lobby::set_player_name`).
pub(crate) struct NameFormat<'a> {
    pub data: &'a dyn LanData,
}
impl lobby::JoinHost for NameFormat<'_> {
    fn conn(&mut self, _link: &PeerLink) -> Result<&mut crate::packets::AdminConn> {
        Err(Error::Protocol("no connection in a name format".into()))
    }
    fn auto_assign_kingdom(&mut self, _session: &mut Session, _player: u32) -> Result<()> {
        Ok(())
    }
    fn duplicate_name(&mut self, name: &[u16], n: u32) -> Result<Vec<u16>> {
        Ok(self.data.format(lobby::DUPLICATE_NAME_KEY, &[name.to_vec(), units(&n.to_string())]))
    }
    fn kingdom_open(&mut self, _kingdom: &[u16]) -> Result<bool> {
        // 0x77E430 on the constructor-empty session info (`lobby_action`
        // refuses a non-empty one): null, so the record counts as open.
        Ok(true)
    }
}

/// Reference resolution of the host's request readers (0x76A4B0 family,
/// 0x76B8F0, the 0x76A880 count rule, 0x76A9C0).
pub(crate) struct Resolve<'a, 'b, 'g> {
    pub st: &'a RefCell<&'b mut State>,
    pub game: &'a RefCell<&'g mut dyn LanGame>,
    pub data: &'a dyn LanData,
}
impl Resolver for Resolve<'_, '_, '_> {
    fn thing(&self, check: ThingCheck, slot: u16) -> Option<u16> {
        self.game.try_borrow().ok().and_then(|g| g.thing(check, slot))
    }
    fn player(&self, id: u32) -> bool {
        self.st.try_borrow().ok().is_some_and(|s| s.session.live_player(id, false).is_some())
    }
    fn definition(&self, registry: Registry, index: u32) -> bool {
        self.data.registries().count(registry).is_some_and(|n| (index as usize) < n)
    }
    fn named(&self, kind: NameKind, key: &[u16]) -> Option<Vec<u16>> {
        match kind {
            NameKind::RandomMapConfig if key.is_empty() => None,
            NameKind::RandomMapConfig => self.data.random_map_config(key),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Staging effects
// ---------------------------------------------------------------------------

/// The node facts the staging processes read.
pub(crate) fn staging_node(st: &State, view: &lobby::NodeView, ordering: Option<u32>) -> StagingNode {
    StagingNode { view: *view, took_over: 0, ordering_player: ordering, local_player: st.local_player, internet: false }
}

/// [`StagingHost`] collecting the effects in native order.
pub(crate) struct StagingEffects {
    pub messages: Vec<LanEvent>,
    pub requests: Vec<(usize, Vec<Field>)>,
    pub freeze: Vec<bool>,
    pub states: Vec<u32>,
    pub launch_task: bool,
    pub reset_order_version: bool,
    pub stored_single: Option<LaunchSettings>,
    pub stored_multi: Option<LaunchSettings>,
    pub countdown_time: i32,
    pub timer: Option<u32>,
    pub ms: u32,
    /// What the not-ready report 0x7725A0 reads.
    node: StagingNode,
    peers: SnapshotDirectory,
}
impl StagingEffects {
    pub fn new(st: &State, data: &dyn LanData) -> Self {
        Self {
            messages: Vec::new(),
            requests: Vec::new(),
            freeze: Vec::new(),
            states: Vec::new(),
            launch_task: false,
            reset_order_version: false,
            stored_single: st.stored_single.clone(),
            stored_multi: st.stored_multi.clone(),
            countdown_time: data.countdown_time(),
            timer: None,
            ms: st.ms,
            node: staging_node(st, &st.view(), st.local_player),
            peers: SnapshotDirectory::of(st),
        }
    }
    fn message(&mut self, kind: u32, key: &str, args: Vec<Vec<u16>>) {
        self.messages.push(LanEvent::Message { kind, key: key.into(), args, number: None });
    }
    pub fn take_requests(&mut self) -> Vec<(usize, Vec<Field>)> {
        std::mem::take(&mut self.requests)
    }
    /// Apply what the processes left for the node (outside an emission loan
    /// the requests go to the deferred list).
    pub fn apply(self, st: &mut State, emission: Option<&mut RequestEmission<'_>>, data: &dyn LanData) -> Result<()> {
        st.events.extend(self.messages);
        if self.reset_order_version {
            // 0x74DBC0(0): every peer of +0x2AC.
            for link in st.peers.clone() {
                st.set_link_6c(&link, 0);
            }
        }
        for state in self.states {
            st.events.push(LanEvent::StateChanged { state });
            // 0x766D40 after the application's state callback.
            st.deferred.version_ack = true;
        }
        if self.launch_task {
            st.events.push(LanEvent::LoadWorld);
        }
        st.stored_single = self.stored_single;
        st.stored_multi = self.stored_multi;
        if let Some(t) = self.timer {
            st.countdown_started = Some(t);
        }
        match emission {
            Some(e) => {
                let player = st.local_player.unwrap_or(0);
                let ctx = DataView { data };
                for (descriptor, fields) in self.requests {
                    e.emit_request(true, descriptor, &fields, player, &ctx)?;
                }
            }
            None => st.deferred.requests.extend(self.requests),
        }
        Ok(())
    }
}
impl StagingHost for StagingEffects {
    fn ui_message(&mut self, kind: u32, key: &'static str, number: Option<i32>) -> Result<()> {
        self.messages.push(LanEvent::Message { kind, key: key.into(), args: Vec::new(), number });
        Ok(())
    }
    fn request(&mut self, descriptor: usize, fields: Vec<Field>) -> Result<()> {
        self.requests.push((descriptor, fields));
        Ok(())
    }
    fn countdown_time(&mut self) -> Result<i32> {
        Ok(self.countdown_time)
    }
    fn start_timer(&mut self, _session: &mut Session) -> Result<()> {
        self.timer = Some(self.ms);
        Ok(())
    }
    /// 0x7725A0: the host asks every player to become ready and names who
    /// lacks the file; a client is told what the request means for it.
    fn not_ready(&mut self, session: &mut Session) -> Result<()> {
        let node = self.node;
        let file_mode = matches!(session.launch.mode, 0 | 2);
        if node.view.we_are_host() {
            self.message(0xF, "host_requested_players_ready", Vec::new());
            for index in 0..session.players.len() {
                let p = &session.players[index];
                if node.view.is_host(&p.link) || staging::player_ready(session, index, &node, &self.peers)? {
                    continue;
                }
                if self.peers.peer(&p.link)?.identity.field_3c != 1 && file_mode {
                    let key = if session.launch.field_0c == 5 { "map_will_not_transfer_host_film|ss" } else { "map_will_not_transfer_host|ss" };
                    let args = vec![p.name.clone(), session.launch.game.pair_20.second.clone()];
                    self.message(0x10, key, args);
                }
            }
        } else if let Some(index) = node.local_player.and_then(|id| session.players.iter().position(|p| p.id == id)) {
            let link = session.players[index].link;
            if link != PeerLink::Null {
                let status = self.peers.peer(&link)?.identity.field_3c;
                if staging::player_ready(session, index, &node, &self.peers)? || status == 1 || !file_mode {
                    self.message(0xF, "host_requested_players_ready_client_is_ready", Vec::new());
                } else if status == 3 {
                    self.message(0x10, "host_requested_players_ready_client_cannot_download_map", Vec::new());
                } else {
                    self.message(0x10, "host_requested_players_ready_client_does_not_have_map", Vec::new());
                }
            }
        }
        Ok(())
    }
    fn launch_step(&mut self, session: &mut Session, step: LaunchStep) -> Result<()> {
        // 0x775DB0(1) (cancel the old task) and 0x770500: the old world and
        // view teardown of the application. Of FinalizeLaunchSettings 0x776B20
        // only the seed is ported; its implicit AIs 0x776580, the record bytes
        // +0x31 / +0x32 and the "MULTIPLAYER" rule are not.
        if step == LaunchStep::Step776B20 && matches!(session.launch.mode, 0 | 1 | 2 | 5) {
            // +0x100, else one draw of the session stream +0x128 % 0xFFFFFFFF,
            // into the creator's +0x4C. +0x128 travels in the join snapshot,
            // so every node draws the same value.
            let seed = if session.field_100 != 0 {
                session.field_100
            } else {
                let mut rng = aa_simulation::NativeRng { state: session.field_128 };
                let v = rng.next_u32() % u32::MAX;
                session.field_128 = rng.state;
                v
            };
            session.launch.active_map_mut().field_4c = seed;
        }
        Ok(())
    }
    fn load(&mut self, session: &mut Session) -> Result<bool> {
        // 0x775EE0 KKC_Session::CreateLaunchTask: a task for map types 0
        // (random), 1 (default creator), 5 (metagame) and 2 (scenario, when
        // +0x50 holds it or the state is 4); none for 3 (none), 4 (use the
        // existing world) or an unknown type.
        let task = match session.launch.mode {
            0 | 1 | 5 => true,
            2 => session.launch.scenario.is_some() || session.launch.game_type == 4,
            _ => false,
        };
        // Only a launch into a game builds a world here (the staging state
        // has no world of its own in this port).
        if task && matches!(session.launch.game_type, 2 | 4 | 5) {
            self.launch_task = true;
        }
        Ok(task)
    }
    fn dump_launch_settings(&mut self, _session: &mut Session, _label: &'static str) -> Result<()> {
        // 0x76D6C0 launch-settings sync dump: not ported (same on every
        // node of this port; an original peer logs it to A).
        Ok(())
    }
    fn timeline_freeze(&mut self, flag: bool) -> Result<()> {
        self.freeze.push(flag);
        Ok(())
    }
    fn reset_order_version(&mut self) -> Result<()> {
        self.reset_order_version = true;
        Ok(())
    }
    fn state_changed(&mut self, _session: &mut Session, event: StateEvent) -> Result<()> {
        self.states.push(event.state);
        Ok(())
    }
    fn copy_launch_settings(&mut self, session: &mut Session, dst: SettingsSlot, src: SettingsSlot) -> Result<()> {
        // KKC_LaunchSettings::operator= 0x779F00 on the wire projection.
        let value = match src {
            SettingsSlot::Launch => Some(session.launch.clone()),
            SettingsSlot::Launch88 => Some(session.launch_88.clone()),
            SettingsSlot::StoreSingle => self.stored_single.clone(),
            SettingsSlot::StoreMulti => self.stored_multi.clone(),
        };
        let Some(value) = value else { return Ok(()) };
        match dst {
            SettingsSlot::Launch => session.launch = value,
            SettingsSlot::Launch88 => session.launch_88 = value,
            SettingsSlot::StoreSingle => self.stored_single = Some(value),
            SettingsSlot::StoreMulti => self.stored_multi = Some(value),
        }
        Ok(())
    }
}

/// What the snapshot reader left for the node.
pub(crate) enum SnapshotEffect {
    State(u32),
    MapStatus { file: Vec<u16> },
}
pub(crate) struct SnapshotEffects {
    pub deferred: Vec<SnapshotEffect>,
    pub view: lobby::NodeView,
}
impl lobby::SnapshotHost for SnapshotEffects {
    fn change_to_state(&mut self, session: &mut Session, state: u32, arg: u32) -> Result<()> {
        let node = StagingNode { view: self.view, took_over: 0, ordering_player: None, local_player: None, internet: false };
        let mut fx = StagingEffectsLite::default();
        staging::change_to_state(session, state, arg != 0, &node, &mut fx)?;
        for s in fx.states {
            self.deferred.push(SnapshotEffect::State(s));
        }
        Ok(())
    }
    fn update_map_status(&mut self, _session: &mut Session, _rmc: bool, file: &[u16], _crc: u32, source_check: bool) -> Result<bool> {
        // 0x7770B0 / 0x770B70: without a source check the client reports
        // SetMapTransferStatus(1). The RMC data checksum 0x59E240 and the
        // map file CRC are not computed by this port.
        if !source_check {
            self.deferred.push(SnapshotEffect::MapStatus { file: file.to_vec() });
        }
        Ok(true)
    }
}
/// The staging host of the snapshot's ChangeToState (no settings slots on a
/// joining client).
#[derive(Default)]
struct StagingEffectsLite {
    states: Vec<u32>,
}
impl StagingHost for StagingEffectsLite {
    fn ui_message(&mut self, _kind: u32, _key: &'static str, _number: Option<i32>) -> Result<()> {
        Ok(())
    }
    fn request(&mut self, _descriptor: usize, _fields: Vec<Field>) -> Result<()> {
        Ok(())
    }
    fn countdown_time(&mut self) -> Result<i32> {
        Err(Error::Protocol("ChangeToState 0x77A750 reads no countdown".into()))
    }
    fn start_timer(&mut self, _session: &mut Session) -> Result<()> {
        Err(Error::Protocol("ChangeToState 0x77A750 starts no timer".into()))
    }
    fn not_ready(&mut self, _session: &mut Session) -> Result<()> {
        Err(Error::Protocol("ChangeToState 0x77A750 reports no readiness".into()))
    }
    fn launch_step(&mut self, _session: &mut Session, _step: LaunchStep) -> Result<()> {
        Err(Error::Protocol("ChangeToState 0x77A750 runs no launch step".into()))
    }
    fn load(&mut self, _session: &mut Session) -> Result<bool> {
        Err(Error::Protocol("ChangeToState 0x77A750 creates no task".into()))
    }
    fn dump_launch_settings(&mut self, _session: &mut Session, _label: &'static str) -> Result<()> {
        Ok(())
    }
    fn timeline_freeze(&mut self, _flag: bool) -> Result<()> {
        Ok(())
    }
    fn reset_order_version(&mut self) -> Result<()> {
        Ok(())
    }
    fn state_changed(&mut self, _session: &mut Session, event: StateEvent) -> Result<()> {
        self.states.push(event.state);
        Ok(())
    }
    fn copy_launch_settings(&mut self, session: &mut Session, dst: SettingsSlot, src: SettingsSlot) -> Result<()> {
        if let (SettingsSlot::Launch88, SettingsSlot::Launch) = (dst, src) {
            session.launch_88 = session.launch.clone();
        } else if let (SettingsSlot::Launch, SettingsSlot::Launch88) = (dst, src) {
            session.launch = session.launch_88.clone();
        }
        Ok(())
    }
}

/// The kingdom setters of AutoAssignKingdomToPlayer 0x778D10.
pub(crate) struct KingdomAssign<'a> {
    pub data: &'a dyn LanData,
    pub view: lobby::NodeView,
    pub local: Option<u32>,
    pub now: u32,
}
impl staging::KingdomHost for KingdomAssign<'_> {
    fn kingdom_record_info(&mut self, session: &Session, record: usize) -> Result<Option<bool>> {
        // 0x77E430 walks the session info's lists: the team format
        // ([session+0x14]+0x28) seats the records (0x781250), which only the
        // game data can say; without a format the lists are empty: null.
        Ok(self.data.kingdom_open(session, record))
    }
    fn assign_kingdom(&mut self, session: &mut Session, player: u32, record: usize) -> Result<()> {
        let name = session.launch.active_map().header.kingdoms[record].name.clone();
        let mut host = SessionHost { session, data: self.data, ordering: None, local: self.local, view: self.view, now: self.now };
        let slot = host.player_slot(player).ok_or_else(|| Error::Protocol(format!("no player {player}")))?;
        let mut fx = RequestMessages::default();
        // 0x778EB2: notify 0.
        requests::set_player_kingdom_777f00(&mut host, slot, &name, false, &mut fx)?;
        let (nation, faction) = (host.session.players[slot].text_30.clone(), host.session.players[slot].text_34.clone());
        requests::check_nation_777cd0(&mut host, &name, &nation, &mut fx)?;
        requests::check_faction_777610(&mut host, &name, &faction, &mut fx)
    }
}

/// The session model as `staging_requests`' Host. Definition stores keep the
/// wire projection's registry indices; the processes speak canonical IDS.
pub(crate) struct SessionHost<'a> {
    pub session: &'a mut Session,
    pub data: &'a dyn LanData,
    pub ordering: Option<u32>,
    pub local: Option<u32>,
    pub view: lobby::NodeView,
    pub now: u32,
}
impl SessionHost<'_> {
    fn ids(&self, registry: Registry, index: Option<u32>) -> Result<Vec<u16>> {
        let Some(index) = index else { return Ok(Vec::new()) };
        let name = self.data.registries().ids(registry, index);
        name.map(units).ok_or_else(|| Error::Protocol(format!("registry {:#x} has no slot {index}", registry.0)))
    }
    fn index(&self, registry: Registry, ids: &[u16]) -> Result<Option<u32>> {
        let ids = c_string(ids);
        if ids.is_empty() {
            return Ok(None);
        }
        let text = String::from_utf16_lossy(ids);
        let index = self.data.registries().index(registry, &text);
        index.map(Some).ok_or_else(|| Error::Protocol(format!("registry {:#x} has no definition {text}", registry.0)))
    }
    fn row_of(&self, k: &lobby::KingdomSetup) -> Result<Row> {
        Ok(Row {
            playable_28: k.flag_28,
            nation_18: self.ids(super::registries::NATIONS, k.nation)?,
            faction_1c: self.ids(super::registries::FACTIONS, k.faction)?,
            handicap_24: self.ids(super::registries::HANDICAPS, k.ai_handicap)?,
        })
    }
    fn record_mut(&mut self, index: usize) -> Result<&mut lobby::KingdomSetup> {
        self.session.launch.active_map_mut().header.kingdoms.get_mut(index).ok_or_else(|| Error::Protocol(format!("no kingdom record {index}")))
    }
    fn player_mut(&mut self, slot: usize) -> Result<&mut lobby::SessionPlayer> {
        self.session.players.get_mut(slot).ok_or_else(|| Error::Protocol(format!("no player slot {slot}")))
    }
}
impl RequestHost for SessionHost<'_> {
    fn countdown_118(&self) -> i32 {
        self.session.launch_countdown
    }
    fn ordering_284(&self) -> Option<u32> {
        self.ordering
    }
    fn local_8(&self) -> Option<u32> {
        self.local
    }
    fn admin_is_host_2c4(&self) -> bool {
        self.view.we_are_host()
    }
    fn player_slot(&self, id: u32) -> Option<usize> {
        self.session.players.iter().position(|p| p.id == id)
    }
    fn kingdom_player_76b920(&self, kingdom: &[u16]) -> Option<usize> {
        self.session.players.iter().position(|p| crate::session::wcs_eq(&p.kingdom, kingdom))
    }
    fn other_player_holds(&self, slot: usize, kingdom: &[u16]) -> bool {
        self.session.players.iter().enumerate().any(|(i, p)| i != slot && crate::session::wcs_eq(&p.kingdom, kingdom))
    }
    fn player_id(&self, slot: usize) -> u32 {
        self.session.players[slot].id
    }
    fn player_link(&self, slot: usize) -> PeerLink {
        self.session.players[slot].link
    }
    fn player_ai(&self, slot: usize) -> bool {
        self.session.players[slot].ai
    }
    fn player_kingdom(&self, slot: usize) -> Vec<u16> {
        self.session.players[slot].kingdom.clone()
    }
    fn next_player_id_e8(&mut self) -> Result<u32> {
        self.session.last_player_id = self.session.last_player_id.wrapping_add(1);
        Ok(self.session.last_player_id)
    }
    fn create_player_773710(&mut self, id: u32, link: PeerLink, ai: bool, implicit: bool, effects: &mut dyn FnMut(u32, bool, bool) -> Result<()>) -> Result<usize> {
        let node = lobby::JoinNode { view: self.view, own_address: None, now: self.now, internet: false, peers_changed: false, multiplayer_assert_ignored: false };
        lobby::create_player(self.session, id, link, ai, implicit, &node, &DataView { data: self.data })?;
        effects(id, ai, implicit)?;
        Ok(self.session.players.len() - 1)
    }
    fn set_text_14(&mut self, slot: usize, text: &[u16]) -> Result<()> {
        let p = self.player_mut(slot)?;
        p.text_14 = text.to_vec();
        p.flag_2c = true;
        Ok(())
    }
    fn store_name_7664f0(&mut self, slot: usize, name: &[u16]) -> Result<()> {
        let p = self.player_mut(slot)?;
        p.name = name.to_vec();
        p.flag_2c = true;
        Ok(())
    }
    fn unique_name_766560(&mut self, slot: usize, name: &[u16]) -> Result<()> {
        lobby::set_player_name(self.session, slot, name, &mut NameFormat { data: self.data })
    }
    fn set_kingdom_7664d0(&mut self, slot: usize, kingdom: &[u16]) -> Result<()> {
        let p = self.player_mut(slot)?;
        p.kingdom = kingdom.to_vec();
        p.flag_2c = true;
        Ok(())
    }
    fn set_position(&mut self, slot: usize, flag_38: bool, team: u32, position: u32) -> Result<()> {
        let p = self.player_mut(slot)?;
        p.flag_38 = flag_38;
        p.field_40 = team;
        p.field_3c = position;
        Ok(())
    }
    fn set_text_30(&mut self, slot: usize, nation: &[u16]) -> Result<()> {
        self.player_mut(slot)?.text_30 = nation.to_vec();
        Ok(())
    }
    fn set_text_34(&mut self, slot: usize, faction: &[u16]) -> Result<()> {
        self.player_mut(slot)?.text_34 = faction.to_vec();
        Ok(())
    }
    fn position_77e1f0(&self, kingdom: &[u16]) -> Result<Option<(u32, u32)>> {
        // An empty name answers -1, -1; a named one searches the session
        // info's lists, constructor-empty here, and finds nothing.
        Ok(c_string(kingdom).is_empty().then_some((u32::MAX, u32::MAX)))
    }
    fn changed_c8(&mut self) {
        self.session.changed = true;
    }
    fn row_index(&self, kingdom: &[u16]) -> Result<Option<usize>> {
        Ok(self.session.launch.active_map().header.kingdoms.iter().position(|k| crate::session::wcs_eq(&k.name, kingdom)))
    }
    fn row(&self, index: usize) -> Result<Row> {
        let k = self.session.launch.active_map().header.kingdoms.get(index).ok_or_else(|| Error::Protocol(format!("no kingdom record {index}")))?;
        self.row_of(k)
    }
    fn base_row(&self, kingdom: &[u16]) -> Result<Option<Row>> {
        // Session+0x68 is launch +0x14, the game settings' map header.
        match self.session.launch.game.header.kingdoms.iter().find(|k| crate::session::wcs_eq(&k.name, kingdom)) {
            Some(k) => self.row_of(k).map(Some),
            None => Ok(None),
        }
    }
    fn store_handicap(&mut self, index: usize, ids: &[u16], _mark: bool) -> Result<()> {
        let def = self.index(super::registries::HANDICAPS, ids)?;
        self.record_mut(index)?.ai_handicap = def;
        Ok(())
    }
    fn store_nation(&mut self, index: usize, ids: &[u16]) -> Result<()> {
        let def = self.index(super::registries::NATIONS, ids)?;
        self.record_mut(index)?.nation = def;
        Ok(())
    }
    fn store_faction(&mut self, index: usize, ids: &[u16], _mark: bool) -> Result<()> {
        let def = self.index(super::registries::FACTIONS, ids)?;
        self.record_mut(index)?.faction = def;
        Ok(())
    }
    fn mark_modified(&mut self, _index: usize) -> Result<()> {
        // The record's +0x68 byte is not part of the wire projection.
        Ok(())
    }
    fn find_definition(&self, registry: &str, ids: &[u16]) -> Result<Option<Vec<u16>>> {
        let registry = match registry {
            "Nation" => super::registries::NATIONS,
            "Faction" => super::registries::FACTIONS,
            other => return Err(Error::Protocol(format!("48D500 Find on registry {other}"))),
        };
        let ids = c_string(ids);
        if ids.is_empty() {
            return Ok(None);
        }
        let text = String::from_utf16_lossy(ids);
        Ok(self.data.registries().index(registry, &text).and_then(|i| self.data.registries().ids(registry, i)).map(units))
    }
    fn nation_factions(&self, nation: &[u16]) -> Result<Vec<Vec<u16>>> {
        self.data.nation_factions(nation)
    }
}

/// `staging_requests`' effects: 0x76DA60's message (the logs are not kept).
#[derive(Default)]
pub(crate) struct RequestMessages {
    pub messages: Vec<LanEvent>,
}
impl RequestEffects for RequestMessages {
    fn session_message_76da60(&mut self, key: &str, shown: bool) -> Result<()> {
        if shown {
            self.messages.push(LanEvent::Message { kind: 0x11, key: key.into(), args: Vec::new(), number: None });
        }
        Ok(())
    }
    fn ordering_player_null(&mut self) -> Result<()> {
        Ok(())
    }
    fn creating_player(&mut self, _id: u32, _ai: bool, _implicit: bool) -> Result<()> {
        Ok(())
    }
    fn check_nation(&mut self, _kingdom: &[u16], _nation: &[u16]) -> Result<()> {
        Ok(())
    }
    fn check_faction(&mut self, _kingdom: &[u16], _faction: &[u16]) -> Result<()> {
        Ok(())
    }
}

fn c_string(s: &[u16]) -> &[u16] {
    &s[..s.iter().position(|&u| u == 0).unwrap_or(s.len())]
}

/// 0x7744C0: destroys the AI players left without a kingdom (0x7737E0 and the
/// rated flag 0x770280 per player).
fn destroy_kingdomless_ais_7744c0(h: &mut SessionHost<'_>) -> Result<()> {
    let node = lobby::JoinNode { view: h.view, own_address: None, now: h.now, internet: false, peers_changed: false, multiplayer_assert_ignored: false };
    while let Some(k) = h.session.players.iter().position(|p| p.ai && c_string(&p.kingdom).is_empty()) {
        h.session.players.remove(k);
        h.session.changed = true;
        lobby::update_rated(h.session, &node, &DataView { data: h.data })?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The ordering host
// ---------------------------------------------------------------------------

/// The live order host of one execution (host 0x752E60 / client 0x74FD10).
pub(crate) struct Orders<'a, 'b, 'g, 't> {
    pub st: &'a RefCell<&'b mut State>,
    pub game: &'a RefCell<&'g mut dyn LanGame>,
    pub data: &'a dyn LanData,
    /// The frame's timeline, admin time (+0x28C before the tick's increment),
    /// gates and x87 precision (0x76DD90 / 0x76F370 run inside processes).
    pub timeline: super::SharedTimeline<'a, 't>,
    pub admin_time: aa_simulation::FloatBits,
    pub gates: aa_runtime::simulation::Gates,
    pub precision: aa_numeric::Precision,
}

impl Orders<'_, '_, '_, '_> {
    /// The AdminPeer of the client being executed (Admin+0x280).
    fn current_link(st: &State, client: &CurrentClient) -> PeerLink {
        match client {
            CurrentClient::Bound(ClientIdentity::OwnNode) => PeerLink::Node,
            CurrentClient::Bound(ClientIdentity::Remote(i)) => {
                st.conns.get(*i as usize).map_or(PeerLink::Null, |c| PeerLink::Conn(c.addr))
            }
            _ => PeerLink::Null,
        }
    }
    fn ordering_link(st: &State, slot: u32) -> Option<PeerLink> {
        nonnull(slot).and_then(|p| st.player_link(p))
    }
}

/// 0x75D710 player context over the slot and the session.
struct SetPlayer<'s> {
    st: &'s mut State,
    slot: &'s mut u32,
    client: PeerLink,
}
impl TryPlayerContext for SetPlayer<'_> {
    type Player = u32;
    type Client = PeerLink;
    type Error = std::convert::Infallible;
    fn film_mode(&mut self) -> std::result::Result<bool, Self::Error> {
        Ok(self.st.session.state == 4)
    }
    fn find_player(&mut self, id: u32, film: bool) -> std::result::Result<Option<u32>, Self::Error> {
        Ok(self.st.session.live_player(id, film))
    }
    fn player_is_film(&mut self, player: &u32) -> std::result::Result<bool, Self::Error> {
        Ok(self.st.session.players.iter().any(|p| p.id == *player && p.film))
    }
    fn is_authorizing_admin(&mut self) -> std::result::Result<bool, Self::Error> {
        Ok(self.st.role == Role::Host)
    }
    fn player_client(&mut self, player: &u32) -> std::result::Result<PeerLink, Self::Error> {
        Ok(self.st.player_link(*player).unwrap_or(PeerLink::Null))
    }
    fn ordering_client(&mut self) -> std::result::Result<PeerLink, Self::Error> {
        Ok(self.client)
    }
    fn bind_client_player(&mut self, client: PeerLink, player: &u32) -> std::result::Result<(), Self::Error> {
        // the client's +0x2C
        match client {
            PeerLink::Node => self.st.own_ordering = Some(*player),
            l => {
                if let Some(c) = self.st.conn_of_link(&l) {
                    c.admin.base.ordering_player = Some(*player);
                }
            }
        }
        Ok(())
    }
    fn set_current_player(&mut self, player: Option<u32>) -> std::result::Result<(), Self::Error> {
        *self.slot = player.unwrap_or(0);
        Ok(())
    }
    fn stop_film(&mut self) -> std::result::Result<(), Self::Error> {
        Ok(())
    }
    fn set_abort(&mut self) -> std::result::Result<(), Self::Error> {
        Ok(())
    }
}

impl OrderingHost for Orders<'_, '_, '_, '_> {
    fn acknowledge_order_version(&mut self, slot: &mut u32, _client: &CurrentClient) -> aa_orders::Result<()> {
        // 751FC0: the ordering player's client +0x6C = 1.
        let mut st = self.st.try_borrow_mut().map_err(orders_err)?;
        if let Some(link) = Orders::ordering_link(&st, *slot) {
            st.set_link_6c(&link, 1);
        }
        Ok(())
    }
    fn has_ordering_client(&mut self, slot: &mut u32, _client: &CurrentClient) -> aa_orders::Result<bool> {
        // 0x766C30: admin+0x284 player and its +4 client.
        let st = self.st.try_borrow().map_err(orders_err)?;
        Ok(Orders::ordering_link(&st, *slot).is_some_and(|l| l != PeerLink::Null))
    }
    fn version_acknowledged(&mut self, slot: &mut u32, _client: &CurrentClient) -> aa_orders::Result<bool> {
        // 0x766C40: the ordering client's +0x6C.
        let st = self.st.try_borrow().map_err(orders_err)?;
        Ok(Orders::ordering_link(&st, *slot).is_some_and(|l| st.link_6c(&l) != 0))
    }
    fn film_session(&mut self, _slot: &mut u32, _client: &CurrentClient) -> aa_orders::Result<bool> {
        let st = self.st.try_borrow().map_err(orders_err)?;
        Ok(st.session.state == 4)
    }
    fn valid(&mut self, _slot: &mut u32, _client: &CurrentClient, validity: Validity, request: usize, _fields: &[Field]) -> aa_orders::Result<bool> {
        Ok(match validity {
            Validity::None => true,
            // [[0xB39AB0]+0x10C] cheats enabled: never in a LAN session here,
            // and a cheat request without the permission writes no decree.
            Validity::CheatsEnabled => false,
            Validity::Function(address) => {
                return Err(aa_orders::Error::Handler {
                    index: request,
                    position: Position::PACKET_START,
                    message: format!("order validity function {address:#x} is not ported"),
                });
            }
        })
    }
    fn sync(&mut self, _slot: &mut u32, _client: &CurrentClient, entry: SyncEntry) -> aa_orders::Result<()> {
        let mut st = self.st.try_borrow_mut().map_err(orders_err)?;
        let st = &mut **st;
        match entry {
            SyncEntry::Int { log, label, value } => {
                if log == Log::A && label == "o.GetBitSize()" {
                    st.stream_bits = value;
                }
                let s = if log == Log::A { &mut st.sync_a } else { &mut st.sync_s };
                s.log_int(true, label, value);
            }
            SyncEntry::Text { log, label, value } => {
                let s = if log == Log::A { &mut st.sync_a } else { &mut st.sync_s };
                s.log_text(true, label, &units(value));
            }
        }
        Ok(())
    }
    fn process(&mut self, slot: &mut u32, client: &CurrentClient, decree: usize, fields: &mut Vec<Field>) -> aa_orders::Result<()> {
        self.run(slot, client, decree, fields, None).map_err(orders_err)
    }
    fn process_with_emission(
        &mut self,
        slot: &mut u32,
        client: &CurrentClient,
        decree: usize,
        fields: &mut Vec<Field>,
        emission: &mut RequestEmission<'_>,
    ) -> aa_orders::Result<()> {
        self.run(slot, client, decree, fields, Some(emission)).map_err(orders_err)
    }
    fn mirror_enabled(&mut self, _slot: &mut u32, _client: &CurrentClient) -> aa_orders::Result<bool> {
        // 0x766C50: no film recorder (AdminNode+0x268) in a LAN session.
        Ok(false)
    }
}

impl Orders<'_, '_, '_, '_> {
    fn run(&mut self, slot: &mut u32, client: &CurrentClient, decree: usize, fields: &mut [Field], mut emission: Option<&mut RequestEmission<'_>>) -> Result<()> {
        let ordering = nonnull(*slot);
        if is_gameplay_decree(decree) {
            // The host's process sees the order with its references as the
            // reader resolved them (the client reads the re-serialized decree).
            let resolved = {
                let game = self.game.try_borrow().map_err(super::err)?;
                let r = ThingOnly { game: &**game };
                payload::normalize_for_decree(fields, &r, &DataView { data: self.data })?
            };
            return self.game.try_borrow_mut().map_err(super::err)?.decree(ordering, decree, &resolved);
        }
        let mut guard = self.st.try_borrow_mut().map_err(super::err)?;
        let st: &mut State = &mut guard;
        let view = st.view();
        match decree {
            d::SET_PLAYER_DECREE => {
                let id = uint(fields, 0)?;
                let link = Orders::current_link(st, client);
                let mut ctx = SetPlayer { st, slot, client: link };
                let _ = cmds::try_process_set_player(id, &mut ctx);
            }
            d::PLAYER_JOIN_DECREE => self.player_join(st, fields, ordering)?,
            // 0x75DA90: `ret 4`.
            d::POST_PLAYER_JOIN_DECREE => {}
            d::PLAYER_LEAVE_DECREE => {
                self.player_leave(st, uint(fields, 0)?, uint(fields, 1)?, emission)?;
            }
            d::SYNC_VALUE_DECREE => {
                // 0x751D00 (host only: AdminNode == +0x2C4).
                if st.role == Role::Host
                    && let Some(player) = ordering
                {
                    let index = uint(fields, 0)? as i32;
                    let hash = uint(fields, 1)? as i32;
                    let link = st.player_link(player);
                    let oos = |st: &State, l: &PeerLink| match l {
                        PeerLink::Conn(a) => st.conns.iter().find(|c| c.addr == *a).is_some_and(|c| c.admin.base.flag_30 != 0),
                        _ => false,
                    };
                    if let Some(link) = link
                        && link != PeerLink::Null
                        && !oos(st, &link)
                        && !st.sync_a.validate(index, hash)
                    {
                        // other_oos: another player's client already out of sync.
                        let other = st.session.players.iter().filter_map(|p| (p.link != link).then_some(p.link)).any(|l| oos(st, &l));
                        if !other {
                            st.deferred.sync_failures.push(player);
                        }
                        if let Some(c) = st.conn_of_link(&link) {
                            c.admin.base.flag_31 = 1;
                        }
                    }
                }
            }
            d::SYNC_FAILURE_DECREE => {
                // 0x75BBC0: "SyncFailure(%d)" and the system messages.
                let player = uint(fields, 0)?;
                st.events.push(LanEvent::SyncFailure { player });
            }
            // 0x752F60: compared only when the ordering player is a film player
            // (there are none in a LAN session; a film peer is refused).
            d::SIM_SYNC_DECREE => {
                if ordering.is_some_and(|o| st.session.players.iter().any(|p| p.id == o && p.film)) {
                    return Err(Error::Unsupported("0x752F60 film sync check in a LAN session"));
                }
            }
            d::START_COUNTDOWN_DECREE => {
                let node = staging_node(st, &view, ordering);
                let dir = SnapshotDirectory::of(st);
                let mut fx = StagingEffects::new(st, self.data);
                staging::start_countdown(&mut st.session, &node, &dir, &mut fx)?;
                let freeze = std::mem::take(&mut fx.freeze);
                fx.apply(st, emission.as_deref_mut(), self.data)?;
                for f in freeze {
                    self.set_frozen(st, f)?;
                }
            }
            d::STOP_COUNTDOWN_DECREE => {
                let node = staging_node(st, &view, ordering);
                let mut fx = StagingEffects::new(st, self.data);
                staging::stop_countdown(&mut st.session, &node, &mut fx)?;
                let freeze = std::mem::take(&mut fx.freeze);
                fx.apply(st, emission.as_deref_mut(), self.data)?;
                for f in freeze {
                    self.set_frozen(st, f)?;
                }
            }
            d::COUNTDOWN_PROGRESS_DECREE => {
                let value = uint(fields, 0)? as i32;
                let node = staging_node(st, &view, ordering);
                let mut fx = StagingEffects::new(st, self.data);
                staging::countdown_progress(&mut st.session, value, &node, &mut fx)?;
                let freeze = std::mem::take(&mut fx.freeze);
                fx.apply(st, emission.as_deref_mut(), self.data)?;
                for f in freeze {
                    self.set_frozen(st, f)?;
                }
            }
            d::LAUNCH_DECREE => {
                let node = staging_node(st, &view, ordering);
                let mut fx = StagingEffects::new(st, self.data);
                staging::launch(&mut st.session, &node, &mut fx)?;
                let freeze = std::mem::take(&mut fx.freeze);
                fx.apply(st, emission.as_deref_mut(), self.data)?;
                for f in freeze {
                    self.set_frozen(st, f)?;
                }
                st.events.push(LanEvent::PlayersChanged);
            }
            d::SET_NEW_STATE_DECREE => {
                // 0x7745C0: +0x54 = state, 0x4145C0 assigns the launch state.
                st.session.launch.game_type = uint(fields, 0)?;
                let s = &mut st.session.launch.state;
                s.kind = uint(fields, 1)?;
                s.flag_04 = uint(fields, 2)? != 0;
                s.text = text_field(fields, 3)?;
                s.field_10 = uint(fields, 4)?;
                s.flag_14 = uint(fields, 5)? != 0;
                s.field_24 = uint(fields, 6)?;
                s.flag_15 = uint(fields, 7)? != 0;
                s.flag_16 = uint(fields, 8)? != 0;
                s.field_18 = uint(fields, 9)?;
                s.field_1c = uint(fields, 10)?;
                s.flag_20 = uint(fields, 11)? != 0;
            }
            d::SET_READY_DECREE => {
                // 0x7712E0: only the ordering player itself.
                let player = uint(fields, 0)?;
                let status = uint(fields, 1)?;
                if player != 0 && Some(player) == ordering {
                    // 0x765EA0: an AI or the host's player is always 2.
                    if let Some(p) = st.session.players.iter_mut().find(|p| p.id == player) {
                        if p.ai || view.is_host(&p.link) {
                            p.status = 2;
                            p.flag_2c = true;
                        } else if p.status != status {
                            p.status = status;
                            p.flag_2c = true;
                        }
                    }
                    st.events.push(LanEvent::PlayersChanged);
                } else if st.local_player == ordering {
                    st.events.push(LanEvent::Message { kind: 0x11, key: "set_player_ready_failed".into(), args: Vec::new(), number: None });
                }
            }
            d::PUBLIC_CHAT_DECREE => {
                // 0x755E00: nothing without an ordering and a local player.
                if ordering.is_some() && st.local_player.is_some() {
                    st.events.push(LanEvent::Chat { player: ordering, fields: fields.to_vec() });
                }
            }
            d::SET_MAP_SEED_DECREE => {
                // 0x7710E0: +0x100 = seed unless a countdown runs.
                if st.session.launch_countdown < 1 {
                    st.session.field_100 = uint(fields, 0)?;
                } else if st.local_player == ordering {
                    st.events.push(LanEvent::Message {
                        kind: 0x11,
                        key: "session_order_rejected_currently_counting_down".into(),
                        args: Vec::new(),
                        number: None,
                    });
                }
            }
            d::SET_PLAYER_KINGDOM_DECREE => {
                // 0x778170.
                let player = uint(fields, 0)?;
                let kingdom = text_field(fields, 1)?;
                let ordering_link = ordering.and_then(|p| st.player_link(p));
                let host_orders = ordering_link.is_some_and(|l| view.is_host(&l));
                let own = st.player_link(player) == ordering_link;
                let reject = if !host_orders && !own {
                    Some(requests::PLAYER_KINGDOM_FAILED_KEY)
                } else if st.session.launch_countdown > 0 {
                    Some(requests::COUNTING_DOWN_KEY)
                } else {
                    None
                };
                if let Some(key) = reject {
                    if st.local_player == ordering {
                        st.events.push(LanEvent::Message { kind: 0x11, key: key.into(), args: Vec::new(), number: None });
                    }
                } else if player != 0 {
                    let taken = st.session.players.iter().any(|p| crate::session::wcs_eq(&p.kingdom, &kingdom));
                    if c_string(&kingdom).is_empty() || !taken {
                        let mut fx = RequestMessages::default();
                        let mut host = self.session_host(st, ordering);
                        let slot = host.player_slot(player).ok_or_else(|| Error::Protocol(format!("0x778170 on absent player {player}")))?;
                        // 0x7782BF: notify 1; the checks run whatever it returned.
                        requests::set_player_kingdom_777f00(&mut host, slot, &kingdom, true, &mut fx)?;
                        let (name, nation, faction) = {
                            let p = &host.session.players[slot];
                            (p.kingdom.clone(), p.text_30.clone(), p.text_34.clone())
                        };
                        requests::check_nation_777cd0(&mut host, &name, &nation, &mut fx)?;
                        requests::check_faction_777610(&mut host, &name, &faction, &mut fx)?;
                        destroy_kingdomless_ais_7744c0(&mut host)?;
                        st.events.extend(fx.messages);
                        st.events.push(LanEvent::PlayersChanged);
                    }
                }
            }
            d::CREATE_EXPLICIT_AI_DECREE => {
                // 0x754F50.
                let (kingdom, profile, name) = (text_field(fields, 0)?, text_field(fields, 1)?, text_field(fields, 2)?);
                let mut fx = RequestMessages::default();
                requests::create_explicit_ai_754f50(&mut self.session_host(st, ordering), &kingdom, &profile, &name, &mut fx)?;
                st.events.extend(fx.messages);
                st.events.push(LanEvent::PlayersChanged);
            }
            d::SET_KINGDOM_NATION_DECREE | d::SET_KINGDOM_FACTION_DECREE | d::SET_AI_HANDICAP_DECREE => {
                // 0x777D90 / 0x777700 / 0x771880.
                let kingdom = text_field(fields, 0)?;
                let registry = match fields.get(1) {
                    Some(Field::Def { registry, .. }) => *registry,
                    _ => return Err(Error::Protocol("kingdom definition field".into())),
                };
                let count = self.data.registries().count(registry).unwrap_or(0);
                let def = def_field(fields, 1, count)?;
                let mut fx = RequestMessages::default();
                let mut host = self.session_host(st, ordering);
                let ids = match def {
                    Some(_) => Some(host.ids(registry, def)?),
                    None => None,
                };
                let ids = ids.as_deref();
                match decree {
                    d::SET_KINGDOM_NATION_DECREE => requests::set_kingdom_nation_777d90(&mut host, &kingdom, ids, &mut fx)?,
                    d::SET_KINGDOM_FACTION_DECREE => requests::set_kingdom_faction_777700(&mut host, &kingdom, ids, &mut fx)?,
                    _ => requests::set_ai_handicap_771880(&mut host, &kingdom, ids, &mut fx)?,
                }
                st.events.extend(fx.messages);
                st.events.push(LanEvent::PlayersChanged);
            }
            d::SET_KINGDOM_COLOR_DECREE => {
                // 0x771C70.
                let kingdom = text_field(fields, 0)?;
                let count = self.data.registries().count(super::registries::KINGDOM_COLORS).unwrap_or(0);
                let def = def_field(fields, 1, count)?;
                let local = st.local_player;
                let host = self.session_host(st, ordering);
                if requests::may_change_76c560(&host, &kingdom)? {
                    let index = host.row_index(&kingdom)?.ok_or_else(|| Error::Protocol("0x771C70 NULL kingdom record (native NULL write)".into()))?;
                    st.session.launch.active_map_mut().header.kingdoms[index].color = def;
                    st.events.push(LanEvent::PlayersChanged);
                } else if local == ordering {
                    st.events.push(LanEvent::Message { kind: 0x11, key: "set_kingdom_color_failed".into(), args: Vec::new(), number: None });
                }
            }
            d::SET_MAP_TRANSFER_STATUS_DECREE => {
                // 0x7554B0 (host): not from the host's own client (log 0xA2C),
                // not an obsolete status (log; +0x2E0 is never set here).
                if st.role == Role::Host
                    && let Some(player) = ordering
                {
                    let status = uint(fields, 0)?;
                    let file = text_field(fields, 1)?;
                    let word = uint(fields, 2)?;
                    let link = st.player_link(player);
                    if link.is_some_and(|l| l != PeerLink::Node) && word == st.session.field_154 {
                        if let Some(c) = link.and_then(|l| st.conns.iter_mut().find(|c| PeerLink::Conn(c.addr) == l)) {
                            c.admin.base.transfer_status = status as i32;
                        }
                        if crate::session::wcs_eq(&file, &st.session.launch.file) {
                            match status {
                                1 => {}
                                3 => {
                                    let key = if st.session.launch.field_0c == 5 { "map_will_not_transfer_host_film|ss" } else { "map_will_not_transfer_host|ss" };
                                    let name = st.session.players.iter().find(|p| p.id == player).map(|p| p.name.clone()).unwrap_or_default();
                                    st.events.push(LanEvent::Message { kind: 0x10, key: key.into(), args: vec![name, file.clone()], number: None });
                                }
                                // 0x74FE30 starts sending the file.
                                2 => return Err(Error::Unsupported("0x74FE30 map file transfer in a LAN session")),
                                _ => return Ok(()),
                            }
                            // 0x74EA20: UpdateMapStatus {client+8, status, 1.0f}.
                            let req = (
                                d::UPDATE_MAP_STATUS_REQUEST,
                                vec![Field::Player(player), Field::Bounded { n: 4, value: status }, Field::Uint { width: 32, value: 0x3f80_0000 }],
                            );
                            match emission.as_deref_mut() {
                                Some(e) => {
                                    let local = st.local_player.unwrap_or(0);
                                    e.emit_request(true, req.0, &req.1, local, &DataView { data: self.data })?;
                                }
                                None => st.deferred.requests.push(req),
                            }
                        }
                    }
                }
            }
            d::UPDATE_MAP_STATUS_DECREE => {
                // 0x74BBB0: only ordered by the host; player's client +0x3C/+0x40.
                let host_orders = ordering.and_then(|p| st.player_link(p)).is_some_and(|l| view.is_host(&l));
                let player = uint(fields, 0)?;
                if host_orders && player != 0 {
                    let status = uint(fields, 1)?;
                    let crc = uint(fields, 2)?;
                    match st.player_link(player) {
                        // (the node's own +0x40 is not part of its identity here)
                        Some(PeerLink::Node) => st.own_identity.field_3c = status,
                        Some(l) => {
                            if let Some(c) = st.conn_of_link(&l) {
                                c.admin.base.field_3c = status as i32;
                                c.admin.base.field_40 = crc as i32;
                            }
                        }
                        None => {}
                    }
                    st.events.push(LanEvent::PlayersChanged);
                }
            }
            d::TASK_STATUS_DECREE => self.task_status(st, fields, ordering)?,
            d::SET_GAME_SPEED_DECREE => {
                // 0x75D810: the ordering player's client must be the host.
                let link = ordering.and_then(|p| st.player_link(p));
                let authority = crate::speed::SpeedAuthority {
                    session: true,
                    ordering_player: ordering.is_some(),
                    ordering_client: link.is_some_and(|l| l != PeerLink::Null),
                    client_is_host: link.is_some_and(|l| view.is_host(&l)),
                };
                if let Some(speed) = crate::speed::set_game_speed_gate(fields, &authority, None)? {
                    self.set_game_speed(st, speed.0)?;
                }
            }
            d::SET_SESSION_LOAD_MAP_DECREE => self.set_session_load_map(st, fields, ordering)?,
            d::SET_TEAMS_DECREE => self.set_teams(st, fields, ordering)?,
            d::SET_MAP_SIZE_DECREE => {
                // 0x75D960: the active creator's size, unless a countdown runs.
                if self.counting_down(st, ordering) {
                    return Ok(());
                }
                let (x, y) = (uint(fields, 0)?, uint(fields, 1)?);
                let map = st.session.launch.active_map_mut();
                map.size_x = x;
                map.size_y = y;
                st.session.changed = true;
                st.events.push(LanEvent::PlayersChanged);
            }
            d::SET_RMC_DECREE => self.set_rmc(st, fields, ordering)?,
            d::SET_RMC_SETTINGS_DECREE => {
                // 0x75D8C0 -> 0x5065C0: the settings of the random-map creator.
                if self.counting_down(st, ordering) {
                    return Ok(());
                }
                st.session.launch.rmc_settings = rmc_settings_of(fields)?;
                st.session.changed = true;
                st.events.push(LanEvent::PlayersChanged);
            }
            d::SET_MAX_PLAYERS_DECREE => {
                st.session.options.max_players = uint(fields, 0)? as i32;
                st.events.push(LanEvent::PlayersChanged);
            }
            d::SET_GAME_NAME_DECREE => {
                st.session.options.name = text_field(fields, 0)?;
                st.events.push(LanEvent::PlayersChanged);
            }
            d::SET_GAME_PASSWORD_DECREE => {
                st.session.options.password = text_field(fields, 0)?;
                st.events.push(LanEvent::PlayersChanged);
            }
            d::SET_ALLOW_OBSERVERS_DECREE => {
                st.session.options.allow_observers = uint(fields, 0)? != 0;
                st.events.push(LanEvent::PlayersChanged);
            }
            d::SET_PLAYER_NAME_DECREE => {
                // 0x75E180: the player's name, made unique (0x766560).
                let player = uint(fields, 0)?;
                let name = text_field(fields, 1)?;
                if let Some(index) = st.session.players.iter().position(|p| p.id == player) {
                    let mut names = NameFormat { data: self.data };
                    lobby::set_player_name(&mut st.session, index, &name, &mut names)?;
                    st.events.push(LanEvent::PlayersChanged);
                }
            }
            d::KICK_PLAYER_DECREE => {
                // 0x75DAE0: the host only; PlayerLeave with reason 4 (kicked).
                let player = uint(fields, 0)?;
                let host_orders = ordering.and_then(|p| st.player_link(p)).is_some_and(|l| view.is_host(&l));
                if host_orders && st.role == Role::Host && player != 0 {
                    let req = (d::PLAYER_LEAVE_REQUEST, vec![Field::Player(player), Field::Bounded { n: 13, value: 4 }]);
                    match emission.as_deref_mut() {
                        Some(e) => {
                            let local = st.local_player.unwrap_or(0);
                            e.emit_request(true, req.0, &req.1, local, &DataView { data: self.data })?;
                        }
                        None => st.deferred.requests.push(req),
                    }
                }
            }
            d::TOGGLE_PAUSED_DECREE => self.vote_paused(st, ordering, None)?,
            d::SET_PAUSED_DECREE => {
                let v = uint(fields, 0)? != 0;
                self.vote_paused(st, ordering, Some(v))?;
            }
            d::SPEED_UP_DECREE | d::SPEED_DOWN_DECREE => {
                // 0x75DAC0 / 0x75DAD0 -> 0x76F530: the host's step of the
                // exponent (refused for a non-host and while GWorld+0x275).
                let host_orders = ordering.and_then(|p| st.player_link(p)).is_some_and(|l| view.is_host(&l));
                if host_orders {
                    let dir = if decree == d::SPEED_UP_DECREE { 0.5f32 } else { -0.5 };
                    let current = {
                        let t = self.timeline.try_borrow().map_err(super::err)?;
                        f32::from_bits(t.speed.0)
                    };
                    // (0x76F530) floor(v / 0.5 + 0.5) * 0.5 + 0.5 * dir, 0x76F370 clamps.
                    let v = (current / 0.5 + 0.5).floor() * 0.5 + dir;
                    self.set_game_speed(st, v.to_bits())?;
                }
            }
            // No process here (`has_process`): refused by name, never a no-op.
            _ => return Err(refused(decree)),
        }
        Ok(())
    }

    /// The rejection shared by the staging setters: `session+0x118 >= 1`.
    fn counting_down(&self, st: &mut State, ordering: Option<u32>) -> bool {
        if st.session.launch_countdown < 1 {
            return false;
        }
        if st.local_player == ordering {
            st.events.push(LanEvent::Message { kind: 0x11, key: requests::COUNTING_DOWN_KEY.into(), args: Vec::new(), number: None });
        }
        true
    }

    /// 0x7791F0 after a change of map or format: the AIs go (their kingdoms
    /// are of the old map, 0x7744C0) and the people sit down again in list
    /// order (0x778D10).
    fn reseat(&self, st: &mut State) -> Result<()> {
        let view = st.view();
        let local = st.local_player;
        let now = st.now.to_bits();
        st.session.players.retain(|p| !p.ai);
        let ids: Vec<u32> = st.session.players.iter().filter(|p| !p.film).map(|p| p.id).collect();
        for p in &mut st.session.players {
            p.kingdom.clear();
            p.flag_2c = true;
        }
        let mut assign = KingdomAssign { data: self.data, view, local, now };
        for id in ids {
            staging::auto_assign_kingdom(&mut st.session, id, &mut assign)?;
        }
        st.session.changed = true;
        st.session.changed_c9 = true;
        st.events.push(LanEvent::PlayersChanged);
        Ok(())
    }

    /// TGC_SetSessionLoadMapOrder 0x75D860 -> KKC_Session::SetSessionLoadMap
    /// 0x779780(file, header, crc, 0): ordered by the host alone, refused
    /// during a countdown. +0x154 += 1, launch +0 and +4 = 2, the file, the
    /// header ([+0x14]: 0x5A46D0), +0xC = 2, +0xC9, +0xC8, +0xFC = crc; a client
    /// reports its map status (0x770B70); the creator from the header
    /// (0x5144A0), the format reset (0x781DD0) and the seats (0x7791F0).
    fn set_session_load_map(&self, st: &mut State, fields: &[Field], ordering: Option<u32>) -> Result<()> {
        let view = st.view();
        let host_orders = ordering.and_then(|p| st.player_link(p)).is_some_and(|l| view.is_host(&l));
        if !host_orders {
            if st.local_player == ordering {
                st.events.push(LanEvent::Message { kind: 0x11, key: "session_order_rejected".into(), args: Vec::new(), number: None });
            }
            return Ok(());
        }
        if self.counting_down(st, ordering) {
            return Ok(());
        }
        let (file, game, crc) = super::read_load_map_fields(fields, self.data, &st.session.launch.game)?;
        let session = &mut st.session;
        session.field_154 = session.field_154.wrapping_add(1);
        session.launch.game_type = 2;
        session.launch.mode = 2;
        session.launch.file = file.clone();
        session.launch.field_0c = 2;
        session.map_crc = crc;
        // 0x5144A0: the scenario creator takes the header.
        session.launch.map_scenario.header = game.header.clone();
        session.launch.map_scenario.conditions = game.conditions.clone();
        session.launch.scenario = Some(file.clone());
        session.launch.game = game;
        // 0x781DD0.
        session.info_text.clear();
        if st.role != Role::Host {
            let word = st.session.field_154;
            st.deferred.requests.push((d::SET_MAP_TRANSFER_STATUS_REQUEST, vec![Field::Bounded { n: 4, value: 1 }, Field::Text(file), Field::Uint { width: 32, value: word }]));
        }
        self.reseat(st)
    }

    /// TGC_SetRMCOrder 0x75D970: the random-map configuration, mode 0, its
    /// kingdom records from the local data (the wire carries the name alone).
    fn set_rmc(&self, st: &mut State, fields: &[Field], ordering: Option<u32>) -> Result<()> {
        if self.counting_down(st, ordering) {
            return Ok(());
        }
        let name = text_field(fields, 0)?;
        let header = self.data.rmc_header(&name).ok_or_else(|| Error::Protocol(format!("no random map configuration {}", String::from_utf16_lossy(&name))))?;
        let session = &mut st.session;
        session.field_154 = session.field_154.wrapping_add(1);
        session.launch.game_type = 2;
        session.launch.mode = 0;
        session.launch.field_0c = 1;
        session.launch.file.clear();
        session.launch.random_map = Some(name);
        session.launch.map_random.header = header;
        session.info_text.clear();
        self.reseat(st)
    }

    /// TGC_SetTeamManagementTeamsOrder 0x75E2D0: the format text and the seats
    /// (0x781250, 0x7791F0).
    fn set_teams(&self, st: &mut State, fields: &[Field], ordering: Option<u32>) -> Result<()> {
        if self.counting_down(st, ordering) {
            return Ok(());
        }
        st.session.info_text = text_field(fields, 0)?;
        self.reseat(st)
    }

    /// 0x75D930 (toggle, 0x754EC0) / 0x75D940 -> 0x754880 the pause vote:
    /// AdminNode+0x2A0 is the client who paused, +0x2A4 the last one, +0x2A8
    /// when it ended; a client's +0x56 is its vote to unpause, +0x58 when it
    /// last paused. Only a player with a kingdom pauses. Whoever paused
    /// unpauses at once; with three humans or more the others vote, and half
    /// of the kingdoms plus one unpause (0x74D2A0, 0x74D270).
    fn vote_paused(&self, st: &mut State, ordering: Option<u32>, value: Option<bool>) -> Result<()> {
        let paused = self.timeline.try_borrow().map_err(super::err)?.paused;
        let want = value.unwrap_or(!paused);
        if want == paused {
            return Ok(());
        }
        let Some(id) = ordering else { return Ok(()) };
        let Some(client) = st.player_link(id) else { return Ok(()) };
        let holds_kingdom = |st: &State, l: &PeerLink| st.session.players.iter().any(|p| p.link == *l && !p.ai && !p.kingdom.is_empty());
        if !holds_kingdom(st, &client) {
            return Ok(());
        }
        let name = st.session.players.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default();
        let humans = st.session.players.iter().filter(|p| !p.ai && !p.film).count();
        let message = |st: &mut State, key: &str, args: Vec<Vec<u16>>| st.events.push(LanEvent::Message { kind: 0xF, key: key.into(), args, number: None });
        let count = |st: &State| -> (usize, usize) {
            let voters: Vec<PeerLink> = st.peers.iter().filter(|l| holds_kingdom(st, l)).copied().collect();
            let votes = voters.iter().filter(|l| st.pause_votes.iter().any(|v| v == *l)).count();
            (votes, voters.len() / 2 + 1)
        };
        let number = |n: usize| units(&n.to_string());
        let now = st.ms as f32 * 0.001;
        if !want {
            if st.pause_holder != Some(client) {
                if humans < 3 && st.pause_holder.is_some() {
                    return Ok(());
                }
                let was = st.pause_votes.iter().position(|v| *v == client);
                match was {
                    Some(i) => {
                        st.pause_votes.remove(i);
                    }
                    None => st.pause_votes.push(client),
                }
                let (votes, needed) = count(st);
                let key = if was.is_some() { "networking_vote_to_pause_player_retracts_vote|sdd" } else { "networking_vote_to_pause_player_votes_to_unpause|sdd" };
                message(st, key, vec![name, number(votes), number(needed)]);
                if votes < needed {
                    return Ok(());
                }
                self.timeline.try_borrow_mut().map_err(super::err)?.paused = false;
                if let Some(h) = st.pause_holder {
                    st.pause_last_paused.retain(|(l, _)| *l != h);
                    st.pause_last_paused.push((h, now));
                }
                st.pause_last = st.pause_holder.take();
                st.pause_votes.clear();
                st.pause_ended = -10000.0;
                message(st, "networking_vote_to_pause_game_unpaused", Vec::new());
            } else {
                self.timeline.try_borrow_mut().map_err(super::err)?.paused = false;
                st.pause_last = st.pause_holder.take();
                st.pause_ended = now;
                message(st, "networking_vote_to_pause_player_game_unpaused|s", vec![name]);
            }
        } else {
            // 0x74D2D0: the votes go, except for the last holder within the delay.
            if st.pause_last != Some(client) || st.pause_ended + PAUSE_DELAY < now {
                st.pause_votes.clear();
            }
            // 0x74B080: a client's next pause only after the delay.
            let last = st.pause_last_paused.iter().find(|(l, _)| *l == client).map(|(_, t)| *t);
            if last.is_some_and(|t| !(t + PAUSE_DELAY < now)) {
                message(st, "networking_vote_to_pause_pause_rejected|s", vec![name]);
                return Ok(());
            }
            self.timeline.try_borrow_mut().map_err(super::err)?.paused = true;
            st.pause_holder = Some(client);
            message(st, "networking_vote_to_pause_player_game_paused|s", vec![name]);
            if humans >= 3 {
                let (votes, needed) = count(st);
                message(st, "networking_vote_to_pause_needed_votes|dd", vec![number(needed), number(votes)]);
            }
        }
        st.events.push(LanEvent::Paused { paused: want, player: ordering });
        Ok(())
    }

    fn session_host<'s>(&'s self, st: &'s mut State, ordering: Option<u32>) -> SessionHost<'s> {
        let (view, local, now) = (st.view(), st.local_player, st.now.to_bits());
        SessionHost { session: &mut st.session, data: self.data, ordering, local, view, now }
    }

    /// 0x76DD90 KKC_Sim::SetFrozen inside the admin tick (aa-runtime
    /// `SimMath::set_frozen`): leaving the frozen state anchors both game ends
    /// at g_gworld+0xE0 and the admin ends at the admin clock estimate (the
    /// real admin clock capped by min(next tick event, +0x28C); the tick ran
    /// because the real clock reached that cap, so it is +0x28C on every node).
    fn set_frozen(&self, st: &mut State, frozen: bool) -> Result<()> {
        use aa_numeric::Binary;
        let world = self.game.try_borrow().map_err(super::err)?.world_clock();
        if !frozen && world.is_none() {
            return Err(Error::Protocol("0x76DD90 SetFrozen(false) requires g_gworld".into()));
        }
        let pc = self.precision;
        let mut t = self.timeline.try_borrow_mut().map_err(super::err)?;
        if t.frozen && !frozen {
            let world = world.expect("checked");
            let real = super::real_admin_clock(st)?;
            let next = t.next_tick_event_admin_time(&self.gates, Some(world.end), pc).map_err(super::err)?;
            let admin = Binary::from_f32_bits(self.admin_time.0).map_err(super::err)?;
            let cap = if next.compare(&admin).is_le() { next } else { admin };
            let real = Binary::from_f32_bits(real.0).map_err(super::err)?;
            let estimate = if real.compare(&cap).is_lt() { real } else { cap };
            let admin_bits = estimate.store_f32().map_err(super::err)?;
            let prior = Binary::from_f32_bits(admin_bits)
                .map_err(super::err)?
                .sub(&Binary::from_i32(1), pc)
                .map_err(super::err)?
                .store_f32()
                .map_err(super::err)?;
            t.begin_game = world.begin;
            t.begin_admin = aa_simulation::FloatBits(prior);
            t.end_game = world.begin;
            t.end_admin = aa_simulation::FloatBits(admin_bits);
            t.max_speed = aa_simulation::FloatBits(aa_runtime::simulation::MAX_SPEED_BITS);
        }
        st.sync_a.log_bool(true, "_frozen", frozen);
        t.frozen = frozen;
        Ok(())
    }

    /// KKC_Sim::SetGameSpeed 0x76F370 from the decree process 0x75D810, its
    /// A entry "m_game_time_per_tick" (0x76F467).
    fn set_game_speed(&self, st: &mut State, bits: u32) -> Result<()> {
        let world = self.game.try_borrow().map_err(super::err)?.world_clock();
        let settings = self.data.speed_settings();
        let context = aa_runtime::simulation::SpeedContext {
            session_mode: st.session.state as i32,
            session_f4: true,
            world_cutscene: world.map(|_| false),
        };
        let mut t = self.timeline.try_borrow_mut().map_err(super::err)?;
        let change = t
            .set_game_speed(aa_simulation::FloatBits(bits), &settings, &context, aa_runtime::simulation::PowPath::Sse2, self.precision)
            .map_err(super::err)?;
        if let aa_runtime::simulation::SyncLog::GameTimePerTick(v) = change.sync_log {
            st.sync_a.log_float_bits(true, "m_game_time_per_tick", v.0);
        }
        Ok(())
    }

    /// PlayerJoin process 0x75DA80 -> 0x758FD0 (and the host's completion).
    fn player_join(&self, st: &mut State, fields: &[Field], ordering: Option<u32>) -> Result<()> {
        let info = lobby::JoinInfo::from_fields(fields)?;
        // Decree stream position after this decree (AdminNode+0x200/+0x204).
        let mut w = aa_orders::commands::WireWriter::new(Position::PACKET_START)?;
        payload::encode(d::PLAYER_JOIN_DECREE, fields, &mut w, &crate::session::LobbyContext)?;
        let payload_bits = w.position().absolute()? - 3;
        let resume_bits = 3 + st.stream_bits as usize + 9 + payload_bits;
        let snapshot = st.sync_a.clone();
        let previous = ordering;
        let view = st.view();
        let mut node = lobby::JoinNode {
            view,
            own_address: st.own_address,
            now: st.now.to_bits(),
            internet: false,
            peers_changed: false,
            multiplayer_assert_ignored: false,
        };
        // 0x764970: the joining address has a connection on every node.
        if let Some(a) = info.address
            && st.role == Role::Client
            && Some(a) != st.own_address
        {
            st.ensure_conn(a)?;
        }
        let r = {
            let (local, now) = (st.local_player, st.now.to_bits());
            let State { session, conns, .. } = st;
            let mut joins = Joins { conns, data: self.data, view, local, now };
            lobby::process_player_join(&info, session, &mut node, &DataView { data: self.data }, &mut joins)?
        };
        for a in r.actions {
            match a {
                lobby::LobbyAction::AddPeer { link } => {
                    if !st.peers.contains(&link) {
                        st.peers.push(link);
                    }
                }
                lobby::LobbyAction::ConnectToPeer { address } => {
                    st.ensure_conn(address)?;
                }
                lobby::LobbyAction::UiMessage { kind, key, args } => st.events.push(LanEvent::Message { kind, key, args, number: None }),
                lobby::LobbyAction::Request { descriptor, fields } => st.deferred.requests.push((descriptor, fields)),
                other => return Err(Error::Protocol(format!("0x758FD0 left {other:?}"))),
            }
        }
        if let lobby::JoinOutcome::Joined { link, host_completion: true, .. } = r.outcome {
            st.pending_joins.push(PendingJoin { link, resume_bits, sync_a: snapshot, player: previous });
        }
        st.events.push(LanEvent::PlayersChanged);
        Ok(())
    }

    /// 0x752FC0 UpdateTaskStatus and its completion.
    fn task_status(&self, st: &mut State, fields: &[Field], ordering: Option<u32>) -> Result<()> {
        let status = uint(fields, 0)?;
        let Some(link) = ordering.and_then(|p| st.player_link(p)) else { return Ok(()) };
        st.set_link_task(&link, status);
        self.check_tasks(st)
    }

    /// 0x74DA50 (every peer of +0x2AC done, success when none failed), the
    /// task history 0x750940 / 0x750990 (not kept here), 0x776560 and
    /// ClearAllTasks 0x74DAA0.
    fn check_tasks(&self, st: &mut State) -> Result<()> {
        let mut success = true;
        for l in &st.peers {
            let t = st.link_task(l);
            success = success && t != 4;
            if matches!(t, 0..=2) {
                return Ok(());
            }
        }
        if success {
            // 0x775BF0: event 9, SetFrozen(0) in game, then for a launch that
            // created its task (+0x16A) the world dump 0x4F8AE0 (aa-play
            // `World` keeps its one draw; its A entries are not logged) and
            // the S restart 0x77D900.
            st.events.push(LanEvent::BeginPlay);
            if matches!(st.session.state, 2 | 4 | 5) {
                self.set_frozen(st, false)?;
            }
            if std::mem::take(&mut st.session.flag_16a) {
                st.sync_s.reset(0, 0);
            }
        } else {
            st.events.push(LanEvent::TasksFailed);
        }
        for l in st.peers.clone() {
            st.set_link_task(&l, 0);
        }
        Ok(())
    }

    /// PlayerLeave 0x755D70 -> ClientLeave 0x755840(client, reason, 1, 1) in
    /// its native order, on what a LAN node keeps: the message, the countdown
    /// abort 0x76CFA0, the unlink, the players destroyed, then SetDisconnecting
    /// on the host (reason != 5) or the connection's end (0x752110 reduced to
    /// `connected` and the player links) and the task check 0x751F70.
    ///
    /// Not here, because nothing a LAN node runs writes them: the pause owner
    /// +0x2A0 and the vote-to-kick lists. Refused: a leave of the node itself
    /// (the `PeerRef::Node` branch of heartbeat::client_leave, a kick seen by
    /// its target) and 0x76EE10 MidgameCreateImplicitAIs, which an in-game
    /// leave needs and aa-play does not have.
    fn player_leave(&self, st: &mut State, player: u32, reason: u32, emission: Option<&mut RequestEmission<'_>>) -> Result<()> {
        // 0x755D70: invalid player, or not the human of its client: logs.
        let Some(link) = st.player_link(player).filter(|_| player != 0) else { return Ok(()) };
        let address = match link {
            PeerLink::Conn(a) => a,
            PeerLink::Node => return Err(Error::Unsupported("0x755840 ClientLeave of the node itself")),
            PeerLink::Null => return Ok(()),
        };
        let Some(index) = st.conn_index(address) else { return Ok(()) };
        if st.conns[index].admin.base.player != Some(player) {
            return Ok(());
        }
        // 0x76EE10 MidgameCreateImplicitAIs: in the game, a kingdom left without a
        // session player gets an implicit computer player on every node; the
        // world's side is the game's. (The player's record goes below.)
        let in_game = st.session.state == 2;
        // 0x7667D0 display name of the leaving player.
        let key = match reason {
            4 => crate::heartbeat::LEAVE_KICKED_KEY,
            0xB => crate::heartbeat::LEAVE_INVALID_CDKEY_KEY,
            _ => crate::heartbeat::LEAVE_QUIT_KEY,
        };
        let name = st.session.players.iter().find(|p| p.id == player).map(|p| p.name.clone()).unwrap_or_default();
        st.events.push(LanEvent::Message { kind: 0xF, key: key.into(), args: vec![name], number: None });
        let view = st.view();
        let node = staging_node(st, &view, None);
        let mut fx = StagingEffects::new(st, self.data);
        staging::abort_countdown(&mut st.session, &node, &mut fx)?;
        fx.apply(st, emission, self.data)?;
        st.peers.retain(|l| *l != link);
        lobby::destroy_link_players(&mut st.session, &link, |_, _| Ok(()))?;
        let (we_are_host, now) = (st.we_are_host(), st.now.to_bits());
        let c = &mut st.conns[index].admin;
        c.base.player = None;
        c.base.ordering_player = None;
        c.connected = false;
        if we_are_host && reason != 5 {
            c.disconnecting = true;
            c.disconnect_started = now;
            c.disconnect_reason = reason;
        }
        st.events.push(LanEvent::PlayersChanged);
        if in_game {
            self.game.try_borrow_mut().map_err(super::err)?.implicit_ai(player)?;
            st.session.changed = true;
        }
        self.check_tasks(st)
    }
}

/// Thing resolution only (gameplay decree fields carry no other reference
/// that the reader resolves against node state).
struct ThingOnly<'a> {
    game: &'a dyn LanGame,
}
impl Resolver for ThingOnly<'_> {
    fn thing(&self, check: ThingCheck, slot: u16) -> Option<u16> {
        self.game.thing(check, slot)
    }
    fn player(&self, _id: u32) -> bool {
        true
    }
    fn definition(&self, _registry: Registry, _index: u32) -> bool {
        true
    }
    fn named(&self, _kind: NameKind, key: &[u16]) -> Option<Vec<u16>> {
        Some(key.to_vec())
    }
}

/// JoinHost over the node's connections (0x764970 created them).
struct Joins<'a> {
    conns: &'a mut Vec<super::Conn>,
    data: &'a dyn LanData,
    view: lobby::NodeView,
    local: Option<u32>,
    now: u32,
}
impl lobby::JoinHost for Joins<'_> {
    fn conn(&mut self, link: &PeerLink) -> Result<&mut crate::packets::AdminConn> {
        let PeerLink::Conn(a) = link else { return Err(Error::Protocol(format!("join target {link:?}"))) };
        self.conns.iter_mut().find(|c| c.addr == *a).map(|c| &mut c.admin).ok_or_else(|| Error::Protocol(format!("no connection {a:?}")))
    }
    fn auto_assign_kingdom(&mut self, session: &mut Session, player: u32) -> Result<()> {
        staging::auto_assign_kingdom(session, player, &mut KingdomAssign { data: self.data, view: self.view, local: self.local, now: self.now })?;
        Ok(())
    }
    fn duplicate_name(&mut self, name: &[u16], n: u32) -> Result<Vec<u16>> {
        Ok(self.data.format(lobby::DUPLICATE_NAME_KEY, &[name.to_vec(), units(&n.to_string())]))
    }
    fn kingdom_open(&mut self, _kingdom: &[u16]) -> Result<bool> {
        // 0x77E430 on the constructor-empty session info: null, so open.
        Ok(true)
    }
}

/// A copy of the node's AdminPeer views (the snapshot writer reads them
/// while the joining connection is borrowed).
pub(crate) struct SnapshotDirectory {
    node: PeerView,
    conns: Vec<(PeerAddress, lobby::PeerIdentity)>,
}
impl SnapshotDirectory {
    pub fn of(st: &State) -> Self {
        Self {
            node: PeerView { address: st.own_address, identity: st.own_identity.clone() },
            conns: st.conns.iter().map(|c| (c.addr, c.admin.base.identity())).collect(),
        }
    }
}
impl lobby::PeerDirectory for SnapshotDirectory {
    fn peer(&self, link: &PeerLink) -> Result<PeerView> {
        match link {
            PeerLink::Node => Ok(self.node.clone()),
            PeerLink::Conn(a) => {
                let (_, identity) = self.conns.iter().find(|(b, _)| b == a).ok_or_else(|| Error::Protocol(format!("no peer {a:?}")))?;
                Ok(PeerView { address: Some(*a), identity: identity.clone() })
            }
            PeerLink::Null => Err(Error::Protocol("null peer".into())),
        }
    }
}

// ---------------------------------------------------------------------------
// The admin-tick steps
// ---------------------------------------------------------------------------

/// 0x752E60 host execution of every peer's queued requests (the node first,
/// then +0x2AC in list order), the host join completions 0x753580 and the
/// SyncFailure decrees 0x750BA0 owed by this tick.
pub(crate) fn host_execute(s: &mut Seams<'_, '_, '_>) -> Result<()> {
    let own = s.st.own_ordering;
    let mut taken: Vec<(usize, Peer)> = Vec::new();
    for link in s.st.peers.clone() {
        let PeerLink::Conn(a) = link else { continue };
        let Some(i) = s.st.conn_index(a) else { continue };
        let c = &mut s.st.conns[i];
        if c.admin.base.ordering_player.is_none() {
            continue;
        }
        let requests = std::mem::replace(&mut c.admin.base.requests, History::new(0, 1)?);
        taken.push((i, Peer { requests, ordering_player: c.admin.base.ordering_player }));
    }
    let result = {
        let st_cell = RefCell::new(&mut *s.st);
        let mut identified: Vec<IdentifiedPeer<'_>> =
            taken.iter_mut().map(|(i, p)| IdentifiedPeer { peer: p, client: ClientIdentity::Remote(*i as u64) }).collect();
        let mut host = Orders { st: &st_cell, game: s.game, data: s.data, timeline: s.timeline, admin_time: s.admin_time, gates: *s.gates, precision: s.precision };
        let resolver = Resolve { st: &st_cell, game: s.game, data: s.data };
        let ctx = DataView { data: s.data };
        s.admin.host_execute_owned_requests(own, &mut identified, &mut host, &resolver, &ctx, s.gate)
    };
    for (i, p) in taken {
        s.st.conns[i].admin.base.requests = p.requests;
    }
    result?;
    // Host join completion 0x753580 (inside the PlayerJoin process natively;
    // no datagram is built in between here).
    for j in std::mem::take(&mut s.st.pending_joins) {
        let PeerLink::Conn(a) = j.link else { continue };
        let Some(ci) = s.st.conn_index(a) else { continue };
        let source = lobby::AdminStateSource {
            conn_address: Some(a),
            field_200: (j.resume_bits / 8) as u32,
            field_204: (j.resume_bits % 8) as u32,
            player: j.player,
            admin_time: s.admin_time.0,
            decree_end: s.admin.decrees.end(),
        };
        let dir = SnapshotDirectory::of(s.st);
        let st = &mut *s.st;
        let admin = &mut *s.admin;
        let mut view = crate::packets::NodeHistories {
            requests: &mut admin.requests,
            backup_requests: &mut st.spare.backup_requests,
            decrees: &mut admin.decrees,
            backup_decrees: &mut st.spare.backup_decrees,
            backup_decree_next: &mut st.spare.backup_next,
        };
        lobby::complete_host_join(&mut st.conns[ci].admin, &mut view, &st.session, &dir, &DataView { data: s.data }, &source, &j.sync_a, false)?;
    }
    // 0x750BA0: the SyncFailure decree straight into the decree stream.
    for player in std::mem::take(&mut s.st.deferred.sync_failures) {
        write_sync_failure(s, player)?;
    }
    Ok(())
}

/// 0x750BA0 TGC_AdminNode emits TGC_SyncFailureOrderDecree(player): A
/// "o.GetBitSize()", the 9-bit id and the 32-bit player, A name and id, the
/// process 0x75D9A0, A "o.GetBitSize()". Natively this runs inside the process
/// of the 314 decree that found the mismatch; here after the tick's requests
/// (the decree stream is lent out while they run), so the 303 can follow
/// decrees it natively precedes. Both nodes of this port see the same order.
fn write_sync_failure(s: &mut Seams<'_, '_, '_>, player: u32) -> Result<()> {
    let bits = |w: &aa_orders::commands::WireWriter| -> Result<u32> { Ok((w.position().absolute()? - 3) as u32) };
    let before = bits(&s.admin.decree_stream)?;
    s.st.sync_a.log_int(true, "o.GetBitSize()", before);
    s.admin.decree_stream.write_uint(d::SYNC_FAILURE_DECREE as u32, aa_orders::flow::DESCRIPTOR_BITS)?;
    let fields = [Field::Uint { width: 32, value: player }];
    payload::encode(d::SYNC_FAILURE_DECREE, &fields, &mut s.admin.decree_stream, &DataView { data: s.data })?;
    s.st.sync_a.log_text(true, "m_decree_type.m_name", &units("TGC_SyncFailureOrderDecree"));
    s.st.sync_a.log_int(true, "m_decree_type.m_id", d::SYNC_FAILURE_DECREE as u32);
    s.st.events.push(LanEvent::SyncFailure { player });
    let after = bits(&s.admin.decree_stream)?;
    s.st.sync_a.log_int(true, "o.GetBitSize()", after);
    Ok(())
}

/// 0x752EA0 with the live Admin+284.
pub(crate) fn host_record(s: &mut Seams<'_, '_, '_>) -> Result<()> {
    let st_cell = RefCell::new(&mut *s.st);
    let mut host = Orders { st: &st_cell, game: s.game, data: s.data, timeline: s.timeline, admin_time: s.admin_time, gates: *s.gates, precision: s.precision };
    s.admin.seal_decrees_with_ordering(&mut host)?;
    Ok(())
}

/// 0x74FD10 client replay of decree packet `index`.
pub(crate) fn client_replay(s: &mut Seams<'_, '_, '_>, index: i32) -> Result<()> {
    let st_cell = RefCell::new(&mut *s.st);
    let mut host = Orders { st: &st_cell, game: s.game, data: s.data, timeline: s.timeline, admin_time: s.admin_time, gates: *s.gates, precision: s.precision };
    let resolver = Resolve { st: &st_cell, game: s.game, data: s.data };
    let ctx = DataView { data: s.data };
    s.admin.client_execute_with_ordering(index, &mut host, &resolver, &ctx, s.gate)?;
    Ok(())
}
