//! The LAN game list. The original's lobby asks with a GameSpy QR2 query on the
//! game's own port (LobbyJoinDirectExecute 0x648030 -> 0x787750, the LAN list
//! 0x79A370): FE FD 00, a 4-byte request key, FF FF FF. The answer is the type
//! byte 0, the key, the server keys as name and value strings ended by an empty
//! string, then the player and team lists (`LanNode::answer_queries` writes it).
//! The Join Direct dialog does not ask: 0x6EC460 -> 0x6EC8B0 -> 0x651A10 connects.
//!
//! The list of 0x79A370 goes through GameSpy's LAN browse, a broadcast on the
//! host's port range. Here [`Browser`] sends the same query there, and to this
//! machine, only while a person has the lobby open. Nothing leaves the local
//! network: a broadcast does not pass a router.
use super::{HOST_PORT_DEFAULT, PORTS_TO_TRY};
use crate::Result;
use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::Instant;

/// The first three bytes of a query.
pub const QUERY_START: [u8; 3] = [0xFE, 0xFD, 0x00];

/// A query with its request key.
pub fn query(key: [u8; 4]) -> [u8; 10] {
    [0xFE, 0xFD, 0x00, key[0], key[1], key[2], key[3], 0xFF, 0xFF, 0xFF]
}

/// A request key that differs between sessions.
pub fn new_key() -> [u8; 4] {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0x5A5A_5A5A);
    nanos.to_le_bytes()
}

/// What a host says of its game (the keys of KKC_GameSpyQR2 0x79A000's session).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GameInfo {
    pub name: String,
    pub version: String,
    pub port: u16,
    pub map: String,
    pub players: i32,
    pub max_players: i32,
    /// `gamemode` is not "openstaging": the game is running.
    pub playing: bool,
    pub password: bool,
}

/// The answer to the query with `key`, `None` for anything else.
pub fn parse_reply(bytes: &[u8], key: [u8; 4]) -> Option<GameInfo> {
    if bytes.len() < 5 || bytes[0] != 0 || bytes[1..5] != key {
        return None;
    }
    let mut info = GameInfo::default();
    let mut strings = bytes[5..].split(|b| *b == 0);
    while let Some(name) = strings.next() {
        if name.is_empty() {
            break;
        }
        let value = String::from_utf8_lossy(strings.next()?).into_owned();
        match name {
            b"hostname" => info.name = value,
            b"gamever" => info.version = value,
            b"hostport" => info.port = value.parse().unwrap_or(0),
            b"mapname" => info.map = value,
            b"numplayers" => info.players = value.parse().unwrap_or(0),
            b"maxplayers" => info.max_players = value.parse().unwrap_or(0),
            b"gamemode" => info.playing = value != "openstaging",
            b"password" => info.password = value != "0",
            _ => {}
        }
    }
    Some(info)
}

/// One game the list shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedGame {
    pub address: SocketAddr,
    pub info: GameInfo,
    pub ping_ms: u32,
}

/// Times a refresh repeats its query, and the gap, for a datagram that was lost.
const QUERIES_PER_REFRESH: u32 = 3;
const QUERY_GAP_MS: u128 = 400;

/// The lobby's LAN list: alive while a person has the lobby open.
pub struct Browser {
    socket: UdpSocket,
    key: [u8; 4],
    broadcast: Option<Ipv4Addr>,
    sent: u32,
    last_sent: Option<Instant>,
    pub games: Vec<ListedGame>,
}

impl Browser {
    /// `broadcast`: the broadcast address, `None` keeps the list on this machine
    /// (a test: nothing goes beyond loopback then).
    pub fn new(broadcast: Option<Ipv4Addr>) -> Result<Self> {
        let bind = if broadcast.is_some() { Ipv4Addr::UNSPECIFIED } else { Ipv4Addr::LOCALHOST };
        let socket = UdpSocket::bind((bind, 0))?;
        socket.set_nonblocking(true)?;
        if broadcast.is_some() {
            socket.set_broadcast(true)?;
        }
        Ok(Self { socket, key: new_key(), broadcast, sent: 0, last_sent: None, games: Vec::new() })
    }

    /// LobbyRefreshServerList: the list starts again.
    pub fn refresh(&mut self) {
        self.key = new_key();
        self.sent = 0;
        self.last_sent = None;
        self.games.clear();
    }

    /// One frame: the queries due, the answers waiting. True when the list changed.
    pub fn pump(&mut self) -> bool {
        if self.sent < QUERIES_PER_REFRESH && self.last_sent.is_none_or(|t| t.elapsed().as_millis() >= QUERY_GAP_MS) {
            let bytes = query(self.key);
            for i in 0..PORTS_TO_TRY {
                let port = HOST_PORT_DEFAULT + i;
                // A failed send (no network yet) leaves the list empty, no more.
                let _ = self.socket.send_to(&bytes, (Ipv4Addr::LOCALHOST, port));
                if let Some(b) = self.broadcast {
                    let _ = self.socket.send_to(&bytes, (b, port));
                }
            }
            self.sent += 1;
            self.last_sent = Some(Instant::now());
        }
        let mut changed = false;
        let mut buffer = [0u8; 2048];
        loop {
            match self.socket.recv_from(&mut buffer) {
                Ok((len, from)) => {
                    let Some(info) = parse_reply(&buffer[..len], self.key) else { continue };
                    // One game seen twice, on this machine's two addresses.
                    if self.games.iter().any(|g| g.address == from || (g.address.port() == from.port() && g.info.name == info.name && (g.address.ip().is_loopback() || from.ip().is_loopback()))) {
                        continue;
                    }
                    let ping_ms = self.last_sent.map_or(0, |t| t.elapsed().as_millis() as u32);
                    self.games.push(ListedGame { address: from, info, ping_ms });
                    changed = true;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == ErrorKind::ConnectionReset => continue,
                Err(_) => break,
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_reads_back() {
        let key = [1, 2, 3, 4];
        let mut b = vec![0u8, 1, 2, 3, 4];
        for (k, v) in [("hostname", "A game"), ("hostport", "7291"), ("numplayers", "2"), ("maxplayers", "8"), ("gamemode", "openstaging"), ("password", "1"), ("mapname", "Berlin")] {
            b.extend_from_slice(k.as_bytes());
            b.push(0);
            b.extend_from_slice(v.as_bytes());
            b.push(0);
        }
        b.push(0);
        b.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        let info = parse_reply(&b, key).unwrap();
        assert_eq!(info.name, "A game");
        assert_eq!(info.port, 7291);
        assert_eq!((info.players, info.max_players), (2, 8));
        assert!(!info.playing && info.password);
        assert_eq!(info.map, "Berlin");
        assert!(parse_reply(&b, [9, 9, 9, 9]).is_none());
    }
}
