//! `aa-parity film <path> [--game-root <dir>] [--data <dir>]
//! [--inferred-registries] [--checkpoints <n>] [--json]`
//!
//! Prints a film's header, its S checkpoints and the decree histogram. The
//! native definition counts some decree payloads need come from the v14 data
//! under `--game-root` (or `AA_GAME_ROOT`), or from the loose data folder
//! `--data` over its `Data.rwd`; without one only the built-in ActorCommand
//! registry is known and payloads that reference other registries stop with an
//! explicit decode error.
//!
//! `aa-parity corpus --game-root <dir> --out <dir> --source <label>=<folder>...
//! [--films <label>=<dir>]... [--aggregate <file>]`
//!
//! Runs the reader over every film of the given profile folders, each on the
//! data it was recorded with, and writes the census (`CORPUS.md`, `corpus.json`
//! under `--out`, the anonymous aggregates in `--aggregate`).
use aa_orders::payload::Registry;
use aa_parity::registries::{
    self, ABILITIES, FORMATIONS, OBJECT_DEFINITIONS, ORGANIZATION_LAYOUTS, SHARED_PROPERTIES,
    registry_name,
};
use aa_parity::{Decree, Film, Registries, classify, decree, film::DecodeFailure, slog};
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

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
    /// `--data <dir>`: a loose data folder over `Data.rwd` instead of v14.
    data: Option<PathBuf>,
}

fn args() -> Result<Args> {
    let mut it = std::env::args().skip(1);
    match it.next().as_deref() {
        Some("film") => {}
        _ => bail!(
            "usage: aa-parity film <path> [--game-root <dir>] [--data <dir>] [--inferred-registries] [--checkpoints <n>] [--json]\n       aa-parity corpus --game-root <dir> --out <dir> --source <label>=<folder>... [--films <label>=<dir>]... [--aggregate <file>]"
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
        data: None,
    };
    let mut film = None;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--game-root" => {
                out.game_root = Some(it.next().context("--game-root needs a directory")?.into())
            }
            "--no-game-root" => out.game_root = None,
            "--data" => out.data = Some(it.next().context("--data needs a directory")?.into()),
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
    if std::env::args().nth(1).as_deref() == Some("corpus") {
        return corpus_main(corpus_args(std::env::args().skip(2))?);
    }
    let args = args()?;
    let registries = match (&args.game_root, &args.data) {
        (Some(root), Some(data)) => Registries::load_overlay(root, Some(data))
            .with_context(|| format!("loading the registries of {}", data.display()))?,
        (Some(root), None) => Registries::load_v14(root, args.inferred)
            .with_context(|| format!("loading the v14 registries from {}", root.display()))?,
        (None, _) => Registries::builtin(),
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

// ---------------------------------------------------------------------------
// `aa-parity corpus`: the film reader over a corpus of real films.
// ---------------------------------------------------------------------------

/// `--source <label>=<folder>`: a profile folder with `data/Films` (or
/// `Data/Films`) and, when it holds an index, its own data.
/// `--films <label>=<dir>`: a films folder recorded on the v14 data.
struct CorpusArgs {
    game_root: PathBuf,
    out: PathBuf,
    sources: Vec<(String, PathBuf)>,
    films: Vec<(String, PathBuf)>,
    aggregate: Option<PathBuf>,
}

fn corpus_args(mut it: impl Iterator<Item = String>) -> Result<CorpusArgs> {
    let mut game_root = std::env::var_os("AA_GAME_ROOT").map(PathBuf::from);
    let mut out = None;
    let mut sources = Vec::new();
    let mut films = Vec::new();
    let mut aggregate = None;
    let pair = |text: String| -> Result<(String, PathBuf)> {
        let (label, path) = text.split_once('=').context("expected <label>=<path>")?;
        Ok((label.to_owned(), path.into()))
    };
    while let Some(a) = it.next() {
        match a.as_str() {
            "--game-root" => game_root = Some(it.next().context("--game-root needs a dir")?.into()),
            "--out" => out = Some(PathBuf::from(it.next().context("--out needs a dir")?)),
            "--source" => sources.push(pair(it.next().context("--source needs a value")?)?),
            "--films" => films.push(pair(it.next().context("--films needs a value")?)?),
            "--aggregate" => {
                aggregate = Some(PathBuf::from(it.next().context("--aggregate needs a file")?))
            }
            other => bail!("unexpected argument {other}"),
        }
    }
    Ok(CorpusArgs {
        game_root: game_root.context("--game-root or AA_GAME_ROOT")?,
        out: out.context("--out <dir>")?,
        sources,
        films,
        aggregate,
    })
}

/// One unique film (by sha256) and where it was found.
#[derive(Clone, Debug)]
struct CorpusFilm {
    sha256: String,
    bytes: Vec<u8>,
    names: Vec<(String, String)>,
}

fn film_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| dir.display().to_string())? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            film_files(&path, out)?;
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("rfm"))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn data_folder(source: &Path) -> Option<PathBuf> {
    ["data", "Data"]
        .iter()
        .map(|d| source.join(d))
        .find(|p| p.is_dir())
}

/// Scan the corpus and freeze every unique film under `<out>/films`, so a QA run
/// rewriting its `LastFilm` while the census runs does not change the input.
fn scan_corpus(args: &CorpusArgs) -> Result<Vec<CorpusFilm>> {
    let frozen = args.out.join("films");
    std::fs::create_dir_all(&frozen)?;
    let mut by_hash: BTreeMap<String, CorpusFilm> = BTreeMap::new();
    let mut add = |label: &str, dir: &Path| -> Result<()> {
        let mut files = Vec::new();
        film_files(dir, &mut files)?;
        files.sort();
        for path in files {
            let bytes = std::fs::read(&path).with_context(|| path.display().to_string())?;
            let sha256 = aa_rules::sources::sha256(&bytes);
            let name = path
                .strip_prefix(dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let row = by_hash.entry(sha256.clone()).or_insert_with(|| CorpusFilm {
                sha256: sha256.clone(),
                bytes,
                names: Vec::new(),
            });
            row.names.push((label.to_owned(), name));
        }
        Ok(())
    };
    for (label, folder) in &args.sources {
        let dir = data_folder(folder)
            .map(|d| d.join("Films"))
            .filter(|d| d.is_dir())
            .with_context(|| format!("no data/Films in {}", folder.display()))?;
        add(label, &dir)?;
    }
    for (label, dir) in &args.films {
        add(label, dir)?;
    }
    for row in by_hash.values() {
        let path = frozen.join(format!("{}.RFM", row.sha256));
        if !path.exists() {
            std::fs::write(&path, &row.bytes)?;
        }
    }
    // A finished film seen by an earlier scan stays in the census after a QA run
    // rotated it out of its folder. An unfinished one was a run in progress.
    for entry in std::fs::read_dir(&frozen)? {
        let path = entry?.path();
        let Some(sha256) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
            continue;
        };
        if by_hash.contains_key(&sha256) {
            continue;
        }
        let bytes = std::fs::read(&path)?;
        if bytes.len() >= 12 && bytes[8..12] != [0; 4] {
            by_hash.insert(
                sha256.clone(),
                CorpusFilm {
                    sha256,
                    bytes,
                    names: vec![("frozen".to_owned(), "rotated out of its folder".to_owned())],
                },
            );
        }
    }
    Ok(by_hash.into_values().collect())
}

/// A data set's registries, computed once and kept under `<out>/registries`:
/// the 12,000 TGI files of a large mod take a while to load.
fn data_set(out: &Path, label: &str, load: impl FnOnce() -> Result<Registries>) -> Result<Registries> {
    let dir = out.join("registries");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{label}.json"));
    if let Ok(bytes) = std::fs::read(&path)
        && let Ok(cached) = serde_json::from_slice::<Registries>(&bytes)
    {
        return Ok(cached);
    }
    let start = std::time::Instant::now();
    let registries = load()?;
    eprintln!(
        "data set {label}: computed in {:.1} s",
        start.elapsed().as_secs_f32()
    );
    std::fs::write(&path, serde_json::to_vec(&registries)?)?;
    Ok(registries)
}

/// Decode one film's Orders with `registries`, stopping at the first record that
/// does not decode: `(records decoded, first failure)`.
fn probe_orders(
    orders: &[u8],
    registry: &aa_orders::Registry,
    registries: &Registries,
) -> (usize, Option<String>) {
    let Ok(records) = aa_parity::film::split_orders(orders) else {
        return (0, Some("Orders records do not split".into()));
    };
    let mut player = None;
    for (tick, body) in records.iter().enumerate() {
        let r = aa_parity::film::decode_record(tick as u32, body, registry, registries, &mut player);
        if let Some(e) = r.error {
            return (tick, Some(e.message));
        }
    }
    (records.len(), None)
}

/// The registry bit widths of a film nothing computed decodes. Only the width of
/// a reference is on the wire: this says what the recording data's counts must
/// have been, not which data that was.
const WIDTHS: [(Registry, &[u32]); 5] = [
    (OBJECT_DEFINITIONS, &[11, 12, 13, 14, 15]),
    (SHARED_PROPERTIES, &[8, 9, 10, 11]),
    (ABILITIES, &[9, 10, 11, 12, 13, 14, 15]),
    (FORMATIONS, &[3, 4, 5]),
    (ORGANIZATION_LAYOUTS, &[4, 5, 6, 7, 8]),
];

fn width_search(orders: &[u8], registry: &aa_orders::Registry) -> Vec<BTreeMap<&'static str, u32>> {
    let mut found = Vec::new();
    let mut choice = vec![0usize; WIDTHS.len()];
    loop {
        let mut trial = Registries::builtin();
        for (i, (reg, widths)) in WIDTHS.iter().enumerate() {
            trial = trial.with_count(
                *reg,
                (1usize << widths[choice[i]]) - 1,
                registries::Provenance::Inferred,
            );
        }
        if probe_orders(orders, registry, &trial).1.is_none() {
            found.push(
                WIDTHS
                    .iter()
                    .enumerate()
                    .map(|(i, (reg, widths))| (registry_name(*reg), widths[choice[i]]))
                    .collect(),
            );
        }
        let mut i = 0;
        loop {
            if i == WIDTHS.len() {
                return found;
            }
            choice[i] += 1;
            if choice[i] < WIDTHS[i].1.len() {
                break;
            }
            choice[i] = 0;
            i += 1;
        }
    }
}

fn utf16(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

/// Kingdoms a film renamed through 328 UpdateKingdomName, with the name. Only a
/// computer player's kingdom gets one, from its SAI (`Rommel AI`; mods name them
/// `Laidoner`). That is read off the films: every film has the human players'
/// kingdoms unnamed, and the SessionPlayers chunk leaves out the `ai` flag
/// (76C310 only logs it), so the split into human and computer is inferred.
/// From the first 329 KKC_MidgameCreateAIsAndSessionPlayers on, a rename is an AI
/// taking over a kingdom whose human left, so `before_takeover` stops there.
fn renamed_kingdoms(film: &Film, before_takeover: bool) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for d in film.ticks.iter().flat_map(|t| &t.decrees) {
        if before_takeover && d.descriptor as usize == decree::id::MIDGAME_CREATE_AIS {
            break;
        }
        if let Decree::UpdateKingdomName { kingdom, name, .. } = &d.decree {
            out.insert(utf16(kingdom).to_ascii_lowercase(), utf16(name));
        }
    }
    out
}

/// Whether session player `player` is a computer: on the film's roster, its
/// kingdom was renamed before any takeover; off the roster, it was created by a
/// 329, which creates AIs.
fn computer_player(film: &Film, renamed: &BTreeMap<String, String>, player: u32) -> bool {
    if !film.players.iter().any(|p| p.id == player) {
        return true;
    }
    film.player_kingdom(player)
        .is_some_and(|k| renamed.contains_key(&k.id.to_ascii_lowercase()))
}

/// Header ids a data set does not define.
fn missing_ids(film: &Film, r: &Registries) -> Vec<String> {
    let has = |list: &[String], id: &str| list.iter().any(|x| x.eq_ignore_ascii_case(id));
    let mut out = BTreeSet::new();
    for k in film
        .header
        .creator_kingdoms
        .iter()
        .chain(&film.header.header_kingdoms)
    {
        if !k.nation.is_empty() && !has(&r.nation_ids, &k.nation) {
            out.insert(format!("nation {}", k.nation));
        }
        if !k.faction.is_empty() && !has(&r.faction_ids, &k.faction) {
            out.insert(format!("faction {}", k.faction));
        }
    }
    if let Some(rmc) = &film.header.rmc
        && !has(&r.rmc_ids, rmc)
    {
        out.insert(format!("rmc {rmc}"));
    }
    let creator = &film.header.map_header.creator;
    let biome = utf16(&creator.biome);
    if !biome.is_empty() && !has(&r.biome_ids, &biome) {
        out.insert(format!("biome {biome}"));
    }
    let weather = utf16(&creator.weather);
    if !weather.is_empty() && !has(&r.weather_ids, &weather) {
        out.insert(format!("weather {weather}"));
    }
    out.into_iter().collect()
}

/// Whether the definition references of a decoded film name what they should in
/// this data set: research commands must name a property that is a technology.
fn semantic_score(film: &Film, r: &Registries) -> (usize, usize) {
    let (mut good, mut all) = (0, 0);
    for d in film.ticks.iter().flat_map(|t| &t.decrees) {
        let wire = match &d.decree {
            Decree::TellSelectedCommand(w) => w,
            Decree::TellActorCommand { command, .. } => command,
            _ => continue,
        };
        if wire.command == Some(35)
            && let Some(extra) = wire.extra
        {
            all += 1;
            if r.property_ids
                .get(extra as usize)
                .is_some_and(|p| p.to_ascii_lowercase().contains("tech"))
            {
                good += 1;
            }
        }
    }
    (good, all)
}

/// 70 FactionUseAbility decrees whose ability is one of the ordering player's
/// faction abilities in this data set: `(good, all)`. A wrong slot order or a
/// wrong data set shows here.
fn faction_ability_score(film: &Film, r: &Registries) -> (usize, usize) {
    let (mut good, mut all) = (0, 0);
    for d in film.ticks.iter().flat_map(|t| &t.decrees) {
        let Decree::FactionUseAbility { ability, .. } = &d.decree else {
            continue;
        };
        let Some(player) = d.player else {
            continue;
        };
        let kingdom = film
            .player_header_kingdom(player)
            .or_else(|| film.player_kingdom(player));
        let Some(kingdom) = kingdom else {
            continue;
        };
        all += 1;
        let name = r
            .ability_ids
            .get(*ability as usize)
            .map(|n| n.to_ascii_lowercase());
        if let (Some(name), Some(list)) = (
            name,
            r.faction_abilities.get(&kingdom.faction.to_ascii_lowercase()),
        ) && list.contains(&name)
        {
            good += 1;
        }
    }
    (good, all)
}

#[derive(Clone, Debug, Default, serde::Serialize)]
struct Tally {
    count: u64,
    films: u64,
    human: u64,
    computer: u64,
    no_player: u64,
}
impl Tally {
    fn add(&mut self, count: u64, human: u64, computer: u64, no_player: u64) {
        self.count += count;
        self.films += 1;
        self.human += human;
        self.computer += computer;
        self.no_player += no_player;
    }
}

/// One film's decrees of a kind.
#[derive(Clone, Copy, Debug, Default)]
struct PerFilm {
    count: u64,
    human: u64,
    computer: u64,
    no_player: u64,
}
fn tally_into(into: &mut BTreeMap<String, Tally>, per: BTreeMap<String, PerFilm>) {
    for (key, p) in per {
        into.entry(key)
            .or_default()
            .add(p.count, p.human, p.computer, p.no_player);
    }
}

#[derive(Clone, Debug, serde::Serialize)]
struct FilmRow {
    sha256: String,
    bytes: usize,
    names: Vec<(String, String)>,
    status: String,
    creator_class: String,
    rmc: Option<String>,
    map_name: String,
    biome: String,
    weather: String,
    extents: [f32; 2],
    seed: u32,
    num_heroes: Option<u32>,
    creator_mode: u32,
    header_mode: u32,
    version: String,
    build: f32,
    v14_check: String,
    players: Vec<(u32, String, Option<u32>, bool)>,
    ticks: usize,
    non_empty_ticks: usize,
    decrees: usize,
    checkpoints: usize,
    failures: usize,
    failure_causes: Vec<(String, usize)>,
    data_set: String,
    decodes_with: Vec<String>,
    widths: Vec<BTreeMap<&'static str, u32>>,
    counts: BTreeMap<&'static str, usize>,
    missing_in_data_set: Vec<String>,
    decodes_on_v14: bool,
    missing_on_v14: Vec<String>,
    research_named: (usize, usize),
    faction_abilities_named: (usize, usize),
    /// `(kingdom, name)` of the 328 renames.
    renames: Vec<(String, String)>,
    /// Random-map settings: `(terrain weights, object counts)`, the weights as
    /// binary32 values.
    random_settings: Option<(Vec<(String, f32)>, Vec<(String, u32)>)>,
    /// `(rule, kind, value)` of the creator's own rules.
    rules: Vec<(String, u32, u32)>,
    /// `(kingdom, nation, faction, team)` of the header's own kingdoms.
    kingdoms: Vec<(String, String, String, String)>,
    /// The map or saved game a `KKC_WorldCreatorLoad` loads.
    source_map: String,
    /// Whether the v14 target has it.
    source_on_v14: bool,
    /// Whether a folder the film was found in has it.
    source_in_folder: bool,
    /// Sync intervals with only clock and decree entries, recomputed from the
    /// film: `(hash equal to the recorded one, all such intervals, checkpoint
    /// intervals, intervals with fewer entries than the decrees account for)`.
    sync_proof: (usize, usize, usize, usize),
    /// The updates a saved game's clock had run when the film starts, found by
    /// the sync hashes.
    sync_clock_offset: Option<u32>,
    /// `(player, computer, 333, 16, 18, command decrees)`.
    per_player: Vec<(u32, bool, u32, u32, u32, u32)>,
}

/// The v14 target's version string and build (`[0xB3888C]` is the data's
/// `Version` var, `[0xB27A80]` is 4129.7 in every AA.exe of the project).
const TARGET_VERSION: &str = "1.0.5.0";
const TARGET_BUILD: f32 = 4129.7;

#[derive(Default)]
struct Totals {
    decrees: BTreeMap<String, Tally>,
    commands: BTreeMap<String, Tally>,
    orders: BTreeMap<String, Tally>,
    abilities: BTreeMap<String, Tally>,
    selectors: BTreeMap<String, Tally>,
    formations: BTreeMap<String, Tally>,
    properties: BTreeMap<String, Tally>,
    objects: BTreeMap<String, Tally>,
}

fn add_film_totals(film: &Film, r: &Registries, totals: &mut Totals) {
    let renamed = renamed_kingdoms(film, true);
    let mut decrees: BTreeMap<String, PerFilm> = BTreeMap::new();
    let mut commands: BTreeMap<String, PerFilm> = BTreeMap::new();
    let mut orders: BTreeMap<String, PerFilm> = BTreeMap::new();
    let mut abilities: BTreeMap<String, PerFilm> = BTreeMap::new();
    let mut selectors: BTreeMap<String, PerFilm> = BTreeMap::new();
    let mut formations: BTreeMap<String, PerFilm> = BTreeMap::new();
    let mut properties: BTreeMap<String, PerFilm> = BTreeMap::new();
    let mut objects: BTreeMap<String, PerFilm> = BTreeMap::new();
    for d in film.ticks.iter().flat_map(|t| &t.decrees) {
        let bump = |map: &mut BTreeMap<String, PerFilm>, key: String| {
            let row = map.entry(key).or_default();
            row.count += 1;
            match d.player.map(|player| computer_player(film, &renamed, player)) {
                Some(true) => row.computer += 1,
                Some(false) => row.human += 1,
                None => row.no_player += 1,
            }
        };
        bump(&mut decrees, format!("{:3} {}", d.descriptor, d.name));
        if let Some(order) = classify::order_kind(d.descriptor as usize, &d.decree) {
            bump(&mut orders, order);
        }
        let wire = match &d.decree {
            Decree::TellSelectedCommand(w) => Some(w),
            Decree::TellActorCommand { command, .. } => Some(command),
            _ => None,
        };
        if let Some(w) = wire {
            if let Some((index, name, _)) = classify::actor_command(&d.decree) {
                bump(&mut commands, format!("{index:02} {name}"));
            } else {
                bump(&mut commands, "-- (no command definition)".into());
            }
            if let Some(name) = w.parameter.and_then(|f| r.formation_ids.get(f as usize)) {
                bump(&mut formations, name.clone());
            }
            if let Some(name) = w.extra.and_then(|p| r.property_ids.get(p as usize)) {
                bump(&mut properties, name.clone());
            }
            if let Some(name) = w.object.and_then(|o| r.object_ids.get(o as usize)) {
                bump(&mut objects, name.clone());
            }
        }
        match &d.decree {
            Decree::TellActorAbility {
                selector, ability, ..
            } => {
                if let Some(label) = r.ability_label(*ability) {
                    bump(&mut abilities, format!("67 {label}"));
                }
                bump(&mut selectors, format!("67 {}", utf16(selector)));
            }
            Decree::TellActorMove { selector, .. } => {
                bump(&mut selectors, format!("66 {}", utf16(selector)));
            }
            Decree::FactionUseAbility { ability, .. } => {
                if let Some(label) = r.ability_label(*ability) {
                    bump(&mut abilities, format!("70 {label}"));
                }
            }
            _ => {}
        }
    }
    tally_into(&mut totals.decrees, decrees);
    tally_into(&mut totals.commands, commands);
    tally_into(&mut totals.orders, orders);
    tally_into(&mut totals.abilities, abilities);
    tally_into(&mut totals.selectors, selectors);
    tally_into(&mut totals.formations, formations);
    tally_into(&mut totals.properties, properties);
    tally_into(&mut totals.objects, objects);
}

fn corpus_main(args: CorpusArgs) -> Result<()> {
    std::fs::create_dir_all(&args.out)?;
    let films = scan_corpus(&args)?;
    eprintln!("{} unique films", films.len());
    let registry = aa_orders::native::registry()?;

    // The data sets, in the order they are tried after the film's own.
    let mut sets: Vec<(String, Registries)> = Vec::new();
    let root = args.game_root.clone();
    sets.push((
        "v14".into(),
        data_set(&args.out, "v14", || Registries::load_v14(&root, true))?,
    ));
    let mut own: BTreeMap<String, String> = BTreeMap::new();
    for (label, folder) in &args.sources {
        let Some(data) = data_folder(folder) else {
            continue;
        };
        let has_index = std::fs::read_dir(&data)?.flatten().any(|e| {
            let name = e.file_name().to_string_lossy().to_ascii_lowercase();
            name.starts_with("index") && name.ends_with(".lst")
        });
        if !has_index {
            eprintln!("{label}: no index in its data folder, no data set of its own");
            continue;
        }
        let r = data_set(&args.out, label, || Registries::load_overlay(&root, Some(&data)))?;
        sets.push((label.clone(), r));
        own.insert(label.clone(), label.clone());
    }
    for (label, r) in &sets {
        let counts: Vec<String> = r
            .counts()
            .map(|(reg, n, _)| format!("{} {n}", registry_name(reg)))
            .collect();
        eprintln!("data set {label}: {}; {} notes", counts.join(", "), r.notes.len());
    }

    let context = SourceContext {
        v14_resources: registries::v14_resources(&root)?,
        folders: args
            .sources
            .iter()
            .filter_map(|(label, folder)| Some((label.clone(), data_folder(folder)?)))
            .collect(),
    };
    let mut rows = Vec::new();
    let mut totals = Totals::default();
    for film in &films {
        // A QA run in progress writes its film with no directory yet.
        let live_qa = film.names.iter().all(|(label, _)| {
            args.films.iter().any(|(films_label, _)| films_label == label)
        }) && film.bytes.len() >= 12
            && film.bytes[8..12] == [0; 4];
        if live_qa {
            eprintln!("{}: a QA film still being written, skipped", &film.sha256[..8]);
            continue;
        }
        let row = corpus_film(film, &sets, &own, &registry, &context, &mut totals);
        eprintln!(
            "{} {:>7} {:<28} {:<22} ticks {:>6} decrees {:>6} fail {:>4} set {} {}",
            &film.sha256[..8],
            film.bytes.len(),
            row.names.first().map_or("", |n| n.0.as_str()),
            row.rmc.as_deref().unwrap_or(row.source_map.as_str()),
            row.ticks,
            row.decrees,
            row.failures,
            row.data_set,
            row.status
        );
        rows.push(row);
    }
    let json = serde_json::json!({
        "films": rows,
        "decrees": totals.decrees,
        "commands": totals.commands,
        "orders": totals.orders,
        "abilities": totals.abilities,
        "selectors": totals.selectors,
        "formations": totals.formations,
        "properties": totals.properties,
        "objects": totals.objects,
        "data_sets": sets.iter().map(|(label, r)| serde_json::json!({
            "label": label,
            "counts": r.counts().map(|(reg, n, p)| (registry_name(reg), n, format!("{p:?}"))).collect::<Vec<_>>(),
            "notes": r.notes,
        })).collect::<Vec<_>>(),
    });
    std::fs::write(
        args.out.join("corpus.json"),
        serde_json::to_vec_pretty(&json)?,
    )?;
    std::fs::write(
        args.out.join("CORPUS.md"),
        corpus_report(&rows, &sets, &totals, true),
    )?;
    if let Some(path) = &args.aggregate {
        let text = corpus_report(&rows, &sets, &totals, false);
        std::fs::write(path, text.replace("\r\n", "\n"))?;
    }
    Ok(())
}

/// What the census knows about where a film's source can come from.
struct SourceContext {
    v14_resources: BTreeSet<String>,
    /// Label -> the source's data folder.
    folders: BTreeMap<String, PathBuf>,
}

/// Decode one film with its own data set, then the others; when none decodes it,
/// search the reference widths.
fn corpus_film(
    film: &CorpusFilm,
    sets: &[(String, Registries)],
    own: &BTreeMap<String, String>,
    registry: &aa_orders::Registry,
    context: &SourceContext,
    totals: &mut Totals,
) -> FilmRow {
    let mut row = FilmRow {
        sha256: film.sha256.clone(),
        bytes: film.bytes.len(),
        names: film.names.clone(),
        status: "decoded".into(),
        creator_class: String::new(),
        rmc: None,
        map_name: String::new(),
        biome: String::new(),
        weather: String::new(),
        extents: [0.0; 2],
        seed: 0,
        num_heroes: None,
        creator_mode: 0,
        header_mode: 0,
        version: String::new(),
        build: 0.0,
        v14_check: String::new(),
        players: Vec::new(),
        ticks: 0,
        non_empty_ticks: 0,
        decrees: 0,
        checkpoints: 0,
        failures: 0,
        failure_causes: Vec::new(),
        data_set: String::new(),
        decodes_with: Vec::new(),
        widths: Vec::new(),
        counts: BTreeMap::new(),
        missing_in_data_set: Vec::new(),
        decodes_on_v14: false,
        missing_on_v14: Vec::new(),
        research_named: (0, 0),
        faction_abilities_named: (0, 0),
        renames: Vec::new(),
        random_settings: None,
        rules: Vec::new(),
        kingdoms: Vec::new(),
        source_map: String::new(),
        source_on_v14: true,
        source_in_folder: false,
        sync_proof: (0, 0, 0, 0),
        sync_clock_offset: None,
        per_player: Vec::new(),
    };
    let orders = match Film::orders_chunk(film.bytes.clone()) {
        Ok(orders) => orders,
        Err(e) => {
            row.status = format!("unreadable: {e}");
            return row;
        }
    };
    // Own data set first, then every other one.
    let mut order: Vec<usize> = (0..sets.len()).collect();
    let is_own = |set: &(String, Registries)| {
        film.names.iter().any(|(label, _)| own.contains_key(label) && label == &set.0)
    };
    order.sort_by_key(|&i| if is_own(&sets[i]) { 0 } else { 1 });
    let mut decoding = Vec::new();
    let mut first_failure = None;
    for &i in &order {
        match probe_orders(&orders, registry, &sets[i].1).1 {
            None => decoding.push(i),
            Some(message) => {
                if first_failure.is_none() {
                    first_failure = Some((sets[i].0.clone(), message));
                }
            }
        }
    }
    row.decodes_with = decoding.iter().map(|&i| sets[i].0.clone()).collect();
    row.decodes_on_v14 = decoding.iter().any(|&i| sets[i].0 == "v14");
    // Among the data sets that decode, the one whose header ids and research
    // references fit best; the film's own data set wins a tie.
    let mut choice: Option<(usize, usize, usize)> = None;
    let mut parsed: Option<Film> = None;
    for &i in &decoding {
        let Ok(decoded) = Film::from_bytes(film.bytes.clone(), &sets[i].1) else {
            continue;
        };
        let missing = missing_ids(&decoded, &sets[i].1).len();
        let (good, all) = semantic_score(&decoded, &sets[i].1);
        let wrong = all - good;
        if choice.is_none_or(|(_, m, w)| (missing, wrong) < (m, w)) {
            choice = Some((i, missing, wrong));
            parsed = Some(decoded);
        }
    }
    let width_only;
    let film_parsed: Option<(Film, &Registries)> = if let (Some((i, ..)), Some(parsed)) =
        (choice, parsed)
    {
        row.data_set = sets[i].0.clone();
        Some((parsed, &sets[i].1))
    } else {
        row.data_set = "none".into();
        row.widths = width_search(&orders, registry);
        if let Some((set, message)) = first_failure {
            row.status = format!("no data set decodes it; first failure on {set}: {message}");
        }
        match row.widths.first() {
            None => None,
            Some(widths) => {
                // The widths decode every record; the counts stay unknown inside
                // them, so the wire values have no names.
                let mut trial = Registries::builtin();
                for (reg, _) in WIDTHS {
                    let bits = widths[registry_name(reg)];
                    trial = trial.with_count(
                        reg,
                        (1usize << bits) - 1,
                        registries::Provenance::Inferred,
                    );
                }
                width_only = trial;
                row.data_set = "widths".into();
                row.status = "decoded with reference widths only: the recording data is not in the corpus"
                    .into();
                Film::from_bytes(film.bytes.clone(), &width_only)
                    .ok()
                    .map(|parsed| (parsed, &width_only))
            }
        }
    };
    let header = match &film_parsed {
        Some((parsed, _)) => Ok(parsed.clone()),
        None => Film::from_bytes(film.bytes.clone(), &sets[0].1),
    };
    let parsed = match header {
        Ok(parsed) => parsed,
        Err(e) => {
            row.status = format!("unreadable: {e}");
            return row;
        }
    };
    let h = &parsed.header;
    let m = &h.map_header;
    row.creator_class = h.creator_class.clone();
    row.rmc = h.rmc.clone();
    row.map_name = utf16(&m.name_key);
    row.biome = utf16(&m.creator.biome);
    row.weather = utf16(&m.creator.weather);
    row.extents = [
        f32::from_bits(m.creator.world_extents_bits[0]),
        f32::from_bits(m.creator.world_extents_bits[1]),
    ];
    row.seed = h.seed;
    row.num_heroes = match &h.creator {
        aa_mapworld::header::DerivedCreator::Random(s) => Some(s.num_heroes),
        _ => None,
    };
    row.creator_mode = m.creator.rules.mode;
    row.header_mode = m.rules.mode;
    row.version = utf16(&m.string_58);
    row.build = f32::from_bits(m.word_50);
    row.v14_check = format!(
        "{:?}",
        aa_parity::film::film_version_check(m, TARGET_VERSION, TARGET_BUILD)
    );
    if let aa_mapworld::header::DerivedCreator::Load {
        source_resource_ids, ..
    } = &h.creator
    {
        row.source_map = utf16(source_resource_ids);
        let key = row.source_map.replace('\\', "/").to_ascii_lowercase();
        row.source_on_v14 = key.is_empty() || context.v14_resources.contains(&key);
        row.source_in_folder = film.names.iter().any(|(label, _)| {
            context
                .folders
                .get(label)
                .is_some_and(|data| data.join(&row.source_map).is_file())
        });
    }
    if let aa_mapworld::header::DerivedCreator::Random(settings) = &h.creator {
        row.random_settings = Some((
            settings
                .terrain_weights
                .iter()
                .map(|(id, bits)| (utf16(id), f32::from_bits(*bits)))
                .collect(),
            settings
                .object_counts
                .iter()
                .map(|(id, count)| (utf16(id), *count))
                .collect(),
        ));
    }
    row.rules = m
        .creator
        .rules
        .rules
        .iter()
        .map(|rule| (utf16(&rule.key), rule.kind, rule.value))
        .collect();
    row.kingdoms = h
        .header_kingdoms
        .iter()
        .map(|k| (k.id.clone(), k.nation.clone(), k.faction.clone(), k.team.clone()))
        .collect();
    let renamed = renamed_kingdoms(&parsed, true);
    row.renames = renamed_kingdoms(&parsed, false).into_iter().collect();
    row.players = parsed
        .players
        .iter()
        .map(|p| {
            let computer = computer_player(&parsed, &renamed, p.id);
            (p.id, p.name.clone(), p.kingdom_index, computer)
        })
        .collect();
    let mut per_player: BTreeMap<u32, (u32, u32, u32, u32)> = BTreeMap::new();
    for d in parsed.ticks.iter().flat_map(|t| &t.decrees) {
        let Some(player) = d.player else {
            continue;
        };
        let row = per_player.entry(player).or_default();
        match &d.decree {
            Decree::SelectionSet { .. } => row.0 += 1,
            Decree::SelectionAdd { .. } => row.1 += 1,
            Decree::SelectionClear => row.2 += 1,
            Decree::TellSelectedCommand(_) | Decree::TellActorCommand { .. } => row.3 += 1,
            _ => {}
        }
    }
    row.per_player = per_player
        .into_iter()
        .map(|(player, (set, add, clear, commands))| {
            let computer = computer_player(&parsed, &renamed, player);
            (player, computer, set, add, clear, commands)
        })
        .collect();
    let account = slog::account(&parsed);
    let mut verified = slog::verify_zero_residual_at(&parsed, &account, 0);
    // A saved game's clock does not start at zero: its intervals fit only at the
    // offset the film continues from. One offset fitting all of them is the proof.
    if verified.iter().any(|v| v.1 != v.2) {
        for offset in 1..4_000_000u32 {
            let trial = slog::verify_zero_residual_at(&parsed, &account, offset);
            if trial.iter().all(|v| v.1 == v.2) {
                row.sync_clock_offset = Some(offset);
                verified = trial;
                break;
            }
        }
    }
    row.sync_proof = (
        verified.iter().filter(|v| v.1 == v.2).count(),
        verified.len(),
        account.len(),
        account.iter().filter(|i| i.residual < 0).count(),
    );
    row.ticks = parsed.world_ticks();
    row.non_empty_ticks = parsed.non_empty_ticks();
    row.decrees = parsed.decree_count();
    row.checkpoints = parsed.checkpoints.len();
    row.failures = parsed.failures().count();
    let mut causes: BTreeMap<String, usize> = BTreeMap::new();
    for t in parsed.failures() {
        let e = t.error.as_ref().expect("a failed record has its error");
        *causes.entry(e.message.clone()).or_default() += 1;
    }
    row.failure_causes = causes.into_iter().collect();
    if let Some((_, set)) = sets.first() {
        row.missing_on_v14 = missing_ids(&parsed, set);
    }
    if let Some((parsed, set)) = &film_parsed {
        row.counts = set
            .counts()
            .map(|(reg, n, _)| (registry_name(reg), n))
            .collect();
        if row.data_set != "widths" {
            row.missing_in_data_set = missing_ids(parsed, set);
            row.research_named = semantic_score(parsed, set);
            row.faction_abilities_named = faction_ability_score(parsed, set);
        }
        add_film_totals(parsed, set, totals);
    }
    row
}

fn table_rows(map: &BTreeMap<String, Tally>) -> Vec<(&String, &Tally)> {
    let mut rows: Vec<_> = map.iter().collect();
    rows.sort_by(|a, b| b.1.count.cmp(&a.1.count).then(a.0.cmp(b.0)));
    rows
}

fn tally_table(out: &mut String, title: &str, map: &BTreeMap<String, Tally>, limit: usize) {
    let total: u64 = map.values().map(|t| t.count).sum();
    let _ = writeln!(out, "\n## {title}\n");
    let _ = writeln!(out, "| kind | count | share | films | human | computer |");
    let _ = writeln!(out, "|---|---:|---:|---:|---:|---:|");
    for (key, t) in table_rows(map).into_iter().take(limit) {
        let _ = writeln!(
            out,
            "| {} | {} | {:.2}% | {} | {} | {} |",
            key.trim(),
            t.count,
            t.count as f64 * 100.0 / total.max(1) as f64,
            t.films,
            t.human,
            t.computer,
        );
    }
    if map.len() > limit {
        let _ = writeln!(out, "\n({} more kinds, not listed.)", map.len() - limit);
    }
}

/// The orders in order of coverage, with the running share of all orders and of
/// the human players' orders.
fn coverage_table(out: &mut String, map: &BTreeMap<String, Tally>) {
    let total: u64 = map.values().map(|t| t.count).sum();
    let human: u64 = map.values().map(|t| t.human).sum();
    let _ = writeln!(
        out,
        "\n## Orders in order of coverage\n\n{total} orders, {human} of them by human players. \"aa-play\" is the `classify::ACTOR_COMMAND_MAPPING` target or the decree's `mapping::Mapper` arm.\n"
    );
    let unmapped = |pick: &dyn Fn(&Tally) -> u64| -> u64 {
        map.iter()
            .filter(|(k, _)| k.ends_with("NO EQUIVALENT"))
            .map(|(_, t)| pick(t))
            .sum()
    };
    let (all_out, human_out) = (unmapped(&|t| t.count), unmapped(&|t| t.human));
    let _ = writeln!(
        out,
        "Orders with no aa-play equivalent: {all_out} ({:.1}%) of all, {human_out} ({:.1}%) of the human players'. A `default order` is a generic command with no ActorCommand (flag bit 6 clear), the player's plain click: the kernel picks the command from the selection and the fields it carries (0x4DA570, 0x61F210).\n",
        all_out as f64 * 100.0 / total.max(1) as f64,
        human_out as f64 * 100.0 / human.max(1) as f64
    );
    let _ = writeln!(out, "| order | aa-play | count | running share | films | human | human rank |");
    let _ = writeln!(out, "|---|---|---:|---:|---:|---:|---:|");
    let mut by_human: Vec<(&String, u64)> = map.iter().map(|(k, t)| (k, t.human)).collect();
    by_human.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let mut running = 0;
    for (key, t) in table_rows(map) {
        running += t.count;
        let (name, to) = key.split_once(" -> ").unwrap_or((key.as_str(), ""));
        let rank = by_human.iter().position(|(k, _)| *k == key).map_or(0, |p| p + 1);
        let _ = writeln!(
            out,
            "| {name} | {to} | {} | {:.2}% | {} | {} | {rank} |",
            t.count,
            running as f64 * 100.0 / total.max(1) as f64,
            t.films,
            t.human,
        );
    }
    let _ = writeln!(
        out,
        "\n## The human players' orders\n\n| order | aa-play | human count | running share of human orders | films |\n|---|---|---:|---:|---:|"
    );
    let mut running = 0;
    for (key, count) in by_human {
        if count == 0 {
            continue;
        }
        running += count;
        let (name, to) = key.split_once(" -> ").unwrap_or((key.as_str(), ""));
        let _ = writeln!(
            out,
            "| {name} | {to} | {count} | {:.2}% | {} |",
            running as f64 * 100.0 / human.max(1) as f64,
            map[key].films
        );
    }
}

fn map_label(r: &FilmRow, private: bool) -> String {
    if let Some(rmc) = &r.rmc {
        return rmc.clone();
    }
    match r.creator_mode {
        // A saved game's name is the player's own.
        2 if !private => "(saved game)".into(),
        _ => r.source_map.clone(),
    }
}

/// The census as Markdown. `private` is CORPUS.md (file names, player names); the
/// other form holds only anonymous aggregates.
fn corpus_report(
    rows: &[FilmRow],
    sets: &[(String, Registries)],
    totals: &Totals,
    private: bool,
) -> String {
    let mut out = String::new();
    let readable: Vec<&FilmRow> = rows.iter().filter(|r| r.ticks > 0).collect();
    let on_computed: Vec<&&FilmRow> = readable
        .iter()
        .filter(|r| r.data_set != "widths" && r.data_set != "none")
        .collect();
    let widths_only = readable.iter().filter(|r| r.data_set == "widths").count();
    let unfinished = rows.iter().filter(|r| r.ticks == 0).count();
    let failures: usize = readable.iter().map(|r| r.failures).sum();
    let _ = writeln!(out, "# Film corpus census\n");
    let _ = writeln!(
        out,
        "Generated by `aa-parity corpus` over the unique real films (by sha256). {} films: {} decode on a data set computed from game data, {} decode only with reference widths because their data is not in the corpus, {} without a chunk directory cannot be opened (the game never closed them). {} world ticks, {} decrees, {} records that do not decode to their exact end.",
        rows.len(),
        on_computed.len(),
        widths_only,
        unfinished,
        readable.iter().map(|r| r.ticks as u64).sum::<u64>(),
        readable.iter().map(|r| r.decrees as u64).sum::<u64>(),
        failures,
    );
    let _ = writeln!(out, "\n## What is proven\n");
    let _ = writeln!(out, "- Decoding: a record counts as decoded only when the native descriptor loop (0x74D710 order) ends exactly on its last bit and every definition index is below its live count (0x411FA0 possibilities, rejected otherwise).");
    let proof = readable.iter().fold((0, 0, 0, 0), |a, r| {
        (
            a.0 + r.sync_proof.0,
            a.1 + r.sync_proof.1,
            a.2 + r.sync_proof.2,
            a.3 + r.sync_proof.3,
        )
    });
    let _ = writeln!(out, "- Decree identity, against the recording: the film's own 311 checkpoints hash every S-logging decree's name and id. Of {} checkpoint intervals, {} hold only clock and decree entries, so their hash can be recomputed from the decode alone: {} of them equal the recorded hash ({} film(s) of a saved game fit at the clock offset their intervals agree on). {} intervals hold fewer entries than the decoded decrees account for.", proof.2, proof.1, proof.0, readable.iter().filter(|r| r.sync_clock_offset.is_some()).count(), proof.3);
    let _ = writeln!(out, "- Counts on v14: the computed ActorCommand, ThingData, Formation, OrganizationLayout, SharedProperty and AbilityDefinition counts equal what NativeSampler read from the running original, and the SharedProperty slots equal its 194 live IDS in order (`tests/registry_counts.rs`).");
    let _ = writeln!(out, "- Counts on the mods: computed by the same rules, with no live reading to compare. The evidence is that the films recorded on them decode, their header ids exist, and the references name what they should (the last two columns of the data set table).");
    let abilities = readable.iter().fold((0, 0), |a, r| {
        (a.0 + r.faction_abilities_named.0, a.1 + r.faction_abilities_named.1)
    });
    let _ = writeln!(out, "- The AbilityDefinition slot order has no live reading to compare; the evidence is the films: {} of {} 70 FactionUseAbility decrees on named data sets name one of the ordering player's own faction abilities.", abilities.0, abilities.1);
    let _ = writeln!(out, "- Human or computer is inferred: a kingdom renamed by 328 UpdateKingdomName is a computer player's (SessionPlayers leaves out the `ai` flag, 76C310 only logs it). A session player off the film's roster was created by 329 KKC_MidgameCreateAIsAndSessionPlayers and counts as a computer; a rename after a 329 is that AI taking over and leaves the human who left as human. The selection decrees agree: humans use 16 and 18, computers 333.");

    let _ = writeln!(out, "\n## Failure causes\n");
    let saved = readable.iter().filter(|r| r.creator_mode != 0).count();
    let team: u64 = totals
        .decrees
        .iter()
        .filter(|(k, _)| k.trim_start().starts_with("23 "))
        .map(|(_, t)| t.count)
        .sum();
    let _ = writeln!(out, "Every failure of the first pass had one of these causes, all fixed in the reader:\n");
    let _ = writeln!(out, "- A film of a saved game ({saved} films) has its creator's rules in mode `savegame`, which aa-mapworld refuses although 0x512420 only stores the word. `film::decode_film_header` reads them.");
    let _ = writeln!(out, "- 23 KKC_TeamCommand ({team} decrees) needs the flags of its definition. They are code-defined (0x475590) and now known: `registries::TEAM_COMMANDS`.");
    let _ = writeln!(out, "- Every other record that stopped had its references read at the wrong width, because the counts of its data were not those of v14. A mod's ThingData takes 14 bits against 11. The counts now come from the data the film was recorded with.");
    let _ = writeln!(out, "- {widths_only} films are on a data set the corpus does not hold (`widths`): their references fit the stock widths, but some index is above the v14 count, so they decode only with counts opened to the width. Their definition references have no names here.");
    let _ = writeln!(out, "- {unfinished} film(s) were never closed: no chunk directory, so the original cannot open them. The zlib streams are all there except the end of Orders.");
    for (i, r) in rows.iter().enumerate() {
        if r.names.iter().any(|(label, _)| label == "salvaged") {
            let _ = writeln!(out, "- F{:02} is a copy of such a film with a rebuilt directory and its complete records ({} ticks), made to count its orders. The original cannot open the film it comes from.", i + 1, r.ticks);
        }
    }

    let _ = writeln!(out, "\n## Data sets\n");
    let _ = writeln!(out, "| set | ThingData | SharedProperty | AbilityDefinition | Formation | OrganizationLayout | Nation | Faction | films it decodes | films chosen | research commands naming a tech | faction abilities naming the player's own | unread children |");
    let _ = writeln!(out, "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for (i, (label, r)) in sets.iter().enumerate() {
        let n = |reg| r.count(reg).map_or(0, |c| c.0);
        let name = set_name(sets, i, private);
        let chosen: Vec<&&FilmRow> = readable.iter().filter(|f| &f.data_set == label).collect();
        let sum = |f: &dyn Fn(&FilmRow) -> (usize, usize)| {
            chosen.iter().fold((0, 0), |a, r| {
                let (x, y) = f(r);
                (a.0 + x, a.1 + y)
            })
        };
        let research = sum(&|r| r.research_named);
        let abilities = sum(&|r| r.faction_abilities_named);
        let _ = writeln!(
            out,
            "| {name} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {}/{} | {}/{} | {} |",
            n(OBJECT_DEFINITIONS),
            n(SHARED_PROPERTIES),
            n(ABILITIES),
            n(FORMATIONS),
            n(ORGANIZATION_LAYOUTS),
            n(registries::NATIONS),
            n(registries::FACTIONS),
            rows.iter().filter(|f| f.decodes_with.contains(label)).count(),
            chosen.len(),
            research.0,
            research.1,
            abilities.0,
            abilities.1,
            r.notes.len()
        );
    }

    let _ = writeln!(out, "\n## The film version check (0x5A3980) on the v14 target\n");
    let _ = writeln!(out, "With `FilmForcePlayback` false (UVars default) the original compares the header's version string (+0x58, the recording game's `Version` var) with its own by `wcscmp`, then the build (+0x50) with 4129.7 as binary32, and throws `failed_film_wrong_version|s` or `failed_film_wrong_build|g` before CreateGWorld. The v14 target has version 1.0.5.0 and build 4129.7, like every AA.exe of the project.\n");
    let mut checks: BTreeMap<String, usize> = BTreeMap::new();
    for r in &readable {
        let check = if r.v14_check == "Accepted" {
            "accepted".to_owned()
        } else if private {
            r.v14_check.clone()
        } else {
            // A mod's version text may hold a name: its first word says enough.
            let version = r.version.split([' ', '/']).next().unwrap_or("");
            format!("`failed_film_wrong_version`: {version}")
        };
        *checks.entry(check).or_default() += 1;
    }
    let _ = writeln!(out, "| result | films |\n|---|---:|");
    for (check, n) in &checks {
        let _ = writeln!(out, "| {} | {n} |", check.replace('|', "/"));
    }
    if unfinished > 0 {
        let _ = writeln!(out, "| no chunk directory: nothing to open (MAP_HEADER is not found) | {unfinished} |");
    }
    let builds: BTreeSet<u32> = readable.iter().map(|r| r.build.to_bits()).collect();
    let _ = writeln!(out, "\n{} build value(s) in the corpus. Passing the check does not mean the data is there: the next section lists what the target would still lack. Every data set in the corpus has `Version = 1.0.5.0` now, so a film with another version string was recorded on a data set that is no longer in the corpus as it was.", builds.len());

    let _ = writeln!(out, "\n## Films that need data the installed game does not have\n");
    let needs: Vec<&&FilmRow> = readable
        .iter()
        .filter(|r| !r.decodes_on_v14 || !r.missing_on_v14.is_empty() || !r.source_on_v14)
        .collect();
    let _ = writeln!(
        out,
        "{} of {} readable films: the v14 registries do not decode their references, their header names ids v14 lacks, or the map or saved game their creator loads is not on v14. A film holds no map: `KKC_WorldCreatorLoad` opens its source by name when the film plays. The `folder` column says whether a folder the film was found in holds that source.\n",
        needs.len(),
        readable.len()
    );
    let id_of = |r: &FilmRow| -> String {
        if private {
            r.sha256[..10].to_owned()
        } else {
            format!(
                "F{:02}",
                rows.iter().position(|x| x.sha256 == r.sha256).unwrap_or(0) + 1
            )
        }
    };
    let _ = writeln!(out, "| film | decodes on v14 | data it decodes on | ids v14 lacks | source | on v14 | folder |\n|---|---|---|---:|---|---|---|");
    for r in &needs {
        let set = sets
            .iter()
            .position(|s| s.0 == r.data_set)
            .map_or(r.data_set.clone(), |p| set_name(sets, p, private));
        let (on_v14, folder) = if r.source_map.is_empty() {
            ("-", "-")
        } else {
            (
                if r.source_on_v14 { "yes" } else { "no" },
                if r.source_in_folder { "yes" } else { "no" },
            )
        };
        let _ = writeln!(
            out,
            "| {} | {} | {set} | {} | {} | {on_v14} | {folder} |",
            id_of(r),
            r.decodes_on_v14,
            r.missing_on_v14.len(),
            map_label(r, private),
        );
    }

    let _ = writeln!(out, "\n## Films\n");
    if private {
        let _ = writeln!(out, "| film | names | creator | map or rmc | biome / weather | size | heroes | seed | modes creator/header | version | players human+computer+none | ticks | checkpoints | decrees | failures | data set | decodes on v14 | status |");
        let _ = writeln!(out, "|---|---|---|---|---|---|---:|---:|---|---|---|---:|---:|---:|---:|---|---|---|");
    } else {
        let _ = writeln!(out, "| film | creator | map or rmc | biome / weather | size | heroes | modes creator/header | players human+computer+none | ticks | checkpoints | decrees | failures | set | 0x5A3980 on v14 |");
        let _ = writeln!(out, "|---|---|---|---|---|---:|---|---|---:|---:|---:|---:|---|---|");
    }
    for r in rows {
        let (humans, computers, none) = player_split(r);
        let map = map_label(r, private);
        let creator = r.creator_class.replace("KKC_WorldCreator", "");
        let creator = if creator.is_empty() && r.ticks > 0 { "Base".to_owned() } else { creator };
        let heroes = r.num_heroes.map_or("-".to_owned(), |h| h.to_string());
        let set = sets
            .iter()
            .position(|s| s.0 == r.data_set)
            .map_or(r.data_set.clone(), |p| set_name(sets, p, private));
        if private {
            let names = r
                .names
                .iter()
                .map(|(label, file)| format!("{label}/{file}"))
                .collect::<Vec<_>>()
                .join("<br>");
            let _ = writeln!(
                out,
                "| {} | {names} | {creator} | {map} | {} / {} | {}x{} | {heroes} | {} | {}/{} | {} {} | {humans}+{computers}+{none} | {} | {} | {} | {} | {set} | {} | {} |",
                id_of(r),
                r.biome, r.weather, r.extents[0], r.extents[1],
                r.seed, r.creator_mode, r.header_mode, r.version, r.build,
                r.ticks, r.checkpoints, r.decrees, r.failures, r.decodes_on_v14,
                r.status.replace('|', "/")
            );
        } else {
            let check = if r.ticks == 0 {
                "no chunk directory".to_owned()
            } else if r.v14_check == "Accepted" {
                "accepted".to_owned()
            } else {
                "wrong version".to_owned()
            };
            let _ = writeln!(
                out,
                "| {} | {creator} | {map} | {} / {} | {}x{} | {heroes} | {}/{} | {humans}+{computers}+{none} | {} | {} | {} | {} | {set} | {check} |",
                id_of(r),
                r.biome, r.weather, r.extents[0], r.extents[1],
                r.creator_mode, r.header_mode,
                r.ticks, r.checkpoints, r.decrees, r.failures
            );
        }
    }

    let mut by_map: BTreeMap<String, (usize, u64, u64)> = BTreeMap::new();
    for r in &readable {
        let e = by_map.entry(map_label(r, false)).or_default();
        e.0 += 1;
        e.1 += r.ticks as u64;
        e.2 += r.decrees as u64;
    }
    let _ = writeln!(out, "\n## Maps\n\n| map or rmc | films | ticks | decrees |\n|---|---:|---:|---:|");
    let mut maps: Vec<_> = by_map.into_iter().collect();
    maps.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(&b.0)));
    for (map, (films, ticks, decrees)) in maps {
        let _ = writeln!(out, "| {map} | {films} | {ticks} | {decrees} |");
    }
    let mut rules: BTreeMap<(String, u32), usize> = BTreeMap::new();
    for r in &readable {
        for (key, kind, value) in &r.rules {
            let _ = kind;
            *rules.entry((key.clone(), *value)).or_default() += 1;
        }
    }
    let _ = writeln!(out, "\n## World rules in the creators\n\n| rule | raw value | films |\n|---|---:|---:|");
    for ((key, value), n) in rules {
        let _ = writeln!(out, "| {key} | {value:#x} | {n} |");
    }

    coverage_table(&mut out, &totals.orders);
    let _ = writeln!(out, "\n## Which decrees carry a human player\n");
    let _ = writeln!(out, "The ordering player of a decree is the last 298 SetPlayer (Admin+0x284, 0x75D710). 311 sync checkpoints and 328 renames run under the hosting player, so their human count says nothing about who gave them. These decrees do not occur at all in the corpus: {}.", zero_decrees(&totals.decrees).join(", "));
    tally_table(&mut out, "Decrees", &totals.decrees, usize::MAX);
    tally_table(&mut out, "ActorCommands (86 and 87)", &totals.commands, usize::MAX);
    tally_table(&mut out, "Abilities (67 TellActorAbility, 70 FactionUseAbility) on the named data sets", &totals.abilities, 60);
    tally_table(&mut out, "Formations named by commands", &totals.formations, usize::MAX);
    tally_table(&mut out, "Properties named by commands", &totals.properties, 40);
    tally_table(&mut out, "Objects named by commands", &totals.objects, 40);

    if private {
        let _ = writeln!(out, "\n## Proof per film\n\n| film | sync intervals recomputed | of | checkpoint intervals | short intervals | clock offset | research naming a tech | faction abilities naming the player's own | ids missing in its data set | ids missing on v14 |\n|---|---:|---:|---:|---:|---:|---:|---:|---|---|");
        for r in rows {
            if r.ticks == 0 {
                continue;
            }
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {}/{} | {}/{} | {} | {} |",
                id_of(r),
                r.sync_proof.0,
                r.sync_proof.1,
                r.sync_proof.2,
                r.sync_proof.3,
                r.sync_clock_offset.map_or("-".to_owned(), |o| o.to_string()),
                r.research_named.0,
                r.research_named.1,
                r.faction_abilities_named.0,
                r.faction_abilities_named.1,
                r.missing_in_data_set.join(", "),
                r.missing_on_v14.join(", "),
            );
        }
        let _ = writeln!(out, "\n## Random-map settings\n");
        for r in rows {
            if let Some((terrain, objects)) = &r.random_settings {
                let _ = writeln!(
                    out,
                    "- {} {} seed {} heroes {}: terrain {}; objects {}",
                    id_of(r),
                    r.rmc.as_deref().unwrap_or(""),
                    r.seed,
                    r.num_heroes.unwrap_or(0),
                    terrain.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(", "),
                    objects.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(", "),
                );
            }
        }
        let _ = writeln!(out, "\n## Players\n");
        for r in rows {
            if r.ticks == 0 {
                continue;
            }
            let _ = writeln!(
                out,
                "- {}: {}; renamed {}",
                id_of(r),
                r.players
                    .iter()
                    .map(|p| format!(
                        "{} {:?} k{} {}",
                        p.0,
                        p.1,
                        p.2.map_or("-".to_owned(), |k| k.to_string()),
                        if p.3 { "computer" } else { "human" }
                    ))
                    .collect::<Vec<_>>()
                    .join(", "),
                r.renames
                    .iter()
                    .map(|(k, n)| format!("{k}={n:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            for (player, computer, set, add, clear, commands) in &r.per_player {
                let _ = writeln!(
                    out,
                    "  - player {player} {}: 333 {set}, 16 {add}, 18 {clear}, commands {commands}",
                    if *computer { "computer" } else { "human" }
                );
            }
        }
        let _ = writeln!(out, "\n## Kingdoms\n");
        for r in rows {
            if r.ticks == 0 {
                continue;
            }
            let _ = writeln!(
                out,
                "- {}: {}",
                id_of(r),
                r.kingdoms
                    .iter()
                    .map(|(k, n, f, t)| format!("{k} {n}/{f} {t}"))
                    .collect::<Vec<_>>()
                    .join("; ")
            );
        }
    }
    out
}

/// The name of data set `i` in a report: its label, or an anonymous one.
fn set_name(sets: &[(String, Registries)], i: usize, private: bool) -> String {
    let label = &sets[i].0;
    if private || label == "v14" {
        return label.clone();
    }
    format!("mod {i}")
}

fn player_split(r: &FilmRow) -> (usize, usize, usize) {
    let none = r.players.iter().filter(|p| p.2.is_none()).count();
    let computers = r.players.iter().filter(|p| p.3).count();
    (r.players.len() - none - computers, computers, none)
}

/// The gameplay decrees that never occur in the census.
fn zero_decrees(decrees: &BTreeMap<String, Tally>) -> Vec<String> {
    [
        (65, "TellActorAttack"),
        (66, "TellActorMove"),
        (67, "TellActorAbility"),
        (68, "GamePlayerKingdom"),
        (77, "PublicChat"),
        (78, "PrivateChat"),
        (79, "TeamChat"),
        (73, "SetGameSpeed"),
        (75, "TogglePaused"),
        (76, "SetPaused"),
        (80, "SpeedUp"),
        (81, "SpeedDown"),
    ]
    .iter()
    .filter(|(id, _)| !decrees.keys().any(|k| k.trim_start().starts_with(&format!("{id} "))))
    .map(|(id, name)| format!("{id} {name}"))
    .collect()
}
