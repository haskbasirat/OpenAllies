//! The stock maps' saved queues (UPDATABLES): every saved deferred entry has an
//! owner aa-play knows. Reads the original data; set AA_GAME_ROOT.
use aa_play::map::SavedOwner;
use aa_play::{GameData, GameMap};
use std::collections::BTreeMap;
use std::path::PathBuf;

const MAPS: [&str; 4] = ["(2v2)Berlin", "(2v2)Kursk", "DeveloperMaps/(2v2)Comrades", "DeveloperMaps/(4FFA)Farmhouses"];
const POST_LOAD_DRAWS: [usize; 4] = [0, 0, 4, 16];

fn root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"))
}

/// The owners of the saved queues, by kind and definition.
fn census(map: &GameMap, queues: &[aa_play::map::SavedPeriodic]) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for q in queues {
        for (o, _) in q.buckets.iter().flatten() {
            let key = match *o {
                SavedOwner::Placed { index, slot } => format!("component {slot} of {}", map.placed[index].thing_id),
                SavedOwner::Doodad { index, slot } => format!("doodad {slot} of {}", map.placed[index].thing_id),
                SavedOwner::Kingdom { .. } => "kingdom".to_owned(),
                SavedOwner::KingdomComponent { component, .. } => format!("kingdom component {component}"),
                SavedOwner::Unknown(id) => format!("unknown {id}"),
            };
            *out.entry(key).or_insert(0) += 1;
        }
    }
    out
}

#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn the_stock_maps_save_their_doodads_on_queue_3() {
    let root = root();
    let data = GameData::load(&root).unwrap();
    for id in MAPS {
        let map = match GameMap::load_with(&root, id, &data) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{id}: {e}");
                continue;
            }
        };
        let deferred = map.saved_deferred.as_ref().expect("saved deferred queues");
        assert_eq!(deferred.len(), 2);
        let events: Vec<usize> = deferred.iter().map(|q| q.buckets.iter().flatten().count()).collect();
        let used: Vec<usize> = deferred.iter().map(|q| q.buckets.iter().filter(|b| !b.is_empty()).count()).collect();
        let rings: Vec<usize> = deferred.iter().map(|q| q.buckets.len()).collect();
        eprintln!("{id}: deferred events {events:?} in {used:?} of {rings:?} slots, cursors {:?}", deferred.iter().map(|q| q.current).collect::<Vec<_>>());
        for (k, n) in census(&map, deferred) {
            eprintln!("  deferred {n} {k}");
        }
        let periodic = map.saved_queues.as_ref().expect("saved periodic queues");
        let unknown: Vec<(String, usize)> = census(&map, periodic).into_iter().filter(|(k, _)| k.starts_with("unknown")).collect();
        eprintln!("  periodic unknown: {unknown:?}");
        eprintln!("  notes: {:?}", map.load_notes);
    }
}

/// The PostLoad pass 0x4FD3B3 adds to queue 1 what no saved queue holds. The film
/// traces of these maps count the draws of 0x581260 (chain 4D3586 -> 50AC60).
#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn post_load_registers_what_no_saved_queue_holds() {
    let root = root();
    let data = GameData::load(&root).unwrap();
    for (id, native) in MAPS.into_iter().zip(POST_LOAD_DRAWS) {
        let map = GameMap::load_with(&root, id, &data).unwrap();
        let order: Vec<usize> = (0..map.placed.len()).filter(|&i| map.placed[i].serialized_id != 0).collect();
        let got = aa_play::scheduler::post_load_registrations(&map, &data, &order);
        let named: Vec<String> = got.iter().map(|&(i, s)| format!("{} {s}", map.placed[i].thing_id)).collect();
        assert_eq!(got.len(), native, "{id}: {named:?}");
    }
}

/// A map's herds: the organization and member records of (2v2)Kursk, their Minds as
/// the original saved them, and the states the regiment and element hosts run.
#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn kursk_keeps_its_herds_and_their_minds() {
    use aa_mind::construct::Kind;
    let root = root();
    let data = GameData::load(&root).unwrap();
    let map = GameMap::load_with(&root, "(2v2)Kursk", &data).unwrap();
    let organizations: Vec<_> = map.placed.iter().filter(|p| p.organization.is_some()).collect();
    let members: Vec<_> = map.placed.iter().filter(|p| p.member_of.is_some()).collect();
    assert_eq!((organizations.len(), members.len()), (7, 10));
    for o in &organizations {
        let (saved, _) = aa_mind::saved_load::decode(Kind::Regiment, o.mind.as_ref().unwrap()).unwrap();
        assert_eq!(saved.base().unwrap().stack.as_deref(), Some(&[0x54][..]), "{}: the rampage", o.thing_id);
        assert!(data.def(data.find(&o.thing_id).unwrap()).rampage.points_to_try > 0, "{}: its config", o.thing_id);
    }
    for m in &members {
        let (saved, _) = aa_mind::saved_load::decode(Kind::Element, m.mind.as_ref().unwrap()).unwrap();
        let top = saved.base().unwrap().stack.as_ref().unwrap()[0];
        assert!(top == 0x68 || top == 0x7c, "{}: state {top:#x}", m.thing_id);
        assert_eq!(m.current_component, Some(2), "{}: the Character is active", m.thing_id);
    }
}
