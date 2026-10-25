//! `aa-parity film <path> [--game-root <dir>] [--inferred-registries]
//! [--checkpoints <n>] [--json]`
//!
//! Prints a film's header, its S checkpoints and the decree histogram. The
//! native definition counts some decree payloads need come from the v14 data
//! under `--game-root` (or `AA_GAME_ROOT`); without one only the built-in
//! ActorCommand registry is known and payloads that reference other
//! registries stop with an explicit decode error.
use aa_parity::{Film, Registries, classify, film::DecodeFailure, slog};
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::path::PathBuf;

struct Args {
    film: PathBuf,
    game_root: Option<PathBuf>,
    inferred: bool,
    checkpoints: usize,
    json: bool,
    /// `--decrees <from>-<to>`: list the decrees of these world ticks.
    decrees: Option<(u32, u32)>,
    /// `--chain <intervals>`: the RNG getter search (`aa_parity::chain`).
    chain: Option<usize>,
    /// `--draws <n>`: draws per getter for `--chain` (default 1).
    draws: u32,
}

fn args() -> Result<Args> {
    let mut it = std::env::args().skip(1);
    match it.next().as_deref() {
        Some("film") => {}
        _ => bail!(
            "usage: aa-parity film <path> [--game-root <dir>] [--inferred-registries] [--checkpoints <n>] [--json]"
        ),
    }
    let mut out = Args {
        film: PathBuf::new(),
        game_root: std::env::var_os("AA_GAME_ROOT").map(PathBuf::from),
        inferred: false,
        checkpoints: 12,
        json: false,
        decrees: None,
        chain: None,
        draws: 1,
    };
    let mut film = None;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--game-root" => {
                out.game_root = Some(it.next().context("--game-root needs a directory")?.into())
            }
            "--no-game-root" => out.game_root = None,
            "--inferred-registries" => out.inferred = true,
            "--checkpoints" => {
                out.checkpoints = it.next().context("--checkpoints needs a count")?.parse()?
            }
            "--json" => out.json = true,
            "--decrees" => {
                let range = it.next().context("--decrees needs <from>-<to>")?;
                let (a, b) = range
                    .split_once('-')
                    .context("--decrees needs <from>-<to>")?;
                out.decrees = Some((a.parse()?, b.parse()?));
            }
            "--chain" => out.chain = Some(it.next().context("--chain needs a count")?.parse()?),
            "--draws" => out.draws = it.next().context("--draws needs a count")?.parse()?,
            other if film.is_none() => film = Some(PathBuf::from(other)),
            other => bail!("unexpected argument {other}"),
        }
    }
    out.film = film.context("missing film path")?;
    Ok(out)
}

fn main() -> Result<()> {
    let args = args()?;
    let registries = match &args.game_root {
        Some(root) => Registries::load_v14(root, args.inferred)
            .with_context(|| format!("loading the v14 registries from {}", root.display()))?,
        None => Registries::builtin(),
    };
    let film = Film::read(&args.film, &registries)?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&summary(&film, &registries))?
        );
        return Ok(());
    }
    if let Some((from, to)) = args.decrees {
        print_decrees(&film, from, to);
        return Ok(());
    }
    if let Some(n) = args.chain {
        let found = aa_parity::chain::search(&film, n, args.draws);
        println!(
            "starting low16 states explaining the first {n} intervals with {} draw(s) per getter: {}",
            args.draws,
            found.len()
        );
        let prepared = aa_parity::chain::prepare(&film);
        for (x, explained) in found.iter().take(4) {
            println!("x = {x:#06x}");
            let mut state = *x;
            for (e, iv) in explained.iter().zip(&prepared) {
                if !e.getters.is_empty() {
                    let all = aa_parity::chain::placements(iv, state, args.draws, 12);
                    println!(
                        "  ticks {}..{}: {} placement(s)",
                        e.from_tick,
                        e.to_tick,
                        all.len()
                    );
                    for p in all {
                        println!("    {p:?}");
                    }
                }
                state = aa_parity::chain::lcg16_n(state, iv.residual * args.draws);
            }
        }
        return Ok(());
    }
    print_text(&film, &registries, args.checkpoints);
    Ok(())
}

/// The decrees of world ticks `from..=to` with the S entries each one logs
/// (`slog::decree_s_entries`).
fn print_decrees(film: &Film, from: u32, to: u32) {
    for t in film.ticks.iter().filter(|t| (from..=to).contains(&t.tick)) {
        for d in &t.decrees {
            let s = slog::decree_s_entries(d.descriptor as usize).map_or(0, |_| 2);
            println!(
                "tick {:>6}  player {:<4}  S+{}  {:>3} {:<38} {:?}",
                t.tick,
                d.player.map_or("-".into(), |p| p.to_string()),
                s,
                d.descriptor,
                d.name,
                d.decree
            );
        }
        if let Some(e) = &t.error {
            println!("tick {:>6}  decode stopped: {}", t.tick, e.message);
        }
    }
}

/// ActorCommand counts of the 86/87 decrees and gameplay decrees per player.
fn command_counts(film: &Film) -> (BTreeMap<String, usize>, BTreeMap<String, usize>) {
    let mut commands = BTreeMap::new();
    let mut by_player = BTreeMap::new();
    for d in film.ticks.iter().flat_map(|t| &t.decrees) {
        if let Some((index, name, _)) = classify::actor_command(&d.decree) {
            *commands.entry(format!("{index:02} {name}")).or_insert(0) += 1;
        }
        if classify::decree_class(&d.decree) == "command" {
            *by_player
                .entry(format!(
                    "player {}",
                    d.player.map_or("none".into(), |p| p.to_string())
                ))
                .or_insert(0) += 1;
        }
    }
    (commands, by_player)
}

fn summary(film: &Film, registries: &Registries) -> serde_json::Value {
    let (actor_commands, commands_by_player) = command_counts(film);
    serde_json::json!({
        "actor_commands": actor_commands,
        "commands_by_player": commands_by_player,
        "source": film.source,
        "chunks": film.chunks,
        "creator_class": film.header.creator_class,
        "seed": film.header.seed,
        "rmc": film.header.rmc,
        "players": film.players,
        "world_ticks": film.world_ticks(),
        "non_empty_ticks": film.non_empty_ticks(),
        "decrees": film.decree_count(),
        "decode_failures": film.failures().count(),
        "checkpoints": film.checkpoints,
        "histogram": film.histogram(),
        "registries": registries.counts().map(|(r, n, p)| (format!("{:#x} {}", r.0, aa_parity::registries::registry_name(r)), n, p)).collect::<Vec<_>>(),
    })
}

fn print_text(film: &Film, registries: &Registries, shown: usize) {
    let h = &film.header;
    println!(
        "film      sha256 {} ({} bytes)",
        film.source.sha256, film.source.bytes
    );
    for c in &film.chunks {
        println!(
            "  chunk   {:<15} packed {:>7}  decoded {:>7}{}",
            c.name,
            c.packed_bytes,
            c.decoded_bytes,
            if c.zlib { "  zlib" } else { "" }
        );
    }
    println!(
        "creator   {}  seed {} ({:#010x})",
        h.creator_class, h.seed, h.seed
    );
    if let Some(rmc) = &h.rmc {
        println!("rmc       {rmc}");
    }
    println!(
        "map       {}  biome {:?}  extents {}x{}",
        String::from_utf16_lossy(&h.map_header.name_key),
        String::from_utf16_lossy(&h.map_header.creator.biome),
        f32::from_bits(h.map_header.creator.world_extents_bits[0]),
        f32::from_bits(h.map_header.creator.world_extents_bits[1])
    );
    for p in &film.players {
        let kingdom = film
            .player_header_kingdom(p.id)
            .or_else(|| film.player_kingdom(p.id));
        println!(
            "player    id {} {:?} kingdom index {:?}{}",
            p.id,
            p.name,
            p.kingdom_index,
            kingdom.map_or(String::new(), |k| format!(
                " -> {} (nation {:?}, faction {:?})",
                k.id, k.nation, k.faction
            ))
        );
    }
    println!("registries");
    for (r, n, p) in registries.counts() {
        println!(
            "  {:#x} {:<18} {:>6}  {:?}",
            r.0,
            aa_parity::registries::registry_name(r),
            n,
            p
        );
    }
    println!(
        "orders    {} world ticks, {} non-empty, {} decrees, {} records stopped early",
        film.world_ticks(),
        film.non_empty_ticks(),
        film.decree_count(),
        film.failures().count()
    );
    let mut reasons: BTreeMap<&str, (usize, u32)> = BTreeMap::new();
    for t in film.failures() {
        let DecodeFailure { message } = t.error.as_ref().expect("failure");
        let e = reasons.entry(message.as_str()).or_insert((0, t.tick));
        e.0 += 1;
    }
    for (m, (n, first)) in reasons.iter().take(8) {
        println!("  {n:>5} x (first tick {first}) {m}");
    }
    println!(
        "checkpoints {} (SetSimSyncValue 311: tick, S last index, hash)",
        film.checkpoints.len()
    );
    for c in film.checkpoints.iter().take(shown) {
        println!(
            "  tick {:>6}  count {:>7}  hash {:>6}",
            c.tick, c.count, c.hash
        );
    }
    if let Some(c) = film.checkpoints.last() {
        println!("  last: tick {} count {} hash {}", c.tick, c.count, c.hash);
    }
    let account = slog::account(film);
    println!("S accounting per checkpoint interval: entries = clock + decree entries + residual");
    for i in account.iter().take(shown) {
        println!(
            "  ticks {:>5}..{:<5} {:>4} = {:>3} + {:>3} + {:>3}",
            i.from_tick, i.to_tick, i.entries, i.clock, i.decree_entries, i.residual
        );
    }
    let residual: i64 = account.iter().map(|i| i64::from(i.residual)).sum();
    let ticks = account.last().map_or(0, |i| i.to_tick);
    println!(
        "  total residual {residual} over {ticks} ticks; min {:?} max {:?}",
        account.iter().map(|i| i.residual).min(),
        account.iter().map(|i| i.residual).max()
    );
    let verified = slog::verify_zero_residual(film);
    println!(
        "zero-residual intervals {} : predicted hash == film hash in {}",
        verified.len(),
        verified.iter().filter(|v| v.1 == v.2).count()
    );
    for (i, ours, theirs) in verified.iter().filter(|v| v.1 != v.2).take(5) {
        println!(
            "  MISMATCH ticks {}..{} predicted {ours} film {theirs}",
            i.from_tick, i.to_tick
        );
    }
    let clock_only = slog::clock_only_intervals(&film.checkpoints);
    println!(
        "clock-only intervals {} ({} consistent with the clock model)",
        clock_only.len(),
        clock_only.iter().filter(|c| c.2).count()
    );
    println!("decree histogram");
    for (name, n) in film.histogram() {
        println!("  {n:>7}  {name}");
    }
    let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
    let mut commands: BTreeMap<String, usize> = BTreeMap::new();
    for d in film.ticks.iter().flat_map(|t| &t.decrees) {
        *classes
            .entry(classify::decree_class(&d.decree))
            .or_insert(0) += 1;
        if let Some((index, name, to)) = classify::actor_command(&d.decree) {
            let key = format!("{:>2} {name} -> {}", index, to.unwrap_or("NO EQUIVALENT"));
            *commands.entry(key).or_insert(0) += 1;
        }
    }
    println!("decree classes {classes:?}");
    println!(
        "command decrees by ordering player {:?}",
        command_counts(film).1
    );
    println!("ActorCommands (86/87) and their aa-play mapping");
    for (name, n) in commands {
        println!("  {n:>7}  {name}");
    }
}
