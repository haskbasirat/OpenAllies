//! The kingdom statistics on the original data: the economy tick's snapshots every
//! 60 s (0x4E37D0 -> 0x58F530), what a recruit writes (0x582E80) and the values the
//! score screen reads by StatData index (0x58F3C0). Reads the original data; set
//! AA_GAME_ROOT.
use aa_play::statistics::{ORGANIZATIONS_RECRUITED, UNITS_PRODUCED};
use aa_play::{Command, GameData, GameMap, PlayerSetup, SkirmishSetup, World};
use std::path::PathBuf;
use std::sync::Arc;

fn berlin() -> World {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let data = Arc::new(GameData::load(&root).unwrap());
    let map = Arc::new(GameMap::load_with(&root, "(2v2)Berlin", &data).unwrap());
    let player = |name: &str, nation: &str, team, start| PlayerSetup { name: name.into(), nation: nation.into(), team, color: [0, 0, 0], controller: aa_play::Controller::Human, start };
    let setup = SkirmishSetup { map_id: map.id.clone(), players: vec![player("a", "rus", 0, 0), player("b", "ger", 1, 1)], starting_resources: None, seed: 1 };
    World::new_skirmish(data, map, &setup).unwrap()
}

#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn snapshots_come_every_60_seconds_and_a_recruit_counts() {
    let mut w = berlin();
    let table = w.data.statistics.clone();
    assert!(table.production_start >= 62 && table.rate_start > table.production_start && table.derived_start > table.rate_start, "{table:?}");
    assert!(!table.derived.is_empty(), "the scores of Scoring.tgi");
    let corps = w.actors.iter().find(|a| a.kingdom == Some(0) && w.data.def(a.def).id == "rus_corps").map(|a| a.id).expect("the corps");
    let hq = w.data.find("regiment_rus_hq_infantry").unwrap();
    w.step(vec![Command::Recruit { kingdom: 0, building: corps, def: hq }]);
    assert!(!w.events.iter().any(|e| matches!(e, aa_play::Event::Refused { .. })), "{:?}", w.events);
    for _ in 0..16 * 61 {
        w.step(vec![]);
    }
    let k = w.books.kingdom(0).unwrap();
    // The graph's first point at the world start (0x4F1E10), the timer's after 60 s.
    assert_eq!(k.statistics.snapshots.len(), 2, "two snapshots after 61 s");
    let snap = &k.statistics.snapshots[1];
    let money = w.data.resources.iter().position(|r| r.id.eq_ignore_ascii_case("money")).unwrap();
    assert!(snap.value(&table, table.production_start + money) > 0.0, "money produced: {:?}", snap.production);
    // 0x58EA30 asks for no HQ component on the thing: an HQ regiment does not count.
    assert_eq!(snap.value(&table, ORGANIZATIONS_RECRUITED), 0.0, "the HQ regiment");
    // A regiment does, and the next snapshot shows it.
    let regiment = w.data.find("regiment_rus_infantry").unwrap();
    aa_play::statistics::recruited(&mut w, 0, regiment, None);
    for _ in 0..16 * 60 {
        w.step(vec![]);
    }
    let k = w.books.kingdom(0).unwrap();
    assert_eq!(k.statistics.snapshots.len(), 3, "three snapshots after 121 s");
    let snap = &k.statistics.snapshots[2];
    assert_eq!(snap.value(&table, ORGANIZATIONS_RECRUITED), 1.0, "the regiment");
    assert_eq!(snap.value(&table, UNITS_PRODUCED), 0.0, "no element counted");
    // A derived score reads its terms: the military score counts the recruit.
    assert!(snap.derived.iter().any(|v| *v != 0.0), "{:?}", snap.derived);
}
