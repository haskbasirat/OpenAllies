//! The map triggers on the original maps (ignored: need AA_GAME_ROOT, e.g.
//! `<game root>`). Run with
//! `cargo test -p aa-play --release --test triggers -- --ignored --nocapture`.
use aa_play::{Controller, GameData, GameMap, PlayerSetup, SkirmishSetup, World};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

fn root() -> PathBuf {
    PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_else(|_| r"<game root>".into()))
}

fn data() -> Arc<GameData> {
    static DATA: OnceLock<Arc<GameData>> = OnceLock::new();
    DATA.get_or_init(|| Arc::new(GameData::load(&root()).expect("GameData::load"))).clone()
}

/// A battle on `map_id` with a player on every start location: the first a
/// human, the others computers, each with its slot's nation (rus / ger when
/// the map leaves it open) and team.
fn world(map_id: &str) -> World {
    let data = data();
    let map = Arc::new(GameMap::load_with(&root(), map_id, &data).unwrap_or_else(|e| panic!("{map_id}: {e:#}")));
    let mut players = Vec::new();
    for (start, &slot) in map.player_slots.iter().enumerate() {
        let s = &map.slots[slot];
        let nation = s.nation.clone().filter(|n| data.nation(n).is_some()).unwrap_or_else(|| if s.team == 0 { "rus".into() } else { "ger".into() });
        players.push(PlayerSetup {
            name: s.id.clone(),
            nation,
            team: s.team,
            color: [255, 255, 255],
            controller: if start == 0 { Controller::Human } else { Controller::Computer(1) },
            start,
        });
    }
    let setup = SkirmishSetup { map_id: map_id.into(), players, starting_resources: None, seed: 1 };
    let mut w = World::new_skirmish(data, map, &setup).unwrap_or_else(|e| panic!("{map_id}: {e:#}"));
    assert!(aa_play::triggers::attach(&mut w, &root()), "{map_id}: no TRIGGERS");
    w
}

fn sun_of(w: &World) -> [[f32; 3]; 3] {
    aa_play::triggers::sun(w).expect("sun").0
}

#[test]
#[ignore]
fn stock_scenarios_set_their_sun() {
    // (2v2)Berlin: Map Init sets sun_night at once (duration 0).
    let mut w = world("(2v2)Berlin");
    w.step(Vec::new());
    assert_eq!(sun_of(&w)[0], [0.4, 0.4, 0.7], "Berlin night diffuse");
    assert_eq!(sun_of(&w)[2], [0.1, 0.1, 0.25], "Berlin night specular");
    assert!(w.triggers.notes.is_empty(), "{:?}", w.triggers.notes);
    // (2v2)Kursk: sun_dusk at once, then after 5 s of activation time the
    // move to sun_night over 3600 s starts (4EDB10 schedules, 4EDDC0 moves).
    let mut w = world("(2v2)Kursk");
    w.step(Vec::new());
    let dusk = sun_of(&w);
    for _ in 0..16 * 6 {
        w.step(Vec::new());
    }
    let later = sun_of(&w);
    eprintln!("Kursk dusk {dusk:?} after 6 s {later:?}");
    assert_ne!(dusk, later, "the dusk -> night transition runs");
    let states = aa_play::triggers::trigger_states(&w);
    assert!(states.iter().all(|(_, _, enabled)| !enabled), "both one-shot triggers fired: {states:?}");
    assert!(w.triggers.notes.is_empty(), "{:?}", w.triggers.notes);
}

#[test]
#[ignore]
fn mission_result_settles_the_battle() {
    // 4E9720(1, 1) -> 4F4DA0: the winner's opponents lose, the game ends.
    let mut w = world("(2v2)Berlin");
    w.step(Vec::new());
    let local = w.kingdoms.iter().find(|k| k.controller == Controller::Human).unwrap().id;
    assert!(aa_play::triggers::set_mission_result(&mut w, local, 1));
    let team = w.kingdoms[local as usize].team;
    assert_eq!(w.winner, Some(team));
    // TeamVictory: the team mates win too, every other kingdom loses.
    for k in w.kingdoms.iter().filter(|k| !k.independent) {
        let want = if k.team == team { 1 } else { 2 };
        assert_eq!(aa_play::triggers::mission_result(&w, k.id), want, "{}", k.name);
    }
    let n = aa_play::triggers::notices(&w);
    eprintln!("{n:?}");
    // A team of two: 4E9720 posts no kingdom_victory (0x35) under
    // TeamVictory, 4F4DA0 posts team_victory (0x38) instead.
    assert!(!n.iter().any(|x| matches!(x, aa_play::triggers::Notice::GameEvent { code: 0x35, .. })));
    assert!(n.iter().any(|x| matches!(x, aa_play::triggers::Notice::GameEvent { code: 0x38, .. })));
    assert!(n.iter().any(|x| matches!(x, aa_play::triggers::Notice::MissionEnded { .. })));
    // A lost mission: while a team mate plays on, two sides remain; once
    // the whole team lost, the other side wins.
    let mut w = world("(2v2)Berlin");
    w.step(Vec::new());
    assert!(aa_play::triggers::set_mission_result(&mut w, local, 2));
    assert!(w.kingdoms[local as usize].defeated);
    let mate = w.kingdoms.iter().find(|k| !k.independent && k.team == team && k.id != local).map(|k| k.id);
    if let Some(mate) = mate {
        assert_eq!(w.winner, None);
        assert!(aa_play::triggers::set_mission_result(&mut w, mate, 2));
    }
    assert!(w.winner.is_some_and(|t| t != team), "{:?}", w.winner);
    // Kingdom+19C also holds what the default victory checks decided: the
    // conditions and 4F4DA0 read one word in the original.
    let mut w = world("(2v2)Berlin");
    w.step(Vec::new());
    assert_eq!(aa_play::triggers::mission_result(&w, local), 0);
    w.kingdoms[local as usize].defeated = true;
    assert_eq!(aa_play::triggers::mission_result(&w, local), 2);
}

#[test]
#[ignore]
fn stock_scenarios_run_without_seam_failures() {
    for id in ["(2v2)Berlin", "(2v2)Kursk", "(2v2)Market_Garden", "(2v2)Moscow", "(3v3)Saint_lo", "(3v3)Stalingrad", "DeveloperMaps/(2v2)TheMesa"] {
        let mut w = world(id);
        for _ in 0..16 * 20 {
            w.step(Vec::new());
        }
        eprintln!("{id}: triggers {:?} notices {} sun {:?}", aa_play::triggers::trigger_states(&w), aa_play::triggers::notices(&w).len(), aa_play::triggers::sun(&w));
        assert!(w.triggers.notes.is_empty(), "{id}: {:?}", w.triggers.notes);
    }
}

#[test]
#[ignore]
fn campaign_missions_run() {
    let maps = root().join("reconstruction").join("assets").join("original").join("Data").join("Maps").join("Campaign");
    let mut ids: Vec<String> = std::fs::read_dir(&maps)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".RMP").map(str::to_string)))
        .collect();
    ids.sort();
    let only = std::env::var("AA_TRIGGER_MAP").ok();
    let seconds: u32 = std::env::var("AA_TRIGGER_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(30);
    let mut failures = Vec::new();
    for id in ids {
        if only.as_ref().is_some_and(|o| !id.contains(o.as_str())) {
            continue;
        }
        let map_id = format!("Campaign/{id}");
        let t = std::time::Instant::now();
        let mut w = world(&map_id);
        for _ in 0..16 * seconds {
            w.step(Vec::new());
        }
        let local = w.kingdoms.iter().find(|k| k.controller == Controller::Human).map(|k| k.id).unwrap_or(0);
        let notices = aa_play::triggers::notices(&w);
        let fired = aa_play::triggers::trigger_states(&w).iter().filter(|(_, _, e)| !e).count();
        eprintln!(
            "{map_id}: {:?} kingdoms {} fired/disabled {} notices {} objectives {:?} result {} winner {:?}",
            t.elapsed(),
            w.kingdoms.len(),
            fired,
            notices.len(),
            aa_play::triggers::objectives(&w, local).iter().map(|o| (o.key.clone(), o.state)).collect::<Vec<_>>(),
            aa_play::triggers::mission_result(&w, local),
            w.winner
        );
        for n in notices.iter().take(12) {
            eprintln!("    {n:?}");
        }
        for n in &w.triggers.notes {
            eprintln!("    NOTE {n}");
        }
        if !w.triggers.notes.is_empty() {
            failures.push((map_id, w.triggers.notes.clone()));
        }
    }
    eprintln!("{} missions with notes", failures.len());
}

/// Every shipped map's trigger elements by opcode, with the module that runs
/// each: no shipped map may use an opcode without a port (a missing one stops
/// the manager with an explicit error), and the list shows which of the
/// element services a shipped script can reach.
#[test]
#[ignore]
fn shipped_maps_use_only_ported_elements() {
    use aa_triggers::decode::{Localization, SavedManager};
    fn rmps(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                rmps(&p, out);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("rmp")) {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    rmps(&root().join("reconstruction").join("assets").join("original").join("Data").join("Maps"), &mut files);
    files.sort();
    let mut census: std::collections::BTreeMap<u32, (String, usize, std::collections::BTreeSet<String>)> = Default::default();
    let mut with_triggers = 0;
    for path in &files {
        let Ok(archive) = aa_mapworld::MapArchive::from_bytes(std::fs::read(path).unwrap()) else { continue };
        let Ok(chunk) = archive.decode_chunk("TRIGGERS", 1 << 28) else { continue };
        let saved = SavedManager::decode(&chunk.bytes, Localization::External, chunk.bytes.len().max(1 << 20)).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        with_triggers += 1;
        let map = path.file_stem().unwrap().to_string_lossy().into_owned();
        for t in &saved.triggers {
            for e in t.events.iter().chain(&t.conditions).chain(&t.actions) {
                let row = census.entry(e.opcode).or_insert_with(|| (e.name.clone(), 0, Default::default()));
                row.1 += 1;
                row.2.insert(map.clone());
            }
        }
    }
    eprintln!("{} maps, {with_triggers} with TRIGGERS", files.len());
    let mut missing = Vec::new();
    for (op, (name, count, maps)) in &census {
        let module = aa_triggers::native_module(*op);
        eprintln!("{op:3} {name:48} {count:5} in {:3} maps  {module:?}", maps.len());
        if module.is_none() {
            missing.push((*op, name.clone()));
        }
    }
    assert!(missing.is_empty(), "{missing:?}");
}
