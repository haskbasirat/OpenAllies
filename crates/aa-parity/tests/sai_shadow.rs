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
    for t in 0..ticks {
        player.step(&mut world);
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
            *real.entry((p, name.clone(), w.flags)).or_default() += 1;
            let formation = w.parameter.and_then(|f| registries.formation_ids.get(f as usize).cloned()).unwrap_or_default();
            timeline.entry(t).or_default().push(format!("film p{p} {name} {:#x} word {:#x} pos {:?} {formation} {target}", w.flags, w.word, w.position.map(f32::from_bits)));
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
        for e in world.ai.log.iter().take(4000) {
            println!("sai {:8.2} k{} {}", e.time, e.kingdom, e.text);
        }
    }
}

#[test]
#[ignore = "reads the QA films and the original data; set AA_GAME_ROOT"]
fn the_sai_beside_a_film() {
    let default = game_root().join("reconstruction/qa-runs/compare-20261022/game/Profile/data/Films/archive-qa-20261025/Berlin2v2_0.RFM");
    let path = std::env::var_os("AA_SHADOW_FILM").map_or(default, PathBuf::from);
    run(&path);
}
