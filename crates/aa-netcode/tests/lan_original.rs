//! A session with the original AA.exe, on loopback: this test hosts on
//! 127.0.0.1 and prints everything that happens, while a person or a script
//! lets the original join (`LobbyJoinDirectExecute 127.0.0.1 7290`). It needs
//! the original running, so it is ignored by default:
//!
//! ```text
//! AA_LAN_SECONDS=120 cargo test -p aa-netcode --release --offline --test lan_original -- --ignored --nocapture
//! ```
//!
//! The connect checks read `AA_LAN_VERSION`, `AA_LAN_BUILD`, `AA_LAN_CHECKSUM`,
//! `AA_LAN_DEPOT` and `AA_LAN_MODS` (what a refused connect printed).
//! How far it went is in research/phase8-net/README.txt section 13.
use aa_netcode::lan::registries::NativeRegistries;
use aa_netcode::lan::{HostOptions, LanConfig, LanData, LanEvent, LanNode, NoWorld};
use aa_netcode::lobby::{KingdomSetup, LaunchSettings};
use aa_netcode::session::PlayerId;
use aa_runtime::simulation::SpeedSettings;
use aa_simulation::FloatBits;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
fn var(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

struct Data {
    reg: NativeRegistries,
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
    fn format(&self, key: &str, args: &[Vec<u16>]) -> Vec<u16> {
        let mut out = units(key);
        for a in args {
            out.extend(units(" "));
            out.extend(a.iter());
        }
        out
    }
    fn countdown_time(&self) -> i32 {
        5
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
}

#[test]
#[ignore = "needs the original AA.exe joining on loopback"]
fn original_joins_on_loopback() {
    let root = PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_else(|_| r"<game root>".into()));
    let cache = std::env::temp_dir().join("aa-netcode-lan-registries.json");
    let reg = NativeRegistries::load_v14(&root, Some(&cache)).expect("registries");
    let name = "Rust host";
    let config = LanConfig {
        name: units(name),
        password: Vec::new(),
        version: units(&var("AA_LAN_VERSION")),
        build: units(&var("AA_LAN_BUILD")),
        data_checksum: var("AA_LAN_CHECKSUM").parse().unwrap_or(0),
        depot_checksums: units(&var("AA_LAN_DEPOT")),
        mods: units(&var("AA_LAN_MODS")),
        player_id: PlayerId { name: units(name), serial: 1, a: 0, b: 0, c: 0 },
    };
    let options = HostOptions {
        game_name: units("rust"),
        password: Vec::new(),
        max_players: 8,
        allow_observers: true,
        port: 7290,
        ports_to_try: 10,
        bind: std::net::Ipv4Addr::LOCALHOST,
    };
    // the Berlin scenario's four records
    let mut launch = LaunchSettings { mode: 1, file: units("(2v2)Berlin"), ..Default::default() };
    for k in 0..4 {
        launch.map_default.header.kingdoms.push(KingdomSetup { name: units(&format!("start_{k}")), flag_28: true, ..Default::default() });
    }
    launch.state.field_10 = 1;
    launch.state.field_18 = 4;
    launch.state.field_1c = 1;
    let mut node = LanNode::host(&config, &options, launch, Box::new(Data { reg })).expect("host");
    println!("hosting on {:?}", node.local_addr());
    let seconds: u64 = var("AA_LAN_SECONDS").parse().unwrap_or(120);
    let start = Instant::now();
    let mut world = NoWorld;
    let mut chat_at = None;
    while start.elapsed() < Duration::from_secs(seconds) {
        // a chat line once a player is in: the decree channel in the other direction
        if chat_at.is_some_and(|t: Instant| t.elapsed() > Duration::from_secs(8)) {
            chat_at = None;
            let line = aa_orders::payload::Field::Text(units("hello from the rust host"));
            let zero = || aa_orders::payload::Field::Uint { width: 32, value: 0 };
            println!("chat: {:?}", node.request(aa_netcode::lan::descriptor::PUBLIC_CHAT_REQUEST, &[zero(), zero(), line]));
        }
        match node.frame(&mut world) {
            Ok(events) => {
                for e in events {
                    if matches!(e, LanEvent::PlayersChanged) && node.session().players.len() == 2 {
                        println!("players: {:?}", node.session().players.iter().map(|p| (p.id, String::from_utf16_lossy(&p.name), p.status)).collect::<Vec<_>>());
                        if chat_at.is_none() {
                            chat_at = Some(Instant::now());
                        }
                    }
                    println!("[{:7.2}] {e:?}", start.elapsed().as_secs_f32());
                    if let LanEvent::ConnectRefused { reason, info } = &e {
                        println!(
                            "refused, reason {reason}: name {:?} version {:?} build {:?} checksum {} depot {:?} mods {:?}",
                            String::from_utf16_lossy(&info.name),
                            String::from_utf16_lossy(&info.version),
                            String::from_utf16_lossy(&info.build),
                            info.data_checksum,
                            String::from_utf16_lossy(&info.depot_checksums),
                            String::from_utf16_lossy(&info.mods)
                        );
                    }
                }
            }
            Err(e) => {
                println!("[{:7.2}] the frame failed: {e}", start.elapsed().as_secs_f32());
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let s = node.session();
    println!("players at the end: {:?}", s.players.iter().map(|p| (p.id, String::from_utf16_lossy(&p.name), p.status)).collect::<Vec<_>>());
}
