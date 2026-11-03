//! A recruit at a location against the original: Berlin2v2_1's paratroopers (fight films).
//! The numbers are the original's, read from its trace of this film: the plane is made at
//! tick 3841 at the height of its definition, the regiment at 4577 with its first member,
//! and each next member 9 or 10 ticks after the last, in the air under the plane.
#![cfg(feature = "play")]
use aa_parity::playback::{FilmData, SyncMode, open_film};
use std::path::{Path, PathBuf};

fn film(root: &Path) -> PathBuf {
    root.join("reconstruction/qa-runs/compare-20261022/game/Profile/data/Films/archive-qa-20261101/Berlin2v2_1.RFM")
}

#[test]
#[ignore = "reads the QA film and the original data; set AA_GAME_ROOT"]
fn a_transport_flies_in_and_drops_its_regiment_one_by_one() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"));
    let data = FilmData::new(&root);
    let (mut world, mut player) = open_film(&film(&root), &data, SyncMode::Report).expect("the film opens");
    let plane_def = world.data.find("rus_transport_air").expect("rus_transport_air");
    let regiment_def = world.data.find("regiment_rus_paratrooper").expect("regiment_rus_paratrooper");
    let member_def = world.data.find("rus_paratrooper").expect("rus_paratrooper");
    let mut plane_at = None;
    let mut highest = 0.0f32;
    let mut plane_gone = None;
    let mut regiment_at = None;
    let mut members: Vec<u64> = Vec::new();
    let mut seen = 0usize;
    let mut in_the_air = false;
    while player.next_record() < 6000 {
        player.step(&mut world);
        let tick = world.tick;
        let plane = world.actors.iter().find(|a| a.def == plane_def);
        if let Some(p) = plane {
            plane_at.get_or_insert(tick);
            highest = highest.max(p.pos.z);
            if p.removed && plane_gone.is_none() {
                plane_gone = Some(tick);
            }
        }
        if regiment_at.is_none() && world.orgs.iter().any(|o| o.def == regiment_def) {
            regiment_at = Some(tick);
        }
        let count = world.actors.iter().filter(|a| a.def == member_def).count();
        if count > seen {
            members.push(tick);
            seen = count;
        }
        in_the_air |= world.actors.iter().any(|a| a.def == member_def && !a.removed && a.pos.z > world.map.height_at(a.pos.truncate()) + 1.0);
    }
    eprintln!("plane at {plane_at:?} height {highest} gone {plane_gone:?}; regiment at {regiment_at:?}; members at {members:?}");
    assert!((highest - 17.0).abs() < 0.001, "the plane flies at its operational height: {highest}");
    assert!(plane_at.is_some(), "the plane exists");
    assert!(plane_gone.is_some(), "the plane leaves the map");
    assert!(in_the_air, "what it drops falls");
    let gaps: Vec<u64> = members.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(gaps.iter().all(|g| *g == 9 || *g == 10), "a member at each of the plane's thinks: {gaps:?}");
}
