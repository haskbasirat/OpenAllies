//! Unit checks of the native rules special_ops follows (synthetic data).
use super::*;
use crate::data::{Def, DefClass, GameData};
use crate::map::GameMap;
use crate::world::{Controller, Kingdom};
use glam::Vec3;
use std::collections::BTreeMap;

fn data() -> GameData {
    let rifleman = Def { id: "rifleman".into(), class: Some(DefClass::Member), health: 200.0, defense: 1.0, targetable: true, damage_factor: vec![1.0; 4], ..Default::default() };
    let mut rules = BTreeMap::new();
    for (k, v) in [("ExperienceFactionGainMultiplier", 2.0), ("ModifierAttributeClampMultiply", 0.05), ("DefaultRadius", 0.5)] {
        rules.insert(k.to_string(), v);
    }
    GameData {
        by_id: [("rifleman".to_string(), 0)].into_iter().collect(),
        defs: vec![rifleman],
        damage_types: ["gunfire", "flame", "explosive", "armor_piercing"].map(String::from).to_vec(),
        rules,
        ..Default::default()
    }
}

fn kingdom(id: u32, team: u32) -> Kingdom {
    Kingdom {
        id,
        name: format!("k{id}"),
        nation: "rus".into(),
        team,
        color: [0, 0, 0],
        controller: Controller::Human,
        stockpile: [0.0; 3],
        income: [0.0; 3],
        unit_limit: 0.0,
        units_used: 0.0,
        defeated: false,
        independent: false,
    }
}

/// Land on the west half (flags 0x200: not water passable), water on the
/// east half (0x120: not land passable, terrain type 2).
fn island_map() -> GameMap {
    let (w, h) = (64u32, 64u32);
    let flags = (0..w * h).map(|t| if t % w < 32 { 0x200 } else { 0x120 }).collect();
    GameMap { width: w, height: h, cell_size: 2.0, flags, heights: vec![0.0; ((w + 1) * (h + 1)) as usize], ..Default::default() }
}

fn world_with(map: GameMap) -> World {
    let mut w = World::empty(Arc::new(data()), Arc::new(map), 1);
    w.kingdoms = vec![kingdom(0, 0), kingdom(1, 1)];
    w
}

/// An ability with two enchantment effects of `duration` and `duration2`.
fn enchanting(duration: f32, duration2: f32) -> GeneralData {
    let effect = |id: &str, d: f32| OpEffect {
        kind: ENCHANTMENT,
        enchantment: Some(Enchantment { id: id.into(), duration: d, kind: "ench_type_generic".into(), modifiers: vec![OpModifier::Add { attribute: DV, amount: 1.0 }], ..Default::default() }),
        ..Default::default()
    };
    let op = SpecialOp { id: "two".into(), cost: 0.0, recharge: 1.0, range: f32::MAX, flags: FLAG_TARGET_LOCATION, attenuation_factor: 1.0, effects: vec![effect("a", duration), effect("b", duration2)], ..Default::default() };
    GeneralData { generals: vec![General { id: "g".into(), public: true, experience_max: 5000.0, abilities: vec![op], ..Default::default() }], ..Default::default() }
}

#[test]
fn target_flags_neutral_and_independent_follow_4c5840() {
    let mut w = world_with(GameMap::default());
    let own = w.spawn_actor(0, Some(0), Vec3::ZERO, 0.0);
    let foe = w.spawn_actor(0, Some(1), Vec3::ZERO, 0.0);
    let none = w.spawn_actor(0, None, Vec3::ZERO, 0.0);
    let f = |t| target_flags(&w, 0, OpTarget::Actor(t));
    assert_eq!(f(own) & (BIT_FRIEND | BIT_ENEMY | BIT_NEUTRAL | BIT_INDEPENDENT | BIT_OWN), BIT_FRIEND | BIT_OWN);
    assert_eq!(f(foe) & (BIT_FRIEND | BIT_ENEMY | BIT_NEUTRAL | BIT_INDEPENDENT | BIT_OWN), BIT_ENEMY);
    // no owner: neither +138 nor +140 -> neutral, and independent
    assert_eq!(f(none) & (BIT_FRIEND | BIT_ENEMY | BIT_NEUTRAL | BIT_INDEPENDENT | BIT_OWN), BIT_NEUTRAL | BIT_INDEPENDENT);
    assert_ne!(f(own) & BIT_BODY, 0);
    assert_ne!(f(own) & BIT_ACTOR, 0);
}

#[test]
fn clamp_keeps_targets_a_cell_inside_552640() {
    let w = world_with(island_map());
    let p = clamp_interior(&w, Vec2::new(-5.0, 500.0));
    assert_eq!(p.x, 2.0);
    assert_eq!(p.y, (128.0f64 - 2.0 - f64::from(f32::from_bits(0x38d1_b717))) as f32);
    let q = clamp_interior(&w, Vec2::new(10.0, 20.0));
    assert_eq!(q, Vec2::new(10.0, 20.0));
}

#[test]
fn primary_island_only_needs_the_main_land() {
    let mut w = world_with(island_map());
    assert!(on_primary_island(&mut w, Vec2::new(20.0, 64.0)));
    assert!(!on_primary_island(&mut w, Vec2::new(100.0, 64.0)), "water");
    assert!(!on_primary_island(&mut w, Vec2::new(-1.0, 64.0)), "outside the map");
}

#[test]
fn properties_tick_passes_over_the_entry_after_a_removed_one() {
    let mut w = world_with(GameMap::default());
    attach(&mut w, Arc::new(enchanting(5.0, 5.0)));
    set_general(&mut w, 0, Some("g"));
    let a = w.spawn_actor(0, Some(0), Vec3::ZERO, 0.0);
    for effect in 0..2 {
        w.special_ops.enchantments.push(Enchanted { target: OpTarget::Actor(a), kingdom: 0, general: 0, ability: 0, effect, created: 0.0 });
    }
    // both expired at 5 s: the first goes, the second (shifted into its
    // place) waits for the next Tick (5818D0 increments after removal)
    w.clock.begin = aa_simulation::FloatBits(6.0f32.to_bits());
    run_properties(&mut w, PropertiesOwner::Actor(a), 1.0);
    assert_eq!(w.special_ops.enchantments.len(), 1);
    assert_eq!(w.special_ops.enchantments[0].effect, 1);
    run_properties(&mut w, PropertiesOwner::Actor(a), 1.0);
    assert!(w.special_ops.enchantments.is_empty());
}

#[test]
fn expiry_is_not_below_the_duration_5031b0() {
    assert!(!expired(4.9375, 0.0, 5.0));
    assert!(expired(5.0, 0.0, 5.0));
    assert!(!expired(f32::NAN, 0.0, 5.0));
}

#[test]
fn nearest_sort_lets_the_right_half_win_ties() {
    let mut w = world_with(GameMap::default());
    let a = w.spawn_actor(0, Some(0), Vec3::new(1.0, 0.0, 0.0), 0.0);
    let b = w.spawn_actor(0, Some(0), Vec3::new(-1.0, 0.0, 0.0), 0.0);
    let c = w.spawn_actor(0, Some(0), Vec3::new(0.5, 0.0, 0.0), 0.0);
    let mut list = vec![OpTarget::Actor(a), OpTarget::Actor(b), OpTarget::Actor(c)];
    sort_nearest(&w, &mut list, Vec2::ZERO);
    assert_eq!(list, vec![OpTarget::Actor(c), OpTarget::Actor(b), OpTarget::Actor(a)]);
}

#[test]
fn organization_gains_credit_the_kingdom_times_the_multiplier() {
    let mut w = world_with(GameMap::default());
    attach(&mut w, Arc::new(enchanting(5.0, 5.0)));
    set_general(&mut w, 0, Some("g"));
    set_starting_experience(&mut w, 100.0);
    credit(&mut w, 0, 10.0);
    assert_eq!(experience(&w, 0).map(|x| x.0), Some(120.0));
    credit(&mut w, 0, 10_000.0);
    assert_eq!(experience(&w, 0).map(|x| x.0), Some(5000.0));
}

#[test]
fn recharge_reads_gworld_e0() {
    let mut w = world_with(GameMap::default());
    attach(&mut w, Arc::new(enchanting(5.0, 5.0)));
    set_general(&mut w, 0, Some("g"));
    w.clock.begin = aa_simulation::FloatBits(10.0f32.to_bits());
    w.special_ops.commanders[0].last_use[0] = Some(9.5);
    assert_eq!(recharge_remaining(&w, 0, 0), 0.5);
    w.clock.begin = aa_simulation::FloatBits(10.5f32.to_bits());
    assert_eq!(recharge_remaining(&w, 0, 0), 0.0);
}

#[test]
fn followup_flare_reaches_the_player_and_the_sai_stream_is_shared() {
    // 7C3C10: a flagged power is ordered and its Followup makes an attack
    // flare (7B0610) on the target's region; the draws are the SAI's own.
    let mut w = world_with(island_map());
    let mut general = enchanting(5.0, 5.0);
    let power = PowerConfig {
        ability: "two".into(),
        location: 2,
        actor_requirements: Requirements { owned: true, ..Default::default() },
        base: 100.0,
        fuzzy: 0.0,
        flare_priority: 500.0,
        flare_duration: 60.0,
        ..Default::default()
    };
    general.ego_powers = vec![("e".into(), vec![power])];
    attach(&mut w, Arc::new(general));
    set_general(&mut w, 0, Some("g"));
    let a = w.spawn_actor(0, Some(0), Vec3::new(20.0, 64.0, 0.0), 0.0);
    w.ai.data = Some(Arc::new(crate::ai::AiData { egos: vec![crate::ai::data::Ego { ids: "e".into(), ..Default::default() }], ..Default::default() }));
    w.ai.regions = Some(crate::ai::regions::Regions::squares(&w.map, 10));
    w.ai.players.push(crate::ai::Player { kingdom: 0, ge_count: 1, ..Default::default() });
    assert!(w.ai.rng.is_none());
    sai::think(&mut w);
    assert_eq!(w.special_ops.sai.uses, 1);
    // the order runs as a decree of the next tick (0x752E60)
    for c in std::mem::take(&mut w.sai_orders) {
        w.apply(c);
    }
    assert_eq!(w.special_ops.enchantments.iter().filter(|e| e.target == OpTarget::Actor(a)).count(), 2);
    let region = w.ai.regions.as_ref().unwrap().region_at(Vec2::new(20.0, 64.0));
    assert_eq!(w.ai.players[0].flares, vec![(crate::ai::goals::Key::Attack(region), 500.0, 60.0)]);
    assert!(w.ai.rng.is_some(), "the draws come from the SAI stream");
}
