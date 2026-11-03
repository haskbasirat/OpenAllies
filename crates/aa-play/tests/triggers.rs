//! The map triggers on the original maps (ignored: need AA_GAME_ROOT, e.g.
//! `<game root>`). Run with
//! `cargo test -p aa-play --release --test triggers -- --ignored --nocapture`.
use aa_play::{Controller, GameData, GameMap, PlayerSetup, SkirmishSetup, World};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

fn root() -> PathBuf {
    PathBuf::from(std::env::var("AA_GAME_ROOT").expect("set AA_GAME_ROOT to the folder of the installed game"))
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

/// The text parameters of the given trigger elements on every shipped map
/// (AA_TRIGGER_CENSUS: comma-separated element names, default the script
/// listen / heard elements and the UI commands; AA_TRIGGER_CENSUS_OUT: a file).
/// The mission maps' kingdom slots without a start location and the independent
/// slots that hold organizations (`map.notes`, `map.placed`): what a mission's
/// session has to create although no player starts there.
#[test]
#[ignore]
fn mission_slot_census() {
    let data = data();
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
    let base = root().join("reconstruction").join("assets").join("original").join("Data").join("Maps");
    let mut files = Vec::new();
    rmps(&base.join("Campaign"), &mut files);
    files.sort();
    for path in &files {
        let id = format!("Campaign/{}", path.file_stem().unwrap().to_string_lossy());
        let map = match aa_play::GameMap::load_with(&root(), &id, &data) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{id}: {e}");
                continue;
            }
        };
        let missing: Vec<&str> = map.slots.iter().enumerate().filter(|(i, x)| !x.independent && !map.player_slots.contains(i)).map(|(_, x)| x.id.as_str()).collect();
        let mut independent_units = std::collections::BTreeMap::<String, usize>::new();
        for p in &map.placed {
            let Some(d) = data.find(&p.thing_id) else { continue };
            let d = data.def(d);
            if !d.actor_data || !p.owner_slot.is_some_and(|s| map.slots.get(s as usize).is_some_and(|x| x.independent)) {
                continue;
            }
            if matches!(d.class, Some(aa_play::data::DefClass::Organization | aa_play::data::DefClass::Member | aa_play::data::DefClass::Truck)) {
                *independent_units.entry(p.thing_id.clone()).or_default() += 1;
            }
        }
        eprintln!("{id}: slots {} players {} {missing:?} independent units {independent_units:?}", map.slots.len(), map.player_slots.len());
    }
}

#[test]
#[ignore]
fn element_text_census() {
    use aa_triggers::decode::{Localization, SavedManager, Value};
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
    let names = std::env::var("AA_TRIGGER_CENSUS").unwrap_or_else(|_| "effect_ui_script_listen,event_script_heard,effect_ui_script_stop_listen,effect_ui_flash_button".into());
    let names: Vec<String> = names.split(',').map(|s| s.trim().to_ascii_lowercase()).collect();
    let mut files = Vec::new();
    rmps(&root().join("reconstruction").join("assets").join("original").join("Data").join("Maps"), &mut files);
    files.sort();
    let mut out = String::new();
    for path in &files {
        let Ok(archive) = aa_mapworld::MapArchive::from_bytes(std::fs::read(path).unwrap()) else { continue };
        let Ok(chunk) = archive.decode_chunk("TRIGGERS", 1 << 28) else { continue };
        let Ok(saved) = SavedManager::decode(&chunk.bytes, Localization::External, chunk.bytes.len().max(1 << 20)) else { continue };
        let map = path.file_stem().unwrap().to_string_lossy().into_owned();
        for t in &saved.triggers {
            for e in t.events.iter().chain(&t.conditions).chain(&t.actions) {
                if !names.iter().any(|n| n == "*" || e.name.eq_ignore_ascii_case(n)) {
                    continue;
                }
                let params: Vec<String> = e
                    .parameters
                    .iter()
                    .map(|p| match &p.value {
                        Value::Text(t) => String::from_utf16_lossy(t),
                        Value::Localized { key, .. } => format!("#{}", String::from_utf16_lossy(key)),
                        Value::Word(w) => format!("{w:#x}"),
                        Value::Position(p) => format!("{:?}", p.map(f32::from_bits)),
                        Value::Empty => String::new(),
                    })
                    .collect();
                out.push_str(&format!("{map}\t{}\t{}\t{}\n", String::from_utf16_lossy(&t.name), e.name, params.join("|")));
            }
        }
    }
    match std::env::var("AA_TRIGGER_CENSUS_OUT") {
        Ok(p) => std::fs::write(p, out).unwrap(),
        Err(_) => eprintln!("{out}"),
    }
}

/// sai_disabled_for_kingdom 62: 7B4AD0 / 7B8AC0 skip the kingdom's thinks.
/// allies03_stalingrad disables kingdom_2 in Map Init; the other computer
/// kingdoms keep thinking and the disabled one sends nothing.
#[test]
#[ignore]
fn a_kingdom_the_script_disabled_does_not_think() {
    let mut w = world("Campaign/allies03_stalingrad");
    aa_play::ai::attach(&mut w, Arc::new(aa_play::ai::AiData::load(&root()).expect("AiData::load")));
    w.ai.logging = true;
    for _ in 0..16 * 40 {
        w.step(Vec::new());
    }
    let disabled: Vec<_> = w.kingdoms.iter().filter(|k| aa_play::triggers::sai_disabled(&w, k.id)).map(|k| k.id).collect();
    assert!(!disabled.is_empty(), "Map Init disables kingdom_2 and independent_4");
    let computers: Vec<_> = w.kingdoms.iter().filter(|k| matches!(k.controller, Controller::Computer(_))).map(|k| k.id).collect();
    let thinking: Vec<_> = computers.iter().filter(|k| !disabled.contains(k)).collect();
    assert!(!thinking.is_empty(), "another computer kingdom plays");
    for l in &w.ai.log {
        assert!(!disabled.contains(&l.kingdom), "a disabled kingdom thought: {} at {}", l.text, l.time);
    }
    // sai_enabled_for_kingdom 61 (542080 -> 79E2F0) gives it back: the flag
    // is the only thing between the think and the kingdom.
    assert!(w.ai.stats.ge_thinks > 0 || w.ai.stats.rai_thinks > 0, "{:?}", w.ai.stats);
}

/// 4E9960 tests Kingdom+238 first: a kingdom whose default victory the script
/// turned off (kingdom_enable_default_victory_condition 114, value 0) is not
/// defeated by the elimination check, whatever it has left. A kingdom without
/// the script's word still is.
#[test]
#[ignore]
fn default_victory_off_keeps_an_eliminated_kingdom_in_the_game() {
    let mut w = world("Campaign/allies03_stalingrad");
    w.step(Vec::new());
    let off: Vec<_> = w.kingdoms.iter().filter(|k| !k.independent && !aa_play::triggers::default_victory(&w, k.id)).map(|k| k.id).collect();
    assert!(!off.is_empty(), "Map Init turns the default victory off for kingdom_1 and kingdom_2");
    let k = off[0];
    // everything the kingdom counts leaves
    let team = w.kingdoms[k as usize].team;
    for i in 0..w.kingdoms.len() {
        if w.kingdoms[i].team != team && !w.kingdoms[i].independent {
            continue;
        }
    }
    for o in &mut w.orgs {
        if o.kingdom == k {
            o.alive = false;
        }
    }
    for a in &mut w.actors {
        if a.kingdom == Some(k) {
            a.alive = false;
            a.removed = true;
        }
    }
    aa_play::victory::team_check(&mut w, k);
    assert!(!w.kingdoms[k as usize].defeated, "Kingdom+238 is off: 4E9960 does nothing");
}

/// The tutorial's first steps: 4F3FC0 answers once `effect_ui_script_listen` ran,
/// and the command heard (54C170) makes `T 01.00 BASICS` give the objective the
/// mission stalled before (`kingdom_gains_objective objective_11`). Objectives list
/// the map's text for the local kingdom (0x633A50's Kingdom+300).
#[test]
#[ignore]
fn the_tutorial_goes_on_when_the_script_hears_its_command() {
    let mut w = world("Campaign/tutorial01");
    let k = w.kingdoms[0].id;
    for _ in 0..16 * 8 {
        w.step(Vec::new());
    }
    assert!(aa_play::triggers::listening(&w, "MessageLogToggle"), "T 00.10 listens from 4 s on");
    assert!(aa_play::triggers::objectives(&w, k).is_empty(), "no objective before the player answers");
    for _ in 0..16 * 10 {
        w.step(Vec::new());
    }
    assert!(aa_play::triggers::objectives(&w, k).is_empty(), "the stall: nothing happens without the command");
    aa_play::triggers::script_heard(&mut w, "MessageLogToggle");
    for _ in 0..16 * 3 {
        w.step(Vec::new());
    }
    let list = aa_play::triggers::objectives(&w, k);
    assert!(!list.is_empty(), "T 01.00 BASICS gives the objective");
    assert!(!aa_play::triggers::listening(&w, "MessageLogToggle"), "T 01.00 stops listening");
    assert!(list.iter().all(|o| o.primary), "{list:?}");
}

/// How far a mission gets with the local kingdom under a computer player: the
/// census of the campaign (`AA_TRIGGER_MAP` a part of the id, `AA_TRIGGER_SECONDS`
/// game seconds, 600 by default): per mission the objectives with their states,
/// the triggers run, the result, every note, and the game time of each change.
#[test]
#[ignore]
fn mission_progress_census() {
    let maps = root().join("reconstruction").join("assets").join("original").join("Data").join("Maps");
    let mut ids: Vec<String> = Vec::new();
    for (dir, prefix) in [(maps.join("Campaign"), "Campaign/"), (maps.clone(), "")] {
        for e in std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()) {
            let name = e.file_name();
            if let Some(n) = name.to_str().and_then(|n| n.strip_suffix(".RMP"))
                && (!prefix.is_empty() || n.starts_with("tutorial"))
            {
                ids.push(format!("{prefix}{n}"));
            }
        }
    }
    ids.sort();
    let only = std::env::var("AA_TRIGGER_MAP").ok();
    let seconds: u32 = std::env::var("AA_TRIGGER_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(600);
    for id in ids {
        if only.as_ref().is_some_and(|o| !id.contains(o.as_str())) {
            continue;
        }
        let data = data();
        let map = Arc::new(GameMap::load_with(&root(), &id, &data).unwrap_or_else(|e| panic!("{id}: {e:#}")));
        let mut players = Vec::new();
        for (start, &slot) in map.player_slots.iter().enumerate() {
            let s = &map.slots[slot];
            let nation = s.nation.clone().filter(|n| data.nation(n).is_some()).unwrap_or_else(|| if s.team == 0 { "rus".into() } else { "ger".into() });
            players.push(PlayerSetup { name: s.id.clone(), nation, team: s.team, color: [255, 255, 255], controller: Controller::Computer(1), start });
        }
        let setup = SkirmishSetup { map_id: id.clone(), players, starting_resources: None, seed: 1 };
        let mut w = World::new_skirmish(data, map, &setup).unwrap_or_else(|e| panic!("{id}: {e:#}"));
        assert!(aa_play::triggers::attach(&mut w, &root()), "{id}: no TRIGGERS");
        let kingdoms: Vec<u32> = w.kingdoms.iter().map(|k| k.id).collect();
        let t = std::time::Instant::now();
        let mut last = Vec::new();
        let mut ended = None;
        for tick in 0..16 * seconds {
            w.step(Vec::new());
            if tick % 16 == 0 {
                // whichever kingdoms the script gives objectives
                let now: Vec<_> = kingdoms.iter().flat_map(|&k| aa_play::triggers::objectives(&w, k).into_iter().map(move |o| (k, o.key.clone(), o.state))).collect();
                if now != last {
                    eprintln!("  {id} T {:.0}: {now:?}", w.time);
                    last = now;
                }
                let r: Vec<(u32, u32)> = kingdoms.iter().map(|&k| (k, aa_play::triggers::mission_result(&w, k))).filter(|x| x.1 != 0).collect();
                if ended.is_none() && (!r.is_empty() || w.winner.is_some()) {
                    ended = Some(w.time);
                    eprintln!("  {id} T {:.0}: results {r:?} winner {:?}", w.time, w.winner);
                    break;
                }
            }
        }
        let states = aa_play::triggers::trigger_states(&w);
        let off = states.iter().filter(|(_, _, e)| !e).count();
        eprintln!("MISSION {id}: {:?} game {:.0} s, triggers off {off}/{}, objectives {:?}, ended {ended:?}", t.elapsed(), w.time, states.len(), last);
        for n in &w.triggers.notes {
            eprintln!("    NOTE {n}");
        }
    }
}

/// Follows every mission as a person would, as far as a test can: what the enabled triggers
/// wait on is what the script asks of the player, so the player selects what it asks to have
/// selected, sends the UI command it listens to, walks all its regiments into the area it
/// watches, attacks (and so captures) the thing it waits to see dead or captured, and recruits
/// what it counts. The others are computer players, as in a campaign. Prints how far each
/// mission gets and what it still asks when the time is up: the objectives open, the script's
/// last steps, the armed conditions. `AA_TRIGGER_MAP`, `AA_TRIGGER_SECONDS`.
#[test]
#[ignore]
fn mission_person_census() {
    use aa_play::ids::Thing;
    use aa_play::triggers::{ScriptThing, armed, script_thing};
    use aa_play::world::Command;
    use aa_play::Unit;
    let maps = root().join("reconstruction").join("assets").join("original").join("Data").join("Maps");
    let mut ids: Vec<String> = Vec::new();
    for e in std::fs::read_dir(maps.join("Campaign")).unwrap().filter_map(|e| e.ok()) {
        if let Some(n) = e.file_name().to_str().and_then(|n| n.strip_suffix(".RMP")) {
            ids.push(format!("Campaign/{n}"));
        }
    }
    ids.sort();
    let only = std::env::var("AA_TRIGGER_MAP").ok();
    let seconds: u32 = std::env::var("AA_TRIGGER_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(1800);
    let walk = std::env::var_os("AA_TRIGGER_WALK").is_some();
    let make = |id: &str, person: Option<usize>| {
        let data = data();
        let map = Arc::new(GameMap::load_with(&root(), id, &data).unwrap_or_else(|e| panic!("{id}: {e:#}")));
        let mut players = Vec::new();
        for (start, &slot) in map.player_slots.iter().enumerate() {
            let s = &map.slots[slot];
            let nation = s.nation.clone().filter(|n| data.nation(n).is_some()).unwrap_or_else(|| if s.team == 0 { "usa".into() } else { "ger".into() });
            let team = if s.team == u32::MAX || s.team_id.is_empty() { map.teams.len() as u32 + start as u32 } else { s.team };
            let p = PlayerSetup { name: s.id.clone(), nation, team, color: [255, 255, 255], controller: if person == Some(start) { Controller::Human } else { Controller::Computer(1) }, start };
            if person == Some(start) {
                players.insert(0, p);
            } else {
                players.push(p);
            }
        }
        let setup = SkirmishSetup { map_id: id.into(), players, starting_resources: None, seed: 1 };
        let mut w = World::new_skirmish(data, map, &setup).unwrap_or_else(|e| panic!("{id}: {e:#}"));
        assert!(aa_play::triggers::attach(&mut w, &root()), "{id}: no TRIGGERS");
        w
    };
    for id in ids {
        if only.as_ref().is_some_and(|o| !id.contains(o.as_str())) {
            continue;
        }
        // the slot the script gives objectives first, the mission's player
        let mut dry = make(&id, None);
        let mut person = None;
        for _ in 0..16 * 60 {
            dry.step(Vec::new());
            person = dry.kingdoms.iter().enumerate().find(|(_, k)| !aa_play::triggers::objectives(&dry, k.id).is_empty()).map(|(i, _)| i);
            if person.is_some() {
                break;
            }
        }
        drop(dry);
        let mut w = make(&id, Some(person.unwrap_or(0)));
        let k = w.kingdoms[0].id;
        let script_name = aa_play::triggers::kingdom_script_name(&w, k).unwrap_or_default();
        let t0 = std::time::Instant::now();
        let mut fired: Vec<String> = Vec::new();
        let mut was_on: std::collections::BTreeMap<u32, bool> = aa_play::triggers::trigger_states(&w).into_iter().map(|t| (t.0, t.2)).collect();
        let mut objectives = Vec::new();
        let mut result = 0;
        let mut orders: std::collections::BTreeMap<String, f32> = Default::default();
        let mut pending: Vec<Command> = Vec::new();
        let mut goal: Option<(String, f32)> = None;
        let mut tried = 0usize;
        for tick in 0..16 * seconds {
            w.step(std::mem::take(&mut pending));
            if tick % 16 != 0 {
                continue;
            }
            for (i, name, on) in aa_play::triggers::trigger_states(&w) {
                let was = was_on.insert(i, on).unwrap_or(false);
                if was && !on {
                    fired.push(format!("{:.0} {name}", w.time));
                }
            }
            if tick % (16 * 120) == 0 {
                eprintln!("  {id} T {:.0} after {:?}", w.time, t0.elapsed());
            }
            objectives = aa_play::triggers::objectives(&w, k).into_iter().map(|o| (o.key.clone(), o.state)).collect::<Vec<_>>();
            result = aa_play::triggers::mission_result(&w, k);
            if result != 0 || w.winner.is_some() {
                break;
            }
            // the script holds a person's hands back during its cutscenes
            if aa_play::triggers::cutscene(&w) {
                continue;
            }
            let own: Vec<Unit> = w.orgs.iter().filter(|o| o.alive && o.kingdom == k && w.data.def(o.def).max_attachments.is_none()).map(|o| Unit::Org(o.id)).collect();
            let mine = |w: &World, kingdom: &str| kingdom.is_empty() || kingdom.eq_ignore_ascii_case(&script_name) || aa_play::triggers::kingdom_script_name(w, k).is_some_and(|n| n.eq_ignore_ascii_case(kingdom));
            let actor_of = |w: &World, parameter: &str| match script_thing(w, parameter) {
                Some(ScriptThing::Thing(Thing::Actor(x))) => Some(x),
                Some(ScriptThing::Thing(Thing::Org(x))) => w.orgs.get(x as usize).and_then(|o| o.members.first().copied()),
                _ => None,
            };
            // what the regiments are asked to go for: an actor to attack or a place to stand
            let mut goals: Vec<(String, Option<aa_play::ActorId>, glam::Vec2)> = Vec::new();
            let trigger_list = armed(&w);
            for a in &trigger_list {
                for e in &a.waits {
                    let p = |i: usize| e.parameters.get(i).map(String::as_str).unwrap_or("");
                    match e.name.as_str() {
                        "event_script_heard" => {
                            if aa_play::triggers::listening(&w, p(0)) {
                                aa_play::triggers::script_heard(&mut w, p(0));
                                // the button a person pressed does what it says
                                if let Some(ids) = p(0).strip_prefix("ActorDataSelectCommand ")
                                    && let Some(def) = w.data.find(ids)
                                {
                                    let building = w.actors.iter().find(|x| x.alive && x.kingdom == Some(k) && w.data.def(x.def).recruits.contains(&def)).map(|x| x.id);
                                    if let Some(building) = building {
                                        pending.push(Command::Recruit { kingdom: k, building, def });
                                    }
                                }
                            }
                        }
                        "actor_selected_by_kingdom" if mine(&w, p(1)) => {
                            let unit = match script_thing(&w, p(0)) {
                                Some(ScriptThing::Thing(Thing::Actor(x))) => Some(Unit::Actor(x)),
                                Some(ScriptThing::Thing(Thing::Org(x))) => Some(Unit::Org(x)),
                                _ => None,
                            };
                            if let Some(u) = unit {
                                aa_play::triggers::set_selection(&mut w, k, vec![u]);
                            }
                        }
                        "actor_data_selected_by_kingdom" if mine(&w, p(1)) => {
                            let found = w.actors.iter().find(|x| x.alive && x.kingdom == Some(k) && w.data.def(x.def).id.eq_ignore_ascii_case(p(0))).map(|x| Unit::Actor(x.id));
                            let found = found.or_else(|| w.orgs.iter().find(|o| o.alive && o.kingdom == k && w.data.def(o.def).id.eq_ignore_ascii_case(p(0))).map(|o| Unit::Org(o.id)));
                            if let Some(u) = found {
                                aa_play::triggers::set_selection(&mut w, k, vec![u]);
                            }
                        }
                        "actor_enters_area" | "actor_inside_area" => {
                            // a named actor of the player's walks there; another's is not the player's to walk
                            let asked = actor_of(&w, p(0));
                            if asked.is_none_or(|x| w.actor(x).is_some_and(|x| x.kingdom == Some(k)))
                                && let Some(ScriptThing::Area(at)) = script_thing(&w, p(1))
                            {
                                goals.push((format!("area {}", p(1)), None, at));
                            }
                        }
                        "actor_captured_by_kingdom" | "actor_owned_by_kingdom" if mine(&w, p(1)) => {
                            if let Some(x) = actor_of(&w, p(0)).and_then(|x| w.actor(x)).filter(|x| x.alive && x.kingdom != Some(k)) {
                                let city = w.data.def(x.def).city;
                                goals.push((format!("take {}", p(0)), (!city || walk).then_some(x.id), x.pos.truncate()));
                            }
                        }
                        "actor_destroys_actor" | "actor_is_dead" | "actor_visible_to_kingdom" => {
                            let which = if e.name == "actor_destroys_actor" { 1 } else { 0 };
                            let team = |who: Option<aa_play::KingdomId>| who.and_then(|q| w.kingdoms.iter().find(|x| x.id == q)).map(|x| x.team);
                            if let Some(x) = actor_of(&w, p(which)).and_then(|x| w.actor(x)).filter(|x| x.alive && team(x.kingdom) != team(Some(k))) {
                                goals.push((format!("kill {}", p(which)), Some(x.id), x.pos.truncate()));
                            }
                        }
                        // the default defeat of another side: its forces fall
                        "kingdom_default_lose" if walk && !mine(&w, p(0)) => {
                            let them: Vec<aa_play::ActorId> = w.actors.iter().filter(|x| x.alive && x.kingdom.and_then(|q| aa_play::triggers::kingdom_script_name(&w, q)).is_some_and(|n| n.eq_ignore_ascii_case(p(0))) && w.data.def(x.def).health > 0.0).map(|x| x.id).collect();
                            if let Some(by) = w.orgs.iter().filter(|o| o.alive && o.kingdom == k).flat_map(|o| o.members.iter().copied()).next() {
                                for a in them {
                                    aa_play::combat::aid_kill(&mut w, a, by);
                                }
                            }
                        }
                        // engaged or routing: the player attacks it
                        "actor_has_state" if p(1) == "0x4" || p(1) == "0xe" => {
                            if let Some(x) = actor_of(&w, p(0)).and_then(|x| w.actor(x)).filter(|x| x.alive && x.kingdom != Some(k)) {
                                goals.push((format!("engage {}", p(0)), Some(x.id), x.pos.truncate()));
                            }
                        }
                        // attached to an HQ of the player's, the regiments it names
                        "actor_has_state" if p(1) == "0x29" => {
                            let org = match script_thing(&w, p(0)) {
                                Some(ScriptThing::Thing(Thing::Org(o))) => Some(o),
                                Some(ScriptThing::Thing(Thing::Actor(x))) => w.actor(x).and_then(|x| x.org),
                                _ => None,
                            };
                            let hq = w.orgs.iter().filter(|o| o.alive && o.kingdom == k && w.data.def(o.def).max_attachments.is_some()).find_map(|o| o.members.first().copied());
                            if let (Some(org), Some(hq)) = (org, hq)
                                && orders.get(p(0)).is_none_or(|&t| w.time - t > 30.0)
                            {
                                orders.insert(p(0).to_string(), w.time);
                                pending.push(Command::Attach { kingdom: k, org, hq });
                            }
                        }
                        "kingdom_recruits_actor" if mine(&w, p(0)) => {
                            let Some(def) = w.data.find(p(1)) else { continue };
                            let building = w.actors.iter().find(|x| x.alive && x.kingdom == Some(k) && w.data.def(x.def).recruits.contains(&def)).map(|x| x.id);
                            if let Some(building) = building
                                && orders.get(p(1)).is_none_or(|&t| w.time - t > 30.0)
                            {
                                orders.insert(p(1).to_string(), w.time);
                                pending.push(Command::Recruit { kingdom: k, building, def });
                            }
                        }
                        "kingdom_finishes_construction_on_actor" if mine(&w, p(0)) => {
                            let Some(def) = w.data.find(p(1)) else { continue };
                            let truck = w.actors.iter().find(|x| x.alive && x.kingdom == Some(k) && w.data.def(x.def).unpacks_to == Some(def)).map(|x| (x.id, x.pos.truncate()));
                            if let Some((truck, at)) = truck
                                && orders.get(p(1)).is_none_or(|&t| w.time - t > 30.0)
                            {
                                orders.insert(p(1).to_string(), w.time);
                                pending.push(Command::Deploy { kingdom: k, truck, at: at + glam::Vec2::new(-12.0, -12.0), facing: 0.0 });
                            }
                        }
                        _ => {}
                    }
                }
            }
            // AA_TRIGGER_WALK: what the script asks of the fight simply happens, so the census
            // shows how far the script goes whatever the battle would have done
            if walk {
                for (name, target, _) in &goals {
                    match target {
                        Some(x) if name.starts_with("take") => {
                            if w.actor(*x).is_some_and(|a| a.alive && a.kingdom != Some(k)) {
                                aa_play::triggers::aid_capture(&mut w, *x, k);
                            }
                        }
                        Some(_) if name.starts_with("engage") => {}
                        Some(x) => {
                            if let Some(by) = w.orgs.iter().filter(|o| o.alive && o.kingdom == k).flat_map(|o| o.members.iter().copied()).next() {
                                let list: Vec<aa_play::ActorId> = match w.actor(*x).and_then(|a| a.org) {
                                    Some(o) => w.orgs[o as usize].members.clone(),
                                    None => vec![*x],
                                };
                                for a in list {
                                    aa_play::combat::aid_kill(&mut w, a, by);
                                }
                            }
                        }
                        None => {}
                    }
                }
                goals.retain(|g| g.1.is_none() || g.0.starts_with("engage"));
            }
            if !own.is_empty() && !goals.is_empty() {
                let key = goal.take().filter(|(g, since): &(String, f32)| goals.iter().any(|x| x.0 == *g) && w.time - *since < 120.0);
                let (name, since) = key.unwrap_or_else(|| {
                    let next = (tried..tried + goals.len()).map(|i| i % goals.len()).next().unwrap();
                    tried += 1;
                    (goals[next].0.clone(), w.time)
                });
                if let Some(g) = goals.iter().find(|x| x.0 == name) {
                    if orders.get(&name).is_none_or(|&t| w.time - t > 30.0) {
                        orders.insert(name.clone(), w.time);
                        pending.push(match g.1 {
                            Some(target) => Command::Attack { kingdom: k, units: own.clone(), target },
                            None => Command::Move { kingdom: k, units: own.clone(), to: g.2, attack_move: true },
                        });
                    }
                }
                goal = Some((name, since));
            }
        }
        let states = aa_play::triggers::trigger_states(&w);
        let off = states.iter().filter(|(_, _, e)| !e).count();
        eprintln!("PERSON {id}: slot {person:?} {:?} game {:.0} s, result {result}, winner {:?}, triggers off {off}/{}", t0.elapsed(), w.time, w.winner, states.len());
        eprintln!("    objectives {objectives:?}");
        for f in fired.iter().rev().take(6).rev() {
            eprintln!("    fired {f}");
        }
        for a in armed(&w).iter().filter(|a| a.waits.iter().any(|e| e.name != "activation_time")).take(12) {
            eprintln!("    asks {}: {}", a.name, a.waits.iter().map(|e| format!("{}({})", e.name, e.parameters.join(","))).collect::<Vec<_>>().join(" "));
        }
        for n in &w.triggers.notes {
            eprintln!("    NOTE {n}");
        }
        let units: Vec<String> = w.orgs.iter().filter(|o| o.alive && o.kingdom == k).map(|o| w.data.def(o.def).id.clone()).chain(w.actors.iter().filter(|a| a.alive && a.kingdom == Some(k) && a.org.is_none()).map(|a| w.data.def(a.def).id.clone())).collect();
        eprintln!("    OWN {units:?}");
    }
}

/// 4D7960 from a trigger's object_destroyed: the damage record stands, so the Destroyed event
/// can name who hit the thing last (54BE10). Element 3 asks for a killer: Tutorial 1's
/// T 02.070 waits on this and the script's rout of the Germans.
#[test]
#[ignore]
fn a_scripted_kill_keeps_its_attacker() {
    use aa_play::ids::Thing;
    use aa_play::triggers::{ScriptThing, script_thing};
    let mut w = world("Campaign/tutorial01");
    w.step(Vec::new());
    let Some(ScriptThing::Thing(Thing::Org(org))) = script_thing(&w, "0x840") else { panic!("no German regiment") };
    let by = w.actors.iter().find(|a| a.alive && a.kingdom == Some(w.kingdoms[0].id)).map(|a| a.id).expect("a player's actor");
    for m in w.orgs[org as usize].members.clone() {
        aa_play::combat::aid_kill(&mut w, m, by);
    }
    for _ in 0..3 {
        w.step(Vec::new());
    }
    assert!(!w.orgs[org as usize].alive);
    assert_eq!(w.combat.org(org).and_then(|c| c.last_hit).map(|h| h.0), Some(by));
}

/// The ability axis08's script uses (its rockets) is a general's, so `trigger_use` has its data
/// when the generals are installed; a world without them notes it as not modelled.
#[test]
#[ignore]
fn a_general_holds_the_ability_of_the_scripts() {
    let g = aa_play::special_ops::GeneralData::load(&root(), &data()).unwrap();
    let op = g.generals.iter().flat_map(|x| x.abilities.iter()).find(|o| o.id.eq_ignore_ascii_case("v_weapons")).expect("v_weapons");
    assert_eq!(op.area, 12.0);
    assert!(op.projectile.is_some());
}

/// 4F1E10's last loop: every kingdom has its first snapshot at the start, the score
/// graph's point 0; the 60 s timer (4E37D0) adds the next.
#[test]
#[ignore]
fn the_score_starts_with_a_snapshot() {
    let mut w = world("(2v2)Berlin");
    let k = w.kingdoms[0].id;
    let count = |w: &World| w.books.kingdom(k).map_or(0, |b| b.statistics.snapshots.len());
    assert_eq!(count(&w), 1);
    for _ in 0..16 * 61 {
        w.step(Vec::new());
    }
    assert_eq!(count(&w), 2);
}

/// 543BA0: a transported creation is a flight; its cargo appears at the unload
/// point on arrival, not at once. A map with transported elements reports the
/// flights it started through Notice::Transport.
#[test]
#[ignore]
fn transported_creations_fly_in() {
    use aa_triggers::decode::{Localization, SavedManager};
    let maps = root().join("reconstruction").join("assets").join("original").join("Data").join("Maps").join("Campaign");
    let mut found = None;
    for e in std::fs::read_dir(&maps).unwrap().flatten() {
        let p = e.path();
        if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("rmp")) {
            continue;
        }
        let Ok(archive) = aa_mapworld::MapArchive::from_bytes(std::fs::read(&p).unwrap()) else { continue };
        let Ok(chunk) = archive.decode_chunk("TRIGGERS", 1 << 28) else { continue };
        let Ok(saved) = SavedManager::decode(&chunk.bytes, Localization::External, chunk.bytes.len().max(1 << 20)) else { continue };
        if saved.triggers.iter().any(|t| t.actions.iter().any(|a| a.opcode == 30)) {
            found = Some(format!("Campaign/{}", p.file_stem().unwrap().to_string_lossy()));
            break;
        }
    }
    let id = found.expect("a campaign map with a transported creation");
    let mut w = world(&id);
    let mut seen = 0;
    for _ in 0..16 * 240 {
        w.step(Vec::new());
        let notices = aa_play::triggers::notices(&w);
        if let Some(aa_play::triggers::Notice::Transport { ids, from, to, seconds, .. }) = notices[seen..].iter().find(|n| matches!(n, aa_play::triggers::Notice::Transport { .. })) {
            eprintln!("{id}: {ids} from {from:?} to {to:?} in {seconds} s");
            assert!(*seconds > 0.0, "a flight takes time");
            return;
        }
        seen = notices.len();
    }
    eprintln!("{id}: no transported creation within 240 s of an idle game");
}

/// What a mission starts with (AA_TRIGGER_MAP, default the first tutorial):
/// tools, not a check.
#[test]
#[ignore]
fn mission_start_listing() {
    let id = std::env::var("AA_TRIGGER_MAP").unwrap_or_else(|_| "Campaign/tutorial01".into());
    let mut w = world(&id);
    for s in &w.map.slots {
        eprintln!("slot {} nation {:?} faction {:?} team {} independent {} color {:?}", s.id, s.nation, s.faction, s.team, s.independent, s.color_id);
    }
    let seconds: u32 = std::env::var("AA_TRIGGER_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    for _ in 0..16 * seconds {
        w.step(Vec::new());
    }
    for k in &w.kingdoms {
        let b = w.books.kingdom(k.id);
        eprintln!("kingdom {} {} team {} {:?} stock {:?}", k.id, k.name, k.team, k.controller, b.map(|b| b.stockpiled.clone()));
    }
    for o in w.orgs.iter().filter(|o| o.alive) {
        eprintln!("org {} {} kingdom {} members {}", o.id, w.data.def(o.def).id, o.kingdom, o.members.len());
    }
    for s in &w.map.slots {
        eprintln!("slot {} index {:?} to kingdoms {:?} to teams {:?}", s.id, s.kingdom_index, s.relations_to_kingdoms, s.relations_to_teams);
    }
    eprintln!("relations {:?}", w.books.relations);
    let mut classes = std::collections::BTreeMap::<String, usize>::new();
    for a in w.actors.iter().filter(|a| a.alive && !a.removed) {
        let d = w.data.def(a.def);
        if a.org.is_none() {
            eprintln!("actor {} {} kingdom {:?} at {:?} class {:?}", a.id, d.id, a.kingdom, a.pos.truncate(), d.class);
        }
        *classes.entry(d.id.clone()).or_default() += 1;
    }
    eprintln!("{classes:?}");
    let mut placed = std::collections::BTreeMap::<String, usize>::new();
    for p in &w.map.placed {
        *placed.entry(p.thing_id.clone()).or_default() += 1;
    }
    eprintln!("placed {placed:?}");
    for p in w.map.placed.iter().filter(|p| p.thing_id.contains("regiment")) {
        let d = w.data.find(&p.thing_id);
        eprintln!("placed org {} owner {:?} at {:?} id {:#x} def {:?} class {:?} actor_data {:?}", p.thing_id, p.owner_slot, p.position, p.serialized_id, d, d.map(|d| w.data.def(d).class), d.map(|d| w.data.def(d).actor_data));
    }
    for n in aa_play::triggers::notices(&w) {
        eprintln!("{n:?}");
    }
    eprintln!("notes {:?}", w.triggers.notes);
}
