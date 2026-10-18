//! Connect (admin packet type 2) and the start of the join flow.
//!
//! * Writer TGC_AdminConn::SerializeConnect(O) 0x750330, reader and
//!   validation SerializeConnect(I) 0x75BF90 (host only).
//! * Player id 0x77BA70 object, codecs 0x74EBC0 / 0x752660 (+0x74CCC0 /
//!   0x74CD30); connect info 0x74EDE0 (copy 0x750720).
//! * Acceptance builds TGC_PlayerJoinOrderRequest (ctor 0x7589C0, emitted by
//!   0x74C9C0, descriptor 306) whose payload 0x77B680 is the connect info
//!   with the connection's address (AdminConn vfunc 0 0x74B4E0 =
//!   NetConn+0x9C, written ip/port by 0x76A7C0).
//! * Rejection: 0x74B620(1, reason): +0x89 disconnecting, +0x90 = NetNode
//!   time, +0x8C reason; the ClientDisconnectDecree (type 11) then carries
//!   the reason.
//!
//! Evidence: research/phase8-net/native-connect.json.
use crate::packets::AdminConn;
use crate::{Error, Result};
use aa_orders::commands::WireWriter;
use aa_orders::payload::{Field, Registry, SchemaContext};
use aa_orders::Cursor;
use serde::{Deserialize, Serialize};

/// TGC_PlayerJoinOrderRequest / Decree descriptor ordinals.
pub const PLAYER_JOIN_REQUEST: usize = 306;
pub const PLAYER_JOIN_DECREE: usize = 317;

/// CString on the wire (writer inline in 0x750330, reader 0x410B50): 16-bit
/// unit count then the units as one `16 * count`-bit buffer. The native
/// writer sends `count & 0xFFFF` units, so a longer string is cut on the
/// wire; this is reproduced.
pub fn write_text(w: &mut WireWriter, units: &[u16]) -> Result<()> {
    let count = units.len() & 0xffff;
    w.write_uint(count as u32, 16)?;
    let bytes: Vec<u8> = units[..count].iter().flat_map(|u| u.to_le_bytes()).collect();
    w.write_raw_bits(&bytes, count * 16)?;
    Ok(())
}
/// 0x410B50.
pub fn read_text(r: &mut Cursor<'_>) -> Result<Vec<u16>> {
    let count = r.read_uint(16)? as usize;
    let raw = r.read_raw_bits(count * 16)?;
    Ok(raw.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect())
}

/// Player id object (0x77BA70, 0x14 bytes): +0 CString, +4..+0x10 words.
/// `Default` = the 0x77B8D0 reset: empty name, zero words.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerId {
    /// +0 key text (the internet ban-list key, 0x74D2F0).
    pub name: Vec<u16>,
    /// +4.
    pub serial: u32,
    /// +8, +0xC, +0x10.
    pub a: u32,
    pub b: u32,
    pub c: u32,
}
/// 0x74EBC0: +4, text +0, then 0x74CCC0 (+8, +0xC, +0x10).
pub fn write_player_id(w: &mut WireWriter, id: &PlayerId) -> Result<()> {
    w.write_uint(id.serial, 32)?;
    write_text(w, &id.name)?;
    w.write_uint(id.a, 32)?;
    w.write_uint(id.b, 32)?;
    w.write_uint(id.c, 32)?;
    Ok(())
}
/// 0x752660 (+0x74CD30).
pub fn read_player_id(r: &mut Cursor<'_>) -> Result<PlayerId> {
    let serial = r.read_uint(32)?;
    let name = read_text(r)?;
    let a = r.read_uint(32)?;
    let b = r.read_uint(32)?;
    let c = r.read_uint(32)?;
    Ok(PlayerId { name, serial, a, b, c })
}

/// Everything a Connect packet carries, in wire order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectInfo {
    /// AdminNode+0xEC player name.
    pub name: Vec<u16>,
    /// AdminNode+0xF0 password typed by the joining player.
    pub password: Vec<u16>,
    /// [0xB3888C] and [0xB388A0] version strings.
    pub version: Vec<u16>,
    pub build: Vec<u16>,
    /// [0xB36530]+8 game data checksum.
    pub data_checksum: u32,
    /// 0x5AF810 depot checksum string and 0x5AF970 mods string.
    pub depot_checksums: Vec<u16>,
    pub mods: Vec<u16>,
    /// AdminNode+0x104.
    pub player_id: PlayerId,
    /// [0xB3654C]+0xBC internet game.
    pub internet: bool,
}

/// 0x750330 body.
pub fn write_connect(w: &mut WireWriter, info: &ConnectInfo) -> Result<()> {
    write_text(w, &info.name)?;
    write_text(w, &info.password)?;
    write_text(w, &info.version)?;
    write_text(w, &info.build)?;
    w.write_uint(info.data_checksum, 32)?;
    write_text(w, &info.depot_checksums)?;
    write_text(w, &info.mods)?;
    write_player_id(w, &info.player_id)?;
    w.write_uint(u32::from(info.internet), 1)?;
    Ok(())
}

/// Body parse of 0x75BF90 (after its three gates).
pub fn read_connect(r: &mut Cursor<'_>) -> Result<ConnectInfo> {
    let name = read_text(r)?;
    let password = read_text(r)?;
    let version = read_text(r)?;
    let build = read_text(r)?;
    let data_checksum = r.read_uint(32)?;
    let depot_checksums = read_text(r)?;
    let mods = read_text(r)?;
    let player_id = read_player_id(r)?;
    let internet = r.read_uint(1)? == 1;
    Ok(ConnectInfo { name, password, version, build, data_checksum, depot_checksums, mods, player_id, internet })
}

/// Host-side values 0x75BF90 compares against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostConnectState {
    /// AdminNode == AdminNode+0x2C4.
    pub we_are_host: bool,
    /// AdminNode+0x2E0 (set when this node became host by migration,
    /// 0x75A350; read through 0x74BB40).
    pub migrated_host: bool,
    /// [0xB3654C]+0xBC.
    pub internet: bool,
    /// Session +0xE4 state (2 and 4 refuse) and +0x118 launch countdown
    /// (0x76ADE0: > 0, a countdown is running, refuses; see
    /// crate::staging::countdown_progress).
    pub session_mode: i32,
    #[serde(alias = "session_players_in_game")]
    pub launch_countdown: i32,
    /// Session +0xD0 password (empty = none).
    pub password: Vec<u16>,
    pub version: Vec<u16>,
    pub build: Vec<u16>,
    pub data_checksum: u32,
    pub depot_checksums: Vec<u16>,
    pub mods: Vec<u16>,
}

/// Peer IPv4 address and port in host order (NetConn+0x9C sockaddr, read by
/// 0x76A7C0 through ntohl/ntohs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerAddress {
    pub ip: u32,
    pub port: u16,
}

/// Outcome of 0x75BF90.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectVerdict {
    /// AdminLayer.cpp log line; nothing read.
    Ignored(u32),
    /// +0x420 accepted; PlayerJoin request (descriptor 306) to emit with
    /// these payload fields (0x7589C0 / 0x74C9C0 / 0x77B680).
    Accepted { info: ConnectInfo, request: Vec<Field> },
    /// 0x74B620(1, reason); `banned_prompt` = 0x75B4D0 ran (UI prompt
    /// "networking_disconnect_allow_banned", internet only).
    Rejected { info: ConnectInfo, reason: u32, banned_prompt: bool },
}

/// _wcsicmp 0x96CAD6 in the "C" LC_CTYPE locale (AA only ever sets
/// LC_NUMERIC, 0x7F1965/0x7F1A39): units 'A'..'Z' fold to lower case, the
/// first differing folded unit decides, comparison stops at a NUL.
pub fn wcsicmp_c(a: &[u16], b: &[u16]) -> bool {
    let fold = |u: u16| if (0x41..=0x5a).contains(&u) { u + 0x20 } else { u };
    let mut i = 0;
    loop {
        let x = a.get(i).copied().map_or(0, fold);
        let y = b.get(i).copied().map_or(0, fold);
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
        i += 1;
    }
}
/// wcscmp 0x96CB83 equality (through 0x40DEE0): stops at a NUL.
pub fn wcs_eq(a: &[u16], b: &[u16]) -> bool {
    let mut i = 0;
    loop {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
        i += 1;
    }
}
fn cstr_len(units: &[u16]) -> usize {
    units.len()
}

/// 0x75BF90: gates, parse, validation in native order (reasons 12, 9, 10,
/// 2, 2, 1, 3, 6, 8, 7) and the acceptance / rejection effects on `conn`.
/// `banned` is the internet ban-list lookup 0x54C240 (key = player id
/// text), consulted by 0x74D2F0 only for an internet game and a non-empty
/// key. `now_bits` is NetNode+0x58 (binary32) for 0x74B620.
pub fn read_and_validate_connect(
    conn: &mut AdminConn,
    r: &mut Cursor<'_>,
    host: &HostConnectState,
    peer: PeerAddress,
    now_bits: u32,
    banned: &mut dyn FnMut(&[u16]) -> bool,
) -> Result<ConnectVerdict> {
    if !host.we_are_host {
        return Ok(ConnectVerdict::Ignored(0x1125));
    }
    if conn.accepted {
        return Ok(ConnectVerdict::Ignored(0x112b));
    }
    if conn.connected || conn.disconnecting {
        return Ok(ConnectVerdict::Ignored(0x1131));
    }
    let info = read_connect(r)?;
    let reason = connect_reason(&info, host, banned);
    match reason {
        Some((reason, banned_prompt)) => {
            conn.disconnecting = true;
            conn.disconnect_started = now_bits;
            conn.disconnect_reason = reason;
            Ok(ConnectVerdict::Rejected { info, reason, banned_prompt })
        }
        None => {
            conn.accepted = true;
            let request = player_join_fields(&info, peer);
            Ok(ConnectVerdict::Accepted { info, request })
        }
    }
}

fn connect_reason(
    info: &ConnectInfo,
    host: &HostConnectState,
    banned: &mut dyn FnMut(&[u16]) -> bool,
) -> Option<(u32, bool)> {
    // 0x74BB40 on the node: the host's +0x2E0.
    if host.we_are_host && host.migrated_host {
        return Some((12, false));
    }
    if info.internet != host.internet {
        return Some((9, false));
    }
    if host.session_mode == 2 || host.session_mode == 4 || host.launch_countdown > 0 {
        return Some((10, false));
    }
    if !wcs_eq(&info.version, &host.version) || !wcs_eq(&info.build, &host.build) {
        return Some((2, false));
    }
    if cstr_len(&host.password) != 0 && !wcs_eq(&info.password, &host.password) {
        return Some((1, false));
    }
    if host.internet && !info.player_id.name.is_empty() && banned(&info.player_id.name) {
        return Some((3, true));
    }
    if !wcsicmp_c(&info.depot_checksums, &host.depot_checksums) {
        return Some((6, false));
    }
    if !wcsicmp_c(&info.mods, &host.mods) {
        return Some((8, false));
    }
    if info.data_checksum != host.data_checksum {
        return Some((7, false));
    }
    None
}

/// TGC_PlayerJoinOrderRequest payload (0x77B680, schema p306): four texts,
/// checksum, the connection address (0x76A7C0: ip, port as 32-bit words),
/// then the player id (0x74EBC0).
pub fn player_join_fields(info: &ConnectInfo, peer: PeerAddress) -> Vec<Field> {
    vec![
        Field::Text(info.name.clone()),
        Field::Text(info.password.clone()),
        Field::Text(info.version.clone()),
        Field::Text(info.build.clone()),
        Field::Uint { width: 32, value: info.data_checksum },
        Field::NetAddr { ip: peer.ip, port: u32::from(peer.port) },
        Field::Uint { width: 32, value: info.player_id.serial },
        Field::Text(info.player_id.name.clone()),
        Field::Uint { width: 32, value: info.player_id.a },
        Field::Uint { width: 32, value: info.player_id.b },
        Field::Uint { width: 32, value: info.player_id.c },
    ]
}

/// Schema context for lobby payloads that read no game definitions; any
/// definition query is a typed error.
pub struct LobbyContext;
impl SchemaContext for LobbyContext {
    fn registry_count(&self, _registry: Registry) -> aa_orders::Result<usize> {
        Err(aa_orders::Error::SchemaContext("lobby payload has no definition registries"))
    }
    fn team_command_flags(&self, _index: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext("lobby payload has no team commands"))
    }
    fn definition_kind(&self, _registry: Registry, _index: u32) -> aa_orders::Result<u32> {
        Err(aa_orders::Error::SchemaContext("lobby payload has no definitions"))
    }
    fn zero_width_list_budget(&self) -> usize {
        0
    }
}

/// Payload bytes of a PlayerJoin request (for tests / emitters).
pub fn encode_player_join(fields: &[Field], w: &mut WireWriter) -> Result<()> {
    aa_orders::payload::encode(PLAYER_JOIN_REQUEST, fields, w, &LobbyContext).map_err(Error::from)
}
