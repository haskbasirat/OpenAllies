//! The computer player beside a real film: the film plays through the playback
//! library, and aa-play's SAI thinks on the film's world without its orders being
//! applied. What it would have ordered is set against what the film's computers
//! did: per kingdom the command names and flags over the whole run, and with
//! `AA_SHADOW_SHOW=from:to` the two timelines.
//!
//! ```text
//! AA_GAME_ROOT=<game root> AA_SHADOW_TICKS=4000 cargo test -p aa-parity --features play --test sai_shadow --release -- --ignored --nocapture
//! ```
//! `AA_SHADOW_FILM=<path>` names another film. Prints only: the SAI's RNG is seeded
//! from a clock natively (0x7A0AA0), so no two games of the original give the same
//! orders either, and an order that differs proves nothing by itself.
//!
//! `AA_SHADOW_GOALS=<file>` with `AA_SHADOW_LOG` sets the goals of every think against the
//! original's goal log of the film's game (`SAIDebug=1 GoalPriorityCSV=<file>` on its command
//! line, research/byte-exact/COMPUTER-PLAYER.md). `AA_SHADOW_LIVE` lets the computers play the
//! film's map themselves, with their orders applied.
#![cfg(feature = "play")]
use aa_parity::classify;
use aa_parity::playback::{FilmData, SyncMode, open_film};
use aa_parity::replay;
use aa_parity::{Decree, Film};
use aa_play::orders::ActorOrder;
use aa_play::{AbilityTarget, Command, Controller, Unit, World};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"))
}

/// The name and flags a command takes in `OrderForm::Computer`.
fn shape(world: &World, c: &Command) -> Option<(u32, &'static str, u32, String)> {
    let slot = |u: &Unit| aa_play::player_orders::native::slot_of(world, *u).map_or_else(|| format!("{u:?}"), |s| s.to_string());
    let unit = |u: &[Unit]| u.iter().map(&slot).collect::<Vec<_>>().join(",");
    let actor = |a: &aa_play::ActorId| world.ids.actor(*a).map_or(0, |g| g.raw());
    Some(match c {
        Command::Move { kingdom, units, to, attack_move } => (*kingdom, if *attack_move { "kill" } else { "move" }, if *attack_move { 0x44 } else { 0x5c }, format!("{} at {to:?}", unit(units))),
        Command::Attack { kingdom, units, target } => (*kingdom, "kill", 0x41, format!("{} word {:#x}", unit(units), actor(target))),
        Command::Follow { kingdom, units, target } => (*kingdom, "move", 0x59, format!("{} word {:#x}", unit(units), actor(target))),
        Command::Stop { kingdom, units } => (*kingdom, "stop", 0x40, unit(units)),
        Command::Recruit { kingdom, building, def } => (*kingdom, "recruit", 0x62, format!("{} def {def}", slot(&Unit::Actor(*building)))),
        Command::Deploy { kingdom, truck, at, .. } => (*kingdom, "unpack", 0x44, format!("{} at {at:?}", slot(&Unit::Actor(*truck)))),
        Command::Research { kingdom, building, property } => (*kingdom, "research", 0x140, format!("{building} {property}")),
        Command::Attach { kingdom, org, hq } => (*kingdom, "hq_attach", 0x41, format!("{org} to {hq}")),
        Command::SetFormation { kingdom, orgs, formation } => (*kingdom, "set_formation", 0xc0, format!("{} {formation}", orgs.iter().map(|o| slot(&Unit::Org(*o))).collect::<Vec<_>>().join(","))),
        Command::Bombard { kingdom, units, target } => (*kingdom, "bombard", if matches!(target, AbilityTarget::Actor(_)) { 0x41 } else { 0x44 }, unit(units)),
        Command::AirAttack { kingdom, building, target } => (*kingdom, "air_attack", if matches!(target, AbilityTarget::Actor(_)) { 0x41 } else { 0x44 }, format!("{building}")),
        Command::AirRecon { kingdom, building, .. } => (*kingdom, "air_recon", 0x44, format!("{building}")),
        Command::Order { kingdom, units, order } => match order {
            ActorOrder::Guard { target, at, .. } => (*kingdom, "guard", if target.is_some() { 0x49 } else { 0x4c }, format!("{} guard {target:?} {at:?}", unit(units))),
            ActorOrder::Retreat { at, .. } => (*kingdom, "retreat", 0x44, format!("{} at {at:?}", unit(units))),
            ActorOrder::Rout => (*kingdom, "rout", 0x40, unit(units)),
            ActorOrder::Explore { at, .. } => (*kingdom, "explore", 0x44, format!("{} at {at:?}", unit(units))),
            ActorOrder::MoveOnto => return None,
            other => (*kingdom, "other", 0, format!("{other:?}")),
        },
        Command::UseAbility { kingdom, ability, .. } => (*kingdom, "use_ability", 0, format!("{ability}")),
        other => (u32::MAX, "other", 0, format!("{other:?}")),
    })
}

/// A kingdom a computer plays.
fn computers_index(world: &World, kingdom: u32) -> bool {
    world.kingdoms.iter().any(|k| k.id == kingdom && matches!(k.controller, Controller::Computer(_)))
}

fn run(path: &PathBuf) {
    let data = FilmData::new(game_root());
    let (mut world, mut player): (World, _) = open_film(path, &data, SyncMode::Report).unwrap_or_else(|e| panic!("open_film: {e:?}"));
    let film: Film = player.film().clone();
    // AA_SHADOW_CHECKPOINT=<index>: the tick of the checkpoint the original stopped at.
    if let Some(index) = std::env::var("AA_SHADOW_CHECKPOINT").ok().and_then(|v| v.parse::<i64>().ok()) {
        for (n, c) in film.checkpoints.iter().enumerate() {
            if i64::from(c.count) == index {
                println!("checkpoint {index} is number {} of {} at record {:?}", n + 1, film.checkpoints.len(), c);
            }
        }
    }
    let registries = aa_parity::registries::Registries::load_v14(&game_root(), true).unwrap();
    let computers = replay::computer_kingdoms(&film);
    let sai = Arc::new(aa_play::ai::AiData::load(&game_root()).unwrap());
    aa_play::ai::attach(&mut world, sai);
    if let Some(seed) = std::env::var("AA_SHADOW_SEED").ok().and_then(|v| v.parse().ok()) {
        aa_play::ai::set_seed(&mut world, seed);
    }
    world.ai.logging = std::env::var_os("AA_SHADOW_LOG").is_some();
    // The film's generals are the kingdoms' profiles, as the session's AI player names them.
    for (k, kingdom) in world.kingdoms.clone().iter().enumerate() {
        let slot = world.ids.kingdom_slot(k as u32).and_then(|s| world.map.slots.get(s)).map(|s| s.id.clone()).unwrap_or_default();
        if computers.iter().any(|c| c.eq_ignore_ascii_case(&slot)) {
            world.kingdoms[k].controller = Controller::Computer(1);
            if let Some(g) = aa_play::special_ops::general(&world, kingdom.id) {
                let id = format!("{}_sai", g.id);
                aa_play::ai::set_profile(&mut world, kingdom.id, &id);
            }
        }
    }
    let show = std::env::var("AA_SHADOW_SHOW").ok().and_then(|v| {
        let (a, b) = v.split_once(':')?;
        Some((a.parse::<usize>().ok()?, b.parse::<usize>().ok()?))
    });
    let ticks = std::env::var("AA_SHADOW_TICKS").ok().and_then(|v| v.parse().ok()).unwrap_or(4000usize).min(film.ticks.len());
    let mut ours: BTreeMap<(u32, &'static str, u32), usize> = BTreeMap::new();
    let mut real: BTreeMap<(u32, String, u16), usize> = BTreeMap::new();
    let mut timeline: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    // AA_SHADOW_LIVE: the computers play the film's map themselves, their orders applied and
    // the film's left out: what they do by themselves against what the film's computers did.
    let live = std::env::var_os("AA_SHADOW_LIVE").is_some();
    for t in 0..ticks {
        if live {
            world.step(Vec::new());
            for e in &world.events {
                match e {
                    aa_play::Event::Captured { actor, kingdom } => println!("live {:7.1} k{kingdom} captured {}", world.time, world.data.def(world.actors[*actor as usize].def).id),
                    aa_play::Event::Recruited { kingdom, org, actor } => {
                        let def = org.map(|o| world.orgs[o as usize].def).or(actor.map(|a| world.actors[a as usize].def));
                        println!("live {:7.1} k{kingdom} recruited {}", world.time, def.map_or("?", |d| world.data.def(d).id.as_str()));
                    }
                    aa_play::Event::Deployed { building } => println!("live {:7.1} deployed {}", world.time, world.data.def(world.actors[*building as usize].def).id),
                    aa_play::Event::Refused { kingdom, reason } if computers_index(&world, *kingdom) => println!("live {:7.1} k{kingdom} refused {reason}", world.time),
                    _ => {}
                }
            }
            if t % (60 * 16) == 0 {
                for k in world.kingdoms.iter() {
                    let orgs = world.orgs.iter().filter(|o| o.alive && o.kingdom == k.id).count();
                    println!("live {:7.1} k{} orgs {orgs} stock {:?}", world.time, k.id, world.books.kingdom(k.id).map(|b| b.stockpiled.iter().take(3).map(|x| *x as i32).collect::<Vec<_>>()));
                }
            }
            continue;
        }
        player.step(&mut world);
        for e in &world.events {
            match e {
                aa_play::Event::Refused { kingdom, reason } => timeline.entry(t).or_default().push(format!("world refused k{kingdom} {reason}")),
                aa_play::Event::Deployed { building } => timeline.entry(t).or_default().push(format!("world deployed {}", world.data.def(world.actors[*building as usize].def).id)),
                aa_play::Event::Captured { actor, kingdom } => timeline.entry(t).or_default().push(format!("world k{kingdom} captured {}", world.data.def(world.actors[*actor as usize].def).id)),
                _ => {}
            }
        }
        for d in &film.ticks[t].decrees {
            let p = d.player.unwrap_or(0);
            let (w, target) = match &d.decree {
                Decree::SelectionSet { slot } => {
                    timeline.entry(t).or_default().push(format!("film p{p} sel {slot}"));
                    continue;
                }
                Decree::SelectionAdd { slot } => {
                    timeline.entry(t).or_default().push(format!("film p{p} add {slot}"));
                    continue;
                }
                Decree::TellSelectedCommand(w) => (w, String::new()),
                Decree::TellActorCommand { command, .. } => (command, format!("{:?}", d.decree).chars().filter(char::is_ascii_digit).take(0).collect()),
                _ => continue,
            };
            let name = classify::actor_command(&d.decree).map_or_else(|| "default".to_owned(), |(_, n, _)| n.to_string());
            let object = w.object.and_then(|o| registries.object_ids.get(o as usize).cloned()).unwrap_or_default();
            *real.entry((p, name.clone(), w.flags)).or_default() += 1;
            let formation = w.parameter.and_then(|f| registries.formation_ids.get(f as usize).cloned()).unwrap_or_default();
            // What the word names: an organization, a member, a structure.
            let thing = match world.ids.thing_at_slot(w.word as u16) {
                Some(aa_play::ids::Thing::Actor(a)) => format!("actor {}", world.data.def(world.actors[a as usize].def).id),
                Some(aa_play::ids::Thing::Org(o)) => format!("org {}", world.data.def(world.orgs[o as usize].def).id),
                other => format!("{other:?}"),
            };
            timeline.entry(t).or_default().push(format!("film p{p} {name} {:#x} word {:#x} ({thing}) pos {:?} {formation} {object} {target}", w.flags, w.word, w.position.map(f32::from_bits)));
        }
        for c in std::mem::take(&mut world.sai_orders) {
            let Some((k, name, flags, text)) = shape(&world, &c) else { continue };
            *ours.entry((k, name, flags)).or_default() += 1;
            timeline.entry(t).or_default().push(format!("ours k{k} {name} {flags:#x} {text}"));
        }
        world.trace.digests.clear();
    }
    println!("film {} over {ticks} ticks; checkpoints {:?}", path.display(), player.stats().first_mismatch);
    println!("-- the film's orders by player");
    for ((p, n, f), c) in &real {
        println!("real p{p:<2} {n:24} {f:#05x} {c}");
    }
    println!("-- the SAI's orders by kingdom");
    for ((k, n, f), c) in &ours {
        println!("ours k{k:<2} {n:24} {f:#05x} {c}");
    }
    if let Some((a, b)) = show {
        for (t, lines) in timeline.range(a..b) {
            for l in lines {
                println!("{t:6} {l}");
            }
        }
    }
    if std::env::var_os("AA_SHADOW_LOG").is_some() {
        for e in world.ai.log.iter() {
            println!("sai {:8.2} k{} {}", e.time, e.kingdom, e.text);
        }
    }
    if let Some(log) = std::env::var_os("AA_SHADOW_GOALS") {
        goal_log_table(&PathBuf::from(log), &world);
    }
}

/// One row of a goal log: the type, the region its label names, the class base, the inertia.
#[derive(Clone)]
struct Row {
    kind: String,
    region: Option<u32>,
    raw: f32,
    inertia: f32,
}

fn region_of(label: &str) -> Option<u32> {
    let (_, r) = label.split_once("region #")?;
    r.chars().take_while(char::is_ascii_digit).collect::<String>().parse().ok()
}

/// `AA_SHADOW_GOALS=<the original's GoalPriorityCSV of the film's game>`, with `AA_SHADOW_LOG`:
/// think by think how many goals of each type the original and we have, and over all thinks how
/// far the class base of the combat goals is, region by region.
fn goal_log_table(path: &PathBuf, world: &World) {
    let text = std::fs::read_to_string(path).expect("the goal log");
    let mut native: BTreeMap<u32, Vec<Row>> = BTreeMap::new();
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split(',').collect();
        let n = f.len();
        if n < 12 {
            continue;
        }
        let think = f[1].parse().unwrap_or(0);
        native.entry(think).or_default().push(Row { kind: f[3].to_owned(), region: region_of(&f[4..n - 7].join(",")), raw: f[n - 7].parse().unwrap_or(0.0), inertia: f[n - 2].parse().unwrap_or(0.0) });
    }
    let mut ours: BTreeMap<u32, Vec<Row>> = BTreeMap::new();
    let (mut think, mut last) = (0, f32::NAN);
    for e in world.ai.log.iter() {
        let Some(row) = e.text.strip_prefix("csv ") else { continue };
        if e.time != last {
            think += 1;
            last = e.time;
        }
        let f: Vec<&str> = row.split('|').collect();
        ours.entry(think).or_default().push(Row { kind: f[0].to_owned(), region: region_of(f[1]), raw: f[2].parse().unwrap_or(0.0), inertia: f[7].parse().unwrap_or(0.0) });
    }
    let kinds = ["ATTACK_REGION", "DEFEND_REGION", "AIR_ATTACK", "RECOVER", "EXPLORE", "UNPACK", "RECRUIT", "RESEARCH", "BOMBARD"];
    println!("think | {} (the original's / ours)", kinds.join(" "));
    let (mut both, mut only_native, mut only_ours, mut inertia) = (0, 0, 0, 0);
    let mut errors = Vec::new();
    for (t, a) in &native {
        let empty = Vec::new();
        let b = ours.get(t).unwrap_or(&empty);
        let cells: Vec<String> = kinds.iter().map(|k| format!("{}/{}", a.iter().filter(|r| r.kind == *k).count(), b.iter().filter(|r| r.kind == *k).count())).collect();
        println!("{t:5} | {}", cells.join(" "));
        for k in ["ATTACK_REGION", "DEFEND_REGION"] {
            let mine: Vec<&Row> = b.iter().filter(|r| r.kind == k).collect();
            let theirs: Vec<&Row> = a.iter().filter(|r| r.kind == k).collect();
            for r in &theirs {
                match mine.iter().find(|m| m.region == r.region) {
                    Some(m) => {
                        both += 1;
                        errors.push((m.raw - r.raw).abs() / r.raw.abs().max(1.0));
                        inertia += usize::from(m.inertia != r.inertia);
                    }
                    None => only_native += 1,
                }
            }
            only_ours += mine.iter().filter(|m| !theirs.iter().any(|r| r.region == m.region)).count();
        }
    }
    errors.sort_by(f32::total_cmp);
    let at = |q: usize| errors.get(errors.len() * q / 100).copied().unwrap_or(0.0);
    println!("combat goals: {both} in both, {only_native} the original's alone, {only_ours} ours alone; inertia differs on {inertia}; class base error median {:.3} p90 {:.3}", at(50), at(90));
}

#[test]
#[ignore = "reads the QA films and the original data; set AA_GAME_ROOT"]
fn the_sai_beside_a_film() {
    let default = game_root().join("reconstruction/qa-runs/compare-20261022/game/Profile/data/Films/archive-qa-20261025/Berlin2v2_0.RFM");
    let path = std::env::var_os("AA_SHADOW_FILM").map_or(default, PathBuf::from);
    run(&path);
}

/// A film of ours (`AA_SHADOW_FILM`) against a FightSampler trace of the original playing it
/// (`AA_SHADOW_TRACE`): the first difference of each lane, and the getters per window. The
/// original stopping on our film is a world difference shown up by our orders.
/// `AA_SHADOW_WATCH=<native gtid hex>:<from>:<to>[,...]` prints those ticks side by side.
#[test]
#[ignore = "reads a film of ours, its native trace and the original data; set AA_GAME_ROOT, AA_SHADOW_FILM and AA_SHADOW_TRACE"]
fn our_film_against_the_original_s_trace() {
    use aa_parity::native_trace::NativeTrace;
    use aa_parity::state_compare::{self, Forcing, StateComparison};
    let root = game_root();
    let path = PathBuf::from(std::env::var_os("AA_SHADOW_FILM").expect("AA_SHADOW_FILM"));
    let registries = aa_parity::registries::Registries::load_v14(&root, true).unwrap();
    let film = Film::read(&path, &registries).unwrap();
    let trace = NativeTrace::read(PathBuf::from(std::env::var_os("AA_SHADOW_TRACE").expect("AA_SHADOW_TRACE")).as_path()).unwrap();
    let mut data = FilmData::new(&root);
    data.force_playback = true;
    let (mut world, _) = open_film(&path, &data, SyncMode::Strict).unwrap();
    let mut comparison = StateComparison::new();
    let mut pairing = replay::Pairing::for_our_film(world.ids.next_slot(), &trace.names);
    let last = trace.ticks.keys().last().copied().unwrap_or(0);
    let forcing = Forcing { rng: std::env::var_os("AA_SHADOW_FORCE").is_some(), ..Forcing::default() };
    let watches: Vec<(u32, u32, u32)> = std::env::var("AA_SHADOW_WATCH").map_or(Vec::new(), |t| {
        t.split(',')
            .filter_map(|w| {
                let mut p = w.split(':');
                Some((u32::from_str_radix(p.next()?, 16).ok()?, p.next()?.parse().ok()?, p.next()?.parse().ok()?))
            })
            .collect()
    });
    let paths = std::env::var("AA_SHADOW_PATHS").ok().and_then(|v| {
        let (a, b) = v.split_once(':')?;
        Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?))
    });
    let mut last_paths: BTreeMap<aa_play::ActorId, Option<(f32, f32)>> = BTreeMap::new();
    let report = replay::run_observed(
        &film,
        &mut world,
        &registries,
        Some(last),
        false,
        &replay::Aliases::new(),
        Some(&mut pairing),
        &mut |step, world, pairing| {
            comparison.observe(step, world, &trace, pairing);
            // AA_SHADOW_PATHS=from:to: every actor whose next node changes, in slots.
            if let Some((from, to)) = paths
                && step >= from
                && step <= to
            {
                for a in world.actors.iter().filter(|a| a.alive) {
                    let next = a.motion.path.first().map(|p| (p.x, p.y));
                    if last_paths.get(&a.id).is_some_and(|l| *l != next) {
                        println!("P {step} {:#x} {} {:?} -> {:?} at {:.4},{:.4} v {}", world.ids.actor(a.id).map_or(0, |g| g.raw()), world.data.def(a.def).id, last_paths.get(&a.id), next, a.pos.x, a.pos.y, a.motion.speed);
                    }
                    last_paths.insert(a.id, next);
                }
            }
            for (gtid, from, to) in &watches {
                if step < *from || step > *to {
                    continue;
                }
                let f = f32::from_bits;
                let native = trace.ticks.get(&step).and_then(|t| t.actors.get(gtid));
                let n = native.map_or("native -".to_owned(), |a| format!("native p {:.5},{:.5} d {:.5},{:.5} st {:x} v {} dest {}", f(a.pos[0]), f(a.pos[1]), f(a.dir[0]), f(a.dir[1]), a.state, a.mover.map_or(-1.0, |m| f(m.speed)), a.mover.map_or("-".to_owned(), |m| format!("{:.5},{:.5}", f(m.dest[0]), f(m.dest[1])))));
                let thing = pairing.and_then(|p| p.ours(*gtid)).and_then(|g| world.ids.thing_at_slot(g as u16));
                let o = match thing {
                    Some(aa_play::ids::Thing::Actor(a)) => {
                        let x = &world.actors[a as usize];
                        format!("ours p {:.5},{:.5} d {:.5},{:.5} v {} path {:?}", x.pos.x, x.pos.y, x.facing.cos(), x.facing.sin(), x.motion.speed, x.motion.path.iter().take(3).map(|p| (p.x, p.y)).collect::<Vec<_>>())
                    }
                    other => format!("ours {other:?}"),
                };
                println!("W {step} {gtid:x} {n} | {o}");
            }
            if let Some(native) = trace.ticks.get(&step) {
                state_compare::force(world, native, forcing, pairing);
            }
            comparison.remember(step, world, &trace, pairing);
        },
    );
    println!("{} of {} checkpoints; first difference {:?}; {} sampled ticks", report.matched, report.checked, report.first_difference.as_ref().map(|d| d.check), comparison.ticks_observed);
    let g = state_compare::getter_census(&world, &trace, report.steps);
    println!("getters: native {} ours {}; first state difference {:?}; first tick difference {:?}", g.native_total, g.ours_total, g.first_stream_difference, g.first_tick_difference);
    for w in g.first_windows.iter().take(8) {
        println!("  window ending {}: native {:?} ours {:?}", w.tick, w.native, w.ours_sites);
    }
    for (lane, l) in &comparison.lanes {
        println!("--- {lane}: first at {:?}; {} sampled ticks differ; {} causes", l.first.as_ref().map(|d| d.tick), l.ticks_differing, l.causes.len());
        let mut causes: Vec<_> = l.causes.values().collect();
        causes.sort_by_key(|c| (c.first_tick, c.signature.clone()));
        for c in causes.iter().take(25) {
            println!(
                "  {:>5}..{:<5} ticks {:>5} fields {:>7} things {:>4} first-on {:>4}{} {} | e.g. {:#x} {:?} {}: native {} / ours {}",
                c.first_tick,
                c.last_tick,
                c.ticks,
                c.fields,
                c.things.len(),
                c.first_on.len(),
                if c.knock_on() { " KNOCK-ON" } else { "" },
                c.signature,
                c.sample.gtid.unwrap_or(0),
                c.sample.name,
                c.sample.field,
                c.sample.native,
                c.sample.ours
            );
        }
    }
}
