pub use aa_netcode::{lobby,session,Error,Result};
pub mod staging{
// KKC_Session staging and launch logic, ported from the original AA.exe:
// player readiness, the launch countdown order processes, KKC_Session::
// Launch, ChangeToState and AutoAssignKingdomToPlayer. Oracle:
// research/phase8-net/native_launch.py ->
// research/skirmish-20261017-order-id-audit/native-launch-registry358.json, replayed by
// tests/staging.rs.
//
// Ported functions (native order kept):
// * 0x765FC0 [`player_ready`], 0x76C960 [`all_ready`];
// * TGC_StartLaunchCountdownOrder process 0x75DB90 -> 0x774A50
//   [`start_countdown`] (ordering-client test 0x74B3C0 + 0x74B900, migrated
//   host 0x74BB40, rejection 0x76DA60, request 0x76CBF0);
// * TGC_StopLaunchCountdownOrder process 0x75DBA0 -> 0x772350
//   [`stop_countdown`] -> 0x76CFA0 [`abort_countdown`] (also called by
//   ClientLeave 0x755840);
// * TGC_LaunchCountdownProgressOrder process 0x75DBB0 -> 0x7723F0
//   [`countdown_progress`];
// * TGC_LaunchOrder process 0x75D8D0 -> 0x77A950 [`launch`];
// * 0x77A750 [`change_to_state`] with 0x77A6E0;
// * 0x778D10 [`auto_assign_kingdom`] with SessionPlayer::SetKingdom 0x7664D0.
//
// [`StagingHost`] carries, in native order, what is not network/session
// logic: UI messages (0x40BC50 with 0x4B9F40 keys), the not-ready report
// 0x7725A0 (UI), AVar LaunchCountdownTime (0x41D430), the session timer
// 0x59AB60, the world loading steps of Launch (0x775DB0, 0x770500,
// 0x776B20, 0x775EE0), the launch-settings sync dump 0x76D6C0, the timeline
// 0x76DD90, ResetOrderVersion 0x74DBC0, the session-state event (0x7844F0
// .. 0x7851C0), KKC_LaunchSettings::operator= 0x779F00 (not ported: the
// copy of every map/game sub-object) and the kingdom setters 0x777F00 /
// 0x777CD0 / 0x777610 (scenario world data). Order requests are returned to
// the caller as [`StagingHost::request`] calls; the caller emits them with
// aa_orders::net::admin::AdminNode::emit_request (gate 0x766CD0, player 0
// = the local player 0x76AC60).
use aa_orders::payload::Field;

use crate::lobby::{LaunchState, NodeView, PeerDirectory, PeerLink, Session};
use crate::session::wcs_eq;
use crate::{Error, Result};

#[path="E:/Games/Axis and Alies/rust-rebuild/crates/aa-netcode/src/staging/launch_owner.rs"] pub mod launch_owner;

/// TGC_LaunchCountdownProgressOrderRequest (descriptor object 0xB40B2C).
pub const LAUNCH_COUNTDOWN_PROGRESS_REQUEST: usize = 326;
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

}
mod tests{
// Replays research/skirmish-20261017-order-id-audit/native-launch-registry358.json: the original launch
// countdown processes (0x774A50, 0x772350 -> 0x76CFA0, 0x7723F0), the
// readiness tests (0x765FC0, 0x76C960), KKC_Session::Launch 0x77A950,
// ChangeToState 0x77A750 and AutoAssignKingdomToPlayer 0x778D10 on native
// KKC_Session objects. Every seam event (UI messages, request emission,
// AVar, timer, world steps, state event, settings copies, kingdom setters)
// is compared in native order, with the request bytes and the session
// fields afterwards.
use aa_netcode::lobby::{NodeView, PeerDirectory, PeerIdentity, PeerLink, PeerView, Session};
use aa_netcode::session::PeerAddress;
use crate::staging::{
    all_ready, auto_assign_kingdom, change_to_state, countdown_progress, launch, player_ready, start_countdown,
    stop_countdown, KingdomHost, LaunchStep, SettingsSlot, StagingHost, StagingNode, StateEvent,
    LAUNCH_COUNTDOWN_PROGRESS_REQUEST,
};
use aa_orders::commands::WireWriter;
use aa_orders::payload::Field;
use aa_orders::Position;
use serde_json::{json, Value};

fn de<T: serde::de::DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

struct Peers {
    node: PeerIdentity,
    conns: Vec<(PeerAddress, PeerIdentity)>,
}
impl PeerDirectory for Peers {
    fn peer(&self, link: &PeerLink) -> aa_netcode::Result<PeerView> {
        match link {
            PeerLink::Node => Ok(PeerView { address: None, identity: self.node.clone() }),
            PeerLink::Conn(a) => {
                let identity = self.conns.iter().find(|(x, _)| x == a).map(|(_, i)| i.clone()).unwrap_or_default();
                Ok(PeerView { address: Some(*a), identity })
            }
            PeerLink::Null => panic!("null peer"),
        }
    }
}

struct Host<'a> {
    launch: &'a Value,
    events: Vec<Value>,
    request: Option<(Vec<u8>, Position)>,
}
fn slot(s: SettingsSlot) -> &'static str {
    match s {
        SettingsSlot::Launch => "launch",
        SettingsSlot::Launch88 => "launch_88",
        SettingsSlot::StoreSingle => "store_single",
        SettingsSlot::StoreMulti => "store_multi",
    }
}
impl StagingHost for Host<'_> {
    fn ui_message(&mut self, kind: u32, key: &'static str, number: Option<i32>) -> aa_netcode::Result<()> {
        let args: Vec<u32> = number.map(|n| n as u32).into_iter().collect();
        self.events.push(json!({"event": "ui_message", "kind": kind, "text": units(key), "args": args}));
        Ok(())
    }
    fn request(&mut self, descriptor: usize, fields: Vec<Field>) -> aa_netcode::Result<()> {
        assert_eq!(descriptor, LAUNCH_COUNTDOWN_PROGRESS_REQUEST);
        self.events.push(json!({"event": "request_gate"}));
        self.events.push(json!({"event": "request_player", "player": 0}));
        let [Field::Uint { width: 32, value }] = fields[..] else { panic!("{fields:?}") };
        let mut w = WireWriter::new(Position::PACKET_START).unwrap();
        w.write_uint(descriptor as u32, aa_orders::registry_bits(aa_orders::native::DESCRIPTORS.len()).unwrap()).unwrap();
        w.write_uint(value, 32).unwrap();
        let end = w.position();
        let bits = end.absolute().unwrap();
        assert!(self.request.is_none());
        self.request = Some((w.bytes()[..bits.div_ceil(8)].to_vec(), end));
        Ok(())
    }
    fn countdown_time(&mut self) -> aa_netcode::Result<i32> {
        self.events.push(json!({"event": "avar", "name": "LaunchCountdownTime"}));
        Ok(self.launch["countdown_time"].as_i64().unwrap() as i32)
    }
    fn start_timer(&mut self, _s: &mut Session) -> aa_netcode::Result<()> {
        self.events.push(json!({"event": "start_timer", "this_is_110": true}));
        Ok(())
    }
    fn not_ready(&mut self, _s: &mut Session) -> aa_netcode::Result<()> {
        self.events.push(json!({"event": "not_ready"}));
        Ok(())
    }
    fn launch_step(&mut self, _s: &mut Session, step: LaunchStep) -> aa_netcode::Result<()> {
        self.events.push(match step {
            LaunchStep::Step775DB0 => json!({"event": "launch_step", "fn": "0x775db0", "arg": 1}),
            LaunchStep::Step770500 => json!({"event": "launch_step", "fn": "0x770500"}),
            LaunchStep::Step776B20 => json!({"event": "launch_step", "fn": "0x776b20"}),
        });
        Ok(())
    }
    fn load(&mut self, _s: &mut Session) -> aa_netcode::Result<bool> {
        self.events.push(json!({"event": "launch_step", "fn": "0x775ee0"}));
        Ok(self.launch["launch_byte"].as_u64().unwrap() != 0)
    }
    fn dump_launch_settings(&mut self, _s: &mut Session, label: &'static str) -> aa_netcode::Result<()> {
        self.events.push(json!({"event": "dump_settings", "label": label, "settings": "launch"}));
        Ok(())
    }
    fn timeline_freeze(&mut self, flag: bool) -> aa_netcode::Result<()> {
        self.events.push(json!({"event": "timeline_freeze", "arg": u8::from(flag)}));
        Ok(())
    }
    fn reset_order_version(&mut self) -> aa_netcode::Result<()> {
        self.events.push(json!({"event": "reset_order_version", "arg": 0}));
        Ok(())
    }
    fn state_changed(&mut self, _s: &mut Session, e: StateEvent) -> aa_netcode::Result<()> {
        // Oracle names: +0x34 (the flag byte) is "field_34", +0x38 (0) is "flag".
        self.events.push(json!({"event": "state_event", "field_0": 0, "text": e.text, "state": e.state,
            "launch_kind": e.launch_state.kind, "launch_field_18": e.launch_state.field_18,
            "field_34": u8::from(e.flag), "flag": 0}));
        Ok(())
    }
    fn copy_launch_settings(&mut self, _s: &mut Session, dst: SettingsSlot, src: SettingsSlot) -> aa_netcode::Result<()> {
        // 0x779F00 is a recording seam in the oracle (no copy is made there).
        self.events.push(json!({"event": "copy_settings", "dst": slot(dst), "src": slot(src)}));
        Ok(())
    }
}
impl KingdomHost for Host<'_> {
    fn kingdom_record_info(&mut self, session: &Session, record: usize) -> aa_netcode::Result<Option<bool>> {
        let name = session.launch.active_map().header.kingdoms[record].name.clone();
        self.events.push(json!({"event": "kingdom_info", "kingdom": name}));
        let open = &self.launch["kingdom_open"][String::from_utf16(&name).unwrap()];
        Ok(open.as_bool())
    }
    fn assign_kingdom(&mut self, session: &mut Session, player: u32, record: usize) -> aa_netcode::Result<()> {
        let name = session.launch.active_map().header.kingdoms[record].name.clone();
        self.events.push(json!({"event": "set_kingdom", "player": player, "kingdom": name, "report": 0}));
        self.events.push(json!({"event": "set_kingdom_30", "record": record, "player": player}));
        self.events.push(json!({"event": "set_kingdom_34", "record": record, "player": player}));
        Ok(())
    }
}

fn link(v: &Value) -> PeerLink {
    de(v)
}

#[test]
fn native_launch_processes() {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/skirmish-20261017-order-id-audit/native-launch-registry358.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    let cases = v["cases"].as_array().unwrap();
    let mut counts = std::collections::BTreeMap::<String, (usize, usize)>::new();
    for (n, c) in cases.iter().enumerate() {
        let case = &c["case"];
        let r = &c["result"];
        let l = &case["launch"];
        let kind = l["kind"].as_str().unwrap();
        assert_eq!(r["status"], "ok", "{n}");
        let mut session: Session = de(&case["session"]);
        session.launch_countdown = l["countdown_118"].as_i64().unwrap() as i32;
        session.host_countdown = l["countdown_11c"].as_i64().unwrap() as i32;
        session.flag_169 = l["flag_169"].as_bool().unwrap();
        (session.flag_16a, session.flag_16b, session.flag_16c) = (false, true, true);
        (session.field_f8, session.field_100) = (0x55, 0x77);
        session.launch.text_30 = units("pending text");
        let nd = &case["node"];
        let node = StagingNode {
            view: NodeView { host: link(&nd["host"]) },
            took_over: l["migrated"].as_i64().unwrap() as i32,
            ordering_player: l["ordering"].as_u64().map(|x| x as u32),
            local_player: l["local"].as_u64().map(|x| x as u32),
            internet: l["internet"].as_bool().unwrap(),
        };
        let mut conns: Vec<(PeerAddress, PeerIdentity)> =
            nd["peers"].as_array().unwrap().iter().map(|p| (de(&p["address"]), de(&p["identity"]))).collect();
        conns.push((de(&nd["conn_address"]), de(&nd["conn_identity"])));
        let peers = Peers { node: de(&nd["identity"]), conns };
        let mut host = Host { launch: l, events: Vec::new(), request: None };
        let mut result = Value::Null;
        match kind {
            "start" => start_countdown(&mut session, &node, &peers, &mut host).unwrap(),
            "stop" => stop_countdown(&mut session, &node, &mut host).unwrap(),
            "progress" => countdown_progress(&mut session, l["value"].as_i64().unwrap() as i32, &node, &mut host).unwrap(),
            "launch" => launch(&mut session, &node, &mut host).unwrap(),
            "change_state" => {
                let st = l["new_state"].as_u64().unwrap() as u32;
                change_to_state(&mut session, st, l["new_flag"].as_u64().unwrap() != 0, &node, &mut host).unwrap()
            }
            "auto_assign" => {
                let id = l["assign_player"].as_u64().unwrap() as u32;
                result = json!(u8::from(auto_assign_kingdom(&mut session, id, &mut host).unwrap()));
            }
            "all_ready" => result = json!(u8::from(all_ready(&session, &node, &peers).unwrap())),
            "ready" => {
                result = json!((0..session.players.len())
                    .map(|i| u8::from(player_ready(&session, i, &node, &peers).unwrap()))
                    .collect::<Vec<_>>())
            }
            k => panic!("{k}"),
        }
        assert_eq!(result, r["result"], "{n} {kind}: result");
        // The oracle seam records ui args for "|d" formats only; others carry [].
        let mut native = r["events"].as_array().unwrap().clone();
        for e in native.iter_mut() {
            if e["event"] == "ui_message" && e.get("args").is_none() {
                e["args"] = json!([]);
            }
        }
        assert_eq!(Value::Array(host.events.clone()), Value::Array(native), "{n} {kind}: events");
        match &host.request {
            Some((bytes, end)) => {
                assert_eq!(json!(bytes), r["request"]["bytes"], "{n} {kind}: request bytes");
                assert_eq!(json!(end), r["request"]["end"], "{n} {kind}: request end");
            }
            None => assert_eq!(r["request"]["end"], json!({"byte": 0, "bit": 3}), "{n} {kind}: no request"),
        }
        let s = &r["session"];
        let mine = json!({
            "countdown_118": session.launch_countdown, "countdown_11c": session.host_countdown,
            "field_164": session.field_164, "flag_169": u8::from(session.flag_169), "flag_16a": u8::from(session.flag_16a),
            "flag_16b": u8::from(session.flag_16b), "flag_16c": u8::from(session.flag_16c), "state": session.state,
            "game_type": session.launch.game_type, "field_f8": session.field_f8, "field_100": session.field_100,
            "changed": u8::from(session.changed), "changed_c9": u8::from(session.changed_c9), "text_84": session.launch.text_30,
            "players": session.players.iter().map(|p| json!({"id": p.id, "kingdom": p.kingdom, "flag_2c": u8::from(p.flag_2c)})).collect::<Vec<_>>(),
        });
        assert_eq!(&mine, s, "{n} {kind}: session after");
        let e = counts.entry(kind.to_string()).or_default();
        e.0 += 1;
        e.1 += usize::from(!host.events.is_empty() || host.request.is_some());
    }
    assert_eq!(cases.len(), 700);
    for (k, (total, active)) in &counts {
        assert!(*active > 0 || k == "ready" || k == "all_ready", "{k}: {active}/{total}");
    }
}
#[test]
fn countdown_uses_full_crt358_descriptor_and_native_wire_bytes(){
    let descriptors=&aa_orders::native::DESCRIPTORS;
    assert_eq!(descriptors.len(),358);
    assert_eq!(descriptors[LAUNCH_COUNTDOWN_PROGRESS_REQUEST].name,"TGC_LaunchCountdownProgressOrderRequest");
    assert_eq!(descriptors[LAUNCH_COUNTDOWN_PROGRESS_REQUEST].handler_va,0x773070);
    let native:Value=serde_json::from_str(include_str!("E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261017-order-id-audit/native-countdown-wire358.json")).unwrap();
    assert_eq!(native["registry_count"],358);assert_eq!(native["rows"].as_array().unwrap().len(),15);
    for row in native["rows"].as_array().unwrap(){
        let mut writer=WireWriter::new(Position::PACKET_START).unwrap();
        writer.write_uint(LAUNCH_COUNTDOWN_PROGRESS_REQUEST as u32,aa_orders::registry_bits(descriptors.len()).unwrap()).unwrap();
        writer.write_uint(row["value"].as_u64().unwrap()as u32,32).unwrap();
        let end=writer.position();let bytes=&writer.bytes()[..end.absolute().unwrap().div_ceil(8)];
        assert_eq!(json!({"bytes":bytes,"end":end}),row["output"],"{row}");
    }
}

}
