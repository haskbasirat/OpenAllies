//! H4: the 2026-10-25 QA film on the stock map (2v2)Berlin (one idle human,
//! three computer players, 4539 world ticks) replayed into aa-play through
//! `Mapper` + `PlayIds`, checkpoint by checkpoint. The film stays in the QA
//! copy; `fixtures/qa_film_berlin_20261025.json` holds its aggregates.
//!
//! `AA_GAME_ROOT='E:\Games\Axis and Alies' cargo test -p aa-parity --test berlin_replay -- --ignored --nocapture`
//! (`AA_PARITY_BERLIN_FILM` overrides the film path).
#![cfg(feature = "play")]
use aa_parity::registries::Registries;
use aa_parity::{Film, replay};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

const FILM: &str = r"E:\Games\Axis and Alies\reconstruction\qa-runs\compare-20261022\game\Profile\data\Films\archive-qa-20261025\Berlin2v2_0.RFM";
const MAP: &str = "(2v2)Berlin";

fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"))
}
fn film_path() -> PathBuf {
    std::env::var_os("AA_PARITY_BERLIN_FILM").map_or_else(|| PathBuf::from(FILM), PathBuf::from)
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/qa_film_berlin_20261025.json")).unwrap()
}
fn checkpoint_json(c: &aa_parity::Checkpoint) -> Value {
    serde_json::json!({"tick": c.tick, "count": c.count, "hash": c.hash})
}

#[test]
#[ignore = "reads the QA Berlin film and the original data; set AA_GAME_ROOT"]
fn berlin_film_decodes_to_its_recorded_aggregates() {
    let root = game_root();
    let film = Film::read(&film_path(), &Registries::load_v14(&root, true).unwrap()).unwrap();
    let row = fixture()["film"].clone();
    assert_eq!(film.source.sha256, row["sha256"].as_str().unwrap());
    assert_eq!(film.source.bytes, row["bytes"].as_u64().unwrap());
    assert_eq!(
        film.header.creator_class,
        row["creator_class"].as_str().unwrap()
    );
    assert_eq!(u64::from(film.header.seed), row["seed"].as_u64().unwrap());
    assert_eq!(film.players.len() as u64, row["players"].as_u64().unwrap());
    assert_eq!(
        film.world_ticks() as u64,
        row["world_ticks"].as_u64().unwrap()
    );
    assert_eq!(film.decree_count() as u64, row["decrees"].as_u64().unwrap());
    assert_eq!(
        film.failures().count() as u64,
        row["decode_failures"].as_u64().unwrap()
    );
    assert_eq!(
        film.checkpoints.len() as u64,
        row["checkpoints"].as_u64().unwrap()
    );
    assert_eq!(
        checkpoint_json(&film.checkpoints[0]),
        row["first_checkpoint"]
    );
    assert_eq!(
        checkpoint_json(film.checkpoints.last().unwrap()),
        row["last_checkpoint"]
    );
    assert_eq!(
        serde_json::to_value(film.histogram()).unwrap(),
        row["histogram"]
    );
    let first = aa_parity::slog::account(&film)[0];
    assert_eq!(
        (first.clock, first.decree_entries, first.residual),
        (17, 18, 3),
        "17 clocks, the 3 kingdom names and 3 x (SelectionSet + recruit), 3 getters"
    );
}

#[test]
#[ignore = "reads the QA Berlin film and the original data; set AA_GAME_ROOT"]
fn berlin_film_replays_into_aa_play() {
    let root = game_root();
    let registries = Registries::load_v14(&root, true).unwrap();
    let film = Film::read(&film_path(), &registries).unwrap();
    let data = Arc::new(aa_play::GameData::load(&root).unwrap());
    let map = Arc::new(aa_play::GameMap::load_with(&root, MAP, &data).unwrap());

    eprintln!("map slots:");
    for (i, s) in map.slots.iter().enumerate() {
        eprintln!(
            "  {i} {:>14} independent {:5} team {:?} colour {:?} nation {:?} faction {:?} index {:?} id {:#x?}",
            s.id,
            s.independent,
            s.team_id,
            s.color_id,
            s.nation,
            s.faction,
            s.kingdom_index,
            s.serialized_id
        );
    }
    eprintln!("film creator kingdoms:");
    for k in &film.header.creator_kingdoms {
        eprintln!(
            "  {:>14} independent {:5} team {:?} colour {:?} nation {:?} faction {:?} handicap {:?}",
            k.id, k.independent, k.team, k.color, k.nation, k.faction, k.handicap
        );
    }
    eprintln!("film header kingdoms:");
    for k in &film.header.header_kingdoms {
        eprintln!(
            "  {:>14} independent {:5} team {:?} colour {:?} nation {:?} faction {:?} handicap {:?}",
            k.id, k.independent, k.team, k.color, k.nation, k.faction, k.handicap
        );
    }
    eprintln!("film rules:");
    for r in &film.header.map_header.creator.rules.rules {
        eprintln!(
            "  {} kind {} value {:#x}",
            String::from_utf16_lossy(&r.key),
            r.kind,
            r.value
        );
    }

    if let Some(m) = &map.id_manager {
        eprintln!(
            "map IDMGR: next slot {} highest slot {} ({} generations)",
            m.next_slot,
            m.highest_slot,
            m.generations.len()
        );
    }
    eprintln!("map things owned by player slots:");
    for (i, p) in map.placed.iter().enumerate() {
        if let Some(s) = p.owner_slot
            && !map.slots[s as usize].independent
        {
            eprintln!(
                "  placed {i} {} owner {} id {:#x} at {:?}",
                p.thing_id, map.slots[s as usize].id, p.serialized_id, p.position
            );
        }
    }

    let plan = replay::plan(&film, &data, &map).unwrap();
    eprintln!("plan: {}", serde_json::to_string_pretty(&plan).unwrap());
    assert_eq!(plan.players.len(), 4);
    assert_eq!(plan.players.iter().filter(|p| p.computer).count(), 3);

    let mut world = replay::build(&plan, &root, data.clone(), map.clone()).unwrap();
    eprintln!(
        "aa-play at battle start: {} kingdoms, {} actors, {} organizations, {} live ids, next slot {}, rng state {:#010x}",
        world.kingdoms.len(),
        world.actors.len(),
        world.orgs.len(),
        world.ids.live(),
        world.ids.next_slot(),
        world.native_rng.state
    );
    for (k, kingdom) in world.kingdoms.iter().enumerate() {
        let actors: Vec<String> = world
            .actors
            .iter()
            .filter(|a| a.kingdom == Some(k as u32) && !a.removed)
            .map(|a| {
                format!(
                    "{}@{:#x}",
                    data.def(a.def).id,
                    world.ids.actor(a.id).map_or(0, |g| g.raw())
                )
            })
            .collect();
        eprintln!(
            "  kingdom {k} {:?} {} {:?}: {}",
            kingdom.name,
            kingdom.nation,
            kingdom.controller,
            actors.join(", ")
        );
    }
    let checks = replay::slot_checks(&film, &plan, &world, film.ticks.len() as u32);
    eprintln!("film selections against aa-play's ids at battle start:");
    for c in &checks {
        eprintln!(
            "  slot {:>5} by player {:?} (kingdom {:?}): aa-play {} (kingdom {:?})",
            c.slot, c.player, c.expected_kingdom, c.ours, c.our_kingdom
        );
    }
    eprintln!("id notes: {:?}", world.ids.notes);

    let report = replay::run(&film, &mut world, &registries.object_ids, None, false);
    eprintln!("report: {}", serde_json::to_string_pretty(&report).unwrap());
    assert_eq!(report.steps as usize, film.world_ticks());
    assert_eq!(report.checked, film.checkpoints.len());

    // Diagnostic: the same replay with the placeholder replacements answering
    // their placeholders' saved slots, as the original's ids do.
    let mut world = replay::build(&plan, &root, data.clone(), map.clone()).unwrap();
    let aliases = replay::placeholder_aliases(&world);
    eprintln!("placeholder aliases: {aliases:?}");
    let aliased = replay::run_aliased(
        &film,
        &mut world,
        &registries.object_ids,
        None,
        true,
        &aliases,
    );
    eprintln!(
        "aliased: checked {} matched {} first difference {}",
        aliased.checked,
        aliased.matched,
        serde_json::to_string_pretty(&aliased.first_difference).unwrap()
    );
    eprintln!("aliased outcomes: {:?}", aliased.outcomes);
    eprintln!("aliased refused: {:?}", aliased.refused);

    // How many world-RNG draws lie between the map's saved GWORLD state and
    // the first battle getter, natively (low 16 bits, unique mod 2^16) and in
    // aa-play (full state).
    let saved = map
        .rng_state
        .expect("a loaded map carries its GWORLD state");
    let lcg = |s: u32| s.wrapping_mul(0x41C6_4E6D).wrapping_add(0x3039);
    let mut s = saved;
    let mut ours = None;
    let first_getter = aliased
        .first_difference
        .as_ref()
        .and_then(|d| d.ours.getter_states.first())
        .map(|g| g.1);
    for n in 0..2_000_000u32 {
        if Some(s & 0xFFFF) == first_getter {
            ours = Some(n);
            break;
        }
        s = lcg(s);
    }
    let film_x = aliased
        .first_difference
        .as_ref()
        .and_then(|d| d.film_states_at_our_placement.clone())
        .unwrap_or_default();
    let mut native = Vec::new();
    for &x in &film_x {
        let mut s = saved & 0xFFFF;
        for n in 0..65536u32 {
            if s == x {
                native.push(n);
                break;
            }
            s = lcg(s) & 0xFFFF;
        }
    }
    eprintln!(
        "map saved GWORLD rng {saved:#010x}; draws to the first battle getter: aa-play {ours:?}, original {native:?} (mod 65536)"
    );
    // Diagnostic: aliases plus the film's low half at the first getter (the
    // first draw after battle start), to see the next difference.
    if let Some(&x) = film_x.first() {
        let mut world = replay::build(&plan, &root, data.clone(), map.clone()).unwrap();
        world.native_rng.state = (world.native_rng.state & 0xFFFF_0000) | x;
        let next = replay::run_aliased(
            &film,
            &mut world,
            &registries.object_ids,
            None,
            true,
            &aliases,
        );
        eprintln!(
            "aliased + low half {x:#06x}: checked {} matched {} first difference {}",
            next.checked,
            next.matched,
            serde_json::to_string_pretty(&next.first_difference).unwrap()
        );
        // Where could the film's getters of that interval sit, given the
        // state aa-play reached there (same draws so far)? Try every update
        // holding all of them, and every split into equal groups over
        // consecutive updates.
        if let Some(d) = &next.first_difference
            && let Some(&(_, x)) = d.ours.getter_states.first()
        {
            let iv = aa_parity::chain::prepare(&film)
                .into_iter()
                .find(|iv| iv.to_tick == d.check.tick)
                .unwrap();
            let update_slot = |k: u32| {
                iv.slots
                    .iter()
                    .position(|s| *s == aa_parity::chain::Slot::Update { update: k })
            };
            let r = iv.residual as usize;
            let mut found = Vec::new();
            for groups in [1usize, 2, 3, 4, 6, 8, 12, 16] {
                if r % groups != 0 {
                    continue;
                }
                for first in iv.from_tick + 1..=iv.to_tick {
                    let Some(slots): Option<Vec<usize>> =
                        (0..groups as u32).map(|g| update_slot(first + g)).collect()
                    else {
                        continue;
                    };
                    let pick: Vec<usize> = slots
                        .iter()
                        .flat_map(|&s| std::iter::repeat_n(s, r / groups))
                        .collect();
                    if aa_parity::chain::solve(&iv, &pick, 1).contains(&x) {
                        found.push((groups, first));
                    }
                }
            }
            eprintln!(
                "  film interval {}..{}: {} getters from low half {x:#06x}; equal groups on consecutive updates that explain it (groups, first update): {found:?}",
                iv.from_tick, iv.to_tick, r
            );
        }
        eprintln!("  outcomes: {:?}", next.outcomes);
        eprintln!("  unresolved: {:?}", next.unresolved);
        eprintln!("  refused: {:?}", next.refused);
    }
    let mut census: std::collections::BTreeMap<String, usize> = Default::default();
    for p in &map.placed {
        let key = match data.find(&p.thing_id) {
            None => "not an aa-play definition".to_string(),
            Some(d) => {
                let def = data.def(d);
                format!(
                    "{} {:?}{}",
                    if def.actor_data { "actor" } else { "object" },
                    def.class,
                    if p.owner_slot.is_some() { " owned" } else { "" }
                )
            }
        };
        *census.entry(key).or_default() += 1;
    }
    eprintln!("map things: {} {census:?}", map.placed.len());
}
