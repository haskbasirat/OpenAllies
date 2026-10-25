//! Random map generator parity (roadmap item 15): aa-play's `mapgen` against
//! the original's world-RNG draw sequence for the QA films' random maps,
//! recorded by NativeSampler while the original plays each film (the trace
//! stays in the QA copy, never in the repo).
//!
//! The world stream is one LCG, so the states of the n-th draw always agree;
//! what the harness compares is the call site of every draw (the return
//! address after the original's 0x85D840 call, or after its unit/modulo
//! wrapper) against the site aa-play labels the same draw with. The first
//! site that differs is the first point where the port's control flow or
//! arithmetic left the original's.
//!
//! `AA_GAME_ROOT='E:\Games\Axis and Alies' cargo test -p aa-parity --features play --test rmg_parity -- --ignored --nocapture`
#![cfg(feature = "play")]
use aa_parity::Film;
use aa_parity::registries::Registries;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const QA: &str = r"E:\Games\Axis and Alies\reconstruction\qa-runs\compare-20261022";
const FILM_2: &str = r"game\Profile\data\Films\archive-qa-20261022\LastFilm_2.RFM";
const FILM_2_TRACE: &str = r"traces\film2-rmg.txt";
/// The second capture of film 2, with the generator's log lines and the world
/// dump (one draw line was lost in it, so draws come from the first).
const FILM_2_LOG: &str = r"traces\film2-rmg-L.txt";
/// KKC_RandomMapGenerator::Generate 0x5925E0 (1344 bytes).
const RMG_MAIN: std::ops::Range<u32> = 0x5925E0..0x592B20;

fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"))
}

/// One native draw: the state before it, its call site and the stack chain.
struct NativeDraw {
    state: u32,
    site: u32,
    chain: Vec<u32>,
}

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s, 16).unwrap_or(0)
}

/// The D lines of a NativeSampler trace up to the session-launch sync reset
/// (the first `SyncReset_77D900` reached from 0x775D4F).
fn native_draws(path: &Path) -> Vec<NativeDraw> {
    let text = std::fs::read_to_string(path).unwrap();
    let mut out = Vec::new();
    for line in text.lines() {
        if line.starts_with("M ") && line.contains("SyncReset_77D900") && line.contains("775d4f") {
            break;
        }
        if !line.starts_with("D ") {
            continue;
        }
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() < 13 {
            continue;
        }
        out.push(NativeDraw { state: hex(p[6]), site: hex(p[10]), chain: p[12].split(',').map(hex).collect() });
    }
    out
}

/// The NativeSampler `L` lines of the generator (logger 0x404F60 / 0x404F30)
/// that aa-play's `GeneratedMap::log` mirrors, as (message without numbers,
/// the doubles' bits): "RMG: terrain %s normalized weight %f" and "balance
/// iteration %d score %f" (integers as doubles).
fn native_log_lines(path: &Path) -> Vec<(String, Vec<u64>)> {
    let text = std::fs::read_to_string(path).unwrap();
    let mut out = Vec::new();
    for line in text.lines() {
        if !line.starts_with("L ") {
            continue;
        }
        let Some((_, msg)) = line.split_once(".cpp:") else { continue };
        let msg = msg.split_once(' ').map_or("", |m| m.1);
        let bits = |s: &str| -> Option<u64> { u64::from_str_radix(s.split_once('[')?.1.trim_end_matches(']'), 16).ok() };
        if let Some(rest) = msg.strip_prefix("RMG: terrain ") {
            let mut w = rest.split_whitespace();
            let ids = w.next().unwrap_or("");
            let v = rest.rsplit(' ').next().and_then(bits);
            if let Some(v) = v {
                out.push((format!("terrain {ids} normalized weight"), vec![v]));
            }
        } else if let Some(rest) = msg.strip_prefix("marker ") {
            // "marker %d placed %f,%f" / "marker %d created at %f,%f".
            let mut w = rest.splitn(2, ' ');
            let index: f64 = w.next().and_then(|s| s.parse().ok()).unwrap_or(-1.0);
            let rest = w.next().unwrap_or("");
            let (what, coords) = if let Some(c) = rest.strip_prefix("placed ") {
                ("marker placed", c)
            } else if let Some(c) = rest.strip_prefix("created at ") {
                ("marker created", c)
            } else {
                continue;
            };
            let xy: Vec<u64> = coords.split(',').filter_map(bits).collect();
            if xy.len() == 2 {
                out.push((what.into(), vec![index.to_bits(), xy[0], xy[1]]));
            }
        } else if let Some(rest) = msg.strip_prefix("balance iteration ") {
            let mut w = rest.split_whitespace();
            let iter: f64 = w.next().and_then(|s| s.parse().ok()).unwrap_or(-1.0);
            let v = rest.rsplit(' ').next().and_then(bits);
            if let Some(v) = v {
                out.push(("balance iteration".into(), vec![iter.to_bits(), v]));
            }
        }
    }
    out
}

/// The stage call in 0x5925E0 a draw belongs to (the return address inside
/// it on the chain), inheriting the previous one when the chain is too short.
fn stages(draws: &[NativeDraw]) -> Vec<u32> {
    let mut stage = 0;
    draws
        .iter()
        .map(|d| {
            if let Some(&s) = std::iter::once(&d.site).chain(&d.chain).find(|a| RMG_MAIN.contains(a)) {
                stage = s;
            }
            stage
        })
        .collect()
}

fn film_2_params(rmg: &aa_play::mapgen::RandomMapData, film: &Film) -> aa_play::mapgen::RandomMapParams {
    let rmc = film.header.rmc.clone().unwrap();
    let info = rmg.catalog().rmcs.into_iter().find(|r| r.ids == rmc).unwrap();
    aa_play::mapgen::RandomMapParams::defaults(&info, 2, u64::from(film.header.seed))
}

/// The world NativeSampler dumped after film 2's generation (`NS_WORLD=1`:
/// `VT` variation index lines, `TL` tile rows of `index:flags:area`, `HB` /
/// `HO` base and overlay height rows of binary32 bits, `TH` things).
struct NativeWorld {
    width: usize,
    tiles: Vec<String>,
    flags: Vec<u32>,
    base: Vec<u32>,
    overlay: Vec<u32>,
    things: Vec<(String, [u32; 3])>,
}

fn native_world(path: &Path) -> NativeWorld {
    let text = std::fs::read_to_string(path).unwrap();
    let mut vt: BTreeMap<usize, String> = BTreeMap::new();
    let mut w = NativeWorld { width: 0, tiles: Vec::new(), flags: Vec::new(), base: Vec::new(), overlay: Vec::new(), things: Vec::new() };
    for line in text.lines() {
        let mut p = line.splitn(2, ' ');
        let (tag, rest) = (p.next().unwrap_or(""), p.next().unwrap_or(""));
        match tag {
            "VT" => {
                let f: Vec<&str> = rest.split_whitespace().collect();
                vt.insert(f[0].parse().unwrap(), f.get(2).copied().unwrap_or("").to_ascii_lowercase());
            }
            "TL" => {
                let (_, cells) = rest.split_once(' ').unwrap();
                let row: Vec<&str> = cells.split(',').collect();
                w.width = row.len();
                for c in row {
                    let f: Vec<&str> = c.split(':').collect();
                    w.tiles.push(vt[&f[0].parse::<usize>().unwrap()].clone());
                    w.flags.push(u32::from_str_radix(f[1], 16).unwrap());
                }
            }
            "HB" | "HO" => {
                let (_, cells) = rest.split_once(' ').unwrap();
                let v = if tag == "HB" { &mut w.base } else { &mut w.overlay };
                v.extend(cells.split(',').map(hex));
            }
            "TH" => {
                let f: Vec<&str> = rest.split_whitespace().collect();
                if let Some(p) = f.iter().find_map(|s| s.strip_prefix("p=")) {
                    let c: Vec<u32> = p.split(',').map(hex).collect();
                    w.things.push((f[1].to_ascii_lowercase(), [c[0], c[1], c[2]]));
                }
            }
            _ => {}
        }
    }
    w
}

/// Film 2's generated world against the original's (tiles, heights,
/// things), after the session start.
#[test]
#[ignore = "reads QA film 2, its native world dump and the original data; set AA_GAME_ROOT"]
fn film_2_world() {
    let qa = Path::new(QA);
    let native = native_world(&qa.join(FILM_2_LOG));
    let film = Film::read(&qa.join(FILM_2), &Registries::builtin()).unwrap();
    let draws = native_draws(&qa.join(FILM_2_TRACE));
    let stage = stages(&draws);
    let start = stage.iter().position(|&s| s != 0).unwrap();
    let rmg = aa_play::mapgen::RandomMapData::load(&game_root()).unwrap();
    let params = film_2_params(&rmg, &film);
    let mut log = vec![aa_simulation::NativeRng::default().state];
    log.extend(draws[..start].iter().map(|d| d.state));
    let generated = rmg.generate_from(&params, &aa_play::mapgen::StreamStart { state: draws[start].state, log }).unwrap();
    let t = &generated.terrain;
    let ours: Vec<String> = t.tiles.iter().map(|v| t.palette[v].id.to_ascii_lowercase()).collect();
    assert_eq!(ours.len(), native.tiles.len(), "tile count");
    let w = native.width;
    let diff: Vec<usize> = (0..ours.len()).filter(|&i| ours[i] != native.tiles[i]).collect();
    eprintln!("tiles: {} of {} differ", diff.len(), ours.len());
    for &i in diff.iter().take(12) {
        eprintln!("  ({}, {}) native {} aa-play {} flags {:x}", i % w, i / w, native.tiles[i], ours[i], native.flags[i]);
    }
    if let Ok(region) = std::env::var("AA_RMG_TILES") {
        // "x0,y0,x1,y1": every tile of the region, native / aa-play.
        let r: Vec<usize> = region.split(',').filter_map(|s| s.parse().ok()).collect();
        for y in r[1]..=r[3] {
            for x in r[0]..=r[2] {
                let i = y * w + x;
                eprintln!("  tile ({x}, {y}) native {} {:x} aa-play {}", native.tiles[i], native.flags[i], ours[i]);
            }
        }
    }
    if !diff.is_empty() && std::env::var_os("AA_RMG_TILEMAP").is_some() {
        for y in 0..ours.len() / w {
            let row: String = (0..w).map(|x| if ours[y * w + x] == native.tiles[y * w + x] { '.' } else { 'X' }).collect();
            eprintln!("{row}");
        }
    }
    for (name, ours, theirs) in [("base", &t.heights, &native.base), ("overlay", &t.mountain_heights, &native.overlay)] {
        let n = ours.iter().zip(theirs.iter()).filter(|(a, b)| a.to_bits() != **b).count();
        let max = ours.iter().zip(theirs.iter()).map(|(a, b)| (a - f32::from_bits(*b)).abs()).fold(0.0f32, f32::max);
        eprintln!("{name} heights: {n} of {} differ, max |diff| {max}", theirs.len());
    }
    // The first created things in creation (GTID) order: the start
    // locations, the required objects (their placeholders replaced by the
    // nations' actors at the session start) and the villages.
    let theirs: Vec<&(String, [u32; 3])> = native.things.iter().filter(|(id, _)| id != "team" && id != "kingdom").take(24).collect();
    eprintln!("first things (native at session start / aa-play generated):");
    let mut same = 0;
    for (i, (id, p)) in theirs.iter().enumerate() {
        let ours = generated.placed.get(i).map(|o| (o.thing_id.clone(), o.position));
        let np = [f32::from_bits(p[0]), f32::from_bits(p[1])];
        let exact = ours.as_ref().is_some_and(|(_, q)| q[0].to_bits() == p[0] && q[1].to_bits() == p[1]);
        same += usize::from(exact);
        eprintln!("  {i:2} {id:32} ({:9.4}, {:9.4})  {:?}{}", np[0], np[1], ours, if exact { "  exact" } else { "" });
    }
    eprintln!("  {same} of {} at the native position", theirs.len());
    let mut counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for (id, _) in &native.things {
        counts.entry(id.clone()).or_default().0 += 1;
    }
    for p in &generated.placed {
        counts.entry(p.thing_id.to_ascii_lowercase()).or_default().1 += 1;
    }
    eprintln!("things (native after session start / aa-play generated):");
    for (id, (n, o)) in &counts {
        if n != o {
            eprintln!("  {id:40} {n:5} {o:5}");
        }
    }
}

/// Where the generator puts starts and villages for the default southern
/// Europe map (diagnostic for the core tests that generate seed 3).
#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn seed_3_starts_and_villages() {
    let rmg = aa_play::mapgen::RandomMapData::load(&game_root()).unwrap();
    let info = rmg.catalog().rmcs.into_iter().find(|r| r.ids == "rmc_southerneurope").unwrap();
    let generated = rmg.generate(&aa_play::mapgen::RandomMapParams::defaults(&info, 2, 3)).unwrap();
    let starts: Vec<[f32; 2]> = generated.placed.iter().filter(|p| p.thing_id.contains("start") || p.thing_id.contains("corps")).map(|p| [p.position[0], p.position[1]]).collect();
    for p in &generated.placed {
        if p.thing_id.contains("start") || p.thing_id.contains("corps") || p.thing_id.contains("village") || p.thing_id.contains("depot") {
            let near = starts.iter().map(|s| ((s[0] - p.position[0]).powi(2) + (s[1] - p.position[1]).powi(2)).sqrt()).fold(f32::MAX, f32::min);
            eprintln!("{:40} {:7.1} {:7.1} owner {:?} nearest start/corps {near:6.1}", p.thing_id, p.position[0], p.position[1], p.owner_slot);
        }
    }
}

#[test]
#[ignore = "reads QA film 2, its native trace and the original data; set AA_GAME_ROOT"]
fn film_2_draw_sites() {
    let root = game_root();
    let qa = Path::new(QA);
    let film = Film::read(&qa.join(FILM_2), &Registries::builtin()).unwrap();
    let native = native_draws(&qa.join(FILM_2_TRACE));
    let stage = stages(&native);
    // The generator starts at the first draw made under 0x5925E0; before it
    // the creator's kingdoms and their random picks drew.
    let start = stage.iter().position(|&s| s != 0).expect("no generator draw in the trace");
    let rmg = aa_play::mapgen::RandomMapData::load(&root).unwrap();
    let params = film_2_params(&rmg, &film);
    let mut log = vec![aa_simulation::NativeRng::default().state];
    log.extend(native[..start].iter().map(|d| d.state));
    let from = aa_play::mapgen::StreamStart { state: native[start].state, log };
    let clock = std::time::Instant::now();
    let (generated, ours) = rmg.generate_traced(&params, Some(&from)).unwrap();
    eprintln!(
        "{} seed {:#x}: native {} generator draws after {start} prefix draws; aa-play {} in {:?}, {} placed things",
        params.rmc,
        film.header.seed,
        native.len() - start,
        ours.len(),
        clock.elapsed(),
        generated.placed.len()
    );
    let native = &native[start..];
    let stage = &stage[start..];
    // Per stage: native draws, and how many of ours line up from its start.
    let mut first_mismatch = None;
    for (i, (d, o)) in native.iter().zip(&ours).enumerate() {
        assert_eq!(d.state, o.0, "stream states differ at draw {i}: the prefix or the LCG is off");
        if d.site != o.1 {
            first_mismatch = Some(i);
            break;
        }
    }
    let matched = first_mismatch.unwrap_or(native.len().min(ours.len()));
    let mut per_stage: BTreeMap<u32, (usize, usize)> = BTreeMap::new();
    for (i, s) in stage.iter().enumerate() {
        let e = per_stage.entry(*s).or_default();
        e.0 += 1;
        if i < matched {
            e.1 += 1;
        }
    }
    eprintln!("stage (return site in 0x5925E0): native draws, matched");
    for (s, (n, m)) in &per_stage {
        eprintln!("  {s:06x}: {n:7} {m:7}{}", if m == n { "  exact" } else { "" });
    }
    match first_mismatch {
        None if native.len() == ours.len() => eprintln!("all {} draw sites match", ours.len()),
        None => eprintln!("the first {matched} sites match; lengths differ: native {} aa-play {}", native.len(), ours.len()),
        Some(i) => {
            eprintln!("first mismatch at generator draw {i} (stage {:06x}):", stage[i]);
            for j in i.saturating_sub(6)..(i + 6).min(native.len()) {
                let o = ours.get(j).map_or("-".to_string(), |o| format!("{:06x}", o.1));
                let chain: Vec<String> = native[j].chain.iter().take(4).map(|c| format!("{c:06x}")).collect();
                eprintln!("  {j:7} native {:06x} [{}]  aa-play {o}{}", native[j].site, chain.join(","), if j == i { "  <==" } else { "" });
            }
        }
    }
    // The original's RMG log lines against the generator's log.
    let native_log = native_log_lines(&qa.join(FILM_2_LOG));
    let mut log_ok = 0;
    let mut log_bad = Vec::new();
    for (i, (what, value_bits)) in native_log.iter().enumerate() {
        let ours = generated.log.iter().filter(|l| l.what == *what).nth(native_log[..i].iter().filter(|(w, _)| w == what).count());
        match ours {
            Some(l) if l.values.iter().map(|v| v.to_bits()).collect::<Vec<_>>() == *value_bits => log_ok += 1,
            Some(l) => log_bad.push(format!("{what}: native {:?} aa-play {:?}", value_bits.iter().map(|b| f64::from_bits(*b)).collect::<Vec<_>>(), l.values)),
            None => {}
        }
    }
    eprintln!("RMG log lines matched {log_ok}, differing {}", log_bad.len());
    for b in log_bad.iter().take(8) {
        eprintln!("  {b}");
    }
    if std::env::var_os("AA_RMG_HIST").is_some() {
        let mut h: BTreeMap<u32, (usize, usize)> = BTreeMap::new();
        for d in native {
            h.entry(d.site).or_default().0 += 1;
        }
        for o in &ours {
            h.entry(o.1).or_default().1 += 1;
        }
        eprintln!("site: native aa-play");
        for (s, (n, o)) in h {
            eprintln!("  {s:06x}: {n:7} {o:7}{}", if n == o { "" } else { "  *" });
        }
    }
}
