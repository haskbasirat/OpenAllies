//! The decree-payload registry counts of the native-order v14 catalog
//! against the live counts NativeSampler read from the original
//! (`[registry]+0x28`, QA copy, Berlin film run of 2026-10-25: `REG` lines).
//!
//! `AA_GAME_ROOT='E:\Games\Axis and Alies' cargo test -p aa-parity --test registry_counts -- --ignored --nocapture`
use aa_parity::registries::*;

#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn catalog_counts_match_the_live_original() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let r = Registries::load_v14(&root, true).unwrap();
    // Measured live on the v14 QA copy.
    let live = [(ACTOR_COMMANDS, 39), (OBJECT_DEFINITIONS, 1959), (FORMATIONS, 5), (ORGANIZATION_LAYOUTS, 10), (SHARED_PROPERTIES, 194), (ABILITIES, 409)];
    let mut bad = Vec::new();
    for (reg, n) in live {
        let ours = r.count(reg);
        eprintln!("{:20} live {n:5} catalog {ours:?}", registry_name(reg));
        if ours.map(|c| c.0) != Some(n) {
            bad.push(registry_name(reg));
        }
    }
    assert!(bad.is_empty(), "counts differ: {bad:?}");
}
