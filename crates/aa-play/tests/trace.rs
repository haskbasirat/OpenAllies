//! The parity trace (core): the native world RNG as `World` draws it
//! (0x85D840 through GetRNG 0x4F02B0), the clock of 0x4F9970 and the
//! simulation S-log emulation (`aa_play::trace`). The RNG cases are the
//! vectors the unchanged original x86 produced
//! (research/phase4-rng/native-rng-vectors.json).
use aa_play::trace::{native_unit, SKind, Trace};
use aa_play::{GameData, GameMap, World};
use std::sync::Arc;

fn world(seed: u64) -> World {
    World::empty(Arc::new(GameData::default()), Arc::new(GameMap::default()), seed)
}

fn vectors() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../research/phase4-rng/native-rng-vectors.json")).expect("vectors")
}

fn u(v: &serde_json::Value) -> u32 {
    v.as_u64().expect("u32") as u32
}

/// The S hash step, written out: `(h * 37 + v) % 65536` in i32 (C remainder).
fn hash(h: i32, v: u32) -> i32 {
    h.wrapping_mul(37).wrapping_add(v as i32) % 65536
}

#[test]
fn world_rng_is_the_native_generator() {
    let v = vectors();
    let mut seeded = 0;
    for c in v["constructors"].as_array().unwrap() {
        let state = u(&c["state"]);
        match c["address"].as_str().unwrap() {
            "0x85d870" => assert_eq!(aa_simulation::NativeRng::default().state, state),
            // CreateGWorld 0x76AA90 seeds GWorld+0xD4 through 0x85D880.
            "0x85d880" | "0x85d8a0" => {
                let w = world(u64::from(u(&c["seed"])));
                assert_eq!(w.native_rng.state, state, "seed {}", c["seed"]);
                seeded += 1;
            }
            other => panic!("unknown constructor {other}"),
        }
    }
    assert!(seeded > 0);
    let mut w = world(1);
    for c in v["bounded"].as_array().unwrap() {
        w.native_rng.state = u(&c["state_before"]);
        assert_eq!(w.random_below(u(&c["bound"])), u(&c["value"]), "{c}");
        assert_eq!(w.native_rng.state, u(&c["state_after"]), "{c}");
    }
    for c in v["drawn"].as_array().unwrap().iter().filter(|c| c["precision"] == "Double53") {
        w.native_rng.state = u(&c["state_before"]);
        assert_eq!(w.random().to_bits(), u(&c["result_bits"]), "{c}");
        assert_eq!(w.native_rng.state, u(&c["state_after"]), "{c}");
    }
}

#[test]
fn unit_float_matches_the_pc53_conversion() {
    let v = vectors();
    let mut n = 0;
    for c in v["converted"].as_array().unwrap().iter().filter(|c| c["precision"] == "Double53") {
        assert_eq!(native_unit(u(&c["value"])).to_bits(), u(&c["result_bits"]), "{c}");
        n += 1;
    }
    assert!(n > 3000);
    // The explicit PC53 arithmetic over edge values and a spread of others.
    let mut x: u32 = 1;
    let edges = [0, 1, 2, 0x7fff_ffff, 0x8000_0000, 0x8000_0001, 0xffff_fffe, 0xffff_ffff, 0xffff_ff7f, 0xffff_ff80];
    for value in edges.into_iter().chain((0..20_000).map(|_| {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        x
    })) {
        let exact = aa_simulation::native_unit_float(value, aa_numeric::Precision::Double53).expect("pc53");
        assert_eq!(native_unit(value).to_bits(), exact.0, "{value:#x}");
    }
    // The interval includes 1.0, as the original's does.
    assert_eq!(native_unit(u32::MAX), 1.0);
}

#[test]
fn every_world_draw_logs_the_getter_state() {
    let mut w = world(972_257_557);
    w.begin_battle();
    w.trace.record(true);
    let before = w.native_rng.state;
    let a = w.random_u32();
    let mid = w.native_rng.state;
    w.random();
    w.random_below(7);
    let mut lcg = aa_simulation::NativeRng { state: before };
    assert_eq!(lcg.next_u32(), a);
    assert_eq!(lcg.state, mid);
    let entries: Vec<_> = w.trace.entries().collect();
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().all(|e| e.1 == SKind::RngGetter));
    assert_eq!(entries[0].2, before);
    assert_eq!(entries[1].2, mid);
    let mut h = 0;
    for e in &entries {
        h = hash(h, e.2);
    }
    assert_eq!(w.trace.checkpoint(), (3, h));
}

#[test]
fn creation_and_battle_start_use_the_world_stream() {
    // CreateGWorld: one getter entry (default state), then 0x85D880(seed).
    let w = world(5);
    assert_eq!(w.native_rng.state, aa_simulation::NativeRng::from_native_seed(5).state);
    assert_eq!(w.trace.checkpoint(), (1, hash(0, aa_simulation::NativeRng::default().state)));
    // The sync dump uses one draw; session launch resets S to (0, 0).
    let mut b = world(5);
    b.begin_battle();
    let mut lcg = aa_simulation::NativeRng::from_native_seed(5);
    lcg.next_u32();
    assert_eq!(b.native_rng.state, lcg.state);
    assert_eq!(b.trace.checkpoint(), (0, 0));
    assert_eq!(b.trace.s_end, 1);
}

#[test]
fn ticks_log_the_begin_time_and_advance_the_native_clock() {
    let mut w = world(7);
    w.begin_battle();
    w.trace.record(true);
    for _ in 0..34 {
        w.step(vec![]);
    }
    assert_eq!(w.tick, 34);
    assert_eq!(w.clock.tick, 34);
    // end = f32(i32 tick * f32 1/16); begin = the previous end.
    assert_eq!(w.time.to_bits(), (34.0f32 * 0.0625).to_bits());
    assert_eq!(w.clock.begin.0, (33.0f32 * 0.0625).to_bits());
    let entries: Vec<_> = w.trace.entries().collect();
    assert_eq!(entries.len(), 34, "an empty world logs only the clock");
    let mut h = 0;
    for (i, e) in entries.iter().enumerate() {
        let tick = i as u32 + 1;
        // GetGameTimeBeginTick before the update: 0, 0, 1/16, 2/16...
        let begin = tick.saturating_sub(2) as f32 * 0.0625;
        assert_eq!(*e, (tick, SKind::Clock, begin.to_bits()), "tick {tick}");
        h = hash(h, begin.to_bits());
    }
    // The 311 checkpoint of record 34: the last index and the hash.
    assert_eq!(w.trace.checkpoint(), (34, h));
    assert_eq!(w.trace.digests.len(), 34);
    assert_eq!(w.trace.digests[33].s_end, 35);
    assert_eq!(w.trace.digests[33].time_bits, w.time.to_bits());
}

/// Decrees logged as the original executes them: name (one hash step per
/// UTF-16 unit) and id before the decree's effect, then the update's clock;
/// the same stream as aa-orders' Synchronizer (0x77DB10, 0x77DA70, 0x77D930).
#[test]
fn decrees_log_their_name_and_id_before_the_clock() {
    let mut w = world(9);
    w.begin_battle();
    w.trace.record(true);
    let name: Vec<u16> = "KKC_UpdateKingdomNameOrderDecree".encode_utf16().collect();
    w.step(vec![]);
    w.step_decrees(vec![
        aa_play::sim::TracedDecree { sync: Some((328, name.clone())), commands: vec![] },
        aa_play::sim::TracedDecree { sync: None, commands: vec![] },
    ]);
    let mut s = aa_orders::net::sync::Synchronizer::new(1 << 20);
    s.reset(0, 0);
    s.log_float_bits(true, "clock", 0);
    s.log_text(true, "m_decree_type.m_name", &name);
    s.log_int(true, "m_decree_type.m_id", 328);
    s.log_float_bits(true, "clock", 0);
    assert_eq!(w.trace.checkpoint(), (s.end() - 1, s.hash));
    let entries: Vec<_> = w.trace.entries().collect();
    assert_eq!(entries[1], (2, SKind::DecreeName, 328));
    assert_eq!(entries[2], (2, SKind::Int(aa_play::trace::DECREE_ID_LABEL), 328));
    assert_eq!(entries[3].1, SKind::Clock);
    assert_eq!(w.trace.decree_name(328), Some(&name[..]));
}

#[test]
fn s_kinds_hash_as_the_native_loggers() {
    let mut t = Trace::default();
    assert_eq!(t.checkpoint(), (0, 0));
    t.log(SKind::Bool("b"), 7);
    t.log(SKind::Marker("m"), 1234);
    t.log(SKind::Int("i"), 0xffff_fffe);
    t.log(SKind::Float("f"), 1.5f32.to_bits());
    let h = hash(hash(hash(hash(0, 1), 0), 0xffff_fffe), 1.5f32.to_bits());
    assert_eq!(t.checkpoint(), (4, h));
    let r = Trace::new(10, 99);
    assert_eq!(r.checkpoint(), (10, 99));
    assert_eq!(SKind::Clock.label(), "g_gworld->GetGameTimeBeginTick()");
    assert_eq!(SKind::RngGetter.label(), "(int)m_rng.GetSeed()");
}
