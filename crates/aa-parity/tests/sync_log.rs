//! What the films' S checkpoints are made of (research/byte-exact/SYNC_LOG.md).
//!
//! Synthetic: the decree entries and the clock reproduce a known hash.
//! Real data (ignored, QA films + AA_GAME_ROOT):
//! * every interval between checkpoints is `clock + decree entries +
//!   residual`, and every residual-0 interval's hash is reproduced exactly;
//! * the residuals are world-RNG getter entries, each followed by exactly one
//!   draw: a single starting low half explains the first 16 intervals of
//!   films 0 and 1, and film 2's first checkpoint is 17 clocks, 10 decree
//!   entries and 3 getters (two in update 8, one while the recruit decree of
//!   record 8 is processed).
use aa_parity::Film;
use aa_parity::chain::{self, Slot};
use aa_parity::registries::Registries;
use aa_parity::slog::{self, SEntryKind, SLog};
use std::path::PathBuf;

const FILMS: &str = r"E:\Games\Axis and Alies\reconstruction\qa-runs\compare-20261022\game\Profile\data\Films\archive-qa-20261022";

fn film(name: &str) -> Film {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let dir =
        std::env::var_os("AA_PARITY_FILMS").map_or_else(|| PathBuf::from(FILMS), PathBuf::from);
    Film::read(&dir.join(name), &Registries::load_v14(&root, true).unwrap()).unwrap()
}

#[test]
fn decree_entries_follow_the_native_shapes() {
    // SetPlayer never logs to S; the film decrees all do (s_log shapes).
    assert_eq!(slog::decree_s_entries(298), None);
    for (descriptor, name) in [
        (311, "KKC_SetSimSyncValueOrderDecree"),
        (333, "SelectionSetOrderDecree"),
        (86, "KKC_TellSelectedCommandOrderDecree"),
        (328, "KKC_UpdateKingdomNameOrderDecree"),
        (16, "SelectionAddOrderDecree"),
        (18, "SelectionClearOrderDecree"),
    ] {
        assert_eq!(slog::decree_s_entries(descriptor), Some(name));
    }
    // The timeline decrees do not log to S.
    assert_eq!(slog::decree_s_entries(73), None);
    // A decree adds a text entry (every unit hashed) and an id entry.
    let mut log = SLog::new();
    log.push_decree(1, 333);
    let mut h = 0i32;
    for u in "SelectionSetOrderDecree".encode_utf16() {
        h = aa_orders::net::sync::step(h, u32::from(u));
    }
    h = aa_orders::net::sync::step(h, 333);
    assert_eq!(log.checkpoint(), (2, h));
}

#[test]
fn low_half_lcg_matches_the_native_generator() {
    let mut rng = 0x1234_5678u32;
    let mut low = rng & 0xffff;
    for _ in 0..1000 {
        rng = rng.wrapping_mul(0x41c6_4e6d).wrapping_add(0x3039);
        low = chain::lcg16(low);
        assert_eq!(rng & 0xffff, low);
    }
    assert_eq!(
        chain::lcg16_n(0x1501, 3),
        chain::lcg16(chain::lcg16(chain::lcg16(0x1501)))
    );
}

#[test]
#[ignore = "reads the QA films and the original data; set AA_GAME_ROOT"]
fn film_intervals_are_clock_decrees_and_rng_getters() {
    for name in ["LastFilm_0.RFM", "LastFilm_1.RFM", "LastFilm_2.RFM"] {
        let f = film(name);
        let account = slog::account(&f);
        let first = account[0];
        assert_eq!(
            (
                first.entries,
                first.clock,
                first.decree_entries,
                first.residual
            ),
            (30, 17, 10, 3),
            "{name}"
        );
        assert!(account.iter().all(|i| i.residual >= 0), "{name}");
        let verified = slog::verify_zero_residual(&f);
        assert!(!verified.is_empty());
        assert!(
            verified.iter().all(|v| v.1 == v.2),
            "{name}: every clock+decree-only interval hash"
        );
    }
}

#[test]
#[ignore = "reads the QA films and the original data; set AA_GAME_ROOT"]
fn one_draw_per_getter_explains_the_first_intervals() {
    for (name, x) in [("LastFilm_0.RFM", 0x1551u32), ("LastFilm_1.RFM", 0x7ee1)] {
        let f = film(name);
        let found = chain::search(&f, 16, 1);
        assert_eq!(
            found.iter().map(|s| s.0).collect::<Vec<_>>(),
            vec![x],
            "{name}"
        );
        // the first interval: two getters in update 7, one while the 86 of
        // record 7 (the recruit) is processed
        let first = &found[0].1[0];
        assert_eq!(
            first.getters.iter().map(|g| g.0).collect::<Vec<_>>(),
            vec![
                Slot::Update { update: 7 },
                Slot::Update { update: 7 },
                Slot::Decree {
                    record: 7,
                    index: 2,
                    descriptor: 86
                }
            ],
            "{name}"
        );
    }
    let f = film("LastFilm_2.RFM");
    let found = chain::search(&f, 6, 1);
    assert_eq!(
        found.iter().map(|s| s.0).collect::<Vec<_>>(),
        vec![0x1501, 0x9501]
    );
    let first = &chain::prepare(&f)[0];
    let placements = chain::placements(first, 0x1501, 1, 16);
    assert!(placements.contains(&vec![
        Slot::Update { update: 8 },
        Slot::Update { update: 8 },
        Slot::Decree {
            record: 8,
            index: 2,
            descriptor: 86
        }
    ]));
}

#[test]
#[ignore = "reads QA film 2 and the original data; set AA_GAME_ROOT"]
fn film_2_first_checkpoint_entry_by_entry() {
    // The 30 entries of film 2's first checkpoint, in order, with the getter
    // states' low halves 0x1501, lcg16(0x1501), lcg16^2(0x1501) (any high
    // halves give the same hash mod 65536).
    // Order: before update k come the decrees of record k - 1 (its admin
    // tick); update k logs its clock, then its getters. Record 17's decrees
    // follow the checkpoint.
    let f = film("LastFilm_2.RFM");
    let mut exact = SLog::new();
    let mut state = 0x1501u32;
    for k in 1..=17u32 {
        exact.push(k, SEntryKind::Clock, slog::clock_bits_before_update(k));
        if k == 8 {
            for _ in 0..2 {
                exact.push(8, SEntryKind::RngGetter, state);
                state = chain::lcg16(state);
            }
        }
        if k < 17 {
            for d in f.decrees_at(k) {
                exact.push_decree(k, d.descriptor as usize);
                if k == 8 && d.descriptor == 86 {
                    exact.push(8, SEntryKind::RngGetter, state);
                    state = chain::lcg16(state);
                }
            }
        }
    }
    let first = f.checkpoints[0];
    assert_eq!(first.tick, 17);
    let (count, hash) = exact.checkpoint();
    assert_eq!(count, first.count);
    assert_eq!(hash.rem_euclid(65536), first.hash.rem_euclid(65536));
}
