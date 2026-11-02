//! Do the fight films fight? Replays each film (`tests/fight_films.rs` table)
//! through the mapper into aa-play and reads the outcome against the native
//! trace's aggregates (`fixtures/qa_films_fight_20261101.json`): first unit at
//! zero health, actors at zero health at the end, rout and state samples.
//!
//! ```text
//! AA_GAME_ROOT=<game root> cargo test -p aa-parity --release --test order_fights -- --ignored --nocapture
//! ```
//! `AA_FIGHT_FILMS_ONLY=<label>` limits it to one film. It reports; the numbers
//! will not match while the movement and combat lanes are open, so it fails only
//! on a mapping regression: an order the mapper calls no-equivalent.
#![cfg(feature = "play")]
use aa_parity::registries::Registries;
use aa_parity::{Film, replay};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const ARCHIVE: &str = "reconstruction/qa-runs/compare-20261022/game/Profile/data/Films/archive-qa-20261101";
const FILMS: &[(&str, &str)] = &[
    ("Comrades2v2_0", "DeveloperMaps/(2v2)Comrades"),
    ("Farmhouses4FFA_0", "DeveloperMaps/(4FFA)Farmhouses"),
    ("Kursk2v2_0", "(2v2)Kursk"),
    ("Berlin2v2_1", "(2v2)Berlin"),
];

fn install(registries: &Registries) {
    replay::set_formation_ids(registries.formation_ids.clone());
    aa_parity::mapping::set_ability_ids(registries.ability_ids.clone());
    replay::set_property_ids(registries.property_ids.clone());
}

fn fixture(label: &str) -> Option<Value> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/qa_films_fight_20261101.json");
    let all: Value = serde_json::from_str(std::fs::read_to_string(path).ok()?.trim_start_matches('\u{feff}')).ok()?;
    all["films"].as_array()?.iter().find(|r| r["label"] == label).cloned()
}

fn trace_of(label: &str) -> String {
    format!("fight-{}-native.txt", label.to_ascii_lowercase())
}

/// The `N` lines of a native trace (GTID -> definition).
fn native_names(root: &Path, trace: String) -> BTreeMap<u32, String> {
    use std::io::BufRead;
    let path = root.join("reconstruction/qa-runs/compare-20261022/traces").join(trace);
    let reader = std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(path).unwrap());
    let mut names = BTreeMap::new();
    for line in reader.lines() {
        let line = line.unwrap();
        if let Some(rest) = line.strip_prefix("N ") {
            let mut it = rest.split(' ');
            if let (Some(g), Some(n)) = (it.next(), it.next())
                && let Ok(g) = u32::from_str_radix(g.trim_start_matches("0x"), 16)
            {
                names.insert(g, n.to_owned());
            }
        }
    }
    names
}

#[test]
#[ignore = "reads the QA films and the original data; set AA_GAME_ROOT"]
fn the_fight_films_fight() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let registries = Registries::load_v14(&root, true).unwrap();
    install(&registries);
    let data = Arc::new(aa_play::GameData::load(&root).unwrap());
    let only = std::env::var("AA_FIGHT_FILMS_ONLY").ok();
    let mut no_equivalent = 0usize;
    for (label, map) in FILMS {
        if only.as_deref().is_some_and(|o| o != *label) {
            continue;
        }
        let film = Film::read(&root.join(ARCHIVE).join(format!("{label}.RFM")), &registries).unwrap();
        let map = Arc::new(aa_play::GameMap::load_with(&root, map, &data).unwrap());
        let plan = replay::plan(&film, &data, &map).unwrap();
        let mut world = replay::build(&plan, &root, data.clone(), map).unwrap();
        let mut first_zero: Option<u32> = None;
        let mut routs = 0usize;
        let mut deaths = 0usize;
        let mut fired = 0usize;
        let mut orders_active: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut died: BTreeMap<String, u64> = BTreeMap::new();
        let aliases = Default::default();
        // The native things in the film's slots, by the `N` lines of its trace:
        // until the ids agree (K1 and what follows) a film slot names another
        // thing here, and this test is about what the orders do.
        let names = native_names(&root, trace_of(label));
        let mut pairing = replay::Pairing::new(world.ids.next_slot(), &names);
        let unpaired = std::env::var_os("AA_ORDER_FIGHTS_UNPAIRED").is_some();
        let report = replay::run_observed(
            &film,
            &mut world,
            &registries.object_ids,
            None,
            false,
            &aliases,
            (!unpaired).then_some(&mut pairing),
            &mut |step, w, _| {
                for e in &w.events {
                    match e {
                        aa_play::Event::Died { actor, .. } => {
                            deaths += 1;
                            first_zero.get_or_insert(step);
                            if let Some(x) = w.actors.get(*actor as usize) {
                                *died.entry(w.data.def(x.def).id.clone()).or_default() += 1;
                            }
                        }
                        aa_play::Event::Routed { .. } => routs += 1,
                        aa_play::Event::Fired { .. } | aa_play::Event::FiredAt { .. } => fired += 1,
                        _ => {}
                    }
                }
                if step % 8 == 0 {
                    for o in w.orgs.iter().filter(|o| o.alive) {
                        let name = if aa_play::orders::guarding(w, o.id) {
                            "guard"
                        } else if aa_play::orders::retreating(w, o.id) {
                            "retreat"
                        } else if aa_play::orders::exploring(w, o.id) {
                            "explore"
                        } else {
                            match o.order {
                                aa_play::Order::Attack { .. } => "kill",
                                aa_play::Order::Move { .. } => "move",
                                aa_play::Order::Deploy { .. } => "deploy",
                                aa_play::Order::Idle => "sleep",
                            }
                        };
                        *orders_active.entry(name).or_default() += 1;
                    }
                }
            },
        );
        let native = fixture(label);
        let trace = native.as_ref().map(|n| &n["native_trace"]);
        let zero_native: u64 = trace.and_then(|t| t["zero_health"].as_object()).map_or(0, |m| m.values().filter_map(Value::as_u64).sum());
        eprintln!(
            "{label}: {} steps, {} of {} checkpoints; first difference {:?}",
            report.steps,
            report.matched,
            report.checked,
            report.first_difference.as_ref().map(|d| d.check.tick)
        );
        eprintln!(
            "{label}: first death at {:?} (native first zero health {:?}), deaths {deaths} (native zero health {zero_native}), routs {routs} (native cai_rout samples {:?}), shots {fired}",
            first_zero,
            trace.map(|t| &t["first_zero_health_tick"]),
            trace.map(|t| &t["mind_states"]["cai_rout"]),
        );
        eprintln!("{label}: org samples per state {orders_active:?}");
        // By definition: ours / native, the largest differences first.
        let mut by_def: Vec<(String, u64, u64)> = Vec::new();
        let zero = trace.and_then(|t| t["zero_health"].as_object());
        for (k, v) in zero.into_iter().flatten() {
            by_def.push((k.clone(), died.get(k).copied().unwrap_or(0), v.as_u64().unwrap_or(0)));
        }
        for (k, v) in &died {
            if zero.is_none_or(|z| !z.contains_key(k)) {
                by_def.push((k.clone(), *v, 0));
            }
        }
        by_def.sort_by_key(|(_, o, n)| std::cmp::Reverse(o.abs_diff(*n)));
        let rows: Vec<String> = by_def.iter().take(16).map(|(k, o, n)| format!("{k} {o}/{n}")).collect();
        eprintln!("{label}: deaths by definition, ours/native: {}", rows.join(", "));
        eprintln!("{label}: outcomes {:#?}", report.outcomes);
        let mut refused: BTreeMap<String, usize> = BTreeMap::new();
        for (_, _, why) in &report.refused {
            *refused.entry(why.clone()).or_default() += 1;
        }
        let mut refused: Vec<_> = refused.into_iter().collect();
        refused.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        eprintln!("{label}: {} refused: {:#?}", report.refused.len(), &refused[..refused.len().min(25)]);
        let mut unresolved: BTreeMap<String, usize> = BTreeMap::new();
        for (_, _, why) in &report.unresolved {
            *unresolved.entry(why.clone()).or_default() += 1;
        }
        eprintln!("{label}: {} unresolved: {unresolved:#?}", report.unresolved.len());
        eprintln!("{label}: dropped {:#?}", report.dropped);
        no_equivalent += report.outcomes.iter().filter(|(k, _)| k.starts_with("no equivalent")).map(|(_, n)| *n).sum::<usize>();
    }
    assert_eq!(no_equivalent, 0, "orders the mapper has no equivalent for");
}

/// The film's orders in tick order with what became of them (film `AA_ORDER_ROWS_FILM`,
/// rows up to tick `AA_ORDER_ROWS_TO`), for reading one film by hand.
#[test]
#[ignore = "reads the QA films and the original data; set AA_GAME_ROOT and AA_ORDER_ROWS_FILM"]
fn order_rows() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let label = std::env::var("AA_ORDER_ROWS_FILM").expect("AA_ORDER_ROWS_FILM");
    let to: u32 = std::env::var("AA_ORDER_ROWS_TO").ok().and_then(|v| v.parse().ok()).unwrap_or(3000);
    let registries = Registries::load_v14(&root, true).unwrap();
    install(&registries);
    let data = Arc::new(aa_play::GameData::load(&root).unwrap());
    let (label, map) = FILMS.iter().find(|(l, _)| *l == label).unwrap();
    let film = Film::read(&root.join(ARCHIVE).join(format!("{label}.RFM")), &registries).unwrap();
    let map = Arc::new(aa_play::GameMap::load_with(&root, map, &data).unwrap());
    let plan = replay::plan(&film, &data, &map).unwrap();
    let mut world = replay::build(&plan, &root, data.clone(), map).unwrap();
    for p in &plan.players {
        eprintln!("player {} slot {} {} kingdom index {} team {} ({}) computer {}", p.player, p.slot_id, p.nation, p.kingdom_index, p.team, p.team_id, p.computer);
    }
    for k in &world.kingdoms {
        eprintln!("kingdom {} {} team {} independent {} relations {:?}", k.id, k.name, k.team, k.independent, world.books.relations.get(k.id as usize));
    }
    let aliases = Default::default();
    let names = native_names(&root, trace_of(label));
    let mut pairing = replay::Pairing::new(world.ids.next_slot(), &names);
    let report = replay::run_observed(&film, &mut world, &registries.object_ids, Some(to), false, &aliases, Some(&mut pairing), &mut |step, w, _| {
        for e in &w.events {
            if std::env::var_os("AA_ORDER_EVENTS").is_some() {
                eprintln!("{step}: {e:?}");
            }
        }
        // One organization, tick by tick: AA_ORDER_ORG=<org>:<from>:<to>.
        if let Ok(v) = std::env::var("AA_ORDER_ORG") {
            let f: Vec<u32> = v.split(':').filter_map(|x| x.parse().ok()).collect();
            if f.len() == 3 && step >= f[1] && step <= f[2] {
                let o = &w.orgs[f[0] as usize];
                let c = w.combat.org(o.id);
                eprintln!(
                    "{step}: org {} {} anchor {:?} order {:?} state {} auto {:?} seen {:?} explicit {:?} task {:?}",
                    o.id,
                    w.data.def(o.def).id,
                    o.anchor,
                    o.order,
                    our_state(w, o.id),
                    c.and_then(|c| c.auto_order.clone()),
                    c.and_then(|c| c.seen_order.clone()),
                    c.and_then(|c| c.explicit),
                    c.and_then(|c| c.task.clone()),
                );
            }
        }
        // Who the engaging organizations have on their lists.
        if std::env::var_os("AA_ORDER_ENGAGED").is_some() && step % 128 == 0 {
            for o in w.orgs.iter().filter(|o| o.alive) {
                let Some(c) = w.combat.org(o.id) else { continue };
                if c.engage.is_empty() {
                    continue;
                }
                let t = c.current_target();
                let rep = t.and_then(|t| aa_play::combat::representative_public(w, t));
                let at = rep.and_then(|a| w.actor(a));
                let firing = o.members.iter().filter(|m| w.actors[**m as usize].combat.target.is_some()).count();
                eprintln!(
                    "{step}: org {} {} k{} at {:?} order {:?} tail {:?} = {:?} k{:?} dist {:?} morale {} firing {firing}/{} state {} auto {:?}",
                    o.id,
                    w.data.def(o.def).id,
                    o.kingdom,
                    c.position,
                    o.order,
                    t,
                    at.map(|a| w.data.def(a.def).id.clone()),
                    at.and_then(|a| a.kingdom),
                    at.map(|a| a.pos.truncate().distance(c.position)),
                    o.morale,
                    o.members.len(),
                    our_state(w, o.id),
                    c.auto_order,
                );
            }
        }
        if let Ok(v) = std::env::var("AA_ORDER_WATCH")
            && step % 32 == 0
        {
            for a in v.split(',').filter_map(|x| x.parse::<usize>().ok()) {
                if let Some(x) = w.actors.get(a) {
                    eprintln!("{step}: actor {a} {} pos {:?} order {:?} activity {:?} alive {} removed {} org {:?} {:?}", w.data.def(x.def).id, x.pos.truncate(), x.order, x.activity, x.alive, x.removed, x.org, x.org.map(|o| w.orgs[o as usize].order.clone()));
                }
            }
        }
    });
    for r in &report.orders {
        let who: Vec<String> = r.slots.iter().map(|s| names.iter().rev().find(|(g, _)| **g as u16 == *s).map_or_else(|| format!("{s:x}"), |(_, n)| n.clone())).collect();
        let c = r.commands.first().map_or(String::new(), |c| c.chars().take(100).collect());
        eprintln!("{:6} p{:?} {:18} {:?} -> {} {} {:?}", r.record, r.player, r.name, who, r.outcome, c, r.refused);
    }
    eprintln!("orgs alive {} actors alive {}", world.orgs.iter().filter(|o| o.alive).count(), world.actors.iter().filter(|a| a.alive && !a.removed).count());
}


/// Why the film's orders find nothing here: for every order whose selection or
/// target is gone, each slot by its native definition and whether the paired thing
/// was never made here, is made later than the order, or is already dead here.
#[test]
#[ignore = "reads the QA films and traces; set AA_GAME_ROOT"]
fn order_gone() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let registries = Registries::load_v14(&root, true).unwrap();
    install(&registries);
    let data = Arc::new(aa_play::GameData::load(&root).unwrap());
    let only = std::env::var("AA_FIGHT_FILMS_ONLY").ok();
    for (label, map) in FILMS {
        if only.as_deref().is_some_and(|o| o != *label) {
            continue;
        }
        let film = Film::read(&root.join(ARCHIVE).join(format!("{label}.RFM")), &registries).unwrap();
        let map = Arc::new(aa_play::GameMap::load_with(&root, map, &data).unwrap());
        let plan = replay::plan(&film, &data, &map).unwrap();
        let mut world = replay::build(&plan, &root, data.clone(), map).unwrap();
        let aliases = Default::default();
        let names = native_names(&root, trace_of(label));
        let start = world.ids.next_slot();
        let mut pairing = replay::Pairing::new(start, &names);
        let mut cursor = u32::from(start);
        let mut made: BTreeMap<u16, (u32, u16)> = BTreeMap::new();
        let mut last_alive: BTreeMap<u16, u32> = BTreeMap::new();
        let report = replay::run_observed(&film, &mut world, &registries.object_ids, None, false, &aliases, Some(&mut pairing), &mut |step, w, p| {
            let Some(p) = p else { return };
            while cursor < u32::from(w.ids.next_slot()) {
                let ours = cursor as u16;
                cursor += 1;
                if let Some(n) = p.native(u32::from(ours)) {
                    made.insert(n as u16, (step, ours));
                }
            }
            if step % 8 == 0 {
                for (n, (_, ours)) in &made {
                    if w.ids.thing_at_slot(*ours).is_some_and(|t| aa_play::ids::unit(t).is_some()) {
                        last_alive.insert(*n, step);
                    }
                }
            }
        });
        let mut rows: BTreeMap<(String, String), usize> = BTreeMap::new();
        for r in &report.orders {
            for &s in &r.slots {
                if s < start {
                    continue;
                }
                let gone = |at: u32| match (made.get(&s), last_alive.get(&s)) {
                    (None, _) => "never made here",
                    (Some((m, _)), _) if *m > at => "made later here",
                    (Some(_), Some(l)) if l + 8 < at => "dead here",
                    (Some(_), None) => "dead here",
                    _ => "alive",
                };
                let g = gone(r.record);
                if g == "alive" {
                    continue;
                }
                let name = names.iter().rev().find(|(g, _)| **g as u16 == s).map_or("(no name)".to_owned(), |(_, n)| n.clone());
                *rows.entry((g.to_owned(), name)).or_default() += 1;
            }
        }
        let mut rows: Vec<_> = rows.into_iter().collect();
        rows.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        eprintln!("{label}: slots of orders with no thing here");
        for ((g, name), n) in rows.iter().take(40) {
            eprintln!("  {n:6} {g:16} {name}");
        }
        let never: usize = names.keys().filter(|g| **g as u16 >= start && !made.contains_key(&(**g as u16))).count();
        eprintln!("{label}: native things {} since the battle started, {never} of them never paired, ours made {}", names.keys().filter(|g| **g as u16 >= start).count(), made.len());
        let mut names_never: BTreeMap<String, usize> = BTreeMap::new();
        for (g, n) in &names {
            if *g as u16 >= start && !made.contains_key(&(*g as u16)) {
                *names_never.entry(n.clone()).or_default() += 1;
            }
        }
        let mut names_never: Vec<_> = names_never.into_iter().collect();
        names_never.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        eprintln!("{label}: never paired by definition: {:?}", &names_never[..names_never.len().min(30)]);
    }
}

/// What each order did: the state of the ordered organization at the first
/// sampled tick at least 16 ticks after the order, native against ours, by the
/// order's name. This reads the handlers without the drift of a whole film.
#[test]
#[ignore = "reads the QA films and traces; set AA_GAME_ROOT"]
fn order_effects() {
    use std::io::BufRead;
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let registries = Registries::load_v14(&root, true).unwrap();
    install(&registries);
    let data = Arc::new(aa_play::GameData::load(&root).unwrap());
    let only = std::env::var("AA_FIGHT_FILMS_ONLY").ok();
    let show = std::env::var("AA_EFFECTS_SHOW").ok();
    for (label, map) in FILMS {
        if only.as_deref().is_some_and(|o| o != *label) {
            continue;
        }
        let path = root.join("reconstruction/qa-runs/compare-20261022/traces").join(trace_of(label));
        let reader = std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(path).unwrap());
        let mut lines: BTreeMap<u32, Vec<(u32, &'static str)>> = BTreeMap::new();
        for line in reader.lines() {
            let line = line.unwrap();
            if !line.starts_with("A ") || !line.contains(" mor=") {
                continue;
            }
            let mut it = line.split(' ').skip(1);
            let (Some(tick), Some(gtid)) = (it.next().and_then(|t| t.parse::<u32>().ok()), it.next().and_then(|g| u32::from_str_radix(g, 16).ok())) else { continue };
            let Some(ms) = line.split(' ').find_map(|t| t.strip_prefix("ms=")) else { continue };
            let Some(state) = ms.split(',').nth(1).and_then(|v| u32::from_str_radix(v, 16).ok()) else { continue };
            lines.entry(tick).or_default().push((gtid, native_state(state)));
        }
        let film = Film::read(&root.join(ARCHIVE).join(format!("{label}.RFM")), &registries).unwrap();
        let map = Arc::new(aa_play::GameMap::load_with(&root, map, &data).unwrap());
        let plan = replay::plan(&film, &data, &map).unwrap();
        let mut world = replay::build(&plan, &root, data.clone(), map).unwrap();
        let aliases = Default::default();
        let names = native_names(&root, trace_of(label));
        let mut pairing = replay::Pairing::new(world.ids.next_slot(), &names);
        // (tick, native slot) -> (native state, ours, what ours is)
        let mut samples: BTreeMap<(u32, u16), (&'static str, &'static str, String)> = BTreeMap::new();
        let report = replay::run_observed(&film, &mut world, &registries.object_ids, None, false, &aliases, Some(&mut pairing), &mut |step, w, p| {
            let Some(rows) = lines.get(&step) else { return };
            for (gtid, native) in rows {
                let ours = p.and_then(|p| p.ours(*gtid)).and_then(|g| w.ids.thing_at_slot(g as u16));
                let (name, what) = match ours {
                    Some(aa_play::ids::Thing::Org(o)) => {
                        let org = &w.orgs[o as usize];
                        (our_state(w, o), format!("org {o} {} k{} at {:?} order {:?} morale {}", w.data.def(org.def).id, org.kingdom, org.anchor, org.order, org.morale))
                    }
                    _ => ("(absent)", String::new()),
                };
                samples.insert((step, *gtid as u16), (native, name, what));
            }
        });
        let mut table: BTreeMap<(String, &'static str, &'static str), usize> = BTreeMap::new();
        for r in &report.orders {
            for &s in &r.slots {
                let at = (r.record + 16..r.record + 40).find(|t| samples.contains_key(&(*t, s)));
                let Some(at) = at else { continue };
                let (n, o, what) = &samples[&(at, s)];
                // A kill or move at a thing: split by what the target is.
                let on = r.commands.iter().find_map(|c| {
                    let t = c.split("target: ").nth(1)?;
                    let n: usize = t.split(|ch: char| !ch.is_ascii_digit()).next()?.parse().ok()?;
                    let d = world.data.def(world.actors.get(n)?.def);
                    Some(if d.city {
                        " on city"
                    } else if d.has_component("ElementComponent") {
                        " on element"
                    } else {
                        " on structure"
                    })
                });
                let key = format!("{}{}", r.name, on.unwrap_or(""));
                *table.entry((key, n, o)).or_default() += 1;
                if show.as_deref().is_some_and(|v| v.split(';').any(|x| x == format!("{},{n},{o}", r.name))) {
                    let before = (r.record.saturating_sub(16)..r.record).find_map(|t| samples.get(&(t, s))).map(|x| x.0);
                    eprintln!("{} {} {s:x} {:?}: at {at} native {before:?}->{n}, ours {o}: {what} {:?} {:?}", r.record, r.name, r.commands.first().map(|c| c.chars().take(90).collect::<String>()), r.refused, r.outcome);
                }
            }
        }
        let mut names: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for ((name, n, o), c) in &table {
            let e = names.entry(name.clone()).or_default();
            e.0 += c;
            if n == o {
                e.1 += c;
            }
        }
        eprintln!("{label}: dropped {:?}", report.dropped);
        eprintln!("{label}: the state 16 ticks after an order, by order");
        for (name, (total, same)) in &names {
            let mut rows: Vec<_> = table.iter().filter(|((k, n, o), _)| k == name && n != o).map(|((_, n, o), c)| (*c, *n, *o)).collect();
            rows.sort_by_key(|r| std::cmp::Reverse(r.0));
            let rows: Vec<String> = rows.iter().take(8).map(|(c, n, o)| format!("{n}/{o} {c}")).collect();
            eprintln!("  {name:28} {same:6} of {total:6} agree; native/ours: {}", rows.join(", "));
        }
    }
}

/// The organization's Mind state against the original: every organization line
/// of the native trace (`ms=` regiment Mind, its top state) against what aa-play
/// has for the paired organization at that tick. Prints the table of (native,
/// ours) sample counts and how much agrees.
#[test]
#[ignore = "reads the QA films and traces; set AA_GAME_ROOT"]
fn mind_state_confusion() {
    use std::io::BufRead;
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let registries = Registries::load_v14(&root, true).unwrap();
    install(&registries);
    let data = Arc::new(aa_play::GameData::load(&root).unwrap());
    let only = std::env::var("AA_FIGHT_FILMS_ONLY").ok();
    for (label, map) in FILMS {
        if only.as_deref().is_some_and(|o| o != *label) {
            continue;
        }
        // tick -> (gtid, native state)
        let path = root.join("reconstruction/qa-runs/compare-20261022/traces").join(trace_of(label));
        let reader = std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(path).unwrap());
        let mut lines: BTreeMap<u32, Vec<(u32, &'static str)>> = BTreeMap::new();
        for line in reader.lines() {
            let line = line.unwrap();
            if !line.starts_with("A ") || !line.contains(" mor=") {
                continue;
            }
            let mut it = line.split(' ').skip(1);
            let (Some(tick), Some(gtid)) = (it.next().and_then(|t| t.parse::<u32>().ok()), it.next().and_then(|g| u32::from_str_radix(g, 16).ok())) else { continue };
            let Some(ms) = line.split(' ').find_map(|t| t.strip_prefix("ms=")) else { continue };
            let Some(state) = ms.split(',').nth(1).and_then(|v| u32::from_str_radix(v, 16).ok()) else { continue };
            lines.entry(tick).or_default().push((gtid, native_state(state)));
        }
        let film = Film::read(&root.join(ARCHIVE).join(format!("{label}.RFM")), &registries).unwrap();
        let map = Arc::new(aa_play::GameMap::load_with(&root, map, &data).unwrap());
        let plan = replay::plan(&film, &data, &map).unwrap();
        let mut world = replay::build(&plan, &root, data.clone(), map).unwrap();
        let aliases = Default::default();
        let names = native_names(&root, trace_of(label));
        let mut pairing = replay::Pairing::new(world.ids.next_slot(), &names);
        let mut table: BTreeMap<(&'static str, &'static str), usize> = BTreeMap::new();
        let mut absent: BTreeMap<&'static str, usize> = BTreeMap::new();
        replay::run_observed(&film, &mut world, &registries.object_ids, None, false, &aliases, Some(&mut pairing), &mut |step, w, p| {
            let Some(rows) = lines.get(&step) else { return };
            for (gtid, native) in rows {
                let ours = p.and_then(|p| p.ours(*gtid)).and_then(|g| w.ids.thing_at_slot(g as u16));
                let name = match ours {
                    Some(aa_play::ids::Thing::Org(o)) => our_state(w, o),
                    _ => {
                        *absent.entry(native).or_default() += 1;
                        continue;
                    }
                };
                if let Ok(show) = std::env::var("AA_CONFUSION_SHOW")
                    && show == format!("{native},{name}")
                    && step % 256 == 0
                {
                    if let Some(aa_play::ids::Thing::Org(o)) = ours {
                        let org = &w.orgs[o as usize];
                        let target = match org.order {
                            aa_play::Order::Attack { target } => w.actors.get(target as usize).map(|t| (w.data.def(t.def).id.clone(), t.kingdom, t.health, t.alive, t.pos.truncate().distance(org.anchor))),
                            _ => None,
                        };
                        eprintln!("{step}: {native} / {name}: org {o} {} k{} at {:?} order {:?} target {target:?}", w.data.def(org.def).id, org.kingdom, org.anchor, org.order);
                    }
                }
                *table.entry((native, name)).or_default() += 1;
            }
        });
        let total: usize = table.values().sum();
        let same: usize = table.iter().filter(|((n, o), _)| n == o).map(|(_, c)| *c).sum();
        eprintln!("{label}: {same} of {total} organization samples agree ({:.1}%)", 100.0 * same as f64 / total.max(1) as f64);
        let mut rows: Vec<_> = table.into_iter().collect();
        rows.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
        for ((n, o), c) in rows.iter().take(40) {
            eprintln!("{label}: native {n:12} ours {o:12} {c}");
        }
        eprintln!("{label}: no paired organization: {absent:?}");
    }
}

fn native_state(vtable: u32) -> &'static str {
    match vtable {
        0xA329F0 => "sleep",
        0xA32A14 => "rampage",
        0xA33804 => "move",
        0xA32B5C => "kill",
        0xA3348C => "bombard",
        0xA32CC8 => "siege",
        0xA32B80 => "chasing",
        0xA32A38 => "construct",
        0xA32BA4 => "repair",
        0xA32BC8 => "guard",
        0xA32A5C => "regroup",
        0xA32A80 => "reform",
        0xA33140 => "engage",
        0xA32AA4 => "retreat",
        0xA32AC8 => "rout",
        0xA32BEC => "explore",
        0xA32EC4 => "join",
        0xA32AEC => "exhausted",
        0xA32B10 => "unpack",
        0xA32B34 => "pack",
        _ => "other",
    }
}

fn our_state(w: &aa_play::World, o: aa_play::OrgId) -> &'static str {
    const NAMES: [&str; 20] = [
        "sleep", "move", "kill", "chasing", "engage", "guard", "explore", "retreat", "rout", "reform", "exhausted", "unpack", "pack",
        "bombard", "repair", "construct", "siege", "join", "rampage", "regroup",
    ];
    let name = format!("{:?}", aa_play::morale::regiment_state(w, o)).to_ascii_lowercase();
    NAMES.iter().find(|n| **n == name).copied().unwrap_or("other")
}
