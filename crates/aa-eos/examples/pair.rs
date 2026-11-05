//! Two machines' worth of online play without the game: `pair host <name>` signs
//! in, makes a lobby and answers every datagram; `pair join <name>` signs in,
//! finds that lobby, joins, sends datagrams of many sizes and checks the answers.
//!
//! The two need different device IDs. Epic's device ID belongs to the Windows
//! user unless the SDK's folders say otherwise, so give each its own folders:
//! `APPDATA`, `LOCALAPPDATA`, and `pair <role> <name> <cache folder>`.
//!
//! Prints what happens, never the secret. Exit status 0 when all of it worked.
use aa_eos::{Config, Event, LobbySpec, Service, UserId};
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let role = args.get(1).map(String::as_str).unwrap_or("");
    let name = args.get(2).cloned().unwrap_or_else(|| "Pair".into());
    let cache = args.get(3).map(std::path::PathBuf::from);
    let code = match role {
        "library" => match Service::check_library() {
            Ok(()) => {
                println!("the library loaded and has every function");
                0
            }
            Err(e) => {
                println!("{e}");
                1
            }
        },
        "host" | "join" => run(role, &name, cache),
        _ => {
            eprintln!("pair library | host <name> [cache folder] | join <name> [cache folder]");
            2
        }
    };
    std::process::exit(code);
}

fn run(role: &str, name: &str, cache: Option<std::path::PathBuf>) -> i32 {
    let config = match Config::load("OpenAllies", "Pair Test", cache) {
        Ok(c) => c,
        Err(e) => {
            println!("settings: {e}");
            return 1;
        }
    };
    let service = match Service::start(config) {
        Ok(s) => s,
        Err(e) => {
            println!("start: {e}");
            return 1;
        }
    };
    println!("started; signing in as {name}");
    service.sign_in(name);
    let me = match wait(&service, Duration::from_secs(60), |e| match e {
        Event::SignedIn { user } => Some(Ok(user.clone())),
        Event::SignInFailed(f) => Some(Err(f.to_string())),
        Event::StartFailed(f) => Some(Err(f.clone())),
        _ => None,
    }) {
        Some(Ok(u)) => u,
        Some(Err(e)) => {
            println!("sign-in: {e}");
            return 1;
        }
        None => {
            println!("sign-in: no answer in 60 s");
            return 1;
        }
    };
    println!("signed in, address {}", me.address());
    if role == "host" { host(&service, &me, name) } else { join(&service, &me) }
}

fn wait<T>(service: &Service, within: Duration, mut pick: impl FnMut(&Event) -> Option<T>) -> Option<T> {
    let start = Instant::now();
    while start.elapsed() < within {
        for e in service.poll() {
            if let Event::SdkLog(line) = &e {
                println!("sdk: {line}");
            }
            if let Some(x) = pick(&e) {
                return Some(x);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}

fn host(service: &Service, me: &UserId, name: &str) -> i32 {
    service.create_lobby(LobbySpec { name: format!("{name}'s Game"), map: "Berlin".into(), players: 1, max_players: 8, password: false, observers: false, version: "pair".into() });
    let made = wait(service, Duration::from_secs(60), |e| match e {
        Event::LobbyCreated { id } => Some(Ok(id.clone())),
        Event::LobbyCreateFailed(f) => Some(Err(f.to_string())),
        _ => None,
    });
    match made {
        Some(Ok(_)) => println!("lobby created"),
        Some(Err(e)) => {
            println!("lobby: {e}");
            return 1;
        }
        None => {
            println!("lobby: no answer in 60 s");
            return 1;
        }
    }
    service.update_lobby(LobbySpec { name: format!("{name}'s Game"), map: "Berlin".into(), players: 1, max_players: 8, password: false, observers: false, version: "pair".into() });
    let mut link = service.link(me);
    let mut buffer = vec![0u8; 65_535];
    let start = Instant::now();
    let mut answered = 0u32;
    let mut last = Instant::now();
    while start.elapsed() < Duration::from_secs(240) {
        while let Some((n, from)) = link.recv_from(&mut buffer) {
            answered += 1;
            last = Instant::now();
            let done = n == 4 && &buffer[..4] == b"done";
            link.send_to(&buffer[..n], from);
            if done {
                println!("answered {answered} datagrams; the other side is done");
                std::thread::sleep(Duration::from_secs(2));
                service.leave_lobby();
                let left = wait(service, Duration::from_secs(10), |e| matches!(e, Event::LobbyLeft).then_some(()));
                println!("left: {}", left.is_some());
                return 0;
            }
        }
        for e in service.poll() {
            match e {
                Event::SdkLog(line) => println!("sdk: {line}"),
                other => println!("event: {other:?}"),
            }
        }
        if answered > 0 && last.elapsed() > Duration::from_secs(60) {
            println!("the other side stopped after {answered} datagrams");
            return 1;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    println!("nobody came in 240 s ({answered} datagrams)");
    1
}

fn join(service: &Service, me: &UserId) -> i32 {
    let start = Instant::now();
    let lobby = loop {
        if start.elapsed() > Duration::from_secs(120) {
            println!("no lobby found in 120 s");
            return 1;
        }
        service.find_lobbies();
        let list = wait(service, Duration::from_secs(30), |e| match e {
            Event::LobbiesFound(l) => Some(Ok(l.clone())),
            Event::LobbySearchFailed(f) => Some(Err(f.to_string())),
            _ => None,
        });
        match list {
            Some(Ok(l)) => {
                println!("found {} lobbies", l.len());
                if let Some(x) = l.into_iter().find(|x| x.version == "pair") {
                    break x;
                }
            }
            Some(Err(e)) => {
                println!("search: {e}");
                return 1;
            }
            None => println!("search: no answer in 30 s"),
        }
        std::thread::sleep(Duration::from_secs(3));
    };
    println!("found: {:?} on map {:?}, {}/{} players", lobby.name, lobby.map, lobby.players, lobby.max_players);
    service.join_lobby(&lobby);
    let joined = wait(service, Duration::from_secs(60), |e| match e {
        Event::LobbyJoined { owner, .. } => Some(Ok(owner.clone())),
        Event::LobbyJoinFailed(f) => Some(Err(f.to_string())),
        _ => None,
    });
    let owner = match joined {
        Some(Ok(o)) => o,
        Some(Err(e)) => {
            println!("join: {e}");
            return 1;
        }
        None => {
            println!("join: no answer in 60 s");
            return 1;
        }
    };
    let mut link = service.link(me);
    let host = link.add_peer(&owner);
    println!("joined; the host's address is {host}");
    let mut buffer = vec![0u8; 65_535];
    // sizes the session sends, one past the packet size, and the UDP maximum
    let sizes = [1usize, 64, 300, 1153, 1169, 1170, 1171, 4096, 20_000, 65_535];
    for (i, &size) in sizes.iter().cycle().take(100).enumerate() {
        let data: Vec<u8> = (0..size).map(|b| (b as u32 * 7 + i as u32) as u8).collect();
        let begun = Instant::now();
        let mut back = false;
        // a datagram may be lost: try again, as the session's channels would
        'tries: for _ in 0..10 {
            link.send_to(&data, host);
            let sent = Instant::now();
            while sent.elapsed() < Duration::from_millis(1500) {
                if let Some((n, from)) = link.recv_from(&mut buffer)
                    && from == host
                    && buffer[..n] == data[..]
                {
                    back = true;
                    break 'tries;
                }
                for e in service.poll() {
                    println!("event: {e:?}");
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        if !back {
            println!("datagram {i} of {size} bytes never came back");
            return 1;
        }
        if i < sizes.len() {
            println!("datagram {i}: {size} bytes came back in {} ms", begun.elapsed().as_millis());
        }
    }
    println!("all 100 datagrams came back");
    for _ in 0..20 {
        link.send_to(b"done", host);
        std::thread::sleep(Duration::from_millis(50));
    }
    service.leave_lobby();
    let left = wait(service, Duration::from_secs(10), |e| matches!(e, Event::LobbyLeft).then_some(()));
    println!("left: {}", left.is_some());
    0
}
