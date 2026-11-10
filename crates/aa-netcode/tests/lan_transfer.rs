//! The map transfer of a LAN session, two nodes on 127.0.0.1 (one process,
//! real UDP sockets, each node with its own profile directory):
//!
//! * the host orders SetSessionLoadMap (91) with the file's checksum
//!   (0x858030 rule, `file::map_checksum`);
//! * a client without the file reports status 2 (SendUpdateMapStatus
//!   0x770B70), the host starts the transfer on its connection
//!   (SetMapTransferStatus 0x7554B0 -> 0x74FE30 -> FileSender 0x761540);
//! * the blocks travel in the type-5 file section (0x74E280 / 0x74C8B0,
//!   FileReceiver 0x761DB0 writes profile directory + name);
//! * the host's heartbeat sends UpdateMapStatus (305) with the progress
//!   (0x74F310) until +0x38 is 1; both nodes store it (0x74BBB0), which the
//!   staging row reads (0x69D6B2);
//! * a client that has the same file reports status 1 at once; one whose
//!   file differs sends nothing (0x770E8A).
//!
//! The pieces are the ported, oracle-proven codecs (tests/file_transfer.rs);
//! their composition here is a session of this port, not compared with the
//! original.
use aa_netcode::lan::registries::{self, NativeRegistries};
use aa_netcode::lan::transfer::MapLookup;
use aa_netcode::lan::{descriptor, HostOptions, LanConfig, LanData, LanEvent, LanNode};
use aa_netcode::lobby::{KingdomSetup, LaunchSettings};
use aa_netcode::session::PlayerId;
use aa_orders::payload::Field;
use aa_runtime::simulation::SpeedSettings;
use aa_simulation::FloatBits;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// A resource name as the original's session carries it
/// (`SetSessionLoadMap(Maps/Campaign/...RMP)` in its log).
const MAP: &str = "Maps/(2v2)Transfer.RMP";

struct Data {
    reg: NativeRegistries,
    dir: PathBuf,
    /// The front end gives its profile directory and looks maps up.
    channel: bool,
}
impl LanData for Data {
    fn registries(&self) -> &NativeRegistries {
        &self.reg
    }
    fn random_map_config(&self, key: &[u16]) -> Option<Vec<u16>> {
        Some(key.to_vec())
    }
    fn scenario(&self, key: &[u16]) -> Option<Vec<u16>> {
        Some(key.to_vec())
    }
    fn format(&self, _key: &str, args: &[Vec<u16>]) -> Vec<u16> {
        args.first().cloned().unwrap_or_default()
    }
    fn countdown_time(&self) -> i32 {
        2
    }
    fn nation_factions(&self, _nation: &[u16]) -> aa_netcode::Result<Vec<Vec<u16>>> {
        Ok(Vec::new())
    }
    fn speed_settings(&self) -> SpeedSettings {
        SpeedSettings {
            game_speed_min: FloatBits::from_f32(-2.0),
            game_speed_max: FloatBits::from_f32(2.0),
            film_min: FloatBits::from_f32(-4.0),
            film_max: FloatBits::from_f32(4.0),
            admin_ticks_per_second: FloatBits::from_f32(16.0),
        }
    }
    fn profile_directory(&self) -> Option<Vec<u16>> {
        if !self.channel {
            return None;
        }
        let mut d = units(&self.dir.to_string_lossy());
        d.push(u16::from(b'\\'));
        Some(d)
    }
    fn map_lookup(&self, file: &[u16]) -> MapLookup {
        if !self.channel {
            return MapLookup::Unchecked;
        }
        let path = self.dir.join(String::from_utf16_lossy(file));
        MapLookup::of_bytes(std::fs::read(path).ok().as_deref())
    }
}

fn data(dir: &Path) -> Box<Data> {
    let mut reg = NativeRegistries::default();
    for (r, n) in [
        (registries::OBJECT_DEFINITIONS, 40),
        (registries::BIOMES, 3),
        (registries::SHARED_PROPERTIES, 20),
        (registries::WEATHERS, 4),
        (registries::NATIONS, 6),
        (registries::FACTIONS, 12),
        (registries::KINGDOM_COLORS, 9),
        (registries::HANDICAPS, 3),
        (registries::GAME_TYPES, 5),
        (registries::WORLD_RULES, 7),
        (registries::ORGANIZATION_LAYOUTS, 10),
        (registries::FORMATIONS, 5),
        (registries::ABILITIES, 30),
    ] {
        reg.counts.insert(r.0, n);
    }
    reg.world_rule_kinds = vec![0; 7];
    Box::new(Data { reg, dir: dir.to_path_buf(), channel: true })
}

fn config(name: &str) -> LanConfig {
    LanConfig {
        name: units(name),
        password: Vec::new(),
        version: units("1.0.5.0"),
        build: units("AA"),
        data_checksum: 0x1234,
        depot_checksums: Vec::new(),
        mods: Vec::new(),
        player_id: PlayerId { name: units(name), serial: 1, a: 0, b: 0, c: 0 },
    }
}

fn launch() -> LaunchSettings {
    let mut ls = LaunchSettings { mode: 1, ..Default::default() };
    for k in ["kingdom_a", "kingdom_b"] {
        ls.map_default.header.kingdoms.push(KingdomSetup { name: units(k), flag_28: true, ..Default::default() });
        ls.game.header.kingdoms.push(KingdomSetup { name: units(k), flag_28: true, ..Default::default() });
    }
    ls.state.field_10 = 1;
    ls.state.field_18 = 4;
    ls.state.field_1c = 1;
    ls
}

fn options() -> HostOptions {
    HostOptions {
        game_name: units("transfer"),
        password: Vec::new(),
        max_players: 4,
        allow_observers: false,
        port: 0,
        ports_to_try: 1,
        bind: std::net::Ipv4Addr::LOCALHOST,
    }
}

/// A fresh scratch directory under the target folder.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("lan-transfer").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The file's bytes: not a multiple of the 64-byte block (and one that is).
fn map_bytes(len: usize) -> Vec<u8> {
    let mut x: u32 = 0x1234_5678;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect()
}

#[derive(Default)]
struct Seen {
    joined: bool,
    messages: Vec<String>,
}
fn note(seen: &mut Seen, events: Vec<LanEvent>) {
    for e in events {
        match e {
            LanEvent::Joined { .. } => seen.joined = true,
            LanEvent::Message { key, .. } => seen.messages.push(key),
            _ => {}
        }
    }
}

struct Pair {
    host: LanNode,
    client: LanNode,
    hs: Seen,
    cs: Seen,
}
impl Pair {
    fn new(host_dir: &Path, client_dir: &Path) -> Self {
        Self::with(data(host_dir), data(client_dir))
    }
    fn with(host_data: Box<Data>, client_data: Box<Data>) -> Self {
        let host = LanNode::host(&config("Host"), &options(), launch(), host_data).unwrap();
        let port = host.local_addr().unwrap().port();
        let client = LanNode::join(&config("Client"), format!("127.0.0.1:{port}").parse().unwrap(), client_data).unwrap();
        let mut p = Self { host, client, hs: Seen::default(), cs: Seen::default() };
        p.until("join", |p| p.cs.joined && p.host.session().players.len() == 2 && p.client.session().players.len() == 2);
        p
    }
    fn frame(&mut self) {
        let (mut hg, mut cg) = (aa_netcode::lan::NoWorld, aa_netcode::lan::NoWorld);
        note(&mut self.hs, self.host.frame(&mut hg).unwrap());
        note(&mut self.cs, self.client.frame(&mut cg).unwrap());
        std::thread::sleep(Duration::from_millis(2));
    }
    fn until(&mut self, what: &str, done: impl Fn(&Self) -> bool) {
        let start = Instant::now();
        while !done(self) {
            assert!(start.elapsed() < Duration::from_secs(30), "timed out: {what} (host {:?}, client {:?})", self.hs.messages, self.cs.messages);
            self.frame();
        }
    }
    fn order_map(&mut self, crc: u32) {
        let mut game = aa_netcode::lobby::GameSettings::default();
        for k in ["north", "south"] {
            game.header.kingdoms.push(KingdomSetup { name: units(k), flag_28: true, ..Default::default() });
        }
        let fields = aa_netcode::lan::load_map_fields(&units(MAP), &game, crc, self.host.data()).unwrap();
        self.host.request(descriptor::SET_SESSION_LOAD_MAP_REQUEST, &fields).unwrap();
        self.until("the map order", |p| [&p.host, &p.client].iter().all(|n| n.session().launch.mode == 2));
    }
    fn client_status(&self) -> (Option<(u32, f32)>, Option<(u32, f32)>) {
        let me = self.client.local_player().unwrap();
        (self.host.map_status(me), self.client.map_status(me))
    }
}

fn put(dir: &Path, bytes: &[u8]) {
    let path = dir.join(MAP);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn transfer_of(len: usize) {
    let (hd, cd) = (scratch(&format!("host-{len}")), scratch(&format!("client-{len}")));
    let bytes = map_bytes(len);
    put(&hd, &bytes);
    let mut p = Pair::new(&hd, &cd);
    p.order_map(aa_netcode::file::map_checksum(&bytes));
    // the transfer runs: status 2 with a growing fraction, then 1 and 1.0
    let mut seen_downloading = Vec::new();
    let start = Instant::now();
    loop {
        assert!(start.elapsed() < Duration::from_secs(60), "the transfer did not end: {:?}", p.client_status());
        let (h, c) = p.client_status();
        if let Some((2, f)) = c {
            seen_downloading.push(f);
        }
        if h == Some((1, 1.0)) && c == Some((1, 1.0)) {
            break;
        }
        p.frame();
    }
    assert!(!seen_downloading.is_empty(), "the client saw itself downloading (0x74F310 status 2)");
    assert!(seen_downloading.windows(2).all(|w| w[0] <= w[1]), "acknowledged progress only grows: {seen_downloading:?}");
    assert!(seen_downloading.iter().all(|&f| f < 1.0));
    let received = std::fs::read(cd.join(MAP)).expect("0x7619A0 wrote profile directory + name");
    assert_eq!(received.len(), bytes.len());
    assert!(received == bytes, "the file arrived byte for byte");
    assert!(!p.hs.messages.iter().any(|m| m.starts_with("map_will_not_transfer")), "{:?}", p.hs.messages);
    // the client may now say it is ready
    let me = p.client.local_player().unwrap();
    p.client.request(descriptor::SET_READY_REQUEST, &[Field::Player(me), Field::Uint { width: 32, value: 1 }]).unwrap();
    p.until("ready", |p| p.host.session().players.iter().find(|q| q.id == me).is_some_and(|q| q.status == 1));
    eprintln!("lan transfer: {len} bytes, {} progress reports seen by the client", seen_downloading.len());
}

#[test]
fn lan_transfer_sends_a_missing_map() {
    transfer_of(6400 + 37);
}

/// A multiple of the block size: the accepted v14 image's terminal rule
/// (0x9EFF80 / 0x9EFF90) keeps the last block.
#[test]
fn lan_transfer_exact_block_multiple() {
    transfer_of(64 * 40);
}

/// The same file on both nodes: status 1 at once, nothing travels.
#[test]
fn lan_transfer_not_needed_when_the_file_matches() {
    let (hd, cd) = (scratch("same-host"), scratch("same-client"));
    let bytes = map_bytes(3000);
    put(&hd, &bytes);
    put(&cd, &bytes);
    let mut p = Pair::new(&hd, &cd);
    p.order_map(aa_netcode::file::map_checksum(&bytes));
    p.until("status 1", |p| p.client_status() == (Some((1, 1.0)), Some((1, 1.0))));
}

/// 0x770E5C: a client whose file differs and whose name has no ".." posts
/// its event and sends no status, so the host never hears of it.
#[test]
fn lan_transfer_differing_file_sends_nothing() {
    let (hd, cd) = (scratch("differ-host"), scratch("differ-client"));
    let bytes = map_bytes(3000);
    put(&hd, &bytes);
    let mut other = bytes.clone();
    other[17] ^= 1;
    put(&cd, &other);
    let mut p = Pair::new(&hd, &cd);
    p.order_map(aa_netcode::file::map_checksum(&bytes));
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        p.frame();
    }
    let (h, _) = p.client_status();
    assert_eq!(h.map(|s| s.0), Some(0), "no SetMapTransferStatus reached the host");
    assert_eq!(std::fs::read(cd.join(MAP)).unwrap(), other, "nothing was written");
}

/// A host whose front end gives no profile directory has no file channel:
/// it says the map will not transfer (its status 3 message) and the client
/// stays without the map.
#[test]
fn lan_transfer_host_without_a_channel_says_so() {
    let (hd, cd) = (scratch("nochannel-host"), scratch("nochannel-client"));
    let bytes = map_bytes(3000);
    put(&hd, &bytes);
    let mut host_data = data(&hd);
    host_data.channel = false;
    let mut p = Pair::with(host_data, data(&cd));
    p.order_map(aa_netcode::file::map_checksum(&bytes));
    p.until("the host's message", |p| p.hs.messages.iter().any(|m| m == "map_will_not_transfer_host|ss"));
    // no UpdateMapStatus followed (0x74BBB0 never stored +0x3C)
    for _ in 0..50 {
        p.frame();
    }
    let (h, c) = p.client_status();
    assert_eq!(h.map(|s| s.0), Some(0));
    assert_eq!(c.map(|s| s.0), Some(0));
    assert!(!cd.join(MAP).exists());
}

/// A host with a file channel but without the file: 0x761540 throws and no
/// frame of the order path catches it (the original's SIM thread ends in its
/// unhandled-exception filter 0x85F520); here the host's frame fails.
#[test]
fn lan_transfer_host_without_the_file_fails_its_frame() {
    let (hd, cd) = (scratch("nofile-host"), scratch("nofile-client"));
    let bytes = map_bytes(3000);
    let mut p = Pair::new(&hd, &cd);
    let mut game = aa_netcode::lobby::GameSettings::default();
    for k in ["north", "south"] {
        game.header.kingdoms.push(KingdomSetup { name: units(k), flag_28: true, ..Default::default() });
    }
    let fields = aa_netcode::lan::load_map_fields(&units(MAP), &game, aa_netcode::file::map_checksum(&bytes), p.host.data()).unwrap();
    p.host.request(descriptor::SET_SESSION_LOAD_MAP_REQUEST, &fields).unwrap();
    let start = Instant::now();
    let error = loop {
        assert!(start.elapsed() < Duration::from_secs(30), "the host's frame never failed");
        let (mut hg, mut cg) = (aa_netcode::lan::NoWorld, aa_netcode::lan::NoWorld);
        match p.host.frame(&mut hg) {
            Ok(events) => note(&mut p.hs, events),
            Err(e) => break e.to_string(),
        }
        note(&mut p.cs, p.client.frame(&mut cg).unwrap());
        std::thread::sleep(Duration::from_millis(2));
    };
    assert!(error.contains("0x761540"), "{error}");
    assert!(!cd.join(MAP).exists());
}
