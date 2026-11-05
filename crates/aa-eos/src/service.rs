//! What the game holds: commands go to the SDK's thread, events and datagrams
//! come back. Nothing here blocks, and nothing here touches the SDK.
use crate::api::{self, LoadError};
use crate::config::{Config, ConfigError};
use crate::{addr, worker};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// A ProductUserId as text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UserId(pub String);

impl UserId {
    /// This player's address on the link.
    pub fn address(&self) -> SocketAddrV4 {
        addr::of_user(&self.0)
    }
}

/// What a lobby advertises. The host's session stays the truth about the game;
/// this is what the list shows before anyone has joined it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LobbySpec {
    pub name: String,
    pub map: String,
    pub players: u32,
    pub max_players: u32,
    pub password: bool,
    /// Observers come on top of the players, so the lobby must have room for them.
    pub observers: bool,
    /// The game's version and build: a list may mark what it cannot join.
    pub version: String,
}

/// A lobby the search found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LobbyInfo {
    pub id: String,
    pub owner: UserId,
    pub name: String,
    pub map: String,
    pub players: u32,
    pub max_players: u32,
    pub password: bool,
    pub version: String,
}

/// A step that failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    /// What was being done, in words.
    pub doing: &'static str,
    /// The SDK's result code and its name (`EOS_NoConnection`).
    pub code: i32,
    pub result: String,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} failed: {} ({})", self.doing, self.result, self.code)
    }
}

/// [`Event::PeerClosed`]'s words when the other player closed the connection.
pub const CLOSED_BY_PEER: &str = "the other player closed it";

/// What happened since the last look.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The platform could not start: nothing online works in this process.
    StartFailed(String),
    SignedIn { user: UserId },
    SignInFailed(Failure),
    /// The sign-in ended and could not be renewed.
    SignedOut { why: String },
    /// `create_lobby` is done and the lobby is on the list.
    LobbyCreated { id: String },
    LobbyCreateFailed(Failure),
    /// `update_lobby` failed; the lobby goes on with what it showed before.
    LobbyUpdateFailed(Failure),
    /// A search's results, without this player's own lobby.
    LobbiesFound(Vec<LobbyInfo>),
    LobbySearchFailed(Failure),
    /// `join_lobby` is done: `owner` is who to reach over the link.
    LobbyJoined { id: String, owner: UserId },
    LobbyJoinFailed(Failure),
    /// `leave_lobby` is done (or there was nothing to leave).
    LobbyLeft,
    /// A connection to this player ended, for this reason in words.
    PeerClosed { user: UserId, why: String },
    /// A warning or error the SDK logged.
    SdkLog(String),
}

/// Why the service did not start.
#[derive(Debug)]
pub enum StartError {
    Library(LoadError),
    Config(ConfigError),
    /// The SDK allows one platform per process, and it can never start twice.
    AlreadyStarted,
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StartError::Library(e) => e.fmt(f),
            StartError::Config(e) => e.fmt(f),
            StartError::AlreadyStarted => f.write_str("Epic Online Services already started once in this run"),
        }
    }
}
impl std::error::Error for StartError {}

pub(crate) enum Command {
    SignIn { display_name: String },
    CreateLobby(LobbySpec),
    UpdateLobby(LobbySpec),
    FindLobbies,
    JoinLobby { id: String, owner: UserId },
    LeaveLobby,
    Send { to: UserId, data: Vec<u8> },
    Shutdown,
}

/// The two sides' mailboxes.
#[derive(Default)]
pub(crate) struct Shared {
    pub events: Mutex<Vec<Event>>,
    pub incoming: Mutex<VecDeque<(SocketAddrV4, Vec<u8>)>>,
    /// Address -> player, for everyone this node has heard of.
    pub peers: Mutex<HashMap<SocketAddrV4, UserId>>,
    /// Set by the worker on its way out, however it goes.
    pub finished: AtomicBool,
}

impl Shared {
    pub fn event(&self, e: Event) {
        if let Ok(mut q) = self.events.lock() {
            q.push(e);
        }
    }
    /// The address of `user`, remembered both ways.
    pub fn peer(&self, user: &UserId) -> SocketAddrV4 {
        let a = user.address();
        if let Ok(mut p) = self.peers.lock() {
            p.insert(a, user.clone());
        }
        a
    }
}

static STARTED: AtomicBool = AtomicBool::new(false);

/// The game's handle on the SDK's thread.
pub struct Service {
    tx: Sender<Command>,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl Service {
    /// Loads the SDK's library and starts its thread. Failures of the platform
    /// itself come as [`Event::StartFailed`].
    pub fn start(config: Config) -> Result<Self, StartError> {
        let api = api::load().map_err(StartError::Library)?;
        if STARTED.swap(true, Ordering::SeqCst) {
            return Err(StartError::AlreadyStarted);
        }
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Shared::default());
        let for_worker = shared.clone();
        let thread = std::thread::Builder::new().name("eos".into()).spawn(move || worker::run(api, config, rx, for_worker));
        match thread {
            Ok(thread) => Ok(Self { tx, shared, thread: Some(thread) }),
            Err(e) => {
                let s = shared.clone();
                s.event(Event::StartFailed(format!("the online thread did not start: {e}")));
                Ok(Self { tx, shared, thread: None })
            }
        }
    }

    /// Only finds out whether the library is there (no platform, no network).
    pub fn check_library() -> Result<(), LoadError> {
        api::load().map(|_| ())
    }

    fn send(&self, c: Command) {
        // an ended worker has reported why
        let _ = self.tx.send(c);
    }

    /// Everything that happened since the last call.
    pub fn poll(&self) -> Vec<Event> {
        self.shared.events.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default()
    }

    /// The thread ended: an event said why, or it panicked. Nothing works now.
    pub fn finished(&self) -> bool {
        self.shared.finished.load(Ordering::SeqCst)
    }

    /// Signs in with this machine's device ID; `display_name` is what other
    /// players see. No Epic account is involved.
    pub fn sign_in(&self, display_name: &str) {
        self.send(Command::SignIn { display_name: display_name.to_string() });
    }
    pub fn create_lobby(&self, spec: LobbySpec) {
        self.send(Command::CreateLobby(spec));
    }
    /// Shows new attributes on the lobby this player owns.
    pub fn update_lobby(&self, spec: LobbySpec) {
        self.send(Command::UpdateLobby(spec));
    }
    pub fn find_lobbies(&self) {
        self.send(Command::FindLobbies);
    }
    /// Joins a lobby of the last search.
    pub fn join_lobby(&self, lobby: &LobbyInfo) {
        self.send(Command::JoinLobby { id: lobby.id.clone(), owner: lobby.owner.clone() });
    }
    /// Leaves or, for its owner, closes the lobby, and closes the connections.
    pub fn leave_lobby(&self) {
        self.send(Command::LeaveLobby);
    }

    /// The datagram path of a session; `me` is this player, from [`Event::SignedIn`].
    pub fn link(&self, me: &UserId) -> Link {
        self.shared.peer(me);
        Link { tx: self.tx.clone(), shared: self.shared.clone(), me: me.address() }
    }
}

impl Drop for Service {
    /// Leaves properly when the game ends, without holding the game up.
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        let Some(thread) = self.thread.take() else { return };
        for _ in 0..200 {
            if self.shared.finished.load(Ordering::SeqCst) {
                let _ = thread.join();
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

/// Datagrams to and from other players, best effort, as UDP is. A player is an
/// IPv4 address computed from the ID ([`addr`]). `Send`; never blocks.
#[derive(Clone)]
pub struct Link {
    tx: Sender<Command>,
    shared: Arc<Shared>,
    me: SocketAddrV4,
}

impl Link {
    pub fn local_addr(&self) -> SocketAddr {
        SocketAddr::V4(self.me)
    }

    /// Makes a player reachable before anything came from them (a client's host,
    /// known from the lobby); returns the address.
    pub fn add_peer(&self, user: &UserId) -> SocketAddr {
        SocketAddr::V4(self.shared.peer(user))
    }

    /// One datagram; `false` when the address is nobody's yet (dropped). A
    /// datagram of any length up to 65,535 bytes goes.
    pub fn send_to(&mut self, bytes: &[u8], to: SocketAddr) -> bool {
        let SocketAddr::V4(to) = to else { return false };
        let user = self.shared.peers.lock().ok().and_then(|p| p.get(&to).cloned());
        let Some(user) = user else { return false };
        let _ = self.tx.send(Command::Send { to: user, data: bytes.to_vec() });
        true
    }

    /// The next waiting datagram: into `buffer`, its length and sender. A longer
    /// one than `buffer` is cut, as a socket would.
    pub fn recv_from(&mut self, buffer: &mut [u8]) -> Option<(usize, SocketAddr)> {
        let (from, data) = self.shared.incoming.lock().ok()?.pop_front()?;
        let n = data.len().min(buffer.len());
        buffer[..n].copy_from_slice(&data[..n]);
        Some((n, SocketAddr::V4(from)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link() -> (Link, mpsc::Receiver<Command>, UserId) {
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Shared::default());
        let me = UserId("0002aaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
        shared.peer(&me);
        (Link { tx, shared, me: me.address() }, rx, me)
    }

    #[test]
    fn a_link_is_what_a_session_needs() {
        fn check<T: Send + Sync + Clone>() {}
        check::<Link>();
    }

    #[test]
    fn an_address_nobody_has_is_dropped() {
        let (mut l, rx, _) = link();
        let nobody = SocketAddr::from(([10, 1, 2, 3], 4000));
        assert!(!l.send_to(b"x", nobody));
        // an IPv6 peer does not exist on this path
        assert!(!l.send_to(b"x", "[::1]:4000".parse().unwrap()));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn a_known_player_is_sent_to_under_their_address() {
        let (mut l, rx, _) = link();
        let host = UserId("0002bbbbbbbbbbbbbbbbbbbbbbbbbbbb".into());
        let to = l.add_peer(&host);
        assert_eq!(to, SocketAddr::V4(host.address()));
        assert!(l.send_to(b"hello", to));
        match rx.try_recv() {
            Ok(Command::Send { to, data }) => {
                assert_eq!(to, host);
                assert_eq!(data, b"hello");
            }
            _ => panic!("no send"),
        }
    }

    #[test]
    fn datagrams_come_out_in_order_and_empty_means_none() {
        let (mut l, _rx, _) = link();
        let from = UserId("0002cccccccccccccccccccccccccccc".into());
        let a = l.shared.peer(&from);
        {
            let mut q = l.shared.incoming.lock().unwrap();
            q.push_back((a, vec![1, 2, 3]));
            q.push_back((a, vec![4]));
        }
        let mut buffer = [0u8; 16];
        assert_eq!(l.recv_from(&mut buffer), Some((3, SocketAddr::V4(a))));
        assert_eq!(&buffer[..3], &[1, 2, 3]);
        assert_eq!(l.recv_from(&mut buffer), Some((1, SocketAddr::V4(a))));
        assert_eq!(buffer[0], 4);
        assert_eq!(l.recv_from(&mut buffer), None);
    }

    #[test]
    fn a_failure_reads_as_a_sentence() {
        let f = Failure { doing: "Signing in", code: 1, result: "EOS_NoConnection".into() };
        assert_eq!(f.to_string(), "Signing in failed: EOS_NoConnection (1)");
    }
}
