//! Census of what the film orders carry (order lane): for every ActorCommand
//! and every default order, which optional fields (`CommandWire::flags`) occur,
//! over the QA fight films and the v14 films of the frozen corpus.
//!
//! ```text
//! AA_GAME_ROOT=<game root> cargo test -p aa-parity --release --test order_census -- --ignored --nocapture
//! ```
//! `AA_ORDER_CENSUS_CORPUS=<dir of RFM files>` adds every film there that decodes
//! on v14. Prints only; proves nothing by itself.

use aa_parity::registries::Registries;
use aa_parity::{Decree, Film, classify};
use std::collections::BTreeMap;
use std::path::PathBuf;

const QA: &str = "reconstruction/qa-runs/compare-20261022/game/Profile/data/Films";
const FILMS: &[&str] = &[
    "archive-qa-20261025/Berlin2v2_0.RFM",
    "archive-qa-20261101/Comrades2v2_0.RFM",
    "archive-qa-20261101/Farmhouses4FFA_0.RFM",
    "archive-qa-20261101/Kursk2v2_0.RFM",
    "archive-qa-20261101/Berlin2v2_1.RFM",
];

#[derive(Default)]
struct Row {
    count: usize,
    trailing: usize,
    boolean: usize,
    selection: BTreeMap<usize, usize>,
}

#[test]
#[ignore = "reads the QA films and the original data; set AA_GAME_ROOT"]
fn order_fields() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let registries = Registries::load_v14(&root, true).unwrap();
    let mut paths: Vec<PathBuf> = FILMS.iter().map(|f| root.join(QA).join(f)).collect();
    if let Some(dir) = std::env::var_os("AA_ORDER_CENSUS_CORPUS") {
        let mut more: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("RFM")))
            .collect();
        more.sort();
        paths.extend(more);
    }
    let mut rows: BTreeMap<(String, u16), Row> = BTreeMap::new();
    let mut other: BTreeMap<String, usize> = BTreeMap::new();
    let mut scalars: BTreeMap<(String, u32), usize> = BTreeMap::new();
    let mut read = 0;
    for path in &paths {
        let Ok(film) = Film::read(path, &registries) else {
            continue;
        };
        if film.failures().next().is_some() {
            continue;
        }
        read += 1;
        let mut selections: BTreeMap<u32, usize> = BTreeMap::new();
        for d in film.ticks.iter().flat_map(|t| &t.decrees) {
            let player = d.player.unwrap_or(0);
            let (wire, selected) = match &d.decree {
                Decree::SelectionSet { slot } => {
                    selections.insert(player, usize::from(*slot != 0));
                    continue;
                }
                Decree::SelectionAdd { .. } => {
                    *selections.entry(player).or_default() += 1;
                    continue;
                }
                Decree::SelectionRemove { .. } => {
                    let s = selections.entry(player).or_default();
                    *s = s.saturating_sub(1);
                    continue;
                }
                Decree::SelectionClear | Decree::SelectionClearNamed { .. } => {
                    selections.insert(player, 0);
                    continue;
                }
                Decree::TellSelectedCommand(wire) => {
                    (wire, selections.get(&player).copied().unwrap_or(0))
                }
                Decree::TellActorCommand { command, .. } => (command, 1),
                _ => {
                    if let Some(kind) = classify::order_kind(d.descriptor as usize, &d.decree) {
                        *other.entry(kind).or_default() += 1;
                    }
                    continue;
                }
            };
            let name = classify::actor_command(&d.decree)
                .map_or_else(|| "default".to_owned(), |(i, n, _)| format!("{i:02} {n}"));
            if wire.flags >> 4 & 1 != 0 {
                *scalars.entry((name.clone(), wire.scalar)).or_default() += 1;
            }
            let row = rows.entry((name, wire.flags)).or_default();
            row.count += 1;
            row.trailing += usize::from(wire.trailing);
            row.boolean += usize::from(wire.boolean);
            *row.selection.entry(selected.min(13)).or_default() += 1;
        }
    }
    eprintln!("{read} films");
    let bit = |flags: u16| {
        const NAMES: [&str; 11] = [
            "target", "object", "pos", "aux", "scalar", "params", "def", "formation", "property",
            "bool", "alt",
        ];
        (0..11)
            .filter(|b| flags >> b & 1 != 0)
            .map(|b| NAMES[b])
            .collect::<Vec<_>>()
            .join("+")
    };
    for ((name, flags), row) in &rows {
        eprintln!(
            "{name:28} {flags:#05x} {:32} n={:6} trailing={:5} bool={:4} selected={:?}",
            bit(*flags),
            row.count,
            row.trailing,
            row.boolean,
            row.selection
        );
    }
    eprintln!("{other:#?}");
    let mut by_name: BTreeMap<&String, Vec<(usize, f32)>> = BTreeMap::new();
    for ((name, bits), n) in &scalars {
        by_name.entry(name).or_default().push((*n, f32::from_bits(*bits)));
    }
    for (name, mut values) in by_name {
        values.sort_by(|a, b| b.0.cmp(&a.0));
        values.truncate(8);
        eprintln!("scalar {name}: {values:?}");
    }
}

/// What the orders of the fight films name, through the `N` lines (GTID ->
/// definition) of their native traces.
#[test]
#[ignore = "reads the QA films and traces; set AA_GAME_ROOT"]
fn order_targets() {
    use std::io::BufRead;
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let registries = Registries::load_v14(&root, true).unwrap();
    let cases = [
        ("archive-qa-20261101/Comrades2v2_0.RFM", "fight-comrades2v2_0-native.txt"),
        ("archive-qa-20261101/Farmhouses4FFA_0.RFM", "fight-farmhouses4ffa_0-native.txt"),
        ("archive-qa-20261101/Kursk2v2_0.RFM", "fight-kursk2v2_0-native.txt"),
        ("archive-qa-20261101/Berlin2v2_1.RFM", "fight-berlin2v2_1-native.txt"),
    ];
    let mut rows: BTreeMap<String, usize> = BTreeMap::new();
    for (film, trace) in cases {
        let film = Film::read(&root.join(QA).join(film), &registries).unwrap();
        let path = root.join("reconstruction/qa-runs/compare-20261022/traces").join(trace);
        let reader = std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(path).unwrap());
        let mut names: BTreeMap<u32, String> = BTreeMap::new();
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
        let by_slot = |slot: u16| -> String {
            names
                .iter()
                .rev()
                .find(|(g, _)| **g as u16 == slot)
                .map_or_else(|| format!("slot{slot:x}?"), |(_, n)| n.clone())
        };
        let mut selections: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
        for d in film.ticks.iter().flat_map(|t| &t.decrees) {
            let player = d.player.unwrap_or(0);
            match &d.decree {
                Decree::SelectionSet { slot } => {
                    let s = selections.entry(player).or_default();
                    s.clear();
                    if *slot != 0 {
                        s.push(*slot);
                    }
                }
                Decree::SelectionAdd { slot } => selections.entry(player).or_default().push(*slot),
                Decree::SelectionRemove { slot } => {
                    selections.entry(player).or_default().retain(|s| s != slot)
                }
                Decree::SelectionClear | Decree::SelectionClearNamed { .. } => {
                    selections.entry(player).or_default().clear()
                }
                Decree::TellSelectedCommand(w) => {
                    let name = classify::actor_command(&d.decree)
                        .map_or_else(|| "default".to_owned(), |(i, n, _)| format!("{i:02} {n}"));
                    let who = selections
                        .get(&player)
                        .and_then(|s| s.first())
                        .map_or_else(|| "none".to_owned(), |s| by_slot(*s));
                    let what = if w.flags & 1 != 0 {
                        names.get(&w.word).cloned().unwrap_or_else(|| "gone".into())
                    } else if w.flags & 4 != 0 {
                        "pos".into()
                    } else {
                        "-".into()
                    };
                    *rows.entry(format!("{name} | {what}")).or_default() += 1;
                    *rows
                        .entry(format!("{name} | by {who}"))
                        .or_default() += 1;
                }
                Decree::TellActorCommand { command, actor } => {
                    let name = classify::actor_command(&d.decree)
                        .map_or_else(|| "default".to_owned(), |(i, n, _)| format!("{i:02} {n}"));
                    *rows.entry(format!("{name} | actor {}", by_slot(*actor))).or_default() += 1;
                    let _ = command;
                }
                _ => {}
            }
        }
    }
    for (k, n) in &rows {
        eprintln!("{n:7} {k}");
    }
}
