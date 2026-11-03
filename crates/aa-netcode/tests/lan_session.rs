//! A LAN session of two nodes on 127.0.0.1 (one process, real UDP sockets):
//! host 0x75AEE0, join (Connect 0x750330 / 0x75BF90, PlayerJoin 306/317,
//! private join messages 0x753580 -> 0x7531A0), ready (99/103), the launch
//! countdown (96/97, 326/330) and launch (0/5), the launch task status
//! (304/315) and begin play (0x775BF0), gameplay decrees executed on both
//! nodes at the same world tick, and the original sync check (302/314 every
//! 17th admin tick, 0x751D00 -> SyncFailure 303) on a perturbed client.
//!
//! Two more sessions check the staging orders on both nodes: SetPlayerKingdom
//! with its release branch and rejections (0x778170 / 0x777F00), CreateExplicitAI
//! 0x754F50 and the cleanup 0x7744C0, kingdom nation and color (0x777D90 /
//! 0x771C70), the launch seed (0x776B20), the not-ready report 0x7725A0, the
//! refusal of an order without a process, and PlayerLeave 0x755D70.
//!
//! The world is a stand-in (`Fake`): a native clock and a world-RNG getter
//! value per tick; the aa-play battle runs in tests/lan_battle.rs. None of this
//! is compared with the original: only a real two-node session of this port.
use aa_netcode::lan::registries::{self, NativeRegistries};
use aa_netcode::lan::{descriptor, HostOptions, LanConfig, LanData, LanEvent, LanGame, LanNode};
use aa_netcode::lobby::{KingdomSetup, LaunchSettings};
use aa_netcode::session::PlayerId;
use aa_orders::payload::{Field, ThingCheck};
use aa_runtime::simulation::SpeedSettings;
use aa_simulation::{FloatBits, NativeClock};
use std::time::{Duration, Instant};

fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
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
    fn nation_factions(&self, nation: &[u16]) -> aa_netcode::Result<Vec<Vec<u16>>> {
        assert_eq!(nation, units("nation_b"));
        Ok(vec![units("faction_a")])
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

fn data() -> Box<Data> {
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
    reg.slots.insert(registries::NATIONS.0, (0..6).map(|i| ["nation_a", "nation_b", "nation_c", "nation_d", "nation_e", "nation_f"][i].to_string()).collect());
    reg.slots.insert(registries::FACTIONS.0, (0..12).map(|i| format!("faction_{}", ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"][i])).collect());
    Box::new(Data { reg })
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

/// Two playable kingdom records on the default creator (map type 1).
fn launch() -> LaunchSettings {
    let mut ls = LaunchSettings { mode: 1, ..Default::default() };
    for k in ["kingdom_a", "kingdom_b", "kingdom_c"] {
        ls.map_default.header.kingdoms.push(KingdomSetup { name: units(k), flag_28: true, ..Default::default() });
        // Session+0x68: the header 0x777CD0 / 0x777610 check against.
        ls.game.header.kingdoms.push(KingdomSetup { name: units(k), flag_28: true, ..Default::default() });
    }
    ls.state.field_10 = 1;
    ls.state.field_18 = 4;
    ls.state.field_1c = 1;
    ls
}

/// The stand-in world.
#[derive(Default)]
struct Fake {
    clock: Option<NativeClock>,
    /// (world tick, ordering player, descriptor, fields) of every gameplay decree.
    decrees: Vec<(u32, Option<u32>, usize, Vec<Field>)>,
    applied_ticks: u32,
}
impl LanGame for Fake {
    fn decree(&mut self, player: Option<u32>, descriptor: usize, fields: &[Field]) -> aa_netcode::Result<()> {
        let tick = self.clock.map_or(0, |c| c.tick);
        self.decrees.push((tick, player, descriptor, fields.to_vec()));
        Ok(())
    }
    fn thing(&self, _check: ThingCheck, slot: u16) -> Option<u16> {
        (slot != 0).then_some(slot)
    }
    fn world_clock(&self) -> Option<NativeClock> {
        self.clock
    }
    fn world_update(&mut self) -> aa_netcode::Result<(NativeClock, Vec<u32>)> {
        let c = self.clock.as_mut().expect("world");
        c.advance_fragment();
        self.applied_ticks += 1;
        // One world-RNG draw per tick, depending on the decrees so far.
        let draw = c.tick.wrapping_mul(2_654_435_761) ^ (self.decrees.len() as u32);
        Ok((*c, vec![draw]))
    }
}

fn move_command(x: f32, y: f32) -> Vec<Field> {
    // 0x61EBA0: mask bit6 command (move, ActorCommand 5) and bit2 position.
    vec![
        Field::Bounded { n: 0x800, value: (1 << 6) | (1 << 2) },
        Field::Def { registry: registries::ACTOR_COMMANDS, index: 5 },
        Field::Uint { width: 32, value: x.to_bits() },
        Field::Uint { width: 32, value: y.to_bits() },
        Field::Uint { width: 1, value: 0 },
    ]
}

fn options() -> HostOptions {
    HostOptions {
        game_name: units("test"),
        password: Vec::new(),
        max_players: 4,
        allow_observers: false,
        port: 0,
        ports_to_try: 1,
        bind: std::net::Ipv4Addr::LOCALHOST,
    }
}

#[derive(Default)]
struct Seen {
    joined: bool,
    load: bool,
    begin: bool,
    sync_failure: Option<u32>,
    messages: Vec<String>,
    disconnected: Option<Option<u32>>,
}
fn note(seen: &mut Seen, events: Vec<LanEvent>) {
    for e in events {
        match e {
            LanEvent::Joined { .. } => seen.joined = true,
            LanEvent::LoadWorld => seen.load = true,
            LanEvent::BeginPlay => seen.begin = true,
            LanEvent::SyncFailure { player } => seen.sync_failure = Some(player),
            LanEvent::Message { key, .. } => seen.messages.push(key),
            LanEvent::Disconnected { reason } => seen.disconnected = Some(reason),
            _ => {}
        }
    }
}

#[test]
fn lan_session_staging_launch_lockstep_and_sync_check() {
    let mut host = LanNode::host(&config("Host"), &options(), launch(), data()).unwrap();
    let port = host.local_addr().unwrap().port();
    let mut client = LanNode::join(&config("Client"), format!("127.0.0.1:{port}").parse().unwrap(), data()).unwrap();
    let (mut hg, mut cg) = (Fake::default(), Fake::default());
    let (mut hs, mut cs) = (Seen::default(), Seen::default());
    let start = Instant::now();
    let mut phase = 0;
    let mut sent_orders = 0;
    let mut perturbed_at = None;
    while start.elapsed() < Duration::from_secs(40) {
        note(&mut hs, host.frame(&mut hg).unwrap());
        note(&mut cs, client.frame(&mut cg).unwrap());
        match phase {
            // joined: both sessions hold both players with kingdoms
            0 if cs.joined && host.session().players.len() == 2 && client.session().players.len() == 2 => {
                let kingdoms: Vec<Vec<u16>> = host.session().players.iter().map(|p| p.kingdom.clone()).collect();
                assert_eq!(kingdoms, vec![units("kingdom_a"), units("kingdom_b")], "0x778D10 auto-assigns the free records in order");
                let ck: Vec<Vec<u16>> = client.session().players.iter().map(|p| p.kingdom.clone()).collect();
                assert_eq!(ck, kingdoms);
                let me = client.local_player().unwrap();
                client.request(descriptor::SET_READY_REQUEST, &[Field::Player(me), Field::Uint { width: 32, value: 1 }]).unwrap();
                phase = 1;
            }
            // the client is ready (and its map status known): launch
            1 if host.session().players.iter().all(|p| p.status == 1 || p.status == 2) => {
                host.start_launch().unwrap();
                phase = 2;
            }
            // the launch task: both build their world and report it
            2 if hs.load && cs.load => {
                assert_eq!(host.session().state, 2);
                assert_eq!(client.session().state, 2);
                hg.clock = Some(NativeClock::stock());
                cg.clock = Some(NativeClock::stock());
                host.report_world_loaded(true).unwrap();
                client.report_world_loaded(true).unwrap();
                phase = 3;
            }
            3 if hs.begin && cs.begin => phase = 4,
            // play: orders from both players
            4 => {
                if hg.applied_ticks > 10 && sent_orders < 6 && hg.applied_ticks % 7 == 0 {
                    let (node, x) = if sent_orders % 2 == 0 { (&mut host, 10.0) } else { (&mut client, 20.0) };
                    if node.request(descriptor::TELL_SELECTED_REQUEST, &move_command(x + sent_orders as f32, 5.0)).unwrap() {
                        sent_orders += 1;
                    }
                }
                if sent_orders == 6 && hg.applied_ticks > 120 && cg.applied_ticks > 120 {
                    phase = 5;
                }
            }
            // no desync so far; now a client whose admin log differs
            5 => {
                assert_eq!(hs.sync_failure, None, "no sync failure before the perturbation: {:?}", hs.messages);
                assert_eq!(cs.sync_failure, None);
                client.perturb_sync_a(7);
                perturbed_at = Some(Instant::now());
                phase = 6;
            }
            6 if hs.sync_failure.is_some() && cs.sync_failure.is_some() => break,
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(phase, 6, "session reached phase {phase} (host {:?}, client {:?})", hs.messages, cs.messages);
    let me = client.local_player().unwrap();
    assert_eq!(hs.sync_failure, Some(me), "the host's 0x751D00 named the client");
    assert_eq!(cs.sync_failure, Some(me), "the client executed the SyncFailure decree");
    assert!(perturbed_at.unwrap().elapsed() < Duration::from_secs(10));
    // Lockstep: every gameplay decree at the same world tick on both nodes.
    let moves: Vec<_> = hg.decrees.iter().filter(|d| d.2 == descriptor::TELL_SELECTED_DECREE).collect();
    assert_eq!(moves.len(), 6, "{:?}", hg.decrees);
    let n = hg.decrees.len().min(cg.decrees.len());
    assert!(n >= 6);
    assert_eq!(&hg.decrees[..n], &cg.decrees[..n], "same gameplay decrees, players, fields and world ticks");
    // Both ordering players appear.
    let players: std::collections::BTreeSet<_> = moves.iter().map(|d| d.1).collect();
    assert_eq!(players.len(), 2);
    eprintln!(
        "lan session: {} world ticks on the host, {} on the client, {} gameplay decrees, sync failure of player {me} detected",
        hg.applied_ticks,
        cg.applied_ticks,
        hg.decrees.len()
    );
}

/// Run both nodes until `done`, or fail with `what`.
fn pump(host: &mut LanNode, client: &mut LanNode, hs: &mut Seen, cs: &mut Seen, what: &str, done: impl Fn(&LanNode, &LanNode, &Seen, &Seen) -> bool) {
    let (mut hg, mut cg) = (aa_netcode::lan::NoWorld, aa_netcode::lan::NoWorld);
    let start = Instant::now();
    while !done(host, client, hs, cs) {
        assert!(start.elapsed() < Duration::from_secs(20), "timed out: {what} (host {:?}, client {:?})", hs.messages, cs.messages);
        note(hs, host.frame(&mut hg).unwrap());
        note(cs, client.frame(&mut cg).unwrap());
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn joined(host: &mut LanNode, client: &mut LanNode, hs: &mut Seen, cs: &mut Seen) {
    pump(host, client, hs, cs, "join", |h, c, _, cs| cs.joined && h.session().players.len() == 2 && c.session().players.len() == 2);
}

fn kingdoms(node: &LanNode) -> Vec<Vec<u16>> {
    node.session().players.iter().map(|p| p.kingdom.clone()).collect()
}

fn record(node: &LanNode, kingdom: &str) -> KingdomSetup {
    node.session().launch.active_map().header.kingdoms.iter().find(|k| k.name == units(kingdom)).unwrap().clone()
}

#[test]
fn lan_session_staging_orders_on_both_nodes() {
    let mut host = LanNode::host(&config("Host"), &options(), launch(), data()).unwrap();
    assert_eq!(host.session().options.field_d8, u32::from(host.local_addr().unwrap().port()), "0x75B11D: +0xD8 holds the bound port");
    let port = host.local_addr().unwrap().port();
    let mut client = LanNode::join(&config("Client"), format!("127.0.0.1:{port}").parse().unwrap(), data()).unwrap();
    let (mut hs, mut cs) = (Seen::default(), Seen::default());
    joined(&mut host, &mut client, &mut hs, &mut cs);
    assert_eq!(client.session().options.field_d8, u32::from(port), "the snapshot carries +0xD8");
    let (me, boss) = (client.local_player().unwrap(), host.local_player().unwrap());

    // 0x778170: a client moves itself; the record it leaves is released.
    client.request(descriptor::SET_PLAYER_KINGDOM_REQUEST, &[Field::Player(me), Field::Text(units("kingdom_c"))]).unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "client moves to kingdom_c", |h, c, _, _| {
        h.session().players[1].kingdom == units("kingdom_c") && c.session().players[1].kingdom == units("kingdom_c")
    });
    // ... but not another player: the key of 0x7781C5, on the ordering node only.
    client.request(descriptor::SET_PLAYER_KINGDOM_REQUEST, &[Field::Player(boss), Field::Text(units("kingdom_b"))]).unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "rejection", |_, _, _, cs| cs.messages.iter().any(|m| m == "set_player_kingdom_failed"));
    assert!(!hs.messages.iter().any(|m| m == "set_player_kingdom_failed"), "0x76DA60 shows only the ordering player");
    assert_eq!(kingdoms(&host)[0], units("kingdom_a"));

    // 0x754F50 on both nodes; 0x765EA0 keeps an AI at status 2.
    host.request(descriptor::CREATE_EXPLICIT_AI_REQUEST, &[Field::Text(units("kingdom_b")), Field::Text(Vec::new()), Field::Text(units("Computer"))]).unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "AI created", |h, c, _, _| h.session().players.len() == 3 && c.session().players.len() == 3);
    assert_eq!(kingdoms(&host), kingdoms(&client));
    let ai = &host.session().players[2];
    assert!(ai.ai && ai.status == 2 && ai.kingdom == units("kingdom_b") && ai.name == units("Computer"));

    // 0x7744C0: the AI that lost its kingdom is destroyed.
    host.request(descriptor::SET_PLAYER_KINGDOM_REQUEST, &[Field::Player(host.session().players[2].id), Field::Text(Vec::new())]).unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "kingdomless AI destroyed", |h, c, _, _| h.session().players.len() == 2 && c.session().players.len() == 2);

    // 0x771C70: the host may recolor; a client may not touch the host's kingdom.
    let color = |index| Field::Def { registry: registries::KINGDOM_COLORS, index };
    host.request(descriptor::SET_KINGDOM_COLOR_REQUEST, &[Field::Text(units("kingdom_a")), color(2)]).unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "color", |h, c, _, _| record(h, "kingdom_a").color == Some(2) && record(c, "kingdom_a").color == Some(2));
    client.request(descriptor::SET_KINGDOM_COLOR_REQUEST, &[Field::Text(units("kingdom_a")), color(3)]).unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "color rejection", |_, _, _, cs| cs.messages.iter().any(|m| m == "set_kingdom_color_failed"));
    assert_eq!(record(&host, "kingdom_a").color, Some(2));

    // 0x777D90: the nation goes to the record as its registry index and to the
    // owner's +0x30 as IDS, on both nodes.
    let nation = Field::Def { registry: registries::NATIONS, index: 1 };
    client.request(descriptor::SET_KINGDOM_NATION_REQUEST, &[Field::Text(units("kingdom_c")), nation]).unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "nation", |h, c, _, _| record(h, "kingdom_c").nation == Some(1) && record(c, "kingdom_c").nation == Some(1));
    assert_eq!(host.session().players[1].text_30, units("nation_b"));
    assert_eq!(client.session().players[1].text_30, units("nation_b"));

    // Orders without a process are refused by name, before the stream.
    // (the pause has one since 0x754880 was ported: lan_session_pause_vote)
    for d in [59usize, 60] {
        match client.request(d, &[Field::Uint { width: 32, value: 0 }, Field::Uint { width: 32, value: 0 }, Field::Text(Vec::new())]) {
            Err(aa_netcode::Error::Unsupported(name)) => assert!(name.contains("Chat"), "{name}"),
            other => panic!("descriptor {d}: {other:?}"),
        }
    }
}

#[test]
fn lan_session_not_ready_report_and_leave() {
    let mut host = LanNode::host(&config("Host"), &options(), launch(), data()).unwrap();
    let port = host.local_addr().unwrap().port();
    let mut client = LanNode::join(&config("Client"), format!("127.0.0.1:{port}").parse().unwrap(), data()).unwrap();
    let (mut hs, mut cs) = (Seen::default(), Seen::default());
    joined(&mut host, &mut client, &mut hs, &mut cs);
    let me = client.local_player().unwrap();

    // The client is not ready: the launch request reports instead of counting.
    host.start_launch().unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "not-ready report", |_, _, hs, cs| {
        hs.messages.iter().any(|m| m == "host_requested_players_ready") && cs.messages.iter().any(|m| m == "host_requested_players_ready_client_is_ready")
    });
    assert_eq!(host.session().host_countdown, 0, "no countdown started");

    // PlayerLeave with reason 4: ClientLeave on the host, type 11 to the client.
    host.request(descriptor::PLAYER_LEAVE_REQUEST, &[Field::Player(me), Field::Bounded { n: 13, value: 4 }]).unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "leave", |h, _, _, cs| h.session().players.len() == 1 && cs.disconnected == Some(Some(4)));
    assert!(hs.messages.iter().any(|m| m == "networking_player_kicked|s"));
}

#[test]
fn lan_session_launch_seed_is_the_same_on_both_nodes() {
    // With the map seed order (0x7710E0), and without it (one draw of the
    // session stream, whose state the joiner got in the snapshot).
    for seed in [Some(4242u32), None] {
        let mut host = LanNode::host(&config("Host"), &options(), launch(), data()).unwrap();
        let port = host.local_addr().unwrap().port();
        let mut client = LanNode::join(&config("Client"), format!("127.0.0.1:{port}").parse().unwrap(), data()).unwrap();
        let (mut hs, mut cs) = (Seen::default(), Seen::default());
        joined(&mut host, &mut client, &mut hs, &mut cs);
        let me = client.local_player().unwrap();
        client.request(descriptor::SET_READY_REQUEST, &[Field::Player(me), Field::Uint { width: 32, value: 1 }]).unwrap();
        pump(&mut host, &mut client, &mut hs, &mut cs, "ready", |h, _, _, _| h.session().players[1].status == 1);
        if let Some(seed) = seed {
            host.request(descriptor::SET_MAP_SEED_REQUEST, &[Field::Uint { width: 32, value: seed }]).unwrap();
        }
        host.start_launch().unwrap();
        pump(&mut host, &mut client, &mut hs, &mut cs, "launch", |_, _, hs, cs| hs.load && cs.load);
        let (h, c) = (host.session().launch.active_map().field_4c, client.session().launch.active_map().field_4c);
        assert_eq!(h, c, "0x776B20 gives both nodes one seed");
        match seed {
            Some(seed) => assert_eq!(h, seed),
            None => assert_ne!(h, 0),
        }
    }
}

/// TGC_SetSessionLoadMapOrder 0x779780 and the staging setters: the same stage
/// on both nodes, the people seated again on the new records.
#[test]
fn lan_session_map_order_on_both_nodes() {
    let mut host = LanNode::host(&config("Host"), &options(), launch(), data()).unwrap();
    let port = host.local_addr().unwrap().port();
    let mut client = LanNode::join(&config("Client"), format!("127.0.0.1:{port}").parse().unwrap(), data()).unwrap();
    let (mut hs, mut cs) = (Seen::default(), Seen::default());
    joined(&mut host, &mut client, &mut hs, &mut cs);

    let mut game = aa_netcode::lobby::GameSettings::default();
    for k in ["north", "south"] {
        game.header.kingdoms.push(KingdomSetup { name: units(k), flag_28: true, ..Default::default() });
    }
    let fields = aa_netcode::lan::load_map_fields(&units("(2v2)Somewhere"), &game, 77, host.data()).unwrap();
    host.request(descriptor::SET_SESSION_LOAD_MAP_REQUEST, &fields).unwrap();
    host.request(descriptor::SET_MAP_SIZE_REQUEST, &[Field::Uint { width: 32, value: 320 }, Field::Uint { width: 32, value: 256 }]).unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "the map order", |h, c, _, _| {
        [h, c].iter().all(|n| n.session().launch.mode == 2 && n.session().launch.active_map().size_x == 320)
    });
    for n in [&host, &client] {
        let s = n.session();
        assert_eq!(s.launch.file, units("(2v2)Somewhere"));
        assert_eq!(s.map_crc, 77);
        assert_eq!(s.launch.active_map().size_y, 256);
        assert_eq!(s.launch.active_map().header.kingdoms.len(), 2);
        assert_eq!(kingdoms(n), vec![units("north"), units("south")], "the people sit down on the new records");
    }
    // a client may not order it: 0x779780 rejects, nothing changes
    let fields = aa_netcode::lan::load_map_fields(&units("(2v2)Elsewhere"), &game, 78, client.data()).unwrap();
    client.request(descriptor::SET_SESSION_LOAD_MAP_REQUEST, &fields).unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "the rejection", |_, _, _, cs| cs.messages.iter().any(|m| m == "session_order_rejected"));
    assert_eq!(host.session().launch.file, units("(2v2)Somewhere"));

    // the format text travels and reseats (0x75E2D0)
    host.request(descriptor::SET_TEAMS_REQUEST, &[Field::Text(units("1v1"))]).unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "the format", |h, c, _, _| h.session().info_text == units("1v1") && c.session().info_text == units("1v1"));
}

/// 0x754880: whoever paused unpauses, with two humans the other cannot, and the
/// next pause of the same client waits for the delay.
#[test]
fn lan_session_pause_vote() {
    let mut host = LanNode::host(&config("Host"), &options(), launch(), data()).unwrap();
    let port = host.local_addr().unwrap().port();
    let mut client = LanNode::join(&config("Client"), format!("127.0.0.1:{port}").parse().unwrap(), data()).unwrap();
    let (mut hs, mut cs) = (Seen::default(), Seen::default());
    joined(&mut host, &mut client, &mut hs, &mut cs);
    let set = |v: u32| [Field::Uint { width: 1, value: v }];

    client.request(descriptor::SET_PAUSED_REQUEST, &set(1)).unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "the pause", |h, c, _, _| h.timeline().paused && c.timeline().paused);
    assert!(hs.messages.iter().any(|m| m == "networking_vote_to_pause_player_game_paused|s"));

    // the host is not who paused, and there are only two humans
    host.request(descriptor::SET_PAUSED_REQUEST, &set(0)).unwrap();
    client.request(descriptor::SET_READY_REQUEST, &[Field::Player(client.local_player().unwrap()), Field::Uint { width: 32, value: 1 }]).unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "the order of the host passed", |h, _, _, _| h.session().players.iter().all(|p| p.status != 0));
    assert!(host.timeline().paused, "only the client who paused unpauses");

    client.request(descriptor::SET_PAUSED_REQUEST, &set(0)).unwrap();
    pump(&mut host, &mut client, &mut hs, &mut cs, "the end of the pause", |h, c, _, _| !h.timeline().paused && !c.timeline().paused);
    assert!(cs.messages.iter().any(|m| m == "networking_vote_to_pause_player_game_unpaused|s"));
}

/// Leaving: the type 10 of a client makes the PlayerLeave of the host with
/// reason 5, the type 8 of a host ends the session of its clients.
#[test]
fn lan_session_leaving_tells_the_others() {
    let mut host = LanNode::host(&config("Host"), &options(), launch(), data()).unwrap();
    let port = host.local_addr().unwrap().port();
    let mut client = LanNode::join(&config("Client"), format!("127.0.0.1:{port}").parse().unwrap(), data()).unwrap();
    let (mut hs, mut cs) = (Seen::default(), Seen::default());
    joined(&mut host, &mut client, &mut hs, &mut cs);
    client.leave();
    pump(&mut host, &mut client, &mut hs, &mut cs, "the client leaves", |h, _, _, _| h.session().players.len() == 1);
    assert!(hs.messages.iter().any(|m| m == "networking_player_quit|s"));

    let mut client = LanNode::join(&config("Second"), format!("127.0.0.1:{port}").parse().unwrap(), data()).unwrap();
    let (mut hs, mut cs) = (Seen::default(), Seen::default());
    joined(&mut host, &mut client, &mut hs, &mut cs);
    host.leave();
    pump(&mut host, &mut client, &mut hs, &mut cs, "the host leaves", |_, _, _, cs| cs.disconnected == Some(None));
}
