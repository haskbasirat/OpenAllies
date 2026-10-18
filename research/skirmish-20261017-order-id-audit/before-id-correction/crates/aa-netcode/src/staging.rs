//! KKC_Session staging and launch logic, ported from the original AA.exe:
//! player readiness, the launch countdown order processes, KKC_Session::
//! Launch, ChangeToState and AutoAssignKingdomToPlayer. Oracle:
//! research/phase8-net/native_launch.py -> native-launch.json, replayed by
//! tests/staging.rs.
//!
//! Ported functions (native order kept):
//! * 0x765FC0 [`player_ready`], 0x76C960 [`all_ready`];
//! * TGC_StartLaunchCountdownOrder process 0x75DB90 -> 0x774A50
//!   [`start_countdown`] (ordering-client test 0x74B3C0 + 0x74B900, migrated
//!   host 0x74BB40, rejection 0x76DA60, request 0x76CBF0);
//! * TGC_StopLaunchCountdownOrder process 0x75DBA0 -> 0x772350
//!   [`stop_countdown`] -> 0x76CFA0 [`abort_countdown`] (also called by
//!   ClientLeave 0x755840);
//! * TGC_LaunchCountdownProgressOrder process 0x75DBB0 -> 0x7723F0
//!   [`countdown_progress`];
//! * TGC_LaunchOrder process 0x75D8D0 -> 0x77A950 [`launch`];
//! * 0x77A750 [`change_to_state`] with 0x77A6E0;
//! * 0x778D10 [`auto_assign_kingdom`] with SessionPlayer::SetKingdom 0x7664D0.
//!
//! [`StagingHost`] carries, in native order, what is not network/session
//! logic: UI messages (0x40BC50 with 0x4B9F40 keys), the not-ready report
//! 0x7725A0 (UI), AVar LaunchCountdownTime (0x41D430), the session timer
//! 0x59AB60, the world loading steps of Launch (0x775DB0, 0x770500,
//! 0x776B20, 0x775EE0), the launch-settings sync dump 0x76D6C0, the timeline
//! 0x76DD90, ResetOrderVersion 0x74DBC0, the session-state event (0x7844F0
//! .. 0x7851C0), KKC_LaunchSettings::operator= 0x779F00 (not ported: the
//! copy of every map/game sub-object) and the kingdom setters 0x777F00 /
//! 0x777CD0 / 0x777610 (scenario world data). Order requests are returned to
//! the caller as [`StagingHost::request`] calls; the caller emits them with
//! aa_orders::net::admin::AdminNode::emit_request (gate 0x766CD0, player 0
//! = the local player 0x76AC60).
use aa_orders::payload::Field;

use crate::lobby::{LaunchState, NodeView, PeerDirectory, PeerLink, Session};
use crate::session::wcs_eq;
use crate::{Error, Result};

pub mod launch_owner;

/// TGC_LaunchCountdownProgressOrderRequest (descriptor object 0xB40B2C).
pub const LAUNCH_COUNTDOWN_PROGRESS_REQUEST: usize = 325;
/// 0x76DA60: "session_order_rejected" (0xA94190), message kind 0x11.
pub const ORDER_REJECTED_KEY: &str = "session_order_rejected";
pub const ORDER_REJECTED_KIND: u32 = 0x11;
/// 0x774A50: "staging_not_original_host" (0xA94E98), kind 0x10.
pub const NOT_ORIGINAL_HOST_KEY: &str = "staging_not_original_host";
pub const NOT_ORIGINAL_HOST_KIND: u32 = 0x10;
/// Countdown messages, kind 0xF: begin (0xA950A8), abort (0xA945D8),
/// number (0xA945A4, formatted with the countdown).
pub const COUNTDOWN_BEGIN_KEY: &str = "launch_countdown_begin";
pub const COUNTDOWN_ABORT_KEY: &str = "launch_countdown_abort";
pub const COUNTDOWN_NUMBER_KEY: &str = "launch_countdown_number|d";
pub const COUNTDOWN_KIND: u32 = 0xF;
/// 0x77A950: label of the launch-settings sync dump 0x76D6C0 (0xA96AA4).
pub const LAUNCH_SETTINGS_DUMP_LABEL: &str = "m_launch_settings";

/// Node facts the processes read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StagingNode {
    /// AdminNode+0x2C4 (0x74B900 compares against it).
    pub view: NodeView,
    /// AdminNode+0x2E0, read through 0x74BB40 (only on the host).
    pub took_over: i32,
    /// AdminNode+0x284 ordering player and AdminNode+8 local player
    /// (SessionPlayer ids).
    pub ordering_player: Option<u32>,
    pub local_player: Option<u32>,
    /// [0xB3654C]+0xBC internet game.
    pub internet: bool,
}
impl StagingNode {
    /// 0x74BB40 on the node: AdminNode+0x2E0 when the node is the host, else 0.
    fn migrated(&self) -> i32 {
        if self.view.we_are_host() {
            self.took_over
        } else {
            0
        }
    }
}

/// World loading steps of KKC_Session::Launch 0x77A950 (session methods).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchStep {
    /// 0x775DB0(1).
    Step775DB0,
    /// 0x770500.
    Step770500,
    /// 0x776B20.
    Step776B20,
}

/// Launch-settings objects 0x779F00 copies between.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSlot {
    /// Session +0x54.
    Launch,
    /// Session +0x88 (the last launch settings).
    Launch88,
    /// [[0xB39D04]+0x104] / [[0xB39D04]+0x108]: the stored single-player /
    /// multiplayer launch settings.
    StoreSingle,
    StoreMulti,
}

/// The 0x3C-byte event ChangeToState posts through 0x7851C0 on [0xB40BA8]
/// (+0 = 0, +4 text, +8 state, +0xC launch state copy, +0x34 flag, +0x38 = 0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateEvent {
    pub text: Vec<u16>,
    pub state: u32,
    pub launch_state: LaunchState,
    pub flag: bool,
}

pub trait StagingHost {
    /// 0x40BC50(&0xB399B0, kind, 0x4B9F40(key)[, number]).
    fn ui_message(&mut self, kind: u32, key: &'static str, number: Option<i32>) -> Result<()>;
    /// Order request through the 0x76CBF0 template (gate 0x766CD0, player 0
    /// = the local player 0x76AC60).
    fn request(&mut self, descriptor: usize, fields: Vec<Field>) -> Result<()>;
    /// AVar LaunchCountdownTime (0x41D430 on 0xB39988, key 0x9F0B94).
    fn countdown_time(&mut self) -> Result<i32>;
    /// 0x59AB60 on session+0x110.
    fn start_timer(&mut self, session: &mut Session) -> Result<()>;
    /// 0x7725A0: reports the players that are not ready (UI).
    fn not_ready(&mut self, session: &mut Session) -> Result<()>;
    fn launch_step(&mut self, session: &mut Session, step: LaunchStep) -> Result<()>;
    /// 0x775EE0: its low byte is the "loaded" flag Launch uses.
    fn load(&mut self, session: &mut Session) -> Result<bool>;
    /// 0x76D6C0(label, &session+0x54): launch-settings sync dump.
    fn dump_launch_settings(&mut self, session: &mut Session, label: &'static str) -> Result<()>;
    /// 0x76DD90(flag) on [0xB36554].
    fn timeline_freeze(&mut self, flag: bool) -> Result<()>;
    /// 0x74DBC0(0) on [0xB36558].
    fn reset_order_version(&mut self) -> Result<()>;
    /// 0x7844F0 .. 0x7851C0 on [0xB40BA8].
    fn state_changed(&mut self, session: &mut Session, event: StateEvent) -> Result<()>;
    /// KKC_LaunchSettings::operator= 0x779F00 (dst = src).
    fn copy_launch_settings(&mut self, session: &mut Session, dst: SettingsSlot, src: SettingsSlot) -> Result<()>;
}

fn player_index(session: &Session, id: u32) -> Result<usize> {
    session
        .players
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| Error::Protocol(format!("SessionPlayer {id} is not in the session")))
}

/// 0x765FC0 for the player at `index`.
pub fn player_ready(session: &Session, index: usize, node: &StagingNode, peers: &dyn PeerDirectory) -> Result<bool> {
    let p = session.players.get(index).ok_or_else(|| Error::Protocol(format!("no player {index}")))?;
    let mut ready = p.status == 1 || p.status == 2;
    if node.internet && session.multiplayer && !p.ai {
        ready = ready && p.flag_2d;
    }
    if node.view.is_host(&p.link) {
        return Ok(true);
    }
    if p.link == PeerLink::Null {
        return Err(Error::Protocol("0x76601F reads +0x3C of a null player connection".into()));
    }
    if peers.peer(&p.link)?.identity.field_3c != 1 {
        let mode = session.launch.mode;
        if mode == 2 || mode == 0 {
            return Ok(false);
        }
    }
    // [player+0x24] CString length 0: no kingdom chosen.
    Ok(if p.kingdom.is_empty() { true } else { ready })
}

/// 0x76C960: every player is the host's or ready.
pub fn all_ready(session: &Session, node: &StagingNode, peers: &dyn PeerDirectory) -> Result<bool> {
    for (i, p) in session.players.iter().enumerate() {
        if !node.view.is_host(&p.link) && !player_ready(session, i, node, peers)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// 0x74B3C0 + 0x74B900: the ordering player's connection is the host.
fn ordering_client_is_host(session: &Session, node: &StagingNode) -> Result<bool> {
    let Some(id) = node.ordering_player else {
        // 0x74B3C0 logs "no current player" and returns null.
        return Ok(false);
    };
    let link = session.players[player_index(session, id)?].link;
    Ok(node.view.is_host(&link))
}

/// 0x76DA60 with "session_order_rejected": the message only on the node
/// whose local player is the ordering player (0x74B3B0; null == null).
fn reject(node: &StagingNode, host: &mut dyn StagingHost) -> Result<()> {
    if node.local_player == node.ordering_player {
        host.ui_message(ORDER_REJECTED_KIND, ORDER_REJECTED_KEY, None)?;
    }
    Ok(())
}

/// 0x76CBF0(value, player 0).
fn countdown_request(value: i32, host: &mut dyn StagingHost) -> Result<()> {
    host.request(LAUNCH_COUNTDOWN_PROGRESS_REQUEST, vec![Field::Uint { width: 32, value: value as u32 }])
}

/// 0x774A50 KKC_Session::StartLaunchCountdown.
pub fn start_countdown(
    session: &mut Session,
    node: &StagingNode,
    peers: &dyn PeerDirectory,
    host: &mut dyn StagingHost,
) -> Result<()> {
    if !ordering_client_is_host(session, node)? {
        return reject(node, host);
    }
    if node.migrated() == 1 {
        return host.ui_message(NOT_ORIGINAL_HOST_KIND, NOT_ORIGINAL_HOST_KEY, None);
    }
    if session.host_countdown > 0 {
        return Ok(());
    }
    if !all_ready(session, node, peers)? {
        return host.not_ready(session);
    }
    if node.view.we_are_host() {
        session.host_countdown = host.countdown_time()?;
        countdown_request(session.host_countdown, host)?;
        host.start_timer(session)?;
    }
    host.ui_message(COUNTDOWN_KIND, COUNTDOWN_BEGIN_KEY, None)
}

/// 0x772350 KKC_Session::StopLaunchCountdown (process of the stop order).
pub fn stop_countdown(session: &mut Session, node: &StagingNode, host: &mut dyn StagingHost) -> Result<()> {
    if !ordering_client_is_host(session, node)? {
        return reject(node, host);
    }
    abort_countdown(session, node, host)
}

/// 0x76CFA0: a running host countdown (+0x11C > 0) is stopped (-1 on the
/// host) and every node sends LaunchCountdownProgress(-1).
pub fn abort_countdown(session: &mut Session, node: &StagingNode, host: &mut dyn StagingHost) -> Result<()> {
    if session.host_countdown > 0 {
        if node.view.we_are_host() {
            session.host_countdown = -1;
        }
        countdown_request(-1, host)?;
    }
    Ok(())
}

/// 0x7723F0 KKC_Session::LaunchCountdownProgress(value).
pub fn countdown_progress(
    session: &mut Session,
    value: i32,
    node: &StagingNode,
    host: &mut dyn StagingHost,
) -> Result<()> {
    if !ordering_client_is_host(session, node)? {
        return reject(node, host);
    }
    if session.launch_countdown == value {
        return Ok(());
    }
    session.launch_countdown = value;
    if value < 1 {
        host.ui_message(COUNTDOWN_KIND, COUNTDOWN_ABORT_KEY, None)
    } else {
        host.ui_message(COUNTDOWN_KIND, COUNTDOWN_NUMBER_KEY, Some(value))
    }
}

fn in_game(state: u32) -> bool {
    matches!(state, 2 | 4 | 5)
}

/// 0x77A950 KKC_Session::Launch.
pub fn launch(session: &mut Session, node: &StagingNode, host: &mut dyn StagingHost) -> Result<()> {
    launch_owner::launch(&mut launch_owner::Laboratory {session,node,host})
}

/// 0x77A750 KKC_Session::ChangeToState(state, flag).
pub fn change_to_state(
    session: &mut Session,
    state: u32,
    flag: bool,
    node: &StagingNode,
    host: &mut dyn StagingHost,
) -> Result<()> {
    launch_owner::change_to_state(&mut launch_owner::Laboratory {session,node,host},state,flag)
}

/// 0x77A6E0: the original host stores the launch settings as the last used
/// ones (multiplayer or single-player slot) when +0x169 is set; then
/// +0x88 = +0x54.
fn leave_state(session: &mut Session, node: &StagingNode, host: &mut dyn StagingHost) -> Result<()> {
    if node.view.we_are_host() && node.migrated() == 0 && session.flag_169 {
        let store = if session.multiplayer { SettingsSlot::StoreMulti } else { SettingsSlot::StoreSingle };
        host.copy_launch_settings(session, store, SettingsSlot::Launch)?;
    }
    host.copy_launch_settings(session, SettingsSlot::Launch88, SettingsSlot::Launch)
}

/// Kingdom data the assignment consults or changes.
pub trait KingdomHost {
    /// 0x77E430 on [session+0x14] for the kingdom record at `record` of the
    /// active map: `None` for a null result, else its +5 byte.
    fn kingdom_record_info(&mut self, session: &Session, record: usize) -> Result<Option<bool>>;
    /// 0x777F00(player, record name, 0), 0x777CD0(record, &player+0x30),
    /// 0x777610(record, &player+0x34) (scenario world data; not ported).
    fn assign_kingdom(&mut self, session: &mut Session, player: u32, record: usize) -> Result<()>;
}

/// 0x778D10 KKC_Session::AutoAssignKingdomToPlayer(player). The original
/// reads the global session [0xB36550] for the editor/film/save test; it is
/// this session.
pub fn auto_assign_kingdom(session: &mut Session, player: u32, host: &mut dyn KingdomHost) -> Result<bool> {
    if !matches!(session.launch.mode, 0 | 1 | 2 | 5) {
        return Ok(false);
    }
    if matches!(session.launch.game_type, 3 | 4) || session.launch.field_0c == 5 {
        // SessionPlayer::SetKingdom 0x7664D0 with the empty string 0xB43B40.
        let p = session
            .player_mut(player)
            .ok_or_else(|| Error::Protocol(format!("SessionPlayer {player} is not in the session")))?;
        p.kingdom.clear();
        p.flag_2c = true;
        return Ok(true);
    }
    let count = session.launch.active_map().header.kingdoms.len();
    for i in 0..count {
        let k = &session.launch.active_map().header.kingdoms[i];
        if k.flag_10 || !k.flag_28 {
            continue;
        }
        if host.kingdom_record_info(session, i)? == Some(false) {
            continue;
        }
        let name = &session.launch.active_map().header.kingdoms[i].name;
        if session.players.iter().any(|p| wcs_eq(&p.kingdom, name)) {
            continue;
        }
        host.assign_kingdom(session, player, i)?;
        return Ok(true);
    }
    Ok(false)
}
