//! Two processes play one aa-play battle over a LAN session on 127.0.0.1:
//! this test is the host, it starts its own binary again as the client
//! (`lan_battle_client`). The host stages Berlin with both humans and a
//! computer player (TGC_CreateExplicitAIOrder 132/143), the map seed
//! (TGC_SetMapSeedOrder 90/92), the countdown and the launch; both build the
//! same world from their replicated session, report their task (304/315),
//! and then run in lockstep through the original's frame driver: every
//! order of either player travels as the original's requests and decrees
//! (selection 332/333, KKC_TellSelectedCommandOrder 84/86) and is applied on
//! both nodes at the same world tick. The child prints a digest of every
//! world tick (aa-play `trace::digest`); the host compares them with its own.
//!
//! A second run makes the client draw one extra world-RNG value at tick 120
//! (AA_LAN_PERTURB). The getter GWorld::GetRNG 0x4F02B0 logs that draw to
//! synchronizer A, so the client's hash leaves the host's at once, and the
//! original sync check (302 -> 0x751D00 -> 303) must report the client out of
//! sync on both nodes. Changing the RNG state alone would prove nothing here:
//! A only sees the state when something draws, and this opening draws once in
//! 340 ticks, before tick 120.
//!
//! Needs the original data (AA_GAME_ROOT, default <game root>);
//! skipped without it.
use aa_netcode::lan::registries::{self, NativeRegistries};
use aa_netcode::lan::{descriptor, HostOptions, LanConfig, LanData, LanEvent, LanGame, LanNode};
use aa_netcode::lobby::{KingdomSetup, LaunchSettings, Session};
use aa_netcode::session::PlayerId;
use aa_orders::payload::{Field, ThingCheck};
use aa_play::player_orders::native::{self, Outbox, Replay, Tables};
use aa_play::{Command, Controller, GameData, GameMap, KingdomId, PlayerSetup, SkirmishSetup, Unit, World};
use aa_runtime::simulation::SpeedSettings;
use aa_simulation::{FloatBits, NativeClock};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

const TICKS: u32 = 300;
const MAP: &str = "(2v2)Berlin";
const SEED: u32 = 20_261_025;

fn root() -> PathBuf {
    PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_else(|_| r"<game root>".into()))
}
fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
fn have_data() -> bool {
    root().join("Data.rwd").exists()
}

fn registries() -> NativeRegistries {
    static R: OnceLock<NativeRegistries> = OnceLock::new();
    R.get_or_init(|| {
        let cache = std::env::temp_dir().join("aa-netcode-lan-registries.json");
        NativeRegistries::load_v14(&root(), Some(&cache)).expect("native registries")
    })
    .clone()
}
fn game_data() -> Arc<GameData> {
    static D: OnceLock<Arc<GameData>> = OnceLock::new();
    D.get_or_init(|| Arc::new(GameData::load(&root()).expect("GameData::load"))).clone()
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
    fn format(&self, _key: &str, args: &[Vec<u16>]) -> Vec<u16> {
        let mut out = args[0].clone();
        out.extend(units(" ("));
        out.extend(args[1].iter());
        out.extend(units(")"));
        out
    }
    fn countdown_time(&self) -> i32 {
        2
    }
    fn nation_factions(&self, _nation: &[u16]) -> aa_netcode::Result<Vec<Vec<u16>>> {
        panic!("no kingdom order in this battle reads a nation's factions")
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

fn config(name: &str) -> LanConfig {
    LanConfig {
        name: units(name),
        password: Vec::new(),
        version: units("1.0.5.0"),
        build: units("AA"),
        data_checksum: 0,
        depot_checksums: Vec::new(),
        mods: Vec::new(),
        player_id: PlayerId { name: units(name), serial: 1, a: 0, b: 0, c: 0 },
    }
}

/// Berlin's four start locations as the kingdom records of the default creator.
fn launch() -> LaunchSettings {
    let mut ls = LaunchSettings { mode: 1, file: units(MAP), ..Default::default() };
    for k in 0..4 {
        ls.map_default.header.kingdoms.push(KingdomSetup { name: units(&format!("start_{k}")), flag_28: true, ..Default::default() });
    }
    ls.state.field_10 = 1;
    ls.state.field_18 = 4;
    ls.state.field_1c = 1;
    ls
}

/// The battle of a launched session: the records with a player, in record
/// order (nation by record: rus, ger, usa, gbr; teams 0/1), the creator's seed
/// (0x776B20 put it there during the launch).
fn setup_of(session: &Session) -> (SkirmishSetup, BTreeMap<u32, KingdomId>) {
    let nations = ["rus", "ger", "usa", "gbr"];
    let mut players = Vec::new();
    let mut kingdoms = BTreeMap::new();
    for (i, k) in session.launch.active_map().header.kingdoms.iter().enumerate() {
        let Some(p) = session.players.iter().find(|p| p.kingdom == k.name) else { continue };
        kingdoms.insert(p.id, players.len() as KingdomId);
        players.push(PlayerSetup {
            name: String::from_utf16_lossy(&p.name),
            nation: nations[i].into(),
            team: (i % 2) as u32,
            color: [255, 255, 255],
            controller: if p.ai { Controller::Computer(1) } else { Controller::Human },
            start: i,
        });
    }
    (SkirmishSetup { map_id: MAP.into(), players, starting_resources: None, seed: u64::from(session.launch.active_map().field_4c) }, kingdoms)
}

/// The aa-play side of the session (what the client's net/ module does).
struct Play {
    world: World,
    tables: Tables,
    replay: Replay,
    kingdoms: BTreeMap<u32, KingdomId>,
    pending: Vec<Command>,
    digests: Vec<(u32, u64)>,
    refused: usize,
    perturb_at: Option<u32>,
    /// World-RNG getter values (GWorld::GetRNG 0x4F02B0) handed to A.
    draws: usize,
}
impl LanGame for Play {
    fn decree(&mut self, player: Option<u32>, d: usize, fields: &[Field]) -> aa_netcode::Result<()> {
        let Some(player) = player else { return Ok(()) };
        let kingdom = self.kingdoms.get(&player).copied();
        match native::commands(&self.world, &self.tables, &mut self.replay, kingdom, player, d, fields) {
            Ok(c) => self.pending.extend(c),
            Err(_) => self.refused += 1,
        }
        Ok(())
    }
    fn thing(&self, _check: ThingCheck, slot: u16) -> Option<u16> {
        aa_play::ids::unit(self.world.ids.thing_at_slot(slot)?).map(|_| slot)
    }
    fn world_clock(&self) -> Option<NativeClock> {
        Some(self.world.clock)
    }
    fn world_update(&mut self) -> aa_netcode::Result<(NativeClock, Vec<u32>)> {
        self.world.trace.record(true);
        if self.perturb_at == Some(self.world.clock.tick) {
            self.world.random_u32();
        }
        let commands = std::mem::take(&mut self.pending);
        self.world.step(commands);
        let draws: Vec<u32> = self
            .world
            .trace
            .entries()
            .filter(|e| e.1 == aa_play::trace::SKind::RngGetter)
            .map(|e| e.2)
            .collect();
        if let Some(log) = self.world.trace.log.as_mut() {
            log.clear();
        }
        self.world.trace.digests.clear();
        let d = aa_play::trace::digest(&self.world);
        self.digests.push((d.tick, d.world ^ u64::from(d.rng_state)));
        self.draws += draws.len();
        Ok((self.world.clock, draws))
    }
}

fn tables() -> Tables {
    let r = registries();
    let slots = |reg: aa_orders::payload::Registry| r.slots.get(&reg.0).cloned().unwrap_or_default();
    Tables {
        objects: slots(registries::OBJECT_DEFINITIONS),
        formations: slots(registries::FORMATIONS),
        properties: slots(registries::SHARED_PROPERTIES),
        property_count: r.count(registries::SHARED_PROPERTIES).unwrap_or(0),
        abilities: slots(registries::ABILITIES),
        ability_count: r.count(registries::ABILITIES).unwrap_or(0),
        organization_layout_count: r.count(registries::ORGANIZATION_LAYOUTS).unwrap_or(0),
    }
}

/// The scripted orders: every 48 world ticks one player moves the first
/// living organization of its kingdom.
fn order(play: &Play, kingdom: KingdomId, tick: u32) -> Option<Command> {
    let w = &play.world;
    // an organization of the kingdom, else its first actor that is not a
    // structure (Berlin starts each side with an HQ and a truck)
    let (unit, mut to) = match w.orgs.iter().find(|o| o.alive && o.kingdom == kingdom) {
        Some(o) => (Unit::Org(o.id), o.anchor),
        None => {
            let (i, a) = w
                .actors
                .iter()
                .enumerate()
                .find(|(_, a)| a.alive && a.kingdom == Some(kingdom) && a.org.is_none() && !w.data.def(a.def).category.eq_ignore_ascii_case("BUILDING"))?;
            (Unit::Actor(i as aa_play::ActorId), a.pos.truncate())
        }
    };
    to.x += 12.0 + (tick % 5) as f32 * 3.0;
    to.y -= 8.0;
    Some(Command::Move { kingdom, units: vec![unit], to, attack_move: false })
}

struct Outcome {
    play: Play,
    sync_failure: Option<u32>,
    orders: usize,
    local: Option<u32>,
}

fn run(node: &mut LanNode, host: bool, perturb: bool, until: impl Fn(&Play) -> bool) -> Outcome {
    let mut play: Option<Play> = None;
    let mut outbox = Outbox::default();
    let mut sync_failure = None;
    let mut ready_sent = false;
    let mut launched = false;
    let mut begun = false;
    let mut orders = 0;
    let mut last_order_tick = 0;
    let start = Instant::now();
    let mut lobby = aa_netcode::lan::NoWorld;
    while start.elapsed() < Duration::from_secs(150) {
        let events = match play.as_mut() {
            Some(p) => node.frame(p).expect("frame"),
            None => node.frame(&mut lobby).expect("frame"),
        };
        for e in events {
            match e {
                LanEvent::LoadWorld => {
                    let (setup, kingdoms) = setup_of(node.session());
                    let data = game_data();
                    let map = Arc::new(GameMap::load_with(&root(), MAP, &data).expect("map"));
                    let mut world = World::new_skirmish(data, map, &setup).expect("new_skirmish");
                    aa_play::ai::attach(&mut world, Arc::new(aa_play::ai::AiData::load(&root()).expect("SAI data")));
                    play = Some(Play {
                        world,
                        tables: tables(),
                        replay: Replay::default(),
                        kingdoms,
                        pending: Vec::new(),
                        digests: Vec::new(),
                        refused: 0,
                        perturb_at: perturb.then_some(120),
                        draws: 0,
                    });
                    node.report_world_loaded(true).expect("task status");
                }
                LanEvent::BeginPlay => begun = true,
                LanEvent::SyncFailure { player } => sync_failure = Some(player),
                _ => {}
            }
        }
        // staging
        if !host && node.joined() && !ready_sent {
            let me = node.local_player().unwrap();
            ready_sent = node.request(descriptor::SET_READY_REQUEST, &[Field::Player(me), Field::Uint { width: 32, value: 1 }]).unwrap();
        }
        if host && !launched && node.session().players.len() == 2 && node.session().players.iter().all(|p| p.status == 1 || p.status == 2) {
            // a computer player on the third start, the map seed, the launch
            node.request(descriptor::CREATE_EXPLICIT_AI_REQUEST, &[Field::Text(units("start_2")), Field::Text(Vec::new()), Field::Text(units("Computer"))]).unwrap();
            node.request(descriptor::SET_MAP_SEED_REQUEST, &[Field::Uint { width: 32, value: SEED }]).unwrap();
            node.start_launch().unwrap();
            launched = true;
        }
        // play: the scripted orders of the local player
        if begun
            && let Some(p) = play.as_ref()
        {
            let tick = p.world.clock.tick;
            if tick >= last_order_tick + 48 && tick > 16 {
                let me = node.local_player().unwrap();
                if let Some(&k) = p.kingdoms.get(&me) {
                    let offset = if host { 0 } else { 24 };
                    if (tick + offset) % 48 < 8 {
                        if let Some(c) = order(p, k, tick) {
                            for (d, f) in native::requests(&p.world, &p.tables, &mut outbox, &c).expect("requests") {
                                node.request(d, &f).expect("request");
                            }
                            orders += 1;
                            last_order_tick = tick;
                        }
                    }
                }
            }
            if until(p) {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Outcome { play: play.expect("the battle was launched"), sync_failure, orders, local: node.local_player() }
}

/// The client process (started by the host test).
#[test]
fn lan_battle_client() {
    let Ok(port) = std::env::var("AA_LAN_PORT") else { return };
    let perturb = std::env::var_os("AA_LAN_PERTURB").is_some();
    let data = Box::new(Data { reg: registries() });
    let mut node = LanNode::join(&config("Client"), format!("127.0.0.1:{port}").parse().unwrap(), data).unwrap();
    let out = run(&mut node, false, perturb, |p| p.world.clock.tick >= TICKS + 40);
    for (t, d) in &out.play.digests {
        println!("DIGEST {t} {d:016x}");
    }
    println!("ORDERS {} REFUSED {} DRAWS {} SYNCFAIL {:?} LOCAL {:?}", out.orders, out.play.refused, out.play.draws, out.sync_failure, out.local);
}

fn host_run(perturb: bool) -> Option<(Outcome, Vec<(u32, u64)>, String)> {
    if !have_data() {
        eprintln!("lan_battle: no original data at {}; skipped", root().display());
        return None;
    }
    // Warm the caches before the client starts.
    registries();
    game_data();
    let options = HostOptions {
        game_name: units("lan battle"),
        password: Vec::new(),
        max_players: 4,
        allow_observers: false,
        port: 0,
        ports_to_try: 1,
        bind: std::net::Ipv4Addr::LOCALHOST,
    };
    let data = Box::new(Data { reg: registries() });
    let mut node = LanNode::host(&config("Host"), &options, launch(), data).unwrap();
    let port = node.local_addr().unwrap().port();
    let exe = std::env::current_exe().unwrap();
    let mut cmd = std::process::Command::new(exe);
    cmd.args(["--exact", "lan_battle_client", "--nocapture", "--test-threads", "1"]).env("AA_LAN_PORT", port.to_string()).stdout(std::process::Stdio::piped());
    if perturb {
        cmd.env("AA_LAN_PERTURB", "1");
    }
    let mut child = cmd.spawn().unwrap();
    let stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || BufReader::new(stdout).lines().map_while(Result::ok).collect::<Vec<String>>());
    let mut out = run(&mut node, true, false, |p| p.world.clock.tick >= TICKS + 80);
    // Keep serving the client until it is done.
    let until = Instant::now() + Duration::from_secs(20);
    while child.try_wait().unwrap().is_none() && Instant::now() < until {
        node.frame(&mut out.play).expect("frame");
        std::thread::sleep(Duration::from_millis(2));
    }
    let _ = child.kill();
    let lines = reader.join().unwrap();
    let mut theirs = Vec::new();
    let mut summary = String::new();
    for l in lines {
        // (the harness prints "test lan_battle_client ... " before the first line)
        if let Some(rest) = l.find("DIGEST ").map(|i| &l[i + 7..]) {
            let mut it = rest.split(' ');
            let t: u32 = it.next().unwrap().parse().unwrap();
            let d = u64::from_str_radix(it.next().unwrap(), 16).unwrap();
            theirs.push((t, d));
        } else if let Some(i) = l.find("ORDERS ") {
            summary = l[i..].to_string();
        }
    }
    Some((out, theirs, summary))
}

#[test]
fn lan_battle_two_processes_in_lockstep() {
    let Some((out, theirs, summary)) = host_run(false) else { return };
    let host = &out.play;
    eprintln!("host: {} ticks, {} orders, {} refused, {} RNG draws; client: {summary}", host.digests.len(), out.orders, host.refused, host.draws);
    assert_eq!(host.refused, 0, "a decree of the scripted orders was refused");
    assert!(summary.contains("REFUSED 0"), "the client refused a decree: {summary}");
    assert!(host.digests.len() as u32 >= TICKS, "host ran {} world ticks", host.digests.len());
    assert!(theirs.len() as u32 >= TICKS, "client ran {} world ticks ({summary})", theirs.len());
    let n = host.digests.len().min(theirs.len());
    for i in 0..n {
        assert_eq!(host.digests[i], theirs[i], "world tick {} differs between the processes", host.digests[i].0);
    }
    assert!(out.orders >= 3, "host issued {} orders", out.orders);
    assert!(summary.contains("SYNCFAIL None"), "client saw a sync failure: {summary}");
    assert_eq!(out.sync_failure, None, "host reported a sync failure without a desync");
    eprintln!("lan battle: {n} world ticks identical in both processes");
}

#[test]
fn lan_battle_detects_a_desynchronized_client() {
    let Some((out, theirs, summary)) = host_run(true) else { return };
    // The client drew once more at tick 120: the worlds differ ...
    let diverged = out.play.digests.iter().zip(&theirs).position(|(a, b)| a != b);
    assert!(diverged.is_some(), "the perturbed client still matches ({summary})");
    // ... and the original sync check named the client on both nodes.
    let client = summary.split("LOCAL Some(").nth(1).and_then(|s| s.split(')').next()).and_then(|s| s.parse::<u32>().ok());
    assert!(out.sync_failure.is_some(), "the host did not detect the desync ({summary})");
    assert_eq!(out.sync_failure, client, "the host named the client");
    assert!(summary.contains(&format!("SYNCFAIL {:?}", out.sync_failure)), "the client executed the SyncFailure decree: {summary}");
}
