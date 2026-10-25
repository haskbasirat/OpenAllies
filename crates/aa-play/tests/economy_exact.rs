//! The exact kingdom economy in aa-play (`aa_play::economy`): contributions
//! through the native 570020 refresh and 0x4E4930 / 0x4E48C0 (aa-economy),
//! the 0x4E9C40 tick, the Commander experience income 0x4CDFD0 and the
//! victory checks from world time 1.0. The aa-economy arithmetic itself is
//! checked against the original x86 in aa-economy's native tests; here aa-play
//! must call it with the same values in the same order.
use aa_economy::{Catalog, Economy, F32Bits, Limits, ResourceDefinition, ResourceKind as K, State, X87Precision};
use aa_play::data::{DefClass, ResourceDef, ResourceKind};
use aa_play::{Controller, Def, GameData, GameMap, Kingdom, World};
use glam::Vec2;
use std::sync::Arc;

const MONEY: usize = 0;

fn components(names: &[&str]) -> u32 {
    names.iter().map(|n| aa_rules::native::COMPONENTS.iter().find(|(c, _)| c == n).map(|(_, b)| 1u32 << b).unwrap()).fold(0, |a, b| a | b)
}

fn data() -> GameData {
    let r = |id: &str, kind, cost: f32| ResourceDef { id: id.into(), kind, purchase_cost: vec![cost, 0.0, 0.0], limited_by: None, limited_provider: false };
    let mut data = GameData { resources: vec![r("money", ResourceKind::Stockpiled, 0.0), r("ammo", ResourceKind::Upkeep, 2.0), r("oil", ResourceKind::Upkeep, 3.0)], ..Default::default() };
    data.rules.insert("EconomyLimitedResourceMax".into(), 100.0);
    data.rules.insert("EconomyStockpileResourceMax".into(), 99999.0);
    data.rules.insert("TeamVictory".into(), 1.0);
    // A recruiting building that produces money and ammo and costs upkeep.
    data.defs.push(Def {
        id: "factory".into(),
        class: Some(DefClass::Building),
        health: 100.0,
        actor_data: true,
        components: components(&["BodyComponent", "EconomyComponent", "RecruitComponent"]),
        has_economy: true,
        production_all: vec![25.0, 7.5, 0.0],
        upkeep_all: vec![0.0, 10.1, 3.3],
        cost_all: vec![0.0; 3],
        ..Default::default()
    });
    data
}

fn world(kingdoms: u32) -> World {
    let mut w = World::empty(Arc::new(data()), Arc::new(GameMap::default()), 1);
    for i in 0..kingdoms {
        w.kingdoms.push(Kingdom {
            id: i,
            name: format!("k{i}"),
            nation: "rus".into(),
            team: i,
            color: [0; 3],
            controller: Controller::Human,
            stockpile: [0.0; 3],
            income: [0.0; 3],
            unit_limit: 0.0,
            units_used: 0.0,
            defeated: false,
            independent: false,
        });
        w.books.kingdom_mut(i, 3).stockpiled[MONEY] = 500.0;
    }
    w
}

fn reference() -> Economy {
    let r = |id: &str, kind, cost: f32| ResourceDefinition { id: id.into(), kind, limited_provider: false, purchase_cost: vec![F32Bits(cost.to_bits()), F32Bits(0), F32Bits(0)] };
    let catalog = Catalog { resources: vec![r("money", K::Stockpiled, 0.0), r("ammo", K::Upkeep, 2.0), r("oil", K::Upkeep, 3.0)] };
    let mut state = State::zeroed(3);
    state.stockpiled[MONEY] = F32Bits(500.0f32.to_bits());
    Economy::new(catalog, state, Limits { provided_maximum: 100, stockpiled_maximum: F32Bits(99999.0f32.to_bits()) }, X87Precision::Double53).unwrap()
}

fn b(v: &[f32]) -> Vec<F32Bits> {
    v.iter().map(|x| F32Bits(x.to_bits())).collect()
}

fn arrays(w: &World, k: u32) -> [Vec<u32>; 4] {
    let kb = w.books.kingdom(k).unwrap();
    let f = |v: &[f32]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    [f(&kb.raw_provided), f(&kb.provided), f(&kb.consumed), f(&kb.stockpiled)]
}

fn of(e: &Economy) -> [Vec<u32>; 4] {
    let s = e.state();
    let f = |v: &[F32Bits]| v.iter().map(|x| x.0).collect::<Vec<_>>();
    [f(&s.raw_provided), f(&s.effective_provided), f(&s.consumed), f(&s.stockpiled)]
}

/// One factory: 570020 adds its upkeep (0x4E48C0, ammo short: the shortage
/// charge buys it with money) then its production (0x4E4930); every tick
/// is 0x4E9C40. Bit-identical to aa-economy driven by hand.
#[test]
fn contributions_and_ticks_follow_the_native_order() {
    let mut w = world(1);
    let a = aa_play::production::spawn_single(&mut w, 0, Some(0), Vec2::new(10.0, 10.0), 0.0);
    let mut e = reference();
    e.adjust_consumption(&b(&[0.0, 10.1, 3.3]), true).unwrap();
    e.adjust_providers(&b(&[25.0, 7.5, 0.0]), true).unwrap();
    for _ in 0..40 {
        w.step(vec![]);
        // The reference tick (the commander-less, statistics-less 0x4E9C40).
        let mut st = aa_economy::tick::TickState { suppress_overdrawn: false, shortage_ratio: F32Bits(0), announced_ratio: F32Bits(0), snapshot_timer: F32Bits(0) };
        let mut singles = aa_economy::tick::RecruitSingles { charging: -1, items: vec![] };
        struct Quiet;
        impl aa_economy::tick::TickWorld for Quiet {
            fn gtid_live(&mut self, _: u32) -> Result<bool, aa_economy::EconomyError> { Ok(false) }
            fn upkeep_shortage(&mut self, _: i32) -> Result<(), aa_economy::EconomyError> { Ok(()) }
            fn upkeep_repaired(&mut self) -> Result<(), aa_economy::EconomyError> { Ok(()) }
            fn team_victory_check(&mut self) -> Result<(), aa_economy::EconomyError> { Ok(()) }
            fn percentage_victory_check(&mut self) -> Result<(), aa_economy::EconomyError> { Ok(()) }
            fn statistics_snapshot(&mut self) -> Result<(), aa_economy::EconomyError> { Ok(()) }
        }
        e.tick(
            aa_economy::tick::TickInput { seconds: F32Bits(0.0625f32.to_bits()), world_time: F32Bits(0), commander_experience_rate: F32Bits(0) },
            aa_economy::tick::TickParts { state: &mut st, singles: &mut singles, commander: None, statistics: None },
            &mut Quiet,
        )
        .unwrap();
    }
    assert_eq!(arrays(&w, 0), of(&e), "kingdom arrays bit-identical to aa-economy");
    // The shortage charges (0x4E1F60): the factory produces 7.5 of the 10.1
    // ammo it uses, so 2.6 ammo are bought at 2 money each, and its 3.3 oil
    // at 3 money each.
    let kb = w.books.kingdom(0).unwrap();
    assert!((kb.consumed[MONEY] - (2.6 * 2.0 + 3.3 * 3.0)).abs() < 1e-4, "{:?}", kb.consumed);
    // Remove the factory: its vectors leave exactly as they came (with the
    // native per-element shortage passes), and money stays where it was.
    w.actors[a as usize].alive = false;
    w.actors[a as usize].removed = true;
    e.adjust_consumption(&b(&[0.0, 10.1, 3.3]), false).unwrap();
    e.adjust_providers(&b(&[25.0, 7.5, 0.0]), false).unwrap();
    aa_play::economy::refresh(&mut w);
    let [raw, provided, consumed, _] = arrays(&w, 0);
    let [eraw, eprovided, econsumed, _] = of(&e);
    assert_eq!((raw, provided, consumed), (eraw, eprovided, econsumed));
    assert!(!w.books.actor(a).unwrap().economy.pays && !w.books.actor(a).unwrap().economy.provides);
}

/// Nothing changed: no refresh, no rounding drift from re-adding.
#[test]
fn an_unchanged_thing_is_not_refreshed() {
    let mut w = world(1);
    aa_play::production::spawn_single(&mut w, 0, Some(0), Vec2::new(10.0, 10.0), 0.0);
    aa_play::economy::refresh(&mut w);
    let before = arrays(&w, 0);
    for _ in 0..5 {
        aa_play::economy::refresh(&mut w);
    }
    assert_eq!(arrays(&w, 0), before);
}

/// 0x4E9960 runs from world time (GWorld+0xE0) 1.0: a kingdom left without
/// counted actors is defeated in the first tick whose begin time is 1.0, and
/// the last team standing wins.
#[test]
fn victory_checks_start_at_world_time_one() {
    let mut w = world(2);
    aa_play::production::spawn_single(&mut w, 0, Some(0), Vec2::new(10.0, 10.0), 0.0);
    for _ in 0..17 {
        w.step(vec![]);
        if w.tick < 17 {
            assert!(!w.kingdoms[1].defeated, "tick {} begins at {}", w.tick, f32::from_bits(w.clock.begin.0));
        }
    }
    assert_eq!(f32::from_bits(w.clock.begin.0), 1.0);
    assert!(w.kingdoms[1].defeated);
    assert_eq!(w.winner, Some(0));
}
