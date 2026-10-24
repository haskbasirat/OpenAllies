//! The generals' special operations on synthetic data with known numbers:
//! the experience pool (cost, periodic income, maximum), recharge and the
//! admission codes, area damage with the target filter, an enchantment's
//! modifiers and expiry, a +SELF ability on the kingdom. The ticks call only
//! `special_ops::tick` (no movement or combat), so units stay put.
use aa_play::data::{Def, DefClass, GameData};
use aa_play::map::GameMap;
use aa_play::special_ops::{self, Enchantment, General, GeneralData, OpEffect, OpModifier, OpTarget, SpecialOp};
use aa_play::world::{AbilityTarget, Command, Controller, Event, Kingdom, Order, Organization, World};
use aa_play::DT;
use glam::{Vec2, Vec3};
use std::collections::BTreeMap;
use std::sync::Arc;

const FRIEND: u32 = 1 << 0;
const ENEMY: u32 = 1 << 1;
const SELF: u32 = 1 << 5;
const BODY: u32 = 1 << 12;
const ACTOR: u32 = 1 << 13;
const INVULNERABLE: u32 = 1 << 14;
const TARGET_LOCATION: u32 = 1 << 5;
const ENCHANTMENT_FLAG: u32 = 1 << 2;
const HAS_ENCHANTMENT: u32 = 1 << 17;

fn data() -> GameData {
    let rifleman = Def { id: "rifleman".into(), class: Some(DefClass::Member), health: 200.0, defense: 1.0, targetable: true, damage_factor: vec![1.0, 1.0, 1.0, 0.8], ..Default::default() };
    let regiment = Def { id: "regiment".into(), class: Some(DefClass::Organization), slots: vec![(0, [0.0, 0.0]), (0, [2.0, 0.0])], ..Default::default() };
    let mut rules = BTreeMap::new();
    for (k, v) in [
        ("StartingExperience", 200.0),
        ("ExperienceFactionGainPeriodic", 1.0),
        ("ExperienceFactionGainMultiplier", 1.0),
        ("CombatDamageMinimum", 1.0),
        ("ModifierAttributeClampAdd", 1.0),
        ("ModifierAttributeClampMultiply", 0.05),
        ("DefaultRadius", 0.5),
    ] {
        rules.insert(k.to_string(), v);
    }
    GameData {
        by_id: [("rifleman".to_string(), 0), ("regiment".to_string(), 1)].into_iter().collect(),
        defs: vec![rifleman, regiment],
        damage_types: ["gunfire", "flame", "explosive", "armor_piercing"].map(String::from).to_vec(),
        rules,
        ..Default::default()
    }
}

/// Ability 0 a bombardment (enemy bodies in 10 m, power 100 explosive,
/// cost 300, recharge 60); 1 a defence enchantment (friendly bodies in 10 m,
/// DV +2 for 5 s, cost 100, recharge 30); 2 a +SELF flag for 10 s.
fn generals() -> GeneralData {
    let bomb = SpecialOp {
        id: "bomb".into(),
        name: "#bomb_name".into(),
        cost: 300.0,
        recharge: 60.0,
        range: f32::MAX,
        area: 10.0,
        required: ENEMY | BODY,
        forbidden: INVULNERABLE,
        flags: TARGET_LOCATION,
        power: 100.0,
        attenuation_min: 10.0,
        attenuation_factor: 1.0,
        effects: vec![OpEffect { kind: 4, amount: 100.0, damage_type: Some(2), ..Default::default() }],
        ..Default::default()
    };
    let guard = SpecialOp {
        id: "guard".into(),
        cost: 100.0,
        recharge: 30.0,
        range: f32::MAX,
        area: 10.0,
        required: FRIEND | BODY,
        forbidden: INVULNERABLE,
        flags: ENCHANTMENT_FLAG | HAS_ENCHANTMENT,
        attenuation_min: 10.0,
        attenuation_factor: 1.0,
        effects: vec![OpEffect {
            kind: 6,
            enchantment: Some(Enchantment {
                id: "property_guard".into(),
                duration: 5.0,
                kind: "ench_type_generic".into(),
                modifiers: vec![OpModifier::Add { attribute: 3, amount: 2.0 }],
                ..Default::default()
            }),
            ..Default::default()
        }],
        ..Default::default()
    };
    let scan = SpecialOp {
        id: "scan".into(),
        cost: 50.0,
        recharge: 30.0,
        range: f32::MAX,
        required: SELF | ACTOR | INVULNERABLE,
        flags: TARGET_LOCATION | HAS_ENCHANTMENT,
        attenuation_factor: 1.0,
        effects: vec![OpEffect {
            kind: 6,
            enchantment: Some(Enchantment {
                id: "property_scan".into(),
                duration: 10.0,
                kind: "ench_type_generic".into(),
                modifiers: vec![OpModifier::Flag { flag: "MINIMAP_NO_FOG".into() }],
                ..Default::default()
            }),
            ..Default::default()
        }],
        ..Default::default()
    };
    GeneralData {
        generals: vec![General { id: "tester".into(), public: true, experience_max: 5000.0, abilities: vec![bomb, guard, scan], ..Default::default() }],
        ..Default::default()
    }
}

fn kingdom(id: u32, team: u32) -> Kingdom {
    Kingdom {
        id,
        name: format!("k{id}"),
        nation: "rus".into(),
        team,
        color: [255, 0, 0],
        controller: Controller::Human,
        stockpile: [0.0; 3],
        income: [0.0; 3],
        unit_limit: 0.0,
        units_used: 0.0,
        defeated: false,
        independent: false,
    }
}

fn regiment(w: &mut World, kingdom: u32, at: Vec2) -> (u32, Vec<u32>) {
    let id = w.orgs.len() as u32;
    let members: Vec<u32> = (0..2)
        .map(|i| {
            let a = w.spawn_actor(0, Some(kingdom), Vec3::new(at.x + 2.0 * i as f32, at.y, 0.0), 0.0);
            w.actors[a as usize].org = Some(id);
            a
        })
        .collect();
    w.orgs.push(Organization { id, def: 1, kingdom, members: members.clone(), hq: None, order: Order::Idle, anchor: at, heading: 0.0, morale: 10.0, experience: 0.0, alive: true });
    (id, members)
}

/// Kingdom 0 (general "tester") with a regiment at (50, 50); kingdom 1's
/// regiments at (54, 50) (in the area) and (90, 50) (outside it).
fn world() -> (World, Vec<u32>, Vec<u32>, Vec<u32>) {
    let mut w = World::empty(Arc::new(data()), Arc::new(GameMap::default()), 3);
    w.kingdoms = vec![kingdom(0, 0), kingdom(1, 1)];
    let (_, own) = regiment(&mut w, 0, Vec2::new(50.0, 50.0));
    let (_, near) = regiment(&mut w, 1, Vec2::new(54.0, 50.0));
    let (_, far) = regiment(&mut w, 1, Vec2::new(90.0, 50.0));
    special_ops::attach(&mut w, Arc::new(generals()));
    special_ops::set_general(&mut w, 0, Some("tester"));
    (w, own, near, far)
}

fn tick(w: &mut World) {
    w.events.clear();
    special_ops::tick(w);
    w.tick += 1;
    w.time = w.tick as f32 * DT;
}

fn apply(w: &mut World, ability: u32, target: Option<AbilityTarget>) -> Result<(), String> {
    w.events.clear();
    w.apply(Command::UseAbility { kingdom: 0, ability, target });
    match w.events.iter().find_map(|e| if let Event::Refused { reason, .. } = e { Some(reason.clone()) } else { None }) {
        Some(r) => Err(r),
        None => Ok(()),
    }
}

#[test]
fn experience_cost_recharge_and_income() {
    let (mut w, _, near, _) = world();
    assert_eq!(special_ops::experience(&w, 0), Some((200.0, 5000.0)));
    // 300 needed: refused for the cost, the HUD button disabled
    let v = special_ops::view(&w, 0, 0).unwrap();
    assert!(!v.affordable && !v.usable);
    assert_eq!(apply(&mut w, 0, Some(AbilityTarget::Actor(near[0]))), Err("special_operation_not_enough_experience".into()));
    // +1 per second (ExperienceFactionGainPeriodic)
    for _ in 0..(100.0 / DT) as u32 {
        tick(&mut w);
    }
    let (xp, _) = special_ops::experience(&w, 0).unwrap();
    assert!((xp - 300.0).abs() < 0.01, "{xp}");
    apply(&mut w, 0, Some(AbilityTarget::Actor(near[0]))).unwrap();
    let (xp, _) = special_ops::experience(&w, 0).unwrap();
    assert!(xp.abs() < 0.01, "{xp}");
    // recharging: code 8, refused, the bar at 0 and rising
    let v = special_ops::view(&w, 0, 0).unwrap();
    assert_eq!((v.available, v.usable), (8, false));
    assert!((v.remaining - 60.0).abs() < 0.01);
    special_ops::set_starting_experience(&mut w, 1000.0);
    assert_eq!(apply(&mut w, 0, Some(AbilityTarget::Actor(near[0]))), Err("special_operation_recharging".into()));
    for _ in 0..(60.0 / DT) as u32 + 1 {
        tick(&mut w);
    }
    assert!(special_ops::view(&w, 0, 0).unwrap().usable);
    // the maximum caps the balance
    special_ops::set_starting_experience(&mut w, 9000.0);
    assert_eq!(special_ops::experience(&w, 0), Some((5000.0, 5000.0)));
}

#[test]
fn bombardment_hits_enemies_in_the_area_only() {
    let (mut w, own, near, far) = world();
    special_ops::set_starting_experience(&mut w, 1000.0);
    // aimed at the ground between the regiments
    apply(&mut w, 0, Some(AbilityTarget::Point(Vec2::new(52.0, 50.0)))).unwrap();
    // 100 power, offense 1, defense 1, explosive factor 1: the damage formula
    let expected = aa_play::combat::damage_amount(100.0, 1.0, 1.0, aa_play::combat::modified_defense(1.0, 0.0, 1.0), 1.0, TARGET_LOCATION, 1.0);
    for &a in &near {
        let h = w.actors[a as usize].health;
        assert!((200.0 - h - expected).abs() < 0.01, "near {a}: {h}, expected -{expected}");
    }
    for &a in own.iter().chain(&far) {
        assert_eq!(w.actors[a as usize].health, 200.0, "untouched {a}");
    }
    assert!(w.events.iter().any(|e| matches!(e, Event::SpecialOperation { kingdom: 0, .. })));
}

#[test]
fn enchantment_adds_defense_and_expires() {
    let (mut w, own, near, _) = world();
    special_ops::set_starting_experience(&mut w, 1000.0);
    apply(&mut w, 1, Some(AbilityTarget::Actor(own[0]))).unwrap();
    for &a in &own {
        assert_eq!(special_ops::mods(&w, a).map(|m| m.dv), Some(2.0), "own {a}");
        assert_eq!(special_ops::enchantments_on(&w, OpTarget::Actor(a)).len(), 1);
    }
    // friendly only: the enemy in the area is not enchanted
    for &a in &near {
        assert!(special_ops::mods(&w, a).is_none_or(|m| m.dv == 0.0), "enemy {a}");
    }
    for _ in 0..(5.0 / DT) as u32 + 2 {
        tick(&mut w);
    }
    for &a in &own {
        assert!(special_ops::mods(&w, a).is_none_or(|m| m.dv == 0.0), "expired {a}");
        assert!(special_ops::enchantments_on(&w, OpTarget::Actor(a)).is_empty());
    }
}

#[test]
fn self_ability_enchants_the_kingdom() {
    let (mut w, ..) = world();
    let v = special_ops::view(&w, 0, 2).unwrap();
    assert!(v.self_target);
    apply(&mut w, 2, None).unwrap();
    assert!(special_ops::kingdom_has_flag(&w, 0, "MINIMAP_NO_FOG"));
    assert!(!special_ops::kingdom_has_flag(&w, 1, "MINIMAP_NO_FOG"));
    assert!((special_ops::experience(&w, 0).unwrap().0 - 150.0).abs() < 0.01);
    for _ in 0..(10.0 / DT) as u32 + 2 {
        tick(&mut w);
    }
    assert!(!special_ops::kingdom_has_flag(&w, 0, "MINIMAP_NO_FOG"));
    // an ability that is not +SELF needs a target
    special_ops::set_starting_experience(&mut w, 1000.0);
    assert_eq!(apply(&mut w, 1, None), Err("special_operation_requires_target".into()));
}
