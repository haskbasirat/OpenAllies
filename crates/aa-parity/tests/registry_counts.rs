//! The decree-payload registry counts and slot orders computed from the game
//! data against what NativeSampler read from the original (`[registry]+0x28`
//! and the SharedProperty slots, QA copy, Berlin film run of 2026-10-25: `REG`
//! and `REGIDS` lines).
//!
//! `AA_GAME_ROOT='<game root>' cargo test -p aa-parity --test registry_counts -- --ignored --nocapture`
use aa_parity::registries::*;
use std::path::PathBuf;

fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"))
}

#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn computed_counts_match_the_live_original() {
    let r = Registries::load_v14(&game_root(), true).unwrap();
    let mut bad = Vec::new();
    for (reg, n) in MEASURED_V14 {
        let ours = r.count(reg);
        eprintln!("{:20} live {n:5} computed {ours:?}", registry_name(reg));
        if ours.map(|c| c.0) != Some(n) {
            bad.push(registry_name(reg));
        }
    }
    assert!(bad.is_empty(), "counts differ: {bad:?}");
    assert!(r.notes.is_empty(), "{:?}", r.notes);
}

/// The SharedProperty slots in order against the local fixture, when it is
/// there (it is git-ignored and nothing else needs it any more).
#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn computed_property_slots_match_the_live_original() {
    let r = Registries::load_v14(&game_root(), true).unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/native_shared_property_slots_v14.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("no local fixture {}: nothing to compare", path.display());
        return;
    };
    let fixture: serde_json::Value =
        serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap();
    let live: Vec<String> = fixture["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_owned())
        .collect();
    let has = |list: &[String], id: &str| list.iter().any(|x| x.eq_ignore_ascii_case(id));
    for (slot, id) in live.iter().enumerate() {
        if !has(&r.property_ids, id) {
            eprintln!("live slot {slot} {id}: not computed");
        }
    }
    for (slot, id) in r.property_ids.iter().enumerate() {
        if !has(&live, id) {
            eprintln!("computed slot {slot} {id}: not live");
        }
    }
    assert_eq!(r.property_ids.len(), live.len());
    for (slot, (ours, theirs)) in r.property_ids.iter().zip(&live).enumerate() {
        assert_eq!(ours, theirs, "SharedProperty slot {slot}");
    }
}

/// A plain load of the stock archive gives the same registries as the v14
/// overlay does: its 23 files add no definition.
#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn stock_archive_load_matches_the_v14_registries() {
    let root = game_root();
    let v14 = Registries::load_v14(&root, true).unwrap();
    let stock = Registries::load_overlay(&root, None).unwrap();
    for (reg, n, _) in v14.counts() {
        assert_eq!(
            stock.count(reg).map(|c| c.0),
            Some(n),
            "{}",
            registry_name(reg)
        );
    }
    assert_eq!(stock.property_ids, v14.property_ids);
    assert_eq!(stock.ability_ids, v14.ability_ids);
}
