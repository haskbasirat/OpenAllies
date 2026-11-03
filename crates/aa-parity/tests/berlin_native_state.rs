//! The Berlin film replayed into aa-play and compared, tick by tick and field
//! by field, with the native sampler's snapshots of the original playing the
//! same film (QA tool `NativeSampler`; the trace stays in the QA copy).
//! Reports, for every lane (rng, existence, economy, production, movement,
//! combat, supply, fog), the first divergence and every divergence to the end
//! of the film collapsed into causes, and the world-RNG getters by native site.
//!
//! `AA_GAME_ROOT='<game root>' cargo test -p aa-parity --test berlin_native_state -- --ignored --nocapture`
//! (`AA_PARITY_BERLIN_FILM` / `AA_PARITY_BERLIN_NATIVE` override the film and
//! trace paths; `AA_PARITY_CENSUS_DIR` writes each run's full report as JSON).
//!
//! The census runs the film several times. `free` is the replay as it stands:
//! what it shows after the first difference is mostly knock-on. The `forced`
//! runs put aa-play back on the native values (`state_compare::force`) after
//! every sampled tick: what still differs there is what one sampled interval
//! gets wrong by itself.
//!
//! `AA_PARITY_WATCH=<native gtid hex>:<from>:<to>[,...]` prints those ticks of
//! the things side by side (position, direction, speed, Mover destination
//! against the first path node); `AA_PARITY_CENSUS_RUNS=free,forced-rng`
//! selects the runs.
#![cfg(feature = "play")]
use aa_parity::native_trace::NativeTrace;
use aa_parity::registries::Registries;
use aa_parity::state_compare::{self, Forcing, GetterCensus, StateComparison};
use aa_parity::{Film, replay};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const FILM: &str = "reconstruction/qa-runs/compare-20261022/game/Profile/data/Films/archive-qa-20261025/Berlin2v2_0.RFM";
const NATIVE: &str = "reconstruction/qa-runs/compare-20261022/traces/berlin2v2_0-native.txt";
const MAP: &str = "(2v2)Berlin";

/// `AA_PARITY_BERLIN_MAP` with `AA_PARITY_BERLIN_FILM` and `AA_PARITY_BERLIN_NATIVE` runs
/// this census on another film and its trace (`FIGHT-FILMS.md`).
fn map_id() -> String {
    std::env::var("AA_PARITY_BERLIN_MAP").unwrap_or_else(|_| MAP.to_owned())
}

fn env_path(var: &str, default: &Path) -> PathBuf {
    std::env::var_os(var).map_or_else(|| default.to_path_buf(), PathBuf::from)
}

fn install_id_tables(registries: &Registries) {
    replay::set_property_ids(registries.property_ids.clone());
    replay::set_formation_ids(registries.formation_ids.clone());
    aa_parity::mapping::set_ability_ids(registries.ability_ids.clone());
    aa_parity::mapping::set_layout_ids(registries.layout_ids.clone());
}

struct Run {
    comparison: StateComparison,
    report: replay::ReplayReport,
    getters: GetterCensus,
    /// (native, aa-play) GTIDs of the things whose slots differ.
    shifted: Vec<(u32, u32)>,
    /// (step, slot, definition) of aa-play's creations.
    created: Vec<(u32, u16, String)>,
}

struct Created {
    cursor: u32,
    list: Vec<(u32, u16, String)>,
}

fn note_creations(
    created: &mut Created,
    step: u32,
    world: &aa_play::World,
    _pairing: Option<&replay::Pairing>,
) {
    let next = u32::from(world.ids.next_slot());
    while created.cursor < next {
        let slot = created.cursor as u16;
        created.cursor += 1;
        if let Some(t) = world.ids.thing_at_slot(slot) {
            created
                .list
                .push((step, slot, replay::thing_definition(world, t)));
        }
    }
}

fn watch_list() -> Vec<(u32, u32, u32)> {
    let Ok(text) = std::env::var("AA_PARITY_WATCH") else {
        return Vec::new();
    };
    text.split(',')
        .filter_map(|w| {
            let mut p = w.split(':');
            Some((
                u32::from_str_radix(p.next()?, 16).ok()?,
                p.next()?.parse().ok()?,
                p.next()?.parse().ok()?,
            ))
        })
        .collect()
}

/// One watched thing's tick, native against aa-play.
fn watch_print(
    watches: &[(u32, u32, u32)],
    step: u32,
    world: &aa_play::World,
    trace: &NativeTrace,
    pairing: Option<&replay::Pairing>,
) {
    use aa_play::ids::Thing;
    for (gtid, from, to) in watches {
        if step < *from || step > *to {
            continue;
        }
        let f32b = |b: u32| f32::from_bits(b);
        let native = trace.ticks.get(&step).and_then(|t| t.actors.get(gtid));
        let n = native.map_or("native -".to_owned(), |a| {
            format!(
                "native p {:.5},{:.5} d {:.5},{:.5} v {} dest {} st {:x} act {} f {:x}",
                f32b(a.pos[0]),
                f32b(a.pos[1]),
                f32b(a.dir[0]),
                f32b(a.dir[1]),
                a.mover.map_or(-1.0, |m| f32b(m.speed)),
                a.mover.map_or("-".to_owned(), |m| format!(
                    "{:.5},{:.5}",
                    f32b(m.dest[0]),
                    f32b(m.dest[1])
                )),
                a.state,
                a.active,
                a.flags,
            ) + &format!(" ms {:x?} mor {:?}", a.mind.map(|m| m[1]), a.morale.map(|m| (f32b(m[0]), f32b(m[1]), m[3], f32b(m[6]), f32b(m[7]))))
        });
        let thing = pairing
            .and_then(|p| p.ours(*gtid))
            .and_then(|g| world.ids.thing_at_slot(g as u16));
        let o = match thing {
            Some(Thing::Actor(a)) => {
                let x = &world.actors[a as usize];
                format!(
                    "ours hp {} target {:?} kingdom {:?} p {:.5},{:.5} d {:.5},{:.5} v {} path {:?} goal {:?} activity {:?}",
                    x.health,
                    x.combat.target,
                    x.kingdom,
                    x.pos.x,
                    x.pos.y,
                    x.facing.cos(),
                    x.facing.sin(),
                    x.motion.speed,
                    x.motion
                        .path
                        .iter()
                        .take(3)
                        .map(|p| (p.x, p.y))
                        .collect::<Vec<_>>(),
                    x.motion.goal.map(|g| (g.x, g.y)),
                    x.activity,
                )
            }
            Some(Thing::Org(o)) => {
                let x = &world.orgs[o as usize];
                format!(
                    "ours org eff {:?} anchor {:.5},{:.5} heading {:.5} order {:?} state {:?} engage {:?} explicit {:?} engaged_by {:?} morale {}",
                    world.combat.org(o).map(|s| (s.attack_efficiency, s.attack_efficiency_desired, s.morale_max, s.living)),
                    x.anchor.x, x.anchor.y, x.heading, x.order,
                    aa_play::morale::regiment_state(world, o),
                    world.combat.org(o).map(|s| s.engage.iter().map(|t| t.target).collect::<Vec<_>>()),
                    world.combat.org(o).and_then(|s| s.explicit),
                    world.combat.org(o).map(|s| s.engaged_by.clone()),
                    x.morale,
                )
            }
            other => format!("ours {other:?}"),
        };
        eprintln!("W {step} {gtid:x} {n} | {o}");
    }
}

fn run(
    root: &Path,
    film: &Film,
    registries: &Registries,
    trace: &NativeTrace,
    forcing: Forcing,
) -> Run {
    let data = Arc::new(aa_play::GameData::load(root).unwrap());
    let map = Arc::new(aa_play::GameMap::load_with(root, &map_id(), &data).unwrap());
    let plan = replay::plan(film, &data, &map).unwrap();
    let mut world = replay::build(&plan, root, data, map).unwrap();
    assert!(
        replay::pick_mismatches(&plan, &world).is_empty(),
        "the random picks reproduce the film header"
    );
    let mut comparison = StateComparison::new();
    let watches = watch_list();
    let mut created = Created {
        cursor: u32::from(world.ids.next_slot()),
        list: Vec::new(),
    };
    let mut pairing = replay::Pairing::new(world.ids.next_slot(), &trace.names);
    let last = trace.ticks.keys().last().copied().unwrap_or(0);
    let report = replay::run_observed(
        film,
        &mut world,
        &registries.object_ids,
        Some(last),
        false,
        &replay::Aliases::new(),
        Some(&mut pairing),
        &mut |step, world, pairing| {
            comparison.observe(step, world, trace, pairing);
            watch_print(&watches, step, world, trace, pairing);
            note_creations(&mut created, step, world, pairing);
            if let Some(native) = trace.ticks.get(&step) {
                state_compare::force(world, native, forcing, pairing);
            }
            comparison.remember(step, world, trace, pairing);
        },
    );
    let getters = state_compare::getter_census(&world, trace, report.steps);
    Run {
        comparison,
        report,
        getters,
        shifted: pairing.shifted(),
        created: created.list,
    }
}

fn print_run(label: &str, r: &Run) {
    eprintln!(
        "\n===== {label}: {} of {} checkpoints; first difference at {:?}; {} sampled ticks compared",
        r.report.matched,
        r.report.checked,
        r.report.first_difference.as_ref().map(|d| d.check),
        r.comparison.ticks_observed
    );
    for (lane, l) in &r.comparison.lanes {
        eprintln!(
            "--- {lane}: first at {:?}; {} sampled ticks differ; {} causes",
            l.first.as_ref().map(|d| d.tick),
            l.ticks_differing,
            l.causes.len()
        );
        let mut causes: Vec<_> = l.causes.values().collect();
        causes.sort_by_key(|c| (c.first_tick, c.signature.clone()));
        for c in causes.iter().take(40) {
            eprintln!(
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
        if causes.len() > 40 {
            eprintln!("  ... {} more", causes.len() - 40);
        }
    }
    let g = &r.getters;
    eprintln!(
        "--- getters: native {} ours {}; first state difference {:?}; first tick difference {:?}; windows {} equal {} short {} long {}",
        g.native_total,
        g.ours_total,
        g.first_stream_difference,
        g.first_tick_difference,
        g.windows,
        g.windows_equal,
        g.windows_short,
        g.windows_long
    );
    let mut groups: Vec<_> = g.by_group.iter().collect();
    groups.sort_by_key(|(_, e)| (e.first_difference_tick.unwrap_or(u32::MAX), e.first_tick));
    for (name, e) in groups {
        eprintln!(
            "  {:>5} native, first {:>5}, in equal windows {:>5}, first difference of {:>4} windows from {:?}: {name}",
            e.native,
            e.first_tick,
            e.in_equal_ticks,
            e.first_difference_in,
            e.first_difference_tick
        );
    }
    eprintln!("--- aa-play getters by site (count, first tick):");
    for (site, (n, first)) in &g.ours_by_site {
        eprintln!("  {n:>5} from {first:>5}: {site}");
    }
    for w in g.first_windows.iter().take(12) {
        eprintln!("  window ending {}: native {:?} ours {:?}", w.tick, w.native, w.ours_sites);
    }
    eprintln!("--- orders:");
    for o in &r.report.orders {
        eprintln!(
            "  record {:>5} player {:?} {:<22} slots {:?} -> {}{}",
            o.record,
            o.player,
            o.name,
            o.slots,
            o.outcome,
            if o.refused.is_empty() {
                String::new()
            } else {
                format!(" REFUSED {:?}", o.refused)
            }
        );
    }
    eprintln!("--- order fields dropped: {:?}", r.report.dropped);
    eprintln!(
        "--- aa-play creations: {} (first {:?})",
        r.created.len(),
        r.created.iter().take(4).collect::<Vec<_>>()
    );
    eprintln!(
        "--- things whose slot differs from the native's: {} (first {:x?})",
        r.shifted.len(),
        r.shifted.iter().take(6).collect::<Vec<_>>()
    );
    let short: Vec<_> = r
        .report
        .intervals
        .iter()
        .filter(|i| i.film_entries != i.our_entries)
        .take(24)
        .map(|i| (i.check.tick, i.film_entries, i.our_entries))
        .collect();
    let total: (i32, i32) = r
        .report
        .intervals
        .iter()
        .fold((0, 0), |a, i| (a.0 + i.film_entries, a.1 + i.our_entries));
    eprintln!(
        "--- S counts: film {} ours {}; intervals that differ in count {} of {}; first (tick, film, ours): {short:?}",
        total.0,
        total.1,
        r.report
            .intervals
            .iter()
            .filter(|i| i.film_entries != i.our_entries)
            .count(),
        r.report.intervals.len()
    );
}

fn save(label: &str, r: &Run) {
    let Some(dir) = std::env::var_os("AA_PARITY_CENSUS_DIR") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let json = serde_json::json!({
        "causes": r.comparison.causes(),
        "lanes": r.comparison.lanes.iter().map(|(k, l)| (k.to_string(), serde_json::json!({
            "first": l.first,
            "ticks_differing": l.ticks_differing,
            "ticks_compared": l.ticks_compared,
        }))).collect::<std::collections::BTreeMap<_, _>>(),
        "ticks_observed": r.comparison.ticks_observed,
        "thing_first": r.comparison.thing_first,
        "report": r.report,
        "getters": r.getters,
        "shifted": r.shifted,
        "created": r.created,
    });
    std::fs::write(
        dir.join(format!("{label}.json")),
        serde_json::to_vec_pretty(&json).unwrap(),
    )
    .unwrap();
}

/// How many entries the map saves on the deferred queues 3 (+1E8) and 4
/// (+208) of its UPDATABLES chunk (aa-play's `saved_periodic` passes over
/// them): (current slot, slots, entries).
fn saved_deferred(root: &Path) -> Option<[(i32, i32, usize); 2]> {
    let bytes = std::fs::read(
        root.join("reconstruction/assets/original/Data/Maps")
            .join(format!("{}.RMP", map_id())),
    )
    .ok()?;
    let archive = aa_mapworld::MapArchive::from_bytes(bytes).ok()?;
    let chunk = archive.decode_chunk("UPDATABLES", 1 << 30).ok()?;
    let mut at = 0usize;
    let b = &chunk.bytes;
    let mut word = || -> Option<u32> {
        let w = u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?);
        at += 4;
        Some(w)
    };
    word()?;
    let mut out = [(0, 0, 0); 2];
    for slot in &mut out {
        word()?;
        let current = word()? as i32;
        let count = word()? as i32;
        let mut entries = 0usize;
        for _ in 0..count {
            let n = word()? as i32;
            for _ in 0..n.max(0) {
                word()?;
                word()?;
                entries += 1;
            }
        }
        *slot = (current, count, entries);
    }
    Some(out)
}

#[test]
#[ignore = "reads the QA Berlin film, its native trace and the original data; set AA_GAME_ROOT"]
fn berlin_native_state_census() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT to the folder of the installed game"));
    let registries = Registries::load_v14(&root, true).unwrap();
    let film = Film::read(&env_path("AA_PARITY_BERLIN_FILM", &root.join(FILM)), &registries).unwrap();
    let trace = NativeTrace::read(&env_path("AA_PARITY_BERLIN_NATIVE", &root.join(NATIVE))).unwrap();
    eprintln!(
        "native trace: {} sampled ticks ({:?}..{:?}), {} named things, {} getters, {} creations",
        trace.ticks.len(),
        trace.ticks.keys().next(),
        trace.ticks.keys().last(),
        trace.names.len(),
        trace.getters.len(),
        trace.creations.len()
    );
    // The trace's own consistency with the film: the first battle getter.
    if let Some(g) = trace.getters.iter().find(|g| g.tick >= 1) {
        eprintln!(
            "native first battle getter: tick {} state {:#010x}",
            g.tick, g.state
        );
    }
    eprintln!(
        "saved deferred queues 3 and 4 (current, slots, entries): {:?}",
        saved_deferred(&root)
    );
    eprintln!("FormationData slots: {:?}", registries.formation_ids);
    install_id_tables(&registries);
    let only = std::env::var("AA_PARITY_CENSUS_RUNS").unwrap_or_default();
    let runs = [
        ("free", Forcing::default()),
        (
            "forced-rng",
            Forcing {
                rng: true,
                ..Default::default()
            },
        ),
        (
            "forced-rng-economy",
            Forcing {
                rng: true,
                economy: true,
                ..Default::default()
            },
        ),
        (
            "forced-all",
            Forcing {
                rng: true,
                economy: true,
                movement: true,
                health: true,
            },
        ),
    ];
    for (label, forcing) in runs {
        if !only.is_empty() && !only.split(',').any(|l| l == label) {
            continue;
        }
        let r = run(&root, &film, &registries, &trace, forcing);
        print_run(label, &r);
        save(label, &r);
        assert!(
            r.comparison.ticks_observed > 0,
            "the native trace samples the replayed ticks"
        );
    }
}
